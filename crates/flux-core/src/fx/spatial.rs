//! 3D and 8D: a parametric binaural renderer. Each virtual source gets interaural time delay
//! (Woodworth), head shadow on the far ear, a rear cue, an elevation notch, distance gain and
//! air absorption. Motion is a function of timeline position, so preview and export match.

use super::space::Reverb;
use super::{Ctx, Processor, P};
use crate::dsp::{Biquad, Delay, FilterType, OnePole, TAU};
use std::f32::consts::PI;

struct Ear {
    d: Delay,
    shadow: OnePole,
    rear: Biquad,
    elev: Biquad,
}
struct Src {
    ears: [Ear; 2],
}
impl Src {
    fn new(sr: f32) -> Self {
        let mk = || Ear {
            d: Delay::new((sr * 0.05) as usize),
            shadow: OnePole::new(sr, 20000.0),
            rear: Biquad::default(),
            elev: Biquad::default(),
        };
        Src { ears: [mk(), mk()] }
    }
}

pub struct Spatial {
    sr: f32,
    p: P,
    src: [Src; 2],
    xo: [[Biquad; 4]; 2],
    room: Reverb,
    rl: Vec<f32>,
    rr: Vec<f32>,
    path: Vec<[f32; 2]>,
    sa: (f32, f32),
    sd: f32,
    air: [OnePole; 2],
    ctr: usize,
}

impl Spatial {
    pub fn new(sr: f32, path: Vec<[f32; 2]>) -> Self {
        let mut room = Reverb::new(sr);
        room.set_rt(0.7, 0.5, 6000.0);
        Spatial {
            sr,
            p: [0.0; 32],
            src: [Src::new(sr), Src::new(sr)],
            xo: [[Biquad::default(); 4]; 2],
            room,
            rl: vec![0.0; super::BLOCK],
            rr: vec![0.0; super::BLOCK],
            path,
            sa: (0.0, 1.0),
            sd: 1.8,
            air: [OnePole::new(sr, 20000.0); 2],
            ctr: 0,
        }
    }

    /// Azimuth (radians, 0 = front, positive = right) and distance scale at time `t` seconds.
    pub fn motion(p: &P, path: &[[f32; 2]], t: f32) -> (f32, f32) {
        let mode = p[0].round() as i32;
        let rate = p[1];
        let dir = match p[2].round() as i32 {
            1 => -1.0,
            _ => 1.0,
        };
        let a = TAU * rate * t;
        let base = p[7].to_radians();
        match mode {
            1 => (base + dir * a, 0.55 + 0.45 * (a * 0.6).sin()),
            2 => {
                let (x, y) = (a.sin(), -(2.0 * a).sin() * 0.6);
                (x.atan2(y), (x.hypot(y) * 0.9).max(0.2))
            }
            3 => {
                let b = a * 1.3;
                (
                    (b.sin() * 0.7 + (b * 2.9).sin() * 0.2).atan2((b * 1.7).cos() * 0.7),
                    0.3 + 0.6 * (b * 0.8).sin().abs(),
                )
            }
            4 => (base, 1.0),
            5 if path.len() > 1 => {
                let u = ((rate.max(0.01) * t).fract() * path.len() as f32) as usize % path.len();
                let q = path[u];
                // path is drawn top-down: x right, y down (front is up)
                (q[0].atan2(-q[1]), q[0].hypot(q[1]).clamp(0.16, 1.0))
            }
            _ => {
                if p[2].round() as i32 == 2 {
                    (base + (PI / 2.0) * a.sin(), 1.0)
                } else {
                    (base + dir * a, 1.0)
                }
            }
        }
    }
}

impl Processor for Spatial {
    fn set(&mut self, p: &P) {
        if p[9] != self.p[9] || self.ctr == 0 {
            for ch in 0..2 {
                for k in 0..2 {
                    self.xo[ch][k].set(
                        FilterType::LowPass,
                        self.sr,
                        p[9],
                        std::f32::consts::FRAC_1_SQRT_2,
                        0.0,
                    );
                    self.xo[ch][k + 2].set(
                        FilterType::HighPass,
                        self.sr,
                        p[9],
                        std::f32::consts::FRAC_1_SQRT_2,
                        0.0,
                    );
                }
            }
        }
        self.p = *p;
        self.ctr = 1;
    }
    fn reset(&mut self) {
        self.ctr = 0;
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], ctx: &Ctx) {
        let p = self.p;
        let n = l.len();
        let elev = p[4].to_radians();
        let depth = p[5] / 100.0;
        let smooth = (-1.0 / (self.sr * 0.001 * (5.0 + p[6] * 4.0))).exp();
        let bass_mono = p[8] > 0.5;
        let doppler = p[10] > 0.5;
        let midside = p[11].round() as i32 == 1;
        let head_r = p[12] / 200.0;
        let room = p[13] / 100.0;
        let lite = p[14] > 0.5;
        let dist = p[3];
        let mut lows = [0.0f32; super::BLOCK];
        for i in 0..n {
            let t = (ctx.pos + i as i64) as f32 / self.sr;
            let (az, ds) = Spatial::motion(&p, &self.path, t);
            // smooth on the unit circle so wrap-around is seamless
            let (s, c) = az.sin_cos();
            self.sa.0 = s + smooth * (self.sa.0 - s);
            self.sa.1 = c + smooth * (self.sa.1 - c);
            self.sd = dist * ds + smooth * (self.sd - dist * ds);
            let az = self.sa.0.atan2(self.sa.1);
            let d = self.sd.max(0.3);

            let (mut a, mut b) = (l[i], r[i]);
            if bass_mono {
                let lo = ({
                    let t_ = self.xo[0][0].run(a);
                    self.xo[0][1].run(t_)
                } + {
                    let t_ = self.xo[1][0].run(b);
                    self.xo[1][1].run(t_)
                }) * 0.5;
                lows[i] = lo;
                a = {
                    let t_ = self.xo[0][2].run(a);
                    self.xo[0][3].run(t_)
                };
                b = {
                    let t_ = self.xo[1][2].run(b);
                    self.xo[1][3].run(t_)
                };
            }
            let (inputs, side): ([(f32, f32); 2], f32) = if midside {
                ([((a + b) * 0.5, az), (0.0, az)], (a - b) * 0.5)
            } else {
                ([(a, az - 0.52), (b, az + 0.52)], 0.0)
            };
            let mut out = [0.0f32; 2];
            let gain = (1.8 / d).clamp(0.12, 2.0);
            for (si, (x, saz)) in inputs.iter().enumerate() {
                if midside && si == 1 {
                    continue;
                }
                let lat = (saz.sin() * elev.cos()).clamp(-1.0, 1.0);
                let theta = lat.asin();
                let itd = head_r / 343.0 * (theta.abs() + theta.abs().sin()) * self.sr;
                let front = saz.cos();
                let common = if doppler { d / 343.0 * self.sr } else { 0.0 };
                let src = &mut self.src[si];
                for (e, ear) in src.ears.iter_mut().enumerate() {
                    // e = 0 left, 1 right; far ear is the one on the other side of the source
                    let far = (e == 0 && lat > 0.0) || (e == 1 && lat < 0.0);
                    let amt = lat.abs() * depth;
                    if self.ctr % 32 == 1 || i == 0 {
                        ear.shadow.set(
                            self.sr,
                            if far {
                                20000.0 - 18500.0 * amt.powf(0.8)
                            } else {
                                20000.0
                            },
                        );
                        if !lite {
                            ear.rear.set(
                                FilterType::HighShelf,
                                self.sr,
                                4000.0,
                                0.7,
                                if front < 0.0 {
                                    -6.0 * -front * depth
                                } else {
                                    0.0
                                },
                            );
                            ear.elev.set(
                                FilterType::Bell,
                                self.sr,
                                8000.0 + 2500.0 * elev.sin(),
                                2.0,
                                -8.0 * elev.abs() / (PI / 2.0) * depth,
                            );
                        }
                    }
                    ear.d.push(*x);
                    let dly = common + if far { itd * depth } else { 0.0 };
                    let mut y = ear.shadow.lp(ear.d.frac(dly));
                    if !lite {
                        y = ear.elev.run(ear.rear.run(y));
                    }
                    let g = if far {
                        1.0 - 0.35 * amt
                    } else {
                        1.0 + 0.15 * amt
                    };
                    out[e] += y * g * gain;
                }
            }
            self.ctr += 1;
            let wet_l = self.air[0].lp(out[0]);
            let wet_r = self.air[1].lp(out[1]);
            let (mut ol, mut or) = (
                wet_l * depth + a * (1.0 - depth) + side,
                wet_r * depth + b * (1.0 - depth) - side,
            );
            if bass_mono {
                ol += lows[i];
                or += lows[i];
            }
            self.rl[i] = (ol + or) * 0.5;
            self.rr[i] = (ol + or) * 0.5;
            l[i] = ol;
            r[i] = or;
        }
        let air_f = 20000.0 - 1400.0 * (dist - 0.3).max(0.0);
        self.air[0].set(self.sr, air_f.max(5000.0));
        self.air[1].set(self.sr, air_f.max(5000.0));
        let send = (room + (dist / 10.0) * 0.3).min(1.0);
        if send > 0.001 {
            let (wl, wr) = (&mut self.rl[..n], &mut self.rr[..n]);
            self.room.process(wl, wr, ctx);
            for i in 0..n {
                l[i] += wl[i] * send * 0.6;
                r[i] += wr[i] * send * 0.6;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fx::{resolve, spec};
    fn energy(kind_params: &[(&str, f32)], t0: f32) -> (f32, f32) {
        let sr = 48000.0;
        let m = kind_params
            .iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect();
        let p = resolve(spec("spatial").unwrap(), &m);
        let mut s = Spatial::new(sr, vec![]);
        s.set(&p);
        let mut rng = crate::dsp::Rng::new(2);
        let (mut el, mut er) = (0.0, 0.0);
        let start = (t0 * sr) as i64;
        for b in 0..40 {
            let mut l: Vec<f32> = (0..512).map(|_| rng.bi() * 0.3).collect();
            let mut r = l.clone();
            s.process(
                &mut l,
                &mut r,
                &Ctx {
                    sr,
                    pos: start + b * 512,
                    len: 0,
                    tempo: 120.0,
                },
            );
            if b > 20 {
                el += l.iter().map(|x| x * x).sum::<f32>();
                er += r.iter().map(|x| x * x).sum::<f32>();
            }
        }
        (el, er)
    }
    #[test]
    fn placed_right_is_louder_on_right() {
        let (l, r) = energy(
            &[
                ("mode", 4.0),
                ("azimuth", 90.0),
                ("bass_mono", 0.0),
                ("smoothing", 0.0),
            ],
            0.0,
        );
        assert!(r > l * 1.5, "l {l} r {r}");
        let (l, r) = energy(
            &[
                ("mode", 4.0),
                ("azimuth", -90.0),
                ("bass_mono", 0.0),
                ("smoothing", 0.0),
            ],
            0.0,
        );
        assert!(l > r * 1.5, "l {l} r {r}");
    }
    #[test]
    fn orbit_moves_with_timeline() {
        let p = resolve(
            spec("spatial").unwrap(),
            &[("rate".to_string(), 0.25)].into_iter().collect(),
        );
        let (a0, _) = Spatial::motion(&p, &[], 0.0);
        let (a1, _) = Spatial::motion(&p, &[], 1.0);
        assert!((a1 - a0 - PI / 2.0).abs() < 1e-4);
    }
}
