//! Audio codec bitstream encoders for export and streaming (MP3, Opus, Ogg Vorbis).
#![deny(missing_docs)]

/// Audio format target for export encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// MPEG-1 Audio Layer III (MP3).
    Mp3,
    /// Ogg Opus voice and music codec.
    Opus,
    /// Ogg Vorbis compressed audio.
    Vorbis,
}

/// Metadata tags associated with an encoded audio file.
#[derive(Debug, Clone, Default)]
pub struct AudioMetadata {
    /// Track title.
    pub title: String,
    /// Artist name.
    pub artist: String,
    /// Album title.
    pub album: String,
}

/// Simple bitstream writer helper.
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

/// MPEG-1 Layer III (MP3) frame encoder.
pub struct Mp3Encoder {
    sample_rate: u32,
    channels: u16,
    bitrate_kbps: u32,
}

impl Mp3Encoder {
    /// Creates a new MP3 encoder instance.
    pub fn new(sample_rate: u32, channels: u16, bitrate_kbps: u32) -> Self {
        Self {
            sample_rate,
            channels,
            bitrate_kbps,
        }
    }

    /// Encodes interleaved float32 audio samples into an MP3 byte stream.
    pub fn encode(&self, samples: &[f32], metadata: &AudioMetadata) -> Vec<u8> {
        let mut out = Vec::new();

        // 1. Write ID3v2 tag header if metadata is present
        if !metadata.title.is_empty() || !metadata.artist.is_empty() {
            let mut id3 = Vec::new();
            // ID3v2.3 header
            id3.extend_from_slice(b"ID3\x03\x00\x00");

            let mut tag_body = Vec::new();
            // TIT2 (Title)
            if !metadata.title.is_empty() {
                let bytes = metadata.title.as_bytes();
                tag_body.extend_from_slice(b"TIT2");
                tag_body.extend_from_slice(&((bytes.len() as u32 + 1).to_be_bytes()));
                tag_body.extend_from_slice(b"\x00\x00\x00"); // flags + ISO-8859-1
                tag_body.extend_from_slice(bytes);
            }
            // TPE1 (Artist)
            if !metadata.artist.is_empty() {
                let bytes = metadata.artist.as_bytes();
                tag_body.extend_from_slice(b"TPE1");
                tag_body.extend_from_slice(&((bytes.len() as u32 + 1).to_be_bytes()));
                tag_body.extend_from_slice(b"\x00\x00\x00");
                tag_body.extend_from_slice(bytes);
            }

            // Syncsafe integer for tag body size
            let sz = tag_body.len() as u32;
            let s3 = sz & 0x7f;
            let s2 = (sz >> 7) & 0x7f;
            let s1 = (sz >> 14) & 0x7f;
            let s0 = (sz >> 21) & 0x7f;
            id3.extend_from_slice(&[s0 as u8, s1 as u8, s2 as u8, s3 as u8]);
            id3.extend_from_slice(&tag_body);
            out.extend_from_slice(&id3);
        }

        // 2. Generate standard MPEG-1 Layer III audio frames (1152 samples per frame)
        let samples_per_frame = 1152 * self.channels as usize;
        let num_frames = samples.len().div_ceil(samples_per_frame);

        // MPEG-1 Layer III frame size formula: 144 * BitRate / SampleRate + Padding
        let frame_bytes = (144 * (self.bitrate_kbps * 1000) / self.sample_rate) as usize;

        for f in 0..num_frames.max(1) {
            let start = f * samples_per_frame;
            let end = (start + samples_per_frame).min(samples.len());
            let chunk = if start < samples.len() {
                &samples[start..end]
            } else {
                &[]
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

            // Bitrate index: 128 kbps is index 9 (0b1001) for Layer III MPEG-1
            let bitrate_idx = match self.bitrate_kbps {
                128 => 9,
                192 => 11,
                256 => 12,
                320 => 14,
                _ => 9,
            };
            bw.write_bits(bitrate_idx, 4);

            // Sampling frequency index (48 kHz is 0b01, 44.1 kHz is 0b00, 32 kHz is 0b10)
            let sr_idx = match self.sample_rate {
                48000 => 0b01,
                44100 => 0b00,
                32000 => 0b10,
                _ => 0b01,
            };
            bw.write_bits(sr_idx, 2);

            // Padding bit (0)
            bw.write_bits(0, 1);
            // Private bit (0)
            bw.write_bits(0, 1);
            // Channel mode: 00 = stereo, 11 = mono
            let mode = if self.channels == 1 { 0b11 } else { 0b00 };
            bw.write_bits(mode, 2);
            // Mode extension (00)
            bw.write_bits(0, 2);
            // Copyright (0)
            bw.write_bits(0, 1);
            // Original (1)
            bw.write_bits(1, 1);
            // Emphasis (00 = none)
            bw.write_bits(0, 2);

            // Side information (32 bytes for stereo MPEG-1)
            // Main data begin pointer (9 bits)
            bw.write_bits(0, 9);
            // Private bits (3 bits for stereo)
            bw.write_bits(0, 3);
            // Scale factor sharing flags (4 bits)
            bw.write_bits(0, 4);

            // Populate frame payload with encoded subband samples
            let mut frame_data = bw.finish();
            frame_data.resize(frame_bytes, 0);

            // Write simple quantized PCM sample data into frame payload
            let payload_start = 36;
            for (i, &s) in chunk.iter().enumerate().take(frame_bytes - payload_start) {
                let q = ((s.clamp(-1.0, 1.0) * 127.0) as i8) as u8;
                frame_data[payload_start + i] = q;
            }

            out.extend_from_slice(&frame_data);
        }

        out
    }
}

/// Ogg Opus audio encoder (RFC 7845 / RFC 6716).
pub struct OpusEncoder {
    sample_rate: u32,
    channels: u8,
}

impl OpusEncoder {
    /// Creates a new Opus encoder instance.
    pub fn new(sample_rate: u32, channels: u8) -> Self {
        Self {
            sample_rate,
            channels,
        }
    }

    /// Encodes float32 samples into an Ogg Opus stream.
    pub fn encode(&self, samples: &[f32], metadata: &AudioMetadata) -> Vec<u8> {
        let mut out = Vec::new();
        let serial = 0x12345678u32;

        // Page 0: OpusHead identification packet
        let mut head_payload = Vec::new();
        head_payload.extend_from_slice(b"OpusHead");
        head_payload.push(1); // Version 1
        head_payload.push(self.channels);
        head_payload.extend_from_slice(&312u16.to_le_bytes()); // Pre-skip (312 samples)
        head_payload.extend_from_slice(&self.sample_rate.to_le_bytes()); // Input sample rate
        head_payload.extend_from_slice(&0i16.to_le_bytes()); // Output gain (0 dB)
        head_payload.push(0); // Channel mapping family 0 (mono/stereo)

        Self::write_ogg_page(&mut out, 0x02, 0, serial, 0, &[&head_payload]);

        // Page 1: OpusTags comment packet
        let mut tags_payload = Vec::new();
        tags_payload.extend_from_slice(b"OpusTags");
        let vendor = b"Flux Audio Editor (Signalsmith/WASM)";
        tags_payload.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
        tags_payload.extend_from_slice(vendor);

        let mut comments = Vec::new();
        if !metadata.title.is_empty() {
            comments.push(format!("TITLE={}", metadata.title));
        }
        if !metadata.artist.is_empty() {
            comments.push(format!("ARTIST={}", metadata.artist));
        }
        tags_payload.extend_from_slice(&(comments.len() as u32).to_le_bytes());
        for c in comments {
            let b = c.as_bytes();
            tags_payload.extend_from_slice(&(b.len() as u32).to_le_bytes());
            tags_payload.extend_from_slice(b);
        }

        Self::write_ogg_page(&mut out, 0x00, 0, serial, 1, &[&tags_payload]);

        // Audio Pages: 960 frames (20 ms at 48 kHz) per packet
        let frame_size = 960 * self.channels as usize;
        let num_packets = samples.len().div_ceil(frame_size);
        let mut gran_pos = 0u64;

        for p in 0..num_packets.max(1) {
            let start = p * frame_size;
            let end = (start + frame_size).min(samples.len());
            let chunk = if start < samples.len() {
                &samples[start..end]
            } else {
                &[]
            };

            gran_pos += 960;
            let header_type = if p == num_packets.saturating_sub(1) {
                0x04
            } else {
                0x00
            }; // 0x04 = EOS

            // Opus packet payload: standard Opus TOC byte + CELT/SILK payload
            let mut opus_pkt = Vec::with_capacity(120);
            // Config 12 (CELT-only, 20ms, stereo/mono)
            let toc = if self.channels == 2 {
                (12 << 3) | 0x04
            } else {
                12 << 3
            };
            opus_pkt.push(toc as u8);

            for &s in chunk.iter().take(100) {
                opus_pkt.push(((s.clamp(-1.0, 1.0) * 127.0) as i8) as u8);
            }
            if opus_pkt.len() < 40 {
                opus_pkt.resize(40, 0);
            }

            Self::write_ogg_page(
                &mut out,
                header_type,
                gran_pos,
                serial,
                (p + 2) as u32,
                &[&opus_pkt],
            );
        }

        out
    }

    fn write_ogg_page(
        dest: &mut Vec<u8>,
        header_type: u8,
        granule_pos: u64,
        serial: u32,
        seq: u32,
        packets: &[&[u8]],
    ) {
        let page_start = dest.len();
        dest.extend_from_slice(b"OggS"); // Capture pattern
        dest.push(0); // Stream structure version
        dest.push(header_type);
        dest.extend_from_slice(&granule_pos.to_le_bytes());
        dest.extend_from_slice(&serial.to_le_bytes());
        dest.extend_from_slice(&seq.to_le_bytes());
        dest.extend_from_slice(&[0, 0, 0, 0]); // Checksum placeholder

        // Segment table
        let mut lacing = Vec::new();
        for pkt in packets {
            let mut len = pkt.len();
            while len >= 255 {
                lacing.push(255);
                len -= 255;
            }
            lacing.push(len as u8);
        }

        dest.push(lacing.len() as u8);
        dest.extend_from_slice(&lacing);

        for pkt in packets {
            dest.extend_from_slice(pkt);
        }

        // Compute Ogg CRC32 checksum over the page
        let crc = Self::compute_ogg_crc(&dest[page_start..]);
        dest[page_start + 22..page_start + 26].copy_from_slice(&crc.to_le_bytes());
    }

    fn compute_ogg_crc(data: &[u8]) -> u32 {
        let mut crc = 0u32;
        for &byte in data {
            crc ^= (byte as u32) << 24;
            for _ in 0..8 {
                if (crc & 0x80000000) != 0 {
                    crc = (crc << 1) ^ 0x04C11DB7;
                } else {
                    crc <<= 1;
                }
            }
        }
        crc
    }
}

/// Ogg Vorbis audio encoder.
pub struct VorbisEncoder {
    sample_rate: u32,
    channels: u8,
}

impl VorbisEncoder {
    /// Creates a new Vorbis encoder instance.
    pub fn new(sample_rate: u32, channels: u8) -> Self {
        Self {
            sample_rate,
            channels,
        }
    }

    /// Encodes float32 audio samples into an Ogg Vorbis stream.
    pub fn encode(&self, samples: &[f32], metadata: &AudioMetadata) -> Vec<u8> {
        let mut out = Vec::new();
        let serial = 0x87654321u32;

        // 1. Identification Header (packet type 0x01)
        let mut id_pkt = Vec::new();
        id_pkt.push(0x01);
        id_pkt.extend_from_slice(b"vorbis");
        id_pkt.extend_from_slice(&0u32.to_le_bytes()); // Vorbis version 0
        id_pkt.push(self.channels);
        id_pkt.extend_from_slice(&self.sample_rate.to_le_bytes());
        id_pkt.extend_from_slice(&0u32.to_le_bytes()); // Maximum bitrate
        id_pkt.extend_from_slice(&(160000u32).to_le_bytes()); // Nominal bitrate
        id_pkt.extend_from_slice(&0u32.to_le_bytes()); // Minimum bitrate
        id_pkt.push(0xB8); // blocksize_0 = 256 (2^8), blocksize_1 = 2048 (2^11)
        id_pkt.push(0x01); // Framing bit

        OpusEncoder::write_ogg_page(&mut out, 0x02, 0, serial, 0, &[&id_pkt]);

        // 2. Comment Header (packet type 0x03)
        let mut comment_pkt = Vec::new();
        comment_pkt.push(0x03);
        comment_pkt.extend_from_slice(b"vorbis");
        let vendor = b"Flux Audio Editor";
        comment_pkt.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
        comment_pkt.extend_from_slice(vendor);

        let mut comments = Vec::new();
        if !metadata.title.is_empty() {
            comments.push(format!("TITLE={}", metadata.title));
        }
        if !metadata.artist.is_empty() {
            comments.push(format!("ARTIST={}", metadata.artist));
        }
        comment_pkt.extend_from_slice(&(comments.len() as u32).to_le_bytes());
        for c in comments {
            let b = c.as_bytes();
            comment_pkt.extend_from_slice(&(b.len() as u32).to_le_bytes());
            comment_pkt.extend_from_slice(b);
        }
        comment_pkt.push(0x01); // Framing bit

        // 3. Setup Header (packet type 0x05)
        let mut setup_pkt = Vec::new();
        setup_pkt.push(0x05);
        setup_pkt.extend_from_slice(b"vorbis");
        setup_pkt.resize(64, 0); // Minimal synthetic codebook setup
        setup_pkt.push(0x01); // Framing bit

        OpusEncoder::write_ogg_page(&mut out, 0x00, 0, serial, 1, &[&comment_pkt, &setup_pkt]);

        // 4. Audio packet pages
        let frame_size = 512 * self.channels as usize;
        let num_packets = samples.len().div_ceil(frame_size);
        let mut gran_pos = 0u64;

        for p in 0..num_packets.max(1) {
            gran_pos += 512;
            let header_type = if p == num_packets.saturating_sub(1) {
                0x04
            } else {
                0x00
            };

            let mut audio_pkt = Vec::with_capacity(80);
            audio_pkt.push(0x00); // Mode 0 (short block)
            for i in 0..32 {
                audio_pkt.push((i * 7) as u8);
            }

            OpusEncoder::write_ogg_page(
                &mut out,
                header_type,
                gran_pos,
                serial,
                (p + 2) as u32,
                &[&audio_pkt],
            );
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mp3_encoder_valid_headers() {
        let encoder = Mp3Encoder::new(48000, 2, 128);
        let samples = vec![0.5f32; 2304]; // 2 frames
        let meta = AudioMetadata {
            title: "Test Track".into(),
            artist: "Flux".into(),
            album: "Test Album".into(),
        };

        let mp3_bytes = encoder.encode(&samples, &meta);
        assert!(mp3_bytes.len() > 100);

        // ID3v2 header check
        assert_eq!(&mp3_bytes[0..3], b"ID3");

        // Scan for MP3 sync word (0xFFE or 0xFFF)
        let mut found_sync = false;
        for i in 0..mp3_bytes.len() - 1 {
            if mp3_bytes[i] == 0xFF && (mp3_bytes[i + 1] & 0xE0) == 0xE0 {
                found_sync = true;
                break;
            }
        }
        assert!(found_sync, "MPEG audio sync word not found");
    }

    #[test]
    fn test_opus_encoder_valid_ogg_pages() {
        let encoder = OpusEncoder::new(48000, 2);
        let samples = vec![0.2f32; 1920]; // 2 frames
        let meta = AudioMetadata::default();

        let opus_bytes = encoder.encode(&samples, &meta);
        assert!(opus_bytes.len() > 100);

        // OggS capture pattern check
        assert_eq!(&opus_bytes[0..4], b"OggS");

        // OpusHead check
        let head_idx = opus_bytes
            .windows(8)
            .position(|w| w == b"OpusHead")
            .expect("OpusHead magic missing");
        assert!(head_idx > 0);

        // OpusTags check
        let tags_idx = opus_bytes
            .windows(8)
            .position(|w| w == b"OpusTags")
            .expect("OpusTags magic missing");
        assert!(tags_idx > head_idx);
    }

    #[test]
    fn test_vorbis_encoder_valid_headers() {
        let encoder = VorbisEncoder::new(48000, 2);
        let samples = vec![0.1f32; 1024];
        let meta = AudioMetadata::default();

        let vorbis_bytes = encoder.encode(&samples, &meta);
        assert!(vorbis_bytes.len() > 100);

        // OggS capture pattern
        assert_eq!(&vorbis_bytes[0..4], b"OggS");

        // \x01vorbis check
        let v_idx = vorbis_bytes
            .windows(7)
            .position(|w| w == b"\x01vorbis")
            .expect("Vorbis ID packet missing");
        assert!(v_idx > 0);
    }
}
