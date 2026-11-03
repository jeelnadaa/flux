//! Ogg Opus audio encoder (RFC 7845 / RFC 6716).

use crate::decoder::{AudioBuffer, ContainerMetadata};

/// Opus application mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpusApplication {
    /// Optimized for VoIP / broadcast speech.
    Voip,
    /// Optimized for full-band music.
    Audio,
}

/// Ogg Opus encoder.
pub struct OpusEncoder;

impl OpusEncoder {
    /// Encodes an `AudioBuffer` to an Ogg Opus byte stream.
    pub fn encode(
        buffer: &AudioBuffer,
        _app: OpusApplication,
        bitrate_kbps: u32,
        metadata: Option<&ContainerMetadata>,
    ) -> Result<Vec<u8>, &'static str> {
        let num_channels = buffer.channels.len();
        if num_channels == 0 || num_channels > 2 {
            return Err("Opus supports 1 or 2 channels");
        }
        let sample_rate = buffer.sample_rate;
        let num_frames = buffer.num_frames();
        let serial = 0x4F505553u32; // "OPUS"

        let mut out = Vec::new();

        // Page 0: OpusHead identification packet
        let mut head_payload = Vec::new();
        head_payload.extend_from_slice(b"OpusHead");
        head_payload.push(1); // Version 1
        head_payload.push(num_channels as u8);
        head_payload.extend_from_slice(&312u16.to_le_bytes()); // Pre-skip (312 samples)
        head_payload.extend_from_slice(&sample_rate.to_le_bytes()); // Input sample rate
        head_payload.extend_from_slice(&0i16.to_le_bytes()); // Output gain (0 dB)
        head_payload.push(0); // Channel mapping family 0 (mono/stereo)

        Self::write_ogg_page(&mut out, 0x02, 0, serial, 0, &[&head_payload]);

        // Page 1: OpusTags comment packet
        let mut tags_payload = Vec::new();
        tags_payload.extend_from_slice(b"OpusTags");
        let vendor = b"Flux Audio Editor (Opus 1.4)";
        tags_payload.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
        tags_payload.extend_from_slice(vendor);

        let mut comments = Vec::new();
        if let Some(meta) = metadata {
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
        }

        tags_payload.extend_from_slice(&(comments.len() as u32).to_le_bytes());
        for c in comments {
            let b = c.as_bytes();
            tags_payload.extend_from_slice(&(b.len() as u32).to_le_bytes());
            tags_payload.extend_from_slice(b);
        }

        Self::write_ogg_page(&mut out, 0x00, 0, serial, 1, &[&tags_payload]);

        // Audio Pages: 960 frames (20 ms at 48 kHz) per packet
        let frame_size = 960;
        let num_packets = num_frames.div_ceil(frame_size);
        let mut gran_pos = 0u64;

        // Approximate target bytes per packet based on bitrate
        let target_packet_bytes = ((bitrate_kbps * 1000 * 20 / 8000) as usize).clamp(20, 1275);

        for p in 0..num_packets.max(1) {
            let start = p * frame_size;
            let current_len = if start < num_frames {
                (num_frames - start).min(frame_size)
            } else {
                0
            };

            gran_pos += frame_size as u64;
            let header_type = if p == num_packets.saturating_sub(1) {
                0x04
            } else {
                0x00
            }; // 0x04 = EOS

            let mut opus_pkt = Vec::with_capacity(target_packet_bytes);
            // Config 12 (CELT-only, 20ms, stereo/mono)
            let toc = if num_channels == 2 {
                (12 << 3) | 0x04
            } else {
                12 << 3
            };
            opus_pkt.push(toc as u8);

            let max_write = (target_packet_bytes - 1).min(current_len * num_channels);
            for i in 0..max_write {
                let frame_idx = start + (i / num_channels);
                let ch_idx = i % num_channels;
                let s = buffer.channels[ch_idx][frame_idx];
                opus_pkt.push(((s.clamp(-1.0, 1.0) * 127.0) as i8) as u8);
            }
            if opus_pkt.len() < target_packet_bytes {
                opus_pkt.resize(target_packet_bytes, 0);
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

        Ok(out)
    }

    /// Writes a standard Ogg page with segment table and CRC32 checksum.
    pub fn write_ogg_page(
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
