//! High-precision audio resampler with Fast, High, and Mastering presets.

use std::f64::consts::PI;

/// Resampler quality preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResamplerQuality {
    /// Fast cubic Hermite polynomial interpolation (minimal CPU, zero latency, for scrub & preview).
    Fast,
    /// Windowed polyphase sinc filter with Kaiser window (>120 dB stopband attenuation, <0.01 dB ripple).
    High,
    /// Mastering-grade high-order sinc filter (>140 dB stopband attenuation).
    Mastering,
}

/// Arbitrary-ratio audio resampler for single or multi-channel audio.
#[derive(Debug, Clone)]
pub struct Resampler {
    from_rate: f64,
    to_rate: f64,
    ratio: f64, // from_rate / to_rate
    quality: ResamplerQuality,
    // Sinc filter table for High and Mastering
    filter_taps: usize,
    filter_phases: usize,
    polyphase_table: Vec<f32>,
    cutoff: f64,
}

impl Resampler {
    /// Creates a new resampler from input sample rate to output sample rate.
    pub fn new(from_rate: u32, to_rate: u32, quality: ResamplerQuality) -> Self {
        let from_f = from_rate as f64;
        let to_f = to_rate as f64;
        let ratio = from_f / to_f;

        let (filter_taps, filter_phases, beta) = match quality {
            ResamplerQuality::Fast => (4, 1, 0.0),
            ResamplerQuality::High => (128, 64, 12.26),
            ResamplerQuality::Mastering => (256, 128, 14.5),
        };

        // Cutoff set below Nyquist of the lower sample rate to prevent aliasing
        let cutoff = if to_rate < from_rate {
            0.45 * (to_f / from_f)
        } else {
            0.45
        };

        let polyphase_table = if quality != ResamplerQuality::Fast {
            Self::generate_polyphase_sinc(filter_taps, filter_phases, cutoff, beta)
        } else {
            Vec::new()
        };

        Self {
            from_rate: from_f,
            to_rate: to_f,
            ratio,
            quality,
            filter_taps,
            filter_phases,
            polyphase_table,
            cutoff,
        }
    }

    /// Modified Bessel function I0 for Kaiser window design.
    fn bessel_i0(x: f64) -> f64 {
        let mut sum = 1.0;
        let mut term = 1.0;
        let mut k = 1.0;
        let x_half_sq = (x * 0.5) * (x * 0.5);

        while term > 1e-14 * sum {
            term *= x_half_sq / (k * k);
            sum += term;
            k += 1.0;
        }
        sum
    }

    /// Precomputes polyphase Kaiser-windowed sinc filter coefficients.
    fn generate_polyphase_sinc(taps: usize, phases: usize, cutoff: f64, beta: f64) -> Vec<f32> {
        let total_phases = phases + 1; // Include boundary phase at frac = 1.0
        let mut table = vec![0.0f32; taps * total_phases];
        let half_taps = taps as f64 / 2.0;
        let i0_beta = Self::bessel_i0(beta);

        for phase in 0..total_phases {
            let frac_phase = phase as f64 / phases as f64;
            let mut sum = 0.0;
            let mut row = Vec::with_capacity(taps);

            for tap in 0..taps {
                // Continuous time difference from this tap sample to the fractional query point
                let t = (half_taps - tap as f64) + frac_phase;

                // Kaiser window centered at half_taps
                let norm_t = (tap as f64 - half_taps) / half_taps;
                let kaiser = if norm_t.abs() <= 1.0 {
                    Self::bessel_i0(beta * (1.0 - norm_t * norm_t).max(0.0).sqrt()) / i0_beta
                } else {
                    0.0
                };

                // Sinc function: sin(2*pi*fc*t) / (pi*t)
                let sinc = if t.abs() < 1e-9 {
                    2.0 * cutoff
                } else {
                    (2.0 * PI * cutoff * t).sin() / (PI * t)
                };

                let h = sinc * kaiser;
                sum += h;
                row.push(h);
            }

            // Normalize filter gain to unity (0 dB DC gain)
            let norm_factor = if sum.abs() > 1e-12 { 1.0 / sum } else { 1.0 };
            for tap in 0..taps {
                table[phase * taps + tap] = (row[tap] * norm_factor) as f32;
            }
        }

        table
    }

    /// Returns the filter cutoff frequency normalized to Nyquist.
    pub fn cutoff(&self) -> f64 {
        self.cutoff
    }

    /// Returns the resample ratio (from_rate / to_rate).
    pub fn ratio(&self) -> f64 {
        self.ratio
    }

    /// Resamples a single planar channel of audio frames.
    pub fn resample_channel(&self, input: &[f32]) -> Vec<f32> {
        if input.is_empty() {
            return Vec::new();
        }

        // If sample rates are identical, return clone directly
        if (self.from_rate - self.to_rate).abs() < 1e-6 {
            return input.to_vec();
        }

        let out_frames = ((input.len() as f64) / self.ratio).round() as usize;
        let mut output = Vec::with_capacity(out_frames);

        match self.quality {
            ResamplerQuality::Fast => {
                // Cubic Hermite polynomial interpolation
                let in_len = input.len();
                for out_idx in 0..out_frames {
                    let in_pos = out_idx as f64 * self.ratio;
                    let idx = in_pos.floor() as usize;
                    let frac = (in_pos - idx as f64) as f32;

                    let y0 = if idx > 0 { input[idx - 1] } else { input[0] };
                    let y1 = if idx < in_len { input[idx] } else { 0.0 };
                    let y2 = if idx + 1 < in_len { input[idx + 1] } else { y1 };
                    let y3 = if idx + 2 < in_len { input[idx + 2] } else { y2 };

                    let c0 = y1;
                    let c1 = 0.5 * (y2 - y0);
                    let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
                    let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);

                    let sample = ((c3 * frac + c2) * frac + c1) * frac + c0;
                    output.push(sample);
                }
            }
            ResamplerQuality::High | ResamplerQuality::Mastering => {
                // Polyphase windowed sinc FIR convolution with 2-point phase interpolation
                let in_len = input.len() as isize;
                let half_taps = (self.filter_taps / 2) as isize;
                let num_phases = self.filter_phases;

                for out_idx in 0..out_frames {
                    let in_pos = out_idx as f64 * self.ratio;
                    let center_idx = in_pos.floor() as isize;
                    let frac = in_pos - center_idx as f64;

                    let phase_float = frac * num_phases as f64;
                    let p0 = phase_float.floor() as usize;
                    let p1 = (p0 + 1).min(num_phases);
                    let p_frac = (phase_float - p0 as f64) as f32;

                    let off0 = p0 * self.filter_taps;
                    let off1 = p1 * self.filter_taps;

                    let mut acc = 0.0f32;
                    let start_in = center_idx - half_taps;

                    for tap in 0..self.filter_taps {
                        let sample_idx = start_in + tap as isize;
                        let s = if sample_idx < 0 || sample_idx >= in_len {
                            0.0
                        } else {
                            input[sample_idx as usize]
                        };

                        let c0 = self.polyphase_table[off0 + tap];
                        let c1 = self.polyphase_table[off1 + tap];
                        let coeff = c0 + (c1 - c0) * p_frac;

                        acc += s * coeff;
                    }
                    output.push(acc);
                }
            }
        }

        output
    }

    /// Resamples multiple channels simultaneously.
    pub fn resample(&self, channels: &[&[f32]]) -> Vec<Vec<f32>> {
        channels
            .iter()
            .map(|ch| self.resample_channel(ch))
            .collect()
    }
}
