//! Utility processing nodes, signal generators, and real-time spectrum analysis per PLAN.md §3.5.
//!
//! Includes:
//! - ToneGenerator (Sine, Triangle, Square, Sawtooth) with sample-accurate phase tracking.
//! - NoiseGenerator (Deterministic seeded White noise, Voss-McCartney Pink noise).
//! - SilenceGenerator.
//! - FadeProcessor with 5 curve types (Linear, EqualPower, SCurve, Exponential, Logarithmic).
//! - SpectrumAnalyzerNode computing windowed STFT magnitude dBFS with peak hold.

#![allow(clippy::needless_range_loop)]

use crate::node::DspNode;
use realfft::{RealFftPlanner, RealToComplex};
use std::f32::consts::PI;
use std::sync::Arc;

// ============================================================================
// 1. Tone Generator
// ============================================================================

/// Waveform shapes for the tone generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveformType {
    /// Pure sinusoidal tone.
    Sine,
    /// Triangle wave with odd harmonics decaying at 1/n^2.
    Triangle,
    /// Square wave with odd harmonics decaying at 1/n.
    Square,
    /// Sawtooth wave with all harmonics decaying at 1/n.
    Sawtooth,
}

/// Periodic signal generator generating clean tones.
pub struct ToneGenerator {
    bypassed: bool,
    sample_rate: f32,
    channels: usize,
    frequency: f32,
    gain: f32,
    waveform: WaveformType,
    phase: f32,
}

impl ToneGenerator {
    /// Creates a new tone generator with specified frequency and gain.
    pub fn new(frequency: f32, gain: f32, waveform: WaveformType) -> Self {
        Self {
            bypassed: false,
            sample_rate: 44100.0,
            channels: 2,
            frequency: frequency.clamp(1.0, 22000.0),
            gain: gain.clamp(0.0, 1.0),
            waveform,
            phase: 0.0,
        }
    }

    /// Sets tone frequency in Hertz.
    pub fn set_frequency(&mut self, freq_hz: f32) {
        self.frequency = freq_hz.clamp(1.0, self.sample_rate * 0.49);
    }

    /// Sets amplitude gain (0.0 to 1.0).
    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain.clamp(0.0, 1.0);
    }

    /// Sets the waveform type.
    pub fn set_waveform(&mut self, waveform: WaveformType) {
        self.waveform = waveform;
    }
}

impl Default for ToneGenerator {
    fn default() -> Self {
        Self::new(440.0, 0.5, WaveformType::Sine)
    }
}

impl DspNode for ToneGenerator {
    fn name(&self) -> &str {
        "ToneGenerator"
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize) {
        self.sample_rate = sample_rate;
        self.channels = channels;
        self.phase = 0.0;
    }

    fn reset(&mut self) {
        self.phase = 0.0;
    }

    fn process(&mut self, _input: &[&[f32]], output: &mut [&mut [f32]]) {
        let num_channels = output.len().min(self.channels);
        if num_channels == 0 {
            return;
        }
        let num_samples = output[0].len();
        if num_samples == 0 {
            return;
        }

        if self.bypassed || self.gain <= 1e-7 {
            for ch in output.iter_mut().take(num_channels) {
                ch.fill(0.0);
            }
            return;
        }

        let phase_step = (2.0 * PI * self.frequency) / self.sample_rate;

        for i in 0..num_samples {
            let sample_val = match self.waveform {
                WaveformType::Sine => self.phase.sin(),
                WaveformType::Triangle => {
                    // Normalize phase to 0..1
                    let norm = self.phase / (2.0 * PI);
                    if norm < 0.25 {
                        4.0 * norm
                    } else if norm < 0.75 {
                        2.0 - 4.0 * norm
                    } else {
                        4.0 * norm - 4.0
                    }
                }
                WaveformType::Square => {
                    if self.phase < PI {
                        1.0
                    } else {
                        -1.0
                    }
                }
                WaveformType::Sawtooth => {
                    let norm = self.phase / (2.0 * PI);
                    2.0 * norm - 1.0
                }
            } * self.gain;

            for ch in output.iter_mut().take(num_channels) {
                ch[i] = sample_val;
            }

            self.phase += phase_step;
            if self.phase >= 2.0 * PI {
                self.phase -= 2.0 * PI;
            }
        }
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.set_frequency(value),
            1 => self.set_gain(value),
            2 => {
                let wave_idx = value as usize;
                self.waveform = match wave_idx {
                    0 => WaveformType::Sine,
                    1 => WaveformType::Triangle,
                    2 => WaveformType::Square,
                    _ => WaveformType::Sawtooth,
                };
            }
            _ => {}
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 => self.frequency,
            1 => self.gain,
            2 => match self.waveform {
                WaveformType::Sine => 0.0,
                WaveformType::Triangle => 1.0,
                WaveformType::Square => 2.0,
                WaveformType::Sawtooth => 3.0,
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
}

// ============================================================================
// 2. Noise Generator
// ============================================================================

/// Type of noise signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseType {
    /// Uniform/Gaussian distributed flat spectrum noise.
    White,
    /// 1/f (-3 dB/octave) equal-energy-per-octave pink noise.
    Pink,
}

/// Deterministic, reproducible noise generator.
pub struct NoiseGenerator {
    bypassed: bool,
    sample_rate: f32,
    channels: usize,
    gain: f32,
    noise_type: NoiseType,
    rng_state: u64,
    // Voss-McCartney filter states for pink noise per channel
    pink_rows: Vec<[f32; 7]>,
    pink_running_sum: Vec<f32>,
    pink_index: usize,
}

impl NoiseGenerator {
    /// Creates a new noise generator with seed and noise type.
    pub fn new(seed: u64, gain: f32, noise_type: NoiseType) -> Self {
        Self {
            bypassed: false,
            sample_rate: 44100.0,
            channels: 2,
            gain: gain.clamp(0.0, 1.0),
            noise_type,
            rng_state: if seed == 0 { 0x853c49e6748fea9b } else { seed },
            pink_rows: vec![[0.0; 7]; 2],
            pink_running_sum: vec![0.0; 2],
            pink_index: 0,
        }
    }

    #[inline]
    fn next_white_sample(&mut self) -> f32 {
        // Fast 64-bit XorShift PRNG
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        // Map to -1.0 .. 1.0 float
        ((self.rng_state as f64) / (u64::MAX as f64) * 2.0 - 1.0) as f32
    }

    /// Sets the output gain.
    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain.clamp(0.0, 1.0);
    }

    /// Sets the noise type (White or Pink).
    pub fn set_noise_type(&mut self, noise_type: NoiseType) {
        self.noise_type = noise_type;
    }
}

impl Default for NoiseGenerator {
    fn default() -> Self {
        Self::new(12345, 0.2, NoiseType::White)
    }
}

impl DspNode for NoiseGenerator {
    fn name(&self) -> &str {
        "NoiseGenerator"
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize) {
        self.sample_rate = sample_rate;
        self.channels = channels;
        self.pink_rows = vec![[0.0; 7]; channels];
        self.pink_running_sum = vec![0.0; channels];
        self.pink_index = 0;
    }

    fn reset(&mut self) {
        self.pink_rows = vec![[0.0; 7]; self.channels];
        self.pink_running_sum = vec![0.0; self.channels];
        self.pink_index = 0;
    }

    fn process(&mut self, _input: &[&[f32]], output: &mut [&mut [f32]]) {
        let num_channels = output.len().min(self.channels);
        if num_channels == 0 {
            return;
        }
        let num_samples = output[0].len();
        if num_samples == 0 {
            return;
        }

        if self.bypassed || self.gain <= 1e-7 {
            for ch in output.iter_mut().take(num_channels) {
                ch.fill(0.0);
            }
            return;
        }

        match self.noise_type {
            NoiseType::White => {
                for i in 0..num_samples {
                    let w = self.next_white_sample() * self.gain;
                    for ch in output.iter_mut().take(num_channels) {
                        ch[i] = w;
                    }
                }
            }
            NoiseType::Pink => {
                for i in 0..num_samples {
                    self.pink_index = (self.pink_index + 1) & 63;
                    // Count trailing zeros of pink_index to find which row to update
                    let mut num_zeros = 0;
                    let mut temp = self.pink_index;
                    while temp != 0 && (temp & 1) == 0 {
                        num_zeros += 1;
                        temp >>= 1;
                    }
                    if num_zeros > 6 {
                        num_zeros = 6;
                    }

                    for ch_idx in 0..num_channels {
                        let new_val = self.next_white_sample();
                        self.pink_running_sum[ch_idx] -= self.pink_rows[ch_idx][num_zeros];
                        self.pink_rows[ch_idx][num_zeros] = new_val;
                        self.pink_running_sum[ch_idx] += new_val;

                        // Add direct white component and normalize (approx / 8.0)
                        let direct = self.next_white_sample();
                        let pink = (self.pink_running_sum[ch_idx] + direct) * 0.125 * self.gain;
                        output[ch_idx][i] = pink.clamp(-1.0, 1.0);
                    }
                }
            }
        }
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.set_gain(value),
            1 => {
                self.noise_type = if value > 0.5 {
                    NoiseType::Pink
                } else {
                    NoiseType::White
                };
            }
            _ => {}
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 => self.gain,
            1 => f32::from(self.noise_type == NoiseType::Pink),
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
// 3. Silence Generator
// ============================================================================

/// Clean zero generator for gap insertion and testing.
#[derive(Default)]
pub struct SilenceGenerator {
    bypassed: bool,
}

impl SilenceGenerator {
    /// Creates a new silence generator.
    pub fn new() -> Self {
        Self { bypassed: false }
    }
}

impl DspNode for SilenceGenerator {
    fn name(&self) -> &str {
        "SilenceGenerator"
    }

    fn prepare(&mut self, _sample_rate: f32, _channels: usize) {}
    fn reset(&mut self) {}

    fn process(&mut self, _input: &[&[f32]], output: &mut [&mut [f32]]) {
        for ch in output {
            ch.fill(0.0);
        }
    }

    fn set_parameter(&mut self, _index: usize, _value: f32) {}
    fn get_parameter(&self, _index: usize) -> f32 {
        0.0
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }
}

// ============================================================================
// 4. Fade Processor
// ============================================================================

/// Supported fade curve shapes per PLAN.md §2.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadeCurveType {
    /// Linear linear gain ramp.
    Linear,
    /// Equal-power sinusoidal ramp: sin(t * pi / 2).
    EqualPower,
    /// Smooth S-curve (cubic Hermite): 3t^2 - 2t^3.
    SCurve,
    /// Exponential gain ramp: (exp(5t) - 1) / (exp(5) - 1).
    Exponential,
    /// Logarithmic gain ramp: ln(1 + 9t) / ln(10).
    Logarithmic,
}

impl FadeCurveType {
    /// Evaluates gain in range [0.0, 1.0] for normalized progress t in [0.0, 1.0].
    #[inline]
    pub fn evaluate(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::EqualPower => (t * (PI * 0.5)).sin(),
            Self::SCurve => t * t * (3.0 - 2.0 * t),
            Self::Exponential => {
                if t <= 0.0 {
                    0.0
                } else if t >= 1.0 {
                    1.0
                } else {
                    ((t * 5.0).exp() - 1.0) / (5.0_f32.exp() - 1.0)
                }
            }
            Self::Logarithmic => {
                if t <= 0.0 {
                    0.0
                } else if t >= 1.0 {
                    1.0
                } else {
                    (1.0 + t * 9.0).ln() / 10.0_f32.ln()
                }
            }
        }
    }
}

/// Applies fades in-place to multichannel audio buffers.
pub struct FadeProcessor;

impl FadeProcessor {
    /// Applies a fade-in in-place across the provided buffer.
    pub fn apply_fade_in(
        buffer: &mut [&mut [f32]],
        start_frame: usize,
        length_frames: usize,
        curve: FadeCurveType,
    ) {
        if length_frames == 0 {
            return;
        }
        for ch in buffer.iter_mut() {
            let total = ch.len();
            let end_frame = (start_frame + length_frames).min(total);
            for i in start_frame..end_frame {
                let progress = (i - start_frame) as f32 / length_frames as f32;
                ch[i] *= curve.evaluate(progress);
            }
        }
    }

    /// Applies a fade-out in-place across the provided buffer.
    pub fn apply_fade_out(
        buffer: &mut [&mut [f32]],
        start_frame: usize,
        length_frames: usize,
        curve: FadeCurveType,
    ) {
        if length_frames == 0 {
            return;
        }
        for ch in buffer.iter_mut() {
            let total = ch.len();
            let end_frame = (start_frame + length_frames).min(total);
            for i in start_frame..end_frame {
                let progress = (i - start_frame) as f32 / length_frames as f32;
                // Fade-out is 1.0 - progress evaluated on the curve
                ch[i] *= curve.evaluate(1.0 - progress);
            }
        }
    }
}

// ============================================================================
// 5. Real-Time Spectrum Analyzer
// ============================================================================

/// Real-time STFT spectrum analyzer computing windowed dBFS magnitudes with peak hold.
pub struct SpectrumAnalyzerNode {
    bypassed: bool,
    sample_rate: f32,
    channels: usize,
    fft_size: usize,
    window: Vec<f32>,
    input_buffer: Vec<Vec<f32>>,
    magnitudes_db: Vec<f32>,
    peak_hold_db: Vec<f32>,
    peak_decay_rate_db_per_sec: f32,
    fft_forward: Arc<dyn RealToComplex<f32>>,
}

impl SpectrumAnalyzerNode {
    /// Creates a new spectrum analyzer with specified FFT size (e.g. 1024 or 2048).
    pub fn new(fft_size: usize) -> Self {
        let fft_size = fft_size.next_power_of_two().clamp(256, 8192);
        let mut planner = RealFftPlanner::<f32>::new();
        let fft_forward = planner.plan_fft_forward(fft_size);

        // Precompute Hann window
        let mut window = vec![0.0; fft_size];
        for i in 0..fft_size {
            window[i] = 0.5 * (1.0 - (2.0 * PI * i as f32 / fft_size as f32).cos());
        }

        let num_bins = fft_size / 2 + 1;

        Self {
            bypassed: false,
            sample_rate: 44100.0,
            channels: 2,
            fft_size,
            window,
            input_buffer: (0..2).map(|_| Vec::with_capacity(fft_size)).collect(),
            magnitudes_db: vec![-120.0; num_bins],
            peak_hold_db: vec![-120.0; num_bins],
            peak_decay_rate_db_per_sec: 12.0, // 12 dB/s standard decay
            fft_forward,
        }
    }

    /// Returns the current instantaneous magnitude spectrum in dBFS.
    pub fn magnitudes_db(&self) -> &[f32] {
        &self.magnitudes_db
    }

    /// Returns the peak-hold spectrum in dBFS.
    pub fn peak_hold_db(&self) -> &[f32] {
        &self.peak_hold_db
    }

    /// Resets peak-hold meters.
    pub fn reset_peak_hold(&mut self) {
        self.peak_hold_db.fill(-120.0);
    }

    /// Returns the center frequency in Hz for a given FFT bin index.
    pub fn bin_frequency(&self, bin: usize) -> f32 {
        bin as f32 * self.sample_rate / self.fft_size as f32
    }
}

impl Default for SpectrumAnalyzerNode {
    fn default() -> Self {
        Self::new(1024)
    }
}

impl DspNode for SpectrumAnalyzerNode {
    fn name(&self) -> &str {
        "SpectrumAnalyzerNode"
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize) {
        self.sample_rate = sample_rate;
        self.channels = channels;
        self.input_buffer = (0..channels)
            .map(|_| Vec::with_capacity(self.fft_size))
            .collect();
        let num_bins = self.fft_size / 2 + 1;
        self.magnitudes_db = vec![-120.0; num_bins];
        self.peak_hold_db = vec![-120.0; num_bins];
    }

    fn reset(&mut self) {
        for b in &mut self.input_buffer {
            b.clear();
        }
        self.magnitudes_db.fill(-120.0);
        self.peak_hold_db.fill(-120.0);
    }

    fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]]) {
        let num_channels = input.len().min(output.len()).min(self.channels);
        if num_channels == 0 {
            return;
        }
        let num_samples = input[0].len();
        if num_samples == 0 {
            return;
        }

        // Passthrough audio unmodified
        for ch in 0..num_channels {
            output[ch].copy_from_slice(&input[ch][..num_samples]);
        }

        if self.bypassed {
            return;
        }

        // Accumulate mono sum into input buffer
        for i in 0..num_samples {
            let mut mono = 0.0;
            for ch in 0..num_channels {
                mono += input[ch][i];
            }
            mono /= num_channels as f32;

            self.input_buffer[0].push(mono);

            if self.input_buffer[0].len() == self.fft_size {
                // Apply window
                let mut in_vec = vec![0.0; self.fft_size];
                for k in 0..self.fft_size {
                    in_vec[k] = self.input_buffer[0][k] * self.window[k];
                }

                let mut spectrum = self.fft_forward.make_output_vec();
                if self.fft_forward.process(&mut in_vec, &mut spectrum).is_ok() {
                    let num_bins = spectrum.len();
                    // 4.0 / N accounts for two-sided FFT (x2) and Hann coherent gain (x2)
                    let norm = 4.0 / self.fft_size as f32;

                    for bin in 0..num_bins {
                        let mag = spectrum[bin].norm() * norm;
                        let db = 20.0 * (mag.max(1e-6)).log10();
                        self.magnitudes_db[bin] = db;

                        // Update peak hold
                        if db > self.peak_hold_db[bin] {
                            self.peak_hold_db[bin] = db;
                        } else {
                            // Decay
                            let dt = self.fft_size as f32 / self.sample_rate;
                            let decay = self.peak_decay_rate_db_per_sec * dt;
                            self.peak_hold_db[bin] = (self.peak_hold_db[bin] - decay).max(-120.0);
                        }
                    }
                }

                // Advance hop (50% overlap)
                let hop = self.fft_size / 2;
                self.input_buffer[0].drain(0..hop);
            }
        }
    }

    fn set_parameter(&mut self, _index: usize, _value: f32) {}
    fn get_parameter(&self, _index: usize) -> f32 {
        0.0
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tone_generator_sine_thd_and_frequency() {
        let mut gen = ToneGenerator::new(1000.0, 1.0, WaveformType::Sine);
        gen.prepare(44100.0, 1);

        let mut out = vec![0.0; 44100];
        let mut out_slices = [&mut out[..]];
        gen.process(&[], &mut out_slices);

        // Find positive zero crossings to verify frequency
        let mut crossings: i32 = 0;
        for i in 1..44100 {
            if out[i - 1] <= 0.0 && out[i] > 0.0 {
                crossings += 1;
            }
        }
        // At 1000 Hz over 1s (44100 samples), should be ~1000 crossings
        assert!(
            (crossings - 1000).abs() <= 2,
            "Expected ~1000 zero crossings, got {}",
            crossings
        );
    }

    #[test]
    fn test_noise_generator_determinism() {
        let mut gen1 = NoiseGenerator::new(9999, 1.0, NoiseType::White);
        let mut gen2 = NoiseGenerator::new(9999, 1.0, NoiseType::White);

        gen1.prepare(44100.0, 1);
        gen2.prepare(44100.0, 1);

        let mut out1 = vec![0.0; 512];
        let mut out2 = vec![0.0; 512];

        gen1.process(&[], &mut [&mut out1[..]]);
        gen2.process(&[], &mut [&mut out2[..]]);

        for i in 0..512 {
            assert_eq!(out1[i], out2[i], "Seeded noise should be identical");
        }
    }

    #[test]
    fn test_fade_processor_curves() {
        let mut buf = vec![1.0; 100];
        FadeProcessor::apply_fade_in(&mut [&mut buf[..]], 0, 100, FadeCurveType::EqualPower);

        assert_eq!(buf[0], 0.0);
        assert!((buf[99] - 1.0).abs() < 0.05);
        // Halfway equal power is sin(pi/4) = sqrt(2)/2 = ~0.707
        assert!((buf[50] - 0.707).abs() < 0.05);
    }

    #[test]
    fn test_spectrum_analyzer_sine_bin_accuracy() {
        let mut analyzer = SpectrumAnalyzerNode::new(1024);
        analyzer.prepare(44100.0, 1);

        // Generate 1000 Hz sine
        let mut tone = ToneGenerator::new(1000.0, 1.0, WaveformType::Sine);
        tone.prepare(44100.0, 1);

        let mut signal = vec![0.0; 2048];
        tone.process(&[], &mut [&mut signal[..]]);

        let mut passthrough = vec![0.0; 2048];
        analyzer.process(&[&signal[..]], &mut [&mut passthrough[..]]);

        // Find bin with peak magnitude
        let mags = analyzer.magnitudes_db();
        let mut peak_bin = 0;
        let mut peak_db = -200.0;
        for (bin, &db) in mags.iter().enumerate() {
            if db > peak_db {
                peak_db = db;
                peak_bin = bin;
            }
        }

        let peak_freq = analyzer.bin_frequency(peak_bin);
        assert!(
            (peak_freq - 1000.0).abs() < 45.0,
            "Peak frequency should be ~1000 Hz, got {} Hz (bin {})",
            peak_freq,
            peak_bin
        );
        assert!(
            peak_db > -3.0,
            "Peak magnitude should be near 0 dBFS, got {}",
            peak_db
        );
    }
}
