//! Flux ML runtime integration, model management, stem separation, voice cleanup, transcription, advanced analysis, mastering, and cloud mocking.
#![deny(missing_docs)]

pub mod cleanup;
pub mod cloud;
pub mod dubbing;
pub mod fillers;
pub mod key_detect;
pub mod magic;
pub mod mastering;
pub mod models;
pub mod runtime;
pub mod stems;
pub mod tempo_detect;
pub mod transcript;
pub mod tts;
pub mod vad;
pub mod voice_clone;

pub use cleanup::{AudioCategory, AudioClassifier, VoiceCleanupConfig, VoiceCleanupEngine};
pub use cloud::{CloudJobRecord, CloudJobStatus, MockCloudClient, PrivacyConsentSettings};
pub use dubbing::{DubLine, DubbingEngine, DubbingProject};
pub use fillers::{FillerDetector, FillerDetectorConfig, FillerOccurrence, FillerType};
pub use key_detect::{KeyDetector, KeyMode, MusicalKey};
pub use magic::{AudioDiagnosis, MagicAssistant, MagicGoal, ProposedEffect};
pub use mastering::{
    AutoMasteringConfig, AutoMasteringEngine, MasteringExplanation, MasteringIntensity,
    MasteringTarget,
};
pub use models::{ModelInfo, ModelManager, ModelTier};
pub use runtime::{ChunkedOverlapAdd, DeviceCapabilities, DeviceProber, MlBackend};
pub use stems::{
    SeparatedStem, StemQualityMode, StemSeparationConfig, StemSeparationEngine, StemType,
};
pub use tempo_detect::{TempoDetector, TempoEstimate};
pub use transcript::{SubtitleFormat, TranscriptBlock, TranscriptDocument, WordToken};
pub use tts::{TtsEngine, TtsRequest, TtsResult, TtsVoice};
pub use vad::{AutoDucker, SegmentType, TimelineSegment, VoiceActivityDetector};
pub use voice_clone::{VoiceCloneConsent, VoiceCloneManager, WatermarkSecurity};
