//! Signal generators for testing DSP algorithms and audio processing chains.

use std::f32::consts::PI;

/// Generates a sine wave.
///
/// # Arguments
/// * `freq` - Frequency in Hz
/// * `sample_rate` - Sampling rate in Hz
/// * `duration_secs` - Duration of the signal in seconds
/// * `amplitude` - Peak amplitude (linear, typically 1.0 = 0 dBFS)
/// * `phase` - Initial phase in radians
pub fn sine(
    freq: f32,
    sample_rate: f32,
    duration_secs: f32,
    amplitude: f32,
    phase: f32,
) -> Vec<f32> {
    let num_samples = (sample_rate * duration_secs).round() as usize;
    let mut out = Vec::with_capacity(num_samples);
    let phase_inc = (2.0 * std::f64::consts::PI * (freq as f64)) / (sample_rate as f64);
    let init_phase = phase as f64;

    for i in 0..num_samples {
        let p = init_phase + (i as f64) * phase_inc;
        let sample = (amplitude as f64) * p.sin();
        out.push(sample as f32);
    }
    out
}

/// Generates a linear frequency chirp / sine sweep.
///
/// # Arguments
/// * `start_freq` - Starting frequency in Hz
/// * `end_freq` - Ending frequency in Hz
/// * `sample_rate` - Sampling rate in Hz
/// * `duration_secs` - Duration in seconds
/// * `amplitude` - Peak amplitude
pub fn chirp_linear(
    start_freq: f32,
    end_freq: f32,
    sample_rate: f32,
    duration_secs: f32,
    amplitude: f32,
) -> Vec<f32> {
    let num_samples = (sample_rate * duration_secs).round() as usize;
    let mut out = Vec::with_capacity(num_samples);
    let t_total = duration_secs;
    let k = (end_freq - start_freq) / t_total;

    for i in 0..num_samples {
        let t = (i as f32) / sample_rate;
        let instantaneous_phase = 2.0 * PI * (start_freq * t + 0.5 * k * t * t);
        out.push(amplitude * instantaneous_phase.sin());
    }
    out
}

/// Generates an exponential / logarithmic frequency chirp.
pub fn chirp_log(
    start_freq: f32,
    end_freq: f32,
    sample_rate: f32,
    duration_secs: f32,
    amplitude: f32,
) -> Vec<f32> {
    let num_samples = (sample_rate * duration_secs).round() as usize;
    let mut out = Vec::with_capacity(num_samples);
    let ratio = end_freq / start_freq;

    for i in 0..num_samples {
        let t = (i as f32) / sample_rate;
        let frac = t / duration_secs;
        let instantaneous_phase =
            2.0 * PI * start_freq * duration_secs * (ratio.powf(frac) - 1.0) / ratio.ln();
        out.push(amplitude * instantaneous_phase.sin());
    }
    out
}

/// Generates white noise using a seeded PRNG for bit-exact determinism.
pub fn white_noise(len: usize, seed: u64, amplitude: f32) -> Vec<f32> {
    let mut rng = fastrand::Rng::with_seed(seed);
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        // Uniform in [-1.0, 1.0]
        let val = rng.f32() * 2.0 - 1.0;
        out.push(val * amplitude);
    }
    out
}

/// Generates pink noise (-3 dB / octave) using Paul Kellet's filter method.
pub fn pink_noise(len: usize, seed: u64, amplitude: f32) -> Vec<f32> {
    let mut rng = fastrand::Rng::with_seed(seed);
    let mut out = Vec::with_capacity(len);

    let mut b0 = 0.0f32;
    let mut b1 = 0.0f32;
    let mut b2 = 0.0f32;
    let mut b3 = 0.0f32;
    let mut b4 = 0.0f32;
    let mut b5 = 0.0f32;
    let mut b6 = 0.0f32;

    for _ in 0..len {
        let white = rng.f32() * 2.0 - 1.0;
        b0 = 0.99886 * b0 + white * 0.0555179;
        b1 = 0.99332 * b1 + white * 0.0750759;
        b2 = 0.96900 * b2 + white * 0.153_852;
        b3 = 0.86650 * b3 + white * 0.3104856;
        b4 = 0.55000 * b4 + white * 0.5329522;
        b5 = -0.7616 * b5 - white * 0.0168980;
        let pink = b0 + b1 + b2 + b3 + b4 + b5 + b6 + white * 0.5362;
        b6 = white * 0.115926;
        // Normalization scale factor for Kellet filter output (~0.11)
        out.push(pink * 0.11 * amplitude);
    }
    out
}

/// Generates a discrete unit impulse at specified delay.
pub fn impulse(len: usize, delay_samples: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; len];
    if delay_samples < len {
        out[delay_samples] = 1.0;
    }
    out
}

/// Generates a multitone signal containing a sum of sinusoids.
pub fn multitone(
    freqs: &[f32],
    sample_rate: f32,
    duration_secs: f32,
    total_amplitude: f32,
) -> Vec<f32> {
    if freqs.is_empty() {
        return silence((sample_rate * duration_secs).round() as usize);
    }
    let per_tone_amp = total_amplitude / (freqs.len() as f32);
    let num_samples = (sample_rate * duration_secs).round() as usize;
    let mut out = vec![0.0f32; num_samples];

    for (idx, &freq) in freqs.iter().enumerate() {
        // Pseudo-random phase distribution to avoid peak crest factor buildup
        let phase = (idx as f32) * (idx as f32) * PI / (freqs.len() as f32);
        let tone = sine(freq, sample_rate, duration_secs, per_tone_amp, phase);
        for i in 0..num_samples {
            out[i] += tone[i];
        }
    }
    out
}

/// Generates constant DC signal.
pub fn dc(value: f32, len: usize) -> Vec<f32> {
    vec![value; len]
}

/// Generates digital silence (all zeros).
pub fn silence(len: usize) -> Vec<f32> {
    vec![0.0f32; len]
}
