//! Production-grade biquad filter and cascaded biquad filter chain (DF2T).
//!
//! Implements Robert Bristow-Johnson (RBJ) Audio EQ Cookbook formulas with
//! 64-bit internal state to prevent limit cycles and coefficient quantization.
//! Denormals are flushed explicitly to comply with WebAssembly execution constraints.

use crate::node::DspNode;

/// Filter response types supported by the biquad filter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BiquadType {
    /// Second-order lowpass filter.
    Lowpass,
    /// Second-order highpass filter.
    Highpass,
    /// Second-order bandpass filter (constant skirt gain, peak gain = Q).
    Bandpass,
    /// Parametric peaking/bell EQ filter.
    Peaking,
    /// Low-shelving EQ filter.
    LowShelf,
    /// High-shelving EQ filter.
    HighShelf,
    /// Notch (band-stop) filter.
    Notch,
}

/// Normalized biquad filter coefficients (with a0 normalized to 1.0).
#[derive(Debug, Clone, Copy)]
pub struct BiquadCoeffs {
    /// Feedforward coefficient b0.
    pub b0: f64,
    /// Feedforward coefficient b1.
    pub b1: f64,
    /// Feedforward coefficient b2.
    pub b2: f64,
    /// Feedback coefficient a1 (negated in DF2T difference equation).
    pub a1: f64,
    /// Feedback coefficient a2 (negated in DF2T difference equation).
    pub a2: f64,
}

impl Default for BiquadCoeffs {
    fn default() -> Self {
        // Identity passthrough
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }
}

impl BiquadCoeffs {
    /// Computes normalized biquad coefficients using RBJ Audio EQ Cookbook equations.
    pub fn calculate(
        filter_type: BiquadType,
        sample_rate: f64,
        frequency_hz: f64,
        q: f64,
        gain_db: f64,
    ) -> Self {
        let sr = sample_rate.max(1.0);
        // Clamp frequency to [10 Hz, 0.499 * Fs]
        let f0 = frequency_hz.clamp(10.0, 0.499 * sr);
        let q_val = q.clamp(0.05, 50.0);

        let omega = 2.0 * std::f64::consts::PI * f0 / sr;
        let sn = omega.sin();
        let cs = omega.cos();
        let alpha = sn / (2.0 * q_val);
        let a = 10.0f64.powf(gain_db / 40.0);

        let (b0, b1, b2, a0, a1, a2) = match filter_type {
            BiquadType::Lowpass => {
                let b0 = (1.0 - cs) / 2.0;
                let b1 = 1.0 - cs;
                let b2 = (1.0 - cs) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cs;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            BiquadType::Highpass => {
                let b0 = (1.0 + cs) / 2.0;
                let b1 = -(1.0 + cs);
                let b2 = (1.0 + cs) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cs;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            BiquadType::Bandpass => {
                let b0 = alpha;
                let b1 = 0.0;
                let b2 = -alpha;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cs;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            BiquadType::Peaking => {
                let b0 = 1.0 + alpha * a;
                let b1 = -2.0 * cs;
                let b2 = 1.0 - alpha * a;
                let a0 = 1.0 + alpha / a;
                let a1 = -2.0 * cs;
                let a2 = 1.0 - alpha / a;
                (b0, b1, b2, a0, a1, a2)
            }
            BiquadType::LowShelf => {
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                let b0 = a * ((a + 1.0) - (a - 1.0) * cs + two_sqrt_a_alpha);
                let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cs);
                let b2 = a * ((a + 1.0) - (a - 1.0) * cs - two_sqrt_a_alpha);
                let a0 = (a + 1.0) + (a - 1.0) * cs + two_sqrt_a_alpha;
                let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cs);
                let a2 = (a + 1.0) + (a - 1.0) * cs - two_sqrt_a_alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            BiquadType::HighShelf => {
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                let b0 = a * ((a + 1.0) + (a - 1.0) * cs + two_sqrt_a_alpha);
                let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cs);
                let b2 = a * ((a + 1.0) + (a - 1.0) * cs - two_sqrt_a_alpha);
                let a0 = (a + 1.0) - (a - 1.0) * cs + two_sqrt_a_alpha;
                let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cs);
                let a2 = (a + 1.0) - (a - 1.0) * cs - two_sqrt_a_alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            BiquadType::Notch => {
                let b0 = 1.0;
                let b1 = -2.0 * cs;
                let b2 = 1.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cs;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };

        let inv_a0 = 1.0 / a0;
        Self {
            b0: b0 * inv_a0,
            b1: b1 * inv_a0,
            b2: b2 * inv_a0,
            a1: a1 * inv_a0,
            a2: a2 * inv_a0,
        }
    }
}

/// Direct Form II Transposed biquad filter state for one audio channel.
#[derive(Debug, Clone, Copy, Default)]
pub struct BiquadState {
    s1: f64,
    s2: f64,
}

impl BiquadState {
    /// Resets filter delay memories to zero.
    pub fn reset(&mut self) {
        self.s1 = 0.0;
        self.s2 = 0.0;
    }

    /// Processes a single input sample through the DF2T difference equation.
    #[inline(always)]
    pub fn process_sample(&mut self, input: f32, coeffs: &BiquadCoeffs) -> f32 {
        let x = input as f64;
        let y = coeffs.b0 * x + self.s1;

        self.s1 = coeffs.b1 * x - coeffs.a1 * y + self.s2;
        self.s2 = coeffs.b2 * x - coeffs.a2 * y;

        // Anti-denormal flush: prevent subnormal float performance degradation in WASM
        if self.s1.abs() < 1e-20 {
            self.s1 = 0.0;
        }
        if self.s2.abs() < 1e-20 {
            self.s2 = 0.0;
        }

        y as f32
    }
}

/// Cascaded chain of biquad filter stages.
#[derive(Debug, Clone)]
pub struct BiquadChain {
    sample_rate: f32,
    channels: usize,
    stages: usize,
    coeffs: Vec<BiquadCoeffs>,
    states: Vec<Vec<BiquadState>>,
    bypassed: bool,
}

impl BiquadChain {
    /// Creates a cascaded biquad chain with `stages` filters.
    pub fn new(stages: usize) -> Self {
        let stages_clamped = stages.max(1);
        Self {
            sample_rate: 48000.0,
            channels: 2,
            stages: stages_clamped,
            coeffs: vec![BiquadCoeffs::default(); stages_clamped],
            states: vec![vec![BiquadState::default(); 2]; stages_clamped],
            bypassed: false,
        }
    }

    /// Configures a specific stage in the chain.
    pub fn configure_stage(
        &mut self,
        stage: usize,
        filter_type: BiquadType,
        freq: f32,
        q: f32,
        gain_db: f32,
    ) {
        if stage < self.stages {
            self.coeffs[stage] = BiquadCoeffs::calculate(
                filter_type,
                self.sample_rate as f64,
                freq as f64,
                q as f64,
                gain_db as f64,
            );
        }
    }

    /// Returns the number of filter stages in this chain.
    pub fn stage_count(&self) -> usize {
        self.stages
    }
}

impl DspNode for BiquadChain {
    fn name(&self) -> &str {
        "BiquadChain"
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize) {
        self.sample_rate = sample_rate;
        self.channels = channels.max(1);
        self.states = vec![vec![BiquadState::default(); self.channels]; self.stages];
    }

    fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]]) {
        let chs = input.len().min(output.len()).min(self.channels);
        if chs == 0 {
            return;
        }
        let frames = input[0].len().min(output[0].len());

        if self.bypassed {
            for ch in 0..chs {
                output[ch][..frames].copy_from_slice(&input[ch][..frames]);
            }
            return;
        }

        // Process through cascaded stages
        for ch in 0..chs {
            // First stage reads from input, writes to output
            let in_ch = input[ch];
            let out_ch = &mut output[ch][..frames];

            if self.stages == 0 {
                out_ch.copy_from_slice(&in_ch[..frames]);
                continue;
            }

            let first_coeff = self.coeffs[0];
            let first_state = &mut self.states[0][ch];
            for i in 0..frames {
                out_ch[i] = first_state.process_sample(in_ch[i], &first_coeff);
            }

            // Subsequent stages process in-place on output buffer
            for stage in 1..self.stages {
                let coeff = self.coeffs[stage];
                let state = &mut self.states[stage][ch];
                for sample in out_ch.iter_mut().take(frames) {
                    *sample = state.process_sample(*sample, &coeff);
                }
            }
        }
    }

    fn reset(&mut self) {
        for stage_states in &mut self.states {
            for state in stage_states.iter_mut() {
                state.reset();
            }
        }
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        // Parameter mapping: stage = index / 3
        // 0: freq (10..20000 Hz)
        // 1: Q (0.1..10)
        // 2: gain_db (-24..+24 dB)
        let stage = index / 3;
        let param_type = index % 3;
        if stage < self.stages {
            let mut freq = 1000.0;
            let mut q = 0.707;
            let mut gain_db = 0.0;

            match param_type {
                0 => freq = value.clamp(20.0, 20000.0),
                1 => q = value.clamp(0.1, 10.0),
                2 => gain_db = value.clamp(-24.0, 24.0),
                _ => {}
            }

            self.configure_stage(stage, BiquadType::Peaking, freq, q, gain_db);
        }
    }

    fn get_parameter(&self, _index: usize) -> f32 {
        0.0
    }

    fn parameter_count(&self) -> usize {
        self.stages * 3
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
    fn test_biquad_peaking_analytic() {
        let coeffs = BiquadCoeffs::calculate(BiquadType::Peaking, 48000.0, 1000.0, 1.0, 6.0);
        // At DC (0 Hz), peaking EQ response has 0 dB gain (y = x)
        let dc_gain = (coeffs.b0 + coeffs.b1 + coeffs.b2) / (1.0 + coeffs.a1 + coeffs.a2);
        assert!((dc_gain - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_biquad_chain_processing() {
        let mut chain = BiquadChain::new(4);
        chain.prepare(48000.0, 1);
        chain.configure_stage(0, BiquadType::Lowpass, 5000.0, 0.707, 0.0);

        let input = vec![1.0; 128];
        let mut output = vec![0.0; 128];

        chain.process(&[&input], &mut [&mut output]);
        // After transition, lowpass step response settles towards 1.0
        assert!((output[127] - 1.0).abs() < 0.05);
    }
}
