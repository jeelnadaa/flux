//! Engines for the smart tools that can use a model: stems, transcripts and text to speech. For each job: the
//! built-in engine (when there is one), the models Flux can download, which one suits this computer, and running
//! the chosen one through the voice engine (see voicemodels.rs; the runner is flux_voice.py).

use crate::api::{err, Api};
use crate::jobs::JobCtx;
use crate::voiceclone::Hardware;
use crate::voicemodels::{self as vm, Catalog, ModelSpec};
use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const ROLES: [&str; 3] = ["stems", "transcript", "tts"];

pub fn role_name(role: &str) -> &'static str {
    match role {
        "stems" => "Stems",
        "transcript" => "Transcripts",
        _ => "Text to speech",
    }
}

/// How many system voices this computer has. Asking the OS starts a process, so it is asked once per run.
fn system_voices() -> usize {
    static N: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *N.get_or_init(|| crate::tools::voices().len())
}

/// The engine that needs no download, if the job has one.
fn builtin(role: &str) -> Option<Value> {
    match role {
        "stems" => Some(json!({
            "id": "spectral", "kind": "builtin", "name": "Built-in split", "tier": "light", "licence": "Flux",
            "note": "Splits by stereo position and harmonic or percussive content. Instant and needs no download, but stems bleed into each other.",
            "installed": true, "runs": "This computer",
        })),
        "tts" => {
            let n = system_voices();
            Some(json!({
                "id": "system", "kind": "builtin", "name": "This computer's voices", "tier": "light", "licence": "System",
                "note": format!("The speech voices that come with your operating system ({n} here). Instant and needs no download; they sound more robotic than a model."),
                "installed": n > 0, "runs": "This computer",
            }))
        }
        _ => None,
    }
}

fn tier_rank(t: &str) -> u8 {
    match t {
        "light" => 0,
        "balanced" => 1,
        "best" => 2,
        // A specialist (Demucs 6 stems): offered, never suggested.
        _ => 3,
    }
}

/// Downloadable models for a job, lightest first. Text to speech can also use Chatterbox Turbo's own built-in
/// voice (no reference clip).
pub fn models_for(c: &Catalog, role: &str) -> Vec<ModelSpec> {
    let mut v: Vec<ModelSpec> = c
        .models
        .iter()
        .filter(|m| m.role == role || (role == "tts" && m.role == "speech" && m.loader == "turbo"))
        .cloned()
        .map(|mut m| {
            if m.role == "speech" {
                m.tier = "best".into();
                m.note = format!(
                    "{} Reads in its own built-in voice; clone a voice on the Voice cloning page.",
                    m.note
                );
            }
            m
        })
        .collect();
    v.sort_by_key(|m| tier_rank(&m.tier));
    v
}

/// The engine that suits this computer, and why.
pub fn suggest(c: &Catalog, role: &str, h: &Hardware) -> (String, String) {
    let ms = models_for(c, role);
    let base = builtin(role)
        .filter(|b| b["installed"] == true)
        .map(|b| b["id"].as_str().unwrap_or("").to_string());
    let fallback = |why: String| match &base {
        Some(b) => (b.clone(), why),
        None => (ms.first().map(|m| m.id.clone()).unwrap_or_default(), why),
    };
    if ms.is_empty() {
        return fallback("Nothing to download for this.".into());
    }
    if h.os == "macos" && h.arch == "x86_64" {
        return fallback("The models need PyTorch 2.6, which isn't made for Intel Macs.".into());
    }
    if h.gpu_vendor == "nvidia" {
        if let Some(m) = ms.iter().rev().find(|m| {
            tier_rank(&m.tier) <= 2 && h.vram_gb >= m.min_vram_gb && h.ram_gb >= m.min_ram_gb
        }) {
            return (
                m.id.clone(),
                format!("Your {} ({:.0} GB) runs it quickly.", h.gpu_name, h.vram_gb),
            );
        }
    }
    // Processor (and Apple silicon, where Demucs can use the graphics cores): the best that stays quick.
    let max_tier = if h.os == "macos" && role == "stems" && h.ram_gb >= 16.0 {
        2
    } else if role == "tts" {
        0
    } else {
        1
    };
    let pick = ms.iter().rev().find(|m| {
        tier_rank(&m.tier) <= max_tier
            && h.ram_gb >= m.min_ram_gb
            && (h.cores >= 8 || tier_rank(&m.tier) == 0)
    });
    match pick {
        Some(m) => (
            m.id.clone(),
            format!(
                "Runs well on your {} with {:.0} GB of memory.",
                if h.os == "macos" { "Mac" } else { "processor" },
                h.ram_gb
            ),
        ),
        None => fallback(format!(
            "This computer has {:.0} GB of memory; the models want more.",
            h.ram_gb
        )),
    }
}

fn engine_ids(c: &Catalog, role: &str) -> Vec<String> {
    builtin(role)
        .map(|b| b["id"].as_str().unwrap_or("").to_string())
        .into_iter()
        .chain(models_for(c, role).into_iter().map(|m| m.id))
        .collect()
}

/// The chosen engine for a job: the saved choice, else the suggestion (see below).
pub fn chosen(api: &Api, role: &str) -> Result<String> {
    let c = vm::catalog(api)?;
    let s = vm::settings(api);
    if let Some(id) = s.engines.get(role) {
        if engine_ids(&c, role).contains(id) {
            return Ok(id.clone());
        }
    }
    // The suggestion once it is downloaded; until then the built-in engine, so the tool works out of the box.
    let sug = suggest(&c, role, crate::voiceclone::hardware()).0;
    let root = vm::models_root(api);
    let have = |id: &str| {
        models_for(&c, role)
            .iter()
            .find(|m| m.id == id)
            .map(|m| vm::model_state(&root, &c, m).installed)
            .unwrap_or(true)
    };
    if have(&sug) {
        return Ok(sug);
    }
    Ok(builtin(role)
        .filter(|b| b["installed"] == true)
        .and_then(|b| b["id"].as_str().map(String::from))
        .unwrap_or(sug))
}

pub fn option(api: &Api, role: &str, key: &str) -> Option<String> {
    vm::settings(api)
        .options
        .get(role)
        .and_then(|o| o.get(key))
        .cloned()
}

fn engine_json(api: &Api, c: &Catalog, root: &Path, m: &ModelSpec, h: &Hardware) -> Value {
    let st = vm::model_state(root, c, m);
    let (fit, why) = vm::fits(m, h);
    json!({
        "id": m.id, "kind": "local", "name": m.name, "tier": m.tier, "licence": m.licence, "note": m.note,
        "size": st.bytes_total, "size_text": vm::gb(st.bytes_total), "have": st.bytes_have, "partial": st.partial,
        "installed": st.installed, "packages": st.packages, "fits": fit, "fits_why": why, "runs": "This computer",
        "voices": m.voices, "languages": m.languages, "language_names": m.language_names, "params": m.params,
        "downloading": vm::active_job(api, root, &m.id),
    })
}

/// `ai.engines`: every job's engines, the choice and the suggestion.
pub fn h_engines(api: &Api, v: &Value) -> Result<Value> {
    let c = vm::catalog(api)?;
    let root = vm::models_root(api);
    let s = vm::settings(api);
    let h = crate::voiceclone::hardware();
    let only = v["role"].as_str();
    let mut roles = vec![];
    for role in ROLES.iter().filter(|r| only.is_none_or(|o| o == **r)) {
        let (sug, why) = suggest(&c, role, h);
        let choice = chosen(api, role)?;
        let mut engines: Vec<Value> = builtin(role).into_iter().collect();
        engines.extend(
            models_for(&c, role)
                .iter()
                .map(|m| engine_json(api, &c, &root, m, h)),
        );
        for e in &mut engines {
            e["suggested"] = json!(e["id"] == sug.as_str());
            e["chosen"] = json!(e["id"] == choice.as_str());
        }
        roles.push(json!({
            "role": role, "name": role_name(role), "choice": choice, "suggested": sug, "why": why,
            "options": s.options.get(*role).cloned().unwrap_or_default(), "engines": engines,
        }));
    }
    let rt = vm::runtime_ready(&root, &c);
    let kind = vm::torch_kind(h);
    let approx = c.runtime.approx_mb.get(kind).copied().unwrap_or(1000) * 1_000_000;
    Ok(json!({
        "roles": roles, "hardware": h, "folder": root.display().to_string(),
        "runtime": {"name": c.runtime.name, "installed": rt.is_some(), "approx_text": format!("about {}", vm::gb(approx))},
    }))
}

/// `ai.engine.set`: choose a job's engine, and its voice or language.
pub fn h_engine_set(api: &Api, v: &Value) -> Result<Value> {
    let role = v["role"].as_str().unwrap_or("");
    if !ROLES.contains(&role) {
        bail!(err("bad_params", "`role` is stems, transcript or tts."));
    }
    let c = vm::catalog(api)?;
    let mut s = vm::settings(api);
    if let Some(e) = v["engine"].as_str() {
        let ids = engine_ids(&c, role);
        if !ids.iter().any(|i| i == e) {
            bail!(err(
                "bad_params",
                format!(
                    "`{e}` isn't an engine for {}. It can be: {}.",
                    role_name(role).to_lowercase(),
                    ids.join(", ")
                )
            ));
        }
        s.engines.insert(role.into(), e.into());
    }
    for key in ["voice", "language"] {
        if let Some(x) = v[key].as_str() {
            s.options
                .entry(role.into())
                .or_default()
                .insert(key.into(), x.into());
        }
    }
    vm::save_settings(api, &s)?;
    h_engines(api, &json!({"role": role}))
}

// ---------------------------------------------------------------- running a downloaded model

fn work_dir(api: &Api) -> Result<PathBuf> {
    let d = vm::models_root(api).join("work");
    std::fs::create_dir_all(&d)?;
    Ok(d)
}

/// A downloaded model for a job, ready to run, or a clear error saying what to download.
pub fn ready(api: &Api, role: &str, id: &str) -> Result<vm::Ready> {
    let c = vm::catalog(api)?;
    let m = models_for(&c, role)
        .into_iter()
        .find(|m| m.id == id)
        .ok_or_else(|| {
            err(
                "not_found",
                format!(
                    "`{id}` isn't a model for {}.",
                    role_name(role).to_lowercase()
                ),
            )
        })?;
    vm::ready_for(api, &m.role, Some(id)).map_err(|e| {
        // Point at the Smart tools settings rather than the cloning page.
        let msg = format!("{e:#}");
        if msg.contains("isn't downloaded") {
            err("not_installed", format!("{} isn't downloaded yet ({}). Download it in Settings, Smart tools, or with ai.model.download.", m.name, vm::gb(m.files.iter().map(|f| f.size).sum())))
        } else {
            e
        }
    })
}

fn run(api: &Api, role: &str, id: &str, op: &str, extra: Value, ctx: &JobCtx) -> Result<Value> {
    let r = ready(api, role, id)?;
    let mut req = vm::request(&r, op, Path::new(""), Path::new(""));
    if let (Some(o), Some(x)) = (req.as_object_mut(), extra.as_object()) {
        for (k, val) in x {
            o.insert(k.clone(), val.clone());
        }
    }
    vm::run_model(&r, req, ctx)
}

fn write_f32(path: &Path, ch: &[&[f32]], sr: u32) -> Result<()> {
    let spec = hound::WavSpec {
        channels: ch.len() as u16,
        sample_rate: sr,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w = hound::WavWriter::create(path, spec)?;
    for i in 0..ch[0].len() {
        for c in ch {
            w.write_sample(c[i])?;
        }
    }
    w.finalize()?;
    Ok(())
}

fn read_stereo(path: &Path) -> Result<(Vec<f32>, Vec<f32>, u32)> {
    let mut rd = hound::WavReader::open(path)?;
    let spec = rd.spec();
    let xs: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => rd.samples::<f32>().collect::<Result<_, _>>()?,
        hound::SampleFormat::Int => {
            let scale = (1i64 << (spec.bits_per_sample - 1)) as f32;
            rd.samples::<i32>()
                .map(|s| s.map(|v| v as f32 / scale))
                .collect::<Result<_, _>>()?
        }
    };
    let ch = spec.channels.max(1) as usize;
    let l: Vec<f32> = xs.iter().step_by(ch).copied().collect();
    let r: Vec<f32> = if ch > 1 {
        xs.iter().skip(1).step_by(ch).copied().collect()
    } else {
        l.clone()
    };
    Ok((l, r, spec.sample_rate))
}

/// Stems from a downloaded model, at the project's rate: (name, (left, right)) in the model's order.
pub fn stems(
    api: &Api,
    id: &str,
    l: &[f32],
    r: &[f32],
    sr: u32,
    ctx: &JobCtx,
) -> Result<Vec<(String, crate::api::Stereo)>> {
    let c = vm::catalog(api)?;
    let m = models_for(&c, "stems")
        .into_iter()
        .find(|m| m.id == id)
        .ok_or_else(|| err("not_found", format!("`{id}` isn't a stems model.")))?;
    let name = m
        .files
        .iter()
        .find_map(|f| f.path.strip_suffix(".yaml"))
        .ok_or_else(|| err("bad_file", "the stems model has no definition file"))?
        .to_string();
    let w = work_dir(api)?;
    let input = w.join(format!("{}-in.wav", ctx.id));
    let out = w.join(format!("{}-stems", ctx.id));
    const MODEL_SR: u32 = 44100;
    let (l2, r2) = (
        crate::dsp::resample(l, sr, MODEL_SR),
        crate::dsp::resample(r, sr, MODEL_SR),
    );
    write_f32(&input, &[&l2, &r2], MODEL_SR)?;
    let res = run(
        api,
        "stems",
        id,
        "stems",
        json!({"input": input, "out": out, "name": name}),
        ctx,
    );
    std::fs::remove_file(&input).ok();
    let res = res.inspect_err(|_| {
        std::fs::remove_dir_all(&out).ok();
    })?;
    let mut v = vec![];
    for s in res["stems"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|x| x.as_str())
    {
        let (sl, sr_, rate) = read_stereo(&out.join(format!("{s}.wav")))?;
        let n = l.len();
        let mut a = crate::dsp::resample(&sl, rate, sr);
        let mut b = crate::dsp::resample(&sr_, rate, sr);
        a.resize(n, 0.0);
        b.resize(n, 0.0);
        v.push((s.to_string(), (a, b)));
    }
    std::fs::remove_dir_all(&out).ok();
    if v.is_empty() {
        bail!(err("failed", "The stems model returned nothing."));
    }
    Ok(v)
}

/// A transcript from a downloaded model: segments with words, times in seconds.
pub fn transcribe(
    api: &Api,
    id: &str,
    l: &[f32],
    r: &[f32],
    sr: u32,
    language: Option<&str>,
    ctx: &JobCtx,
) -> Result<Value> {
    let w = work_dir(api)?;
    let input = w.join(format!("{}-in.wav", ctx.id));
    let out = w.join(format!("{}-transcript.json", ctx.id));
    let mono: Vec<f32> = l.iter().zip(r).map(|(a, b)| 0.5 * (a + b)).collect();
    write_f32(&input, &[&crate::dsp::resample(&mono, sr, 16000)], 16000)?;
    let res = run(
        api,
        "transcript",
        id,
        "transcribe",
        json!({"input": input, "out": out, "language": language.unwrap_or("auto")}),
        ctx,
    );
    std::fs::remove_file(&input).ok();
    res?;
    let t: Value = serde_json::from_slice(&std::fs::read(&out)?)?;
    std::fs::remove_file(&out).ok();
    Ok(t)
}

/// Speech from a downloaded model into `out` (a WAV), in the chosen or given voice.
pub fn speak(
    api: &Api,
    id: &str,
    text: &str,
    voice: Option<&str>,
    out: &Path,
    ctx: &JobCtx,
) -> Result<Value> {
    let c = vm::catalog(api)?;
    let m = models_for(&c, "tts")
        .into_iter()
        .find(|m| m.id == id)
        .ok_or_else(|| err("not_found", format!("`{id}` isn't a text to speech model.")))?;
    let voice = match voice
        .map(String::from)
        .or_else(|| option(api, "tts", "voice"))
    {
        Some(v) if m.voices.is_empty() || m.voices.iter().any(|x| x.id == v) => Some(v),
        _ => m.voices.first().map(|v| v.id.clone()),
    };
    // Chatterbox reads about a sentence or two at a time well; Kokoro splits long text itself.
    let segs: Vec<Value> = if m.loader == "kokoro" {
        vec![json!({"type": "text", "value": text})]
    } else {
        sentences(text, 280)
            .into_iter()
            .map(|s| json!({"type": "text", "value": s}))
            .collect()
    };
    run(
        api,
        "tts",
        id,
        "tts",
        json!({"out": out, "voice": voice, "segments": segs}),
        ctx,
    )
}

/// Text in pieces of at most about `max` characters, split at sentence ends.
fn sentences(text: &str, max: usize) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    let mut cur = String::new();
    for word in text.split_whitespace() {
        if !cur.is_empty() && cur.len() + word.len() + 1 > max {
            out.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
        if word.ends_with(['.', '!', '?']) && cur.len() > max / 2 {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

// ---------------------------------------------------------------- transcripts

fn transcript_path(api: &Api, project: &str) -> PathBuf {
    let safe: String = project
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    api.lock()
        .store
        .root
        .join("Transcripts")
        .join(format!("{safe}.json"))
}

/// Turn a model's transcript into Flux's: every time also as an exact sample position at the project's rate.
pub fn shape_transcript(raw: &Value, sr: u32, engine: &str, offset: i64, hash: u64) -> Value {
    let at = |t: &Value| (t.as_f64().unwrap_or(0.0) * sr as f64).round() as i64 + offset;
    let mut words = vec![];
    let mut segs = vec![];
    for sg in raw["segments"].as_array().into_iter().flatten() {
        let (a, b) = (at(&sg["start"]), at(&sg["end"]));
        segs.push(json!({"text": sg["text"], "at": a, "len": (b - a).max(0)}));
        for w in sg["words"].as_array().into_iter().flatten() {
            let (a, b) = (at(&w["start"]), at(&w["end"]));
            words.push(json!({"word": w["word"].as_str().unwrap_or("").trim(), "at": a, "len": (b - a).max(0), "p": w["p"]}));
        }
    }
    json!({
        "engine": engine, "language": raw["language"], "sample_rate": sr, "project_hash": hash.to_string(),
        "made": crate::project::now_secs(), "text": segs.iter().filter_map(|s| s["text"].as_str()).collect::<Vec<_>>().join(" "),
        "segments": segs, "words": words,
    })
}

pub fn save_transcript(api: &Api, project: &str, t: &Value) -> Result<()> {
    let p = transcript_path(api, project);
    std::fs::create_dir_all(p.parent().unwrap())?;
    crate::store::atomic_write(&p, serde_json::to_string(t)?.as_bytes())
}

pub fn load_transcript(api: &Api, project: &str) -> Option<Value> {
    serde_json::from_slice(&std::fs::read(transcript_path(api, project)).ok()?).ok()
}

pub fn delete_transcript(api: &Api, project: &str) {
    std::fs::remove_file(transcript_path(api, project)).ok();
}

const FILLERS: [&str; 12] = [
    "um", "umm", "uh", "uhh", "uhm", "erm", "er", "ah", "hmm", "mm", "mhm", "eh",
];

fn bare(w: &str) -> String {
    w.chars()
        .filter(|c| c.is_alphanumeric() || *c == '\'')
        .collect::<String>()
        .to_lowercase()
}

/// Filler words (um, uh, ...) and stutters (a word said twice in a row), with exact positions.
pub fn fillers(t: &Value) -> Vec<Value> {
    let words: Vec<&Value> = t["words"].as_array().into_iter().flatten().collect();
    let mut out = vec![];
    for (i, w) in words.iter().enumerate() {
        let b = bare(w["word"].as_str().unwrap_or(""));
        if b.is_empty() {
            continue;
        }
        if FILLERS.contains(&b.as_str()) {
            out.push(json!({"word": w["word"], "kind": "filler", "at": w["at"], "len": w["len"]}));
        } else if i + 1 < words.len() && bare(words[i + 1]["word"].as_str().unwrap_or("")) == b {
            out.push(json!({"word": w["word"], "kind": "repeat", "at": w["at"], "len": w["len"]}));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hw(os: &str, arch: &str, ram: f32, gpu: &str, vram: f32, cores: usize) -> Hardware {
        Hardware {
            os: os.into(),
            arch: arch.into(),
            cores,
            ram_gb: ram,
            gpu_vendor: gpu.into(),
            gpu_name: if gpu == "nvidia" {
                "GeForce".into()
            } else {
                String::new()
            },
            vram_gb: vram,
        }
    }

    #[test]
    fn suggestions_follow_the_computer() {
        let c: Catalog = serde_json::from_str(include_str!("voice_models.json")).unwrap();
        let big = hw("windows", "x86_64", 32.0, "nvidia", 8.0, 16);
        assert_eq!(suggest(&c, "stems", &big).0, "demucs-ft");
        assert_eq!(suggest(&c, "transcript", &big).0, "whisper-large-v3-turbo");
        assert_eq!(suggest(&c, "tts", &big).0, "chatterbox-turbo");
        let laptop = hw("windows", "x86_64", 16.0, "intel", 0.0, 8);
        assert_eq!(suggest(&c, "stems", &laptop).0, "demucs");
        assert_eq!(suggest(&c, "transcript", &laptop).0, "whisper-small");
        assert_eq!(suggest(&c, "tts", &laptop).0, "kokoro");
        let small = hw("linux", "x86_64", 3.0, "", 0.0, 2);
        assert_eq!(suggest(&c, "stems", &small).0, "spectral");
        assert_eq!(suggest(&c, "transcript", &small).0, "whisper-base");
        let mac = hw("macos", "aarch64", 16.0, "apple", 0.0, 10);
        assert_eq!(suggest(&c, "stems", &mac).0, "demucs-ft");
        assert_eq!(suggest(&c, "transcript", &mac).0, "whisper-small");
        let intel_mac = hw("macos", "x86_64", 16.0, "intel", 0.0, 8);
        assert_eq!(suggest(&c, "stems", &intel_mac).0, "spectral");
        let small_gpu = hw("windows", "x86_64", 16.0, "nvidia", 2.0, 8);
        assert_eq!(suggest(&c, "transcript", &small_gpu).0, "whisper-small");
    }

    #[test]
    fn fillers_and_repeats_are_found_at_exact_samples() {
        let raw = json!({"language": "en", "segments": [{"start": 0.0, "end": 3.0, "text": "So um I I think, uh, yes.",
            "words": [{"start": 0.0, "end": 0.2, "word": " So"}, {"start": 0.25, "end": 0.5, "word": " um"}, {"start": 0.6, "end": 0.7, "word": " I"},
                      {"start": 0.8, "end": 0.9, "word": " I"}, {"start": 1.0, "end": 1.4, "word": " think,"}, {"start": 1.5, "end": 1.75, "word": " uh,"},
                      {"start": 2.0, "end": 2.5, "word": " yes."}]}]});
        let t = shape_transcript(&raw, 48000, "whisper-base", 480, 7);
        assert_eq!(t["words"][1]["at"], 12000 + 480);
        assert_eq!(t["words"][1]["len"], 12000);
        let f = fillers(&t);
        let got: Vec<(String, String, i64)> = f
            .iter()
            .map(|x| {
                (
                    x["word"].as_str().unwrap().to_string(),
                    x["kind"].as_str().unwrap().to_string(),
                    x["at"].as_i64().unwrap(),
                )
            })
            .collect();
        assert_eq!(
            got,
            vec![
                ("um".to_string(), "filler".to_string(), 12480),
                ("I".to_string(), "repeat".to_string(), 29280),
                ("uh,".to_string(), "filler".to_string(), 72480)
            ]
        );
    }

    #[test]
    fn sentences_stay_short_and_keep_every_word() {
        let t =
            "One two three. Four five six seven! Eight nine ten eleven twelve thirteen fourteen.";
        let v = sentences(t, 30);
        assert!(v.iter().all(|s| s.len() <= 40), "{v:?}");
        assert_eq!(v.join(" "), t);
    }
}
