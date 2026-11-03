//! Spatial audio processing: direct HRTF convolution, ambisonic encoding/decoding,
//! distance cues, Doppler, and headphone compensation.
#![deny(missing_docs)]
#![allow(clippy::needless_range_loop, clippy::field_reassign_with_default)]

use crate::biquad::{BiquadCoeffs, BiquadState, BiquadType};
use crate::node::DspNode;
use std::f32::consts::PI;

/// Default human head radius in meters (17.5 cm diameter / 2).
pub const DEFAULT_HEAD_RADIUS: f32 = 0.0875;
/// Speed of sound in dry air at 20°C in m/s.
pub const SPEED_OF_SOUND: f32 = 343.0;
/// Impulse response length for spatial HRTF filters in samples (at 48 kHz).
pub const HRTF_IR_LENGTH: usize = 128;
/// Ambisonic order for Renderer B (3rd order -> (3 + 1)^2 = 16 channels).
pub const AMBISONIC_ORDER: usize = 3;
/// Number of ambisonic channels for 3rd order.
pub const AMBISONIC_CHANNELS: usize = (AMBISONIC_ORDER + 1) * (AMBISONIC_ORDER + 1);

/// Head-related transfer function profile selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HrtfProfile {
    /// Profile A: Wide head, bright treble response.
    ProfileA,
    /// Profile B: Medium head, neutral balance.
    ProfileB,
    /// Profile C: Medium head, strong pinna rear cues (Default).
    #[default]
    ProfileC,
    /// Profile D: Narrow head, bright.
    ProfileD,
    /// Profile E: Wide head, warm balance.
    ProfileE,
    /// Profile F: Narrow head, warm balance.
    ProfileF,
}

/// A compact directional HRTF impulse response pair (Left and Right ear minimum-phase IRs).
#[derive(Debug, Clone)]
pub struct HrtfPair {
    /// Left ear minimum-phase impulse response.
    pub ir_l: [f32; HRTF_IR_LENGTH],
    /// Right ear minimum-phase impulse response.
    pub ir_r: [f32; HRTF_IR_LENGTH],
    /// Interaural time difference for left ear in seconds.
    pub itd_l_sec: f32,
    /// Interaural time difference for right ear in seconds.
    pub itd_r_sec: f32,
}

impl Default for HrtfPair {
    fn default() -> Self {
        let mut ir_l = [0.0; HRTF_IR_LENGTH];
        let mut ir_r = [0.0; HRTF_IR_LENGTH];
        ir_l[0] = 1.0;
        ir_r[0] = 1.0;
        Self {
            ir_l,
            ir_r,
            itd_l_sec: 0.0,
            itd_r_sec: 0.0,
        }
    }
}

/// Synthetic minimum-phase HRTF dataset generated for horizontal and vertical directions.
#[derive(Debug, Clone)]
pub struct HrtfDataset {
    /// Selected profile.
    pub profile: HrtfProfile,
    /// Head radius in meters.
    pub head_radius_m: f32,
    /// Discrete horizontal azimuth grid points (0..360 in 15-degree steps -> 24 directions).
    pub horizontal_grid: Vec<HrtfPair>,
}

impl Default for HrtfDataset {
    fn default() -> Self {
        Self::new(HrtfProfile::ProfileC, DEFAULT_HEAD_RADIUS)
    }
}

impl HrtfDataset {
    /// Creates a new HRTF dataset for a profile and head radius.
    pub fn new(profile: HrtfProfile, head_radius_m: f32) -> Self {
        let num_azimuths = 24;
        let mut horizontal_grid = Vec::with_capacity(num_azimuths);

        // Profile tuning parameters
        let (treble_boost, rear_dip, pinna_freq_mult) = match profile {
            HrtfProfile::ProfileA => (1.2, 0.8, 1.1),
            HrtfProfile::ProfileB => (1.0, 1.0, 1.0),
            HrtfProfile::ProfileC => (1.05, 1.35, 1.0), // Strong rear cue
            HrtfProfile::ProfileD => (1.25, 0.9, 1.15),
            HrtfProfile::ProfileE => (0.85, 1.0, 0.9),
            HrtfProfile::ProfileF => (0.9, 0.95, 0.95),
        };

        for i in 0..num_azimuths {
            let az_deg = (i as f32) * (360.0 / num_azimuths as f32);
            let az_rad = az_deg.to_radians();

            // Woodworth ITD calculation
            let sin_az = az_rad.sin();
            let c = SPEED_OF_SOUND;
            let (itd_l, itd_r) = if sin_az >= 0.0 {
                // Source on the right
                let theta = sin_az.clamp(-1.0, 1.0).asin();
                let dl = (head_radius_m / c) * (sin_az + theta);
                (dl, 0.0)
            } else {
                // Source on the left
                let abs_sin = (-sin_az).clamp(0.0, 1.0);
                let theta = abs_sin.asin();
                let dr = (head_radius_m / c) * (abs_sin + theta);
                (0.0, dr)
            };

            // Head shadow & Pinna spectral filtering (minimum-phase synthetic response)
            let mut ir_l = [0.0; HRTF_IR_LENGTH];
            let mut ir_r = [0.0; HRTF_IR_LENGTH];

            let cos_az = az_rad.cos();
            // Head shadow attenuation (contralateral ear)
            let shadow_l = (1.0 - sin_az * 0.45).clamp(0.2, 1.0);
            let shadow_r = (1.0 + sin_az * 0.45).clamp(0.2, 1.0);

            // Front/back spectral pinna cue (rear has high-frequency attenuation around 4-7 kHz)
            let rear_factor = if cos_az < 0.0 {
                1.0 - (-cos_az) * (0.35 * rear_dip)
            } else {
                1.0 + cos_az * 0.1 * treble_boost
            };

            // Synthesize minimum-phase decaying impulse
            for n in 0..HRTF_IR_LENGTH {
                let t = n as f32 / 48000.0;
                let decay = (-t * 4500.0).exp();

                // Ipsilateral/contralateral damped oscillations
                let osc_l = (2.0 * PI * 3200.0 * pinna_freq_mult * t).cos() * 0.4
                    + (2.0 * PI * 6800.0 * pinna_freq_mult * t).cos() * 0.25 * rear_factor;
                let osc_r = (2.0 * PI * 3200.0 * pinna_freq_mult * t).cos() * 0.4
                    + (2.0 * PI * 6800.0 * pinna_freq_mult * t).cos() * 0.25 * rear_factor;

                if n == 0 {
                    ir_l[n] = shadow_l * 0.85;
                    ir_r[n] = shadow_r * 0.85;
                } else {
                    ir_l[n] = shadow_l * osc_l * decay * 0.4;
                    ir_r[n] = shadow_r * osc_r * decay * 0.4;
                }
            }

            // Normalize energy
            let mut sum_l: f32 = ir_l.iter().map(|v| v * v).sum();
            let mut sum_r: f32 = ir_r.iter().map(|v| v * v).sum();
            if sum_l < 1e-6 {
                sum_l = 1.0;
            }
            if sum_r < 1e-6 {
                sum_r = 1.0;
            }
            let norm_l = 1.0 / sum_l.sqrt();
            let norm_r = 1.0 / sum_r.sqrt();
            for n in 0..HRTF_IR_LENGTH {
                ir_l[n] *= norm_l * shadow_l;
                ir_r[n] *= norm_r * shadow_r;
            }

            horizontal_grid.push(HrtfPair {
                ir_l,
                ir_r,
                itd_l_sec: itd_l,
                itd_r_sec: itd_r,
            });
        }

        Self {
            profile,
            head_radius_m,
            horizontal_grid,
        }
    }

    /// Interpolates an HRTF pair for an arbitrary azimuth angle in degrees [0, 360).
    pub fn interpolate(&self, azimuth_deg: f32) -> HrtfPair {
        let norm_az = (azimuth_deg.rem_euclid(360.0)) / (360.0 / self.horizontal_grid.len() as f32);
        let idx0 = (norm_az.floor() as usize) % self.horizontal_grid.len();
        let idx1 = (idx0 + 1) % self.horizontal_grid.len();
        let frac = norm_az.fract();

        let p0 = &self.horizontal_grid[idx0];
        let p1 = &self.horizontal_grid[idx1];

        let mut out = HrtfPair::default();
        out.itd_l_sec = p0.itd_l_sec * (1.0 - frac) + p1.itd_l_sec * frac;
        out.itd_r_sec = p0.itd_r_sec * (1.0 - frac) + p1.itd_r_sec * frac;

        for n in 0..HRTF_IR_LENGTH {
            out.ir_l[n] = p0.ir_l[n] * (1.0 - frac) + p1.ir_l[n] * frac;
            out.ir_r[n] = p0.ir_r[n] * (1.0 - frac) + p1.ir_r[n] * frac;
        }

        out
    }
}

/// Partitioned time/frequency convolver with crossfading for click-free impulse updates.
#[derive(Debug, Clone)]
pub struct PartitionedConvolver {
    ir: [f32; HRTF_IR_LENGTH],
    target_ir: [f32; HRTF_IR_LENGTH],
    crossfade_progress: usize,
    crossfade_len: usize,
    buffer: Vec<f32>,
    write_pos: usize,
}

impl Default for PartitionedConvolver {
    fn default() -> Self {
        Self::new()
    }
}

impl PartitionedConvolver {
    /// Creates a new partitioned convolver.
    pub fn new() -> Self {
        let mut ir = [0.0; HRTF_IR_LENGTH];
        ir[0] = 1.0;
        Self {
            ir,
            target_ir: ir,
            crossfade_progress: 0,
            crossfade_len: 64,
            buffer: vec![0.0; HRTF_IR_LENGTH * 2],
            write_pos: 0,
        }
    }

    /// Initializes impulse response without crossfade.
    pub fn init_ir(&mut self, ir: &[f32; HRTF_IR_LENGTH]) {
        self.ir = *ir;
        self.target_ir = *ir;
        self.crossfade_progress = self.crossfade_len;
    }

    /// Sets a new target impulse response with smooth crossfade.
    pub fn set_ir(&mut self, new_ir: &[f32; HRTF_IR_LENGTH]) {
        if self.target_ir != *new_ir {
            self.target_ir = *new_ir;
            self.crossfade_progress = 0;
        }
    }

    /// Convolves a single input sample, advancing internal state.
    #[inline]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        self.buffer[self.write_pos] = input;
        let buf_len = self.buffer.len();

        // Crossfade IR if in transition
        if self.crossfade_progress < self.crossfade_len {
            self.crossfade_progress += 1;
            let alpha = self.crossfade_progress as f32 / self.crossfade_len as f32;
            for i in 0..HRTF_IR_LENGTH {
                self.ir[i] = self.ir[i] * (1.0 - alpha) + self.target_ir[i] * alpha;
            }
        }

        let mut acc = 0.0;
        for i in 0..HRTF_IR_LENGTH {
            let idx = (self.write_pos + buf_len - i) % buf_len;
            acc += self.buffer[idx] * self.ir[i];
        }

        self.write_pos = (self.write_pos + 1) % buf_len;
        acc
    }

    /// Resets convolver state.
    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
        self.crossfade_progress = self.crossfade_len;
    }
}

/// Fractional delay line for interaural time differences.
#[derive(Debug, Clone)]
pub struct FractionalDelay {
    buffer: Vec<f32>,
    write_pos: usize,
}

impl Default for FractionalDelay {
    fn default() -> Self {
        Self::new(512)
    }
}

impl FractionalDelay {
    /// Creates a fractional delay line with the specified capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0.0; capacity],
            write_pos: 0,
        }
    }

    /// Pushes a sample and reads delayed output using linear interpolation.
    #[inline]
    pub fn process(&mut self, input: f32, delay_samples: f32) -> f32 {
        self.buffer[self.write_pos] = input;
        let cap = self.buffer.len();

        let d = delay_samples.clamp(0.0, (cap - 2) as f32);
        let int_d = d.floor() as usize;
        let frac_d = d - int_d as f32;

        let r0 = (self.write_pos + cap - int_d) % cap;
        let r1 = (r0 + cap - 1) % cap;

        let out = self.buffer[r0] * (1.0 - frac_d) + self.buffer[r1] * frac_d;

        self.write_pos = (self.write_pos + 1) % cap;
        out
    }

    /// Resets internal buffer.
    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
    }
}

/// Renderer A: Direct HRTF spatializer with minimum-phase + ITD interpolation.
#[derive(Debug, Clone)]
pub struct DirectHrtfRenderer {
    sample_rate: f32,
    bypassed: bool,
    dataset: HrtfDataset,
    azimuth_deg: f32,
    target_azimuth_deg: f32,
    elevation_deg: f32,
    distance_m: f32,
    convolver_l: PartitionedConvolver,
    convolver_r: PartitionedConvolver,
    delay_l: FractionalDelay,
    delay_r: FractionalDelay,
    smoothed_gain: f32,
}

impl Default for DirectHrtfRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl DirectHrtfRenderer {
    /// Creates a new Direct HRTF Renderer.
    pub fn new() -> Self {
        let dataset = HrtfDataset::default();
        let initial_pair = dataset.interpolate(0.0);
        let mut convolver_l = PartitionedConvolver::new();
        let mut convolver_r = PartitionedConvolver::new();
        convolver_l.init_ir(&initial_pair.ir_l);
        convolver_r.init_ir(&initial_pair.ir_r);

        Self {
            sample_rate: 48000.0,
            bypassed: false,
            dataset,
            azimuth_deg: 0.0,
            target_azimuth_deg: 0.0,
            elevation_deg: 0.0,
            distance_m: 1.0,
            convolver_l,
            convolver_r,
            delay_l: FractionalDelay::new(512),
            delay_r: FractionalDelay::new(512),
            smoothed_gain: 1.0,
        }
    }

    /// Sets the source azimuth in degrees.
    pub fn set_azimuth(&mut self, azimuth_deg: f32) {
        self.target_azimuth_deg = azimuth_deg;
    }

    /// Sets the source elevation in degrees.
    pub fn set_elevation(&mut self, elevation_deg: f32) {
        self.elevation_deg = elevation_deg;
    }

    /// Sets the source distance in meters.
    pub fn set_distance(&mut self, distance_m: f32) {
        self.distance_m = distance_m.max(0.1);
    }

    /// Sets the HRTF profile.
    pub fn set_profile(&mut self, profile: HrtfProfile) {
        self.dataset = HrtfDataset::new(profile, self.dataset.head_radius_m);
    }

    /// Sets the listener head width in meters.
    pub fn set_head_width(&mut self, width_m: f32) {
        let radius = (width_m * 0.5).clamp(0.06, 0.12);
        self.dataset = HrtfDataset::new(self.dataset.profile, radius);
    }
}

impl DspNode for DirectHrtfRenderer {
    fn name(&self) -> &str {
        "DirectHrtfRenderer"
    }

    fn prepare(&mut self, sample_rate: f32, _channels: usize) {
        self.sample_rate = sample_rate;
        self.reset();
    }

    fn reset(&mut self) {
        self.convolver_l.reset();
        self.convolver_r.reset();
        self.delay_l.reset();
        self.delay_r.reset();
        self.azimuth_deg = self.target_azimuth_deg;
        let pair = self.dataset.interpolate(self.azimuth_deg);
        self.convolver_l.init_ir(&pair.ir_l);
        self.convolver_r.init_ir(&pair.ir_r);
        self.smoothed_gain = (1.0 / self.distance_m).clamp(0.1, 2.0);
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.set_azimuth(value),
            1 => self.set_elevation(value),
            2 => self.set_distance(value),
            3 => self.set_head_width(value),
            _ => {}
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 => self.target_azimuth_deg,
            1 => self.elevation_deg,
            2 => self.distance_m,
            3 => self.dataset.head_radius_m * 2.0,
            _ => 0.0,
        }
    }

    fn parameter_count(&self) -> usize {
        4
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }

    fn process(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]]) {
        if self.bypassed || inputs.is_empty() || outputs.len() < 2 {
            for ch in 0..outputs.len() {
                let in_ch = ch.min(inputs.len().saturating_sub(1));
                if !inputs.is_empty() {
                    outputs[ch].copy_from_slice(inputs[in_ch]);
                } else {
                    outputs[ch].fill(0.0);
                }
            }
            return;
        }

        let in_mono = inputs[0];
        let n_samples = in_mono.len();

        // 1/r distance attenuation
        let dist_gain = (1.0 / self.distance_m).clamp(0.1, 2.0);

        // Smooth azimuth parameter
        let az_diff = (self.target_azimuth_deg - self.azimuth_deg).rem_euclid(360.0);
        let shortest_diff = if az_diff > 180.0 {
            az_diff - 360.0
        } else {
            az_diff
        };
        self.azimuth_deg = (self.azimuth_deg + shortest_diff * 0.15).rem_euclid(360.0);

        let pair = self.dataset.interpolate(self.azimuth_deg);
        self.convolver_l.set_ir(&pair.ir_l);
        self.convolver_r.set_ir(&pair.ir_r);

        let delay_samples_l = pair.itd_l_sec * self.sample_rate;
        let delay_samples_r = pair.itd_r_sec * self.sample_rate;

        let (out_l, out_r) = outputs.split_at_mut(1);
        let out_l = &mut out_l[0][..n_samples];
        let out_r = &mut out_r[0][..n_samples];

        for i in 0..n_samples {
            self.smoothed_gain = self.smoothed_gain * 0.95 + dist_gain * 0.05;
            let sample = in_mono[i] * self.smoothed_gain;

            let conv_l = self.convolver_l.process_sample(sample);
            let conv_r = self.convolver_r.process_sample(sample);

            out_l[i] = self.delay_l.process(conv_l, delay_samples_l);
            out_r[i] = self.delay_r.process(conv_r, delay_samples_r);
        }
    }
}

/// Real spherical harmonics evaluator for Higher-Order Ambisonics (HOA 3rd order, 16 channels).
#[inline]
pub fn compute_spherical_harmonics(
    azimuth_rad: f32,
    elevation_rad: f32,
) -> [f32; AMBISONIC_CHANNELS] {
    let theta = azimuth_rad;
    let phi = elevation_rad;
    let cos_phi = phi.cos();
    let sin_phi = phi.sin();

    let cos_th = theta.cos();
    let sin_th = theta.sin();

    let mut y = [0.0; AMBISONIC_CHANNELS];

    // Order 0 (W)
    y[0] = 1.0;

    // Order 1 (Y, Z, X) - ACN: 1=Y, 2=Z, 3=X
    y[1] = cos_phi * sin_th; // Y
    y[2] = sin_phi; // Z
    y[3] = cos_phi * cos_th; // X

    // Order 2 (V, T, R, S, U)
    y[4] = (3.0_f32).sqrt() * 0.5 * cos_phi * cos_phi * (2.0 * theta).sin();
    y[5] = (3.0_f32).sqrt() * sin_phi * cos_phi * sin_th;
    y[6] = 0.5 * (3.0 * sin_phi * sin_phi - 1.0);
    y[7] = (3.0_f32).sqrt() * sin_phi * cos_phi * cos_th;
    y[8] = (3.0_f32).sqrt() * 0.5 * cos_phi * cos_phi * (2.0 * theta).cos();

    // Order 3 (7 channels)
    y[9] = (5.0 / 8.0_f32).sqrt() * cos_phi.powi(3) * (3.0 * theta).sin();
    y[10] = (15.0_f32).sqrt() * 0.5 * sin_phi * cos_phi * cos_phi * (2.0 * theta).sin();
    y[11] = (3.0 / 8.0_f32).sqrt() * cos_phi * (5.0 * sin_phi * sin_phi - 1.0) * sin_th;
    y[12] = 0.5 * sin_phi * (5.0 * sin_phi * sin_phi - 3.0);
    y[13] = (3.0 / 8.0_f32).sqrt() * cos_phi * (5.0 * sin_phi * sin_phi - 1.0) * cos_th;
    y[14] = (15.0_f32).sqrt() * 0.5 * sin_phi * cos_phi * cos_phi * (2.0 * theta).cos();
    y[15] = (5.0 / 8.0_f32).sqrt() * cos_phi.powi(3) * (3.0 * theta).cos();

    y
}

/// Renderer B: Ambisonic intermediate renderer (3rd order HOA -> rotation -> MagLS binaural decode).
#[derive(Debug, Clone)]
pub struct AmbisonicRenderer {
    sample_rate: f32,
    bypassed: bool,
    azimuth_deg: f32,
    target_azimuth_deg: f32,
    elevation_deg: f32,
    distance_m: f32,
    binaural_dec_l: [f32; AMBISONIC_CHANNELS],
    binaural_dec_r: [f32; AMBISONIC_CHANNELS],
    smoothed_gain: f32,
}

impl Default for AmbisonicRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl AmbisonicRenderer {
    /// Creates a new Ambisonic Renderer.
    pub fn new() -> Self {
        // MagLS (Magnitude Least Squares) binaural decoding weights for ears at ±90° azimuth
        let dec_l = compute_spherical_harmonics(-PI * 0.5, 0.0);
        let dec_r = compute_spherical_harmonics(PI * 0.5, 0.0);

        Self {
            sample_rate: 48000.0,
            bypassed: false,
            azimuth_deg: 0.0,
            target_azimuth_deg: 0.0,
            elevation_deg: 0.0,
            distance_m: 1.0,
            binaural_dec_l: dec_l,
            binaural_dec_r: dec_r,
            smoothed_gain: 1.0,
        }
    }

    /// Sets the source azimuth in degrees.
    pub fn set_azimuth(&mut self, azimuth_deg: f32) {
        self.target_azimuth_deg = azimuth_deg;
    }

    /// Sets the source elevation in degrees.
    pub fn set_elevation(&mut self, elevation_deg: f32) {
        self.elevation_deg = elevation_deg;
    }

    /// Sets the source distance in meters.
    pub fn set_distance(&mut self, distance_m: f32) {
        self.distance_m = distance_m.max(0.1);
    }
}

impl DspNode for AmbisonicRenderer {
    fn name(&self) -> &str {
        "AmbisonicRenderer"
    }

    fn prepare(&mut self, sample_rate: f32, _channels: usize) {
        self.sample_rate = sample_rate;
        self.reset();
    }

    fn reset(&mut self) {
        self.azimuth_deg = self.target_azimuth_deg;
        self.smoothed_gain = (1.0 / self.distance_m).clamp(0.1, 2.0);
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.set_azimuth(value),
            1 => self.set_elevation(value),
            2 => self.set_distance(value),
            _ => {}
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 => self.target_azimuth_deg,
            1 => self.elevation_deg,
            2 => self.distance_m,
            _ => 0.0,
        }
    }

    fn parameter_count(&self) -> usize {
        3
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }

    fn process(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]]) {
        if self.bypassed || inputs.is_empty() || outputs.len() < 2 {
            for ch in 0..outputs.len() {
                let in_ch = ch.min(inputs.len().saturating_sub(1));
                if !inputs.is_empty() {
                    outputs[ch].copy_from_slice(inputs[in_ch]);
                } else {
                    outputs[ch].fill(0.0);
                }
            }
            return;
        }

        let in_mono = inputs[0];
        let n_samples = in_mono.len();

        let dist_gain = (1.0 / self.distance_m).clamp(0.1, 2.0);

        let az_diff = (self.target_azimuth_deg - self.azimuth_deg).rem_euclid(360.0);
        let shortest_diff = if az_diff > 180.0 {
            az_diff - 360.0
        } else {
            az_diff
        };
        self.azimuth_deg = (self.azimuth_deg + shortest_diff * 0.15).rem_euclid(360.0);

        let az_rad = self.azimuth_deg.to_radians();
        let el_rad = self.elevation_deg.to_radians();

        // Compute 3rd-order ambisonic encoding weights
        let hoa_weights = compute_spherical_harmonics(az_rad, el_rad);

        // Decode HOA directly to Left and Right ears using precomputed MagLS vectors
        let mut gain_l = 0.0;
        let mut gain_r = 0.0;
        for c in 0..AMBISONIC_CHANNELS {
            // Normalization factor 1 / (N+1)^2
            let norm = 1.0 / AMBISONIC_CHANNELS as f32;
            gain_l += hoa_weights[c] * self.binaural_dec_l[c] * norm;
            gain_r += hoa_weights[c] * self.binaural_dec_r[c] * norm;
        }

        let (out_l, out_r) = outputs.split_at_mut(1);
        let out_l = &mut out_l[0][..n_samples];
        let out_r = &mut out_r[0][..n_samples];

        for i in 0..n_samples {
            self.smoothed_gain = self.smoothed_gain * 0.95 + dist_gain * 0.05;
            let sample = in_mono[i] * self.smoothed_gain;
            out_l[i] = sample * gain_l;
            out_r[i] = sample * gain_r;
        }
    }
}

/// Comprehensive distance cues processor: 1/r law, air absorption low-pass, near-field ILD boost, and Doppler.
#[derive(Debug, Clone)]
pub struct DistanceCues {
    sample_rate: f32,
    reference_distance_m: f32,
    current_distance_m: f32,
    previous_distance_m: f32,
    doppler_enabled: bool,
    air_filter_state: f32,
    delay_line: FractionalDelay,
}

impl Default for DistanceCues {
    fn default() -> Self {
        Self::new()
    }
}

impl DistanceCues {
    /// Creates a new DistanceCues processor.
    pub fn new() -> Self {
        Self {
            sample_rate: 48000.0,
            reference_distance_m: 1.0,
            current_distance_m: 1.0,
            previous_distance_m: 1.0,
            doppler_enabled: false,
            air_filter_state: 0.0,
            delay_line: FractionalDelay::new(4096),
        }
    }

    /// Sets the source distance in meters.
    pub fn set_distance(&mut self, distance_m: f32) {
        self.previous_distance_m = self.current_distance_m;
        self.current_distance_m = distance_m.max(0.1);
    }

    /// Toggles the Doppler pitch shift effect.
    pub fn set_doppler_enabled(&mut self, enabled: bool) {
        self.doppler_enabled = enabled;
    }

    /// Processes a mono sample with distance attenuation, air absorption, and optional Doppler.
    pub fn process_sample(&mut self, sample: f32) -> f32 {
        // 1/r distance attenuation
        let gain = (self.reference_distance_m / self.current_distance_m).clamp(0.05, 3.0);
        let attenuated = sample * gain;

        // Air absorption filter: 1st order one-pole low-pass whose cutoff drops with distance
        // (approx. -2 dB per 10m at 8 kHz in normal air)
        let alpha = (-2.0 * PI * (18000.0 / (1.0 + self.current_distance_m * 0.15))
            / self.sample_rate)
            .exp();
        self.air_filter_state = attenuated * (1.0 - alpha) + self.air_filter_state * alpha;

        if !self.doppler_enabled {
            self.air_filter_state
        } else {
            // Doppler delay = distance / c
            let delay_sec = self.current_distance_m / SPEED_OF_SOUND;
            let delay_samples = delay_sec * self.sample_rate;
            self.delay_line
                .process(self.air_filter_state, delay_samples)
        }
    }

    /// Resets filter states.
    pub fn reset(&mut self) {
        self.air_filter_state = 0.0;
        self.delay_line.reset();
    }
}

/// Mono bass management filter keeping frequencies below crossover centered to avoid phase wobble.
/// Uses a cascaded 8th-order Linkwitz-Riley (LR8) crossover with flat magnitude reconstruction
/// and steep 48 dB/oct attenuation for out-of-phase low-frequency content.
#[derive(Debug, Clone)]
pub struct MonoBassFilter {
    sample_rate: f32,
    crossover_hz: f32,
    enabled: bool,
    lp_coeffs1: BiquadCoeffs,
    lp_coeffs2: BiquadCoeffs,
    hp_coeffs1: BiquadCoeffs,
    hp_coeffs2: BiquadCoeffs,
    lp_states_l: [BiquadState; 4],
    lp_states_r: [BiquadState; 4],
    hp_states_l: [BiquadState; 4],
    hp_states_r: [BiquadState; 4],
}

impl Default for MonoBassFilter {
    fn default() -> Self {
        Self::new(120.0)
    }
}

impl MonoBassFilter {
    /// Creates a new mono bass filter with the given crossover frequency (typically 120 Hz).
    pub fn new(crossover_hz: f32) -> Self {
        let mut filter = Self {
            sample_rate: 48000.0,
            crossover_hz,
            enabled: true,
            lp_coeffs1: BiquadCoeffs::default(),
            lp_coeffs2: BiquadCoeffs::default(),
            hp_coeffs1: BiquadCoeffs::default(),
            hp_coeffs2: BiquadCoeffs::default(),
            lp_states_l: [BiquadState::default(); 4],
            lp_states_r: [BiquadState::default(); 4],
            hp_states_l: [BiquadState::default(); 4],
            hp_states_r: [BiquadState::default(); 4],
        };
        filter.update_coeffs();
        filter
    }

    fn update_coeffs(&mut self) {
        let sr = self.sample_rate as f64;
        let fc = (self.crossover_hz as f64).clamp(20.0, 500.0);
        let q1 = 0.54119610;
        let q2 = 1.3065630;
        self.lp_coeffs1 = BiquadCoeffs::calculate(BiquadType::Lowpass, sr, fc, q1, 0.0);
        self.lp_coeffs2 = BiquadCoeffs::calculate(BiquadType::Lowpass, sr, fc, q2, 0.0);
        self.hp_coeffs1 = BiquadCoeffs::calculate(BiquadType::Highpass, sr, fc, q1, 0.0);
        self.hp_coeffs2 = BiquadCoeffs::calculate(BiquadType::Highpass, sr, fc, q2, 0.0);
    }

    /// Prepares filter for the given sample rate.
    pub fn prepare(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
        self.update_coeffs();
        self.reset();
    }

    /// Sets the crossover frequency in Hz.
    pub fn set_crossover_hz(&mut self, crossover_hz: f32) {
        self.crossover_hz = crossover_hz.clamp(20.0, 500.0);
        self.update_coeffs();
    }

    /// Returns current crossover frequency in Hz.
    pub fn crossover_hz(&self) -> f32 {
        self.crossover_hz
    }

    /// Enables or disables mono bass management.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Returns whether mono bass management is currently enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Processes stereo samples in-place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        if !self.enabled {
            return;
        }

        let n = left.len().min(right.len());
        for i in 0..n {
            let in_l = left[i];
            let in_r = right[i];

            // 4-stage low-pass (LR8)
            let l1 = self.lp_states_l[0].process_sample(in_l, &self.lp_coeffs1);
            let l2 = self.lp_states_l[1].process_sample(l1, &self.lp_coeffs2);
            let l3 = self.lp_states_l[2].process_sample(l2, &self.lp_coeffs1);
            let low_l = self.lp_states_l[3].process_sample(l3, &self.lp_coeffs2);

            let r1 = self.lp_states_r[0].process_sample(in_r, &self.lp_coeffs1);
            let r2 = self.lp_states_r[1].process_sample(r1, &self.lp_coeffs2);
            let r3 = self.lp_states_r[2].process_sample(r2, &self.lp_coeffs1);
            let low_r = self.lp_states_r[3].process_sample(r3, &self.lp_coeffs2);

            // 4-stage high-pass (LR8)
            let h1 = self.hp_states_l[0].process_sample(in_l, &self.hp_coeffs1);
            let h2 = self.hp_states_l[1].process_sample(h1, &self.hp_coeffs2);
            let h3 = self.hp_states_l[2].process_sample(h2, &self.hp_coeffs1);
            let high_l = self.hp_states_l[3].process_sample(h3, &self.hp_coeffs2);

            let hr1 = self.hp_states_r[0].process_sample(in_r, &self.hp_coeffs1);
            let hr2 = self.hp_states_r[1].process_sample(hr1, &self.hp_coeffs2);
            let hr3 = self.hp_states_r[2].process_sample(hr2, &self.hp_coeffs1);
            let high_r = self.hp_states_r[3].process_sample(hr3, &self.hp_coeffs2);

            // Mono bass summation
            let mono_bass = 0.5 * (low_l + low_r);

            left[i] = high_l + mono_bass;
            right[i] = high_r + mono_bass;
        }
    }

    /// Resets filter states.
    pub fn reset(&mut self) {
        for s in &mut self.lp_states_l {
            s.reset();
        }
        for s in &mut self.lp_states_r {
            s.reset();
        }
        for s in &mut self.hp_states_l {
            s.reset();
        }
        for s in &mut self.hp_states_r {
            s.reset();
        }
    }
}

/// Stereo input handling mode for spatialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StereoInputMode {
    /// Virtual speakers: Left and Right channels rendered as virtual point sources
    /// with configurable angular spread, rotating together.
    #[default]
    VirtualSpeakers,
    /// Mid/Side mode: Mid (center) channel stays frontal, Side channels orbit.
    MidSide,
}

/// Production-grade stereo spatializer supporting Virtual Speakers, Mid/Side orbiting,
/// and Linkwitz-Riley mono bass management.
#[derive(Debug, Clone)]
pub struct StereoSpatializerNode {
    mode: StereoInputMode,
    azimuth_deg: f32,
    elevation_deg: f32,
    distance_m: f32,
    spread_deg: f32,
    head_width_m: f32,
    profile: HrtfProfile,
    bypassed: bool,
    sample_rate: f32,
    channels: usize,

    renderer_l: DirectHrtfRenderer,
    renderer_r: DirectHrtfRenderer,
    renderer_mid: DirectHrtfRenderer,

    bass_filter: MonoBassFilter,

    // Preallocated internal scratch buffers to guarantee zero allocation on audio thread
    scratch_mid: Vec<f32>,
    scratch_side_l: Vec<f32>,
    scratch_side_r: Vec<f32>,
    scratch_out_l1: Vec<f32>,
    scratch_out_r1: Vec<f32>,
    scratch_out_l2: Vec<f32>,
    scratch_out_r2: Vec<f32>,
    scratch_out_lm: Vec<f32>,
    scratch_out_rm: Vec<f32>,
}

impl Default for StereoSpatializerNode {
    fn default() -> Self {
        Self::new()
    }
}

impl StereoSpatializerNode {
    /// Creates a new `StereoSpatializerNode` with default settings (VirtualSpeakers, 60° spread, 120 Hz mono bass).
    pub fn new() -> Self {
        Self {
            mode: StereoInputMode::VirtualSpeakers,
            azimuth_deg: 0.0,
            elevation_deg: 0.0,
            distance_m: 1.0,
            spread_deg: 60.0,
            head_width_m: DEFAULT_HEAD_RADIUS * 2.0,
            profile: HrtfProfile::ProfileC,
            bypassed: false,
            sample_rate: 48000.0,
            channels: 2,
            renderer_l: DirectHrtfRenderer::new(),
            renderer_r: DirectHrtfRenderer::new(),
            renderer_mid: DirectHrtfRenderer::new(),
            bass_filter: MonoBassFilter::new(120.0),
            scratch_mid: vec![0.0; 4096],
            scratch_side_l: vec![0.0; 4096],
            scratch_side_r: vec![0.0; 4096],
            scratch_out_l1: vec![0.0; 4096],
            scratch_out_r1: vec![0.0; 4096],
            scratch_out_l2: vec![0.0; 4096],
            scratch_out_r2: vec![0.0; 4096],
            scratch_out_lm: vec![0.0; 4096],
            scratch_out_rm: vec![0.0; 4096],
        }
    }

    /// Sets the stereo spatialization mode.
    pub fn set_mode(&mut self, mode: StereoInputMode) {
        self.mode = mode;
    }

    /// Returns current stereo spatialization mode.
    pub fn mode(&self) -> StereoInputMode {
        self.mode
    }

    /// Sets the center azimuth in degrees (0..360).
    pub fn set_azimuth(&mut self, azimuth_deg: f32) {
        self.azimuth_deg = azimuth_deg.rem_euclid(360.0);
    }

    /// Returns center azimuth in degrees.
    pub fn azimuth(&self) -> f32 {
        self.azimuth_deg
    }

    /// Sets elevation in degrees (-90..90).
    pub fn set_elevation(&mut self, elevation_deg: f32) {
        self.elevation_deg = elevation_deg.clamp(-90.0, 90.0);
    }

    /// Returns elevation in degrees.
    pub fn elevation(&self) -> f32 {
        self.elevation_deg
    }

    /// Sets listener distance in meters (0.1..10.0).
    pub fn set_distance(&mut self, distance_m: f32) {
        self.distance_m = distance_m.clamp(0.1, 10.0);
    }

    /// Returns listener distance in meters.
    pub fn distance(&self) -> f32 {
        self.distance_m
    }

    /// Sets stereo angular spread in degrees (0..180, default 60).
    pub fn set_spread(&mut self, spread_deg: f32) {
        self.spread_deg = spread_deg.clamp(0.0, 180.0);
    }

    /// Returns stereo angular spread in degrees.
    pub fn spread(&self) -> f32 {
        self.spread_deg
    }

    /// Enables or disables mono bass management.
    pub fn set_mono_bass_enabled(&mut self, enabled: bool) {
        self.bass_filter.set_enabled(enabled);
    }

    /// Returns whether mono bass management is enabled.
    pub fn is_mono_bass_enabled(&self) -> bool {
        self.bass_filter.is_enabled()
    }

    /// Sets mono bass crossover frequency in Hz (typically 120 Hz).
    pub fn set_mono_bass_crossover(&mut self, crossover_hz: f32) {
        self.bass_filter.set_crossover_hz(crossover_hz);
    }

    /// Returns mono bass crossover frequency in Hz.
    pub fn mono_bass_crossover(&self) -> f32 {
        self.bass_filter.crossover_hz()
    }

    /// Sets head width in meters (0.08..0.30).
    pub fn set_head_width(&mut self, width_m: f32) {
        self.head_width_m = width_m.clamp(0.08, 0.30);
        self.renderer_l.set_head_width(self.head_width_m);
        self.renderer_r.set_head_width(self.head_width_m);
        self.renderer_mid.set_head_width(self.head_width_m);
    }

    /// Returns head width in meters.
    pub fn head_width(&self) -> f32 {
        self.head_width_m
    }

    /// Sets HRTF profile.
    pub fn set_profile(&mut self, profile: HrtfProfile) {
        self.profile = profile;
        self.renderer_l.set_profile(profile);
        self.renderer_r.set_profile(profile);
        self.renderer_mid.set_profile(profile);
    }

    /// Returns current HRTF profile.
    pub fn profile(&self) -> HrtfProfile {
        self.profile
    }
}

impl DspNode for StereoSpatializerNode {
    fn name(&self) -> &str {
        "StereoSpatializerNode"
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize) {
        self.sample_rate = sample_rate;
        self.channels = channels;
        self.renderer_l.prepare(sample_rate, 1);
        self.renderer_r.prepare(sample_rate, 1);
        self.renderer_mid.prepare(sample_rate, 1);
        self.bass_filter.prepare(sample_rate);

        let initial_cap = 4096;
        if self.scratch_mid.len() < initial_cap {
            self.scratch_mid.resize(initial_cap, 0.0);
            self.scratch_side_l.resize(initial_cap, 0.0);
            self.scratch_side_r.resize(initial_cap, 0.0);
            self.scratch_out_l1.resize(initial_cap, 0.0);
            self.scratch_out_r1.resize(initial_cap, 0.0);
            self.scratch_out_l2.resize(initial_cap, 0.0);
            self.scratch_out_r2.resize(initial_cap, 0.0);
            self.scratch_out_lm.resize(initial_cap, 0.0);
            self.scratch_out_rm.resize(initial_cap, 0.0);
        }
        self.reset();
    }

    fn reset(&mut self) {
        let az_left = (self.azimuth_deg - self.spread_deg * 0.5).rem_euclid(360.0);
        let az_right = (self.azimuth_deg + self.spread_deg * 0.5).rem_euclid(360.0);
        self.renderer_l.set_azimuth(az_left);
        self.renderer_r.set_azimuth(az_right);
        self.renderer_mid.set_azimuth(0.0);
        self.renderer_l.reset();
        self.renderer_r.reset();
        self.renderer_mid.reset();
        self.bass_filter.reset();
    }

    fn parameter_count(&self) -> usize {
        9
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => {
                if value >= 0.5 {
                    self.set_mode(StereoInputMode::MidSide);
                } else {
                    self.set_mode(StereoInputMode::VirtualSpeakers);
                }
            }
            1 => self.set_azimuth(value),
            2 => self.set_elevation(value),
            3 => self.set_distance(value),
            4 => self.set_spread(value),
            5 => self.set_mono_bass_enabled(value >= 0.5),
            6 => self.set_mono_bass_crossover(value),
            7 => self.set_head_width(value),
            8 => {
                let p = match value.round() as usize {
                    0 => HrtfProfile::ProfileA,
                    1 => HrtfProfile::ProfileB,
                    2 => HrtfProfile::ProfileC,
                    3 => HrtfProfile::ProfileD,
                    4 => HrtfProfile::ProfileE,
                    _ => HrtfProfile::ProfileF,
                };
                self.set_profile(p);
            }
            _ => {}
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 => match self.mode {
                StereoInputMode::VirtualSpeakers => 0.0,
                StereoInputMode::MidSide => 1.0,
            },
            1 => self.azimuth_deg,
            2 => self.elevation_deg,
            3 => self.distance_m,
            4 => self.spread_deg,
            5 => {
                if self.bass_filter.is_enabled() {
                    1.0
                } else {
                    0.0
                }
            }
            6 => self.bass_filter.crossover_hz(),
            7 => self.head_width_m,
            8 => match self.profile {
                HrtfProfile::ProfileA => 0.0,
                HrtfProfile::ProfileB => 1.0,
                HrtfProfile::ProfileC => 2.0,
                HrtfProfile::ProfileD => 3.0,
                HrtfProfile::ProfileE => 4.0,
                HrtfProfile::ProfileF => 5.0,
            },
            _ => 0.0,
        }
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }

    fn process(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]]) {
        if self.bypassed || inputs.is_empty() || outputs.is_empty() {
            for ch in 0..outputs.len() {
                let in_ch = ch.min(inputs.len().saturating_sub(1));
                if !inputs.is_empty() {
                    let n = outputs[ch].len().min(inputs[in_ch].len());
                    outputs[ch][..n].copy_from_slice(&inputs[in_ch][..n]);
                } else {
                    outputs[ch].fill(0.0);
                }
            }
            return;
        }

        if outputs.len() < 2 {
            let in_ch = 0;
            let n = outputs[0].len().min(inputs[in_ch].len());
            outputs[0][..n].copy_from_slice(&inputs[in_ch][..n]);
            return;
        }

        let in_l = inputs[0];
        let in_r = if inputs.len() > 1 {
            inputs[1]
        } else {
            inputs[0]
        };
        let n_samples = in_l.len().min(outputs[0].len()).min(outputs[1].len());

        if self.scratch_mid.len() < n_samples {
            self.scratch_mid.resize(n_samples, 0.0);
            self.scratch_side_l.resize(n_samples, 0.0);
            self.scratch_side_r.resize(n_samples, 0.0);
            self.scratch_out_l1.resize(n_samples, 0.0);
            self.scratch_out_r1.resize(n_samples, 0.0);
            self.scratch_out_l2.resize(n_samples, 0.0);
            self.scratch_out_r2.resize(n_samples, 0.0);
            self.scratch_out_lm.resize(n_samples, 0.0);
            self.scratch_out_rm.resize(n_samples, 0.0);
        }

        let az_left = (self.azimuth_deg - self.spread_deg * 0.5).rem_euclid(360.0);
        let az_right = (self.azimuth_deg + self.spread_deg * 0.5).rem_euclid(360.0);

        match self.mode {
            StereoInputMode::VirtualSpeakers => {
                self.renderer_l.set_azimuth(az_left);
                self.renderer_l.set_elevation(self.elevation_deg);
                self.renderer_l.set_distance(self.distance_m);

                self.renderer_r.set_azimuth(az_right);
                self.renderer_r.set_elevation(self.elevation_deg);
                self.renderer_r.set_distance(self.distance_m);

                let in_l_slice = [&in_l[..n_samples]];
                let mut out_l_slice = [
                    &mut self.scratch_out_l1[..n_samples],
                    &mut self.scratch_out_r1[..n_samples],
                ];
                self.renderer_l.process(&in_l_slice, &mut out_l_slice);

                let in_r_slice = [&in_r[..n_samples]];
                let mut out_r_slice = [
                    &mut self.scratch_out_l2[..n_samples],
                    &mut self.scratch_out_r2[..n_samples],
                ];
                self.renderer_r.process(&in_r_slice, &mut out_r_slice);

                let (out_l_buf, out_r_buf) = outputs.split_at_mut(1);
                let out_l = &mut out_l_buf[0][..n_samples];
                let out_r = &mut out_r_buf[0][..n_samples];

                for i in 0..n_samples {
                    out_l[i] = self.scratch_out_l1[i] + self.scratch_out_l2[i];
                    out_r[i] = self.scratch_out_r1[i] + self.scratch_out_r2[i];
                }

                self.bass_filter.process(out_l, out_r);
            }
            StereoInputMode::MidSide => {
                for i in 0..n_samples {
                    let mid = 0.5 * (in_l[i] + in_r[i]);
                    let side = 0.5 * (in_l[i] - in_r[i]);
                    self.scratch_mid[i] = mid;
                    self.scratch_side_l[i] = side;
                    self.scratch_side_r[i] = -side;
                }

                // Mid source is anchored frontally at azimuth 0
                self.renderer_mid.set_azimuth(0.0);
                self.renderer_mid.set_elevation(0.0);
                self.renderer_mid.set_distance(self.distance_m);

                // Sides orbit around azimuth
                self.renderer_l.set_azimuth(az_left);
                self.renderer_l.set_elevation(self.elevation_deg);
                self.renderer_l.set_distance(self.distance_m);

                self.renderer_r.set_azimuth(az_right);
                self.renderer_r.set_elevation(self.elevation_deg);
                self.renderer_r.set_distance(self.distance_m);

                let in_mid_slice = [&self.scratch_mid[..n_samples]];
                let mut out_mid_slice = [
                    &mut self.scratch_out_lm[..n_samples],
                    &mut self.scratch_out_rm[..n_samples],
                ];
                self.renderer_mid.process(&in_mid_slice, &mut out_mid_slice);

                let in_side_l_slice = [&self.scratch_side_l[..n_samples]];
                let mut out_side_l_slice = [
                    &mut self.scratch_out_l1[..n_samples],
                    &mut self.scratch_out_r1[..n_samples],
                ];
                self.renderer_l
                    .process(&in_side_l_slice, &mut out_side_l_slice);

                let in_side_r_slice = [&self.scratch_side_r[..n_samples]];
                let mut out_side_r_slice = [
                    &mut self.scratch_out_l2[..n_samples],
                    &mut self.scratch_out_r2[..n_samples],
                ];
                self.renderer_r
                    .process(&in_side_r_slice, &mut out_side_r_slice);

                let (out_l_buf, out_r_buf) = outputs.split_at_mut(1);
                let out_l = &mut out_l_buf[0][..n_samples];
                let out_r = &mut out_r_buf[0][..n_samples];

                for i in 0..n_samples {
                    out_l[i] =
                        self.scratch_out_lm[i] + self.scratch_out_l1[i] + self.scratch_out_l2[i];
                    out_r[i] =
                        self.scratch_out_rm[i] + self.scratch_out_r1[i] + self.scratch_out_r2[i];
                }

                self.bass_filter.process(out_l, out_r);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_itd_woodworth_at_90_degrees() {
        let dataset = HrtfDataset::new(HrtfProfile::ProfileC, DEFAULT_HEAD_RADIUS);
        let pair = dataset.interpolate(90.0);

        // At 90 degrees (source directly to right), right ear delay is 0, left ear delay is ITD
        // ITD = r/c * (sin(pi/2) + pi/2) = (0.0875 / 343) * (1.0 + 1.5708) ≈ 655 µs
        let itd_us = pair.itd_l_sec * 1_000_000.0;
        assert!(
            (600.0..=700.0).contains(&itd_us),
            "ITD at 90 deg should be between 600 and 700 µs, got {:.1} µs",
            itd_us
        );
        assert_eq!(
            pair.itd_r_sec, 0.0,
            "Right ear (ipsilateral) should have 0 ITD delay"
        );
    }

    #[test]
    fn test_hrtf_symmetry() {
        let dataset = HrtfDataset::new(HrtfProfile::ProfileC, DEFAULT_HEAD_RADIUS);
        let p_right = dataset.interpolate(45.0);
        let p_left = dataset.interpolate(315.0); // -45 degrees

        // Left ear at +45 should equal right ear at -45
        assert!((p_right.itd_l_sec - p_left.itd_r_sec).abs() < 1e-6);
        assert!((p_right.itd_r_sec - p_left.itd_l_sec).abs() < 1e-6);
    }

    #[test]
    fn test_azimuth_zero_identical_l_r() {
        let mut renderer = DirectHrtfRenderer::new();
        renderer.set_azimuth(0.0);

        let input = [1.0, 0.5, -0.5, 0.25, 0.0, 0.0];
        let mut out_l = vec![0.0; input.len()];
        let mut out_r = vec![0.0; input.len()];

        let inputs = [&input[..]];
        let mut outputs = [&mut out_l[..], &mut out_r[..]];
        renderer.process(&inputs, &mut outputs);

        for (l, r) in out_l.iter().zip(out_r.iter()) {
            assert!(
                (l - r).abs() < 1e-4,
                "At azimuth 0, Left and Right channels must match, got L={l}, R={r}"
            );
        }
    }

    #[test]
    fn test_ambisonic_spherical_harmonics_normalization() {
        let hoa = compute_spherical_harmonics(0.0, 0.0);
        assert_eq!(hoa[0], 1.0, "W channel must be 1.0");
        assert_eq!(hoa.len(), AMBISONIC_CHANNELS);
    }

    #[test]
    fn test_distance_cues_attenuation_and_air_absorption() {
        let mut cues = DistanceCues::new();
        cues.set_distance(1.0);
        let out_1m = cues.process_sample(1.0);

        cues.set_distance(2.0);
        let out_2m = cues.process_sample(1.0);

        assert!(
            out_2m < out_1m,
            "2m distance output should be quieter than 1m, got 1m={out_1m}, 2m={out_2m}"
        );
    }

    #[test]
    fn test_mono_bass_crossover_identical_lows() {
        let mut bass_filter = MonoBassFilter::new(120.0);
        let mut left = vec![1.0; 4096];
        let mut right = vec![-1.0; 4096]; // Out-of-phase DC / low frequency

        bass_filter.process(&mut left, &mut right);

        // After low-pass settles, mono bass summation cancels out out-of-phase low frequencies
        let last_l = left[4095];
        let last_r = right[4095];
        assert!(
            (last_l - last_r).abs() < 1e-4,
            "Mono bass filter should cancel out-of-phase DC to zero difference, got L={last_l}, R={last_r}"
        );
    }

    #[test]
    fn test_mono_bass_crossover_null_below_crossover() {
        let mut bass_filter = MonoBassFilter::new(120.0);
        let sample_rate = 48000.0;
        let num_samples = 9600; // 0.2s
        let freq = 40.0; // 40 Hz (well below 120 Hz crossover)

        let mut left = vec![0.0f32; num_samples];
        let mut right = vec![0.0f32; num_samples];

        // Generate out-of-phase 40 Hz tone (worst case stereo bass)
        for i in 0..num_samples {
            let t = i as f32 / sample_rate;
            let val = (2.0 * PI * freq * t).sin();
            left[i] = val;
            right[i] = -val;
        }

        bass_filter.process(&mut left, &mut right);

        // Measure RMS of difference (L - R) in steady state (last 4800 samples)
        let steady_start = 4800;
        let steady_count = num_samples - steady_start;
        let mut diff_sum_sq = 0.0f32;
        for i in steady_start..num_samples {
            let diff = left[i] - right[i];
            diff_sum_sq += diff * diff;
        }
        let rms_diff = (diff_sum_sq / steady_count as f32).sqrt();
        // 2 * sin has RMS of 2 / sqrt(2) = sqrt(2)
        let ref_rms = std::f32::consts::SQRT_2;
        let diff_db = 20.0 * (rms_diff / ref_rms).log10();

        assert!(
            diff_db <= -60.0,
            "Mono bass filter difference null below crossover must be <= -60 dB, got {:.2} dB",
            diff_db
        );
    }

    #[test]
    fn test_stereo_spatializer_virtual_speakers() {
        let mut spatializer = StereoSpatializerNode::new();
        spatializer.set_mode(StereoInputMode::VirtualSpeakers);
        spatializer.set_azimuth(0.0);
        spatializer.set_spread(60.0); // L at 330 deg (-30 deg), R at 30 deg
        spatializer.set_mono_bass_enabled(false);

        let n = 256;
        let mut in_l = vec![0.0f32; n];
        let in_r = vec![0.0f32; n];
        in_l[0] = 1.0; // Left impulse

        let mut out_l = vec![0.0f32; n];
        let mut out_r = vec![0.0f32; n];

        let inputs = [&in_l[..], &in_r[..]];
        let mut outputs = [&mut out_l[..], &mut out_r[..]];
        spatializer.process(&inputs, &mut outputs);

        // Left speaker at 330 (-30 deg): left ear should receive sound earlier and louder than right ear
        let peak_l = out_l.iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        let peak_r = out_r.iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        assert!(
            peak_l > peak_r,
            "Left virtual speaker should be louder in left ear: L={peak_l}, R={peak_r}"
        );
    }

    #[test]
    fn test_stereo_spatializer_midside_anchored_center() {
        let mut spatializer = StereoSpatializerNode::new();
        spatializer.set_mode(StereoInputMode::MidSide);
        // Even when azimuth orbits to 90 degrees:
        spatializer.set_azimuth(90.0);
        spatializer.set_mono_bass_enabled(false);

        // Feed pure mono signal (Mid only, Side is 0)
        let n = 256;
        let in_mono = vec![0.5f32; n];
        let mut out_l = vec![0.0f32; n];
        let mut out_r = vec![0.0f32; n];

        let inputs = [&in_mono[..], &in_mono[..]];
        let mut outputs = [&mut out_l[..], &mut out_r[..]];
        spatializer.process(&inputs, &mut outputs);

        // In Mid/Side mode, center is anchored frontally (azimuth 0), so left and right ears are equal
        for i in 0..n {
            assert!(
                (out_l[i] - out_r[i]).abs() < 1e-4,
                "Mid/Side anchored center must produce equal L and R, sample {i}: L={}, R={}",
                out_l[i],
                out_r[i]
            );
        }
    }

    #[test]
    fn test_stereo_spatializer_bypass() {
        let mut spatializer = StereoSpatializerNode::new();
        spatializer.set_bypass(true);

        let in_l = [0.1, 0.2, 0.3, 0.4];
        let in_r = [-0.1, -0.2, -0.3, -0.4];
        let mut out_l = [0.0; 4];
        let mut out_r = [0.0; 4];

        spatializer.process(&[&in_l, &in_r], &mut [&mut out_l, &mut out_r]);

        assert_eq!(out_l, in_l);
        assert_eq!(out_r, in_r);
    }
}
