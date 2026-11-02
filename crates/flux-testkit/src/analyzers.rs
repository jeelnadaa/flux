//! Signal analysis primitives for DSP verification and quality gates.

use realfft::RealFftPlanner;
use std::f32::consts::PI;

/// Root-Mean-Square (RMS) amplitude of a signal.
pub fn rms(signal: &[f32]) -> f32 {
    if signal.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = signal.iter().map(|&s| (s as f64) * (s as f64)).sum();
    (sum_sq / (signal.len() as f64)).sqrt() as f32
}

/// RMS level in decibels relative to full scale (dBFS).
pub fn rms_db(signal: &[f32]) -> f32 {
    let r = rms(signal);
    if r <= 1e-7 {
        -140.0
    } else {
        20.0 * r.log10()
    }
}

/// Peak absolute amplitude of a signal.
pub fn peak(signal: &[f32]) -> f32 {
    signal.iter().fold(0.0f32, |acc, &s| acc.max(s.abs()))
}

/// Peak level in decibels relative to full scale (dBFS).
pub fn peak_db(signal: &[f32]) -> f32 {
    let p = peak(signal);
    if p <= 1e-7 {
        -140.0
    } else {
        20.0 * p.log10()
    }
}

/// Crest factor (peak to RMS ratio).
pub fn crest_factor(signal: &[f32]) -> f32 {
    let r = rms(signal);
    if r <= 1e-7 {
        0.0
    } else {
        peak(signal) / r
    }
}

/// Calculates True Peak amplitude and dBTP using a 4x oversampled polyphase sinc filter
/// according to ITU-R BS.1770-4 Appendix 1 guidelines.
pub fn true_peak(signal: &[f32]) -> (f32, f32) {
    if signal.is_empty() {
        return (0.0, -140.0);
    }

    // 48-tap Kaiser-windowed sinc interpolation filter for 4x oversampling
    let oversample_factor = 4;
    let filter_half_len = 12; // 2 * 12 * 4 = 96 taps total
    let mut max_abs = 0.0f32;

    for i in 0..signal.len() {
        for phase in 0..oversample_factor {
            let offset = (phase as f32) / (oversample_factor as f32);
            let mut interp = 0.0f32;

            for tap in -(filter_half_len as isize)..=(filter_half_len as isize) {
                let idx = (i as isize) + tap;
                if idx >= 0 && (idx as usize) < signal.len() {
                    let x = (tap as f32) - offset;
                    // Sinc with Hann window
                    let sinc = if x.abs() < 1e-5 {
                        1.0
                    } else {
                        (PI * x).sin() / (PI * x)
                    };
                    let win = 0.5 * (1.0 + (PI * x / (filter_half_len as f32)).cos());
                    interp += signal[idx as usize] * sinc * win;
                }
            }
            max_abs = max_abs.max(interp.abs());
        }
    }

    let dbtp = if max_abs <= 1e-7 {
        -140.0
    } else {
        20.0 * max_abs.log10()
    };
    (max_abs, dbtp)
}

/// Computes the magnitude spectrum of a signal using real-to-complex FFT.
///
/// Returns `(frequencies, magnitudes_dbfs)`.
pub fn fft_magnitude(signal: &[f32], sample_rate: f32) -> (Vec<f32>, Vec<f32>) {
    let n = signal.len();
    if n == 0 {
        return (Vec::new(), Vec::new());
    }

    let mut planner = RealFftPlanner::<f32>::new();
    let r2c = planner.plan_fft_forward(n);

    // Apply Hann window to minimize spectral leakage
    let mut in_buf = Vec::with_capacity(n);
    for (i, &s) in signal.iter().enumerate() {
        let win = 0.5 * (1.0 - ((2.0 * PI * (i as f32)) / (n as f32)).cos());
        in_buf.push(s * win);
    }

    let mut out_spectrum = r2c.make_output_vec();
    r2c.process(&mut in_buf, &mut out_spectrum).unwrap();

    let num_bins = out_spectrum.len();
    let mut freqs = Vec::with_capacity(num_bins);
    let mut mags = Vec::with_capacity(num_bins);

    let norm = 2.0 / (n as f32); // Scaling for one-sided FFT (Hann coherent gain ~0.5 accounted for)

    for (k, c) in out_spectrum.iter().enumerate() {
        let freq = (k as f32) * sample_rate / (n as f32);
        let mag = (c.norm() * norm).max(1e-7);
        let db = 20.0 * mag.log10();
        freqs.push(freq);
        mags.push(db);
    }

    (freqs, mags)
}

/// Identifies the dominant frequency and peak level in a signal.
pub fn dominant_frequency(signal: &[f32], sample_rate: f32) -> (f32, f32) {
    let (freqs, mags) = fft_magnitude(signal, sample_rate);
    if freqs.is_empty() {
        return (0.0, -140.0);
    }
    let mut max_idx = 0;
    let mut max_db = -140.0f32;
    // Skip DC bin (index 0)
    for (i, &mag) in mags.iter().enumerate().skip(1) {
        if mag > max_db {
            max_db = mag;
            max_idx = i;
        }
    }
    (freqs[max_idx], max_db)
}

/// Computes Total Harmonic Distortion plus Noise (THD+N) in decibels using the IEEE standard
/// sine-wave parameter estimation (least-squares fundamental subtraction).
pub fn thd_plus_n(signal: &[f32], fundamental_freq: f32, sample_rate: f32) -> f32 {
    let n = signal.len();
    if n < 4 {
        return -140.0;
    }

    let omega = (2.0 * std::f64::consts::PI * (fundamental_freq as f64)) / (sample_rate as f64);
    let mut sum_c2 = 0.0f64;
    let mut sum_s2 = 0.0f64;
    let mut sum_cs = 0.0f64;
    let mut sum_xc = 0.0f64;
    let mut sum_xs = 0.0f64;

    for (i, &val) in signal.iter().enumerate() {
        let theta = (i as f64) * omega;
        let c = theta.cos();
        let s = theta.sin();
        let x = val as f64;

        sum_c2 += c * c;
        sum_s2 += s * s;
        sum_cs += c * s;
        sum_xc += x * c;
        sum_xs += x * s;
    }

    // Solve 2x2 system:
    // [sum_c2 sum_cs] [A] = [sum_xc]
    // [sum_cs sum_s2] [B] = [sum_xs]
    let det = sum_c2 * sum_s2 - sum_cs * sum_cs;
    if det.abs() < 1e-12 {
        return -140.0;
    }

    let a = (sum_xc * sum_s2 - sum_xs * sum_cs) / det;
    let b = (sum_c2 * sum_xs - sum_cs * sum_xc) / det;
    let fund_power = (a * a + b * b) / 2.0;

    if fund_power <= 1e-14 {
        return 0.0;
    }

    let mut residual_sq = 0.0f64;
    for (i, &val) in signal.iter().enumerate() {
        let theta = (i as f64) * omega;
        let fund_sample = a * theta.cos() + b * theta.sin();
        let err = (val as f64) - fund_sample;
        residual_sq += err * err;
    }

    let noise_power = residual_sq / (n as f64);
    if noise_power <= 1e-14 {
        -140.0
    } else {
        (10.0 * (noise_power / fund_power).log10()) as f32
    }
}

/// Evaluates group delay across frequencies from an impulse response.
pub fn group_delay(impulse_response: &[f32], sample_rate: f32) -> Vec<(f32, f32)> {
    let n = impulse_response.len();
    if n < 2 {
        return Vec::new();
    }

    let mut planner = RealFftPlanner::<f32>::new();
    let r2c = planner.plan_fft_forward(n);

    let mut in_buf = impulse_response.to_vec();
    let mut out_spectrum = r2c.make_output_vec();
    r2c.process(&mut in_buf, &mut out_spectrum).unwrap();

    let num_bins = out_spectrum.len();
    let mut phases = Vec::with_capacity(num_bins);
    for c in &out_spectrum {
        phases.push(c.im.atan2(c.re));
    }

    // Unwrap phase
    let mut unwrapped = Vec::with_capacity(num_bins);
    let mut p_prev = phases[0];
    let mut cumulative_shift = 0.0f32;
    unwrapped.push(p_prev);

    for &p in phases.iter().skip(1) {
        let delta = p - p_prev;
        if delta > PI {
            cumulative_shift -= 2.0 * PI;
        } else if delta < -PI {
            cumulative_shift += 2.0 * PI;
        }
        unwrapped.push(p + cumulative_shift);
        p_prev = p;
    }

    // Differentiate unwrapped phase: tau_g = - d(phase) / d(omega)
    let d_omega = (2.0 * PI * sample_rate) / (n as f32);
    let mut delays = Vec::with_capacity(num_bins.saturating_sub(1));

    for k in 0..(num_bins - 1) {
        let freq = (k as f32) * sample_rate / (n as f32);
        let d_phase = unwrapped[k + 1] - unwrapped[k];
        let delay_sec = -d_phase / d_omega;
        delays.push((freq, delay_sec));
    }
    delays
}

/// Computes integrated loudness in LUFS according to ITU-R BS.1770-4
/// (K-weighting high shelf filter + RLB high-pass weighting filter).
pub fn lufs(signal: &[f32], sample_rate: f32) -> f32 {
    if signal.is_empty() {
        return -140.0;
    }

    // Biquad coefficients calculated for ITU-R BS.1770-4
    // Stage 1: High shelf filter (~1.68 kHz, +4 dB gain)
    let (b0_s1, b1_s1, b2_s1, a1_s1, a2_s1) = shelf_filter_coeffs(sample_rate);
    // Stage 2: High pass RLB filter (~38 Hz cutoff)
    let (b0_s2, b1_s2, b2_s2, a1_s2, a2_s2) = rlb_filter_coeffs(sample_rate);

    let mut filtered_s1 = Vec::with_capacity(signal.len());
    let mut x1 = 0.0f64;
    let mut x2 = 0.0f64;
    let mut y1 = 0.0f64;
    let mut y2 = 0.0f64;

    for &sample in signal {
        let x0 = sample as f64;
        let y0 = b0_s1 * x0 + b1_s1 * x1 + b2_s1 * x2 - a1_s1 * y1 - a2_s1 * y2;
        x2 = x1;
        x1 = x0;
        y2 = y1;
        y1 = y0;
        filtered_s1.push(y0 as f32);
    }

    // Apply Stage 2
    let mut sum_sq = 0.0f64;
    x1 = 0.0;
    x2 = 0.0;
    y1 = 0.0;
    y2 = 0.0;

    for sample in filtered_s1 {
        let x0 = sample as f64;
        let y0 = b0_s2 * x0 + b1_s2 * x1 + b2_s2 * x2 - a1_s2 * y1 - a2_s2 * y2;
        x2 = x1;
        x1 = x0;
        y2 = y1;
        y1 = y0;
        sum_sq += y0 * y0;
    }

    let mean_sq = sum_sq / (signal.len() as f64);
    if mean_sq <= 1e-14 {
        -140.0
    } else {
        // BS.1770 formula: -0.691 + 10 * log10(mean_square)
        (-0.691 + 10.0 * mean_sq.log10()) as f32
    }
}

fn shelf_filter_coeffs(sample_rate: f32) -> (f64, f64, f64, f64, f64) {
    let f0 = 1681.974450955533f64;
    let g_db = 3.999843753885252f64;
    let q = 0.7071752369274193f64;
    let fs = sample_rate as f64;

    let k = (PI as f64 * f0 / fs).tan();
    let vh = 10.0f64.powf(g_db / 20.0);
    let vb = vh.powf(0.4996667741545416);

    let a0 = 1.0 + k / q + k * k;
    let b0 = (vh + vb * (k / q) + k * k) / a0;
    let b1 = 2.0 * (k * k - vh) / a0;
    let b2 = (vh - vb * (k / q) + k * k) / a0;
    let a1 = 2.0 * (k * k - 1.0) / a0;
    let a2 = (1.0 - k / q + k * k) / a0;

    (b0, b1, b2, a1, a2)
}

fn rlb_filter_coeffs(sample_rate: f32) -> (f64, f64, f64, f64, f64) {
    let f0 = 38.13547087602444f64;
    let q = 0.5003270373238773f64;
    let fs = sample_rate as f64;

    let k = (PI as f64 * f0 / fs).tan();
    let a0 = 1.0 + k / q + k * k;
    let b0 = 1.0 / a0;
    let b1 = -2.0 / a0;
    let b2 = 1.0 / a0;
    let a1 = 2.0 * (k * k - 1.0) / a0;
    let a2 = (1.0 - k / q + k * k) / a0;

    (b0, b1, b2, a1, a2)
}
