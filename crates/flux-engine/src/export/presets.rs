//! Export destination presets for streaming, podcast, video, CD, and archival.

use super::normalize::NormalizationMode;
use flux_dsp::dither::DitherType;

/// Predefined export destination preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportPresetId {
    /// Streaming platforms (Spotify, Apple Music, Tidal): -14 LUFS, -1.0 dBTP.
    Streaming,
    /// Podcast stereo: -16 LUFS, -1.0 dBTP.
    PodcastStereo,
    /// Podcast mono: -19 LUFS, -1.0 dBTP.
    PodcastMono,
    /// YouTube audio: -14 LUFS, -1.0 dBTP.
    YouTube,
    /// Red Book CD Audio: 16-bit / 44.1 kHz with TPDF dither, -0.3 dBFS peak.
    CdAudio,
    /// Archival studio master: 24-bit / 48 kHz FLAC, -0.1 dBFS peak.
    Archival,
}

/// Resolved configuration parameters for an export destination preset.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportPresetConfig {
    /// Preset identifier.
    pub id: ExportPresetId,
    /// Human-readable display label.
    pub name: &'static str,
    /// Human-readable description.
    pub description: &'static str,
    /// Target sample rate (Hz), or None to keep project rate.
    pub sample_rate: Option<u32>,
    /// Target bit depth (16, 24, 32).
    pub bit_depth: Option<u8>,
    /// Dither algorithm.
    pub dither: DitherType,
    /// Loudness / Peak normalization mode.
    pub normalization: NormalizationMode,
    /// Channel downmix requirement (e.g. force mono for Podcast mono).
    pub force_mono: bool,
    /// Recommended container format extension.
    pub file_extension: &'static str,
}

impl ExportPresetId {
    /// Resolves the preset configuration.
    pub fn config(&self) -> ExportPresetConfig {
        match self {
            Self::Streaming => ExportPresetConfig {
                id: *self,
                name: "Streaming (Spotify / Apple Music / Tidal)",
                description: "Target -14 LUFS with -1.0 dBTP true-peak ceiling for streaming services",
                sample_rate: None,
                bit_depth: None,
                dither: DitherType::None,
                normalization: NormalizationMode::Lufs {
                    target_lufs: -14.0,
                    true_peak_ceiling_dbtp: -1.0,
                },
                force_mono: false,
                file_extension: "wav",
            },
            Self::PodcastStereo => ExportPresetConfig {
                id: *self,
                name: "Podcast (Stereo)",
                description: "Target -16 LUFS with -1.0 dBTP true-peak ceiling for stereo spoken-word / podcast distribution",
                sample_rate: None,
                bit_depth: None,
                dither: DitherType::None,
                normalization: NormalizationMode::Lufs {
                    target_lufs: -16.0,
                    true_peak_ceiling_dbtp: -1.0,
                },
                force_mono: false,
                file_extension: "mp3",
            },
            Self::PodcastMono => ExportPresetConfig {
                id: *self,
                name: "Podcast (Mono)",
                description: "Target -19 LUFS with -1.0 dBTP true-peak ceiling and mono downmix",
                sample_rate: None,
                bit_depth: None,
                dither: DitherType::None,
                normalization: NormalizationMode::Lufs {
                    target_lufs: -19.0,
                    true_peak_ceiling_dbtp: -1.0,
                },
                force_mono: true,
                file_extension: "mp3",
            },
            Self::YouTube => ExportPresetConfig {
                id: *self,
                name: "YouTube",
                description: "Target -14 LUFS with -1.0 dBTP true-peak ceiling optimized for video audio",
                sample_rate: Some(48000),
                bit_depth: None,
                dither: DitherType::None,
                normalization: NormalizationMode::Lufs {
                    target_lufs: -14.0,
                    true_peak_ceiling_dbtp: -1.0,
                },
                force_mono: false,
                file_extension: "wav",
            },
            Self::CdAudio => ExportPresetConfig {
                id: *self,
                name: "CD Audio (Red Book)",
                description: "16-bit / 44.1 kHz PCM with TPDF dither and -0.3 dBFS peak ceiling",
                sample_rate: Some(44100),
                bit_depth: Some(16),
                dither: DitherType::Tpdf,
                normalization: NormalizationMode::Peak {
                    target_dbfs: -0.3,
                },
                force_mono: false,
                file_extension: "wav",
            },
            Self::Archival => ExportPresetConfig {
                id: *self,
                name: "Archival Master (24-bit FLAC)",
                description: "24-bit / 48 kHz FLAC lossless archival with -0.1 dBFS peak margin",
                sample_rate: Some(48000),
                bit_depth: Some(24),
                dither: DitherType::None,
                normalization: NormalizationMode::Peak {
                    target_dbfs: -0.1,
                },
                force_mono: false,
                file_extension: "flac",
            },
        }
    }

    /// Returns all available preset IDs.
    pub fn all() -> &'static [ExportPresetId] {
        &[
            Self::Streaming,
            Self::PodcastStereo,
            Self::PodcastMono,
            Self::YouTube,
            Self::CdAudio,
            Self::Archival,
        ]
    }
}
