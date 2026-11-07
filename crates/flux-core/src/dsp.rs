//! DSP building blocks shared by every effect. All state is preallocated in `new`;
//! `process` paths never allocate.

use std::f32::consts::PI;

pub const TAU: f32 = 2.0 * PI;

#[inline]
pub fn db_to_lin(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}
#[inline]
pub fn lin_to_db(l: f32) -> f32 {
    20.0 * l.max(1e-12).log10()
}
#[inline]
pub fn flush(x: f32) -> f32 {
    if x.abs() < 1e-20 {
        0.0
    } else {
        x
    }
}

/// Deterministic PRNG (SplitMix64). Every random effect is seeded from the project.
#[derive(Clone, Debug)]
pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in [0, 1).
    #[inline]
    pub fn f(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    /// Uniform in [-1, 1).
    #[inline]
    pub fn bi(&mut self) -> f32 {
        self.f() * 2.0 - 1.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FilterType {
    LowPass,
    HighPass,
    BandPass,
    Notch,
    Bell,
    LowShelf,
    HighShelf,
    AllPass,
}

/// Two biquads in series.
#[inline]
pub fn run2(f: &mut [Biquad; 2], x: f32) -> f32 {
    let t = f[0].run(x);
    f[1].run(t)
}

/// RBJ cookbook biquad, transposed direct form II.
#[derive(Clone, Copy, Debug)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}
impl Default for Biquad {
    fn default() -> Self {
        Biquad {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            z1: 0.0,
            z2: 0.0,
        }
    }
}
impl Biquad {
    /// Normalized coefficients (a0 = 1).
    pub fn from_coefs(b0: f32, b1: f32, b2: f32, a1: f32, a2: f32) -> Self {
        Biquad {
            b0,
            b1,
            b2,
            a1,
            a2,
            z1: 0.0,
            z2: 0.0,
        }
    }
    pub fn new(t: FilterType, sr: f32, f: f32, q: f32, gain_db: f32) -> Self {
        let mut b = Biquad::default();
        b.set(t, sr, f, q, gain_db);
        b
    }
    pub fn set(&mut self, t: FilterType, sr: f32, f: f32, q: f32, gain_db: f32) {
        let f = f.clamp(5.0, sr * 0.49);
        let q = q.max(0.05);
        let w = TAU * f / sr;
        let (s, c) = w.sin_cos();
        let alpha = s / (2.0 * q);
        let a = 10f32.powf(gain_db / 40.0);
        let (b0, b1, b2, a0, a1, a2) = match t {
            FilterType::LowPass => (
                (1.0 - c) / 2.0,
                1.0 - c,
                (1.0 - c) / 2.0,
                1.0 + alpha,
                -2.0 * c,
                1.0 - alpha,
            ),
            FilterType::HighPass => (
                (1.0 + c) / 2.0,
                -(1.0 + c),
                (1.0 + c) / 2.0,
                1.0 + alpha,
                -2.0 * c,
                1.0 - alpha,
            ),
            FilterType::BandPass => (alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * c, 1.0 - alpha),
            FilterType::Notch => (1.0, -2.0 * c, 1.0, 1.0 + alpha, -2.0 * c, 1.0 - alpha),
            FilterType::AllPass => (
                1.0 - alpha,
                -2.0 * c,
                1.0 + alpha,
                1.0 + alpha,
                -2.0 * c,
                1.0 - alpha,
            ),
            FilterType::Bell => (
                1.0 + alpha * a,
                -2.0 * c,
                1.0 - alpha * a,
                1.0 + alpha / a,
                -2.0 * c,
                1.0 - alpha / a,
            ),
            FilterType::LowShelf => {
                let sq = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) - (a - 1.0) * c + sq),
                    2.0 * a * ((a - 1.0) - (a + 1.0) * c),
                    a * ((a + 1.0) - (a - 1.0) * c - sq),
                    (a + 1.0) + (a - 1.0) * c + sq,
                    -2.0 * ((a - 1.0) + (a + 1.0) * c),
                    (a + 1.0) + (a - 1.0) * c - sq,
                )
            }
            FilterType::HighShelf => {
                let sq = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) + (a - 1.0) * c + sq),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * c),
                    a * ((a + 1.0) + (a - 1.0) * c - sq),
                    (a + 1.0) - (a - 1.0) * c + sq,
                    2.0 * ((a - 1.0) - (a + 1.0) * c),
                    (a + 1.0) - (a - 1.0) * c - sq,
                )
            }
        };
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }
    #[inline]
    pub fn run(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = flush(self.b1 * x - self.a1 * y + self.z2);
        self.z2 = flush(self.b2 * x - self.a2 * y);
        y
    }
    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
    /// Magnitude response in dB at frequency `f`.
    pub fn response_db(&self, sr: f32, f: f32) -> f32 {
        let w = TAU * f / sr;
        let (c1, s1) = (w.cos(), -w.sin());
        let (c2, s2) = ((2.0 * w).cos(), -(2.0 * w).sin());
        let nr = self.b0 + self.b1 * c1 + self.b2 * c2;
        let ni = self.b1 * s1 + self.b2 * s2;
        let dr = 1.0 + self.a1 * c1 + self.a2 * c2;
        let di = self.a1 * s1 + self.a2 * s2;
        10.0 * ((nr * nr + ni * ni) / (dr * dr + di * di).max(1e-30))
            .max(1e-30)
            .log10()
    }
}

/// One-pole low-pass, used for smoothing and gentle tone.
#[derive(Clone, Copy, Debug, Default)]
pub struct OnePole {
    a: f32,
    z: f32,
}
impl OnePole {
    pub fn new(sr: f32, f: f32) -> Self {
        let mut o = OnePole::default();
        o.set(sr, f);
        o
    }
    pub fn set(&mut self, sr: f32, f: f32) {
        self.a = (-TAU * f.clamp(1.0, sr * 0.49) / sr).exp();
    }
    #[inline]
    pub fn lp(&mut self, x: f32) -> f32 {
        self.z = flush(x + self.a * (self.z - x));
        self.z
    }
    #[inline]
    pub fn hp(&mut self, x: f32) -> f32 {
        x - self.lp(x)
    }
}

/// Smoothed parameter (one-pole, ~`ms` time constant). Avoids zipper noise.
#[derive(Clone, Copy, Debug)]
pub struct Smooth {
    pub v: f32,
    pub target: f32,
    a: f32,
}
impl Smooth {
    pub fn new(sr: f32, ms: f32, v: f32) -> Self {
        Smooth {
            v,
            target: v,
            a: (-1.0 / (sr * ms * 0.001).max(1.0)).exp(),
        }
    }
    #[inline]
    pub fn tick(&mut self) -> f32 {
        self.v = self.target + self.a * (self.v - self.target);
        self.v
    }
}

/// Fractional delay line with linear and cubic reads.
#[derive(Clone, Debug)]
pub struct Delay {
    buf: Vec<f32>,
    w: usize,
}
impl Delay {
    pub fn new(max: usize) -> Self {
        Delay {
            buf: vec![0.0; max.max(4).next_power_of_two()],
            w: 0,
        }
    }
    #[inline]
    pub fn push(&mut self, x: f32) {
        let m = self.buf.len() - 1;
        self.w = (self.w + 1) & m;
        self.buf[self.w] = x;
    }
    /// Integer delay `d` samples (0 = most recent).
    #[inline]
    pub fn tap(&self, d: usize) -> f32 {
        let m = self.buf.len() - 1;
        self.buf[(self.w.wrapping_sub(d)) & m]
    }
    #[inline]
    pub fn frac(&self, d: f32) -> f32 {
        let d = d.clamp(0.0, (self.buf.len() - 3) as f32);
        let i = d as usize;
        let t = d - i as f32;
        let a = self.tap(i);
        let b = self.tap(i + 1);
        a + (b - a) * t
    }
    pub fn len(&self) -> usize {
        self.buf.len()
    }
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
    pub fn clear(&mut self) {
        self.buf.iter_mut().for_each(|x| *x = 0.0);
    }
}

/// Peak/RMS envelope follower with separate attack and release.
#[derive(Clone, Copy, Debug)]
pub struct Env {
    pub v: f32,
    att: f32,
    rel: f32,
}
impl Env {
    pub fn new(sr: f32, att_ms: f32, rel_ms: f32) -> Self {
        let mut e = Env {
            v: 0.0,
            att: 0.0,
            rel: 0.0,
        };
        e.set(sr, att_ms, rel_ms);
        e
    }
    pub fn set(&mut self, sr: f32, att_ms: f32, rel_ms: f32) {
        self.att = (-1.0 / (sr * att_ms.max(0.01) * 0.001)).exp();
        self.rel = (-1.0 / (sr * rel_ms.max(0.1) * 0.001)).exp();
    }
    #[inline]
    pub fn run(&mut self, x: f32) -> f32 {
        let c = if x > self.v { self.att } else { self.rel };
        self.v = flush(x + c * (self.v - x));
        self.v
    }
}

/// Sine LFO.
#[derive(Clone, Copy, Debug, Default)]
pub struct Lfo {
    pub ph: f32,
}
impl Lfo {
    #[inline]
    pub fn next(&mut self, sr: f32, hz: f32) -> f32 {
        let v = (self.ph * TAU).sin();
        self.ph += hz / sr;
        if self.ph >= 1.0 {
            self.ph -= 1.0;
        }
        v
    }
}

/// Periodic Hann window.
pub fn hann(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| 0.5 - 0.5 * (TAU * i as f32 / n as f32).cos())
        .collect()
}

/// Offline windowed-sinc resampler (Kaiser-ish window). Used once at import.
pub fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let ratio = to as f64 / from as f64;
    let out_len = (input.len() as f64 * ratio).round() as usize;
    let cutoff = ratio.min(1.0) * 0.95;
    let half = 24i64;
    let mut out = vec![0.0f32; out_len];
    for (j, o) in out.iter_mut().enumerate() {
        let pos = j as f64 / ratio;
        let c = pos.floor() as i64;
        let mut acc = 0.0f64;
        let mut wsum = 0.0f64;
        for k in (c - half + 1)..=(c + half) {
            let x = pos - k as f64;
            let sinc = if x.abs() < 1e-9 {
                1.0
            } else {
                (std::f64::consts::PI * x * cutoff).sin() / (std::f64::consts::PI * x) * 1.0
                    / cutoff
            };
            let t = x / half as f64;
            let win = if t.abs() >= 1.0 {
                0.0
            } else {
                (0.42
                    + 0.5 * (std::f64::consts::PI * t).cos()
                    + 0.08 * (2.0 * std::f64::consts::PI * t).cos())
                .max(0.0)
            };
            let w = sinc * win * cutoff;
            wsum += w;
            if k >= 0 && (k as usize) < input.len() {
                acc += input[k as usize] as f64 * w;
            }
        }
        *o = if wsum.abs() > 1e-9 {
            (acc / wsum) as f32
        } else {
            0.0
        };
    }
    out
}

/// Cubic Hermite interpolation of four points at fraction `t`.
#[inline]
pub fn hermite(xm1: f32, x0: f32, x1: f32, x2: f32, t: f32) -> f32 {
    let c = (x1 - xm1) * 0.5;
    let v = x0 - x1;
    let w = c + v;
    let a = w + v + (x2 - x0) * 0.5;
    let b = w + a;
    ((a * t - b) * t + c) * t + x0
}

/// Equal-power pan gains for pan in [-1, 1].
#[inline]
pub fn pan_gains(pan: f32) -> (f32, f32) {
    if pan == 0.0 {
        return (1.0, 1.0);
    }
    let a = (pan.clamp(-1.0, 1.0) + 1.0) * 0.25 * PI;
    (
        a.cos() * std::f32::consts::SQRT_2,
        a.sin() * std::f32::consts::SQRT_2,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn biquad_bell_gain_matches_response() {
        let b = Biquad::new(FilterType::Bell, 48000.0, 1000.0, 1.0, 6.0);
        assert!((b.response_db(48000.0, 1000.0) - 6.0).abs() < 0.05);
        assert!(b.response_db(48000.0, 50.0).abs() < 0.2);
    }
    #[test]
    fn resample_preserves_sine_frequency_and_level() {
        let sr = 44100.0;
        let x: Vec<f32> = (0..44100)
            .map(|i| (TAU * 1000.0 * i as f32 / sr).sin() * 0.5)
            .collect();
        let y = resample(&x, 44100, 48000);
        assert_eq!(y.len(), 48000);
        let rms = (y[2000..46000].iter().map(|v| v * v).sum::<f32>() / 44000.0).sqrt();
        assert!((rms - 0.5 / 2f32.sqrt()).abs() < 0.005, "rms {rms}");
        // zero crossings ≈ 2 per cycle
        let zc = y[1000..47000]
            .windows(2)
            .filter(|w| w[0] <= 0.0 && w[1] > 0.0)
            .count();
        assert!((zc as i32 - 958).abs() <= 2, "zc {zc}");
    }
    #[test]
    fn rng_is_deterministic() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }
}
