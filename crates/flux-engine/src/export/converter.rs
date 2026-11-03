//! Single-file and batch format converter preserving metadata and artwork.

use super::job::{
    ExportChannelMode, ExportError, ExportJob, ExportOptions, ExportProgress, ExportTargetFormat,
};
use super::normalize::NormalizationMode;
use crate::decoder::{AudioBuffer, ContainerMetadata};
use flux_dsp::dither::DitherType;
use std::sync::atomic::AtomicBool;

/// Options for format conversion.
#[derive(Debug, Clone, PartialEq)]
pub struct FormatConvertOptions {
    /// Target format.
    pub target_format: ExportTargetFormat,
    /// Target sample rate (or None to preserve original).
    pub target_sample_rate: Option<u32>,
    /// Target channel mode.
    pub target_channel_mode: ExportChannelMode,
    /// Optional metadata override; if None, original metadata is preserved.
    pub override_metadata: Option<ContainerMetadata>,
    /// Optional normalization.
    pub normalization: NormalizationMode,
    /// Dither algorithm.
    pub dither: DitherType,
}

impl Default for FormatConvertOptions {
    fn default() -> Self {
        Self {
            target_format: ExportTargetFormat::Wav(super::wav::WavBitsPerSample::Bits16),
            target_sample_rate: None,
            target_channel_mode: ExportChannelMode::Keep,
            override_metadata: None,
            normalization: NormalizationMode::Off,
            dither: DitherType::None,
        }
    }
}

/// Standalone audio format converter.
pub struct FormatConverter;

impl FormatConverter {
    /// Converts an `AudioBuffer` and its associated `ContainerMetadata` to target format bytes.
    pub fn convert<F>(
        source: &AudioBuffer,
        source_metadata: Option<&ContainerMetadata>,
        options: &FormatConvertOptions,
        cancel_token: Option<&AtomicBool>,
        on_progress: F,
    ) -> Result<Vec<u8>, ExportError>
    where
        F: FnMut(ExportProgress),
    {
        // Preserve original metadata and artwork unless explicitly overridden
        let final_metadata = if let Some(override_meta) = &options.override_metadata {
            Some(override_meta.clone())
        } else {
            source_metadata.cloned()
        };

        let export_opts = ExportOptions {
            format: options.target_format,
            sample_rate: options.target_sample_rate,
            channel_mode: options.target_channel_mode,
            region: None,
            tail_frames: 0,
            fade_out_frames: 0,
            normalization: options.normalization,
            dither: options.dither,
            metadata: final_metadata,
            force_rf64: false,
        };

        ExportJob::run(source, &export_opts, cancel_token, on_progress)
    }
}
