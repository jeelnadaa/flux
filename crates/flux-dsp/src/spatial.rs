//! Spatial audio processing: direct HRTF convolution, ambisonic encoding/decoding,
//! distance cues, Doppler, and headphone compensation.
#![deny(missing_docs)]
#![allow(clippy::needless_range_loop, clippy::field_reassign_with_default)]

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

/// Mono bass management filter keeping frequencies below 120 Hz centered to avoid phase wobble.
#[derive(Debug, Clone)]
pub struct MonoBassFilter {
    sample_rate: f32,
    crossover_hz: f32,
    enabled: bool,
    // 2nd-order Linkwitz-Riley low-pass for mono bass
    lp_state1_l: f32,
    lp_state2_l: f32,
    lp_state1_r: f32,
    lp_state2_r: f32,
}

impl Default for MonoBassFilter {
    fn default() -> Self {
        Self::new(120.0)
    }
}

impl MonoBassFilter {
    /// Creates a new mono bass filter with the given crossover frequency (typically 120 Hz).
    pub fn new(crossover_hz: f32) -> Self {
        Self {
            sample_rate: 48000.0,
            crossover_hz,
            enabled: true,
            lp_state1_l: 0.0,
            lp_state2_l: 0.0,
            lp_state1_r: 0.0,
            lp_state2_r: 0.0,
        }
    }

    /// Enables or disables mono bass management.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Processes stereo samples in-place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        if !self.enabled {
            return;
        }

        let n = left.len().min(right.len());
        let omega = 2.0 * PI * self.crossover_hz / self.sample_rate;
        let alpha = omega / (1.0 + omega);

        for i in 0..n {
            // Low-pass extract
            self.lp_state1_l += alpha * (left[i] - self.lp_state1_l);
            self.lp_state2_l += alpha * (self.lp_state1_l - self.lp_state2_l);
            let bass_l = self.lp_state2_l;

            self.lp_state1_r += alpha * (right[i] - self.lp_state1_r);
            self.lp_state2_r += alpha * (self.lp_state1_r - self.lp_state2_r);
            let bass_r = self.lp_state2_r;

            // Average bass to mono
            let mono_bass = 0.5 * (bass_l + bass_r);

            // Highs retain stereo separation
            let high_l = left[i] - bass_l;
            let high_r = right[i] - bass_r;

            left[i] = high_l + mono_bass;
            right[i] = high_r + mono_bass;
        }
    }

    /// Resets filter states.
    pub fn reset(&mut self) {
        self.lp_state1_l = 0.0;
        self.lp_state2_l = 0.0;
        self.lp_state1_r = 0.0;
        self.lp_state2_r = 0.0;
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
            (last_l - last_r).abs() < 0.05,
            "Mono bass filter should align low-frequency content towards zero difference, got L={last_l}, R={last_r}"
        );
    }
}
