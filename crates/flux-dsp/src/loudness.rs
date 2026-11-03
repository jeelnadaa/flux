//! ITU-R BS.1770-4 and EBU R128 loudness metering: Momentary (M), Short-Term (S),
//! Integrated (I), Loudness Range (LRA EBU Tech 3342), and 4x True-Peak (dBTP).

use std::f32::consts::PI;

/// Loudness measurement results according to ITU-R BS.1770-4 and EBU R128.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoudnessMeasurement {
    /// Integrated loudness in LUFS / LKFS (gated).
    pub integrated_lufs: f32,
    /// Maximum momentary loudness (400 ms window) in LUFS.
    pub max_momentary_lufs: f32,
    /// Maximum short-term loudness (3000 ms window) in LUFS.
    pub max_short_term_lufs: f32,
    /// Loudness Range (LRA) in LU (EBU Tech 3342).
    pub loudness_range_lu: f32,
    /// Maximum True Peak level in dBTP (4x oversampled).
    pub max_true_peak_dbtp: f32,
    /// Linear True Peak amplitude.
    pub max_true_peak_linear: f32,
}

/// BS.1770-4 / EBU R128 loudness meter.
pub struct LoudnessMeter;

impl LoudnessMeter {
    /// Analyzes multi-channel audio frames and returns complete loudness metrics.
    pub fn analyze(channels: &[&[f32]], sample_rate: u32) -> LoudnessMeasurement {
        if channels.is_empty() || channels[0].is_empty() {
            return LoudnessMeasurement {
                integrated_lufs: -140.0,
                max_momentary_lufs: -140.0,
                max_short_term_lufs: -140.0,
                loudness_range_lu: 0.0,
                max_true_peak_dbtp: -140.0,
                max_true_peak_linear: 0.0,
            };
        }

        let sr = sample_rate as f32;
        let num_channels = channels.len();
        let total_frames = channels[0].len();

        // 1. Apply K-weighting filter to each channel
        let mut k_filtered: Vec<Vec<f32>> = Vec::with_capacity(num_channels);
        for ch in channels {
            k_filtered.push(Self::apply_k_weighting(ch, sr));
        }

        // 2. Sliding block energy: 400 ms blocks (momentary) with 75% overlap (100 ms step)
        let block_size_400ms = ((0.400 * sr).round() as usize).max(1);
        let step_size_100ms = ((0.100 * sr).round() as usize).max(1);
        let block_size_3000ms = ((3.000 * sr).round() as usize).max(1);

        let num_100ms_steps = if total_frames >= block_size_400ms {
            (total_frames - block_size_400ms) / step_size_100ms + 1
        } else {
            1
        };

        let mut momentary_powers = Vec::with_capacity(num_100ms_steps);
        let mut max_m_lufs = -140.0f32;

        for step in 0..num_100ms_steps {
            let start = step * step_size_100ms;
            let end = (start + block_size_400ms).min(total_frames);
            let len = end.saturating_sub(start).max(1);

            let mut block_sum = 0.0f64;
            for ch in &k_filtered {
                let mut ch_sum = 0.0f64;
                for &sample in &ch[start..end] {
                    ch_sum += (sample as f64) * (sample as f64);
                }
                block_sum += ch_sum / (len as f64);
            }

            let avg_power = block_sum / (num_channels as f64);
            momentary_powers.push(avg_power);

            let m_lufs = if avg_power > 1e-14 {
                (-0.691 + 10.0 * avg_power.log10()) as f32
            } else {
                -140.0
            };
            if m_lufs > max_m_lufs {
                max_m_lufs = m_lufs;
            }
        }

        // 3. Short-term loudness (3000 ms sliding window)
        let num_st_steps = if total_frames >= block_size_3000ms {
            (total_frames - block_size_3000ms) / step_size_100ms + 1
        } else {
            1
        };

        let mut st_lufs_list = Vec::with_capacity(num_st_steps);
        let mut max_s_lufs = -140.0f32;

        for step in 0..num_st_steps {
            let start = step * step_size_100ms;
            let end = (start + block_size_3000ms).min(total_frames);
            let len = end.saturating_sub(start).max(1);

            let mut block_sum = 0.0f64;
            for ch in &k_filtered {
                let mut ch_sum = 0.0f64;
                for &sample in &ch[start..end] {
                    ch_sum += (sample as f64) * (sample as f64);
                }
                block_sum += ch_sum / (len as f64);
            }

            let avg_power = block_sum / (num_channels as f64);
            let s_lufs = if avg_power > 1e-14 {
                (-0.691 + 10.0 * avg_power.log10()) as f32
            } else {
                -140.0
            };

            st_lufs_list.push(s_lufs);
            if s_lufs > max_s_lufs {
                max_s_lufs = s_lufs;
            }
        }

        // 4. Gated Integrated Loudness (BS.1770-4)
        // Pass 1: Absolute threshold at -70 LKFS
        let abs_threshold_power = 10.0f64.powf((-70.0 + 0.691) / 10.0);
        let mut pass1_powers = Vec::new();

        for &p in &momentary_powers {
            if p >= abs_threshold_power {
                pass1_powers.push(p);
            }
        }

        let integrated_lufs = if pass1_powers.is_empty() {
            -140.0
        } else {
            let pass1_avg = pass1_powers.iter().sum::<f64>() / (pass1_powers.len() as f64);
            let pass1_lufs = -0.691 + 10.0 * pass1_avg.log10();

            // Pass 2: Relative threshold at -10 LU below Pass 1 average
            let rel_threshold_lufs = pass1_lufs - 10.0;
            let rel_threshold_power = 10.0f64.powf((rel_threshold_lufs + 0.691) / 10.0);

            let mut pass2_powers = Vec::new();
            for &p in &pass1_powers {
                if p >= rel_threshold_power {
                    pass2_powers.push(p);
                }
            }

            if pass2_powers.is_empty() {
                -140.0
            } else {
                let final_avg = pass2_powers.iter().sum::<f64>() / (pass2_powers.len() as f64);
                (-0.691 + 10.0 * final_avg.log10()) as f32
            }
        };

        // 5. Loudness Range (LRA per EBU Tech 3342)
        let lra = Self::compute_lra(&st_lufs_list);

        // 6. Max True Peak (4x oversampled sinc)
        let (max_tp_lin, max_tp_dbtp) = Self::compute_true_peak(channels);

        LoudnessMeasurement {
            integrated_lufs,
            max_momentary_lufs: max_m_lufs,
            max_short_term_lufs: max_s_lufs,
            loudness_range_lu: lra,
            max_true_peak_dbtp: max_tp_dbtp,
            max_true_peak_linear: max_tp_lin,
        }
    }

    /// Computes Loudness Range (LRA) according to EBU Tech 3342.
    fn compute_lra(st_lufs: &[f32]) -> f32 {
        if st_lufs.is_empty() {
            return 0.0;
        }

        // Absolute gate: -70 LKFS
        let abs_gated: Vec<f32> = st_lufs.iter().copied().filter(|&l| l >= -70.0).collect();
        if abs_gated.is_empty() {
            return 0.0;
        }

        // Calculate power mean of abs_gated
        let sum_power: f64 = abs_gated
            .iter()
            .map(|&l| 10.0f64.powf((l as f64 + 0.691) / 10.0))
            .sum();
        let avg_power = sum_power / (abs_gated.len() as f64);
        let ungated_lufs = (-0.691 + 10.0 * avg_power.log10()) as f32;

        // Relative threshold: -20 LU below ungated level
        let rel_threshold = ungated_lufs - 20.0;
        let mut rel_gated: Vec<f32> = abs_gated
            .into_iter()
            .filter(|&l| l >= rel_threshold)
            .collect();
        if rel_gated.is_empty() {
            return 0.0;
        }

        rel_gated.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // 10th percentile and 95th percentile
        let n = rel_gated.len() as f32;
        let idx_10 = ((0.10 * (n - 1.0)).round() as usize).min(rel_gated.len() - 1);
        let idx_95 = ((0.95 * (n - 1.0)).round() as usize).min(rel_gated.len() - 1);

        (rel_gated[idx_95] - rel_gated[idx_10]).max(0.0)
    }

    /// Computes True Peak with 4x polyphase sinc oversampling.
    fn compute_true_peak(channels: &[&[f32]]) -> (f32, f32) {
        let oversample = 4;
        let half_taps = 12;
        let mut global_max = 0.0f32;

        for ch in channels {
            let len = ch.len();
            for i in 0..len {
                for phase in 0..oversample {
                    let offset = (phase as f32) / (oversample as f32);
                    let mut interp = 0.0f32;

                    for tap in -(half_taps as isize)..=(half_taps as isize) {
                        let idx = (i as isize) + tap;
                        if idx >= 0 && (idx as usize) < len {
                            let x = (tap as f32) - offset;
                            let sinc = if x.abs() < 1e-5 {
                                1.0
                            } else {
                                (PI * x).sin() / (PI * x)
                            };
                            let win = 0.5 * (1.0 + (PI * x / (half_taps as f32)).cos());
                            interp += ch[idx as usize] * sinc * win;
                        }
                    }
                    if interp.abs() > global_max {
                        global_max = interp.abs();
                    }
                }
            }
        }

        let dbtp = if global_max <= 1e-7 {
            -140.0
        } else {
            20.0 * global_max.log10()
        };

        (global_max, dbtp)
    }

    /// Applies two-stage K-weighting biquad filter.
    fn apply_k_weighting(signal: &[f32], sample_rate: f32) -> Vec<f32> {
        let (b0_s1, b1_s1, b2_s1, a1_s1, a2_s1) = shelf_coeffs(sample_rate);
        let (b0_s2, b1_s2, b2_s2, a1_s2, a2_s2) = rlb_coeffs(sample_rate);

        // Stage 1: High shelf filter
        let mut s1_out = Vec::with_capacity(signal.len());
        let mut x1 = 0.0f64;
        let mut x2 = 0.0f64;
        let mut y1 = 0.0f64;
        let mut y2 = 0.0f64;

        for &s in signal {
            let x0 = s as f64;
            let y0 = b0_s1 * x0 + b1_s1 * x1 + b2_s1 * x2 - a1_s1 * y1 - a2_s1 * y2;
            x2 = x1;
            x1 = x0;
            y2 = y1;
            y1 = y0;
            s1_out.push(y0 as f32);
        }

        // Stage 2: High pass RLB filter
        let mut s2_out = Vec::with_capacity(signal.len());
        x1 = 0.0;
        x2 = 0.0;
        y1 = 0.0;
        y2 = 0.0;

        for s in s1_out {
            let x0 = s as f64;
            let y0 = b0_s2 * x0 + b1_s2 * x1 + b2_s2 * x2 - a1_s2 * y1 - a2_s2 * y2;
            x2 = x1;
            x1 = x0;
            y2 = y1;
            y1 = y0;
            s2_out.push(y0 as f32);
        }

        s2_out
    }
}

fn shelf_coeffs(sample_rate: f32) -> (f64, f64, f64, f64, f64) {
    let f0 = 1681.974450955533f64;
    let g_db = 3.999843753885252f64;
    let q = 0.7071752369274193f64;
    let fs = sample_rate as f64;

    let k = (std::f64::consts::PI * f0 / fs).tan();
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

fn rlb_coeffs(sample_rate: f32) -> (f64, f64, f64, f64, f64) {
    let f0 = 38.13547087602452f64;
    let q = 0.5003270373238773f64;
    let fs = sample_rate as f64;

    let k = (std::f64::consts::PI * f0 / fs).tan();

    let a0 = 1.0 + k / q + k * k;
    let b0 = 1.0 / a0;
    let b1 = -2.0 / a0;
    let b2 = 1.0 / a0;
    let a1 = 2.0 * (k * k - 1.0) / a0;
    let a2 = (1.0 - k / q + k * k) / a0;

    (b0, b1, b2, a1, a2)
}
