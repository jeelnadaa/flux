//! Project model and edit operations. Positions are integer samples at the project rate.
//! Source audio is never modified: clips reference media by id.

use crate::fx;
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const FORMAT_VERSION: u32 = 2;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Effect {
    pub id: u32,
    pub kind: String,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "one")]
    pub mix: f32,
    #[serde(default)]
    pub params: BTreeMap<String, f32>,
    /// Which preset card added it (e.g. "slowed"), so creative screens can find their effects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// Learned noise profile (noise reduction).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<Vec<f32>>,
    /// Media id of an impulse response (convolver).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ir: Option<String>,
    /// Drawn motion path for the stage, x right / y down in [-1, 1].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<[f32; 2]>,
    /// A/B/C parameter slots.
    #[serde(default)]
    pub slots: Vec<Option<BTreeMap<String, f32>>>,
    #[serde(default)]
    pub slot: u8,
}
fn yes() -> bool {
    true
}
fn one() -> f32 {
    1.0
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Clip {
    pub id: u32,
    pub media: String,
    pub name: String,
    /// Timeline start in samples.
    pub start: i64,
    /// Offset into the media in samples.
    pub offset: i64,
    pub len: i64,
    #[serde(default)]
    pub gain_db: f32,
    #[serde(default)]
    pub fade_in: i64,
    #[serde(default)]
    pub fade_out: i64,
    /// "equal" (equal power), "s", "log", "lin"
    #[serde(default = "equal")]
    pub curve: String,
    #[serde(default)]
    pub reverse: bool,
    #[serde(default)]
    pub fx: Vec<Effect>,
}
fn equal() -> String {
    "equal".into()
}
impl Clip {
    pub fn end(&self) -> i64 {
        self.start + self.len
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Lane {
    pub id: u32,
    /// "volume" (dB), "pan" (-1..1), or "fx:<effect id>:<param key>"
    pub target: String,
    /// (sample, value) points sorted by sample
    pub points: Vec<[f64; 2]>,
    #[serde(default = "linear")]
    pub shape: String,
}
fn linear() -> String {
    "linear".into()
}
impl Lane {
    pub fn value_at(&self, s: i64) -> Option<f32> {
        let p = &self.points;
        if p.is_empty() {
            return None;
        }
        let x = s as f64;
        if x <= p[0][0] {
            return Some(p[0][1] as f32);
        }
        for w in p.windows(2) {
            if x <= w[1][0] {
                let t = (x - w[0][0]) / (w[1][0] - w[0][0]).max(1.0);
                let t = match self.shape.as_str() {
                    "hold" => 0.0,
                    "smooth" => t * t * (3.0 - 2.0 * t),
                    _ => t,
                };
                return Some((w[0][1] + (w[1][1] - w[0][1]) * t) as f32);
            }
        }
        Some(p[p.len() - 1][1] as f32)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Track {
    pub id: u32,
    pub name: String,
    #[serde(default = "c1")]
    pub color: u8,
    #[serde(default)]
    pub gain_db: f32,
    #[serde(default)]
    pub pan: f32,
    #[serde(default)]
    pub mute: bool,
    #[serde(default)]
    pub solo: bool,
    #[serde(default)]
    pub arm: bool,
    #[serde(default)]
    pub fx: Vec<Effect>,
    #[serde(default)]
    pub clips: Vec<Clip>,
    #[serde(default)]
    pub lanes: Vec<Lane>,
}
fn c1() -> u8 {
    1
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Master {
    #[serde(default)]
    pub gain_db: f32,
    #[serde(default)]
    pub fx: Vec<Effect>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Marker {
    pub id: u32,
    pub pos: i64,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct MediaRef {
    pub id: String,
    pub name: String,
    /// File inside the project's media folder, or an absolute path when linked.
    pub path: String,
    #[serde(default)]
    pub original: String,
    #[serde(default)]
    pub linked: bool,
    pub frames: i64,
    pub channels: u16,
    pub source_rate: u32,
    #[serde(default)]
    pub format: String,
    #[serde(default)]
    pub bits: u16,
    #[serde(default)]
    pub tags: BTreeMap<String, String>,
    #[serde(default)]
    pub synthetic: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Selection {
    pub start: i64,
    pub end: i64,
    #[serde(default)]
    pub track: Option<u32>,
    #[serde(default)]
    pub clip: Option<u32>,
    #[serde(default)]
    pub fx: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Project {
    pub version: u32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: String,
    pub sample_rate: u32,
    #[serde(default = "tempo")]
    pub tempo: f32,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub seed: u64,
    #[serde(default)]
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub master: Master,
    #[serde(default)]
    pub markers: Vec<Marker>,
    #[serde(default)]
    pub media: Vec<MediaRef>,
    #[serde(default)]
    pub selection: Selection,
    #[serde(default)]
    pub loop_on: bool,
    #[serde(default)]
    pub loop_range: Option<(i64, i64)>,
    #[serde(default)]
    pub next_id: u32,
    #[serde(default)]
    pub created: u64,
    #[serde(default)]
    pub modified: u64,
    /// Target loudness for export and auto-master, in LUFS.
    #[serde(default = "target")]
    pub target_lufs: f32,
    /// Library folder tag ("My Audio", "Client work", ...).
    #[serde(default)]
    pub folder: String,
}
fn tempo() -> f32 {
    120.0
}
fn target() -> f32 {
    -14.0
}

pub fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl Project {
    pub fn new(name: &str, sample_rate: u32) -> Self {
        let t = now_secs();
        let id = format!(
            "p{:x}{:04x}",
            t,
            (crate::dsp::Rng::new(t ^ 0xF1).next_u64() & 0xffff)
        );
        Project {
            version: FORMAT_VERSION,
            id,
            name: name.to_string(),
            kind: "blank".into(),
            sample_rate,
            tempo: 120.0,
            key: String::new(),
            seed: t,
            tracks: vec![],
            master: Master::default(),
            markers: vec![],
            media: vec![],
            selection: Selection::default(),
            loop_on: false,
            loop_range: None,
            next_id: 1,
            created: t,
            modified: t,
            target_lufs: -14.0,
            folder: "My Audio".into(),
        }
    }

    /// Reject projects that could not have been written by Flux (hostile input).
    pub fn validate(&mut self) -> Result<()> {
        if crate::store::safe_id(&self.id).is_err() {
            bail!("the project id is invalid");
        }
        if !(8000..=384_000).contains(&self.sample_rate) {
            bail!("the project sample rate is invalid");
        }
        if self.version > FORMAT_VERSION {
            bail!("this project was made by a newer version of Flux");
        }
        if self.tracks.len() > 512 || self.media.len() > 4096 {
            bail!("the project is too large");
        }
        if !(20.0..=400.0).contains(&self.tempo) {
            self.tempo = 120.0;
        }
        let mut max_id = 0;
        for t in &mut self.tracks {
            max_id = max_id.max(t.id);
            t.gain_db = t.gain_db.clamp(-96.0, 24.0);
            t.pan = t.pan.clamp(-1.0, 1.0);
            for c in &mut t.clips {
                max_id = max_id.max(c.id);
                if c.len <= 0 || c.offset < 0 || c.start < 0 {
                    bail!("a clip in the project is damaged");
                }
                c.fade_in = c.fade_in.clamp(0, c.len);
                c.fade_out = c.fade_out.clamp(0, c.len);
                for e in &c.fx {
                    max_id = max_id.max(e.id);
                }
            }
            for e in &t.fx {
                max_id = max_id.max(e.id);
            }
            for l in &mut t.lanes {
                max_id = max_id.max(l.id);
                l.points.retain(|p| p[0].is_finite() && p[1].is_finite());
                l.points.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());
            }
        }
        for e in &self.master.fx {
            max_id = max_id.max(e.id);
        }
        for m in &self.markers {
            max_id = max_id.max(m.id);
        }
        for e in self
            .tracks
            .iter_mut()
            .flat_map(|t| t.fx.iter_mut())
            .chain(self.master.fx.iter_mut())
        {
            if let Some(sp) = fx::spec(&e.kind) {
                for p in sp.params {
                    if let Some(v) = e.params.get_mut(p.key) {
                        *v = if v.is_finite() {
                            v.clamp(p.min, p.max)
                        } else {
                            p.def
                        };
                    }
                }
            }
        }
        self.next_id = self.next_id.max(max_id + 1);
        Ok(())
    }

    pub fn alloc(&mut self) -> u32 {
        let i = self.next_id.max(1);
        self.next_id = i + 1;
        i
    }

    /// Stable content hash (FNV-1a over canonical JSON, ignoring timestamps).
    pub fn hash(&self) -> u64 {
        let mut c = self.clone();
        c.modified = 0;
        let s = serde_json::to_string(&c).unwrap_or_default();
        let mut h: u64 = 0xcbf29ce484222325;
        for b in s.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    /// Timeline length in samples (end of the last clip).
    pub fn length(&self) -> i64 {
        self.tracks
            .iter()
            .flat_map(|t| t.clips.iter())
            .map(|c| c.end())
            .max()
            .unwrap_or(0)
    }

    pub fn media_ref(&self, id: &str) -> Option<&MediaRef> {
        self.media.iter().find(|m| m.id == id)
    }

    pub fn track(&self, id: u32) -> Result<&Track> {
        self.tracks
            .iter()
            .find(|t| t.id == id)
            .ok_or_else(|| anyhow!("no track {id}"))
    }
    pub fn track_mut(&mut self, id: u32) -> Result<&mut Track> {
        self.tracks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| anyhow!("no track {id}"))
    }
    pub fn clip_loc(&self, id: u32) -> Result<(usize, usize)> {
        for (ti, t) in self.tracks.iter().enumerate() {
            if let Some(ci) = t.clips.iter().position(|c| c.id == id) {
                return Ok((ti, ci));
            }
        }
        bail!("no clip {id}")
    }
    pub fn clip(&self, id: u32) -> Result<&Clip> {
        let (t, c) = self.clip_loc(id)?;
        Ok(&self.tracks[t].clips[c])
    }
    pub fn clip_mut(&mut self, id: u32) -> Result<&mut Clip> {
        let (t, c) = self.clip_loc(id)?;
        Ok(&mut self.tracks[t].clips[c])
    }

    pub fn add_track(&mut self, name: &str) -> u32 {
        let id = self.alloc();
        let color = (self.tracks.len() % 8) as u8 + 1;
        self.tracks.push(Track {
            id,
            name: name.to_string(),
            color,
            gain_db: 0.0,
            pan: 0.0,
            mute: false,
            solo: false,
            arm: false,
            fx: vec![],
            clips: vec![],
            lanes: vec![],
        });
        id
    }

    pub fn remove_track(&mut self, id: u32) -> Result<()> {
        let i = self
            .tracks
            .iter()
            .position(|t| t.id == id)
            .ok_or_else(|| anyhow!("no track {id}"))?;
        self.tracks.remove(i);
        Ok(())
    }

    pub fn move_track(&mut self, id: u32, index: usize) -> Result<()> {
        let i = self
            .tracks
            .iter()
            .position(|t| t.id == id)
            .ok_or_else(|| anyhow!("no track {id}"))?;
        let t = self.tracks.remove(i);
        self.tracks.insert(index.min(self.tracks.len()), t);
        Ok(())
    }

    pub fn add_clip(
        &mut self,
        track: u32,
        media: &str,
        start: i64,
        offset: i64,
        len: Option<i64>,
    ) -> Result<u32> {
        let m = self
            .media_ref(media)
            .ok_or_else(|| anyhow!("no media {media}"))?
            .clone();
        let offset = offset.clamp(0, m.frames);
        let len = len
            .unwrap_or(m.frames - offset)
            .clamp(1, (m.frames - offset).max(1));
        let id = self.alloc();
        let name = m
            .name
            .rsplit_once('.')
            .map(|(a, _)| a.to_string())
            .unwrap_or(m.name.clone());
        let t = self.track_mut(track)?;
        t.clips.push(Clip {
            id,
            media: media.to_string(),
            name,
            start: start.max(0),
            offset,
            len,
            gain_db: 0.0,
            fade_in: 0,
            fade_out: 0,
            curve: "equal".into(),
            reverse: false,
            fx: vec![],
        });
        t.clips.sort_by_key(|c| c.start);
        Ok(id)
    }

    pub fn move_clip(&mut self, id: u32, start: i64, track: Option<u32>) -> Result<()> {
        let (ti, ci) = self.clip_loc(id)?;
        let mut c = self.tracks[ti].clips.remove(ci);
        c.start = start.max(0);
        let dest = match track {
            Some(t) => self
                .tracks
                .iter()
                .position(|x| x.id == t)
                .ok_or_else(|| anyhow!("no track {t}"))?,
            None => ti,
        };
        self.tracks[dest].clips.push(c);
        self.tracks[dest].clips.sort_by_key(|c| c.start);
        Ok(())
    }

    /// Trim a clip's edges. `start`/`end` are new timeline edges; the media offset follows.
    pub fn trim_clip(&mut self, id: u32, start: Option<i64>, end: Option<i64>) -> Result<()> {
        let frames = {
            let c = self.clip(id)?;
            self.media_ref(&c.media)
                .map(|m| m.frames)
                .unwrap_or(i64::MAX)
        };
        let c = self.clip_mut(id)?;
        if let Some(s) = start {
            let s = s.max(c.start - c.offset).min(c.end() - 1).max(0);
            let d = s - c.start;
            c.start = s;
            c.offset += d;
            c.len -= d;
        }
        if let Some(e) = end {
            let max_end = c.start + (frames - c.offset);
            let e = e.clamp(c.start + 1, max_end);
            c.len = e - c.start;
        }
        c.fade_in = c.fade_in.min(c.len);
        c.fade_out = c.fade_out.min(c.len);
        Ok(())
    }

    /// Split a clip at timeline sample `at`. Returns the id of the right-hand part.
    pub fn split_clip(&mut self, id: u32, at: i64) -> Result<u32> {
        let (ti, ci) = self.clip_loc(id)?;
        let c = self.tracks[ti].clips[ci].clone();
        if at <= c.start || at >= c.end() {
            bail!("split point is outside the clip");
        }
        let nid = self.alloc();
        let left_len = at - c.start;
        let mut right = c.clone();
        right.id = nid;
        right.start = at;
        right.len = c.len - left_len;
        if !c.reverse {
            right.offset = c.offset + left_len;
        }
        right.fade_in = 0;
        right.fade_out = c.fade_out.min(right.len);
        right.fx = c.fx.iter().map(|e| Effect { id: 0, ..e.clone() }).collect();
        for e in &mut right.fx {
            e.id = self.next_id;
            self.next_id += 1;
        }
        let left = &mut self.tracks[ti].clips[ci];
        left.len = left_len;
        if left.reverse {
            left.offset = c.offset + right.len;
        }
        left.fade_out = 0;
        left.fade_in = left.fade_in.min(left.len);
        self.tracks[ti].clips.insert(ci + 1, right);
        Ok(nid)
    }

    pub fn delete_clip(&mut self, id: u32) -> Result<()> {
        let (ti, ci) = self.clip_loc(id)?;
        self.tracks[ti].clips.remove(ci);
        Ok(())
    }

    pub fn duplicate_clip(&mut self, id: u32) -> Result<u32> {
        let (ti, ci) = self.clip_loc(id)?;
        let mut c = self.tracks[ti].clips[ci].clone();
        c.id = self.alloc();
        c.start = c.end();
        for e in &mut c.fx {
            e.id = self.next_id;
            self.next_id += 1;
        }
        let nid = c.id;
        self.tracks[ti].clips.push(c);
        self.tracks[ti].clips.sort_by_key(|c| c.start);
        Ok(nid)
    }

    /// Remove `[start, end)` from the timeline. With ripple, later material moves left.
    /// `tracks` limits the edit; `None` means every track.
    pub fn cut_range(
        &mut self,
        start: i64,
        end: i64,
        ripple: bool,
        tracks: Option<&[u32]>,
    ) -> Result<()> {
        if end <= start {
            bail!("empty range");
        }
        let d = end - start;
        let ids: Vec<u32> = self
            .tracks
            .iter()
            .filter(|t| tracks.map(|v| v.contains(&t.id)).unwrap_or(true))
            .map(|t| t.id)
            .collect();
        for tid in ids {
            // split clips crossing the edges
            let crossing: Vec<(u32, i64)> = self
                .track(tid)?
                .clips
                .iter()
                .flat_map(|c| {
                    [
                        (c.id, start, c.start < start && c.end() > start),
                        (c.id, end, c.start < end && c.end() > end),
                    ]
                })
                .filter(|x| x.2)
                .map(|x| (x.0, x.1))
                .collect();
            for (cid, at) in crossing {
                // the id may now refer to the left part; find the clip that contains `at`
                let target = self
                    .track(tid)?
                    .clips
                    .iter()
                    .find(|c| c.start < at && c.end() > at)
                    .map(|c| c.id)
                    .unwrap_or(cid);
                let _ = self.split_clip(target, at);
            }
            let t = self.track_mut(tid)?;
            t.clips.retain(|c| !(c.start >= start && c.end() <= end));
            if ripple {
                for c in &mut t.clips {
                    if c.start >= end {
                        c.start -= d;
                    }
                }
                for l in &mut t.lanes {
                    l.points
                        .retain(|p| p[0] < start as f64 || p[0] >= end as f64);
                    for p in &mut l.points {
                        if p[0] >= end as f64 {
                            p[0] -= d as f64;
                        }
                    }
                }
            }
        }
        if ripple && tracks.is_none() {
            self.markers.retain(|m| m.pos < start || m.pos >= end);
            for m in &mut self.markers {
                if m.pos >= end {
                    m.pos -= d;
                }
            }
        }
        Ok(())
    }

    /// Insert `len` samples of silence at `at` on every track, moving later material right.
    pub fn insert_silence(&mut self, at: i64, len: i64) -> Result<()> {
        let ids: Vec<u32> = self.tracks.iter().map(|t| t.id).collect();
        for tid in ids {
            let cross: Vec<u32> = self
                .track(tid)?
                .clips
                .iter()
                .filter(|c| c.start < at && c.end() > at)
                .map(|c| c.id)
                .collect();
            for c in cross {
                self.split_clip(c, at)?;
            }
            for c in &mut self.track_mut(tid)?.clips {
                if c.start >= at {
                    c.start += len;
                }
            }
        }
        Ok(())
    }

    // ---- effects ----

    pub fn chain_mut(&mut self, target: &str) -> Result<&mut Vec<Effect>> {
        if target == "master" {
            return Ok(&mut self.master.fx);
        }
        if let Some(t) = target.strip_prefix("track:") {
            return Ok(&mut self.track_mut(t.parse()?)?.fx);
        }
        if let Some(c) = target.strip_prefix("clip:") {
            return Ok(&mut self.clip_mut(c.parse()?)?.fx);
        }
        bail!("target must be master, track:<id> or clip:<id>")
    }

    pub fn find_fx(&self, id: u32) -> Option<(String, &Effect)> {
        if let Some(e) = self.master.fx.iter().find(|e| e.id == id) {
            return Some(("master".into(), e));
        }
        for t in &self.tracks {
            if let Some(e) = t.fx.iter().find(|e| e.id == id) {
                return Some((format!("track:{}", t.id), e));
            }
            for c in &t.clips {
                if let Some(e) = c.fx.iter().find(|e| e.id == id) {
                    return Some((format!("clip:{}", c.id), e));
                }
            }
        }
        None
    }

    pub fn fx_mut(&mut self, id: u32) -> Result<&mut Effect> {
        let (target, _) = self.find_fx(id).ok_or_else(|| anyhow!("no effect {id}"))?;
        let chain = self.chain_mut(&target)?;
        chain
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or_else(|| anyhow!("no effect {id}"))
    }

    pub fn add_fx(
        &mut self,
        target: &str,
        kind: &str,
        index: Option<usize>,
        params: &BTreeMap<String, f32>,
        group: Option<String>,
    ) -> Result<u32> {
        let spec = fx::spec(kind).ok_or_else(|| anyhow!("unknown effect '{kind}'"))?;
        let target = if spec.master_only { "master" } else { target };
        let mut p = fx::defaults(spec);
        for (k, v) in params {
            if !p.contains_key(k) {
                bail!("effect '{kind}' has no parameter '{k}'");
            }
            p.insert(k.clone(), *v);
        }
        let id = self.alloc();
        let chain = self.chain_mut(target)?;
        if spec.master_only {
            // only one speed stage; replace it
            chain.retain(|e| e.kind != kind);
        }
        let e = Effect {
            id,
            kind: kind.into(),
            enabled: true,
            mix: spec.default_mix,
            params: p,
            group,
            profile: None,
            ir: None,
            path: vec![],
            slots: vec![None, None, None],
            slot: 0,
        };
        let i = index.unwrap_or(chain.len()).min(chain.len());
        chain.insert(i, e);
        Ok(id)
    }

    pub fn remove_fx(&mut self, id: u32) -> Result<()> {
        let (target, _) = self.find_fx(id).ok_or_else(|| anyhow!("no effect {id}"))?;
        self.chain_mut(&target)?.retain(|e| e.id != id);
        for t in &mut self.tracks {
            t.lanes
                .retain(|l| !l.target.starts_with(&format!("fx:{id}:")));
        }
        Ok(())
    }

    pub fn move_fx(&mut self, id: u32, index: usize) -> Result<()> {
        let (target, _) = self.find_fx(id).ok_or_else(|| anyhow!("no effect {id}"))?;
        let chain = self.chain_mut(&target)?;
        let i = chain.iter().position(|e| e.id == id).unwrap();
        let e = chain.remove(i);
        chain.insert(index.min(chain.len()), e);
        Ok(())
    }

    pub fn set_fx_params(&mut self, id: u32, params: &BTreeMap<String, f32>) -> Result<()> {
        let e = self.fx_mut(id)?;
        let spec = fx::spec(&e.kind).ok_or_else(|| anyhow!("unknown effect"))?;
        for (k, v) in params {
            let ps = spec
                .params
                .iter()
                .find(|p| p.key == k)
                .ok_or_else(|| anyhow!("effect '{}' has no parameter '{k}'", e.kind))?;
            if !v.is_finite() {
                bail!("parameter '{k}' must be a finite number");
            }
            e.params.insert(k.clone(), v.clamp(ps.min, ps.max));
        }
        Ok(())
    }

    /// Store the current parameters in the active slot and load slot `n` (0..3).
    pub fn fx_slot(&mut self, id: u32, n: u8) -> Result<()> {
        let e = self.fx_mut(id)?;
        if n > 2 {
            bail!("slot must be 0, 1 or 2");
        }
        while e.slots.len() < 3 {
            e.slots.push(None);
        }
        let cur = e.slot as usize;
        e.slots[cur] = Some(e.params.clone());
        if let Some(p) = e.slots[n as usize].clone() {
            e.params = p;
        }
        e.slot = n;
        Ok(())
    }

    pub fn add_marker(&mut self, pos: i64, name: &str) -> u32 {
        let id = self.alloc();
        self.markers.push(Marker {
            id,
            pos,
            name: name.into(),
        });
        self.markers.sort_by_key(|m| m.pos);
        id
    }

    pub fn set_lane(
        &mut self,
        track: u32,
        target: &str,
        points: Vec<[f64; 2]>,
        shape: Option<String>,
    ) -> Result<u32> {
        let id = self.alloc();
        let t = self.track_mut(track)?;
        let mut pts = points;
        pts.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap_or(std::cmp::Ordering::Equal));
        if let Some(l) = t.lanes.iter_mut().find(|l| l.target == target) {
            l.points = pts;
            if let Some(s) = shape {
                l.shape = s;
            }
            return Ok(l.id);
        }
        t.lanes.push(Lane {
            id,
            target: target.into(),
            points: pts,
            shape: shape.unwrap_or_else(linear),
        });
        Ok(id)
    }
}

/// Fade gain for position `t` in 0..1 with the given curve (rising).
pub fn fade_gain(curve: &str, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match curve {
        "lin" => t,
        "log" => (1.0 - (1.0 - t).powi(3)).max(0.0),
        "s" => t * t * (3.0 - 2.0 * t),
        _ => (t * std::f32::consts::FRAC_PI_2).sin(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn proj() -> Project {
        let mut p = Project::new("t", 48000);
        p.media.push(MediaRef {
            id: "m1".into(),
            name: "a.wav".into(),
            path: "a.wav".into(),
            frames: 480000,
            channels: 2,
            source_rate: 48000,
            ..Default::default()
        });
        let t = p.add_track("A");
        p.add_clip(t, "m1", 0, 0, None).unwrap();
        p
    }
    #[test]
    fn split_keeps_exact_sample_positions() {
        let mut p = proj();
        let c = p.tracks[0].clips[0].id;
        let r = p.split_clip(c, 123457).unwrap();
        let l = p.clip(c).unwrap();
        let rr = p.clip(r).unwrap();
        assert_eq!(l.len, 123457);
        assert_eq!(rr.start, 123457);
        assert_eq!(rr.offset, 123457);
        assert_eq!(l.len + rr.len, 480000);
    }
    #[test]
    fn ripple_cut_moves_later_material() {
        let mut p = proj();
        p.cut_range(48000, 96000, true, None).unwrap();
        assert_eq!(p.length(), 480000 - 48000);
        let cl = &p.tracks[0].clips;
        assert_eq!(cl.len(), 2);
        assert_eq!(cl[1].start, 48000);
        assert_eq!(cl[1].offset, 96000);
    }
    #[test]
    fn trim_respects_media_bounds() {
        let mut p = proj();
        let c = p.tracks[0].clips[0].id;
        p.trim_clip(c, Some(1000), Some(10_000_000)).unwrap();
        let cl = p.clip(c).unwrap();
        assert_eq!(cl.offset, 1000);
        assert_eq!(cl.end(), 480000);
    }
    #[test]
    fn hash_ignores_modified_time() {
        let mut p = proj();
        let h = p.hash();
        p.modified += 100;
        assert_eq!(h, p.hash());
        p.tracks[0].gain_db = 1.0;
        assert_ne!(h, p.hash());
    }
    #[test]
    fn unknown_params_are_rejected() {
        let mut p = proj();
        let mut m = BTreeMap::new();
        m.insert("nope".into(), 1.0);
        assert!(p.add_fx("master", "eq", None, &m, None).is_err());
        let id = p
            .add_fx("master", "eq", None, &BTreeMap::new(), None)
            .unwrap();
        m.clear();
        m.insert("b1_gain".into(), 99.0);
        p.set_fx_params(id, &m).unwrap();
        assert_eq!(p.find_fx(id).unwrap().1.params["b1_gain"], 18.0);
    }
}
