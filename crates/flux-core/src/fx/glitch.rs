//! Glitch effects. Decisions are hashed from (seed, grid step), so they depend only on the
//! timeline position: seeking, previewing and exporting give the same result.

use super::{Ctx, Processor, P};
use crate::dsp::{hann, Rng};

fn hash(seed: u64, step: i64) -> f32 {
    let mut r = Rng::new(seed.wrapping_mul(0x100000001B3) ^ step as u64);
    r.f()
}

struct Ring {
    l: Vec<f32>,
    r: Vec<f32>,
    w: usize,
}
impl Ring {
    fn new(n: usize) -> Self {
        let n = n.next_power_of_two();
        Ring {
            l: vec![0.0; n],
            r: vec![0.0; n],
            w: 0,
        }
    }
    #[inline]
    fn push(&mut self, a: f32, b: f32) {
        let m = self.l.len() - 1;
        self.w = (self.w + 1) & m;
        self.l[self.w] = a;
        self.r[self.w] = b;
    }
    /// Read `back` samples behind the write head (fractional, linear).
    #[inline]
    fn read(&self, back: f32) -> (f32, f32) {
        let m = self.l.len() - 1;
        let back = back.clamp(0.0, (m - 2) as f32);
        let i = back as usize;
        let t = back - i as f32;
        let (a0, b0) = (
            self.l[self.w.wrapping_sub(i) & m],
            self.r[self.w.wrapping_sub(i) & m],
        );
        let (a1, b1) = (
            self.l[self.w.wrapping_sub(i + 1) & m],
            self.r[self.w.wrapping_sub(i + 1) & m],
        );
        (a0 + (a1 - a0) * t, b0 + (b1 - b0) * t)
    }
}

fn step_len(ctx: &Ctx, den: f32) -> f32 {
    60.0 / ctx.tempo.clamp(30.0, 300.0) * 4.0 / den * ctx.sr
}

pub struct Stutter {
    ring: Ring,
    p: P,
}
impl Stutter {
    pub fn new(sr: f32) -> Self {
        Stutter {
            ring: Ring::new((sr * 4.0) as usize),
            p: [0.0; 32],
        }
    }
}
impl Processor for Stutter {
    fn set(&mut self, p: &P) {
        self.p = *p;
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], ctx: &Ctx) {
        let p = self.p;
        let den = [4.0, 8.0, 16.0, 32.0, 64.0][(p[0].round() as usize).min(4)];
        let sl = step_len(ctx, den);
        let prob = p[1] / 100.0;
        let ramp = p[2];
        let gate = (p[3] / 100.0).max(0.05);
        let seed = p[4] as u64;
        for i in 0..l.len() {
            self.ring.push(l[i], r[i]);
            let t = (ctx.pos + i as i64) as f32;
            if t < 0.0 {
                continue;
            }
            let step = (t / sl) as i64;
            if hash(seed, step) >= prob {
                continue;
            }
            let into = t - step as f32 * sl;
            let slice = sl / 4.0;
            let rep = (into / slice) as i32;
            if rep == 0 {
                continue;
            }
            let pos_in = into - rep as f32 * slice;
            let rate = 2f32.powf(ramp * rep as f32 / 36.0);
            let read = (pos_in * rate) % slice;
            let back = into - read;
            let (a, b) = self.ring.read(back);
            let g = if pos_in < slice * gate { 1.0 } else { 0.0 };
            let fade = (pos_in / 64.0).min(1.0) * ((slice * gate - pos_in) / 64.0).clamp(0.0, 1.0);
            l[i] = a * g * fade;
            r[i] = b * g * fade;
        }
    }
}

pub struct TapeStop {
    sr: f32,
    ring: Ring,
    p: P,
    rp: f32,
    region_start: i64,
}
impl TapeStop {
    pub fn new(sr: f32) -> Self {
        TapeStop {
            sr,
            ring: Ring::new((sr * 10.0) as usize),
            p: [0.0; 32],
            rp: 0.0,
            region_start: i64::MIN,
        }
    }
}
impl Processor for TapeStop {
    fn set(&mut self, p: &P) {
        self.p = *p;
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], ctx: &Ctx) {
        let at = (self.p[0] * self.sr) as i64;
        let tl = (self.p[1] * self.sr) as i64;
        let mode = self.p[2].round() as i32;
        let (stop0, start0) = match mode {
            1 => (i64::MAX, at),
            2 => (at - tl, at),
            _ => (at, i64::MAX),
        };
        for i in 0..l.len() {
            self.ring.push(l[i], r[i]);
            let t = ctx.pos + i as i64;
            if t >= stop0 && t < stop0 + tl {
                if self.region_start != stop0 {
                    self.region_start = stop0;
                    self.rp = 0.0;
                }
                let u = (t - stop0) as f32 / tl as f32;
                // `rp` is how far the read head has fallen behind the write head
                self.rp += 1.0 - (1.0 - u) * (1.0 - u);
                let (a, b) = self.ring.read(self.rp);
                let g = 1.0 - u.powi(4);
                l[i] = a * g;
                r[i] = b * g;
            } else if t >= start0 && t < start0 + tl {
                if self.region_start != start0 {
                    self.region_start = start0;
                    self.rp = 0.0;
                }
                let u = (t - start0) as f32 / tl as f32;
                self.rp += 1.0 - u * u;
                let (a, b) = self.ring.read(self.rp);
                let g = (u * 4.0).min(1.0);
                // fade into the live signal over the last 10% of the ramp
                let x = ((u - 0.9) / 0.1).clamp(0.0, 1.0);
                l[i] = a * g * (1.0 - x) + l[i] * x;
                r[i] = b * g * (1.0 - x) + r[i] * x;
            }
        }
    }
}

pub struct Granular {
    sr: f32,
    ring: Ring,
    p: P,
    rng: Rng,
    win: Vec<f32>,
    grains: [(f32, f32, f32); 2],
    seed: f32,
}
impl Granular {
    pub fn new(sr: f32) -> Self {
        Granular {
            sr,
            ring: Ring::new((sr * 2.0) as usize),
            p: [0.0; 32],
            rng: Rng::new(5),
            win: hann(1024),
            grains: [(0.0, 0.0, 0.0), (0.5, 0.0, 0.0)],
            seed: -1.0,
        }
    }
}
impl Processor for Granular {
    fn set(&mut self, p: &P) {
        if p[3] != self.seed {
            self.seed = p[3];
            self.rng = Rng::new(p[3] as u64);
        }
        self.p = *p;
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        let glen = self.p[0] * 0.001 * self.sr;
        let spray = self.p[1] / 100.0 * self.sr * 0.5;
        let freeze = self.p[2] > 0.5;
        for i in 0..l.len() {
            if !freeze {
                self.ring.push(l[i], r[i]);
            }
            let (mut a, mut b) = (0.0, 0.0);
            for g in &mut self.grains {
                // g = (phase 0..1, start back offset, unused)
                if g.0 >= 1.0 {
                    g.0 -= 1.0;
                    g.1 = glen + self.rng.f() * spray;
                }
                let w = self.win[((g.0 * 1023.0) as usize).min(1023)];
                let (x, y) = self
                    .ring
                    .read(g.1 - g.0 * glen + if freeze { 0.0 } else { g.0 * glen });
                a += x * w;
                b += y * w;
                g.0 += 1.0 / glen.max(1.0);
            }
            l[i] = a;
            r[i] = b;
        }
    }
}

pub struct TranceGate {
    sr: f32,
    p: P,
    g: f32,
}
impl TranceGate {
    pub fn new(sr: f32) -> Self {
        TranceGate {
            sr,
            p: [0.0; 32],
            g: 1.0,
        }
    }
}
impl Processor for TranceGate {
    fn set(&mut self, p: &P) {
        self.p = *p;
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], ctx: &Ctx) {
        let bits = self.p[0] as u32;
        let depth = self.p[1] / 100.0;
        let den = if self.p[2] > 0.5 { 8.0 } else { 16.0 };
        let sl = step_len(ctx, den);
        let a = (-1.0 / (self.sr * (self.p[3].max(0.1)) * 0.001)).exp();
        for i in 0..l.len() {
            let t = (ctx.pos + i as i64).max(0) as f32;
            let step = ((t / sl) as u32) % 16;
            let on = bits & (1 << step) != 0;
            let target = if on { 1.0 } else { 1.0 - depth };
            self.g = target + a * (self.g - target);
            l[i] *= self.g;
            r[i] *= self.g;
        }
    }
}

pub struct Glitch {
    ring: Ring,
    p: P,
}
impl Glitch {
    pub fn new(sr: f32) -> Self {
        Glitch {
            ring: Ring::new((sr * 4.0) as usize),
            p: [0.0; 32],
        }
    }
}
impl Processor for Glitch {
    fn set(&mut self, p: &P) {
        self.p = *p;
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], ctx: &Ctx) {
        let chaos = self.p[0] / 100.0;
        let seed = self.p[1] as u64;
        let sl = step_len(ctx, 16.0);
        for i in 0..l.len() {
            self.ring.push(l[i], r[i]);
            let t = (ctx.pos + i as i64).max(0) as f32;
            let step = (t / sl) as i64;
            let h = hash(seed, step);
            if h >= chaos * 0.6 {
                continue;
            }
            let into = t - step as f32 * sl;
            let what = (hash(seed + 1, step) * 4.0) as i32;
            let (a, b) = match what {
                // reverse the previous step
                0 => self.ring.read(2.0 * into + 1.0),
                // repeat a 1/64 slice
                1 => self.ring.read(into - (into % (sl / 4.0))),
                // crush
                2 => {
                    let q = 8.0;
                    ((l[i] * q).round() / q, (r[i] * q).round() / q)
                }
                // drop out
                _ => (0.0, 0.0),
            };
            let f = (into / 48.0).min(1.0) * ((sl - into) / 48.0).clamp(0.0, 1.0);
            l[i] = l[i] * (1.0 - f) + a * f;
            r[i] = r[i] * (1.0 - f) + b * f;
        }
    }
}
