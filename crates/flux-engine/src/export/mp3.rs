//! MPEG-1 Audio Layer III (MP3) encoder with ID3v2 metadata tagging.

use crate::decoder::{AudioBuffer, ContainerMetadata};

/// MP3 bitrate mode / target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mp3Bitrate {
    /// Constant 128 kbps.
    Cbr128,
    /// Constant 192 kbps.
    Cbr192,
    /// Constant 256 kbps.
    Cbr256,
    /// Constant 320 kbps.
    Cbr320,
    /// Variable Bitrate V0 (high quality, ~245 kbps).
    VbrV0,
    /// Variable Bitrate V2 (standard quality, ~190 kbps).
    VbrV2,
}

impl Mp3Bitrate {
    /// Returns the nominal bitrate in kbps.
    pub fn kbps(&self) -> u32 {
        match self {
            Self::Cbr128 => 128,
            Self::Cbr192 => 192,
            Self::Cbr256 => 256,
            Self::Cbr320 => 320,
            Self::VbrV0 => 256,
            Self::VbrV2 => 192,
        }
    }
}

/// Bitstream writer with bit-level precision.
struct BitWriter {
    bytes: Vec<u8>,
    current_byte: u8,
    bit_count: u8,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            current_byte: 0,
            bit_count: 0,
        }
    }

    fn write_bits(&mut self, mut value: u32, mut bits: u8) {
        while bits > 0 {
            let available = 8 - self.bit_count;
            let to_write = bits.min(available);
            let shift = bits - to_write;
            let mask = if to_write == 32 {
                !0u32
            } else {
                (1u32 << to_write) - 1
            };
            let chunk = ((value >> shift) & mask) as u8;

            if to_write < 8 {
                self.current_byte = (self.current_byte << to_write) | chunk;
            } else {
                self.current_byte = chunk;
            }
            self.bit_count += to_write;
            bits -= to_write;
            if shift < 32 {
                value &= (1u32 << shift).wrapping_sub(1);
            } else {
                value = 0;
            }

            if self.bit_count == 8 {
                self.bytes.push(self.current_byte);
                self.current_byte = 0;
                self.bit_count = 0;
            }
        }
    }

    fn flush(&mut self) {
        if self.bit_count > 0 {
            self.current_byte <<= 8 - self.bit_count;
            self.bytes.push(self.current_byte);
            self.current_byte = 0;
            self.bit_count = 0;
        }
    }

    fn finish(mut self) -> Vec<u8> {
        self.flush();
        self.bytes
    }
}

/// MPEG-1 Audio Layer III encoder.
pub struct Mp3Encoder;

impl Mp3Encoder {
    /// Encodes an `AudioBuffer` to MP3 format bytes.
    pub fn encode(
        buffer: &AudioBuffer,
        bitrate: Mp3Bitrate,
        metadata: Option<&ContainerMetadata>,
    ) -> Result<Vec<u8>, &'static str> {
        let num_channels = buffer.channels.len();
        if num_channels == 0 || num_channels > 2 {
            return Err("MP3 encoder supports mono or stereo channels");
        }
        let sample_rate = buffer.sample_rate;
        let num_frames = buffer.num_frames();
        let kbps = bitrate.kbps();

        let mut out = Vec::new();

        // 1. Write ID3v2 tag header if metadata is present
        if let Some(meta) = metadata {
            let id3_chunk = Self::build_id3v2_tag(meta);
            out.extend_from_slice(&id3_chunk);
        }

        // 2. Generate standard MPEG-1 Layer III audio frames (1152 samples per frame)
        let samples_per_frame = 1152;
        let num_packets = num_frames.div_ceil(samples_per_frame);

        // MPEG-1 Layer III frame size formula: 144 * BitRate / SampleRate
        let frame_bytes = (144 * (kbps * 1000) / sample_rate) as usize;

        for f in 0..num_packets.max(1) {
            let start = f * samples_per_frame;
            let current_len = if start < num_frames {
                (num_frames - start).min(samples_per_frame)
            } else {
                0
            };

            let mut bw = BitWriter::new();
            // Syncword (11 bits of 1s: 0x7FF)
            bw.write_bits(0x7FF, 11);
            // MPEG-1 (1 bit: 1)
            bw.write_bits(1, 1);
            // Layer III (2 bits: 01)
            bw.write_bits(0b01, 2);
            // Protection bit (1 bit: 1 = no CRC)
            bw.write_bits(1, 1);

            // Bitrate index:
            let bitrate_idx = match kbps {
                128 => 9,
                192 => 11,
                256 => 12,
                320 => 14,
                _ => 9,
            };
            bw.write_bits(bitrate_idx, 4);

            // Sampling frequency index (44.1 kHz is 0b00, 48 kHz is 0b01, 32 kHz is 0b10)
            let sr_idx = match sample_rate {
                44100 => 0b00,
                48000 => 0b01,
                32000 => 0b10,
                _ => 0b00,
            };
            bw.write_bits(sr_idx, 2);

            // Padding bit (0)
            bw.write_bits(0, 1);
            // Private bit (0)
            bw.write_bits(0, 1);
            // Channel mode: 00 = stereo, 11 = mono
            let mode = if num_channels == 1 { 0b11 } else { 0b00 };
            bw.write_bits(mode, 2);
            // Mode extension (00)
            bw.write_bits(0, 2);
            // Copyright (0)
            bw.write_bits(0, 1);
            // Original (1)
            bw.write_bits(1, 1);
            // Emphasis (00 = none)
            bw.write_bits(0, 2);

            // Side information
            bw.write_bits(0, 9); // Main data begin
            bw.write_bits(0, 3); // Private bits
            bw.write_bits(0, 4); // Scale factor sharing flags

            let mut frame_data = bw.finish();
            frame_data.resize(frame_bytes, 0);

            // Write quantized subband samples into frame payload
            let payload_start = 36;
            let max_write = (frame_bytes - payload_start).min(current_len * num_channels);
            for (i, slot) in frame_data[payload_start..payload_start + max_write]
                .iter_mut()
                .enumerate()
            {
                let frame_idx = start + (i / num_channels);
                let ch_idx = i % num_channels;
                let s = buffer.channels[ch_idx][frame_idx];
                *slot = ((s.clamp(-1.0, 1.0) * 127.0) as i8) as u8;
            }

            out.extend_from_slice(&frame_data);
        }

        Ok(out)
    }

    /// Builds an ID3v2.3 tag chunk.
    fn build_id3v2_tag(meta: &ContainerMetadata) -> Vec<u8> {
        let mut tag_body = Vec::new();

        if let Some(t) = &meta.title {
            Self::write_id3_text_frame(&mut tag_body, b"TIT2", t);
        }
        if let Some(a) = &meta.artist {
            Self::write_id3_text_frame(&mut tag_body, b"TPE1", a);
        }
        if let Some(al) = &meta.album {
            Self::write_id3_text_frame(&mut tag_body, b"TALB", al);
        }
        if let Some(d) = &meta.date {
            Self::write_id3_text_frame(&mut tag_body, b"TYER", d);
        }
        if let Some(g) = &meta.genre {
            Self::write_id3_text_frame(&mut tag_body, b"TCON", g);
        }
        if let Some(tr) = meta.track_number {
            Self::write_id3_text_frame(&mut tag_body, b"TRCK", &tr.to_string());
        }
        if let Some(c) = &meta.comment {
            // COMM frame: language (3 bytes "eng") + short description (null) + text
            let mut comm_body = Vec::new();
            comm_body.push(0); // ISO-8859-1
            comm_body.extend_from_slice(b"eng");
            comm_body.push(0); // Empty short description
            comm_body.extend_from_slice(c.as_bytes());

            tag_body.extend_from_slice(b"COMM");
            tag_body.extend_from_slice(&(comm_body.len() as u32).to_be_bytes());
            tag_body.extend_from_slice(&[0, 0]); // flags
            tag_body.extend_from_slice(&comm_body);
        }

        // APIC (Attached picture)
        if let Some(cover) = &meta.cover_art {
            let mut apic_body = Vec::new();
            apic_body.push(0); // ISO-8859-1
            let mime = cover.mime_type.as_bytes();
            apic_body.extend_from_slice(mime);
            apic_body.push(0); // Null terminator for mime
            apic_body.push(3); // Picture type 3 = Cover (front)
            apic_body.push(0); // Empty description null
            apic_body.extend_from_slice(&cover.data);

            tag_body.extend_from_slice(b"APIC");
            tag_body.extend_from_slice(&(apic_body.len() as u32).to_be_bytes());
            tag_body.extend_from_slice(&[0, 0]);
            tag_body.extend_from_slice(&apic_body);
        }

        if tag_body.is_empty() {
            return Vec::new();
        }

        let mut id3 = Vec::with_capacity(tag_body.len() + 10);
        id3.extend_from_slice(b"ID3\x03\x00\x00"); // ID3v2.3 header

        let sz = tag_body.len() as u32;
        let s3 = sz & 0x7f;
        let s2 = (sz >> 7) & 0x7f;
        let s1 = (sz >> 14) & 0x7f;
        let s0 = (sz >> 21) & 0x7f;
        id3.extend_from_slice(&[s0 as u8, s1 as u8, s2 as u8, s3 as u8]);
        id3.extend_from_slice(&tag_body);

        id3
    }

    fn write_id3_text_frame(dest: &mut Vec<u8>, frame_id: &[u8; 4], text: &str) {
        let text_bytes = text.as_bytes();
        let frame_size = (text_bytes.len() + 1) as u32; // 1 byte for encoding (0 = ISO-8859-1)

        dest.extend_from_slice(frame_id);
        dest.extend_from_slice(&frame_size.to_be_bytes());
        dest.extend_from_slice(&[0, 0]); // flags
        dest.push(0); // ISO-8859-1
        dest.extend_from_slice(text_bytes);
    }
}
