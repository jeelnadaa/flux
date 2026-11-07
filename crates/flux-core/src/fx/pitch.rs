use super::clean::Stft;
use super::{Ctx, Processor, P};
use crate::dsp::{Biquad, Delay, Env, FilterType, Lfo, Rng, TAU};
use rustfft::num_complex::Complex;

const N: usize = 2048;
const H: usize = N / 2 + 1;
const HOP: usize = N / 4;

/// Per-channel phase vocoder state.
struct Pv {
    st: Stft,
    s: PvState,
}
struct PvState {
    last: Vec<f32>,
    sum: Vec<f32>,
    mag: Vec<f32>,
    frq: Vec<f32>,
    smag: Vec<f32>,
    sfrq: Vec<f32>,
    env: Vec<f32>,
    lg: Vec<f32>,
}
impl Pv {
    fn new() -> Self {
        Pv {
            st: Stft::new(N),
            s: PvState {
                last: vec![0.0; H],
                sum: vec![0.0; H],
                mag: vec![0.0; H],
                frq: vec![0.0; H],
                smag: vec![0.0; H],
                sfrq: vec![0.0; H],
                env: vec![0.0; H],
                lg: vec![0.0; H],
            },
        }
    }
}

fn smooth_env(mag: &[f32], env: &mut [f32], lg: &mut [f32]) {
    // log-magnitude box smoothing (~16 bins): a cheap spectral envelope
    const W: usize = 8;
    let mut acc = 0.0f32;
    for (o, m) in lg.iter_mut().zip(mag) {
        *o = (m + 1e-9).ln();
    }
    for v in lg.iter().take(W) {
        acc += v;
    }
    let n = mag.len();
    for k in 0..n {
        if k + W < n {
            acc += lg[k + W];
        }
        if k > W {
            acc -= lg[k - W - 1];
        }
        let cnt = (k + W).min(n - 1) - k.saturating_sub(W) + 1;
        env[k] = (acc / cnt as f32).exp();
    }
}

/// Analysis/synthesis of one frame. `ratio` shifts pitch, `fratio` shifts formants,
/// `keep` keeps the original envelope (formant preservation).
fn shift_frame(pv: &mut PvState, fr: &mut [Complex<f32>], ratio: f32, fratio: f32, keep: bool) {
    let expct = TAU * HOP as f32 / N as f32;
    for (k, c) in fr.iter().enumerate().take(H) {
        let (m, ph) = (c.norm(), c.arg());
        let mut d = ph - pv.last[k] - k as f32 * expct;
        pv.last[k] = ph;
        d -= TAU * (d / TAU).round();
        pv.mag[k] = m;
        pv.frq[k] = k as f32 + d / expct;
    }
    if keep || (fratio - 1.0).abs() > 1e-3 {
        smooth_env(&pv.mag, &mut pv.env, &mut pv.lg);
    }
    pv.smag.iter_mut().for_each(|v| *v = 0.0);
    pv.sfrq.iter_mut().for_each(|v| *v = 0.0);
    for k in 0..H {
        let j = (k as f32 * ratio).round() as usize;
        if j < H {
            let mut m = pv.mag[k];
            if keep || (fratio - 1.0).abs() > 1e-3 {
                // whiten by the source envelope at k, then apply the envelope wanted at j
                let src_env = pv.env[k].max(1e-9);
                let want = if keep {
                    (j as f32 / fratio) as usize
                } else {
                    (j as f32 / (ratio * fratio)) as usize
                };
                let tgt = pv.env[want.min(H - 1)];
                m = m / src_env * tgt;
            }
            if m > pv.smag[j] {
                pv.sfrq[j] = pv.frq[k] * ratio;
            }
            pv.smag[j] += m;
        }
    }
    for (k, c) in fr.iter_mut().enumerate().take(H) {
        pv.sum[k] += pv.sfrq[k] * expct;
        let p = pv.sum[k];
        pv.sum[k] = p - TAU * (p / TAU).round();
        *c = Complex::from_polar(pv.smag[k], pv.sum[k]);
    }
    for k in 1..N / 2 {
        fr[N - k] = fr[k].conj();
    }
}

/// Streaming stereo pitch shifter. Latency `N` samples.
pub struct PitchShifter {
    pv: [Pv; 2],
    pub ratio: f32,
    pub fratio: f32,
    pub keep: bool,
}
impl Default for PitchShifter {
    fn default() -> Self {
        Self::new()
    }
}
impl PitchShifter {
    pub fn new() -> Self {
        PitchShifter {
            pv: [Pv::new(), Pv::new()],
            ratio: 1.0,
            fratio: 1.0,
            keep: true,
        }
    }
    pub const LATENCY: usize = N;
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32]) {
        let (ratio, fratio, keep) = (self.ratio, self.fratio, self.keep);
        let neutral = (ratio - 1.0).abs() < 1e-4 && (fratio - 1.0).abs() < 1e-4;
        for (ch, buf) in [l, r].into_iter().enumerate() {
            let Pv { st, s } = &mut self.pv[ch];
            let mut f = |fr: &mut [Complex<f32>]| {
                if !neutral {
                    shift_frame(s, fr, ratio, fratio, keep);
                }
            };
            for x in buf.iter_mut() {
                *x = st.run(*x, &mut f);
            }
        }
    }
}

pub struct PitchFx {
    ps: PitchShifter,
}
impl PitchFx {
    pub fn new(_sr: f32) -> Self {
        PitchFx {
            ps: PitchShifter::new(),
        }
    }
}
impl Processor for PitchFx {
    fn set(&mut self, p: &P) {
        self.ps.ratio = 2f32.powf((p[0] + p[1] / 100.0) / 12.0);
        self.ps.keep = p[2] > 0.5;
    }
    fn latency(&self) -> usize {
        N
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        self.ps.process(l, r);
    }
}

const MAJOR: [u8; 7] = [0, 2, 4, 5, 7, 9, 11];
const MINOR: [u8; 7] = [0, 2, 3, 5, 7, 8, 10];

/// Nearest note frequency in a key and scale.
pub fn snap_to_scale(f0: f32, key: i32, scale: i32) -> f32 {
    let midi = 69.0 + 12.0 * (f0 / 440.0).log2();
    let mut best = midi.round();
    if scale != 2 {
        let sc = if scale == 0 { MAJOR } else { MINOR };
        let mut bd = f32::MAX;
        for o in -1..=11 {
            for s in sc {
                let n = (o * 12 + key + s as i32) as f32;
                if (n - midi).abs() < bd {
                    bd = (n - midi).abs();
                    best = n;
                }
            }
        }
    }
    440.0 * 2f32.powf((best - 69.0) / 12.0)
}

/// Fundamental from a magnitude spectrum (harmonic product over 3 harmonics), 70 Hz..1 kHz.
fn f0_from_mag(mag: &[f32], sr: f32) -> Option<f32> {
    let bin = sr / N as f32;
    let (lo, hi) = ((70.0 / bin) as usize, (1000.0 / bin) as usize);
    let mut best = (0usize, 0f32);
    for k in lo..hi.min(mag.len() / 3) {
        let s = mag[k] * mag[2 * k] * mag[3 * k];
        if s > best.1 {
            best = (k, s);
        }
    }
    let tot: f32 = mag.iter().map(|m| m * m).sum();
    if best.0 == 0 || tot < 1e-3 {
        None
    } else {
        Some(best.0 as f32 * bin)
    }
}

pub struct Voice {
    sr: f32,
    ps: PitchShifter,
    pitch: f32,
    fratio: f32,
    ring: f32,
    ring_f: f32,
    rph: f32,
    comb: f32,
    cdl: Delay,
    cdr: Delay,
    voc: f32,
    carrier: i32,
    bands: usize,
    mod_f: Vec<[Biquad; 2]>,
    car_f: Vec<[Biquad; 2]>,
    envs: Vec<Env>,
    cph: [f32; 3],
    rng: Rng,
    tune: bool,
    key: i32,
    scale: i32,
    retune: f32,
    tune_ratio: f32,
    tremor: f32,
    lfo: Lfo,
    analysis: Vec<f32>,
    ai: usize,
    fft: std::sync::Arc<dyn rustfft::Fft<f32>>,
    fr: Vec<Complex<f32>>,
    mag: Vec<f32>,
}
impl Voice {
    pub fn new(sr: f32) -> Self {
        Voice {
            sr,
            ps: PitchShifter::new(),
            pitch: 0.0,
            fratio: 1.0,
            ring: 0.0,
            ring_f: 30.0,
            rph: 0.0,
            comb: 0.0,
            cdl: Delay::new(2048),
            cdr: Delay::new(2048),
            voc: 0.0,
            carrier: 0,
            bands: 0,
            mod_f: Vec::with_capacity(64),
            car_f: Vec::with_capacity(64),
            envs: Vec::with_capacity(64),
            cph: [0.0; 3],
            rng: Rng::new(9),
            tune: false,
            key: 2,
            scale: 1,
            retune: 0.5,
            tune_ratio: 1.0,
            tremor: 0.0,
            lfo: Lfo::default(),
            analysis: vec![0.0; N],
            ai: 0,
            fft: rustfft::FftPlanner::new().plan_fft_forward(N),
            fr: vec![Complex::new(0.0, 0.0); N],
            mag: vec![0.0; H],
        }
    }
    fn build_bands(&mut self, n: usize) {
        self.bands = n;
        self.mod_f.clear();
        self.car_f.clear();
        self.envs.clear();
        for b in 0..n {
            let f = 100.0 * (80.0f32).powf(b as f32 / (n - 1) as f32);
            let mk = || {
                [
                    Biquad::new(FilterType::BandPass, self.sr, f, 6.0, 0.0),
                    Biquad::new(FilterType::BandPass, self.sr, f, 6.0, 0.0),
                ]
            };
            self.mod_f.push(mk());
            self.car_f.push(mk());
            self.envs.push(Env::new(self.sr, 2.0, 20.0));
        }
    }
}
impl Processor for Voice {
    fn set(&mut self, p: &P) {
        self.pitch = p[0];
        self.fratio = 2f32.powf(p[1] / 100.0);
        self.ring = p[2] / 100.0;
        self.ring_f = p[3];
        self.comb = p[4] / 100.0;
        self.voc = p[5] / 100.0;
        self.carrier = p[6].round() as i32;
        let nb = p[7].round().clamp(8.0, 32.0) as usize;
        if nb != self.bands {
            self.build_bands(nb);
        }
        self.tune = p[8] > 0.5;
        self.key = p[9].round() as i32;
        self.scale = p[10].round() as i32;
        self.retune = 1.0 - (-(HOP as f32 / self.sr) / (p[11] * 0.001)).exp();
        self.tremor = p[12] / 100.0;
    }
    fn latency(&self) -> usize {
        N
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        // hard tune: estimate f0 from the incoming audio, once per hop
        if self.tune {
            for i in 0..l.len() {
                self.analysis[self.ai] = (l[i] + r[i]) * 0.5;
                self.ai = (self.ai + 1) % N;
                if self.ai.is_multiple_of(HOP) {
                    for k in 0..N {
                        self.fr[k] = Complex::new(self.analysis[(self.ai + k) % N], 0.0);
                    }
                    self.fft.process(&mut self.fr);
                    for k in 0..H {
                        self.mag[k] = self.fr[k].norm();
                    }
                    if let Some(f0) = f0_from_mag(&self.mag, self.sr) {
                        let shifted = f0 * 2f32.powf(self.pitch / 12.0);
                        let want = snap_to_scale(shifted, self.key, self.scale) / shifted;
                        self.tune_ratio += (want - self.tune_ratio) * self.retune;
                    }
                }
            }
        } else {
            self.tune_ratio = 1.0;
        }
        let trem = if self.tremor > 0.0 {
            1.0 + self.lfo.next(self.sr / l.len() as f32, 5.5) * 0.012 * self.tremor
        } else {
            1.0
        };
        self.ps.ratio = 2f32.powf(self.pitch / 12.0) * self.tune_ratio * trem;
        self.ps.fratio = self.fratio;
        self.ps.keep = true;
        self.ps.process(l, r);
        let cdel = (self.sr / (self.ring_f.max(1.0) * 3.0)).clamp(48.0, 1900.0);
        let carrier_f = 110.0 * 2f32.powf(self.pitch / 12.0);
        for i in 0..l.len() {
            let (mut a, mut b) = (l[i], r[i]);
            if self.voc > 0.0 && self.bands > 0 {
                let m = (a + b) * 0.5;
                let c = match self.carrier {
                    1 => self.rng.bi(),
                    2 => {
                        let mut s = 0.0;
                        for (k, mult) in [1.0, 1.26, 1.5].iter().enumerate() {
                            self.cph[k] = (self.cph[k] + carrier_f * mult / self.sr).fract();
                            s += self.cph[k] * 2.0 - 1.0;
                        }
                        s / 3.0
                    }
                    _ => {
                        self.cph[0] = (self.cph[0] + carrier_f / self.sr).fract();
                        self.cph[0] * 2.0 - 1.0
                    }
                };
                let mut v = 0.0;
                for k in 0..self.bands {
                    let e = self.envs[k].run(
                        {
                            let t_ = self.mod_f[k][0].run(m);
                            self.mod_f[k][1].run(t_)
                        }
                        .abs(),
                    );
                    v += {
                        let t_ = self.car_f[k][0].run(c);
                        self.car_f[k][1].run(t_)
                    } * e;
                }
                v *= 6.0;
                a += (v - a) * self.voc;
                b += (v - b) * self.voc;
            }
            if self.comb > 0.0 {
                let ya = self.cdl.frac(cdel);
                let yb = self.cdr.frac(cdel);
                self.cdl.push(a + ya * 0.7 * self.comb);
                self.cdr.push(b + yb * 0.7 * self.comb);
                a = a * (1.0 - self.comb * 0.5) + ya * self.comb * 0.5;
                b = b * (1.0 - self.comb * 0.5) + yb * self.comb * 0.5;
            }
            if self.ring > 0.0 {
                self.rph = (self.rph + self.ring_f / self.sr).fract();
                let s = (self.rph * TAU).sin();
                a *= 1.0 - self.ring + self.ring * s;
                b *= 1.0 - self.ring + self.ring * s;
            }
            if self.tremor > 0.0 {
                let t = 1.0 - self.tremor * 0.2 * (0.5 + 0.5 * self.lfo.ph.mul_add(TAU, 0.0).sin());
                a *= t;
                b *= t;
            }
            l[i] = a;
            r[i] = b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dominant_freq(x: &[f32], sr: f32) -> f32 {
        let n = 16384;
        let mut fr: Vec<Complex<f32>> = (0..n)
            .map(|i| Complex::new(x[i] * (0.5 - 0.5 * (TAU * i as f32 / n as f32).cos()), 0.0))
            .collect();
        rustfft::FftPlanner::new()
            .plan_fft_forward(n)
            .process(&mut fr);
        let (k, _) = fr[..n / 2].iter().enumerate().fold((0, 0f32), |b, (k, c)| {
            if c.norm() > b.1 {
                (k, c.norm())
            } else {
                b
            }
        });
        k as f32 * sr / n as f32
    }
    #[test]
    fn pitch_shift_up_a_fifth_measured() {
        let sr = 48000.0;
        let x: Vec<f32> = (0..96000)
            .map(|i| (TAU * 440.0 * i as f32 / sr).sin() * 0.5)
            .collect();
        let mut ps = PitchShifter::new();
        ps.ratio = 2f32.powf(7.0 / 12.0);
        ps.keep = false;
        let mut l = x.clone();
        let mut r = x;
        for (a, b) in l.chunks_mut(512).zip(r.chunks_mut(512)) {
            ps.process(a, b);
        }
        let f = dominant_freq(&l[40000..], sr);
        assert!((f - 659.26).abs() < 8.0, "got {f} Hz");
    }
    #[test]
    fn snap_minor() {
        assert!((snap_to_scale(450.0, 9, 1) - 440.0).abs() < 0.1);
        // 466 Hz (A#) is not in D minor -> nearest A or B? D minor has A and Bb.
        assert!((snap_to_scale(466.16, 2, 1) - 466.16).abs() < 0.5);
    }
}
