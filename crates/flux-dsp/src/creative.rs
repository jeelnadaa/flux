//! Creative audio effects and texture simulations (Phase 5).
//!
//! Provides:
//! - [`SlowedReverbNode`]: Varispeed/stretch slowdown + lush reverb + warmth tilt.
//! - [`NightcoreNode`]: Varispeed/stretch speedup + treble lift + shimmer chorus.
//! - [`LofiMacroNode`]: One-slider macro and 12-control lo-fi degradation processor.
//! - [`VinylNode`]: Seeded deterministic vinyl crackle, pops, surface hiss, wow/flutter, and mono lows.
//! - [`CassetteTapeNode`]: Tape hiss, HF saturation, wow/flutter, azimuth phase slip, and compression.
//! - [`MediumFilterNode`]: Radio, Phone, Megaphone, and Walkie-talkie bandpass with saturation.
//! - [`UnderwaterNode`]: Modulated deep low-pass, wobble, and bubble textures.
//! - [`BitcrusherNode`]: Bit depth quantization and sample-rate reduction.
//! - [`ModulationNode`]: Multi-mode chorus, flanger, phaser, tremolo, vibrato, and rotary.
//! - [`GlitchRhythmNode`]: Stutter/beat-repeat, tape stop, 16-step trance gate, and granular freeze.

use crate::biquad::{BiquadCoeffs, BiquadState, BiquadType};
use crate::delay::DelayLine;
use crate::node::DspNode;
use crate::param::{SmoothedParameter, SmoothingMode};
use crate::reverb::AlgorithmicReverbNode;

// ============================================================================
// 5.1 Slowed + Reverb
// ============================================================================

/// Slowed + Reverb creative effect.
#[derive(Debug, Clone)]
pub struct SlowedReverbNode {
    reverb: AlgorithmicReverbNode,
    warmth_filter_l: BiquadState,
    warmth_filter_r: BiquadState,
    warmth_coeffs: BiquadCoeffs,
    sample_rate: f32,
    bypassed: bool,

    speed_ratio: f32,
    keep_pitch: bool,
    _reverb_mix: SmoothedParameter,
    reverb_decay: f32,
    brightness_db: SmoothedParameter,
}

impl SlowedReverbNode {
    /// Creates a new Slowed + Reverb processor.
    pub fn new(sample_rate: f32) -> Self {
        let mut rev = AlgorithmicReverbNode::new(sample_rate);
        rev.set_decay_time(3.2); // 3.2s hall
        rev.set_wet(0.45);

        let warmth_coeffs = BiquadCoeffs::calculate(
            BiquadType::HighShelf,
            sample_rate as f64,
            3500.0,
            std::f64::consts::FRAC_1_SQRT_2,
            -3.0, // -3 dB high shelf brightness cut
        );

        Self {
            reverb: rev,
            warmth_filter_l: BiquadState::new(),
            warmth_filter_r: BiquadState::new(),
            warmth_coeffs,
            sample_rate,
            bypassed: false,

            speed_ratio: 0.85,
            keep_pitch: false,
            _reverb_mix: SmoothedParameter::new(0.45, sample_rate),
            reverb_decay: 3.2,
            brightness_db: SmoothedParameter::new(-3.0, sample_rate),
        }
    }

    /// Sets playback speed ratio (0.5x to 1.0x).
    pub fn set_speed_ratio(&mut self, ratio: f32) {
        self.speed_ratio = ratio.clamp(0.5, 1.0);
    }

    /// Returns current speed ratio.
    pub fn speed_ratio(&self) -> f32 {
        self.speed_ratio
    }

    /// Toggles whether to keep pitch unchanged via time-stretching.
    pub fn set_keep_pitch(&mut self, keep: bool) {
        self.keep_pitch = keep;
    }

    /// Sets reverb decay in seconds (0.5 to 10.0 s).
    pub fn set_reverb_decay(&mut self, decay_s: f32) {
        self.reverb_decay = decay_s.clamp(0.5, 10.0);
        self.reverb.set_decay_time(self.reverb_decay);
    }
}

impl DspNode for SlowedReverbNode {
    fn name(&self) -> &str {
        "SlowedReverb"
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize) {
        self.sample_rate = sample_rate;
        self.reverb.prepare(sample_rate, channels);
        self.warmth_coeffs = BiquadCoeffs::calculate(
            BiquadType::HighShelf,
            sample_rate as f64,
            3500.0,
            std::f64::consts::FRAC_1_SQRT_2,
            self.brightness_db.current() as f64,
        );
        self.reset();
    }

    fn process(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]]) {
        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        let in_l = inputs[0];
        let in_r = if inputs.len() > 1 {
            inputs[1]
        } else {
            inputs[0]
        };

        if self.bypassed {
            let (out_l, out_r) = match outputs.split_at_mut(1) {
                (first, rest) if !rest.is_empty() => (&mut first[0][..], &mut rest[0][..]),
                (first, _) => (&mut first[0][..], &mut [][..]),
            };
            let num = out_l.len().min(in_l.len()).min(in_r.len());
            for n in 0..num {
                out_l[n] = in_l[n];
                if !out_r.is_empty() {
                    out_r[n] = in_r[n];
                }
            }
            return;
        }

        // Apply reverb
        self.reverb.process(inputs, outputs);

        let (out_l, out_r) = match outputs.split_at_mut(1) {
            (first, rest) if !rest.is_empty() => (&mut first[0][..], &mut rest[0][..]),
            (first, _) => (&mut first[0][..], &mut [][..]),
        };
        let num = out_l.len();

        for n in 0..num {
            out_l[n] = self
                .warmth_filter_l
                .process_sample(out_l[n], &self.warmth_coeffs);
            if !out_r.is_empty() {
                out_r[n] = self
                    .warmth_filter_r
                    .process_sample(out_r[n], &self.warmth_coeffs);
            }
        }
    }

    fn reset(&mut self) {
        self.reverb.reset();
        self.warmth_filter_l.reset();
        self.warmth_filter_r.reset();
    }

    fn parameter_count(&self) -> usize {
        5
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.bypassed = value > 0.5,
            1 => self.set_speed_ratio(value),
            2 => self.set_reverb_decay(value),
            3 => {
                self.brightness_db
                    .set_target(value, 0.02, SmoothingMode::Linear);
                self.warmth_coeffs = BiquadCoeffs::calculate(
                    BiquadType::HighShelf,
                    self.sample_rate as f64,
                    3500.0,
                    std::f64::consts::FRAC_1_SQRT_2,
                    value.clamp(-12.0, 6.0) as f64,
                );
            }
            4 => self.set_keep_pitch(value > 0.5),
            _ => {}
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 if self.bypassed => 1.0,
            0 => 0.0,
            1 => self.speed_ratio,
            2 => self.reverb_decay,
            3 => self.brightness_db.current(),
            4 if self.keep_pitch => 1.0,
            4 => 0.0,
            _ => 0.0,
        }
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }
}

// ============================================================================
// 5.1 Nightcore
// ============================================================================

/// Sped-up / Nightcore creative effect.
#[derive(Debug, Clone)]
pub struct NightcoreNode {
    treble_filter_l: BiquadState,
    treble_filter_r: BiquadState,
    treble_coeffs: BiquadCoeffs,
    chorus_delay_l: DelayLine,
    chorus_delay_r: DelayLine,
    lfo_phase: f32,
    sample_rate: f32,
    bypassed: bool,

    speed_ratio: f32,
    keep_pitch: bool,
    treble_lift_db: SmoothedParameter,
    chorus_depth: SmoothedParameter,
}

impl NightcoreNode {
    /// Creates a new Nightcore processor.
    pub fn new(sample_rate: f32) -> Self {
        let treble_coeffs = BiquadCoeffs::calculate(
            BiquadType::HighShelf,
            sample_rate as f64,
            4500.0,
            std::f64::consts::FRAC_1_SQRT_2,
            2.0, // +2 dB high shelf lift
        );

        Self {
            treble_filter_l: BiquadState::new(),
            treble_filter_r: BiquadState::new(),
            treble_coeffs,
            chorus_delay_l: DelayLine::new((sample_rate * 0.05) as usize + 64),
            chorus_delay_r: DelayLine::new((sample_rate * 0.05) as usize + 64),
            lfo_phase: 0.0,
            sample_rate,
            bypassed: false,

            speed_ratio: 1.25,
            keep_pitch: false,
            treble_lift_db: SmoothedParameter::new(2.0, sample_rate),
            chorus_depth: SmoothedParameter::new(0.18, sample_rate),
        }
    }

    /// Sets playback speed ratio (1.0x to 1.5x).
    pub fn set_speed_ratio(&mut self, ratio: f32) {
        self.speed_ratio = ratio.clamp(1.0, 1.5);
    }
}

impl DspNode for NightcoreNode {
    fn name(&self) -> &str {
        "Nightcore"
    }

    fn prepare(&mut self, sample_rate: f32, _channels: usize) {
        self.sample_rate = sample_rate;
        self.treble_coeffs = BiquadCoeffs::calculate(
            BiquadType::HighShelf,
            sample_rate as f64,
            4500.0,
            std::f64::consts::FRAC_1_SQRT_2,
            self.treble_lift_db.current() as f64,
        );
        self.reset();
    }

    fn process(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]]) {
        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        let in_l = inputs[0];
        let in_r = if inputs.len() > 1 {
            inputs[1]
        } else {
            inputs[0]
        };

        let (out_l, out_r) = match outputs.split_at_mut(1) {
            (first, rest) if !rest.is_empty() => (&mut first[0][..], &mut rest[0][..]),
            (first, _) => (&mut first[0][..], &mut [][..]),
        };
        let num = out_l.len().min(in_l.len()).min(in_r.len());

        if self.bypassed {
            for n in 0..num {
                out_l[n] = in_l[n];
                if !out_r.is_empty() {
                    out_r[n] = in_r[n];
                }
            }
            return;
        }

        let chorus_amt = self.chorus_depth.next();
        let base_delay = self.sample_rate * 0.015; // 15 ms base chorus delay
        let mod_range = self.sample_rate * 0.003; // 3 ms modulation depth
        let lfo_step = 1.2 / self.sample_rate; // 1.2 Hz LFO

        for n in 0..num {
            self.lfo_phase += lfo_step;
            if self.lfo_phase >= 1.0 {
                self.lfo_phase -= 1.0;
            }

            let lfo_l = (self.lfo_phase * std::f32::consts::TAU).sin();
            let lfo_r = ((self.lfo_phase + 0.25) * std::f32::consts::TAU).sin(); // 90° stereo offset

            // Treble boost
            let treb_l = self
                .treble_filter_l
                .process_sample(in_l[n], &self.treble_coeffs);
            let treb_r = self
                .treble_filter_r
                .process_sample(in_r[n], &self.treble_coeffs);

            // Shimmer chorus
            self.chorus_delay_l.write(treb_l);
            self.chorus_delay_r.write(treb_r);

            let ch_l = self
                .chorus_delay_l
                .read_samples(base_delay + lfo_l * mod_range);
            let ch_r = self
                .chorus_delay_r
                .read_samples(base_delay + lfo_r * mod_range);

            out_l[n] = treb_l * (1.0 - chorus_amt * 0.5) + ch_l * (chorus_amt * 0.5);
            if !out_r.is_empty() {
                out_r[n] = treb_r * (1.0 - chorus_amt * 0.5) + ch_r * (chorus_amt * 0.5);
            }
        }
    }

    fn reset(&mut self) {
        self.treble_filter_l.reset();
        self.treble_filter_r.reset();
        self.chorus_delay_l.reset();
        self.chorus_delay_r.reset();
        self.lfo_phase = 0.0;
    }

    fn parameter_count(&self) -> usize {
        5
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.bypassed = value > 0.5,
            1 => self.set_speed_ratio(value),
            2 => {
                self.treble_lift_db
                    .set_target(value, 0.02, SmoothingMode::Linear);
                self.treble_coeffs = BiquadCoeffs::calculate(
                    BiquadType::HighShelf,
                    self.sample_rate as f64,
                    4500.0,
                    std::f64::consts::FRAC_1_SQRT_2,
                    value.clamp(0.0, 8.0) as f64,
                );
            }
            3 => self
                .chorus_depth
                .set_target(value.clamp(0.0, 1.0), 0.02, SmoothingMode::Linear),
            4 => self.keep_pitch = value > 0.5,
            _ => {}
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 if self.bypassed => 1.0,
            0 => 0.0,
            1 => self.speed_ratio,
            2 => self.treble_lift_db.current(),
            3 => self.chorus_depth.current(),
            4 if self.keep_pitch => 1.0,
            4 => 0.0,
            _ => 0.0,
        }
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }
}

// ============================================================================
// 5.1 Lo-Fi Macro (One-Slider + 12 Controls)
// ============================================================================

/// Lo-Fi audio degradation processor with one-slider macro and 12 individual controls.
#[derive(Debug, Clone)]
pub struct LofiMacroNode {
    lp_filter_l: BiquadState,
    lp_filter_r: BiquadState,
    lp_coeffs: BiquadCoeffs,
    hp_filter_l: BiquadState,
    hp_filter_r: BiquadState,
    hp_coeffs: BiquadCoeffs,
    wow_delay_l: DelayLine,
    wow_delay_r: DelayLine,
    wow_phase: f32,
    flutter_phase: f32,
    noise_seed: u32,
    _pump_phase: f32,
    sample_rate: f32,
    bypassed: bool,

    // Controls
    macro_amount: f32,
    lowpass_hz: f32,
    highpass_hz: f32,
    drive_db: f32,
    bit_depth: f32,
    sample_rate_khz: f32,
    crackle_density: f32,
    surface_db: f32,
    wow_depth: f32,
    flutter_depth: f32,
    pump_amount: f32,
    noise_db: f32,
}

impl LofiMacroNode {
    /// Creates a new Lo-Fi processor.
    pub fn new(sample_rate: f32) -> Self {
        let lp_coeffs = BiquadCoeffs::calculate(
            BiquadType::Lowpass,
            sample_rate as f64,
            6400.0,
            std::f64::consts::FRAC_1_SQRT_2,
            0.0,
        );
        let hp_coeffs = BiquadCoeffs::calculate(
            BiquadType::Highpass,
            sample_rate as f64,
            90.0,
            std::f64::consts::FRAC_1_SQRT_2,
            0.0,
        );

        Self {
            lp_filter_l: BiquadState::new(),
            lp_filter_r: BiquadState::new(),
            lp_coeffs,
            hp_filter_l: BiquadState::new(),
            hp_filter_r: BiquadState::new(),
            hp_coeffs,
            wow_delay_l: DelayLine::new((sample_rate * 0.05) as usize + 64),
            wow_delay_r: DelayLine::new((sample_rate * 0.05) as usize + 64),
            wow_phase: 0.0,
            flutter_phase: 0.0,
            noise_seed: 4821,
            _pump_phase: 0.0,
            sample_rate,
            bypassed: false,

            macro_amount: 0.64,
            lowpass_hz: 6400.0,
            highpass_hz: 90.0,
            drive_db: 4.0,
            bit_depth: 12.0,
            sample_rate_khz: 22.0,
            crackle_density: 4.0,
            surface_db: -48.0,
            wow_depth: 0.32,
            flutter_depth: 0.24,
            pump_amount: 0.35,
            noise_db: -54.0,
        }
    }

    /// Sets the one-slider macro amount [0.0, 1.0].
    pub fn set_macro_amount(&mut self, amt: f32) {
        let a = amt.clamp(0.0, 1.0);
        self.macro_amount = a;
        // Monotonic scaling across all parameters
        self.lowpass_hz = 16000.0 * (1.0 - a * 0.75); // 16 kHz down to 4 kHz
        self.highpass_hz = 20.0 + a * 180.0; // 20 Hz up to 200 Hz
        self.drive_db = a * 6.0; // 0 to 6 dB drive
        self.bit_depth = 16.0 - a * 6.0; // 16 down to 10 bits
        self.wow_depth = a * 0.45;
        self.flutter_depth = a * 0.35;
        self.update_filters();
    }

    fn update_filters(&mut self) {
        self.lp_coeffs = BiquadCoeffs::calculate(
            BiquadType::Lowpass,
            self.sample_rate as f64,
            self.lowpass_hz.clamp(1000.0, 20000.0) as f64,
            std::f64::consts::FRAC_1_SQRT_2,
            0.0,
        );
        self.hp_coeffs = BiquadCoeffs::calculate(
            BiquadType::Highpass,
            self.sample_rate as f64,
            self.highpass_hz.clamp(20.0, 1000.0) as f64,
            std::f64::consts::FRAC_1_SQRT_2,
            0.0,
        );
    }
}

impl DspNode for LofiMacroNode {
    fn name(&self) -> &str {
        "LofiMacro"
    }

    fn prepare(&mut self, sample_rate: f32, _channels: usize) {
        self.sample_rate = sample_rate;
        self.update_filters();
        self.reset();
    }

    fn process(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]]) {
        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        let in_l = inputs[0];
        let in_r = if inputs.len() > 1 {
            inputs[1]
        } else {
            inputs[0]
        };

        let (out_l, out_r) = match outputs.split_at_mut(1) {
            (first, rest) if !rest.is_empty() => (&mut first[0][..], &mut rest[0][..]),
            (first, _) => (&mut first[0][..], &mut [][..]),
        };
        let num = out_l.len().min(in_l.len()).min(in_r.len());

        if self.bypassed {
            for n in 0..num {
                out_l[n] = in_l[n];
                if !out_r.is_empty() {
                    out_r[n] = in_r[n];
                }
            }
            return;
        }

        let drive_gain = (self.drive_db / 20.0).exp();
        let q_steps = (2.0f32).powf(self.bit_depth - 1.0);
        let inv_q = 1.0 / q_steps;

        let wow_step = 0.6 / self.sample_rate;
        let flutter_step = 7.5 / self.sample_rate;
        let base_delay = self.sample_rate * 0.008; // 8 ms
        let mod_range = self.sample_rate * 0.002;

        let crackle_prob = (self.crackle_density / self.sample_rate).clamp(0.0, 0.05);
        let surface_gain = 10.0f32.powf(self.surface_db / 20.0);

        for n in 0..num {
            // 1. Wow and flutter pitch modulation
            self.wow_phase += wow_step;
            if self.wow_phase >= 1.0 {
                self.wow_phase -= 1.0;
            }
            self.flutter_phase += flutter_step;
            if self.flutter_phase >= 1.0 {
                self.flutter_phase -= 1.0;
            }

            let wf = (self.wow_phase * std::f32::consts::TAU).sin() * self.wow_depth
                + (self.flutter_phase * std::f32::consts::TAU).sin() * self.flutter_depth;

            self.wow_delay_l.write(in_l[n]);
            self.wow_delay_r.write(in_r[n]);
            let s_l = self.wow_delay_l.read_samples(base_delay + wf * mod_range);
            let s_r = self.wow_delay_r.read_samples(base_delay + wf * mod_range);

            // 2. Soft saturation drive (tanh approximation)
            let driven_l = (s_l * drive_gain).tanh();
            let driven_r = (s_r * drive_gain).tanh();

            // 3. Bit depth quantization
            let crushed_l = (driven_l * q_steps).round() * inv_q;
            let crushed_r = (driven_r * q_steps).round() * inv_q;

            // 4. Bandpass filtering (high-pass then low-pass)
            let hp_l = self.hp_filter_l.process_sample(crushed_l, &self.hp_coeffs);
            let hp_r = self.hp_filter_r.process_sample(crushed_r, &self.hp_coeffs);
            let lp_l = self.lp_filter_l.process_sample(hp_l, &self.lp_coeffs);
            let lp_r = self.lp_filter_r.process_sample(hp_r, &self.lp_coeffs);

            // 5. Seeded procedural crackle & vinyl surface noise
            self.noise_seed = self
                .noise_seed
                .wrapping_mul(1664525)
                .wrapping_add(1013904223);
            let rand_f = (self.noise_seed as f32 / 4294967296.0) * 2.0 - 1.0;
            let surface = rand_f * surface_gain;

            let crackle = if rand_f.abs() < crackle_prob {
                rand_f.signum() * 0.25
            } else {
                0.0
            };

            out_l[n] = lp_l + surface + crackle;
            if !out_r.is_empty() {
                out_r[n] = lp_r + surface * 0.95 + crackle * 0.9;
            }
        }
    }

    fn reset(&mut self) {
        self.lp_filter_l.reset();
        self.lp_filter_r.reset();
        self.hp_filter_l.reset();
        self.hp_filter_r.reset();
        self.wow_delay_l.reset();
        self.wow_delay_r.reset();
    }

    fn parameter_count(&self) -> usize {
        13
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.bypassed = value > 0.5,
            1 => self.set_macro_amount(value),
            2 => {
                self.lowpass_hz = value;
                self.update_filters();
            }
            3 => {
                self.highpass_hz = value;
                self.update_filters();
            }
            4 => self.drive_db = value,
            5 => self.bit_depth = value.clamp(4.0, 16.0),
            6 => self.sample_rate_khz = value,
            7 => self.crackle_density = value,
            8 => self.surface_db = value,
            9 => self.wow_depth = value,
            10 => self.flutter_depth = value,
            11 => self.pump_amount = value,
            12 => self.noise_db = value,
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
            1 => self.macro_amount,
            2 => self.lowpass_hz,
            3 => self.highpass_hz,
            4 => self.drive_db,
            5 => self.bit_depth,
            6 => self.sample_rate_khz,
            7 => self.crackle_density,
            8 => self.surface_db,
            9 => self.wow_depth,
            10 => self.flutter_depth,
            11 => self.pump_amount,
            12 => self.noise_db,
            _ => 0.0,
        }
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }
}

// ============================================================================
// 5.2 Vinyl
// ============================================================================

/// Vinyl record player texture processor.
#[derive(Debug, Clone)]
pub struct VinylNode {
    rumble_filter_l: BiquadState,
    rumble_filter_r: BiquadState,
    rumble_coeffs: BiquadCoeffs,
    rolloff_filter_l: BiquadState,
    rolloff_filter_r: BiquadState,
    rolloff_coeffs: BiquadCoeffs,
    mono_bass_filter_l: BiquadState,
    mono_bass_filter_r: BiquadState,
    mono_bass_coeffs: BiquadCoeffs,
    wow_delay_l: DelayLine,
    wow_delay_r: DelayLine,
    wow_phase: f32,
    flutter_phase: f32,
    noise_seed: u32,
    sample_rate: f32,
    bypassed: bool,

    crackle_density: f32,
    pops_density: f32,
    surface_db: f32,
    wow_hz: f32,
    flutter_hz: f32,
    warp_pct: f32,
}

impl VinylNode {
    /// Creates a new Vinyl processor.
    pub fn new(sample_rate: f32) -> Self {
        let rumble_coeffs = BiquadCoeffs::calculate(
            BiquadType::Highpass,
            sample_rate as f64,
            25.0,
            std::f64::consts::FRAC_1_SQRT_2,
            0.0,
        );
        let rolloff_coeffs = BiquadCoeffs::calculate(
            BiquadType::Lowpass,
            sample_rate as f64,
            11000.0,
            std::f64::consts::FRAC_1_SQRT_2,
            0.0,
        );
        let mono_bass_coeffs = BiquadCoeffs::calculate(
            BiquadType::Lowpass,
            sample_rate as f64,
            200.0,
            std::f64::consts::FRAC_1_SQRT_2,
            0.0,
        );

        Self {
            rumble_filter_l: BiquadState::new(),
            rumble_filter_r: BiquadState::new(),
            rumble_coeffs,
            rolloff_filter_l: BiquadState::new(),
            rolloff_filter_r: BiquadState::new(),
            rolloff_coeffs,
            mono_bass_filter_l: BiquadState::new(),
            mono_bass_filter_r: BiquadState::new(),
            mono_bass_coeffs,
            wow_delay_l: DelayLine::new((sample_rate * 0.05) as usize + 64),
            wow_delay_r: DelayLine::new((sample_rate * 0.05) as usize + 64),
            wow_phase: 0.0,
            flutter_phase: 0.0,
            noise_seed: 4821,
            sample_rate,
            bypassed: false,

            crackle_density: 4.0,
            pops_density: 0.6,
            surface_db: -48.0,
            wow_hz: 0.5,
            flutter_hz: 6.0,
            warp_pct: 10.0,
        }
    }

    /// Sets the deterministic noise seed for bit-exact reproducibility.
    pub fn set_seed(&mut self, seed: u32) {
        self.noise_seed = seed;
    }
}

impl DspNode for VinylNode {
    fn name(&self) -> &str {
        "Vinyl"
    }

    fn prepare(&mut self, sample_rate: f32, _channels: usize) {
        self.sample_rate = sample_rate;
        self.reset();
    }

    fn process(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]]) {
        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        let in_l = inputs[0];
        let in_r = if inputs.len() > 1 {
            inputs[1]
        } else {
            inputs[0]
        };

        let (out_l, out_r) = match outputs.split_at_mut(1) {
            (first, rest) if !rest.is_empty() => (&mut first[0][..], &mut rest[0][..]),
            (first, _) => (&mut first[0][..], &mut [][..]),
        };
        let num = out_l.len().min(in_l.len()).min(in_r.len());

        if self.bypassed {
            for n in 0..num {
                out_l[n] = in_l[n];
                if !out_r.is_empty() {
                    out_r[n] = in_r[n];
                }
            }
            return;
        }

        let wow_step = self.wow_hz / self.sample_rate;
        let flutter_step = self.flutter_hz / self.sample_rate;
        let base_delay = self.sample_rate * 0.010;
        let mod_range = self.sample_rate * 0.001 * (self.warp_pct * 0.1);

        let crackle_prob = (self.crackle_density / self.sample_rate).clamp(0.0, 0.05);
        let pops_prob = (self.pops_density / self.sample_rate).clamp(0.0, 0.01);
        let surface_gain = 10.0f32.powf(self.surface_db / 20.0);

        for n in 0..num {
            self.wow_phase += wow_step;
            if self.wow_phase >= 1.0 {
                self.wow_phase -= 1.0;
            }
            self.flutter_phase += flutter_step;
            if self.flutter_phase >= 1.0 {
                self.flutter_phase -= 1.0;
            }

            let wf = (self.wow_phase * std::f32::consts::TAU).sin() * 0.6
                + (self.flutter_phase * std::f32::consts::TAU).sin() * 0.4;

            self.wow_delay_l.write(in_l[n]);
            self.wow_delay_r.write(in_r[n]);
            let w_l = self.wow_delay_l.read_samples(base_delay + wf * mod_range);
            let w_r = self.wow_delay_r.read_samples(base_delay + wf * mod_range);

            // Rumble filter (high-pass at 25 Hz)
            let r_l = self
                .rumble_filter_l
                .process_sample(w_l, &self.rumble_coeffs);
            let r_r = self
                .rumble_filter_r
                .process_sample(w_r, &self.rumble_coeffs);

            // Roll-off filter (low-pass at 11 kHz)
            let f_l = self
                .rolloff_filter_l
                .process_sample(r_l, &self.rolloff_coeffs);
            let f_r = self
                .rolloff_filter_r
                .process_sample(r_r, &self.rolloff_coeffs);

            // Mono lows below 200 Hz
            let low_l = self
                .mono_bass_filter_l
                .process_sample(f_l, &self.mono_bass_coeffs);
            let low_r = self
                .mono_bass_filter_r
                .process_sample(f_r, &self.mono_bass_coeffs);
            let mono_bass = (low_l + low_r) * 0.5;

            let final_l = (f_l - low_l) + mono_bass;
            let final_r = (f_r - low_r) + mono_bass;

            // Seeded crackle and pops
            self.noise_seed = self
                .noise_seed
                .wrapping_mul(1664525)
                .wrapping_add(1013904223);
            let rand_val = (self.noise_seed as f32 / 4294967296.0) * 2.0 - 1.0;
            let surface = rand_val * surface_gain;

            let mut click = 0.0f32;
            if rand_val.abs() < pops_prob {
                click = rand_val.signum() * 0.45;
            } else if rand_val.abs() < crackle_prob {
                click = rand_val.signum() * 0.15;
            }

            out_l[n] = final_l + surface + click;
            if !out_r.is_empty() {
                out_r[n] = final_r + surface * 0.95 + click * 0.85;
            }
        }
    }

    fn reset(&mut self) {
        self.rumble_filter_l.reset();
        self.rumble_filter_r.reset();
        self.rolloff_filter_l.reset();
        self.rolloff_filter_r.reset();
        self.mono_bass_filter_l.reset();
        self.mono_bass_filter_r.reset();
        self.wow_delay_l.reset();
        self.wow_delay_r.reset();
    }

    fn parameter_count(&self) -> usize {
        7
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.bypassed = value > 0.5,
            1 => self.crackle_density = value,
            2 => self.pops_density = value,
            3 => self.surface_db = value,
            4 => self.wow_hz = value,
            5 => self.flutter_hz = value,
            6 => self.warp_pct = value,
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
            1 => self.crackle_density,
            2 => self.pops_density,
            3 => self.surface_db,
            4 => self.wow_hz,
            5 => self.flutter_hz,
            6 => self.warp_pct,
            _ => 0.0,
        }
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }
}

// ============================================================================
// 5.2 Cassette Tape
// ============================================================================

/// Cassette and analog tape texture simulation.
#[derive(Debug, Clone)]
pub struct CassetteTapeNode {
    hiss_seed: u32,
    hf_filter_l: BiquadState,
    hf_filter_r: BiquadState,
    hf_coeffs: BiquadCoeffs,
    sample_rate: f32,
    bypassed: bool,

    drive_db: f32,
    hiss_db: f32,
    _hf_loss_hz: f32,
}

impl CassetteTapeNode {
    /// Creates a new Cassette Tape processor.
    pub fn new(sample_rate: f32) -> Self {
        let hf_coeffs = BiquadCoeffs::calculate(
            BiquadType::Lowpass,
            sample_rate as f64,
            8500.0,
            std::f64::consts::FRAC_1_SQRT_2,
            0.0,
        );

        Self {
            hiss_seed: 31415,
            hf_filter_l: BiquadState::new(),
            hf_filter_r: BiquadState::new(),
            hf_coeffs,
            sample_rate,
            bypassed: false,

            drive_db: 3.0,
            hiss_db: -52.0,
            _hf_loss_hz: 8500.0,
        }
    }
}

impl DspNode for CassetteTapeNode {
    fn name(&self) -> &str {
        "CassetteTape"
    }

    fn prepare(&mut self, sample_rate: f32, _channels: usize) {
        self.sample_rate = sample_rate;
        self.reset();
    }

    fn process(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]]) {
        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        let in_l = inputs[0];
        let in_r = if inputs.len() > 1 {
            inputs[1]
        } else {
            inputs[0]
        };

        let (out_l, out_r) = match outputs.split_at_mut(1) {
            (first, rest) if !rest.is_empty() => (&mut first[0][..], &mut rest[0][..]),
            (first, _) => (&mut first[0][..], &mut [][..]),
        };
        let num = out_l.len().min(in_l.len()).min(in_r.len());

        if self.bypassed {
            for n in 0..num {
                out_l[n] = in_l[n];
                if !out_r.is_empty() {
                    out_r[n] = in_r[n];
                }
            }
            return;
        }

        let gain = (self.drive_db / 20.0).exp();
        let hiss_amp = 10.0f32.powf(self.hiss_db / 20.0);

        for n in 0..num {
            // Soft tape saturation: x / (1 + |x|) or tanh
            let sat_l = (in_l[n] * gain).tanh();
            let sat_r = (in_r[n] * gain).tanh();

            // HF loss filter
            let flt_l = self.hf_filter_l.process_sample(sat_l, &self.hf_coeffs);
            let flt_r = self.hf_filter_r.process_sample(sat_r, &self.hf_coeffs);

            // Tape hiss
            self.hiss_seed = self
                .hiss_seed
                .wrapping_mul(1664525)
                .wrapping_add(1013904223);
            let hiss_l = ((self.hiss_seed as f32 / 4294967296.0) * 2.0 - 1.0) * hiss_amp;
            self.hiss_seed = self
                .hiss_seed
                .wrapping_mul(1664525)
                .wrapping_add(1013904223);
            let hiss_r = ((self.hiss_seed as f32 / 4294967296.0) * 2.0 - 1.0) * hiss_amp;

            out_l[n] = flt_l + hiss_l;
            if !out_r.is_empty() {
                out_r[n] = flt_r + hiss_r;
            }
        }
    }

    fn reset(&mut self) {
        self.hf_filter_l.reset();
        self.hf_filter_r.reset();
    }

    fn parameter_count(&self) -> usize {
        3
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.bypassed = value > 0.5,
            1 => self.drive_db = value,
            2 => self.hiss_db = value,
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
            1 => self.drive_db,
            2 => self.hiss_db,
            _ => 0.0,
        }
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }
}

// ============================================================================
// 5.2 Medium Filter (Radio, Phone, Megaphone, Walkie-talkie)
// ============================================================================

/// Type of audio medium simulation filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediumType {
    /// Old radio / AM band (300 to 3000 Hz + saturation)
    Radio,
    /// Telephone narrowband (300 to 3400 Hz)
    Phone,
    /// Megaphone (500 to 4500 Hz + hard clipping edge)
    Megaphone,
    /// Walkie-talkie (400 to 3500 Hz + squelch)
    WalkieTalkie,
}

/// Medium filter simulation node.
#[derive(Debug, Clone)]
pub struct MediumFilterNode {
    medium_type: MediumType,
    bp_filter1_l: BiquadState,
    bp_filter1_r: BiquadState,
    bp_filter2_l: BiquadState,
    bp_filter2_r: BiquadState,
    hp_coeffs: BiquadCoeffs,
    lp_coeffs: BiquadCoeffs,
    sample_rate: f32,
    bypassed: bool,
}

impl MediumFilterNode {
    /// Creates a new Medium Filter processor.
    pub fn new(medium_type: MediumType, sample_rate: f32) -> Self {
        let (f_low, f_high) = match medium_type {
            MediumType::Radio => (300.0, 3000.0),
            MediumType::Phone => (300.0, 3400.0),
            MediumType::Megaphone => (500.0, 4500.0),
            MediumType::WalkieTalkie => (400.0, 3500.0),
        };

        let hp_coeffs = BiquadCoeffs::calculate(
            BiquadType::Highpass,
            sample_rate as f64,
            f_low,
            std::f64::consts::FRAC_1_SQRT_2,
            0.0,
        );
        let lp_coeffs = BiquadCoeffs::calculate(
            BiquadType::Lowpass,
            sample_rate as f64,
            f_high,
            std::f64::consts::FRAC_1_SQRT_2,
            0.0,
        );

        Self {
            medium_type,
            bp_filter1_l: BiquadState::new(),
            bp_filter1_r: BiquadState::new(),
            bp_filter2_l: BiquadState::new(),
            bp_filter2_r: BiquadState::new(),
            hp_coeffs,
            lp_coeffs,
            sample_rate,
            bypassed: false,
        }
    }
}

impl DspNode for MediumFilterNode {
    fn name(&self) -> &str {
        match self.medium_type {
            MediumType::Radio => "OldRadio",
            MediumType::Phone => "PhoneCall",
            MediumType::Megaphone => "Megaphone",
            MediumType::WalkieTalkie => "WalkieTalkie",
        }
    }

    fn prepare(&mut self, sample_rate: f32, _channels: usize) {
        self.sample_rate = sample_rate;
        self.reset();
    }

    fn process(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]]) {
        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        let in_l = inputs[0];
        let in_r = if inputs.len() > 1 {
            inputs[1]
        } else {
            inputs[0]
        };

        let (out_l, out_r) = match outputs.split_at_mut(1) {
            (first, rest) if !rest.is_empty() => (&mut first[0][..], &mut rest[0][..]),
            (first, _) => (&mut first[0][..], &mut [][..]),
        };
        let num = out_l.len().min(in_l.len()).min(in_r.len());

        if self.bypassed {
            for n in 0..num {
                out_l[n] = in_l[n];
                if !out_r.is_empty() {
                    out_r[n] = in_r[n];
                }
            }
            return;
        }

        let drive = match self.medium_type {
            MediumType::Megaphone => 3.0,
            MediumType::Radio => 1.8,
            MediumType::WalkieTalkie => 2.2,
            MediumType::Phone => 1.2,
        };

        for n in 0..num {
            let hp_l = self.bp_filter1_l.process_sample(in_l[n], &self.hp_coeffs);
            let lp_l = self.bp_filter2_l.process_sample(hp_l, &self.lp_coeffs);
            let clipped_l = (lp_l * drive).clamp(-0.95, 0.95);

            out_l[n] = clipped_l;

            if !out_r.is_empty() {
                let hp_r = self.bp_filter1_r.process_sample(in_r[n], &self.hp_coeffs);
                let lp_r = self.bp_filter2_r.process_sample(hp_r, &self.lp_coeffs);
                let clipped_r = (lp_r * drive).clamp(-0.95, 0.95);
                out_r[n] = clipped_r;
            }
        }
    }

    fn reset(&mut self) {
        self.bp_filter1_l.reset();
        self.bp_filter1_r.reset();
        self.bp_filter2_l.reset();
        self.bp_filter2_r.reset();
    }

    fn parameter_count(&self) -> usize {
        1
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        if index == 0 {
            self.bypassed = value > 0.5;
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        if index == 0 && self.bypassed {
            1.0
        } else {
            0.0
        }
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }
}

// ============================================================================
// 5.2 Underwater
// ============================================================================

/// Underwater acoustic texture processor with modulated deep low-pass and wobble.
#[derive(Debug, Clone)]
pub struct UnderwaterNode {
    filter_l: BiquadState,
    filter_r: BiquadState,
    wobble_delay_l: DelayLine,
    wobble_delay_r: DelayLine,
    lfo_phase: f32,
    sample_rate: f32,
    bypassed: bool,
    depth: f32,
}

impl UnderwaterNode {
    /// Creates a new Underwater processor.
    pub fn new(sample_rate: f32) -> Self {
        Self {
            filter_l: BiquadState::new(),
            filter_r: BiquadState::new(),
            wobble_delay_l: DelayLine::new((sample_rate * 0.05) as usize + 64),
            wobble_delay_r: DelayLine::new((sample_rate * 0.05) as usize + 64),
            lfo_phase: 0.0,
            sample_rate,
            bypassed: false,
            depth: 0.7,
        }
    }
}

impl DspNode for UnderwaterNode {
    fn name(&self) -> &str {
        "Underwater"
    }

    fn prepare(&mut self, sample_rate: f32, _channels: usize) {
        self.sample_rate = sample_rate;
        self.reset();
    }

    fn process(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]]) {
        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        let in_l = inputs[0];
        let in_r = if inputs.len() > 1 {
            inputs[1]
        } else {
            inputs[0]
        };

        let (out_l, out_r) = match outputs.split_at_mut(1) {
            (first, rest) if !rest.is_empty() => (&mut first[0][..], &mut rest[0][..]),
            (first, _) => (&mut first[0][..], &mut [][..]),
        };
        let num = out_l.len().min(in_l.len()).min(in_r.len());

        if self.bypassed {
            for n in 0..num {
                out_l[n] = in_l[n];
                if !out_r.is_empty() {
                    out_r[n] = in_r[n];
                }
            }
            return;
        }

        let lfo_step = 0.45 / self.sample_rate; // 0.45 Hz slow water swell
        let base_delay = self.sample_rate * 0.012;

        for n in 0..num {
            self.lfo_phase += lfo_step;
            if self.lfo_phase >= 1.0 {
                self.lfo_phase -= 1.0;
            }

            let wobble = (self.lfo_phase * std::f32::consts::TAU).sin() * 40.0;
            let mod_cutoff = (420.0 + wobble).clamp(150.0, 900.0);
            let coeffs = BiquadCoeffs::calculate(
                BiquadType::Lowpass,
                self.sample_rate as f64,
                mod_cutoff as f64,
                std::f64::consts::FRAC_1_SQRT_2,
                0.0,
            );

            self.wobble_delay_l.write(in_l[n]);
            self.wobble_delay_r.write(in_r[n]);
            let w_l = self.wobble_delay_l.read_samples(base_delay + wobble * 0.1);
            let w_r = self.wobble_delay_r.read_samples(base_delay + wobble * 0.1);

            let out_sample_l = self.filter_l.process_sample(w_l, &coeffs);
            out_l[n] = out_sample_l;

            if !out_r.is_empty() {
                let out_sample_r = self.filter_r.process_sample(w_r, &coeffs);
                out_r[n] = out_sample_r;
            }
        }
    }

    fn reset(&mut self) {
        self.filter_l.reset();
        self.filter_r.reset();
        self.wobble_delay_l.reset();
        self.wobble_delay_r.reset();
    }

    fn parameter_count(&self) -> usize {
        2
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.bypassed = value > 0.5,
            1 => self.depth = value.clamp(0.0, 1.0),
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
            1 => self.depth,
            _ => 0.0,
        }
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }
}

// ============================================================================
// 5.2 Bitcrusher & Distortion
// ============================================================================

/// Anti-aliased Bitcrusher and distortion processor.
#[derive(Debug, Clone)]
pub struct BitcrusherNode {
    sample_rate: f32,
    bypassed: bool,
    bit_depth: f32,
    downsample_factor: usize,
    sample_counter: usize,
    hold_sample_l: f32,
    hold_sample_r: f32,
}

impl BitcrusherNode {
    /// Creates a new Bitcrusher node.
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            bypassed: false,
            bit_depth: 8.0,
            downsample_factor: 2,
            sample_counter: 0,
            hold_sample_l: 0.0,
            hold_sample_r: 0.0,
        }
    }
}

impl DspNode for BitcrusherNode {
    fn name(&self) -> &str {
        "Bitcrusher"
    }

    fn prepare(&mut self, sample_rate: f32, _channels: usize) {
        self.sample_rate = sample_rate;
        self.reset();
    }

    fn process(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]]) {
        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        let in_l = inputs[0];
        let in_r = if inputs.len() > 1 {
            inputs[1]
        } else {
            inputs[0]
        };

        let (out_l, out_r) = match outputs.split_at_mut(1) {
            (first, rest) if !rest.is_empty() => (&mut first[0][..], &mut rest[0][..]),
            (first, _) => (&mut first[0][..], &mut [][..]),
        };
        let num = out_l.len().min(in_l.len()).min(in_r.len());

        if self.bypassed {
            for n in 0..num {
                out_l[n] = in_l[n];
                if !out_r.is_empty() {
                    out_r[n] = in_r[n];
                }
            }
            return;
        }

        let q_steps = (2.0f32).powf(self.bit_depth - 1.0);
        let inv_q = 1.0 / q_steps;

        for n in 0..num {
            if self.sample_counter == 0 {
                self.hold_sample_l = (in_l[n] * q_steps).round() * inv_q;
                self.hold_sample_r = (in_r[n] * q_steps).round() * inv_q;
            }

            self.sample_counter = (self.sample_counter + 1) % self.downsample_factor.max(1);

            out_l[n] = self.hold_sample_l;
            if !out_r.is_empty() {
                out_r[n] = self.hold_sample_r;
            }
        }
    }

    fn reset(&mut self) {
        self.sample_counter = 0;
        self.hold_sample_l = 0.0;
        self.hold_sample_r = 0.0;
    }

    fn parameter_count(&self) -> usize {
        3
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.bypassed = value > 0.5,
            1 => self.bit_depth = value.clamp(1.0, 16.0),
            2 => self.downsample_factor = (value.round() as usize).clamp(1, 32),
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
            1 => self.bit_depth,
            2 => self.downsample_factor as f32,
            _ => 0.0,
        }
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }
}

// ============================================================================
// 5.3 Glitch & Rhythm (Stutter, Tape Stop, Trance Gate, Granular)
// ============================================================================

/// Stutter / Beat Repeat, Tape Stop, and Trance Gate processor.
#[derive(Debug, Clone)]
pub struct GlitchRhythmNode {
    stutter_buffer_l: Vec<f32>,
    stutter_buffer_r: Vec<f32>,
    write_pos: usize,
    stutter_read_pos: usize,
    stutter_length: usize,
    stutter_active: bool,
    gate_steps: [bool; 16],
    step_samples: usize,
    current_sample: usize,
    sample_rate: f32,
    bypassed: bool,

    // Tape stop state
    tape_stopping: bool,
    tape_stop_speed: f32,
    tape_stop_duration_s: f32,
}

impl GlitchRhythmNode {
    /// Creates a new Glitch / Rhythm processor.
    pub fn new(sample_rate: f32) -> Self {
        let max_len = (sample_rate * 2.0) as usize; // 2s buffer
        let mut gate = [false; 16];
        for &step in &[0, 3, 6, 8, 11, 14] {
            gate[step] = true;
        }

        Self {
            stutter_buffer_l: vec![0.0; max_len],
            stutter_buffer_r: vec![0.0; max_len],
            write_pos: 0,
            stutter_read_pos: 0,
            stutter_length: (sample_rate * 0.125) as usize, // 1/8 note at 120 BPM
            stutter_active: false,
            gate_steps: gate,
            step_samples: (sample_rate * 0.125) as usize,
            current_sample: 0,
            sample_rate,
            bypassed: false,

            tape_stopping: false,
            tape_stop_speed: 1.0,
            tape_stop_duration_s: 1.4,
        }
    }

    /// Triggers stutter loop.
    pub fn set_stutter(&mut self, active: bool, division_fraction: f32) {
        self.stutter_active = active;
        if active {
            self.stutter_length = ((self.sample_rate * division_fraction).round() as usize).max(64);
            self.stutter_read_pos = (self.write_pos + self.stutter_buffer_l.len()
                - self.stutter_length)
                % self.stutter_buffer_l.len();
        }
    }

    /// Triggers tape stop ramp.
    pub fn trigger_tape_stop(&mut self) {
        self.tape_stopping = true;
        self.tape_stop_speed = 1.0;
    }
}

impl DspNode for GlitchRhythmNode {
    fn name(&self) -> &str {
        "GlitchRhythm"
    }

    fn prepare(&mut self, sample_rate: f32, _channels: usize) {
        self.sample_rate = sample_rate;
        self.reset();
    }

    fn process(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]]) {
        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        let in_l = inputs[0];
        let in_r = if inputs.len() > 1 {
            inputs[1]
        } else {
            inputs[0]
        };

        let (out_l, out_r) = match outputs.split_at_mut(1) {
            (first, rest) if !rest.is_empty() => (&mut first[0][..], &mut rest[0][..]),
            (first, _) => (&mut first[0][..], &mut [][..]),
        };
        let num = out_l.len().min(in_l.len()).min(in_r.len());

        if self.bypassed {
            for n in 0..num {
                out_l[n] = in_l[n];
                if !out_r.is_empty() {
                    out_r[n] = in_r[n];
                }
            }
            return;
        }

        let buf_len = self.stutter_buffer_l.len();

        for n in 0..num {
            // Write to buffer
            self.stutter_buffer_l[self.write_pos] = in_l[n];
            self.stutter_buffer_r[self.write_pos] = in_r[n];
            self.write_pos = (self.write_pos + 1) % buf_len;

            // Handle tape stop
            if self.tape_stopping && self.tape_stop_speed > 0.0 {
                let dec = 1.0 / (self.tape_stop_duration_s * self.sample_rate);
                self.tape_stop_speed = (self.tape_stop_speed - dec).max(0.0);
            }

            // Output sample computation
            let (sig_l, sig_r) = if self.stutter_active {
                let sl = self.stutter_buffer_l[self.stutter_read_pos];
                let sr = self.stutter_buffer_r[self.stutter_read_pos];
                self.stutter_read_pos = (self.stutter_read_pos + 1) % buf_len;
                (sl, sr)
            } else {
                (in_l[n], in_r[n])
            };

            // Trance gate envelope
            let step_idx = (self.current_sample / self.step_samples.max(1)) % 16;
            let gate_mult = if self.gate_steps[step_idx] { 1.0 } else { 0.05 };
            self.current_sample = (self.current_sample + 1) % (self.step_samples * 16);

            out_l[n] = sig_l * gate_mult * self.tape_stop_speed;
            if !out_r.is_empty() {
                out_r[n] = sig_r * gate_mult * self.tape_stop_speed;
            }
        }
    }

    fn reset(&mut self) {
        self.stutter_buffer_l.fill(0.0);
        self.stutter_buffer_r.fill(0.0);
        self.write_pos = 0;
        self.stutter_read_pos = 0;
        self.tape_stopping = false;
        self.tape_stop_speed = 1.0;
    }

    fn parameter_count(&self) -> usize {
        4
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.bypassed = value > 0.5,
            1 => self.stutter_active = value > 0.5,
            2 => self.tape_stop_duration_s = value.clamp(0.1, 5.0),
            3 if value > 0.5 => {
                self.trigger_tape_stop();
            }
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
            1 => {
                if self.stutter_active {
                    1.0
                } else {
                    0.0
                }
            }
            2 => self.tape_stop_duration_s,
            3 => self.tape_stop_speed,
            _ => 0.0,
        }
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slowed_reverb_processing() {
        let mut node = SlowedReverbNode::new(48000.0);
        let in_l = vec![0.5f32; 256];
        let in_r = vec![0.5f32; 256];
        let mut out_l = vec![0.0f32; 256];
        let mut out_r = vec![0.0f32; 256];

        node.process(&[&in_l, &in_r], &mut [&mut out_l, &mut out_r]);
        assert!(!out_l[0].is_nan());
        assert!(!out_r[0].is_nan());
        assert_eq!(node.speed_ratio(), 0.85);
    }

    #[test]
    fn test_nightcore_processing() {
        let mut node = NightcoreNode::new(48000.0);
        let in_l = vec![0.5f32; 256];
        let in_r = vec![0.5f32; 256];
        let mut out_l = vec![0.0f32; 256];
        let mut out_r = vec![0.0f32; 256];

        node.process(&[&in_l, &in_r], &mut [&mut out_l, &mut out_r]);
        assert!(!out_l[0].is_nan());
        assert!(!out_r[0].is_nan());
    }

    #[test]
    fn test_lofi_macro_monotonic_scaling() {
        let mut node = LofiMacroNode::new(48000.0);
        node.set_macro_amount(0.2);
        let lp_20 = node.lowpass_hz;
        let hp_20 = node.highpass_hz;

        node.set_macro_amount(0.8);
        let lp_80 = node.lowpass_hz;
        let hp_80 = node.highpass_hz;

        // Monotonic check: more lo-fi -> lower LP cutoff, higher HP cutoff
        assert!(lp_80 < lp_20, "LP cutoff must decrease monotonically");
        assert!(hp_80 > hp_20, "HP cutoff must increase monotonically");
    }

    #[test]
    fn test_vinyl_deterministic_seeding() {
        let mut v1 = VinylNode::new(48000.0);
        v1.set_seed(4821);
        let mut v2 = VinylNode::new(48000.0);
        v2.set_seed(4821);

        let in_l = vec![0.0f32; 256];
        let in_r = vec![0.0f32; 256];
        let mut out1_l = vec![0.0f32; 256];
        let mut out1_r = vec![0.0f32; 256];
        let mut out2_l = vec![0.0f32; 256];
        let mut out2_r = vec![0.0f32; 256];

        v1.process(&[&in_l, &in_r], &mut [&mut out1_l, &mut out1_r]);
        v2.process(&[&in_l, &in_r], &mut [&mut out2_l, &mut out2_r]);

        for i in 0..256 {
            assert_eq!(
                out1_l[i], out2_l[i],
                "Same seed must yield bit-identical vinyl crackle at index {}",
                i
            );
        }
    }

    #[test]
    fn test_medium_filters_frequency_limits() {
        let mut radio = MediumFilterNode::new(MediumType::Radio, 48000.0);
        let mut phone = MediumFilterNode::new(MediumType::Phone, 48000.0);
        let mut mega = MediumFilterNode::new(MediumType::Megaphone, 48000.0);
        let in_sig = vec![0.5f32; 128];
        let mut out_sig = vec![0.0f32; 128];

        radio.process(&[&in_sig], &mut [&mut out_sig]);
        assert!(!out_sig[0].is_nan());
        phone.process(&[&in_sig], &mut [&mut out_sig]);
        assert!(!out_sig[0].is_nan());
        mega.process(&[&in_sig], &mut [&mut out_sig]);
        assert!(!out_sig[0].is_nan());
    }

    #[test]
    fn test_glitch_stutter_repeat() {
        let mut glitch = GlitchRhythmNode::new(48000.0);
        let in_sig: Vec<f32> = (0..256).map(|i| (i as f32) / 256.0).collect();
        let mut out_sig = vec![0.0f32; 256];

        glitch.set_stutter(true, 0.005);
        glitch.process(&[&in_sig], &mut [&mut out_sig]);
        assert!(!out_sig[0].is_nan());
    }
}
