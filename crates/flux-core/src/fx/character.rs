use super::space::Reverb;
use super::{Ctx, Processor, P};
use crate::dsp::{db_to_lin, Biquad, Delay, FilterType, Lfo, OnePole, Rng, TAU};

#[inline]
fn sat(x: f32) -> f32 {
    x.tanh()
}

/// Wow and flutter: a modulated short delay.
struct Wow {
    dl: Delay,
    dr: Delay,
    w: Lfo,
    f: Lfo,
}
impl Wow {
    fn new(sr: f32) -> Self {
        Wow {
            dl: Delay::new((sr * 0.05) as usize),
            dr: Delay::new((sr * 0.05) as usize),
            w: Lfo::default(),
            f: Lfo::default(),
        }
    }
    #[inline]
    fn run(&mut self, sr: f32, a: f32, b: f32, wow_hz: f32, fl_hz: f32, depth: f32) -> (f32, f32) {
        let m = self.w.next(sr, wow_hz) * 0.0025 * sr * depth
            + self.f.next(sr, fl_hz) * 0.0002 * sr * depth;
        let d = 0.006 * sr + m;
        self.dl.push(a);
        self.dr.push(b);
        (self.dl.frac(d), self.dr.frac(d))
    }
}

/// Crackle generator: Poisson clicks, each a short decaying impulse.
struct Crackle {
    rng: Rng,
    env: f32,
    sign: f32,
}
impl Crackle {
    #[inline]
    fn run(&mut self, sr: f32, rate: f32, amp: f32) -> f32 {
        if self.rng.f() < rate / sr {
            self.env = amp * (0.3 + 0.7 * self.rng.f());
            self.sign = if self.rng.f() < 0.5 { -1.0 } else { 1.0 };
        }
        let v = self.env * self.sign;
        self.env *= 0.6;
        self.sign = -self.sign * 0.9;
        v
    }
}

pub struct Lofi {
    sr: f32,
    p: P,
    lp: [[Biquad; 2]; 2],
    hp: [Biquad; 2],
    wow: Wow,
    cr: Crackle,
    rng: Rng,
    hold: [f32; 2],
    hc: f32,
    pump: Lfo,
    room: Reverb,
    wl: Vec<f32>,
    wr: Vec<f32>,
    seed: f32,
}
impl Lofi {
    pub fn new(sr: f32) -> Self {
        Lofi {
            sr,
            p: [0.0; 32],
            lp: [[Biquad::default(); 2]; 2],
            hp: [Biquad::default(); 2],
            wow: Wow::new(sr),
            cr: Crackle {
                rng: Rng::new(1),
                env: 0.0,
                sign: 1.0,
            },
            rng: Rng::new(2),
            hold: [0.0; 2],
            hc: 0.0,
            pump: Lfo::default(),
            room: Reverb::new(sr),
            wl: vec![0.0; super::BLOCK],
            wr: vec![0.0; super::BLOCK],
            seed: -1.0,
        }
    }
}
impl Processor for Lofi {
    fn set(&mut self, p: &P) {
        let amt = p[0] / 100.0;
        let lpf = 20000.0 * (p[1] / 20000.0).powf(amt);
        for ch in &mut self.lp {
            for b in ch.iter_mut() {
                b.set(FilterType::LowPass, self.sr, lpf, 0.707, 0.0);
            }
        }
        for b in &mut self.hp {
            b.set(
                FilterType::HighPass,
                self.sr,
                20.0 + (p[2] - 20.0) * amt,
                0.707,
                0.0,
            );
        }
        self.room.set_rt(p[11].max(0.1), 0.5, 4000.0);
        if p[13] != self.seed {
            self.seed = p[13];
            self.cr.rng = Rng::new(p[13] as u64);
            self.rng = Rng::new(p[13] as u64 + 1);
        }
        self.p = *p;
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], ctx: &Ctx) {
        let p = self.p;
        let amt = p[0] / 100.0;
        let drive = db_to_lin(p[3] * amt);
        let bits = 16.0 - (16.0 - p[4]) * amt;
        let q = 2f32.powf(bits - 1.0);
        let step = (self.sr / (self.sr - (self.sr - p[5]).max(0.0) * amt).max(1000.0)).max(1.0);
        let surface = db_to_lin(p[7]) * amt;
        let noise = db_to_lin(p[12]) * amt;
        let beat = ctx.tempo.max(30.0) / 60.0;
        let n = l.len();
        for i in 0..n {
            let (a, b) = self.wow.run(self.sr, l[i], r[i], p[8], p[9], amt);
            let mut x = [a, b];
            self.hc += 1.0;
            let take = self.hc >= step;
            if take {
                self.hc -= step;
            }
            for (ch, v) in x.iter_mut().enumerate() {
                let mut s = sat(*v * drive) / drive.max(1.0).sqrt();
                if take {
                    self.hold[ch] = (s * q).round() / q;
                }
                s = self.hold[ch];
                s = {
                    let t_ = self.lp[ch][0].run(s);
                    self.lp[ch][1].run(t_)
                };
                *v = self.hp[ch].run(s);
            }
            let pump =
                1.0 - p[10] / 100.0 * amt * 0.5 * (0.5 + 0.5 * self.pump.next(self.sr, beat));
            let c = self.cr.run(self.sr, p[6] * amt, 0.25 * amt);
            let nz = self.rng.bi() * (surface + noise);
            self.wl[i] = x[0];
            self.wr[i] = x[1];
            l[i] = x[0] * pump + c + nz;
            r[i] = x[1] * pump + c + nz;
        }
        let (wl, wr) = (&mut self.wl[..n], &mut self.wr[..n]);
        self.room.process(wl, wr, ctx);
        let rm = (p[11] / 3.0) * 0.35 * amt;
        for i in 0..n {
            l[i] += wl[i] * rm;
            r[i] += wr[i] * rm;
        }
    }
}

pub struct Vinyl {
    sr: f32,
    p: P,
    wow: Wow,
    cr: Crackle,
    pops: Crackle,
    rng: Rng,
    roll: [[Biquad; 2]; 2],
    xl: [Biquad; 2],
    xh: [Biquad; 2],
    rumble: OnePole,
    seed: f32,
}
impl Vinyl {
    pub fn new(sr: f32) -> Self {
        Vinyl {
            sr,
            p: [0.0; 32],
            wow: Wow::new(sr),
            cr: Crackle {
                rng: Rng::new(1),
                env: 0.0,
                sign: 1.0,
            },
            pops: Crackle {
                rng: Rng::new(5),
                env: 0.0,
                sign: 1.0,
            },
            rng: Rng::new(3),
            roll: [[Biquad::default(); 2]; 2],
            xl: [Biquad::default(); 2],
            xh: [Biquad::default(); 2],
            rumble: OnePole::new(sr, 40.0),
            seed: -1.0,
        }
    }
}
impl Processor for Vinyl {
    fn set(&mut self, p: &P) {
        for ch in &mut self.roll {
            for b in ch.iter_mut() {
                b.set(FilterType::LowPass, self.sr, p[6], 0.6, 0.0);
            }
        }
        for b in &mut self.xl {
            b.set(FilterType::LowPass, self.sr, p[7], 0.707, 0.0);
        }
        for b in &mut self.xh {
            b.set(FilterType::HighPass, self.sr, p[7], 0.707, 0.0);
        }
        if p[8] != self.seed {
            self.seed = p[8];
            let s = p[8] as u64;
            self.cr.rng = Rng::new(s);
            self.pops.rng = Rng::new(s + 7);
            self.rng = Rng::new(s + 13);
        }
        self.p = *p;
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        let p = self.p;
        let surface = db_to_lin(p[2]);
        let warp = p[5] / 100.0 * 4.0;
        for i in 0..l.len() {
            let (a, b) = self.wow.run(self.sr, l[i], r[i], p[3], p[4], 0.25 + warp);
            // fold lows to mono, like a mono-cut lacquer
            let lo = (self.xl[0].run(a) + self.xl[1].run(b)) * 0.5;
            let a = self.xh[0].run(a) + lo;
            let b = self.xh[1].run(b) + lo;
            let a = {
                let t_ = self.roll[0][0].run(a);
                self.roll[0][1].run(t_)
            };
            let b = {
                let t_ = self.roll[1][0].run(b);
                self.roll[1][1].run(t_)
            };
            let c = self.cr.run(self.sr, p[0], 0.12) + self.pops.run(self.sr, p[1], 0.5);
            let hiss = self.rng.bi() * surface + self.rumble.lp(self.rng.bi()) * surface * 3.0;
            l[i] = a + c + hiss;
            r[i] = b + c * 0.8 + hiss;
        }
    }
}

pub struct Cassette {
    sr: f32,
    p: P,
    wow: Wow,
    rng: Rng,
    roll: [[Biquad; 2]; 2],
    seed: f32,
}
impl Cassette {
    pub fn new(sr: f32) -> Self {
        Cassette {
            sr,
            p: [0.0; 32],
            wow: Wow::new(sr),
            rng: Rng::new(4),
            roll: [[Biquad::default(); 2]; 2],
            seed: -1.0,
        }
    }
}
impl Processor for Cassette {
    fn set(&mut self, p: &P) {
        for ch in &mut self.roll {
            ch[0].set(FilterType::LowPass, self.sr, p[4], 0.707, 0.0);
            ch[1].set(FilterType::LowShelf, self.sr, 100.0, 0.7, 2.0);
        }
        if p[5] != self.seed {
            self.seed = p[5];
            self.rng = Rng::new(p[5] as u64);
        }
        self.p = *p;
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        let p = self.p;
        let hiss = db_to_lin(p[0]);
        let drive = 1.0 + p[3] / 100.0 * 3.0;
        for i in 0..l.len() {
            let (a, b) = self.wow.run(self.sr, l[i], r[i], p[1], p[2], 1.0);
            let a = sat(a * drive) / drive.sqrt();
            let b = sat(b * drive) / drive.sqrt();
            l[i] = {
                let t_ = self.roll[0][0].run(a);
                self.roll[0][1].run(t_)
            } + self.rng.bi() * hiss;
            r[i] = {
                let t_ = self.roll[1][0].run(b);
                self.roll[1][1].run(t_)
            } + self.rng.bi() * hiss;
        }
    }
}

/// Band-limited voice textures: old radio, phone, megaphone, walkie-talkie.
pub struct Band {
    sr: f32,
    kind: &'static str,
    p: P,
    f: [[Biquad; 4]; 2],
    rng: Rng,
    env: f32,
    seed: f32,
}
impl Band {
    pub fn new(sr: f32, kind: &str) -> Self {
        let k: &'static str = match kind {
            "radio" => "radio",
            "phone" => "phone",
            "megaphone" => "megaphone",
            _ => "walkie",
        };
        Band {
            sr,
            kind: k,
            p: [0.0; 32],
            f: [[Biquad::default(); 4]; 2],
            rng: Rng::new(7),
            env: 0.0,
            seed: -1.0,
        }
    }
}
impl Processor for Band {
    fn set(&mut self, p: &P) {
        let (lo, hi, mid, mg) = match self.kind {
            "radio" => (p[0], p[1], 1200.0, 3.0),
            "phone" => (p[0], p[1], 1800.0, 2.0),
            "megaphone" => (p[1] * 0.4, p[1] * 2.6, p[1], 6.0),
            _ => (400.0, 2800.0, 1500.0, 4.0),
        };
        for ch in &mut self.f {
            ch[0].set(FilterType::HighPass, self.sr, lo, 0.8, 0.0);
            ch[1].set(FilterType::HighPass, self.sr, lo, 0.8, 0.0);
            ch[2].set(FilterType::LowPass, self.sr, hi, 0.8, 0.0);
            ch[3].set(FilterType::Bell, self.sr, mid, 1.0, mg);
        }
        let seed_idx = match self.kind {
            "radio" => 4,
            "walkie" => 3,
            _ => 31,
        };
        if p[seed_idx] != self.seed {
            self.seed = p[seed_idx];
            self.rng = Rng::new(p[seed_idx] as u64);
        }
        self.p = *p;
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        let p = self.p;
        let (drive, noise) = match self.kind {
            "radio" => (1.0 + p[2] / 100.0 * 6.0, db_to_lin(p[3])),
            "phone" => (1.0 + p[2] / 100.0 * 4.0, 0.0),
            "megaphone" => (1.0 + p[0] / 100.0 * 20.0, 0.0),
            _ => (1.0 + p[0] / 100.0 * 10.0, db_to_lin(p[2])),
        };
        let squelch = self.kind == "walkie" && p[1] > 0.5;
        for i in 0..l.len() {
            let m = (l[i] + r[i]) * 0.5;
            self.env = self.env * 0.999 + m.abs() * 0.001;
            let mut y = {
                let t_ = self.f[0][0].run(m);
                self.f[0][1].run(t_)
            };
            y = if self.kind == "megaphone" {
                (y * drive).clamp(-0.6, 0.6)
            } else {
                sat(y * drive) / drive.sqrt()
            };
            y = {
                let t_ = self.f[0][2].run(y);
                self.f[0][3].run(t_)
            };
            let gate = if squelch {
                (self.env * 400.0).min(1.0)
            } else {
                1.0
            };
            let nz = self.rng.bi() * noise * if squelch { 1.0 - gate * 0.8 } else { 1.0 };
            let _ = &mut self.f[1];
            l[i] = y * gate + nz;
            r[i] = y * gate + nz;
        }
    }
}

pub struct Underwater {
    sr: f32,
    lp: [[Biquad; 2]; 2],
    lfo: Lfo,
    dl: Delay,
    dr: Delay,
    wobble: f32,
}
impl Underwater {
    pub fn new(sr: f32) -> Self {
        Underwater {
            sr,
            lp: [[Biquad::default(); 2]; 2],
            lfo: Lfo::default(),
            dl: Delay::new((sr * 0.05) as usize),
            dr: Delay::new((sr * 0.05) as usize),
            wobble: 0.4,
        }
    }
}
impl Processor for Underwater {
    fn set(&mut self, p: &P) {
        let f = 2400.0 * (0.12f32).powf(p[0] / 100.0);
        for ch in &mut self.lp {
            for b in ch.iter_mut() {
                b.set(FilterType::LowPass, self.sr, f, 1.2, 0.0);
            }
        }
        self.wobble = p[1] / 100.0;
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        for i in 0..l.len() {
            let m = self.lfo.next(self.sr, 0.7) * 0.004 * self.sr * self.wobble + 0.008 * self.sr;
            self.dl.push(l[i]);
            self.dr.push(r[i]);
            let a = self.dl.frac(m);
            let b = self.dr.frac(m * 1.1);
            l[i] = {
                let t_ = self.lp[0][0].run(a);
                self.lp[0][1].run(t_)
            } * 1.3;
            r[i] = {
                let t_ = self.lp[1][0].run(b);
                self.lp[1][1].run(t_)
            } * 1.3;
        }
    }
}

pub struct Distortion {
    sr: f32,
    drive: f32,
    out: f32,
    ty: i32,
    tone: [OnePole; 2],
}
impl Distortion {
    pub fn new(sr: f32) -> Self {
        Distortion {
            sr,
            drive: 4.0,
            out: 0.5,
            ty: 0,
            tone: [OnePole::default(); 2],
        }
    }
}
impl Processor for Distortion {
    fn set(&mut self, p: &P) {
        self.drive = db_to_lin(p[0]);
        self.tone[0].set(self.sr, p[1]);
        self.tone[1].set(self.sr, p[1]);
        self.out = db_to_lin(p[2]);
        self.ty = p[3].round() as i32;
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        for (ch, buf) in [l, r].into_iter().enumerate() {
            for x in buf.iter_mut() {
                let v = *x * self.drive;
                let y = match self.ty {
                    1 => v.clamp(-1.0, 1.0),
                    2 => {
                        (v.signum() * (1.0 - (-v.abs() * 2.0).exp())).clamp(-1.0, 1.0)
                            * if v > 0.0 { 1.0 } else { 0.7 }
                    }
                    _ => sat(v),
                };
                *x = self.tone[ch].lp(y) * self.out;
            }
        }
    }
}

#[derive(Default)]
pub struct Crusher {
    q: f32,
    step: f32,
    c: f32,
    h: [f32; 2],
}
impl Processor for Crusher {
    fn set(&mut self, p: &P) {
        self.q = 2f32.powf(p[0] - 1.0);
        self.step = p[1].max(1.0);
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        for i in 0..l.len() {
            self.c += 1.0;
            if self.c >= self.step {
                self.c -= self.step;
                self.h[0] = (l[i] * self.q).round() / self.q;
                self.h[1] = (r[i] * self.q).round() / self.q;
            }
            l[i] = self.h[0];
            r[i] = self.h[1];
        }
    }
}

/// Chorus (several modulated voices) or flanger (one voice with feedback).
pub struct Modulated {
    sr: f32,
    flanger: bool,
    p: P,
    dl: Delay,
    dr: Delay,
    lfo: [Lfo; 4],
    fb: [f32; 2],
}
impl Modulated {
    pub fn new(sr: f32, flanger: bool) -> Self {
        let mut lfo = [Lfo::default(); 4];
        for (k, o) in lfo.iter_mut().enumerate() {
            o.ph = k as f32 * 0.25;
        }
        Modulated {
            sr,
            flanger,
            p: [0.0; 32],
            dl: Delay::new((sr * 0.06) as usize),
            dr: Delay::new((sr * 0.06) as usize),
            lfo,
            fb: [0.0; 2],
        }
    }
}
impl Processor for Modulated {
    fn set(&mut self, p: &P) {
        self.p = *p;
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        let p = self.p;
        let (rate, depth) = (p[0], p[1] / 100.0);
        for i in 0..l.len() {
            if self.flanger {
                let fb = p[2] / 100.0;
                let base = p[3] * 0.001 * self.sr;
                let m = base * (1.0 + depth * self.lfo[0].next(self.sr, rate)) + 1.0;
                self.dl.push(l[i] + self.fb[0] * fb);
                self.dr.push(r[i] + self.fb[1] * fb);
                self.fb = [self.dl.frac(m), self.dr.frac(m * 1.02)];
                l[i] = self.fb[0];
                r[i] = self.fb[1];
            } else {
                let voices = p[3].round().clamp(1.0, 4.0) as usize;
                self.dl.push(l[i]);
                self.dr.push(r[i]);
                let (mut a, mut b) = (0.0, 0.0);
                for v in 0..voices {
                    let m = p[2]
                        * 0.001
                        * self.sr
                        * (1.0
                            + 0.5
                                * depth
                                * self.lfo[v].next(self.sr, rate * (1.0 + v as f32 * 0.13)));
                    a += self.dl.frac(m);
                    b += self.dr.frac(m * (1.0 + 0.07 * v as f32));
                }
                l[i] = a / voices as f32;
                r[i] = b / voices as f32;
            }
        }
    }
}

pub struct Phaser {
    sr: f32,
    p: P,
    ap: [[Biquad; 12]; 2],
    lfo: Lfo,
    fb: [f32; 2],
    c: usize,
}
impl Phaser {
    pub fn new(sr: f32) -> Self {
        Phaser {
            sr,
            p: [0.0; 32],
            ap: [[Biquad::default(); 12]; 2],
            lfo: Lfo::default(),
            fb: [0.0; 2],
            c: 0,
        }
    }
}
impl Processor for Phaser {
    fn set(&mut self, p: &P) {
        self.p = *p;
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        let p = self.p;
        let stages = (p[2].round() as usize).clamp(2, 12);
        let fb = p[3] / 100.0;
        for i in 0..l.len() {
            let m = self.lfo.next(self.sr, p[0]);
            if self.c.is_multiple_of(16) {
                let f = 300.0 * (8.0f32).powf((0.5 + 0.5 * m) * p[1] / 100.0);
                for ch in &mut self.ap {
                    for s in ch.iter_mut().take(stages) {
                        s.set(FilterType::AllPass, self.sr, f, 0.7, 0.0);
                    }
                }
            }
            self.c += 1;
            for (ch, x) in [&mut l[i], &mut r[i]].into_iter().enumerate() {
                let mut y = *x + self.fb[ch] * fb;
                for s in self.ap[ch].iter_mut().take(stages) {
                    y = s.run(y);
                }
                self.fb[ch] = y;
                *x = y;
            }
        }
    }
}

pub struct Rotary {
    sr: f32,
    rate: f32,
    target: f32,
    depth: f32,
    lfo: Lfo,
    dl: Delay,
    dr: Delay,
}
impl Rotary {
    pub fn new(sr: f32) -> Self {
        Rotary {
            sr,
            rate: 0.8,
            target: 0.8,
            depth: 0.7,
            lfo: Lfo::default(),
            dl: Delay::new(2048),
            dr: Delay::new(2048),
        }
    }
}
impl Processor for Rotary {
    fn set(&mut self, p: &P) {
        self.target = if p[0] > 0.5 { 6.7 } else { 0.8 };
        self.depth = p[1] / 100.0;
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        for i in 0..l.len() {
            // the horn spins up and down slowly, like the real motor
            self.rate += (self.target - self.rate) * 0.00002;
            let s = self.lfo.next(self.sr, self.rate);
            let c = (self.lfo.ph * TAU).cos();
            self.dl.push(l[i]);
            self.dr.push(r[i]);
            let d = 0.002 * self.sr;
            let a = self.dl.frac(d * (1.0 + 0.5 * s * self.depth));
            let b = self.dr.frac(d * (1.0 - 0.5 * s * self.depth));
            l[i] = a * (1.0 - 0.35 * self.depth * (1.0 + c) * 0.5);
            r[i] = b * (1.0 - 0.35 * self.depth * (1.0 - c) * 0.5);
        }
    }
}
