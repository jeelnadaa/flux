use super::{Ctx, Processor, P};
use crate::dsp::{db_to_lin, Biquad, Delay, FilterType, Lfo, OnePole, Rng};
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::Arc;

const N: usize = 8;
const BASE: [f32; N] = [
    1557.0, 1617.0, 1491.0, 1422.0, 1277.0, 1356.0, 1188.0, 1116.0,
];

/// Feedback delay network reverb (8 lines, Householder feedback). Decay is set by RT60 directly.
pub struct Reverb {
    sr: f32,
    lines: Vec<Delay>,
    len: [usize; N],
    g: [f32; N],
    damp: [OnePole; N],
    pre: Delay,
    pre_n: usize,
    lc: [Biquad; 2],
    width: f32,
    early: Delay,
    last: [f32; 7],
}
impl Reverb {
    pub fn new(sr: f32) -> Self {
        let lines = (0..N).map(|_| Delay::new((sr * 0.2) as usize)).collect();
        let mut r = Reverb {
            sr,
            lines,
            len: [1000; N],
            g: [0.0; N],
            damp: [OnePole::default(); N],
            pre: Delay::new((sr * 0.3) as usize),
            pre_n: 0,
            lc: [Biquad::default(); 2],
            width: 1.0,
            early: Delay::new((sr * 0.1) as usize),
            last: [f32::NAN; 7],
        };
        let mut p = [0.0; 32];
        p[..7].copy_from_slice(&[3.2, 24.0, 6000.0, 120.0, 100.0, 1.0, 0.0]);
        r.set(&p);
        r
    }
    pub fn set_rt(&mut self, rt60: f32, size: f32, damping: f32) {
        let k = self.sr / 44100.0 * size.clamp(0.3, 1.5);
        for (i, base) in BASE.iter().enumerate() {
            self.len[i] = ((base * k) as usize).max(16);
            // gain per pass so energy falls 60 dB in rt60 seconds
            self.g[i] = 10f32.powf(-3.0 * self.len[i] as f32 / (rt60.max(0.05) * self.sr));
            self.damp[i].set(self.sr, damping);
        }
    }
}
impl Processor for Reverb {
    fn set(&mut self, p: &P) {
        if p[..7] == self.last {
            return;
        }
        self.last.copy_from_slice(&p[..7]);
        self.set_rt(p[0], p[5], p[2]);
        self.pre_n = (p[1] * 0.001 * self.sr) as usize;
        for b in &mut self.lc {
            b.set(FilterType::HighPass, self.sr, p[3], 0.707, 0.0);
        }
        self.width = p[4] / 100.0;
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        let er = [(0.0043, 0.6), (0.0121, 0.5), (0.0197, 0.4), (0.0273, 0.3)];
        for i in 0..l.len() {
            let x = (l[i] + r[i]) * 0.5;
            self.pre.push(x);
            let xin = self.pre.tap(self.pre_n);
            self.early.push(xin);
            let mut o = [0.0f32; N];
            let mut sum = 0.0;
            for (k, ok) in o.iter_mut().enumerate() {
                *ok = self.damp[k].lp(self.lines[k].tap(self.len[k] - 1)) * self.g[k];
                sum += *ok;
            }
            let h = sum * (2.0 / N as f32);
            for (k, (line, ok)) in self.lines.iter_mut().zip(o).enumerate() {
                let inj = if k % 2 == 0 { xin } else { -xin };
                line.push(ok - h + inj * 0.35);
            }
            let mut e = 0.0;
            for (t, g) in er {
                e += self.early.tap((t * self.sr) as usize) * g;
            }
            let wl = o[0] + o[2] + o[4] + o[6] + e * 0.5;
            let wr = o[1] + o[3] + o[5] + o[7] + e * 0.5;
            let mid = (wl + wr) * 0.5;
            let side = (wl - wr) * 0.5 * self.width;
            l[i] = self.lc[0].run(mid + side) * 0.7;
            r[i] = self.lc[1].run(mid - side) * 0.7;
        }
    }
}

pub struct DelayFx {
    sr: f32,
    dl: Delay,
    dr: Delay,
    t: f32,
    fb: f32,
    ping: bool,
    tl: OnePole,
    tr: OnePole,
}
impl DelayFx {
    pub fn new(sr: f32) -> Self {
        DelayFx {
            sr,
            dl: Delay::new((sr * 2.1) as usize),
            dr: Delay::new((sr * 2.1) as usize),
            t: 0.35 * sr,
            fb: 0.35,
            ping: false,
            tl: OnePole::new(sr, 8000.0),
            tr: OnePole::new(sr, 8000.0),
        }
    }
}
impl Processor for DelayFx {
    fn set(&mut self, p: &P) {
        self.t = p[0] * 0.001 * self.sr;
        self.fb = p[1] / 100.0;
        self.ping = p[2] > 0.5;
        self.tl.set(self.sr, p[3]);
        self.tr.set(self.sr, p[3]);
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        for i in 0..l.len() {
            let yl = self.dl.frac(self.t - 1.0);
            let yr = self.dr.frac(self.t - 1.0);
            if self.ping {
                self.dl.push(self.tl.lp((l[i] + r[i]) * 0.5 + yr * self.fb));
                self.dr.push(self.tr.lp(yl * self.fb));
            } else {
                self.dl.push(self.tl.lp(l[i] + yl * self.fb));
                self.dr.push(self.tr.lp(r[i] + yr * self.fb));
            }
            l[i] = yl;
            r[i] = yr;
        }
    }
}

/// Uniformly partitioned FFT convolution with a loaded impulse response.
/// Partitioned spectra of one channel.
type Parts = Vec<Vec<Complex<f32>>>;

pub struct Convolver {
    ir: Option<(Parts, Parts)>,
    fft: Arc<dyn Fft<f32>>,
    ifft: Arc<dyn Fft<f32>>,
    hist: [Vec<Vec<Complex<f32>>>; 2],
    hp: usize,
    inbuf: [Vec<f32>; 2],
    outbuf: [Vec<f32>; 2],
    fill: usize,
    gain: f32,
    scratch: Vec<Complex<f32>>,
    acc: Vec<Complex<f32>>,
}
const B: usize = 512;
impl Convolver {
    pub fn new(ir: Option<(Vec<f32>, Vec<f32>)>) -> Self {
        let mut pl = FftPlanner::new();
        let fft = pl.plan_fft_forward(2 * B);
        let ifft = pl.plan_fft_inverse(2 * B);
        let parts = |x: &Vec<f32>| -> Vec<Vec<Complex<f32>>> {
            x.chunks(B)
                .map(|c| {
                    let mut v: Vec<Complex<f32>> = (0..2 * B)
                        .map(|i| Complex::new(if i < c.len() { c[i] } else { 0.0 }, 0.0))
                        .collect();
                    fft.process(&mut v);
                    v
                })
                .collect()
        };
        let irp = ir.map(|(a, b)| (parts(&a), parts(&b)));
        let np = irp.as_ref().map(|p| p.0.len()).unwrap_or(1);
        let z = vec![vec![Complex::new(0.0, 0.0); 2 * B]; np];
        Convolver {
            ir: irp,
            fft,
            ifft,
            hist: [z.clone(), z],
            hp: 0,
            inbuf: [vec![0.0; 2 * B], vec![0.0; 2 * B]],
            outbuf: [vec![0.0; B], vec![0.0; B]],
            fill: 0,
            gain: 1.0,
            scratch: vec![Complex::new(0.0, 0.0); 2 * B],
            acc: vec![Complex::new(0.0, 0.0); 2 * B],
        }
    }
}
impl Processor for Convolver {
    fn set(&mut self, p: &P) {
        self.gain = db_to_lin(p[0]);
    }
    fn latency(&self) -> usize {
        if self.ir.is_some() {
            B
        } else {
            0
        }
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        let Some(ir) = &self.ir else { return };
        let np = ir.0.len();
        for i in 0..l.len() {
            let x = [l[i], r[i]];
            l[i] = self.outbuf[0][self.fill] * self.gain;
            r[i] = self.outbuf[1][self.fill] * self.gain;
            self.inbuf[0][B + self.fill] = x[0];
            self.inbuf[1][B + self.fill] = x[1];
            self.fill += 1;
            if self.fill == B {
                self.fill = 0;
                self.hp = (self.hp + 1) % np;
                for ch in 0..2 {
                    for (k, s) in self.scratch.iter_mut().enumerate() {
                        *s = Complex::new(self.inbuf[ch][k], 0.0);
                    }
                    self.fft.process(&mut self.scratch);
                    self.hist[ch][self.hp].copy_from_slice(&self.scratch);
                    self.acc
                        .iter_mut()
                        .for_each(|c| *c = Complex::new(0.0, 0.0));
                    let parts = if ch == 0 { &ir.0 } else { &ir.1 };
                    for (pi, part) in parts.iter().enumerate() {
                        let h = &self.hist[ch][(self.hp + np - pi) % np];
                        for k in 0..2 * B {
                            self.acc[k] += h[k] * part[k];
                        }
                    }
                    self.ifft.process(&mut self.acc);
                    let s = 1.0 / (2 * B) as f32;
                    for k in 0..B {
                        self.outbuf[ch][k] = self.acc[B + k].re * s;
                    }
                    self.inbuf[ch].copy_within(B..2 * B, 0);
                }
            }
        }
    }
}

/// Row 10: the mix heard from a concert seat, with crowd and PA color.
pub struct Row10 {
    sr: f32,
    rev: Reverb,
    air: [OnePole; 2],
    pa: [Biquad; 2],
    crowd: f32,
    crowd_bp: [Biquad; 2],
    rng: Rng,
    lfo: Lfo,
    dist: f32,
    applause: bool,
    wet: Vec<f32>,
    wet2: Vec<f32>,
    seed: f32,
}
impl Row10 {
    pub fn new(sr: f32) -> Self {
        Row10 {
            sr,
            rev: Reverb::new(sr),
            air: [OnePole::default(); 2],
            pa: [Biquad::default(); 2],
            crowd: 0.1,
            crowd_bp: [Biquad::default(); 2],
            rng: Rng::new(10),
            lfo: Lfo::default(),
            dist: 0.5,
            applause: true,
            wet: vec![0.0; super::BLOCK],
            wet2: vec![0.0; super::BLOCK],
            seed: -1.0,
        }
    }
}
impl Processor for Row10 {
    fn set(&mut self, p: &P) {
        let row = p[0];
        self.dist = (row / 40.0).clamp(0.02, 1.0);
        self.rev.set_rt(2.2, 1.3, 7000.0 - 3500.0 * self.dist);
        for a in &mut self.air {
            a.set(self.sr, 16000.0 - 9000.0 * self.dist);
        }
        for b in &mut self.pa {
            b.set(FilterType::Bell, self.sr, 2500.0, 0.8, p[2] / 100.0 * 6.0);
        }
        self.crowd = db_to_lin(p[1]);
        for b in &mut self.crowd_bp {
            b.set(FilterType::BandPass, self.sr, 900.0, 0.6, 0.0);
        }
        self.applause = p[3] > 0.5;
        if p[4] != self.seed {
            self.seed = p[4];
            self.rng = Rng::new(p[4] as u64);
        }
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], ctx: &Ctx) {
        let n = l.len();
        self.wet[..n].copy_from_slice(&l[..n]);
        self.wet2[..n].copy_from_slice(&r[..n]);
        let (w1, w2) = (&mut self.wet[..n], &mut self.wet2[..n]);
        self.rev.process(w1, w2, ctx);
        let direct = 1.0 - self.dist * 0.6;
        let wetg = 0.3 + self.dist * 0.9;
        for i in 0..n {
            let t = (ctx.pos + i as i64) as f32 / self.sr;
            let end = ctx.len as f32 / self.sr;
            let clap = if self.applause && ctx.len > 0 {
                ((-(t / 2.5)).exp() + (-((end - t).max(0.0) / 2.5)).exp()).min(1.0)
            } else {
                0.0
            };
            let swell = 0.6 + 0.4 * self.lfo.next(self.sr, 0.07);
            let nz = self.rng.bi() * (self.crowd * swell + clap * 0.5 * self.rng.f().powi(6) * 4.0);
            let c1 = self.crowd_bp[0].run(nz);
            let c2 = self.crowd_bp[1].run(self.rng.bi() * self.crowd * swell);
            let a = self.pa[0].run(self.air[0].lp(l[i] * direct + w1[i] * wetg));
            let b = self.pa[1].run(self.air[1].lp(r[i] * direct + w2[i] * wetg));
            l[i] = a + c1;
            r[i] = b + c2;
        }
    }
}

/// Next door: the song through a neighbor's wall.
pub struct NextDoor {
    sr: f32,
    lp: [[Biquad; 2]; 2],
    bass: [Biquad; 2],
    rev: Reverb,
    drift: bool,
    lfo: Lfo,
    rng: Rng,
    dv: f32,
    wl: Vec<f32>,
    wr: Vec<f32>,
}
impl NextDoor {
    pub fn new(sr: f32) -> Self {
        NextDoor {
            sr,
            lp: [[Biquad::default(); 2]; 2],
            bass: [Biquad::default(); 2],
            rev: Reverb::new(sr),
            drift: true,
            lfo: Lfo::default(),
            rng: Rng::new(3),
            dv: 1.0,
            wl: vec![0.0; super::BLOCK],
            wr: vec![0.0; super::BLOCK],
        }
    }
}
impl Processor for NextDoor {
    fn set(&mut self, p: &P) {
        for ch in 0..2 {
            for k in 0..2 {
                self.lp[ch][k].set(FilterType::LowPass, self.sr, p[0], 0.6, 0.0);
            }
            self.bass[ch].set(FilterType::LowShelf, self.sr, 120.0, 0.7, p[1]);
        }
        self.rev.set_rt(p[2], 0.5, 3000.0);
        self.drift = p[3] > 0.5;
        self.rng = Rng::new(p[4] as u64);
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], ctx: &Ctx) {
        let n = l.len();
        for i in 0..n {
            let m = (l[i] + r[i]) * 0.5;
            let a = self.bass[0].run({
                let t_ = self.lp[0][0].run(m);
                self.lp[0][1].run(t_)
            });
            let b = self.bass[1].run({
                let t_ = self.lp[1][0].run(m);
                self.lp[1][1].run(t_)
            });
            self.wl[i] = a;
            self.wr[i] = b;
            l[i] = a;
            r[i] = b;
        }
        let (wl, wr) = (&mut self.wl[..n], &mut self.wr[..n]);
        self.rev.process(wl, wr, ctx);
        for i in 0..n {
            if self.drift && i % 64 == 0 {
                self.dv = 0.8 + 0.2 * self.lfo.next(self.sr / 64.0, 0.05) + 0.02 * self.rng.bi();
            }
            let g = if self.drift { self.dv } else { 1.0 } * 0.6;
            l[i] = (l[i] + wl[i] * 0.4) * g;
            r[i] = (r[i] + wr[i] * 0.4) * g;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Measured RT60 of the impulse response is within 20% of the requested decay.
    #[test]
    fn reverb_rt60_matches_setting() {
        let sr = 48000.0;
        for rt in [0.9f32, 3.2] {
            let mut rv = Reverb::new(sr);
            let mut p = [0.0; 32];
            p[..7].copy_from_slice(&[rt, 0.0, 20000.0, 20.0, 100.0, 1.0, 0.0]);
            rv.set(&p);
            let n = (sr * (rt * 1.6 + 0.5)) as usize;
            let mut l = vec![0.0f32; n];
            l[0] = 1.0;
            let mut r = l.clone();
            for (a, b) in l.chunks_mut(512).zip(r.chunks_mut(512)) {
                rv.process(
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
            // Schroeder backward integration, fit -5..-25 dB, extrapolate to 60 dB
            let e: Vec<f64> = l
                .iter()
                .zip(&r)
                .map(|(a, b)| (a * a + b * b) as f64)
                .collect();
            let mut cum = vec![0.0f64; n];
            let mut s = 0.0;
            for i in (0..n).rev() {
                s += e[i];
                cum[i] = s;
            }
            let tot = cum[0];
            let t_at = |db: f64| {
                cum.iter()
                    .position(|c| 10.0 * (c / tot).log10() < db)
                    .unwrap() as f32
                    / sr
            };
            let measured = (t_at(-25.0) - t_at(-5.0)) * 3.0;
            assert!(
                (measured - rt).abs() / rt < 0.2,
                "rt {rt}: measured {measured}"
            );
        }
    }
}
