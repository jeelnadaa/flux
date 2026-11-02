//! High-quality audio dither: TPDF and psychoacoustically noise-shaped.

/// Supported dither algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DitherType {
    /// No dithering (direct truncation/rounding).
    None,
    /// Standard Triangular Probability Density Function (flat spectrum, 0 correlation).
    Tpdf,
    /// Psychoacoustically noise-shaped dither (Lipshitz/Vanderkooy curve for 16-bit).
    NoiseShaped,
}

/// Target bit depth for quantization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitDepth {
    /// 16-bit signed integer (-32768 to 32767).
    Bits16,
    /// 24-bit signed integer (-8388608 to 8388607).
    Bits24,
    /// 32-bit floating point (no quantization, dither bypassed).
    Float32,
}

/// PRNG and error feedback state for audio dithering.
#[derive(Debug, Clone)]
pub struct Dither {
    dither_type: DitherType,
    bit_depth: BitDepth,
    // Deterministic xorshift PRNG state
    prng_state: u64,
    // Error feedback history for noise shaping (per channel)
    error_history: Vec<[f32; 5]>,
}

impl Dither {
    /// Creates a new dither processor.
    pub fn new(dither_type: DitherType, bit_depth: BitDepth, channels: usize) -> Self {
        Self {
            dither_type,
            bit_depth,
            prng_state: 0x8543_2198_7654_3210, // Deterministic seed
            error_history: vec![[0.0; 5]; channels],
        }
    }

    /// Resets the PRNG seed and clears error feedback history.
    pub fn reset(&mut self, seed: u64) {
        self.prng_state = if seed == 0 {
            0x8543_2198_7654_3210
        } else {
            seed
        };
        for hist in &mut self.error_history {
            hist.fill(0.0);
        }
    }

    /// Generates next uniform random float in [-0.5, 0.5).
    #[inline]
    fn next_uniform_state(state: &mut u64) -> f32 {
        let mut x = *state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *state = x;
        let mantissa = (x & 0x00ff_ffff) as f32 / 16777216.0;
        mantissa - 0.5
    }

    /// Generates next TPDF dither value in (-1.0, 1.0) with triangular distribution.
    #[inline]
    fn next_tpdf_state(state: &mut u64) -> f32 {
        Self::next_uniform_state(state) + Self::next_uniform_state(state)
    }

    /// Applies dithering and quantization to a multi-channel buffer of normalized [-1.0, 1.0] samples.
    /// Returns quantized samples in the same normalized float range, ready for int conversion or playback.
    pub fn process(&mut self, channels: &mut [&mut [f32]]) {
        if self.bit_depth == BitDepth::Float32 || self.dither_type == DitherType::None {
            return;
        }

        let lsb = match self.bit_depth {
            BitDepth::Bits16 => 1.0 / 32768.0,
            BitDepth::Bits24 => 1.0 / 8388608.0,
            BitDepth::Float32 => return,
        };
        let inv_lsb = 1.0 / lsb;

        // Lipshitz / Vanderkooy noise shaping coefficients (F-weighting curve)
        // H(z) = 1 - 2.033 z^-1 + 2.165 z^-2 - 1.959 z^-3 + 1.369 z^-4 - 0.468 z^-5
        const SHAPING_COEFFS: [f32; 5] = [2.033, -2.165, 1.959, -1.369, 0.468];

        let prng = &mut self.prng_state;
        let dtype = self.dither_type;

        if channels.len() > self.error_history.len() {
            self.error_history.resize(channels.len(), [0.0; 5]);
        }

        for (ch, buffer) in channels.iter_mut().enumerate() {
            let hist = &mut self.error_history[ch];

            for sample in buffer.iter_mut() {
                let dither_noise = match dtype {
                    DitherType::Tpdf | DitherType::NoiseShaped => Self::next_tpdf_state(prng) * lsb,
                    DitherType::None => 0.0,
                };

                let mut shaped_input = *sample;
                if dtype == DitherType::NoiseShaped {
                    let mut feedback = 0.0f32;
                    for i in 0..5 {
                        feedback += SHAPING_COEFFS[i] * hist[i];
                    }
                    shaped_input += feedback;
                }

                // Add dither noise and quantize to nearest integer LSB
                let quantized_lsb = ((shaped_input + dither_noise) * inv_lsb).round();
                let quantized_sample = (quantized_lsb * lsb).clamp(-1.0, 1.0 - lsb);

                // Calculate quantization error for feedback
                let error = shaped_input - quantized_sample;
                for i in (1..5).rev() {
                    hist[i] = hist[i - 1];
                }
                hist[0] = error;

                *sample = quantized_sample;
            }
        }
    }
}
