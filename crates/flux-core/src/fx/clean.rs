use super::{Ctx, Processor, P};
use crate::dsp::{db_to_lin, hann, Biquad, FilterType};
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::Arc;

pub const NR_N: usize = 2048;

/// Streaming STFT with 75% overlap and weighted overlap-add. Latency = `n` samples.
pub struct Stft {
    pub n: usize,
    hop: usize,
    win: Vec<f32>,
    inp: Vec<f32>,
    out: Vec<f32>,
    pos: usize,
    fft: Arc<dyn Fft<f32>>,
    ifft: Arc<dyn Fft<f32>>,
    pub frame: Vec<Complex<f32>>,
    norm: f32,
}
impl Stft {
    pub fn new(n: usize) -> Self {
        let mut pl = FftPlanner::new();
        let hop = n / 4;
        Stft {
            n,
            hop,
            win: hann(n),
            inp: vec![0.0; n],
            out: vec![0.0; n + hop],
            pos: 0,
            fft: pl.plan_fft_forward(n),
            ifft: pl.plan_fft_inverse(n),
            frame: vec![Complex::new(0.0, 0.0); n],
            norm: 1.0 / (n as f32 * 1.5),
        }
    }
    /// Push one sample, get one (delayed) sample. `f` edits the spectrum of each frame.
    #[inline]
    pub fn run<F: FnMut(&mut [Complex<f32>])>(&mut self, x: f32, f: &mut F) -> f32 {
        let y = self.out[self.pos];
        self.inp[self.pos + self.n - self.hop] = x;
        self.pos += 1;
        if self.pos == self.hop {
            self.pos = 0;
            for i in 0..self.n {
                self.frame[i] = Complex::new(self.inp[i] * self.win[i], 0.0);
            }
            self.fft.process(&mut self.frame);
            f(&mut self.frame);
            self.ifft.process(&mut self.frame);
            self.out.copy_within(self.hop.., 0);
            let l = self.out.len();
            self.out[l - self.hop..].iter_mut().for_each(|v| *v = 0.0);
            for i in 0..self.n {
                self.out[i] += self.frame[i].re * self.win[i] * self.norm;
            }
            self.inp.copy_within(self.hop.., 0);
        }
        y
    }
}

/// Learn an average noise power spectrum from a mono excerpt.
pub fn learn_profile(x: &[f32]) -> Vec<f32> {
    let n = NR_N;
    let mut pl = FftPlanner::new();
    let fft = pl.plan_fft_forward(n);
    let w = hann(n);
    let mut acc = vec![0.0f32; n / 2 + 1];
    let mut frames = 0;
    let mut i = 0;
    while i + n <= x.len() {
        let mut fr: Vec<Complex<f32>> =
            (0..n).map(|k| Complex::new(x[i + k] * w[k], 0.0)).collect();
        fft.process(&mut fr);
        for k in 0..=n / 2 {
            acc[k] += fr[k].norm_sqr();
        }
        frames += 1;
        i += n / 4;
    }
    if frames > 0 {
        acc.iter_mut().for_each(|v| *v /= frames as f32);
    }
    acc
}

/// Spectral noise reduction: Wiener-style gains against a learned or tracked noise floor.
pub struct NoiseReduce {
    st: [Stft; 2],
    noise: [Vec<f32>; 2],
    learned: bool,
    pw: [Vec<f32>; 2],
    g: [Vec<f32>; 2],
    over: f32,
    gmin: f32,
    smooth: f32,
    listen: bool,
}
impl NoiseReduce {
    pub fn new(sr: f32, profile: Option<Vec<f32>>) -> Self {
        let h = NR_N / 2 + 1;
        let learned = profile.as_ref().map(|p| p.len() == h).unwrap_or(false);
        let prof = if learned {
            profile.unwrap()
        } else {
            vec![1e-9; h]
        };
        let _ = sr;
        NoiseReduce {
            st: [Stft::new(NR_N), Stft::new(NR_N)],
            noise: [prof.clone(), prof],
            learned,
            pw: [vec![0.0; h], vec![0.0; h]],
            g: [vec![1.0; h], vec![1.0; h]],
            over: 2.0,
            gmin: 0.1,
            smooth: 0.5,
            listen: false,
        }
    }
}
impl Processor for NoiseReduce {
    fn set(&mut self, p: &P) {
        self.gmin = db_to_lin((-p[0]).max(p[3]));
        self.over = 1.0 + p[1] / 100.0 * 4.0;
        // frames are 512 samples apart at 48 kHz (~10.7 ms)
        self.smooth = (-10.7 / p[2].max(1.0)).exp();
        self.listen = p[4] > 0.5;
    }
    fn latency(&self) -> usize {
        NR_N
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        let h = NR_N / 2 + 1;
        for (ch, buf) in [l, r].into_iter().enumerate() {
            let (noise, pw, g, learned, over, gmin, sm, listen) = (
                &mut self.noise[ch],
                &mut self.pw[ch],
                &mut self.g[ch],
                self.learned,
                self.over,
                self.gmin,
                self.smooth,
                self.listen,
            );
            let mut f = |fr: &mut [Complex<f32>]| {
                for k in 0..h {
                    let p = fr[k].norm_sqr();
                    pw[k] = 0.7 * pw[k] + 0.3 * p;
                    if !learned {
                        // minimum tracking with slow upward drift
                        noise[k] = if pw[k] < noise[k] {
                            pw[k]
                        } else {
                            noise[k] * 1.002 + 1e-12
                        };
                    }
                    let want = (1.0 - over * noise[k] / pw[k].max(1e-12)).max(gmin);
                    g[k] = if want > g[k] {
                        want
                    } else {
                        sm * g[k] + (1.0 - sm) * want
                    };
                    let gg = if listen { 1.0 - g[k] } else { g[k] };
                    fr[k] *= gg;
                    if k > 0 && k < NR_N / 2 {
                        fr[NR_N - k] *= gg;
                    }
                }
            };
            for x in buf.iter_mut() {
                *x = self.st[ch].run(*x, &mut f);
            }
        }
    }
}

/// Mains hum removal: notches at the fundamental and harmonics. Auto mode decides 50 or 60 Hz
/// from the first second of input.
pub struct Hum {
    sr: f32,
    mode: i32,
    harm: usize,
    q: f32,
    f0: f32,
    nl: [Biquad; 10],
    nr: [Biquad; 10],
    nn: usize,
    g50: (f32, f32, f32),
    g60: (f32, f32, f32),
    seen: usize,
}
impl Hum {
    pub fn new(sr: f32) -> Self {
        let mut h = Hum {
            sr,
            mode: 0,
            harm: 6,
            q: 30.0,
            f0: 50.0,
            nl: [Biquad::default(); 10],
            nr: [Biquad::default(); 10],
            nn: 0,
            g50: (0.0, 0.0, 0.0),
            g60: (0.0, 0.0, 0.0),
            seen: 0,
        };
        h.rebuild();
        h
    }
    fn rebuild(&mut self) {
        // fixed storage: rebuilding happens on the audio thread when params change
        self.nn = 0;
        for k in 1..=self.harm.min(10) {
            let f = self.f0 * k as f32;
            if f < self.sr * 0.45 {
                self.nl[self.nn] = Biquad::new(FilterType::Notch, self.sr, f, self.q, 0.0);
                self.nr[self.nn] = self.nl[self.nn];
                self.nn += 1;
            }
        }
    }
}
fn goertzel(s: &mut (f32, f32, f32), x: f32, coef: f32) {
    let s0 = x + coef * s.0 - s.1;
    s.1 = s.0;
    s.0 = s0;
    s.2 += x * x;
}
impl Processor for Hum {
    fn set(&mut self, p: &P) {
        let mode = p[0].round() as i32;
        let harm = p[1].round().clamp(1.0, 10.0) as usize;
        if mode != self.mode || harm != self.harm || p[2] != self.q {
            self.mode = mode;
            self.harm = harm;
            self.q = p[2];
            if mode == 1 {
                self.f0 = 50.0;
            } else if mode == 2 {
                self.f0 = 60.0;
            }
            self.rebuild();
        }
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        if self.mode == 0 && self.seen < self.sr as usize {
            let c50 = 2.0 * (crate::dsp::TAU * 50.0 / self.sr).cos();
            let c60 = 2.0 * (crate::dsp::TAU * 60.0 / self.sr).cos();
            for i in 0..l.len() {
                let m = (l[i] + r[i]) * 0.5;
                goertzel(&mut self.g50, m, c50);
                goertzel(&mut self.g60, m, c60);
            }
            self.seen += l.len();
            if self.seen >= self.sr as usize {
                let p = |s: (f32, f32, f32), c: f32| s.0 * s.0 + s.1 * s.1 - c * s.0 * s.1;
                self.f0 = if p(self.g60, c60) > p(self.g50, c50) {
                    60.0
                } else {
                    50.0
                };
                self.rebuild();
            }
        }
        for i in 0..l.len() {
            for f in &mut self.nl[..self.nn] {
                l[i] = f.run(l[i]);
            }
            for f in &mut self.nr[..self.nn] {
                r[i] = f.run(r[i]);
            }
        }
    }
}

const LA: usize = 64;
/// Shared look-ahead buffer for repair processors.
struct Ring {
    b: [f32; 256],
    w: usize,
}
impl Ring {
    fn new() -> Self {
        Ring {
            b: [0.0; 256],
            w: 0,
        }
    }
    #[inline]
    fn at(&self, back: usize) -> f32 {
        self.b[(self.w + 256 - back) & 255]
    }
    #[inline]
    fn set(&mut self, back: usize, v: f32) {
        self.b[(self.w + 256 - back) & 255] = v;
    }
    #[inline]
    fn push(&mut self, x: f32) {
        self.w = (self.w + 1) & 255;
        self.b[self.w] = x;
    }
}

pub struct DeClick {
    rings: [Ring; 2],
    rms: [f32; 2],
    k: f32,
    a: f32,
}
impl DeClick {
    pub fn new(sr: f32) -> Self {
        DeClick {
            rings: [Ring::new(), Ring::new()],
            rms: [1e-4; 2],
            k: 10.0,
            a: (-1.0 / (sr * 0.005)).exp(),
        }
    }
}
impl Processor for DeClick {
    fn set(&mut self, p: &P) {
        self.k = 16.0 - p[0] / 100.0 * 12.0;
    }
    fn latency(&self) -> usize {
        LA
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        for (ch, buf) in [l, r].into_iter().enumerate() {
            let rg = &mut self.rings[ch];
            for x in buf.iter_mut() {
                rg.push(*x);
                // examine the sample 16 back so the repair window has context on both sides
                let d = rg.at(16) - 2.0 * rg.at(17) + rg.at(18);
                let ad = d.abs();
                if ad > self.k * self.rms[ch].sqrt() * 2.0 && ad > 0.02 {
                    let (a0, a1, b0, b1) = (rg.at(26), rg.at(25), rg.at(8), rg.at(7));
                    for j in 0..16 {
                        let t = (j + 1) as f32 / 17.0;
                        rg.set(24 - j, crate::dsp::hermite(a0, a1, b0, b1, t));
                    }
                } else {
                    self.rms[ch] = self.a * self.rms[ch] + (1.0 - self.a) * d * d;
                }
                *x = rg.at(LA);
            }
        }
    }
}

pub struct DeClip {
    rings: [Ring; 2],
    th: f32,
}
impl DeClip {
    pub fn new() -> Self {
        DeClip {
            rings: [Ring::new(), Ring::new()],
            th: 0.98,
        }
    }
}
impl Default for DeClip {
    fn default() -> Self {
        Self::new()
    }
}
impl Processor for DeClip {
    fn set(&mut self, p: &P) {
        self.th = p[0] / 100.0;
    }
    fn latency(&self) -> usize {
        LA
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        for (ch, buf) in [l, r].into_iter().enumerate() {
            let rg = &mut self.rings[ch];
            for x in buf.iter_mut() {
                rg.push(*x);
                // a clipped run ends at index 2 back when 1 back is below threshold
                if rg.at(1).abs() < self.th && rg.at(2).abs() >= self.th {
                    let mut len = 0;
                    while len < 40 && rg.at(2 + len).abs() >= self.th {
                        len += 1;
                    }
                    if (2..40).contains(&len) {
                        // cubic through two points either side of the run
                        let s = 2 + len;
                        let pts = [
                            (-2.0f32, rg.at(s + 1)),
                            (-1.0, rg.at(s)),
                            (len as f32, rg.at(1)),
                            (len as f32 + 1.0, rg.at(0)),
                        ];
                        for j in 0..len {
                            let t = j as f32;
                            let mut v = 0.0;
                            for (a, (xa, ya)) in pts.iter().enumerate() {
                                let mut w = 1.0;
                                for (b, (xb, _)) in pts.iter().enumerate() {
                                    if a != b && (xa - xb).abs() > 1e-6 {
                                        w *= (t - xb) / (xa - xb);
                                    }
                                }
                                v += ya * w;
                            }
                            let orig = rg.at(s - 1 - j);
                            if v.abs() > orig.abs() && v.signum() == orig.signum() {
                                rg.set(s - 1 - j, v.clamp(-2.0, 2.0));
                            }
                        }
                    }
                }
                *x = rg.at(LA);
            }
        }
    }
}

pub struct DeWind {
    sr: f32,
    hp: [[Biquad; 2]; 2],
}
impl DeWind {
    pub fn new(sr: f32) -> Self {
        DeWind {
            sr,
            hp: [[Biquad::default(); 2]; 2],
        }
    }
}
impl Processor for DeWind {
    fn set(&mut self, p: &P) {
        let f = 40.0 + p[0] * 2.0;
        for ch in &mut self.hp {
            for b in ch.iter_mut() {
                b.set(
                    FilterType::HighPass,
                    self.sr,
                    f,
                    std::f32::consts::FRAC_1_SQRT_2,
                    0.0,
                );
            }
        }
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        for i in 0..l.len() {
            l[i] = {
                let t_ = self.hp[0][0].run(l[i]);
                self.hp[0][1].run(t_)
            };
            r[i] = {
                let t_ = self.hp[1][0].run(r[i]);
                self.hp[1][1].run(t_)
            };
        }
    }
}

#[derive(Default)]
pub struct Stereo {
    width: f32,
    bl: f32,
    br: f32,
    swap: bool,
    mono: bool,
    il: f32,
    ir: f32,
}
impl Processor for Stereo {
    fn set(&mut self, p: &P) {
        self.width = p[0] / 100.0;
        let b = p[1] / 100.0;
        self.bl = (1.0 - b).min(1.0);
        self.br = (1.0 + b).min(1.0);
        self.swap = p[2] > 0.5;
        self.mono = p[3] > 0.5;
        self.il = if p[4] > 0.5 { -1.0 } else { 1.0 };
        self.ir = if p[5] > 0.5 { -1.0 } else { 1.0 };
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        for i in 0..l.len() {
            let (mut a, mut b) = (l[i] * self.il, r[i] * self.ir);
            if self.swap {
                std::mem::swap(&mut a, &mut b);
            }
            let m = (a + b) * 0.5;
            let s = if self.mono {
                0.0
            } else {
                (a - b) * 0.5 * self.width
            };
            l[i] = (m + s) * self.bl;
            r[i] = (m - s) * self.br;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stft_passthrough_is_transparent() {
        let mut st = Stft::new(1024);
        let x: Vec<f32> = (0..20000).map(|i| (i as f32 * 0.01).sin()).collect();
        let y: Vec<f32> = x.iter().map(|v| st.run(*v, &mut |_| {})).collect();
        for i in 5000..19000 {
            assert!(
                (y[i] - x[i - 1024]).abs() < 1e-3,
                "at {i}: {} vs {}",
                y[i],
                x[i - 1024]
            );
        }
    }
    #[test]
    fn noise_reduction_lowers_noise_and_keeps_tone() {
        let sr = 48000.0;
        let mut rng = crate::dsp::Rng::new(4);
        let n = 48000 * 3;
        let tone: Vec<f32> = (0..n)
            .map(|i| (crate::dsp::TAU * 1000.0 * i as f32 / sr).sin() * 0.3)
            .collect();
        let noise: Vec<f32> = (0..n).map(|_| rng.bi() * 0.03).collect();
        let prof = learn_profile(&noise[..48000]);
        let mut nr = NoiseReduce::new(sr, Some(prof));
        let mut p = [0.0; 32];
        p[..5].copy_from_slice(&[24.0, 45.0, 50.0, -60.0, 0.0]);
        nr.set(&p);
        let run = |nr: &mut NoiseReduce, x: &[f32]| {
            let mut l = x.to_vec();
            let mut r = x.to_vec();
            for (a, b) in l.chunks_mut(512).zip(r.chunks_mut(512)) {
                nr.process(
                    a,
                    b,
                    &Ctx {
                        sr,
                        pos: 0,
                        len: 0,
                        tempo: 120.0,
                    },
                );
            }
            l
        };
        let only_noise = run(&mut nr, &noise);
        let rms = |v: &[f32]| (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt();
        let red = crate::dsp::lin_to_db(rms(&only_noise[48000..]) / rms(&noise[48000..]));
        assert!(red < -12.0, "noise only reduced by {red} dB");
        let mut nr2 = NoiseReduce::new(sr, Some(learn_profile(&noise[..48000])));
        nr2.set(&p);
        let mix: Vec<f32> = tone.iter().zip(&noise).map(|(a, b)| a + b).collect();
        let out = run(&mut nr2, &mix);
        let keep = crate::dsp::lin_to_db(rms(&out[60000..]) / rms(&tone[60000..]));
        assert!(keep.abs() < 1.0, "tone level changed by {keep} dB");
    }
}
