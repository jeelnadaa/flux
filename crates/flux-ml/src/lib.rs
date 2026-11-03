//! Flux ML runtime integration, model management, stem separation, voice cleanup, transcription, and cloud mocking.
#![deny(missing_docs)]

pub mod cleanup;
pub mod cloud;
pub mod fillers;
pub mod models;
pub mod runtime;
pub mod stems;
pub mod transcript;
pub mod vad;

pub use cleanup::{AudioCategory, AudioClassifier, VoiceCleanupConfig, VoiceCleanupEngine};
pub use cloud::{CloudJobRecord, CloudJobStatus, MockCloudClient, PrivacyConsentSettings};
pub use fillers::{FillerDetector, FillerDetectorConfig, FillerOccurrence, FillerType};
pub use models::{ModelInfo, ModelManager, ModelTier};
pub use runtime::{ChunkedOverlapAdd, DeviceCapabilities, DeviceProber, MlBackend};
pub use stems::{
    SeparatedStem, StemQualityMode, StemSeparationConfig, StemSeparationEngine, StemType,
};
pub use transcript::{SubtitleFormat, TranscriptBlock, TranscriptDocument, WordToken};
pub use vad::{AutoDucker, SegmentType, TimelineSegment, VoiceActivityDetector};
