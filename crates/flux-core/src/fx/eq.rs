use super::{Ctx, Processor, P};
use crate::dsp::{db_to_lin, Biquad, FilterType};

pub fn band_type(i: f32) -> FilterType {
    match i.round() as i32 {
        1 => FilterType::LowShelf,
        2 => FilterType::HighShelf,
        3 => FilterType::LowPass,
        4 => FilterType::HighPass,
        5 => FilterType::Notch,
        _ => FilterType::Bell,
    }
}

pub struct Eq {
    sr: f32,
    l: [Biquad; 5],
    r: [Biquad; 5],
    active: [bool; 5],
    out: f32,
    last: P,
}
impl Eq {
    pub fn new(sr: f32) -> Self {
        Eq {
            sr,
            l: [Biquad::default(); 5],
            r: [Biquad::default(); 5],
            active: [false; 5],
            out: 1.0,
            last: [f32::NAN; 32],
        }
    }
}
impl Processor for Eq {
    fn set(&mut self, p: &P) {
        if p[..21] == self.last[..21] {
            return;
        }
        self.last = *p;
        for b in 0..5 {
            let (t, f, g, q) = (
                band_type(p[b * 4]),
                p[b * 4 + 1],
                p[b * 4 + 2],
                p[b * 4 + 3],
            );
            self.l[b].set(t, self.sr, f, q, g);
            self.r[b].set(t, self.sr, f, q, g);
            self.active[b] = !matches!(
                t,
                FilterType::Bell | FilterType::LowShelf | FilterType::HighShelf
            ) || g.abs() > 0.01;
        }
        self.out = db_to_lin(p[20]);
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        for i in 0..l.len() {
            let (mut a, mut b) = (l[i], r[i]);
            for k in 0..5 {
                if self.active[k] {
                    a = self.l[k].run(a);
                    b = self.r[k].run(b);
                }
            }
            l[i] = a * self.out;
            r[i] = b * self.out;
        }
    }
}

pub const GEQ_FREQS: [f32; 10] = [
    31.0, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];
pub struct Geq {
    sr: f32,
    l: [Biquad; 10],
    r: [Biquad; 10],
    g: [f32; 10],
}
impl Geq {
    pub fn new(sr: f32) -> Self {
        Geq {
            sr,
            l: [Biquad::default(); 10],
            r: [Biquad::default(); 10],
            g: [f32::NAN; 10],
        }
    }
}
impl Processor for Geq {
    fn set(&mut self, p: &P) {
        for k in 0..10 {
            if p[k] != self.g[k] {
                self.g[k] = p[k];
                self.l[k].set(FilterType::Bell, self.sr, GEQ_FREQS[k], 1.4, p[k]);
                self.r[k].set(FilterType::Bell, self.sr, GEQ_FREQS[k], 1.4, p[k]);
            }
        }
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &Ctx) {
        for i in 0..l.len() {
            for k in 0..10 {
                if self.g[k].abs() > 0.01 {
                    l[i] = self.l[k].run(l[i]);
                    r[i] = self.r[k].run(r[i]);
                }
            }
        }
    }
}

/// Response of the parametric EQ in dB at `f` for UI curves and tests.
pub fn eq_response(params: &P, sr: f32, f: f32) -> f32 {
    let mut s = params[20];
    for b in 0..5 {
        let bq = Biquad::new(
            band_type(params[b * 4]),
            sr,
            params[b * 4 + 1],
            params[b * 4 + 3],
            params[b * 4 + 2],
        );
        s += bq.response_db(sr, f);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fx::{resolve, spec};
    #[test]
    fn eq_boost_measured_on_rendered_sine() {
        let sr = 48000.0;
        let mut m = std::collections::BTreeMap::new();
        m.insert("b3_gain".to_string(), 6.0);
        m.insert("b3_freq".to_string(), 1000.0);
        let p = resolve(spec("eq").unwrap(), &m);
        let mut e = Eq::new(sr);
        e.set(&p);
        let mut l: Vec<f32> = (0..48000)
            .map(|i| (crate::dsp::TAU * 1000.0 * i as f32 / sr).sin() * 0.25)
            .collect();
        let mut r = l.clone();
        for (a, b) in l.chunks_mut(512).zip(r.chunks_mut(512)) {
            e.process(
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
        let rms = (l[24000..].iter().map(|v| v * v).sum::<f32>() / 24000.0).sqrt();
        let gain = crate::dsp::lin_to_db(rms / (0.25 / 2f32.sqrt()));
        assert!((gain - 6.0).abs() < 0.2, "measured {gain} dB");
    }
}
