//! Live processing. Sound from the computer (system audio, captured from an output device) or from
//! a microphone runs through a live preset and out to any output device, in real time.
//!
//! A live preset is a small modular rig: parallel lanes of effects that are summed, a post chain
//! after the sum, modulators (LFOs, random, step sequencers, envelope followers) that move any
//! parameter, and macros that move many parameters from one knob. Every module is one of the
//! same effects the editor uses, so a preset sounds the same live and offline (`live.render`).
//!
//! Real-time rules: the audio callback never allocates, locks or does I/O. The control thread
//! builds a `Graph`, sends it over a lock-free queue, and gets the old one back to drop. Parameter
//! edits arrive as fixed-size messages and glide (`fx::Slot`).

use crate::dsp::{db_to_lin, lin_to_db, Rng};
use crate::fx::{self, Chain, Ctx, Extra, Slot, BLOCK, MAXP, P};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering::Relaxed};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const MAX_LANES: usize = 8;
pub const MAX_MODULES: usize = 16;
pub const MAX_MODS: usize = 16;
pub const MAX_TARGETS: usize = 8;
pub const MAX_MACROS: usize = 12;
pub const MAX_STEPS: usize = 32;
/// Modulation is applied every this many samples.
const CONTROL: usize = 64;
/// Effects that can't run live: speed changes the timeline, the convolver and noise reduction
/// need data taken from a file, tape stop is tied to a position in a file.
pub const NOT_LIVE: &[&str] = &["speed", "convolver", "noise", "tapestop"];

pub fn live_kind(kind: &str) -> bool {
    fx::spec(kind).is_some() && !NOT_LIVE.contains(&kind)
}

// ---------------------------------------------------------------- preset model

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Module {
    pub id: String,
    pub kind: String,
    pub enabled: bool,
    /// Dry/wet, 0..1.
    pub mix: f32,
    pub params: BTreeMap<String, f32>,
    /// Custom motion path for the spatial effect (x, y in -1..1).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<[f32; 2]>,
}
impl Default for Module {
    fn default() -> Self {
        Module {
            id: String::new(),
            kind: "gain".into(),
            enabled: true,
            mix: 1.0,
            params: BTreeMap::new(),
            path: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Lane {
    pub id: String,
    pub name: String,
    pub gain_db: f32,
    /// -1 (left) .. 1 (right).
    pub pan: f32,
    pub mute: bool,
    pub modules: Vec<Module>,
}
impl Default for Lane {
    fn default() -> Self {
        Lane {
            id: String::new(),
            name: "Lane".into(),
            gain_db: 0.0,
            pan: 0.0,
            mute: false,
            modules: vec![],
        }
    }
}

/// Where a modulator or macro points: a module's parameter, or its `mix`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Target {
    pub module: String,
    pub param: String,
    /// Modulator targets: how far the modulator moves the parameter, -1..1 of its whole range.
    pub amount: f32,
    /// Macro targets: the value at macro 0 and at macro 1.
    pub from: f32,
    pub to: f32,
    /// Macro targets: response curve, -1 (fast start) .. 1 (slow start). 0 is straight.
    pub curve: f32,
}

pub const SHAPES: &[(&str, &str)] = &[
    ("sine", "Smooth up and down."),
    ("triangle", "Straight lines up and down."),
    ("saw", "Rises, then drops at once."),
    ("ramp", "Drops, then rises at once."),
    ("square", "Jumps between two values."),
    (
        "random",
        "A new random value every cycle (sample and hold).",
    ),
    ("smooth", "Random values that glide into each other."),
    (
        "steps",
        "A step sequencer. Plays the steps you draw, one per cycle division.",
    ),
    ("follower", "Follows how loud the incoming sound is."),
    ("noise", "A random wiggle on every update."),
];

pub const SYNCS: &[&str] = &[
    "", "4/1", "2/1", "1/1", "1/2", "1/2d", "1/2t", "1/4", "1/4d", "1/4t", "1/8", "1/8d", "1/8t",
    "1/16", "1/16t", "1/32",
];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Modulator {
    pub id: String,
    pub name: String,
    pub shape: String,
    /// Cycles per second when `sync` is empty.
    pub rate_hz: f32,
    /// Tempo sync, e.g. "1/4" (one cycle per beat) or "1/8t". Empty for free rate.
    pub sync: String,
    /// 0..1.
    pub depth: f32,
    /// Shifts the whole modulation, -1..1.
    pub offset: f32,
    /// Starting point in the cycle, 0..1.
    pub phase: f32,
    /// Moves around 0 (bipolar) instead of only upward from 0.
    pub bipolar: bool,
    /// Rounds off jumps, 0..1.
    pub smooth: f32,
    /// Steps for the step sequencer, each 0..1.
    pub steps: Vec<f32>,
    /// Envelope follower attack and release.
    pub attack_ms: f32,
    pub release_ms: f32,
    pub seed: u32,
    pub targets: Vec<Target>,
}
impl Default for Modulator {
    fn default() -> Self {
        Modulator {
            id: String::new(),
            name: "LFO".into(),
            shape: "sine".into(),
            rate_hz: 0.5,
            sync: String::new(),
            depth: 0.5,
            offset: 0.0,
            phase: 0.0,
            bipolar: true,
            smooth: 0.0,
            steps: vec![],
            attack_ms: 10.0,
            release_ms: 200.0,
            seed: 1,
            targets: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Macro {
    pub id: String,
    pub name: String,
    /// 0..1.
    pub value: f32,
    pub targets: Vec<Target>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct LivePreset {
    pub format: u32,
    pub id: String,
    pub name: String,
    /// "output" (for computer sound), "input" (for a microphone) or "any".
    pub kind: String,
    pub description: String,
    pub tags: Vec<String>,
    pub builtin: bool,
    /// Beats per minute for tempo-synced modulators and effects.
    pub tempo: f32,
    pub input_gain_db: f32,
    pub output_gain_db: f32,
    /// 0 (mono) .. 2 (extra wide).
    pub width: f32,
    pub balance: f32,
    /// A transparent limiter at the very end, so nothing can blast your ears.
    pub safety_limiter: bool,
    pub lanes: Vec<Lane>,
    pub post: Vec<Module>,
    pub mods: Vec<Modulator>,
    pub macros: Vec<Macro>,
}
impl Default for LivePreset {
    fn default() -> Self {
        LivePreset {
            format: 1,
            id: String::new(),
            name: "Untitled".into(),
            kind: "any".into(),
            description: String::new(),
            tags: vec![],
            builtin: false,
            tempo: 120.0,
            input_gain_db: 0.0,
            output_gain_db: 0.0,
            width: 1.0,
            balance: 0.0,
            safety_limiter: true,
            lanes: vec![],
            post: vec![],
            mods: vec![],
            macros: vec![],
        }
    }
}

/// Tooltips for the preset-level controls, modulator and macro fields (UI and MCP).
pub fn tips() -> serde_json::Value {
    serde_json::json!({
        "preset": {
            "tempo": "Beats per minute for tempo-synced modulators, stutter and trance gate.",
            "input_gain_db": "Level of the incoming sound before any module.",
            "output_gain_db": "Level of the finished sound.",
            "width": "0 is mono, 1 unchanged, 2 extra wide. Applied after everything else.",
            "balance": "Shifts the finished sound left or right.",
            "safety_limiter": "A limiter at -1 dB at the very end so a wild setting can't blast your ears.",
            "kind": "What the preset is for: sound from your computer, your microphone, or both.",
        },
        "lane": {
            "lanes": "Lanes run side by side on the same incoming sound, then mix together. Use them for parallel effects, like a dry voice plus an octave plus a huge reverb.",
            "post": "The post chain runs on the mix of all lanes, in order.",
            "gain_db": "Level of this lane in the mix.",
            "pan": "Where this lane sits, left to right.",
            "mute": "Silences this lane.",
        },
        "module": {
            "enabled": "Turns this module on or off without losing its settings.",
            "mix": "Balance between the sound going in and this module's result.",
        },
        "mod": {
            "shape": "The movement this modulator makes.",
            "rate_hz": "Cycles per second when not synced to the tempo.",
            "sync": "Locks the speed to the tempo. 1/4 is one cycle per beat; t is triplet, d is dotted.",
            "depth": "How strong the movement is.",
            "offset": "Shifts the movement up or down.",
            "phase": "Where in its cycle it starts. Two modulators with different phase move out of step.",
            "bipolar": "On: moves both ways around the setting. Off: only moves up from it.",
            "smooth": "Rounds off sudden jumps.",
            "steps": "The values of the step sequencer, one per step.",
            "attack_ms": "Envelope follower: how fast it rises when the sound gets louder.",
            "release_ms": "Envelope follower: how fast it falls when the sound gets quieter.",
            "seed": "Starting point for the random shapes. Same seed, same pattern.",
            "amount": "How far this modulator moves this parameter, as a share of its whole range. Negative moves it the other way.",
        },
        "macro": {
            "value": "Turn this one knob to move every parameter it controls at once.",
            "from": "The parameter's value when the macro is at 0.",
            "to": "The parameter's value when the macro is at 1.",
            "curve": "How the parameter follows the knob. Negative changes fast at first, positive slow at first.",
        },
        "shapes": SHAPES.iter().map(|(k, v)| (k.to_string(), serde_json::Value::from(*v))).collect::<serde_json::Map<_, _>>(),
    })
}

fn bad(msg: impl Into<String>) -> anyhow::Error {
    crate::api::err("bad_params", msg)
}

fn clampf(v: f32, lo: f32, hi: f32, def: f32) -> f32 {
    if v.is_finite() {
        v.clamp(lo, hi)
    } else {
        def
    }
}

impl LivePreset {
    pub fn modules(&self) -> impl Iterator<Item = &Module> {
        self.lanes
            .iter()
            .flat_map(|l| l.modules.iter())
            .chain(self.post.iter())
    }
    pub fn module(&self, id: &str) -> Option<&Module> {
        self.modules().find(|m| m.id == id)
    }
    pub fn module_mut(&mut self, id: &str) -> Option<&mut Module> {
        self.lanes
            .iter_mut()
            .flat_map(|l| l.modules.iter_mut())
            .chain(self.post.iter_mut())
            .find(|m| m.id == id)
    }
    /// A fresh id with this prefix that nothing in the preset uses.
    pub fn new_id(&self, prefix: &str) -> String {
        let used = |id: &str| {
            self.modules().any(|m| m.id == id)
                || self.lanes.iter().any(|l| l.id == id)
                || self.mods.iter().any(|m| m.id == id)
                || self.macros.iter().any(|m| m.id == id)
        };
        (1..)
            .map(|i| format!("{prefix}{i}"))
            .find(|id| !used(id))
            .unwrap()
    }

    /// Check and tidy a preset: known kinds and parameters, values in range, unique ids, sane
    /// sizes, and modulator or macro targets that exist. Fills missing ids.
    pub fn validate(&mut self) -> Result<()> {
        self.format = 1;
        self.name = self.name.trim().chars().take(80).collect();
        if self.name.is_empty() {
            self.name = "Untitled".into();
        }
        self.description = self.description.chars().take(400).collect();
        self.tags.truncate(12);
        for t in &mut self.tags {
            *t = t.chars().take(24).collect();
        }
        if !["output", "input", "any"].contains(&self.kind.as_str()) {
            bail!(bad("kind must be output, input or any"));
        }
        self.tempo = clampf(self.tempo, 30.0, 300.0, 120.0);
        self.input_gain_db = clampf(self.input_gain_db, -36.0, 24.0, 0.0);
        self.output_gain_db = clampf(self.output_gain_db, -36.0, 24.0, 0.0);
        self.width = clampf(self.width, 0.0, 2.0, 1.0);
        self.balance = clampf(self.balance, -1.0, 1.0, 0.0);
        if self.lanes.len() > MAX_LANES {
            bail!(bad(format!("a preset can have at most {MAX_LANES} lanes")));
        }
        if self.post.len() > MAX_MODULES {
            bail!(bad(format!(
                "the post chain can have at most {MAX_MODULES} modules"
            )));
        }
        if self.mods.len() > MAX_MODS {
            bail!(bad(format!(
                "a preset can have at most {MAX_MODS} modulators"
            )));
        }
        if self.macros.len() > MAX_MACROS {
            bail!(bad(format!(
                "a preset can have at most {MAX_MACROS} macros"
            )));
        }
        // ids: fill blanks, then require uniqueness
        let mut seen = std::collections::HashSet::new();
        // collect existing ids first so filled ones don't collide
        for id in self
            .lanes
            .iter()
            .map(|l| &l.id)
            .chain(self.modules().map(|m| &m.id))
            .chain(self.mods.iter().map(|m| &m.id))
            .chain(self.macros.iter().map(|m| &m.id))
        {
            if !id.is_empty() {
                seen.insert(id.clone());
            }
        }
        let mut fill = |id: &mut String, prefix: &str, n: &mut usize| {
            if id.trim().is_empty() {
                loop {
                    *n += 1;
                    let c = format!("{prefix}{n}");
                    if !seen.contains(&c) {
                        seen.insert(c.clone());
                        *id = c;
                        break;
                    }
                }
            }
            id.truncate(40);
        };
        let (mut nl, mut nm, mut no, mut na) = (0, 0, 0, 0);
        for l in &mut self.lanes {
            fill(&mut l.id, "lane", &mut nl);
            for m in &mut l.modules {
                fill(&mut m.id, "m", &mut nm);
            }
        }
        for m in &mut self.post {
            fill(&mut m.id, "m", &mut nm);
        }
        for m in &mut self.mods {
            fill(&mut m.id, "mod", &mut no);
        }
        for m in &mut self.macros {
            fill(&mut m.id, "macro", &mut na);
        }
        let mut uniq = std::collections::HashSet::new();
        for id in self
            .lanes
            .iter()
            .map(|l| l.id.clone())
            .chain(self.modules().map(|m| m.id.clone()))
            .chain(self.mods.iter().map(|m| m.id.clone()))
            .chain(self.macros.iter().map(|m| m.id.clone()))
        {
            if !uniq.insert(id.clone()) {
                bail!(bad(format!("the id `{id}` is used twice")));
            }
        }
        for l in &mut self.lanes {
            if l.modules.len() > MAX_MODULES {
                bail!(bad(format!(
                    "a lane can have at most {MAX_MODULES} modules"
                )));
            }
            l.name = l.name.chars().take(40).collect();
            l.gain_db = clampf(l.gain_db, -60.0, 12.0, 0.0);
            l.pan = clampf(l.pan, -1.0, 1.0, 0.0);
        }
        let check_module = |m: &mut Module| -> Result<()> {
            if !live_kind(&m.kind) {
                bail!(bad(format!(
                    "`{}` can't be used live. Call live.kinds for the list.",
                    m.kind
                )));
            }
            let spec = fx::spec(&m.kind).unwrap();
            for (k, v) in m.params.iter_mut() {
                let Some(ps) = spec.params.iter().find(|p| p.key == k) else {
                    bail!(bad(format!(
                        "{} has no parameter `{k}`. It has: {}",
                        m.kind,
                        spec.params
                            .iter()
                            .map(|p| p.key)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )));
                };
                *v = clampf(*v, ps.min, ps.max, ps.def);
            }
            m.mix = clampf(m.mix, 0.0, 1.0, 1.0);
            m.path.truncate(256);
            for p in &mut m.path {
                p[0] = clampf(p[0], -1.0, 1.0, 0.0);
                p[1] = clampf(p[1], -1.0, 1.0, 0.0);
            }
            Ok(())
        };
        for l in &mut self.lanes {
            for m in &mut l.modules {
                check_module(m)?;
            }
        }
        for m in &mut self.post {
            check_module(m)?;
        }
        let range = |p: &LivePreset, t: &Target| -> Result<(f32, f32)> {
            let Some(m) = p.module(&t.module) else {
                bail!(bad(format!("no module `{}` in this preset", t.module)));
            };
            if t.param == "mix" {
                return Ok((0.0, 1.0));
            }
            let spec = fx::spec(&m.kind).unwrap();
            spec.params
                .iter()
                .find(|x| x.key == t.param)
                .map(|x| (x.min, x.max))
                .ok_or_else(|| bad(format!("{} has no parameter `{}`", m.kind, t.param)))
        };
        let snapshot = self.clone();
        for m in &mut self.mods {
            if !SHAPES.iter().any(|s| s.0 == m.shape) {
                bail!(bad(format!(
                    "unknown modulator shape `{}`. Shapes: {}",
                    m.shape,
                    SHAPES.iter().map(|s| s.0).collect::<Vec<_>>().join(", ")
                )));
            }
            if !SYNCS.contains(&m.sync.as_str()) {
                bail!(bad(format!(
                    "unknown sync `{}`. Use one of: {}",
                    m.sync,
                    SYNCS[1..].join(", ")
                )));
            }
            m.name = m.name.chars().take(40).collect();
            m.rate_hz = clampf(m.rate_hz, 0.01, 40.0, 0.5);
            m.depth = clampf(m.depth, 0.0, 1.0, 0.5);
            m.offset = clampf(m.offset, -1.0, 1.0, 0.0);
            m.phase = clampf(m.phase, 0.0, 1.0, 0.0);
            m.smooth = clampf(m.smooth, 0.0, 1.0, 0.0);
            m.attack_ms = clampf(m.attack_ms, 0.1, 2000.0, 10.0);
            m.release_ms = clampf(m.release_ms, 1.0, 5000.0, 200.0);
            m.steps.truncate(MAX_STEPS);
            for s in &mut m.steps {
                *s = clampf(*s, 0.0, 1.0, 0.0);
            }
            if m.targets.len() > MAX_TARGETS {
                bail!(bad(format!(
                    "a modulator can have at most {MAX_TARGETS} targets"
                )));
            }
            for t in &mut m.targets {
                range(&snapshot, t)?;
                t.amount = clampf(t.amount, -1.0, 1.0, 0.0);
            }
        }
        for m in &mut self.macros {
            m.name = m.name.chars().take(40).collect();
            m.value = clampf(m.value, 0.0, 1.0, 0.0);
            if m.targets.len() > MAX_TARGETS {
                bail!(bad(format!(
                    "a macro can have at most {MAX_TARGETS} targets"
                )));
            }
            for t in &mut m.targets {
                let (lo, hi) = range(&snapshot, t)?;
                t.from = clampf(t.from, lo, hi, lo);
                t.to = clampf(t.to, lo, hi, hi);
                t.curve = clampf(t.curve, -1.0, 1.0, 0.0);
            }
        }
        Ok(())
    }
}

/// Macro response: x in 0..1, curve in -1..1.
pub fn macro_curve(x: f32, curve: f32) -> f32 {
    x.clamp(0.0, 1.0).powf(2f32.powf(curve * 2.0))
}

/// A module's parameters with macros applied: (resolved parameters, mix).
fn base_of(p: &LivePreset, m: &Module) -> (P, f32) {
    let spec = fx::spec(&m.kind).expect("validated kind");
    let mut map = m.params.clone();
    let mut mix = m.mix;
    for mac in &p.macros {
        for t in mac.targets.iter().filter(|t| t.module == m.id) {
            let v = t.from + (t.to - t.from) * macro_curve(mac.value, t.curve);
            if t.param == "mix" {
                mix = v;
            } else {
                map.insert(t.param.clone(), v);
            }
        }
    }
    (fx::resolve(spec, &map), mix.clamp(0.0, 1.0))
}

fn sync_beats(s: &str) -> Option<f32> {
    let (core, mul) = if let Some(c) = s.strip_suffix('t') {
        (c, 2.0 / 3.0)
    } else if let Some(c) = s.strip_suffix('d') {
        (c, 1.5)
    } else {
        (s, 1.0)
    };
    let (a, b) = core.split_once('/')?;
    let (a, b): (f32, f32) = (a.parse().ok()?, b.parse().ok()?);
    // "1/4" is a quarter note: one beat
    Some(a / b * 4.0 * mul)
}

/// Modulator rate in Hz.
pub fn mod_rate(m: &Modulator, tempo: f32) -> f32 {
    match sync_beats(&m.sync) {
        Some(beats) if beats > 0.0 => tempo / 60.0 / beats,
        _ => m.rate_hz,
    }
}

// ---------------------------------------------------------------- real-time graph

const POST: u8 = 255;
const MIX: u8 = 255;

#[derive(Clone, Copy, Default, Debug)]
struct Tgt {
    loc: u8,
    slot: u8,
    param: u8,
    amount: f32,
    min: f32,
    max: f32,
    log: bool,
}

/// Everything a modulator needs on the audio thread. Fixed size, so it can be sent without
/// allocating.
#[derive(Clone, Copy, Debug)]
pub struct ModRt {
    shape: u8,
    rate: f32,
    depth: f32,
    offset: f32,
    phase0: f32,
    bipolar: bool,
    smooth: f32,
    steps: [f32; MAX_STEPS],
    nsteps: u8,
    att: f32,
    rel: f32,
    seed: u32,
    targets: [Tgt; MAX_TARGETS],
    ntargets: u8,
}

struct ModState {
    rt: ModRt,
    phase: f32,
    cycle: i64,
    held: f32,
    next: f32,
    out: f32,
    env: f32,
    rng: Rng,
}

impl ModState {
    fn new(rt: ModRt) -> Self {
        let mut rng = Rng::new(rt.seed as u64 * 7919 + 13);
        let (held, next) = (rng.bi(), rng.bi());
        ModState {
            rt,
            phase: rt.phase0,
            cycle: 0,
            held,
            next,
            out: 0.0,
            env: 0.0,
            rng,
        }
    }
    /// Advance by `n` samples; `level` is the input's RMS for the follower. Returns -1..1 or 0..1.
    fn tick(&mut self, n: usize, sr: f32, level: f32) -> f32 {
        let rt = &self.rt;
        let dt = n as f32 / sr;
        self.phase += rt.rate * dt;
        while self.phase >= 1.0 {
            self.phase -= 1.0;
            self.cycle += 1;
            self.held = self.next;
            self.next = self.rng.bi();
        }
        let ph = self.phase;
        let raw = match rt.shape {
            0 => (ph * std::f32::consts::TAU).sin(),
            1 => 1.0 - 4.0 * (ph - 0.5).abs(),
            2 => 2.0 * ph - 1.0,
            3 => 1.0 - 2.0 * ph,
            4 => {
                if ph < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            5 => self.held,
            6 => {
                let t = ph * ph * (3.0 - 2.0 * ph);
                self.held + (self.next - self.held) * t
            }
            7 => {
                let n = rt.nsteps.max(1) as i64;
                let i = (self.cycle.rem_euclid(n)) as usize;
                if rt.nsteps == 0 {
                    0.0
                } else {
                    rt.steps[i] * 2.0 - 1.0
                }
            }
            8 => {
                let k = if level > self.env {
                    1.0 - (-dt / (rt.att * 0.001)).exp()
                } else {
                    1.0 - (-dt / (rt.rel * 0.001)).exp()
                };
                self.env += (level - self.env) * k;
                // map -60..0 dBFS to -1..1
                let db = lin_to_db(self.env.max(1e-6));
                ((db + 60.0) / 30.0 - 1.0).clamp(-1.0, 1.0)
            }
            _ => self.rng.bi(),
        };
        let v = if rt.bipolar { raw } else { (raw + 1.0) * 0.5 };
        let target = (rt.offset + rt.depth * v).clamp(-2.0, 2.0);
        if rt.smooth > 0.0 {
            let tau = rt.smooth * rt.smooth * 0.5 + 1e-4;
            let k = 1.0 - (-dt / tau).exp();
            self.out += (target - self.out) * k;
        } else {
            self.out = target;
        }
        self.out
    }
}

struct GLane {
    chain: Chain,
    gain: f32,
    gain_s: f32,
    pan: f32,
    mute: bool,
}

/// Lane levels and the preset-level controls, sent as one message.
#[derive(Clone, Copy, Debug)]
pub struct Levels {
    in_gain: f32,
    out_gain: f32,
    width: f32,
    balance: f32,
    limiter: bool,
    tempo: f32,
    lanes: [(f32, f32, bool); MAX_LANES],
}

pub enum Msg {
    Graph(Box<Graph>),
    Base {
        loc: u8,
        slot: u8,
        p: P,
        mix: f32,
        enabled: bool,
    },
    Mod {
        idx: u8,
        rt: ModRt,
    },
    Levels(Levels),
    Bypass(bool),
}

pub struct Graph {
    sr: f32,
    pos: i64,
    tempo: f32,
    lanes: Vec<GLane>,
    post: Chain,
    /// Base parameters per location (lanes, then post) and slot.
    base: Vec<Vec<(P, f32)>>,
    mods: Vec<ModState>,
    /// Slots touched by any modulator: (location, slot).
    modded: Vec<(u8, u8)>,
    vals: [f32; MAX_MODS],
    in_gain: (f32, f32),
    out_gain: (f32, f32),
    width: (f32, f32),
    balance: (f32, f32),
    limiter_on: bool,
    limiter: Slot,
    bypass: bool,
    bypass_x: f32,
    dl: Vec<f32>,
    dr: Vec<f32>,
    al: Vec<f32>,
    ar: Vec<f32>,
    tl: Vec<f32>,
    tr: Vec<f32>,
    /// Structure fingerprint: a preset with the same one can be updated in place.
    pub sig: String,
}

/// What the structure of a preset is, ignoring values. Same signature, same graph shape.
pub fn signature(p: &LivePreset) -> String {
    let mut s = String::new();
    for l in &p.lanes {
        s.push('[');
        for m in &l.modules {
            s.push_str(&m.kind);
            s.push(':');
            s.push_str(&m.id);
            s.push_str(&format!("{}", m.path.len()));
            s.push(',');
        }
        s.push(']');
    }
    s.push('|');
    for m in &p.post {
        s.push_str(&m.kind);
        s.push(':');
        s.push_str(&m.id);
        s.push_str(&format!("{}", m.path.len()));
        s.push(',');
    }
    s.push_str(&format!("|{}", p.mods.len()));
    s
}

fn locate(p: &LivePreset, id: &str) -> Option<(u8, u8)> {
    for (li, l) in p.lanes.iter().enumerate() {
        if let Some(si) = l.modules.iter().position(|m| m.id == id) {
            return Some((li as u8, si as u8));
        }
    }
    p.post
        .iter()
        .position(|m| m.id == id)
        .map(|si| (POST, si as u8))
}

fn mod_rt(p: &LivePreset, m: &Modulator) -> ModRt {
    let mut steps = [0.0; MAX_STEPS];
    for (i, s) in m.steps.iter().take(MAX_STEPS).enumerate() {
        steps[i] = *s;
    }
    let mut targets = [Tgt::default(); MAX_TARGETS];
    let mut n = 0;
    for t in m.targets.iter().take(MAX_TARGETS) {
        let (Some((loc, slot)), Some(module)) = (locate(p, &t.module), p.module(&t.module)) else {
            continue;
        };
        let spec = fx::spec(&module.kind).unwrap();
        let (param, min, max, log) = if t.param == "mix" {
            (MIX, 0.0, 1.0, false)
        } else {
            match spec.params.iter().position(|x| x.key == t.param) {
                Some(i) => {
                    let ps = spec.params[i];
                    (i as u8, ps.min, ps.max, ps.log && ps.min > 0.0)
                }
                None => continue,
            }
        };
        targets[n] = Tgt {
            loc,
            slot,
            param,
            amount: t.amount,
            min,
            max,
            log,
        };
        n += 1;
    }
    ModRt {
        shape: SHAPES.iter().position(|s| s.0 == m.shape).unwrap_or(0) as u8,
        rate: mod_rate(m, p.tempo),
        depth: m.depth,
        offset: m.offset,
        phase0: m.phase,
        bipolar: m.bipolar,
        smooth: m.smooth,
        steps,
        nsteps: m.steps.len().min(MAX_STEPS) as u8,
        att: m.attack_ms,
        rel: m.release_ms,
        seed: m.seed,
        targets,
        ntargets: n as u8,
    }
}

fn levels(p: &LivePreset) -> Levels {
    let mut lanes = [(1.0, 0.0, false); MAX_LANES];
    for (i, l) in p.lanes.iter().take(MAX_LANES).enumerate() {
        lanes[i] = (db_to_lin(l.gain_db), l.pan, l.mute);
    }
    Levels {
        in_gain: db_to_lin(p.input_gain_db),
        out_gain: db_to_lin(p.output_gain_db),
        width: p.width,
        balance: p.balance,
        limiter: p.safety_limiter,
        tempo: p.tempo,
        lanes,
    }
}

fn build_slot(m: &Module, base: &(P, f32), sr: f32, id: u32) -> Slot {
    let extra = Extra {
        path: m.path.clone(),
        ..Default::default()
    };
    let kind = fx::spec(&m.kind).unwrap().kind;
    let proc = fx::build(kind, sr, &extra).expect("validated kind");
    let mut s = Slot::new(id, kind, m.enabled, base.1, proc);
    s.set_params(&base.0);
    s
}

impl Graph {
    /// Build the graph for a validated preset. Allocates: call it off the audio thread.
    pub fn build(p: &LivePreset, sr: f32) -> Graph {
        let mut id = 0u32;
        let mut base = vec![];
        let mut lanes = vec![];
        for l in &p.lanes {
            let b: Vec<(P, f32)> = l.modules.iter().map(|m| base_of(p, m)).collect();
            let slots = l
                .modules
                .iter()
                .zip(&b)
                .map(|(m, b)| {
                    id += 1;
                    build_slot(m, b, sr, id)
                })
                .collect();
            lanes.push(GLane {
                chain: Chain { slots },
                gain: db_to_lin(l.gain_db),
                gain_s: db_to_lin(l.gain_db),
                pan: l.pan,
                mute: l.mute,
            });
            base.push(b);
        }
        let pb: Vec<(P, f32)> = p.post.iter().map(|m| base_of(p, m)).collect();
        let post = Chain {
            slots: p
                .post
                .iter()
                .zip(&pb)
                .map(|(m, b)| {
                    id += 1;
                    build_slot(m, b, sr, id)
                })
                .collect(),
        };
        base.push(pb);
        let mods: Vec<ModState> = p.mods.iter().map(|m| ModState::new(mod_rt(p, m))).collect();
        let mut modded = vec![];
        for m in &mods {
            for t in &m.rt.targets[..m.rt.ntargets as usize] {
                if !modded.contains(&(t.loc, t.slot)) {
                    modded.push((t.loc, t.slot));
                }
            }
        }
        let lim_spec = fx::spec("limiter").unwrap();
        let mut lim = Slot::new(
            u32::MAX,
            "limiter",
            true,
            1.0,
            fx::build("limiter", sr, &Extra::default()).unwrap(),
        );
        let mut lp = BTreeMap::new();
        lp.insert("ceiling".to_string(), -1.0);
        lp.insert("lookahead".to_string(), 1.5);
        lp.insert("release".to_string(), 80.0);
        lim.set_params(&fx::resolve(lim_spec, &lp));
        let lv = levels(p);
        Graph {
            sr,
            pos: 0,
            tempo: p.tempo,
            lanes,
            post,
            base,
            mods,
            modded,
            vals: [0.0; MAX_MODS],
            in_gain: (lv.in_gain, lv.in_gain),
            out_gain: (lv.out_gain, lv.out_gain),
            width: (lv.width, lv.width),
            balance: (lv.balance, lv.balance),
            limiter_on: lv.limiter,
            limiter: lim,
            bypass: false,
            bypass_x: 0.0,
            dl: vec![0.0; BLOCK],
            dr: vec![0.0; BLOCK],
            al: vec![0.0; BLOCK],
            ar: vec![0.0; BLOCK],
            tl: vec![0.0; BLOCK],
            tr: vec![0.0; BLOCK],
            sig: signature(p),
        }
    }

    /// Messages that bring this graph's values in line with `p` (same signature). The caller
    /// sends them to the audio thread.
    pub fn updates(p: &LivePreset) -> Vec<Msg> {
        let mut out = vec![];
        for (li, l) in p.lanes.iter().enumerate() {
            for (si, m) in l.modules.iter().enumerate() {
                let (pp, mix) = base_of(p, m);
                out.push(Msg::Base {
                    loc: li as u8,
                    slot: si as u8,
                    p: pp,
                    mix,
                    enabled: m.enabled,
                });
            }
        }
        for (si, m) in p.post.iter().enumerate() {
            let (pp, mix) = base_of(p, m);
            out.push(Msg::Base {
                loc: POST,
                slot: si as u8,
                p: pp,
                mix,
                enabled: m.enabled,
            });
        }
        for (i, m) in p.mods.iter().enumerate() {
            out.push(Msg::Mod {
                idx: i as u8,
                rt: mod_rt(p, m),
            });
        }
        out.push(Msg::Levels(levels(p)));
        out
    }

    fn loc_index(&self, loc: u8) -> usize {
        if loc == POST {
            self.lanes.len()
        } else {
            loc as usize
        }
    }
    fn slot_mut(&mut self, loc: u8, slot: u8) -> Option<&mut Slot> {
        if loc == POST {
            self.post.slots.get_mut(slot as usize)
        } else {
            self.lanes
                .get_mut(loc as usize)
                .and_then(|l| l.chain.slots.get_mut(slot as usize))
        }
    }

    /// Apply one control message (audio thread). Returns a graph to hand back for dropping.
    pub fn apply(&mut self, m: Msg) -> Option<Box<Graph>> {
        match m {
            Msg::Graph(g) => return Some(g),
            Msg::Base {
                loc,
                slot,
                p,
                mix,
                enabled,
            } => {
                let li = self.loc_index(loc);
                if let Some(b) = self.base.get_mut(li).and_then(|v| v.get_mut(slot as usize)) {
                    *b = (p, mix);
                }
                let modded = self.modded.contains(&(loc, slot));
                if let Some(s) = self.slot_mut(loc, slot) {
                    s.enabled = enabled;
                    if !modded {
                        s.set_params(&p);
                        s.mix = mix;
                    }
                }
            }
            Msg::Mod { idx, rt } => {
                if let Some(m) = self.mods.get_mut(idx as usize) {
                    let keep_phase = m.phase;
                    m.rt = rt;
                    m.phase = keep_phase;
                }
            }
            Msg::Levels(l) => {
                self.in_gain.1 = l.in_gain;
                self.out_gain.1 = l.out_gain;
                self.width.1 = l.width;
                self.balance.1 = l.balance;
                self.limiter_on = l.limiter;
                self.tempo = l.tempo;
                for (i, lane) in self.lanes.iter_mut().enumerate().take(MAX_LANES) {
                    lane.gain = l.lanes[i].0;
                    lane.pan = l.lanes[i].1;
                    lane.mute = l.lanes[i].2;
                }
            }
            Msg::Bypass(b) => self.bypass = b,
        }
        None
    }

    fn modulate(&mut self, n: usize, level: f32) {
        if self.mods.is_empty() {
            return;
        }
        let sr = self.sr;
        for (i, m) in self.mods.iter_mut().enumerate() {
            self.vals[i] = m.tick(n, sr, level);
        }
        for k in 0..self.modded.len() {
            let (loc, slot) = self.modded[k];
            let li = self.loc_index(loc);
            let Some(&(mut p, mut mix)) = self.base.get(li).and_then(|v| v.get(slot as usize))
            else {
                continue;
            };
            for (mi, m) in self.mods.iter().enumerate() {
                let v = self.vals[mi];
                for t in &m.rt.targets[..m.rt.ntargets as usize] {
                    if t.loc != loc || t.slot != slot {
                        continue;
                    }
                    let d = v * t.amount;
                    if t.param == MIX {
                        mix = (mix + d).clamp(0.0, 1.0);
                        continue;
                    }
                    let i = t.param as usize;
                    if i >= MAXP {
                        continue;
                    }
                    p[i] = if t.log {
                        let span = (t.max / t.min).ln();
                        let n = ((p[i].max(t.min) / t.min).ln() / span + d).clamp(0.0, 1.0);
                        t.min * (n * span).exp()
                    } else {
                        (p[i] + d * (t.max - t.min)).clamp(t.min, t.max)
                    };
                }
            }
            if let Some(s) = self.slot_mut(loc, slot) {
                s.set_params(&p);
                s.mix = mix;
            }
        }
    }

    /// Process up to `BLOCK` stereo samples in place.
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len()).min(BLOCK);
        let mut i = 0;
        while i < n {
            let e = (i + CONTROL).min(n);
            self.process_part(&mut l[i..e], &mut r[i..e]);
            i = e;
        }
    }

    fn process_part(&mut self, l: &mut [f32], r: &mut [f32]) {
        let n = l.len();
        // input gain (smoothed) and the level the follower sees
        let mut sum = 0.0;
        for k in 0..n {
            self.in_gain.0 += (self.in_gain.1 - self.in_gain.0) * 0.002;
            l[k] *= self.in_gain.0;
            r[k] *= self.in_gain.0;
            sum += l[k] * l[k] + r[k] * r[k];
        }
        let level = (sum / (2 * n).max(1) as f32).sqrt();
        self.modulate(n, level);
        let ctx = Ctx {
            sr: self.sr,
            pos: self.pos,
            len: i64::MAX / 4,
            tempo: self.tempo,
        };
        self.pos += n as i64;
        self.dl[..n].copy_from_slice(l);
        self.dr[..n].copy_from_slice(r);
        if !self.lanes.is_empty() {
            self.al[..n].fill(0.0);
            self.ar[..n].fill(0.0);
            for lane in &mut self.lanes {
                self.tl[..n].copy_from_slice(&self.dl[..n]);
                self.tr[..n].copy_from_slice(&self.dr[..n]);
                lane.chain
                    .process(&mut self.tl[..n], &mut self.tr[..n], &ctx);
                // keep processing muted lanes so their tails stay in time, but don't add them
                let want = if lane.mute { 0.0 } else { lane.gain };
                let (pl, pr) = pan_gains(lane.pan);
                for k in 0..n {
                    lane.gain_s += (want - lane.gain_s) * 0.002;
                    self.al[k] += self.tl[k] * lane.gain_s * pl;
                    self.ar[k] += self.tr[k] * lane.gain_s * pr;
                }
            }
            l.copy_from_slice(&self.al[..n]);
            r.copy_from_slice(&self.ar[..n]);
        }
        self.post.process(l, r, &ctx);
        for k in 0..n {
            self.width.0 += (self.width.1 - self.width.0) * 0.002;
            self.balance.0 += (self.balance.1 - self.balance.0) * 0.002;
            self.out_gain.0 += (self.out_gain.1 - self.out_gain.0) * 0.002;
            let (m, s) = ((l[k] + r[k]) * 0.5, (l[k] - r[k]) * 0.5 * self.width.0);
            let (bl, br) = pan_gains(self.balance.0);
            l[k] = (m + s) * bl * self.out_gain.0;
            r[k] = (m - s) * br * self.out_gain.0;
        }
        if self.limiter_on {
            self.limiter.process(l, r, &ctx);
        }
        // bypass crossfades to the untouched input over ~10 ms
        let want = if self.bypass { 1.0 } else { 0.0 };
        let step = 1.0 / (self.sr * 0.01);
        for k in 0..n {
            self.bypass_x += (want - self.bypass_x).clamp(-step, step);
            let x = self.bypass_x;
            let (a, b) = (l[k], r[k]);
            l[k] = (a + (self.dl[k] - a) * x).clamp(-1.0, 1.0);
            r[k] = (b + (self.dr[k] - b) * x).clamp(-1.0, 1.0);
            if !l[k].is_finite() {
                l[k] = 0.0;
            }
            if !r[k].is_finite() {
                r[k] = 0.0;
            }
        }
    }
}

/// Equal-power-ish pan that leaves the center at unity: -1 is left, 1 is right.
fn pan_gains(p: f32) -> (f32, f32) {
    let p = p.clamp(-1.0, 1.0);
    if p <= 0.0 {
        (1.0, 1.0 + p)
    } else {
        (1.0 - p, 1.0)
    }
}

// ---------------------------------------------------------------- test sources

/// Built-in sources, for trying presets without a device and for tests.
pub struct Demo {
    voice: bool,
    sr: f32,
    t: u64,
    rng: Rng,
    ph: [f32; 4],
    f1: [crate::dsp::Biquad; 2],
    f2: [crate::dsp::Biquad; 2],
    lp: crate::dsp::OnePole,
}

impl Demo {
    pub fn new(voice: bool, sr: f32) -> Demo {
        use crate::dsp::{Biquad, FilterType};
        Demo {
            voice,
            sr,
            t: 0,
            rng: Rng::new(7),
            ph: [0.0; 4],
            f1: [Biquad::new(FilterType::Bell, sr, 700.0, 6.0, 18.0); 2],
            f2: [Biquad::new(FilterType::Bell, sr, 1200.0, 6.0, 15.0); 2],
            lp: crate::dsp::OnePole::new(sr, 3000.0),
        }
    }
    /// Fill with the next stereo samples.
    pub fn fill(&mut self, l: &mut [f32], r: &mut [f32]) {
        use std::f32::consts::TAU;
        let sr = self.sr;
        for k in 0..l.len() {
            let t = self.t as f32 / sr;
            self.t += 1;
            let (a, b) = if self.voice {
                // a buzzy voice-like source: a glottal saw through moving vowel formants,
                // in phrases with pauses, so gates and followers have something to do
                let phrase = t % 3.0;
                let on = if phrase < 2.2 { 1.0 } else { 0.0 };
                let f0 = 110.0 + 25.0 * (t * 1.3).sin() + 10.0 * (t * 0.4).sin();
                self.ph[0] = (self.ph[0] + f0 / sr).fract();
                let src = (2.0 * self.ph[0] - 1.0) * on;
                let vowel = (t * 1.7).sin() * 0.5 + 0.5;
                if self.t.is_multiple_of(256) {
                    for f in &mut self.f1 {
                        f.set(
                            crate::dsp::FilterType::Bell,
                            sr,
                            400.0 + 500.0 * vowel,
                            5.0,
                            18.0,
                        );
                    }
                    for f in &mut self.f2 {
                        f.set(
                            crate::dsp::FilterType::Bell,
                            sr,
                            900.0 + 1400.0 * (1.0 - vowel),
                            5.0,
                            15.0,
                        );
                    }
                }
                let x = self.f2[0].run(self.f1[0].run(src)) * 0.04 + self.rng.bi() * 0.002 * on;
                (x, x)
            } else {
                // a short groove at 120 BPM: kick, hats, a minor chord pad and a plucked line
                let beat = t * 2.0;
                let bt = beat.fract() / 2.0;
                let kick = (TAU * (45.0 + 120.0 * (-bt * 40.0).exp()) * bt).sin()
                    * (-bt * 9.0).exp()
                    * 0.6;
                let st = (beat * 2.0).fract() / 4.0;
                let hat = self.rng.bi() * (-st * 160.0).exp() * 0.12;
                let bar = (beat / 4.0) as i64 % 4;
                let roots = [220.0, 174.61, 261.63, 196.0];
                let root = roots[bar as usize];
                let mut pad = 0.0;
                for (i, m) in [1.0, 1.189, 1.498].iter().enumerate() {
                    self.ph[i] = (self.ph[i] + root * m / sr).fract();
                    pad += 2.0 * self.ph[i] - 1.0;
                }
                let pad = self.lp.lp(pad * 0.05);
                let note = (beat * 2.0) as i64 % 8;
                let scale = [0.0, 3.0, 7.0, 10.0, 12.0, 10.0, 7.0, 3.0];
                let f = root * 2.0 * 2f32.powf(scale[note as usize] / 12.0);
                self.ph[3] = (self.ph[3] + f / sr).fract();
                let pl = (self.ph[3] * TAU).sin() * (-st * 4.0 * 4.0).exp() * 0.18;
                let pan = (t * 0.25 * TAU).sin() * 0.4;
                (
                    kick + hat * (1.0 - pan) + pad + pl * (1.0 - pan),
                    kick + hat * (1.0 + pan) + pad * 0.9 + pl * (1.0 + pan),
                )
            };
            l[k] = a;
            r[k] = b;
        }
    }
}

/// Render `seconds` of a source through a preset, offline. Returns (input, output) stereo.
pub fn render(
    p: &LivePreset,
    sr: f32,
    input: &mut dyn FnMut(&mut [f32], &mut [f32]),
    seconds: f32,
) -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>) {
    let total = (seconds.clamp(0.1, 600.0) * sr) as usize;
    let mut g = Graph::build(p, sr);
    let (mut il, mut ir, mut ol, mut or) = (
        Vec::with_capacity(total),
        Vec::with_capacity(total),
        Vec::with_capacity(total),
        Vec::with_capacity(total),
    );
    let (mut l, mut r) = (vec![0.0; BLOCK], vec![0.0; BLOCK]);
    let mut done = 0;
    while done < total {
        let n = (total - done).min(BLOCK);
        input(&mut l[..n], &mut r[..n]);
        il.extend_from_slice(&l[..n]);
        ir.extend_from_slice(&r[..n]);
        g.process(&mut l[..n], &mut r[..n]);
        ol.extend_from_slice(&l[..n]);
        or.extend_from_slice(&r[..n]);
        done += n;
    }
    (il, ir, ol, or)
}

// ---------------------------------------------------------------- devices and routes

fn f2u(v: f32) -> u32 {
    v.to_bits()
}
fn u2f(v: u32) -> f32 {
    f32::from_bits(v)
}

#[derive(Default)]
pub struct LiveMeters {
    pub in_peak: AtomicU32,
    pub in_rms: AtomicU32,
    pub out_peak: AtomicU32,
    pub out_rms: AtomicU32,
    pub frames: AtomicU64,
    pub underruns: AtomicU64,
    pub overruns: AtomicU64,
    pub fill_ms: AtomicU32,
    pub alive: AtomicBool,
    pub device_errors: AtomicU64,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct RouteInfo {
    pub route: String,
    pub source: String,
    pub source_device: String,
    pub output: String,
    pub output_device: String,
    pub sample_rate: u32,
    pub preset: String,
}

/// Linear resampler that also absorbs the clock drift between two devices by steering its
/// ratio toward a target fill of the input ring.
struct Drift {
    nominal: f64,
    frac: f64,
    prev: (f32, f32),
    cur: (f32, f32),
    target: usize,
    /// Dropouts only count once audio has started arriving.
    primed: bool,
}

/// The output side of a route: pulls input, processes and writes to the device.
pub struct OutCb {
    graph: Box<Graph>,
    rx: rtrb::Consumer<Msg>,
    gtx: rtrb::Producer<Box<Graph>>,
    ring: Option<rtrb::Consumer<f32>>,
    demo: Option<Demo>,
    drift: Drift,
    l: Vec<f32>,
    r: Vec<f32>,
    at: usize,
    meters: Arc<LiveMeters>,
    pk: (f32, f32),
    ms: (f32, f32),
    count: usize,
}

impl OutCb {
    fn new(
        graph: Box<Graph>,
        rx: rtrb::Consumer<Msg>,
        gtx: rtrb::Producer<Box<Graph>>,
        ring: Option<rtrb::Consumer<f32>>,
        demo: Option<Demo>,
        (in_sr, out_sr): (u32, u32),
        meters: Arc<LiveMeters>,
    ) -> OutCb {
        OutCb {
            graph,
            rx,
            gtx,
            ring,
            demo,
            drift: Drift {
                nominal: in_sr as f64 / out_sr as f64,
                frac: 0.0,
                prev: (0.0, 0.0),
                cur: (0.0, 0.0),
                // ~15 ms of cushion
                target: (in_sr as usize * 15 / 1000).max(64),
                primed: false,
            },
            l: vec![0.0; BLOCK],
            r: vec![0.0; BLOCK],
            at: BLOCK,
            meters,
            pk: (0.0, 0.0),
            ms: (0.0, 0.0),
            count: 0,
        }
    }

    fn pull(&mut self, n: usize) {
        let (l, r) = (&mut self.l[..n], &mut self.r[..n]);
        if let Some(d) = self.demo.as_mut() {
            d.fill(l, r);
            return;
        }
        let Some(ring) = self.ring.as_mut() else {
            l.fill(0.0);
            r.fill(0.0);
            return;
        };
        let avail = ring.slots() / 2;
        let dr = &mut self.drift;
        // far too much buffered (the output stalled): skip ahead to the target
        if avail > dr.target * 8 {
            let skip = avail - dr.target;
            if let Ok(c) = ring.read_chunk(skip * 2) {
                c.commit_all();
            }
            self.meters.overruns.fetch_add(1, Relaxed);
        }
        let avail = ring.slots() / 2;
        if !dr.primed {
            if avail < dr.target {
                l.fill(0.0);
                r.fill(0.0);
                return;
            }
            dr.primed = true;
        }
        let err = (avail as f64 - dr.target as f64) / dr.target as f64;
        let ratio = dr.nominal * (1.0 + (err * 0.002).clamp(-0.005, 0.005));
        let mut under = false;
        for k in 0..n {
            while dr.frac >= 1.0 {
                dr.prev = dr.cur;
                match (ring.pop(), ring.pop()) {
                    (Ok(a), Ok(b)) => dr.cur = (a, b),
                    _ => {
                        under = true;
                        dr.cur = (dr.cur.0 * 0.95, dr.cur.1 * 0.95);
                    }
                }
                dr.frac -= 1.0;
            }
            let f = dr.frac as f32;
            l[k] = dr.prev.0 + (dr.cur.0 - dr.prev.0) * f;
            r[k] = dr.prev.1 + (dr.cur.1 - dr.prev.1) * f;
            dr.frac += ratio;
        }
        if under {
            self.meters.underruns.fetch_add(1, Relaxed);
        }
        let sr = self.graph.sr.max(1.0);
        self.meters
            .fill_ms
            .store(f2u(avail as f32 / sr * 1000.0 / dr.nominal as f32), Relaxed);
    }

    fn produce(&mut self) {
        while let Ok(m) = self.rx.pop() {
            if let Some(mut g) = self.graph.apply(m) {
                // a new graph: keep the old one's bypass state, return the old one for dropping
                g.bypass = self.graph.bypass;
                g.bypass_x = self.graph.bypass_x;
                let old = std::mem::replace(&mut self.graph, g);
                let _ = self.gtx.push(old);
            }
        }
        self.pull(BLOCK);
        let mut ip = 0f32;
        let mut is = 0f32;
        for k in 0..BLOCK {
            let v = self.l[k].abs().max(self.r[k].abs());
            ip = ip.max(v);
            is += self.l[k] * self.l[k] + self.r[k] * self.r[k];
        }
        self.graph.process(&mut self.l, &mut self.r);
        let mut op = 0f32;
        let mut os = 0f32;
        for k in 0..BLOCK {
            op = op.max(self.l[k].abs().max(self.r[k].abs()));
            os += self.l[k] * self.l[k] + self.r[k] * self.r[k];
        }
        self.pk = (self.pk.0.max(ip), self.pk.1.max(op));
        self.ms = (self.ms.0 + is, self.ms.1 + os);
        self.count += BLOCK;
        if self.count >= 2048 {
            let n = (self.count * 2) as f32;
            let m = &self.meters;
            m.in_peak.store(f2u(self.pk.0), Relaxed);
            m.out_peak.store(f2u(self.pk.1), Relaxed);
            m.in_rms.store(f2u((self.ms.0 / n).sqrt()), Relaxed);
            m.out_rms.store(f2u((self.ms.1 / n).sqrt()), Relaxed);
            self.pk = (0.0, 0.0);
            self.ms = (0.0, 0.0);
            self.count = 0;
        }
        self.meters.frames.fetch_add(BLOCK as u64, Relaxed);
        self.at = 0;
    }

    /// Fill an interleaved device buffer.
    pub fn fill<T: Copy>(&mut self, out: &mut [T], ch: usize, conv: impl Fn(f32) -> T) {
        let ch = ch.max(1);
        for f in out.chunks_mut(ch) {
            if self.at >= BLOCK {
                self.produce();
            }
            let (a, b) = (self.l[self.at], self.r[self.at]);
            self.at += 1;
            if ch == 1 {
                f[0] = conv((a + b) * 0.5);
            } else {
                f[0] = conv(a);
                f[1] = conv(b);
                for x in f.iter_mut().skip(2) {
                    *x = conv(0.0);
                }
            }
        }
    }
}

/// A running route: a capture stream (or a demo source) feeding an output stream.
pub struct Runner {
    pub info: RouteInfo,
    pub meters: Arc<LiveMeters>,
    pub started: Instant,
    pub sig: String,
    pub bypass: bool,
    pub error: Option<String>,
    tx: rtrb::Producer<Msg>,
    garbage: rtrb::Consumer<Box<Graph>>,
    stop: Arc<AtomicBool>,
    threads: Vec<std::thread::JoinHandle<()>>,
}

impl Runner {
    fn send(&mut self, m: Msg) {
        let mut m = m;
        for _ in 0..200 {
            match self.tx.push(m) {
                Ok(()) => break,
                Err(rtrb::PushError::Full(back)) => {
                    m = back;
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
        }
        while self.garbage.pop().is_ok() {}
    }

    /// Bring the running graph in line with `p`: in place when the structure is the same,
    /// otherwise by swapping in a new graph.
    pub fn update(&mut self, p: &LivePreset) {
        let sig = signature(p);
        if sig == self.sig {
            for m in Graph::updates(p) {
                self.send(m);
            }
        } else {
            let g = Graph::build(p, self.info.sample_rate as f32);
            self.sig = sig;
            self.send(Msg::Graph(Box::new(g)));
        }
        self.info.preset = p.id.clone();
    }

    pub fn set_bypass(&mut self, on: bool) {
        self.bypass = on;
        self.send(Msg::Bypass(on));
    }

    pub fn stop(mut self) {
        self.stop.store(true, Relaxed);
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
        while self.garbage.pop().is_ok() {}
    }

    pub fn status(&self) -> serde_json::Value {
        let m = &self.meters;
        let db = |v: f32| (lin_to_db(v.max(1e-9)) * 10.0).round() / 10.0;
        let lat_ms = u2f(m.fill_ms.load(Relaxed));
        serde_json::json!({
            "running": m.alive.load(Relaxed),
            "info": self.info,
            "bypass": self.bypass,
            "seconds": self.started.elapsed().as_secs_f32(),
            "frames": m.frames.load(Relaxed),
            "in_peak_db": db(u2f(m.in_peak.load(Relaxed))),
            "in_rms_db": db(u2f(m.in_rms.load(Relaxed))),
            "out_peak_db": db(u2f(m.out_peak.load(Relaxed))),
            "out_rms_db": db(u2f(m.out_rms.load(Relaxed))),
            "buffer_ms": (lat_ms * 10.0).round() / 10.0,
            "underruns": m.underruns.load(Relaxed),
            "overruns": m.overruns.load(Relaxed),
            "device_errors": m.device_errors.load(Relaxed),
            "error": self.error,
        })
    }
}

/// Where a route's sound comes from.
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    /// Capture what an output device is playing (loopback). `None` is the default output.
    System(Option<String>),
    /// A microphone or other input. `None` is the default input.
    Input(Option<String>),
    DemoMusic,
    DemoVoice,
}

pub struct StartOpts {
    pub route: String,
    pub source: Source,
    /// Output device id. `None` is the default output; `Some("none")` plays nowhere (tests).
    pub output: Option<String>,
}

#[cfg(feature = "device")]
fn find_device(id: &Option<String>, output: bool) -> Result<cpal::Device> {
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = crate::vdev::host();
    if let Some(want) = id {
        let list = if output {
            host.output_devices()
        } else {
            host.input_devices()
        };
        if let Ok(mut it) = list {
            if let Some(d) =
                it.find(|d| d.id().map(|i| i.to_string()).ok().as_deref() == Some(want.as_str()))
            {
                return Ok(d);
            }
        }
        // a loopback source may also be given as an input id (Linux monitors)
        if output {
            if let Ok(mut it) = host.input_devices() {
                if let Some(d) = it
                    .find(|d| d.id().map(|i| i.to_string()).ok().as_deref() == Some(want.as_str()))
                {
                    return Ok(d);
                }
            }
        }
        bail!(crate::api::err(
            "not_found",
            "That audio device isn't connected any more. Pick another in Live."
        ));
    }
    let d = if output {
        host.default_output_device()
    } else {
        host.default_input_device()
    };
    d.ok_or_else(|| {
        crate::api::err(
            "no_device",
            if output {
                "No audio output was found."
            } else {
                "No microphone or input was found."
            },
        )
    })
}

#[cfg(feature = "device")]
fn dev_id_name(d: &cpal::Device) -> (String, String) {
    use cpal::traits::DeviceTrait;
    let id = d.id().map(|i| i.to_string()).unwrap_or_default();
    let name = d
        .description()
        .map(|x| x.name().to_string())
        .unwrap_or_else(|_| id.clone());
    (id, name)
}

/// Start a route. `p` must be validated.
pub fn start(o: StartOpts, p: &LivePreset) -> Result<Runner> {
    let meters = Arc::new(LiveMeters::default());
    let stop = Arc::new(AtomicBool::new(false));
    let (tx, rx) = rtrb::RingBuffer::<Msg>::new(512);
    let (gtx, grx) = rtrb::RingBuffer::<Box<Graph>>::new(32);
    let null_out =
        o.output.as_deref() == Some("none") || std::env::var_os("FLUX_NULL_AUDIO").is_some();
    let demo = matches!(o.source, Source::DemoMusic | Source::DemoVoice);
    let mut threads = vec![];
    let mut info = RouteInfo {
        route: o.route.clone(),
        source: match &o.source {
            Source::System(_) => "system",
            Source::Input(_) => "input",
            Source::DemoMusic => "demo-music",
            Source::DemoVoice => "demo-voice",
        }
        .into(),
        preset: p.id.clone(),
        ..Default::default()
    };

    // ---- capture side
    #[allow(unused_mut)]
    let mut ring_rx: Option<rtrb::Consumer<f32>> = None;
    #[allow(unused_mut)]
    let mut in_sr: u32 = 48000;
    if demo {
        info.source_device = if o.source == Source::DemoMusic {
            "Test music".into()
        } else {
            "Test voice".into()
        };
    } else if null_out && std::env::var_os("FLUX_NULL_AUDIO").is_some() {
        bail!(crate::api::err(
            "no_device",
            "Audio devices are off (FLUX_NULL_AUDIO). Use a test source."
        ));
    } else {
        #[cfg(feature = "device")]
        {
            use cpal::traits::{DeviceTrait, StreamTrait};
            let (dev, loopback) = match &o.source {
                Source::System(id) => {
                    let d = find_device(id, true)?;
                    let lb = d.supports_output();
                    (d, lb)
                }
                Source::Input(id) => (find_device(id, false)?, false),
                _ => unreachable!(),
            };
            let (sid, sname) = dev_id_name(&dev);
            // feedback guard: capturing an output and playing back into the same output
            if loopback {
                let out_id = match &o.output {
                    Some(x) if x != "none" => Some(x.clone()),
                    Some(_) => None,
                    None => find_device(&None, true).ok().map(|d| dev_id_name(&d).0),
                };
                if out_id.as_deref() == Some(sid.as_str()) {
                    bail!(crate::api::err(
                        "feedback",
                        format!("Flux would capture {sname} and play back into it, which feeds back forever. Send the app you want to hear (for example your music player) to one output, capture that one here, and listen on a different output, such as your headphones."),
                    ));
                }
            }
            let cfg = if loopback {
                dev.default_output_config()
            } else {
                dev.default_input_config()
            }
            .map_err(|e| crate::api::err("device", format!("{sname} has no usable format: {e}")))?;
            in_sr = cfg.sample_rate();
            let ch = cfg.channels().max(1) as usize;
            let fmt = cfg.sample_format();
            let (mut ptx, prx) = rtrb::RingBuffer::<f32>::new(in_sr as usize * 2);
            ring_rx = Some(prx);
            info.source_device = sname.clone();
            let st = stop.clone();
            let m2 = meters.clone();
            let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();
            let scfg = cpal::StreamConfig {
                channels: ch as u16,
                sample_rate: in_sr,
                buffer_size: cpal::BufferSize::Default,
            };
            threads.push(
                std::thread::Builder::new()
                    .name("flux-live-in".into())
                    .spawn(move || {
                        let m3 = m2.clone();
                        let errf = move |_e: cpal::Error| {
                            m3.device_errors.fetch_add(1, Relaxed);
                        };
                        let mut push = move |frames: &mut dyn Iterator<Item = (f32, f32)>| {
                            for (a, b) in frames {
                                if ptx.slots() < 2 {
                                    m2.overruns.fetch_add(1, Relaxed);
                                    break;
                                }
                                let _ = ptx.push(a);
                                let _ = ptx.push(b);
                            }
                        };
                        let res = match fmt {
                            cpal::SampleFormat::I16 => dev.build_input_stream::<i16, _, _>(
                                scfg,
                                move |d: &[i16], _| {
                                    let mut it = d.chunks(ch).map(|f| {
                                        let a = f[0] as f32 / 32768.0;
                                        (a, f.get(1).map(|v| *v as f32 / 32768.0).unwrap_or(a))
                                    });
                                    push(&mut it)
                                },
                                errf,
                                None,
                            ),
                            _ => dev.build_input_stream::<f32, _, _>(
                                scfg,
                                move |d: &[f32], _| {
                                    let mut it =
                                        d.chunks(ch).map(|f| (f[0], *f.get(1).unwrap_or(&f[0])));
                                    push(&mut it)
                                },
                                errf,
                                None,
                            ),
                        };
                        match res {
                            Ok(s) => {
                                if let Err(e) = s.play() {
                                    let _ = ready_tx.send(Err(e.to_string()));
                                    return;
                                }
                                let _ = ready_tx.send(Ok(()));
                                while !st.load(Relaxed) {
                                    std::thread::sleep(Duration::from_millis(20));
                                }
                                drop(s);
                            }
                            Err(e) => {
                                let _ = ready_tx.send(Err(e.to_string()));
                            }
                        }
                    })?,
            );
            match ready_rx.recv_timeout(Duration::from_secs(8)) {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    stop.store(true, Relaxed);
                    let hint = if loopback && cfg!(target_os = "macos") {
                        " Capturing computer sound needs macOS 14.6 or later, and permission to record system audio."
                    } else {
                        ""
                    };
                    bail!(crate::api::err(
                        "device",
                        format!("Couldn't start capturing from {sname}: {e}.{hint}")
                    ));
                }
                Err(_) => {
                    stop.store(true, Relaxed);
                    bail!(crate::api::err(
                        "device",
                        format!("{sname} didn't respond.")
                    ));
                }
            }
        }
        #[cfg(not(feature = "device"))]
        bail!(crate::api::err(
            "no_device",
            "This build has no audio devices."
        ));
    }

    // ---- output side
    let out_sr_pref = in_sr;
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(String, String, u32), String>>();
    let st = stop.clone();
    let m2 = meters.clone();
    let p2 = p.clone();
    let out_id = o.output.clone();
    let source = o.source.clone();
    threads.push(
        std::thread::Builder::new()
            .name("flux-live-out".into())
            .spawn(move || {
                let mk = |sr: u32, ring: Option<rtrb::Consumer<f32>>, rx, gtx| {
                    let demo = match source {
                        Source::DemoMusic => Some(Demo::new(false, sr as f32)),
                        Source::DemoVoice => Some(Demo::new(true, sr as f32)),
                        _ => None,
                    };
                    let src_sr = if demo.is_some() { sr } else { out_sr_pref };
                    OutCb::new(
                        Box::new(Graph::build(&p2, sr as f32)),
                        rx,
                        gtx,
                        ring,
                        demo,
                        (src_sr, sr),
                        m2.clone(),
                    )
                };
                if null_out {
                    let sr = out_sr_pref;
                    let mut cb = mk(sr, ring_rx, rx, gtx);
                    let _ = ready_tx.send(Ok(("none".into(), "Nowhere (silent)".into(), sr)));
                    m2.alive.store(true, Relaxed);
                    let n = (sr / 100) as usize;
                    let mut buf = vec![0f32; n * 2];
                    let mut next = Instant::now();
                    while !st.load(Relaxed) {
                        cb.fill(&mut buf, 2, |v| v);
                        next += Duration::from_millis(10);
                        let now = Instant::now();
                        if next > now {
                            std::thread::sleep(next - now);
                        } else {
                            next = now;
                        }
                    }
                    m2.alive.store(false, Relaxed);
                    return;
                }
                #[cfg(feature = "device")]
                {
                    use cpal::traits::{DeviceTrait, StreamTrait};
                    let dev = match find_device(&out_id, true) {
                        Ok(d) => d,
                        Err(e) => {
                            let _ = ready_tx.send(Err(format!("{e:#}")));
                            return;
                        }
                    };
                    let (oid, oname) = dev_id_name(&dev);
                    let def = match dev.default_output_config() {
                        Ok(c) => c,
                        Err(e) => {
                            let _ =
                                ready_tx.send(Err(format!("{oname} has no usable format: {e}")));
                            return;
                        }
                    };
                    // run the output at the source rate when it can, so nothing is resampled
                    let mut sr = def.sample_rate();
                    let mut fmt = def.sample_format();
                    if let Ok(confs) = dev.supported_output_configs() {
                        for c in confs {
                            if c.min_sample_rate() <= out_sr_pref
                                && c.max_sample_rate() >= out_sr_pref
                                && c.sample_format() == cpal::SampleFormat::F32
                            {
                                sr = out_sr_pref;
                                fmt = cpal::SampleFormat::F32;
                                break;
                            }
                        }
                    }
                    let ch = def.channels().max(1) as usize;
                    let cfg = cpal::StreamConfig {
                        channels: ch as u16,
                        sample_rate: sr,
                        buffer_size: cpal::BufferSize::Default,
                    };
                    let mut cb = mk(sr, ring_rx, rx, gtx);
                    let m3 = m2.clone();
                    let errf = move |_e: cpal::Error| {
                        m3.device_errors.fetch_add(1, Relaxed);
                    };
                    let stream = match fmt {
                        cpal::SampleFormat::I16 => dev.build_output_stream::<i16, _, _>(
                            cfg,
                            move |d: &mut [i16], _| cb.fill(d, ch, |v| (v * 32767.0) as i16),
                            errf,
                            None,
                        ),
                        _ => dev.build_output_stream::<f32, _, _>(
                            cfg,
                            move |d: &mut [f32], _| cb.fill(d, ch, |v| v),
                            errf,
                            None,
                        ),
                    };
                    match stream {
                        Ok(s) => {
                            if let Err(e) = s.play() {
                                let _ = ready_tx.send(Err(format!("couldn't start {oname}: {e}")));
                                return;
                            }
                            m2.alive.store(true, Relaxed);
                            let _ = ready_tx.send(Ok((oid, oname, sr)));
                            while !st.load(Relaxed) {
                                std::thread::sleep(Duration::from_millis(20));
                            }
                            drop(s);
                            m2.alive.store(false, Relaxed);
                        }
                        Err(e) => {
                            let _ = ready_tx.send(Err(format!("couldn't open {oname}: {e}")));
                        }
                    }
                }
                #[cfg(not(feature = "device"))]
                {
                    let _ = (
                        &out_id,
                        ready_tx.send(Err("This build has no audio devices.".into())),
                    );
                }
            })?,
    );
    match ready_rx.recv_timeout(Duration::from_secs(8)) {
        Ok(Ok((id, name, sr))) => {
            info.output = id;
            info.output_device = name;
            info.sample_rate = sr;
        }
        Ok(Err(e)) => {
            stop.store(true, Relaxed);
            for t in threads {
                let _ = t.join();
            }
            bail!(crate::api::err("device", e));
        }
        Err(_) => {
            stop.store(true, Relaxed);
            bail!(crate::api::err(
                "device",
                "The output device didn't respond."
            ));
        }
    }
    Ok(Runner {
        info,
        meters,
        started: Instant::now(),
        sig: signature(p),
        bypass: false,
        error: None,
        tx,
        garbage: grx,
        stop,
        threads,
    })
}

/// How this computer can capture its own sound.
pub fn capture_support() -> serde_json::Value {
    let (how, note) = if cfg!(target_os = "windows") {
        ("loopback", "Pick any output as the source. Flux hears what plays there. Listen on a different output, like headphones, so it doesn't feed back. The original still plays where it was; to hear only the processed sound, use the Flux speaker.")
    } else if cfg!(target_os = "macos") {
        ("loopback", "Needs macOS 14.6 or later. macOS asks for permission to record system audio the first time.")
    } else {
        ("monitor", "Pick a \"Monitor of ...\" input as the source (PipeWire and PulseAudio list one for each output).")
    };
    serde_json::json!({ "system": how, "note": note, "virtual_device": crate::vdev::can_create(),
        "apps_note": "Play to the Flux microphone, then pick Flux microphone as the microphone in the call app. On Windows and macOS this uses a virtual audio cable you install." })
}

/// Live state held by the session.
#[derive(Default)]
pub struct Live {
    pub routes: BTreeMap<String, Runner>,
    /// Linux: the system output to put back when the computer-sound route stops (Flux made the Flux speaker the
    /// system output while it ran).
    pub restore_sink: Option<String>,
}

impl Drop for Live {
    fn drop(&mut self) {
        if let Some(o) = self.restore_sink.take() {
            let _ = crate::vdev::set_default_sink(&o);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(sr: f32) -> impl FnMut(&mut [f32], &mut [f32]) {
        let mut ph = 0f32;
        move |l: &mut [f32], r: &mut [f32]| {
            for k in 0..l.len() {
                ph = (ph + 440.0 / sr).fract();
                let v = (ph * std::f32::consts::TAU).sin() * 0.25;
                l[k] = v;
                r[k] = v;
            }
        }
    }
    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }
    fn module(kind: &str, params: &[(&str, f32)]) -> Module {
        Module {
            kind: kind.into(),
            params: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn empty_preset_passes_sound_through_unchanged() {
        let mut p = LivePreset {
            safety_limiter: false,
            ..Default::default()
        };
        p.validate().unwrap();
        let (il, _, ol, _) = render(&p, 48000.0, &mut tone(48000.0), 1.0);
        let diff = il
            .iter()
            .zip(&ol)
            .map(|(a, b)| (a - b).abs())
            .fold(0f32, f32::max);
        assert!(diff < 1e-5, "max difference {diff}");
    }

    #[test]
    fn gain_module_changes_level_by_its_decibels() {
        let mut p = LivePreset {
            safety_limiter: false,
            post: vec![module("gain", &[("gain", -6.0)])],
            ..Default::default()
        };
        p.validate().unwrap();
        let (il, _, ol, _) = render(&p, 48000.0, &mut tone(48000.0), 1.0);
        let d = lin_to_db(rms(&ol[4800..])) - lin_to_db(rms(&il[4800..]));
        assert!((d + 6.0).abs() < 0.1, "got {d} dB");
    }

    #[test]
    fn parallel_lanes_sum_and_pan() {
        let mut p = LivePreset {
            safety_limiter: false,
            lanes: vec![
                Lane {
                    pan: -1.0,
                    ..Default::default()
                },
                Lane {
                    pan: -1.0,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        p.validate().unwrap();
        let (il, _, ol, or) = render(&p, 48000.0, &mut tone(48000.0), 0.5);
        // two dry lanes hard left: left is doubled, right is silent
        let d = lin_to_db(rms(&ol[4800..])) - lin_to_db(rms(&il[4800..]));
        assert!((d - 6.02).abs() < 0.1, "left {d} dB");
        assert!(rms(&or[4800..]) < 1e-4);
    }

    #[test]
    fn lfo_moves_a_parameter_and_square_lfo_gates_the_level() {
        // a square LFO on gain: half the time -24 dB lower
        let mut p = LivePreset {
            safety_limiter: false,
            post: vec![Module {
                id: "g".into(),
                ..module("gain", &[("gain", 0.0)])
            }],
            mods: vec![Modulator {
                shape: "square".into(),
                rate_hz: 2.0,
                depth: 1.0,
                bipolar: false,
                targets: vec![Target {
                    module: "g".into(),
                    param: "gain".into(),
                    amount: -0.4, // 40% of the 60 dB range
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
        p.validate().unwrap();
        let (_, _, ol, _) = render(&p, 48000.0, &mut tone(48000.0), 1.0);
        let win: Vec<f32> = ol.chunks(2400).map(rms).collect();
        let hi = win.iter().cloned().fold(0f32, f32::max);
        let lo = win.iter().cloned().fold(1f32, f32::min);
        let swing = lin_to_db(hi) - lin_to_db(lo.max(1e-6));
        assert!(swing > 18.0, "level swing only {swing} dB");
    }

    #[test]
    fn macro_sets_parameters_between_its_ends() {
        let mut p = LivePreset {
            safety_limiter: false,
            post: vec![Module {
                id: "g".into(),
                ..module("gain", &[])
            }],
            macros: vec![Macro {
                value: 0.5,
                targets: vec![Target {
                    module: "g".into(),
                    param: "gain".into(),
                    from: 0.0,
                    to: -12.0,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
        p.validate().unwrap();
        let (il, _, ol, _) = render(&p, 48000.0, &mut tone(48000.0), 0.5);
        let d = lin_to_db(rms(&ol[4800..])) - lin_to_db(rms(&il[4800..]));
        assert!((d + 6.0).abs() < 0.1, "got {d} dB");
    }

    #[test]
    fn validate_rejects_unknown_kinds_params_targets_and_duplicates() {
        let bad = |p: LivePreset| {
            let mut p = p;
            p.validate().is_err()
        };
        assert!(bad(LivePreset {
            post: vec![module("nope", &[])],
            ..Default::default()
        }));
        assert!(bad(LivePreset {
            post: vec![module("speed", &[])],
            ..Default::default()
        }));
        assert!(bad(LivePreset {
            post: vec![module("gain", &[("loud", 1.0)])],
            ..Default::default()
        }));
        assert!(bad(LivePreset {
            post: vec![
                Module {
                    id: "a".into(),
                    ..module("gain", &[])
                },
                Module {
                    id: "a".into(),
                    ..module("gain", &[])
                }
            ],
            ..Default::default()
        }));
        assert!(bad(LivePreset {
            mods: vec![Modulator {
                targets: vec![Target {
                    module: "ghost".into(),
                    param: "gain".into(),
                    amount: 1.0,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        }));
        // out of range values are clamped, not rejected
        let mut p = LivePreset {
            post: vec![module("gain", &[("gain", 999.0)])],
            width: 9.0,
            ..Default::default()
        };
        p.validate().unwrap();
        assert_eq!(p.post[0].params["gain"], 24.0);
        assert_eq!(p.width, 2.0);
        assert!(!p.post[0].id.is_empty());
    }

    #[test]
    fn safety_limiter_keeps_peaks_under_the_ceiling() {
        let mut p = LivePreset {
            post: vec![module("gain", &[("gain", 24.0)])],
            ..Default::default()
        };
        p.validate().unwrap();
        let (_, _, ol, or) = render(&p, 48000.0, &mut tone(48000.0), 1.0);
        let pk = ol[4800..]
            .iter()
            .chain(&or[4800..])
            .fold(0f32, |m, v| m.max(v.abs()));
        assert!(lin_to_db(pk) < -0.8, "peak {} dBFS", lin_to_db(pk));
    }

    #[test]
    fn in_place_updates_match_a_fresh_build() {
        let mut a = LivePreset {
            safety_limiter: false,
            post: vec![Module {
                id: "g".into(),
                ..module("gain", &[("gain", 0.0)])
            }],
            ..Default::default()
        };
        a.validate().unwrap();
        let mut b = a.clone();
        b.post[0].params.insert("gain".into(), -12.0);
        assert_eq!(signature(&a), signature(&b));
        let mut g = Graph::build(&a, 48000.0);
        for m in Graph::updates(&b) {
            assert!(g.apply(m).is_none());
        }
        let mut src = tone(48000.0);
        let mut last = 0.0;
        for _ in 0..200 {
            let (mut l, mut r) = (vec![0.0; BLOCK], vec![0.0; BLOCK]);
            src(&mut l, &mut r);
            let inr = rms(&l);
            g.process(&mut l, &mut r);
            last = lin_to_db(rms(&l)) - lin_to_db(inr);
        }
        assert!((last + 12.0).abs() < 0.1, "got {last}");
    }

    #[test]
    fn sync_divisions_follow_tempo() {
        let m = Modulator {
            sync: "1/4".into(),
            ..Default::default()
        };
        assert!((mod_rate(&m, 120.0) - 2.0).abs() < 1e-6);
        let m = Modulator {
            sync: "1/8t".into(),
            ..Default::default()
        };
        assert!((mod_rate(&m, 120.0) - 6.0).abs() < 1e-4);
        let m = Modulator {
            sync: "1/1".into(),
            ..Default::default()
        };
        assert!((mod_rate(&m, 60.0) - 0.25).abs() < 1e-6);
    }

    #[test]
    fn every_modulator_shape_stays_finite_and_in_range() {
        for (i, (shape, _)) in SHAPES.iter().enumerate() {
            let mut p = LivePreset {
                post: vec![Module {
                    id: "f".into(),
                    ..module("reverb", &[])
                }],
                mods: vec![Modulator {
                    shape: shape.to_string(),
                    rate_hz: 13.0,
                    depth: 1.0,
                    steps: vec![0.0, 1.0, 0.25],
                    smooth: (i % 3) as f32 * 0.3,
                    targets: vec![
                        Target {
                            module: "f".into(),
                            param: "decay".into(),
                            amount: 1.0,
                            ..Default::default()
                        },
                        Target {
                            module: "f".into(),
                            param: "mix".into(),
                            amount: -1.0,
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                }],
                ..Default::default()
            };
            p.validate().unwrap();
            let (_, _, ol, or) = render(&p, 48000.0, &mut tone(48000.0), 0.5);
            assert!(
                ol.iter()
                    .chain(&or)
                    .all(|v| v.is_finite() && v.abs() <= 1.0),
                "{shape}"
            );
        }
    }

    #[test]
    fn demo_route_runs_on_the_null_device_and_meters_move() {
        let mut p = LivePreset {
            post: vec![module("gain", &[("gain", -6.0)])],
            ..Default::default()
        };
        p.validate().unwrap();
        let mut r = start(
            StartOpts {
                route: "output".into(),
                source: Source::DemoMusic,
                output: Some("none".into()),
            },
            &p,
        )
        .unwrap();
        // Levels are measured over a window, so wait (up to 3 s on a busy machine) for it to fill.
        let settle = |r: &Runner, want: f64, tol: f64| {
            let mut last = (0.0, 0.0);
            for _ in 0..60 {
                std::thread::sleep(Duration::from_millis(50));
                let s = r.status();
                last = (
                    s["in_rms_db"].as_f64().unwrap(),
                    s["out_rms_db"].as_f64().unwrap(),
                );
                if s["frames"].as_u64().unwrap_or(0) > 9600
                    && last.0 > -40.0
                    && (last.1 - last.0 - want).abs() < tol
                {
                    break;
                }
            }
            last
        };
        let (i, o) = settle(&r, -6.0, 1.0);
        let s = r.status();
        assert_eq!(s["running"], true);
        assert!(s["frames"].as_u64().unwrap() > 9000, "{s}");
        assert!(i > -40.0, "input level {i}");
        assert!((o - i + 6.0).abs() < 1.0, "in {i} out {o}");
        // a live edit lands without a rebuild
        p.post[0].params.insert("gain".into(), -18.0);
        r.update(&p);
        let (i, o) = settle(&r, -18.0, 1.0);
        assert!((o - i + 18.0).abs() < 1.0, "after edit: in {i} out {o}");
        // bypass returns the input
        r.set_bypass(true);
        let (i, o) = settle(&r, 0.0, 0.5);
        assert!((o - i).abs() < 0.5, "bypass: in {i} out {o}");
        r.stop();
    }
}
