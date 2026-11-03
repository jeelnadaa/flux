//! Peak and BS.1770 LUFS loudness normalization with true-peak ceiling.

use crate::decoder::AudioBuffer;
use std::f64::consts::PI;

/// Normalization mode for export processing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NormalizationMode {
    /// No normalization applied.
    Off,
    /// Normalize peak amplitude to target dBFS (e.g. -0.3 dBFS).
    Peak {
        /// Target peak level in dBFS.
        target_dbfs: f32,
    },
    /// Normalize integrated loudness to target LUFS with true-peak limiter ceiling.
    Lufs {
        /// Target integrated loudness in LUFS (e.g. -14.0 LUFS).
        target_lufs: f32,
        /// Maximum allowable true peak in dBTP (e.g. -1.0 dBTP).
        true_peak_ceiling_dbtp: f32,
    },
}

/// Normalizer helper.
pub struct Normalizer;

impl Normalizer {
    /// Applies normalization in-place to an `AudioBuffer`.
    pub fn apply(buffer: &mut AudioBuffer, mode: NormalizationMode) {
        match mode {
            NormalizationMode::Off => {}
            NormalizationMode::Peak { target_dbfs } => {
                Self::apply_peak(buffer, target_dbfs);
            }
            NormalizationMode::Lufs {
                target_lufs,
                true_peak_ceiling_dbtp,
            } => {
                Self::apply_lufs(buffer, target_lufs, true_peak_ceiling_dbtp);
            }
        }
    }

    /// Peak normalization.
    fn apply_peak(buffer: &mut AudioBuffer, target_dbfs: f32) {
        let mut max_peak = 0.0f32;
        for ch in &buffer.channels {
            for &s in ch {
                let a = s.abs();
                if a > max_peak {
                    max_peak = a;
                }
            }
        }

        if max_peak <= 1e-7 {
            return;
        }

        let target_linear = 10.0f32.powf(target_dbfs / 20.0);
        let gain = target_linear / max_peak;

        for ch in &mut buffer.channels {
            for s in ch.iter_mut() {
                *s *= gain;
            }
        }
    }

    /// BS.1770-4 LUFS integrated loudness normalization with true-peak ceiling.
    fn apply_lufs(buffer: &mut AudioBuffer, target_lufs: f32, true_peak_ceiling_dbtp: f32) {
        let sample_rate = buffer.sample_rate as f32;
        let num_frames = buffer.num_frames();
        if num_frames == 0 || buffer.channels.is_empty() {
            return;
        }

        // 1. Calculate BS.1770 integrated loudness
        let measured_lufs = Self::measure_integrated_lufs(buffer, sample_rate);
        if measured_lufs <= -100.0 {
            return; // Silence or near-zero signal
        }

        // 2. Compute required loudness gain
        let lufs_delta = target_lufs - measured_lufs;
        let gain_linear = 10.0f32.powf(lufs_delta / 20.0);

        for ch in &mut buffer.channels {
            for s in ch.iter_mut() {
                *s *= gain_linear;
            }
        }

        // 3. Measure 4x True Peak and enforce ceiling
        let max_tp_dbtp = Self::measure_max_true_peak(buffer);
        if max_tp_dbtp > true_peak_ceiling_dbtp {
            let reduction_db = true_peak_ceiling_dbtp - max_tp_dbtp;
            let reduction_linear = 10.0f32.powf(reduction_db / 20.0);
            for ch in &mut buffer.channels {
                for s in ch.iter_mut() {
                    *s *= reduction_linear;
                }
            }
        }
    }

    /// Measures integrated loudness across all channels using ITU-R BS.1770-4 K-weighting.
    pub fn measure_integrated_lufs(buffer: &AudioBuffer, sample_rate: f32) -> f32 {
        let (b0_s1, b1_s1, b2_s1, a1_s1, a2_s1) = shelf_filter_coeffs(sample_rate);
        let (b0_s2, b1_s2, b2_s2, a1_s2, a2_s2) = rlb_filter_coeffs(sample_rate);

        let mut channel_sum_sq = 0.0f64;

        for ch in &buffer.channels {
            if ch.is_empty() {
                continue;
            }

            // Stage 1: High shelf filter
            let mut filtered_s1 = Vec::with_capacity(ch.len());
            let mut x1 = 0.0f64;
            let mut x2 = 0.0f64;
            let mut y1 = 0.0f64;
            let mut y2 = 0.0f64;

            for &sample in ch {
                let x0 = sample as f64;
                let y0 = b0_s1 * x0 + b1_s1 * x1 + b2_s1 * x2 - a1_s1 * y1 - a2_s1 * y2;
                x2 = x1;
                x1 = x0;
                y2 = y1;
                y1 = y0;
                filtered_s1.push(y0 as f32);
            }

            // Stage 2: High pass RLB filter
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

            let mean_sq = sum_sq / (ch.len() as f64);
            channel_sum_sq += mean_sq;
        }

        let avg_mean_sq = channel_sum_sq / (buffer.channels.len() as f64);
        if avg_mean_sq <= 1e-14 {
            -140.0
        } else {
            (-0.691 + 10.0 * avg_mean_sq.log10()) as f32
        }
    }

    /// Measures maximum true-peak level in dBTP across all channels using 4x oversampling.
    pub fn measure_max_true_peak(buffer: &AudioBuffer) -> f32 {
        let oversample_factor = 4;
        let filter_half_len = 12;
        let mut global_max = 0.0f32;

        for ch in &buffer.channels {
            for i in 0..ch.len() {
                for phase in 0..oversample_factor {
                    let offset = (phase as f32) / (oversample_factor as f32);
                    let mut interp = 0.0f32;

                    for tap in -(filter_half_len as isize)..=(filter_half_len as isize) {
                        let idx = (i as isize) + tap;
                        if idx >= 0 && (idx as usize) < ch.len() {
                            let x = (tap as f32) - offset;
                            let sinc = if x.abs() < 1e-5 {
                                1.0
                            } else {
                                (std::f32::consts::PI * x).sin() / (std::f32::consts::PI * x)
                            };
                            let win = 0.5
                                * (1.0
                                    + (std::f32::consts::PI * x / (filter_half_len as f32)).cos());
                            interp += ch[idx as usize] * sinc * win;
                        }
                    }
                    global_max = global_max.max(interp.abs());
                }
            }
        }

        if global_max <= 1e-7 {
            -140.0
        } else {
            20.0 * global_max.log10()
        }
    }
}

fn shelf_filter_coeffs(sample_rate: f32) -> (f64, f64, f64, f64, f64) {
    let f0 = 1681.974450955533f64;
    let g_db = 3.999843753885252f64;
    let q = 0.7071752369274193f64;
    let fs = sample_rate as f64;

    let k = (PI * f0 / fs).tan();
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
    let f0 = 38.13547087602452f64;
    let q = 0.5003270373238773f64;
    let fs = sample_rate as f64;

    let k = (PI * f0 / fs).tan();

    let a0 = 1.0 + k / q + k * k;
    let b0 = 1.0 / a0;
    let b1 = -2.0 / a0;
    let b2 = 1.0 / a0;
    let a1 = 2.0 * (k * k - 1.0) / a0;
    let a2 = (1.0 - k / q + k * k) / a0;

    (b0, b1, b2, a1, a2)
}
