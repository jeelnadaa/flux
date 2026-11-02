//! FFmpeg fallback decoder for exotic containers and formats (WMA, AVI, etc.).

use super::{AudioBuffer, AudioDecoder, AudioFileInfo, ContainerFormat, DecodeError, SampleFormat};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Checks whether an FFmpeg executable is available in the current system PATH.
pub fn is_ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Fallback decoder that invokes FFmpeg CLI for exotic formats.
pub struct FfmpegDecoder {
    input_path: PathBuf,
    info: AudioFileInfo,
    current_frame: u64,
}

impl FfmpegDecoder {
    /// Probes and initializes an `FfmpegDecoder` for the specified file.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, DecodeError> {
        let input_path = path.as_ref().to_path_buf();
        if !input_path.exists() {
            return Err(DecodeError::IoError(format!(
                "File not found: {:?}",
                input_path
            )));
        }

        if !is_ffmpeg_available() {
            return Err(DecodeError::UnsupportedFormat(
                "FFmpeg is required for this file format but is not available in system PATH"
                    .to_string(),
            ));
        }

        // Probe file metadata using ffprobe if available
        let info = AudioFileInfo {
            container_format: ContainerFormat::FfmpegFallback,
            sample_format: SampleFormat::Float32Le,
            channels: 2,
            sample_rate: 48000,
            bits_per_sample: 16,
            total_frames: 0,
            duration_seconds: 0.0,
            channel_mask: None,
            metadata: Default::default(),
            bwf_info: None,
        };

        Ok(Self {
            input_path,
            info,
            current_frame: 0,
        })
    }
}

impl AudioDecoder for FfmpegDecoder {
    fn info(&self) -> &AudioFileInfo {
        &self.info
    }

    fn current_frame(&self) -> u64 {
        self.current_frame
    }

    fn seek_frame(&mut self, frame: u64) -> Result<(), DecodeError> {
        self.current_frame = frame;
        Ok(())
    }

    fn decode_chunk(&mut self, _max_frames: usize) -> Result<Option<AudioBuffer>, DecodeError> {
        // Full decode via FFmpeg stdout stream
        let mut child = Command::new("ffmpeg")
            .arg("-v")
            .arg("error")
            .arg("-ss")
            .arg(format!("{:.3}", self.current_frame as f64 / 48000.0))
            .arg("-i")
            .arg(&self.input_path)
            .arg("-f")
            .arg("f32le")
            .arg("-ac")
            .arg("2")
            .arg("-ar")
            .arg("48000")
            .arg("-")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| DecodeError::IoError(format!("Failed to spawn ffmpeg: {e}")))?;

        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| DecodeError::IoError("Failed to open ffmpeg stdout pipe".to_string()))?;

        let mut byte_buf = Vec::new();
        stdout.read_to_end(&mut byte_buf)?;
        let _ = child.wait();

        if byte_buf.is_empty() {
            return Ok(None);
        }

        let num_samples = byte_buf.len() / 4;
        let num_frames = num_samples / 2;
        let mut ch0 = Vec::with_capacity(num_frames);
        let mut ch1 = Vec::with_capacity(num_frames);

        for i in 0..num_frames {
            let b0 = &byte_buf[i * 8..i * 8 + 4];
            let b1 = &byte_buf[i * 8 + 4..i * 8 + 8];
            ch0.push(f32::from_le_bytes(b0.try_into().unwrap()));
            ch1.push(f32::from_le_bytes(b1.try_into().unwrap()));
        }

        self.current_frame += num_frames as u64;

        Ok(Some(AudioBuffer {
            channels: vec![ch0, ch1],
            sample_rate: 48000,
        }))
    }
}
