//! Minimal, deterministic RIFF/WAVE file reader and writer.
//!
//! Supports 16-bit signed integer PCM and 32-bit IEEE floating-point formats.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// Supported WAV audio sample formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavFormat {
    /// 16-bit linear signed integer PCM.
    Pcm16,
    /// 32-bit IEEE 754 floating point.
    Float32,
}

/// Parsed in-memory representation of a WAV audio file.
#[derive(Debug, Clone)]
pub struct WavData {
    /// Audio sample format.
    pub format: WavFormat,
    /// Number of interleaved audio channels.
    pub channels: usize,
    /// Sampling rate in Hz.
    pub sample_rate: u32,
    /// Multi-channel de-interleaved audio samples (each channel in -1.0..=1.0 or raw float).
    pub channel_samples: Vec<Vec<f32>>,
    /// Total duration in seconds.
    pub duration_secs: f32,
}

/// Writes multi-channel audio samples to WAV format bytes.
pub fn encode_wav(
    channels: &[&[f32]],
    sample_rate: u32,
    format: WavFormat,
) -> Result<Vec<u8>, String> {
    if channels.is_empty() {
        return Err("Cannot encode WAV with 0 channels".to_string());
    }

    let num_channels = channels.len();
    let num_frames = channels[0].len();
    for (idx, ch) in channels.iter().enumerate() {
        if ch.len() != num_frames {
            return Err(format!(
                "Channel {} length {} does not match channel 0 length {}",
                idx,
                ch.len(),
                num_frames
            ));
        }
    }

    let (audio_format_code, bits_per_sample) = match format {
        WavFormat::Pcm16 => (1u16, 16u16),
        WavFormat::Float32 => (3u16, 32u16),
    };

    let bytes_per_sample = (bits_per_sample / 8) as usize;
    let block_align = (num_channels * bytes_per_sample) as u16;
    let byte_rate = sample_rate * (block_align as u32);
    let data_len = (num_frames * num_channels * bytes_per_sample) as u32;
    let riff_len = 36 + data_len;

    let mut buf = Vec::with_capacity((riff_len + 8) as usize);

    // RIFF Header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&riff_len.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    // "fmt " chunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // subchunk1 size: 16
    buf.extend_from_slice(&audio_format_code.to_le_bytes());
    buf.extend_from_slice(&(num_channels as u16).to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&bits_per_sample.to_le_bytes());

    // "data" chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_len.to_le_bytes());

    // Interleave samples
    match format {
        WavFormat::Pcm16 => {
            for f in 0..num_frames {
                for ch in channels.iter().take(num_channels) {
                    let s = ch[f].clamp(-1.0, 1.0);
                    let sample_i16 = (s * 32767.0).round() as i16;
                    buf.extend_from_slice(&sample_i16.to_le_bytes());
                }
            }
        }
        WavFormat::Float32 => {
            for f in 0..num_frames {
                for ch in channels.iter().take(num_channels) {
                    let s = ch[f];
                    buf.extend_from_slice(&s.to_le_bytes());
                }
            }
        }
    }

    Ok(buf)
}

/// Decodes WAV format bytes into structured audio data.
pub fn decode_wav(bytes: &[u8]) -> Result<WavData, String> {
    if bytes.len() < 44 {
        return Err("WAV data too short for header (< 44 bytes)".to_string());
    }

    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("Invalid RIFF/WAVE header".to_string());
    }

    let mut offset = 12;
    let mut fmt_info: Option<(WavFormat, usize, u32, usize)> = None;
    let mut data_slice: Option<&[u8]> = None;

    while offset + 8 <= bytes.len() {
        let chunk_id = &bytes[offset..offset + 4];
        let chunk_len = u32::from_le_bytes([
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]) as usize;
        offset += 8;

        if chunk_id == b"fmt " {
            if chunk_len < 16 || offset + 16 > bytes.len() {
                return Err("Malformed fmt chunk".to_string());
            }
            let format_code = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            let channels = u16::from_le_bytes([bytes[offset + 2], bytes[offset + 3]]) as usize;
            let sample_rate = u32::from_le_bytes([
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]);
            let bits_per_sample = u16::from_le_bytes([bytes[offset + 14], bytes[offset + 15]]);

            let wav_format = match (format_code, bits_per_sample) {
                (1, 16) => WavFormat::Pcm16,
                (3, 32) => WavFormat::Float32,
                _ => {
                    return Err(format!(
                        "Unsupported WAV format: code={}, bits={}",
                        format_code, bits_per_sample
                    ))
                }
            };
            fmt_info = Some((wav_format, channels, sample_rate, bits_per_sample as usize));
        } else if chunk_id == b"data" {
            let actual_len = chunk_len.min(bytes.len() - offset);
            data_slice = Some(&bytes[offset..offset + actual_len]);
        }

        offset += chunk_len;
        // Pad byte for odd chunk lengths per RIFF standard
        if !chunk_len.is_multiple_of(2) {
            offset += 1;
        }
    }

    let (format, channels, sample_rate, bits_per_sample) =
        fmt_info.ok_or_else(|| "Missing 'fmt ' chunk in WAV".to_string())?;
    let data = data_slice.ok_or_else(|| "Missing 'data' chunk in WAV".to_string())?;

    let bytes_per_sample = bits_per_sample / 8;
    let block_align = channels * bytes_per_sample;
    if block_align == 0 {
        return Err("Invalid zero block align".to_string());
    }
    let num_frames = data.len() / block_align;

    let mut channel_samples: Vec<Vec<f32>> = vec![Vec::with_capacity(num_frames); channels];

    match format {
        WavFormat::Pcm16 => {
            let mut read_idx = 0;
            for _ in 0..num_frames {
                for ch_samples in channel_samples.iter_mut().take(channels) {
                    let s_i16 = i16::from_le_bytes([data[read_idx], data[read_idx + 1]]);
                    let s_f32 = (s_i16 as f32) / 32767.0;
                    ch_samples.push(s_f32);
                    read_idx += 2;
                }
            }
        }
        WavFormat::Float32 => {
            let mut read_idx = 0;
            for _ in 0..num_frames {
                for ch_samples in channel_samples.iter_mut().take(channels) {
                    let s_f32 = f32::from_le_bytes([
                        data[read_idx],
                        data[read_idx + 1],
                        data[read_idx + 2],
                        data[read_idx + 3],
                    ]);
                    ch_samples.push(s_f32);
                    read_idx += 4;
                }
            }
        }
    }

    let duration_secs = (num_frames as f32) / (sample_rate as f32);

    Ok(WavData {
        format,
        channels,
        sample_rate,
        channel_samples,
        duration_secs,
    })
}

/// Reads and decodes a WAV audio file from disk.
pub fn read_wav_file<P: AsRef<Path>>(path: P) -> Result<WavData, String> {
    let mut file = File::open(path).map_err(|e| format!("Failed to open WAV file: {}", e))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|e| format!("Failed to read WAV file: {}", e))?;
    decode_wav(&bytes)
}

/// Encodes and writes audio samples to a WAV file on disk.
pub fn write_wav_file<P: AsRef<Path>>(
    path: P,
    channels: &[&[f32]],
    sample_rate: u32,
    format: WavFormat,
) -> Result<(), String> {
    let bytes = encode_wav(channels, sample_rate, format)?;
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
    }
    let mut file = File::create(path).map_err(|e| format!("Failed to create WAV file: {}", e))?;
    file.write_all(&bytes)
        .map_err(|e| format!("Failed to write WAV file bytes: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wav_roundtrip_pcm16() {
        let sig1 = [0.0, 0.5, -0.5, 0.99];
        let sig2 = [-0.99, -0.2, 0.3, 0.0];
        let channels = [&sig1[..], &sig2[..]];

        let encoded = encode_wav(&channels, 48000, WavFormat::Pcm16).expect("Encode failed");
        let decoded = decode_wav(&encoded).expect("Decode failed");

        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.sample_rate, 48000);
        assert_eq!(decoded.format, WavFormat::Pcm16);
        assert_eq!(decoded.channel_samples[0].len(), 4);

        for (orig, dec) in sig1.iter().zip(decoded.channel_samples[0].iter()) {
            assert!((orig - dec).abs() < 1e-4);
        }
    }

    #[test]
    fn test_wav_roundtrip_float32() {
        let sig = [0.0, 1.25, -2.5, 4.0];
        let channels = [&sig[..]];

        let encoded = encode_wav(&channels, 96000, WavFormat::Float32).expect("Encode failed");
        let decoded = decode_wav(&encoded).expect("Decode failed");

        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.sample_rate, 96000);
        assert_eq!(decoded.format, WavFormat::Float32);

        for (orig, dec) in sig.iter().zip(decoded.channel_samples[0].iter()) {
            assert_eq!(orig, dec); // Exact IEEE 754 bit-exact roundtrip
        }
    }
}
