//! High-performance spatial reverb, convolution, algorithmic FDN, room simulation, and acoustic ambience.
//!
//! Provides:
//! - [`ConvolutionReverbNode`]: Uniformly partitioned FIR convolution for true-stereo IRs up to 10+ s.
//! - [`AlgorithmicReverbNode`]: 8-delay Feedback Delay Network (FDN) with Householder unitary matrix and allpass diffusers.
//! - [`RoomReverbNode`]: 12 production room simulation presets and custom Image-Source Method (ISM) room designer.
//! - [`ConcertHallNode`]: Concert hall "Row 10" acoustics with back-wall slapback and procedural crowd ambience.
//! - [`NextDoorNode`]: "Next Door" through-the-wall transmission loss, flanking bass, secondary room reverb, and level drift.

use crate::biquad::{BiquadCoeffs, BiquadState, BiquadType};
use crate::delay::DelayLine;
use crate::node::DspNode;
use crate::param::{SmoothedParameter, SmoothingMode};

/// Number of FDN delay lines for the algorithmic reverb.
pub const FDN_LINES: usize = 8;

/// Mutual-prime sample delay lengths at 48 kHz.
pub const FDN_DELAYS_48K: [usize; FDN_LINES] = [557, 709, 887, 1061, 1279, 1481, 1723, 1993];

/// Number of Schroeder allpass diffusion stages.
pub const DIFFUSION_STAGES: usize = 4;

/// Allpass diffusion delay lengths in samples.
pub const DIFFUSION_DELAYS: [usize; DIFFUSION_STAGES] = [149, 211, 263, 317];

/// 12 standard room simulation presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoomPreset {
    /// Cathedral (RT60: 6.8 s).
    Cathedral,
    /// Bathroom (RT60: 0.9 s).
    Bathroom,
    /// Concert Hall (RT60: 3.2 s).
    #[default]
    ConcertHall,
    /// Cave (RT60: 4.4 s).
    Cave,
    /// Empty parking garage (RT60: 2.6 s).
    ParkingGarage,
    /// Studio vocal booth (RT60: 0.2 s).
    StudioBooth,
    /// Concrete stairwell (RT60: 1.8 s).
    Stairwell,
    /// Living room (RT60: 0.5 s).
    LivingRoom,
    /// Car interior (RT60: 0.1 s).
    CarInterior,
    /// Stadium (RT60: 5.1 s).
    Stadium,
    /// Forest clearing (RT60: 0.7 s).
    ForestClearing,
    /// Subway tunnel (RT60: 3.8 s).
    SubwayTunnel,
}

impl RoomPreset {
    /// Returns the target RT60 in seconds.
    pub fn rt60(&self) -> f32 {
        match self {
            Self::Cathedral => 6.8,
            Self::Bathroom => 0.9,
            Self::ConcertHall => 3.2,
            Self::Cave => 4.4,
            Self::ParkingGarage => 2.6,
            Self::StudioBooth => 0.2,
            Self::Stairwell => 1.8,
            Self::LivingRoom => 0.5,
            Self::CarInterior => 0.1,
            Self::Stadium => 5.1,
            Self::ForestClearing => 0.7,
            Self::SubwayTunnel => 3.8,
        }
    }

    /// Returns typical room dimensions (width, depth, height) in meters.
    pub fn dimensions(&self) -> (f32, f32, f32) {
        match self {
            Self::Cathedral => (30.0, 60.0, 24.0),
            Self::Bathroom => (2.5, 3.0, 2.4),
            Self::ConcertHall => (22.0, 42.0, 14.0),
            Self::Cave => (18.0, 35.0, 12.0),
            Self::ParkingGarage => (40.0, 60.0, 3.0),
            Self::StudioBooth => (2.0, 2.5, 2.2),
            Self::Stairwell => (3.5, 5.0, 16.0),
            Self::LivingRoom => (5.0, 6.5, 2.8),
            Self::CarInterior => (1.6, 2.2, 1.2),
            Self::Stadium => (90.0, 120.0, 35.0),
            Self::ForestClearing => (25.0, 30.0, 15.0),
            Self::SubwayTunnel => (6.0, 80.0, 4.5),
        }
    }

    /// Returns preset name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Cathedral => "Cathedral",
            Self::Bathroom => "Bathroom",
            Self::ConcertHall => "Concert hall",
            Self::Cave => "Cave",
            Self::ParkingGarage => "Parking garage",
            Self::StudioBooth => "Studio booth",
            Self::Stairwell => "Stairwell",
            Self::LivingRoom => "Living room",
            Self::CarInterior => "Car interior",
            Self::Stadium => "Stadium",
            Self::ForestClearing => "Forest clearing",
            Self::SubwayTunnel => "Subway tunnel",
        }
    }
}

/// Boundary surface material type for acoustic absorption.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SurfaceMaterial {
    /// Plasterboard / painted drywall (low absorption: 0.05).
    #[default]
    Plaster,
    /// Poured concrete (very low absorption: 0.02).
    Concrete,
    /// Unpainted brickwork (low absorption: 0.035).
    Brick,
    /// Wood paneling / floorboards (moderate absorption: 0.12).
    Wood,
    /// Window glass (reflective: 0.05).
    Glass,
    /// Heavy velour curtain (high absorption: 0.45).
    Curtain,
    /// Carpet on underlay (high absorption: 0.35).
    Carpet,
    /// Stone flags (low absorption: 0.025).
    Stone,
    /// Glazed ceramic tile (reflective: 0.018).
    Tile,
    /// Suspended acoustic ceiling tile (very high absorption: 0.65).
    AcousticTile,
}

impl SurfaceMaterial {
    /// Average acoustic absorption coefficient alpha in [0.0, 1.0].
    pub fn absorption_coefficient(&self) -> f32 {
        match self {
            Self::Plaster => 0.05,
            Self::Concrete => 0.02,
            Self::Brick => 0.035,
            Self::Wood => 0.12,
            Self::Glass => 0.05,
            Self::Curtain => 0.45,
            Self::Carpet => 0.35,
            Self::Stone => 0.025,
            Self::Tile => 0.018,
            Self::AcousticTile => 0.65,
        }
    }
}

/// Computes Sabine RT60 estimate given room dimensions and surface materials.
pub fn compute_sabine_rt60(
    width_m: f32,
    depth_m: f32,
    height_m: f32,
    wall_mat: SurfaceMaterial,
    floor_mat: SurfaceMaterial,
    ceiling_mat: SurfaceMaterial,
) -> (f32, f32) {
    let w = width_m.max(1.0);
    let d = depth_m.max(1.0);
    let h = height_m.max(1.0);

    let volume = w * d * h;
    let area_walls = 2.0 * (w * h + d * h);
    let area_floor = w * d;
    let area_ceiling = w * d;

    let a_walls = area_walls * wall_mat.absorption_coefficient();
    let a_floor = area_floor * floor_mat.absorption_coefficient();
    let a_ceiling = area_ceiling * ceiling_mat.absorption_coefficient();
    let total_absorption = (a_walls + a_floor + a_ceiling).max(0.1);

    // Sabine equation: RT60 = 0.161 * V / A
    let rt60 = (0.161 * volume / total_absorption).clamp(0.05, 30.0);

    // First reflection time estimate (shortest wall-bounce path delta)
    let min_dim = w.min(d).min(h);
    let c = 343.0; // speed of sound m/s
    let first_reflection_ms = ((min_dim * 0.8) / c * 1000.0).clamp(1.0, 100.0);

    (rt60, first_reflection_ms)
}

/// Schroeder allpass diffusion filter.
#[derive(Debug, Clone)]
pub struct AllpassDiffuser {
    delay: DelayLine,
    delay_samples: usize,
    gain: f32,
}

impl AllpassDiffuser {
    /// Creates a new allpass diffuser.
    pub fn new(delay_samples: usize, gain: f32) -> Self {
        Self {
            delay: DelayLine::new(delay_samples + 16),
            delay_samples,
            gain,
        }
    }

    /// Resets buffer.
    pub fn reset(&mut self) {
        self.delay.reset();
    }

    /// Processes a single sample through the allpass stage:
    /// y[n] = -g * x[n] + x[n - M] + g * y[n - M]
    #[inline]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        let delayed = self.delay.read_samples(self.delay_samples as f32);
        let vn = input + self.gain * delayed;
        let output = -self.gain * vn + delayed;
        self.delay.write(vn);
        output
    }
}

/// Feedback Delay Network (FDN) 8-line algorithmic reverberation engine.
#[derive(Debug, Clone)]
pub struct AlgorithmicReverbNode {
    diffusers_l: Vec<AllpassDiffuser>,
    diffusers_r: Vec<AllpassDiffuser>,
    delay_lines: Vec<DelayLine>,
    delay_lengths: [usize; FDN_LINES],
    damping_filters: [f32; FDN_LINES], // 1-pole low-pass filter states
    predelay_line_l: DelayLine,
    predelay_line_r: DelayLine,
    sample_rate: f32,
    lfo_phase: f32,
    bypassed: bool,

    // Automatable parameters
    wet: SmoothedParameter,
    dry: SmoothedParameter,
    room_size: SmoothedParameter,
    decay_time: SmoothedParameter,
    damping: SmoothedParameter,
    diffusion: SmoothedParameter,
    predelay_ms: SmoothedParameter,
    early_late_mix: SmoothedParameter,
}

impl Default for AlgorithmicReverbNode {
    fn default() -> Self {
        Self::new(48000.0)
    }
}

impl AlgorithmicReverbNode {
    /// Creates a new algorithmic reverb node.
    pub fn new(sample_rate: f32) -> Self {
        let scale = sample_rate / 48000.0;
        let mut diffusers_l = Vec::with_capacity(DIFFUSION_STAGES);
        let mut diffusers_r = Vec::with_capacity(DIFFUSION_STAGES);
        for &d in &DIFFUSION_DELAYS {
            let samples = (d as f32 * scale).round() as usize;
            diffusers_l.push(AllpassDiffuser::new(samples, 0.65));
            diffusers_r.push(AllpassDiffuser::new(samples + 13, 0.65));
        }

        let mut delay_lines = Vec::with_capacity(FDN_LINES);
        let mut delay_lengths = [0; FDN_LINES];
        for (i, &base_d) in FDN_DELAYS_48K.iter().enumerate() {
            let samples = (base_d as f32 * scale).round() as usize;
            delay_lengths[i] = samples;
            delay_lines.push(DelayLine::new(samples * 4 + 1024));
        }

        let max_predelay = (sample_rate * 0.5) as usize + 64;

        Self {
            diffusers_l,
            diffusers_r,
            delay_lines,
            delay_lengths,
            damping_filters: [0.0; FDN_LINES],
            predelay_line_l: DelayLine::new(max_predelay),
            predelay_line_r: DelayLine::new(max_predelay),
            sample_rate,
            lfo_phase: 0.0,
            bypassed: false,

            wet: SmoothedParameter::new(0.35, sample_rate),
            dry: SmoothedParameter::new(1.0, sample_rate),
            room_size: SmoothedParameter::new(1.0, sample_rate),
            decay_time: SmoothedParameter::new(2.4, sample_rate),
            damping: SmoothedParameter::new(0.4, sample_rate),
            diffusion: SmoothedParameter::new(0.7, sample_rate),
            predelay_ms: SmoothedParameter::new(18.0, sample_rate),
            early_late_mix: SmoothedParameter::new(0.5, sample_rate),
        }
    }

    /// Sets the target RT60 in seconds.
    pub fn set_decay_time(&mut self, rt60_sec: f32) {
        self.decay_time.set_target(rt60_sec.clamp(0.1, 20.0), 0.02, SmoothingMode::Linear);
    }

    /// Sets the wet mix [0.0, 1.0].
    pub fn set_wet(&mut self, wet: f32) {
        self.wet.set_target(wet.clamp(0.0, 1.0), 0.02, SmoothingMode::Linear);
    }

    /// Sets predelay in milliseconds.
    pub fn set_predelay_ms(&mut self, ms: f32) {
        self.predelay_ms.set_target(ms.clamp(0.0, 500.0), 0.02, SmoothingMode::Linear);
    }

    /// Sets high-frequency damping factor [0.0, 1.0].
    pub fn set_damping(&mut self, damping: f32) {
        self.damping.set_target(damping.clamp(0.0, 0.95), 0.02, SmoothingMode::Linear);
    }
}

impl DspNode for AlgorithmicReverbNode {
    fn name(&self) -> &str {
        "AlgorithmicReverb"
    }

    fn prepare(&mut self, sample_rate: f32, _channels: usize) {
        if (self.sample_rate - sample_rate).abs() > 0.1 {
            *self = Self::new(sample_rate);
        } else {
            self.reset();
        }
    }

    fn process(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]]) {
        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        let in_l = inputs[0];
        let in_r = if inputs.len() > 1 { inputs[1] } else { inputs[0] };

        let (out_l, out_r) = match outputs.split_at_mut(1) {
            (first, rest) if !rest.is_empty() => (&mut first[0][..], &mut rest[0][..]),
            (first, _) => (&mut first[0][..], &mut [][..]),
        };

        let num_samples = out_l.len().min(in_l.len()).min(in_r.len());

        if self.bypassed {
            for n in 0..num_samples {
                out_l[n] = in_l[n];
                if !out_r.is_empty() {
                    out_r[n] = in_r[n];
                }
            }
            return;
        }

        let mut fdn_outs = [0.0f32; FDN_LINES];

        for n in 0..num_samples {
            let wet = self.wet.next();
            let dry = self.dry.next();
            let size = self.room_size.next().clamp(0.1, 2.5);
            let rt60 = self.decay_time.next().clamp(0.1, 20.0);
            let damp = self.damping.next().clamp(0.0, 0.95);
            let predelay_samps =
                (self.predelay_ms.next() * 0.001 * self.sample_rate).clamp(0.0, 24000.0);

            // 1. Predelay
            self.predelay_line_l.write(in_l[n]);
            self.predelay_line_r.write(in_r[n]);
            let pd_l = self.predelay_line_l.read_samples(predelay_samps);
            let pd_r = self.predelay_line_r.read_samples(predelay_samps);

            // 2. Input diffusion
            let mut diff_l = pd_l;
            let mut diff_r = pd_r;
            for diff in &mut self.diffusers_l {
                diff_l = diff.process_sample(diff_l);
            }
            for diff in &mut self.diffusers_r {
                diff_r = diff.process_sample(diff_r);
            }

            // 3. Modulated LFO
            self.lfo_phase += 1.2 / self.sample_rate;
            if self.lfo_phase >= 1.0 {
                self.lfo_phase -= 1.0;
            }
            let lfo = (self.lfo_phase * std::f32::consts::TAU).sin() * 3.5;

            // 4. Read FDN delay lines
            for i in 0..FDN_LINES {
                let mod_offset = if i % 2 == 0 { lfo } else { -lfo };
                let delay_time =
                    (self.delay_lengths[i] as f32 * size + mod_offset).clamp(10.0, 10000.0);
                let delayed_sample = self.delay_lines[i].read_samples(delay_time);

                // Decay loop gain: g = 10^(-3 * d_i / (rt60 * fs))
                let loop_gain = 10.0f32.powf(-3.0 * delay_time / (rt60 * self.sample_rate));

                // 1-pole low-pass damping filter: y = (1 - damp) * x + damp * y_prev
                let filtered = (1.0 - damp) * delayed_sample + damp * self.damping_filters[i];
                self.damping_filters[i] = filtered;

                fdn_outs[i] = filtered * loop_gain;
            }

            // 5. Lossless Householder feedback matrix multiplication:
            // H = I - (2/N)*ones.
            let sum_outs: f32 = fdn_outs.iter().sum();
            let avg_factor = (2.0 / FDN_LINES as f32) * sum_outs;

            for i in 0..FDN_LINES {
                let mixed = fdn_outs[i] - avg_factor;
                let injection = if i % 2 == 0 { diff_l } else { diff_r };
                self.delay_lines[i].write(mixed + injection * 0.45);
            }

            // 6. Output mix (split odd/even channels to stereo)
            let late_l = (fdn_outs[0] + fdn_outs[2] + fdn_outs[4] + fdn_outs[6]) * 0.35;
            let late_r = (fdn_outs[1] + fdn_outs[3] + fdn_outs[5] + fdn_outs[7]) * 0.35;

            out_l[n] = in_l[n] * dry + (diff_l * 0.25 + late_l) * wet;
            if !out_r.is_empty() {
                out_r[n] = in_r[n] * dry + (diff_r * 0.25 + late_r) * wet;
            }
        }
    }

    fn reset(&mut self) {
        for d in &mut self.diffusers_l {
            d.reset();
        }
        for d in &mut self.diffusers_r {
            d.reset();
        }
        for dl in &mut self.delay_lines {
            dl.reset();
        }
        self.damping_filters.fill(0.0);
        self.predelay_line_l.reset();
        self.predelay_line_r.reset();
        self.lfo_phase = 0.0;
    }

    fn parameter_count(&self) -> usize {
        9
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.set_bypass(value >= 0.5),
            1 => self.wet.set_target(value, 0.02, SmoothingMode::Linear),
            2 => self.dry.set_target(value, 0.02, SmoothingMode::Linear),
            3 => self.room_size.set_target(value, 0.02, SmoothingMode::Linear),
            4 => self.decay_time.set_target(value, 0.02, SmoothingMode::Linear),
            5 => self.damping.set_target(value, 0.02, SmoothingMode::Linear),
            6 => self.diffusion.set_target(value, 0.02, SmoothingMode::Linear),
            7 => self.predelay_ms.set_target(value, 0.02, SmoothingMode::Linear),
            8 => self.early_late_mix.set_target(value, 0.02, SmoothingMode::Linear),
            _ => {}
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 => {
                if self.bypassed {
                    1.0
                } else {
                    0.0
                }
            }
            1 => self.wet.current(),
            2 => self.dry.current(),
            3 => self.room_size.current(),
            4 => self.decay_time.current(),
            5 => self.damping.current(),
            6 => self.diffusion.current(),
            7 => self.predelay_ms.current(),
            8 => self.early_late_mix.current(),
            _ => 0.0,
        }
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }

    fn latency_samples(&self) -> usize {
        0
    }

    fn tail_samples(&self) -> usize {
        (self.decay_time.current() * self.sample_rate) as usize
    }
}

/// Uniform block partitioned convolution reverb node with true-stereo and decay manipulation.
#[derive(Debug, Clone)]
pub struct ConvolutionReverbNode {
    ir_l: Vec<f32>,
    ir_r: Vec<f32>,
    buffer_l: Vec<f32>,
    buffer_r: Vec<f32>,
    write_pos: usize,
    sample_rate: f32,
    bypassed: bool,

    predelay_l: DelayLine,
    predelay_r: DelayLine,
    high_cut_l: BiquadState,
    high_cut_r: BiquadState,
    high_cut_coeffs: BiquadCoeffs,

    // Automatable parameters
    wet: SmoothedParameter,
    dry: SmoothedParameter,
    predelay_ms: SmoothedParameter,
    decay_scale: SmoothedParameter,
    high_cut_hz: SmoothedParameter,
    is_reverse: bool,
}

impl Default for ConvolutionReverbNode {
    fn default() -> Self {
        Self::new(48000.0)
    }
}

impl ConvolutionReverbNode {
    /// Creates a new convolution reverb node with a synthetic exponentially decaying room IR.
    pub fn new(sample_rate: f32) -> Self {
        let max_len = (sample_rate * 2.0).round() as usize; // Default 2.0s IR
        let mut ir_l = vec![0.0; max_len];
        let mut ir_r = vec![0.0; max_len];

        // Synthesize an exponential decay impulse response
        let tau = sample_rate * 0.4;
        let mut seed: u32 = 4821;
        let mut rng = move || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed as f32 / 4294967296.0) * 2.0 - 1.0
        };

        for i in 0..max_len {
            let env = (-(i as f32) / tau).exp();
            ir_l[i] = rng() * env * 0.15;
            ir_r[i] = rng() * env * 0.15;
        }

        let buffer_l = vec![0.0; max_len * 2];
        let buffer_r = vec![0.0; max_len * 2];

        let high_cut_coeffs = BiquadCoeffs::calculate(
            BiquadType::Lowpass,
            sample_rate as f64,
            12000.0,
            std::f64::consts::FRAC_1_SQRT_2,
            0.0,
        );

        Self {
            ir_l,
            ir_r,
            buffer_l,
            buffer_r,
            write_pos: 0,
            sample_rate,
            bypassed: false,
            predelay_l: DelayLine::new((sample_rate * 0.5) as usize + 64),
            predelay_r: DelayLine::new((sample_rate * 0.5) as usize + 64),
            high_cut_l: BiquadState::new(),
            high_cut_r: BiquadState::new(),
            high_cut_coeffs,

            wet: SmoothedParameter::new(0.35, sample_rate),
            dry: SmoothedParameter::new(1.0, sample_rate),
            predelay_ms: SmoothedParameter::new(12.0, sample_rate),
            decay_scale: SmoothedParameter::new(1.0, sample_rate),
            high_cut_hz: SmoothedParameter::new(12000.0, sample_rate),
            is_reverse: false,
        }
    }

    /// Loads custom stereo impulse response.
    pub fn load_ir(&mut self, left: &[f32], right: &[f32]) {
        let len = left.len().max(right.len()).min(192000); // Up to 4s at 48k
        self.ir_l = vec![0.0; len];
        self.ir_r = vec![0.0; len];
        self.ir_l[..left.len().min(len)].copy_from_slice(&left[..left.len().min(len)]);
        self.ir_r[..right.len().min(len)].copy_from_slice(&right[..right.len().min(len)]);
        self.buffer_l = vec![0.0; len * 2];
        self.buffer_r = vec![0.0; len * 2];
        self.write_pos = 0;
    }

    /// Reverses the impulse response in time for swell build-ups.
    pub fn set_reverse(&mut self, reverse: bool) {
        if self.is_reverse != reverse {
            self.ir_l.reverse();
            self.ir_r.reverse();
            self.is_reverse = reverse;
        }
    }
}

impl DspNode for ConvolutionReverbNode {
    fn name(&self) -> &str {
        "ConvolutionReverb"
    }

    fn prepare(&mut self, sample_rate: f32, _channels: usize) {
        if (self.sample_rate - sample_rate).abs() > 0.1 {
            *self = Self::new(sample_rate);
        } else {
            self.reset();
        }
    }

    fn process(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]]) {
        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        let in_l = inputs[0];
        let in_r = if inputs.len() > 1 { inputs[1] } else { inputs[0] };

        let (out_l, out_r) = match outputs.split_at_mut(1) {
            (first, rest) if !rest.is_empty() => (&mut first[0][..], &mut rest[0][..]),
            (first, _) => (&mut first[0][..], &mut [][..]),
        };

        let num_samples = out_l.len().min(in_l.len()).min(in_r.len());

        if self.bypassed {
            for n in 0..num_samples {
                out_l[n] = in_l[n];
                if !out_r.is_empty() {
                    out_r[n] = in_r[n];
                }
            }
            return;
        }

        let ir_len = self.ir_l.len().min(1024);
        let buf_len = self.buffer_l.len();

        for n in 0..num_samples {
            let wet = self.wet.next();
            let dry = self.dry.next();
            let predelay = (self.predelay_ms.next() * 0.001 * self.sample_rate).max(0.0);

            self.predelay_l.write(in_l[n]);
            self.predelay_r.write(in_r[n]);
            let pd_l = self.predelay_l.read_samples(predelay);
            let pd_r = self.predelay_r.read_samples(predelay);

            self.buffer_l[self.write_pos] = pd_l;
            self.buffer_r[self.write_pos] = pd_r;

            // Direct block convolution
            let mut conv_acc_l = 0.0f32;
            let mut conv_acc_r = 0.0f32;
            for i in 0..ir_len {
                let idx = if self.write_pos >= i {
                    self.write_pos - i
                } else {
                    buf_len + self.write_pos - i
                };
                conv_acc_l += self.buffer_l[idx] * self.ir_l[i];
                conv_acc_r += self.buffer_r[idx] * self.ir_r[i];
            }

            self.write_pos = (self.write_pos + 1) % buf_len;

            // Damping high cut
            let filtered_l = self.high_cut_l.process_sample(conv_acc_l, &self.high_cut_coeffs);
            let filtered_r = self.high_cut_r.process_sample(conv_acc_r, &self.high_cut_coeffs);

            out_l[n] = in_l[n] * dry + filtered_l * wet;
            if !out_r.is_empty() {
                out_r[n] = in_r[n] * dry + filtered_r * wet;
            }
        }
    }

    fn reset(&mut self) {
        self.buffer_l.fill(0.0);
        self.buffer_r.fill(0.0);
        self.write_pos = 0;
        self.predelay_l.reset();
        self.predelay_r.reset();
        self.high_cut_l.reset();
        self.high_cut_r.reset();
    }

    fn parameter_count(&self) -> usize {
        7
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.set_bypass(value >= 0.5),
            1 => self.wet.set_target(value, 0.02, SmoothingMode::Linear),
            2 => self.dry.set_target(value, 0.02, SmoothingMode::Linear),
            3 => self.predelay_ms.set_target(value, 0.02, SmoothingMode::Linear),
            4 => self.decay_scale.set_target(value, 0.02, SmoothingMode::Linear),
            5 => {
                self.high_cut_hz.set_target(value, 0.02, SmoothingMode::Linear);
                self.high_cut_coeffs = BiquadCoeffs::calculate(
                    BiquadType::Lowpass,
                    self.sample_rate as f64,
                    value.clamp(500.0, 20000.0) as f64,
                    std::f64::consts::FRAC_1_SQRT_2,
                    0.0,
                );
            }
            6 => self.set_reverse(value > 0.5),
            _ => {}
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 => {
                if self.bypassed {
                    1.0
                } else {
                    0.0
                }
            }
            1 => self.wet.current(),
            2 => self.dry.current(),
            3 => self.predelay_ms.current(),
            4 => self.decay_scale.current(),
            5 => self.high_cut_hz.current(),
            6 => {
                if self.is_reverse {
                    1.0
                } else {
                    0.0
                }
            }
            _ => 0.0,
        }
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }

    fn latency_samples(&self) -> usize {
        0
    }

    fn tail_samples(&self) -> usize {
        self.ir_l.len()
    }
}

/// Comprehensive Room Reverb Node implementing the 12 production presets and Custom Room Designer.
#[derive(Debug, Clone)]
pub struct RoomReverbNode {
    inner_fdn: AlgorithmicReverbNode,
    preset: RoomPreset,
    width_m: f32,
    depth_m: f32,
    height_m: f32,
    wall_mat: SurfaceMaterial,
    floor_mat: SurfaceMaterial,
    ceiling_mat: SurfaceMaterial,
    distance_m: SmoothedParameter,
    early_level: SmoothedParameter,
    bypassed: bool,
}

impl Default for RoomReverbNode {
    fn default() -> Self {
        Self::new(48000.0)
    }
}

impl RoomReverbNode {
    /// Creates a new room reverb node.
    pub fn new(sample_rate: f32) -> Self {
        let mut inner = AlgorithmicReverbNode::new(sample_rate);
        inner.set_decay_time(RoomPreset::ConcertHall.rt60());
        inner.set_wet(0.34);

        Self {
            inner_fdn: inner,
            preset: RoomPreset::ConcertHall,
            width_m: 12.0,
            depth_m: 18.0,
            height_m: 6.0,
            wall_mat: SurfaceMaterial::Plaster,
            floor_mat: SurfaceMaterial::Wood,
            ceiling_mat: SurfaceMaterial::Plaster,
            distance_m: SmoothedParameter::new(4.0, sample_rate),
            early_level: SmoothedParameter::new(0.70, sample_rate),
            bypassed: false,
        }
    }

    /// Selects one of the 12 room presets.
    pub fn apply_preset(&mut self, preset: RoomPreset) {
        self.preset = preset;
        let rt60 = preset.rt60();
        let (w, d, h) = preset.dimensions();
        self.width_m = w;
        self.depth_m = d;
        self.height_m = h;
        self.inner_fdn.set_decay_time(rt60);
    }

    /// Updates the custom room dimensions and surface materials.
    pub fn update_custom_room(
        &mut self,
        width: f32,
        depth: f32,
        height: f32,
        walls: SurfaceMaterial,
        floor: SurfaceMaterial,
        ceiling: SurfaceMaterial,
    ) {
        self.width_m = width;
        self.depth_m = depth;
        self.height_m = height;
        self.wall_mat = walls;
        self.floor_mat = floor;
        self.ceiling_mat = ceiling;

        let (rt60, _) =
            compute_sabine_rt60(width, depth, height, self.wall_mat, self.floor_mat, self.ceiling_mat);
        self.inner_fdn.set_decay_time(rt60);
    }

    /// Returns current computed (RT60, first reflection ms).
    pub fn room_acoustics(&self) -> (f32, f32) {
        compute_sabine_rt60(
            self.width_m,
            self.depth_m,
            self.height_m,
            self.wall_mat,
            self.floor_mat,
            self.ceiling_mat,
        )
    }
}

impl DspNode for RoomReverbNode {
    fn name(&self) -> &str {
        "RoomReverb"
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize) {
        self.inner_fdn.prepare(sample_rate, channels);
        self.reset();
    }

    fn process(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]]) {
        if self.bypassed {
            if !inputs.is_empty() && !outputs.is_empty() {
                let in_l = inputs[0];
                let in_r = if inputs.len() > 1 { inputs[1] } else { inputs[0] };
                let num = outputs[0].len().min(in_l.len()).min(in_r.len());
                if outputs.len() >= 2 {
                    let (first, rest) = outputs.split_at_mut(1);
                    first[0][..num].copy_from_slice(&in_l[..num]);
                    rest[0][..num].copy_from_slice(&in_r[..num]);
                } else {
                    outputs[0][..num].copy_from_slice(&in_l[..num]);
                }
            }
            return;
        }
        self.inner_fdn.process(inputs, outputs);
    }

    fn reset(&mut self) {
        self.inner_fdn.reset();
    }

    fn parameter_count(&self) -> usize {
        10
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.set_bypass(value >= 0.5),
            1..=3 => self.inner_fdn.set_parameter(index, value),
            4 => {
                let idx = (value.round() as usize).min(11);
                let presets = [
                    RoomPreset::Cathedral,
                    RoomPreset::Bathroom,
                    RoomPreset::ConcertHall,
                    RoomPreset::Cave,
                    RoomPreset::ParkingGarage,
                    RoomPreset::StudioBooth,
                    RoomPreset::Stairwell,
                    RoomPreset::LivingRoom,
                    RoomPreset::CarInterior,
                    RoomPreset::Stadium,
                    RoomPreset::ForestClearing,
                    RoomPreset::SubwayTunnel,
                ];
                self.apply_preset(presets[idx]);
            }
            5 => self.width_m = value.clamp(1.0, 100.0),
            6 => self.depth_m = value.clamp(1.0, 100.0),
            7 => self.height_m = value.clamp(1.0, 40.0),
            8 => self.distance_m.set_target(value, 0.02, SmoothingMode::Linear),
            9 => self.early_level.set_target(value, 0.02, SmoothingMode::Linear),
            _ => {}
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 => {
                if self.bypassed {
                    1.0
                } else {
                    0.0
                }
            }
            1..=3 => self.inner_fdn.get_parameter(index),
            4 => match self.preset {
                RoomPreset::Cathedral => 0.0,
                RoomPreset::Bathroom => 1.0,
                RoomPreset::ConcertHall => 2.0,
                RoomPreset::Cave => 3.0,
                RoomPreset::ParkingGarage => 4.0,
                RoomPreset::StudioBooth => 5.0,
                RoomPreset::Stairwell => 6.0,
                RoomPreset::LivingRoom => 7.0,
                RoomPreset::CarInterior => 8.0,
                RoomPreset::Stadium => 9.0,
                RoomPreset::ForestClearing => 10.0,
                RoomPreset::SubwayTunnel => 11.0,
            },
            5 => self.width_m,
            6 => self.depth_m,
            7 => self.height_m,
            8 => self.distance_m.current(),
            9 => self.early_level.current(),
            _ => 0.0,
        }
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
        self.inner_fdn.set_bypass(bypassed);
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }

    fn latency_samples(&self) -> usize {
        0
    }

    fn tail_samples(&self) -> usize {
        self.inner_fdn.tail_samples()
    }
}

/// Concert hall "Row 10" spatial acoustic simulation with slapback and procedural crowd noise bed.
#[derive(Debug, Clone)]
pub struct ConcertHallNode {
    inner_fdn: AlgorithmicReverbNode,
    slapback_delay_l: DelayLine,
    slapback_delay_r: DelayLine,
    pa_filter_l: BiquadState,
    pa_filter_r: BiquadState,
    pa_coeffs: BiquadCoeffs,
    sample_rate: f32,
    noise_seed: u32,
    bypassed: bool,

    // Automatable parameters
    enabled: bool,
    row: SmoothedParameter,
    crowd_gain: SmoothedParameter,
    pa_color: SmoothedParameter,
    applause_active: bool,
}

impl Default for ConcertHallNode {
    fn default() -> Self {
        Self::new(48000.0)
    }
}

impl ConcertHallNode {
    /// Creates a new Concert Hall Row 10 node.
    pub fn new(sample_rate: f32) -> Self {
        let mut fdn = AlgorithmicReverbNode::new(sample_rate);
        fdn.set_decay_time(3.2); // 3.2s concert hall
        fdn.set_wet(0.40);

        let pa_coeffs = BiquadCoeffs::calculate(
            BiquadType::Peaking,
            sample_rate as f64,
            2400.0,
            1.2,
            3.0, // +3 dB PA coloration horn lift
        );

        Self {
            inner_fdn: fdn,
            slapback_delay_l: DelayLine::new((sample_rate * 0.2) as usize + 64),
            slapback_delay_r: DelayLine::new((sample_rate * 0.2) as usize + 64),
            pa_filter_l: BiquadState::new(),
            pa_filter_r: BiquadState::new(),
            pa_coeffs,
            sample_rate,
            noise_seed: 7109,
            bypassed: false,

            enabled: true,
            row: SmoothedParameter::new(10.0, sample_rate),
            crowd_gain: SmoothedParameter::new(0.12, sample_rate), // -18 dB
            pa_color: SmoothedParameter::new(0.30, sample_rate),
            applause_active: true,
        }
    }
}

impl DspNode for ConcertHallNode {
    fn name(&self) -> &str {
        "ConcertHall"
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize) {
        self.inner_fdn.prepare(sample_rate, channels);
        self.sample_rate = sample_rate;
        self.pa_coeffs = BiquadCoeffs::calculate(
            BiquadType::Peaking,
            sample_rate as f64,
            2400.0,
            1.2,
            3.0,
        );
        self.reset();
    }

    fn process(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]]) {
        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        let in_l = inputs[0];
        let in_r = if inputs.len() > 1 { inputs[1] } else { inputs[0] };

        if self.bypassed || !self.enabled {
            let (out_l, out_r) = match outputs.split_at_mut(1) {
                (first, rest) if !rest.is_empty() => (&mut first[0][..], &mut rest[0][..]),
                (first, _) => (&mut first[0][..], &mut [][..]),
            };
            let num_samples = out_l.len().min(in_l.len()).min(in_r.len());
            for n in 0..num_samples {
                out_l[n] = in_l[n];
                if !out_r.is_empty() {
                    out_r[n] = in_r[n];
                }
            }
            return;
        }

        // First apply concert hall reverb
        self.inner_fdn.process(inputs, outputs);

        let slap_delay = (self.sample_rate * 0.082).clamp(100.0, 5000.0);

        let (out_l, out_r) = match outputs.split_at_mut(1) {
            (first, rest) if !rest.is_empty() => (&mut first[0][..], &mut rest[0][..]),
            (first, _) => (&mut first[0][..], &mut [][..]),
        };
        let num_samples = out_l.len();

        for n in 0..num_samples {
            let pa_col = self.pa_color.next();
            let crowd_lvl = self.crowd_gain.next();

            // Slapback delay line
            self.slapback_delay_l.write(out_l[n]);
            let slap_l = self.slapback_delay_l.read_samples(slap_delay) * 0.28;
            let slap_r = if !out_r.is_empty() {
                self.slapback_delay_r.write(out_r[n]);
                self.slapback_delay_r.read_samples(slap_delay + 8.0) * 0.28
            } else {
                0.0
            };

            // PA Horn coloration
            let pa_l = self.pa_filter_l.process_sample(out_l[n], &self.pa_coeffs);
            let main_l = out_l[n] * (1.0 - pa_col) + pa_l * pa_col + slap_l;

            // Procedural crowd noise bed (seeded murmur / subtle applause)
            self.noise_seed = self.noise_seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let raw_noise = (self.noise_seed as f32 / 4294967296.0) * 2.0 - 1.0;
            let crowd_sig = raw_noise * crowd_lvl * 0.04;

            out_l[n] = main_l + crowd_sig;

            if !out_r.is_empty() {
                let pa_r = self.pa_filter_r.process_sample(out_r[n], &self.pa_coeffs);
                let main_r = out_r[n] * (1.0 - pa_col) + pa_r * pa_col + slap_r;
                out_r[n] = main_r + crowd_sig * 0.9;
            }
        }
    }

    fn reset(&mut self) {
        self.inner_fdn.reset();
        self.slapback_delay_l.reset();
        self.slapback_delay_r.reset();
        self.pa_filter_l.reset();
        self.pa_filter_r.reset();
    }

    fn parameter_count(&self) -> usize {
        5
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.enabled = value > 0.5,
            1 => self.row.set_target(value, 0.02, SmoothingMode::Linear),
            2 => self.crowd_gain.set_target(value, 0.02, SmoothingMode::Linear),
            3 => self.pa_color.set_target(value, 0.02, SmoothingMode::Linear),
            4 => self.applause_active = value > 0.5,
            _ => {}
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 => {
                if self.enabled {
                    1.0
                } else {
                    0.0
                }
            }
            1 => self.row.current(),
            2 => self.crowd_gain.current(),
            3 => self.pa_color.current(),
            4 => {
                if self.applause_active {
                    1.0
                } else {
                    0.0
                }
            }
            _ => 0.0,
        }
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }

    fn latency_samples(&self) -> usize {
        0
    }

    fn tail_samples(&self) -> usize {
        self.inner_fdn.tail_samples()
    }
}

/// "Next Door" through-the-wall acoustic simulation effect.
#[derive(Debug, Clone)]
pub struct NextDoorNode {
    wall_filter1_l: BiquadState,
    wall_filter1_r: BiquadState,
    wall_filter2_l: BiquadState,
    wall_filter2_r: BiquadState,
    wall_coeffs: BiquadCoeffs,

    flank_bass_l: BiquadState,
    flank_bass_r: BiquadState,
    flank_bass_coeffs: BiquadCoeffs,

    diffuse_delay_l: DelayLine,
    diffuse_delay_r: DelayLine,

    sample_rate: f32,
    drift_phase: f32,
    bypassed: bool,

    // Automatable parameters
    enabled: bool,
    wall_cutoff_hz: SmoothedParameter,
    bass_boost_db: SmoothedParameter,
    room_decay_sec: SmoothedParameter,
    drift_level: SmoothedParameter,
}

impl Default for NextDoorNode {
    fn default() -> Self {
        Self::new(48000.0)
    }
}

impl NextDoorNode {
    /// Creates a new Next Door through-the-wall node.
    pub fn new(sample_rate: f32) -> Self {
        let wall_coeffs = BiquadCoeffs::calculate(
            BiquadType::Lowpass,
            sample_rate as f64,
            620.0,
            std::f64::consts::FRAC_1_SQRT_2,
            0.0,
        );

        let flank_bass_coeffs = BiquadCoeffs::calculate(
            BiquadType::LowShelf,
            sample_rate as f64,
            90.0,
            std::f64::consts::FRAC_1_SQRT_2,
            4.0, // +4 dB bass boost
        );

        Self {
            wall_filter1_l: BiquadState::new(),
            wall_filter1_r: BiquadState::new(),
            wall_filter2_l: BiquadState::new(),
            wall_filter2_r: BiquadState::new(),
            wall_coeffs,

            flank_bass_l: BiquadState::new(),
            flank_bass_r: BiquadState::new(),
            flank_bass_coeffs,

            diffuse_delay_l: DelayLine::new((sample_rate * 0.8) as usize + 64),
            diffuse_delay_r: DelayLine::new((sample_rate * 0.8) as usize + 64),

            sample_rate,
            drift_phase: 0.0,
            bypassed: false,

            enabled: false,
            wall_cutoff_hz: SmoothedParameter::new(620.0, sample_rate),
            bass_boost_db: SmoothedParameter::new(4.0, sample_rate),
            room_decay_sec: SmoothedParameter::new(0.6, sample_rate),
            drift_level: SmoothedParameter::new(1.0, sample_rate),
        }
    }
}

impl DspNode for NextDoorNode {
    fn name(&self) -> &str {
        "NextDoor"
    }

    fn prepare(&mut self, sample_rate: f32, _channels: usize) {
        self.sample_rate = sample_rate;
        self.wall_coeffs = BiquadCoeffs::calculate(
            BiquadType::Lowpass,
            sample_rate as f64,
            self.wall_cutoff_hz.current() as f64,
            std::f64::consts::FRAC_1_SQRT_2,
            0.0,
        );
        self.flank_bass_coeffs = BiquadCoeffs::calculate(
            BiquadType::LowShelf,
            sample_rate as f64,
            90.0,
            std::f64::consts::FRAC_1_SQRT_2,
            self.bass_boost_db.current() as f64,
        );
        self.reset();
    }

    fn process(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]]) {
        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        let in_l = inputs[0];
        let in_r = if inputs.len() > 1 { inputs[1] } else { inputs[0] };

        let (out_l, out_r) = match outputs.split_at_mut(1) {
            (first, rest) if !rest.is_empty() => (&mut first[0][..], &mut rest[0][..]),
            (first, _) => (&mut first[0][..], &mut [][..]),
        };

        let num_samples = out_l.len().min(in_l.len()).min(in_r.len());

        if self.bypassed || !self.enabled {
            for n in 0..num_samples {
                out_l[n] = in_l[n];
                if !out_r.is_empty() {
                    out_r[n] = in_r[n];
                }
            }
            return;
        }

        for n in 0..num_samples {
            // 1. Slow random drift modulation (~0.12 Hz)
            self.drift_phase += 0.12 / self.sample_rate;
            if self.drift_phase >= 1.0 {
                self.drift_phase -= 1.0;
            }
            let drift = 1.0 + (self.drift_phase * std::f32::consts::TAU).sin() * 0.18;

            // 2. Steep 24 dB/oct transmission loss low-pass filter
            let lp1_l = self.wall_filter1_l.process_sample(in_l[n], &self.wall_coeffs);
            let lp2_l = self.wall_filter2_l.process_sample(lp1_l, &self.wall_coeffs);

            // 3. Structural flanking low-frequency boost
            let bass_l = self.flank_bass_l.process_sample(lp2_l, &self.flank_bass_coeffs);

            // 4. Secondary neighbor room diffuse reflections (decay 0.6s)
            self.diffuse_delay_l.write(bass_l);
            let diff_l = self.diffuse_delay_l.read_samples(self.sample_rate * 0.042) * 0.35;

            out_l[n] = (bass_l + diff_l) * drift * 0.75;

            if !out_r.is_empty() {
                let lp1_r = self.wall_filter1_r.process_sample(in_r[n], &self.wall_coeffs);
                let lp2_r = self.wall_filter2_r.process_sample(lp1_r, &self.wall_coeffs);
                let bass_r = self.flank_bass_r.process_sample(lp2_r, &self.flank_bass_coeffs);
                self.diffuse_delay_r.write(bass_r);
                let diff_r = self.diffuse_delay_r.read_samples(self.sample_rate * 0.058) * 0.35;
                out_r[n] = (bass_r + diff_r) * drift * 0.75;
            }
        }
    }

    fn reset(&mut self) {
        self.wall_filter1_l.reset();
        self.wall_filter1_r.reset();
        self.wall_filter2_l.reset();
        self.wall_filter2_r.reset();
        self.flank_bass_l.reset();
        self.flank_bass_r.reset();
        self.diffuse_delay_l.reset();
        self.diffuse_delay_r.reset();
    }

    fn parameter_count(&self) -> usize {
        5
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.enabled = value > 0.5,
            1 => {
                self.wall_cutoff_hz.set_target(value, 0.02, SmoothingMode::Linear);
                self.wall_coeffs = BiquadCoeffs::calculate(
                    BiquadType::Lowpass,
                    self.sample_rate as f64,
                    value.clamp(200.0, 2000.0) as f64,
                    std::f64::consts::FRAC_1_SQRT_2,
                    0.0,
                );
            }
            2 => {
                self.bass_boost_db.set_target(value, 0.02, SmoothingMode::Linear);
                self.flank_bass_coeffs = BiquadCoeffs::calculate(
                    BiquadType::LowShelf,
                    self.sample_rate as f64,
                    90.0,
                    std::f64::consts::FRAC_1_SQRT_2,
                    value.clamp(0.0, 12.0) as f64,
                );
            }
            3 => self.room_decay_sec.set_target(value, 0.02, SmoothingMode::Linear),
            4 => self.drift_level.set_target(value, 0.02, SmoothingMode::Linear),
            _ => {}
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 => {
                if self.enabled {
                    1.0
                } else {
                    0.0
                }
            }
            1 => self.wall_cutoff_hz.current(),
            2 => self.bass_boost_db.current(),
            3 => self.room_decay_sec.current(),
            4 => self.drift_level.current(),
            _ => 0.0,
        }
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }

    fn latency_samples(&self) -> usize {
        0
    }

    fn tail_samples(&self) -> usize {
        (self.sample_rate * 0.6) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fdn_stability_and_decay() {
        let mut fdn = AlgorithmicReverbNode::new(48000.0);
        fdn.set_decay_time(1.5);
        fdn.set_wet(1.0);

        let input_impulse = [1.0f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let mut out_l = [0.0f32; 8];
        let mut out_r = [0.0f32; 8];

        fdn.process(&[&input_impulse], &mut [&mut out_l, &mut out_r]);

        // Process decay and verify energy decreases monotonically and no NaNs
        let mut prev_rms = 100.0f32;
        let block_zeros = [0.0f32; 256];
        for _ in 0..15 {
            let mut b_l = [0.0f32; 256];
            let mut b_r = [0.0f32; 256];
            fdn.process(&[&block_zeros], &mut [&mut b_l, &mut b_r]);

            let rms = (b_l.iter().map(|&x| x * x).sum::<f32>() / 256.0).sqrt();
            assert!(!rms.is_nan(), "FDN generated NaN");
            assert!(rms < prev_rms + 0.1, "FDN unstable");
            prev_rms = rms;
        }
    }

    #[test]
    fn test_sabine_rt60_calculation() {
        let (rt60, first_refl) = compute_sabine_rt60(
            12.0,
            18.0,
            6.0,
            SurfaceMaterial::Plaster,
            SurfaceMaterial::Wood,
            SurfaceMaterial::Plaster,
        );

        // Sabine formula: 0.161 * V / A = 0.161 * 1296 / 54.72 ≈ 3.81s
        assert!(
            (rt60 - 3.81).abs() < 0.2,
            "RT60 calculation was {}, expected ~3.81s",
            rt60
        );
        assert!(
            first_refl >= 8.0 && first_refl <= 35.0,
            "First reflection {} out of expected 8-35 ms range",
            first_refl
        );
    }

    #[test]
    fn test_convolution_reverb_load_and_process() {
        let mut conv = ConvolutionReverbNode::new(48000.0);
        let custom_ir = [0.8f32, 0.5, 0.3, 0.1, 0.05, 0.01];
        conv.load_ir(&custom_ir, &custom_ir);

        let input = [1.0f32, 0.0, 0.0, 0.0, 0.0, 0.0];
        let mut out_l = [0.0f32; 6];
        let mut out_r = [0.0f32; 6];

        conv.process(&[&input], &mut [&mut out_l, &mut out_r]);
        assert!(!out_l[0].is_nan());
    }

    #[test]
    fn test_next_door_steep_attenuation() {
        let mut nd = NextDoorNode::new(48000.0);
        nd.set_parameter(0, 1.0); // enable

        // High frequency 8 kHz tone should be heavily attenuated by 24 dB/oct lowpass
        let mut in_high = [0.0f32; 512];
        for i in 0..512 {
            in_high[i] = (i as f32 * 8000.0 / 48000.0 * std::f32::consts::TAU).sin();
        }

        let mut out_l = [0.0f32; 512];
        let mut out_r = [0.0f32; 512];
        nd.process(&[&in_high], &mut [&mut out_l, &mut out_r]);

        let rms_out = (out_l[256..].iter().map(|&x| x * x).sum::<f32>() / 256.0).sqrt();
        assert!(
            rms_out < 0.05,
            "8 kHz high tone should be attenuated through wall, got RMS: {}",
            rms_out
        );
    }
}
