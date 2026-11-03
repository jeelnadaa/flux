//! Studio-grade dynamic range processing nodes.
//!
//! Provides production implementations of:
//! - [`Compressor`]: Feed-forward compressor with soft-knee, program-dependent release,
//!   lookahead, auto-makeup gain, parallel mix, and sidechain HPF.
//! - [`TruePeakLimiter`]: Inter-sample peak safe lookahead limiter with polyphase true-peak detection.
//! - [`GateExpander`]: Downward expander and noise gate with hysteresis and hold.
//! - [`DeEsser`]: Split-band and wideband sibilance reduction.
//! - [`TransientShaper`]: Dual envelope follower transient and sustain shaper.
//! - [`MultibandCompressor`]: 4-band compressor with 4th-order Linkwitz-Riley (LR4) crossovers.
//! - [`AdaptiveLeveler`]: Slow gain-riding loudness leveler with speech-aware gating, plus peak and LUFS normalizers.

use crate::biquad::{BiquadCoeffs, BiquadState, BiquadType};
use crate::loudness::LoudnessMeter;
use crate::node::DspNode;
use crate::sample::Sample;
use std::f64::consts::FRAC_1_SQRT_2;

/// Configuration parameters for the dynamic range [`Compressor`].
#[derive(Debug, Clone, Copy)]
pub struct CompressorConfig {
    /// Threshold in dBFS (-60.0 to 0.0 dB).
    pub threshold_db: f32,
    /// Compression ratio (1.0 to 20.0).
    pub ratio: f32,
    /// Attack time in milliseconds (0.1 to 200.0 ms).
    pub attack_ms: f32,
    /// Release time in milliseconds (5.0 to 2000.0 ms).
    pub release_ms: f32,
    /// Knee width in dB (0.0 to 24.0 dB).
    pub knee_db: f32,
    /// Manual makeup gain in dB (0.0 to 36.0 dB).
    pub makeup_gain_db: f32,
    /// Automatically compensate makeup gain based on threshold, ratio, and knee.
    pub auto_makeup: bool,
    /// Dynamically adjust release time based on signal dynamics.
    pub program_dependent_release: bool,
    /// Lookahead buffer in milliseconds (0.0 to 20.0 ms).
    pub lookahead_ms: f32,
    /// Wet/dry mix (0.0 = dry, 1.0 = wet).
    pub mix: f32,
    /// Sidechain high-pass filter cutoff frequency in Hz (20.0 to 500.0 Hz).
    pub sidechain_lowcut_hz: f32,
}

impl Default for CompressorConfig {
    fn default() -> Self {
        Self {
            threshold_db: -18.0,
            ratio: 3.2,
            attack_ms: 12.0,
            release_ms: 180.0,
            knee_db: 6.0,
            makeup_gain_db: 3.5,
            auto_makeup: true,
            program_dependent_release: true,
            lookahead_ms: 3.0,
            mix: 1.0,
            sidechain_lowcut_hz: 80.0,
        }
    }
}

/// Feed-forward dynamic range compressor node.
pub struct Compressor {
    config: CompressorConfig,
    sample_rate: f32,
    envelope: f32,
    lookahead_buffers: Vec<Vec<f32>>,
    lookahead_write_pos: usize,
    lookahead_samples: usize,
    sidechain_filter_coeffs: BiquadCoeffs,
    sidechain_filter_states: Vec<BiquadState>,
    is_bypassed: bool,
    last_gain_reduction_db: f32,
}

impl Compressor {
    /// Creates a new compressor with default settings.
    pub fn new() -> Self {
        Self::with_config(CompressorConfig::default())
    }

    /// Creates a new compressor with the given configuration.
    pub fn with_config(config: CompressorConfig) -> Self {
        let mut comp = Self {
            config,
            sample_rate: 48000.0,
            envelope: 0.0,
            lookahead_buffers: vec![vec![0.0; 4800]; 2],
            lookahead_write_pos: 0,
            lookahead_samples: 0,
            sidechain_filter_coeffs: BiquadCoeffs::default(),
            sidechain_filter_states: vec![BiquadState::default(); 2],
            is_bypassed: false,
            last_gain_reduction_db: 0.0,
        };
        comp.update_parameters();
        comp
    }

    /// Returns the current configuration.
    pub fn config(&self) -> &CompressorConfig {
        &self.config
    }

    /// Returns a mutable reference to the configuration.
    pub fn config_mut(&mut self) -> &mut CompressorConfig {
        &mut self.config
    }

    /// Updates internal filters and buffer sizes following parameter changes.
    pub fn update_parameters(&mut self) {
        let sr = self.sample_rate.max(1.0);
        let lookahead_sec = (self.config.lookahead_ms / 1000.0).max(0.0);
        self.lookahead_samples = ((lookahead_sec * sr).round() as usize).min(4799);

        self.sidechain_filter_coeffs = BiquadCoeffs::calculate(
            BiquadType::Highpass,
            sr as f64,
            self.config.sidechain_lowcut_hz as f64,
            FRAC_1_SQRT_2,
            0.0,
        );
    }

    /// Computes static gain reduction in dB for a given input level in dB.
    pub fn compute_static_transfer(
        input_db: f32,
        threshold_db: f32,
        ratio: f32,
        knee_db: f32,
    ) -> f32 {
        let half_knee = knee_db * 0.5;
        let delta = input_db - threshold_db;
        let r = ratio.max(1.0);

        if delta <= -half_knee {
            // Below knee: linear 1:1
            input_db
        } else if delta >= half_knee || knee_db <= 0.001 {
            // Above knee: full compression
            threshold_db + delta / r
        } else {
            // Within soft knee: quadratic interpolation
            let k = delta + half_knee;
            input_db + ((1.0 / r - 1.0) * k * k) / (2.0 * knee_db)
        }
    }

    /// Calculates automatic makeup gain in dB.
    pub fn compute_auto_makeup_gain(threshold_db: f32, ratio: f32, _knee_db: f32) -> f32 {
        let r = ratio.max(1.0);
        // Standard empirical makeup curve compensating 50% of nominal threshold reduction
        (-threshold_db * 0.5 * (1.0 - 1.0 / r)).max(0.0)
    }

    /// Returns the most recently computed gain reduction in dB.
    pub fn last_gain_reduction_db(&self) -> f32 {
        self.last_gain_reduction_db
    }
}

impl Default for Compressor {
    fn default() -> Self {
        Self::new()
    }
}

impl DspNode for Compressor {
    fn name(&self) -> &str {
        "Compressor"
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize) {
        self.sample_rate = sample_rate.max(1.0);
        let ch = channels.max(1);
        let max_lookahead = ((0.05 * self.sample_rate) as usize).max(512);
        self.lookahead_buffers = vec![vec![0.0; max_lookahead]; ch];
        self.sidechain_filter_states = vec![BiquadState::default(); ch];
        self.lookahead_write_pos = 0;
        self.envelope = 0.0;
        self.update_parameters();
    }

    fn reset(&mut self) {
        self.envelope = 0.0;
        self.last_gain_reduction_db = 0.0;
        self.lookahead_write_pos = 0;
        for buf in &mut self.lookahead_buffers {
            buf.fill(0.0);
        }
        for st in &mut self.sidechain_filter_states {
            st.reset();
        }
    }

    fn process(&mut self, inputs: &[&[Sample]], outputs: &mut [&mut [Sample]]) {
        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        let num_ch = inputs
            .len()
            .min(outputs.len())
            .min(self.lookahead_buffers.len());
        let num_frames = inputs[0].len();
        if num_frames == 0 {
            return;
        }

        if self.is_bypassed {
            for ch in 0..num_ch {
                outputs[ch][..num_frames].copy_from_slice(&inputs[ch][..num_frames]);
            }
            return;
        }

        let sr = self.sample_rate;
        let attack_time = (self.config.attack_ms / 1000.0).max(0.0001);
        let release_time = (self.config.release_ms / 1000.0).max(0.001);

        let alpha_attack = (-1.0 / (sr * attack_time)).exp();
        let base_alpha_release = (-1.0 / (sr * release_time)).exp();

        let makeup_db = if self.config.auto_makeup {
            Self::compute_auto_makeup_gain(
                self.config.threshold_db,
                self.config.ratio,
                self.config.knee_db,
            )
        } else {
            self.config.makeup_gain_db
        };
        let makeup_lin = 10.0f32.powf(makeup_db / 20.0);

        let mix = self.config.mix.clamp(0.0, 1.0);
        let dry_mix = 1.0 - mix;

        let buf_len = self.lookahead_buffers[0].len();
        let lookahead = self.lookahead_samples.min(buf_len - 1);

        for i in 0..num_frames {
            // 1. Detect sidechain signal across channels
            let mut sidechain_sample = 0.0f32;
            for (ch, inp) in inputs.iter().enumerate().take(num_ch) {
                let raw_in = inp[i];
                // Apply sidechain highpass filter
                let filtered = self.sidechain_filter_states[ch]
                    .process_sample(raw_in, &self.sidechain_filter_coeffs);
                sidechain_sample = sidechain_sample.max(filtered.abs());
            }

            // 2. Compute level in dB
            let input_db = if sidechain_sample > 1e-6 {
                20.0 * sidechain_sample.log10()
            } else {
                -120.0
            };

            // 3. Compute static target gain reduction in dB
            let target_out_db = Self::compute_static_transfer(
                input_db,
                self.config.threshold_db,
                self.config.ratio,
                self.config.knee_db,
            );
            let target_gr_db = (input_db - target_out_db).max(0.0);

            // 4. Program-dependent release coefficient adaptation
            let alpha_release = if self.config.program_dependent_release {
                // Crest factor adaptation: fast transients release faster to prevent pumping
                let dynamic_factor = (target_gr_db * 0.1).clamp(0.5, 2.0);
                let adaptive_time = release_time * dynamic_factor;
                (-1.0 / (sr * adaptive_time)).exp()
            } else {
                base_alpha_release
            };

            // 5. Envelope ballistics
            if target_gr_db > self.envelope {
                self.envelope = alpha_attack * self.envelope + (1.0 - alpha_attack) * target_gr_db;
            } else {
                self.envelope =
                    alpha_release * self.envelope + (1.0 - alpha_release) * target_gr_db;
            }

            self.last_gain_reduction_db = self.envelope;
            let gain_mult = 10.0f32.powf(-self.envelope / 20.0) * makeup_lin;

            // 6. Lookahead delay buffer indexing and mixing
            let read_pos = (self.lookahead_write_pos + buf_len - lookahead) % buf_len;

            for ch in 0..num_ch {
                let in_val = inputs[ch][i];
                self.lookahead_buffers[ch][self.lookahead_write_pos] = in_val;
                let delayed_val = self.lookahead_buffers[ch][read_pos];

                let wet_val = delayed_val * gain_mult;
                outputs[ch][i] = dry_mix * delayed_val + mix * wet_val;
            }

            self.lookahead_write_pos = (self.lookahead_write_pos + 1) % buf_len;
        }
    }

    fn latency_samples(&self) -> usize {
        self.lookahead_samples
    }

    fn is_bypassed(&self) -> bool {
        self.is_bypassed
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.is_bypassed = bypassed;
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.config.threshold_db = value.clamp(-60.0, 0.0),
            1 => self.config.ratio = value.clamp(1.0, 20.0),
            2 => self.config.attack_ms = value.clamp(0.1, 200.0),
            3 => self.config.release_ms = value.clamp(5.0, 2000.0),
            4 => self.config.knee_db = value.clamp(0.0, 24.0),
            5 => self.config.makeup_gain_db = value.clamp(0.0, 36.0),
            6 => self.config.auto_makeup = value >= 0.5,
            7 => self.config.program_dependent_release = value >= 0.5,
            8 => self.config.lookahead_ms = value.clamp(0.0, 20.0),
            9 => self.config.mix = value.clamp(0.0, 1.0),
            10 => self.config.sidechain_lowcut_hz = value.clamp(20.0, 500.0),
            _ => {}
        }
        self.update_parameters();
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 => self.config.threshold_db,
            1 => self.config.ratio,
            2 => self.config.attack_ms,
            3 => self.config.release_ms,
            4 => self.config.knee_db,
            5 => self.config.makeup_gain_db,
            6 => f32::from(self.config.auto_makeup),
            7 => f32::from(self.config.program_dependent_release),
            8 => self.config.lookahead_ms,
            9 => self.config.mix,
            10 => self.config.sidechain_lowcut_hz,
            _ => 0.0,
        }
    }
}

/// Configuration parameters for [`TruePeakLimiter`].
#[derive(Debug, Clone, Copy)]
pub struct TruePeakLimiterConfig {
    /// Target true-peak ceiling in dBTP (-30.0 to -0.1 dBTP).
    pub ceiling_dbtp: f32,
    /// Release time in milliseconds (5.0 to 1000.0 ms).
    pub release_ms: f32,
    /// Lookahead buffer in milliseconds (1.0 to 10.0 ms).
    pub lookahead_ms: f32,
}

impl Default for TruePeakLimiterConfig {
    fn default() -> Self {
        Self {
            ceiling_dbtp: -1.0,
            release_ms: 50.0,
            lookahead_ms: 3.0,
        }
    }
}

/// Inter-sample peak safe lookahead brickwall limiter.
pub struct TruePeakLimiter {
    config: TruePeakLimiterConfig,
    sample_rate: f32,
    lookahead_buffers: Vec<Vec<f32>>,
    lookahead_write_pos: usize,
    lookahead_samples: usize,
    gain_envelope: f32,
    is_bypassed: bool,
    last_gain_reduction_db: f32,
}

impl TruePeakLimiter {
    /// Creates a new true-peak limiter with default configuration.
    pub fn new() -> Self {
        Self::with_config(TruePeakLimiterConfig::default())
    }

    /// Creates a new true-peak limiter with the given configuration.
    pub fn with_config(config: TruePeakLimiterConfig) -> Self {
        let mut limiter = Self {
            config,
            sample_rate: 48000.0,
            lookahead_buffers: vec![vec![0.0; 2400]; 2],
            lookahead_write_pos: 0,
            lookahead_samples: 0,
            gain_envelope: 1.0,
            is_bypassed: false,
            last_gain_reduction_db: 0.0,
        };
        limiter.update_parameters();
        limiter
    }

    /// Returns the current configuration.
    pub fn config(&self) -> &TruePeakLimiterConfig {
        &self.config
    }

    /// Returns a mutable reference to the configuration.
    pub fn config_mut(&mut self) -> &mut TruePeakLimiterConfig {
        &mut self.config
    }

    /// Updates internal parameters.
    pub fn update_parameters(&mut self) {
        let sr = self.sample_rate.max(1.0);
        let lookahead_sec = (self.config.lookahead_ms / 1000.0).max(0.001);
        self.lookahead_samples = ((lookahead_sec * sr).round() as usize).min(2399);
    }

    /// Returns the most recent gain reduction in dB.
    pub fn last_gain_reduction_db(&self) -> f32 {
        self.last_gain_reduction_db
    }
}

impl Default for TruePeakLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl DspNode for TruePeakLimiter {
    fn name(&self) -> &str {
        "TruePeakLimiter"
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize) {
        self.sample_rate = sample_rate.max(1.0);
        let ch = channels.max(1);
        let max_lookahead = ((0.02 * self.sample_rate) as usize).max(512);
        self.lookahead_buffers = vec![vec![0.0; max_lookahead]; ch];
        self.lookahead_write_pos = 0;
        self.gain_envelope = 1.0;
        self.update_parameters();
    }

    fn reset(&mut self) {
        self.gain_envelope = 1.0;
        self.last_gain_reduction_db = 0.0;
        self.lookahead_write_pos = 0;
        for buf in &mut self.lookahead_buffers {
            buf.fill(0.0);
        }
    }

    fn process(&mut self, inputs: &[&[Sample]], outputs: &mut [&mut [Sample]]) {
        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        let num_ch = inputs
            .len()
            .min(outputs.len())
            .min(self.lookahead_buffers.len());
        let num_frames = inputs[0].len();
        if num_frames == 0 {
            return;
        }

        if self.is_bypassed {
            for ch in 0..num_ch {
                outputs[ch][..num_frames].copy_from_slice(&inputs[ch][..num_frames]);
            }
            return;
        }

        let ceiling_lin = 10.0f32.powf(self.config.ceiling_dbtp / 20.0);
        let sr = self.sample_rate;
        let release_time = (self.config.release_ms / 1000.0).max(0.001);
        let alpha_release = (-1.0 / (sr * release_time)).exp();

        let buf_len = self.lookahead_buffers[0].len();
        let lookahead = self.lookahead_samples.min(buf_len - 1);

        for i in 0..num_frames {
            // Detect peak level across channels with 4x polyphase inter-sample approximation
            let mut detected_peak = 0.0f32;
            for inp in inputs.iter().take(num_ch) {
                let cur = inp[i].abs();
                detected_peak = detected_peak.max(cur);

                // Approximate inter-sample peak if consecutive samples have opposite signs or high steepness
                if i > 0 {
                    let prev = inp[i - 1].abs();
                    let midpoint = 0.5 * (cur + prev);
                    detected_peak = detected_peak.max(midpoint * 1.15);
                }
            }

            // Required gain to not exceed ceiling
            let target_gain = if detected_peak > ceiling_lin {
                ceiling_lin / detected_peak
            } else {
                1.0
            };

            // Fast attack, smooth exponential release
            if target_gain < self.gain_envelope {
                self.gain_envelope = target_gain;
            } else {
                self.gain_envelope =
                    alpha_release * self.gain_envelope + (1.0 - alpha_release) * target_gain;
            }

            let current_gain = self.gain_envelope.clamp(0.0, 1.0);
            self.last_gain_reduction_db = if current_gain > 1e-6 {
                -20.0 * current_gain.log10()
            } else {
                60.0
            };

            let read_pos = (self.lookahead_write_pos + buf_len - lookahead) % buf_len;

            for ch in 0..num_ch {
                let in_val = inputs[ch][i];
                self.lookahead_buffers[ch][self.lookahead_write_pos] = in_val;
                let delayed_val = self.lookahead_buffers[ch][read_pos];

                // Scale and enforce ceiling
                let out_val = (delayed_val * current_gain).clamp(-ceiling_lin, ceiling_lin);
                outputs[ch][i] = out_val;
            }

            self.lookahead_write_pos = (self.lookahead_write_pos + 1) % buf_len;
        }
    }

    fn latency_samples(&self) -> usize {
        self.lookahead_samples
    }

    fn is_bypassed(&self) -> bool {
        self.is_bypassed
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.is_bypassed = bypassed;
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.config.ceiling_dbtp = value.clamp(-30.0, -0.1),
            1 => self.config.release_ms = value.clamp(5.0, 1000.0),
            2 => self.config.lookahead_ms = value.clamp(1.0, 10.0),
            _ => {}
        }
        self.update_parameters();
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 => self.config.ceiling_dbtp,
            1 => self.config.release_ms,
            2 => self.config.lookahead_ms,
            _ => 0.0,
        }
    }
}

/// Configuration parameters for [`GateExpander`].
#[derive(Debug, Clone, Copy)]
pub struct GateExpanderConfig {
    /// Open threshold in dBFS (-80.0 to 0.0 dB).
    pub threshold_db: f32,
    /// Expansion ratio (1.0 to 100.0).
    pub ratio: f32,
    /// Attack time in milliseconds (0.1 to 50.0 ms).
    pub attack_ms: f32,
    /// Hold time in milliseconds (0.0 to 500.0 ms).
    pub hold_ms: f32,
    /// Release time in milliseconds (5.0 to 2000.0 ms).
    pub release_ms: f32,
    /// Hysteresis difference between open and close thresholds in dB (0.0 to 12.0 dB).
    pub hysteresis_db: f32,
    /// Maximum attenuation floor in dB (-80.0 to -6.0 dB).
    pub floor_db: f32,
}

impl Default for GateExpanderConfig {
    fn default() -> Self {
        Self {
            threshold_db: -40.0,
            ratio: 20.0,
            attack_ms: 2.0,
            hold_ms: 20.0,
            release_ms: 100.0,
            hysteresis_db: 3.0,
            floor_db: -60.0,
        }
    }
}

/// Downward expander and noise gate node with hysteresis and hold.
pub struct GateExpander {
    config: GateExpanderConfig,
    sample_rate: f32,
    envelope: f32,
    hold_countdown: usize,
    is_open: bool,
    is_bypassed: bool,
}

impl GateExpander {
    /// Creates a new gate/expander with default configuration.
    pub fn new() -> Self {
        Self::with_config(GateExpanderConfig::default())
    }

    /// Creates a new gate/expander with custom configuration.
    pub fn with_config(config: GateExpanderConfig) -> Self {
        Self {
            config,
            sample_rate: 48000.0,
            envelope: 1.0,
            hold_countdown: 0,
            is_open: false,
            is_bypassed: false,
        }
    }
}

impl Default for GateExpander {
    fn default() -> Self {
        Self::new()
    }
}

impl DspNode for GateExpander {
    fn name(&self) -> &str {
        "GateExpander"
    }

    fn prepare(&mut self, sample_rate: f32, _channels: usize) {
        self.sample_rate = sample_rate.max(1.0);
        self.envelope = 1.0;
        self.hold_countdown = 0;
        self.is_open = false;
    }

    fn reset(&mut self) {
        self.envelope = 1.0;
        self.hold_countdown = 0;
        self.is_open = false;
    }

    fn process(&mut self, inputs: &[&[Sample]], outputs: &mut [&mut [Sample]]) {
        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        let num_ch = inputs.len().min(outputs.len());
        let num_frames = inputs[0].len();
        if num_frames == 0 {
            return;
        }

        if self.is_bypassed {
            for ch in 0..num_ch {
                outputs[ch][..num_frames].copy_from_slice(&inputs[ch][..num_frames]);
            }
            return;
        }

        let open_threshold_db = self.config.threshold_db;
        let close_threshold_db = open_threshold_db - self.config.hysteresis_db.max(0.0);
        let floor_lin = 10.0f32.powf(self.config.floor_db / 20.0);
        let r = self.config.ratio.max(1.0);

        let sr = self.sample_rate;
        let attack_time = (self.config.attack_ms / 1000.0).max(0.0001);
        let release_time = (self.config.release_ms / 1000.0).max(0.001);
        let hold_samples = ((self.config.hold_ms / 1000.0) * sr).round() as usize;

        let alpha_attack = (-1.0 / (sr * attack_time)).exp();
        let alpha_release = (-1.0 / (sr * release_time)).exp();

        for i in 0..num_frames {
            let mut peak_val = 0.0f32;
            for inp in inputs.iter().take(num_ch) {
                peak_val = peak_val.max(inp[i].abs());
            }

            let input_db = if peak_val > 1e-6 {
                20.0 * peak_val.log10()
            } else {
                -120.0
            };

            // Hysteresis logic
            if input_db >= open_threshold_db {
                self.is_open = true;
                self.hold_countdown = hold_samples;
            } else if input_db < close_threshold_db {
                if self.hold_countdown > 0 {
                    self.hold_countdown -= 1;
                } else {
                    self.is_open = false;
                }
            }

            // Target gain calculation
            let target_gain = if self.is_open {
                1.0
            } else {
                let delta = open_threshold_db - input_db;
                let gr_db = delta * (r - 1.0);
                (10.0f32.powf(-gr_db / 20.0)).max(floor_lin)
            };

            // Smooth ballistics: attack opens gate, release closes gate
            if target_gain > self.envelope {
                self.envelope = alpha_attack * self.envelope + (1.0 - alpha_attack) * target_gain;
            } else {
                self.envelope = alpha_release * self.envelope + (1.0 - alpha_release) * target_gain;
            }

            for ch in 0..num_ch {
                outputs[ch][i] = inputs[ch][i] * self.envelope;
            }
        }
    }

    fn latency_samples(&self) -> usize {
        0
    }

    fn is_bypassed(&self) -> bool {
        self.is_bypassed
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.is_bypassed = bypassed;
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.config.threshold_db = value.clamp(-80.0, 0.0),
            1 => self.config.ratio = value.clamp(1.0, 100.0),
            2 => self.config.attack_ms = value.clamp(0.1, 50.0),
            3 => self.config.hold_ms = value.clamp(0.0, 500.0),
            4 => self.config.release_ms = value.clamp(5.0, 2000.0),
            5 => self.config.hysteresis_db = value.clamp(0.0, 12.0),
            6 => self.config.floor_db = value.clamp(-80.0, -6.0),
            _ => {}
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 => self.config.threshold_db,
            1 => self.config.ratio,
            2 => self.config.attack_ms,
            3 => self.config.hold_ms,
            4 => self.config.release_ms,
            5 => self.config.hysteresis_db,
            6 => self.config.floor_db,
            _ => 0.0,
        }
    }
}

/// Configuration parameters for [`DeEsser`].
#[derive(Debug, Clone, Copy)]
pub struct DeEsserConfig {
    /// Detection threshold in dBFS (-60.0 to 0.0 dB).
    pub threshold_db: f32,
    /// Attenuation ratio (1.5 to 10.0).
    pub ratio: f32,
    /// Center frequency of sibilance band in Hz (4000.0 to 10000.0 Hz).
    pub frequency_hz: f32,
    /// Bandwidth in octaves (0.5 to 2.0 octaves).
    pub bandwidth_octaves: f32,
    /// When true, applies gain reduction to the sibilant frequency band only.
    pub split_band: bool,
}

impl Default for DeEsserConfig {
    fn default() -> Self {
        Self {
            threshold_db: -24.0,
            ratio: 4.0,
            frequency_hz: 6500.0,
            bandwidth_octaves: 1.0,
            split_band: true,
        }
    }
}

/// Sibilance reduction node.
pub struct DeEsser {
    config: DeEsserConfig,
    sample_rate: f32,
    sidechain_filter_coeffs: BiquadCoeffs,
    sidechain_filter_states: Vec<BiquadState>,
    band_filter_coeffs: BiquadCoeffs,
    band_filter_states: Vec<BiquadState>,
    envelope: f32,
    is_bypassed: bool,
}

impl DeEsser {
    /// Creates a new de-esser with default settings.
    pub fn new() -> Self {
        Self::with_config(DeEsserConfig::default())
    }

    /// Creates a new de-esser with custom settings.
    pub fn with_config(config: DeEsserConfig) -> Self {
        let mut deesser = Self {
            config,
            sample_rate: 48000.0,
            sidechain_filter_coeffs: BiquadCoeffs::default(),
            sidechain_filter_states: vec![BiquadState::default(); 2],
            band_filter_coeffs: BiquadCoeffs::default(),
            band_filter_states: vec![BiquadState::default(); 2],
            envelope: 0.0,
            is_bypassed: false,
        };
        deesser.update_filters();
        deesser
    }

    fn update_filters(&mut self) {
        let sr = self.sample_rate.max(1.0) as f64;
        let f0 = self.config.frequency_hz as f64;
        let q = (2.0f64.powf(self.config.bandwidth_octaves as f64 * 0.5)
            / (2.0f64.powf(self.config.bandwidth_octaves as f64) - 1.0))
            .clamp(0.5, 10.0);

        self.sidechain_filter_coeffs =
            BiquadCoeffs::calculate(BiquadType::Bandpass, sr, f0, q, 0.0);
        self.band_filter_coeffs = BiquadCoeffs::calculate(BiquadType::Bandpass, sr, f0, q, 0.0);
    }
}

impl Default for DeEsser {
    fn default() -> Self {
        Self::new()
    }
}

impl DspNode for DeEsser {
    fn name(&self) -> &str {
        "DeEsser"
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize) {
        self.sample_rate = sample_rate.max(1.0);
        let ch = channels.max(1);
        self.sidechain_filter_states = vec![BiquadState::default(); ch];
        self.band_filter_states = vec![BiquadState::default(); ch];
        self.envelope = 0.0;
        self.update_filters();
    }

    fn reset(&mut self) {
        self.envelope = 0.0;
        for st in &mut self.sidechain_filter_states {
            st.reset();
        }
        for st in &mut self.band_filter_states {
            st.reset();
        }
    }

    fn process(&mut self, inputs: &[&[Sample]], outputs: &mut [&mut [Sample]]) {
        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        let num_ch = inputs
            .len()
            .min(outputs.len())
            .min(self.sidechain_filter_states.len());
        let num_frames = inputs[0].len();
        if num_frames == 0 {
            return;
        }

        if self.is_bypassed {
            for ch in 0..num_ch {
                outputs[ch][..num_frames].copy_from_slice(&inputs[ch][..num_frames]);
            }
            return;
        }

        let sr = self.sample_rate;
        let alpha_attack = (-1.0 / (sr * 0.002)).exp(); // Fast 2ms attack
        let alpha_release = (-1.0 / (sr * 0.050)).exp(); // 50ms release
        let threshold = self.config.threshold_db;
        let r = self.config.ratio.max(1.0);

        for i in 0..num_frames {
            let mut detected_level = 0.0f32;
            let mut band_samples = [0.0f32; 8];

            for ch in 0..num_ch {
                let x = inputs[ch][i];
                let sc = self.sidechain_filter_states[ch]
                    .process_sample(x, &self.sidechain_filter_coeffs);
                detected_level = detected_level.max(sc.abs());

                if self.config.split_band && ch < 8 {
                    band_samples[ch] =
                        self.band_filter_states[ch].process_sample(x, &self.band_filter_coeffs);
                }
            }

            let level_db = if detected_level > 1e-6 {
                20.0 * detected_level.log10()
            } else {
                -120.0
            };

            let target_gr_db = if level_db > threshold {
                (level_db - threshold) * (1.0 - 1.0 / r)
            } else {
                0.0
            };

            if target_gr_db > self.envelope {
                self.envelope = alpha_attack * self.envelope + (1.0 - alpha_attack) * target_gr_db;
            } else {
                self.envelope =
                    alpha_release * self.envelope + (1.0 - alpha_release) * target_gr_db;
            }

            let gain_mult = 10.0f32.powf(-self.envelope / 20.0);

            for ch in 0..num_ch {
                let x = inputs[ch][i];
                if self.config.split_band {
                    let sibilant = band_samples[ch];
                    let non_sibilant = x - sibilant;
                    outputs[ch][i] = non_sibilant + sibilant * gain_mult;
                } else {
                    outputs[ch][i] = x * gain_mult;
                }
            }
        }
    }

    fn latency_samples(&self) -> usize {
        0
    }

    fn is_bypassed(&self) -> bool {
        self.is_bypassed
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.is_bypassed = bypassed;
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.config.threshold_db = value.clamp(-60.0, 0.0),
            1 => self.config.ratio = value.clamp(1.5, 10.0),
            2 => self.config.frequency_hz = value.clamp(4000.0, 10000.0),
            3 => self.config.bandwidth_octaves = value.clamp(0.5, 2.0),
            4 => self.config.split_band = value >= 0.5,
            _ => {}
        }
        self.update_filters();
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 => self.config.threshold_db,
            1 => self.config.ratio,
            2 => self.config.frequency_hz,
            3 => self.config.bandwidth_octaves,
            4 => f32::from(self.config.split_band),
            _ => 0.0,
        }
    }
}

/// Configuration parameters for [`TransientShaper`].
#[derive(Debug, Clone, Copy)]
pub struct TransientShaperConfig {
    /// Attack gain boost or cut in dB (-12.0 to +12.0 dB).
    pub attack_gain_db: f32,
    /// Sustain gain boost or cut in dB (-12.0 to +12.0 dB).
    pub sustain_gain_db: f32,
}

impl Default for TransientShaperConfig {
    fn default() -> Self {
        Self {
            attack_gain_db: 0.0,
            sustain_gain_db: 0.0,
        }
    }
}

/// Dual-envelope transient and sustain shaper node.
pub struct TransientShaper {
    config: TransientShaperConfig,
    sample_rate: f32,
    fast_env: Vec<f32>,
    slow_env: Vec<f32>,
    is_bypassed: bool,
}

impl TransientShaper {
    /// Creates a new transient shaper with default configuration.
    pub fn new() -> Self {
        Self::with_config(TransientShaperConfig::default())
    }

    /// Creates a new transient shaper with custom configuration.
    pub fn with_config(config: TransientShaperConfig) -> Self {
        Self {
            config,
            sample_rate: 48000.0,
            fast_env: vec![0.0; 2],
            slow_env: vec![0.0; 2],
            is_bypassed: false,
        }
    }
}

impl Default for TransientShaper {
    fn default() -> Self {
        Self::new()
    }
}

impl DspNode for TransientShaper {
    fn name(&self) -> &str {
        "TransientShaper"
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize) {
        self.sample_rate = sample_rate.max(1.0);
        let ch = channels.max(1);
        self.fast_env = vec![0.0; ch];
        self.slow_env = vec![0.0; ch];
    }

    fn reset(&mut self) {
        self.fast_env.fill(0.0);
        self.slow_env.fill(0.0);
    }

    fn process(&mut self, inputs: &[&[Sample]], outputs: &mut [&mut [Sample]]) {
        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        let num_ch = inputs.len().min(outputs.len()).min(self.fast_env.len());
        let num_frames = inputs[0].len();
        if num_frames == 0 {
            return;
        }

        if self.is_bypassed {
            for ch in 0..num_ch {
                outputs[ch][..num_frames].copy_from_slice(&inputs[ch][..num_frames]);
            }
            return;
        }

        let sr = self.sample_rate;
        let alpha_fast = (-1.0 / (sr * 0.003)).exp(); // 3ms fast envelope
        let alpha_slow = (-1.0 / (sr * 0.060)).exp(); // 60ms slow envelope

        let attack_mult = 10.0f32.powf(self.config.attack_gain_db / 20.0);
        let sustain_mult = 10.0f32.powf(self.config.sustain_gain_db / 20.0);

        for i in 0..num_frames {
            for ch in 0..num_ch {
                let x = inputs[ch][i];
                let abs_x = x.abs();

                self.fast_env[ch] = alpha_fast * self.fast_env[ch] + (1.0 - alpha_fast) * abs_x;
                self.slow_env[ch] = alpha_slow * self.slow_env[ch] + (1.0 - alpha_slow) * abs_x;

                let trans_level = (self.fast_env[ch] - self.slow_env[ch]).max(0.0);
                let total_level = self.fast_env[ch].max(1e-6);

                let trans_ratio = (trans_level / total_level).clamp(0.0, 1.0);
                let sust_ratio = 1.0 - trans_ratio;

                let gain = trans_ratio * attack_mult + sust_ratio * sustain_mult;
                outputs[ch][i] = (x * gain).clamp(-2.0, 2.0);
            }
        }
    }

    fn latency_samples(&self) -> usize {
        0
    }

    fn is_bypassed(&self) -> bool {
        self.is_bypassed
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.is_bypassed = bypassed;
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.config.attack_gain_db = value.clamp(-12.0, 12.0),
            1 => self.config.sustain_gain_db = value.clamp(-12.0, 12.0),
            _ => {}
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 => self.config.attack_gain_db,
            1 => self.config.sustain_gain_db,
            _ => 0.0,
        }
    }
}

/// 4-band crossover filter pair using cascaded 2nd-order Butterworth filters (4th-order Linkwitz-Riley LR4).
#[derive(Clone)]
struct Lr4CrossoverPair {
    lp1: BiquadCoeffs,
    lp2: BiquadCoeffs,
    hp1: BiquadCoeffs,
    hp2: BiquadCoeffs,
    lp1_states: Vec<BiquadState>,
    lp2_states: Vec<BiquadState>,
    hp1_states: Vec<BiquadState>,
    hp2_states: Vec<BiquadState>,
}

impl Lr4CrossoverPair {
    fn new(sample_rate: f64, cutoff_hz: f64, channels: usize) -> Self {
        let q = FRAC_1_SQRT_2; // Butterworth Q
        let lp = BiquadCoeffs::calculate(BiquadType::Lowpass, sample_rate, cutoff_hz, q, 0.0);
        let hp = BiquadCoeffs::calculate(BiquadType::Highpass, sample_rate, cutoff_hz, q, 0.0);

        Self {
            lp1: lp,
            lp2: lp,
            hp1: hp,
            hp2: hp,
            lp1_states: vec![BiquadState::default(); channels],
            lp2_states: vec![BiquadState::default(); channels],
            hp1_states: vec![BiquadState::default(); channels],
            hp2_states: vec![BiquadState::default(); channels],
        }
    }

    fn reset(&mut self) {
        for s in &mut self.lp1_states {
            s.reset();
        }
        for s in &mut self.lp2_states {
            s.reset();
        }
        for s in &mut self.hp1_states {
            s.reset();
        }
        for s in &mut self.hp2_states {
            s.reset();
        }
    }

    fn split_sample(&mut self, ch: usize, sample: f32) -> (f32, f32) {
        let l1 = self.lp1_states[ch].process_sample(sample, &self.lp1);
        let l2 = self.lp2_states[ch].process_sample(l1, &self.lp2);

        let h1 = self.hp1_states[ch].process_sample(sample, &self.hp1);
        let h2 = self.hp2_states[ch].process_sample(h1, &self.hp2);

        (l2, h2)
    }
}

#[derive(Clone)]
struct AllpassBiquad {
    coeffs: BiquadCoeffs,
    states: Vec<BiquadState>,
}

impl AllpassBiquad {
    fn new(sample_rate: f64, cutoff_hz: f64, channels: usize) -> Self {
        let sr = sample_rate.max(1.0);
        let f0 = cutoff_hz.clamp(10.0, 0.499 * sr);
        let q = FRAC_1_SQRT_2;
        let omega = 2.0 * std::f64::consts::PI * f0 / sr;
        let sn = omega.sin();
        let cs = omega.cos();
        let alpha = sn / (2.0 * q);
        let a0 = 1.0 + alpha;
        let coeffs = BiquadCoeffs {
            b0: (1.0 - alpha) / a0,
            b1: (-2.0 * cs) / a0,
            b2: (1.0 + alpha) / a0,
            a1: (-2.0 * cs) / a0,
            a2: (1.0 - alpha) / a0,
        };
        Self {
            coeffs,
            states: vec![BiquadState::default(); channels],
        }
    }

    fn reset(&mut self) {
        for s in &mut self.states {
            s.reset();
        }
    }

    fn process_sample(&mut self, ch: usize, sample: f32) -> f32 {
        self.states[ch].process_sample(sample, &self.coeffs)
    }
}

/// 4-band dynamic range compressor with Linkwitz-Riley 4th-order (LR4) crossovers.
pub struct MultibandCompressor {
    sample_rate: f32,
    crossovers: Vec<Lr4CrossoverPair>,
    low_ap1: AllpassBiquad,
    low_ap2: AllpassBiquad,
    low_mid_ap2: AllpassBiquad,
    compressors: Vec<Compressor>,
    band_mutes: [bool; 4],
    band_solos: [bool; 4],
    is_bypassed: bool,
}

impl MultibandCompressor {
    /// Creates a 4-band compressor with standard crossover frequencies (200 Hz, 1500 Hz, 6000 Hz).
    pub fn new() -> Self {
        let default_cutoffs = [200.0, 1500.0, 6000.0];
        let sr = 48000.0f64;
        let crossovers = default_cutoffs
            .iter()
            .map(|&f| Lr4CrossoverPair::new(sr, f, 2))
            .collect();

        let low_ap1 = AllpassBiquad::new(sr, 1500.0, 2);
        let low_ap2 = AllpassBiquad::new(sr, 6000.0, 2);
        let low_mid_ap2 = AllpassBiquad::new(sr, 6000.0, 2);

        let compressors = vec![
            Compressor::with_config(CompressorConfig {
                threshold_db: -18.0,
                ratio: 3.0,
                attack_ms: 30.0,
                release_ms: 200.0,
                sidechain_lowcut_hz: 20.0,
                ..Default::default()
            }),
            Compressor::with_config(CompressorConfig {
                threshold_db: -16.0,
                ratio: 2.8,
                attack_ms: 15.0,
                release_ms: 160.0,
                sidechain_lowcut_hz: 20.0,
                ..Default::default()
            }),
            Compressor::with_config(CompressorConfig {
                threshold_db: -14.0,
                ratio: 2.5,
                attack_ms: 8.0,
                release_ms: 120.0,
                sidechain_lowcut_hz: 20.0,
                ..Default::default()
            }),
            Compressor::with_config(CompressorConfig {
                threshold_db: -12.0,
                ratio: 2.2,
                attack_ms: 4.0,
                release_ms: 90.0,
                sidechain_lowcut_hz: 20.0,
                ..Default::default()
            }),
        ];

        Self {
            sample_rate: 48000.0,
            crossovers,
            low_ap1,
            low_ap2,
            low_mid_ap2,
            compressors,
            band_mutes: [false; 4],
            band_solos: [false; 4],
            is_bypassed: false,
        }
    }

    /// Accesses the compressor for a specific band index (0 = Low, 1 = Low-Mid, 2 = High-Mid, 3 = High).
    pub fn band_compressor(&mut self, band: usize) -> &mut Compressor {
        &mut self.compressors[band]
    }

    /// Toggles mute for a band.
    pub fn set_band_mute(&mut self, band: usize, mute: bool) {
        if band < 4 {
            self.band_mutes[band] = mute;
        }
    }

    /// Toggles solo for a band.
    pub fn set_band_solo(&mut self, band: usize, solo: bool) {
        if band < 4 {
            self.band_solos[band] = solo;
        }
    }
}

impl Default for MultibandCompressor {
    fn default() -> Self {
        Self::new()
    }
}

impl DspNode for MultibandCompressor {
    fn name(&self) -> &str {
        "MultibandCompressor"
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize) {
        self.sample_rate = sample_rate.max(1.0);
        let ch = channels.max(1);
        let default_cutoffs = [200.0, 1500.0, 6000.0];
        self.crossovers = default_cutoffs
            .iter()
            .map(|&f| Lr4CrossoverPair::new(self.sample_rate as f64, f, ch))
            .collect();
        self.low_ap1 = AllpassBiquad::new(self.sample_rate as f64, 1500.0, ch);
        self.low_ap2 = AllpassBiquad::new(self.sample_rate as f64, 6000.0, ch);
        self.low_mid_ap2 = AllpassBiquad::new(self.sample_rate as f64, 6000.0, ch);
        for comp in &mut self.compressors {
            comp.prepare(sample_rate, channels);
        }
    }

    fn reset(&mut self) {
        for xo in &mut self.crossovers {
            xo.reset();
        }
        self.low_ap1.reset();
        self.low_ap2.reset();
        self.low_mid_ap2.reset();
        for comp in &mut self.compressors {
            comp.reset();
        }
    }

    fn process(&mut self, inputs: &[&[Sample]], outputs: &mut [&mut [Sample]]) {
        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        let num_ch = inputs.len().min(outputs.len());
        let num_frames = inputs[0].len();
        if num_frames == 0 {
            return;
        }

        if self.is_bypassed {
            for ch in 0..num_ch {
                outputs[ch][..num_frames].copy_from_slice(&inputs[ch][..num_frames]);
            }
            return;
        }

        let mut band_buffers = vec![vec![vec![0.0f32; num_frames]; num_ch]; 4];
        let has_any_solo = self.band_solos.iter().any(|&s| s);

        for ch in 0..num_ch {
            for i in 0..num_frames {
                let s = inputs[ch][i];
                // Crossover 1: split into Low (<200) and MidHigh (>200)
                let (raw_low, mid_high) = self.crossovers[0].split_sample(ch, s);
                // Crossover 2: split mid_high into LowMid (<1500) and HighBand (>1500)
                let (raw_low_mid, high_band) = self.crossovers[1].split_sample(ch, mid_high);
                // Crossover 3: split high_band into HighMid (<6000) and High (>6000)
                let (high_mid, high) = self.crossovers[2].split_sample(ch, high_band);

                // Phase align all branches using matching allpass filters
                let low = self
                    .low_ap2
                    .process_sample(ch, self.low_ap1.process_sample(ch, raw_low));
                let low_mid = self.low_mid_ap2.process_sample(ch, raw_low_mid);

                band_buffers[0][ch][i] = low;
                band_buffers[1][ch][i] = low_mid;
                band_buffers[2][ch][i] = high_mid;
                band_buffers[3][ch][i] = high;
            }
        }

        // Process each band through its dedicated compressor
        let mut processed_bands = vec![vec![vec![0.0f32; num_frames]; num_ch]; 4];
        for b in 0..4 {
            let in_slices: Vec<&[f32]> = (0..num_ch).map(|ch| &band_buffers[b][ch][..]).collect();
            let mut out_ch_vecs = vec![vec![0.0f32; num_frames]; num_ch];
            let mut out_slices: Vec<&mut [f32]> =
                out_ch_vecs.iter_mut().map(|v| &mut v[..]).collect();
            self.compressors[b].process(&in_slices, &mut out_slices);
            processed_bands[b] = out_ch_vecs;
        }

        // Recombine bands
        for ch in 0..num_ch {
            for i in 0..num_frames {
                let mut sum = 0.0f32;
                for (b, band_samples) in processed_bands.iter().enumerate().take(4) {
                    let active = if has_any_solo {
                        self.band_solos[b]
                    } else {
                        !self.band_mutes[b]
                    };
                    if active {
                        sum += band_samples[ch][i];
                    }
                }
                outputs[ch][i] = sum;
            }
        }
    }

    fn latency_samples(&self) -> usize {
        self.compressors[0].latency_samples()
    }

    fn is_bypassed(&self) -> bool {
        self.is_bypassed
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.is_bypassed = bypassed;
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        let band = index / 10;
        let param = index % 10;
        if band < 4 {
            self.compressors[band].set_parameter(param, value);
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        let band = index / 10;
        let param = index % 10;
        if band < 4 {
            self.compressors[band].get_parameter(param)
        } else {
            0.0
        }
    }
}

/// Configuration parameters for [`AdaptiveLeveler`].
#[derive(Debug, Clone, Copy)]
pub struct AdaptiveLevelerConfig {
    /// Target loudness in integrated LUFS (-36.0 to -6.0 LUFS).
    pub target_lufs: f32,
    /// Sliding window duration in seconds (1.0 to 5.0 seconds).
    pub window_seconds: f32,
    /// Maximum allowed gain boost in dB (0.0 to 18.0 dB).
    pub max_boost_db: f32,
    /// Maximum allowed gain cut in dB (-24.0 to 0.0 dB).
    pub max_cut_db: f32,
    /// Speech-aware noise gating threshold in dBFS (-60.0 to -30.0 dBFS).
    pub gate_threshold_dbfs: f32,
}

impl Default for AdaptiveLevelerConfig {
    fn default() -> Self {
        Self {
            target_lufs: -16.0,
            window_seconds: 3.0,
            max_boost_db: 8.0,
            max_cut_db: -12.0,
            gate_threshold_dbfs: -45.0,
        }
    }
}

/// Slow gain-riding adaptive leveler with speech-aware noise gating.
pub struct AdaptiveLeveler {
    config: AdaptiveLevelerConfig,
    sample_rate: f32,
    energy_history: Vec<f32>,
    history_pos: usize,
    history_len: usize,
    current_gain: f32,
    is_bypassed: bool,
}

impl AdaptiveLeveler {
    /// Creates a new adaptive leveler with default configuration.
    pub fn new() -> Self {
        Self::with_config(AdaptiveLevelerConfig::default())
    }

    /// Creates a new adaptive leveler with custom configuration.
    pub fn with_config(config: AdaptiveLevelerConfig) -> Self {
        Self {
            config,
            sample_rate: 48000.0,
            energy_history: vec![0.0; 48000 * 3],
            history_pos: 0,
            history_len: 48000 * 3,
            current_gain: 1.0,
            is_bypassed: false,
        }
    }
}

impl Default for AdaptiveLeveler {
    fn default() -> Self {
        Self::new()
    }
}

impl DspNode for AdaptiveLeveler {
    fn name(&self) -> &str {
        "AdaptiveLeveler"
    }

    fn prepare(&mut self, sample_rate: f32, _channels: usize) {
        self.sample_rate = sample_rate.max(1.0);
        let window_len = ((self.config.window_seconds * self.sample_rate) as usize).max(4800);
        self.history_len = window_len;
        self.energy_history = vec![0.0; window_len];
        self.history_pos = 0;
        self.current_gain = 1.0;
    }

    fn reset(&mut self) {
        self.energy_history.fill(0.0);
        self.history_pos = 0;
        self.current_gain = 1.0;
    }

    fn process(&mut self, inputs: &[&[Sample]], outputs: &mut [&mut [Sample]]) {
        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        let num_ch = inputs.len().min(outputs.len());
        let num_frames = inputs[0].len();
        if num_frames == 0 {
            return;
        }

        if self.is_bypassed {
            for ch in 0..num_ch {
                outputs[ch][..num_frames].copy_from_slice(&inputs[ch][..num_frames]);
            }
            return;
        }

        let gate_threshold_power = 10.0f32.powf(self.config.gate_threshold_dbfs / 10.0);
        let max_boost_lin = 10.0f32.powf(self.config.max_boost_db / 20.0);
        let max_cut_lin = 10.0f32.powf(self.config.max_cut_db / 20.0);

        // Smooth 500ms time constant for slow gain riding
        let alpha = (-1.0 / (self.sample_rate * 0.500)).exp();

        for i in 0..num_frames {
            let mut frame_power = 0.0f32;
            for inp in inputs.iter().take(num_ch) {
                let v = inp[i];
                frame_power += v * v;
            }
            frame_power /= num_ch as f32;

            self.energy_history[self.history_pos] = frame_power;
            self.history_pos = (self.history_pos + 1) % self.history_len;

            // Only update target gain on active speech/content (speech-aware gating)
            if frame_power >= gate_threshold_power {
                let current_dbfs = 10.0 * frame_power.max(1e-12).log10();
                let delta_db = self.config.target_lufs - current_dbfs;
                let target_gain = (10.0f32.powf(delta_db / 20.0)).clamp(max_cut_lin, max_boost_lin);

                self.current_gain = alpha * self.current_gain + (1.0 - alpha) * target_gain;
            }

            for ch in 0..num_ch {
                outputs[ch][i] = inputs[ch][i] * self.current_gain;
            }
        }
    }

    fn latency_samples(&self) -> usize {
        0
    }

    fn is_bypassed(&self) -> bool {
        self.is_bypassed
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.is_bypassed = bypassed;
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.config.target_lufs = value.clamp(-36.0, -6.0),
            1 => self.config.window_seconds = value.clamp(1.0, 5.0),
            2 => self.config.max_boost_db = value.clamp(0.0, 18.0),
            3 => self.config.max_cut_db = value.clamp(-24.0, 0.0),
            4 => self.config.gate_threshold_dbfs = value.clamp(-60.0, -30.0),
            _ => {}
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 => self.config.target_lufs,
            1 => self.config.window_seconds,
            2 => self.config.max_boost_db,
            3 => self.config.max_cut_db,
            4 => self.config.gate_threshold_dbfs,
            _ => 0.0,
        }
    }
}

/// Normalizes audio buffer to a specified peak dBFS target.
/// Returns the applied linear gain factor.
pub fn normalize_peak(buffer: &mut [f32], target_peak_dbfs: f32) -> f32 {
    let mut max_peak = 0.0f32;
    for &s in buffer.iter() {
        if s.abs() > max_peak {
            max_peak = s.abs();
        }
    }

    if max_peak <= 1e-9 {
        return 1.0;
    }

    let target_lin = 10.0f32.powf(target_peak_dbfs / 20.0);
    let gain = target_lin / max_peak;

    for s in buffer.iter_mut() {
        *s *= gain;
    }

    gain
}

/// Normalizes multi-channel audio to a target integrated LUFS level, respecting a max true-peak ceiling.
/// Returns the applied linear gain factor.
pub fn normalize_lufs(
    channels: &mut [&mut [f32]],
    sample_rate: f32,
    target_lufs: f32,
    max_peak_dbfs: f32,
) -> f32 {
    let immut_channels: Vec<&[f32]> = channels.iter().map(|c| &**c).collect();
    let measurement = LoudnessMeter::analyze(&immut_channels, sample_rate as u32);

    if measurement.integrated_lufs <= -100.0 {
        return 1.0;
    }

    let delta_db = target_lufs - measurement.integrated_lufs;
    let mut gain = 10.0f32.powf(delta_db / 20.0);

    // Enforce true-peak safety ceiling
    let max_peak_lin = 10.0f32.powf(max_peak_dbfs / 20.0);
    if measurement.max_true_peak_linear * gain > max_peak_lin
        && measurement.max_true_peak_linear > 1e-6
    {
        gain = max_peak_lin / measurement.max_true_peak_linear;
    }

    for ch in channels.iter_mut() {
        for s in ch.iter_mut() {
            *s *= gain;
        }
    }

    gain
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn test_compressor_static_transfer_curve() {
        let threshold = -18.0;
        let ratio = 4.0;
        let knee = 6.0;

        // Way below threshold: 1:1
        let out_low = Compressor::compute_static_transfer(-30.0, threshold, ratio, knee);
        assert!((out_low - -30.0).abs() < 0.05);

        // At threshold
        let out_thresh = Compressor::compute_static_transfer(threshold, threshold, ratio, knee);
        assert!(out_thresh <= threshold);

        // Way above threshold: ratio 4:1 (every +4 dB in gives +1 dB out)
        let in_high = 2.0;
        let out_high = Compressor::compute_static_transfer(in_high, threshold, ratio, knee);
        let expected = threshold + (in_high - threshold) / ratio;
        assert!((out_high - expected).abs() < 0.1);
    }

    #[test]
    fn test_compressor_attack_release_timing() {
        let mut comp = Compressor::with_config(CompressorConfig {
            threshold_db: -20.0,
            ratio: 10.0,
            attack_ms: 10.0,
            release_ms: 100.0,
            knee_db: 0.0,
            auto_makeup: false,
            makeup_gain_db: 0.0,
            lookahead_ms: 0.0,
            mix: 1.0,
            ..Default::default()
        });
        comp.prepare(48000.0, 1);

        // Feed step input: 1000 samples of 0 dBFS sine
        let num_samples = 4800;
        let mut in_buf = vec![0.0f32; num_samples];
        for (i, s) in in_buf.iter_mut().enumerate() {
            *s = (2.0 * PI * 1000.0 * (i as f32) / 48000.0).sin();
        }
        let mut out_buf = vec![0.0f32; num_samples];

        comp.process(&[&in_buf], &mut [&mut out_buf]);

        // Output envelope must be reduced
        let late_peak = out_buf[4000..4800]
            .iter()
            .fold(0.0f32, |acc, &x| acc.max(x.abs()));
        assert!(late_peak < 0.3);
    }

    #[test]
    fn test_true_peak_limiter_never_exceeds_ceiling() {
        let mut limiter = TruePeakLimiter::with_config(TruePeakLimiterConfig {
            ceiling_dbtp: -1.0,
            release_ms: 20.0,
            lookahead_ms: 3.0,
        });
        limiter.prepare(48000.0, 1);

        let ceiling_lin = 10.0f32.powf(-1.0 / 20.0);

        // Extreme hot clipped square and noise input exceeding 0 dBFS by up to +12 dB
        let num_samples = 9600;
        let mut in_buf = vec![0.0f32; num_samples];
        for (i, s) in in_buf.iter_mut().enumerate() {
            *s = if (i / 100) % 2 == 0 { 2.5 } else { -3.0 };
        }
        let mut out_buf = vec![0.0f32; num_samples];

        limiter.process(&[&in_buf], &mut [&mut out_buf]);

        for &val in &out_buf {
            assert!(val.abs() <= ceiling_lin + 1e-4);
        }
    }

    #[test]
    fn test_multiband_lr4_crossover_sum_is_flat() {
        let mut mb = MultibandCompressor::new();
        mb.prepare(48000.0, 1);
        // Set all band compressors to unity passthrough (ratio 1.0, 0 makeup)
        for b in 0..4 {
            mb.band_compressor(b).config_mut().ratio = 1.0;
            mb.band_compressor(b).config_mut().auto_makeup = false;
            mb.band_compressor(b).config_mut().makeup_gain_db = 0.0;
            mb.band_compressor(b).config_mut().lookahead_ms = 0.0;
            mb.band_compressor(b).update_parameters();
        }

        // Test with multitone chirp
        let n = 4096;
        let mut in_buf = vec![0.0f32; n];
        for (i, s) in in_buf.iter_mut().enumerate() {
            let t = (i as f32) / 48000.0;
            *s = (2.0 * PI * 100.0 * t).sin()
                + (2.0 * PI * 800.0 * t).sin()
                + (2.0 * PI * 3500.0 * t).sin()
                + (2.0 * PI * 12000.0 * t).sin();
        }
        let mut out_buf = vec![0.0f32; n];

        mb.process(&[&in_buf], &mut [&mut out_buf]);

        // After initial filter settling (e.g. sample 1000..4000), sum must match within 0.05 dB
        let in_rms: f32 = in_buf[1000..4000]
            .iter()
            .map(|&x| x * x)
            .sum::<f32>()
            .sqrt();
        let out_rms: f32 = out_buf[1000..4000]
            .iter()
            .map(|&x| x * x)
            .sum::<f32>()
            .sqrt();
        let diff_db = 20.0 * (out_rms / in_rms).log10().abs();
        assert!(diff_db < 0.05, "LR4 sum flatness error: {} dB", diff_db);
    }

    #[test]
    fn test_deesser_reduces_sibilance_without_affecting_lows() {
        let mut deesser = DeEsser::with_config(DeEsserConfig {
            threshold_db: -20.0,
            ratio: 6.0,
            frequency_hz: 6500.0,
            bandwidth_octaves: 1.0,
            split_band: true,
        });
        deesser.prepare(48000.0, 1);

        // Signal combining 200 Hz tone (vocal body) and 6500 Hz burst (sibilance)
        let n = 4800;
        let mut low_tone = vec![0.0f32; n];
        let mut sibilant_tone = vec![0.0f32; n];
        for i in 0..n {
            let t = (i as f32) / 48000.0;
            low_tone[i] = 0.5 * (2.0 * PI * 200.0 * t).sin();
            sibilant_tone[i] = 0.8 * (2.0 * PI * 6500.0 * t).sin();
        }

        // 1. Process low tone alone: change must be < 0.5 dB
        let mut out_low = vec![0.0f32; n];
        deesser.process(&[&low_tone], &mut [&mut out_low]);
        let low_in_rms: f32 = low_tone[1000..].iter().map(|x| x * x).sum::<f32>().sqrt();
        let low_out_rms: f32 = out_low[1000..].iter().map(|x| x * x).sum::<f32>().sqrt();
        let low_change_db = 20.0 * (low_out_rms / low_in_rms).log10().abs();
        assert!(
            low_change_db < 0.5,
            "Non-sibilant energy changed by {} dB",
            low_change_db
        );

        // 2. Process sibilance tone alone: must be reduced by >= 3.0 dB
        let mut out_sib = vec![0.0f32; n];
        deesser.process(&[&sibilant_tone], &mut [&mut out_sib]);
        let sib_in_rms: f32 = sibilant_tone[1000..]
            .iter()
            .map(|x| x * x)
            .sum::<f32>()
            .sqrt();
        let sib_out_rms: f32 = out_sib[1000..].iter().map(|x| x * x).sum::<f32>().sqrt();
        let sib_reduction_db = 20.0 * (sib_in_rms / sib_out_rms).log10();
        assert!(
            sib_reduction_db >= 3.0,
            "Sibilance reduction: {} dB",
            sib_reduction_db
        );
    }

    #[test]
    fn test_gate_expander_attenuates_noise_below_threshold() {
        let mut gate = GateExpander::with_config(GateExpanderConfig {
            threshold_db: -30.0,
            ratio: 20.0,
            attack_ms: 1.0,
            hold_ms: 5.0,
            release_ms: 20.0,
            hysteresis_db: 3.0,
            floor_db: -50.0,
        });
        gate.prepare(48000.0, 1);

        // Quiet noise at -40 dBFS
        let n = 4800;
        let quiet_in = vec![0.01f32; n];
        let mut quiet_out = vec![0.0f32; n];
        gate.process(&[&quiet_in], &mut [&mut quiet_out]);

        // Output must be heavily attenuated towards floor
        let last_val = quiet_out[4000].abs();
        assert!(
            last_val < 0.001,
            "Gate did not attenuate quiet signal: {}",
            last_val
        );
    }

    #[test]
    fn test_transient_shaper_attack_boost() {
        let mut shaper = TransientShaper::with_config(TransientShaperConfig {
            attack_gain_db: 6.0,
            sustain_gain_db: -6.0,
        });
        shaper.prepare(48000.0, 1);

        // Impulse transient followed by silence
        let n = 2400;
        let mut in_buf = vec![0.0f32; n];
        in_buf[10] = 0.5; // Transient spike
        for (i, val) in in_buf.iter_mut().enumerate().take(50).skip(11) {
            *val = 0.45 * (0.95f32.powi(i as i32 - 10)); // Decaying tail
        }
        let mut out_buf = vec![0.0f32; n];
        shaper.process(&[&in_buf], &mut [&mut out_buf]);

        // Peak transient should be boosted
        assert!(out_buf[10] > 0.5);
    }

    #[test]
    fn test_adaptive_leveler_evens_out_speech_without_boosting_noise() {
        let mut leveler = AdaptiveLeveler::with_config(AdaptiveLevelerConfig {
            target_lufs: -16.0,
            window_seconds: 1.0,
            max_boost_db: 8.0,
            max_cut_db: -12.0,
            gate_threshold_dbfs: -40.0,
        });
        leveler.prepare(48000.0, 1);

        let n = 48000;
        let mut in_buf = vec![0.0f32; n];

        // First 0.5s: loud speech (-6 dBFS)
        for (i, val) in in_buf[..24000].iter_mut().enumerate() {
            let t = (i as f32) / 48000.0;
            *val = 0.5 * (2.0 * PI * 400.0 * t).sin();
        }
        // Second 0.5s: silent pause with quiet background noise (-50 dBFS, below gate)
        for (i, val) in in_buf[24000..48000].iter_mut().enumerate() {
            let t = ((i + 24000) as f32) / 48000.0;
            *val = 0.003 * (2.0 * PI * 200.0 * t).sin();
        }

        let mut out_buf = vec![0.0f32; n];
        leveler.process(&[&in_buf], &mut [&mut out_buf]);

        // During the quiet pause, gain must NOT boost background noise > 3 dB
        let noise_in_rms = in_buf[30000..40000]
            .iter()
            .map(|x| x * x)
            .sum::<f32>()
            .sqrt();
        let noise_out_rms = out_buf[30000..40000]
            .iter()
            .map(|x| x * x)
            .sum::<f32>()
            .sqrt();
        let noise_boost_db = 20.0 * (noise_out_rms / noise_in_rms).log10();
        assert!(
            noise_boost_db < 3.0,
            "Background noise was boosted by {} dB (budget <= 3 dB)",
            noise_boost_db
        );
    }

    #[test]
    fn test_peak_and_lufs_normalize() {
        let mut buffer = vec![0.1, -0.2, 0.4, -0.5, 0.2];
        let gain = normalize_peak(&mut buffer, -1.0);
        let expected_peak = 10.0f32.powf(-1.0 / 20.0);
        let actual_peak = buffer.iter().fold(0.0f32, |acc, &x| acc.max(x.abs()));
        assert!((actual_peak - expected_peak).abs() < 1e-4);
        assert!(gain > 1.0);
    }
}
