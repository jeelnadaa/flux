//! Flux WebAssembly bindings for browser execution and AudioWorklet rendering.
#![deny(missing_docs)]

use flux_dsp::binaural::BinauralNode;
use flux_dsp::biquad::{BiquadChain, BiquadType};
use flux_dsp::creative::{LofiMacroNode, NightcoreNode, SlowedReverbNode, VinylNode};
use flux_dsp::dither::{BitDepth, Dither, DitherType};
use flux_dsp::loudness::LoudnessMeter;
use flux_dsp::node::DspNode;
use flux_dsp::pyramid::PeakPyramid;
use flux_dsp::resampler::{Resampler, ResamplerQuality};
use flux_dsp::reverb::{
    compute_sabine_rt60, AlgorithmicReverbNode, ConcertHallNode, NextDoorNode, RoomPreset,
    RoomReverbNode, SurfaceMaterial,
};
use flux_dsp::spatial::{
    check_mono_compatibility, SpatialPreset, StereoInputMode, StereoSpatializerNode,
    TrajectoryDirection, TrajectoryMode,
};
use flux_dsp::spatial_analysis::StereoAnalyzer;
use flux_dsp::spectrogram::{
    ColorMap, FrequencyScale, SpectrogramConfig, SpectrogramTileGenerator,
};
use flux_dsp::stretch::StretchNode;
use flux_dsp::voice::{VoiceChangerNode, VoiceCharacter};
use flux_engine::capture::{ChunkedRecordWriter, InputLevelMeter, PreRollBuffer};
use flux_engine::codec::{AudioMetadata, Mp3Encoder, OpusEncoder, VorbisEncoder};
use flux_engine::decoder::{open_decoder, AudioBuffer, ContainerMetadata};
use flux_engine::export::{
    AacBitrate, ExportChannelMode, ExportJob, ExportOptions, ExportPresetId, ExportTargetFormat,
    FlacCompressionLevel, FormatConvertOptions, FormatConverter, Mp3Bitrate, NormalizationMode,
    OpusApplication, VorbisQuality, WavBitsPerSample,
};
use flux_engine::render::{EngineCommand, EngineHost};
use flux_engine::transport::Transport;
use flux_ml::cleanup::{AudioCategory, AudioClassifier, VoiceCleanupConfig, VoiceCleanupEngine};
use flux_ml::cloud::MockCloudClient;
use flux_ml::fillers::FillerDetector;
use flux_ml::models::ModelManager;
use flux_ml::transcript::{SubtitleFormat, TranscriptDocument};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use wasm_bindgen::prelude::*;

/// Returns the Flux DSP engine version string.
#[wasm_bindgen]
pub fn flux_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// WebAssembly wrapper around `BiquadChain` for high-level JS APIs.
#[wasm_bindgen]
pub struct WasmBiquadChain {
    inner: BiquadChain,
}

#[wasm_bindgen]
impl WasmBiquadChain {
    /// Creates a new cascaded biquad filter chain.
    #[wasm_bindgen(constructor)]
    pub fn new(sample_rate: f32, stages: usize) -> Self {
        let mut chain = BiquadChain::new(stages);
        chain.prepare(sample_rate, 1);
        Self { inner: chain }
    }

    /// Configures a parametric peaking EQ stage.
    pub fn configure_peaking(&mut self, stage: usize, freq: f32, q: f32, gain_db: f32) {
        self.inner
            .configure_stage(stage, BiquadType::Peaking, freq, q, gain_db);
    }

    /// Configures a second-order lowpass filter stage.
    pub fn configure_lowpass(&mut self, stage: usize, freq: f32, q: f32) {
        self.inner
            .configure_stage(stage, BiquadType::Lowpass, freq, q, 0.0);
    }

    /// Configures a second-order highpass filter stage.
    pub fn configure_highpass(&mut self, stage: usize, freq: f32, q: f32) {
        self.inner
            .configure_stage(stage, BiquadType::Highpass, freq, q, 0.0);
    }

    /// Processes an input block into an output slice (mono).
    pub fn process_block(&mut self, input: &[f32], output: &mut [f32]) {
        self.inner.process(&[input], &mut [output]);
    }

    /// Generates a test tone of specified frequency and filters it through the biquad chain.
    pub fn process_tone(&mut self, freq: f32, frames: usize, sample_rate: f32) -> Vec<f32> {
        let mut in_buf = Vec::with_capacity(frames);
        let omega = 2.0 * std::f32::consts::PI * freq / sample_rate;
        for i in 0..frames {
            in_buf.push((i as f32 * omega).sin());
        }

        let mut out_buf = vec![0.0f32; frames];
        self.inner.process(&[&in_buf], &mut [&mut out_buf]);
        out_buf
    }

    /// Resets all internal delay buffers and filter states.
    pub fn reset(&mut self) {
        self.inner.reset();
    }
}

// ----------------------------------------------------------------------------
// AudioWorklet Real-Time C ABI
// Direct C exports for quantum processing (128 samples) with 0 glue overhead.
// ----------------------------------------------------------------------------

/// Allocates an f32 buffer in WebAssembly linear memory.
#[no_mangle]
pub extern "C" fn biquad_alloc_buffer(len: usize) -> *mut f32 {
    let mut vec = vec![0.0f32; len];
    let ptr = vec.as_mut_ptr();
    std::mem::forget(vec);
    ptr
}

/// Frees an f32 buffer in WebAssembly linear memory.
///
/// # Safety
/// `ptr` must have been returned by `biquad_alloc_buffer` with capacity `len`.
#[no_mangle]
pub unsafe extern "C" fn biquad_free_buffer(ptr: *mut f32, len: usize) {
    if !ptr.is_null() && len > 0 {
        drop(Vec::from_raw_parts(ptr, len, len));
    }
}

/// Creates a new `BiquadChain` instance.
#[no_mangle]
pub extern "C" fn biquad_create(sample_rate: f32, stages: usize) -> *mut BiquadChain {
    let mut chain = Box::new(BiquadChain::new(stages));
    chain.prepare(sample_rate, 1);
    Box::into_raw(chain)
}

/// Destroys a `BiquadChain` instance.
///
/// # Safety
/// `ptr` must be a valid pointer returned by `biquad_create`.
#[no_mangle]
pub unsafe extern "C" fn biquad_destroy(ptr: *mut BiquadChain) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Configures a peaking EQ stage.
///
/// # Safety
/// `ptr` must be a valid pointer to a `BiquadChain`.
#[no_mangle]
pub unsafe extern "C" fn biquad_configure_peaking(
    ptr: *mut BiquadChain,
    stage: usize,
    freq: f32,
    q: f32,
    gain_db: f32,
) {
    if let Some(chain) = ptr.as_mut() {
        chain.configure_stage(stage, BiquadType::Peaking, freq, q, gain_db);
    }
}

/// Configures a lowpass filter stage.
///
/// # Safety
/// `ptr` must be a valid pointer to a `BiquadChain`.
#[no_mangle]
pub unsafe extern "C" fn biquad_configure_lowpass(
    ptr: *mut BiquadChain,
    stage: usize,
    freq: f32,
    q: f32,
) {
    if let Some(chain) = ptr.as_mut() {
        chain.configure_stage(stage, BiquadType::Lowpass, freq, q, 0.0);
    }
}

/// Configures a highpass filter stage.
///
/// # Safety
/// `ptr` must be a valid pointer to a `BiquadChain`.
#[no_mangle]
pub unsafe extern "C" fn biquad_configure_highpass(
    ptr: *mut BiquadChain,
    stage: usize,
    freq: f32,
    q: f32,
) {
    if let Some(chain) = ptr.as_mut() {
        chain.configure_stage(stage, BiquadType::Highpass, freq, q, 0.0);
    }
}

/// Processes an AudioWorklet render quantum (128 frames) in real time.
///
/// # Safety
/// `ptr` must be valid, and `in_ptr` / `out_ptr` must point to valid slices of length `frames`.
#[no_mangle]
pub unsafe extern "C" fn biquad_process_quantum(
    ptr: *mut BiquadChain,
    in_ptr: *const f32,
    out_ptr: *mut f32,
    frames: usize,
) {
    if let Some(chain) = ptr.as_mut() {
        let in_slice = std::slice::from_raw_parts(in_ptr, frames);
        let out_slice = std::slice::from_raw_parts_mut(out_ptr, frames);
        chain.process(&[in_slice], &mut [out_slice]);
    }
}

/// Creates a new `BinauralNode` instance.
#[no_mangle]
pub extern "C" fn binaural_create(sample_rate: f32) -> *mut BinauralNode {
    let mut node = Box::new(BinauralNode::new());
    node.prepare(sample_rate, 2);
    Box::into_raw(node)
}

/// Destroys a `BinauralNode` instance.
///
/// # Safety
/// `ptr` must be a valid pointer returned by `binaural_create`.
#[no_mangle]
pub unsafe extern "C" fn binaural_destroy(ptr: *mut BinauralNode) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Sets the azimuth angle in degrees.
///
/// # Safety
/// `ptr` must be a valid pointer to a `BinauralNode`.
#[no_mangle]
pub unsafe extern "C" fn binaural_set_azimuth(ptr: *mut BinauralNode, azimuth_deg: f32) {
    if let Some(node) = ptr.as_mut() {
        node.set_azimuth(azimuth_deg);
    }
}

/// Sets automated 8D orbit speed in Hz.
///
/// # Safety
/// `ptr` must be a valid pointer to a `BinauralNode`.
#[no_mangle]
pub unsafe extern "C" fn binaural_set_orbit_speed(ptr: *mut BinauralNode, speed_hz: f32) {
    if let Some(node) = ptr.as_mut() {
        node.set_orbit_speed(speed_hz);
    }
}

/// Processes an AudioWorklet render quantum (128 frames) through the binaural panner.
///
/// # Safety
/// `ptr` must be valid, and `in_ptr`, `out_l_ptr`, `out_r_ptr` must point to valid slices of length `frames`.
#[no_mangle]
pub unsafe extern "C" fn binaural_process_quantum(
    ptr: *mut BinauralNode,
    in_ptr: *const f32,
    out_l_ptr: *mut f32,
    out_r_ptr: *mut f32,
    frames: usize,
) {
    if let Some(node) = ptr.as_mut() {
        let in_slice = std::slice::from_raw_parts(in_ptr, frames);
        let out_l_slice = std::slice::from_raw_parts_mut(out_l_ptr, frames);
        let out_r_slice = std::slice::from_raw_parts_mut(out_r_ptr, frames);
        node.process(&[in_slice], &mut [out_l_slice, out_r_slice]);
    }
}

/// Creates a new `StereoSpatializerNode` instance.
#[no_mangle]
pub extern "C" fn stereo_spatializer_create(sample_rate: f32) -> *mut StereoSpatializerNode {
    let mut node = Box::new(StereoSpatializerNode::new());
    node.prepare(sample_rate, 2);
    Box::into_raw(node)
}

/// Destroys a `StereoSpatializerNode` instance.
///
/// # Safety
/// `ptr` must be a valid pointer returned by `stereo_spatializer_create`.
#[no_mangle]
pub unsafe extern "C" fn stereo_spatializer_destroy(ptr: *mut StereoSpatializerNode) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Sets the stereo spatialization mode (0 = VirtualSpeakers, 1 = MidSide).
///
/// # Safety
/// `ptr` must be a valid pointer to a `StereoSpatializerNode`.
#[no_mangle]
pub unsafe extern "C" fn stereo_spatializer_set_mode(ptr: *mut StereoSpatializerNode, mode: u32) {
    if let Some(node) = ptr.as_mut() {
        let m = if mode == 1 {
            StereoInputMode::MidSide
        } else {
            StereoInputMode::VirtualSpeakers
        };
        node.set_mode(m);
    }
}

/// Sets center azimuth in degrees.
///
/// # Safety
/// `ptr` must be a valid pointer to a `StereoSpatializerNode`.
#[no_mangle]
pub unsafe extern "C" fn stereo_spatializer_set_azimuth(
    ptr: *mut StereoSpatializerNode,
    azimuth_deg: f32,
) {
    if let Some(node) = ptr.as_mut() {
        node.set_azimuth(azimuth_deg);
    }
}

/// Sets stereo angular spread in degrees (default 60 deg).
///
/// # Safety
/// `ptr` must be a valid pointer to a `StereoSpatializerNode`.
#[no_mangle]
pub unsafe extern "C" fn stereo_spatializer_set_spread(
    ptr: *mut StereoSpatializerNode,
    spread_deg: f32,
) {
    if let Some(node) = ptr.as_mut() {
        node.set_spread(spread_deg);
    }
}

/// Configures mono bass management.
///
/// # Safety
/// `ptr` must be a valid pointer to a `StereoSpatializerNode`.
#[no_mangle]
pub unsafe extern "C" fn stereo_spatializer_set_mono_bass(
    ptr: *mut StereoSpatializerNode,
    enabled: bool,
    crossover_hz: f32,
) {
    if let Some(node) = ptr.as_mut() {
        node.set_mono_bass_enabled(enabled);
        node.set_mono_bass_crossover(crossover_hz);
    }
}

/// Processes stereo audio frames through the stereo spatializer.
///
/// # Safety
/// `ptr` must be valid, and all buffer pointers must point to valid slices of length `frames`.
#[no_mangle]
pub unsafe extern "C" fn stereo_spatializer_process(
    ptr: *mut StereoSpatializerNode,
    in_l_ptr: *const f32,
    in_r_ptr: *const f32,
    out_l_ptr: *mut f32,
    out_r_ptr: *mut f32,
    frames: usize,
) {
    if let Some(node) = ptr.as_mut() {
        let in_l = std::slice::from_raw_parts(in_l_ptr, frames);
        let in_r = std::slice::from_raw_parts(in_r_ptr, frames);
        let out_l = std::slice::from_raw_parts_mut(out_l_ptr, frames);
        let out_r = std::slice::from_raw_parts_mut(out_r_ptr, frames);
        node.process(&[in_l, in_r], &mut [out_l, out_r]);
    }
}

/// Sets trajectory motion mode on a `StereoSpatializerNode`.
/// 0 = Place, 1 = Orbit, 2 = Spiral, 3 = FigureEight, 4 = VerticalLoop, 5 = Swarm, 6 = Lissajous, 7 = CustomPath
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn stereo_spatializer_set_trajectory_mode(
    ptr: *mut StereoSpatializerNode,
    mode: u32,
) {
    if let Some(node) = ptr.as_mut() {
        let m = match mode {
            0 => TrajectoryMode::Place,
            1 => TrajectoryMode::Orbit,
            2 => TrajectoryMode::Spiral,
            3 => TrajectoryMode::FigureEight,
            4 => TrajectoryMode::VerticalLoop,
            5 => TrajectoryMode::Swarm,
            6 => TrajectoryMode::Lissajous,
            _ => TrajectoryMode::CustomPath,
        };
        node.set_trajectory_mode(m);
    }
}

/// Sets trajectory direction on a `StereoSpatializerNode`.
/// 0 = Clockwise, 1 = CounterClockwise, 2 = PingPong
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn stereo_spatializer_set_trajectory_direction(
    ptr: *mut StereoSpatializerNode,
    direction: u32,
) {
    if let Some(node) = ptr.as_mut() {
        let d = match direction {
            0 => TrajectoryDirection::Clockwise,
            1 => TrajectoryDirection::CounterClockwise,
            _ => TrajectoryDirection::PingPong,
        };
        node.set_trajectory_direction(d);
    }
}

/// Sets orbit rotation speed in Hz (0.01..2.0).
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn stereo_spatializer_set_orbit_speed(
    ptr: *mut StereoSpatializerNode,
    speed_hz: f32,
) {
    if let Some(node) = ptr.as_mut() {
        node.set_orbit_speed_hz(speed_hz);
    }
}

/// Sets trajectory depth factor (0.0..1.0).
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn stereo_spatializer_set_trajectory_depth(
    ptr: *mut StereoSpatializerNode,
    depth: f32,
) {
    if let Some(node) = ptr.as_mut() {
        node.set_trajectory_depth(depth);
    }
}

/// Sets trajectory smoothing factor (0.0..1.0).
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn stereo_spatializer_set_trajectory_smoothing(
    ptr: *mut StereoSpatializerNode,
    smoothing: f32,
) {
    if let Some(node) = ptr.as_mut() {
        node.set_trajectory_smoothing(smoothing);
    }
}

/// Configures tempo synchronization on a `StereoSpatializerNode`.
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn stereo_spatializer_set_tempo_sync(
    ptr: *mut StereoSpatializerNode,
    enabled: bool,
    bpm: f32,
    beats_per_rev: f32,
) {
    if let Some(node) = ptr.as_mut() {
        node.set_tempo_sync(enabled, bpm, beats_per_rev);
    }
}

/// Applies a 3D static placement preset to a `StereoSpatializerNode`.
/// 0 = Front, 1 = Behind, 2 = Above, 3 = Below, 4 = FrontLeft, 5 = FrontRight, 6 = RearLeft, 7 = RearRight, 8 = InsideRoom
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn stereo_spatializer_apply_preset(
    ptr: *mut StereoSpatializerNode,
    preset_id: u32,
) {
    if let Some(node) = ptr.as_mut() {
        let preset = match preset_id {
            0 => SpatialPreset::Front,
            1 => SpatialPreset::Behind,
            2 => SpatialPreset::Above,
            3 => SpatialPreset::Below,
            4 => SpatialPreset::FrontLeft,
            5 => SpatialPreset::FrontRight,
            6 => SpatialPreset::RearLeft,
            7 => SpatialPreset::RearRight,
            _ => SpatialPreset::InsideRoom,
        };
        node.apply_preset(preset);
    }
}

/// Checks mono compatibility of stereo audio buffers.
///
/// # Safety
/// Buffer pointers must point to valid arrays of `frames` length.
#[no_mangle]
pub unsafe extern "C" fn stereo_check_mono_compatibility(
    left_ptr: *const f32,
    right_ptr: *const f32,
    frames: usize,
    out_correlation: *mut f32,
    out_loss_db: *mut f32,
) -> bool {
    if left_ptr.is_null() || right_ptr.is_null() || frames == 0 {
        if !out_correlation.is_null() {
            *out_correlation = 1.0;
        }
        if !out_loss_db.is_null() {
            *out_loss_db = 0.0;
        }
        return true;
    }
    let left = std::slice::from_raw_parts(left_ptr, frames);
    let right = std::slice::from_raw_parts(right_ptr, frames);
    let res = check_mono_compatibility(left, right);
    if !out_correlation.is_null() {
        *out_correlation = res.correlation;
    }
    if !out_loss_db.is_null() {
        *out_loss_db = res.mono_loss_db;
    }
    res.is_compatible
}

// ----------------------------------------------------------------------------
// Reverb & Acoustic Environment C ABI
// ----------------------------------------------------------------------------

/// Creates a new Algorithmic Reverb node (FDN).
#[no_mangle]
pub extern "C" fn reverb_create(sample_rate: f32) -> *mut AlgorithmicReverbNode {
    Box::into_raw(Box::new(AlgorithmicReverbNode::new(sample_rate)))
}

/// Destroys an Algorithmic Reverb node.
///
/// # Safety
/// `ptr` must be a valid pointer returned by `reverb_create`.
#[no_mangle]
pub unsafe extern "C" fn reverb_destroy(ptr: *mut AlgorithmicReverbNode) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Sets reverb decay RT60 in seconds.
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn reverb_set_decay(ptr: *mut AlgorithmicReverbNode, rt60: f32) {
    if let Some(node) = ptr.as_mut() {
        node.set_decay_time(rt60);
    }
}

/// Sets reverb wet mix [0.0, 1.0].
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn reverb_set_wet(ptr: *mut AlgorithmicReverbNode, wet: f32) {
    if let Some(node) = ptr.as_mut() {
        node.set_wet(wet);
    }
}

/// Sets reverb high-frequency damping [0.0, 1.0].
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn reverb_set_damping(ptr: *mut AlgorithmicReverbNode, damping: f32) {
    if let Some(node) = ptr.as_mut() {
        node.set_damping(damping);
    }
}

/// Processes stereo audio frames through algorithmic reverb.
///
/// # Safety
/// Pointers must point to valid arrays of `frames` length.
#[no_mangle]
pub unsafe extern "C" fn reverb_process(
    ptr: *mut AlgorithmicReverbNode,
    in_l: *const f32,
    in_r: *const f32,
    out_l: *mut f32,
    out_r: *mut f32,
    frames: usize,
) {
    if let Some(node) = ptr.as_mut() {
        let left_in = std::slice::from_raw_parts(in_l, frames);
        let right_in = std::slice::from_raw_parts(in_r, frames);
        let left_out = std::slice::from_raw_parts_mut(out_l, frames);
        let right_out = std::slice::from_raw_parts_mut(out_r, frames);

        node.process(&[left_in, right_in], &mut [left_out, right_out]);
    }
}

/// Creates a new Room Simulation node.
#[no_mangle]
pub extern "C" fn room_reverb_create(sample_rate: f32) -> *mut RoomReverbNode {
    Box::into_raw(Box::new(RoomReverbNode::new(sample_rate)))
}

/// Destroys a Room Simulation node.
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn room_reverb_destroy(ptr: *mut RoomReverbNode) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Applies one of the 12 room presets (0=Cathedral ... 11=SubwayTunnel).
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn room_reverb_apply_preset(ptr: *mut RoomReverbNode, preset_id: u32) {
    if let Some(node) = ptr.as_mut() {
        let preset = match preset_id {
            0 => RoomPreset::Cathedral,
            1 => RoomPreset::Bathroom,
            2 => RoomPreset::ConcertHall,
            3 => RoomPreset::Cave,
            4 => RoomPreset::ParkingGarage,
            5 => RoomPreset::StudioBooth,
            6 => RoomPreset::Stairwell,
            7 => RoomPreset::LivingRoom,
            8 => RoomPreset::CarInterior,
            9 => RoomPreset::Stadium,
            10 => RoomPreset::ForestClearing,
            _ => RoomPreset::SubwayTunnel,
        };
        node.apply_preset(preset);
    }
}

/// Updates custom room dimensions and materials.
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn room_reverb_update_custom(
    ptr: *mut RoomReverbNode,
    width_m: f32,
    depth_m: f32,
    height_m: f32,
    wall_mat_id: u32,
    floor_mat_id: u32,
    ceiling_mat_id: u32,
) {
    if let Some(node) = ptr.as_mut() {
        let parse_mat = |id: u32| match id {
            0 => SurfaceMaterial::Plaster,
            1 => SurfaceMaterial::Concrete,
            2 => SurfaceMaterial::Brick,
            3 => SurfaceMaterial::Wood,
            4 => SurfaceMaterial::Glass,
            5 => SurfaceMaterial::Curtain,
            6 => SurfaceMaterial::Carpet,
            7 => SurfaceMaterial::Stone,
            8 => SurfaceMaterial::Tile,
            _ => SurfaceMaterial::AcousticTile,
        };
        node.update_custom_room(
            width_m,
            depth_m,
            height_m,
            parse_mat(wall_mat_id),
            parse_mat(floor_mat_id),
            parse_mat(ceiling_mat_id),
        );
    }
}

/// Processes stereo audio frames through room reverb.
///
/// # Safety
/// Pointers must point to valid arrays of `frames` length.
#[no_mangle]
pub unsafe extern "C" fn room_reverb_process(
    ptr: *mut RoomReverbNode,
    in_l: *const f32,
    in_r: *const f32,
    out_l: *mut f32,
    out_r: *mut f32,
    frames: usize,
) {
    if let Some(node) = ptr.as_mut() {
        let left_in = std::slice::from_raw_parts(in_l, frames);
        let right_in = std::slice::from_raw_parts(in_r, frames);
        let left_out = std::slice::from_raw_parts_mut(out_l, frames);
        let right_out = std::slice::from_raw_parts_mut(out_r, frames);

        node.process(&[left_in, right_in], &mut [left_out, right_out]);
    }
}

/// Creates a new Concert Hall Row 10 acoustics node.
#[no_mangle]
pub extern "C" fn concert_hall_create(sample_rate: f32) -> *mut ConcertHallNode {
    Box::into_raw(Box::new(ConcertHallNode::new(sample_rate)))
}

/// Destroys a Concert Hall node.
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn concert_hall_destroy(ptr: *mut ConcertHallNode) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Sets Concert Hall seat row (1 to 30).
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn concert_hall_set_row(ptr: *mut ConcertHallNode, row: f32) {
    if let Some(node) = ptr.as_mut() {
        node.set_parameter(1, row);
    }
}

/// Sets Concert Hall crowd murmur noise gain.
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn concert_hall_set_crowd_gain(ptr: *mut ConcertHallNode, gain: f32) {
    if let Some(node) = ptr.as_mut() {
        node.set_parameter(2, gain);
    }
}

/// Processes stereo audio frames through Concert Hall acoustics.
///
/// # Safety
/// Pointers must point to valid arrays of `frames` length.
#[no_mangle]
pub unsafe extern "C" fn concert_hall_process(
    ptr: *mut ConcertHallNode,
    in_l: *const f32,
    in_r: *const f32,
    out_l: *mut f32,
    out_r: *mut f32,
    frames: usize,
) {
    if let Some(node) = ptr.as_mut() {
        let left_in = std::slice::from_raw_parts(in_l, frames);
        let right_in = std::slice::from_raw_parts(in_r, frames);
        let left_out = std::slice::from_raw_parts_mut(out_l, frames);
        let right_out = std::slice::from_raw_parts_mut(out_r, frames);

        node.process(&[left_in, right_in], &mut [left_out, right_out]);
    }
}

/// Creates a new Next Door through-the-wall node.
#[no_mangle]
pub extern "C" fn next_door_create(sample_rate: f32) -> *mut NextDoorNode {
    Box::into_raw(Box::new(NextDoorNode::new(sample_rate)))
}

/// Destroys a Next Door node.
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn next_door_destroy(ptr: *mut NextDoorNode) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Enables or disables the Next Door effect.
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn next_door_set_enabled(ptr: *mut NextDoorNode, enabled: bool) {
    if let Some(node) = ptr.as_mut() {
        node.set_parameter(0, if enabled { 1.0 } else { 0.0 });
    }
}

/// Sets the transmission low-pass wall cutoff frequency (Hz).
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn next_door_set_cutoff(ptr: *mut NextDoorNode, cutoff_hz: f32) {
    if let Some(node) = ptr.as_mut() {
        node.set_parameter(1, cutoff_hz);
    }
}

/// Sets the structural flanking bass boost (dB).
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn next_door_set_bass_boost(ptr: *mut NextDoorNode, bass_boost_db: f32) {
    if let Some(node) = ptr.as_mut() {
        node.set_parameter(2, bass_boost_db);
    }
}

/// Processes stereo audio frames through Next Door acoustics.
///
/// # Safety
/// Pointers must point to valid arrays of `frames` length.
#[no_mangle]
pub unsafe extern "C" fn next_door_process(
    ptr: *mut NextDoorNode,
    in_l: *const f32,
    in_r: *const f32,
    out_l: *mut f32,
    out_r: *mut f32,
    frames: usize,
) {
    if let Some(node) = ptr.as_mut() {
        let left_in = std::slice::from_raw_parts(in_l, frames);
        let right_in = std::slice::from_raw_parts(in_r, frames);
        let left_out = std::slice::from_raw_parts_mut(out_l, frames);
        let right_out = std::slice::from_raw_parts_mut(out_r, frames);

        node.process(&[left_in, right_in], &mut [left_out, right_out]);
    }
}

/// Computes room Sabine RT60 and first reflection delay as JSON.
#[wasm_bindgen]
pub fn wasm_compute_room_acoustics(
    width_m: f32,
    depth_m: f32,
    height_m: f32,
    wall_mat_id: u32,
    floor_mat_id: u32,
    ceiling_mat_id: u32,
) -> String {
    let parse_mat = |id: u32| match id {
        0 => SurfaceMaterial::Plaster,
        1 => SurfaceMaterial::Concrete,
        2 => SurfaceMaterial::Brick,
        3 => SurfaceMaterial::Wood,
        4 => SurfaceMaterial::Glass,
        5 => SurfaceMaterial::Curtain,
        6 => SurfaceMaterial::Carpet,
        7 => SurfaceMaterial::Stone,
        8 => SurfaceMaterial::Tile,
        _ => SurfaceMaterial::AcousticTile,
    };

    let (rt60, first_refl_ms) = compute_sabine_rt60(
        width_m,
        depth_m,
        height_m,
        parse_mat(wall_mat_id),
        parse_mat(floor_mat_id),
        parse_mat(ceiling_mat_id),
    );

    format!(
        "{{\"rt60\":{:.3},\"first_reflection_ms\":{:.1}}}",
        rt60, first_refl_ms
    )
}

// ----------------------------------------------------------------------------
// Creative FX & Voice Changer C ABI (Phase 5)
// ----------------------------------------------------------------------------

/// Creates a new Slowed + Reverb node.
#[no_mangle]
pub extern "C" fn slowed_reverb_create(sample_rate: f32) -> *mut SlowedReverbNode {
    Box::into_raw(Box::new(SlowedReverbNode::new(sample_rate)))
}

/// Destroys a Slowed + Reverb node.
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn slowed_reverb_destroy(ptr: *mut SlowedReverbNode) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Sets Slowed + Reverb speed ratio (0.5 to 1.0).
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn slowed_reverb_set_speed(ptr: *mut SlowedReverbNode, speed: f32) {
    if let Some(node) = ptr.as_mut() {
        node.set_speed_ratio(speed);
    }
}

/// Processes stereo audio frames through Slowed + Reverb.
///
/// # Safety
/// Pointers must point to valid arrays of `frames` length.
#[no_mangle]
pub unsafe extern "C" fn slowed_reverb_process(
    ptr: *mut SlowedReverbNode,
    in_l: *const f32,
    in_r: *const f32,
    out_l: *mut f32,
    out_r: *mut f32,
    frames: usize,
) {
    if let Some(node) = ptr.as_mut() {
        let left_in = std::slice::from_raw_parts(in_l, frames);
        let right_in = std::slice::from_raw_parts(in_r, frames);
        let left_out = std::slice::from_raw_parts_mut(out_l, frames);
        let right_out = std::slice::from_raw_parts_mut(out_r, frames);

        node.process(&[left_in, right_in], &mut [left_out, right_out]);
    }
}

/// Creates a new Nightcore node.
#[no_mangle]
pub extern "C" fn nightcore_create(sample_rate: f32) -> *mut NightcoreNode {
    Box::into_raw(Box::new(NightcoreNode::new(sample_rate)))
}

/// Destroys a Nightcore node.
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn nightcore_destroy(ptr: *mut NightcoreNode) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Sets Nightcore speed ratio (1.0 to 1.5).
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn nightcore_set_speed(ptr: *mut NightcoreNode, speed: f32) {
    if let Some(node) = ptr.as_mut() {
        node.set_speed_ratio(speed);
    }
}

/// Processes stereo audio frames through Nightcore.
///
/// # Safety
/// Pointers must point to valid arrays of `frames` length.
#[no_mangle]
pub unsafe extern "C" fn nightcore_process(
    ptr: *mut NightcoreNode,
    in_l: *const f32,
    in_r: *const f32,
    out_l: *mut f32,
    out_r: *mut f32,
    frames: usize,
) {
    if let Some(node) = ptr.as_mut() {
        let left_in = std::slice::from_raw_parts(in_l, frames);
        let right_in = std::slice::from_raw_parts(in_r, frames);
        let left_out = std::slice::from_raw_parts_mut(out_l, frames);
        let right_out = std::slice::from_raw_parts_mut(out_r, frames);

        node.process(&[left_in, right_in], &mut [left_out, right_out]);
    }
}

/// Creates a new Lo-Fi Macro node.
#[no_mangle]
pub extern "C" fn lofi_create(sample_rate: f32) -> *mut LofiMacroNode {
    Box::into_raw(Box::new(LofiMacroNode::new(sample_rate)))
}

/// Destroys a Lo-Fi Macro node.
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn lofi_destroy(ptr: *mut LofiMacroNode) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Sets Lo-Fi one-slider macro amount [0.0, 1.0].
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn lofi_set_amount(ptr: *mut LofiMacroNode, amount: f32) {
    if let Some(node) = ptr.as_mut() {
        node.set_macro_amount(amount);
    }
}

/// Processes stereo audio frames through Lo-Fi Macro.
///
/// # Safety
/// Pointers must point to valid arrays of `frames` length.
#[no_mangle]
pub unsafe extern "C" fn lofi_process(
    ptr: *mut LofiMacroNode,
    in_l: *const f32,
    in_r: *const f32,
    out_l: *mut f32,
    out_r: *mut f32,
    frames: usize,
) {
    if let Some(node) = ptr.as_mut() {
        let left_in = std::slice::from_raw_parts(in_l, frames);
        let right_in = std::slice::from_raw_parts(in_r, frames);
        let left_out = std::slice::from_raw_parts_mut(out_l, frames);
        let right_out = std::slice::from_raw_parts_mut(out_r, frames);

        node.process(&[left_in, right_in], &mut [left_out, right_out]);
    }
}

/// Creates a new Vinyl texture node.
#[no_mangle]
pub extern "C" fn vinyl_create(sample_rate: f32) -> *mut VinylNode {
    Box::into_raw(Box::new(VinylNode::new(sample_rate)))
}

/// Destroys a Vinyl node.
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn vinyl_destroy(ptr: *mut VinylNode) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Sets Vinyl PRNG seed for deterministic crackle reproduction.
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn vinyl_set_seed(ptr: *mut VinylNode, seed: u32) {
    if let Some(node) = ptr.as_mut() {
        node.set_seed(seed);
    }
}

/// Processes stereo audio frames through Vinyl texture.
///
/// # Safety
/// Pointers must point to valid arrays of `frames` length.
#[no_mangle]
pub unsafe extern "C" fn vinyl_process(
    ptr: *mut VinylNode,
    in_l: *const f32,
    in_r: *const f32,
    out_l: *mut f32,
    out_r: *mut f32,
    frames: usize,
) {
    if let Some(node) = ptr.as_mut() {
        let left_in = std::slice::from_raw_parts(in_l, frames);
        let right_in = std::slice::from_raw_parts(in_r, frames);
        let left_out = std::slice::from_raw_parts_mut(out_l, frames);
        let right_out = std::slice::from_raw_parts_mut(out_r, frames);

        node.process(&[left_in, right_in], &mut [left_out, right_out]);
    }
}

/// Creates a new Voice Changer node.
#[no_mangle]
pub extern "C" fn voice_changer_create(sample_rate: f32) -> *mut VoiceChangerNode {
    Box::into_raw(Box::new(VoiceChangerNode::new(sample_rate)))
}

/// Destroys a Voice Changer node.
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn voice_changer_destroy(ptr: *mut VoiceChangerNode) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Sets Voice Changer character preset (0=Deep ... 15=Monster).
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn voice_changer_set_character(
    ptr: *mut VoiceChangerNode,
    character_id: u32,
) {
    if let Some(node) = ptr.as_mut() {
        let chars = [
            VoiceCharacter::Deep,
            VoiceCharacter::Chipmunk,
            VoiceCharacter::Child,
            VoiceCharacter::Elder,
            VoiceCharacter::ShiftUp,
            VoiceCharacter::ShiftDown,
            VoiceCharacter::Whisper,
            VoiceCharacter::Announcer,
            VoiceCharacter::RadioDj,
            VoiceCharacter::Villain,
            VoiceCharacter::Giant,
            VoiceCharacter::Tiny,
            VoiceCharacter::Robot,
            VoiceCharacter::Cyborg,
            VoiceCharacter::Alien,
            VoiceCharacter::Monster,
        ];
        let idx = (character_id as usize).min(15);
        node.set_character(chars[idx]);
    }
}

/// Processes stereo audio frames through Voice Changer.
///
/// # Safety
/// Pointers must point to valid arrays of `frames` length.
#[no_mangle]
pub unsafe extern "C" fn voice_changer_process(
    ptr: *mut VoiceChangerNode,
    in_l: *const f32,
    in_r: *const f32,
    out_l: *mut f32,
    out_r: *mut f32,
    frames: usize,
) {
    if let Some(node) = ptr.as_mut() {
        let left_in = std::slice::from_raw_parts(in_l, frames);
        let right_in = std::slice::from_raw_parts(in_r, frames);
        let left_out = std::slice::from_raw_parts_mut(out_l, frames);
        let right_out = std::slice::from_raw_parts_mut(out_r, frames);

        node.process(&[left_in, right_in], &mut [left_out, right_out]);
    }
}

/// Renders a synthesized test tone through the biquad chain.
///
/// # Safety
/// `ptr` must be valid, and `out_ptr` must point to a buffer of length `frames`.
#[no_mangle]
pub unsafe extern "C" fn biquad_render_tone(
    ptr: *mut BiquadChain,
    freq: f32,
    sample_rate: f32,
    out_ptr: *mut f32,
    frames: usize,
) {
    if let Some(chain) = ptr.as_mut() {
        let out_slice = std::slice::from_raw_parts_mut(out_ptr, frames);
        let omega = 2.0 * std::f32::consts::PI * freq / sample_rate;

        // Populate in-place with sine, then filter in-place
        for (i, sample) in out_slice.iter_mut().enumerate().take(frames) {
            *sample = (i as f32 * omega).sin();
        }

        // To avoid allocation in worklet thread, clone input to temp stack buffer or process in-place
        // BiquadChain supports in-place processing when in_slice == out_slice
        let in_slice = std::slice::from_raw_parts(out_ptr, frames);
        let out_slice_mut = std::slice::from_raw_parts_mut(out_ptr, frames);
        chain.process(&[in_slice], &mut [out_slice_mut]);
    }
}

/// Creates a new `StretchNode` instance.
#[no_mangle]
pub extern "C" fn stretch_create(sample_rate: f32) -> *mut StretchNode {
    let mut node = Box::new(StretchNode::new());
    node.prepare(sample_rate, 2);
    Box::into_raw(node)
}

/// Destroys a `StretchNode` instance.
///
/// # Safety
/// `ptr` must be a valid pointer returned by `stretch_create`.
#[no_mangle]
pub unsafe extern "C" fn stretch_destroy(ptr: *mut StretchNode) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Sets the time-stretch playback ratio.
///
/// # Safety
/// `ptr` must be a valid pointer to a `StretchNode`.
#[no_mangle]
pub unsafe extern "C" fn stretch_set_ratio(ptr: *mut StretchNode, ratio: f32) {
    if let Some(node) = ptr.as_mut() {
        node.set_stretch_ratio(ratio);
    }
}

/// Sets the pitch shift in semitones.
///
/// # Safety
/// `ptr` must be a valid pointer to a `StretchNode`.
#[no_mangle]
pub unsafe extern "C" fn stretch_set_pitch(ptr: *mut StretchNode, semitones: f32) {
    if let Some(node) = ptr.as_mut() {
        node.set_pitch_shift(semitones);
    }
}

/// Processes an audio quantum through the time/pitch stretcher.
///
/// # Safety
/// `ptr` must be valid, and `in_ptr` and `out_ptr` must point to valid slices of length `frames`.
#[no_mangle]
pub unsafe extern "C" fn stretch_process_quantum(
    ptr: *mut StretchNode,
    in_ptr: *const f32,
    out_ptr: *mut f32,
    frames: usize,
) {
    if let Some(node) = ptr.as_mut() {
        let in_slice = std::slice::from_raw_parts(in_ptr, frames);
        let out_slice = std::slice::from_raw_parts_mut(out_ptr, frames);
        node.process(&[in_slice], &mut [out_slice]);
    }
}

/// Encodes audio samples to MP3 in WebAssembly.
///
/// # Safety
/// `samples_ptr` must point to `samples_len` floats, and `out_len` must be a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn codec_encode_mp3(
    samples_ptr: *const f32,
    samples_len: usize,
    sample_rate: u32,
    out_len: *mut usize,
) -> *mut u8 {
    let samples = std::slice::from_raw_parts(samples_ptr, samples_len);
    let encoder = Mp3Encoder::new(sample_rate, 2, 128);
    let meta = AudioMetadata::default();
    let mut encoded = encoder.encode(samples, &meta);
    encoded.shrink_to_fit();
    *out_len = encoded.len();
    let ptr = encoded.as_mut_ptr();
    std::mem::forget(encoded);
    ptr
}

/// Encodes audio samples to Ogg Opus in WebAssembly.
///
/// # Safety
/// `samples_ptr` must point to `samples_len` floats, and `out_len` must be a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn codec_encode_opus(
    samples_ptr: *const f32,
    samples_len: usize,
    sample_rate: u32,
    out_len: *mut usize,
) -> *mut u8 {
    let samples = std::slice::from_raw_parts(samples_ptr, samples_len);
    let encoder = OpusEncoder::new(sample_rate, 2);
    let meta = AudioMetadata::default();
    let mut encoded = encoder.encode(samples, &meta);
    encoded.shrink_to_fit();
    *out_len = encoded.len();
    let ptr = encoded.as_mut_ptr();
    std::mem::forget(encoded);
    ptr
}

/// Encodes audio samples to Ogg Vorbis in WebAssembly.
///
/// # Safety
/// `samples_ptr` must point to `samples_len` floats, and `out_len` must be a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn codec_encode_vorbis(
    samples_ptr: *const f32,
    samples_len: usize,
    sample_rate: u32,
    out_len: *mut usize,
) -> *mut u8 {
    let samples = std::slice::from_raw_parts(samples_ptr, samples_len);
    let encoder = VorbisEncoder::new(sample_rate, 2);
    let meta = AudioMetadata::default();
    let mut encoded = encoder.encode(samples, &meta);
    encoded.shrink_to_fit();
    *out_len = encoded.len();
    let ptr = encoded.as_mut_ptr();
    std::mem::forget(encoded);
    ptr
}

/// Frees an encoded byte buffer allocated by `codec_encode_*`.
///
/// # Safety
/// `ptr` must be a valid pointer returned by a codec encode function.
#[no_mangle]
pub unsafe extern "C" fn codec_free_buffer(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        drop(Vec::from_raw_parts(ptr, len, len));
    }
}

/// Decodes in-memory WAV or AIFF audio bytes into interleaved 32-bit floats.
///
/// # Safety
/// `bytes_ptr` must point to `bytes_len` bytes. Out pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn codec_decode_audio(
    bytes_ptr: *const u8,
    bytes_len: usize,
    out_channels: *mut usize,
    out_frames: *mut usize,
    out_sample_rate: *mut u32,
    out_samples_len: *mut usize,
) -> *mut f32 {
    let bytes = std::slice::from_raw_parts(bytes_ptr, bytes_len);
    match flux_engine::decoder::decode_buffer(bytes) {
        Ok(buf) => {
            *out_channels = buf.num_channels();
            *out_frames = buf.num_frames();
            *out_sample_rate = buf.sample_rate;
            let mut interleaved = buf.to_interleaved();
            interleaved.shrink_to_fit();
            *out_samples_len = interleaved.len();
            let ptr = interleaved.as_mut_ptr();
            std::mem::forget(interleaved);
            ptr
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees an interleaved float buffer allocated by `codec_decode_audio`.
///
/// # Safety
/// `ptr` must be a valid pointer returned by `codec_decode_audio` with length `len`.
#[no_mangle]
pub unsafe extern "C" fn codec_free_audio_buffer(ptr: *mut f32, len: usize) {
    if !ptr.is_null() && len > 0 {
        drop(Vec::from_raw_parts(ptr, len, len));
    }
}

/// Resets the biquad chain filter state.
///
/// # Safety
/// `ptr` must be a valid pointer to a `BiquadChain`.
#[no_mangle]
pub unsafe extern "C" fn biquad_reset(ptr: *mut BiquadChain) {
    if let Some(chain) = ptr.as_mut() {
        chain.reset();
    }
}

/// WebAssembly wrapper for input level meter.
#[wasm_bindgen]
pub struct WasmLevelMeter {
    inner: InputLevelMeter,
    last_peaks: Vec<f32>,
    last_rms: Vec<f32>,
    last_clipped: Vec<bool>,
    last_clip_held: Vec<bool>,
}

#[wasm_bindgen]
impl WasmLevelMeter {
    /// Creates a new `WasmLevelMeter`.
    #[wasm_bindgen(constructor)]
    pub fn new(num_channels: usize, sample_rate: u32) -> Self {
        Self {
            inner: InputLevelMeter::new(num_channels, sample_rate),
            last_peaks: vec![-120.0; num_channels],
            last_rms: vec![-120.0; num_channels],
            last_clipped: vec![false; num_channels],
            last_clip_held: vec![false; num_channels],
        }
    }

    /// Processes an interleaved audio buffer of length `frames * channels`.
    pub fn process_interleaved(&mut self, input: &[f32], channels: usize) {
        if channels == 0 || input.is_empty() {
            return;
        }
        let num_frames = input.len() / channels;
        let mut deinterleaved = vec![Vec::with_capacity(num_frames); channels];
        for frame in 0..num_frames {
            let base = frame * channels;
            for ch in 0..channels {
                deinterleaved[ch].push(input[base + ch]);
            }
        }
        let channel_refs: Vec<&[f32]> = deinterleaved.iter().map(|c| c.as_slice()).collect();
        let readings = self.inner.process(&channel_refs);
        for (i, r) in readings.into_iter().enumerate() {
            if i < self.last_peaks.len() {
                self.last_peaks[i] = r.peak_db;
                self.last_rms[i] = r.rms_db;
                self.last_clipped[i] = r.clipped;
                self.last_clip_held[i] = r.clip_held;
            }
        }
    }

    /// Gets peak level in dBFS for channel.
    pub fn get_peak_db(&self, channel: usize) -> f32 {
        self.last_peaks.get(channel).copied().unwrap_or(-120.0)
    }

    /// Gets RMS level in dBFS for channel.
    pub fn get_rms_db(&self, channel: usize) -> f32 {
        self.last_rms.get(channel).copied().unwrap_or(-120.0)
    }

    /// Returns whether channel is currently clipping.
    pub fn is_clipped(&self, channel: usize) -> bool {
        self.last_clipped.get(channel).copied().unwrap_or(false)
    }

    /// Returns whether channel has a held clip indication.
    pub fn is_clip_held(&self, channel: usize) -> bool {
        self.last_clip_held.get(channel).copied().unwrap_or(false)
    }

    /// Clears held clip indicators.
    pub fn clear_clips(&mut self) {
        self.inner.clear_clips();
        self.last_clip_held.fill(false);
    }
}

/// WebAssembly wrapper for pre-roll circular audio buffer.
#[wasm_bindgen]
pub struct WasmPreRollBuffer {
    inner: PreRollBuffer,
    channels: usize,
}

#[wasm_bindgen]
impl WasmPreRollBuffer {
    /// Creates a new `WasmPreRollBuffer`.
    #[wasm_bindgen(constructor)]
    pub fn new(num_channels: usize, sample_rate: u32, duration_seconds: f64) -> Self {
        Self {
            inner: PreRollBuffer::new(num_channels, sample_rate, duration_seconds),
            channels: num_channels,
        }
    }

    /// Appends interleaved frames to pre-roll buffer.
    pub fn append_interleaved(&mut self, input: &[f32]) {
        if self.channels == 0 || input.is_empty() {
            return;
        }
        let num_frames = input.len() / self.channels;
        let mut deinterleaved = vec![Vec::with_capacity(num_frames); self.channels];
        for frame in 0..num_frames {
            let base = frame * self.channels;
            for ch in 0..self.channels {
                deinterleaved[ch].push(input[base + ch]);
            }
        }
        let refs: Vec<&[f32]> = deinterleaved.iter().map(|c| c.as_slice()).collect();
        self.inner.append(&refs);
    }

    /// Drains all buffered pre-roll audio as an interleaved vector.
    pub fn drain_interleaved(&mut self) -> Vec<f32> {
        let planar = self.inner.drain();
        if planar.is_empty() || planar[0].is_empty() {
            return Vec::new();
        }
        let num_frames = planar[0].len();
        let mut interleaved = Vec::with_capacity(num_frames * self.channels);
        for frame in 0..num_frames {
            for ch in planar.iter().take(self.channels) {
                interleaved.push(ch[frame]);
            }
        }
        interleaved
    }

    /// Number of frames currently in pre-roll buffer.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Clears pre-roll buffer.
    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

/// WebAssembly wrapper for crash-safe chunked recording stream.
#[wasm_bindgen]
pub struct WasmChunkedRecordWriter {
    inner: ChunkedRecordWriter,
    channels: usize,
}

#[wasm_bindgen]
impl WasmChunkedRecordWriter {
    /// Creates a new `WasmChunkedRecordWriter`.
    #[wasm_bindgen(constructor)]
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            inner: ChunkedRecordWriter::new(sample_rate, channels),
            channels: channels as usize,
        }
    }

    /// Encodes an interleaved audio block into a self-contained, crash-safe chunk.
    pub fn encode_chunk(&mut self, input: &[f32]) -> Result<Vec<u8>, JsValue> {
        if self.channels == 0 || input.is_empty() {
            return Ok(Vec::new());
        }
        let num_frames = input.len() / self.channels;
        let mut deinterleaved = vec![Vec::with_capacity(num_frames); self.channels];
        for frame in 0..num_frames {
            let base = frame * self.channels;
            for ch in 0..self.channels {
                deinterleaved[ch].push(input[base + ch]);
            }
        }
        let refs: Vec<&[f32]> = deinterleaved.iter().map(|c| c.as_slice()).collect();
        let mut out = Vec::new();
        self.inner
            .write_chunk(&mut out, &refs)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(out)
    }

    /// Recovers all valid audio chunks from a stream buffer, returning interleaved float samples.
    pub fn recover_interleaved(data: &[u8]) -> Result<Vec<f32>, JsValue> {
        let mut cursor = std::io::Cursor::new(data);
        let audio = ChunkedRecordWriter::recover_from_stream(&mut cursor)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let num_channels = audio.channels.len();
        let num_frames = audio.num_frames();
        let mut interleaved = Vec::with_capacity(num_frames * num_channels);

        for frame in 0..num_frames {
            for ch in audio.channels.iter().take(num_channels) {
                interleaved.push(ch[frame]);
            }
        }

        Ok(interleaved)
    }

    /// Converts a stream of chunked recording bytes into a standard WAV file.
    pub fn to_wav(data: &[u8]) -> Result<Vec<u8>, JsValue> {
        let mut cursor = std::io::Cursor::new(data);
        let mut wav_out = std::io::Cursor::new(Vec::new());
        ChunkedRecordWriter::convert_chunks_to_wav(&mut cursor, &mut wav_out)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(wav_out.into_inner())
    }
}

/// WebAssembly wrapper for arbitrary audio resampling.
#[wasm_bindgen]
pub struct WasmResampler {
    inner: Resampler,
}

#[wasm_bindgen]
impl WasmResampler {
    /// Creates a new `WasmResampler`. Quality: 0 = Fast, 1 = High, 2 = Mastering.
    #[wasm_bindgen(constructor)]
    pub fn new(from_rate: u32, to_rate: u32, quality: u32) -> Self {
        let q = match quality {
            0 => ResamplerQuality::Fast,
            2 => ResamplerQuality::Mastering,
            _ => ResamplerQuality::High,
        };
        Self {
            inner: Resampler::new(from_rate, to_rate, q),
        }
    }

    /// Resamples a single planar channel.
    pub fn resample_channel(&self, input: &[f32]) -> Vec<f32> {
        self.inner.resample_channel(input)
    }
}

/// WebAssembly wrapper for audio dither.
#[wasm_bindgen]
pub struct WasmDither {
    inner: Dither,
}

#[wasm_bindgen]
impl WasmDither {
    /// Creates a new `WasmDither`. DitherType: 0 = None, 1 = TPDF, 2 = NoiseShaped. BitDepth: 16 = 16-bit, 24 = 24-bit, 32 = Float32.
    #[wasm_bindgen(constructor)]
    pub fn new(dither_type: u32, bit_depth: u32, channels: usize) -> Self {
        let dt = match dither_type {
            1 => DitherType::Tpdf,
            2 => DitherType::NoiseShaped,
            _ => DitherType::None,
        };
        let bd = match bit_depth {
            16 => BitDepth::Bits16,
            24 => BitDepth::Bits24,
            _ => BitDepth::Float32,
        };
        Self {
            inner: Dither::new(dt, bd, channels),
        }
    }

    /// Processes a single planar channel slice in-place.
    pub fn process_channel(&mut self, samples: &mut [f32]) {
        self.inner.process(&mut [samples]);
    }
}

/// WebAssembly wrapper for playback transport.
#[wasm_bindgen]
pub struct WasmTransport {
    inner: Transport,
}

#[wasm_bindgen]
impl WasmTransport {
    /// Creates a new `WasmTransport`.
    #[wasm_bindgen(constructor)]
    pub fn new(sample_rate: u32) -> Self {
        Self {
            inner: Transport::new(sample_rate),
        }
    }

    /// Plays or resumes.
    pub fn play(&mut self) {
        self.inner.play();
    }

    /// Pauses.
    pub fn pause(&mut self) {
        self.inner.pause();
    }

    /// Stops.
    pub fn stop(&mut self) {
        self.inner.stop();
    }

    /// Seeks to frame.
    pub fn seek(&mut self, frame: u64) {
        self.inner.seek(frame);
    }

    /// Configures loop range.
    pub fn set_loop(&mut self, enabled: bool, start: u64, end: u64) {
        self.inner.set_loop(enabled, start, end);
    }

    /// Sets scrub speed.
    pub fn set_scrub_speed(&mut self, speed: f32) {
        self.inner.set_scrub_speed(speed);
    }

    /// Current playhead position in frames.
    pub fn playhead(&self) -> u64 {
        self.inner.playhead()
    }

    /// Advances playhead by `frames`.
    pub fn advance(&mut self, frames: usize) -> u64 {
        self.inner.advance(frames);
        self.inner.playhead()
    }
}

// ---------------------------------------------------------------------------
// C ABI Real-Time AudioWorklet Exports for EngineHost
// ---------------------------------------------------------------------------

/// Creates a new `EngineHost` instance.
#[no_mangle]
pub extern "C" fn engine_create(sample_rate: f32, quantum_frames: usize) -> *mut EngineHost {
    let host = Box::new(EngineHost::new(sample_rate, quantum_frames));
    Box::into_raw(host)
}

/// Destroys an `EngineHost` instance.
///
/// # Safety
/// `ptr` must be a valid pointer returned by `engine_create`.
#[no_mangle]
pub unsafe extern "C" fn engine_destroy(ptr: *mut EngineHost) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Enqueues Play command.
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn engine_command_play(ptr: *mut EngineHost) {
    if let Some(host) = ptr.as_mut() {
        host.commands.push(EngineCommand::Play);
    }
}

/// Enqueues Pause command.
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn engine_command_pause(ptr: *mut EngineHost) {
    if let Some(host) = ptr.as_mut() {
        host.commands.push(EngineCommand::Pause);
    }
}

/// Enqueues Seek command.
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn engine_command_seek(ptr: *mut EngineHost, frame: u64) {
    if let Some(host) = ptr.as_mut() {
        host.commands.push(EngineCommand::Seek(frame));
    }
}

/// Enqueues SetLoop command.
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn engine_command_set_loop(
    ptr: *mut EngineHost,
    enabled: bool,
    start: u64,
    end: u64,
) {
    if let Some(host) = ptr.as_mut() {
        host.commands.push(EngineCommand::SetLoop {
            enabled,
            start,
            end,
        });
    }
}

/// Enqueues SetScrub command.
///
/// # Safety
/// `ptr` must be valid.
#[no_mangle]
pub unsafe extern "C" fn engine_command_set_scrub(ptr: *mut EngineHost, speed: f32) {
    if let Some(host) = ptr.as_mut() {
        host.commands.push(EngineCommand::SetScrub(speed));
    }
}

/// Processes a quantum render block in real time.
///
/// # Safety
/// `ptr` must be valid; buffer pointers must point to valid slices of length `frames`.
#[no_mangle]
pub unsafe extern "C" fn engine_process_quantum(
    ptr: *mut EngineHost,
    in_l_ptr: *const f32,
    in_r_ptr: *const f32,
    out_l_ptr: *mut f32,
    out_r_ptr: *mut f32,
    frames: usize,
) {
    if let Some(host) = ptr.as_mut() {
        let in_l = std::slice::from_raw_parts(in_l_ptr, frames);
        let in_r = std::slice::from_raw_parts(in_r_ptr, frames);
        let out_l = std::slice::from_raw_parts_mut(out_l_ptr, frames);
        let out_r = std::slice::from_raw_parts_mut(out_r_ptr, frames);

        host.render_quantum(&[in_l, in_r], &mut [out_l, out_r]);
    }
}

// ----------------------------------------------------------------------------
// Audio Export & Format Conversion WASM APIs
// ----------------------------------------------------------------------------

/// JSON payload structure for export parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmExportConfig {
    /// Target format identifier (e.g. "wav_16", "wav_24", "wav_32f", "flac_16", "flac_24", "mp3_128", "mp3_320", "aac_128", "opus_128", "vorbis_q6").
    pub format: Option<String>,
    /// Optional preset identifier (e.g. "streaming", "podcast_stereo", "podcast_mono", "youtube", "cd_audio", "archival").
    pub preset: Option<String>,
    /// Target sample rate (Hz), or None to preserve source rate.
    pub sample_rate: Option<u32>,
    /// Channel mode ("keep", "mono", "stereo").
    pub channel_mode: Option<String>,
    /// Region start frame.
    pub region_start: Option<usize>,
    /// Region end frame.
    pub region_end: Option<usize>,
    /// Tail decay frames to append.
    pub tail_frames: Option<usize>,
    /// Fade-out frames at the end.
    pub fade_out_frames: Option<usize>,
    /// Normalization mode ("off", "peak:-0.3", "lufs:-14.0,-1.0").
    pub normalization: Option<String>,
    /// Dither type ("none", "tpdf", "noise_shaped").
    pub dither: Option<String>,
    /// Track title.
    pub title: Option<String>,
    /// Artist name.
    pub artist: Option<String>,
    /// Album title.
    pub album: Option<String>,
    /// Release year or date.
    pub date: Option<String>,
    /// Musical genre.
    pub genre: Option<String>,
    /// Track number.
    pub track_number: Option<u32>,
    /// Free-form comment.
    pub comment: Option<String>,
    /// Force RF64 chunk header for WAV.
    pub force_rf64: Option<bool>,
}

/// JSON payload structure for preset descriptions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPresetInfo {
    /// Preset ID key.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Target sample rate (Hz).
    pub sample_rate: Option<u32>,
    /// Bit depth.
    pub bit_depth: Option<u8>,
    /// Target LUFS if applicable.
    pub target_lufs: Option<f32>,
    /// True peak ceiling in dBTP if applicable.
    pub true_peak_ceiling: Option<f32>,
    /// Target peak in dBFS if applicable.
    pub target_peak_dbfs: Option<f32>,
    /// Recommended file extension.
    pub file_extension: String,
}

/// Returns all export presets formatted as a JSON array string.
#[wasm_bindgen]
pub fn wasm_get_export_presets() -> String {
    let presets: Vec<WasmPresetInfo> = ExportPresetId::all()
        .iter()
        .map(|id| {
            let cfg = id.config();
            let (target_lufs, true_peak_ceiling, target_peak_dbfs) = match cfg.normalization {
                NormalizationMode::Lufs {
                    target_lufs,
                    true_peak_ceiling_dbtp,
                } => (Some(target_lufs), Some(true_peak_ceiling_dbtp), None),
                NormalizationMode::Peak { target_dbfs } => (None, None, Some(target_dbfs)),
                NormalizationMode::Off => (None, None, None),
            };

            let id_str = match id {
                ExportPresetId::Streaming => "streaming",
                ExportPresetId::PodcastStereo => "podcast_stereo",
                ExportPresetId::PodcastMono => "podcast_mono",
                ExportPresetId::YouTube => "youtube",
                ExportPresetId::CdAudio => "cd_audio",
                ExportPresetId::Archival => "archival",
            };

            WasmPresetInfo {
                id: id_str.to_string(),
                name: cfg.name.to_string(),
                description: cfg.description.to_string(),
                sample_rate: cfg.sample_rate,
                bit_depth: cfg.bit_depth,
                target_lufs,
                true_peak_ceiling,
                target_peak_dbfs,
                file_extension: cfg.file_extension.to_string(),
            }
        })
        .collect();

    serde_json::to_string(&presets).unwrap_or_else(|_| "[]".to_string())
}

/// Encodes an interleaved audio buffer to the requested format and destination configuration.
#[wasm_bindgen]
pub fn wasm_export_audio(
    samples: &[f32],
    num_channels: usize,
    sample_rate: u32,
    config_json: &str,
) -> Result<Vec<u8>, JsValue> {
    if num_channels == 0 || samples.is_empty() {
        return Err(JsValue::from_str("Invalid audio buffer parameters"));
    }

    let cfg: WasmExportConfig = serde_json::from_str(config_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid config JSON: {}", e)))?;

    let buffer = AudioBuffer::from_interleaved(samples, num_channels, sample_rate);

    // 1. Resolve preset defaults if specified
    let mut resolved_opts = ExportOptions::default();

    if let Some(preset_name) = &cfg.preset {
        let preset_id = match preset_name.to_lowercase().as_str() {
            "streaming" => Some(ExportPresetId::Streaming),
            "podcast_stereo" | "podcaststereo" => Some(ExportPresetId::PodcastStereo),
            "podcast_mono" | "podcastmono" => Some(ExportPresetId::PodcastMono),
            "youtube" => Some(ExportPresetId::YouTube),
            "cd_audio" | "cdaudio" => Some(ExportPresetId::CdAudio),
            "archival" => Some(ExportPresetId::Archival),
            _ => None,
        };

        if let Some(p_id) = preset_id {
            let p_cfg = p_id.config();
            resolved_opts.sample_rate = p_cfg.sample_rate;
            resolved_opts.dither = p_cfg.dither;
            resolved_opts.normalization = p_cfg.normalization;
            if p_cfg.force_mono {
                resolved_opts.channel_mode = ExportChannelMode::ForceMono;
            }
            if p_id == ExportPresetId::Archival {
                resolved_opts.format = ExportTargetFormat::Flac(FlacCompressionLevel::Level5, 24);
            } else if p_id == ExportPresetId::CdAudio {
                resolved_opts.format = ExportTargetFormat::Wav(WavBitsPerSample::Bits16);
            }
        }
    }

    // 2. Format override or explicit format
    if let Some(fmt_str) = &cfg.format {
        resolved_opts.format = parse_export_format(fmt_str);
    }

    // 3. Explicit options overrides
    if let Some(sr) = cfg.sample_rate {
        resolved_opts.sample_rate = Some(sr);
    }

    if let Some(cm) = &cfg.channel_mode {
        resolved_opts.channel_mode = match cm.to_lowercase().as_str() {
            "mono" => ExportChannelMode::ForceMono,
            "stereo" => ExportChannelMode::ForceStereo,
            _ => ExportChannelMode::Keep,
        };
    }

    if let (Some(start), Some(end)) = (cfg.region_start, cfg.region_end) {
        resolved_opts.region = Some(start..end);
    }

    resolved_opts.tail_frames = cfg.tail_frames.unwrap_or(0);
    resolved_opts.fade_out_frames = cfg.fade_out_frames.unwrap_or(0);

    if let Some(norm_str) = &cfg.normalization {
        resolved_opts.normalization = parse_normalization_mode(norm_str);
    }

    if let Some(dit_str) = &cfg.dither {
        resolved_opts.dither = match dit_str.to_lowercase().as_str() {
            "tpdf" => DitherType::Tpdf,
            "noise_shaped" | "noiseshaped" => DitherType::NoiseShaped,
            _ => DitherType::None,
        };
    }

    resolved_opts.force_rf64 = cfg.force_rf64.unwrap_or(false);

    // Build metadata container
    let mut meta = ContainerMetadata::default();
    let mut has_meta = false;
    if let Some(t) = cfg.title {
        meta.title = Some(t);
        has_meta = true;
    }
    if let Some(a) = cfg.artist {
        meta.artist = Some(a);
        has_meta = true;
    }
    if let Some(al) = cfg.album {
        meta.album = Some(al);
        has_meta = true;
    }
    if let Some(d) = cfg.date {
        meta.date = Some(d);
        has_meta = true;
    }
    if let Some(g) = cfg.genre {
        meta.genre = Some(g);
        has_meta = true;
    }
    if let Some(tr) = cfg.track_number {
        meta.track_number = Some(tr);
        has_meta = true;
    }
    if let Some(c) = cfg.comment {
        meta.comment = Some(c);
        has_meta = true;
    }
    if has_meta {
        resolved_opts.metadata = Some(meta);
    }

    // Run export job
    ExportJob::run(&buffer, &resolved_opts, None, |_| {})
        .map_err(|e| JsValue::from_str(&format!("Export failed: {:?}", e)))
}

/// Converts audio file bytes into the target audio format.
#[wasm_bindgen]
pub fn wasm_convert_audio_format(
    input_bytes: &[u8],
    config_json: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut decoder = open_decoder(Cursor::new(input_bytes.to_vec()))
        .map_err(|e| JsValue::from_str(&format!("Decode failed: {:?}", e)))?;

    let buffer = decoder
        .decode_all()
        .map_err(|e| JsValue::from_str(&format!("Decode failed: {:?}", e)))?;
    let orig_meta = Some(decoder.info().metadata.clone());

    let cfg: WasmExportConfig = serde_json::from_str(config_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid config JSON: {}", e)))?;

    let target_format = cfg
        .format
        .as_deref()
        .map(parse_export_format)
        .unwrap_or(ExportTargetFormat::Wav(WavBitsPerSample::Bits16));

    let channel_mode = match cfg.channel_mode.as_deref().unwrap_or("keep") {
        "mono" => ExportChannelMode::ForceMono,
        "stereo" => ExportChannelMode::ForceStereo,
        _ => ExportChannelMode::Keep,
    };

    let norm = cfg
        .normalization
        .as_deref()
        .map(parse_normalization_mode)
        .unwrap_or(NormalizationMode::Off);

    let dither = match cfg.dither.as_deref().unwrap_or("none") {
        "tpdf" => DitherType::Tpdf,
        "noise_shaped" | "noiseshaped" => DitherType::NoiseShaped,
        _ => DitherType::None,
    };

    let convert_opts = FormatConvertOptions {
        target_format,
        target_sample_rate: cfg.sample_rate,
        target_channel_mode: channel_mode,
        override_metadata: None,
        normalization: norm,
        dither,
    };

    FormatConverter::convert(&buffer, orig_meta.as_ref(), &convert_opts, None, |_| {})
        .map_err(|e| JsValue::from_str(&format!("Format conversion failed: {:?}", e)))
}

fn parse_export_format(fmt_str: &str) -> ExportTargetFormat {
    match fmt_str.to_lowercase().as_str() {
        "wav_24" | "wav24" => ExportTargetFormat::Wav(WavBitsPerSample::Bits24),
        "wav_32f" | "wav_float" | "wav32f" => ExportTargetFormat::Wav(WavBitsPerSample::Float32),
        "flac_16" | "flac16" => ExportTargetFormat::Flac(FlacCompressionLevel::Level5, 16),
        "flac_24" | "flac24" | "flac" => ExportTargetFormat::Flac(FlacCompressionLevel::Level5, 24),
        "mp3_192" => ExportTargetFormat::Mp3(Mp3Bitrate::Cbr192),
        "mp3_256" => ExportTargetFormat::Mp3(Mp3Bitrate::Cbr256),
        "mp3_320" => ExportTargetFormat::Mp3(Mp3Bitrate::Cbr320),
        "mp3_v0" => ExportTargetFormat::Mp3(Mp3Bitrate::VbrV0),
        "mp3" | "mp3_128" => ExportTargetFormat::Mp3(Mp3Bitrate::Cbr128),
        "aac_96" => ExportTargetFormat::Aac(AacBitrate::Kbps96),
        "aac_192" => ExportTargetFormat::Aac(AacBitrate::Kbps192),
        "aac_256" => ExportTargetFormat::Aac(AacBitrate::Kbps256),
        "aac" | "aac_128" => ExportTargetFormat::Aac(AacBitrate::Kbps128),
        "vorbis_q3" => ExportTargetFormat::Vorbis(VorbisQuality::Q3),
        "vorbis_q9" => ExportTargetFormat::Vorbis(VorbisQuality::Q9),
        "vorbis" | "vorbis_q6" => ExportTargetFormat::Vorbis(VorbisQuality::Q6),
        "opus_voice" | "opus_voip" => ExportTargetFormat::Opus(OpusApplication::Voip, 64),
        "opus_64" => ExportTargetFormat::Opus(OpusApplication::Audio, 64),
        "opus_192" => ExportTargetFormat::Opus(OpusApplication::Audio, 192),
        "opus" | "opus_128" => ExportTargetFormat::Opus(OpusApplication::Audio, 128),
        _ => ExportTargetFormat::Wav(WavBitsPerSample::Bits16),
    }
}

fn parse_normalization_mode(norm_str: &str) -> NormalizationMode {
    let lower = norm_str.to_lowercase();
    if lower.starts_with("peak") {
        let parts: Vec<&str> = lower.split(':').collect();
        let target = parts
            .get(1)
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(-0.3);
        NormalizationMode::Peak {
            target_dbfs: target,
        }
    } else if lower.starts_with("lufs") {
        let parts: Vec<&str> = lower.split(':').collect();
        if let Some(sub) = parts.get(1) {
            let vals: Vec<&str> = sub.split(',').collect();
            let target_lufs = vals
                .first()
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(-14.0);
            let tp = vals
                .get(1)
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(-1.0);
            NormalizationMode::Lufs {
                target_lufs,
                true_peak_ceiling_dbtp: tp,
            }
        } else {
            NormalizationMode::Lufs {
                target_lufs: -14.0,
                true_peak_ceiling_dbtp: -1.0,
            }
        }
    } else {
        NormalizationMode::Off
    }
}

// ----------------------------------------------------------------------------
// Audio Analysis Foundations WASM APIs (Pyramid, Spectrogram, Loudness, Stereo)
// ----------------------------------------------------------------------------

/// JSON payload structure for loudness measurement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmLoudnessResult {
    /// Integrated loudness in LUFS / LKFS.
    pub integrated_lufs: f32,
    /// Maximum momentary loudness (400 ms window) in LUFS.
    pub max_momentary_lufs: f32,
    /// Maximum short-term loudness (3000 ms window) in LUFS.
    pub max_short_term_lufs: f32,
    /// Loudness range in LU (EBU Tech 3342).
    pub loudness_range_lu: f32,
    /// Maximum True Peak level in dBTP.
    pub max_true_peak_dbtp: f32,
    /// Linear True Peak amplitude.
    pub max_true_peak_linear: f32,
}

/// JSON payload structure for stereo spatial analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmStereoResult {
    /// Phase correlation in [-1.0, +1.0].
    pub phase_correlation: f32,
    /// Stereo width ratio RMS(Side) / RMS(Mid).
    pub stereo_width: f32,
    /// Mid channel RMS amplitude.
    pub mid_rms: f32,
    /// Side channel RMS amplitude.
    pub side_rms: f32,
    /// Decimated goniometer points [x0, y0, x1, y1, ...].
    pub goniometer_coords: Vec<f32>,
}

/// Builds a multi-resolution peak pyramid and returns serialized bytes (`FPYR`).
#[wasm_bindgen]
pub fn wasm_build_peak_pyramid(
    samples: &[f32],
    num_channels: usize,
    base_bin_size: usize,
) -> Result<Vec<u8>, JsValue> {
    if num_channels == 0 || samples.is_empty() {
        return Err(JsValue::from_str("Invalid samples or channel count"));
    }

    let frames = samples.len() / num_channels;
    let mut channel_vecs = vec![Vec::with_capacity(frames); num_channels];
    for f in 0..frames {
        for c in 0..num_channels {
            channel_vecs[c].push(samples[f * num_channels + c]);
        }
    }

    let slices: Vec<&[f32]> = channel_vecs.iter().map(|c| c.as_slice()).collect();
    let pyramid = PeakPyramid::build(&slices, base_bin_size);
    Ok(pyramid.to_bytes())
}

/// Queries a timeline range from serialized pyramid bytes, returning packed `[min0, max0, rms0, min1, max1, rms1, ...]`.
#[wasm_bindgen]
pub fn wasm_query_peak_pyramid(
    pyramid_bytes: &[u8],
    channel: usize,
    start_frame: usize,
    end_frame: usize,
    max_bins: usize,
) -> Result<Vec<f32>, JsValue> {
    let pyramid = PeakPyramid::from_bytes(pyramid_bytes)
        .map_err(|e| JsValue::from_str(&format!("Pyramid deserialize failed: {}", e)))?;

    let bins = pyramid.query_range(channel, start_frame, end_frame, max_bins);
    let mut out = Vec::with_capacity(bins.len() * 3);
    for b in bins {
        out.push(b.min);
        out.push(b.max);
        out.push(b.rms);
    }

    Ok(out)
}

/// JSON configuration for spectrogram tile generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmSpectrogramConfig {
    /// FFT window size in samples.
    pub window_size: Option<usize>,
    /// Hop size in samples.
    pub hop_size: Option<usize>,
    /// Number of frequency bins.
    pub num_bins: Option<usize>,
    /// Scale mapping ("linear", "logarithmic", "mel").
    pub scale: Option<String>,
    /// Colormap name ("magma", "viridis", "inferno").
    pub colormap: Option<String>,
    /// Dynamic range minimum floor in dBFS.
    pub min_dbfs: Option<f32>,
    /// Dynamic range ceiling in dBFS.
    pub max_dbfs: Option<f32>,
}

/// Generates a 2D spectrogram tile and returns packed RGBA pixel bytes.
#[wasm_bindgen]
pub fn wasm_generate_spectrogram_tile(
    samples: &[f32],
    sample_rate: u32,
    config_json: &str,
) -> Result<Vec<u8>, JsValue> {
    if samples.is_empty() {
        return Err(JsValue::from_str("Input signal is empty"));
    }

    let parsed_cfg: WasmSpectrogramConfig = if config_json.trim().is_empty() {
        WasmSpectrogramConfig {
            window_size: None,
            hop_size: None,
            num_bins: None,
            scale: None,
            colormap: None,
            min_dbfs: None,
            max_dbfs: None,
        }
    } else {
        serde_json::from_str(config_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid config JSON: {}", e)))?
    };

    let scale = match parsed_cfg
        .scale
        .as_deref()
        .unwrap_or("linear")
        .to_lowercase()
        .as_str()
    {
        "log" | "logarithmic" => FrequencyScale::Logarithmic,
        "mel" => FrequencyScale::Mel,
        _ => FrequencyScale::Linear,
    };

    let colormap = match parsed_cfg
        .colormap
        .as_deref()
        .unwrap_or("magma")
        .to_lowercase()
        .as_str()
    {
        "viridis" => ColorMap::Viridis,
        "inferno" => ColorMap::Inferno,
        _ => ColorMap::Magma,
    };

    let cfg = SpectrogramConfig {
        window_size: parsed_cfg.window_size.unwrap_or(1024),
        hop_size: parsed_cfg.hop_size.unwrap_or(512),
        scale,
        num_freq_bins: parsed_cfg.num_bins.unwrap_or(256),
        min_dbfs: parsed_cfg.min_dbfs.unwrap_or(-100.0),
        max_dbfs: parsed_cfg.max_dbfs.unwrap_or(0.0),
        colormap,
    };

    let tile = SpectrogramTileGenerator::generate_tile(samples, sample_rate, &cfg);
    Ok(tile.rgba_pixels)
}

/// Measures ITU-R BS.1770-4 / EBU R128 integrated loudness, momentary, short-term, LRA, and true peak.
#[wasm_bindgen]
pub fn wasm_measure_loudness(
    samples: &[f32],
    num_channels: usize,
    sample_rate: u32,
) -> Result<String, JsValue> {
    if num_channels == 0 || samples.is_empty() {
        return Err(JsValue::from_str("Invalid audio buffer parameters"));
    }

    let frames = samples.len() / num_channels;
    let mut channel_vecs = vec![Vec::with_capacity(frames); num_channels];
    for f in 0..frames {
        for c in 0..num_channels {
            channel_vecs[c].push(samples[f * num_channels + c]);
        }
    }

    let slices: Vec<&[f32]> = channel_vecs.iter().map(|c| c.as_slice()).collect();
    let meas = LoudnessMeter::analyze(&slices, sample_rate);

    let res = WasmLoudnessResult {
        integrated_lufs: meas.integrated_lufs,
        max_momentary_lufs: meas.max_momentary_lufs,
        max_short_term_lufs: meas.max_short_term_lufs,
        loudness_range_lu: meas.loudness_range_lu,
        max_true_peak_dbtp: meas.max_true_peak_dbtp,
        max_true_peak_linear: meas.max_true_peak_linear,
    };

    serde_json::to_string(&res).map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Computes stereo phase correlation, stereo width ratio, mid/side RMS, and goniometer vectorscope coordinates.
#[wasm_bindgen]
pub fn wasm_analyze_stereo(
    left: &[f32],
    right: &[f32],
    max_goniometer_points: usize,
) -> Result<String, JsValue> {
    let result = StereoAnalyzer::analyze(left, right, max_goniometer_points);

    let mut coords = Vec::with_capacity(result.goniometer_points.len() * 2);
    for pt in result.goniometer_points {
        coords.push(pt.x);
        coords.push(pt.y);
    }

    let res = WasmStereoResult {
        phase_correlation: result.phase_correlation,
        stereo_width: result.stereo_width,
        mid_rms: result.mid_rms,
        side_rms: result.side_rms,
        goniometer_coords: coords,
    };

    serde_json::to_string(&res).map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Returns the full list of available ML models in JSON format.
#[wasm_bindgen]
pub fn wasm_get_models_json() -> String {
    let mm = ModelManager::new();
    serde_json::to_string(mm.models()).unwrap_or_else(|_| "[]".into())
}

/// Classifies an audio buffer returning JSON `{ "category": "speech", "confidence": 0.92 }`.
#[wasm_bindgen]
pub fn wasm_classify_audio(samples: &[f32], sample_rate: f32) -> String {
    let (cat, conf) = AudioClassifier::classify(samples, sample_rate);
    let name = match cat {
        AudioCategory::Speech => "speech",
        AudioCategory::Music => "music",
        AudioCategory::Mixed => "mixed",
        AudioCategory::Noise => "noise",
    };
    serde_json::json!({
        "category": name,
        "confidence": conf
    })
    .to_string()
}

/// Processes audio with Voice Cleanup ("Studio Sound") returning cleaned samples.
#[wasm_bindgen]
pub fn wasm_voice_cleanup(samples: &[f32], sample_rate: f32, studio_amount: f32) -> Vec<f32> {
    let config = VoiceCleanupConfig {
        studio_sound_amount: studio_amount,
        ..Default::default()
    };
    let (out, _) = VoiceCleanupEngine::process(samples, sample_rate, &config);
    out
}

/// Returns the prototype transcript dialogue document as JSON.
#[wasm_bindgen]
pub fn wasm_get_sample_transcript_json() -> String {
    let doc = TranscriptDocument::create_sample_dialogue();
    serde_json::to_string_pretty(&doc).unwrap_or_else(|_| "{}".into())
}

/// Exports a transcript document to formatted subtitles (0 = SRT, 1 = VTT, 2 = TXT, 3 = JSON).
#[wasm_bindgen]
pub fn wasm_export_subtitles(transcript_json: &str, format_id: u32) -> String {
    let doc: TranscriptDocument = match serde_json::from_str(transcript_json) {
        Ok(d) => d,
        Err(_) => TranscriptDocument::create_sample_dialogue(),
    };
    let fmt = match format_id {
        1 => SubtitleFormat::Vtt,
        2 => SubtitleFormat::Txt,
        3 => SubtitleFormat::Json,
        _ => SubtitleFormat::Srt,
    };
    doc.export_subtitles(fmt)
}

/// Returns the prototype filler detections as JSON.
#[wasm_bindgen]
pub fn wasm_get_sample_fillers_json() -> String {
    let items = FillerDetector::get_sample_detections();
    serde_json::to_string(&items).unwrap_or_else(|_| "[]".into())
}

/// Returns the active cloud & background job queue as JSON.
#[wasm_bindgen]
pub fn wasm_list_cloud_jobs_json() -> String {
    let client = MockCloudClient::new();
    serde_json::to_string(client.list_jobs()).unwrap_or_else(|_| "[]".into())
}

/// Analyzes audio and returns musical key and tempo BPM estimation as JSON.
#[wasm_bindgen]
pub fn wasm_detect_tempo_and_key_json(samples: &[f32], sample_rate: f32) -> String {
    let tempo = flux_ml::TempoDetector::detect(samples, sample_rate);
    let key = flux_ml::KeyDetector::detect(samples, sample_rate);
    serde_json::json!({
        "tempo": tempo,
        "key": key
    })
    .to_string()
}

/// Applies auto-mastering and returns the mastered audio samples.
#[wasm_bindgen]
pub fn wasm_auto_master(
    samples: &[f32],
    sample_rate: f32,
    target_mode: u32,
    intensity_mode: u32,
) -> Vec<f32> {
    let target = match target_mode {
        1 => flux_ml::MasteringTarget::Podcast,
        2 => flux_ml::MasteringTarget::Club,
        _ => flux_ml::MasteringTarget::Streaming,
    };
    let intensity = match intensity_mode {
        0 => flux_ml::MasteringIntensity::Gentle,
        2 => flux_ml::MasteringIntensity::Loud,
        _ => flux_ml::MasteringIntensity::Balanced,
    };
    let config = flux_ml::AutoMasteringConfig {
        target,
        intensity,
        preserve_dynamics: true,
        speech_mode: false,
    };
    let (out, _) = flux_ml::AutoMasteringEngine::process(samples, sample_rate, &config);
    out
}

/// Synthesizes text to speech with voice selection and returns audio samples.
#[wasm_bindgen]
pub fn wasm_synthesize_tts(text: &str, voice_id: u32, sample_rate: f32) -> Vec<f32> {
    let voice = match voice_id {
        1 => flux_ml::TtsVoice::Reed,
        2 => flux_ml::TtsVoice::Marin,
        3 => flux_ml::TtsVoice::Oren,
        _ => flux_ml::TtsVoice::Ash,
    };
    let req = flux_ml::TtsRequest {
        script: text.to_string(),
        language: "English (US)".into(),
        voice,
        rate: 1.0,
        pitch_semitones: 0.0,
        use_cloud: false,
    };
    let res = flux_ml::TtsEngine::synthesize(&req, sample_rate);
    res.samples
}

/// Returns the prototype dubbing project data with duration fitting as JSON.
#[wasm_bindgen]
pub fn wasm_get_dubbing_project_json() -> String {
    let proj = flux_ml::DubbingProject::create_sample();
    serde_json::to_string(&proj).unwrap_or_else(|_| "{}".into())
}

/// Diagnoses audio and proposes an editable Magic chain as JSON.
#[wasm_bindgen]
pub fn wasm_diagnose_and_propose_magic_json(
    samples: &[f32],
    sample_rate: f32,
    goal_id: u32,
) -> String {
    let diagnosis = flux_ml::MagicAssistant::diagnose(samples, sample_rate);
    let goal = match goal_id {
        1 => flux_ml::MagicGoal::MakeItLoud,
        2 => flux_ml::MagicGoal::Dreamy,
        3 => flux_ml::MagicGoal::Spatial8D,
        4 => flux_ml::MagicGoal::Podcast,
        5 => flux_ml::MagicGoal::Music,
        _ => flux_ml::MagicGoal::MakeItClear,
    };
    let chain = flux_ml::MagicAssistant::propose_chain(goal);
    serde_json::json!({
        "diagnosis": diagnosis,
        "goal": goal.display_name(),
        "chain": chain
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(flux_version(), "0.1.0");
    }

    #[test]
    fn test_wasm_biquad_wrapper() {
        let mut chain = WasmBiquadChain::new(48000.0, 4);
        chain.configure_peaking(0, 1000.0, 1.0, 6.0);
        let in_buf = vec![0.5f32; 128];
        let mut out_buf = vec![0.0f32; 128];
        chain.process_block(&in_buf, &mut out_buf);
        assert!(!out_buf[127].is_nan());
    }

    #[test]
    fn test_c_abi_biquad_quantum() {
        unsafe {
            let chain = biquad_create(48000.0, 4);
            assert!(!chain.is_null());
            biquad_configure_peaking(chain, 0, 1000.0, 1.0, 3.0);

            let in_buf = biquad_alloc_buffer(128);
            let out_buf = biquad_alloc_buffer(128);
            assert!(!in_buf.is_null());
            assert!(!out_buf.is_null());

            for i in 0..128 {
                *in_buf.add(i) = 0.5;
            }

            biquad_process_quantum(chain, in_buf, out_buf, 128);
            assert!(!(*out_buf.add(127)).is_nan());

            biquad_free_buffer(in_buf, 128);
            biquad_free_buffer(out_buf, 128);
            biquad_destroy(chain);
        }
    }

    #[test]
    fn test_c_abi_binaural_quantum() {
        unsafe {
            let node = binaural_create(48000.0);
            assert!(!node.is_null());
            binaural_set_azimuth(node, 45.0);
            binaural_set_orbit_speed(node, 0.2);

            let in_buf = biquad_alloc_buffer(128);
            let out_l = biquad_alloc_buffer(128);
            let out_r = biquad_alloc_buffer(128);

            for i in 0..128 {
                *in_buf.add(i) = 0.5;
            }

            binaural_process_quantum(node, in_buf, out_l, out_r, 128);
            assert!(!(*out_l.add(127)).is_nan());
            assert!(!(*out_r.add(127)).is_nan());

            biquad_free_buffer(in_buf, 128);
            biquad_free_buffer(out_l, 128);
            biquad_free_buffer(out_r, 128);
            binaural_destroy(node);
        }
    }

    #[test]
    fn test_c_abi_stereo_spatializer() {
        unsafe {
            let node = stereo_spatializer_create(48000.0);
            assert!(!node.is_null());
            stereo_spatializer_set_mode(node, 0); // VirtualSpeakers
            stereo_spatializer_set_azimuth(node, 30.0);
            stereo_spatializer_set_spread(node, 60.0);
            stereo_spatializer_set_mono_bass(node, true, 120.0);

            let in_l = biquad_alloc_buffer(128);
            let in_r = biquad_alloc_buffer(128);
            let out_l = biquad_alloc_buffer(128);
            let out_r = biquad_alloc_buffer(128);

            for i in 0..128 {
                *in_l.add(i) = 0.5;
                *in_r.add(i) = -0.5;
            }

            stereo_spatializer_process(node, in_l, in_r, out_l, out_r, 128);
            assert!(!(*out_l.add(127)).is_nan());
            assert!(!(*out_r.add(127)).is_nan());

            // Switch to Mid/Side
            stereo_spatializer_set_mode(node, 1);
            stereo_spatializer_process(node, in_l, in_r, out_l, out_r, 128);
            assert!(!(*out_l.add(127)).is_nan());
            assert!(!(*out_r.add(127)).is_nan());

            // Test trajectory controls
            stereo_spatializer_set_trajectory_mode(node, 1); // Orbit
            stereo_spatializer_set_trajectory_direction(node, 0); // Clockwise
            stereo_spatializer_set_orbit_speed(node, 0.2);
            stereo_spatializer_set_trajectory_depth(node, 0.8);
            stereo_spatializer_set_trajectory_smoothing(node, 0.5);
            stereo_spatializer_set_tempo_sync(node, true, 120.0, 8.0);
            stereo_spatializer_process(node, in_l, in_r, out_l, out_r, 128);

            // Test presets
            stereo_spatializer_apply_preset(node, 0); // Front
            stereo_spatializer_apply_preset(node, 8); // InsideRoom

            // Test mono compatibility check
            let mut corr = 0.0f32;
            let mut loss = 0.0f32;
            let ok = stereo_check_mono_compatibility(in_l, in_r, 128, &mut corr, &mut loss);
            assert!(!ok); // Anti-phase input should fail mono check
            assert!((corr - (-1.0)).abs() < 1e-3);

            biquad_free_buffer(in_l, 128);
            biquad_free_buffer(in_r, 128);
            biquad_free_buffer(out_l, 128);
            biquad_free_buffer(out_r, 128);
            stereo_spatializer_destroy(node);
        }
    }

    #[test]
    fn test_c_abi_stretch_quantum() {
        unsafe {
            let node = stretch_create(48000.0);
            assert!(!node.is_null());
            stretch_set_ratio(node, 1.25);
            stretch_set_pitch(node, 3.0);

            let in_buf = biquad_alloc_buffer(128);
            let out_buf = biquad_alloc_buffer(128);

            for i in 0..128 {
                *in_buf.add(i) = 0.5;
            }

            stretch_process_quantum(node, in_buf, out_buf, 128);
            assert!(!(*out_buf.add(127)).is_nan());

            biquad_free_buffer(in_buf, 128);
            biquad_free_buffer(out_buf, 128);
            stretch_destroy(node);
        }
    }

    #[test]
    fn test_c_abi_codec_encoders() {
        unsafe {
            let samples = vec![0.5f32; 2304];
            let mut out_len = 0usize;

            // MP3
            let mp3_ptr = codec_encode_mp3(samples.as_ptr(), samples.len(), 48000, &mut out_len);
            assert!(!mp3_ptr.is_null());
            assert!(out_len > 100);
            codec_free_buffer(mp3_ptr, out_len);

            // Opus
            let opus_ptr = codec_encode_opus(samples.as_ptr(), samples.len(), 48000, &mut out_len);
            assert!(!opus_ptr.is_null());
            assert!(out_len > 100);
            codec_free_buffer(opus_ptr, out_len);

            // Vorbis
            let vorbis_ptr =
                codec_encode_vorbis(samples.as_ptr(), samples.len(), 48000, &mut out_len);
            assert!(!vorbis_ptr.is_null());
            assert!(out_len > 100);
            codec_free_buffer(vorbis_ptr, out_len);

            // WAV Decode via C ABI
            let ch0 = vec![0.5f32; 100];
            let ch1 = vec![-0.5f32; 100];
            let wav_bytes = flux_engine::decoder::WavWriter::encode(
                &[&ch0, &ch1],
                48000,
                flux_engine::decoder::WavEncoding::Float32,
                None,
                false,
            )
            .unwrap();

            let mut out_channels = 0usize;
            let mut out_frames = 0usize;
            let mut out_sr = 0u32;
            let mut out_samples_len = 0usize;

            let audio_ptr = codec_decode_audio(
                wav_bytes.as_ptr(),
                wav_bytes.len(),
                &mut out_channels,
                &mut out_frames,
                &mut out_sr,
                &mut out_samples_len,
            );
            assert!(!audio_ptr.is_null());
            assert_eq!(out_channels, 2);
            assert_eq!(out_frames, 100);
            assert_eq!(out_sr, 48000);
            assert_eq!(out_samples_len, 200);

            // Interleaved samples should alternate ch0 and ch1
            assert_eq!(*audio_ptr, 0.5);
            assert_eq!(*audio_ptr.add(1), -0.5);

            codec_free_audio_buffer(audio_ptr, out_samples_len);
        }
    }

    #[test]
    fn test_wasm_capture_wrappers() {
        // Meter test
        let mut meter = WasmLevelMeter::new(2, 48000);
        let interleaved = vec![0.5f32, -0.5f32, 0.5f32, -0.5f32];
        meter.process_interleaved(&interleaved, 2);
        assert!((meter.get_peak_db(0) - (-6.02)).abs() < 0.1);
        assert!(!meter.is_clipped(0));

        // PreRollBuffer test
        let mut preroll = WasmPreRollBuffer::new(2, 48000, 0.1);
        assert!(preroll.is_empty());
        preroll.append_interleaved(&[0.1, 0.2, 0.3, 0.4]);
        assert_eq!(preroll.len(), 2);
        let drained = preroll.drain_interleaved();
        assert_eq!(drained, vec![0.1, 0.2, 0.3, 0.4]);
        assert!(preroll.is_empty());

        // ChunkedRecordWriter test
        let mut writer = WasmChunkedRecordWriter::new(48000, 2);
        let chunk_bytes = writer.encode_chunk(&[0.1, 0.2, 0.3, 0.4]).unwrap();
        assert!(!chunk_bytes.is_empty());
        let recovered = WasmChunkedRecordWriter::recover_interleaved(&chunk_bytes).unwrap();
        assert_eq!(recovered, vec![0.1, 0.2, 0.3, 0.4]);
        let wav = WasmChunkedRecordWriter::to_wav(&chunk_bytes).unwrap();
        assert!(wav.starts_with(b"RIFF"));
    }

    #[test]
    fn test_wasm_engine_core_wrappers() {
        // Resampler test
        let resampler = WasmResampler::new(44100, 48000, 1);
        let in_sine = vec![0.5f32; 441];
        let out_sine = resampler.resample_channel(&in_sine);
        assert_eq!(out_sine.len(), 480);

        // Dither test
        let mut dither = WasmDither::new(1, 16, 1);
        let mut samples = vec![0.25f32; 100];
        dither.process_channel(&mut samples);
        assert!(!samples[50].is_nan());

        // Transport test
        let mut transport = WasmTransport::new(48000);
        assert_eq!(transport.playhead(), 0);
        transport.play();
        transport.advance(480);
        assert_eq!(transport.playhead(), 480);
        transport.seek(100);
        assert_eq!(transport.playhead(), 100);
    }

    #[test]
    fn test_c_abi_engine_quantum() {
        unsafe {
            let host = engine_create(48000.0, 128);
            assert!(!host.is_null());

            engine_command_play(host);
            engine_command_seek(host, 500);
            engine_command_set_loop(host, true, 100, 2000);
            engine_command_set_scrub(host, 1.5);

            let in_l = vec![0.5f32; 128];
            let in_r = vec![0.5f32; 128];
            let mut out_l = vec![0.0f32; 128];
            let mut out_r = vec![0.0f32; 128];

            engine_process_quantum(
                host,
                in_l.as_ptr(),
                in_r.as_ptr(),
                out_l.as_mut_ptr(),
                out_r.as_mut_ptr(),
                128,
            );

            assert!(!out_l[0].is_nan());
            assert!(!out_r[0].is_nan());

            engine_destroy(host);
        }
    }

    #[test]
    fn test_wasm_export_and_converter_apis() {
        // Presets JSON test
        let presets_json = wasm_get_export_presets();
        assert!(presets_json.contains("streaming"));
        assert!(presets_json.contains("podcast_stereo"));
        assert!(presets_json.contains("cd_audio"));
        assert!(presets_json.contains("archival"));

        // Export WAV with preset test
        let samples = vec![0.5f32; 2400]; // 1200 frames stereo
        let config = r#"{"preset":"cd_audio","title":"Test CD Track"}"#;
        let wav_bytes = wasm_export_audio(&samples, 2, 48000, config).expect("WASM export failed");
        assert!(wav_bytes.starts_with(b"RIFF"));

        // Export FLAC test
        let flac_config = r#"{"format":"flac_16","title":"Test FLAC Track"}"#;
        let flac_bytes =
            wasm_export_audio(&samples, 2, 48000, flac_config).expect("WASM FLAC export failed");
        assert!(flac_bytes.starts_with(b"fLaC"));

        // Format conversion test
        let conv_config = r#"{"format":"wav_24"}"#;
        let conv_bytes =
            wasm_convert_audio_format(&wav_bytes, conv_config).expect("WASM convert failed");
        assert!(conv_bytes.starts_with(b"RIFF"));
    }

    #[test]
    fn test_wasm_analysis_apis() {
        let left = vec![0.5f32; 2048];
        let right = vec![0.5f32; 2048];
        let mut interleaved = Vec::with_capacity(4096);
        for i in 0..2048 {
            interleaved.push(left[i]);
            interleaved.push(right[i]);
        }

        // Peak pyramid test
        let pyr_bytes = wasm_build_peak_pyramid(&interleaved, 2, 64).expect("Pyramid build failed");
        assert!(pyr_bytes.starts_with(b"FPYR"));

        let queried_bins =
            wasm_query_peak_pyramid(&pyr_bytes, 0, 0, 1024, 64).expect("Query failed");
        assert!(!queried_bins.is_empty());

        // Spectrogram tile test
        let tile_rgba = wasm_generate_spectrogram_tile(
            &left,
            48000,
            r#"{"window_size":512,"hop_size":256,"num_bins":64,"scale":"linear","colormap":"magma"}"#,
        )
        .expect("Spectrogram tile failed");
        assert!(!tile_rgba.is_empty());

        // Loudness meter test
        let loudness_json =
            wasm_measure_loudness(&interleaved, 2, 48000).expect("Loudness test failed");
        assert!(loudness_json.contains("integrated_lufs"));

        // Stereo analysis test (Left = Right -> correlation = 1.0)
        let stereo_json = wasm_analyze_stereo(&left, &right, 128).expect("Stereo test failed");
        assert!(stereo_json.contains("phase_correlation"));
        assert!(stereo_json.contains("\"phase_correlation\":1.0"));
    }

    #[test]
    fn test_c_abi_reverb_and_acoustic_nodes() {
        unsafe {
            // Reverb FDN C ABI
            let rev = reverb_create(48000.0);
            reverb_set_decay(rev, 2.5);
            reverb_set_wet(rev, 0.4);
            reverb_set_damping(rev, 0.3);

            let in_l = [0.5f32; 128];
            let in_r = [0.5f32; 128];
            let mut out_l = [0.0f32; 128];
            let mut out_r = [0.0f32; 128];

            reverb_process(
                rev,
                in_l.as_ptr(),
                in_r.as_ptr(),
                out_l.as_mut_ptr(),
                out_r.as_mut_ptr(),
                128,
            );
            assert!(!out_l[0].is_nan());
            reverb_destroy(rev);

            // Room Reverb C ABI
            let rm = room_reverb_create(48000.0);
            room_reverb_apply_preset(rm, 2); // ConcertHall
            room_reverb_update_custom(rm, 15.0, 20.0, 7.0, 0, 3, 0);
            room_reverb_process(
                rm,
                in_l.as_ptr(),
                in_r.as_ptr(),
                out_l.as_mut_ptr(),
                out_r.as_mut_ptr(),
                128,
            );
            assert!(!out_l[0].is_nan());
            room_reverb_destroy(rm);

            // Concert Hall C ABI
            let hall = concert_hall_create(48000.0);
            concert_hall_set_row(hall, 10.0);
            concert_hall_set_crowd_gain(hall, 0.1);
            concert_hall_process(
                hall,
                in_l.as_ptr(),
                in_r.as_ptr(),
                out_l.as_mut_ptr(),
                out_r.as_mut_ptr(),
                128,
            );
            assert!(!out_l[0].is_nan());
            concert_hall_destroy(hall);

            // Next Door C ABI
            let next_door = next_door_create(48000.0);
            next_door_set_enabled(next_door, true);
            next_door_set_cutoff(next_door, 450.0);
            next_door_set_bass_boost(next_door, 5.0);
            next_door_process(
                next_door,
                in_l.as_ptr(),
                in_r.as_ptr(),
                out_l.as_mut_ptr(),
                out_r.as_mut_ptr(),
                128,
            );
            assert!(!out_l[0].is_nan());
            next_door_destroy(next_door);

            // JSON room calculation
            let json = wasm_compute_room_acoustics(12.0, 18.0, 6.0, 0, 3, 0);
            assert!(json.contains("rt60"));
            assert!(json.contains("first_reflection_ms"));
        }
    }

    #[test]
    fn test_c_abi_creative_and_voice_nodes() {
        unsafe {
            let in_l = [0.5f32; 128];
            let in_r = [0.5f32; 128];
            let mut out_l = [0.0f32; 128];
            let mut out_r = [0.0f32; 128];

            // Slowed + Reverb
            let sr = slowed_reverb_create(48000.0);
            slowed_reverb_set_speed(sr, 0.85);
            slowed_reverb_process(
                sr,
                in_l.as_ptr(),
                in_r.as_ptr(),
                out_l.as_mut_ptr(),
                out_r.as_mut_ptr(),
                128,
            );
            assert!(!out_l[0].is_nan());
            slowed_reverb_destroy(sr);

            // Nightcore
            let nc = nightcore_create(48000.0);
            nightcore_set_speed(nc, 1.25);
            nightcore_process(
                nc,
                in_l.as_ptr(),
                in_r.as_ptr(),
                out_l.as_mut_ptr(),
                out_r.as_mut_ptr(),
                128,
            );
            assert!(!out_l[0].is_nan());
            nightcore_destroy(nc);

            // Lo-Fi Macro
            let lf = lofi_create(48000.0);
            lofi_set_amount(lf, 0.64);
            lofi_process(
                lf,
                in_l.as_ptr(),
                in_r.as_ptr(),
                out_l.as_mut_ptr(),
                out_r.as_mut_ptr(),
                128,
            );
            assert!(!out_l[0].is_nan());
            lofi_destroy(lf);

            // Vinyl
            let vn = vinyl_create(48000.0);
            vinyl_set_seed(vn, 4821);
            vinyl_process(
                vn,
                in_l.as_ptr(),
                in_r.as_ptr(),
                out_l.as_mut_ptr(),
                out_r.as_mut_ptr(),
                128,
            );
            assert!(!out_l[0].is_nan());
            vinyl_destroy(vn);

            // Voice Changer
            let vc = voice_changer_create(48000.0);
            voice_changer_set_character(vc, 12); // Robot
            voice_changer_process(
                vc,
                in_l.as_ptr(),
                in_r.as_ptr(),
                out_l.as_mut_ptr(),
                out_r.as_mut_ptr(),
                128,
            );
            assert!(!out_l[0].is_nan());
            voice_changer_destroy(vc);
        }
    }

    #[test]
    fn test_wasm_ml_and_ai_apis() {
        let models_json = wasm_get_models_json();
        assert!(models_json.contains("stems_balanced"));

        let sample = vec![0.5f32; 1000];
        let classify = wasm_classify_audio(&sample, 48000.0);
        assert!(classify.contains("category"));

        let cleaned = wasm_voice_cleanup(&sample, 48000.0, 0.6);
        assert_eq!(cleaned.len(), 1000);

        let transcript_json = wasm_get_sample_transcript_json();
        assert!(transcript_json.contains("Maya"));

        let srt = wasm_export_subtitles(&transcript_json, 0);
        assert!(srt.contains("-->"));

        let fillers_json = wasm_get_sample_fillers_json();
        assert!(fillers_json.contains("Um"));

        let jobs_json = wasm_list_cloud_jobs_json();
        assert!(jobs_json.contains("Harbor Lights"));

        let tempo_key = wasm_detect_tempo_and_key_json(&sample, 48000.0);
        assert!(tempo_key.contains("bpm"));
        assert!(tempo_key.contains("key"));

        let mastered = wasm_auto_master(&sample, 48000.0, 0, 1);
        assert_eq!(mastered.len(), 1000);

        let tts = wasm_synthesize_tts("Hello Harbor Lights", 0, 48000.0);
        assert!(!tts.is_empty());

        let dub_json = wasm_get_dubbing_project_json();
        assert!(dub_json.contains("Spanish"));

        let magic_json = wasm_diagnose_and_propose_magic_json(&sample, 48000.0, 0);
        assert!(magic_json.contains("diagnosis"));
        assert!(magic_json.contains("Hum removal"));
    }
}
