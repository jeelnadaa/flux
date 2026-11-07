//! The one engine: the same `Renderer` drives real-time playback and offline export, so the two
//! null. Plans are built off the audio thread; the audio thread only processes preallocated
//! buffers and applies POD messages.

use crate::dsp::{db_to_lin, hermite, pan_gains, Delay};
use crate::fx::{self, Chain, Ctx, Slot, BLOCK, P};
use crate::media::Pool;
use crate::project::{fade_gain, Effect, Project};
use std::collections::BTreeMap;
use std::sync::Arc;

const TAIL: i64 = 48000 * 2;

pub struct ClipPlan {
    pub id: u32,
    pub chain: Chain,
}

pub struct TrackPlan {
    pub id: u32,
    pub chain: Chain,
    pub clips: Vec<ClipPlan>,
    pub gain: f32,
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
    pub latency: usize,
    pdc: Delay,
    pdc_r: Delay,
    /// (lane index, slot index in chain, param index) for automated effect params
    fx_lanes: Vec<(usize, usize, usize)>,
    params: Vec<P>,
    pub peak: f32,
    /// Smoothed gain and pan actually applied.
    gain_s: f32,
    pan_s: f32,
}

/// One-pole glide coefficient for a block of `n` samples.
fn glide_k(n: usize, sr: f32) -> f32 {
    1.0 - (-(n as f32) / (sr * crate::fx::GLIDE_MS * 0.001)).exp()
}
fn glide(cur: &mut f32, target: f32, k: f32) {
    let d = target - *cur;
    if d.abs() <= 1e-6 {
        *cur = target;
    } else {
        *cur += d * k;
    }
}

pub struct Plan {
    pub project: Arc<Project>,
    pub media: Arc<Pool>,
    pub tracks: Vec<TrackPlan>,
    pub master: Chain,
    pub master_gain: f32,
    pub speed: f32,
    pub keep_pitch: bool,
    pub max_lat: usize,
    pub master_lat: usize,
    pub len: i64,
}

fn extra_for(e: &Effect, media: &Pool) -> fx::Extra {
    let ir = e.ir.as_ref().and_then(|id| media.get(id)).map(|m| {
        (
            m.data[0].as_ref().clone(),
            m.data[m.data.len() - 1].as_ref().clone(),
        )
    });
    fx::Extra {
        profile: e.profile.clone(),
        ir,
        path: e.path.clone(),
    }
}

pub fn build_chain(effects: &[Effect], sr: f32, media: &Pool) -> (Chain, Vec<P>) {
    let mut c = Chain::default();
    let mut ps = Vec::new();
    for e in effects {
        let Some(spec) = fx::spec(&e.kind) else {
            continue;
        };
        let Some(proc) = fx::build(&e.kind, sr, &extra_for(e, media)) else {
            continue;
        };
        let p = fx::resolve(spec, &e.params);
        ps.push(p);
        let mut slot = Slot::new(e.id, spec.kind, e.enabled, e.mix, proc);
        slot.set_params(&p);
        c.slots.push(slot);
    }
    (c, ps)
}

impl Plan {
    pub fn build(project: &Project, media: Arc<Pool>) -> Plan {
        let sr = project.sample_rate as f32;
        let mut tracks = Vec::new();
        for t in &project.tracks {
            let (chain, params) = build_chain(&t.fx, sr, &media);
            let latency: usize = chain
                .slots
                .iter()
                .filter(|s| s.enabled)
                .map(|s| s.proc.latency())
                .sum();
            let clips = t
                .clips
                .iter()
                .filter(|c| !c.fx.is_empty())
                .map(|c| ClipPlan {
                    id: c.id,
                    chain: build_chain(&c.fx, sr, &media).0,
                })
                .collect();
            let mut fx_lanes = vec![];
            for (li, l) in t.lanes.iter().enumerate() {
                if let Some(rest) = l.target.strip_prefix("fx:") {
                    let mut it = rest.splitn(2, ':');
                    if let (Some(id), Some(key)) =
                        (it.next().and_then(|s| s.parse::<u32>().ok()), it.next())
                    {
                        if let Some(si) = t.fx.iter().position(|e| e.id == id) {
                            if let Some(pi) = fx::spec(&t.fx[si].kind)
                                .and_then(|s| s.params.iter().position(|p| p.key == key))
                            {
                                fx_lanes.push((li, si, pi));
                            }
                        }
                    }
                }
            }
            tracks.push(TrackPlan {
                id: t.id,
                chain,
                clips,
                gain: db_to_lin(t.gain_db),
                pan: t.pan,
                mute: t.mute,
                solo: t.solo,
                latency,
                pdc: Delay::new(8192),
                pdc_r: Delay::new(8192),
                fx_lanes,
                params,
                peak: 0.0,
                gain_s: db_to_lin(t.gain_db),
                pan_s: t.pan,
            });
        }
        let max_lat = tracks
            .iter()
            .map(|t| t.latency)
            .max()
            .unwrap_or(0)
            .min(8000);
        let (master, _) = build_chain(&project.master.fx, sr, &media);
        let master_lat = master
            .slots
            .iter()
            .filter(|s| s.enabled)
            .map(|s| s.proc.latency())
            .sum();
        let sp = project
            .master
            .fx
            .iter()
            .find(|e| e.kind == "speed" && e.enabled);
        let speed = sp
            .map(|e| e.params.get("rate").copied().unwrap_or(1.0))
            .unwrap_or(1.0)
            .clamp(0.5, 2.0);
        let keep_pitch = sp
            .map(|e| {
                e.params.get("keep_pitch").copied().unwrap_or(0.0) > 0.5
                    && e.params.get("mode").copied().unwrap_or(0.0) < 1.5
            })
            .unwrap_or(false);
        Plan {
            project: Arc::new(project.clone()),
            media,
            tracks,
            master,
            master_gain: db_to_lin(project.master.gain_db),
            speed,
            keep_pitch,
            max_lat,
            master_lat,
            len: project.length(),
        }
    }
}

/// Messages from the control thread. All plain data: no allocation on the audio thread.
#[derive(Clone, Copy, Debug)]
pub enum Msg {
    Params {
        fx: u32,
        p: P,
    },
    Fx {
        fx: u32,
        enabled: bool,
        mix: f32,
    },
    Track {
        id: u32,
        gain: f32,
        pan: f32,
        mute: bool,
        solo: bool,
    },
    MasterGain(f32),
    Speed {
        rate: f32,
        keep_pitch: bool,
    },
    Hold(bool),
    Seek(i64),
    Play(bool),
    Loop(Option<(i64, i64)>),
    SafetyLimiter(bool),
}

pub struct Renderer {
    pub plan: Plan,
    /// Timeline position of the next block (fractional when speed != 1).
    pub pos: f64,
    pub playing: bool,
    pub hold: bool,
    pub loop_range: Option<(i64, i64)>,
    tl: Vec<f32>,
    tr: Vec<f32>,
    cl: Vec<f32>,
    cr: Vec<f32>,
    ml: Vec<f32>,
    mr: Vec<f32>,
    dl: Vec<f32>,
    dr: Vec<f32>,
    // varispeed FIFO at timeline rate
    fl: Vec<f32>,
    fr: Vec<f32>,
    fstart: f64,
    fcount: usize,
    frender: i64,
    pitch: crate::fx::pitch::PitchShifter,
    rms_proc: f32,
    rms_dry: f32,
    hold_gain: f32,
    /// Smoothed master gain.
    mg_s: f32,
    pub peak: [f32; 2],
    pub safety: bool,
    fade_in: f32,
}

const FIFO: usize = 8192;

impl Renderer {
    pub fn new(plan: Plan) -> Self {
        let mg = plan.master_gain;
        Renderer {
            plan,
            pos: 0.0,
            playing: false,
            hold: false,
            loop_range: None,
            tl: vec![0.0; BLOCK],
            tr: vec![0.0; BLOCK],
            cl: vec![0.0; BLOCK],
            cr: vec![0.0; BLOCK],
            ml: vec![0.0; BLOCK],
            mr: vec![0.0; BLOCK],
            dl: vec![0.0; BLOCK],
            dr: vec![0.0; BLOCK],
            fl: vec![0.0; FIFO],
            fr: vec![0.0; FIFO],
            fstart: 0.0,
            fcount: 0,
            frender: 0,
            pitch: crate::fx::pitch::PitchShifter::new(),
            rms_proc: 1e-6,
            rms_dry: 1e-6,
            hold_gain: 1.0,
            mg_s: mg,
            peak: [0.0; 2],
            safety: false,
            fade_in: 1.0,
        }
    }

    /// Replace the plan, keeping transport state.
    pub fn swap_plan(&mut self, plan: Plan) -> Plan {
        let old = std::mem::replace(&mut self.plan, plan);
        self.fcount = 0;
        old
    }

    /// Total output latency in timeline samples.
    pub fn latency(&self) -> usize {
        self.plan.max_lat
            + self.plan.master_lat
            + if self.plan.keep_pitch && (self.plan.speed - 1.0).abs() > 1e-4 {
                crate::fx::pitch::PitchShifter::LATENCY
            } else {
                0
            }
    }

    pub fn seek(&mut self, pos: i64) {
        self.pos = pos.max(0) as f64;
        self.fcount = 0;
        self.fade_in = 0.0;
    }

    pub fn apply(&mut self, m: Msg) {
        match m {
            Msg::Params { fx, p } => {
                let find = |c: &mut Chain| -> bool {
                    if let Some(s) = c.find(fx) {
                        s.set_params(&p);
                        true
                    } else {
                        false
                    }
                };
                if find(&mut self.plan.master) {
                    return;
                }
                for t in &mut self.plan.tracks {
                    if let Some(i) = t.chain.slots.iter().position(|s| s.id == fx) {
                        t.chain.slots[i].set_params(&p);
                        if i < t.params.len() {
                            t.params[i] = p;
                        }
                        return;
                    }
                    for c in &mut t.clips {
                        if find(&mut c.chain) {
                            return;
                        }
                    }
                }
            }
            Msg::Fx { fx, enabled, mix } => {
                let set = |c: &mut Chain| {
                    if let Some(s) = c.find(fx) {
                        s.enabled = enabled;
                        s.mix = mix;
                    }
                };
                set(&mut self.plan.master);
                for t in &mut self.plan.tracks {
                    set(&mut t.chain);
                    for c in &mut t.clips {
                        set(&mut c.chain);
                    }
                }
            }
            Msg::Track {
                id,
                gain,
                pan,
                mute,
                solo,
            } => {
                if let Some(t) = self.plan.tracks.iter_mut().find(|t| t.id == id) {
                    t.gain = gain;
                    t.pan = pan;
                    t.mute = mute;
                    t.solo = solo;
                }
            }
            Msg::MasterGain(g) => self.plan.master_gain = g,
            Msg::Speed { rate, keep_pitch } => {
                self.plan.speed = rate.clamp(0.5, 2.0);
                self.plan.keep_pitch = keep_pitch;
            }
            Msg::Hold(h) => self.hold = h,
            Msg::Seek(p) => self.seek(p),
            Msg::Play(p) => {
                if p && !self.playing {
                    self.fade_in = 0.0;
                }
                self.playing = p;
            }
            Msg::Loop(l) => self.loop_range = l,
            Msg::SafetyLimiter(s) => self.safety = s,
        }
    }

    /// Mix every track at timeline rate for `n` <= BLOCK samples starting at `pos`.
    /// Writes the processed mix to (ml, mr) and the effect-free mix to (dl, dr).
    fn mix(&mut self, pos: i64, n: usize) {
        let proj = self.plan.project.clone();
        let sr = proj.sample_rate as f32;
        let any_solo = self.plan.tracks.iter().any(|t| t.solo);
        self.ml[..n].fill(0.0);
        self.mr[..n].fill(0.0);
        self.dl[..n].fill(0.0);
        self.dr[..n].fill(0.0);
        let ctx = Ctx {
            sr,
            pos,
            len: self.plan.len,
            tempo: proj.tempo,
        };
        let max_lat = self.plan.max_lat;
        let gk = glide_k(n, sr);
        for (ti, t) in proj.tracks.iter().enumerate() {
            let tp = &mut self.plan.tracks[ti];
            self.tl[..n].fill(0.0);
            self.tr[..n].fill(0.0);
            let audible = !tp.mute && (!any_solo || tp.solo);
            for c in &t.clips {
                let has_chain = tp.clips.iter().any(|cp| cp.id == c.id);
                let tail = if has_chain { TAIL } else { 0 };
                if c.end() + tail <= pos || c.start >= pos + n as i64 {
                    continue;
                }
                let Some(m) = self.plan.media.get(&c.media) else {
                    continue;
                };
                let g = db_to_lin(c.gain_db);
                self.cl[..n].fill(0.0);
                self.cr[..n].fill(0.0);
                let a = (c.start - pos).max(0) as usize;
                let z = ((c.end() - pos).max(0) as usize).min(n);
                for k in a..z {
                    let local = pos + k as i64 - c.start;
                    let si = if c.reverse {
                        c.offset + c.len - 1 - local
                    } else {
                        c.offset + local
                    };
                    if si < 0 {
                        continue;
                    }
                    let mut f = g;
                    if c.fade_in > 0 && local < c.fade_in {
                        f *= fade_gain(&c.curve, local as f32 / c.fade_in as f32);
                    }
                    let to_end = c.len - local;
                    if c.fade_out > 0 && to_end <= c.fade_out {
                        f *= fade_gain(&c.curve, (to_end - 1) as f32 / c.fade_out as f32);
                    }
                    self.cl[k] = m.get(0, si as usize) * f;
                    self.cr[k] = m.get(1, si as usize) * f;
                }
                if audible {
                    for k in 0..n {
                        self.dl[k] += self.cl[k] * tp.gain;
                        self.dr[k] += self.cr[k] * tp.gain;
                    }
                }
                if has_chain {
                    if let Some(cp) = tp.clips.iter_mut().find(|cp| cp.id == c.id) {
                        cp.chain.process(&mut self.cl[..n], &mut self.cr[..n], &ctx);
                    }
                }
                for k in 0..n {
                    self.tl[k] += self.cl[k];
                    self.tr[k] += self.cr[k];
                }
            }
            // automation of effect parameters (block rate)
            for &(li, si, pi) in &tp.fx_lanes {
                if let Some(v) = t.lanes[li].value_at(pos) {
                    if si < tp.params.len() {
                        tp.params[si][pi] = v;
                        let p = tp.params[si];
                        tp.chain.slots[si].set_params(&p);
                    }
                }
            }
            tp.chain.process(&mut self.tl[..n], &mut self.tr[..n], &ctx);
            let vol = t.lanes.iter().find(|l| l.target == "volume");
            let panl = t.lanes.iter().find(|l| l.target == "pan");
            let (g0, g1) = match vol {
                Some(l) => (
                    db_to_lin(l.value_at(pos).unwrap_or(0.0)),
                    db_to_lin(l.value_at(pos + n as i64).unwrap_or(0.0)),
                ),
                None => {
                    let g0 = tp.gain_s;
                    glide(&mut tp.gain_s, tp.gain, gk);
                    (g0, tp.gain_s)
                }
            };
            glide(&mut tp.pan_s, tp.pan, gk);
            let pan = panl.and_then(|l| l.value_at(pos)).unwrap_or(tp.pan_s);
            let (pl, pr) = pan_gains(pan);
            let delay = max_lat - tp.latency.min(max_lat);
            let mut pk = 0f32;
            for k in 0..n {
                let g = g0 + (g1 - g0) * k as f32 / n as f32;
                let (mut a, mut b) = (self.tl[k] * g * pl, self.tr[k] * g * pr);
                if delay > 0 {
                    tp.pdc.push(a);
                    tp.pdc_r.push(b);
                    a = tp.pdc.tap(delay);
                    b = tp.pdc_r.tap(delay);
                }
                pk = pk.max(a.abs()).max(b.abs());
                if audible {
                    self.ml[k] += a;
                    self.mr[k] += b;
                }
            }
            tp.peak = pk.max(tp.peak * 0.9);
        }
    }

    /// Render `out_l.len()` output samples. Advances the transport when playing.
    pub fn render(&mut self, out_l: &mut [f32], out_r: &mut [f32]) {
        let total = out_l.len();
        let mut done = 0;
        while done < total {
            let n = (total - done).min(BLOCK);
            let (ol, or) = (&mut out_l[done..done + n], &mut out_r[done..done + n]);
            if !self.playing {
                ol.fill(0.0);
                or.fill(0.0);
                done += n;
                continue;
            }
            let speed = if self.hold {
                1.0
            } else {
                self.plan.speed as f64
            };
            let pos = self.pos as i64;
            if (speed - 1.0).abs() < 1e-6 {
                self.mix(pos, n);
                if self.hold {
                    ol[..n].copy_from_slice(&self.dl[..n]);
                    or[..n].copy_from_slice(&self.dr[..n]);
                } else {
                    ol.copy_from_slice(&self.ml[..n]);
                    or.copy_from_slice(&self.mr[..n]);
                }
                self.pos += n as f64;
            } else {
                self.varispeed(ol, or, speed);
            }
            let ctx = Ctx {
                sr: self.plan.project.sample_rate as f32,
                pos: self.pos as i64,
                len: self.plan.len,
                tempo: self.plan.project.tempo,
            };
            if !self.hold {
                if self.plan.keep_pitch && (speed - 1.0).abs() > 1e-6 {
                    self.pitch.ratio = 1.0 / speed as f32;
                    self.pitch.keep = true;
                    self.pitch.process(ol, or);
                }
                self.plan.master.process(ol, or, &ctx);
            }
            let mg = self.plan.master_gain;
            let mut e_proc = 0.0;
            let mut e_dry = 0.0;
            for k in 0..n {
                e_dry += self.dl[k] * self.dl[k] + self.dr[k] * self.dr[k];
                e_proc += ol[k] * ol[k] + or[k] * or[k];
            }
            // loudness matching for "hold for original": slow RMS of both paths
            if !self.hold {
                self.rms_proc = 0.98 * self.rms_proc + 0.02 * (e_proc * mg * mg / n as f32);
                self.rms_dry = 0.98 * self.rms_dry + 0.02 * (e_dry / n as f32);
                if self.rms_dry > 1e-7 {
                    self.hold_gain = (self.rms_proc / self.rms_dry).sqrt().clamp(0.05, 8.0);
                }
            }
            let m0 = self.mg_s;
            glide(
                &mut self.mg_s,
                mg,
                glide_k(n, self.plan.project.sample_rate as f32),
            );
            let m1 = self.mg_s;
            let mut pk = [0f32; 2];
            for k in 0..n {
                if self.fade_in < 1.0 {
                    self.fade_in = (self.fade_in + 1.0 / 2400.0).min(1.0);
                }
                let g = if self.hold {
                    self.hold_gain
                } else {
                    m0 + (m1 - m0) * k as f32 / n as f32
                };
                let f = g * self.fade_in;
                ol[k] *= f;
                or[k] *= f;
                if self.safety {
                    ol[k] = ol[k].clamp(-0.98, 0.98);
                    or[k] = or[k].clamp(-0.98, 0.98);
                }
                pk[0] = pk[0].max(ol[k].abs());
                pk[1] = pk[1].max(or[k].abs());
            }
            self.peak = pk;
            if let Some((a, b)) = self.loop_range {
                if b > a && self.pos as i64 >= b {
                    self.seek(a);
                }
            }
            done += n;
        }
    }

    fn varispeed(&mut self, ol: &mut [f32], or: &mut [f32], speed: f64) {
        let n = ol.len();
        if self.fcount == 0 {
            self.fstart = self.pos.floor() - 1.0;
            self.frender = self.fstart as i64;
        }
        for k in 0..n {
            // ensure 4 samples of FIFO around the read position
            loop {
                let have_to = self.fstart + self.fcount as f64;
                if have_to >= self.pos + 3.0 && self.fcount >= 4 {
                    break;
                }
                if self.fcount + BLOCK > FIFO {
                    // drop consumed samples
                    let drop = ((self.pos - self.fstart).floor() as usize)
                        .saturating_sub(2)
                        .min(self.fcount);
                    self.fl.copy_within(drop..self.fcount, 0);
                    self.fr.copy_within(drop..self.fcount, 0);
                    self.fcount -= drop;
                    self.fstart += drop as f64;
                }
                let from = self.frender;
                self.mix(from.max(0), BLOCK);
                let c = self.fcount;
                if from < 0 {
                    self.fl[c..c + BLOCK].fill(0.0);
                    self.fr[c..c + BLOCK].fill(0.0);
                } else {
                    self.fl[c..c + BLOCK].copy_from_slice(&self.ml[..BLOCK]);
                    self.fr[c..c + BLOCK].copy_from_slice(&self.mr[..BLOCK]);
                }
                self.fcount += BLOCK;
                self.frender += BLOCK as i64;
            }
            let x = self.pos - self.fstart;
            let i = x.floor() as usize;
            let t = (x - i as f64) as f32;
            let i = i.max(1);
            ol[k] = hermite(
                self.fl[i - 1],
                self.fl[i],
                self.fl[i + 1],
                self.fl[i + 2],
                t,
            );
            or[k] = hermite(
                self.fr[i - 1],
                self.fr[i],
                self.fr[i + 1],
                self.fr[i + 2],
                t,
            );
            self.pos += speed;
        }
        // the dry path is not meaningful under varispeed; make hold match silence-free
        self.dl[..n].copy_from_slice(&ol[..n]);
        self.dr[..n].copy_from_slice(&or[..n]);
    }

    /// Offline render of `[start, end)` (timeline) with tails. Calls `progress(0..1)`;
    /// returns None if `cancel` was set.
    pub fn render_offline(
        &mut self,
        start: i64,
        end: i64,
        tail: bool,
        progress: &mut dyn FnMut(f32) -> bool,
    ) -> Option<(Vec<f32>, Vec<f32>)> {
        self.seek(start);
        self.fade_in = 1.0;
        self.playing = true;
        let lat = self.latency();
        let speed = self.plan.speed as f64;
        let out_len = (((end - start) as f64) / speed).ceil() as usize;
        let tail_len = if tail {
            (self.plan.project.sample_rate as usize) * 6
        } else {
            0
        };
        let total = lat + out_len + tail_len;
        let mut l = Vec::with_capacity(total);
        let mut r = Vec::with_capacity(total);
        let mut bl = vec![0.0; BLOCK];
        let mut br = vec![0.0; BLOCK];
        let mut quiet = 0usize;
        while l.len() < total {
            let n = BLOCK.min(total - l.len());
            self.render(&mut bl[..n], &mut br[..n]);
            l.extend_from_slice(&bl[..n]);
            r.extend_from_slice(&br[..n]);
            if l.len() > lat + out_len {
                let pk = bl[..n]
                    .iter()
                    .chain(&br[..n])
                    .fold(0f32, |m, v| m.max(v.abs()));
                quiet = if pk < 3e-5 { quiet + n } else { 0 };
                if quiet > 24000 {
                    break;
                }
            }
            if l.len() % (BLOCK * 64) == 0 && !progress(l.len() as f32 / total as f32) {
                return None;
            }
        }
        self.playing = false;
        let l = l.split_off(lat.min(l.len()));
        let r = r.split_off(lat.min(r.len()));
        // trim the silent part of the tail
        let mut keep = l.len();
        while keep > out_len && l[keep - 1].abs() < 3e-5 && r[keep - 1].abs() < 3e-5 {
            keep -= 1;
        }
        Some((l[..keep].to_vec(), r[..keep].to_vec()))
    }
}

/// Convenience: render a project once.
pub fn render_project(
    p: &Project,
    media: Arc<Pool>,
    range: Option<(i64, i64)>,
    tail: bool,
) -> (Vec<f32>, Vec<f32>) {
    let mut r = Renderer::new(Plan::build(p, media));
    let (a, b) = range.unwrap_or((0, p.length()));
    r.render_offline(a, b, tail, &mut |_| true)
        .unwrap_or_default()
}

/// Map of every effect's resolved params, for diffing on the control thread.
pub fn param_map(p: &Project) -> BTreeMap<u32, (P, bool, f32)> {
    let mut m = BTreeMap::new();
    let mut add = |e: &Effect| {
        if let Some(s) = fx::spec(&e.kind) {
            m.insert(e.id, (fx::resolve(s, &e.params), e.enabled, e.mix));
        }
    };
    p.master.fx.iter().for_each(&mut add);
    for t in &p.tracks {
        t.fx.iter().for_each(&mut add);
        for c in &t.clips {
            c.fx.iter().for_each(&mut add);
        }
    }
    m
}

/// True when two projects differ only in values that can be sent as messages.
pub fn same_structure(a: &Project, b: &Project) -> bool {
    let shape = |p: &Project| {
        let mut s = String::new();
        s.push_str(&format!("{}|", p.sample_rate));
        for e in &p.master.fx {
            s.push_str(&format!("m{}:{}:{}|", e.id, e.kind, e.path.len()));
            if e.kind == "speed" {
                s.push('s');
            }
        }
        for t in &p.tracks {
            s.push_str(&format!("t{}[", t.id));
            for e in &t.fx {
                s.push_str(&format!("{}:{}:{},", e.id, e.kind, e.path.len()));
            }
            for c in &t.clips {
                s.push_str(&format!(
                    "c{}:{}:{}:{}:{}:{}:{}:{}:{}:{},",
                    c.id,
                    c.media,
                    c.start,
                    c.offset,
                    c.len,
                    c.gain_db,
                    c.fade_in,
                    c.fade_out,
                    c.reverse,
                    c.curve
                ));
                for e in &c.fx {
                    s.push_str(&format!("{}:{}", e.id, e.kind));
                }
            }
            s.push_str(&serde_json::to_string(&t.lanes).unwrap_or_default());
            s.push(']');
        }
        s.push_str(&format!("{}", p.tempo));
        s
    };
    shape(a) == shape(b)
        && a.master
            .fx
            .iter()
            .filter(|e| e.kind == "speed")
            .map(|e| (e.enabled, e.params.clone()))
            .collect::<Vec<_>>()
            == b.master
                .fx
                .iter()
                .filter(|e| e.kind == "speed")
                .map(|e| (e.enabled, e.params.clone()))
                .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::Media;
    use crate::project::MediaRef;

    fn setup(secs: usize) -> (Project, Arc<Pool>) {
        let sr = 48000;
        let n = sr * secs;
        let l: Vec<f32> = (0..n)
            .map(|i| (crate::dsp::TAU * 220.0 * i as f32 / sr as f32).sin() * 0.4)
            .collect();
        let m = Media::new("m1", vec![l.clone(), l], sr as u32, sr as u32);
        let mut pool = Pool::new();
        pool.insert("m1".into(), Arc::new(m));
        let mut p = Project::new("t", sr as u32);
        p.media.push(MediaRef {
            id: "m1".into(),
            name: "a.wav".into(),
            path: "a".into(),
            frames: n as i64,
            channels: 2,
            source_rate: sr as u32,
            ..Default::default()
        });
        let t = p.add_track("A");
        p.add_clip(t, "m1", 0, 0, None).unwrap();
        (p, Arc::new(pool))
    }

    #[test]
    fn unprocessed_render_is_bit_exact() {
        let (p, pool) = setup(2);
        let (l, _) = render_project(&p, pool.clone(), None, false);
        let m = &pool["m1"];
        assert_eq!(l.len(), 96000);
        for (i, v) in l.iter().enumerate() {
            assert_eq!(v.to_bits(), m.get(0, i).to_bits(), "sample {i}");
        }
    }

    /// Largest jump between neighbouring samples of a DC signal while gains change.
    fn max_step(out: &[f32]) -> f32 {
        out.windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0, f32::max)
    }

    #[test]
    fn parameter_and_gain_changes_glide_without_clicks() {
        let sr = 48000usize;
        let n = sr;
        let m = Media::new("dc", vec![vec![0.5; n], vec![0.5; n]], sr as u32, sr as u32);
        let mut pool = Pool::new();
        pool.insert("dc".into(), Arc::new(m));
        let mut p = Project::new("t", sr as u32);
        p.media.push(MediaRef {
            id: "dc".into(),
            name: "dc".into(),
            path: "dc".into(),
            frames: n as i64,
            channels: 2,
            source_rate: sr as u32,
            ..Default::default()
        });
        let t = p.add_track("A");
        p.add_clip(t, "dc", 0, 0, None).unwrap();
        let fx = p
            .add_fx("master", "gain", None, &BTreeMap::new(), None)
            .unwrap();
        let mut r = Renderer::new(Plan::build(&p, Arc::new(pool)));
        r.fade_in = 1.0;
        r.playing = true;
        let mut out = vec![];
        let block = |r: &mut Renderer, out: &mut Vec<f32>, len: usize| {
            let mut a = vec![0.0; len];
            let mut b = vec![0.0; len];
            r.render(&mut a, &mut b);
            out.extend(a);
        };
        block(&mut r, &mut out, 4800);
        // effect gain -24 dB, then track gain and master gain jumps
        let mut q = crate::fx::resolve(crate::fx::spec("gain").unwrap(), &BTreeMap::new());
        q[0] = -24.0;
        r.apply(Msg::Params { fx, p: q });
        block(&mut r, &mut out, 9600);
        r.apply(Msg::Track {
            id: t,
            gain: 2.0,
            pan: 0.0,
            mute: false,
            solo: false,
        });
        r.apply(Msg::MasterGain(0.25));
        block(&mut r, &mut out, 9600);
        let step = max_step(&out[4000..]);
        // an instant -24 dB jump on 0.5 would be a 0.47 step; a 20 ms glide is ~0.001 per sample
        assert!(step < 0.01, "largest sample step {step}");
        // settled at the targets: 0.5 * 10^(-24/20) * 2 * 0.25
        let want = 0.5 * 10f32.powf(-24.0 / 20.0) * 2.0 * 0.25;
        let last = *out.last().unwrap();
        assert!(
            (last - want).abs() < want * 0.02,
            "settled at {last}, want {want}"
        );
    }

    #[test]
    fn realtime_and_offline_null() {
        let (mut p, pool) = setup(2);
        let tid = p.tracks[0].id;
        p.add_fx(
            &format!("track:{tid}"),
            "compressor",
            None,
            &BTreeMap::new(),
            None,
        )
        .unwrap();
        p.add_fx("master", "reverb", None, &BTreeMap::new(), None)
            .unwrap();
        let (off, _) = render_project(&p, pool.clone(), Some((0, 96000)), false);
        // "real time": odd callback sizes, as an audio device would ask for
        let mut r = Renderer::new(Plan::build(&p, pool));
        r.fade_in = 1.0;
        r.playing = true;
        let mut out = vec![];
        let sizes = [441usize, 128, 1000, 64, 333];
        let mut k = 0;
        while out.len() < 96000 {
            let n = sizes[k % sizes.len()].min(96000 - out.len());
            let mut a = vec![0.0; n];
            let mut b = vec![0.0; n];
            r.render(&mut a, &mut b);
            out.extend(a);
            k += 1;
        }
        let mut diff = 0f32;
        for i in 0..90000 {
            diff = diff.max((out[i] - off[i]).abs());
        }
        assert!(diff < 1e-5, "max difference {diff}");
    }

    #[test]
    fn speed_changes_duration() {
        let (mut p, pool) = setup(2);
        let mut m = BTreeMap::new();
        m.insert("rate".into(), 0.8);
        p.add_fx("master", "speed", None, &m, None).unwrap();
        let (l, _) = render_project(&p, pool, None, false);
        assert!(
            (l.len() as f32 - 96000.0 / 0.8).abs() < 600.0,
            "len {}",
            l.len()
        );
    }

    #[test]
    fn fades_and_mute() {
        let (mut p, pool) = setup(1);
        let c = p.tracks[0].clips[0].id;
        p.clip_mut(c).unwrap().fade_in = 4800;
        let (l, _) = render_project(&p, pool.clone(), None, false);
        assert_eq!(l[0], 0.0);
        assert!(l[100].abs() < 0.03);
        p.tracks[0].mute = true;
        let (l, _) = render_project(&p, pool, None, false);
        assert!(l.iter().all(|v| *v == 0.0));
    }
}
