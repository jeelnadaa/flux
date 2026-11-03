//! Background export job execution with cancellation, progress tracking,
//! region selection, tail padding, fade-out, resampling, normalization, and dithering.

use std::ops::Range;
use std::sync::atomic::{AtomicBool, Ordering};

use super::aac::{AacBitrate, AacEncoder};
use super::flac::{FlacCompressionLevel, FlacEncoder};
use super::mp3::{Mp3Bitrate, Mp3Encoder};
use super::normalize::{NormalizationMode, Normalizer};
use super::opus::{OpusApplication, OpusEncoder};
use super::vorbis::{VorbisEncoder, VorbisQuality};
use super::wav::{WavBitsPerSample, WavEncoder};

use crate::decoder::{AudioBuffer, ContainerMetadata};
use flux_dsp::dither::{BitDepth, Dither, DitherType};
use flux_dsp::resampler::{Resampler, ResamplerQuality};

/// Target audio container and codec format for export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportTargetFormat {
    /// WAV with specified bit depth / float format.
    Wav(WavBitsPerSample),
    /// Lossless FLAC with compression level and bits per sample (16 or 24).
    Flac(FlacCompressionLevel, u8),
    /// MPEG-1 Audio Layer III (MP3).
    Mp3(Mp3Bitrate),
    /// Advanced Audio Coding (AAC-LC) ADTS.
    Aac(AacBitrate),
    /// Ogg Vorbis.
    Vorbis(VorbisQuality),
    /// Ogg Opus with application mode and target bitrate (kbps).
    Opus(OpusApplication, u32),
}

/// Output channel configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportChannelMode {
    /// Preserve source channel count.
    Keep,
    /// Downmix to mono (averaging left + right).
    ForceMono,
    /// Duplicate mono to stereo or keep stereo.
    ForceStereo,
}

/// Full configuration for an audio export job.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportOptions {
    /// Target format and codec parameters.
    pub format: ExportTargetFormat,
    /// Target sample rate in Hz, or None to keep source sample rate.
    pub sample_rate: Option<u32>,
    /// Target channel configuration.
    pub channel_mode: ExportChannelMode,
    /// Optional timeline region to export (start_frame..end_frame).
    pub region: Option<Range<usize>>,
    /// Number of tail frames to append (e.g. reverb/delay decay).
    pub tail_frames: usize,
    /// Number of fade-out frames to apply to the end of the exported audio.
    pub fade_out_frames: usize,
    /// Loudness / Peak normalization mode.
    pub normalization: NormalizationMode,
    /// Dither algorithm when reducing bit depth.
    pub dither: DitherType,
    /// Metadata tags and cover art to embed in the exported file.
    pub metadata: Option<ContainerMetadata>,
    /// Force RF64 chunk header for WAV format.
    pub force_rf64: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            format: ExportTargetFormat::Wav(WavBitsPerSample::Bits16),
            sample_rate: None,
            channel_mode: ExportChannelMode::Keep,
            region: None,
            tail_frames: 0,
            fade_out_frames: 0,
            normalization: NormalizationMode::Off,
            dither: DitherType::None,
            metadata: None,
            force_rf64: false,
        }
    }
}

/// Progress update published during background export.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportProgress {
    /// Normalized progress from 0.0 to 1.0.
    pub fraction: f32,
    /// Current pipeline stage name.
    pub stage: &'static str,
    /// Number of frames processed so far.
    pub frames_processed: usize,
    /// Total frames in the render pipeline.
    pub total_frames: usize,
}

/// Errors that can occur during export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    /// The export was aborted by the user via cancellation token.
    Cancelled,
    /// Input parameters or buffer invalid.
    InvalidInput(&'static str),
    /// Codec bitstream encoding failed.
    EncodeError(&'static str),
}

/// Export pipeline executor.
pub struct ExportJob;

impl ExportJob {
    /// Executes the complete export pipeline synchronously or inside a background thread/worker.
    pub fn run<F>(
        source: &AudioBuffer,
        options: &ExportOptions,
        cancel_token: Option<&AtomicBool>,
        mut on_progress: F,
    ) -> Result<Vec<u8>, ExportError>
    where
        F: FnMut(ExportProgress),
    {
        if source.channels.is_empty() {
            return Err(ExportError::InvalidInput("Source buffer has 0 channels"));
        }
        let total_source_frames = source.num_frames();
        if total_source_frames == 0 {
            return Err(ExportError::InvalidInput("Source buffer has 0 frames"));
        }

        Self::check_cancelled(cancel_token)?;

        // 1. Stage: Region slicing and tail handling
        let (region_start, region_end) = match &options.region {
            Some(range) => {
                let start = range.start.min(total_source_frames);
                let end = range.end.min(total_source_frames).max(start);
                (start, end)
            }
            None => (0, total_source_frames),
        };
        let region_len = region_end.saturating_sub(region_start);
        let total_frames = region_len + options.tail_frames;

        on_progress(ExportProgress {
            fraction: 0.05,
            stage: "slicing",
            frames_processed: 0,
            total_frames,
        });

        // Copy region + tail frames into working buffer
        let mut work_channels = Vec::with_capacity(source.channels.len());
        for ch in &source.channels {
            let mut ch_buf = Vec::with_capacity(total_frames);
            if region_len > 0 {
                ch_buf.extend_from_slice(&ch[region_start..region_end]);
            }
            if options.tail_frames > 0 {
                ch_buf.resize(total_frames, 0.0);
            }
            work_channels.push(ch_buf);
        }

        // Apply smooth cosine fade-out if configured
        if options.fade_out_frames > 0 && total_frames > 0 {
            let fade_len = options.fade_out_frames.min(total_frames);
            let fade_start = total_frames - fade_len;
            for ch in &mut work_channels {
                for (i, sample) in ch[fade_start..].iter_mut().enumerate() {
                    let progress = (i as f32) / (fade_len as f32);
                    // Equal-power cosine curve: cos(progress * pi / 2)
                    let gain = (progress * std::f32::consts::FRAC_PI_2).cos();
                    *sample *= gain;
                }
            }
        }

        Self::check_cancelled(cancel_token)?;

        // 2. Stage: Channel mode configuration
        let final_channels = match options.channel_mode {
            ExportChannelMode::Keep => work_channels,
            ExportChannelMode::ForceMono => {
                if work_channels.len() == 1 {
                    work_channels
                } else {
                    let mut mono = vec![0.0f32; total_frames];
                    let inv_ch = 1.0 / (work_channels.len() as f32);
                    for ch in &work_channels {
                        for (i, &s) in ch.iter().enumerate() {
                            mono[i] += s * inv_ch;
                        }
                    }
                    vec![mono]
                }
            }
            ExportChannelMode::ForceStereo => {
                if work_channels.len() == 1 {
                    vec![work_channels[0].clone(), work_channels[0].clone()]
                } else if work_channels.len() >= 2 {
                    vec![work_channels[0].clone(), work_channels[1].clone()]
                } else {
                    work_channels
                }
            }
        };

        let mut work_buffer = AudioBuffer::from_channels(final_channels, source.sample_rate);

        // 3. Stage: Resampling
        if let Some(target_rate) = options.sample_rate {
            if target_rate != work_buffer.sample_rate {
                on_progress(ExportProgress {
                    fraction: 0.25,
                    stage: "resampling",
                    frames_processed: total_frames / 4,
                    total_frames,
                });

                let resampler =
                    Resampler::new(work_buffer.sample_rate, target_rate, ResamplerQuality::High);
                let mut resampled_channels = Vec::with_capacity(work_buffer.channels.len());
                for ch in &work_buffer.channels {
                    resampled_channels.push(resampler.resample_channel(ch));
                }
                work_buffer = AudioBuffer::from_channels(resampled_channels, target_rate);
            }
        }

        Self::check_cancelled(cancel_token)?;

        // 4. Stage: Loudness & Peak Normalization
        if options.normalization != NormalizationMode::Off {
            on_progress(ExportProgress {
                fraction: 0.50,
                stage: "normalizing",
                frames_processed: total_frames / 2,
                total_frames,
            });
            Normalizer::apply(&mut work_buffer, options.normalization);
        }

        Self::check_cancelled(cancel_token)?;

        // 5. Stage: Dithering
        if options.dither != DitherType::None {
            on_progress(ExportProgress {
                fraction: 0.70,
                stage: "dithering",
                frames_processed: (total_frames * 3) / 4,
                total_frames,
            });

            let bit_depth = match options.format {
                ExportTargetFormat::Wav(WavBitsPerSample::Bits16)
                | ExportTargetFormat::Flac(_, 16)
                | ExportTargetFormat::Mp3(_)
                | ExportTargetFormat::Aac(_)
                | ExportTargetFormat::Vorbis(_)
                | ExportTargetFormat::Opus(_, _) => BitDepth::Bits16,
                ExportTargetFormat::Wav(WavBitsPerSample::Bits24)
                | ExportTargetFormat::Flac(_, 24) => BitDepth::Bits24,
                ExportTargetFormat::Wav(WavBitsPerSample::Float32) => BitDepth::Float32,
                _ => BitDepth::Bits16,
            };

            let mut dither = Dither::new(options.dither, bit_depth, work_buffer.channels.len());
            let mut slices: Vec<&mut [f32]> = work_buffer
                .channels
                .iter_mut()
                .map(|c| c.as_mut_slice())
                .collect();
            dither.process(&mut slices);
        }

        Self::check_cancelled(cancel_token)?;

        // 6. Stage: Encoding
        on_progress(ExportProgress {
            fraction: 0.85,
            stage: "encoding",
            frames_processed: total_frames,
            total_frames,
        });

        let out_bytes = match options.format {
            ExportTargetFormat::Wav(wav_bits) => WavEncoder::encode(
                &work_buffer,
                wav_bits,
                options.metadata.as_ref(),
                options.force_rf64,
            )
            .map_err(ExportError::EncodeError)?,
            ExportTargetFormat::Flac(level, bps) => {
                FlacEncoder::encode(&work_buffer, bps, level, options.metadata.as_ref())
                    .map_err(ExportError::EncodeError)?
            }
            ExportTargetFormat::Mp3(bitrate) => {
                Mp3Encoder::encode(&work_buffer, bitrate, options.metadata.as_ref())
                    .map_err(ExportError::EncodeError)?
            }
            ExportTargetFormat::Aac(bitrate) => {
                AacEncoder::encode(&work_buffer, bitrate, options.metadata.as_ref())
                    .map_err(ExportError::EncodeError)?
            }
            ExportTargetFormat::Vorbis(quality) => {
                VorbisEncoder::encode(&work_buffer, quality, options.metadata.as_ref())
                    .map_err(ExportError::EncodeError)?
            }
            ExportTargetFormat::Opus(app, kbps) => {
                OpusEncoder::encode(&work_buffer, app, kbps, options.metadata.as_ref())
                    .map_err(ExportError::EncodeError)?
            }
        };

        on_progress(ExportProgress {
            fraction: 1.0,
            stage: "finished",
            frames_processed: total_frames,
            total_frames,
        });

        Ok(out_bytes)
    }

    #[inline]
    fn check_cancelled(token: Option<&AtomicBool>) -> Result<(), ExportError> {
        if let Some(t) = token {
            if t.load(Ordering::Relaxed) {
                return Err(ExportError::Cancelled);
            }
        }
        Ok(())
    }
}
