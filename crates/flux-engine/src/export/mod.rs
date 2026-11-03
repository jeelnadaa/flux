//! Audio export and encoding pipeline (WAV, RF64, FLAC, MP3, AAC, Vorbis, Opus).

pub mod aac;
pub mod converter;
pub mod flac;
pub mod job;
pub mod mp3;
pub mod normalize;
pub mod opus;
pub mod presets;
pub mod vorbis;
pub mod wav;

pub use aac::{AacBitrate, AacEncoder};
pub use converter::{FormatConvertOptions, FormatConverter};
pub use flac::{FlacCompressionLevel, FlacEncoder};
pub use job::{
    ExportChannelMode, ExportError, ExportJob, ExportOptions, ExportProgress, ExportTargetFormat,
};
pub use mp3::{Mp3Bitrate, Mp3Encoder};
pub use normalize::{NormalizationMode, Normalizer};
pub use opus::{OpusApplication, OpusEncoder};
pub use presets::{ExportPresetConfig, ExportPresetId};
pub use vorbis::{VorbisEncoder, VorbisQuality};
pub use wav::{WavBitsPerSample, WavEncoder};
