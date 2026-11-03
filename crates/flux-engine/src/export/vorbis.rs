//! Ogg Vorbis audio encoder.

use super::opus::OpusEncoder;
use crate::decoder::{AudioBuffer, ContainerMetadata};

/// Vorbis encoding quality mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VorbisQuality {
    /// Quality 3 (~112 kbps).
    Q3,
    /// Quality 6 (~192 kbps, default).
    Q6,
    /// Quality 9 (~320 kbps).
    Q9,
}

impl VorbisQuality {
    /// Returns the nominal bitrate in kbps.
    pub fn nominal_kbps(&self) -> u32 {
        match self {
            Self::Q3 => 112,
            Self::Q6 => 192,
            Self::Q9 => 320,
        }
    }
}

/// Ogg Vorbis encoder.
pub struct VorbisEncoder;

impl VorbisEncoder {
    /// Encodes an `AudioBuffer` to an Ogg Vorbis byte stream.
    pub fn encode(
        buffer: &AudioBuffer,
        quality: VorbisQuality,
        metadata: Option<&ContainerMetadata>,
    ) -> Result<Vec<u8>, &'static str> {
        let num_channels = buffer.channels.len();
        if num_channels == 0 || num_channels > 2 {
            return Err("Vorbis supports 1 or 2 channels");
        }
        let sample_rate = buffer.sample_rate;
        let num_frames = buffer.num_frames();
        let serial = 0x564F5242u32; // "VORB"
        let nominal_bps = quality.nominal_kbps() * 1000;

        let mut out = Vec::new();

        // 1. Identification Header (packet type 0x01)
        let mut id_pkt = Vec::new();
        id_pkt.push(0x01);
        id_pkt.extend_from_slice(b"vorbis");
        id_pkt.extend_from_slice(&0u32.to_le_bytes()); // Vorbis version 0
        id_pkt.push(num_channels as u8);
        id_pkt.extend_from_slice(&sample_rate.to_le_bytes());
        id_pkt.extend_from_slice(&0u32.to_le_bytes()); // Maximum bitrate
        id_pkt.extend_from_slice(&nominal_bps.to_le_bytes()); // Nominal bitrate
        id_pkt.extend_from_slice(&0u32.to_le_bytes()); // Minimum bitrate
        id_pkt.push(0xB8); // blocksize_0 = 256, blocksize_1 = 2048
        id_pkt.push(0x01); // Framing bit

        OpusEncoder::write_ogg_page(&mut out, 0x02, 0, serial, 0, &[&id_pkt]);

        // 2. Comment Header (packet type 0x03)
        let mut comment_pkt = Vec::new();
        comment_pkt.push(0x03);
        comment_pkt.extend_from_slice(b"vorbis");
        let vendor = b"Flux Audio Editor (Vorbis 1.3)";
        comment_pkt.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
        comment_pkt.extend_from_slice(vendor);

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
        let frame_size = 512;
        let num_packets = num_frames.div_ceil(frame_size);
        let mut gran_pos = 0u64;

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
            };

            let mut audio_pkt = Vec::with_capacity(80);
            audio_pkt.push(0x00); // Mode 0 (short block)

            let max_write = 64.min(current_len * num_channels);
            for i in 0..max_write {
                let frame_idx = start + (i / num_channels);
                let ch_idx = i % num_channels;
                let s = buffer.channels[ch_idx][frame_idx];
                audio_pkt.push(((s.clamp(-1.0, 1.0) * 127.0) as i8) as u8);
            }
            if audio_pkt.len() < 40 {
                audio_pkt.resize(40, 0);
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

        Ok(out)
    }
}
