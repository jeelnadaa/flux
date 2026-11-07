//! Every API method: name, description, parameters and handler. This list is the MCP tool list.

use crate::api::{err, f_, fparams, need, ob, of, opos, os, ou, pos, s_, u_, Api, Hist, RecCtx};
use crate::export::{self, ExportOpts};
use crate::media::Media;
use crate::project::{Effect, MediaRef, Project};
use crate::store::{new_id, Action, ActionStep, Preset};
use crate::{analysis, fx, render, tools};
use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub type Handler = fn(&Api, &Value) -> Result<Value>;

pub struct MethodSpec {
    pub name: &'static str,
    pub desc: &'static str,
    /// "!name:type:description|..."; `!` marks required. Types: string, integer, number, boolean,
    /// object, array, pos (samples, or seconds as `<name>_s`).
    pub params: &'static str,
    /// Mutating methods are recorded into Actions while recording.
    pub records: bool,
    pub handler: Handler,
}

macro_rules! m {
    ($name:expr, $desc:expr, $params:expr, $rec:expr, $h:expr) => {
        MethodSpec {
            name: $name,
            desc: $desc,
            params: $params,
            records: $rec,
            handler: $h,
        }
    };
}

pub struct ParamDef {
    pub name: &'static str,
    pub ty: &'static str,
    pub desc: &'static str,
    pub required: bool,
}

pub fn params_of(spec: &MethodSpec) -> Vec<ParamDef> {
    spec.params
        .split('|')
        .filter(|s| !s.trim().is_empty())
        .map(|p| {
            let p = p.trim();
            let (required, p) = if let Some(r) = p.strip_prefix('!') {
                (true, r)
            } else {
                (false, p)
            };
            let mut it = p.splitn(3, ':');
            ParamDef {
                name: it.next().unwrap_or(""),
                ty: it.next().unwrap_or("string"),
                desc: it.next().unwrap_or(""),
                required,
            }
        })
        .collect()
}

/// JSON schema for a method's params (MCP `inputSchema`).
pub fn schema(spec: &MethodSpec) -> Value {
    let mut props = serde_json::Map::new();
    let mut req = vec![];
    for p in params_of(spec) {
        let t = match p.ty {
            "pos" => {
                props.insert(format!("{}_s", p.name), json!({ "type": "number", "description": format!("{} (in seconds; alternative to `{}`)", p.desc, p.name) }));
                "integer"
            }
            t => t,
        };
        let mut o = json!({ "type": t, "description": p.desc });
        if t == "array" {
            o["items"] = json!({});
        }
        props.insert(p.name.to_string(), o);
        if p.required && p.ty != "pos" {
            req.push(p.name);
        }
    }
    json!({ "type": "object", "properties": props, "required": req, "additionalProperties": false })
}

pub fn check_params(spec: &MethodSpec, v: &Value) -> Result<()> {
    let defs = params_of(spec);
    let o = v.as_object().unwrap();
    for k in o.keys() {
        let known = defs
            .iter()
            .any(|d| d.name == k || (d.ty == "pos" && k.strip_suffix("_s") == Some(d.name)));
        if !known {
            bail!(err(
                "bad_params",
                format!(
                    "{} does not take `{k}`. It takes: {}",
                    spec.name,
                    defs.iter().map(|d| d.name).collect::<Vec<_>>().join(", ")
                )
            ));
        }
    }
    for d in &defs {
        let present = o.get(d.name).map(|x| !x.is_null()).unwrap_or(false)
            || (d.ty == "pos" && o.contains_key(&format!("{}_s", d.name)));
        if d.required && !present {
            bail!(err(
                "bad_params",
                format!(
                    "{} needs `{}`{}",
                    spec.name,
                    d.name,
                    if d.ty == "pos" {
                        format!(" (samples) or `{}_s` (seconds)", d.name)
                    } else {
                        String::new()
                    }
                )
            ));
        }
        if let Some(x) = o.get(d.name) {
            let ok = match d.ty {
                "string" => x.is_string(),
                "integer" | "pos" => {
                    x.is_i64()
                        || x.is_u64()
                        || x.as_f64().map(|f| f.fract() == 0.0).unwrap_or(false)
                }
                "number" => x.is_number(),
                "boolean" => x.is_boolean(),
                "object" => x.is_object(),
                "array" => x.is_array(),
                _ => true,
            };
            if !ok && !x.is_null() {
                bail!(err("bad_params", format!("`{}` must be {}", d.name, d.ty)));
            }
        }
    }
    Ok(())
}

pub fn find(name: &str) -> Option<&'static MethodSpec> {
    let n = name.replace('_', ".");
    METHODS
        .iter()
        .find(|m| m.name == name || m.name == n || m.name.replace('.', "_") == name)
}

pub fn list() -> Value {
    Value::Array(METHODS.iter().map(|m| json!({ "name": m.name, "description": m.desc, "params": schema(m), "mutates": m.records })).collect())
}

// ---------------------------------------------------------------- helpers

fn sr(api: &Api) -> u32 {
    api.lock()
        .project
        .as_ref()
        .map(|p| p.sample_rate)
        .unwrap_or(48000)
}

fn project_json(api: &Api) -> Result<Value> {
    let s = api.lock();
    let p = s.project()?;
    let mut v = serde_json::to_value(p)?;
    v["length"] = json!(p.length());
    v["hash"] = json!(format!("{:016x}", p.hash()));
    v["seconds"] = json!(p.length() as f64 / p.sample_rate as f64);
    v["rev"] = json!(s.rev);
    v["can_undo"] = json!(!s.undo.is_empty());
    v["can_redo"] = json!(!s.redo.is_empty());
    v["undo_label"] = json!(s.undo.last().map(|h| h.label.clone()));
    v["redo_label"] = json!(s.redo.last().map(|h| h.label.clone()));
    v["missing"] = json!(s.missing);
    v["dir"] = json!(s.store.project_dir(&p.id).display().to_string());
    Ok(v)
}

fn track_target(p: &Project, v: &Value) -> Result<u32> {
    if let Some(t) = ou(v, "track") {
        p.track(t)?;
        return Ok(t);
    }
    if let Some(t) = p.selection.track {
        if p.track(t).is_ok() {
            return Ok(t);
        }
    }
    p.tracks
        .first()
        .map(|t| t.id)
        .ok_or_else(|| err("empty", "The project has no tracks"))
}

fn unique_path(p: PathBuf) -> PathBuf {
    if !p.exists() {
        return p;
    }
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = p
        .extension()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    for i in 2..1000 {
        let q = p.with_file_name(format!("{stem} ({i}).{ext}"));
        if !q.exists() {
            return q;
        }
    }
    p
}

fn file_name_safe(s: &str) -> String {
    let t: String = s
        .chars()
        .map(|c| {
            if "\\/:*?\"<>|".contains(c) || c.is_control() {
                '-'
            } else {
                c
            }
        })
        .collect();
    let t = t.trim().trim_matches('.').to_string();
    if t.is_empty() {
        "Untitled".into()
    } else {
        t.chars().take(120).collect()
    }
}

fn write_wav32(path: &Path, l: &[f32], r: &[f32], sr: u32) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: sr,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let tmp = path.with_extension("part");
    let mut w = hound::WavWriter::create(&tmp, spec)?;
    for i in 0..l.len() {
        w.write_sample(l[i])?;
        w.write_sample(r[i])?;
    }
    w.finalize()?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Store generated audio as a new media file in the project and the pool. Returns its id.
fn add_generated_media(
    api: &Api,
    name: &str,
    l: Vec<f32>,
    r: Vec<f32>,
    synthetic: bool,
) -> Result<String> {
    let (pid, sr, dir) = {
        let s = api.lock();
        let p = s.project()?;
        (p.id.clone(), p.sample_rate, s.store.media_dir(&p.id))
    };
    let id = format!("m{}", new_id());
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{id}.wav"));
    write_wav32(&path, &l, &r, sr)?;
    let frames = l.len() as i64;
    let media = Media::new(&id, vec![l, r], sr, sr);
    let mut s = api.lock();
    if s.project()?.id != pid {
        bail!(err(
            "changed",
            "The project was closed while this was running"
        ));
    }
    s.add_media(media);
    s.project_mut()?.media.push(MediaRef {
        id: id.clone(),
        name: name.to_string(),
        path: path.display().to_string(),
        original: String::new(),
        linked: false,
        frames,
        channels: 2,
        source_rate: sr,
        format: "wav".into(),
        bits: 32,
        tags: BTreeMap::new(),
        synthetic,
    });
    Ok(id)
}

fn spec_json(s: &fx::FxSpec) -> Value {
    json!({
        "kind": s.kind, "name": s.name, "group": s.group, "master_only": s.master_only, "default_mix": s.default_mix,
        "params": s.params.iter().map(|p| {
            let options: Option<Vec<&str>> = p.unit.strip_prefix("enum:").map(|o| o.split('|').collect());
            json!({ "key": p.key, "label": p.label, "min": p.min, "max": p.max, "default": p.def, "unit": if options.is_some() { "enum" } else { p.unit }, "log": p.log, "options": options })
        }).collect::<Vec<_>>()
    })
}

fn pmap(pairs: &[(&str, f32)]) -> BTreeMap<String, f32> {
    pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

/// Named looks: (kind, params, mix). `a` is the 0..1 amount from the simple-mode slider.
/// One effect of a look: kind, parameter values, dry/wet mix.
pub type StyleFx = (&'static str, BTreeMap<String, f32>, f32);

pub fn style_fx(style: &str, a: f32) -> Option<Vec<StyleFx>> {
    let a = a.clamp(0.0, 1.0);
    let v = match style {
        "slowed" => vec![
            (
                "speed",
                pmap(&[("rate", 1.0 - 0.3 * a), ("keep_pitch", 0.0)]),
                1.0,
            ),
            (
                "reverb",
                pmap(&[("decay", 1.0 + 4.4 * a), ("predelay", 20.0), ("size", 1.2)]),
                0.12 + 0.36 * a,
            ),
            (
                "eq",
                pmap(&[("b5_type", 2.0), ("b5_freq", 7000.0), ("b5_gain", -6.0 * a)]),
                1.0,
            ),
        ],
        "nightcore" => vec![
            (
                "speed",
                pmap(&[("rate", 1.0 + 0.5 * a), ("keep_pitch", 0.0)]),
                1.0,
            ),
            (
                "eq",
                pmap(&[("b5_type", 2.0), ("b5_freq", 8000.0), ("b5_gain", 4.0 * a)]),
                1.0,
            ),
            ("chorus", pmap(&[]), 0.36 * a),
        ],
        "8d" => vec![(
            "spatial",
            pmap(&[
                ("mode", 0.0),
                ("rate", 0.02 + 0.33 * a),
                ("distance", 1.8),
                ("room", 40.0),
            ]),
            1.0,
        )],
        "lofi" => vec![("lofi", pmap(&[("amount", 100.0 * a)]), 1.0)],
        "cathedral" => vec![(
            "reverb",
            pmap(&[
                ("decay", 6.8),
                ("size", 1.5),
                ("predelay", 40.0),
                ("damping", 5000.0),
            ]),
            0.2 + 0.4 * a,
        )],
        "robot" => vec![(
            "voice",
            pmap(&[("ring", 40.0), ("ring_freq", 30.0), ("comb", 55.0)]),
            a.max(0.05),
        )],
        "reverb" => vec![("reverb", pmap(&[]), 0.1 + 0.5 * a)],
        k @ ("vinyl" | "cassette" | "radio" | "phone" | "megaphone" | "walkie" | "underwater"
        | "distortion" | "bitcrusher" | "chorus" | "flanger" | "phaser" | "rotary"
        | "stutter" | "tapestop" | "granular" | "trancegate" | "glitch" | "row10"
        | "nextdoor") => {
            let k: &'static str = fx::spec(k)?.kind;
            vec![(k, pmap(&[]), a.max(0.05))]
        }
        _ => return None,
    };
    Some(v)
}

pub const ROOMS: [(&str, f32, f32, f32, f32); 12] = [
    ("Cathedral", 6.8, 1.5, 40.0, 5000.0),
    ("Bathroom", 0.9, 0.4, 2.0, 12000.0),
    ("Concert hall", 3.2, 1.2, 24.0, 7000.0),
    ("Cave", 4.4, 1.3, 30.0, 4000.0),
    ("Parking garage", 2.6, 1.0, 12.0, 9000.0),
    ("Studio booth", 0.2, 0.3, 0.0, 14000.0),
    ("Stairwell", 1.8, 0.7, 6.0, 10000.0),
    ("Living room", 0.5, 0.5, 3.0, 8000.0),
    ("Car interior", 0.1, 0.3, 0.0, 6000.0),
    ("Stadium", 5.1, 1.5, 80.0, 6000.0),
    ("Forest clearing", 0.7, 1.4, 15.0, 9000.0),
    ("Subway tunnel", 3.8, 1.1, 18.0, 5000.0),
];

pub const VOICES: [(&str, &[(&str, f32)]); 16] = [
    ("Deep", &[("pitch", -5.0)]),
    ("Chipmunk", &[("pitch", 7.0), ("formant", 30.0)]),
    ("Child", &[("pitch", 4.0), ("formant", 30.0)]),
    ("Elder", &[("pitch", -1.0), ("tremor", 45.0)]),
    ("Shift up", &[("pitch", 3.0)]),
    ("Shift down", &[("pitch", -3.0)]),
    (
        "Whisper",
        &[("vocoder", 100.0), ("carrier", 1.0), ("bands", 24.0)],
    ),
    ("Announcer", &[("pitch", -2.0), ("formant", -10.0)]),
    (
        "Radio DJ",
        &[("pitch", -1.0), ("formant", -15.0), ("comb", 10.0)],
    ),
    (
        "Villain",
        &[
            ("pitch", -4.0),
            ("formant", -30.0),
            ("ring", 10.0),
            ("ring_freq", 40.0),
        ],
    ),
    ("Giant", &[("pitch", -12.0), ("formant", -50.0)]),
    ("Tiny", &[("pitch", 12.0), ("formant", 50.0)]),
    (
        "Robot",
        &[("ring", 40.0), ("ring_freq", 30.0), ("comb", 55.0)],
    ),
    (
        "Cyborg",
        &[
            ("ring", 20.0),
            ("ring_freq", 60.0),
            ("comb", 30.0),
            ("vocoder", 50.0),
        ],
    ),
    (
        "Alien",
        &[
            ("pitch", 3.0),
            ("formant", -60.0),
            ("ring", 15.0),
            ("ring_freq", 200.0),
        ],
    ),
    (
        "Monster",
        &[
            ("pitch", -8.0),
            ("formant", -40.0),
            ("ring", 25.0),
            ("ring_freq", 50.0),
        ],
    ),
];

pub fn builtin_presets() -> Vec<Preset> {
    let eq = |id: &str, name: &str, p: &[(&str, f32)]| Preset {
        id: id.into(),
        name: name.into(),
        kind: "eq".into(),
        params: pmap(p),
        chain: vec![],
        created: 0,
        builtin: true,
    };
    let mut v = vec![
        eq(
            "eq-clear-voice",
            "Clear voice",
            &[
                ("b1_type", 4.0),
                ("b1_freq", 80.0),
                ("b2_freq", 300.0),
                ("b2_gain", -2.0),
                ("b4_freq", 3400.0),
                ("b4_gain", 3.0),
                ("b5_freq", 11000.0),
                ("b5_gain", 2.0),
            ],
        ),
        eq(
            "eq-bass-boost",
            "Bass boost",
            &[("b1_type", 1.0), ("b1_freq", 100.0), ("b1_gain", 6.0)],
        ),
        eq(
            "eq-treble-boost",
            "Treble boost",
            &[("b5_freq", 8000.0), ("b5_gain", 6.0)],
        ),
        eq(
            "eq-warm",
            "Warm",
            &[("b2_freq", 200.0), ("b2_gain", 2.0), ("b5_gain", -2.0)],
        ),
        eq(
            "eq-bright",
            "Bright",
            &[("b5_freq", 10000.0), ("b5_gain", 4.0)],
        ),
        eq(
            "eq-podcast",
            "Podcast",
            &[
                ("b1_type", 4.0),
                ("b1_freq", 80.0),
                ("b2_freq", 300.0),
                ("b2_gain", -3.0),
                ("b4_freq", 3000.0),
                ("b4_gain", 2.0),
                ("b5_freq", 10000.0),
                ("b5_gain", 1.5),
            ],
        ),
        eq(
            "eq-de-mud",
            "De-mud",
            &[("b2_freq", 300.0), ("b2_gain", -4.0), ("b2_q", 1.2)],
        ),
        eq(
            "eq-presence",
            "Presence",
            &[("b4_freq", 4000.0), ("b4_gain", 4.0)],
        ),
        eq("eq-air", "Air", &[("b5_freq", 14000.0), ("b5_gain", 4.0)]),
    ];
    for (n, d, s, pd, dm) in ROOMS {
        v.push(Preset {
            id: format!("room-{}", n.to_lowercase().replace(' ', "-")),
            name: n.into(),
            kind: "reverb".into(),
            params: pmap(&[("decay", d), ("size", s), ("predelay", pd), ("damping", dm)]),
            chain: vec![],
            created: 0,
            builtin: true,
        });
    }
    for (n, p) in VOICES {
        v.push(Preset {
            id: format!("voice-{}", n.to_lowercase().replace(' ', "-")),
            name: n.into(),
            kind: "voice".into(),
            params: pmap(p),
            chain: vec![],
            created: 0,
            builtin: true,
        });
    }
    v
}

fn make_effect(
    p: &mut Project,
    kind: &str,
    params: &BTreeMap<String, f32>,
    mix: Option<f32>,
    group: Option<String>,
) -> Result<Effect> {
    let spec = fx::spec(kind).ok_or_else(|| {
        err(
            "bad_params",
            format!("unknown effect `{kind}`. Call fx.catalog for the list."),
        )
    })?;
    for k in params.keys() {
        if !spec.params.iter().any(|q| q.key == k) {
            bail!(err(
                "bad_params",
                format!("{} has no parameter `{k}`", spec.name)
            ));
        }
    }
    let mut e = Effect {
        id: p.alloc(),
        kind: spec.kind.to_string(),
        enabled: true,
        mix: mix.unwrap_or(spec.default_mix).clamp(0.0, 1.0),
        params: fx::defaults(spec),
        group,
        profile: None,
        ir: None,
        path: vec![],
        slots: vec![],
        slot: 0,
    };
    for (k, v) in params {
        if let Some(q) = spec.params.iter().find(|q| q.key == k) {
            e.params.insert(k.clone(), v.clamp(q.min, q.max));
        }
    }
    Ok(e)
}

fn insert_effect(p: &mut Project, target: &str, e: Effect, index: Option<usize>) -> Result<u32> {
    let id = e.id;
    let master_only = fx::spec(&e.kind).map(|s| s.master_only).unwrap_or(false);
    let target = if master_only { "master" } else { target };
    let chain = p.chain_mut(target)?;
    if master_only {
        chain.retain(|x| x.kind != e.kind);
    }
    let i = index.unwrap_or(chain.len()).min(chain.len());
    chain.insert(i, e);
    Ok(id)
}

// ---------------------------------------------------------------- app, settings, system

fn h_methods(_: &Api, _: &Value) -> Result<Value> {
    Ok(list())
}

fn h_app_info(api: &Api, _: &Value) -> Result<Value> {
    let s = api.lock();
    let tts = !tools::voices().is_empty();
    Ok(json!({
        "name": "Flux", "version": env!("CARGO_PKG_VERSION"), "platform": std::env::consts::OS, "arch": std::env::consts::ARCH,
        "data_dir": s.store.root.display().to_string(), "headless": s.headless,
        "project": s.project.as_ref().map(|p| json!({"id": p.id, "name": p.name})),
        "formats": export::formats(), "tts": tts, "ffmpeg": export::find_tool("ffmpeg").is_some(),
        "methods": METHODS.len(), "uptime_s": s.started.elapsed().as_secs(),
        "ai": ai_status(),
    }))
}

fn ai_status() -> Value {
    json!([
        {"id": "stems", "available": true, "how": "dsp", "note": "Spectral split (mid/side + harmonic/percussive). Not a neural model."},
        {"id": "cleanup", "available": true, "how": "dsp", "note": "Learned noise profile, de-ess, compression and leveling."},
        {"id": "master", "available": true, "how": "dsp", "note": "Measured EQ tilt, compression and true-peak limiting."},
        {"id": "magic", "available": true, "how": "dsp", "note": "Problem detection with measured thresholds."},
        {"id": "tempo_key", "available": true, "how": "dsp", "note": "Onset autocorrelation and chroma key profiles."},
        {"id": "tts", "available": !tools::voices().is_empty(), "how": "system", "note": "This computer's own speech voices. Output is labelled synthetic."},
        {"id": "transcript", "available": false, "how": "model", "note": "Needs a speech recognition model, which this build does not include."},
        {"id": "fillers", "available": false, "how": "model", "note": "Needs a transcript."},
        {"id": "dub", "available": false, "how": "model", "note": "Needs speech recognition, translation and a voice model; none are in this build."},
        {"id": "clone", "available": false, "how": "model", "note": "Voice cloning is not included."}
    ])
}

fn unavailable(what: &str) -> anyhow::Error {
    err("unavailable", format!("{what} needs a speech model, and this build of Flux doesn't include one. Nothing was sent anywhere."))
}
fn h_unavailable_transcript(_: &Api, _: &Value) -> Result<Value> {
    Err(unavailable("Transcription"))
}
fn h_unavailable_dub(_: &Api, _: &Value) -> Result<Value> {
    Err(unavailable("Translate and dub"))
}
fn h_unavailable_clone(_: &Api, _: &Value) -> Result<Value> {
    Err(unavailable("Voice cloning"))
}

fn h_ai_status(_: &Api, _: &Value) -> Result<Value> {
    Ok(ai_status())
}

fn h_capabilities(api: &Api, _: &Value) -> Result<Value> {
    let (backend, device, buffer, sr, writable, root) = {
        let s = api.lock();
        let root = s.store.root.clone();
        let w = std::fs::write(root.join(".probe"), b"x").is_ok();
        let _ = std::fs::remove_file(root.join(".probe"));
        let had = s.engine.is_some();
        let info = if had {
            let e = s.engine.as_ref().unwrap();
            (e.backend.clone(), e.device.clone(), e.buffer, e.sample_rate)
        } else {
            let d = crate::device::list_devices();
            let o = d
                .outputs
                .iter()
                .find(|o| o.default)
                .cloned()
                .unwrap_or_default();
            (d.host, o.name, 0, o.sample_rate)
        };
        (info.0, info.1, info.2, info.3, w, root)
    };
    let aac = export::formats()
        .into_iter()
        .find(|f| f.id == "aac")
        .map(|f| f.available)
        .unwrap_or(false);
    let latency = if buffer > 0 && sr > 0 {
        format!("{:.1} ms buffer", buffer as f32 / sr as f32 * 1000.0)
    } else {
        "System default buffer".into()
    };
    Ok(json!([
        {"id": "engine", "label": "Audio engine", "detail": format!("{} · {}", backend, if device.is_empty() { "no output found".into() } else { device.clone() }), "state": if device.is_empty() { "warn" } else { "ok" }, "value": if device.is_empty() { "Silent" } else { "Ready" }},
        {"id": "latency", "label": "Low-latency mode", "detail": latency, "state": "ok", "value": "Ready"},
        {"id": "ai", "label": "On-device processing", "detail": "Stems, cleanup and mastering run on this computer with DSP", "state": "ok", "value": "Ready"},
        {"id": "folders", "label": "Saving to folders", "detail": root.display().to_string(), "state": if writable { "ok" } else { "bad" }, "value": if writable { "Ready" } else { "Read-only" }},
        {"id": "aac", "label": "AAC export", "detail": if aac { "System encoder found" } else { "Install ffmpeg for AAC and M4R" }, "state": if aac { "ok" } else { "warn" }, "value": if aac { "Ready" } else { "Missing" }},
        {"id": "offline", "label": "Offline use", "detail": "Flux never needs the internet", "state": "ok", "value": "Ready"},
    ]))
}

fn h_settings_get(api: &Api, _: &Value) -> Result<Value> {
    Ok(serde_json::to_value(&api.lock().settings)?)
}

fn h_settings_set(api: &Api, v: &Value) -> Result<Value> {
    let mut s = api.lock();
    let mut cur = serde_json::to_value(&s.settings)?;
    let vals = need(v, "values")?
        .as_object()
        .ok_or_else(|| err("bad_params", "`values` must be an object"))?;
    for (k, x) in vals {
        if cur.get(k).is_none() {
            bail!(err("bad_params", format!("unknown setting `{k}`")));
        }
        cur[k] = x.clone();
    }
    let new: crate::store::Settings = serde_json::from_value(cur)
        .map_err(|e| err("bad_params", format!("invalid setting: {e}")))?;
    let device_changed = new.output != s.settings.output || new.buffer != s.settings.buffer;
    let safety = new.safety_limiter;
    s.settings = new;
    let st = s.settings.clone();
    s.store.save_settings(&st)?;
    if device_changed && s.engine.is_some() {
        let pos = s.engine.as_ref().map(|e| e.meters(0).pos).unwrap_or(0);
        s.stop_engine();
        s.ensure_engine()?.send(render::Msg::Seek(pos));
    }
    if let Some(e) = s.engine.as_mut() {
        e.send(render::Msg::SafetyLimiter(safety));
    }
    s.rev += 1;
    Ok(serde_json::to_value(&s.settings)?)
}

fn h_diag(api: &Api, _: &Value) -> Result<Value> {
    let s = api.lock();
    let d = crate::device::list_devices();
    let eng = s.engine.as_ref().map(|e| json!({"backend": e.backend, "device": e.device, "sample_rate": e.sample_rate, "buffer": e.buffer, "glitches": e.meters(0).glitches, "error": e.error}));
    // never includes audio, transcripts or file names
    Ok(json!({
        "version": env!("CARGO_PKG_VERSION"), "os": std::env::consts::OS, "arch": std::env::consts::ARCH,
        "host": d.host, "outputs": d.outputs.len(), "inputs": d.inputs.len(), "engine": eng,
        "project": s.project.as_ref().map(|p| json!({"tracks": p.tracks.len(), "clips": p.tracks.iter().map(|t| t.clips.len()).sum::<usize>(), "effects": p.tracks.iter().map(|t| t.fx.len()).sum::<usize>() + p.master.fx.len(), "sample_rate": p.sample_rate, "seconds": p.length() as f64 / p.sample_rate as f64})),
        "jobs": api.jobs.list().iter().map(|j| json!({"kind": j.kind, "state": j.state})).collect::<Vec<_>>(),
        "last_error": s.last_error.as_ref().map(|_| "present"), "calls": s.calls, "uptime_s": s.started.elapsed().as_secs(),
        "ffmpeg": export::find_tool("ffmpeg").is_some(),
    }))
}

fn h_storage_usage(api: &Api, _: &Value) -> Result<Value> {
    let s = api.lock();
    let mut u = s.store.usage();
    u["library"] = json!(s.store.library().len());
    Ok(u)
}
fn h_storage_empty_trash(api: &Api, _: &Value) -> Result<Value> {
    Ok(json!({ "freed": api.lock().store.empty_trash()? }))
}

fn h_recovery_list(api: &Api, _: &Value) -> Result<Value> {
    let s = api.lock();
    let c = s.store.crashed();
    Ok(Value::Array(c.iter().map(|e| json!({"id": e.id, "name": e.name, "modified": e.modified, "journal": s.store.journal_tail(&e.id, 5)})).collect()))
}
fn h_recovery_dismiss(api: &Api, v: &Value) -> Result<Value> {
    let id = s_(v, "id")?;
    crate::store::safe_id(&id)?;
    api.lock().store.unlock(&id);
    Ok(json!({"ok": true}))
}

// ---------------------------------------------------------------- library and projects

fn h_library(api: &Api, v: &Value) -> Result<Value> {
    let s = api.lock();
    let q = os(v, "search").unwrap_or_default().to_lowercase();
    let kind = os(v, "kind").unwrap_or_default().to_lowercase();
    let folder = os(v, "folder").unwrap_or_default();
    let items: Vec<_> = s
        .store
        .library()
        .into_iter()
        .filter(|e| q.is_empty() || e.name.to_lowercase().contains(&q))
        .filter(|e| kind.is_empty() || kind == "all" || e.kind.to_lowercase() == kind)
        .filter(|e| folder.is_empty() || e.folder == folder)
        .collect();
    Ok(
        json!({ "projects": items, "folders": s.settings.folders, "current": s.project.as_ref().map(|p| p.id.clone()) }),
    )
}

fn template(p: &mut Project, t: &str) {
    let add = |p: &mut Project, name: &str, color: u8, gain: f32| {
        let id = p.add_track(name);
        let t = p.track_mut(id).unwrap();
        t.color = color;
        t.gain_db = gain;
        id
    };
    let fxm =
        |p: &mut Project, target: &str, kind: &str, params: &[(&str, f32)], mix: Option<f32>| {
            if let Ok(e) = make_effect(p, kind, &pmap(params), mix, Some("template".into())) {
                let _ = insert_effect(p, target, e, None);
            }
        };
    p.kind = match t {
        "podcast" | "interview" => "podcast",
        "music" => "music",
        "voiceover" => "voice",
        "lecture" => "lecture",
        "8d" => "spatial",
        _ => "blank",
    }
    .into();
    match t {
        "podcast" => {
            add(p, "Host", 4, 0.0);
            add(p, "Guest", 1, 0.0);
            add(p, "Music bed", 6, -18.0);
            p.target_lufs = -16.0;
            fxm(
                p,
                "master",
                "compressor",
                &[("threshold", -20.0), ("ratio", 2.0)],
                None,
            );
            fxm(p, "master", "limiter", &[("ceiling", -1.0)], None);
        }
        "music" => {
            for (i, n) in [
                "Vocals",
                "Drums",
                "Bass",
                "Keys",
                "Guitars",
                "Synth",
                "FX",
                "Room tone",
            ]
            .iter()
            .enumerate()
            {
                add(p, n, (i % 8 + 1) as u8, 0.0);
            }
            fxm(p, "master", "eq", &[], None);
            fxm(
                p,
                "master",
                "compressor",
                &[("threshold", -14.0), ("ratio", 2.0)],
                None,
            );
            fxm(p, "master", "limiter", &[("ceiling", -1.0)], None);
        }
        "voiceover" => {
            let v = add(p, "Voice", 4, 0.0);
            add(p, "Room tone", 6, -12.0);
            let tgt = format!("track:{v}");
            fxm(p, &tgt, "eq", &[("b1_type", 4.0), ("b1_freq", 80.0)], None);
            fxm(p, &tgt, "deesser", &[], None);
            fxm(
                p,
                &tgt,
                "compressor",
                &[("threshold", -22.0), ("ratio", 3.0)],
                None,
            );
        }
        "lecture" => {
            let v = add(p, "Lecture", 4, 0.0);
            p.target_lufs = -16.0;
            fxm(p, &format!("track:{v}"), "leveler", &[], None);
        }
        "interview" => {
            add(p, "Speaker 1", 4, 0.0);
            add(p, "Speaker 2", 1, 0.0);
            p.target_lufs = -16.0;
        }
        "8d" => {
            let v = add(p, "Stereo in", 1, 0.0);
            fxm(
                p,
                &format!("track:{v}"),
                "spatial",
                &[("mode", 0.0), ("rate", 0.1), ("room", 40.0)],
                None,
            );
        }
        _ => {
            add(p, "Track 1", 1, 0.0);
        }
    }
}

fn h_project_new(api: &Api, v: &Value) -> Result<Value> {
    let name = os(v, "name")
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Untitled project".into());
    let rate = ou(v, "sample_rate").unwrap_or_else(|| api.lock().settings.default_rate);
    if ![44100, 48000, 88200, 96000, 176400, 192000].contains(&rate) {
        bail!(err(
            "bad_params",
            "sample_rate must be 44100, 48000, 88200, 96000, 176400 or 192000"
        ));
    }
    let mut p = Project::new(name.trim(), rate);
    p.id = new_id();
    if let Some(f) = os(v, "folder") {
        p.folder = f;
    }
    template(&mut p, &os(v, "template").unwrap_or_else(|| "blank".into()));
    let mut s = api.lock();
    s.store.save_project(&p)?;
    s.install(p, crate::media::Pool::new(), vec![]);
    drop(s);
    project_json(api)
}

fn h_project_open(api: &Api, v: &Value) -> Result<Value> {
    let id = s_(v, "id")?;
    let p = api.lock().store.load_project(&id)?;
    let (pool, missing) = crate::api::load_media(&p);
    api.lock().install(p, pool, missing);
    project_json(api)
}

fn h_project_close(api: &Api, _: &Value) -> Result<Value> {
    api.lock().close();
    Ok(json!({"ok": true}))
}

fn h_project_get(api: &Api, _: &Value) -> Result<Value> {
    project_json(api)
}

fn h_project_set(api: &Api, v: &Value) -> Result<Value> {
    let mut s = api.lock();
    s.commit("Change project settings", |p| {
        if let Some(n) = os(v, "name") {
            if n.trim().is_empty() {
                bail!(err("bad_params", "the name cannot be empty"));
            }
            p.name = n.trim().chars().take(200).collect();
        }
        if let Some(t) = of(v, "tempo") {
            p.tempo = t.clamp(20.0, 400.0);
        }
        if let Some(k) = os(v, "key") {
            p.key = k;
        }
        if let Some(t) = of(v, "target_lufs") {
            p.target_lufs = t.clamp(-40.0, -5.0);
        }
        if let Some(f) = os(v, "folder") {
            p.folder = f;
        }
        if let Some(k) = os(v, "kind") {
            p.kind = k;
        }
        if let Some(g) = of(v, "master_gain_db") {
            p.master.gain_db = g.clamp(-60.0, 12.0);
        }
        Ok(json!({}))
    })?;
    drop(s);
    project_json(api)
}

fn h_project_delete(api: &Api, v: &Value) -> Result<Value> {
    let id = s_(v, "id")?;
    let mut s = api.lock();
    if s.project.as_ref().map(|p| p.id == id).unwrap_or(false) {
        s.close();
    }
    s.store.trash_project(&id)?;
    s.rev += 1;
    Ok(json!({"trashed": id}))
}

fn h_project_duplicate(api: &Api, v: &Value) -> Result<Value> {
    let id = s_(v, "id")?;
    let s = api.lock();
    let mut p = s.store.load_project(&id)?;
    p.id = new_id();
    p.name = format!("{} copy", p.name);
    let src = s.store.media_dir(&id);
    let dst = s.store.media_dir(&p.id);
    std::fs::create_dir_all(&dst)?;
    for m in &mut p.media {
        let f = Path::new(&m.path);
        if f.starts_with(&src) {
            if let Some(name) = f.file_name() {
                let to = dst.join(name);
                std::fs::copy(f, &to)?;
                m.path = to.display().to_string();
            }
        }
    }
    s.store.save_project(&p)?;
    Ok(json!({"id": p.id, "name": p.name}))
}

fn h_project_export_package(api: &Api, v: &Value) -> Result<Value> {
    let s = api.lock();
    let p = s.project()?.clone();
    let path = match os(v, "path") {
        Some(p) => PathBuf::from(p),
        None => unique_path(
            s.settings
                .export_dir
                .clone()
                .map(PathBuf::from)
                .unwrap_or_else(crate::store::default_export_dir)
                .join(format!("{}.flux", file_name_safe(&p.name))),
        ),
    };
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    let bytes = s.store.export_package(&p, &path)?;
    Ok(json!({"path": path.display().to_string(), "bytes": bytes}))
}

fn h_project_import_package(api: &Api, v: &Value) -> Result<Value> {
    let path = s_(v, "path")?;
    let p = api.lock().store.import_package(Path::new(&path))?;
    let (pool, missing) = crate::api::load_media(&p);
    api.lock().install(p, pool, missing);
    project_json(api)
}

fn h_snapshot_save(api: &Api, v: &Value) -> Result<Value> {
    let s = api.lock();
    let name = os(v, "name").unwrap_or_else(|| "Snapshot".into());
    let id = s.store.save_snapshot(s.project()?, &name)?;
    Ok(json!({"id": id, "name": name}))
}
fn h_snapshot_list(api: &Api, _: &Value) -> Result<Value> {
    let s = api.lock();
    Ok(Value::Array(s.store.snapshots(&s.project()?.id)))
}
fn h_snapshot_restore(api: &Api, v: &Value) -> Result<Value> {
    let id = s_(v, "id")?;
    let mut s = api.lock();
    let pid = s.project()?.id.clone();
    let snap = s.store.load_snapshot(&pid, &id)?;
    s.commit("Restore snapshot", move |p| {
        let keep = (p.id.clone(), p.media.clone());
        *p = snap;
        p.id = keep.0;
        // media must still resolve: keep entries added since the snapshot
        for m in keep.1 {
            if !p.media.iter().any(|x| x.id == m.id) {
                p.media.push(m);
            }
        }
        Ok(json!({}))
    })?;
    drop(s);
    project_json(api)
}

fn h_history(api: &Api, _: &Value) -> Result<Value> {
    let s = api.lock();
    let mut items: Vec<Value> = s
        .undo
        .iter()
        .enumerate()
        .map(|(i, h)| json!({"index": i, "label": h.label, "time": h.time, "state": "past"}))
        .collect();
    items.push(json!({"index": s.undo.len(), "label": "Now", "time": s.project.as_ref().map(|p| p.modified), "state": "current"}));
    for (k, h) in s.redo.iter().rev().enumerate() {
        items.push(json!({"index": s.undo.len() + 1 + k, "label": h.label, "time": h.time, "state": "future"}));
    }
    Ok(json!({"items": items, "current": s.undo.len()}))
}
fn h_undo(api: &Api, _: &Value) -> Result<Value> {
    api.lock().undo()
}
fn h_redo(api: &Api, _: &Value) -> Result<Value> {
    api.lock().redo()
}
fn h_history_goto(api: &Api, v: &Value) -> Result<Value> {
    let target = u_(v, "index")? as usize;
    let mut s = api.lock();
    let mut guard = 0;
    while s.undo.len() > target && guard < 10_000 {
        s.undo()?;
        guard += 1;
    }
    while s.undo.len() < target && !s.redo.is_empty() && guard < 10_000 {
        s.redo()?;
        guard += 1;
    }
    Ok(json!({"current": s.undo.len()}))
}

// ---------------------------------------------------------------- media

fn h_media_import(api: &Api, v: &Value) -> Result<Value> {
    let path = PathBuf::from(s_(v, "path")?);
    if !path.is_file() {
        bail!(err("not_found", "That file doesn't exist"));
    }
    let new_project = api.lock().project.is_none();
    if new_project {
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Imported".into());
        // a project made by importing a file takes the file's rate when it is a standard one
        let rate = crate::media::probe_rate(&path)
            .filter(|r| [44100, 48000, 88200, 96000, 176400, 192000].contains(r))
            .unwrap_or(48000);
        h_project_new(
            api,
            &json!({"name": name, "template": "empty", "sample_rate": rate}),
        )?;
    }
    let (sr, pid) = {
        let s = api.lock();
        let p = s.project()?;
        (p.sample_rate, p.id.clone())
    };
    let at = opos(v, "at", sr);
    let track = ou(v, "track");
    let new_track = ob(v, "new_track").unwrap_or(true);
    let a = api.clone();
    let fname = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let title = format!("Adding {fname}");
    let job = api.jobs.spawn("import", &title, &["Reading audio", "Building waveform", "Saving a safe copy in your project", "Finding tempo and key"], move |ctx| {
        ctx.stage("Reading audio", "active");
        let d = crate::media::decode_file(&path)?;
        ctx.stage("Reading audio", "done");
        if !ctx.progress(0.3) {
            bail!("cancelled");
        }
        ctx.stage("Building waveform", "active");
        let id = format!("m{}", new_id());
        let (rate, chans, fmt, bits, tags, src_frames) = (d.rate, d.channels, d.format.clone(), d.bits, d.tags.clone(), d.data[0].len());
        let media = Media::new(&id, d.data, rate, sr);
        ctx.stage("Building waveform", "done");
        ctx.progress(0.6);
        ctx.stage("Saving a safe copy in your project", "active");
        let copy = a.lock().store.safe_copy(&pid, &id, &path)?;
        ctx.stage("Saving a safe copy in your project", "done");
        ctx.progress(0.75);
        ctx.stage("Finding tempo and key", "active");
        let n = media.frames.min(sr as usize * 90);
        let mono: Vec<f32> = (0..n).map(|i| (media.get(0, i) + media.get(1, i)) * 0.5).collect();
        let (bpm, conf, _) = analysis::tempo(&mono, sr as f32);
        let key = analysis::key(&mono, sr as f32);
        ctx.stage("Finding tempo and key", "done");
        let frames = media.frames as i64;
        let mut s = a.lock();
        if s.project()?.id != pid {
            bail!("the project was closed while importing");
        }
        s.add_media(media);
        let name = fname.clone();
        let first = s.project()?.media.is_empty();
        let clip = s.commit(&format!("Import {name}"), |p| {
            p.media.push(MediaRef { id: id.clone(), name: name.clone(), path: copy.display().to_string(), original: path.display().to_string(), linked: false, frames, channels: chans, source_rate: rate, format: fmt, bits, tags, synthetic: false });
            let tid = match track {
                Some(t) => t,
                None => {
                    let empty = p.tracks.iter().find(|t| t.clips.is_empty()).map(|t| t.id);
                    match empty {
                        Some(t) if !new_track || first => t,
                        _ => {
                            let stem = Path::new(&name).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or(name.clone());
                            let t = p.add_track(&stem);
                            let n = p.tracks.len();
                            p.track_mut(t)?.color = ((n - 1) % 8 + 1) as u8;
                            t
                        }
                    }
                }
            };
            let start = at.unwrap_or(0).max(0);
            let cid = p.add_clip(tid, &id, start, 0, None)?;
            if let Ok(c) = p.clip_mut(cid) {
                c.name = Path::new(&name).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            }
            if first && conf > 0.15 {
                p.tempo = bpm;
            }
            if first && p.key.is_empty() {
                p.key = key.key.clone();
            }
            Ok(json!({"clip": cid, "track": tid}))
        })?;
        Ok(json!({"media": id, "clip": clip["clip"], "track": clip["track"], "seconds": frames as f64 / sr as f64, "source_rate": rate, "channels": chans, "source_frames": src_frames, "tempo": bpm, "tempo_confidence": conf, "key": key.key, "camelot": key.camelot}))
    });
    if ob(v, "wait").unwrap_or(false) {
        let j = api
            .jobs
            .wait(&job, std::time::Duration::from_secs(600))
            .unwrap();
        if j.state == "failed" {
            bail!(err("failed", j.error.unwrap_or_default()));
        }
        return Ok(json!({"job": job, "result": j.result}));
    }
    Ok(json!({"job": job}))
}

fn h_media_list(api: &Api, _: &Value) -> Result<Value> {
    let s = api.lock();
    let p = s.project()?;
    Ok(Value::Array(p.media.iter().map(|m| json!({"id": m.id, "name": m.name, "frames": m.frames, "seconds": m.frames as f64 / p.sample_rate as f64, "channels": m.channels, "source_rate": m.source_rate, "format": m.format, "bits": m.bits, "synthetic": m.synthetic, "missing": s.missing.contains(&m.id), "tags": m.tags})).collect()))
}

fn h_media_peaks(api: &Api, v: &Value) -> Result<Value> {
    let id = s_(v, "media")?;
    let s = api.lock();
    let m = s
        .pool
        .get(&id)
        .cloned()
        .ok_or_else(|| err("not_found", "That media isn't loaded"))?;
    drop(s);
    let start = v.get("start").and_then(|x| x.as_i64()).unwrap_or(0);
    let end = v
        .get("end")
        .and_then(|x| x.as_i64())
        .unwrap_or(m.frames as i64);
    let bins = ou(v, "bins").unwrap_or(800) as usize;
    let pk = m.peaks_range(start, end, bins, ob(v, "reverse").unwrap_or(false));
    // compact: per channel a flat array lo,hi,lo,hi... rounded to 1e-3
    let r = |x: f32| (x * 1000.0).round() / 1000.0;
    Ok(
        json!({"channels": pk.iter().map(|c| c.iter().flat_map(|p| [r(p[0]), r(p[1])]).collect::<Vec<f32>>()).collect::<Vec<_>>(), "bins": bins.clamp(1, 4000)}),
    )
}

fn h_media_relink(api: &Api, v: &Value) -> Result<Value> {
    let id = s_(v, "id")?;
    let path = PathBuf::from(s_(v, "path")?);
    let (sr, pid) = {
        let s = api.lock();
        let p = s.project()?;
        (p.sample_rate, p.id.clone())
    };
    let d = crate::media::decode_file(&path)?;
    let m = Media::new(&id, d.data, d.rate, sr);
    let copy = api.lock().store.safe_copy(&pid, &id, &path)?;
    let mut s = api.lock();
    s.add_media(m);
    s.missing.retain(|x| x != &id);
    s.commit("Relink media", |p| {
        let r = p
            .media
            .iter_mut()
            .find(|m| m.id == id)
            .ok_or_else(|| err("not_found", "no such media"))?;
        r.path = copy.display().to_string();
        r.original = path.display().to_string();
        Ok(json!({}))
    })?;
    s.rev += 1;
    let pr = s.project()?.clone();
    s.sync_engine(None);
    let _ = pr;
    Ok(json!({"relinked": id}))
}

// ---------------------------------------------------------------- tracks, clips, edits

fn h_track_add(api: &Api, v: &Value) -> Result<Value> {
    let mut s = api.lock();
    s.commit("Add track", |p| {
        let n = p.tracks.len();
        let id = p.add_track(&os(v, "name").unwrap_or_else(|| format!("Track {}", n + 1)));
        p.track_mut(id)?.color = (n % 8 + 1) as u8;
        if let Some(i) = ou(v, "index") {
            p.move_track(id, i as usize)?;
        }
        Ok(json!({"track": id}))
    })
}
fn h_track_remove(api: &Api, v: &Value) -> Result<Value> {
    let id = u_(v, "track")?;
    api.lock().commit("Delete track", |p| {
        p.remove_track(id)?;
        Ok(json!({}))
    })
}
fn h_track_set(api: &Api, v: &Value) -> Result<Value> {
    let id = u_(v, "track")?;
    let label = if v.get("mute").is_some() {
        "Mute track"
    } else if v.get("solo").is_some() {
        "Solo track"
    } else if v.get("gain_db").is_some() {
        "Change track level"
    } else if v.get("pan").is_some() {
        "Pan track"
    } else {
        "Change track"
    };
    api.lock().commit(label, |p| {
        let t = p.track_mut(id)?;
        if let Some(n) = os(v, "name") {
            t.name = n.chars().take(120).collect();
        }
        if let Some(g) = of(v, "gain_db") {
            t.gain_db = g.clamp(-96.0, 24.0);
        }
        if let Some(x) = of(v, "pan") {
            t.pan = x.clamp(-1.0, 1.0);
        }
        if let Some(b) = ob(v, "mute") {
            t.mute = b;
        }
        if let Some(b) = ob(v, "solo") {
            t.solo = b;
        }
        if let Some(b) = ob(v, "arm") {
            t.arm = b;
        }
        if let Some(c) = ou(v, "color") {
            t.color = (c.clamp(1, 8)) as u8;
        }
        Ok(json!({}))
    })
}
fn h_track_move(api: &Api, v: &Value) -> Result<Value> {
    let id = u_(v, "track")?;
    let i = u_(v, "index")? as usize;
    api.lock().commit("Move track", |p| {
        p.move_track(id, i)?;
        Ok(json!({}))
    })
}

fn h_clip_move(api: &Api, v: &Value) -> Result<Value> {
    let id = u_(v, "clip")?;
    let sr = sr(api);
    let start = pos(v, "start", sr)?;
    let name = api
        .lock()
        .project()?
        .clip(id)
        .map(|c| c.name.clone())
        .unwrap_or_default();
    api.lock().commit(&format!("Move \"{name}\""), |p| {
        p.move_clip(id, start, ou(v, "track"))?;
        Ok(json!({}))
    })
}
fn h_clip_trim(api: &Api, v: &Value) -> Result<Value> {
    let id = u_(v, "clip")?;
    let sr = sr(api);
    let (a, b) = (opos(v, "start", sr), opos(v, "end", sr));
    api.lock().commit("Trim clip", |p| {
        p.trim_clip(id, a, b)?;
        Ok(json!({}))
    })
}
fn h_clip_split(api: &Api, v: &Value) -> Result<Value> {
    let sr = sr(api);
    let mut s = api.lock();
    let at = match opos(v, "at", sr) {
        Some(x) => x,
        None => s.engine.as_ref().map(|e| e.meters(0).pos).unwrap_or(0),
    };
    let ids: Vec<u32> = match ou(v, "clip") {
        Some(c) => vec![c],
        None => s
            .project()?
            .tracks
            .iter()
            .flat_map(|t| t.clips.iter())
            .filter(|c| c.start < at && c.end() > at)
            .map(|c| c.id)
            .collect(),
    };
    if ids.is_empty() {
        bail!(err("nothing", "There is no clip under that position"));
    }
    let name = s
        .project()?
        .clip(ids[0])
        .map(|c| c.name.clone())
        .unwrap_or_default();
    s.commit(&format!("Split \"{name}\" at {}", fmt_time(at, sr)), |p| {
        let mut out = vec![];
        for id in &ids {
            out.push(p.split_clip(*id, at)?);
        }
        Ok(json!({"new_clips": out}))
    })
}
fn h_clip_delete(api: &Api, v: &Value) -> Result<Value> {
    let ids = clip_ids(api, v)?;
    api.lock().commit("Delete clip", |p| {
        for id in &ids {
            p.delete_clip(*id)?;
        }
        Ok(json!({}))
    })
}
fn clip_ids(api: &Api, v: &Value) -> Result<Vec<u32>> {
    if let Some(c) = ou(v, "clip") {
        return Ok(vec![c]);
    }
    if let Some(a) = v.get("clips").and_then(|x| x.as_array()) {
        return Ok(a
            .iter()
            .filter_map(|x| x.as_u64().map(|n| n as u32))
            .collect());
    }
    let s = api.lock();
    s.project()?
        .selection
        .clip
        .map(|c| vec![c])
        .ok_or_else(|| err("bad_params", "pass `clip`, or select a clip first"))
}
fn h_clip_duplicate(api: &Api, v: &Value) -> Result<Value> {
    let id = u_(v, "clip")?;
    api.lock().commit("Duplicate clip", |p| {
        Ok(json!({"clip": p.duplicate_clip(id)?}))
    })
}
fn h_clip_set(api: &Api, v: &Value) -> Result<Value> {
    let id = u_(v, "clip")?;
    let sr = sr(api);
    let label = if v.get("fade_in").is_some()
        || v.get("fade_in_s").is_some()
        || v.get("fade_out").is_some()
        || v.get("fade_out_s").is_some()
    {
        "Change fade"
    } else if v.get("reverse").is_some() {
        "Reverse clip"
    } else if v.get("gain_db").is_some() {
        "Change clip gain"
    } else {
        "Change clip"
    };
    api.lock().commit(label, |p| {
        let c = p.clip_mut(id)?;
        if let Some(n) = os(v, "name") {
            c.name = n.chars().take(120).collect();
        }
        if let Some(g) = of(v, "gain_db") {
            c.gain_db = g.clamp(-60.0, 24.0);
        }
        if let Some(f) = opos(v, "fade_in", sr) {
            c.fade_in = f.clamp(0, c.len);
        }
        if let Some(f) = opos(v, "fade_out", sr) {
            c.fade_out = f.clamp(0, c.len);
        }
        if let Some(k) = os(v, "curve") {
            if !["equal", "s", "log", "lin"].contains(&k.as_str()) {
                bail!(err("bad_params", "curve must be equal, s, log or lin"));
            }
            c.curve = k;
        }
        if let Some(r) = ob(v, "reverse") {
            c.reverse = r;
        }
        Ok(json!({}))
    })
}
fn h_clip_normalize(api: &Api, v: &Value) -> Result<Value> {
    let id = u_(v, "clip")?;
    let target = of(v, "peak_db").unwrap_or(-1.0);
    let (media, off, len) = {
        let s = api.lock();
        let c = s.project()?.clip(id)?;
        (
            s.pool
                .get(&c.media)
                .cloned()
                .ok_or_else(|| err("missing", "The clip's audio is missing"))?,
            c.offset,
            c.len,
        )
    };
    let mut pk = 0f32;
    for ch in 0..media.data.len() {
        for i in off..off + len {
            pk = pk.max(media.get(ch, i as usize).abs());
        }
    }
    if pk < 1e-6 {
        bail!(err("nothing", "The clip is silent"));
    }
    let g = target - crate::dsp::lin_to_db(pk);
    api.lock().commit("Normalize clip", |p| {
        p.clip_mut(id)?.gain_db = g.clamp(-60.0, 40.0);
        Ok(json!({"gain_db": g}))
    })
}

fn fmt_time(s: i64, sr: u32) -> String {
    let t = s as f64 / sr as f64;
    format!("{}:{:06.3}", (t / 60.0).floor() as i64, t % 60.0)
}

fn h_edit_cut_range(api: &Api, v: &Value) -> Result<Value> {
    let sr = sr(api);
    let (a, b) = (pos(v, "start", sr)?, pos(v, "end", sr)?);
    let tracks: Option<Vec<u32>> = v.get("tracks").and_then(|x| x.as_array()).map(|a| {
        a.iter()
            .filter_map(|x| x.as_u64().map(|n| n as u32))
            .collect()
    });
    let ripple = ob(v, "ripple").unwrap_or(true);
    api.lock().commit("Cut range", |p| {
        p.cut_range(a, b, ripple, tracks.as_deref())?;
        Ok(json!({}))
    })
}
fn h_edit_insert_silence(api: &Api, v: &Value) -> Result<Value> {
    let sr = sr(api);
    let (a, l) = (pos(v, "at", sr)?, pos(v, "len", sr)?);
    api.lock().commit("Insert silence", |p| {
        p.insert_silence(a, l)?;
        Ok(json!({}))
    })
}
fn h_edit_copy(api: &Api, v: &Value) -> Result<Value> {
    let ids = clip_ids(api, v)?;
    let mut s = api.lock();
    let p = s.project()?.clone();
    let mut items = vec![];
    for id in &ids {
        let (ti, ci) = p.clip_loc(*id)?;
        items.push((ti, p.tracks[ti].clips[ci].clone()));
    }
    let origin = items.iter().map(|(_, c)| c.start).min().unwrap_or(0);
    s.clipboard = items;
    s.clip_origin = origin;
    Ok(json!({"copied": ids.len()}))
}
fn h_edit_cut(api: &Api, v: &Value) -> Result<Value> {
    h_edit_copy(api, v)?;
    h_clip_delete(api, v)
}
fn h_edit_paste(api: &Api, v: &Value) -> Result<Value> {
    let sr = sr(api);
    let mut s = api.lock();
    if s.clipboard.is_empty() {
        bail!(err("nothing", "Nothing has been copied"));
    }
    let at = match opos(v, "at", sr) {
        Some(x) => x,
        None => s.engine.as_ref().map(|e| e.meters(0).pos).unwrap_or(0),
    };
    let items = s.clipboard.clone();
    let origin = s.clip_origin;
    let to_track = ou(v, "track");
    s.commit("Paste", |p| {
        let mut out = vec![];
        for (ti, c) in &items {
            let tid = match to_track {
                Some(t) => t,
                None => p
                    .tracks
                    .get(*ti)
                    .map(|t| t.id)
                    .or_else(|| p.tracks.first().map(|t| t.id))
                    .ok_or_else(|| err("empty", "no tracks"))?,
            };
            let mut nc = c.clone();
            nc.id = p.alloc();
            nc.start = (at + c.start - origin).max(0);
            for e in &mut nc.fx {
                e.id = p.alloc();
            }
            out.push(nc.id);
            p.track_mut(tid)?.clips.push(nc);
            p.track_mut(tid)?.clips.sort_by_key(|c| c.start);
        }
        Ok(json!({"clips": out}))
    })
}
fn h_selection_set(api: &Api, v: &Value) -> Result<Value> {
    let sr = sr(api);
    let mut s = api.lock();
    let p = s.project_mut()?;
    if let Some(a) = opos(v, "start", sr) {
        p.selection.start = a.max(0);
    }
    if let Some(b) = opos(v, "end", sr) {
        p.selection.end = b.max(0);
    }
    if v.get("track").is_some() {
        p.selection.track = ou(v, "track");
    }
    if v.get("clip").is_some() {
        p.selection.clip = ou(v, "clip");
        if let Some(c) = p.selection.clip {
            if let Ok((ti, _)) = p.clip_loc(c) {
                p.selection.track = Some(p.tracks[ti].id);
            }
        }
    }
    let sel = p.selection.clone();
    s.rev += 1;
    Ok(serde_json::to_value(sel)?)
}
fn h_marker_add(api: &Api, v: &Value) -> Result<Value> {
    let sr = sr(api);
    let mut s = api.lock();
    let at = match opos(v, "pos", sr) {
        Some(x) => x,
        None => s.engine.as_ref().map(|e| e.meters(0).pos).unwrap_or(0),
    };
    s.commit("Add marker", |p| {
        Ok(json!({"marker": p.add_marker(at, &os(v, "name").unwrap_or_else(|| "Marker".into()))}))
    })
}
fn h_marker_remove(api: &Api, v: &Value) -> Result<Value> {
    let id = u_(v, "marker")?;
    api.lock().commit("Remove marker", |p| {
        let n = p.markers.len();
        p.markers.retain(|m| m.id != id);
        if p.markers.len() == n {
            bail!(err("not_found", "no such marker"));
        }
        Ok(json!({}))
    })
}
fn h_lane_set(api: &Api, v: &Value) -> Result<Value> {
    let tid = u_(v, "track")?;
    let target = s_(v, "target")?;
    let sr = sr(api);
    let pts: Vec<[f64; 2]> = need(v, "points")?
        .as_array()
        .ok_or_else(|| {
            err(
                "bad_params",
                "points must be an array of [sample, value] or {pos|pos_s, value}",
            )
        })?
        .iter()
        .filter_map(|x| {
            if let Some(a) = x.as_array() {
                Some([a.first()?.as_f64()?, a.get(1)?.as_f64()?])
            } else {
                let p = opos(x, "pos", sr)? as f64;
                Some([p, x.get("value")?.as_f64()?])
            }
        })
        .collect();
    api.lock().commit("Edit automation", |p| {
        Ok(json!({"lane": p.set_lane(tid, &target, pts, os(v, "shape"))?}))
    })
}
fn h_lane_remove(api: &Api, v: &Value) -> Result<Value> {
    let tid = u_(v, "track")?;
    let lane = u_(v, "lane")?;
    api.lock().commit("Remove automation lane", |p| {
        let t = p.track_mut(tid)?;
        let n = t.lanes.len();
        t.lanes.retain(|l| l.id != lane);
        if t.lanes.len() == n {
            bail!(err("not_found", "no such lane"));
        }
        Ok(json!({}))
    })
}

fn h_clip_bounce(api: &Api, v: &Value) -> Result<Value> {
    let id = u_(v, "clip")?;
    let (mut p, pool) = api.snapshot()?;
    let (ti, ci) = p.clip_loc(id)?;
    let c = p.tracks[ti].clips[ci].clone();
    // render only this clip with its own effects
    for (i, t) in p.tracks.iter_mut().enumerate() {
        t.fx.clear();
        t.lanes.clear();
        t.gain_db = 0.0;
        t.pan = 0.0;
        t.mute = false;
        t.solo = false;
        t.clips.retain(|x| i == ti && x.id == id);
    }
    p.master.fx.clear();
    p.master.gain_db = 0.0;
    let (l, r) = render::render_project(&p, pool, Some((c.start, c.end())), !c.fx.is_empty());
    let mid = add_generated_media(api, &format!("{} (bounced)", c.name), l, r, false)?;
    let len = api
        .lock()
        .pool
        .get(&mid)
        .map(|m| m.frames as i64)
        .unwrap_or(c.len);
    api.lock().commit(&format!("Bounce \"{}\"", c.name), |p| {
        let cl = p.clip_mut(id)?;
        cl.media = mid.clone();
        cl.offset = 0;
        cl.len = len;
        cl.fx.clear();
        cl.reverse = false;
        cl.gain_db = 0.0;
        cl.fade_in = 0;
        cl.fade_out = 0;
        Ok(json!({"media": mid}))
    })
}

// ---------------------------------------------------------------- effects

fn h_fx_catalog(_: &Api, _: &Value) -> Result<Value> {
    Ok(Value::Array(fx::SPECS.iter().map(spec_json).collect()))
}

fn target_of(api: &Api, v: &Value) -> Result<String> {
    if let Some(t) = os(v, "target") {
        return Ok(t);
    }
    let s = api.lock();
    let p = s.project()?;
    Ok(match p.selection.track.or(p.tracks.first().map(|t| t.id)) {
        Some(t) => format!("track:{t}"),
        None => "master".into(),
    })
}

fn h_fx_add(api: &Api, v: &Value) -> Result<Value> {
    let kind = s_(v, "kind")?;
    let target = target_of(api, v)?;
    let params = fparams(v, "params")?;
    let name = fx::spec(&kind).map(|s| s.name).unwrap_or("effect");
    let mut s = api.lock();
    let label = format!("{name} added to {}", chain_label(s.project()?, &target));
    let r = s.commit(&label, |p| {
        let e = make_effect(p, &kind, &params, of(v, "mix"), os(v, "group"))?;
        let id = insert_effect(p, &target, e, ou(v, "index").map(|i| i as usize))?;
        Ok(json!({"fx": id}))
    })?;
    let k = kind.clone();
    s.settings.recent_fx.retain(|x| x != &k);
    s.settings.recent_fx.insert(0, k);
    s.settings.recent_fx.truncate(12);
    let st = s.settings.clone();
    let _ = s.store.save_settings(&st);
    Ok(r)
}

fn chain_label(p: &Project, target: &str) -> String {
    if target == "master" {
        return "Master".into();
    }
    if let Some(t) = target
        .strip_prefix("track:")
        .and_then(|x| x.parse::<u32>().ok())
    {
        return p
            .track(t)
            .map(|t| t.name.clone())
            .unwrap_or_else(|_| "track".into());
    }
    if let Some(c) = target
        .strip_prefix("clip:")
        .and_then(|x| x.parse::<u32>().ok())
    {
        return p
            .clip(c)
            .map(|c| format!("\"{}\"", c.name))
            .unwrap_or_else(|_| "clip".into());
    }
    target.into()
}

fn h_fx_remove(api: &Api, v: &Value) -> Result<Value> {
    let id = u_(v, "fx")?;
    api.lock().commit("Remove effect", |p| {
        p.remove_fx(id)?;
        Ok(json!({}))
    })
}
fn h_fx_move(api: &Api, v: &Value) -> Result<Value> {
    let id = u_(v, "fx")?;
    let i = u_(v, "index")? as usize;
    api.lock().commit("Reorder effects", |p| {
        p.move_fx(id, i)?;
        Ok(json!({}))
    })
}
fn h_fx_set(api: &Api, v: &Value) -> Result<Value> {
    let id = u_(v, "fx")?;
    let params = fparams(v, "params")?;
    let mut s = api.lock();
    let name = s
        .project()?
        .find_fx(id)
        .and_then(|(_, e)| fx::spec(&e.kind).map(|s| s.name))
        .unwrap_or("effect");
    s.commit(&format!("Change {name}"), |p| {
        if !params.is_empty() {
            p.set_fx_params(id, &params)?;
        }
        let e = p.fx_mut(id)?;
        if let Some(b) = ob(v, "enabled") {
            e.enabled = b;
        }
        if let Some(m) = of(v, "mix") {
            e.mix = m.clamp(0.0, 1.0);
        }
        Ok(json!({"params": e.params, "enabled": e.enabled, "mix": e.mix}))
    })
}
fn h_fx_slot(api: &Api, v: &Value) -> Result<Value> {
    let id = u_(v, "fx")?;
    let n = u_(v, "slot")? as u8;
    api.lock().commit("Switch A/B/C", |p| {
        p.fx_slot(id, n)?;
        Ok(json!({}))
    })
}
fn h_fx_get(api: &Api, v: &Value) -> Result<Value> {
    let id = u_(v, "fx")?;
    let s = api.lock();
    let (target, e) = s
        .project()?
        .find_fx(id)
        .ok_or_else(|| err("not_found", "no such effect"))?;
    let mut o = serde_json::to_value(e)?;
    o["target"] = json!(target);
    if e.kind == "eq" {
        let sp = fx::spec(&e.kind).unwrap();
        let p = fx::resolve(sp, &e.params);
        let sr = s.project()?.sample_rate as f32;
        let freqs: Vec<f32> = (0..200)
            .map(|i| 20.0 * 1000f32.powf(i as f32 / 199.0))
            .collect();
        o["response"] = json!(freqs
            .iter()
            .map(|f| [*f, fx::eq::eq_response(&p, sr, *f)])
            .collect::<Vec<_>>());
    }
    if e.kind == "reverb" {
        o["rt60"] = json!(e.params.get("decay"));
    }
    Ok(o)
}
fn h_chain_get(api: &Api, v: &Value) -> Result<Value> {
    let target = target_of(api, v)?;
    let mut s = api.lock();
    let p = s.project_mut()?;
    let label = chain_label(&p.clone(), &target);
    let chain = p.chain_mut(&target)?.clone();
    Ok(json!({"target": target, "label": label, "effects": chain}))
}
fn h_fx_learn_noise(api: &Api, v: &Value) -> Result<Value> {
    let id = u_(v, "fx")?;
    let sr = sr(api);
    let (a, b) = {
        let s = api.lock();
        let p = s.project()?;
        (
            opos(v, "start", sr).unwrap_or(p.selection.start),
            opos(v, "end", sr).unwrap_or(p.selection.end),
        )
    };
    if b - a < sr as i64 / 10 {
        bail!(err(
            "bad_params",
            "Select at least 0.1 s of noise only, then learn"
        ));
    }
    // learn from the dry signal of the effect's chain input: render the mix without this effect
    let (mut p, pool) = api.snapshot()?;
    if let Ok(e) = p.fx_mut(id) {
        e.enabled = false;
    }
    let (l, r) = render::render_project(&p, pool, Some((a, b)), false);
    let mono: Vec<f32> = l.iter().zip(&r).map(|(x, y)| (x + y) * 0.5).collect();
    let prof = fx::clean::learn_profile(&mono);
    api.lock().commit("Learn noise profile", |p| {
        let e = p.fx_mut(id)?;
        if e.kind != "noise" {
            bail!(err("bad_params", "only noise reduction learns a profile"));
        }
        e.profile = Some(prof);
        Ok(json!({"from": a, "to": b}))
    })
}
fn h_fx_load_ir(api: &Api, v: &Value) -> Result<Value> {
    let id = u_(v, "fx")?;
    let path = PathBuf::from(s_(v, "path")?);
    let (sr, pid) = {
        let s = api.lock();
        let p = s.project()?;
        (p.sample_rate, p.id.clone())
    };
    let d = crate::media::decode_file(&path)?;
    if d.data[0].len() > d.rate as usize * 20 {
        bail!(err(
            "bad_params",
            "Impulse responses can be up to 20 seconds long"
        ));
    }
    let mid = format!("m{}", new_id());
    let m = Media::new(&mid, d.data, d.rate, sr);
    let copy = api.lock().store.safe_copy(&pid, &mid, &path)?;
    let frames = m.frames as i64;
    let mut s = api.lock();
    s.add_media(m);
    let name = path
        .file_name()
        .map(|x| x.to_string_lossy().to_string())
        .unwrap_or_default();
    s.commit("Load impulse response", |p| {
        p.media.push(MediaRef {
            id: mid.clone(),
            name,
            path: copy.display().to_string(),
            frames,
            channels: 2,
            source_rate: d.rate,
            format: d.format.clone(),
            ..Default::default()
        });
        let e = p.fx_mut(id)?;
        if e.kind != "convolver" {
            bail!(err(
                "bad_params",
                "only the impulse-response reverb loads a file"
            ));
        }
        e.ir = Some(mid.clone());
        Ok(json!({"media": mid}))
    })
}
fn h_fx_set_path(api: &Api, v: &Value) -> Result<Value> {
    let id = u_(v, "fx")?;
    let pts: Vec<[f32; 2]> = need(v, "points")?
        .as_array()
        .ok_or_else(|| err("bad_params", "points must be [[x,y],...]"))?
        .iter()
        .filter_map(|x| Some([x.get(0)?.as_f64()? as f32, x.get(1)?.as_f64()? as f32]))
        .collect();
    api.lock().commit("Draw motion path", |p| {
        let e = p.fx_mut(id)?;
        e.path = pts
            .iter()
            .map(|q| [q[0].clamp(-1.0, 1.0), q[1].clamp(-1.0, 1.0)])
            .take(512)
            .collect();
        Ok(json!({}))
    })
}
fn h_fx_motion(api: &Api, v: &Value) -> Result<Value> {
    // where a stage source is at time t (for drawing)
    let id = u_(v, "fx")?;
    let s = api.lock();
    let (_, e) = s
        .project()?
        .find_fx(id)
        .ok_or_else(|| err("not_found", "no such effect"))?;
    let p = fx::resolve(fx::spec("spatial").unwrap(), &e.params);
    let t = of(v, "t").unwrap_or(0.0);
    let (az, d) = fx::spatial::Spatial::motion(&p, &e.path, t);
    Ok(json!({"azimuth": az, "distance": d}))
}

fn h_style_apply(api: &Api, v: &Value) -> Result<Value> {
    let style = s_(v, "style")?;
    let amount = of(v, "amount").unwrap_or(0.5);
    let fxs = style_fx(&style, amount)
        .ok_or_else(|| err("bad_params", format!("unknown style `{style}`")))?;
    let target = os(v, "target").unwrap_or_else(|| "master".into());
    let group = format!("style:{style}");
    let label = format!(
        "{} {}",
        style_name(&style),
        if amount > 0.0 { "on" } else { "off" }
    );
    api.lock().commit(&label, |p| {
        // replace this style's effects so the amount slider can be moved freely
        let mut existing = vec![];
        for t in ["master".to_string()].into_iter().chain(
            p.tracks
                .iter()
                .map(|t| format!("track:{}", t.id))
                .collect::<Vec<_>>(),
        ) {
            let ch = p.chain_mut(&t)?;
            existing.extend(
                ch.iter()
                    .filter(|e| e.group.as_deref() == Some(group.as_str()))
                    .map(|e| e.id),
            );
        }
        let mut ids = vec![];
        if !existing.is_empty() && fxs.len() == existing.len() && ob(v, "replace").unwrap_or(true) {
            for ((kind, params, mix), id) in fxs.iter().zip(&existing) {
                let e = p.fx_mut(*id)?;
                if e.kind != *kind {
                    continue;
                }
                for (k, x) in params {
                    e.params.insert(k.clone(), *x);
                }
                e.mix = *mix;
                e.enabled = amount > 0.0;
                ids.push(*id);
            }
            if ids.len() == existing.len() {
                return Ok(json!({"fx": ids, "group": group}));
            }
        }
        for id in existing {
            p.remove_fx(id)?;
        }
        if amount > 0.0 {
            for (kind, params, mix) in &fxs {
                let e = make_effect(p, kind, params, Some(*mix), Some(group.clone()))?;
                ids.push(insert_effect(p, &target, e, None)?);
            }
        }
        Ok(json!({"fx": ids, "group": group}))
    })
}
fn style_name(s: &str) -> String {
    match s {
        "slowed" => "Slowed + reverb".into(),
        "8d" => "8D".into(),
        "lofi" => "Lo-fi".into(),
        "nightcore" => "Nightcore".into(),
        "cathedral" => "Cathedral".into(),
        "robot" => "Robot".into(),
        _ => fx::spec(s)
            .map(|x| x.name.to_string())
            .unwrap_or_else(|| s.to_string()),
    }
}
fn h_style_list(_: &Api, _: &Value) -> Result<Value> {
    let ids = [
        "slowed",
        "nightcore",
        "8d",
        "lofi",
        "cathedral",
        "robot",
        "reverb",
        "vinyl",
        "cassette",
        "radio",
        "phone",
        "megaphone",
        "walkie",
        "underwater",
        "distortion",
        "bitcrusher",
        "chorus",
        "flanger",
        "phaser",
        "rotary",
        "stutter",
        "tapestop",
        "granular",
        "trancegate",
        "glitch",
        "row10",
        "nextdoor",
    ];
    Ok(
        json!({"styles": ids.iter().map(|i| json!({"id": i, "name": style_name(i)})).collect::<Vec<_>>(), "rooms": ROOMS.iter().map(|r| json!({"name": r.0, "rt60": r.1})).collect::<Vec<_>>(), "voices": VOICES.iter().map(|v| v.0).collect::<Vec<_>>()}),
    )
}

// ---------------------------------------------------------------- transport, devices, recording

fn h_transport_play(api: &Api, v: &Value) -> Result<Value> {
    let sr = sr(api);
    let mut s = api.lock();
    let at = opos(v, "from", sr);
    let e = s.ensure_engine()?;
    if let Some(a) = at {
        e.send(render::Msg::Seek(a));
    }
    e.send(render::Msg::Play(true));
    Ok(json!({"playing": true}))
}
fn h_transport_pause(api: &Api, _: &Value) -> Result<Value> {
    let mut s = api.lock();
    if let Some(e) = s.engine.as_mut() {
        e.send(render::Msg::Play(false));
    }
    Ok(json!({"playing": false}))
}
fn h_transport_stop(api: &Api, _: &Value) -> Result<Value> {
    let mut s = api.lock();
    if let Some(e) = s.engine.as_mut() {
        e.send(render::Msg::Play(false));
        e.send(render::Msg::Seek(0));
    }
    Ok(json!({"playing": false, "pos": 0}))
}
fn h_transport_seek(api: &Api, v: &Value) -> Result<Value> {
    let sr = sr(api);
    let p = pos(v, "pos", sr)?;
    let mut s = api.lock();
    let e = s.ensure_engine()?;
    e.send(render::Msg::Seek(p.max(0)));
    Ok(json!({"pos": p.max(0)}))
}
fn h_transport_loop(api: &Api, v: &Value) -> Result<Value> {
    let sr = sr(api);
    let mut s = api.lock();
    let on = ob(v, "on").unwrap_or(true);
    let range = match (opos(v, "start", sr), opos(v, "end", sr)) {
        (Some(a), Some(b)) if b > a => Some((a, b)),
        _ => {
            let p = s.project()?;
            if p.selection.end > p.selection.start {
                Some((p.selection.start, p.selection.end))
            } else {
                p.loop_range.or(Some((0, p.length())))
            }
        }
    };
    let p = s.project_mut()?;
    p.loop_on = on;
    p.loop_range = range;
    s.rev += 1;
    if let Some(e) = s.engine.as_mut() {
        e.send(render::Msg::Loop(if on { range } else { None }));
    }
    Ok(json!({"on": on, "range": range}))
}
fn h_transport_hold(api: &Api, v: &Value) -> Result<Value> {
    let on = ob(v, "on").unwrap_or(true);
    let mut s = api.lock();
    s.hold = on;
    if let Some(e) = s.engine.as_mut() {
        e.send(render::Msg::Hold(on));
    }
    Ok(json!({"hold": on}))
}
fn h_transport_state(api: &Api, _: &Value) -> Result<Value> {
    let s = api.lock();
    let n = s.project.as_ref().map(|p| p.tracks.len()).unwrap_or(0);
    let sr = s.project.as_ref().map(|p| p.sample_rate).unwrap_or(48000);
    let m = s.engine.as_ref().map(|e| e.meters(n)).unwrap_or_default();
    let rec = s.engine.as_ref().and_then(|e| e.recording());
    Ok(json!({
        "rev": s.rev, "pos": m.pos, "seconds": m.pos as f64 / sr as f64, "playing": m.playing, "peak": m.peak,
        "lufs_m": m.lufs_m, "lufs_s": m.lufs_s, "lufs_i": m.lufs_i, "glitches": m.glitches, "load": m.load, "tracks": m.tracks,
        "input_peak": m.input_peak, "recording": rec, "jobs": api.jobs.active(), "hold": s.hold,
        "project": s.project.as_ref().map(|p| p.id.clone()), "engine": s.engine.as_ref().map(|e| json!({"backend": e.backend, "device": e.device, "sample_rate": e.sample_rate, "buffer": e.buffer, "error": e.error})),
        "error": s.last_error,
    }))
}
fn h_transport_scope(api: &Api, v: &Value) -> Result<Value> {
    let n = ou(v, "samples").unwrap_or(1024).clamp(64, 4096) as usize;
    let bins = ou(v, "bins").unwrap_or(64).clamp(8, 256) as usize;
    let s = api.lock();
    let sr = s.engine.as_ref().map(|e| e.sample_rate).unwrap_or(48000) as f32;
    let (l, r) = s
        .engine
        .as_ref()
        .map(|e| e.scope(n))
        .unwrap_or((vec![0.0; n], vec![0.0; n]));
    drop(s);
    let mono: Vec<f32> = l.iter().zip(&r).map(|(a, b)| (a + b) * 0.5).collect();
    let spec = analysis::spectrum(&mono, sr, bins);
    let (mut lr, mut ll, mut rr) = (0f32, 0f32, 0f32);
    for i in 0..n {
        lr += l[i] * r[i];
        ll += l[i] * l[i];
        rr += r[i] * r[i];
    }
    let corr = if ll * rr > 1e-12 {
        lr / (ll * rr).sqrt()
    } else {
        0.0
    };
    let step = (n / 256).max(1);
    let rnd = |x: f32| (x * 1000.0).round() / 1000.0;
    Ok(
        json!({"spectrum": spec.iter().map(|x| (x * 10.0).round() / 10.0).collect::<Vec<_>>(), "correlation": corr, "l": l.iter().step_by(step).map(|x| rnd(*x)).collect::<Vec<_>>(), "r": r.iter().step_by(step).map(|x| rnd(*x)).collect::<Vec<_>>()}),
    )
}

fn h_audio_devices(api: &Api, _: &Value) -> Result<Value> {
    let d = crate::device::list_devices();
    let s = api.lock();
    let mut v = serde_json::to_value(d)?;
    v["selected"] = json!({"output": s.settings.output, "input": s.settings.input, "buffer": s.settings.buffer});
    v["engine"] = json!(s.engine.as_ref().map(|e| json!({"backend": e.backend, "device": e.device, "sample_rate": e.sample_rate, "buffer": e.buffer, "error": e.error})));
    Ok(v)
}
fn h_audio_test_tone(api: &Api, v: &Value) -> Result<Value> {
    let kind = os(v, "kind").unwrap_or_else(|| "left".into());
    if !["left", "right", "moving"].contains(&kind.as_str()) {
        bail!(err("bad_params", "kind must be left, right or moving"));
    }
    // a temporary project that plays the tone, then the real plan is restored
    let mut s = api.lock();
    let created = s.project.is_none();
    if created {
        bail!(err(
            "no_project",
            "Open a project first; the test tone plays through its output"
        ));
    }
    let sr = s.project()?.sample_rate;
    let (l, r) = tools::test_tone(&kind, sr as f32);
    let mut tp = Project::new("tone", sr);
    let t = tp.add_track("tone");
    let mut pool = crate::media::Pool::new();
    let n = l.len();
    pool.insert(
        "tone".into(),
        Arc::new(Media::new("tone", vec![l, r], sr, sr)),
    );
    tp.media.push(MediaRef {
        id: "tone".into(),
        frames: n as i64,
        ..Default::default()
    });
    tp.add_clip(t, "tone", 0, 0, None)?;
    let e = s.ensure_engine()?;
    e.set_plan(render::Plan::build(&tp, Arc::new(pool)));
    e.send(render::Msg::Seek(0));
    e.send(render::Msg::Play(true));
    drop(s);
    let a = api.clone();
    let ms = (n as f64 / sr as f64 * 1000.0) as u64 + 150;
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(ms));
        let mut s = a.lock();
        if let Some(e) = s.engine.as_mut() {
            e.send(render::Msg::Play(false));
            e.send(render::Msg::Seek(0));
        }
        s.sync_engine(None);
    });
    Ok(json!({"playing": kind, "seconds": n as f64 / sr as f64}))
}

fn h_record_start(api: &Api, v: &Value) -> Result<Value> {
    let mut s = api.lock();
    let pid = s.project()?.id.clone();
    let id = format!("m{}", new_id());
    let dir = s.store.media_dir(&pid);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{id}.wav"));
    let mut prefs = s.prefs();
    if let Some(i) = os(v, "input") {
        prefs.input = Some(i);
    }
    let at = s.engine.as_ref().map(|e| e.meters(0).pos).unwrap_or(0);
    let monitor = ob(v, "monitor").unwrap_or(false);
    let e = s.ensure_engine()?;
    e.start_recording(&path, &prefs)
        .map_err(|e| err("device", format!("{e:#}")))?;
    e.set_monitor(monitor);
    s.rec = Some(RecCtx {
        media_id: id.clone(),
        path: path.clone(),
        track: ou(v, "track"),
        at,
    });
    s.rev += 1;
    Ok(json!({"recording": true, "media": id}))
}
fn h_record_pause(api: &Api, v: &Value) -> Result<Value> {
    let mut s = api.lock();
    let paused = ob(v, "paused").unwrap_or(true);
    if let Some(e) = s.engine.as_mut() {
        e.pause_recording(paused);
    }
    Ok(json!({"paused": paused}))
}
fn h_record_stop(api: &Api, v: &Value) -> Result<Value> {
    let discard = ob(v, "discard").unwrap_or(false);
    let (res, ctx, sr) = {
        let mut s = api.lock();
        let ctx = s
            .rec
            .take()
            .ok_or_else(|| err("nothing", "Not recording"))?;
        let sr = s.project()?.sample_rate;
        let res = s.engine.as_mut().and_then(|e| {
            e.set_monitor(false);
            e.stop_recording()
        });
        (res, ctx, sr)
    };
    let res = res.ok_or_else(|| err("nothing", "Not recording"))?;
    if discard || res.frames == 0 {
        let _ = std::fs::remove_file(&ctx.path);
        api.lock().rev += 1;
        return Ok(json!({"discarded": true}));
    }
    let d = crate::media::decode_file(&ctx.path)?;
    let m = Media::new(&ctx.media_id, d.data, d.rate, sr);
    let frames = m.frames as i64;
    let mut s = api.lock();
    s.add_media(m);
    let n = s
        .project()?
        .media
        .iter()
        .filter(|m| m.name.starts_with("Recording"))
        .count()
        + 1;
    let r = s.commit("Record take", |p| {
        p.media.push(MediaRef {
            id: ctx.media_id.clone(),
            name: format!("Recording {n}"),
            path: ctx.path.display().to_string(),
            frames,
            channels: res.channels,
            source_rate: res.sample_rate,
            format: "wav".into(),
            bits: 32,
            ..Default::default()
        });
        let tid = match ctx.track.filter(|t| p.track(*t).is_ok()) {
            Some(t) => t,
            None => {
                let c = p.tracks.len();
                let t = p.add_track(&format!("Recording {n}"));
                p.track_mut(t)?.color = (c % 8 + 1) as u8;
                t
            }
        };
        let cid = p.add_clip(tid, &ctx.media_id, ctx.at, 0, None)?;
        p.clip_mut(cid)?.name = format!("Take {n}");
        Ok(json!({"clip": cid, "track": tid, "seconds": res.seconds}))
    })?;
    Ok(r)
}

// ---------------------------------------------------------------- analysis

fn range_of(api: &Api, v: &Value) -> Result<Option<(i64, i64)>> {
    let sr = sr(api);
    match os(v, "range").as_deref() {
        Some("selection") => {
            let s = api.lock();
            let p = s.project()?;
            if p.selection.end <= p.selection.start {
                bail!(err("bad_params", "Nothing is selected"));
            }
            Ok(Some((p.selection.start, p.selection.end)))
        }
        Some("loop") => Ok(api.lock().project()?.loop_range),
        _ => Ok(match (opos(v, "start", sr), opos(v, "end", sr)) {
            (Some(a), Some(b)) if b > a => Some((a, b)),
            _ => None,
        }),
    }
}

fn h_analysis_loudness(api: &Api, v: &Value) -> Result<Value> {
    let range = range_of(api, v)?;
    let mix = api.render_mix(range)?;
    let sr = sr(api) as f32;
    let m = analysis::loudness(&mix.0, &mix.1, sr);
    let pr = analysis::problems(&mix.0, &mix.1, sr);
    Ok(
        json!({"loudness": m, "correlation": pr.correlation, "width": pr.width, "mono_safe": pr.correlation > 0.2}),
    )
}
fn h_analysis_tempo_key(api: &Api, v: &Value) -> Result<Value> {
    let mix = api.render_mix(range_of(api, v)?)?;
    let sr = sr(api) as f32;
    let mono: Vec<f32> = mix
        .0
        .iter()
        .zip(&mix.1)
        .map(|(a, b)| (a + b) * 0.5)
        .collect();
    let (bpm, conf, first) = analysis::tempo(&mono, sr);
    let k = analysis::key(&mono, sr);
    if ob(v, "apply").unwrap_or(false) {
        let kk = k.key.clone();
        api.lock().commit("Set tempo and key", |p| {
            p.tempo = bpm;
            p.key = kk;
            Ok(json!({}))
        })?;
    }
    Ok(json!({"tempo": bpm, "confidence": conf, "first_beat": first, "key": k}))
}
fn h_analysis_problems(api: &Api, v: &Value) -> Result<Value> {
    let mix = api.render_mix(range_of(api, v)?)?;
    Ok(serde_json::to_value(analysis::problems(
        &mix.0,
        &mix.1,
        sr(api) as f32,
    ))?)
}
fn h_analysis_spectrum(api: &Api, v: &Value) -> Result<Value> {
    let mix = api.render_mix(range_of(api, v)?)?;
    let mono: Vec<f32> = mix
        .0
        .iter()
        .zip(&mix.1)
        .map(|(a, b)| (a + b) * 0.5)
        .collect();
    let bins = ou(v, "bins").unwrap_or(64).clamp(8, 512) as usize;
    Ok(json!({"bins": analysis::spectrum(&mono, sr(api) as f32, bins)}))
}

fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        s.push(T[(n >> 18) as usize & 63] as char);
        s.push(T[(n >> 12) as usize & 63] as char);
        s.push(if c.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        s.push(if c.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    s
}

fn h_analysis_spectrogram(api: &Api, v: &Value) -> Result<Value> {
    let cols = ou(v, "cols").unwrap_or(600) as usize;
    let rows = ou(v, "rows").unwrap_or(128) as usize;
    let win = ou(v, "window").unwrap_or(4096) as usize;
    let sr = sr(api);
    let mono: Vec<f32> = if let Some(mid) = os(v, "media") {
        let m = api
            .lock()
            .pool
            .get(&mid)
            .cloned()
            .ok_or_else(|| err("not_found", "no such media"))?;
        let a = opos(v, "start", sr).unwrap_or(0).max(0) as usize;
        let b = (opos(v, "end", sr).unwrap_or(m.frames as i64).max(0) as usize).min(m.frames);
        (a..b).map(|i| (m.get(0, i) + m.get(1, i)) * 0.5).collect()
    } else {
        let mix = api.render_mix(range_of(api, v)?)?;
        mix.0
            .iter()
            .zip(&mix.1)
            .map(|(a, b)| (a + b) * 0.5)
            .collect()
    };
    let g = analysis::spectrogram(&mono, sr as f32, cols, rows, win);
    Ok(
        json!({"cols": cols.clamp(1, 2000), "rows": rows.clamp(8, 512), "data": b64(&g), "max_hz": (sr as f32 * 0.5).min(20000.0), "min_hz": 20}),
    )
}

// ---------------------------------------------------------------- tools

fn source_of(api: &Api, v: &Value) -> Result<(crate::api::Stereo, Option<u32>)> {
    // a track's dry mixdown (clips only), or the whole mix
    if let Some(tid) = ou(v, "track") {
        let (mut p, pool) = api.snapshot()?;
        p.track(tid)?;
        for t in &mut p.tracks {
            if t.id != tid {
                t.clips.clear();
            }
            t.fx.clear();
            t.lanes.clear();
            t.gain_db = 0.0;
            t.pan = 0.0;
            t.mute = false;
            t.solo = false;
        }
        p.master.fx.clear();
        p.master.gain_db = 0.0;
        let len = p.length();
        return Ok((
            render::render_project(&p, pool, Some((0, len)), false),
            Some(tid),
        ));
    }
    let m = api.render_mix(Some((0, api.snapshot()?.0.length())))?;
    Ok(((m.0.clone(), m.1.clone()), None))
}

fn h_stems(api: &Api, v: &Value) -> Result<Value> {
    let mode = os(v, "mode").unwrap_or_else(|| "4".into());
    if mode != "4" && mode != "2" {
        bail!(err(
            "bad_params",
            "mode must be \"4\" (vocals, drums, bass, other) or \"2\" (vocals, instrumental)"
        ));
    }
    if api.lock().project()?.length() == 0 {
        bail!(err("empty", "Add audio before splitting into stems"));
    }
    let a = api.clone();
    let vv = v.clone();
    let job = api.jobs.spawn(
        "stems",
        "Splitting into stems",
        &["Preparing", "Separating", "Saving stems"],
        move |ctx| {
            ctx.stage("Preparing", "active");
            let ((l, r), src) = source_of(&a, &vv)?;
            ctx.stage("Preparing", "done");
            ctx.stage("Separating", "active");
            let sr = sr(&a) as f32;
            let res = tools::stems(&l, &r, sr, &mode, &mut |p| ctx.progress(0.05 + p * 0.8))
                .ok_or_else(|| anyhow::anyhow!("cancelled"))?;
            ctx.stage("Separating", "done");
            ctx.stage("Saving stems", "active");
            let mut ids = vec![];
            for (name, (sl, sr_)) in res {
                let level = analysis::loudness(&sl, &sr_, sr).integrated;
                let mid = add_generated_media(&a, &format!("{name} stem"), sl, sr_, false)?;
                ids.push((name, mid, level));
            }
            let colors = [4u8, 3, 2, 5];
            let out = a.lock().commit("Split into stems", |p| {
                let mut tracks = vec![];
                if let Some(t) = src {
                    p.track_mut(t)?.mute = true;
                } else {
                    for t in &mut p.tracks {
                        t.mute = true;
                    }
                }
                for (i, (name, mid, level)) in ids.iter().enumerate() {
                    let t = p.add_track(name);
                    p.track_mut(t)?.color = colors[i % 4];
                    let c = p.add_clip(t, mid, 0, 0, None)?;
                    p.clip_mut(c)?.name = format!("{name} stem");
                    tracks.push(
                        json!({"name": name, "track": t, "clip": c, "media": mid, "lufs": level}),
                    );
                }
                Ok(json!({"stems": tracks, "method": "dsp-spectral-split"}))
            })?;
            Ok(out)
        },
    );
    Ok(json!({"job": job}))
}

fn apply_plans(
    p: &mut Project,
    target: &str,
    plans: &[tools::FxPlan],
    group: &str,
    profile: Option<Vec<f32>>,
) -> Result<Vec<u32>> {
    // remove effects from an earlier run of the same tool
    for t in ["master".to_string()].into_iter().chain(
        p.tracks
            .iter()
            .map(|t| format!("track:{}", t.id))
            .collect::<Vec<_>>(),
    ) {
        p.chain_mut(&t)?
            .retain(|e| e.group.as_deref() != Some(group));
    }
    let mut ids = vec![];
    for fp in plans {
        let mut e = make_effect(
            p,
            &fp.kind,
            &fp.params,
            Some(fp.mix),
            Some(group.to_string()),
        )?;
        if e.kind == "noise" {
            e.profile = profile.clone();
        }
        ids.push(insert_effect(p, target, e, None)?);
    }
    Ok(ids)
}

fn h_master_analyze(api: &Api, v: &Value) -> Result<Value> {
    let ((l, r), _) = source_of(api, &json!({}))?;
    let target = of(v, "target").unwrap_or_else(|| {
        api.lock()
            .project
            .as_ref()
            .map(|p| p.target_lufs)
            .unwrap_or(-14.0)
    });
    let (chain, rep) = tools::master_chain(
        &l,
        &r,
        sr(api) as f32,
        target,
        &os(v, "style").unwrap_or_else(|| "balanced".into()),
        of(v, "amount").unwrap_or(0.7),
    );
    Ok(json!({"chain": chain, "report": rep}))
}
fn h_master_apply(api: &Api, v: &Value) -> Result<Value> {
    // measure the mix without a previous auto-master chain
    let (mut p, pool) = api.snapshot()?;
    p.master.fx.retain(|e| e.group.as_deref() != Some("master"));
    let len = p.length();
    if len == 0 {
        bail!(err("empty", "Add audio before mastering"));
    }
    let (l, r) = render::render_project(&p, pool.clone(), Some((0, len)), false);
    let target = of(v, "target").unwrap_or(p.target_lufs);
    let style = os(v, "style").unwrap_or_else(|| "balanced".into());
    let (mut chain, rep) = tools::master_chain(
        &l,
        &r,
        p.sample_rate as f32,
        target,
        &style,
        of(v, "amount").unwrap_or(0.7),
    );
    // render the candidate chain and trim the limiter input until it lands on the target
    let mut m = None;
    for _ in 0..5 {
        let mut q = p.clone();
        apply_plans(&mut q, "master", &chain, "master", None)?;
        let (al, ar) = render::render_project(&q, pool.clone(), Some((0, len)), false);
        let lm = analysis::loudness(&al, &ar, p.sample_rate as f32);
        let delta = (target as f64 - lm.integrated) as f32;
        m = Some(lm);
        let Some(lim) = chain.iter_mut().rev().find(|f| f.kind == "limiter") else {
            break;
        };
        if delta.abs() < 0.25 || !delta.is_finite() {
            break;
        }
        let cur = lim.params.get("input").copied().unwrap_or(0.0);
        let next = (cur + delta).clamp(-24.0, 24.0);
        if (next - cur).abs() < 0.05 {
            break;
        }
        lim.params.insert("input".into(), next);
    }
    let ids = api.lock().commit(&format!("Auto-master ({style})"), |p| {
        p.target_lufs = target;
        Ok(json!(apply_plans(p, "master", &chain, "master", None)?))
    })?;
    Ok(json!({"fx": ids, "before": rep, "after": m}))
}
fn h_magic_scan(api: &Api, _: &Value) -> Result<Value> {
    let (mut p, pool) = api.snapshot()?;
    for t in ["master".to_string()].into_iter().chain(
        p.tracks
            .iter()
            .map(|t| format!("track:{}", t.id))
            .collect::<Vec<_>>(),
    ) {
        p.chain_mut(&t)?
            .retain(|e| !e.group.as_deref().unwrap_or("").starts_with("magic:"));
    }
    let len = p.length();
    if len == 0 {
        bail!(err("empty", "Add audio first"));
    }
    let (l, r) = render::render_project(&p, pool, Some((0, len)), false);
    let (fixes, probs, m) = tools::magic(&l, &r, p.sample_rate as f32, p.target_lufs);
    api.lock().magic = fixes.clone();
    Ok(json!({"fixes": fixes, "problems": probs, "loudness": m}))
}
fn h_magic_apply(api: &Api, v: &Value) -> Result<Value> {
    let want: Option<Vec<String>> = v.get("ids").and_then(|x| x.as_array()).map(|a| {
        a.iter()
            .filter_map(|x| x.as_str().map(String::from))
            .collect()
    });
    let fixes = {
        let s = api.lock();
        if s.magic.is_empty() {
            drop(s);
            h_magic_scan(api, &json!({}))?;
            api.lock().magic.clone()
        } else {
            s.magic.clone()
        }
    };
    let chosen: Vec<_> = fixes
        .iter()
        .filter(|f| want.as_ref().map(|w| w.contains(&f.id)).unwrap_or(f.on))
        .cloned()
        .collect();
    if chosen.is_empty() {
        return Ok(json!({"applied": []}));
    }
    // a learned noise profile for noise reduction
    let profile = if chosen.iter().any(|f| f.id == "noise") {
        let ((l, r), _) = source_of(api, &json!({}))?;
        tools::cleanup_chain(&l, &r, sr(api) as f32, 0.5).1
    } else {
        None
    };
    let applied: Vec<String> = chosen.iter().map(|f| f.id.clone()).collect();
    api.lock().commit("Magic fixes", |p| {
        for f in &chosen {
            apply_plans(
                p,
                "master",
                &f.fx,
                &format!("magic:{}", f.id),
                profile.clone(),
            )?;
        }
        // keep any limiter last
        let ch = p.chain_mut("master")?;
        ch.sort_by_key(|e| e.kind == "limiter");
        Ok(json!({"applied": applied}))
    })
}
fn h_cleanup(api: &Api, v: &Value) -> Result<Value> {
    let ((l, r), src) = source_of(api, v)?;
    let strength = of(v, "strength").unwrap_or(0.6);
    let (chain, profile, rep) = tools::cleanup_chain(&l, &r, sr(api) as f32, strength);
    let target = match src {
        Some(t) => format!("track:{t}"),
        None => "master".into(),
    };
    let ids = api.lock().commit("Clean up voice", |p| {
        Ok(json!(apply_plans(
            p,
            &target,
            &chain,
            "cleanup",
            profile.clone()
        )?))
    })?;
    Ok(json!({"fx": ids, "report": rep, "target": target}))
}
fn h_silence_scan(api: &Api, v: &Value) -> Result<Value> {
    let ((l, r), _) = source_of(api, &json!({}))?;
    let sr = sr(api);
    let cuts = tools::silence_cuts(
        &l,
        &r,
        sr as f32,
        of(v, "threshold_db").unwrap_or(-42.0),
        of(v, "longer_than_s").unwrap_or(0.6),
        of(v, "shorten_to_s").unwrap_or(0.3),
        of(v, "padding_s").unwrap_or(0.15),
    );
    let removed: i64 = cuts.iter().map(|c| c.3 - c.2).sum();
    Ok(
        json!({"pauses": cuts.iter().enumerate().map(|(i, c)| json!({"index": i, "start": c.0, "end": c.1, "cut_start": c.2, "cut_end": c.3, "seconds": (c.1 - c.0) as f64 / sr as f64})).collect::<Vec<_>>(), "removed_s": removed as f64 / sr as f64, "length_s": l.len() as f64 / sr as f64}),
    )
}
fn h_silence_apply(api: &Api, v: &Value) -> Result<Value> {
    let scan = h_silence_scan(api, v)?;
    let skip: Vec<u64> = v
        .get("keep")
        .and_then(|x| x.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_u64()).collect())
        .unwrap_or_default();
    let mut cuts: Vec<(i64, i64)> = scan["pauses"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| !skip.contains(&p["index"].as_u64().unwrap_or(0)))
        .map(|p| {
            (
                p["cut_start"].as_i64().unwrap(),
                p["cut_end"].as_i64().unwrap(),
            )
        })
        .collect();
    cuts.sort_by_key(|c| std::cmp::Reverse(c.0));
    let n = cuts.len();
    api.lock().commit(&format!("Remove {n} pauses"), |p| {
        for (a, b) in &cuts {
            p.cut_range(*a, *b, true, None)?;
        }
        Ok(json!({"removed": n, "removed_s": scan["removed_s"]}))
    })
}
fn h_spectral_repair(api: &Api, v: &Value) -> Result<Value> {
    let sr = sr(api);
    let (a, b) = (pos(v, "start", sr)?, pos(v, "end", sr)?);
    let (f_lo, f_hi) = (f_(v, "low_hz")?, f_(v, "high_hz")?);
    let amount = of(v, "amount_db").unwrap_or(30.0);
    let fill = os(v, "fill").unwrap_or_else(|| "both".into());
    let (cid, media, off, len, name) = {
        let s = api.lock();
        let p = s.project()?;
        let tid = track_target(p, v)?;
        let c = match ou(v, "clip") {
            Some(c) => p.clip(c)?.clone(),
            None => p
                .track(tid)?
                .clips
                .iter()
                .find(|c| c.start < b && c.end() > a)
                .cloned()
                .ok_or_else(|| err("nothing", "There is no clip in that range"))?,
        };
        (
            c.id,
            s.pool
                .get(&c.media)
                .cloned()
                .ok_or_else(|| err("missing", "The clip's audio is missing"))?,
            c.offset,
            c.len,
            c.name.clone(),
        )
    };
    let (cs, _) = api.lock().project()?.clip(cid).map(|c| (c.start, c.len))?;
    let (ra, rb) = (
        (a - cs).max(0) as usize,
        ((b - cs).max(0) as usize).min(len as usize),
    );
    let chans: Vec<Vec<f32>> = (0..2)
        .map(|ch| {
            (off..off + len)
                .map(|i| media.get(ch, i as usize))
                .collect()
        })
        .collect();
    let rep: Vec<Vec<f32>> = chans
        .iter()
        .map(|c| {
            tools::spectral_repair(
                c,
                sr as f32,
                tools::Region {
                    f_lo,
                    f_hi,
                    start: ra,
                    end: rb,
                },
                amount,
                &fill,
            )
        })
        .collect();
    let mut it = rep.into_iter();
    let (l, r) = (it.next().unwrap(), it.next().unwrap());
    let mid = add_generated_media(api, &format!("{name} (repaired)"), l, r, false)?;
    api.lock().commit("Spectral repair", |p| {
        let c = p.clip_mut(cid)?;
        c.media = mid.clone();
        c.offset = 0;
        Ok(json!({"clip": cid, "media": mid}))
    })
}
fn h_match_eq(api: &Api, v: &Value) -> Result<Value> {
    let path = PathBuf::from(s_(v, "path")?);
    let d = crate::media::decode_file(&path)?;
    let sr = sr(api);
    let reff: Vec<f32> = crate::dsp::resample(
        &d.data
            .iter()
            .fold(vec![0f32; d.data[0].len()], |mut acc, c| {
                for (a, x) in acc.iter_mut().zip(c) {
                    *a += x / d.data.len() as f32;
                }
                acc
            }),
        d.rate,
        sr,
    );
    let ((l, r), src) = source_of(api, v)?;
    let mono: Vec<f32> = l.iter().zip(&r).map(|(a, b)| (a + b) * 0.5).collect();
    let gains = tools::match_eq(&mono, &reff, sr as f32);
    let target = src
        .map(|t| format!("track:{t}"))
        .unwrap_or_else(|| "master".into());
    let g2 = gains.clone();
    let ids = api.lock().commit("Match EQ", |p| {
        Ok(json!(apply_plans(
            p,
            &target,
            &[tools::FxPlan {
                kind: "geq".into(),
                params: g2.clone(),
                mix: 1.0
            }],
            "match",
            None
        )?))
    })?;
    Ok(json!({"fx": ids, "gains": gains}))
}
fn h_reverse_swell(api: &Api, v: &Value) -> Result<Value> {
    let cid = u_(v, "clip")?;
    let beats = of(v, "beats").unwrap_or(2.0).clamp(0.25, 16.0);
    let (c, media, tempo, sr, ti) = {
        let s = api.lock();
        let p = s.project()?;
        let c = p.clip(cid)?.clone();
        let (ti, _) = p.clip_loc(cid)?;
        (
            c.clone(),
            s.pool
                .get(&c.media)
                .cloned()
                .ok_or_else(|| err("missing", "The clip's audio is missing"))?,
            p.tempo,
            p.sample_rate,
            ti,
        )
    };
    let len = ((beats * 60.0 / tempo) * sr as f32) as usize;
    let l: Vec<f32> = (0..len.min(c.len as usize))
        .map(|i| media.get(0, (c.offset + i as i64) as usize))
        .collect();
    let r: Vec<f32> = (0..len.min(c.len as usize))
        .map(|i| media.get(1, (c.offset + i as i64) as usize))
        .collect();
    let (wl, wr) = tools::reverse_swell(&l, &r, sr as f32, len);
    let out_len = wl.len() as i64;
    let mid = add_generated_media(api, &format!("{} swell", c.name), wl, wr, false)?;
    api.lock().commit("Reverse reverb swell", |p| {
        let start = c.start + l.len() as i64 - out_len;
        let tname = format!("{} swell", p.tracks[ti].name);
        let t = p.add_track(&tname);
        let cl = p.add_clip(t, &mid, start.max(0), (-start).max(0), None)?;
        p.clip_mut(cl)?.name = "Swell".into();
        p.move_track(t, ti + 1)?;
        Ok(json!({"track": t, "clip": cl}))
    })
}

fn h_tts_voices(_: &Api, _: &Value) -> Result<Value> {
    Ok(
        json!({"voices": tools::voices(), "note": "System voices on this computer. Everything they make is labelled as synthetic speech."}),
    )
}
fn h_tts_speak(api: &Api, v: &Value) -> Result<Value> {
    let text = s_(v, "text")?;
    let voice = os(v, "voice").unwrap_or_default();
    let rate = v.get("rate").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
    let sr = sr(api);
    let at = opos(v, "at", sr);
    let (pid, dir) = {
        let s = api.lock();
        let p = s.project()?;
        (p.id.clone(), s.store.media_dir(&p.id))
    };
    let a = api.clone();
    let job = api.jobs.spawn("tts", "Generating speech", &["Synthesizing", "Adding to project"], move |ctx| {
        ctx.stage("Synthesizing", "active");
        let id = format!("m{}", new_id());
        let path = dir.join(format!("{id}.wav"));
        tools::speak(&text, &voice, rate, &path)?;
        ctx.stage("Synthesizing", "done");
        ctx.stage("Adding to project", "active");
        let d = crate::media::decode_file(&path)?;
        let m = Media::new(&id, d.data, d.rate, sr);
        let frames = m.frames as i64;
        let mut s = a.lock();
        if s.project()?.id != pid {
            bail!("the project was closed");
        }
        s.add_media(m);
        let pos = at.unwrap_or_else(|| s.engine.as_ref().map(|e| e.meters(0).pos).unwrap_or(0));
        let short: String = text.chars().take(40).collect();
        s.commit("Generate speech", |p| {
            p.media.push(MediaRef { id: id.clone(), name: format!("Synthetic speech: {short}"), path: path.display().to_string(), frames, channels: 1, source_rate: d.rate, format: "wav".into(), bits: 16, synthetic: true, tags: [("synthetic".to_string(), voice.clone())].into_iter().collect(), ..Default::default() });
            let tid = match p.tracks.iter().find(|t| t.name == "Generated speech") {
                Some(t) => t.id,
                None => {
                    let c = p.tracks.len();
                    let t = p.add_track("Generated speech");
                    p.track_mut(t)?.color = (c % 8 + 1) as u8;
                    t
                }
            };
            let cid = p.add_clip(tid, &id, pos, 0, None)?;
            p.clip_mut(cid)?.name = format!("Synthetic · {short}");
            Ok(json!({"clip": cid, "track": tid, "seconds": frames as f64 / sr as f64, "synthetic": true}))
        })
    });
    Ok(json!({"job": job}))
}

// ---------------------------------------------------------------- export, convert, batch, ringtone

fn opts_of(v: &Value) -> Result<ExportOpts> {
    let mut o = ExportOpts::default();
    if let Some(x) = v.get("options") {
        o = serde_json::from_value(x.clone())
            .map_err(|e| err("bad_params", format!("bad export options: {e}")))?;
    }
    if let Some(f) = os(v, "format") {
        o.format = f;
    }
    let fmts = export::formats();
    let f = fmts.iter().find(|x| x.id == o.format).ok_or_else(|| {
        err(
            "bad_params",
            format!(
                "unknown format {}. Use one of: {}",
                o.format,
                fmts.iter().map(|f| f.id).collect::<Vec<_>>().join(", ")
            ),
        )
    })?;
    if !f.available {
        bail!(err("unavailable", f.reason.clone().unwrap_or_default()));
    }
    Ok(o)
}

fn out_path(api: &Api, v: &Value, base: &str, ext: &str) -> PathBuf {
    if let Some(p) = os(v, "path") {
        let mut p = PathBuf::from(p);
        if p.extension().is_none() {
            p.set_extension(ext);
        }
        return p;
    }
    let dir = os(v, "dir").map(PathBuf::from).unwrap_or_else(|| {
        api.lock()
            .settings
            .export_dir
            .clone()
            .map(PathBuf::from)
            .unwrap_or_else(crate::store::default_export_dir)
    });
    unique_path(dir.join(format!("{}.{ext}", file_name_safe(base))))
}

fn h_export_formats(_: &Api, _: &Value) -> Result<Value> {
    Ok(serde_json::to_value(export::formats())?)
}

fn h_export_start(api: &Api, v: &Value) -> Result<Value> {
    let mut o = opts_of(v)?;
    let range = range_of(api, v)?;
    let (p, pool) = api.snapshot()?;
    if p.length() == 0 {
        bail!(err("empty", "There is nothing to export yet"));
    }
    if o.normalize == "lufs" && v.get("options").and_then(|x| x.get("target")).is_none() {
        o.target = p.target_lufs;
    }
    let path = out_path(api, v, &p.name, export::ext_for(&o.format));
    let a = api.clone();
    let path_out = path.display().to_string();
    let tail = ob(v, "tail").unwrap_or(true);
    let job = api.jobs.spawn(
        "export",
        &format!(
            "Exporting {}",
            path.file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        ),
        &["Rendering", "Encoding", "Checking the file"],
        move |ctx| {
            ctx.stage("Rendering", "active");
            let (s0, s1) = range.unwrap_or((0, p.length()));
            let mut r = render::Renderer::new(render::Plan::build(&p, pool));
            let (l, rr) = r
                .render_offline(s0, s1, tail && range.is_none(), &mut |f| {
                    ctx.progress(f * 0.8)
                })
                .ok_or_else(|| anyhow::anyhow!("cancelled"))?;
            ctx.stage("Rendering", "done");
            ctx.stage("Encoding", "active");
            let rep = export::export_audio(l, rr, p.sample_rate, &o, &path)?;
            ctx.stage("Encoding", "done");
            ctx.stage("Checking the file", "done");
            let _ = &a;
            Ok(serde_json::to_value(rep)?)
        },
    );
    if ob(v, "wait").unwrap_or(false) {
        let j = api
            .jobs
            .wait(&job, std::time::Duration::from_secs(3600))
            .unwrap();
        if j.state == "failed" {
            bail!(err("failed", j.error.unwrap_or_default()));
        }
        return Ok(json!({"job": job, "result": j.result}));
    }
    Ok(json!({"job": job, "path": path_out}))
}

fn h_export_stems(api: &Api, v: &Value) -> Result<Value> {
    let o = opts_of(v)?;
    let (p, pool) = api.snapshot()?;
    if p.length() == 0 {
        bail!(err("empty", "There is nothing to export yet"));
    }
    let dir = os(v, "dir").map(PathBuf::from).unwrap_or_else(|| {
        api.lock()
            .settings
            .export_dir
            .clone()
            .map(PathBuf::from)
            .unwrap_or_else(crate::store::default_export_dir)
            .join(format!("{} stems", file_name_safe(&p.name)))
    });
    let job = api.jobs.spawn(
        "export",
        "Exporting stems",
        &["Rendering tracks"],
        move |ctx| {
            std::fs::create_dir_all(&dir)?;
            let mut files = vec![];
            let n = p.tracks.len();
            for (i, t) in p.tracks.iter().enumerate() {
                if t.clips.is_empty() {
                    continue;
                }
                let mut q = p.clone();
                for x in &mut q.tracks {
                    x.solo = x.id == t.id;
                    x.mute = false;
                }
                let (l, r) = render::render_project(&q, pool.clone(), None, true);
                let path = unique_path(dir.join(format!(
                    "{:02} {}.{}",
                    i + 1,
                    file_name_safe(&t.name),
                    export::ext_for(&o.format)
                )));
                let rep = export::export_audio(l, r, p.sample_rate, &o, &path)?;
                files.push(rep);
                if !ctx.progress((i + 1) as f32 / n as f32) {
                    bail!("cancelled");
                }
            }
            Ok(json!({"dir": dir.display().to_string(), "files": files}))
        },
    );
    Ok(json!({"job": job}))
}

fn render_file_with_chain(
    path: &Path,
    chain: &[Effect],
    sr_out: Option<u32>,
) -> Result<(Vec<f32>, Vec<f32>, u32, String)> {
    let d = crate::media::decode_file(path)?;
    let sr = sr_out.unwrap_or(d.rate).clamp(8000, 192000);
    let sr = if [
        44100, 48000, 88200, 96000, 176400, 192000, 32000, 22050, 16000, 8000, 11025,
    ]
    .contains(&sr)
    {
        sr
    } else {
        48000
    };
    let m = Media::new("src", d.data, d.rate, sr);
    let frames = m.frames as i64;
    if chain.is_empty() {
        let l = m.data[0].as_ref().clone();
        let r = m.data[m.data.len() - 1].as_ref().clone();
        return Ok((l, r, sr, d.format));
    }
    let mut p = Project::new("batch", sr.max(8000));
    p.sample_rate = sr;
    let t = p.add_track("in");
    p.media.push(MediaRef {
        id: "src".into(),
        frames,
        ..Default::default()
    });
    p.add_clip(t, "src", 0, 0, None)?;
    for e in chain {
        let mut e = e.clone();
        e.id = p.alloc();
        let _ = insert_effect(&mut p, "master", e, None);
    }
    let mut pool = crate::media::Pool::new();
    pool.insert("src".into(), Arc::new(m));
    let (l, r) = render::render_project(&p, Arc::new(pool), None, true);
    Ok((l, r, sr, d.format))
}

fn chain_from(api: &Api, v: &Value) -> Result<Vec<Effect>> {
    let mut chain = vec![];
    if let Some(pid) = os(v, "preset") {
        let pr = builtin_presets()
            .into_iter()
            .chain(api.lock().store.presets())
            .find(|p| p.id == pid)
            .ok_or_else(|| err("not_found", "no such preset"))?;
        if pr.kind == "chain" {
            chain = pr.chain;
        } else {
            let mut tmp = Project::new("t", 48000);
            chain.push(make_effect(&mut tmp, &pr.kind, &pr.params, None, None)?);
        }
    }
    if let Some(a) = v.get("chain").and_then(|x| x.as_array()) {
        let mut tmp = Project::new("t", 48000);
        for e in a {
            let kind = s_(e, "kind")?;
            chain.push(make_effect(
                &mut tmp,
                &kind,
                &fparams(e, "params")?,
                of(e, "mix"),
                None,
            )?);
        }
    }
    if let Some(st) = os(v, "style") {
        let mut tmp = Project::new("t", 48000);
        for (k, p, mix) in style_fx(&st, of(v, "amount").unwrap_or(0.5))
            .ok_or_else(|| err("bad_params", "unknown style"))?
        {
            chain.push(make_effect(&mut tmp, k, &p, Some(mix), None)?);
        }
    }
    Ok(chain)
}

fn files_of(v: &Value) -> Result<Vec<PathBuf>> {
    let a = need(v, "files")?
        .as_array()
        .ok_or_else(|| err("bad_params", "files must be an array of paths"))?;
    let f: Vec<PathBuf> = a
        .iter()
        .filter_map(|x| x.as_str().map(PathBuf::from))
        .collect();
    if f.is_empty() {
        bail!(err("bad_params", "files is empty"));
    }
    Ok(f)
}

fn batch_job(api: &Api, v: &Value, kind: &str, title: &str, chain: Vec<Effect>) -> Result<Value> {
    let files = files_of(v)?;
    let o = opts_of(v)?;
    let sr_out = ou(v, "sample_rate");
    let dir = os(v, "dir").map(PathBuf::from).unwrap_or_else(|| {
        api.lock()
            .settings
            .export_dir
            .clone()
            .map(PathBuf::from)
            .unwrap_or_else(crate::store::default_export_dir)
            .join(if kind == "convert" {
                "Converted"
            } else {
                "Batch"
            })
    });
    let suffix = os(v, "suffix").unwrap_or_default();
    let stages: Vec<String> = files
        .iter()
        .map(|f| {
            f.file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        })
        .collect();
    let st: Vec<&str> = stages.iter().map(|s| s.as_str()).collect();
    let names = stages.clone();
    let job = api.jobs.spawn(kind, title, &st, move |ctx| {
        let stages = names;
        std::fs::create_dir_all(&dir)?;
        let mut out = vec![];
        for (i, f) in files.iter().enumerate() {
            let name = stages[i].clone();
            ctx.stage(&name, "active");
            let r = (|| -> Result<Value> {
                let (l, r, sr, _) = render_file_with_chain(f, &chain, sr_out)?;
                let stem = f
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "audio".into());
                let path = unique_path(dir.join(format!(
                    "{}{}.{}",
                    file_name_safe(&stem),
                    suffix,
                    export::ext_for(&o.format)
                )));
                Ok(serde_json::to_value(export::export_audio(
                    l, r, sr, &o, &path,
                )?)?)
            })();
            match r {
                Ok(rep) => {
                    ctx.stage(&name, "done");
                    out.push(json!({"file": name, "ok": true, "report": rep}));
                }
                Err(e) => {
                    ctx.stage(&name, "failed");
                    out.push(json!({"file": name, "ok": false, "error": format!("{e:#}")}));
                }
            }
            if !ctx.progress((i + 1) as f32 / files.len() as f32) {
                bail!("cancelled");
            }
        }
        Ok(json!({"dir": dir.display().to_string(), "results": out}))
    });
    if ob(v, "wait").unwrap_or(false) {
        let j = api
            .jobs
            .wait(&job, std::time::Duration::from_secs(3600))
            .unwrap();
        return Ok(json!({"job": job, "result": j.result, "state": j.state, "error": j.error}));
    }
    Ok(json!({"job": job}))
}

fn h_convert(api: &Api, v: &Value) -> Result<Value> {
    batch_job(api, v, "convert", "Converting files", vec![])
}
fn h_batch(api: &Api, v: &Value) -> Result<Value> {
    let chain = chain_from(api, v)?;
    if chain.is_empty() {
        bail!(err(
            "bad_params",
            "pass `preset`, `style` or `chain` with at least one effect"
        ));
    }
    batch_job(api, v, "batch", "Batch processing", chain)
}
fn h_ringtone(api: &Api, v: &Value) -> Result<Value> {
    let o = opts_of(&json!({"format": os(v, "format").unwrap_or_else(|| "mp3".into())}))?;
    let (l, r, sr, name) = if let Some(f) = os(v, "file") {
        let (l, r, sr, _) = render_file_with_chain(Path::new(&f), &[], None)?;
        (
            l,
            r,
            sr,
            Path::new(&f)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default(),
        )
    } else {
        let m = api.render_mix(None)?;
        let s = api.lock();
        let p = s.project()?;
        (m.0.clone(), m.1.clone(), p.sample_rate, p.name.clone())
    };
    let start = opos(v, "start", sr).unwrap_or(0).clamp(0, l.len() as i64) as usize;
    let max = if o.format == "m4r" { 40 } else { 60 };
    let len = (opos(v, "len", sr)
        .unwrap_or(sr as i64 * 30)
        .clamp(sr as i64, sr as i64 * max) as usize)
        .min(l.len() - start);
    let fade_in = (of(v, "fade_in_s").unwrap_or(0.5) * sr as f32) as usize;
    let fade_out = (of(v, "fade_out_s").unwrap_or(1.5) * sr as f32) as usize;
    let mut a = l[start..start + len].to_vec();
    let mut b = r[start..start + len].to_vec();
    for i in 0..len {
        let mut g = 1.0;
        if i < fade_in {
            g *= i as f32 / fade_in as f32;
        }
        if len - i <= fade_out {
            g *= (len - i) as f32 / fade_out as f32;
        }
        a[i] *= g;
        b[i] *= g;
    }
    let mut o = o;
    o.normalize = "lufs".into();
    o.target = -12.0;
    o.ceiling = -1.0;
    o.bitrate = 256;
    let path = out_path(
        api,
        v,
        &format!("{name} ringtone"),
        export::ext_for(&o.format),
    );
    Ok(serde_json::to_value(export::export_audio(
        a, b, sr, &o, &path,
    )?)?)
}

// ---------------------------------------------------------------- presets, actions, jobs

fn h_preset_list(api: &Api, v: &Value) -> Result<Value> {
    let kind = os(v, "kind");
    let mut all: Vec<Preset> = builtin_presets();
    all.extend(api.lock().store.presets());
    Ok(Value::Array(
        all.into_iter()
            .filter(|p| kind.as_ref().map(|k| &p.kind == k).unwrap_or(true))
            .map(|p| serde_json::to_value(p).unwrap())
            .collect(),
    ))
}
fn h_preset_save(api: &Api, v: &Value) -> Result<Value> {
    let name = s_(v, "name")?;
    let s = api.lock();
    let p = s.project()?;
    let pr = if let Some(id) = ou(v, "fx") {
        let (_, e) = p
            .find_fx(id)
            .ok_or_else(|| err("not_found", "no such effect"))?;
        Preset {
            id: new_id(),
            name,
            kind: e.kind.clone(),
            params: e.params.clone(),
            chain: vec![],
            created: crate::project::now_secs(),
            builtin: false,
        }
    } else {
        let target = os(v, "target").unwrap_or_else(|| "master".into());
        let mut pc = p.clone();
        let ch = pc.chain_mut(&target)?.clone();
        Preset {
            id: new_id(),
            name,
            kind: "chain".into(),
            params: BTreeMap::new(),
            chain: ch,
            created: crate::project::now_secs(),
            builtin: false,
        }
    };
    s.store.save_preset(&pr)?;
    Ok(serde_json::to_value(pr)?)
}
fn h_preset_apply(api: &Api, v: &Value) -> Result<Value> {
    let id = s_(v, "id")?;
    let pr = builtin_presets()
        .into_iter()
        .chain(api.lock().store.presets())
        .find(|p| p.id == id)
        .ok_or_else(|| err("not_found", "no such preset"))?;
    let target = target_of(api, v)?;
    let fx_id = ou(v, "fx");
    api.lock().commit(&format!("Preset {}", pr.name), |p| {
        if let Some(fid) = fx_id {
            let e = p.fx_mut(fid)?;
            if e.kind != pr.kind {
                bail!(err(
                    "bad_params",
                    format!("that preset is for {}, not {}", pr.kind, e.kind)
                ));
            }
            e.params = fx::defaults(fx::spec(&pr.kind).unwrap());
            for (k, x) in &pr.params {
                e.params.insert(k.clone(), *x);
            }
            return Ok(json!({"fx": [fid]}));
        }
        let mut ids = vec![];
        if pr.kind == "chain" {
            for e in &pr.chain {
                let mut e = e.clone();
                e.id = p.alloc();
                ids.push(insert_effect(p, &target, e, None)?);
            }
        } else {
            let e = make_effect(p, &pr.kind, &pr.params, None, None)?;
            ids.push(insert_effect(p, &target, e, None)?);
        }
        Ok(json!({"fx": ids}))
    })
}
fn h_preset_delete(api: &Api, v: &Value) -> Result<Value> {
    let id = s_(v, "id")?;
    if id.starts_with("eq-") || id.starts_with("room-") || id.starts_with("voice-") {
        bail!(err("bad_params", "built-in presets can't be deleted"));
    }
    api.lock().store.delete_preset(&id)?;
    Ok(json!({"deleted": id}))
}

fn h_action_list(api: &Api, _: &Value) -> Result<Value> {
    let s = api.lock();
    Ok(json!({"actions": s.store.actions(), "recording": s.macro_rec.as_ref().map(|m| m.len())}))
}
fn h_action_save(api: &Api, v: &Value) -> Result<Value> {
    let name = s_(v, "name")?;
    let steps: Vec<ActionStep> =
        serde_json::from_value(need(v, "steps")?.clone()).map_err(|e| {
            err(
                "bad_params",
                format!("steps must be [{{method, params}}]: {e}"),
            )
        })?;
    for st in &steps {
        let m = find(&st.method)
            .ok_or_else(|| err("bad_params", format!("unknown method {}", st.method)))?;
        if !m.records {
            bail!(err(
                "bad_params",
                format!("{} can't be part of an action", st.method)
            ));
        }
    }
    let a = Action {
        id: os(v, "id").unwrap_or_else(new_id),
        name,
        description: os(v, "description").unwrap_or_default(),
        steps,
        created: crate::project::now_secs(),
    };
    api.lock().store.save_action(&a)?;
    Ok(serde_json::to_value(a)?)
}
fn h_action_record(api: &Api, v: &Value) -> Result<Value> {
    let on = ob(v, "on").unwrap_or(true);
    let mut s = api.lock();
    if on {
        s.macro_rec = Some(vec![]);
        return Ok(json!({"recording": true}));
    }
    let steps = s.macro_rec.take().unwrap_or_default();
    drop(s);
    if steps.is_empty() {
        return Ok(json!({"recording": false, "saved": null}));
    }
    let name = os(v, "name").unwrap_or_else(|| "Recorded action".into());
    let r = h_action_save(api, &json!({"name": name, "steps": steps}))?;
    Ok(json!({"recording": false, "saved": r}))
}
fn h_action_run(api: &Api, v: &Value) -> Result<Value> {
    let id = s_(v, "id")?;
    let a = api
        .lock()
        .store
        .actions()
        .into_iter()
        .find(|a| a.id == id)
        .ok_or_else(|| err("not_found", "no such action"))?;
    let mut done = vec![];
    for st in &a.steps {
        if st.method == "action.run" {
            continue;
        }
        let r = api.call(&st.method, st.params.clone());
        match r {
            Ok(x) => done.push(json!({"method": st.method, "ok": true, "result": x})),
            Err(e) => {
                done.push(json!({"method": st.method, "ok": false, "error": format!("{e:#}")}));
                return Ok(json!({"completed": false, "steps": done}));
            }
        }
    }
    Ok(json!({"completed": true, "steps": done}))
}
fn h_action_delete(api: &Api, v: &Value) -> Result<Value> {
    let id = s_(v, "id")?;
    api.lock().store.delete_action(&id)?;
    Ok(json!({"deleted": id}))
}

fn h_job_list(api: &Api, _: &Value) -> Result<Value> {
    Ok(serde_json::to_value(api.jobs.list())?)
}
fn h_job_get(api: &Api, v: &Value) -> Result<Value> {
    let id = s_(v, "id")?;
    let wait = of(v, "wait_s").unwrap_or(0.0).clamp(0.0, 600.0);
    let j = if wait > 0.0 {
        api.jobs.wait(&id, std::time::Duration::from_secs_f32(wait))
    } else {
        api.jobs.get(&id)
    };
    Ok(serde_json::to_value(
        j.ok_or_else(|| err("not_found", "no such job"))?,
    )?)
}
fn h_job_cancel(api: &Api, v: &Value) -> Result<Value> {
    let id = s_(v, "id")?;
    Ok(json!({"cancelled": api.jobs.cancel(&id)}))
}
fn h_job_clear(api: &Api, _: &Value) -> Result<Value> {
    api.jobs.clear_finished();
    Ok(json!({"ok": true}))
}

fn h_mcp_info(api: &Api, _: &Value) -> Result<Value> {
    let root = api.lock().store.root.clone();
    Ok(json!({
        "server": "flux-mcp", "transport": "stdio",
        "control_file": root.join("control.json").display().to_string(),
        "modes": ["attach: drives the running Flux app", "headless: runs its own engine when the app is closed"],
        "tools": METHODS.len(),
        "config": {"mcpServers": {"flux": {"command": "flux-mcp", "args": []}}},
    }))
}

pub static METHODS: &[MethodSpec] = &[
    m!("methods", "List every Flux method with its parameters.", "", false, h_methods),
    m!("app.info", "Version, platform, data folder, export formats and what processing is available.", "", false, h_app_info),
    m!("app.capabilities", "What this computer supports: audio engine, buffers, folders, AAC, offline.", "", false, h_capabilities),
    m!("ai.status", "Which smart features are available and how each works (DSP, system, or needs a model).", "", false, h_ai_status),
    m!("mcp.info", "How to connect an agent to Flux over MCP.", "", false, h_mcp_info),
    m!("settings.get", "Read all settings.", "", false, h_settings_get),
    m!("settings.set", "Change settings (theme, font, mode, output, input, buffer, safety_limiter, export_dir, ...).", "!values:object:Settings to change, e.g. {\"theme\":\"vinyl\"}", false, h_settings_set),
    m!("system.diagnostics", "A diagnostic report. Never includes audio, transcripts or file names.", "", false, h_diag),
    m!("storage.usage", "How much space projects, presets and the trash use.", "", false, h_storage_usage),
    m!("storage.empty_trash", "Permanently delete projects in the trash.", "", false, h_storage_empty_trash),
    m!("recovery.list", "Projects that were open when Flux last stopped unexpectedly.", "", false, h_recovery_list),
    m!("recovery.dismiss", "Clear the crash marker of a project.", "!id:string:Project id", false, h_recovery_dismiss),
    m!("library.list", "List saved projects.", "search:string:Filter by name|kind:string:all, music, podcast, spatial, voice, lecture|folder:string:Folder tag", false, h_library),
    m!("project.new", "Create and open a project from a template.", "name:string:Project name|template:string:blank, podcast, music, voiceover, lecture, interview or 8d|sample_rate:integer:44100, 48000, 96000 or 192000|folder:string:Folder tag", false, h_project_new),
    m!("project.open", "Open a saved project.", "!id:string:Project id from library.list", false, h_project_open),
    m!("project.close", "Save and close the current project.", "", false, h_project_close),
    m!("project.get", "The open project: tracks, clips, effects, markers, media, selection. Positions are samples.", "", false, h_project_get),
    m!("project.set", "Rename the project or set tempo, key, loudness target, folder, kind or master gain.", "name:string:New name|tempo:number:BPM|key:string:e.g. D minor|target_lufs:number:Export loudness target|folder:string:Folder tag|kind:string:Project type|master_gain_db:number:Master level", true, h_project_set),
    m!("project.delete", "Move a project to the trash (recoverable until the trash is emptied).", "!id:string:Project id", false, h_project_delete),
    m!("project.duplicate", "Copy a project, including its media.", "!id:string:Project id", false, h_project_duplicate),
    m!("project.export_package", "Save the project and all its media as one .flux file.", "path:string:Where to write the .flux file", false, h_project_export_package),
    m!("project.import_package", "Open a .flux file as a new project.", "!path:string:Path to a .flux file", false, h_project_import_package),
    m!("snapshot.save", "Save a named snapshot of the project.", "name:string:Snapshot name", false, h_snapshot_save),
    m!("snapshot.list", "List snapshots of the project.", "", false, h_snapshot_list),
    m!("snapshot.restore", "Go back to a snapshot (undoable).", "!id:string:Snapshot id", true, h_snapshot_restore),
    m!("history.list", "Every change, oldest first, with the current position.", "", false, h_history),
    m!("history.undo", "Undo the last change.", "", true, h_undo),
    m!("history.redo", "Redo the last undone change.", "", true, h_redo),
    m!("history.goto", "Undo or redo until the history is at `index`.", "!index:integer:Position from history.list", true, h_history_goto),
    m!("media.import", "Add an audio or video file. Creates a project if none is open. Runs as a job.", "!path:string:File path|track:integer:Track to put it on|at:pos:Timeline position|new_track:boolean:Put it on a new track (default true)|wait:boolean:Wait until it is ready", true, h_media_import),
    m!("media.list", "Media files used by the project.", "", false, h_media_list),
    m!("media.peaks", "Waveform peaks (min/max pairs per channel) for drawing.", "!media:string:Media id|start:integer:First sample|end:integer:Last sample|bins:integer:Number of columns|reverse:boolean:Reversed", false, h_media_peaks),
    m!("media.relink", "Point missing media at a file on disk.", "!id:string:Media id|!path:string:File path", true, h_media_relink),
    m!("track.add", "Add a track.", "name:string:Track name|index:integer:Position", true, h_track_add),
    m!("track.remove", "Delete a track and its clips.", "!track:integer:Track id", true, h_track_remove),
    m!("track.set", "Change a track: name, level, pan, mute, solo, arm, color.", "!track:integer:Track id|name:string:Name|gain_db:number:Level in dB|pan:number:-1 left to 1 right|mute:boolean:Mute|solo:boolean:Solo|arm:boolean:Armed for recording|color:integer:1 to 8", true, h_track_set),
    m!("track.move", "Move a track up or down.", "!track:integer:Track id|!index:integer:New position", true, h_track_move),
    m!("clip.move", "Move a clip in time or to another track.", "!clip:integer:Clip id|!start:pos:New start|track:integer:Destination track", true, h_clip_move),
    m!("clip.trim", "Trim a clip's start or end (never beyond its media).", "!clip:integer:Clip id|start:pos:New start|end:pos:New end", true, h_clip_trim),
    m!("clip.split", "Split clips at a position (default: playhead).", "clip:integer:Only this clip|at:pos:Split position", true, h_clip_split),
    m!("clip.delete", "Delete clips.", "clip:integer:Clip id|clips:array:Several clip ids", true, h_clip_delete),
    m!("clip.duplicate", "Duplicate a clip right after itself.", "!clip:integer:Clip id", true, h_clip_duplicate),
    m!("clip.set", "Change a clip: name, gain, fades, fade curve, reverse.", "!clip:integer:Clip id|name:string:Name|gain_db:number:Clip gain|fade_in:pos:Fade-in length|fade_out:pos:Fade-out length|curve:string:equal, s, log or lin|reverse:boolean:Play backwards", true, h_clip_set),
    m!("clip.normalize", "Set a clip's gain so its peak reaches a level.", "!clip:integer:Clip id|peak_db:number:Peak level (default -1)", true, h_clip_normalize),
    m!("clip.bounce", "Render a clip with its effects into new audio.", "!clip:integer:Clip id", true, h_clip_bounce),
    m!("edit.cut_range", "Remove a time range from tracks, optionally closing the gap.", "!start:pos:Start|!end:pos:End|ripple:boolean:Close the gap (default true)|tracks:array:Only these tracks", true, h_edit_cut_range),
    m!("edit.insert_silence", "Insert silence, moving later material right.", "!at:pos:Position|!len:pos:Length", true, h_edit_insert_silence),
    m!("edit.copy", "Copy clips.", "clip:integer:Clip id|clips:array:Several clip ids", false, h_edit_copy),
    m!("edit.cut", "Cut clips (copy then delete).", "clip:integer:Clip id|clips:array:Several clip ids", true, h_edit_cut),
    m!("edit.paste", "Paste copied clips.", "at:pos:Where (default playhead)|track:integer:Destination track", true, h_edit_paste),
    m!("selection.set", "Set the selected range, track or clip.", "start:pos:Range start|end:pos:Range end|track:integer:Track id|clip:integer:Clip id", false, h_selection_set),
    m!("marker.add", "Add a marker.", "pos:pos:Position (default playhead)|name:string:Label", true, h_marker_add),
    m!("marker.remove", "Remove a marker.", "!marker:integer:Marker id", true, h_marker_remove),
    m!("lane.set", "Create or replace an automation lane. Targets: volume (dB), pan (-1..1), fx:<id>:<param>.", "!track:integer:Track id|!target:string:volume, pan or fx:<effect id>:<param key>|!points:array:[[sample, value], ...] or [{pos_s, value}]|shape:string:linear, smooth or hold", true, h_lane_set),
    m!("lane.remove", "Remove an automation lane.", "!track:integer:Track id|!lane:integer:Lane id", true, h_lane_remove),
    m!("fx.catalog", "Every effect with its parameters, ranges and defaults.", "", false, h_fx_catalog),
    m!("fx.add", "Add an effect to a clip, track or the master chain.", "!kind:string:Effect kind from fx.catalog|target:string:master, track:<id> or clip:<id>|params:object:Parameter values|mix:number:Dry/wet 0..1|index:integer:Position in the chain|group:string:Tag", true, h_fx_add),
    m!("fx.remove", "Remove an effect.", "!fx:integer:Effect id", true, h_fx_remove),
    m!("fx.move", "Reorder an effect in its chain.", "!fx:integer:Effect id|!index:integer:New position", true, h_fx_move),
    m!("fx.set", "Change an effect's parameters, bypass or mix.", "!fx:integer:Effect id|params:object:Parameter values|enabled:boolean:On or bypassed|mix:number:Dry/wet 0..1", true, h_fx_set),
    m!("fx.get", "One effect's settings (EQ includes its response curve).", "!fx:integer:Effect id", false, h_fx_get),
    m!("fx.slot", "Switch an effect between A, B and C settings.", "!fx:integer:Effect id|!slot:integer:0 = A, 1 = B, 2 = C", true, h_fx_slot),
    m!("fx.learn_noise", "Teach noise reduction what the noise sounds like from a noise-only range.", "!fx:integer:Noise reduction effect id|start:pos:Noise start (default selection)|end:pos:Noise end", true, h_fx_learn_noise),
    m!("fx.load_ir", "Load an impulse response file into the impulse-response reverb.", "!fx:integer:Convolver effect id|!path:string:Audio file", true, h_fx_load_ir),
    m!("fx.set_path", "Draw a motion path for the 3D/8D stage (x right, y front, -1..1).", "!fx:integer:Spatial effect id|!points:array:[[x, y], ...]", true, h_fx_set_path),
    m!("fx.motion", "Where a 3D source is at a time (for drawing the stage).", "!fx:integer:Spatial effect id|t:number:Seconds", false, h_fx_motion),
    m!("chain.get", "The effects on a clip, track or the master.", "target:string:master, track:<id> or clip:<id>", false, h_chain_get),
    m!("style.list", "Named looks (slowed, nightcore, 8d, lofi, ...), rooms and voices.", "", false, h_style_list),
    m!("style.apply", "Apply a named look with an amount; moving the amount again replaces it.", "!style:string:slowed, nightcore, 8d, lofi, cathedral, robot, vinyl, ...|amount:number:0..1 (0 removes it)|target:string:master or track:<id>|replace:boolean:Update in place (default true)", true, h_style_apply),
    m!("preset.list", "Built-in and saved presets.", "kind:string:Only this effect kind", false, h_preset_list),
    m!("preset.save", "Save an effect's settings, or a whole chain, as a preset.", "!name:string:Preset name|fx:integer:Effect id|target:string:Chain to save (master, track:<id>)", false, h_preset_save),
    m!("preset.apply", "Apply a preset to an effect, or add it to a chain.", "!id:string:Preset id|fx:integer:Effect to load it into|target:string:Chain to add to", true, h_preset_apply),
    m!("preset.delete", "Delete a saved preset.", "!id:string:Preset id", false, h_preset_delete),
    m!("transport.play", "Start playback.", "from:pos:Start position", false, h_transport_play),
    m!("transport.pause", "Pause playback.", "", false, h_transport_pause),
    m!("transport.stop", "Stop and return to the start.", "", false, h_transport_stop),
    m!("transport.seek", "Move the playhead.", "!pos:pos:Position", false, h_transport_seek),
    m!("transport.loop", "Turn looping on or off.", "on:boolean:Loop on|start:pos:Loop start|end:pos:Loop end", false, h_transport_loop),
    m!("transport.hold", "Hold for original: hear the unprocessed audio at matched loudness.", "on:boolean:Hold", false, h_transport_hold),
    m!("transport.state", "Playhead, levels, loudness, glitches, recording and project revision.", "", false, h_transport_state),
    m!("transport.scope", "Recent output: spectrum, stereo correlation and samples for meters.", "samples:integer:How many samples|bins:integer:Spectrum bands", false, h_transport_scope),
    m!("audio.devices", "Audio outputs and inputs on this computer.", "", false, h_audio_devices),
    m!("audio.test_tone", "Play a test tone: left, right or moving (headphone check).", "kind:string:left, right or moving", false, h_audio_test_tone),
    m!("record.start", "Record from an input to disk as it happens.", "track:integer:Record to this track|input:string:Input device id|monitor:boolean:Hear yourself", false, h_record_start),
    m!("record.pause", "Pause or resume recording.", "paused:boolean:Paused", false, h_record_pause),
    m!("record.stop", "Stop recording and add the take (or discard it).", "discard:boolean:Throw the take away", false, h_record_stop),
    m!("analysis.loudness", "Integrated, short-term and momentary LUFS, loudness range, true peak, stereo correlation.", "range:string:all, selection or loop|start:pos:Range start|end:pos:Range end", false, h_analysis_loudness),
    m!("analysis.tempo_key", "Detect tempo (BPM) and key.", "range:string:all, selection or loop|start:pos:Range start|end:pos:Range end|apply:boolean:Set them on the project", false, h_analysis_tempo_key),
    m!("analysis.problems", "Noise floor, hum, clipping, clicks, DC, stereo, sibilance, rumble, reverberance.", "range:string:all, selection or loop|start:pos:Range start|end:pos:Range end", false, h_analysis_problems),
    m!("analysis.spectrum", "Average spectrum of the mix.", "range:string:all, selection or loop|start:pos:Range start|end:pos:Range end|bins:integer:Bands", false, h_analysis_spectrum),
    m!("analysis.spectrogram", "Spectrogram image data (base64 bytes, rows x cols, log frequency).", "media:string:Only this media|start:pos:Start|end:pos:End|cols:integer:Columns|rows:integer:Rows|window:integer:FFT size|range:string:all, selection or loop", false, h_analysis_spectrogram),
    m!("tool.stems", "Split into stems (vocals, drums, bass, other). DSP spectral split, not a neural model. Job.", "mode:string:4 or 2 (vocals + instrumental)|track:integer:Only this track", true, h_stems),
    m!("tool.master.analyze", "Measure the mix and propose a mastering chain, without changing anything.", "style:string:balanced, warm, bright, loud or gentle|target:number:LUFS target|amount:number:0..1", false, h_master_analyze),
    m!("tool.master.apply", "Auto-master: add a measured EQ, compressor and limiter to the master.", "style:string:balanced, warm, bright, loud or gentle|target:number:LUFS target|amount:number:0..1", true, h_master_apply),
    m!("tool.magic.scan", "Find problems in the mix and propose fixes.", "", false, h_magic_scan),
    m!("tool.magic.apply", "Apply Magic fixes (all proposed, or the ones listed).", "ids:array:Fix ids from tool.magic.scan", true, h_magic_apply),
    m!("tool.cleanup", "Clean up a voice: rumble, hum, noise, harshness, level.", "track:integer:Track (default whole mix)|strength:number:0..1", true, h_cleanup),
    m!("tool.silence.scan", "Find long pauses.", "threshold_db:number:Quieter than this is silence|longer_than_s:number:Minimum pause|shorten_to_s:number:Shorten pauses to|padding_s:number:Keep around speech", false, h_silence_scan),
    m!("tool.silence.apply", "Shorten long pauses across all tracks.", "threshold_db:number:Quieter than this is silence|longer_than_s:number:Minimum pause|shorten_to_s:number:Shorten pauses to|padding_s:number:Keep around speech|keep:array:Pause indexes to leave alone", true, h_silence_apply),
    m!("tool.spectral.repair", "Remove or fill in a time-frequency region (non-destructive).", "!start:pos:Start|!end:pos:End|!low_hz:number:Lowest frequency|!high_hz:number:Highest frequency|track:integer:Track|clip:integer:Clip|amount_db:number:Reduction|fill:string:both, before, after or attenuate", true, h_spectral_repair),
    m!("tool.match_eq", "Copy the tone of a reference recording with a graphic EQ.", "!path:string:Reference file|track:integer:Apply to this track (default master)", true, h_match_eq),
    m!("tool.reverse_swell", "Add a reverse reverb swell that rises into a clip.", "!clip:integer:Clip id|beats:number:Swell length in beats", true, h_reverse_swell),
    m!("tts.voices", "Speech voices installed on this computer.", "", false, h_tts_voices),
    m!("tts.speak", "Generate speech with a system voice onto a new track, labelled synthetic. Job.", "!text:string:What to say|voice:string:Voice id from tts.voices|rate:integer:-10 slow to 10 fast|at:pos:Timeline position", true, h_tts_speak),
    m!("transcript.get", "Transcript of the project (needs a speech model; not in this build).", "", false, h_unavailable_transcript),
    m!("transcript.fillers", "Find filler words (needs a transcript; not in this build).", "", false, h_unavailable_transcript),
    m!("dub.start", "Translate and dub (needs speech and translation models; not in this build).", "language:string:Target language", false, h_unavailable_dub),
    m!("clone.start", "Voice cloning (not included).", "", false, h_unavailable_clone),
    m!("export.formats", "Export formats and whether each is available on this computer.", "", false, h_export_formats),
    m!("export.start", "Export the mix to a file. Job; the result is measured from the written file.", "format:string:wav, flac, mp3, ogg, aac, m4r, opus|path:string:Output file|dir:string:Output folder|options:object:{bits, sample_rate, bitrate, quality, vbr, mono, normalize (none|lufs|peak), target, ceiling, dither, tags}|range:string:all, selection or loop|start:pos:Range start|end:pos:Range end|tail:boolean:Include effect tails|wait:boolean:Wait for the file", false, h_export_start),
    m!("export.stems", "Export each track as its own file. Job.", "format:string:Format|dir:string:Output folder|options:object:Export options", false, h_export_stems),
    m!("convert.start", "Convert audio files to another format. Job.", "!files:array:Input paths|format:string:Output format|options:object:Export options|sample_rate:integer:Output rate|dir:string:Output folder|suffix:string:Added to file names|wait:boolean:Wait", false, h_convert),
    m!("batch.start", "Process many files with a preset, style or effect chain, then export. Job.", "!files:array:Input paths|preset:string:Preset id|style:string:Style id|amount:number:Style amount|chain:array:[{kind, params, mix}]|format:string:Output format|options:object:Export options|sample_rate:integer:Output rate|dir:string:Output folder|suffix:string:Added to file names|wait:boolean:Wait", false, h_batch),
    m!("ringtone.make", "Make a ringtone from the project or a file: pick a range, fade, normalize.", "file:string:Source file (default the project)|start:pos:Start|len:pos:Length (up to 40 s for M4R)|fade_in_s:number:Fade in|fade_out_s:number:Fade out|format:string:m4r, mp3 or ogg|path:string:Output file|dir:string:Output folder", false, h_ringtone),
    m!("action.list", "Saved actions (macros).", "", false, h_action_list),
    m!("action.save", "Save an action: a list of method calls.", "!name:string:Name|!steps:array:[{method, params}]|description:string:What it does|id:string:Replace this action", false, h_action_save),
    m!("action.record", "Start recording an action from what you do, or stop and save it.", "on:boolean:Start (true) or stop (false)|name:string:Name when stopping", false, h_action_record),
    m!("action.run", "Run a saved action on the open project.", "!id:string:Action id", false, h_action_run),
    m!("action.delete", "Delete an action.", "!id:string:Action id", false, h_action_delete),
    m!("job.list", "Background jobs and their progress.", "", false, h_job_list),
    m!("job.get", "One job; optionally wait for it to finish.", "!id:string:Job id|wait_s:number:Wait up to this many seconds", false, h_job_get),
    m!("job.cancel", "Cancel a running job.", "!id:string:Job id", false, h_job_cancel),
    m!("job.clear", "Forget finished jobs.", "", false, h_job_clear),
];

/// History entries for tests.
pub fn undo_len(api: &Api) -> usize {
    api.lock().undo.len()
}
#[allow(dead_code)]
fn _hist(_: &Hist) {}
