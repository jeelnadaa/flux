//! Flux DSP core processing library.
#![deny(missing_docs)]

pub mod binaural;
pub mod biquad;
pub mod creative;
pub mod delay;
pub mod dither;
pub mod dynamics;
pub mod eq;
pub mod loudness;
pub mod node;
pub mod param;
pub mod pyramid;
pub mod resampler;
pub mod restoration;
pub mod reverb;
pub mod sample;
pub mod spatial;
pub mod spatial_analysis;
pub mod spectrogram;
pub mod stretch;
pub mod tempo;
pub mod utility;
pub mod voice;

pub use binaural::BinauralNode;
pub use biquad::{BiquadChain, BiquadCoeffs, BiquadState, BiquadType};
pub use delay::DelayLine;
pub use dither::{BitDepth, Dither, DitherType};
pub use dynamics::{
    normalize_lufs, normalize_peak, AdaptiveLeveler, AdaptiveLevelerConfig, Compressor,
    CompressorConfig, DeEsser, DeEsserConfig, GateExpander, GateExpanderConfig,
    MultibandCompressor, TransientShaper, TransientShaperConfig, TruePeakLimiter,
    TruePeakLimiterConfig,
};
pub use eq::{
    EqBandConfig, EqFilterType, EqMode, EqPresetDefinition, EqPresets, GraphicEqPresets, MatchEq,
    ParametricEq, LINEAR_PHASE_FIR_LENGTH, MAX_EQ_BANDS,
};
pub use loudness::{LoudnessMeasurement, LoudnessMeter};
pub use node::{DspNode, GainNode, PassthroughNode};
pub use param::{AutomationLane, AutomationPoint, SmoothedParameter, SmoothingMode};
pub use pyramid::{PeakBin, PeakPyramid, PyramidLevel};
pub use resampler::{Resampler, ResamplerQuality};
pub use restoration::{
    DcOffsetFilter, DeClicker, DeClipper, DeWindFilter, HumRemover, PanLaw, PauseInterval,
    SilenceRemover, SpectralRepair, StationaryNoiseReduction, StereoTools,
};
pub use reverb::{
    compute_sabine_rt60, AlgorithmicReverbNode, ConcertHallNode, ConvolutionReverbNode,
    NextDoorNode, RoomPreset, RoomReverbNode, SurfaceMaterial,
};
pub use sample::Sample;
pub use spatial::{
    compute_spherical_harmonics, AmbisonicRenderer, DirectHrtfRenderer, DistanceCues,
    FractionalDelay, HrtfDataset, HrtfPair, HrtfProfile, MonoBassFilter, PartitionedConvolver,
    StereoInputMode, StereoSpatializerNode, AMBISONIC_CHANNELS, AMBISONIC_ORDER,
    DEFAULT_HEAD_RADIUS, HRTF_IR_LENGTH, SPEED_OF_SOUND,
};
pub use spatial_analysis::{GoniometerPoint, StereoAnalysisResult, StereoAnalyzer};
pub use spectrogram::{
    ColorMap, FrequencyScale, SpectrogramConfig, SpectrogramTile, SpectrogramTileGenerator,
};
pub use stretch::{StretchMode, StretchNode};
pub use tempo::{TapTempo, TempoMap, TempoPoint};
pub use utility::{
    FadeCurveType, FadeProcessor, HearingProtectionConfig, HearingProtectionNode, NoiseGenerator,
    NoiseType, SilenceGenerator, SpectrumAnalyzerNode, ToneGenerator, WaveformType,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_type_precision() {
        let s: Sample = 0.0;
        assert_eq!(s, 0.0);
    }

    #[test]
    fn test_gain_node_basic() {
        let mut gain = GainNode::new(0.5);
        gain.prepare(48000.0, 1);
        let in_buf = vec![1.0; 128];
        let mut out_buf = vec![0.0; 128];

        gain.process(&[&in_buf], &mut [&mut out_buf]);
        // With smoothing from 0.5 to 0.5, gain stays ~0.5
        assert!((out_buf[127] - 0.5).abs() < 0.01);
    }
}
