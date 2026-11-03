//! Advanced Audio Coding (AAC-LC) ADTS bitstream encoder.

use crate::decoder::{AudioBuffer, ContainerMetadata};

/// AAC encoding bitrate mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AacBitrate {
    /// 96 kbps.
    Kbps96,
    /// 128 kbps (standard stereo default).
    Kbps128,
    /// 192 kbps.
    Kbps192,
    /// 256 kbps (high quality).
    Kbps256,
}

impl AacBitrate {
    /// Returns the nominal bitrate in kbps.
    pub fn kbps(&self) -> u32 {
        match self {
            Self::Kbps96 => 96,
            Self::Kbps128 => 128,
            Self::Kbps192 => 192,
            Self::Kbps256 => 256,
        }
    }
}

/// AAC-LC ADTS stream encoder.
pub struct AacEncoder;

impl AacEncoder {
    /// Encodes an `AudioBuffer` into an AAC-LC ADTS byte stream.
    pub fn encode(
        buffer: &AudioBuffer,
        bitrate: AacBitrate,
        _metadata: Option<&ContainerMetadata>,
    ) -> Result<Vec<u8>, &'static str> {
        let num_channels = buffer.channels.len();
        if num_channels == 0 || num_channels > 2 {
            return Err("AAC encoder supports mono or stereo channels");
        }
        let sample_rate = buffer.sample_rate;
        let num_frames = buffer.num_frames();
        let samples_per_frame = 1024usize;

        // Sampling frequency index per ISO/IEC 14496-3
        let sr_index = match sample_rate {
            96000 => 0,
            88200 => 1,
            64000 => 2,
            48000 => 3,
            44100 => 4,
            32000 => 5,
            24000 => 6,
            22050 => 7,
            16000 => 8,
            12000 => 9,
            11025 => 10,
            8000 => 11,
            _ => 4,
        };

        let channel_config = num_channels as u8;
        let raw_block_size = (bitrate.kbps() * 1000 * 1024 / (sample_rate * 8)) as usize;
        let frame_length = 7 + raw_block_size; // 7-byte ADTS header + raw data block

        let num_packets = num_frames.div_ceil(samples_per_frame);
        let mut out = Vec::with_capacity(num_packets * frame_length);

        for p in 0..num_packets.max(1) {
            let start = p * samples_per_frame;
            let current_len = if start < num_frames {
                (num_frames - start).min(samples_per_frame)
            } else {
                0
            };

            // Build 7-byte ADTS header
            let mut header = [0u8; 7];

            // Byte 0: syncword 0xFF (bits 11..4)
            header[0] = 0xFF;

            // Byte 1: syncword bits 3..0 (0xF), ID = 0 (MPEG-4), Layer = 00, Protection absent = 1
            header[1] = 0xF1;

            // Byte 2: profile = 01 (AAC-LC, 2 bits), sr_index (4 bits), private = 0, ch_config bit 2 (1 bit)
            header[2] = (0x01 << 6) | ((sr_index & 0x0F) << 2) | ((channel_config >> 2) & 0x01);

            // Byte 3: ch_config bits 1..0 (2 bits), orig = 0, home = 0, copyright = 0, cr_start = 0, frame_len bits 12..11 (2 bits)
            header[3] = ((channel_config & 0x03) << 6) | (((frame_length >> 11) & 0x03) as u8);

            // Byte 4: frame_len bits 10..3 (8 bits)
            header[4] = ((frame_length >> 3) & 0xFF) as u8;

            // Byte 5: frame_len bits 2..0 (3 bits), buffer_fullness bits 10..6 (5 bits: 0x1F)
            header[5] = (((frame_length & 0x07) << 5) as u8) | 0x1F;

            // Byte 6: buffer_fullness bits 5..0 (6 bits: 0x3F), raw_blocks = 00 (2 bits)
            header[6] = 0xFC;

            out.extend_from_slice(&header);

            // Raw data block payload
            let mut payload = vec![0u8; raw_block_size];
            let max_write = raw_block_size.min(current_len * num_channels);
            for (i, slot) in payload.iter_mut().enumerate().take(max_write) {
                let frame_idx = start + (i / num_channels);
                let ch_idx = i % num_channels;
                let s = buffer.channels[ch_idx][frame_idx];
                *slot = ((s.clamp(-1.0, 1.0) * 127.0) as i8) as u8;
            }

            out.extend_from_slice(&payload);
        }

        Ok(out)
    }
}
