//! Voice cloning: which local model suits this computer, consent-gated voice profiles, scripts, and speaking.
//!
//! Flux only clones a voice with recorded consent. A profile holds the consent statement, a recording of the
//! voice owner saying it, and a reference clip, all on this computer (`<data>/Voices/<id>/`). Nothing is sent
//! anywhere. Models (Chatterbox, MIT) are downloaded, chosen and run by `voicemodels.rs`; `clone.speak` and
//! `clone.convert` hand it the script pieces or the recording and label what comes back as synthetic.

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
    if h.os == "macos" && !apple {
        return Pick {
            model: None,
            device: "cpu",
            speed: "slow",
            convert: None,
            reason: "Voice cloning needs PyTorch 2.6, which isn't made for Intel Macs.".into(),
        };
    }
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
    let convert = big.then_some("chatterbox-vc");
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

/// Hesitations a script can ask for as placeholders: [hmm], (um), *uh*. The voice says them as words, in the cloned
/// voice, so they sound like the speaker rather than a sound effect. Written plainly (hmm, um) they work the same.
pub const HESITATIONS: &[(&str, &[&str])] = &[
    ("hmm", &["hmm", "hm", "hmmm", "mm", "mmm", "thinking"]),
    ("um", &["um", "umm", "erm"]),
    ("uh", &["uh", "uhh", "er", "err"]),
    ("mm-hmm", &["mm-hmm", "mhm", "uh-huh", "agree"]),
    ("ah", &["ah", "ahh", "aah"]),
    ("oh", &["oh", "ohh", "ooh"]),
    ("huh", &["huh"]),
    ("uh-oh", &["uh-oh"]),
    ("well", &["well"]),
];

/// Plain words that count as hesitations when they stand alone in the text.
const HESITATION_WORDS: &[&str] = &[
    "hmm", "hm", "hmmm", "mm", "mmm", "um", "umm", "erm", "uh", "uhh", "er", "mm-hmm", "mhm",
    "uh-huh", "ah", "ahh", "huh",
];

/// Longest pause a script can ask for, in milliseconds.
pub const MAX_PAUSE_MS: u32 = 5000;

/// A script, split into what is said, the hesitations, the sounds and the pauses between.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "lowercase")]
pub enum Part {
    Text(String),
    Hesitation(&'static str),
    Sound(&'static str),
    Pause(u32),
}

/// Read a pause placeholder: pause, beat, long pause, pause 2s, pause 300ms, 1.5s.
fn pause_ms(inner: &str) -> Option<u32> {
    match inner {
        "pause" | "short pause" => return Some(500),
        "beat" => return Some(300),
        "long pause" => return Some(1200),
        _ => {}
    }
    let rest = inner.strip_prefix("pause").map(str::trim).unwrap_or(inner);
    let (num, scale) = if let Some(n) = rest.strip_suffix("ms") {
        (n, 1.0)
    } else if let Some(n) = rest
        .strip_suffix("seconds")
        .or_else(|| rest.strip_suffix("second"))
    {
        (n, 1000.0)
    } else {
        (rest.strip_suffix('s')?, 1000.0)
    };
    let v: f64 = num.trim().parse().ok()?;
    (v.is_finite() && v > 0.0).then(|| ((v * scale).round() as u32).clamp(50, MAX_PAUSE_MS))
}

/// Parse a script with placeholders. Sounds ([laugh], (sighs)) are performed by the model. Hesitations ([hmm], [um])
/// and plain hmm, um, uh are said as words in the voice. Pauses ([pause], [pause 2s]) become silence. An unknown tag
/// in [] {} or <> is refused with the list of known ones, because models guess badly. Text in () or ** that isn't a
/// known placeholder is kept as words.
pub fn parse_script(s: &str) -> Result<Vec<Part>> {
    if s.chars().count() > 5000 {
        bail!(err(
            "bad_params",
            "That script is too long. Keep it under 5000 characters per call."
        ));
    }
    parse_long(s)
}

/// `parse_script` without its length limit, for text to speech, which has always taken long text.
fn parse_long(s: &str) -> Result<Vec<Part>> {
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
                let inner = inner.split_whitespace().collect::<Vec<_>>().join(" ");
                let find = |list: &[(&'static str, &[&str])]| {
                    list.iter()
                        .find(|(_, al)| al.contains(&inner.as_str()))
                        .map(|(t, _)| *t)
                };
                let part = find(TAGS)
                    .map(Part::Sound)
                    .or_else(|| find(HESITATIONS).map(Part::Hesitation))
                    .or_else(|| pause_ms(&inner).map(Part::Pause));
                match part {
                    Some(p) => {
                        if !text.trim().is_empty() {
                            out.push(Part::Text(text.trim().to_string()));
                        }
                        text.clear();
                        out.push(p);
                        i = j + 1;
                        continue;
                    }
                    None if matches!(cs[i], '[' | '{' | '<') => {
                        let known: Vec<&str> =
                            TAGS.iter().chain(HESITATIONS).map(|t| t.0).collect();
                        bail!(err(
                            "bad_params",
                            format!("Flux doesn't know [{inner}]. Use one of: {}, or a pause such as [pause], [long pause] or [pause 2s].", known.join(", "))
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

/// One piece of the script as Chatterbox Turbo reads it: words with [tag] markers, without pauses.
fn turbo_piece(parts: &[Part]) -> String {
    let mut out = String::new();
    for (k, p) in parts.iter().enumerate() {
        let w = match p {
            Part::Text(t) => t.clone(),
            Part::Sound(s) => format!("[{s}]"),
            Part::Pause(_) => continue,
            Part::Hesitation(h) => {
                // A hesitation is said as a word, set off by a comma like people write it. It starts with a capital
                // where a sentence starts.
                let start = out.trim_end().is_empty() || out.trim_end().ends_with(['.', '!', '?']);
                let mut w = if start {
                    let mut c = h.chars();
                    c.next()
                        .map(|f| f.to_uppercase().chain(c).collect())
                        .unwrap_or_default()
                } else {
                    h.to_string()
                };
                let next_is_words = matches!(parts.get(k + 1), Some(Part::Text(t)) if !t.starts_with([',', '.', '!', '?', ';', ':']));
                if next_is_words {
                    w.push(',');
                } else if parts.get(k + 1).is_none() {
                    w.push_str("...");
                }
                w
            }
        };
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&w);
    }
    out
}

/// The script as Chatterbox Turbo reads it: words with [tag] markers. A pause shows as "...".
pub fn turbo_text(parts: &[Part]) -> String {
    segments(parts)
        .iter()
        .map(|s| match s {
            Segment::Say(t) => t.clone(),
            Segment::Silence(_) => "...".into(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// What the generator does, in order: speak a piece in the voice, or leave silence. Pauses split the script so the
/// silence is exact rather than left to the model.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "lowercase")]
pub enum Segment {
    Say(String),
    Silence(u32),
}

pub fn segments(parts: &[Part]) -> Vec<Segment> {
    let mut out = vec![];
    let mut piece: Vec<Part> = vec![];
    let mut silence = 0u32;
    for p in parts {
        if let Part::Pause(ms) = p {
            if !piece.is_empty() {
                out.push(Segment::Say(turbo_piece(&piece)));
                piece.clear();
            }
            // Pauses that touch add up, to at most MAX_PAUSE_MS.
            silence = (silence + ms).min(MAX_PAUSE_MS);
        } else {
            if silence > 0 {
                out.push(Segment::Silence(silence));
                silence = 0;
            }
            piece.push(p.clone());
        }
    }
    if !piece.is_empty() {
        out.push(Segment::Say(turbo_piece(&piece)));
    }
    if silence > 0 {
        out.push(Segment::Silence(silence));
    }
    out
}

/// Hesitations in the script, written as placeholders or as plain words.
pub fn hesitation_count(parts: &[Part]) -> usize {
    parts
        .iter()
        .map(|p| match p {
            Part::Hesitation(_) => 1,
            Part::Text(t) => t
                .split(|c: char| {
                    c.is_whitespace() || matches!(c, ',' | '.' | '!' | '?' | ';' | ':' | '"')
                })
                .filter(|w| HESITATION_WORDS.contains(&w.to_lowercase().as_str()))
                .count(),
            _ => 0,
        })
        .sum()
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

// ---------------------------------------------------------------- methods

pub fn h_status(api: &Api, _: &Value) -> Result<Value> {
    let h = hardware();
    let pick = recommend(h);
    let ready = crate::voicemodels::ready_for(api, "speech", None);
    let why = match (&ready, pick.model) {
        (Ok(r), _) => format!("Ready: {}.", r.model.name),
        (Err(_), None) => pick.reason.clone(),
        (Err(e), Some(_)) => e.to_string(),
    };
    let models = crate::voicemodels::h_models(api, &Value::Null)?;
    Ok(json!({
        "available": ready.is_ok(), "why": why, "hardware": h, "pick": pick,
        "models": models["models"], "runtime": models["runtime"], "settings": models["settings"],
        "profiles": profiles(api).len(),
        "statement_example": consent_statement("your name"),
    }))
}

pub fn h_models(api: &Api, v: &Value) -> Result<Value> {
    crate::voicemodels::h_models(api, v)
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
        "hesitations": HESITATIONS.iter().map(|(t, al)| json!({"tag": t, "also": al})).collect::<Vec<_>>(),
        "pauses": [
            {"tag": "beat", "ms": 300},
            {"tag": "pause", "ms": 500},
            {"tag": "long pause", "ms": 1200},
            {"tag": "pause 2s", "ms": 2000, "note": format!("Any length, as pause 300ms or pause 1.5s, up to {} seconds.", MAX_PAUSE_MS / 1000)},
        ],
        "how": "Sounds the voice performs: [laugh], (laughs), *sighs*, {cough}. Hesitations said in the voice: [hmm], [um], [uh], or write them as words (hmm, um). Silence: [pause], [long pause], [pause 2s]. To keep your own timing, laughs and hesitations exactly, record yourself and use clone.convert.",
        "example": "So I opened the box [laugh] and [hmm] it was empty. [pause] [sigh] Of course it was.",
    }))
}

/// Check a script without speaking it: what will be said and which sounds go where.
pub fn h_script(_: &Api, v: &Value) -> Result<Value> {
    let text = os(v, "text").ok_or_else(|| err("bad_params", "`text` is required"))?;
    let parts = parse_script(&text)?;
    let segs = segments(&parts);
    Ok(json!({
        "parts": parts,
        "model_text": turbo_text(&parts),
        "segments": segs,
        "sounds": parts.iter().filter(|p| matches!(p, Part::Sound(_))).count(),
        "hesitations": hesitation_count(&parts),
        "silence_ms": segs.iter().map(|s| match s { Segment::Silence(ms) => *ms, _ => 0 }).sum::<u32>(),
    }))
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

/// Sounds only some models can perform: for the others they are left out (and counted), never read aloud.
fn drop_sounds(text: &str) -> (String, usize) {
    let mut t = text.to_string();
    let mut n = 0;
    for (tag, _) in TAGS {
        let k = format!("[{tag}]");
        n += t.matches(&k).count();
        t = t.replace(&k, " ");
    }
    (t.split_whitespace().collect::<Vec<_>>().join(" "), n)
}

/// A script for an engine that only reads plain words: sounds left out (and counted), pauses as a comma.
pub fn plain_script(s: &str) -> Result<(String, usize)> {
    let parts = parse_long(s)?;
    let (mut out, mut n) = (vec![], 0);
    for seg in segments(&parts) {
        if let Segment::Say(t) = seg {
            let (t, d) = drop_sounds(&t);
            n += d;
            if !t.is_empty() {
                out.push(t);
            }
        }
    }
    if out.is_empty() {
        bail!(err("bad_params", "There's nothing to say in that script."));
    }
    Ok((out.join(", "), n))
}

/// A script as pieces a speech model can run. Models that can't perform sounds leave them out (and count them).
pub fn model_segments(s: &str, sounds: bool, max: Option<usize>) -> Result<(Vec<Value>, usize)> {
    let parts = parse_long(s)?;
    let (mut out, mut n) = (vec![], 0);
    for seg in segments(&parts) {
        match seg {
            Segment::Silence(ms) => out.push(json!({"type": "silence", "value": ms})),
            Segment::Say(t) => {
                let (t, d) = if sounds { (t, 0) } else { drop_sounds(&t) };
                n += d;
                let pieces = match max {
                    Some(m) => crate::smart::sentences(&t, m),
                    None => vec![t].into_iter().filter(|x| !x.is_empty()).collect(),
                };
                out.extend(
                    pieces
                        .into_iter()
                        .map(|x| json!({"type": "text", "value": x})),
                );
            }
        }
    }
    if !out.iter().any(|x| x["type"] == "text") {
        bail!(err("bad_params", "There's nothing to say in that script."));
    }
    Ok((out, n))
}

/// Where a generated file goes: held to be listened to, onto the open project, or into the export folder.
struct Dest {
    take: bool,
    project: Option<(String, u32)>,
    path: PathBuf,
    id: String,
    at: Option<i64>,
}

fn dest(api: &Api, v: &Value) -> Result<Dest> {
    if crate::assets::wants_take(v)? {
        let (id, path) = crate::assets::new_take(api)?;
        return Ok(Dest {
            take: true,
            project: None,
            path,
            id,
            at: None,
        });
    }
    let id = format!("m{}", crate::store::new_id());
    if let Some((pid, dir, sr)) = crate::methods::speech_target(api) {
        std::fs::create_dir_all(&dir)?;
        return Ok(Dest {
            take: false,
            project: Some((pid, sr)),
            path: dir.join(format!("{id}.wav")),
            at: crate::api::opos(v, "at", sr),
            id,
        });
    }
    let dir = api
        .lock()
        .settings
        .export_dir
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(crate::store::default_export_dir);
    std::fs::create_dir_all(&dir)?;
    Ok(Dest {
        take: false,
        project: None,
        path: dir.join(format!("Synthetic speech {id}.wav")),
        at: None,
        id,
    })
}

fn finish(api: &Api, d: &Dest, short: &str, voice: &str, mut res: Value) -> Result<Value> {
    if d.take {
        let kind = if short == "converted take" {
            "converted take"
        } else {
            "cloned speech"
        };
        let held = crate::assets::finish(
            api,
            crate::assets::NewTake {
                id: &d.id,
                path: &d.path,
                short,
                kind,
                voice,
                sounds_left_out: res["sounds_left_out"].as_u64().unwrap_or(0) as usize,
            },
        )?;
        res["take"] = held["take"].clone();
        res["seconds"] = held["seconds"].clone();
    } else if let Some((pid, sr)) = &d.project {
        let added = crate::methods::add_generated(
            api,
            crate::methods::Place {
                project: pid,
                sr: *sr,
                at: d.at,
            },
            &d.id,
            &d.path,
            short,
            voice,
        )?;
        for (k, x) in added.as_object().into_iter().flatten() {
            res[k] = x.clone();
        }
    } else {
        res["path"] = json!(d.path.display().to_string());
    }
    res["synthetic"] = json!(true);
    Ok(res)
}

fn wait_or_job(api: &Api, v: &Value, job: String) -> Result<Value> {
    if v.get("wait").and_then(|x| x.as_bool()).unwrap_or(false) {
        let j = api
            .jobs
            .wait(&job, std::time::Duration::from_secs(3600))
            .unwrap();
        if j.state != "done" {
            bail!(err(
                if j.state == "cancelled" {
                    "cancelled"
                } else {
                    "failed"
                },
                j.error.unwrap_or_else(|| "Cancelled.".into())
            ));
        }
        return Ok(json!({"job": job, "result": j.result}));
    }
    Ok(json!({"job": job}))
}

fn label(name: &str, model: &str) -> String {
    format!("Synthetic speech in the voice of {name}, made with their recorded consent by Flux using {model}.")
}

fn voice_name(api: &Api, voice: &str) -> String {
    profiles(api)
        .into_iter()
        .find(|p| p.id == voice)
        .map(|p| p.name)
        .unwrap_or_default()
}

/// Speak a script in a consented voice with the chosen speech model. The script is checked first.
pub fn h_speak(api: &Api, v: &Value) -> Result<Value> {
    let voice = need_voice(api, v)?;
    let text = os(v, "text").ok_or_else(|| err("bad_params", "`text` is required"))?;
    let parts = parse_script(&text)?;
    let r = crate::voicemodels::ready_for(api, "speech", v.get("model").and_then(|x| x.as_str()))?;
    let name = voice_name(api, &voice);
    let reference = voices_dir(api).join(&voice).join("reference.wav");
    let mut dropped = 0;
    let segs: Vec<Value> = segments(&parts)
        .into_iter()
        .filter_map(|s| match s {
            Segment::Silence(ms) => Some(json!({"type": "silence", "value": ms})),
            Segment::Say(t) => {
                let (t, n) = if r.model.sounds {
                    (t, 0)
                } else {
                    drop_sounds(&t)
                };
                dropped += n;
                (!t.is_empty()).then(|| json!({"type": "say", "value": t}))
            }
        })
        .collect();
    let d = dest(api, v)?;
    let a = api.clone();
    let short: String = text.chars().take(40).collect();
    let next = if d.project.is_some() {
        "Adding to project"
    } else {
        "Saving"
    };
    let job = api.jobs.spawn(
        "clone-speak",
        &format!("Speaking as {name}"),
        &["Speaking", next],
        move |ctx| {
            ctx.stage("Speaking", "active");
            let mut req = crate::voicemodels::request(&r, "speak", &reference, &d.path);
            req["segments"] = json!(segs);
            let mut res = crate::voicemodels::run_model(&r, req, ctx)?;
            crate::voicemodels::label_wav(&d.path, &label(&name, &r.model.name))?;
            ctx.stage("Speaking", "done");
            ctx.stage(next, "active");
            res["model"] = json!(r.model.id);
            res["sounds_left_out"] = json!(dropped);
            finish(&a, &d, &short, &voice, res)
        },
    );
    wait_or_job(api, v, job)
}

/// Turn your own spoken take into the cloned voice, keeping its timing, laughs and hesitations.
pub fn h_convert(api: &Api, v: &Value) -> Result<Value> {
    let voice = need_voice(api, v)?;
    let path = os(v, "path").ok_or_else(|| {
        err(
            "bad_params",
            "`path` is required: your recording to turn into the voice.",
        )
    })?;
    load_clip(&path, "recording", 0.5, 600.0)?;
    let r = crate::voicemodels::ready_for(api, "convert", v.get("model").and_then(|x| x.as_str()))?;
    let name = voice_name(api, &voice);
    let reference = voices_dir(api).join(&voice).join("reference.wav");
    let d = dest(api, v)?;
    let a = api.clone();
    let next = if d.project.is_some() {
        "Adding to project"
    } else {
        "Saving"
    };
    let job = api.jobs.spawn(
        "clone-convert",
        &format!("Turning your take into {name}"),
        &["Converting", next],
        move |ctx| {
            ctx.stage("Converting", "active");
            let mut req = crate::voicemodels::request(&r, "convert", &reference, &d.path);
            req["input"] = json!(path);
            let mut res = crate::voicemodels::run_model(&r, req, ctx)?;
            crate::voicemodels::label_wav(&d.path, &label(&name, &r.model.name))?;
            ctx.stage("Converting", "done");
            ctx.stage(next, "active");
            res["model"] = json!(r.model.id);
            finish(&a, &d, "converted take", &voice, res)
        },
    );
    wait_or_job(api, v, job)
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
    fn a_script_becomes_pieces_for_any_engine() {
        let (s, n) = model_segments("Hi [laugh] [pause 2s] bye", false, None).unwrap();
        assert_eq!(n, 1);
        assert_eq!(
            s.iter()
                .map(|x| x["type"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["text", "silence", "text"]
        );
        assert_eq!(s[1]["value"], 2000);
        assert!(!s[0]["value"].as_str().unwrap().contains('['));

        let (s, n) = model_segments("Hi [laugh] bye", true, Some(280)).unwrap();
        assert_eq!(n, 0);
        assert!(s[0]["value"].as_str().unwrap().contains("[laugh]"));

        let (t, n) = plain_script("Hi [sigh] [pause] bye").unwrap();
        assert_eq!((t.as_str(), n), ("Hi, bye", 1));

        assert!(model_segments("[pause]", false, None).is_err());
        assert!(plain_script("[laugh]").is_err());
        assert!(plain_script("Say [xyzzy]").is_err());
        assert!(
            plain_script(&"word ".repeat(2000)).is_ok(),
            "text to speech takes long text"
        );
    }

    #[test]
    fn the_model_follows_the_computer() {
        let p = recommend(&hw("windows", "x86_64", 32.0, "nvidia", 8.0));
        assert_eq!(
            (p.model, p.device, p.speed, p.convert),
            (
                Some("chatterbox-turbo"),
                "cuda",
                "fast",
                Some("chatterbox-vc")
            )
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
        assert_eq!((p.device, p.convert), ("mps", Some("chatterbox-vc")));
        let p = recommend(&hw("macos", "aarch64", 8.0, "apple", 0.0));
        assert_eq!((p.device, p.convert), ("mps", None));
        let p = recommend(&hw("macos", "x86_64", 16.0, "intel", 0.0));
        assert_eq!(p.model, None);
        assert!(p.reason.contains("Intel"));
        assert_eq!(
            recommend(&hw("linux", "x86_64", 16.0, "amd", 0.0)).device,
            "cpu"
        );
        assert_eq!(recommend(&hw("linux", "x86_64", 4.0, "", 0.0)).model, None);
        assert!(recommend(&hw("linux", "x86_64", 4.0, "", 0.0))
            .reason
            .contains("4 GB"));
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
    fn hesitation_and_pause_placeholders_are_said_in_the_voice_and_timed() {
        let p = parse_script("[hmm] I think so. (um) Well [laugh] [pause] it's *uh* fine [long pause] [pause 2s] Bye [hmm]").unwrap();
        assert_eq!(
            p,
            vec![
                Part::Hesitation("hmm"),
                Part::Text("I think so.".into()),
                Part::Hesitation("um"),
                Part::Text("Well".into()),
                Part::Sound("laugh"),
                Part::Pause(500),
                Part::Text("it's".into()),
                Part::Hesitation("uh"),
                Part::Text("fine".into()),
                Part::Pause(1200),
                Part::Pause(2000),
                Part::Text("Bye".into()),
                Part::Hesitation("hmm"),
            ]
        );
        assert_eq!(
            segments(&p),
            vec![
                Segment::Say("Hmm, I think so. Um, Well [laugh]".into()),
                Segment::Silence(500),
                Segment::Say("it's uh, fine".into()),
                Segment::Silence(3200),
                Segment::Say("Bye hmm...".into()),
            ]
        );
        assert_eq!(
            turbo_text(&p),
            "Hmm, I think so. Um, Well [laugh] ... it's uh, fine ... Bye hmm..."
        );
        // Placeholders and plain words both count as hesitations.
        assert_eq!(hesitation_count(&p), 4);
        assert_eq!(
            hesitation_count(&parse_script("Hmm, um, I mean... uh-huh.").unwrap()),
            3
        );
        // Aliases, spacing and case.
        assert_eq!(
            parse_script("[ Mm-Hmm ]").unwrap(),
            vec![Part::Hesitation("mm-hmm")]
        );
        assert_eq!(parse_script("{erm}").unwrap(), vec![Part::Hesitation("um")]);
        assert_eq!(
            parse_script("<thinking>").unwrap(),
            vec![Part::Hesitation("hmm")]
        );
        // Pause lengths: any unit, clamped to 50 ms .. 5 s; pauses that touch add up to at most 5 s.
        assert_eq!(
            parse_script("[pause 300ms]").unwrap(),
            vec![Part::Pause(300)]
        );
        assert_eq!(parse_script("[1.5s]").unwrap(), vec![Part::Pause(1500)]);
        assert_eq!(
            parse_script("[pause 2 seconds]").unwrap(),
            vec![Part::Pause(2000)]
        );
        assert_eq!(
            parse_script("[pause 90s]").unwrap(),
            vec![Part::Pause(MAX_PAUSE_MS)]
        );
        assert_eq!(parse_script("[beat]").unwrap(), vec![Part::Pause(300)]);
        let long = parse_script("a [pause 4s] [pause 4s] b").unwrap();
        assert_eq!(segments(&long)[1], Segment::Silence(MAX_PAUSE_MS));
        assert!(parse_script("[pause -1s]").is_err());
        assert!(parse_script("[pause NaNs]").is_err());
        // A script that starts with silence keeps it.
        assert_eq!(
            segments(&parse_script("[pause] hi").unwrap()),
            vec![Segment::Silence(500), Segment::Say("hi".into())]
        );
        // The refusal names hesitations and pauses too.
        let e = parse_script("[yodel]").unwrap_err().to_string();
        assert!(
            e.contains("hmm") && e.contains("laugh") && e.contains("pause"),
            "{e}"
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
