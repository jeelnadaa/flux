//! RIFF/WAVE and RF64 audio encoder.
//!
//! Supports 16-bit PCM, 24-bit PCM, 32-bit float, multi-channel layouts,
//! automatic/forced RF64 64-bit chunk formatting for > 4 GB files, and
//! RIFF `LIST INFO` metadata tagging.

use crate::decoder::{AudioBuffer, ContainerMetadata};

/// Supported WAV bit depth / sample format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavBitsPerSample {
    /// 16-bit linear signed integer PCM.
    Bits16,
    /// 24-bit linear signed integer PCM.
    Bits24,
    /// 32-bit IEEE 754 floating point.
    Float32,
}

/// RIFF/WAVE and RF64 encoder.
pub struct WavEncoder;

impl WavEncoder {
    /// Encodes an `AudioBuffer` to WAV or RF64 bytes.
    pub fn encode(
        buffer: &AudioBuffer,
        format: WavBitsPerSample,
        metadata: Option<&ContainerMetadata>,
        force_rf64: bool,
    ) -> Result<Vec<u8>, &'static str> {
        let num_channels = buffer.channels.len();
        if num_channels == 0 {
            return Err("Cannot encode WAV with 0 channels");
        }
        let num_frames = buffer.num_frames();
        let sample_rate = buffer.sample_rate;

        let (audio_format_code, bits_per_sample, bytes_per_sample) = match format {
            WavBitsPerSample::Bits16 => (1u16, 16u16, 2usize),
            WavBitsPerSample::Bits24 => (1u16, 24u16, 3usize),
            WavBitsPerSample::Float32 => (3u16, 32u16, 4usize),
        };

        let block_align = (num_channels * bytes_per_sample) as u16;
        let byte_rate = sample_rate * (block_align as u32);
        let audio_data_len = (num_frames * num_channels * bytes_per_sample) as u64;

        // Build LIST INFO metadata chunk if metadata exists
        let info_chunk = metadata
            .map(Self::build_list_info_chunk)
            .unwrap_or_default();
        let info_chunk_len = info_chunk.len() as u64;

        // Standard RIFF limit is 4 GB (0xFFFFFFFF bytes)
        let is_rf64 = force_rf64 || (audio_data_len + info_chunk_len + 100 > 0xFFFF_FF00);

        let mut out = Vec::new();

        if is_rf64 {
            // --- RF64 Format ---
            // 1. "RF64" magic
            out.extend_from_slice(b"RF64");
            out.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes()); // -1 placeholder
            out.extend_from_slice(b"WAVE");

            // 2. "ds64" chunk (28 bytes payload)
            out.extend_from_slice(b"ds64");
            out.extend_from_slice(&28u32.to_le_bytes()); // chunk size 28

            // Total RF64 size minus 8
            let ds64_chunk_total = 8 + 28;
            let fmt_chunk_total = 8 + 16;
            let data_header_total = 8;
            let riff_size = 4
                + ds64_chunk_total
                + fmt_chunk_total
                + info_chunk_len
                + data_header_total
                + audio_data_len;

            out.extend_from_slice(&riff_size.to_le_bytes()); // riffSize (u64)
            out.extend_from_slice(&audio_data_len.to_le_bytes()); // dataSize (u64)
            out.extend_from_slice(&(num_frames as u64).to_le_bytes()); // sampleCount (u64)
            out.extend_from_slice(&0u32.to_le_bytes()); // tableLength (u32 = 0)
        } else {
            // --- Standard RIFF Format ---
            let riff_len = (36 + info_chunk_len + audio_data_len) as u32;
            out.extend_from_slice(b"RIFF");
            out.extend_from_slice(&riff_len.to_le_bytes());
            out.extend_from_slice(b"WAVE");
        }

        // "fmt " chunk
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16u32.to_le_bytes()); // Chunk size: 16
        out.extend_from_slice(&audio_format_code.to_le_bytes());
        out.extend_from_slice(&(num_channels as u16).to_le_bytes());
        out.extend_from_slice(&sample_rate.to_le_bytes());
        out.extend_from_slice(&byte_rate.to_le_bytes());
        out.extend_from_slice(&block_align.to_le_bytes());
        out.extend_from_slice(&bits_per_sample.to_le_bytes());

        // Append LIST INFO chunk if present
        if !info_chunk.is_empty() {
            out.extend_from_slice(&info_chunk);
        }

        // "data" chunk header
        out.extend_from_slice(b"data");
        if is_rf64 {
            out.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        } else {
            out.extend_from_slice(&(audio_data_len as u32).to_le_bytes());
        }

        // Interleaved audio samples
        match format {
            WavBitsPerSample::Bits16 => {
                for f in 0..num_frames {
                    for ch in 0..num_channels {
                        let s = buffer.channels[ch][f].clamp(-1.0, 1.0);
                        let sample_i16 = (s * 32767.0).round() as i16;
                        out.extend_from_slice(&sample_i16.to_le_bytes());
                    }
                }
            }
            WavBitsPerSample::Bits24 => {
                for f in 0..num_frames {
                    for ch in 0..num_channels {
                        let s = buffer.channels[ch][f].clamp(-1.0, 1.0);
                        let sample_i32 = (s * 8388607.0).round() as i32;
                        let b0 = sample_i32 as u8;
                        let b1 = (sample_i32 >> 8) as u8;
                        let b2 = (sample_i32 >> 16) as u8;
                        out.extend_from_slice(&[b0, b1, b2]);
                    }
                }
            }
            WavBitsPerSample::Float32 => {
                for f in 0..num_frames {
                    for ch in 0..num_channels {
                        let s = buffer.channels[ch][f];
                        out.extend_from_slice(&s.to_le_bytes());
                    }
                }
            }
        }

        // Pad to word boundary if odd length
        if !out.len().is_multiple_of(2) {
            out.push(0);
        }

        Ok(out)
    }

    /// Builds a RIFF `LIST INFO` chunk containing metadata strings.
    fn build_list_info_chunk(meta: &ContainerMetadata) -> Vec<u8> {
        let mut subchunks = Vec::new();

        if let Some(t) = &meta.title {
            Self::write_info_subchunk(&mut subchunks, b"INAM", t);
        }
        if let Some(a) = &meta.artist {
            Self::write_info_subchunk(&mut subchunks, b"IART", a);
        }
        if let Some(al) = &meta.album {
            Self::write_info_subchunk(&mut subchunks, b"IPRD", al);
        }
        if let Some(date) = &meta.date {
            Self::write_info_subchunk(&mut subchunks, b"ICRD", date);
        }
        if let Some(g) = &meta.genre {
            Self::write_info_subchunk(&mut subchunks, b"IGNR", g);
        }

        if subchunks.is_empty() {
            return Vec::new();
        }

        let mut list_chunk = Vec::with_capacity(subchunks.len() + 12);
        list_chunk.extend_from_slice(b"LIST");
        let list_len = (subchunks.len() + 4) as u32;
        list_chunk.extend_from_slice(&list_len.to_le_bytes());
        list_chunk.extend_from_slice(b"INFO");
        list_chunk.extend_from_slice(&subchunks);

        list_chunk
    }

    fn write_info_subchunk(dest: &mut Vec<u8>, fourcc: &[u8; 4], text: &str) {
        dest.extend_from_slice(fourcc);
        let bytes = text.as_bytes();
        let len_with_null = (bytes.len() + 1) as u32;
        dest.extend_from_slice(&len_with_null.to_le_bytes());
        dest.extend_from_slice(bytes);
        dest.push(0); // Null terminator
        if !len_with_null.is_multiple_of(2) {
            dest.push(0); // Word alignment pad
        }
    }
}
