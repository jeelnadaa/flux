use super::{Ctx, Processor, P};
use crate::dsp::{db_to_lin, lin_to_db, pan_gains, Biquad, Delay, Env, FilterType, OnePole};

#[derive(Default)]
pub struct Gain {
    g: f32,
    pl: f32,
    pr: f32,
    /// Gains applied at the end of the last block; each block ramps from these.
    last: Option<(f32, f32)>,
}
impl Processor for Gain {
    fn set(&mut self, p: &P) {
        self.g = db_to_lin(p[0]);
        let (a, b) = pan_gains(p[1] / 100.0);
        self.pl = a.min(1.0);
        self.pr = b.min(1.0);
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        let (tl, tr) = (self.g * self.pl, self.g * self.pr);
        let (sl, sr) = self.last.unwrap_or((tl, tr));
        let n = l.len().max(1) as f32;
        for (i, (a, b)) in l.iter_mut().zip(r.iter_mut()).enumerate() {
            let t = (i + 1) as f32 / n;
            *a *= sl + (tl - sl) * t;
            *b *= sr + (tr - sr) * t;
        }
        self.last = Some((tl, tr));
    }
    fn reset(&mut self) {
        self.last = None;
    }
}

/// Static compressor curve with soft knee: returns gain reduction in dB (<= 0).
pub fn comp_gain_db(x_db: f32, th: f32, ratio: f32, knee: f32) -> f32 {
    let over = x_db - th;
    if knee > 0.0 && over.abs() <= knee / 2.0 {
        (1.0 / ratio - 1.0) * (over + knee / 2.0).powi(2) / (2.0 * knee)
    } else if over > 0.0 {
        (1.0 / ratio - 1.0) * over
    } else {
        0.0
    }
}

pub struct Compressor {
    sr: f32,
    env: Env,
    th: f32,
    ratio: f32,
    knee: f32,
    makeup: f32,
    la: usize,
    dl: Delay,
    dr: Delay,
    sc: [Biquad; 2],
    pub gr_db: f32,
}
impl Compressor {
    pub fn new(sr: f32) -> Self {
        Compressor {
            sr,
            env: Env::new(sr, 10.0, 100.0),
            th: -18.0,
            ratio: 3.0,
            knee: 6.0,
            makeup: 1.0,
            la: 0,
            dl: Delay::new(1024),
            dr: Delay::new(1024),
            sc: [Biquad::default(); 2],
            gr_db: 0.0,
        }
    }
}
impl Processor for Compressor {
    fn set(&mut self, p: &P) {
        self.th = p[0];
        self.ratio = p[1].max(1.0);
        self.env.set(self.sr, p[2], p[3]);
        self.knee = p[4];
        let auto = if p[7] > 0.5 {
            -comp_gain_db(0.0, self.th, self.ratio, self.knee) * 0.5
        } else {
            0.0
        };
        self.makeup = db_to_lin(p[5] + auto);
        self.la = (p[6] * 0.001 * self.sr) as usize;
        for b in &mut self.sc {
            b.set(FilterType::HighPass, self.sr, p[8], 0.707, 0.0);
        }
    }
    fn latency(&self) -> usize {
        0
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        for i in 0..l.len() {
            let d = self.sc[0].run(l[i]).abs().max(self.sc[1].run(r[i]).abs());
            let e = self.env.run(d);
            let g = comp_gain_db(lin_to_db(e), self.th, self.ratio, self.knee);
            self.gr_db = g;
            let gl = db_to_lin(g) * self.makeup;
            self.dl.push(l[i]);
            self.dr.push(r[i]);
            l[i] = self.dl.tap(self.la) * gl;
            r[i] = self.dr.tap(self.la) * gl;
        }
    }
}

/// Look-ahead brick-wall peak limiter.
pub struct Limiter {
    sr: f32,
    ceil: f32,
    input: f32,
    rel: f32,
    la: usize,
    dl: Delay,
    dr: Delay,
    peak: Vec<f32>,
    pi: usize,
    g: f32,
}
impl Limiter {
    pub fn new(sr: f32) -> Self {
        Limiter {
            sr,
            ceil: 0.89,
            input: 1.0,
            rel: 0.999,
            la: 144,
            dl: Delay::new(1024),
            dr: Delay::new(1024),
            peak: vec![0.0; 1024],
            pi: 0,
            g: 1.0,
        }
    }
}
impl Processor for Limiter {
    fn set(&mut self, p: &P) {
        self.ceil = db_to_lin(p[0]);
        self.input = db_to_lin(p[1]);
        self.rel = (-1.0 / (self.sr * p[2] * 0.001)).exp();
        self.la = ((p[3] * 0.001 * self.sr) as usize).clamp(1, 1000);
    }
    fn latency(&self) -> usize {
        // fixed at the default so the chain's dry path stays aligned
        (0.003 * self.sr) as usize
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        let la = self.latency();
        for i in 0..l.len() {
            let (a, b) = (l[i] * self.input, r[i] * self.input);
            self.dl.push(a);
            self.dr.push(b);
            let pk = a.abs().max(b.abs());
            self.peak[self.pi] = pk;
            self.pi = (self.pi + 1) % la.max(1);
            // gain needed for the loudest sample inside the look-ahead window
            let mut m = 0f32;
            for v in &self.peak[..la.max(1)] {
                m = m.max(*v);
            }
            let need = if m > self.ceil { self.ceil / m } else { 1.0 };
            self.g = if need < self.g {
                need
            } else {
                need + self.rel * (self.g - need)
            };
            let (x, y) = (self.dl.tap(la), self.dr.tap(la));
            l[i] = (x * self.g).clamp(-self.ceil, self.ceil);
            r[i] = (y * self.g).clamp(-self.ceil, self.ceil);
        }
    }
}

pub struct Gate {
    sr: f32,
    env: Env,
    th: f32,
    range: f32,
    hold: usize,
    held: usize,
    g: Env,
}
impl Gate {
    pub fn new(sr: f32) -> Self {
        Gate {
            sr,
            env: Env::new(sr, 0.5, 30.0),
            th: 0.003,
            range: 0.001,
            hold: 2400,
            held: 0,
            g: Env::new(sr, 1.0, 150.0),
        }
    }
}
impl Processor for Gate {
    fn set(&mut self, p: &P) {
        self.th = db_to_lin(p[0]);
        self.range = db_to_lin(p[1]);
        self.g.set(self.sr, p[2], p[4]);
        self.hold = (p[3] * 0.001 * self.sr) as usize;
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        for i in 0..l.len() {
            let e = self.env.run(l[i].abs().max(r[i].abs()));
            let open = if e > self.th {
                self.held = self.hold;
                true
            } else if self.held > 0 {
                self.held -= 1;
                true
            } else {
                false
            };
            let g = self.g.run(if open { 1.0 } else { self.range });
            l[i] *= g;
            r[i] *= g;
        }
    }
}

pub struct DeEsser {
    sr: f32,
    bp: [Biquad; 2],
    env: Env,
    th: f32,
    max: f32,
    pub gr_db: f32,
}
impl DeEsser {
    pub fn new(sr: f32) -> Self {
        DeEsser {
            sr,
            bp: [Biquad::default(); 2],
            env: Env::new(sr, 1.0, 60.0),
            th: -30.0,
            max: 6.0,
            gr_db: 0.0,
        }
    }
}
impl Processor for DeEsser {
    fn set(&mut self, p: &P) {
        for b in &mut self.bp {
            b.set(FilterType::HighPass, self.sr, p[0] * 0.8, 0.707, 0.0);
        }
        self.th = p[1];
        self.max = p[2];
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        for i in 0..l.len() {
            let s = self.bp[0].run(l[i]).abs().max(self.bp[1].run(r[i]).abs());
            let e = lin_to_db(self.env.run(s));
            let gr = (-(e - self.th).max(0.0) * 0.75).max(-self.max);
            self.gr_db = gr;
            let g = db_to_lin(gr);
            l[i] *= g;
            r[i] *= g;
        }
    }
}

pub struct Transient {
    fast: Env,
    slow: Env,
    att: f32,
    sus: f32,
}
impl Transient {
    pub fn new(sr: f32) -> Self {
        Transient {
            fast: Env::new(sr, 0.5, 50.0),
            slow: Env::new(sr, 20.0, 50.0),
            att: 0.0,
            sus: 0.0,
        }
    }
}
impl Processor for Transient {
    fn set(&mut self, p: &P) {
        self.att = p[0] / 100.0;
        self.sus = p[1] / 100.0;
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        for i in 0..l.len() {
            let x = l[i].abs().max(r[i].abs());
            let f = self.fast.run(x);
            let s = self.slow.run(x);
            let t = ((f - s) / (s + 1e-4)).clamp(0.0, 4.0);
            let g =
                (1.0 + self.att * t * 0.5 + self.sus * (1.0 - t.min(1.0)) * 0.5).clamp(0.1, 4.0);
            l[i] *= g;
            r[i] *= g;
        }
    }
}

pub struct Leveler {
    sr: f32,
    rms: OnePole,
    g: f32,
    target: f32,
    max: f32,
    a: f32,
}
impl Leveler {
    pub fn new(sr: f32) -> Self {
        Leveler {
            sr,
            rms: OnePole::new(sr, 3.0),
            g: 1.0,
            target: 0.1,
            max: 4.0,
            a: 0.9999,
        }
    }
}
impl Processor for Leveler {
    fn set(&mut self, p: &P) {
        self.target = db_to_lin(p[0]);
        self.max = db_to_lin(p[1]);
        self.a = (-1.0 / (self.sr * p[2] * 0.001)).exp();
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        for i in 0..l.len() {
            let ms = self.rms.lp((l[i] * l[i] + r[i] * r[i]) * 0.5).max(0.0);
            let rms = ms.sqrt();
            // only level signal above a floor, so pauses are not pumped up
            let want = if rms > 0.003 {
                (self.target / rms).clamp(1.0 / self.max, self.max)
            } else {
                self.g
            };
            self.g = want + self.a * (self.g - want);
            l[i] *= self.g;
            r[i] *= self.g;
        }
    }
}

/// Three-band compressor with Linkwitz-Riley (2 x Butterworth) crossovers.
pub struct Multiband {
    sr: f32,
    xl: [[Biquad; 4]; 2],
    xh: [[Biquad; 4]; 2],
    env: [Env; 3],
    th: f32,
    ratio: f32,
    gain: [f32; 3],
}
impl Multiband {
    pub fn new(sr: f32) -> Self {
        Multiband {
            sr,
            xl: [[Biquad::default(); 4]; 2],
            xh: [[Biquad::default(); 4]; 2],
            env: [Env::new(sr, 10.0, 150.0); 3],
            th: -20.0,
            ratio: 2.5,
            gain: [1.0; 3],
        }
    }
}
impl Processor for Multiband {
    fn set(&mut self, p: &P) {
        for ch in 0..2 {
            for k in 0..2 {
                self.xl[ch][k].set(
                    FilterType::LowPass,
                    self.sr,
                    p[0],
                    std::f32::consts::FRAC_1_SQRT_2,
                    0.0,
                );
                self.xl[ch][k + 2].set(
                    FilterType::HighPass,
                    self.sr,
                    p[0],
                    std::f32::consts::FRAC_1_SQRT_2,
                    0.0,
                );
                self.xh[ch][k].set(
                    FilterType::LowPass,
                    self.sr,
                    p[1],
                    std::f32::consts::FRAC_1_SQRT_2,
                    0.0,
                );
                self.xh[ch][k + 2].set(
                    FilterType::HighPass,
                    self.sr,
                    p[1],
                    std::f32::consts::FRAC_1_SQRT_2,
                    0.0,
                );
            }
        }
        self.th = p[2];
        self.ratio = p[3];
        for e in &mut self.env {
            e.set(self.sr, p[4], p[5]);
        }
        self.gain = [db_to_lin(p[6]), db_to_lin(p[7]), db_to_lin(p[8])];
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        for i in 0..l.len() {
            let mut bands = [[0.0f32; 3]; 2];
            for (ch, x) in [l[i], r[i]].into_iter().enumerate() {
                let f = &mut self.xl[ch];
                let low = {
                    let t_ = f[0].run(x);
                    f[1].run(t_)
                };
                let rest = {
                    let t_ = f[2].run(x);
                    f[3].run(t_)
                };
                let h = &mut self.xh[ch];
                let mid = {
                    let t_ = h[0].run(rest);
                    h[1].run(t_)
                };
                let high = {
                    let t_ = h[2].run(rest);
                    h[3].run(t_)
                };
                bands[ch] = [low, mid, high];
            }
            let mut o = [0.0f32; 2];
            for (b, env) in self.env.iter_mut().enumerate().take(3) {
                let (x0, x1) = (bands[0][b], bands[1][b]);
                let e = env.run(x0.abs().max(x1.abs()));
                let g =
                    db_to_lin(comp_gain_db(lin_to_db(e), self.th, self.ratio, 6.0)) * self.gain[b];
                o[0] += x0 * g;
                o[1] += x1 * g;
            }
            l[i] = o[0];
            r[i] = o[1];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fx::{resolve, spec};
    fn run(kind: &str, set: &[(&str, f32)], amp: f32) -> Vec<f32> {
        let sr = 48000.0;
        let m = set.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        let p = resolve(spec(kind).unwrap(), &m);
        let mut pr = crate::fx::build(kind, sr, &Default::default()).unwrap();
        pr.set(&p);
        let mut l: Vec<f32> = (0..96000)
            .map(|i| (crate::dsp::TAU * 440.0 * i as f32 / sr).sin() * amp)
            .collect();
        let mut r = l.clone();
        for (a, b) in l.chunks_mut(512).zip(r.chunks_mut(512)) {
            pr.process(
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
    }
    #[test]
    fn limiter_holds_ceiling() {
        let y = run("limiter", &[("ceiling", -3.0), ("input", 12.0)], 0.9);
        let pk = y[4800..].iter().fold(0f32, |m, v| m.max(v.abs()));
        assert!(pk <= db_to_lin(-3.0) + 1e-4, "peak {pk}");
        assert!(pk > db_to_lin(-3.5));
    }
    #[test]
    fn compressor_reduces_by_static_curve() {
        // 0 dBFS-peak sine, threshold -20, ratio 4, no knee, no makeup: steady state reduction 15 dB.
        let y = run(
            "compressor",
            &[
                ("threshold", -20.0),
                ("ratio", 4.0),
                ("knee", 0.0),
                ("auto_makeup", 0.0),
                ("attack", 1.0),
                ("release", 50.0),
            ],
            1.0,
        );
        let pk = y[48000..].iter().fold(0f32, |m, v| m.max(v.abs()));
        let got = lin_to_db(pk);
        assert!((got - (-15.0)).abs() < 1.5, "peak {got} dB");
    }
}
