//! Voice cloning: which local model suits this computer, consent-gated voice profiles, and the gate itself.
//!
//! Flux only clones a voice with recorded consent. A profile holds the consent statement, a recording of the
//! voice owner saying it, and a reference clip, all on this computer (`<data>/Voices/<id>/`). Nothing is sent
//! anywhere. Speaking in a cloned voice needs a model installed under `<data>/Models/<model>/`; this build does
//! not download one, so `clone.speak` says exactly what it would use and why it can't yet.
//!
//! Models are limited to permissive licences (PLAN.md section 4): Chatterbox (MIT) and OpenVoice V2 with MeloTTS
//! (MIT). Which one is chosen depends on the graphics chip and memory of the computer.

use crate::api::{err, Api};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// What this computer has, as far as Flux can tell without special permissions.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct Hardware {
    pub os: String,
    pub arch: String,
    pub cores: usize,
    pub ram_gb: f32,
    /// `nvidia`, `apple`, `amd`, `intel` or empty.
    pub gpu_vendor: String,
    pub gpu_name: String,
    /// Dedicated video memory in GB (0 when unknown or shared).
    pub vram_gb: f32,
}

/// A local model Flux can clone voices with.
pub struct Model {
    pub id: &'static str,
    pub name: &'static str,
    pub licence: &'static str,
    pub source: &'static str,
    /// Approximate download, in GB. The exact size is shown before any download.
    pub size_gb: f32,
    pub languages: u32,
    pub needs: &'static str,
    pub note: &'static str,
}

pub const MODELS: &[Model] = &[
    Model {
        id: "chatterbox-turbo",
        name: "Chatterbox Turbo",
        licence: "MIT",
        source: "https://huggingface.co/ResembleAI/chatterbox-turbo",
        size_gb: 1.4,
        languages: 1,
        needs: "Best on an NVIDIA graphics chip with 4 GB of video memory or Apple silicon; also runs on a processor with 8 GB of memory, more slowly.",
        note: "The most expressive: clones a voice from one 5 to 20 second clip and performs [laugh], [chuckle], [sigh], [gasp], [cough] and other sounds written in the script, and says hmm, um and uh as written. English. Adds an inaudible watermark marking speech as synthetic.",
    },
    Model {
        id: "chatterbox",
        name: "Chatterbox Multilingual (with voice conversion)",
        licence: "MIT",
        source: "https://huggingface.co/ResembleAI/chatterbox",
        size_gb: 3.2,
        languages: 23,
        needs: "An NVIDIA graphics chip with 6 GB of video memory, or Apple silicon with 16 GB of memory.",
        note: "23 languages and an emotion control. Its voice converter turns your own spoken take into the cloned voice, keeping every hmm, laugh and pause exactly where you made it.",
    },
    Model {
        id: "openvoice-v2",
        name: "OpenVoice V2 with MeloTTS",
        licence: "MIT",
        source: "https://huggingface.co/myshell-ai/OpenVoiceV2",
        size_gb: 0.6,
        languages: 6,
        needs: "Any 64-bit processor with 8 GB of memory. No graphics chip needed.",
        note: "The lightest. Copies the tone of a voice onto clear base speech. No laugh or sigh sounds.",
    },
];

/// The model for a computer, how it would run, and why.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Pick {
    pub model: Option<&'static str>,
    /// `cuda`, `mps` or `cpu`.
    pub device: &'static str,
    /// Rough speed: `fast` (faster than real time), `ok` (about real time) or `slow`.
    pub speed: &'static str,
    /// The model that turns your own spoken take into the cloned voice, if this computer can run it.
    pub convert: Option<&'static str>,
    pub reason: String,
}

/// Choose a model from the hardware. Pure, so it is tested for every kind of computer.
/// Chatterbox Turbo is the first choice wherever it fits, because it performs laughs, sighs and hesitations.
pub fn recommend(h: &Hardware) -> Pick {
    let apple = h.os == "macos" && h.arch == "aarch64";
    if h.ram_gb > 0.0 && h.ram_gb < 7.5 {
        return Pick {
            model: None,
            device: "cpu",
            speed: "slow",
            convert: None,
            reason: format!(
                "This computer has {:.0} GB of memory. Voice cloning needs at least 8 GB.",
                h.ram_gb
            ),
        };
    }
    let big = (h.gpu_vendor == "nvidia" && h.vram_gb >= 5.5) || (apple && h.ram_gb >= 15.5);
    let convert = big.then_some("chatterbox");
    if h.gpu_vendor == "nvidia" && h.vram_gb >= 3.5 {
        return Pick {
            model: Some("chatterbox-turbo"),
            device: "cuda",
            speed: "fast",
            convert,
            reason: format!("Your {} has {:.0} GB of video memory: the most expressive model runs faster than real time.", h.gpu_name, h.vram_gb),
        };
    }
    if apple {
        return Pick {
            model: Some("chatterbox-turbo"),
            device: "mps",
            speed: "ok",
            convert,
            reason: format!("Apple silicon with {:.0} GB of memory runs the most expressive model on its graphics cores.", h.ram_gb),
        };
    }
    let gpu = if h.gpu_vendor == "nvidia" {
        format!(
            "Your {} has {:.0} GB of video memory, under the 4 GB it needs, so ",
            h.gpu_name, h.vram_gb
        )
    } else {
        "No graphics chip Flux can use for this, so ".to_string()
    };
    Pick {
        model: Some("chatterbox-turbo"),
        device: "cpu",
        speed: if h.cores >= 8 { "ok" } else { "slow" },
        convert: None,
        reason: format!("{gpu}the most expressive model runs on the processor. It works, more slowly. Turning your own recording into the voice needs a graphics chip."),
    }
}

/// Sounds a script can ask for. Written as [laugh], (laughs), *laughs*, {laugh} or <laugh>; all mean the same.
pub const TAGS: &[(&str, &[&str])] = &[
    ("laugh", &["laugh", "laughs", "laughing", "lol", "haha"]),
    ("chuckle", &["chuckle", "chuckles", "giggle", "giggles"]),
    ("sigh", &["sigh", "sighs"]),
    ("gasp", &["gasp", "gasps"]),
    ("cough", &["cough", "coughs"]),
    ("clear throat", &["clear throat", "clears throat", "throat"]),
    ("sniff", &["sniff", "sniffs"]),
    ("groan", &["groan", "groans"]),
    ("shush", &["shush", "shh"]),
];

/// A script, split into what is said and the sounds between.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "lowercase")]
pub enum Part {
    Text(String),
    Sound(&'static str),
}

/// Parse a script with placeholders. Hesitations like hmm, um and uh are words and stay in the text, so the voice
/// says them. An unknown tag in [] {} or <> is refused with the list of known ones, because models guess badly.
/// Text in () or ** that isn't a known sound is kept as words.
pub fn parse_script(s: &str) -> Result<Vec<Part>> {
    if s.chars().count() > 5000 {
        bail!(err(
            "bad_params",
            "That script is too long. Keep it under 5000 characters per call."
        ));
    }
    let mut out: Vec<Part> = vec![];
    let mut text = String::new();
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        let close = match cs[i] {
            '[' => Some(']'),
            '(' => Some(')'),
            '{' => Some('}'),
            '*' => Some('*'),
            '<' => Some('>'),
            _ => None,
        };
        if let Some(cl) = close {
            if let Some(j) = cs[i + 1..].iter().position(|&c| c == cl).map(|k| i + 1 + k) {
                let inner: String = cs[i + 1..j]
                    .iter()
                    .collect::<String>()
                    .trim()
                    .to_lowercase();
                match TAGS
                    .iter()
                    .find(|(_, al)| al.contains(&inner.as_str()))
                    .map(|(t, _)| *t)
                {
                    Some(t) => {
                        if !text.trim().is_empty() {
                            out.push(Part::Text(text.trim().to_string()));
                        }
                        text.clear();
                        out.push(Part::Sound(t));
                        i = j + 1;
                        continue;
                    }
                    None if matches!(cs[i], '[' | '{' | '<') => {
                        let known: Vec<&str> = TAGS.iter().map(|t| t.0).collect();
                        bail!(err(
                            "bad_params",
                            format!("Flux doesn't know the sound [{inner}]. Use one of: {}. Hesitations like hmm and um are words: write them as they sound.", known.join(", "))
                        ));
                    }
                    None => {}
                }
            }
        }
        text.push(cs[i]);
        i += 1;
    }
    if !text.trim().is_empty() {
        out.push(Part::Text(text.trim().to_string()));
    }
    if out.is_empty() {
        bail!(err("bad_params", "The script is empty."));
    }
    Ok(out)
}

/// The script as Chatterbox Turbo reads it: words with [tag] markers.
pub fn turbo_text(parts: &[Part]) -> String {
    parts
        .iter()
        .map(|p| match p {
            Part::Text(t) => t.clone(),
            Part::Sound(s) => format!("[{s}]"),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let mut c = std::process::Command::new(cmd);
    c.args(args)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // no console window
    }
    let o = c.output().ok()?;
    o.status
        .success()
        .then(|| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// Parse `nvidia-smi --query-gpu=name,memory.total --format=csv,noheader,nounits`: the largest card wins.
pub fn parse_nvidia_smi(s: &str) -> Option<(String, f32)> {
    s.lines()
        .filter_map(|l| {
            let (n, m) = l.rsplit_once(',')?;
            Some((n.trim().to_string(), m.trim().parse::<f32>().ok()? / 1024.0))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
}

fn probe() -> Hardware {
    let mut h = Hardware {
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        cores: std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1),
        ..Default::default()
    };
    let gb = |b: f64| (b / 1_073_741_824.0 * 10.0).round() as f32 / 10.0;
    if cfg!(target_os = "linux") {
        if let Ok(m) = std::fs::read_to_string("/proc/meminfo") {
            if let Some(kb) = m
                .lines()
                .find(|l| l.starts_with("MemTotal:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse::<f64>().ok())
            {
                h.ram_gb = gb(kb * 1024.0);
            }
        }
    } else if cfg!(target_os = "macos") {
        if let Some(b) = run("sysctl", &["-n", "hw.memsize"]).and_then(|s| s.parse::<f64>().ok()) {
            h.ram_gb = gb(b);
        }
        if h.arch == "aarch64" {
            h.gpu_vendor = "apple".into();
            h.gpu_name = run("sysctl", &["-n", "machdep.cpu.brand_string"])
                .unwrap_or_else(|| "Apple silicon".into());
        }
    } else if cfg!(windows) {
        if let Some(b) = run(
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                "(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory",
            ],
        )
        .and_then(|s| s.parse::<f64>().ok())
        {
            h.ram_gb = gb(b);
        }
        if let Some(n) = run(
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                "(Get-CimInstance Win32_VideoController | Select-Object -First 1).Name",
            ],
        ) {
            let l = n.to_lowercase();
            h.gpu_vendor = if l.contains("nvidia") {
                "nvidia"
            } else if l.contains("amd") || l.contains("radeon") {
                "amd"
            } else if l.contains("intel") {
                "intel"
            } else {
                ""
            }
            .into();
            h.gpu_name = n;
        }
    }
    if let Some((name, vram)) = run(
        "nvidia-smi",
        &[
            "--query-gpu=name,memory.total",
            "--format=csv,noheader,nounits",
        ],
    )
    .as_deref()
    .and_then(parse_nvidia_smi)
    {
        h.gpu_vendor = "nvidia".into();
        h.gpu_name = name;
        h.vram_gb = (vram * 10.0).round() / 10.0;
    }
    h
}

/// The hardware, probed once per run (it takes a moment on Windows).
pub fn hardware() -> &'static Hardware {
    static H: OnceLock<Hardware> = OnceLock::new();
    H.get_or_init(probe)
}

// ---------------------------------------------------------------- profiles and consent

/// What the voice owner reads aloud. Their recording of it is kept with the profile.
pub fn consent_statement(name: &str) -> String {
    format!("I am {name}. I agree that Flux may make a copy of my voice on this computer, and I know anything it says in my voice will be labelled synthetic.")
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    /// `me` or `someone-else`.
    pub owner: String,
    pub statement: String,
    pub created: u64,
    pub reference_seconds: f32,
    pub consent_seconds: f32,
    pub reference_rms_db: f32,
}

fn voices_dir(api: &Api) -> PathBuf {
    api.lock().store.root.join("Voices")
}

fn os(v: &Value, k: &str) -> Option<String> {
    v.get(k)
        .and_then(|x| x.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Decode, fold to mono at 24 kHz, and measure. Rejects silence, wild lengths and anything that isn't audio.
fn load_clip(path: &str, what: &str, min_s: f32, max_s: f32) -> Result<(Vec<f32>, f32, f32)> {
    let p = Path::new(path);
    let meta = std::fs::metadata(p)
        .map_err(|_| err("not_found", format!("The {what} file isn't there.")))?;
    if meta.len() > 200 * 1024 * 1024 {
        bail!(err("bad_file", format!("The {what} file is too big.")));
    }
    let d = crate::media::decode_file(p).map_err(|_| {
        err(
            "bad_file",
            format!("Flux couldn't read the {what} as audio."),
        )
    })?;
    let n = d.data.first().map(|c| c.len()).unwrap_or(0);
    let mono: Vec<f32> = (0..n)
        .map(|i| d.data.iter().map(|c| c[i]).sum::<f32>() / d.data.len().max(1) as f32)
        .collect();
    let mono = crate::dsp::resample(&mono, d.rate, 24000);
    let secs = mono.len() as f32 / 24000.0;
    if secs < min_s || secs > max_s {
        bail!(err(
            "bad_params",
            format!(
                "The {what} is {secs:.1} s long. It needs to be {min_s:.0} to {max_s:.0} seconds."
            )
        ));
    }
    let rms = (mono.iter().map(|x| x * x).sum::<f32>() / mono.len().max(1) as f32).sqrt();
    let db = crate::dsp::lin_to_db(rms.max(1e-9));
    if db < -50.0 {
        bail!(err(
            "bad_params",
            format!("The {what} is nearly silent ({db:.0} dB). Record closer to the microphone.")
        ));
    }
    Ok((mono, secs, (db * 10.0).round() / 10.0))
}

fn write_wav(path: &Path, x: &[f32]) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 24000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec)?;
    for s in x {
        w.write_sample((s.clamp(-1.0, 1.0) * 32767.0) as i16)?;
    }
    w.finalize()?;
    Ok(())
}

fn slug(s: &str) -> String {
    let t: String = s
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let t = t
        .split('-')
        .filter(|x| !x.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if t.is_empty() {
        "voice".into()
    } else {
        t.chars().take(32).collect()
    }
}

pub fn profiles(api: &Api) -> Vec<Profile> {
    let mut out: Vec<Profile> = std::fs::read_dir(voices_dir(api))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            serde_json::from_slice(&std::fs::read(e.path().join("profile.json")).ok()?).ok()
        })
        .collect();
    out.sort_by_key(|p| p.created);
    out
}

fn installed(api: &Api, model: &str) -> bool {
    api.lock()
        .store
        .root
        .join("Models")
        .join(model)
        .join("manifest.json")
        .exists()
}

// ---------------------------------------------------------------- methods

fn model_json(m: &Model) -> Value {
    json!({"id": m.id, "name": m.name, "licence": m.licence, "source": m.source, "size_gb": m.size_gb, "languages": m.languages, "needs": m.needs, "note": m.note})
}

pub fn h_status(api: &Api, _: &Value) -> Result<Value> {
    let h = hardware();
    let pick = recommend(h);
    let ready = pick.model.map(|m| installed(api, m)).unwrap_or(false);
    let why = match (pick.model, ready) {
        (None, _) => pick.reason.clone(),
        (Some(_), true) => "Ready.".to_string(),
        (Some(m), false) => {
            let mm = MODELS.iter().find(|x| x.id == m).unwrap();
            format!("Flux would use {} ({} licence, about {:.1} GB). This build doesn't download models yet, so cloned speech isn't available. Your voice profiles and consent recordings are kept and will work once it is.", mm.name, mm.licence, mm.size_gb)
        }
    };
    Ok(json!({
        "available": ready, "why": why, "hardware": h, "pick": pick,
        "models": MODELS.iter().map(model_json).collect::<Vec<_>>(),
        "profiles": profiles(api).len(),
        "statement_example": consent_statement("your name"),
    }))
}

pub fn h_models(_: &Api, _: &Value) -> Result<Value> {
    Ok(
        json!({"models": MODELS.iter().map(model_json).collect::<Vec<_>>(), "pick": recommend(hardware()), "hardware": hardware()}),
    )
}

pub fn h_profiles(api: &Api, _: &Value) -> Result<Value> {
    Ok(serde_json::to_value(profiles(api))?)
}

/// Create a voice profile. Refused unless the consent fields are all there and both recordings are real audio.
pub fn h_profile_create(api: &Api, v: &Value) -> Result<Value> {
    let name =
        os(v, "name").ok_or_else(|| err("bad_params", "Say whose voice this is (`name`)."))?;
    if name.chars().count() > 60 {
        bail!(err("bad_params", "That name is too long."));
    }
    let owner = os(v, "owner").unwrap_or_default();
    if owner != "me" && owner != "someone-else" {
        bail!(err(
            "consent_required",
            "Say whose voice it is: `owner` is `me` or `someone-else`."
        ));
    }
    if v.get("consent").and_then(|x| x.as_bool()) != Some(true) {
        bail!(err("consent_required", "Flux only clones a voice when its owner agrees. Set `consent` to true only if they have."));
    }
    let consent_audio = os(v, "consent_audio").ok_or_else(|| {
        err(
            "consent_required",
            format!(
                "Record the voice owner reading this, and pass it as `consent_audio`: \"{}\"",
                consent_statement(&name)
            ),
        )
    })?;
    let reference = os(v, "reference").ok_or_else(|| {
        err(
            "bad_params",
            "Pass a 5 to 60 second clip of the voice as `reference`.",
        )
    })?;
    let (c, cs, _) = load_clip(&consent_audio, "consent recording", 3.0, 60.0)?;
    let (r, rs, rdb) = load_clip(&reference, "voice clip", 5.0, 60.0)?;
    let base = voices_dir(api);
    let mut id = slug(&name);
    let mut n = 2;
    while base.join(&id).exists() {
        id = format!("{}-{n}", slug(&name));
        n += 1;
    }
    let dir = base.join(&id);
    std::fs::create_dir_all(&dir)?;
    write_wav(&dir.join("consent.wav"), &c)?;
    write_wav(&dir.join("reference.wav"), &r)?;
    let p = Profile {
        id,
        name: name.clone(),
        owner,
        statement: consent_statement(&name),
        created: crate::project::now_secs(),
        reference_seconds: (rs * 10.0).round() / 10.0,
        consent_seconds: (cs * 10.0).round() / 10.0,
        reference_rms_db: rdb,
    };
    let tmp = dir.join("profile.json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(&p)?)?;
    std::fs::rename(&tmp, dir.join("profile.json"))?;
    Ok(serde_json::to_value(p)?)
}

pub fn h_profile_delete(api: &Api, v: &Value) -> Result<Value> {
    let id = os(v, "id").ok_or_else(|| err("bad_params", "`id` is required"))?;
    if id.contains(['/', '\\', '.']) {
        bail!(err("bad_params", "That isn't a voice id."));
    }
    let dir = voices_dir(api).join(&id);
    if !dir.join("profile.json").exists() {
        bail!(err("not_found", "No voice with that id."));
    }
    std::fs::remove_dir_all(&dir)?;
    Ok(json!({"deleted": id}))
}

pub fn h_tags(_: &Api, _: &Value) -> Result<Value> {
    Ok(json!({
        "tags": TAGS.iter().map(|(t, al)| json!({"tag": t, "also": al})).collect::<Vec<_>>(),
        "how": "Write sounds in the script as [laugh], (laughs), *sighs* or {cough}. Write hesitations as words: hmm, um, uh, mm-hmm. To keep your own timing exactly, record yourself and use clone.convert.",
        "example": "So I opened the box [laugh] and, hmm, it was empty. [sigh] Of course it was.",
    }))
}

/// Check a script without speaking it: what will be said and which sounds go where.
pub fn h_script(_: &Api, v: &Value) -> Result<Value> {
    let text = os(v, "text").ok_or_else(|| err("bad_params", "`text` is required"))?;
    let parts = parse_script(&text)?;
    Ok(
        json!({"parts": parts, "model_text": turbo_text(&parts), "sounds": parts.iter().filter(|p| matches!(p, Part::Sound(_))).count()}),
    )
}

fn need_voice(api: &Api, v: &Value) -> Result<String> {
    let id = os(v, "voice").ok_or_else(|| {
        err(
            "bad_params",
            "`voice` is required (an id from clone.profiles).",
        )
    })?;
    if !profiles(api).iter().any(|p| p.id == id) {
        bail!(err(
            "consent_required",
            "There is no consented voice with that id. Create one with clone.profile.create."
        ));
    }
    Ok(id)
}

fn not_ready(api: &Api, model: Option<&str>) -> anyhow::Error {
    match model {
        Some(m) if installed(api, m) => err(
            "unavailable",
            "The model is installed, but this build can't run it yet.",
        ),
        _ => err(
            "unavailable",
            h_status(api, &Value::Null)
                .ok()
                .and_then(|s| s["why"].as_str().map(String::from))
                .unwrap_or_else(|| "Voice cloning isn't available.".into()),
        ),
    }
}

/// Speak a script in a consented voice. The script is checked first, so mistakes show even before a model is ready.
pub fn h_speak(api: &Api, v: &Value) -> Result<Value> {
    need_voice(api, v)?;
    let text = os(v, "text").ok_or_else(|| err("bad_params", "`text` is required"))?;
    parse_script(&text)?;
    Err(not_ready(api, recommend(hardware()).model))
}

/// Turn your own spoken take into the cloned voice, keeping its timing, laughs and hesitations.
pub fn h_convert(api: &Api, v: &Value) -> Result<Value> {
    need_voice(api, v)?;
    let path = os(v, "path").ok_or_else(|| {
        err(
            "bad_params",
            "`path` is required: your recording to turn into the voice.",
        )
    })?;
    load_clip(&path, "recording", 0.5, 600.0)?;
    let pick = recommend(hardware());
    if pick.convert.is_none() {
        bail!(err("unavailable", "Turning a recording into the voice needs an NVIDIA graphics chip with 6 GB of video memory or Apple silicon with 16 GB of memory. Write a script with [laugh] and other sounds instead."));
    }
    Err(not_ready(api, pick.convert))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hw(os: &str, arch: &str, ram: f32, vendor: &str, vram: f32) -> Hardware {
        Hardware {
            os: os.into(),
            arch: arch.into(),
            cores: 8,
            ram_gb: ram,
            gpu_vendor: vendor.into(),
            gpu_name: "Test GPU".into(),
            vram_gb: vram,
        }
    }

    #[test]
    fn the_model_follows_the_computer() {
        let p = recommend(&hw("windows", "x86_64", 32.0, "nvidia", 8.0));
        assert_eq!(
            (p.model, p.device, p.speed, p.convert),
            (Some("chatterbox-turbo"), "cuda", "fast", Some("chatterbox"))
        );
        let p = recommend(&hw("windows", "x86_64", 16.0, "nvidia", 4.0));
        assert_eq!(
            (p.model, p.device, p.convert),
            (Some("chatterbox-turbo"), "cuda", None)
        );
        let p = recommend(&hw("windows", "x86_64", 16.0, "nvidia", 2.0));
        assert_eq!((p.model, p.device), (Some("chatterbox-turbo"), "cpu"));
        assert!(p.reason.contains("2 GB"));
        let p = recommend(&hw("macos", "aarch64", 16.0, "apple", 0.0));
        assert_eq!((p.device, p.convert), ("mps", Some("chatterbox")));
        let p = recommend(&hw("macos", "aarch64", 8.0, "apple", 0.0));
        assert_eq!((p.device, p.convert), ("mps", None));
        assert_eq!(
            recommend(&hw("macos", "x86_64", 16.0, "intel", 0.0)).device,
            "cpu"
        );
        assert_eq!(
            recommend(&hw("linux", "x86_64", 16.0, "amd", 0.0)).device,
            "cpu"
        );
        assert_eq!(recommend(&hw("linux", "x86_64", 4.0, "", 0.0)).model, None);
        assert!(recommend(&hw("linux", "x86_64", 4.0, "", 0.0))
            .reason
            .contains("4 GB"));
        assert!(MODELS
            .iter()
            .all(|m| m.licence == "MIT" || m.licence == "Apache-2.0"));
    }

    #[test]
    fn scripts_keep_hesitations_as_words_and_turn_placeholders_into_sounds() {
        let p = parse_script(
            "So I opened it [laugh] and, hmm, it was (sighs) empty. *giggles* Um. {clear throat}",
        )
        .unwrap();
        assert_eq!(
            p,
            vec![
                Part::Text("So I opened it".into()),
                Part::Sound("laugh"),
                Part::Text("and, hmm, it was".into()),
                Part::Sound("sigh"),
                Part::Text("empty.".into()),
                Part::Sound("chuckle"),
                Part::Text("Um.".into()),
                Part::Sound("clear throat"),
            ]
        );
        assert_eq!(
            turbo_text(&p),
            "So I opened it [laugh] and, hmm, it was [sigh] empty. [chuckle] Um. [clear throat]"
        );
        // Parentheses that aren't a sound stay as words.
        assert_eq!(
            parse_script("It was (mostly) fine").unwrap(),
            vec![Part::Text("It was (mostly) fine".into())]
        );
        // Unknown bracket tags are refused, with the list.
        let e = parse_script("hello [yodel]").unwrap_err().to_string();
        assert!(e.contains("yodel") && e.contains("laugh"), "{e}");
        assert!(parse_script("   ").is_err());
        assert!(parse_script(&"a".repeat(5001)).is_err());
        // Unclosed brackets are just text.
        assert_eq!(
            parse_script("wait [laugh").unwrap(),
            vec![Part::Text("wait [laugh".into())]
        );
    }

    #[test]
    fn nvidia_smi_output_parses_and_the_biggest_card_wins() {
        assert_eq!(
            parse_nvidia_smi("NVIDIA GeForce RTX 3060, 12288\nNVIDIA T400, 2048\n"),
            Some(("NVIDIA GeForce RTX 3060".into(), 12.0))
        );
        assert_eq!(parse_nvidia_smi("garbage"), None);
        assert_eq!(parse_nvidia_smi(""), None);
    }

    #[test]
    fn the_probe_reports_this_computer() {
        let h = hardware();
        assert!(h.cores >= 1);
        assert_eq!(h.os, std::env::consts::OS);
    }
}
