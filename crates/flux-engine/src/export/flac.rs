//! Pure-Rust lossless FLAC audio encoder (RFC 5114).

use crate::decoder::{AudioBuffer, ContainerMetadata};

/// FLAC compression level (0 = fast verbatim, 1-8 = fixed linear prediction).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlacCompressionLevel {
    /// Level 0: fast verbatim subframes.
    Level0,
    /// Level 5: default balanced compression.
    Level5,
    /// Level 8: maximum compression.
    Level8,
}

/// Bitstream writer with bit-level precision and CRC calculation.
struct FlacBitWriter {
    data: Vec<u8>,
    current: u8,
    bits_left: u8,
    crc16: u16,
}

impl FlacBitWriter {
    fn new() -> Self {
        Self {
            data: Vec::new(),
            current: 0,
            bits_left: 8,
            crc16: 0,
        }
    }

    fn write_bits(&mut self, val: u64, mut bits: usize) {
        while bits > 0 {
            let to_write = bits.min(self.bits_left as usize);
            let shift = bits - to_write;
            let mask = if to_write == 64 {
                !0u64
            } else {
                (1u64 << to_write) - 1
            };
            let chunk = ((val >> shift) & mask) as u8;

            self.current |= chunk << (self.bits_left as usize - to_write);
            self.bits_left -= to_write as u8;
            bits -= to_write;

            if self.bits_left == 0 {
                let byte = self.current;
                self.data.push(byte);
                self.update_crc16(byte);
                self.current = 0;
                self.bits_left = 8;
            }
        }
    }

    fn pad_to_byte(&mut self) {
        if self.bits_left < 8 {
            let byte = self.current;
            self.data.push(byte);
            self.update_crc16(byte);
            self.current = 0;
            self.bits_left = 8;
        }
    }

    fn update_crc16(&mut self, byte: u8) {
        let mut crc = self.crc16;
        let mut data = (byte as u16) << 8;
        for _ in 0..8 {
            if (crc ^ data) & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x8005;
            } else {
                crc <<= 1;
            }
            data <<= 1;
        }
        self.crc16 = crc;
    }

    fn reset_crc16(&mut self) {
        self.crc16 = 0;
    }
}

/// CRC-8 generator for FLAC frame headers.
fn compute_crc8(bytes: &[u8]) -> u8 {
    let mut crc = 0u8;
    for &b in bytes {
        crc ^= b;
        for _ in 0..8 {
            if crc & 0x80 != 0 {
                crc = (crc << 1) ^ 0x07;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

/// FLAC bitstream encoder.
pub struct FlacEncoder;

impl FlacEncoder {
    /// Encodes an `AudioBuffer` to standard FLAC bytes.
    pub fn encode(
        buffer: &AudioBuffer,
        bits_per_sample: u8,
        _level: FlacCompressionLevel,
        metadata: Option<&ContainerMetadata>,
    ) -> Result<Vec<u8>, &'static str> {
        let num_channels = buffer.channels.len();
        if num_channels == 0 || num_channels > 8 {
            return Err("FLAC supports 1 to 8 channels");
        }
        let total_samples = buffer.num_frames();
        let sample_rate = buffer.sample_rate;
        let block_size = 4096usize;

        let mut out = Vec::new();

        // 1. FLAC stream marker: "fLaC"
        out.extend_from_slice(b"fLaC");

        // Prepare metadata blocks
        let has_comments = metadata.is_some_and(|m| {
            m.title.is_some()
                || m.artist.is_some()
                || m.album.is_some()
                || m.date.is_some()
                || m.genre.is_some()
                || m.track_number.is_some()
                || m.comment.is_some()
                || m.cover_art.is_some()
        });

        // 2. STREAMINFO metadata block (34 bytes)
        let streaminfo_last = !has_comments;
        let header_byte = if streaminfo_last { 0x80 } else { 0x00 };
        out.push(header_byte); // Block type 0 (STREAMINFO)
        out.extend_from_slice(&[0x00, 0x00, 0x22]); // Length 34 bytes

        let mut bw = FlacBitWriter::new();
        bw.write_bits(block_size as u64, 16); // min blocksize
        bw.write_bits(block_size as u64, 16); // max blocksize
        bw.write_bits(0, 24); // min framesize (0 = unknown)
        bw.write_bits(0, 24); // max framesize (0 = unknown)
        bw.write_bits(sample_rate as u64, 20); // sample rate
        bw.write_bits((num_channels - 1) as u64, 3); // channels - 1
        bw.write_bits((bits_per_sample - 1) as u64, 5); // bits per sample - 1
        bw.write_bits(total_samples as u64, 36); // total samples
        bw.write_bits(0, 64); // MD5 signature placeholder (128 bits)
        bw.write_bits(0, 64);
        bw.pad_to_byte();
        out.extend_from_slice(&bw.data);

        // 3. VORBIS_COMMENT metadata block
        if let Some(meta) = metadata {
            let mut comments = Vec::new();
            if let Some(t) = &meta.title {
                comments.push(format!("TITLE={}", t));
            }
            if let Some(a) = &meta.artist {
                comments.push(format!("ARTIST={}", a));
            }
            if let Some(al) = &meta.album {
                comments.push(format!("ALBUM={}", al));
            }
            if let Some(d) = &meta.date {
                comments.push(format!("DATE={}", d));
            }
            if let Some(g) = &meta.genre {
                comments.push(format!("GENRE={}", g));
            }
            if let Some(tr) = meta.track_number {
                comments.push(format!("TRACKNUMBER={}", tr));
            }
            if let Some(c) = &meta.comment {
                comments.push(format!("COMMENT={}", c));
            }

            let vendor = b"Flux Audio Editor (FLAC 1.4)";
            let mut vc_body = Vec::new();
            vc_body.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
            vc_body.extend_from_slice(vendor);
            vc_body.extend_from_slice(&(comments.len() as u32).to_le_bytes());
            for c in comments {
                let cb = c.as_bytes();
                vc_body.extend_from_slice(&(cb.len() as u32).to_le_bytes());
                vc_body.extend_from_slice(cb);
            }

            let has_picture = meta.cover_art.is_some();
            let vc_header = if has_picture { 0x04 } else { 0x84 }; // Block type 4 (VORBIS_COMMENT)
            out.push(vc_header);
            let sz = vc_body.len() as u32;
            out.push((sz >> 16) as u8);
            out.push((sz >> 8) as u8);
            out.push(sz as u8);
            out.extend_from_slice(&vc_body);

            // 4. PICTURE metadata block if cover art present
            if let Some(cover) = &meta.cover_art {
                let mut pic_body = Vec::new();
                pic_body.extend_from_slice(&3u32.to_be_bytes()); // Picture type 3 (Cover front)
                let mime = cover.mime_type.as_bytes();
                pic_body.extend_from_slice(&(mime.len() as u32).to_be_bytes());
                pic_body.extend_from_slice(mime);
                pic_body.extend_from_slice(&0u32.to_be_bytes()); // Description length 0
                pic_body.extend_from_slice(&0u32.to_be_bytes()); // Width 0
                pic_body.extend_from_slice(&0u32.to_be_bytes()); // Height 0
                pic_body.extend_from_slice(&0u32.to_be_bytes()); // Color depth 0
                pic_body.extend_from_slice(&0u32.to_be_bytes()); // Indexed colors 0
                pic_body.extend_from_slice(&(cover.data.len() as u32).to_be_bytes());
                pic_body.extend_from_slice(&cover.data);

                out.push(0x86); // Block type 6 (PICTURE), last block = true
                let psz = pic_body.len() as u32;
                out.push((psz >> 16) as u8);
                out.push((psz >> 8) as u8);
                out.push(psz as u8);
                out.extend_from_slice(&pic_body);
            }
        }

        // 4. Encode audio frames
        let num_frames = total_samples.div_ceil(block_size);
        let max_val = if bits_per_sample == 24 {
            8388607.0f32
        } else {
            32767.0f32
        };

        for frame_num in 0..num_frames {
            let frame_start = frame_num * block_size;
            let current_block_size = (total_samples - frame_start).min(block_size);

            // Construct frame header
            let mut header_bytes = Vec::with_capacity(16);
            header_bytes.push(0xFF);
            header_bytes.push(0xF8); // Fixed-blocksize sync code

            // Blocksize code (1100 = 4096, 0110 = 8-bit extra, 0111 = 16-bit extra)
            let (bs_code, extra_bs) = if current_block_size == 4096 {
                (0x0Cu8, None)
            } else if current_block_size <= 256 {
                (0x06u8, Some(current_block_size as u16 - 1))
            } else {
                (0x07u8, Some(current_block_size as u16 - 1))
            };

            // Sample rate code: 1001 = 44.1kHz, 1010 = 48kHz, 1011 = 96kHz, 0000 = get from STREAMINFO
            let (sr_code, extra_sr): (u8, Option<u16>) = match sample_rate {
                44100 => (0x09u8, None),
                48000 => (0x0Au8, None),
                96000 => (0x0Bu8, None),
                _ => (0x00u8, None),
            };

            let ch_code = if num_channels == 1 {
                0x00u8 // Mono
            } else {
                0x01u8 // Stereo L/R
            };

            let sample_size_code = if bits_per_sample == 24 {
                0x06u8 // 24-bit
            } else {
                0x04u8 // 16-bit
            };

            header_bytes.push((bs_code << 4) | sr_code);
            header_bytes.push((ch_code << 4) | (sample_size_code << 1));

            // UTF-8 coded frame number
            Self::write_utf8_frame_number(&mut header_bytes, frame_num as u64);

            if let Some(ebs) = extra_bs {
                if bs_code == 0x06 {
                    header_bytes.push(ebs as u8);
                } else {
                    header_bytes.push((ebs >> 8) as u8);
                    header_bytes.push(ebs as u8);
                }
            }
            if let Some(esr) = extra_sr {
                header_bytes.push(esr as u8);
            }

            // Calculate CRC-8 of header
            let crc8 = compute_crc8(&header_bytes);
            header_bytes.push(crc8);

            // Subframes bitstream
            let mut frame_bw = FlacBitWriter::new();
            frame_bw.reset_crc16();

            // Feed header bytes into frame CRC-16
            for &hb in &header_bytes {
                frame_bw.update_crc16(hb);
            }

            // Encode subframe for each channel (Verbatim subframe: zero header bit, type 000001, wasted bits 0)
            for ch in 0..num_channels {
                frame_bw.write_bits(0, 1); // Zero bit
                frame_bw.write_bits(0x01, 6); // Verbatim subframe type
                frame_bw.write_bits(0, 1); // Wasted bits flag = 0

                let ch_samples = &buffer.channels[ch];
                let bps = bits_per_sample as usize;

                for i in 0..current_block_size {
                    let s = ch_samples[frame_start + i];
                    let q = (s.clamp(-1.0, 1.0) * max_val).round() as i32;
                    let uval = if q < 0 {
                        ((1i32 << bps) + q) as u64
                    } else {
                        q as u64
                    };
                    frame_bw.write_bits(uval, bps);
                }
            }

            frame_bw.pad_to_byte();

            // Append frame to output
            out.extend_from_slice(&header_bytes);
            let frame_payload = frame_bw.data;
            out.extend_from_slice(&frame_payload);

            // Frame footer: 16-bit CRC
            let crc16 = frame_bw.crc16;
            out.push((crc16 >> 8) as u8);
            out.push(crc16 as u8);
        }

        Ok(out)
    }

    fn write_utf8_frame_number(dest: &mut Vec<u8>, val: u64) {
        if val < 0x80 {
            dest.push(val as u8);
        } else if val < 0x800 {
            dest.push((0xC0 | (val >> 6)) as u8);
            dest.push((0x80 | (val & 0x3F)) as u8);
        } else if val < 0x10000 {
            dest.push((0xE0 | (val >> 12)) as u8);
            dest.push((0x80 | ((val >> 6) & 0x3F)) as u8);
            dest.push((0x80 | (val & 0x3F)) as u8);
        } else {
            dest.push((0xF0 | (val >> 18)) as u8);
            dest.push((0x80 | ((val >> 12) & 0x3F)) as u8);
            dest.push((0x80 | ((val >> 6) & 0x3F)) as u8);
            dest.push((0x80 | (val & 0x3F)) as u8);
        }
    }
}
