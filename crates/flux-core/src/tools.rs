//! Processing tools built from DSP and analysis. None of these use a neural model: the stem
//! split is a mid/side + harmonic/percussive spectral split, labelled as such in the UI.

use crate::analysis::{self, Problems};
use crate::dsp::{hann, lin_to_db};
use rustfft::{num_complex::Complex, FftPlanner};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub type Stereo = (Vec<f32>, Vec<f32>);

fn median(v: &mut [f32]) -> f32 {
    let m = v.len() / 2;
    *v.select_nth_unstable_by(m, |a, b| {
        a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
    })
    .1
}

/// Split stereo audio into stems. `mode` "4" gives vocals, drums, bass, other; "2" gives vocals
/// and instrumental. Stems always sum back to the input exactly ("other" is the residual).
pub fn stems(
    l: &[f32],
    r: &[f32],
    sr: f32,
    mode: &str,
    progress: &mut dyn FnMut(f32) -> bool,
) -> Option<Vec<(String, Stereo)>> {
    const N: usize = 2048;
    const H: usize = N / 4;
    const K: usize = 17;
    let n = l.len();
    let fft = FftPlanner::new().plan_fft_forward(N);
    let ifft = FftPlanner::new().plan_fft_inverse(N);
    let w = hann(N);
    let bins = N / 2 + 1;
    let seg = (sr as usize * 20).max(N * 8);
    let pad = N * 2;
    let mut voc = (vec![0f32; n], vec![0f32; n]);
    let mut drm = (vec![0f32; n], vec![0f32; n]);
    let mut bas = (vec![0f32; n], vec![0f32; n]);
    let bin_hz = sr / N as f32;
    let mut s0 = 0usize;
    while s0 < n {
        if !progress(s0 as f32 / n as f32) {
            return None;
        }
        let a = s0.saturating_sub(pad);
        let z = (s0 + seg + pad).min(n);
        let frames = (z - a) / H + 1;
        let mut sl = vec![vec![Complex::new(0f32, 0f32); bins]; frames];
        let mut sr_ = sl.clone();
        let mut mag = vec![vec![0f32; bins]; frames];
        let mut buf = vec![Complex::new(0f32, 0f32); N];
        for f in 0..frames {
            for (ch, dst) in [(l, &mut sl[f]), (r, &mut sr_[f])] {
                for k in 0..N {
                    let i = a as isize + (f * H) as isize + k as isize - (N / 2) as isize;
                    let v = if i >= 0 && (i as usize) < n {
                        ch[i as usize]
                    } else {
                        0.0
                    };
                    buf[k] = Complex::new(v * w[k], 0.0);
                }
                fft.process(&mut buf);
                dst.copy_from_slice(&buf[..bins]);
            }
            for b in 0..bins {
                mag[f][b] = (sl[f][b] + sr_[f][b]).norm() * 0.5;
            }
        }
        // harmonic: median across time; percussive: median across frequency
        let mut hm = vec![vec![0f32; bins]; frames];
        let mut pm = vec![vec![0f32; bins]; frames];
        let mut tmp = [0f32; K];
        for f in 0..frames {
            for b in 0..bins {
                for (j, t) in tmp.iter_mut().enumerate() {
                    let ff = (f + j).saturating_sub(K / 2).min(frames - 1);
                    *t = mag[ff][b];
                }
                hm[f][b] = median(&mut tmp);
                for (j, t) in tmp.iter_mut().enumerate() {
                    let bb = (b + j).saturating_sub(K / 2).min(bins - 1);
                    *t = mag[f][bb];
                }
                pm[f][b] = median(&mut tmp);
            }
        }
        let mut outs = [
            (vec![0f32; z - a], vec![0f32; z - a]),
            (vec![0f32; z - a], vec![0f32; z - a]),
            (vec![0f32; z - a], vec![0f32; z - a]),
        ];
        let mut wsum = vec![0f32; z - a];
        for f in 0..frames {
            let start = (f * H) as isize - (N / 2) as isize;
            for (k, wk) in w.iter().enumerate().take(N) {
                let i = start + k as isize;
                if i >= 0 && (i as usize) < wsum.len() {
                    wsum[i as usize] += wk * wk;
                }
            }
            let mut masks = [[0f32; 1025]; 3];
            for b in 0..bins {
                let (x, y) = (sl[f][b], sr_[f][b]);
                let e = x.norm_sqr() + y.norm_sqr() + 1e-12;
                let center = (2.0 * (x * y.conj()).re.max(0.0) / e).clamp(0.0, 1.0);
                let h2 = hm[f][b] * hm[f][b];
                let p2 = pm[f][b] * pm[f][b];
                let harm = h2 / (h2 + p2 + 1e-12);
                let perc = 1.0 - harm;
                let hz = b as f32 * bin_hz;
                let vband = if (180.0..7000.0).contains(&hz) {
                    1.0
                } else if (120.0..12000.0).contains(&hz) {
                    0.4
                } else {
                    0.0
                };
                let v = (center.powi(3) * harm * vband).clamp(0.0, 1.0);
                let bassw = if hz < 160.0 {
                    1.0
                } else if hz < 260.0 {
                    (260.0 - hz) / 100.0
                } else {
                    0.0
                };
                let d = (perc * (1.0 - v) * if hz < 60.0 { 0.5 } else { 1.0 }).clamp(0.0, 1.0);
                let bs = (harm * (1.0 - v) * bassw).clamp(0.0, 1.0 - d);
                masks[0][b] = v;
                masks[1][b] = d;
                masks[2][b] = bs;
            }
            for (si, out) in outs.iter_mut().enumerate() {
                for (spec, dst) in [(&sl[f], &mut out.0), (&sr_[f], &mut out.1)] {
                    for b in 0..bins {
                        buf[b] = spec[b] * masks[si][b];
                    }
                    for b in 1..N / 2 {
                        buf[N - b] = buf[b].conj();
                    }
                    ifft.process(&mut buf);
                    for k in 0..N {
                        let i = start + k as isize;
                        if i >= 0 && (i as usize) < dst.len() {
                            dst[i as usize] += buf[k].re / N as f32 * w[k];
                        }
                    }
                }
            }
        }
        // write the core of this segment (excluding padding) with normalized overlap-add
        let core_a = s0;
        let core_z = (s0 + seg).min(n);
        for (si, out) in outs.iter().enumerate() {
            let dst = match si {
                0 => &mut voc,
                1 => &mut drm,
                _ => &mut bas,
            };
            for i in core_a..core_z {
                let j = i - a;
                let g = 1.0 / wsum[j].max(1e-3);
                dst.0[i] = out.0[j] * g;
                dst.1[i] = out.1[j] * g;
            }
        }
        s0 += seg;
    }
    progress(1.0);
    let residual = |subs: &[&Stereo]| -> Stereo {
        let mut o = (l.to_vec(), r.to_vec());
        for s in subs {
            for i in 0..n {
                o.0[i] -= s.0[i];
                o.1[i] -= s.1[i];
            }
        }
        o
    };
    if mode == "2" {
        let inst = residual(&[&voc]);
        Some(vec![("Vocals".into(), voc), ("Instrumental".into(), inst)])
    } else {
        let other = residual(&[&voc, &drm, &bas]);
        Some(vec![
            ("Vocals".into(), voc),
            ("Drums".into(), drm),
            ("Bass".into(), bas),
            ("Other".into(), other),
        ])
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct FxPlan {
    pub kind: String,
    pub params: BTreeMap<String, f32>,
    pub mix: f32,
}
fn fxp(kind: &str, params: &[(&str, f32)], mix: f32) -> FxPlan {
    FxPlan {
        kind: kind.into(),
        params: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        mix,
    }
}

/// Auto-master: a chain chosen from measurements of the mix.
pub fn master_chain(
    l: &[f32],
    r: &[f32],
    sr: f32,
    target: f32,
    style: &str,
    amount: f32,
) -> (Vec<FxPlan>, Value) {
    let a = amount.clamp(0.0, 1.0);
    let mono: Vec<f32> = l.iter().zip(r).map(|(x, y)| (x + y) * 0.5).collect();
    let sp = analysis::spectrum(&mono, sr, 10);
    let pr = analysis::problems(l, r, sr);
    let m = analysis::loudness(l, r, sr);
    // tilt: compare low and high bands with the mids, pull towards a gentle -3 dB/oct-ish balance
    let mid = (sp[4] + sp[5]) * 0.5;
    let low = (sp[0] + sp[1]) * 0.5 - mid;
    let high = (sp[8] + sp[9]) * 0.5 - mid;
    let (lw, hw) = match style {
        "warm" => (2.5, -1.5),
        "bright" => (-0.5, 2.5),
        "loud" => (1.0, 1.0),
        "gentle" => (0.5, 0.5),
        _ => (1.0, 1.0),
    };
    let low_g = ((-6.0 - low).clamp(-6.0, 6.0) * 0.4 + lw) * a;
    let high_g = ((-12.0 - high).clamp(-6.0, 6.0) * 0.4 + hw) * a;
    let mud = ((sp[3] - mid).clamp(0.0, 8.0) * -0.4) * a;
    let ratio = match style {
        "loud" => 3.0,
        "gentle" => 1.6,
        _ => 2.2,
    };
    let thr = (m.integrated as f32 + 2.0).clamp(-40.0, -8.0);
    let mut chain = vec![
        fxp(
            "eq",
            &[
                ("b1_type", 4.0),
                ("b1_freq", 28.0),
                ("b1_q", 0.7),
                ("b2_type", 1.0),
                ("b2_freq", 110.0),
                ("b2_gain", low_g),
                ("b2_q", 0.7),
                ("b3_freq", 300.0),
                ("b3_gain", mud),
                ("b3_q", 1.0),
                ("b5_type", 2.0),
                ("b5_freq", 9000.0),
                ("b5_gain", high_g),
                ("b5_q", 0.7),
            ],
            1.0,
        ),
        fxp(
            "compressor",
            &[
                ("threshold", thr),
                ("ratio", 1.0 + (ratio - 1.0) * a),
                ("attack", 25.0),
                ("release", 200.0),
                ("knee", 8.0),
                ("auto_makeup", 0.0),
            ],
            1.0,
        ),
    ];
    if pr.width < 0.25 && style != "gentle" {
        chain.push(fxp("stereo", &[("width", 100.0 + 20.0 * a)], 1.0));
    }
    let input = (target - m.integrated as f32).clamp(-24.0, 24.0);
    chain.push(fxp(
        "limiter",
        &[
            ("ceiling", -1.0),
            ("input", input),
            ("release", 80.0),
            ("lookahead", 3.0),
        ],
        1.0,
    ));
    let report = json!({ "integrated": m.integrated, "true_peak": m.true_peak, "lra": m.lra, "low_db": low, "high_db": high, "width": pr.width, "style": style, "target": target });
    (chain, report)
}

#[derive(Serialize, Clone, Debug)]
pub struct Fix {
    pub id: String,
    pub title: String,
    pub detail: String,
    pub severity: String,
    pub fx: Vec<FxPlan>,
    pub on: bool,
}

/// Magic: find problems and propose fixes, each a small effect chain.
pub fn magic(
    l: &[f32],
    r: &[f32],
    sr: f32,
    target: f32,
) -> (Vec<Fix>, Problems, analysis::Loudness) {
    let p = analysis::problems(l, r, sr);
    let m = analysis::loudness(l, r, sr);
    let mut fixes = vec![];
    if let Some(hz) = p.hum_hz {
        fixes.push(Fix {
            id: "hum".into(),
            title: format!("Remove {hz:.0} Hz hum"),
            detail: format!("Mains hum {:.0} dB above its neighbours", p.hum_db),
            severity: "warn".into(),
            fx: vec![fxp(
                "hum",
                &[
                    ("freq", if hz < 55.0 { 1.0 } else { 2.0 }),
                    ("harmonics", 6.0),
                    ("q", 30.0),
                ],
                1.0,
            )],
            on: true,
        });
    }
    if p.noise_floor_db > -60.0 && p.noise_floor_db < -20.0 {
        let amt = ((p.noise_floor_db + 70.0) * 0.6).clamp(6.0, 24.0);
        fixes.push(Fix {
            id: "noise".into(),
            title: "Reduce background noise".into(),
            detail: format!("Noise floor at {:.0} dB", p.noise_floor_db),
            severity: "warn".into(),
            fx: vec![fxp("noise", &[("amount", amt), ("sensitivity", 45.0)], 1.0)],
            on: true,
        });
    }
    if p.clipped_samples > 32 {
        fixes.push(Fix {
            id: "clip".into(),
            title: "Repair clipping".into(),
            detail: format!("{} clipped samples", p.clipped_samples),
            severity: "bad".into(),
            fx: vec![fxp("declip", &[("threshold", 98.0)], 1.0)],
            on: true,
        });
    }
    if p.clicks > 3 {
        fixes.push(Fix {
            id: "clicks".into(),
            title: "Remove clicks".into(),
            detail: format!("{} clicks found", p.clicks),
            severity: "warn".into(),
            fx: vec![fxp("declick", &[("sensitivity", 50.0)], 1.0)],
            on: true,
        });
    }
    if p.low_end_db > 6.0 || p.dc_offset.abs() > 0.005 {
        fixes.push(Fix {
            id: "rumble".into(),
            title: "Cut rumble".into(),
            detail: if p.dc_offset.abs() > 0.005 {
                format!("DC offset {:.3}", p.dc_offset)
            } else {
                format!("Low end {:.0} dB above the mids", p.low_end_db)
            },
            severity: "info".into(),
            fx: vec![fxp(
                "eq",
                &[("b1_type", 4.0), ("b1_freq", 70.0), ("b1_q", 0.7)],
                1.0,
            )],
            on: true,
        });
    }
    if p.sibilance_db > 3.0 {
        fixes.push(Fix {
            id: "sibilance".into(),
            title: "Soften harsh S sounds".into(),
            detail: format!("Sibilance {:.0} dB above the mids", p.sibilance_db),
            severity: "info".into(),
            fx: vec![fxp(
                "deesser",
                &[("freq", 6800.0), ("threshold", -30.0), ("amount", 6.0)],
                1.0,
            )],
            on: true,
        });
    }
    if p.crest_db > 20.0 {
        fixes.push(Fix {
            id: "dynamics".into(),
            title: "Even out the level".into(),
            detail: format!("Peaks {:.0} dB above the average", p.crest_db),
            severity: "info".into(),
            fx: vec![fxp(
                "compressor",
                &[
                    ("threshold", (m.integrated as f32).clamp(-40.0, -10.0)),
                    ("ratio", 2.5),
                    ("attack", 15.0),
                    ("release", 180.0),
                ],
                1.0,
            )],
            on: true,
        });
    }
    if p.correlation < 0.0 {
        fixes.push(Fix {
            id: "phase".into(),
            title: "Fix out-of-phase stereo".into(),
            detail: format!("Correlation {:+.2}: the mix cancels in mono", p.correlation),
            severity: "bad".into(),
            fx: vec![fxp("stereo", &[("invert_r", 1.0)], 1.0)],
            on: true,
        });
    }
    if m.integrated > -69.0 && (m.integrated - target as f64).abs() > 2.0 {
        let input = (target as f64 - m.integrated).clamp(-24.0, 24.0) as f32;
        fixes.push(Fix {
            id: "loudness".into(),
            title: format!("Set loudness to {target:.0} LUFS"),
            detail: format!("Now {:.1} LUFS", m.integrated),
            severity: "info".into(),
            fx: if input > 0.0 {
                vec![fxp("limiter", &[("ceiling", -1.0), ("input", input)], 1.0)]
            } else {
                vec![fxp("gain", &[("gain", input)], 1.0)]
            },
            on: true,
        });
    }
    (fixes, p, m)
}

/// Voice cleanup: high-pass, learned noise reduction, de-ess, compress, level.
pub fn cleanup_chain(
    l: &[f32],
    r: &[f32],
    sr: f32,
    strength: f32,
) -> (Vec<FxPlan>, Option<Vec<f32>>, Value) {
    let s = strength.clamp(0.0, 1.0);
    let mono: Vec<f32> = l.iter().zip(r).map(|(a, b)| (a + b) * 0.5).collect();
    // learn a noise profile from the quietest half second
    let w = (sr * 0.5) as usize;
    let mut best = (0usize, f32::MAX);
    if mono.len() > w * 2 {
        for i in (0..mono.len() - w).step_by(w / 4) {
            let e: f32 = mono[i..i + w].iter().map(|v| v * v).sum::<f32>() / w as f32;
            if e > 1e-10 && e < best.1 {
                best = (i, e);
            }
        }
    }
    let profile = if best.1 < f32::MAX {
        Some(crate::fx::clean::learn_profile(&mono[best.0..best.0 + w]))
    } else {
        None
    };
    let p = analysis::problems(l, r, sr);
    let mut chain = vec![fxp(
        "eq",
        &[
            ("b1_type", 4.0),
            ("b1_freq", 80.0),
            ("b1_q", 0.7),
            ("b4_freq", 3500.0),
            ("b4_gain", 2.0 * s),
            ("b4_q", 1.0),
        ],
        1.0,
    )];
    if p.hum_hz.is_some() {
        chain.push(fxp("hum", &[("freq", 0.0)], 1.0));
    }
    chain.push(fxp(
        "noise",
        &[
            ("amount", 8.0 + 16.0 * s),
            ("sensitivity", 40.0 + 20.0 * s),
            ("keep", -30.0),
        ],
        1.0,
    ));
    chain.push(fxp(
        "deesser",
        &[
            ("freq", 6500.0),
            ("threshold", -32.0),
            ("amount", 3.0 + 5.0 * s),
        ],
        1.0,
    ));
    chain.push(fxp(
        "compressor",
        &[
            ("threshold", -24.0),
            ("ratio", 1.5 + 2.0 * s),
            ("attack", 8.0),
            ("release", 150.0),
            ("auto_makeup", 1.0),
        ],
        1.0,
    ));
    chain.push(fxp(
        "leveler",
        &[("target", -20.0), ("max_gain", 6.0 + 6.0 * s)],
        1.0,
    ));
    let rep = json!({ "noise_floor_db": p.noise_floor_db, "hum": p.hum_hz, "profile_from_s": best.0 as f32 / sr, "reverb_s": p.reverb_s });
    (chain, profile, rep)
}

/// Silence removal plan: returns cuts (start, end) to delete, in timeline samples.
pub fn silence_cuts(
    l: &[f32],
    r: &[f32],
    sr: f32,
    thresh_db: f32,
    longer_than_s: f32,
    shorten_to_s: f32,
    pad_s: f32,
) -> Vec<(i64, i64, i64, i64)> {
    let min_len = (longer_than_s * sr) as usize;
    let keep = (shorten_to_s * sr) as i64;
    let pad = (pad_s * sr) as i64;
    analysis::silences(l, r, sr, thresh_db, min_len.max(1))
        .into_iter()
        .filter_map(|(a, b)| {
            // keep `pad` either side of speech, then shorten what remains to `keep`
            let a2 = a + pad;
            let b2 = b - pad;
            let len = b2 - a2;
            if len <= keep {
                return None;
            }
            let cut_a = a2 + keep / 2;
            let cut_b = b2 - (keep - keep / 2);
            Some((a, b, cut_a, cut_b))
        })
        .filter(|c| c.3 > c.2)
        .collect()
}

/// Attenuate or fill a time-frequency rectangle. `fill`: "both" | "before" | "after" | "attenuate".
/// A time-frequency rectangle: frequencies in Hz, times in samples.
#[derive(Clone, Copy, Debug)]
pub struct Region {
    pub f_lo: f32,
    pub f_hi: f32,
    pub start: usize,
    pub end: usize,
}

pub fn spectral_repair(x: &[f32], sr: f32, region: Region, amount_db: f32, fill: &str) -> Vec<f32> {
    let Region {
        f_lo,
        f_hi,
        start,
        end,
    } = region;
    const N: usize = 2048;
    const H: usize = N / 4;
    let fft = FftPlanner::new().plan_fft_forward(N);
    let ifft = FftPlanner::new().plan_fft_inverse(N);
    let w = hann(N);
    let n = x.len();
    let frames = n / H + 2;
    let bins = N / 2 + 1;
    let mut spec = vec![vec![Complex::new(0f32, 0f32); bins]; frames];
    let mut buf = vec![Complex::new(0f32, 0f32); N];
    for (f, s) in spec.iter_mut().enumerate() {
        for k in 0..N {
            let i = (f * H) as isize + k as isize - (N / 2) as isize;
            buf[k] = Complex::new(
                if i >= 0 && (i as usize) < n {
                    x[i as usize]
                } else {
                    0.0
                } * w[k],
                0.0,
            );
        }
        fft.process(&mut buf);
        s.copy_from_slice(&buf[..bins]);
    }
    let bin = |f: f32| ((f / sr * N as f32) as usize).min(bins - 1);
    let (b0, b1) = (
        bin(f_lo.min(f_hi)),
        bin(f_lo.max(f_hi)).max(bin(f_lo.min(f_hi)) + 1),
    );
    let (f0, f1) = ((start / H).min(frames - 1), (end / H + 1).min(frames - 1));
    let g = crate::dsp::db_to_lin(-amount_db.abs());
    let before = f0.saturating_sub(2);
    let after = (f1 + 2).min(frames - 1);
    for f in f0..=f1 {
        let t = if f1 > f0 {
            (f - f0) as f32 / (f1 - f0) as f32
        } else {
            0.5
        };
        let (pre, post) = (spec[before].clone(), spec[after].clone());
        for (b, cell) in spec[f].iter_mut().enumerate().take(b1).skip(b0) {
            let c = *cell;
            let (pb, ab) = (pre[b], post[b]);
            let ph = c.arg();
            let m = match fill {
                "before" => pb.norm(),
                "after" => ab.norm(),
                "both" => pb.norm() * (1.0 - t) + ab.norm() * t,
                _ => c.norm() * g,
            }
            .min(c.norm());
            let m = if fill == "attenuate" {
                m
            } else {
                m.max(c.norm() * g)
            };
            *cell = Complex::from_polar(m, ph);
        }
    }
    let mut out = vec![0f32; n];
    let mut ws = vec![0f32; n];
    for (f, s) in spec.iter().enumerate() {
        buf[..bins].copy_from_slice(s);
        for b in 1..N / 2 {
            buf[N - b] = buf[b].conj();
        }
        ifft.process(&mut buf);
        for k in 0..N {
            let i = (f * H) as isize + k as isize - (N / 2) as isize;
            if i >= 0 && (i as usize) < n {
                out[i as usize] += buf[k].re / N as f32 * w[k];
                ws[i as usize] += w[k] * w[k];
            }
        }
    }
    for i in 0..n {
        out[i] /= ws[i].max(1e-3);
    }
    out
}

/// Match EQ: graphic EQ gains that move `x` towards the tone of `reference`.
pub fn match_eq(x: &[f32], reference: &[f32], sr: f32) -> BTreeMap<String, f32> {
    let keys = [
        "g31", "g63", "g125", "g250", "g500", "g1k", "g2k", "g4k", "g8k", "g16k",
    ];
    let band = |s: &[f32]| -> Vec<f32> {
        crate::fx::eq::GEQ_FREQS
            .iter()
            .map(|f| {
                let sp = analysis::spectrum(s, sr, 96);
                let (lo, hi) = (20f32.ln(), (sr * 0.5).min(20000.0).ln());
                let mut acc = 0.0;
                let mut c = 0.0;
                for (b, v) in sp.iter().enumerate() {
                    let fb = (lo + (hi - lo) * (b as f32 + 0.5) / 96.0).exp();
                    if fb > f / 1.41 && fb < f * 1.41 {
                        acc += v;
                        c += 1.0;
                    }
                }
                if c > 0.0 {
                    acc / c
                } else {
                    -120.0
                }
            })
            .collect()
    };
    let (a, b) = (band(x), band(reference));
    let off = a.iter().zip(&b).map(|(p, q)| q - p).sum::<f32>() / 10.0;
    keys.iter()
        .enumerate()
        .map(|(i, k)| (k.to_string(), (b[i] - a[i] - off).clamp(-12.0, 12.0)))
        .collect()
}

/// Reverse reverb swell: the reversed wet tail of the first `len` samples, ending at the clip start.
pub fn reverse_swell(x_l: &[f32], x_r: &[f32], sr: f32, len: usize) -> Stereo {
    let len = len.min(x_l.len()).max(1);
    let mut l: Vec<f32> = x_l[..len].iter().rev().copied().collect();
    let mut r: Vec<f32> = x_r[..len].iter().rev().copied().collect();
    let tail = (sr * 3.0) as usize;
    l.extend(std::iter::repeat_n(0.0, tail));
    r.extend(std::iter::repeat_n(0.0, tail));
    let mut rv = crate::fx::build("reverb", sr, &Default::default()).unwrap();
    let mut pm = BTreeMap::new();
    pm.insert("decay".to_string(), 3.0);
    rv.set(&crate::fx::resolve(crate::fx::spec("reverb").unwrap(), &pm));
    let ctx = crate::fx::Ctx {
        sr,
        pos: 0,
        len: l.len() as i64,
        tempo: 120.0,
    };
    for (a, b) in l
        .chunks_mut(crate::fx::BLOCK)
        .zip(r.chunks_mut(crate::fx::BLOCK))
    {
        rv.process(a, b, &ctx);
    }
    // keep only the tail beyond the dry part, then reverse it so it swells into the clip
    let wet_l: Vec<f32> = l.iter().rev().copied().collect();
    let wet_r: Vec<f32> = r.iter().rev().copied().collect();
    let n = wet_l.len();
    let fade = (sr * 0.05) as usize;
    let mut out = (wet_l, wet_r);
    for i in 0..fade.min(n) {
        let g = i as f32 / fade as f32;
        out.0[i] *= g;
        out.1[i] *= g;
    }
    out
}

/// The listener's head profile from first-run setup, as 3D renderer settings: (depth %, room %). Depth scales the
/// far-ear delay, head shadow and rear cue; a little room helps sound seem outside the head for some listeners.
/// A, D: lighter cues (bright); B: the default; C: full cues, strongest behind; E, F: strong cues with some room.
pub fn hrtf_profile(profile: &str) -> (f32, f32) {
    match profile {
        "A" | "D" => (65.0, 0.0),
        "C" => (100.0, 8.0),
        "E" | "F" => (90.0, 15.0),
        _ => (80.0, 0.0),
    }
}

/// Test tones for the headphone check: "left", "right", "moving". The moving sound uses the listener's head width
/// and profile, so each profile on the head-shape screen can be heard.
pub fn test_tone(kind: &str, sr: f32, head_cm: f32, profile: &str) -> Stereo {
    let n = (sr * if kind == "moving" { 4.0 } else { 1.2 }) as usize;
    let mut l = vec![0f32; n];
    let mut r = vec![0f32; n];
    let mut rng = crate::dsp::Rng::new(4);
    for i in 0..n {
        let t = i as f32 / sr;
        let env = (t * 20.0).min(1.0) * ((n - i) as f32 / sr * 20.0).min(1.0);
        let s = (crate::dsp::TAU * 660.0 * t).sin() * 0.2 * env;
        match kind {
            "left" => l[i] = s,
            "right" => r[i] = s,
            _ => {
                let v = rng.bi() * 0.15 * env;
                l[i] = v;
                r[i] = v;
            }
        }
    }
    if kind == "moving" {
        let mut sp = crate::fx::build("spatial", sr, &Default::default()).unwrap();
        let mut pm = BTreeMap::new();
        pm.insert("mode".into(), 4.0);
        let (depth, room) = hrtf_profile(profile);
        pm.insert("head_width".into(), head_cm.clamp(12.0, 22.0));
        pm.insert("depth".into(), depth);
        pm.insert("room".into(), room);
        let spec = crate::fx::spec("spatial").unwrap();
        let seg = n / 4;
        for (k, az) in [0.0f32, -90.0, 90.0, 180.0].iter().enumerate() {
            pm.insert("azimuth".into(), *az);
            sp.set(&crate::fx::resolve(spec, &pm));
            let (a, b) = (k * seg, ((k + 1) * seg).min(n));
            let ctx = crate::fx::Ctx {
                sr,
                pos: a as i64,
                len: n as i64,
                tempo: 120.0,
            };
            for (x, y) in l[a..b]
                .chunks_mut(crate::fx::BLOCK)
                .zip(r[a..b].chunks_mut(crate::fx::BLOCK))
            {
                sp.process(x, y, &ctx);
            }
        }
    }
    (l, r)
}

#[derive(Serialize, Clone, Debug)]
pub struct Voice {
    pub id: String,
    pub name: String,
    pub engine: String,
    pub language: String,
}

/// System text-to-speech voices installed on this computer.
pub fn voices() -> Vec<Voice> {
    let run = |cmd: &mut std::process::Command| -> Option<String> {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        cmd.output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
    };
    let mut out = vec![];
    if cfg!(windows) {
        let ps = "Add-Type -AssemblyName System.Speech; $s=New-Object System.Speech.Synthesis.SpeechSynthesizer; $s.GetInstalledVoices() | Where-Object {$_.Enabled} | ForEach-Object { $_.VoiceInfo.Name + '|' + $_.VoiceInfo.Culture.Name }";
        if let Some(s) = run(std::process::Command::new("powershell").args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            ps,
        ])) {
            for line in s.lines().filter(|l| l.contains('|')) {
                let mut it = line.trim().splitn(2, '|');
                let name = it.next().unwrap_or("").to_string();
                out.push(Voice {
                    id: name.clone(),
                    name,
                    engine: "Windows speech".into(),
                    language: it.next().unwrap_or("").into(),
                });
            }
        }
    } else if cfg!(target_os = "macos") {
        if let Some(s) = run(std::process::Command::new("say").args(["-v", "?"])) {
            for line in s.lines() {
                let name = line.split("  ").next().unwrap_or("").trim().to_string();
                let lang = line
                    .split_whitespace()
                    .find(|w| w.contains('_'))
                    .unwrap_or("")
                    .to_string();
                if !name.is_empty() {
                    out.push(Voice {
                        id: name.clone(),
                        name,
                        engine: "macOS speech".into(),
                        language: lang,
                    });
                }
            }
        }
    } else if let Some(s) = run(std::process::Command::new("espeak-ng").arg("--voices")) {
        for line in s.lines().skip(1) {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() > 4 {
                out.push(Voice {
                    id: cols[4].into(),
                    name: cols[3].replace('_', " "),
                    engine: "eSpeak NG".into(),
                    language: cols[1].into(),
                });
            }
        }
    }
    out
}

/// Synthesize speech with a system voice to a WAV file. `rate` is -10..10.
pub fn speak(text: &str, voice: &str, rate: i32, out: &std::path::Path) -> anyhow::Result<()> {
    use anyhow::{bail, Context};
    if text.trim().is_empty() {
        bail!("there is no text to speak");
    }
    if text.len() > 100_000 {
        bail!("the text is too long (100,000 characters at most)");
    }
    let dir = out.parent().context("bad path")?;
    std::fs::create_dir_all(dir)?;
    let txt = out.with_extension("txt");
    std::fs::write(&txt, text)?;
    let mut cmd;
    if cfg!(windows) {
        let esc = |s: &str| s.replace('\'', "''");
        let ps = format!(
            "Add-Type -AssemblyName System.Speech; $s=New-Object System.Speech.Synthesis.SpeechSynthesizer; {} $s.Rate={}; $s.SetOutputToWaveFile('{}'); $s.Speak([IO.File]::ReadAllText('{}')); $s.Dispose()",
            if voice.is_empty() { String::new() } else { format!("$s.SelectVoice('{}');", esc(voice)) },
            rate.clamp(-10, 10),
            esc(&out.display().to_string()),
            esc(&txt.display().to_string())
        );
        cmd = std::process::Command::new("powershell");
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", &ps]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
    } else if cfg!(target_os = "macos") {
        cmd = std::process::Command::new("say");
        if !voice.is_empty() {
            cmd.args(["-v", voice]);
        }
        cmd.args(["-r", &format!("{}", 175 + rate.clamp(-10, 10) * 12)])
            .arg("-o")
            .arg(out)
            .args(["--file-format=WAVE", "--data-format=LEI16@22050", "-f"])
            .arg(&txt);
    } else {
        cmd = std::process::Command::new("espeak-ng");
        if !voice.is_empty() {
            cmd.args(["-v", voice]);
        }
        cmd.args(["-s", &format!("{}", 175 + rate.clamp(-10, 10) * 12)])
            .arg("-w")
            .arg(out)
            .arg("-f")
            .arg(&txt);
    }
    let res = cmd
        .output()
        .context("no system speech engine was found on this computer")?;
    let _ = std::fs::remove_file(&txt);
    if !res.status.success() || !out.exists() {
        bail!(
            "speech synthesis failed: {}",
            String::from_utf8_lossy(&res.stderr)
                .lines()
                .last()
                .unwrap_or("no output")
        );
    }
    Ok(())
}

pub fn db(x: f32) -> f32 {
    lin_to_db(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
    }

    #[test]
    fn a_wider_head_delays_the_far_ear_more() {
        // One click at 90 degrees left: it reaches the right ear later, by the Woodworth delay for that head.
        let sr = 48000.0;
        let arrival = |head: f32| {
            let mut sp = crate::fx::build("spatial", sr, &Default::default()).unwrap();
            let mut pm = BTreeMap::new();
            for (k, v) in [
                ("mode", 4.0),
                ("azimuth", -90.0),
                ("head_width", head),
                ("smoothing", 0.0),
            ] {
                pm.insert(k.to_string(), v);
            }
            sp.set(&crate::fx::resolve(
                crate::fx::spec("spatial").unwrap(),
                &pm,
            ));
            let n = 48000;
            let (mut l, mut r) = (vec![0f32; n], vec![0f32; n]);
            l[24000] = 1.0;
            r[24000] = 1.0;
            let ctx = crate::fx::Ctx {
                sr,
                pos: 0,
                len: n as i64,
                tempo: 120.0,
            };
            for (x, y) in l
                .chunks_mut(crate::fx::BLOCK)
                .zip(r.chunks_mut(crate::fx::BLOCK))
            {
                sp.process(x, y, &ctx);
            }
            let peak = |x: &[f32]| {
                (23900..24300)
                    .max_by(|a, b| x[*a].abs().partial_cmp(&x[*b].abs()).unwrap())
                    .unwrap() as i64
                    - 24000
            };
            (peak(&l), peak(&r))
        };
        assert_eq!(arrival(12.0), (0, 13));
        assert_eq!(arrival(22.0), (0, 24));
    }

    #[test]
    fn the_moving_tone_follows_head_width_and_profile() {
        let sr = 48000.0;
        // Second quarter of the moving sound is at the left (-90 degrees): the right ear hears it later and quieter.
        let part = |head: f32, prof: &str| {
            let (l, r) = test_tone("moving", sr, head, prof);
            let q = l.len() / 4;
            let (a, b) = (q + q / 4, 2 * q - q / 4);
            (l[a..b].to_vec(), r[a..b].to_vec())
        };
        assert_ne!(
            part(22.0, "B").1,
            part(12.0, "B").1,
            "head width changes the moving sound"
        );
        let ild = |l: &[f32], r: &[f32]| 20.0 * (rms(l) / rms(r)).log10();
        let (la, ra) = part(17.5, "A");
        let (lc, rc) = part(17.5, "C");
        assert!(
            ild(&lc, &rc) > ild(&la, &ra) + 0.5,
            "C {} A {}",
            ild(&lc, &rc),
            ild(&la, &ra)
        );
    }
    use crate::dsp::{Rng, TAU};
    fn mix(sr: f32, secs: f32) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
        // centered "voice" (harmonic, 300 Hz + harmonics) plus wide noisy hats
        let n = (sr * secs) as usize;
        let mut rng = Rng::new(7);
        let mut voice = vec![0f32; n];
        let (mut l, mut r) = (vec![0f32; n], vec![0f32; n]);
        for i in 0..n {
            let t = i as f32 / sr;
            let v = (1..6)
                .map(|h| (TAU * 300.0 * h as f32 * t).sin() / h as f32)
                .sum::<f32>()
                * 0.15;
            voice[i] = v;
            let hat = if (i % (sr as usize / 4)) < 400 {
                rng.bi() * 0.3
            } else {
                0.0
            };
            l[i] = v + hat;
            r[i] = v - hat * 0.8;
        }
        (l, r, voice)
    }
    #[test]
    fn stems_sum_to_original() {
        let sr = 22050.0;
        let (l, r, _) = mix(sr, 3.0);
        let s = stems(&l, &r, sr, "4", &mut |_| true).unwrap();
        assert_eq!(s.len(), 4);
        for i in (0..l.len()).step_by(97) {
            let sl: f32 = s.iter().map(|x| x.1 .0[i]).sum();
            let sr_: f32 = s.iter().map(|x| x.1 .1[i]).sum();
            assert!((sl - l[i]).abs() < 1e-4 && (sr_ - r[i]).abs() < 1e-4);
        }
    }
    #[test]
    fn vocal_stem_captures_the_centered_harmonic_part() {
        let sr = 22050.0;
        let (l, r, voice) = mix(sr, 3.0);
        let s = stems(&l, &r, sr, "2", &mut |_| true).unwrap();
        let v = &s[0].1;
        // correlation of the vocal stem with the true voice is high, and the instrumental's is low
        let corr = |a: &[f32], b: &[f32]| {
            let (mut ab, mut aa, mut bb) = (0f64, 0f64, 0f64);
            for i in 2000..a.len() - 2000 {
                ab += (a[i] * b[i]) as f64;
                aa += (a[i] * a[i]) as f64;
                bb += (b[i] * b[i]) as f64;
            }
            ab / (aa * bb).sqrt()
        };
        let cv = corr(&v.0, &voice);
        assert!(cv > 0.9, "vocal stem correlation {cv}");
        let inst = &s[1].1;
        let e_voice_in_inst = corr(&inst.0, &voice);
        assert!(
            e_voice_in_inst < 0.5,
            "voice leaks into instrumental: {e_voice_in_inst}"
        );
    }
    #[test]
    fn stems_cancel() {
        let sr = 22050.0;
        let (l, r, _) = mix(sr, 2.0);
        let mut calls = 0;
        assert!(stems(&l, &r, sr, "4", &mut |_| {
            calls += 1;
            false
        })
        .is_none());
        assert_eq!(calls, 1);
    }
    #[test]
    fn magic_finds_hum_and_loudness() {
        let sr = 48000.0;
        let n = 48000 * 4;
        let mut rng = Rng::new(1);
        let x: Vec<f32> = (0..n)
            .map(|i| {
                (TAU * 50.0 * i as f32 / sr).sin() * 0.05
                    + (TAU * 440.0 * i as f32 / sr).sin() * 0.05
                    + rng.bi() * 0.001
            })
            .collect();
        let (fixes, _, _) = magic(&x, &x, sr, -14.0);
        let ids: Vec<&str> = fixes.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"hum"), "{ids:?}");
        assert!(ids.contains(&"loudness"), "{ids:?}");
    }
    #[test]
    fn spectral_repair_removes_a_tone_in_the_box_only() {
        let sr = 48000.0;
        let n = 48000;
        let x: Vec<f32> = (0..n)
            .map(|i| {
                (TAU * 3000.0 * i as f32 / sr).sin() * 0.3
                    + (TAU * 300.0 * i as f32 / sr).sin() * 0.3
            })
            .collect();
        let y = spectral_repair(
            &x,
            sr,
            Region {
                f_lo: 2500.0,
                f_hi: 3500.0,
                start: 12000,
                end: 36000,
            },
            40.0,
            "attenuate",
        );
        let g = |s: &[f32], f: f32| -> f32 {
            let c = 2.0 * (TAU * f / sr).cos();
            let (mut a, mut b) = (0f32, 0f32);
            for v in s {
                let z = v + c * a - b;
                b = a;
                a = z;
            }
            (a * a + b * b - c * a * b).sqrt() / s.len() as f32
        };
        let (mid_x, mid_y) = (&x[16000..32000], &y[16000..32000]);
        assert!(lin_to_db(g(mid_y, 3000.0)) < lin_to_db(g(mid_x, 3000.0)) - 25.0);
        assert!((lin_to_db(g(mid_y, 300.0)) - lin_to_db(g(mid_x, 300.0))).abs() < 0.5);
        // outside the box the audio is unchanged
        let d: f32 = x[2000..8000]
            .iter()
            .zip(&y[2000..8000])
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f32::max);
        assert!(d < 1e-3, "{d}");
    }
    #[test]
    fn silence_cuts_shorten_pauses() {
        let sr = 1000.0;
        let mut x = vec![0.5f32; 1000];
        x.extend(vec![0.0; 2000]);
        x.extend(vec![0.5; 1000]);
        let c = silence_cuts(&x, &x, sr, -40.0, 0.6, 0.3, 0.1);
        assert_eq!(c.len(), 1);
        let removed = c[0].3 - c[0].2;
        assert!((removed - 1500).abs() <= 20, "{removed}");
    }
}
