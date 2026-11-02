//! Signal comparison helpers and null test analyzers.

use crate::analyzers::{fft_magnitude, peak_db, rms_db};

/// Summary result of a null subtraction test between two audio signals.
#[derive(Debug, Clone, PartialEq)]
pub struct NullResult {
    /// Sample-by-sample difference vector (a - b).
    pub residual: Vec<f32>,
    /// Maximum absolute peak error in decibels relative to full scale.
    pub max_error_db: f32,
    /// Root-mean-square error in decibels relative to full scale.
    pub rms_error_db: f32,
    /// Whether every sample is bit-exact identical.
    pub is_bit_exact: bool,
}

/// Evaluates null subtraction residual between two signals.
pub fn null_residual(signal_a: &[f32], signal_b: &[f32]) -> NullResult {
    let len = signal_a.len().min(signal_b.len());
    let mut residual = Vec::with_capacity(len);
    let mut bit_exact = signal_a.len() == signal_b.len();

    for i in 0..len {
        let diff = signal_a[i] - signal_b[i];
        if signal_a[i].to_bits() != signal_b[i].to_bits() {
            bit_exact = false;
        }
        residual.push(diff);
    }

    let max_err = peak_db(&residual);
    let rms_err = rms_db(&residual);

    NullResult {
        residual,
        max_error_db: max_err,
        rms_error_db: rms_err,
        is_bit_exact: bit_exact,
    }
}

/// Computes the Signal-to-Noise Ratio (SNR) of a test signal against a clean reference in decibels.
pub fn snr(reference: &[f32], degraded: &[f32]) -> f32 {
    let len = reference.len().min(degraded.len());
    if len == 0 {
        return 0.0;
    }

    let mut sig_power = 0.0f64;
    let mut noise_power = 0.0f64;

    for i in 0..len {
        let s = reference[i] as f64;
        let n = (reference[i] - degraded[i]) as f64;
        sig_power += s * s;
        noise_power += n * n;
    }

    if noise_power <= 1e-14 {
        // Practically infinite SNR (identical signals)
        140.0
    } else if sig_power <= 1e-14 {
        -140.0
    } else {
        (10.0 * (sig_power / noise_power).log10()) as f32
    }
}

/// Computes mean absolute spectral distance across audible/active frequency bins in decibels.
pub fn spectral_distance(ref_signal: &[f32], test_signal: &[f32], sample_rate: f32) -> f32 {
    let (_, ref_mags) = fft_magnitude(ref_signal, sample_rate);
    let (_, test_mags) = fft_magnitude(test_signal, sample_rate);

    let num_bins = ref_mags.len().min(test_mags.len());
    if num_bins == 0 {
        return 0.0;
    }

    let mut total_diff = 0.0f32;
    let mut count = 0usize;
    for i in 0..num_bins {
        // Focus comparison on active spectral content above -80 dBFS
        if ref_mags[i] > -80.0 || test_mags[i] > -80.0 {
            total_diff += (ref_mags[i] - test_mags[i]).abs();
            count += 1;
        }
    }
    if count == 0 {
        0.0
    } else {
        total_diff / (count as f32)
    }
}
