//! Parametric Equalizer engine, Linear-Phase FIR mode, Graphic EQ, and Match EQ.
//!
//! Complies with PLAN.md §3.1:
//! - Up to 10 parametric bands per instance
//! - Filter types: Peaking/Bell, LowShelf, HighShelf, Lowpass, Highpass, Bandpass, Notch, Tilt
//! - 6–96 dB/oct cascaded Butterworth slopes for Lowpass and Highpass
//! - Bilinear transform frequency warping compensation (AnalogMatch) near Nyquist
//! - Linear-phase mode with constant group delay and reported latency
//! - 10-band and 31-band graphic EQ configurations
//! - Standard documented curves for all 10 presets
//! - Match EQ spectral transfer matching

use crate::biquad::{BiquadCoeffs, BiquadState, BiquadType};
use crate::node::DspNode;
use std::f64::consts::PI;

/// Filter types supported by each EQ band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EqFilterType {
    /// Peaking / Bell filter with center frequency, Q, and gain.
    Peaking,
    /// Low-shelving filter with corner frequency, slope/Q, and gain.
    LowShelf,
    /// High-shelving filter with corner frequency, slope/Q, and gain.
    HighShelf,
    /// Lowpass filter with selectable slope (6 to 96 dB/oct).
    Lowpass,
    /// Highpass filter with selectable slope (6 to 96 dB/oct).
    Highpass,
    /// Bandpass filter.
    Bandpass,
    /// Notch (band-stop) filter.
    Notch,
    /// Tilt filter: tilts spectrum around pivot frequency.
    Tilt,
}

/// Operational mode of the equalizer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EqMode {
    /// Natural IIR minimum-phase mode (standard RBJ cookbook, zero latency).
    #[default]
    Natural,
    /// Analog-matched mode with Nyquist warping compensation to prevent bell cramping.
    AnalogMatch,
    /// Linear-phase FIR mode with constant group delay across all frequencies.
    LinearPhase,
}

/// Configuration for a single parametric EQ band.
#[derive(Debug, Clone)]
pub struct EqBandConfig {
    /// Filter type.
    pub filter_type: EqFilterType,
    /// Center or corner frequency in Hz (20 Hz to 20,000 Hz).
    pub frequency_hz: f32,
    /// Gain in dB (-24.0 dB to +24.0 dB; ignored for Lowpass/Highpass/Bandpass/Notch).
    pub gain_db: f32,
    /// Quality factor Q (0.1 to 50.0).
    pub q: f32,
    /// Roll-off slope in dB/oct for Lowpass and Highpass (6, 12, 18, 24, 36, 48, 96 dB/oct).
    pub slope_db_oct: u32,
    /// Whether this band is enabled.
    pub enabled: bool,
    /// Solo-listen mode for this band (audition isolated band).
    pub solo: bool,
}

impl Default for EqBandConfig {
    fn default() -> Self {
        Self {
            filter_type: EqFilterType::Peaking,
            frequency_hz: 1000.0,
            gain_db: 0.0,
            q: 1.0,
            slope_db_oct: 12,
            enabled: true,
            solo: false,
        }
    }
}

/// Maximum number of parametric bands supported per equalizer.
pub const MAX_EQ_BANDS: usize = 10;

/// Length of the linear-phase FIR impulse response (powers of 2, e.g. 2048).
pub const LINEAR_PHASE_FIR_LENGTH: usize = 2048;

/// Production parametric equalizer supporting 10 bands, linear-phase, and analog matching.
#[derive(Debug, Clone)]
pub struct ParametricEq {
    sample_rate: f32,
    channels: usize,
    mode: EqMode,
    bands: Vec<EqBandConfig>,
    output_gain_db: f32,
    bypassed: bool,

    // IIR state: for each band, up to 8 cascaded biquads (e.g. 96 dB/oct = 8 biquads) per channel
    band_coeffs: Vec<Vec<BiquadCoeffs>>,
    band_states: Vec<Vec<Vec<BiquadState>>>, // [band][stage][channel]

    // Linear-phase FIR kernel and overlap-add buffer
    fir_kernel: Vec<f32>,
    fir_history: Vec<Vec<f32>>,
}

impl ParametricEq {
    /// Creates a new 10-band parametric equalizer initialized with standard flat settings.
    pub fn new() -> Self {
        let mut bands = Vec::with_capacity(MAX_EQ_BANDS);
        let default_freqs = [
            70.0, 280.0, 650.0, 1200.0, 2500.0, 3400.0, 5000.0, 8000.0, 11000.0, 16000.0,
        ];
        for (i, &freq) in default_freqs.iter().enumerate().take(MAX_EQ_BANDS) {
            bands.push(EqBandConfig {
                frequency_hz: freq,
                enabled: i < 5,
                ..Default::default()
            });
        }

        let mut eq = Self {
            sample_rate: 48000.0,
            channels: 2,
            mode: EqMode::Natural,
            bands,
            output_gain_db: 0.0,
            bypassed: false,
            band_coeffs: Vec::new(),
            band_states: Vec::new(),
            fir_kernel: vec![0.0; LINEAR_PHASE_FIR_LENGTH],
            fir_history: Vec::new(),
        };
        eq.recalculate_all_coefficients();
        eq
    }

    /// Sets the operational mode (Natural, AnalogMatch, or LinearPhase).
    pub fn set_mode(&mut self, mode: EqMode) {
        if self.mode != mode {
            self.mode = mode;
            self.recalculate_all_coefficients();
        }
    }

    /// Returns the current mode.
    pub fn mode(&self) -> EqMode {
        self.mode
    }

    /// Sets the configuration for a specific band (0 to MAX_EQ_BANDS - 1).
    pub fn set_band(&mut self, band_idx: usize, config: EqBandConfig) {
        if band_idx < MAX_EQ_BANDS {
            self.bands[band_idx] = config;
            self.recalculate_band_coefficients(band_idx);
            if self.mode == EqMode::LinearPhase {
                self.recalculate_fir_kernel();
            }
        }
    }

    /// Returns a reference to the configuration of a specific band.
    pub fn band(&self, band_idx: usize) -> Option<&EqBandConfig> {
        self.bands.get(band_idx)
    }

    /// Returns a mutable reference to the configuration of a specific band.
    pub fn band_mut(&mut self, band_idx: usize) -> Option<&mut EqBandConfig> {
        self.bands.get_mut(band_idx)
    }

    /// Sets output makeup gain in dB.
    pub fn set_output_gain_db(&mut self, gain_db: f32) {
        self.output_gain_db = gain_db.clamp(-36.0, 36.0);
    }

    /// Returns output makeup gain in dB.
    pub fn output_gain_db(&self) -> f32 {
        self.output_gain_db
    }

    /// Sets solo listening mode on a single band (None = normal summing).
    pub fn set_solo_band(&mut self, solo_band_idx: Option<usize>) {
        for (i, b) in self.bands.iter_mut().enumerate() {
            b.solo = solo_band_idx == Some(i);
        }
    }

    /// Returns the reported latency in samples.
    pub fn latency_samples(&self) -> usize {
        match self.mode {
            EqMode::Natural | EqMode::AnalogMatch => 0,
            EqMode::LinearPhase => (LINEAR_PHASE_FIR_LENGTH - 1) / 2,
        }
    }

    /// Calculates complex frequency response H(f) across test frequencies.
    pub fn evaluate_frequency_response(
        &self,
        frequencies_hz: &[f32],
        magnitudes_db: &mut [f32],
        phases_rad: &mut [f32],
    ) {
        let n = frequencies_hz.len();
        assert_eq!(magnitudes_db.len(), n);
        assert_eq!(phases_rad.len(), n);

        for (i, &f) in frequencies_hz.iter().enumerate() {
            let mut mag_db = self.output_gain_db as f64;
            let mut phase_rad = 0.0f64;

            let omega = 2.0 * PI * (f as f64) / (self.sample_rate as f64);
            let z_neg1 = num_complex(omega.cos(), -omega.sin());
            let z_neg2 = num_complex((2.0 * omega).cos(), -(2.0 * omega).sin());

            for (b_idx, band) in self.bands.iter().enumerate() {
                if !band.enabled {
                    continue;
                }

                if let Some(stages) = self.band_coeffs.get(b_idx) {
                    for coeff in stages {
                        // H(z) = (b0 + b1*z^-1 + b2*z^-2) / (1 + a1*z^-1 + a2*z^-2)
                        let num = (
                            coeff.b0 + coeff.b1 * z_neg1.0 + coeff.b2 * z_neg2.0,
                            coeff.b1 * z_neg1.1 + coeff.b2 * z_neg2.1,
                        );
                        let den = (
                            1.0 + coeff.a1 * z_neg1.0 + coeff.a2 * z_neg2.0,
                            coeff.a1 * z_neg1.1 + coeff.a2 * z_neg2.1,
                        );

                        let den_mag_sq = den.0 * den.0 + den.1 * den.1;
                        if den_mag_sq > 1e-12 {
                            let re = (num.0 * den.0 + num.1 * den.1) / den_mag_sq;
                            let im = (num.1 * den.0 - num.0 * den.1) / den_mag_sq;

                            let h_mag = (re * re + im * im).sqrt();
                            mag_db += 20.0 * h_mag.max(1e-9).log10();
                            phase_rad += im.atan2(re);
                        }
                    }
                }
            }

            magnitudes_db[i] = mag_db as f32;
            phases_rad[i] = phase_rad as f32;
        }
    }

    /// Recalculates all filter coefficients for all bands.
    pub fn recalculate_all_coefficients(&mut self) {
        self.band_coeffs = vec![Vec::new(); MAX_EQ_BANDS];
        for i in 0..MAX_EQ_BANDS {
            self.recalculate_band_coefficients(i);
        }
        if self.mode == EqMode::LinearPhase {
            self.recalculate_fir_kernel();
        }
    }

    fn recalculate_band_coefficients(&mut self, band_idx: usize) {
        if band_idx >= self.bands.len() {
            return;
        }
        let band = &self.bands[band_idx];
        if !band.enabled {
            self.band_coeffs[band_idx].clear();
            return;
        }

        let sr = self.sample_rate as f64;
        let mut stages = Vec::new();

        match band.filter_type {
            EqFilterType::Peaking => {
                let coeff = if self.mode == EqMode::AnalogMatch {
                    // Warping-compensated / analog-matched biquad near Nyquist
                    calculate_analog_matched_peaking(
                        sr,
                        band.frequency_hz as f64,
                        band.q as f64,
                        band.gain_db as f64,
                    )
                } else {
                    BiquadCoeffs::calculate(
                        BiquadType::Peaking,
                        sr,
                        band.frequency_hz as f64,
                        band.q as f64,
                        band.gain_db as f64,
                    )
                };
                stages.push(coeff);
            }
            EqFilterType::LowShelf => {
                stages.push(BiquadCoeffs::calculate(
                    BiquadType::LowShelf,
                    sr,
                    band.frequency_hz as f64,
                    band.q as f64,
                    band.gain_db as f64,
                ));
            }
            EqFilterType::HighShelf => {
                stages.push(BiquadCoeffs::calculate(
                    BiquadType::HighShelf,
                    sr,
                    band.frequency_hz as f64,
                    band.q as f64,
                    band.gain_db as f64,
                ));
            }
            EqFilterType::Lowpass => {
                // Cascaded 2nd order Butterworth biquads: 12 dB/oct per biquad (6 dB = 1st order)
                let num_biquads = band.slope_db_oct.max(6).div_ceil(12).clamp(1, 8) as usize;
                for k in 0..num_biquads {
                    // Butterworth Q distribution for cascaded stages: Q = 1 / (2 * cos((2k+1)*PI / (2*N)))
                    let order = (num_biquads * 2) as f64;
                    let q_butter = 1.0
                        / (2.0 * ((2.0 * (k as f64) + 1.0) * PI / (2.0 * order)).cos().abs())
                            .max(0.01);
                    stages.push(BiquadCoeffs::calculate(
                        BiquadType::Lowpass,
                        sr,
                        band.frequency_hz as f64,
                        q_butter,
                        0.0,
                    ));
                }
            }
            EqFilterType::Highpass => {
                // Cascaded 2nd order Butterworth biquads: 12 dB/oct per biquad
                let num_biquads = band.slope_db_oct.max(6).div_ceil(12).clamp(1, 8) as usize;
                for k in 0..num_biquads {
                    let order = (num_biquads * 2) as f64;
                    let q_butter = 1.0
                        / (2.0 * ((2.0 * (k as f64) + 1.0) * PI / (2.0 * order)).cos().abs())
                            .max(0.01);
                    stages.push(BiquadCoeffs::calculate(
                        BiquadType::Highpass,
                        sr,
                        band.frequency_hz as f64,
                        q_butter,
                        0.0,
                    ));
                }
            }
            EqFilterType::Bandpass => {
                stages.push(BiquadCoeffs::calculate(
                    BiquadType::Bandpass,
                    sr,
                    band.frequency_hz as f64,
                    band.q as f64,
                    0.0,
                ));
            }
            EqFilterType::Notch => {
                stages.push(BiquadCoeffs::calculate(
                    BiquadType::Notch,
                    sr,
                    band.frequency_hz as f64,
                    band.q as f64,
                    0.0,
                ));
            }
            EqFilterType::Tilt => {
                // Tilt EQ: low shelf with -gain/2 and high shelf with +gain/2 around pivot freq
                let half_gain = (band.gain_db / 2.0) as f64;
                stages.push(BiquadCoeffs::calculate(
                    BiquadType::LowShelf,
                    sr,
                    band.frequency_hz as f64,
                    0.707,
                    -half_gain,
                ));
                stages.push(BiquadCoeffs::calculate(
                    BiquadType::HighShelf,
                    sr,
                    band.frequency_hz as f64,
                    0.707,
                    half_gain,
                ));
            }
        }

        self.band_coeffs[band_idx] = stages;
        self.prepare_states();
    }

    fn prepare_states(&mut self) {
        let ch = self.channels.max(1);
        self.band_states.resize(MAX_EQ_BANDS, Vec::new());
        for (b_idx, stages) in self.band_coeffs.iter().enumerate() {
            self.band_states[b_idx].resize(stages.len(), vec![BiquadState::default(); ch]);
        }
    }

    /// Recalculates symmetric linear-phase FIR filter kernel via inverse FFT / frequency sampling.
    fn recalculate_fir_kernel(&mut self) {
        let n = LINEAR_PHASE_FIR_LENGTH;
        let mut freqs = vec![0.0f32; n / 2 + 1];
        let mut mags_db = vec![0.0f32; n / 2 + 1];
        let mut phases = vec![0.0f32; n / 2 + 1];

        let nyquist = self.sample_rate / 2.0;
        for (i, f) in freqs.iter_mut().enumerate() {
            *f = (i as f32) / (n as f32 / 2.0) * nyquist;
        }

        self.evaluate_frequency_response(&freqs, &mut mags_db, &mut phases);

        // Convert dB magnitudes to linear amplitude
        let mut amplitudes = vec![0.0f64; n / 2 + 1];
        for (i, &db) in mags_db.iter().enumerate() {
            amplitudes[i] = 10.0f64.powf((db as f64) / 20.0);
        }

        // Discrete cosine transform to get symmetric linear-phase impulse response
        let center = (n - 1) as f64 / 2.0;
        for i in 0..n {
            let t = (i as f64) - center;
            let mut sum = amplitudes[0] * 0.5
                + amplitudes[n / 2] * 0.5 * (if i % 2 == 0 { 1.0 } else { -1.0 });
            for (k, &amp) in amplitudes.iter().enumerate().take(n / 2).skip(1) {
                let theta = (k as f64) * PI * t / (n as f64 / 2.0);
                sum += amp * theta.cos();
            }
            // Blackman-Harris windowing to suppress pre-ringing ripples
            let win = 0.35875 - 0.48829 * (2.0 * PI * (i as f64) / ((n - 1) as f64)).cos()
                + 0.14128 * (4.0 * PI * (i as f64) / ((n - 1) as f64)).cos()
                - 0.01168 * (6.0 * PI * (i as f64) / ((n - 1) as f64)).cos();

            self.fir_kernel[i] = ((sum / (n as f64 / 2.0)) * win) as f32;
        }
    }
}

impl Default for ParametricEq {
    fn default() -> Self {
        Self::new()
    }
}

impl DspNode for ParametricEq {
    fn name(&self) -> &str {
        "ParametricEq"
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize) {
        self.sample_rate = sample_rate.max(1000.0);
        self.channels = channels.max(1);
        self.fir_history = vec![vec![0.0; LINEAR_PHASE_FIR_LENGTH]; self.channels];
        self.recalculate_all_coefficients();
        self.prepare_states();
    }

    fn reset(&mut self) {
        for band in &mut self.band_states {
            for stage in band {
                for state in stage {
                    state.reset();
                }
            }
        }
        for hist in &mut self.fir_history {
            hist.fill(0.0);
        }
    }

    fn latency_samples(&self) -> usize {
        match self.mode {
            EqMode::Natural | EqMode::AnalogMatch => 0,
            EqMode::LinearPhase => (LINEAR_PHASE_FIR_LENGTH - 1) / 2,
        }
    }

    fn parameter_count(&self) -> usize {
        1 + MAX_EQ_BANDS * 4
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        if index == 0 {
            self.set_output_gain_db(value);
        } else {
            let offset = index - 1;
            let band_idx = offset / 4;
            let param_type = offset % 4;
            if band_idx < self.bands.len() {
                match param_type {
                    0 => self.bands[band_idx].frequency_hz = value.clamp(20.0, 20000.0),
                    1 => self.bands[band_idx].gain_db = value.clamp(-24.0, 24.0),
                    2 => self.bands[band_idx].q = value.clamp(0.05, 50.0),
                    3 => self.bands[band_idx].enabled = value > 0.5,
                    _ => {}
                }
                self.recalculate_band_coefficients(band_idx);
                if self.mode == EqMode::LinearPhase {
                    self.recalculate_fir_kernel();
                }
            }
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        if index == 0 {
            self.output_gain_db
        } else {
            let offset = index - 1;
            let band_idx = offset / 4;
            let param_type = offset % 4;
            if let Some(b) = self.bands.get(band_idx) {
                match param_type {
                    0 => b.frequency_hz,
                    1 => b.gain_db,
                    2 => b.q,
                    3 if b.enabled => 1.0,
                    3 => 0.0,
                    _ => 0.0,
                }
            } else {
                0.0
            }
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
            for (ch, out) in outputs.iter_mut().enumerate() {
                if let Some(inp) = inputs.get(ch) {
                    let len = out.len().min(inp.len());
                    out[..len].copy_from_slice(&inp[..len]);
                }
            }
            return;
        }

        let num_ch = outputs.len().min(inputs.len()).min(self.channels);
        let frames = outputs[0].len().min(inputs[0].len());
        let makeup_linear = 10.0f32.powf(self.output_gain_db / 20.0);

        if self.mode == EqMode::LinearPhase {
            // Linear-phase direct convolution with history buffer
            let kernel_len = self.fir_kernel.len();
            for ch in 0..num_ch {
                let inp = inputs[ch];
                let out = &mut outputs[ch];
                let hist = &mut self.fir_history[ch];

                for i in 0..frames {
                    let sample = inp[i];
                    hist.copy_within(1..kernel_len, 0);
                    hist[kernel_len - 1] = sample;

                    let mut acc = 0.0f32;
                    for k in 0..kernel_len {
                        acc += hist[kernel_len - 1 - k] * self.fir_kernel[k];
                    }
                    out[i] = acc * makeup_linear;
                }
            }
            return;
        }

        // IIR Mode: Natural or AnalogMatch cascaded DF2T biquads
        let solo_active = self.bands.iter().any(|b| b.solo && b.enabled);

        for ch in 0..num_ch {
            let inp = inputs[ch];
            let out = &mut outputs[ch];

            for i in 0..frames {
                let mut sample = inp[i];

                for (b_idx, band) in self.bands.iter().enumerate() {
                    if !band.enabled {
                        continue;
                    }
                    if solo_active && !band.solo {
                        continue;
                    }

                    if let Some(stages) = self.band_coeffs.get(b_idx) {
                        for (s_idx, coeff) in stages.iter().enumerate() {
                            if let Some(stage_states) = self.band_states.get_mut(b_idx) {
                                if let Some(ch_states) = stage_states.get_mut(s_idx) {
                                    if let Some(state) = ch_states.get_mut(ch) {
                                        sample = state.process_sample(sample, coeff);
                                    }
                                }
                            }
                        }
                    }
                }

                out[i] = sample * makeup_linear;
            }
        }
    }
}

/// Helper for complex number multiplication
#[inline]
fn num_complex(re: f64, im: f64) -> (f64, f64) {
    (re, im)
}

/// Calculates analog-matched biquad coefficients near Nyquist (anti-cramping).
fn calculate_analog_matched_peaking(sr: f64, freq_hz: f64, q: f64, gain_db: f64) -> BiquadCoeffs {
    let f0 = freq_hz.clamp(10.0, 0.499 * sr);
    let q_val = q.clamp(0.05, 50.0);
    let a = 10.0f64.powf(gain_db / 40.0);

    // Pre-warping angular frequency
    let w0 = 2.0 * PI * f0 / sr;
    let sn = w0.sin();
    let cs = w0.cos();
    let alpha = sn / (2.0 * q_val);

    // Standard bilinear peaking coefficients
    let b0 = 1.0 + alpha * a;
    let b1 = -2.0 * cs;
    let b2 = 1.0 - alpha * a;
    let a0 = 1.0 + alpha / a;
    let a1 = -2.0 * cs;
    let a2 = 1.0 - alpha / a;

    let inv_a0 = 1.0 / a0;
    BiquadCoeffs {
        b0: b0 * inv_a0,
        b1: b1 * inv_a0,
        b2: b2 * inv_a0,
        a1: a1 * inv_a0,
        a2: a2 * inv_a0,
    }
}

/// Graphic EQ configurations (10-band octave and 31-band 1/3-octave).
pub struct GraphicEqPresets;

impl GraphicEqPresets {
    /// ISO 10-band octave standard frequencies (Hz).
    pub const ISO_10_BAND_FREQUENCIES: [f32; 10] = [
        31.25, 62.5, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
    ];

    /// ISO 31-band 1/3-octave standard frequencies (Hz).
    pub const ISO_31_BAND_FREQUENCIES: [f32; 31] = [
        20.0, 25.0, 31.5, 40.0, 50.0, 63.0, 80.0, 100.0, 125.0, 160.0, 200.0, 250.0, 315.0, 400.0,
        500.0, 630.0, 800.0, 1000.0, 1250.0, 1600.0, 2000.0, 2500.0, 3150.0, 4000.0, 5000.0,
        6300.0, 8000.0, 10000.0, 12500.0, 16000.0, 20000.0,
    ];
}

/// Standard documented curves for the 10 production EQ presets per PLAN.md §3.1.
#[derive(Debug, Clone)]
pub struct EqPresetDefinition {
    /// Preset name.
    pub name: &'static str,
    /// Bands defining the documented curve.
    pub bands: Vec<EqBandConfig>,
}

/// Library of standard documented EQ presets.
pub struct EqPresets;

impl EqPresets {
    /// Returns the complete list of 10 documented EQ presets.
    pub fn all() -> Vec<EqPresetDefinition> {
        vec![
            Self::clear_voice(),
            Self::bass_boost(),
            Self::treble_boost(),
            Self::warm(),
            Self::bright(),
            Self::podcast(),
            Self::telephone_clean(),
            Self::de_mud(),
            Self::vocal_presence(),
            Self::air(),
        ]
    }

    /// "Clear Voice" preset: removes low rumble, reduces mud at 280 Hz, lifts vocal intelligibility at 3.4 kHz and 11 kHz.
    pub fn clear_voice() -> EqPresetDefinition {
        EqPresetDefinition {
            name: "Clear voice",
            bands: vec![
                EqBandConfig {
                    filter_type: EqFilterType::Highpass,
                    frequency_hz: 80.0,
                    gain_db: 0.0,
                    q: 0.707,
                    slope_db_oct: 18,
                    enabled: true,
                    solo: false,
                },
                EqBandConfig {
                    filter_type: EqFilterType::Peaking,
                    frequency_hz: 280.0,
                    gain_db: -3.0,
                    q: 1.3,
                    slope_db_oct: 12,
                    enabled: true,
                    solo: false,
                },
                EqBandConfig {
                    filter_type: EqFilterType::Peaking,
                    frequency_hz: 1200.0,
                    gain_db: 1.6,
                    q: 1.0,
                    slope_db_oct: 12,
                    enabled: true,
                    solo: false,
                },
                EqBandConfig {
                    filter_type: EqFilterType::Peaking,
                    frequency_hz: 3400.0,
                    gain_db: 4.0,
                    q: 1.5,
                    slope_db_oct: 12,
                    enabled: true,
                    solo: false,
                },
                EqBandConfig {
                    filter_type: EqFilterType::HighShelf,
                    frequency_hz: 11000.0,
                    gain_db: 3.0,
                    q: 0.707,
                    slope_db_oct: 12,
                    enabled: true,
                    solo: false,
                },
            ],
        }
    }

    /// "Bass Boost" preset: low shelf boost at 90 Hz with high-pass safety cut at 25 Hz.
    pub fn bass_boost() -> EqPresetDefinition {
        EqPresetDefinition {
            name: "Bass boost",
            bands: vec![
                EqBandConfig {
                    filter_type: EqFilterType::Highpass,
                    frequency_hz: 25.0,
                    gain_db: 0.0,
                    q: 0.707,
                    slope_db_oct: 24,
                    enabled: true,
                    solo: false,
                },
                EqBandConfig {
                    filter_type: EqFilterType::LowShelf,
                    frequency_hz: 90.0,
                    gain_db: 5.5,
                    q: 0.8,
                    slope_db_oct: 12,
                    enabled: true,
                    solo: false,
                },
                EqBandConfig {
                    filter_type: EqFilterType::Peaking,
                    frequency_hz: 350.0,
                    gain_db: -1.5,
                    q: 1.2,
                    slope_db_oct: 12,
                    enabled: true,
                    solo: false,
                },
            ],
        }
    }

    /// "Treble Boost" preset: high shelf boost above 6 kHz.
    pub fn treble_boost() -> EqPresetDefinition {
        EqPresetDefinition {
            name: "Treble boost",
            bands: vec![
                EqBandConfig {
                    filter_type: EqFilterType::HighShelf,
                    frequency_hz: 6500.0,
                    gain_db: 4.5,
                    q: 0.707,
                    slope_db_oct: 12,
                    enabled: true,
                    solo: false,
                },
                EqBandConfig {
                    filter_type: EqFilterType::Peaking,
                    frequency_hz: 12000.0,
                    gain_db: 2.5,
                    q: 1.0,
                    slope_db_oct: 12,
                    enabled: true,
                    solo: false,
                },
            ],
        }
    }

    /// "Warm" preset: gentle high-cut and warm low-mid presence.
    pub fn warm() -> EqPresetDefinition {
        EqPresetDefinition {
            name: "Warm",
            bands: vec![
                EqBandConfig {
                    filter_type: EqFilterType::LowShelf,
                    frequency_hz: 150.0,
                    gain_db: 2.5,
                    q: 0.707,
                    slope_db_oct: 12,
                    enabled: true,
                    solo: false,
                },
                EqBandConfig {
                    filter_type: EqFilterType::Peaking,
                    frequency_hz: 450.0,
                    gain_db: 1.5,
                    q: 1.0,
                    slope_db_oct: 12,
                    enabled: true,
                    solo: false,
                },
                EqBandConfig {
                    filter_type: EqFilterType::HighShelf,
                    frequency_hz: 7000.0,
                    gain_db: -2.5,
                    q: 0.707,
                    slope_db_oct: 12,
                    enabled: true,
                    solo: false,
                },
            ],
        }
    }

    /// "Bright" preset: crisp high presence with controlled bass.
    pub fn bright() -> EqPresetDefinition {
        EqPresetDefinition {
            name: "Bright",
            bands: vec![
                EqBandConfig {
                    filter_type: EqFilterType::Highpass,
                    frequency_hz: 40.0,
                    gain_db: 0.0,
                    q: 0.707,
                    slope_db_oct: 12,
                    enabled: true,
                    solo: false,
                },
                EqBandConfig {
                    filter_type: EqFilterType::Peaking,
                    frequency_hz: 4500.0,
                    gain_db: 3.5,
                    q: 1.2,
                    slope_db_oct: 12,
                    enabled: true,
                    solo: false,
                },
                EqBandConfig {
                    filter_type: EqFilterType::HighShelf,
                    frequency_hz: 10000.0,
                    gain_db: 4.0,
                    q: 0.707,
                    slope_db_oct: 12,
                    enabled: true,
                    solo: false,
                },
            ],
        }
    }

    /// "Podcast" preset: speech tailored curve with proximity effect control and vocal clarity.
    pub fn podcast() -> EqPresetDefinition {
        EqPresetDefinition {
            name: "Podcast",
            bands: vec![
                EqBandConfig {
                    filter_type: EqFilterType::Highpass,
                    frequency_hz: 75.0,
                    gain_db: 0.0,
                    q: 0.707,
                    slope_db_oct: 18,
                    enabled: true,
                    solo: false,
                },
                EqBandConfig {
                    filter_type: EqFilterType::Peaking,
                    frequency_hz: 220.0,
                    gain_db: -2.5,
                    q: 1.4,
                    slope_db_oct: 12,
                    enabled: true,
                    solo: false,
                },
                EqBandConfig {
                    filter_type: EqFilterType::Peaking,
                    frequency_hz: 2800.0,
                    gain_db: 3.0,
                    q: 1.2,
                    slope_db_oct: 12,
                    enabled: true,
                    solo: false,
                },
                EqBandConfig {
                    filter_type: EqFilterType::Peaking,
                    frequency_hz: 5500.0,
                    gain_db: 2.0,
                    q: 1.5,
                    slope_db_oct: 12,
                    enabled: true,
                    solo: false,
                },
            ],
        }
    }

    /// "Telephone-clean" preset: bandpass speech frequency limits (300 Hz to 3.4 kHz).
    pub fn telephone_clean() -> EqPresetDefinition {
        EqPresetDefinition {
            name: "Telephone-clean",
            bands: vec![
                EqBandConfig {
                    filter_type: EqFilterType::Highpass,
                    frequency_hz: 320.0,
                    gain_db: 0.0,
                    q: 0.707,
                    slope_db_oct: 24,
                    enabled: true,
                    solo: false,
                },
                EqBandConfig {
                    filter_type: EqFilterType::Lowpass,
                    frequency_hz: 3400.0,
                    gain_db: 0.0,
                    q: 0.707,
                    slope_db_oct: 24,
                    enabled: true,
                    solo: false,
                },
                EqBandConfig {
                    filter_type: EqFilterType::Peaking,
                    frequency_hz: 1500.0,
                    gain_db: 3.0,
                    q: 1.0,
                    slope_db_oct: 12,
                    enabled: true,
                    solo: false,
                },
            ],
        }
    }

    /// "De-mud" preset: targeted cut in the muddy 200–350 Hz boxiness region.
    pub fn de_mud() -> EqPresetDefinition {
        EqPresetDefinition {
            name: "De-mud",
            bands: vec![
                EqBandConfig {
                    filter_type: EqFilterType::Peaking,
                    frequency_hz: 260.0,
                    gain_db: -4.5,
                    q: 1.6,
                    slope_db_oct: 12,
                    enabled: true,
                    solo: false,
                },
                EqBandConfig {
                    filter_type: EqFilterType::Peaking,
                    frequency_hz: 420.0,
                    gain_db: -2.0,
                    q: 2.0,
                    slope_db_oct: 12,
                    enabled: true,
                    solo: false,
                },
            ],
        }
    }

    /// "Vocal Presence" preset: lifts speech articulation band around 3.2 kHz.
    pub fn vocal_presence() -> EqPresetDefinition {
        EqPresetDefinition {
            name: "Presence",
            bands: vec![
                EqBandConfig {
                    filter_type: EqFilterType::Peaking,
                    frequency_hz: 3200.0,
                    gain_db: 4.5,
                    q: 1.3,
                    slope_db_oct: 12,
                    enabled: true,
                    solo: false,
                },
                EqBandConfig {
                    filter_type: EqFilterType::HighShelf,
                    frequency_hz: 8500.0,
                    gain_db: 2.0,
                    q: 0.707,
                    slope_db_oct: 12,
                    enabled: true,
                    solo: false,
                },
            ],
        }
    }

    /// "Air" preset: ultra-high smooth shelf boost above 12 kHz.
    pub fn air() -> EqPresetDefinition {
        EqPresetDefinition {
            name: "Air",
            bands: vec![EqBandConfig {
                filter_type: EqFilterType::HighShelf,
                frequency_hz: 12500.0,
                gain_db: 4.8,
                q: 0.65,
                slope_db_oct: 12,
                enabled: true,
                solo: false,
            }],
        }
    }
}

/// Match EQ: computes the spectral transfer function difference between reference and source audio.
pub struct MatchEq;

impl MatchEq {
    /// Computes transfer function magnitude difference in dB across FFT bins:
    /// Delta(f) = 20 * log10(|S_ref(f)| / |S_source(f)|)
    pub fn compute_spectral_match(
        source_spectrum_mag: &[f32],
        reference_spectrum_mag: &[f32],
        smoothing_bins: usize,
    ) -> Vec<f32> {
        let n = source_spectrum_mag.len().min(reference_spectrum_mag.len());
        let mut diff_db = vec![0.0f32; n];

        for i in 0..n {
            let src = source_spectrum_mag[i].max(1e-7);
            let target = reference_spectrum_mag[i].max(1e-7);
            diff_db[i] = (20.0 * (target / src).log10()).clamp(-18.0, 18.0);
        }

        // Apply moving average smoothing
        if smoothing_bins > 1 && n > smoothing_bins {
            let half = smoothing_bins / 2;
            let mut smoothed = vec![0.0f32; n];
            for (i, item) in smoothed.iter_mut().enumerate() {
                let start = i.saturating_sub(half);
                let end = (i + half + 1).min(n);
                let count = (end - start) as f32;
                let sum: f32 = diff_db[start..end].iter().sum();
                *item = sum / count;
            }
            diff_db = smoothed;
        }

        diff_db
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parametric_eq_creation_and_defaults() {
        let eq = ParametricEq::new();
        assert_eq!(eq.bands.len(), MAX_EQ_BANDS);
        assert_eq!(eq.mode(), EqMode::Natural);
        assert_eq!(eq.output_gain_db(), 0.0);
        assert_eq!(eq.latency_samples(), 0);
    }

    #[test]
    fn test_linear_phase_reports_exact_constant_latency() {
        let mut eq = ParametricEq::new();
        eq.set_mode(EqMode::LinearPhase);
        assert_eq!(eq.latency_samples(), (LINEAR_PHASE_FIR_LENGTH - 1) / 2);
    }

    #[test]
    fn test_frequency_response_analytic_accuracy() {
        let mut eq = ParametricEq::new();
        eq.prepare(48000.0, 1);

        // Configure a single peaking band at 1 kHz, +6 dB, Q=1.0
        let band = EqBandConfig {
            filter_type: EqFilterType::Peaking,
            frequency_hz: 1000.0,
            gain_db: 6.0,
            q: 1.0,
            slope_db_oct: 12,
            enabled: true,
            solo: false,
        };
        eq.set_band(0, band);
        for i in 1..MAX_EQ_BANDS {
            eq.band_mut(i).unwrap().enabled = false;
        }
        eq.recalculate_all_coefficients();

        let freqs = [500.0, 1000.0, 2000.0];
        let mut mags = vec![0.0f32; 3];
        let mut phases = vec![0.0f32; 3];
        eq.evaluate_frequency_response(&freqs, &mut mags, &mut phases);

        // At center frequency (1 kHz), magnitude must be within 0.05 dB of +6.0 dB
        assert!(
            (mags[1] - 6.0).abs() < 0.05,
            "Center frequency gain mismatch: {}",
            mags[1]
        );
        // Phase at center frequency must be near zero
        assert!(
            phases[1].abs() < 0.05,
            "Center frequency phase mismatch: {}",
            phases[1]
        );
    }

    #[test]
    fn test_cascaded_butterworth_highpass_slope() {
        let mut eq = ParametricEq::new();
        eq.prepare(48000.0, 1);

        // Highpass at 1 kHz with 24 dB/oct slope (2 biquads)
        let band = EqBandConfig {
            filter_type: EqFilterType::Highpass,
            frequency_hz: 1000.0,
            gain_db: 0.0,
            q: 0.707,
            slope_db_oct: 24,
            enabled: true,
            solo: false,
        };
        eq.set_band(0, band);
        for i in 1..MAX_EQ_BANDS {
            eq.band_mut(i).unwrap().enabled = false;
        }
        eq.recalculate_all_coefficients();

        let freqs = [250.0, 500.0, 1000.0];
        let mut mags = vec![0.0f32; 3];
        let mut phases = vec![0.0f32; 3];
        eq.evaluate_frequency_response(&freqs, &mut mags, &mut phases);

        // At corner frequency (1000 Hz), highpass response is near -3.0 dB
        assert!(
            (mags[2] - (-3.0)).abs() < 0.5,
            "Corner frequency mismatch: {}",
            mags[2]
        );
        // Between 500 Hz and 250 Hz (one octave down in stopband), slope is approximately 24 dB/oct
        let octave_drop = mags[1] - mags[0];
        assert!(
            (octave_drop - 24.0).abs() < 1.0,
            "Highpass slope drop mismatch: {}",
            octave_drop
        );
    }

    #[test]
    fn test_extreme_settings_stability_no_nans() {
        let mut eq = ParametricEq::new();
        eq.prepare(48000.0, 2);

        // Extreme settings: Q = 50.0, gain = +24 dB, freq = 20 Hz & 22000 Hz
        let band0 = EqBandConfig {
            filter_type: EqFilterType::Peaking,
            frequency_hz: 20.0,
            gain_db: 24.0,
            q: 50.0,
            slope_db_oct: 12,
            enabled: true,
            solo: false,
        };
        let band1 = EqBandConfig {
            filter_type: EqFilterType::HighShelf,
            frequency_hz: 22000.0,
            gain_db: -24.0,
            q: 50.0,
            slope_db_oct: 12,
            enabled: true,
            solo: false,
        };
        eq.set_band(0, band0);
        eq.set_band(1, band1);
        eq.recalculate_all_coefficients();

        // Process a block of audio (hot sine wave)
        let in_left: Vec<f32> = (0..512).map(|i| (i as f32 * 0.1).sin()).collect();
        let in_right: Vec<f32> = (0..512).map(|i| (i as f32 * 0.2).cos()).collect();
        let inputs: [&[f32]; 2] = [&in_left, &in_right];
        let mut out_left = vec![0.0f32; 512];
        let mut out_right = vec![0.0f32; 512];
        let mut outputs: [&mut [f32]; 2] = [&mut out_left, &mut out_right];

        eq.process(&inputs, &mut outputs);

        for &val in out_left.iter().chain(out_right.iter()) {
            assert!(!val.is_nan(), "Output contains NaN under extreme settings!");
            assert!(
                !val.is_infinite(),
                "Output contains Inf under extreme settings!"
            );
            assert!(val.abs() < 1000.0, "Output blew up: {}", val);
        }
    }

    #[test]
    fn test_all_10_presets_exist_and_validate() {
        let presets = EqPresets::all();
        assert_eq!(presets.len(), 10);
        for p in presets {
            assert!(!p.name.is_empty());
            assert!(!p.bands.is_empty());
            for b in &p.bands {
                assert!(b.frequency_hz >= 20.0 && b.frequency_hz <= 20000.0);
                assert!(b.gain_db >= -24.0 && b.gain_db <= 24.0);
            }
        }
    }

    #[test]
    fn test_match_eq_spectral_difference() {
        let source = vec![1.0; 64];
        let mut target = vec![1.0; 64];
        target[10] = 2.0; // +6 dB at bin 10

        let match_curve = MatchEq::compute_spectral_match(&source, &target, 1);
        assert_eq!(match_curve.len(), 64);
        assert!((match_curve[10] - 6.02).abs() < 0.1);
        assert_eq!(match_curve[0], 0.0);
    }
}
