//! Measurement: ITU-R BS.1770-4 loudness, true peak, loudness range, tempo, key, spectra,
//! silence and problem detection. Everything is computed from rendered samples.

use crate::dsp::{lin_to_db, run2, Biquad, TAU};
use rustfft::{num_complex::Complex, FftPlanner};
use serde::Serialize;

/// K-weighting pre-filter coefficients for any sample rate (BS.1770 via bilinear design).
pub fn k_weighting(sr: f32) -> [Biquad; 2] {
    let sr = sr as f64;
    // stage 1: high shelf
    let (f0, g, q) = (
        1681.974450955533f64,
        3.999843853973347f64,
        0.7071752369554196f64,
    );
    let k = (std::f64::consts::PI * f0 / sr).tan();
    let vh = 10f64.powf(g / 20.0);
    let vb = vh.powf(0.4996667741545416);
    let a0 = 1.0 + k / q + k * k;
    let s1 = Biquad::from_coefs(
        ((vh + vb * k / q + k * k) / a0) as f32,
        (2.0 * (k * k - vh) / a0) as f32,
        ((vh - vb * k / q + k * k) / a0) as f32,
        (2.0 * (k * k - 1.0) / a0) as f32,
        ((1.0 - k / q + k * k) / a0) as f32,
    );
    // stage 2: high pass
    let (f0, q) = (38.13547087602444f64, 0.5003270373238773f64);
    let k = (std::f64::consts::PI * f0 / sr).tan();
    let a0 = 1.0 + k / q + k * k;
    let s2 = Biquad::from_coefs(
        (1.0 / a0) as f32,
        (-2.0 / a0) as f32,
        (1.0 / a0) as f32,
        (2.0 * (k * k - 1.0) / a0) as f32,
        ((1.0 - k / q + k * k) / a0) as f32,
    );
    [s1, s2]
}

/// Mean-square energy of the K-weighted signal in 100 ms steps (summed over channels).
fn k_energies(l: &[f32], r: &[f32], sr: f32) -> Vec<f64> {
    let step = (sr * 0.1) as usize;
    let mut fl = k_weighting(sr);
    let mut fr = k_weighting(sr);
    let mut out = Vec::with_capacity(l.len() / step.max(1) + 1);
    let (mut acc, mut n) = (0f64, 0usize);
    for i in 0..l.len() {
        let a = run2(&mut fl, l[i]) as f64;
        let b = run2(&mut fr, r[i]) as f64;
        acc += a * a + b * b;
        n += 1;
        if n == step {
            out.push(acc / n as f64);
            acc = 0.0;
            n = 0;
        }
    }
    out
}

fn lufs(ms: f64) -> f64 {
    -0.691 + 10.0 * ms.max(1e-20).log10()
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Loudness {
    pub integrated: f64,
    pub short_term_max: f64,
    pub momentary_max: f64,
    pub lra: f64,
    pub true_peak: f64,
    pub sample_peak: f64,
    /// Short-term loudness every 0.5 s.
    pub curve: Vec<f32>,
}

pub fn gated(blocks: &[f64]) -> f64 {
    let abs: Vec<f64> = blocks
        .iter()
        .copied()
        .filter(|e| lufs(*e) > -70.0)
        .collect();
    if abs.is_empty() {
        return -70.0;
    }
    let mean = abs.iter().sum::<f64>() / abs.len() as f64;
    let rel = lufs(mean) - 10.0;
    let g: Vec<f64> = abs.into_iter().filter(|e| lufs(*e) > rel).collect();
    if g.is_empty() {
        return -70.0;
    }
    lufs(g.iter().sum::<f64>() / g.len() as f64)
}

pub fn loudness(l: &[f32], r: &[f32], sr: f32) -> Loudness {
    let e = k_energies(l, r, sr);
    let win = |n: usize, hop: usize| -> Vec<f64> {
        if e.len() < n {
            return if e.is_empty() {
                vec![]
            } else {
                vec![e.iter().sum::<f64>() / e.len() as f64]
            };
        }
        (0..=e.len() - n)
            .step_by(hop)
            .map(|i| e[i..i + n].iter().sum::<f64>() / n as f64)
            .collect()
    };
    let m = win(4, 1); // 400 ms, 75% overlap
    let st = win(30, 1); // 3 s
    let integrated = gated(&m);
    // LRA (EBU Tech 3342): short-term, abs gate -70, rel gate -20, 10th..95th percentile
    let stl: Vec<f64> = st.iter().map(|v| lufs(*v)).filter(|v| *v > -70.0).collect();
    let lra = if stl.len() > 1 {
        let mean =
            lufs(st.iter().copied().filter(|v| lufs(*v) > -70.0).sum::<f64>() / stl.len() as f64);
        let mut g: Vec<f64> = stl.into_iter().filter(|v| *v > mean - 20.0).collect();
        g.sort_by(|a, b| a.partial_cmp(b).unwrap());
        if g.len() > 1 {
            let p = |q: f64| g[((g.len() - 1) as f64 * q).round() as usize];
            p(0.95) - p(0.10)
        } else {
            0.0
        }
    } else {
        0.0
    };
    let curve = win(30, 5)
        .iter()
        .map(|v| lufs(*v).max(-70.0) as f32)
        .collect();
    Loudness {
        integrated,
        short_term_max: st.iter().map(|v| lufs(*v)).fold(-70.0, f64::max),
        momentary_max: m.iter().map(|v| lufs(*v)).fold(-70.0, f64::max),
        lra,
        true_peak: lin_to_db(true_peak(l).max(true_peak(r))) as f64,
        sample_peak: lin_to_db(l.iter().chain(r).fold(0f32, |m, v| m.max(v.abs()))) as f64,
        curve,
    }
}

/// 4x oversampled peak (BS.1770 Annex 2) with a 48-tap windowed-sinc interpolator.
pub fn true_peak(x: &[f32]) -> f32 {
    const TAPS: usize = 12;
    const OS: usize = 4;
    let mut h = [[0f32; TAPS]; OS];
    for (p, hp) in h.iter_mut().enumerate() {
        for (k, v) in hp.iter_mut().enumerate() {
            let t = k as f32 - (TAPS / 2) as f32 + 1.0 - p as f32 / OS as f32;
            let sinc = if t.abs() < 1e-6 {
                1.0
            } else {
                (std::f32::consts::PI * t).sin() / (std::f32::consts::PI * t)
            };
            let w = 0.5 + 0.5 * (std::f32::consts::PI * t / (TAPS as f32 / 2.0)).cos();
            *v = sinc * w;
        }
    }
    let mut peak = 0f32;
    for i in 0..x.len() {
        peak = peak.max(x[i].abs());
        if i + TAPS / 2 >= x.len() || i < TAPS / 2 {
            continue;
        }
        for hp in h.iter().skip(1) {
            let mut s = 0.0;
            for (k, c) in hp.iter().enumerate() {
                s += x[i + k + 1 - TAPS / 2] * c;
            }
            peak = peak.max(s.abs());
        }
    }
    peak
}

/// Running loudness for live meters: momentary (400 ms) and short-term (3 s). Fixed memory.
pub struct LiveLoudness {
    f: [[Biquad; 2]; 2],
    step: usize,
    acc: f64,
    n: usize,
    ring: [f64; 30],
    ri: usize,
    filled: usize,
    pub momentary: f32,
    pub short_term: f32,
    sum_e: f64,
    sum_n: f64,
}
impl LiveLoudness {
    pub fn new(sr: f32) -> Self {
        LiveLoudness {
            f: [k_weighting(sr), k_weighting(sr)],
            step: (sr * 0.1) as usize,
            acc: 0.0,
            n: 0,
            ring: [0.0; 30],
            ri: 0,
            filled: 0,
            momentary: -70.0,
            short_term: -70.0,
            sum_e: 0.0,
            sum_n: 0.0,
        }
    }
    pub fn reset(&mut self) {
        self.ring = [0.0; 30];
        self.filled = 0;
        self.sum_e = 0.0;
        self.sum_n = 0.0;
        self.momentary = -70.0;
        self.short_term = -70.0;
    }
    /// Ungated running integrated loudness since the last reset (cheap live estimate).
    pub fn integrated(&self) -> f32 {
        if self.sum_n == 0.0 {
            -70.0
        } else {
            lufs(self.sum_e / self.sum_n) as f32
        }
    }
    pub fn push(&mut self, l: &[f32], r: &[f32]) {
        for i in 0..l.len() {
            let a = run2(&mut self.f[0], l[i]) as f64;
            let b = run2(&mut self.f[1], r[i]) as f64;
            self.acc += a * a + b * b;
            self.n += 1;
            if self.n == self.step {
                let e = self.acc / self.n as f64;
                self.ring[self.ri] = e;
                self.ri = (self.ri + 1) % 30;
                self.filled = (self.filled + 1).min(30);
                self.acc = 0.0;
                self.n = 0;
                let m: f64 = (1..=4.min(self.filled))
                    .map(|k| self.ring[(self.ri + 30 - k) % 30])
                    .sum::<f64>()
                    / 4.0;
                let s: f64 = self.ring.iter().sum::<f64>() / self.filled.max(1) as f64;
                self.momentary = lufs(m).max(-70.0) as f32;
                self.short_term = lufs(s).max(-70.0) as f32;
                if self.momentary > -70.0 {
                    self.sum_e += e;
                    self.sum_n += 1.0;
                }
            }
        }
    }
}

/// Magnitude spectrum (dB) on `bins` log-spaced bands from 20 Hz to 20 kHz.
pub fn spectrum(x: &[f32], sr: f32, bins: usize) -> Vec<f32> {
    let n = 4096.min(x.len().next_power_of_two()).max(256);
    if x.len() < 64 {
        return vec![-120.0; bins];
    }
    let fft = FftPlanner::new().plan_fft_forward(n);
    let mut acc = vec![0f32; n / 2];
    let hop = n / 2;
    let mut frames = 0;
    let mut buf = vec![Complex::new(0.0, 0.0); n];
    let w = crate::dsp::hann(n);
    let mut i = 0;
    while i + n <= x.len().max(n) && frames < 64 {
        for k in 0..n {
            buf[k] = Complex::new(x.get(i + k).copied().unwrap_or(0.0) * w[k], 0.0);
        }
        fft.process(&mut buf);
        for k in 0..n / 2 {
            acc[k] += buf[k].norm_sqr();
        }
        frames += 1;
        i += hop.max((x.len() / 64).max(hop));
    }
    let norm = 4.0 / (n as f32 * n as f32 * 0.25) / frames.max(1) as f32;
    log_bands(&acc, n, sr, bins)
        .into_iter()
        .map(|e| 10.0 * (e * norm + 1e-14).log10())
        .collect()
}

fn log_bands(pw: &[f32], n: usize, sr: f32, bins: usize) -> Vec<f32> {
    let (lo, hi) = (20f32.ln(), (sr * 0.5).min(20000.0).ln());
    (0..bins)
        .map(|b| {
            let f0 = (lo + (hi - lo) * b as f32 / bins as f32).exp();
            let f1 = (lo + (hi - lo) * (b + 1) as f32 / bins as f32).exp();
            let k0 = ((f0 * n as f32 / sr) as usize).min(pw.len() - 1);
            let k1 = ((f1 * n as f32 / sr) as usize).clamp(k0 + 1, pw.len());
            pw[k0..k1].iter().copied().fold(0.0, f32::max)
        })
        .collect()
}

/// Spectrogram: `cols` x `rows` of 0..255 levels (log frequency), mono of the input.
pub fn spectrogram(x: &[f32], sr: f32, cols: usize, rows: usize, n: usize) -> Vec<u8> {
    let cols = cols.clamp(1, 2000);
    let rows = rows.clamp(8, 512);
    let n = n.clamp(256, 16384).next_power_of_two();
    let fft = FftPlanner::new().plan_fft_forward(n);
    let w = crate::dsp::hann(n);
    let mut out = vec![0u8; cols * rows];
    let mut buf = vec![Complex::new(0.0, 0.0); n];
    let mut pw = vec![0f32; n / 2];
    for c in 0..cols {
        let center = (x.len() as f64 * (c as f64 + 0.5) / cols as f64) as isize;
        for k in 0..n {
            let i = center - (n / 2) as isize + k as isize;
            let v = if i >= 0 && (i as usize) < x.len() {
                x[i as usize]
            } else {
                0.0
            };
            buf[k] = Complex::new(v * w[k], 0.0);
        }
        fft.process(&mut buf);
        for k in 0..n / 2 {
            pw[k] = buf[k].norm_sqr() * 16.0 / (n as f32 * n as f32);
        }
        let b = log_bands(&pw, n, sr, rows);
        for (r, e) in b.iter().enumerate() {
            let db = 10.0 * (e + 1e-14).log10();
            out[(rows - 1 - r) * cols + c] = (((db + 100.0) / 100.0).clamp(0.0, 1.0) * 255.0) as u8;
        }
    }
    out
}

/// Tempo by onset-envelope autocorrelation, 60..200 BPM. Returns (bpm, confidence 0..1, first beat sample).
pub fn tempo(x: &[f32], sr: f32) -> (f32, f32, i64) {
    let hop = 256usize;
    let n = 1024usize;
    if x.len() < n * 8 {
        return (120.0, 0.0, 0);
    }
    let fft = FftPlanner::new().plan_fft_forward(n);
    let w = crate::dsp::hann(n);
    let mut prev = vec![0f32; n / 2];
    let mut buf = vec![Complex::new(0.0, 0.0); n];
    let mut env = Vec::with_capacity(x.len() / hop);
    let mut i = 0;
    while i + n <= x.len() {
        for k in 0..n {
            buf[k] = Complex::new(x[i + k] * w[k], 0.0);
        }
        fft.process(&mut buf);
        let mut flux = 0.0;
        for k in 1..n / 2 {
            let m = (1.0 + 1000.0 * buf[k].norm()).ln();
            flux += (m - prev[k]).max(0.0);
            prev[k] = m;
        }
        env.push(flux);
        i += hop;
    }
    // remove the slow trend
    let mean: f32 = env.iter().sum::<f32>() / env.len() as f32;
    let env: Vec<f32> = env.iter().map(|v| (v - mean).max(0.0)).collect();
    let fps = sr / hop as f32;
    let (lag_lo, lag_hi) = ((fps * 60.0 / 200.0) as usize, (fps * 60.0 / 60.0) as usize);
    let mut best = (0usize, 0f32);
    let mut energy = 0f32;
    for v in &env {
        energy += v * v;
    }
    let mut ac = vec![0f32; lag_hi + 2];
    for (lag, a) in ac
        .iter_mut()
        .enumerate()
        .take(lag_hi + 1)
        .skip(lag_lo.max(1))
    {
        let mut s = 0.0;
        for k in lag..env.len() {
            s += env[k] * env[k - lag];
        }
        *a = s;
    }
    for lag in lag_lo.max(1)..=lag_hi {
        // weight towards 120 BPM (log-Gaussian), and add the double-lag peak
        let bpm = fps * 60.0 / lag as f32;
        let wgt = (-0.5 * ((bpm / 120.0).log2() / 0.9).powi(2)).exp();
        let near = |k: usize| {
            (k.saturating_sub(1)..=k + 1)
                .map(|j| ac.get(j).copied().unwrap_or(0.0))
                .fold(0f32, f32::max)
        };
        let s = (ac[lag] + 0.5 * near(lag * 2)) * wgt;
        if s > best.1 {
            best = (lag, s);
        }
    }
    let lag = best.0.max(1);
    // parabolic refinement
    let (a, b, c) = (ac[lag - 1], ac[lag], ac[(lag + 1).min(ac.len() - 1)]);
    let d = if a - 2.0 * b + c != 0.0 {
        0.5 * (a - c) / (a - 2.0 * b + c)
    } else {
        0.0
    };
    let bpm = fps * 60.0 / (lag as f32 + d.clamp(-0.5, 0.5));
    let conf = (ac[lag] / energy.max(1e-9)).clamp(0.0, 1.0);
    // beat phase: offset maximizing the comb sum
    let mut ph = (0usize, 0f32);
    for o in 0..lag {
        let s: f32 = (o..env.len()).step_by(lag).map(|k| env[k]).sum();
        if s > ph.1 {
            ph = (o, s);
        }
    }
    (
        (bpm * 10.0).round() / 10.0,
        conf,
        (ph.0 * hop + n / 2) as i64,
    )
}

pub const KEYS: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
const CAMELOT_MAJ: [&str; 12] = [
    "8B", "3B", "10B", "5B", "12B", "7B", "2B", "9B", "4B", "11B", "6B", "1B",
];
const CAMELOT_MIN: [&str; 12] = [
    "5A", "12A", "7A", "2A", "9A", "4A", "11A", "6A", "1A", "8A", "3A", "10A",
];

#[derive(Serialize, Clone, Debug)]
pub struct KeyResult {
    pub key: String,
    pub tonic: usize,
    pub minor: bool,
    pub camelot: String,
    pub confidence: f32,
    pub chroma: Vec<f32>,
}

/// Key by chroma correlation with Krumhansl-Kessler profiles.
pub fn key(x: &[f32], sr: f32) -> KeyResult {
    let n = 8192usize;
    let fft = FftPlanner::new().plan_fft_forward(n);
    let w = crate::dsp::hann(n);
    let mut chroma = [0f32; 12];
    let mut buf = vec![Complex::new(0.0, 0.0); n];
    let hop = (x.len() / 200).max(n / 2);
    let mut i = 0;
    while i + n <= x.len() {
        for k in 0..n {
            buf[k] = Complex::new(x[i + k] * w[k], 0.0);
        }
        fft.process(&mut buf);
        for (k, c) in buf.iter().enumerate().take(n / 2).skip(1) {
            let f = k as f32 * sr / n as f32;
            if !(55.0..=2000.0).contains(&f) {
                continue;
            }
            let midi = 69.0 + 12.0 * (f / 440.0).log2();
            let pc = (midi.round() as i32).rem_euclid(12) as usize;
            chroma[pc] += c.norm();
        }
        i += hop;
    }
    let maj = [
        6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88,
    ];
    let min = [
        6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17,
    ];
    let corr = |p: &[f32; 12], t: usize| -> f32 {
        let mp = p.iter().sum::<f32>() / 12.0;
        let mc = chroma.iter().sum::<f32>() / 12.0;
        let (mut s, mut a, mut b) = (0.0, 0.0, 0.0);
        for k in 0..12 {
            let x = chroma[(k + t) % 12] - mc;
            let y = p[k] - mp;
            s += x * y;
            a += x * x;
            b += y * y;
        }
        s / (a * b).sqrt().max(1e-9)
    };
    let mut res: Vec<(f32, usize, bool)> = vec![];
    for t in 0..12 {
        res.push((corr(&maj, t), t, false));
        res.push((corr(&min, t), t, true));
    }
    res.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let (c, t, m) = res[0];
    let margin = (res[0].0 - res[1].0).max(0.0);
    let mx = chroma.iter().copied().fold(1e-9, f32::max);
    KeyResult {
        key: format!("{} {}", KEYS[t], if m { "minor" } else { "major" }),
        tonic: t,
        minor: m,
        camelot: if m { CAMELOT_MIN[t] } else { CAMELOT_MAJ[t] }.to_string(),
        confidence: (c.max(0.0) * 0.6 + margin * 4.0).clamp(0.0, 1.0),
        chroma: chroma.iter().map(|v| v / mx).collect(),
    }
}

/// Regions quieter than `thresh_db` for at least `min_len` samples: (start, end).
pub fn silences(l: &[f32], r: &[f32], sr: f32, thresh_db: f32, min_len: usize) -> Vec<(i64, i64)> {
    let win = (sr * 0.01) as usize;
    let th = crate::dsp::db_to_lin(thresh_db);
    let mut out = vec![];
    let mut start: Option<usize> = None;
    let mut i = 0;
    while i < l.len() {
        let e = i + win.min(l.len() - i);
        let mut s = 0f32;
        for k in i..e {
            s += l[k] * l[k] + r[k] * r[k];
        }
        let rms = (s / (2 * (e - i)) as f32).sqrt();
        if rms < th {
            if start.is_none() {
                start = Some(i);
            }
        } else if let Some(s0) = start.take() {
            if i - s0 >= min_len {
                out.push((s0 as i64, i as i64));
            }
        }
        i = e;
    }
    if let Some(s0) = start {
        if l.len() - s0 >= min_len {
            out.push((s0 as i64, l.len() as i64));
        }
    }
    out
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Problems {
    pub noise_floor_db: f32,
    pub hum_hz: Option<f32>,
    pub hum_db: f32,
    pub clipped_samples: usize,
    pub clicks: usize,
    pub dc_offset: f32,
    pub correlation: f32,
    pub width: f32,
    pub sibilance_db: f32,
    pub low_end_db: f32,
    pub crest_db: f32,
    pub reverb_s: f32,
}

fn goertzel_power(x: &[f32], sr: f32, f: f32) -> f32 {
    let c = 2.0 * (TAU * f / sr).cos();
    let (mut s1, mut s2) = (0f32, 0f32);
    for v in x {
        let s0 = v + c * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    (s1 * s1 + s2 * s2 - c * s1 * s2) / (x.len() as f32 * x.len() as f32) * 4.0
}

pub fn problems(l: &[f32], r: &[f32], sr: f32) -> Problems {
    let n = l.len();
    if n == 0 {
        return Problems::default();
    }
    let mono: Vec<f32> = l.iter().zip(r).map(|(a, b)| (a + b) * 0.5).collect();
    // noise floor: 10th percentile of 50 ms RMS (ignoring digital silence)
    let w = (sr * 0.05) as usize;
    let mut rms: Vec<f32> = mono
        .chunks(w.max(1))
        .map(|c| (c.iter().map(|v| v * v).sum::<f32>() / c.len() as f32).sqrt())
        .filter(|v| *v > 1e-6)
        .collect();
    rms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let noise_floor_db = rms
        .get(rms.len() / 10)
        .map(|v| lin_to_db(*v))
        .unwrap_or(-120.0);
    // hum: power at 50/60 Hz and harmonics versus neighbours, on the quietest-but-not-silent second
    let seg = &mono[..n.min(sr as usize * 20)];
    let score = |f0: f32| -> f32 {
        let mut s = 0.0;
        for h in 1..=3 {
            let on = goertzel_power(seg, sr, f0 * h as f32);
            let off = 0.5
                * (goertzel_power(seg, sr, f0 * h as f32 - 7.0)
                    + goertzel_power(seg, sr, f0 * h as f32 + 7.0));
            s += 10.0 * ((on + 1e-14) / (off + 1e-14)).log10();
        }
        s / 3.0
    };
    let (s50, s60) = (score(50.0), score(60.0));
    let (hum_hz, hum_db) = if s50.max(s60) > 9.0 {
        (Some(if s50 >= s60 { 50.0 } else { 60.0 }), s50.max(s60))
    } else {
        (None, s50.max(s60))
    };
    let clipped_samples = l.iter().chain(r).filter(|v| v.abs() >= 0.999).count();
    // clicks: second difference outliers
    let mut d2: Vec<f32> = mono
        .windows(3)
        .map(|w| (w[0] - 2.0 * w[1] + w[2]).abs())
        .collect();
    let mean_d2 = d2.iter().sum::<f32>() / d2.len().max(1) as f32;
    let clicks = {
        let th = (mean_d2 * 25.0).max(0.2);
        let mut c = 0;
        let mut last = 0usize;
        for (i, v) in d2.iter().enumerate() {
            if *v > th && i > last + 64 {
                c += 1;
                last = i;
            }
        }
        c
    };
    d2.clear();
    let dc_offset = mono.iter().sum::<f32>() / n as f32;
    let (mut lr, mut ll, mut rr, mut mm, mut ss) = (0f64, 0f64, 0f64, 0f64, 0f64);
    for i in 0..n {
        let (a, b) = (l[i] as f64, r[i] as f64);
        lr += a * b;
        ll += a * a;
        rr += b * b;
        mm += ((a + b) * 0.5).powi(2);
        ss += ((a - b) * 0.5).powi(2);
    }
    let correlation = (lr / (ll * rr).sqrt().max(1e-12)) as f32;
    let width = (ss / (mm + ss).max(1e-12)) as f32 * 2.0;
    let sp = spectrum(&mono, sr, 48);
    let band = |f0: f32, f1: f32| -> f32 {
        let (lo, hi) = (20f32.ln(), (sr * 0.5).min(20000.0).ln());
        let mut s = -120f32;
        for (b, v) in sp.iter().enumerate() {
            let f = (lo + (hi - lo) * (b as f32 + 0.5) / 48.0).exp();
            if f >= f0 && f < f1 {
                s = s.max(*v);
            }
        }
        s
    };
    let mid = band(500.0, 2000.0);
    let sibilance_db = band(5000.0, 9000.0) - mid;
    let low_end_db = band(30.0, 120.0) - mid;
    let peak = mono.iter().fold(0f32, |m, v| m.max(v.abs()));
    let total_rms = (mono.iter().map(|v| v * v).sum::<f32>() / n as f32).sqrt();
    let crest_db = lin_to_db(peak) - lin_to_db(total_rms);
    // reverberance: median decay slope after energy drops (rough estimate)
    let env: Vec<f32> = mono
        .chunks((sr * 0.01) as usize)
        .map(|c| lin_to_db((c.iter().map(|v| v * v).sum::<f32>() / c.len() as f32).sqrt()))
        .collect();
    let mut slopes = vec![];
    let mut i = 0;
    while i + 30 < env.len() {
        if env[i] > noise_floor_db + 25.0 && env[i + 1] < env[i] - 1.0 {
            let mut j = i + 1;
            while j < env.len() - 1 && env[j + 1] < env[j] + 0.5 && env[j] > noise_floor_db + 5.0 {
                j += 1;
            }
            let drop = env[i] - env[j];
            if j - i >= 5 && drop > 10.0 {
                slopes.push(drop / ((j - i) as f32 * 0.01));
            }
            i = j;
        }
        i += 1;
    }
    slopes.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let reverb_s = slopes
        .get(slopes.len() / 2)
        .map(|s| 60.0 / s)
        .unwrap_or(0.0)
        .min(10.0);
    Problems {
        noise_floor_db,
        hum_hz,
        hum_db,
        clipped_samples,
        clicks,
        dc_offset,
        correlation,
        width,
        sibilance_db,
        low_end_db,
        crest_db,
        reverb_s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sine(f: f32, amp: f32, secs: f32, sr: f32) -> Vec<f32> {
        (0..(secs * sr) as usize)
            .map(|i| (TAU * f * i as f32 / sr).sin() * amp)
            .collect()
    }
    #[test]
    fn bs1770_reference_sine() {
        // 1 kHz sine, RMS 0.1 (-20 dB) in both channels: -0.691 + 10*log10(0.02) + K gain (+0.69 dB) = -17.0 LUFS
        for sr in [44100.0, 48000.0, 96000.0] {
            let x = sine(1000.0, 0.1 * 2f32.sqrt(), 10.0, sr);
            let l = loudness(&x, &x, sr);
            assert!(
                (l.integrated - (-17.0)).abs() < 0.15,
                "sr {sr} integrated {}",
                l.integrated
            );
        }
    }
    #[test]
    fn mono_sine_minus_23() {
        // one channel, 1 kHz sine whose RMS is -23 dB reads -23 LUFS (K gain and -0.691 cancel)
        let sr = 48000.0;
        let amp = crate::dsp::db_to_lin(-23.0 + 3.01);
        let x = sine(1000.0, amp, 20.0, sr);
        let z = vec![0.0; x.len()];
        let l = loudness(&x, &z, sr);
        assert!((l.integrated + 23.0).abs() < 0.1, "{}", l.integrated);
        assert!(l.lra < 0.5);
    }
    #[test]
    fn true_peak_exceeds_sample_peak() {
        let sr = 48000.0;
        let x: Vec<f32> = (0..48000)
            .map(|i| (TAU * 12000.0 * i as f32 / sr + 0.785).sin() * 0.5)
            .collect();
        let sp = x.iter().fold(0f32, |m, v| m.max(v.abs()));
        let tp = true_peak(&x);
        assert!(tp > sp + 0.05 && (tp - 0.5).abs() < 0.03, "tp {tp} sp {sp}");
    }
    #[test]
    fn tempo_of_click_track() {
        let sr = 44100.0;
        for bpm in [90.0f32, 120.0, 128.0, 150.0] {
            let n = (sr * 20.0) as usize;
            let period = (sr * 60.0 / bpm) as usize;
            let mut x = vec![0f32; n];
            let mut rng = crate::dsp::Rng::new(1);
            for b in (0..n).step_by(period) {
                for k in 0..800.min(n - b) {
                    x[b + k] += rng.bi() * (-(k as f32) / 120.0).exp() * 0.8;
                }
            }
            let (t, _, _) = tempo(&x, sr);
            assert!((t - bpm).abs() < 1.5, "want {bpm} got {t}");
        }
    }
    #[test]
    fn key_of_triads() {
        let sr = 44100.0;
        let note = |m: i32| 440.0 * 2f32.powf((m - 69) as f32 / 12.0);
        // C major progression: C F G C ; A minor: Am Dm E Am
        let prog = |chords: &[[i32; 3]]| -> Vec<f32> {
            let mut out = vec![];
            for c in chords {
                let seg: Vec<f32> = (0..(sr as usize))
                    .map(|i| {
                        c.iter()
                            .map(|m| (TAU * note(*m) * i as f32 / sr).sin() * 0.2)
                            .sum()
                    })
                    .collect();
                out.extend(seg);
            }
            out
        };
        let c = prog(&[
            [60, 64, 67],
            [65, 69, 72],
            [67, 71, 74],
            [60, 64, 67],
            [60, 64, 67],
        ]);
        let k = key(&c, sr);
        assert_eq!(k.key, "C major", "{:?}", k.key);
        let a = prog(&[
            [57, 60, 64],
            [62, 65, 69],
            [64, 68, 71],
            [57, 60, 64],
            [57, 60, 64],
        ]);
        assert_eq!(key(&a, sr).key, "A minor");
    }
    #[test]
    fn silence_and_hum_detection() {
        let sr = 48000.0;
        let mut x = sine(440.0, 0.3, 1.0, sr);
        x.extend(vec![0.0; 48000]);
        x.extend(sine(440.0, 0.3, 1.0, sr));
        let s = silences(&x, &x, sr, -50.0, 24000);
        assert_eq!(s.len(), 1);
        assert!(
            (s[0].0 - 48000).abs() < 600 && (s[0].1 - 96000).abs() < 600,
            "{s:?}"
        );
        let mut rng = crate::dsp::Rng::new(3);
        let h: Vec<f32> = (0..(sr as usize * 4))
            .map(|i| (TAU * 60.0 * i as f32 / sr).sin() * 0.05 + rng.bi() * 0.01)
            .collect();
        assert_eq!(problems(&h, &h, sr).hum_hz, Some(60.0));
    }
    #[test]
    fn live_meter_matches_offline() {
        let sr = 48000.0;
        let x = sine(1000.0, 0.2, 5.0, sr);
        let mut lv = LiveLoudness::new(sr);
        for c in x.chunks(441) {
            lv.push(c, c);
        }
        let off = loudness(&x, &x, sr).integrated as f32;
        assert!(
            (lv.short_term - off).abs() < 0.2,
            "{} {}",
            lv.short_term,
            off
        );
    }
}
