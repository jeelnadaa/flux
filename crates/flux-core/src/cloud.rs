//! Cloud engines for stems, transcripts and text to speech. Off unless the person turns one on: choosing a cloud
//! engine needs `allow_cloud: true`, which the app asks for after saying what is sent and where. Keys come from an
//! environment variable or the system keychain (Windows Credential Manager, macOS Keychain, Secret Service on
//! Linux), never from Flux's own files, and are never returned, logged or put in an error. Free engines need no key:
//! they are shared public demos on Hugging Face Spaces.
//!
//! Every service is reached over HTTPS (or this computer, for tests and local OpenAI-compatible servers). What is
//! sent is only what the job needs: 16 kHz mono audio for a transcript, the song for stems, the text for speech.

use crate::api::{err, Api};
use crate::jobs::JobCtx;
use crate::voicemodels as vm;
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;

pub struct Provider {
    pub id: &'static str,
    pub name: &'static str,
    /// Environment variables checked for the key, in order. Empty: no key at all.
    pub env: &'static [&'static str],
    /// False for services that work without a key (a key only raises their limits).
    pub key_needed: bool,
    /// Default address. `{space}` is filled in for Hugging Face Spaces.
    pub base: &'static str,
    pub keys_at: &'static str,
}

pub const PROVIDERS: &[Provider] = &[
    Provider {
        id: "openai",
        name: "OpenAI",
        env: &["FLUX_OPENAI_KEY", "OPENAI_API_KEY"],
        key_needed: true,
        base: "https://api.openai.com/v1",
        keys_at: "https://platform.openai.com/api-keys",
    },
    Provider {
        id: "groq",
        name: "Groq",
        env: &["FLUX_GROQ_KEY", "GROQ_API_KEY"],
        key_needed: true,
        base: "https://api.groq.com/openai/v1",
        keys_at: "https://console.groq.com/keys",
    },
    Provider {
        id: "deepgram",
        name: "Deepgram",
        env: &["FLUX_DEEPGRAM_KEY", "DEEPGRAM_API_KEY"],
        key_needed: true,
        base: "https://api.deepgram.com/v1",
        keys_at: "https://console.deepgram.com",
    },
    Provider {
        id: "elevenlabs",
        name: "ElevenLabs",
        env: &["FLUX_ELEVENLABS_KEY", "ELEVENLABS_API_KEY"],
        key_needed: true,
        base: "https://api.elevenlabs.io/v1",
        keys_at: "https://elevenlabs.io/app/settings/api-keys",
    },
    Provider {
        id: "replicate",
        name: "Replicate",
        env: &["FLUX_REPLICATE_KEY", "REPLICATE_API_TOKEN"],
        key_needed: true,
        base: "https://api.replicate.com/v1",
        keys_at: "https://replicate.com/account/api-tokens",
    },
    Provider {
        id: "hf",
        name: "Hugging Face Spaces",
        env: &["FLUX_HF_KEY", "HF_TOKEN"],
        key_needed: false,
        base: "https://{space}.hf.space",
        keys_at: "https://huggingface.co/settings/tokens",
    },
];

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    OpenAiTranscribe,
    Deepgram,
    GradioWhisper,
    OpenAiSpeech,
    ElevenLabs,
    GradioKokoro,
    ReplicateDemucs,
    GradioSeparation,
}

pub struct Engine {
    pub id: &'static str,
    pub role: &'static str,
    pub provider: &'static str,
    pub name: &'static str,
    pub kind: Kind,
    /// The service's model name (or Space, or Replicate version).
    pub model: &'static str,
    pub note: &'static str,
    /// Word timing (transcripts): filler finding needs it.
    pub words: bool,
    /// Stems it returns, in order.
    pub stems: &'static [&'static str],
    pub voices: &'static [(&'static str, &'static str)],
}

const KOKORO_VOICES: &[(&str, &str)] = &[
    ("af_heart", "Heart (US English)"),
    ("af_bella", "Bella (US English)"),
    ("am_michael", "Michael (US English)"),
    ("am_fenrir", "Fenrir (US English)"),
    ("bf_emma", "Emma (UK English)"),
    ("bm_george", "George (UK English)"),
];

pub const ENGINES: &[Engine] = &[
    Engine {
        id: "cloud-free-whisper",
        role: "transcript",
        provider: "hf",
        name: "Whisper Large v3, free online",
        kind: Kind::GradioWhisper,
        model: "hf-audio-whisper-large-v3",
        note: "Free, no key: a shared public demo, so it can be slow, busy or asleep. Gives the text without word timing, so it can't find filler words.",
        words: false,
        stems: &[],
        voices: &[],
    },
    Engine {
        id: "cloud-groq-whisper",
        role: "transcript",
        provider: "groq",
        name: "Whisper Large v3 Turbo on Groq",
        kind: Kind::OpenAiTranscribe,
        model: "whisper-large-v3-turbo",
        note: "Very fast, with word timing. Needs a Groq key; their free plan has daily limits.",
        words: true,
        stems: &[],
        voices: &[],
    },
    Engine {
        id: "cloud-openai-whisper",
        role: "transcript",
        provider: "openai",
        name: "Whisper on OpenAI",
        kind: Kind::OpenAiTranscribe,
        model: "whisper-1",
        note: "With word timing. Needs an OpenAI key; paid by the minute. Also works with any OpenAI-compatible server (set its address).",
        words: true,
        stems: &[],
        voices: &[],
    },
    Engine {
        id: "cloud-deepgram",
        role: "transcript",
        provider: "deepgram",
        name: "Deepgram Nova-3",
        kind: Kind::Deepgram,
        model: "nova-3",
        note: "Fast and accurate, with word timing. Needs a Deepgram key; new accounts get free credit.",
        words: true,
        stems: &[],
        voices: &[],
    },
    Engine {
        id: "cloud-free-kokoro",
        role: "tts",
        provider: "hf",
        name: "Kokoro, free online",
        kind: Kind::GradioKokoro,
        model: "remsky-kokoro-tts-zero",
        note: "Free, no key: a shared public demo of Kokoro, so it can be slow, busy or asleep. Your text is sent to it.",
        words: false,
        stems: &[],
        voices: KOKORO_VOICES,
    },
    Engine {
        id: "cloud-openai-tts",
        role: "tts",
        provider: "openai",
        name: "OpenAI speech",
        kind: Kind::OpenAiSpeech,
        model: "gpt-4o-mini-tts",
        note: "Natural voices. Needs an OpenAI key; paid by the character.",
        words: false,
        stems: &[],
        voices: &[
            ("alloy", "Alloy"),
            ("ash", "Ash"),
            ("coral", "Coral"),
            ("echo", "Echo"),
            ("fable", "Fable"),
            ("nova", "Nova"),
            ("onyx", "Onyx"),
            ("sage", "Sage"),
            ("shimmer", "Shimmer"),
        ],
    },
    Engine {
        id: "cloud-elevenlabs",
        role: "tts",
        provider: "elevenlabs",
        name: "ElevenLabs Flash",
        kind: Kind::ElevenLabs,
        model: "eleven_flash_v2_5",
        note: "Expressive voices in 32 languages. Needs an ElevenLabs key; their free plan has a monthly allowance. Any voice id from your library works.",
        words: false,
        stems: &[],
        voices: &[
            ("21m00Tcm4TlvDq8ikWAM", "Rachel (premade)"),
            ("pNInz6obpgDQGcFmaJgB", "Adam (premade)"),
        ],
    },
    Engine {
        id: "cloud-free-separation",
        role: "stems",
        provider: "hf",
        name: "Vocal split, free online",
        kind: Kind::GradioSeparation,
        model: "abidlabs-music-separation",
        note: "Free, no key: a shared public demo, so it can be slow, busy or asleep. Gives two stems (vocals and instrumental). Your audio is sent to it.",
        words: false,
        stems: &["vocals", "instrumental"],
        voices: &[],
    },
    Engine {
        id: "cloud-replicate-demucs",
        role: "stems",
        provider: "replicate",
        name: "Demucs on Replicate",
        kind: Kind::ReplicateDemucs,
        model: "ff5ad47d2685b27c0f820ee134c4394140804f08fa755cb20c846459faa4b337",
        note: "Demucs v4 on a cloud graphics chip: four stems in about a minute. Needs a Replicate key; paid by the second.",
        words: false,
        stems: &["vocals", "drums", "bass", "other"],
        voices: &[],
    },
];

pub fn provider(id: &str) -> Option<&'static Provider> {
    PROVIDERS.iter().find(|p| p.id == id)
}

pub fn engine(id: &str) -> Option<&'static Engine> {
    ENGINES.iter().find(|e| e.id == id)
}

pub fn is_cloud(id: &str) -> bool {
    engine(id).is_some()
}

pub fn for_role(role: &str) -> impl Iterator<Item = &'static Engine> + '_ {
    ENGINES.iter().filter(move |e| e.role == role)
}

// ---------------------------------------------------------------- keys

fn service() -> String {
    std::env::var("FLUX_KEYRING_SERVICE").unwrap_or_else(|_| "Flux".into())
}

fn env_key(p: &Provider) -> Option<(String, &'static str)> {
    p.env.iter().find_map(|v| {
        std::env::var(v)
            .ok()
            .filter(|k| !k.trim().is_empty())
            .map(|k| (k.trim().to_string(), *v))
    })
}

/// Where the key would come from, without reading the keychain: `env:NAME`, `keychain` or `missing`.
pub fn key_source(api: &Api, p: &Provider) -> String {
    if let Some((_, v)) = env_key(p) {
        return format!("env:{v}");
    }
    if vm::settings(api).cloud_keys.contains(p.id) {
        return "keychain".into();
    }
    "missing".into()
}

fn key(api: &Api, p: &Provider) -> Result<Option<String>> {
    if let Some((k, _)) = env_key(p) {
        return Ok(Some(k));
    }
    if !vm::settings(api).cloud_keys.contains(p.id) {
        return Ok(None);
    }
    let e = keyring::Entry::new(&service(), p.id).map_err(keychain_err)?;
    match e.get_password() {
        Ok(k) => Ok(Some(k)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(keychain_err(e)),
    }
}

fn keychain_err(e: keyring::Error) -> anyhow::Error {
    // The keychain's own message, never the key.
    err(
        "unavailable",
        format!("The system keychain isn't available ({e}). Set the key in an environment variable instead."),
    )
}

/// `ai.cloud.key.set`: keep a key in the system keychain.
pub fn h_key_set(api: &Api, v: &Value) -> Result<Value> {
    let p = provider_arg(v)?;
    let k = v["key"].as_str().unwrap_or("").trim().to_string();
    if k.is_empty() || k.len() > 4096 || k.chars().any(|c| c.is_control()) {
        bail!(err(
            "bad_params",
            "`key` is the key text, without spaces or line breaks."
        ));
    }
    let e = keyring::Entry::new(&service(), p.id).map_err(keychain_err)?;
    e.set_password(&k).map_err(keychain_err)?;
    let mut s = vm::settings(api);
    s.cloud_keys.insert(p.id.into());
    vm::save_settings(api, &s)?;
    Ok(json!({"provider": p.id, "key": key_source(api, p)}))
}

/// `ai.cloud.key.delete`: forget a key Flux kept in the keychain (environment variables are left alone).
pub fn h_key_delete(api: &Api, v: &Value) -> Result<Value> {
    let p = provider_arg(v)?;
    let mut s = vm::settings(api);
    if s.cloud_keys.remove(p.id) {
        if let Ok(e) = keyring::Entry::new(&service(), p.id) {
            match e.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => {}
                Err(e) => return Err(keychain_err(e)),
            }
        }
        vm::save_settings(api, &s)?;
    }
    Ok(json!({"provider": p.id, "key": key_source(api, p)}))
}

/// `ai.cloud.set`: a service's address (an OpenAI-compatible server, a mirror), or back to the default.
pub fn h_cloud_set(api: &Api, v: &Value) -> Result<Value> {
    let p = provider_arg(v)?;
    let mut s = vm::settings(api);
    match v["base_url"].as_str().map(str::trim) {
        Some("") | None => {
            s.cloud_base.remove(p.id);
        }
        Some(u) => {
            check_url(u)?;
            s.cloud_base
                .insert(p.id.into(), u.trim_end_matches('/').into());
        }
    }
    vm::save_settings(api, &s)?;
    Ok(json!({"provider": p.id, "base_url": base(api, p, "{space}")}))
}

fn provider_arg(v: &Value) -> Result<&'static Provider> {
    let id = v["provider"].as_str().unwrap_or("");
    provider(id).ok_or_else(|| {
        err(
            "bad_params",
            format!(
                "`provider` is one of: {}.",
                PROVIDERS
                    .iter()
                    .map(|p| p.id)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )
    })
}

fn check_url(u: &str) -> Result<()> {
    let ok = u.starts_with("https://")
        || u.starts_with("http://127.0.0.1:")
        || u.starts_with("http://localhost:")
        || u.starts_with("http://[::1]:");
    if !ok {
        bail!(err(
            "bad_params",
            "Flux only talks to services over HTTPS, or to a server on this computer."
        ));
    }
    Ok(())
}

fn base(api: &Api, p: &Provider, space: &str) -> String {
    vm::settings(api)
        .cloud_base
        .get(p.id)
        .cloned()
        .unwrap_or_else(|| p.base.to_string())
        .replace("{space}", space)
}

// ---------------------------------------------------------------- listing and choosing

pub fn engine_json(api: &Api, e: &Engine) -> Value {
    let p = provider(e.provider).expect("engine's provider");
    let src = key_source(api, p);
    let allowed = vm::settings(api).cloud_ok.contains(e.id);
    let ready = !p.key_needed || src != "missing";
    json!({
        "id": e.id, "kind": "cloud", "name": e.name, "tier": "cloud", "note": e.note, "licence": "Online service",
        "provider": p.id, "provider_name": p.name, "runs": p.name, "free": !p.key_needed,
        "key_needed": p.key_needed, "key": src, "key_env": p.env, "keys_at": p.keys_at,
        "allowed": allowed, "installed": ready, "words": e.words, "stems": e.stems,
        "voices": e.voices.iter().map(|(id, name)| json!({"id": id, "name": name})).collect::<Vec<_>>(),
        "sends": sends(e),
    })
}

/// What leaves the computer, said plainly.
pub fn sends(e: &Engine) -> String {
    let p = provider(e.provider).map(|p| p.name).unwrap_or("");
    match e.role {
        "transcript" => format!("The audio being transcribed is sent to {p}."),
        "stems" => format!("The audio being split is sent to {p}."),
        _ => format!("The text to speak is sent to {p}."),
    }
}

/// Before a cloud engine is chosen: the person must have said yes to sending to it.
pub fn check_allowed(api: &Api, id: &str, allow: bool) -> Result<()> {
    let e = engine(id).ok_or_else(|| err("not_found", format!("`{id}` isn't a cloud engine.")))?;
    let mut s = vm::settings(api);
    if s.cloud_ok.contains(id) {
        return Ok(());
    }
    if !allow {
        bail!(err(
            "consent_required",
            format!("{} runs online. {} Ask the person first, then call again with `allow_cloud: true`.", e.name, sends(e))
        ));
    }
    s.cloud_ok.insert(id.into());
    vm::save_settings(api, &s)
}

struct Ready {
    e: &'static Engine,
    p: &'static Provider,
    key: Option<String>,
    base: String,
}

fn ready(api: &Api, id: &str) -> Result<Ready> {
    let e = engine(id).ok_or_else(|| err("not_found", format!("`{id}` isn't a cloud engine.")))?;
    let p = provider(e.provider).expect("engine's provider");
    if !vm::settings(api).cloud_ok.contains(id) {
        bail!(err(
            "consent_required",
            format!("{} runs online and hasn't been turned on. Choose it in Settings, Smart tools, or with ai.engine.set and `allow_cloud: true`.", e.name)
        ));
    }
    let key = key(api, p)?;
    if p.key_needed && key.is_none() {
        bail!(err(
            "needs_key",
            format!(
                "{} needs a {} key. Add it in Settings, Smart tools, with ai.cloud.key.set, or in the environment variable {}. Get one at {}.",
                e.name, p.name, p.env.last().unwrap_or(&""), p.keys_at
            )
        ));
    }
    let b = base(api, p, e.model);
    check_url(&b)?;
    Ok(Ready { e, p, key, base: b })
}

/// For a job's checks before it starts: is this cloud engine usable now?
pub fn check_ready(api: &Api, id: &str) -> Result<()> {
    ready(api, id).map(|_| ())
}

// ---------------------------------------------------------------- HTTP

fn client() -> Result<reqwest::blocking::Client> {
    vm::http_client(Duration::from_secs(600))
}

fn wav16(ch: &[&[f32]], sr: u32) -> Result<Vec<u8>> {
    let mut cur = std::io::Cursor::new(Vec::new());
    {
        let spec = hound::WavSpec {
            channels: ch.len() as u16,
            sample_rate: sr,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::new(&mut cur, spec)?;
        for i in 0..ch[0].len() {
            for c in ch {
                w.write_sample((c[i].clamp(-1.0, 1.0) * 32767.0).round() as i16)?;
            }
        }
        w.finalize()?;
    }
    Ok(cur.into_inner())
}

/// A clear error for a service's answer, with its message but never the request, the key or the audio.
fn answer(r: reqwest::blocking::Response, who: &str) -> Result<reqwest::blocking::Response> {
    let st = r.status();
    if st.is_success() {
        return Ok(r);
    }
    let body = r.text().unwrap_or_default();
    let msg = serde_json::from_str::<Value>(&body)
        .ok()
        .and_then(|j| {
            [
                &j["error"]["message"],
                &j["detail"]["message"],
                &j["detail"],
                &j["err_msg"],
                &j["error"],
                &j["message"],
            ]
            .iter()
            .find_map(|x| x.as_str().map(String::from))
        })
        .unwrap_or_default();
    let msg: String = msg.chars().take(200).collect();
    let code = match st.as_u16() {
        401 | 403 => "needs_key",
        429 => "busy",
        _ => "failed",
    };
    let what = match code {
        "needs_key" => format!("{who} refused the key ({st})."),
        "busy" => format!("{who} is busy or the plan's limit is reached ({st}). Try again later or choose another engine."),
        _ => format!("{who} answered {st}."),
    };
    bail!(err(
        code,
        if msg.is_empty() {
            what
        } else {
            format!("{what} {msg}")
        }
    ))
}

fn net(e: reqwest::Error, who: &str) -> anyhow::Error {
    err(
        "offline",
        format!("Couldn't reach {who}: {}. Check the connection, or choose an engine that runs on this computer.", e.without_url()),
    )
}

fn get_bytes(
    c: &reqwest::blocking::Client,
    url: &str,
    auth: Option<(&str, String)>,
    who: &str,
) -> Result<Vec<u8>> {
    check_url(url)?;
    let mut rq = c.get(url);
    if let Some((h, v)) = auth {
        rq = rq.header(h, v);
    }
    let r = answer(rq.send().map_err(|e| net(e, who))?, who)?;
    Ok(r.bytes().map_err(|e| net(e, who))?.to_vec())
}

fn decode_bytes(b: &[u8], ext: &str) -> Result<crate::media::Decoded> {
    let dir = std::env::temp_dir().join(format!("flux-cloud-{}", crate::store::new_id()));
    std::fs::create_dir_all(&dir)?;
    let p = dir.join(format!("audio.{ext}"));
    std::fs::write(&p, b)?;
    let d = crate::media::decode_file(&p);
    std::fs::remove_dir_all(&dir).ok();
    d.map_err(|_| err("failed", "The service sent back audio Flux can't read."))
}

// ---------------------------------------------------------------- Hugging Face Spaces (Gradio)

fn gradio(
    c: &reqwest::blocking::Client,
    r: &Ready,
    endpoint: &str,
    data: Value,
    files: &[(&str, Vec<u8>)],
    ctx: &JobCtx,
) -> Result<Vec<Value>> {
    let who = r.p.name;
    let auth = |rq: reqwest::blocking::RequestBuilder| match &r.key {
        Some(k) => rq.bearer_auth(k),
        None => rq,
    };
    let mut paths = vec![];
    for (name, bytes) in files {
        let part = reqwest::blocking::multipart::Part::bytes(bytes.clone())
            .file_name(name.to_string())
            .mime_str("audio/wav")?;
        let form = reqwest::blocking::multipart::Form::new().part("files", part);
        let up = auth(
            c.post(format!("{}/gradio_api/upload", r.base))
                .multipart(form),
        )
        .send()
        .map_err(|e| net(e, who))?;
        let v: Vec<String> = answer(up, who)?.json().context("upload answer")?;
        paths.push(v.into_iter().next().unwrap_or_default());
    }
    // `{"$file": i}` in the data becomes the i-th uploaded file.
    fn fill(v: Value, paths: &[String]) -> Value {
        match v {
            Value::Object(o) if o.contains_key("$file") => {
                let i = o["$file"].as_u64().unwrap_or(0) as usize;
                json!({"path": paths.get(i).cloned().unwrap_or_default(), "meta": {"_type": "gradio.FileData"}})
            }
            Value::Array(a) => Value::Array(a.into_iter().map(|x| fill(x, paths)).collect()),
            x => x,
        }
    }
    let body = json!({"data": fill(data, &paths)});
    let call = auth(
        c.post(format!("{}/gradio_api/call{endpoint}", r.base))
            .json(&body),
    )
    .send()
    .map_err(|e| net(e, who))?;
    let ev: Value = answer(call, who)?.json().context("call answer")?;
    let id = ev["event_id"].as_str().unwrap_or("").to_string();
    if ctx.cancelled() {
        bail!("cancelled");
    }
    ctx.progress(0.3);
    let res = auth(c.get(format!("{}/gradio_api/call{endpoint}/{id}", r.base)))
        .send()
        .map_err(|e| net(e, who))?;
    let text = answer(res, who)?.text().map_err(|e| net(e, who))?;
    parse_sse(&text, r.e.name)
}

/// The result of a Gradio call's event stream: the `complete` event's data.
fn parse_sse(text: &str, name: &str) -> Result<Vec<Value>> {
    let mut event = "";
    for line in text.lines() {
        if let Some(e) = line.strip_prefix("event:") {
            event = e.trim();
        } else if let Some(d) = line.strip_prefix("data:") {
            match event {
                "complete" => {
                    return serde_json::from_str::<Vec<Value>>(d.trim()).map_err(|_| {
                        err("failed", format!("{name} sent an answer Flux can't read."))
                    })
                }
                "error" => {
                    let m = serde_json::from_str::<Value>(d.trim())
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_default();
                    let m: String = m.chars().take(200).collect();
                    bail!(err(
                        "busy",
                        format!("{name} couldn't do it right now{}. Free demos sleep and have limits: try again, or choose another engine.", if m.is_empty() { String::new() } else { format!(" ({m})") })
                    ));
                }
                _ => {}
            }
        }
    }
    bail!(err(
        "busy",
        format!("{name} didn't finish. Free demos sleep and have limits: try again, or choose another engine.")
    ))
}

fn gradio_file(c: &reqwest::blocking::Client, r: &Ready, v: &Value) -> Result<Vec<u8>> {
    let url = v["url"]
        .as_str()
        .map(String::from)
        .or_else(|| {
            v["path"]
                .as_str()
                .map(|p| format!("{}/gradio_api/file={p}", r.base))
        })
        .ok_or_else(|| err("failed", format!("{} sent no audio.", r.e.name)))?;
    let auth = r
        .key
        .clone()
        .map(|k| ("Authorization", format!("Bearer {k}")));
    // Only send the key back to the Space itself.
    let auth = auth.filter(|_| url.starts_with(&r.base));
    get_bytes(c, &url, auth, r.p.name)
}

fn ext_of(v: &Value) -> String {
    let p = v["path"].as_str().or(v["url"].as_str()).unwrap_or("");
    p.rsplit('.')
        .next()
        .filter(|e| e.len() <= 4 && !e.contains('/'))
        .unwrap_or("wav")
        .to_lowercase()
}

// ---------------------------------------------------------------- transcripts

/// Ten minutes of 16 kHz 16-bit mono is 19 MB, under the 25 MB most services take.
const CHUNK_S: usize = 600;

/// A transcript from a cloud engine, in the same shape the local runner gives: segments with words, in seconds.
pub fn transcribe(
    api: &Api,
    id: &str,
    mono16k: &[f32],
    language: Option<&str>,
    ctx: &JobCtx,
) -> Result<Value> {
    let r = ready(api, id)?;
    let c = client()?;
    let lang = language.filter(|l| *l != "auto" && !l.is_empty());
    let mut all = json!({"language": Value::Null, "segments": []});
    let n = mono16k.len().div_ceil(CHUNK_S * 16000).max(1);
    for (i, chunk) in mono16k.chunks(CHUNK_S * 16000).enumerate() {
        if ctx.cancelled() {
            bail!("cancelled");
        }
        let wav = wav16(&[chunk], 16000)?;
        let dur = chunk.len() as f64 / 16000.0;
        let part = match r.e.kind {
            Kind::OpenAiTranscribe => openai_transcribe(&c, &r, wav, lang)?,
            Kind::Deepgram => deepgram(&c, &r, wav, lang)?,
            Kind::GradioWhisper => {
                let out = gradio(
                    &c,
                    &r,
                    "/transcribe",
                    json!([{"$file": 0}, "transcribe"]),
                    &[("audio.wav", wav)],
                    ctx,
                )?;
                let text = out
                    .first()
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                json!({"language": lang, "segments": [{"start": 0.0, "end": dur, "text": text, "words": []}]})
            }
            _ => bail!(err(
                "bad_params",
                format!("{} doesn't make transcripts.", r.e.name)
            )),
        };
        merge_chunk(&mut all, &part, (i * CHUNK_S) as f64);
        ctx.progress(0.1 + 0.85 * (i + 1) as f32 / n as f32);
    }
    all["words"] = json!(r.e.words);
    Ok(all)
}

/// Add one chunk's transcript, shifted to where the chunk starts.
pub fn merge_chunk(all: &mut Value, part: &Value, offset_s: f64) {
    if all["language"].is_null() {
        all["language"] = part["language"].clone();
    }
    let shift = |v: &Value| json!(v.as_f64().unwrap_or(0.0) + offset_s);
    let segs = all["segments"].as_array_mut().expect("segments");
    for s in part["segments"].as_array().into_iter().flatten() {
        let words: Vec<Value> = s["words"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|w| json!({"word": w["word"], "start": shift(&w["start"]), "end": shift(&w["end"]), "p": w["p"]}))
            .collect();
        segs.push(json!({"start": shift(&s["start"]), "end": shift(&s["end"]), "text": s["text"], "words": words}));
    }
}

fn openai_transcribe(
    c: &reqwest::blocking::Client,
    r: &Ready,
    wav: Vec<u8>,
    lang: Option<&str>,
) -> Result<Value> {
    let who = r.p.name;
    let mut form = reqwest::blocking::multipart::Form::new()
        .part(
            "file",
            reqwest::blocking::multipart::Part::bytes(wav)
                .file_name("audio.wav")
                .mime_str("audio/wav")?,
        )
        .text("model", r.e.model)
        .text("response_format", "verbose_json")
        .text("timestamp_granularities[]", "word")
        .text("timestamp_granularities[]", "segment");
    if let Some(l) = lang {
        form = form.text("language", l.to_string());
    }
    let mut rq = c
        .post(format!("{}/audio/transcriptions", r.base))
        .multipart(form);
    if let Some(k) = &r.key {
        rq = rq.bearer_auth(k);
    }
    let v: Value = answer(rq.send().map_err(|e| net(e, who))?, who)?
        .json()
        .context("transcript answer")?;
    Ok(from_openai(&v))
}

/// OpenAI's verbose JSON (words listed apart from segments) as segments holding their words.
pub fn from_openai(v: &Value) -> Value {
    let words: Vec<&Value> = v["words"].as_array().into_iter().flatten().collect();
    let mut segs: Vec<Value> = v["segments"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|s| json!({"start": s["start"], "end": s["end"], "text": s["text"].as_str().unwrap_or("").trim(), "words": []}))
        .collect();
    if segs.is_empty() {
        let end = words.last().map(|w| w["end"].clone()).unwrap_or(json!(0.0));
        segs.push(json!({"start": 0.0, "end": end, "text": v["text"].as_str().unwrap_or("").trim(), "words": []}));
    }
    for w in words {
        let t = w["start"].as_f64().unwrap_or(0.0);
        let i = segs
            .iter()
            .rposition(|s| s["start"].as_f64().unwrap_or(0.0) <= t + 1e-6)
            .unwrap_or(0);
        segs[i]["words"].as_array_mut().expect("words").push(
            json!({"word": w["word"].as_str().unwrap_or("").trim(), "start": w["start"], "end": w["end"], "p": Value::Null}),
        );
    }
    json!({"language": v["language"], "segments": segs})
}

fn deepgram(
    c: &reqwest::blocking::Client,
    r: &Ready,
    wav: Vec<u8>,
    lang: Option<&str>,
) -> Result<Value> {
    let who = r.p.name;
    let mut url = format!(
        "{}/listen?model={}&smart_format=true&punctuate=true",
        r.base, r.e.model
    );
    match lang {
        Some(l) => url.push_str(&format!("&language={l}")),
        None => url.push_str("&detect_language=true"),
    }
    let mut rq = c.post(url).header("Content-Type", "audio/wav").body(wav);
    if let Some(k) = &r.key {
        rq = rq.header("Authorization", format!("Token {k}"));
    }
    let v: Value = answer(rq.send().map_err(|e| net(e, who))?, who)?
        .json()
        .context("transcript answer")?;
    Ok(from_deepgram(&v))
}

/// Deepgram's word list, split into sentences at end punctuation.
pub fn from_deepgram(v: &Value) -> Value {
    let ch = &v["results"]["channels"][0];
    let alt = &ch["alternatives"][0];
    let mut segs: Vec<Value> = vec![];
    let mut cur: Vec<Value> = vec![];
    let flush = |cur: &mut Vec<Value>, segs: &mut Vec<Value>| {
        if cur.is_empty() {
            return;
        }
        let text = cur
            .iter()
            .filter_map(|w| w["word"].as_str())
            .collect::<Vec<_>>()
            .join(" ");
        segs.push(json!({"start": cur[0]["start"], "end": cur[cur.len() - 1]["end"], "text": text, "words": std::mem::take(cur)}));
    };
    for w in alt["words"].as_array().into_iter().flatten() {
        let word = w["punctuated_word"]
            .as_str()
            .or(w["word"].as_str())
            .unwrap_or("");
        cur.push(json!({"word": word, "start": w["start"], "end": w["end"], "p": w["confidence"]}));
        if word.ends_with(['.', '?', '!']) {
            flush(&mut cur, &mut segs);
        }
    }
    flush(&mut cur, &mut segs);
    let lang = ch["detected_language"].clone();
    json!({"language": if lang.is_null() { v["metadata"]["language"].clone() } else { lang }, "segments": segs})
}

// ---------------------------------------------------------------- text to speech

/// Speech from a cloud engine into `out` (a 32-bit float WAV). Returns its length and rate.
pub fn speak(
    api: &Api,
    id: &str,
    text: &str,
    voice: Option<&str>,
    speed: f32,
    out: &Path,
    ctx: &JobCtx,
) -> Result<Value> {
    let r = ready(api, id)?;
    let c = client()?;
    let saved = crate::smart::option(api, "tts", "voice");
    let voice = match voice.or(saved.as_deref()) {
        Some(v)
            if r.e.voices.iter().any(|x| x.0 == v)
                || (r.e.kind == Kind::ElevenLabs && !v.is_empty()) =>
        {
            v.to_string()
        }
        _ => {
            r.e.voices
                .first()
                .map(|v| v.0.to_string())
                .unwrap_or_default()
        }
    };
    let max = match r.e.kind {
        Kind::GradioKokoro => 1000,
        _ => 4000,
    };
    let pieces = crate::smart::sentences(text, max);
    let (mut all, mut sr) = (Vec::<f32>::new(), 0u32);
    for (i, piece) in pieces.iter().enumerate() {
        if ctx.cancelled() {
            bail!("cancelled");
        }
        let (mono, rate) = match r.e.kind {
            Kind::OpenAiSpeech => {
                let body = json!({"model": r.e.model, "voice": voice, "input": piece, "response_format": "wav", "speed": speed});
                let mut rq = c.post(format!("{}/audio/speech", r.base)).json(&body);
                if let Some(k) = &r.key {
                    rq = rq.bearer_auth(k);
                }
                let b = answer(rq.send().map_err(|e| net(e, r.p.name))?, r.p.name)?
                    .bytes()
                    .map_err(|e| net(e, r.p.name))?;
                mono_of(decode_bytes(&b, "wav")?)
            }
            Kind::ElevenLabs => {
                let body = json!({"text": piece, "model_id": r.e.model, "voice_settings": {"speed": speed.clamp(0.7, 1.2)}});
                let mut rq = c
                    .post(format!(
                        "{}/text-to-speech/{voice}?output_format=pcm_24000",
                        r.base
                    ))
                    .json(&body);
                if let Some(k) = &r.key {
                    rq = rq.header("xi-api-key", k);
                }
                let b = answer(rq.send().map_err(|e| net(e, r.p.name))?, r.p.name)?
                    .bytes()
                    .map_err(|e| net(e, r.p.name))?;
                let pcm: Vec<f32> = b
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|x| i16::from_le_bytes(*x) as f32 / 32768.0)
                    .collect();
                (pcm, 24000)
            }
            Kind::GradioKokoro => {
                let outv = gradio(
                    &c,
                    &r,
                    "/generate_speech_from_ui",
                    json!([piece, [voice], speed]),
                    &[],
                    ctx,
                )?;
                let f = outv.first().cloned().unwrap_or(Value::Null);
                let b = gradio_file(&c, &r, &f)?;
                mono_of(decode_bytes(&b, &ext_of(&f))?)
            }
            _ => bail!(err("bad_params", format!("{} doesn't speak.", r.e.name))),
        };
        if sr == 0 {
            sr = rate;
        }
        all.extend(if rate == sr {
            mono
        } else {
            crate::dsp::resample(&mono, rate, sr)
        });
        ctx.progress(0.1 + 0.85 * (i + 1) as f32 / pieces.len() as f32);
    }
    if all.is_empty() || sr == 0 {
        bail!(err("failed", format!("{} sent back no audio.", r.e.name)));
    }
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sr,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w = hound::WavWriter::create(out, spec)?;
    for s in &all {
        w.write_sample(*s)?;
    }
    w.finalize()?;
    Ok(
        json!({"sr": sr, "seconds": (all.len() as f64 / sr as f64 * 1000.0).round() / 1000.0, "voice": voice, "engine": id}),
    )
}

fn mono_of(d: crate::media::Decoded) -> (Vec<f32>, u32) {
    let n = d.data.first().map(|c| c.len()).unwrap_or(0);
    let k = d.data.len().max(1) as f32;
    let m = (0..n)
        .map(|i| d.data.iter().map(|c| c[i]).sum::<f32>() / k)
        .collect();
    (m, d.rate)
}

// ---------------------------------------------------------------- stems

/// Stems from a cloud engine at the project's rate: (name, (left, right)) in the engine's order.
pub fn stems(
    api: &Api,
    id: &str,
    l: &[f32],
    r_: &[f32],
    sr: u32,
    ctx: &JobCtx,
) -> Result<Vec<(String, crate::api::Stereo)>> {
    let r = ready(api, id)?;
    let c = client()?;
    const RATE: u32 = 44100;
    let wav = wav16(
        &[
            &crate::dsp::resample(l, sr, RATE),
            &crate::dsp::resample(r_, sr, RATE),
        ],
        RATE,
    )?;
    ctx.progress(0.1);
    let files: Vec<(String, Vec<u8>, String)> = match r.e.kind {
        Kind::GradioSeparation => {
            let out = gradio(
                &c,
                &r,
                "/inference",
                json!([{"$file": 0}]),
                &[("song.wav", wav)],
                ctx,
            )?;
            if out.len() < 2 {
                bail!(err(
                    "failed",
                    format!("{} sent back {} stems, not 2.", r.e.name, out.len())
                ));
            }
            r.e.stems
                .iter()
                .zip(&out)
                .map(|(n, f)| Ok((n.to_string(), gradio_file(&c, &r, f)?, ext_of(f))))
                .collect::<Result<_>>()?
        }
        Kind::ReplicateDemucs => replicate(&c, &r, wav, ctx)?,
        _ => bail!(err(
            "bad_params",
            format!("{} doesn't split stems.", r.e.name)
        )),
    };
    let n = l.len();
    let mut v = vec![];
    for (name, bytes, ext) in files {
        let d = decode_bytes(&bytes, &ext)?;
        let left = d.data.first().cloned().unwrap_or_default();
        let right = d.data.get(1).cloned().unwrap_or_else(|| left.clone());
        let mut a = crate::dsp::resample(&left, d.rate, sr);
        let mut b = crate::dsp::resample(&right, d.rate, sr);
        a.resize(n, 0.0);
        b.resize(n, 0.0);
        v.push((name, (a, b)));
    }
    Ok(v)
}

fn replicate(
    c: &reqwest::blocking::Client,
    r: &Ready,
    wav: Vec<u8>,
    ctx: &JobCtx,
) -> Result<Vec<(String, Vec<u8>, String)>> {
    let who = r.p.name;
    let key = r.key.clone().unwrap_or_default();
    let form = reqwest::blocking::multipart::Form::new().part(
        "content",
        reqwest::blocking::multipart::Part::bytes(wav)
            .file_name("song.wav")
            .mime_str("audio/wav")?,
    );
    let up: Value = answer(
        c.post(format!("{}/files", r.base))
            .bearer_auth(&key)
            .multipart(form)
            .send()
            .map_err(|e| net(e, who))?,
        who,
    )?
    .json()?;
    let audio = up["urls"]["get"].as_str().unwrap_or("").to_string();
    let body = json!({"version": r.e.model, "input": {"audio": audio, "format": "wav", "model": "htdemucs"}});
    let mut p: Value = answer(
        c.post(format!("{}/predictions", r.base))
            .bearer_auth(&key)
            .json(&body)
            .send()
            .map_err(|e| net(e, who))?,
        who,
    )?
    .json()?;
    let get = p["urls"]["get"].as_str().unwrap_or("").to_string();
    let cancel = p["urls"]["cancel"].as_str().unwrap_or("").to_string();
    let t0 = std::time::Instant::now();
    loop {
        match p["status"].as_str().unwrap_or("") {
            "succeeded" => break,
            "failed" | "canceled" => {
                let m: String = p["error"]
                    .as_str()
                    .unwrap_or("")
                    .chars()
                    .take(200)
                    .collect();
                bail!(err(
                    "failed",
                    format!("{} couldn't split it. {m}", r.e.name)
                ));
            }
            _ => {}
        }
        if ctx.cancelled() {
            if cancel.starts_with(&r.base) {
                let _ = c.post(&cancel).bearer_auth(&key).send();
            }
            bail!("cancelled");
        }
        if t0.elapsed() > Duration::from_secs(1800) {
            bail!(err(
                "failed",
                format!("{} took more than 30 minutes.", r.e.name)
            ));
        }
        ctx.progress((0.15 + t0.elapsed().as_secs_f32() / 120.0).min(0.8));
        std::thread::sleep(Duration::from_millis(if cfg!(test) { 10 } else { 1500 }));
        check_url(&get)?;
        p = answer(
            c.get(&get)
                .bearer_auth(&key)
                .send()
                .map_err(|e| net(e, who))?,
            who,
        )?
        .json()?;
    }
    let out = &p["output"];
    let mut files = vec![];
    for s in r.e.stems {
        let url = out[*s]
            .as_str()
            .ok_or_else(|| err("failed", format!("{} sent no {s} stem.", r.e.name)))?;
        // Results live on Replicate's file host; the key only goes to the API itself.
        let auth = url
            .starts_with(&r.base)
            .then(|| ("Authorization", format!("Bearer {key}")));
        let ext = ext_of(&json!({"url": url}));
        files.push((s.to_string(), get_bytes(c, url, auth, who)?, ext));
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_shift_to_where_they_start() {
        let mut all = json!({"language": null, "segments": []});
        let part = json!({"language": "en", "segments": [{"start": 1.0, "end": 2.0, "text": "hi", "words": [{"word": "hi", "start": 1.0, "end": 1.5, "p": 0.9}]}]});
        merge_chunk(&mut all, &part, 0.0);
        merge_chunk(&mut all, &part, 600.0);
        assert_eq!(all["language"], "en");
        assert_eq!(all["segments"][1]["start"], 601.0);
        assert_eq!(all["segments"][1]["words"][0]["end"], 601.5);
    }

    #[test]
    fn service_answers_become_segments_with_words() {
        let o = json!({"language": "english", "text": "Hi there. Bye.", "segments": [{"start": 0.0, "end": 1.0, "text": " Hi there."}, {"start": 1.0, "end": 2.0, "text": " Bye."}],
            "words": [{"word": "Hi", "start": 0.1, "end": 0.3}, {"word": "there.", "start": 0.4, "end": 0.8}, {"word": "Bye.", "start": 1.2, "end": 1.5}]});
        let t = from_openai(&o);
        assert_eq!(t["segments"][0]["words"].as_array().unwrap().len(), 2);
        assert_eq!(t["segments"][1]["words"][0]["word"], "Bye.");
        assert_eq!(t["segments"][0]["text"], "Hi there.");
        let d = json!({"results": {"channels": [{"detected_language": "en", "alternatives": [{"words": [
            {"word": "so", "punctuated_word": "So", "start": 0.0, "end": 0.2, "confidence": 0.9},
            {"word": "um", "punctuated_word": "um,", "start": 0.3, "end": 0.5, "confidence": 0.8},
            {"word": "yes", "punctuated_word": "yes.", "start": 0.6, "end": 0.9, "confidence": 0.9},
            {"word": "ok", "punctuated_word": "Ok.", "start": 1.0, "end": 1.2, "confidence": 0.9}]}]}]}});
        let t = from_deepgram(&d);
        assert_eq!(t["language"], "en");
        assert_eq!(t["segments"].as_array().unwrap().len(), 2);
        assert_eq!(t["segments"][0]["text"], "So um, yes.");
        assert_eq!(t["segments"][1]["start"], 1.0);
    }

    #[test]
    fn gradio_streams_give_the_result_or_a_clear_error() {
        let ok = "event: generating\ndata: null\n\nevent: complete\ndata: [\"hello\", {\"path\": \"/tmp/a.wav\"}]\n\n";
        assert_eq!(parse_sse(ok, "X").unwrap()[0], "hello");
        let e = parse_sse("event: error\ndata: \"GPU quota exceeded\"\n\n", "X").unwrap_err();
        assert_eq!(crate::api::error_code(&e), "busy");
        assert!(format!("{e:#}").contains("GPU quota"));
        assert_eq!(
            crate::api::error_code(&parse_sse("", "X").unwrap_err()),
            "busy"
        );
    }

    #[test]
    fn only_https_or_this_computer() {
        assert!(check_url("https://api.openai.com/v1").is_ok());
        assert!(check_url("http://127.0.0.1:8080/v1").is_ok());
        assert!(check_url("http://localhost:1234").is_ok());
        assert!(check_url("http://example.com").is_err());
        assert!(check_url("ftp://x").is_err());
    }
}
