//! Online engines for stems, transcripts and text to speech, end to end through the JSON API against a local mock
//! of each service (OpenAI-compatible, Deepgram, ElevenLabs, Replicate, and Hugging Face Spaces' Gradio API).
//! Nothing leaves this computer.
//!
//! Covers: nothing is sent without the person's yes; keys from the environment and the keychain, never shown or
//! saved in Flux's files; each service's request shape and answer; word positions; failures (refused key, rate
//! limit, a busy free demo, no connection) as clear errors that carry no audio, text or key.

use flux_core::api::Api;
use flux_core::store::Store;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "flux-test-{}-{}-{}",
        name,
        std::process::id(),
        flux_core::store::new_id()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn call(a: &Api, m: &str, p: Value) -> Value {
    a.call(m, p.clone())
        .unwrap_or_else(|e| panic!("{m} {p}: {e:#}"))
}

fn fails(a: &Api, m: &str, p: Value) -> (&'static str, String) {
    let e = a.call(m, p).expect_err(m);
    (flux_core::api::error_code(&e), format!("{e:#}"))
}

/// A job's result, or its error as (code-ish message).
fn run(a: &Api, r: &Value) -> Result<Value, String> {
    let job = r["job"].as_str().unwrap_or_else(|| panic!("{r}"));
    let j = call(a, "job.get", json!({"id": job, "wait_s": 60}));
    if j["state"] == "done" {
        Ok(j["result"].clone())
    } else {
        Err(j["error"].as_str().unwrap_or("").to_string())
    }
}

fn wav(secs: f32, sr: u32, ch: u16, gain: f32) -> Vec<u8> {
    let mut cur = std::io::Cursor::new(vec![]);
    {
        let spec = hound::WavSpec {
            channels: ch,
            sample_rate: sr,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::new(&mut cur, spec).unwrap();
        for i in 0..(sr as f32 * secs) as usize {
            let v = (std::f32::consts::TAU * 220.0 * i as f32 / sr as f32).sin() * gain;
            for _ in 0..ch {
                w.write_sample((v * 32767.0) as i16).unwrap();
            }
        }
        w.finalize().unwrap();
    }
    cur.into_inner()
}

// ---------------------------------------------------------------- the mock services

#[derive(Clone, Debug)]
struct Req {
    method: String,
    url: String,
    auth: Option<String>,
    body: Vec<u8>,
}

#[derive(Clone, Default)]
struct Mock {
    port: u16,
    reqs: Arc<Mutex<Vec<Req>>>,
    /// Paths (prefix) answered with this status instead.
    fail: Arc<Mutex<Vec<(String, u16)>>>,
    /// The Gradio stream says error.
    gradio_error: Arc<Mutex<bool>>,
}

impl Mock {
    fn base(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
    fn hits(&self, prefix: &str) -> Vec<Req> {
        self.reqs
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.url.starts_with(prefix))
            .cloned()
            .collect()
    }
    fn all_bodies(&self) -> Vec<u8> {
        self.reqs
            .lock()
            .unwrap()
            .iter()
            .flat_map(|r| r.body.clone())
            .collect()
    }
}

fn contains(hay: &[u8], needle: &str) -> bool {
    hay.windows(needle.len()).any(|w| w == needle.as_bytes())
}

fn serve() -> Mock {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let m = Mock {
        port: server.server_addr().to_ip().unwrap().port(),
        ..Default::default()
    };
    let m2 = m.clone();
    std::thread::spawn(move || {
        let polls = Arc::new(Mutex::new(0));
        for mut req in server.incoming_requests() {
            let mut body = vec![];
            req.as_reader().read_to_end(&mut body).unwrap();
            let url = req.url().to_string();
            let auth = req
                .headers()
                .iter()
                .find(|h| h.field.equiv("Authorization") || h.field.equiv("xi-api-key"))
                .map(|h| h.value.to_string());
            m2.reqs.lock().unwrap().push(Req {
                method: req.method().to_string(),
                url: url.clone(),
                auth: auth.clone(),
                body: body.clone(),
            });
            let base = m2.base();
            let jsn = |v: Value| {
                tiny_http::Response::from_data(v.to_string()).with_header(
                    tiny_http::Header::from_bytes("Content-Type", "application/json").unwrap(),
                )
            };
            if let Some((_, st)) = m2
                .fail
                .lock()
                .unwrap()
                .iter()
                .find(|(p, _)| url.starts_with(p.as_str()))
            {
                let _ = req.respond(
                    jsn(json!({"error": {"message": "nope from the mock"}})).with_status_code(*st),
                );
                continue;
            }
            let r = if url.starts_with("/v1/audio/transcriptions") {
                // Words apart from segments, like the real verbose JSON.
                jsn(
                    json!({"language": "english", "text": "So um I I think it works.", "duration": 2.0,
                    "segments": [{"start": 0.0, "end": 1.0, "text": " So um I"}, {"start": 1.0, "end": 2.0, "text": " I think it works."}],
                    "words": [{"word": "So", "start": 0.0, "end": 0.2}, {"word": "um", "start": 0.25, "end": 0.45},
                              {"word": "I", "start": 0.5, "end": 0.6}, {"word": "I", "start": 1.0, "end": 1.1},
                              {"word": "think", "start": 1.2, "end": 1.4}, {"word": "it", "start": 1.5, "end": 1.6},
                              {"word": "works.", "start": 1.7, "end": 2.0}]}),
                )
            } else if url.starts_with("/v1/audio/speech") {
                tiny_http::Response::from_data(wav(0.5, 24000, 1, 0.3))
            } else if url.starts_with("/v1/listen") {
                jsn(
                    json!({"metadata": {}, "results": {"channels": [{"detected_language": "en", "alternatives": [{"transcript": "uh yes.", "words": [
                    {"word": "uh", "punctuated_word": "Uh,", "start": 0.5, "end": 0.7, "confidence": 0.8},
                    {"word": "yes", "punctuated_word": "yes.", "start": 0.8, "end": 1.0, "confidence": 0.9}]}]}]}}),
                )
            } else if url.starts_with("/v1/text-to-speech/") {
                // Raw 16-bit PCM at 24 kHz: 0.25 s.
                tiny_http::Response::from_data(vec![0u8; 24000 / 4 * 2])
            } else if url.starts_with("/v1/files") {
                jsn(json!({"urls": {"get": format!("{base}/v1/files/f1")}}))
            } else if url == "/v1/predictions" {
                jsn(
                    json!({"status": "starting", "urls": {"get": format!("{base}/v1/predictions/p1"), "cancel": format!("{base}/v1/predictions/p1/cancel")}}),
                )
            } else if url == "/v1/predictions/p1" {
                let mut n = polls.lock().unwrap();
                *n += 1;
                if *n < 2 {
                    jsn(
                        json!({"status": "processing", "urls": {"get": format!("{base}/v1/predictions/p1")}}),
                    )
                } else {
                    let o: serde_json::Map<String, Value> = ["vocals", "drums", "bass", "other"]
                        .iter()
                        .map(|s| (s.to_string(), json!(format!("{base}/delivery/{s}.wav"))))
                        .collect();
                    jsn(json!({"status": "succeeded", "output": o}))
                }
            } else if let Some(s) = url.strip_prefix("/delivery/") {
                let g = match s {
                    "vocals.wav" => 0.4,
                    "drums.wav" => 0.2,
                    _ => 0.1,
                };
                tiny_http::Response::from_data(wav(1.0, 44100, 2, g))
            } else if url.contains("/gradio_api/upload") {
                jsn(json!(["/tmp/gradio/abc/audio.wav"]))
            } else if url.contains("/gradio_api/call/") && req.method() == &tiny_http::Method::Post
            {
                jsn(json!({"event_id": "e1"}))
            } else if url.contains("/gradio_api/call/") {
                let space = url.split('/').nth(2).unwrap_or("").to_string();
                let file = |p: &str| json!({"path": p, "url": format!("{base}/spaces/{space}/gradio_api/file={p}")});
                let data = if url.contains("/transcribe/") {
                    json!(["So um I I think it works."])
                } else if url.contains("/generate_speech_from_ui/") {
                    json!([file("/tmp/out.wav"), null, "1.0 s"])
                } else {
                    json!([file("/tmp/vocals.wav"), file("/tmp/inst.wav")])
                };
                let text = if *m2.gradio_error.lock().unwrap() {
                    "event: error\ndata: \"ZeroGPU quota exceeded\"\n\n".to_string()
                } else {
                    format!("event: generating\ndata: null\n\nevent: complete\ndata: {data}\n\n")
                };
                tiny_http::Response::from_data(text)
            } else if url.contains("/gradio_api/file=") {
                tiny_http::Response::from_data(wav(0.75, 24000, 1, 0.3))
            } else {
                tiny_http::Response::from_data(vec![]).with_status_code(404)
            };
            let _ = req.respond(r);
        }
    });
    m
}

// ---------------------------------------------------------------- the fixture

const ENV: &[&str] = &[
    "FLUX_OPENAI_KEY",
    "OPENAI_API_KEY",
    "FLUX_GROQ_KEY",
    "GROQ_API_KEY",
    "FLUX_DEEPGRAM_KEY",
    "DEEPGRAM_API_KEY",
    "FLUX_ELEVENLABS_KEY",
    "ELEVENLABS_API_KEY",
    "FLUX_REPLICATE_KEY",
    "REPLICATE_API_TOKEN",
    "FLUX_HF_KEY",
    "HF_TOKEN",
];

static SERIAL: Mutex<()> = Mutex::new(());

struct Fx {
    _one_at_a_time: std::sync::MutexGuard<'static, ()>,
    a: Api,
    d: PathBuf,
    mock: Mock,
}

fn fixture() -> Fx {
    let guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("FLUX_NULL_AUDIO", "1");
    // Keys on the computer running the tests must not leak in, and the keychain entries are the tests' own.
    for v in ENV {
        std::env::remove_var(v);
    }
    std::env::set_var(
        "FLUX_KEYRING_SERVICE",
        format!("Flux test {}", flux_core::store::new_id()),
    );
    let d = tmp("cloud");
    let a = Api::new(Store::at(d.join("data")).unwrap(), true);
    let mock = serve();
    for p in ["openai", "groq", "deepgram", "elevenlabs", "replicate"] {
        call(
            &a,
            "ai.cloud.set",
            json!({"provider": p, "base_url": format!("{}/v1", mock.base())}),
        );
    }
    call(
        &a,
        "ai.cloud.set",
        json!({"provider": "hf", "base_url": format!("{}/spaces/{{space}}", mock.base())}),
    );
    Fx {
        _one_at_a_time: guard,
        a,
        d,
        mock,
    }
}

fn song(dir: &Path) -> String {
    let p = dir.join("song.wav");
    std::fs::write(&p, wav(2.0, 48000, 2, 0.5)).unwrap();
    p.display().to_string()
}

fn engine<'a>(e: &'a Value, role: &str, id: &str) -> &'a Value {
    e["roles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["role"] == role)
        .unwrap()["engines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"] == id)
        .unwrap_or_else(|| panic!("no {id}"))
}

#[test]
fn nothing_is_sent_without_a_yes_and_keys_stay_secret() {
    let fx = fixture();
    let a = &fx.a;
    let e = call(a, "ai.engines", json!({}));
    let groq = engine(&e, "transcript", "cloud-groq-whisper");
    assert_eq!(groq["kind"], "cloud");
    assert_eq!(groq["key"], "missing");
    assert_eq!(groq["allowed"], false);
    assert_eq!(
        groq["suggested"], false,
        "online engines are never suggested"
    );
    assert!(groq["sends"].as_str().unwrap().contains("Groq"));
    let free = engine(&e, "transcript", "cloud-free-whisper");
    assert_eq!(free["free"], true);
    assert_eq!(free["installed"], true, "no key needed");
    assert_eq!(free["words"], false);
    assert_eq!(e["providers"].as_array().unwrap().len(), 6);

    // Choosing one without the person's yes changes nothing and says what would be sent.
    let (code, msg) = fails(
        a,
        "ai.engine.set",
        json!({"role": "transcript", "engine": "cloud-free-whisper"}),
    );
    assert_eq!(code, "consent_required");
    assert!(msg.contains("sent to Hugging Face Spaces"), "{msg}");
    assert!(fx.mock.hits("/spaces").is_empty());
    // Using one per call without it turned on is refused too.
    call(a, "project.new", json!({"name": "Cloud"}));
    call(
        a,
        "media.import",
        json!({"path": song(&fx.d), "wait": true}),
    );
    assert_eq!(
        fails(
            a,
            "transcript.make",
            json!({"engine": "cloud-free-whisper"})
        )
        .0,
        "consent_required"
    );
    // An engine of another job is refused.
    assert_eq!(
        fails(a, "tool.stems", json!({"engine": "cloud-free-whisper"})).0,
        "bad_params"
    );

    // A keyed service without a key says where to get one.
    call(
        a,
        "ai.engine.set",
        json!({"role": "transcript", "engine": "cloud-groq-whisper", "allow_cloud": true}),
    );
    let (code, msg) = fails(a, "transcript.make", json!({}));
    assert_eq!(code, "needs_key");
    assert!(
        msg.contains("GROQ_API_KEY") && msg.contains("console.groq.com"),
        "{msg}"
    );

    // Only HTTPS or this computer.
    assert_eq!(
        fails(
            a,
            "ai.cloud.set",
            json!({"provider": "openai", "base_url": "http://example.com/v1"})
        )
        .0,
        "bad_params"
    );
    assert_eq!(
        fails(
            a,
            "ai.cloud.key.set",
            json!({"provider": "nope", "key": "x"})
        )
        .0,
        "bad_params"
    );
    assert_eq!(
        fails(
            a,
            "ai.cloud.key.set",
            json!({"provider": "groq", "key": "two\nlines"})
        )
        .0,
        "bad_params"
    );

    // The keychain: kept there, never in Flux's files or answers, and forgotten on delete. Where a computer has no
    // keychain (a server with no Secret Service), the answer says so and points at environment variables.
    let secret = "gsk-test-secret-1234567890";
    match a.call(
        "ai.cloud.key.set",
        json!({"provider": "groq", "key": secret}),
    ) {
        Ok(r) => {
            assert_eq!(r["key"], "keychain");
            assert!(!r.to_string().contains(secret));
            let r = run(a, &call(a, "transcript.make", json!({}))).unwrap();
            assert_eq!(r["engine"], "cloud-groq-whisper");
            let h = fx.mock.hits("/v1/audio/transcriptions");
            assert_eq!(
                h.last().unwrap().auth.as_deref(),
                Some(format!("Bearer {secret}").as_str())
            );
            let e = call(a, "ai.engines", json!({"role": "transcript"}));
            assert!(!e.to_string().contains(secret));
            call(a, "ai.cloud.key.delete", json!({"provider": "groq"}));
            assert_eq!(fails(a, "transcript.make", json!({})).0, "needs_key");
        }
        Err(e) => {
            if cfg!(windows) {
                panic!("Windows always has Credential Manager: {e:#}");
            }
            assert_eq!(flux_core::api::error_code(&e), "unavailable", "{e:#}");
            assert!(format!("{e:#}").contains("environment variable"));
        }
    }
    let mut files = String::new();
    for f in walk(&fx.d.join("data")) {
        files.push_str(&String::from_utf8_lossy(
            &std::fs::read(f).unwrap_or_default(),
        ));
    }
    assert!(
        !files.contains(secret),
        "the key never lands in Flux's files"
    );

    // From the environment.
    std::env::set_var("GROQ_API_KEY", "env-key-1");
    let e = call(a, "ai.engines", json!({"role": "transcript"}));
    assert_eq!(
        engine(&e, "transcript", "cloud-groq-whisper")["key"],
        "env:GROQ_API_KEY"
    );
    run(a, &call(a, "transcript.make", json!({}))).unwrap();
    assert_eq!(
        fx.mock
            .hits("/v1/audio/transcriptions")
            .last()
            .unwrap()
            .auth
            .as_deref(),
        Some("Bearer env-key-1")
    );
    std::env::remove_var("GROQ_API_KEY");
}

fn walk(d: &Path) -> Vec<PathBuf> {
    let mut out = vec![];
    for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else {
            out.push(p);
        }
    }
    out
}

#[test]
fn each_service_is_asked_the_right_way_and_answers_land_at_exact_positions() {
    let fx = fixture();
    let a = &fx.a;
    for v in [
        "FLUX_OPENAI_KEY",
        "FLUX_DEEPGRAM_KEY",
        "FLUX_ELEVENLABS_KEY",
        "FLUX_REPLICATE_KEY",
    ] {
        std::env::set_var(v, format!("k-{v}"));
    }
    call(a, "project.new", json!({"name": "Cloud"}));
    call(
        a,
        "media.import",
        json!({"path": song(&fx.d), "wait": true}),
    );
    let allow = |role: &str, id: &str| {
        call(
            a,
            "ai.engine.set",
            json!({"role": role, "engine": id, "allow_cloud": true}),
        )
    };

    // OpenAI-compatible transcript: 16 kHz mono WAV with the model, words placed in their segments, at samples.
    allow("transcript", "cloud-openai-whisper");
    let r = run(a, &call(a, "transcript.make", json!({"language": "en"}))).unwrap();
    assert_eq!(r["words"], 7);
    let q = fx.mock.hits("/v1/audio/transcriptions").pop().unwrap();
    assert_eq!(q.auth.as_deref(), Some("Bearer k-FLUX_OPENAI_KEY"));
    assert!(
        contains(&q.body, "whisper-1")
            && contains(&q.body, "verbose_json")
            && contains(&q.body, "RIFF")
    );
    assert!(contains(&q.body, "name=\"language\"\r\n\r\nen"));
    let wav_at = q.body.windows(4).position(|w| w == b"RIFF").unwrap();
    let rate = u32::from_le_bytes(q.body[wav_at + 24..wav_at + 28].try_into().unwrap());
    let chans = u16::from_le_bytes(q.body[wav_at + 22..wav_at + 24].try_into().unwrap());
    assert_eq!((rate, chans), (16000, 1));
    let t = call(a, "transcript.get", json!({}));
    assert_eq!(t["engine"], "cloud-openai-whisper");
    assert_eq!(t["segments"].as_array().unwrap().len(), 2);
    assert_eq!(t["words"][1]["at"], 12000, "0.25 s at 48 kHz");
    let f = call(a, "transcript.fillers", json!({}));
    let kinds: Vec<(&str, i64)> = f["fillers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| (x["kind"].as_str().unwrap(), x["at"].as_i64().unwrap()))
        .collect();
    assert_eq!(kinds, [("filler", 12000), ("repeat", 24000)]);

    // Deepgram: audio as the body, its own key header, sentences from punctuation.
    allow("transcript", "cloud-deepgram");
    run(a, &call(a, "transcript.make", json!({}))).unwrap();
    let q = fx.mock.hits("/v1/listen").pop().unwrap();
    assert_eq!(q.auth.as_deref(), Some("Token k-FLUX_DEEPGRAM_KEY"));
    assert!(q.url.contains("model=nova-3") && q.url.contains("detect_language=true"));
    assert_eq!(&q.body[0..4], b"RIFF");
    let t = call(a, "transcript.get", json!({}));
    assert_eq!(t["language"], "en");
    assert_eq!(t["words"][0]["at"], 24000);

    // The free one: text only, said plainly, no fillers to find.
    allow("transcript", "cloud-free-whisper");
    run(a, &call(a, "transcript.make", json!({}))).unwrap();
    let up = fx
        .mock
        .hits("/spaces/hf-audio-whisper-large-v3/gradio_api/upload");
    assert_eq!(up.len(), 1);
    assert!(up[0].auth.is_none(), "no key needed, none sent");
    let callreq = fx
        .mock
        .hits("/spaces/hf-audio-whisper-large-v3/gradio_api/call/transcribe")
        .into_iter()
        .find(|r| r.method == "POST")
        .unwrap();
    assert!(
        contains(&callreq.body, "/tmp/gradio/abc/audio.wav")
            && contains(&callreq.body, "gradio.FileData")
    );
    let t = call(a, "transcript.get", json!({}));
    assert_eq!(t["text"], "So um I I think it works.");
    assert_eq!(t["words"].as_array().unwrap().len(), 0);

    // Speech: OpenAI (WAV), ElevenLabs (raw PCM), free Kokoro (a file to fetch). Each is labelled synthetic.
    allow("tts", "cloud-openai-tts");
    let r = run(
        a,
        &call(
            a,
            "tts.speak",
            json!({"text": "Hello there.", "voice": "coral", "speed": 1.25}),
        ),
    )
    .unwrap();
    assert_eq!(r["engine"], "cloud-openai-tts");
    assert_eq!(r["seconds"], 0.5);
    let q = fx.mock.hits("/v1/audio/speech").pop().unwrap();
    let body: Value = serde_json::from_slice(&q.body).unwrap();
    assert_eq!(body["voice"], "coral");
    assert_eq!(body["speed"], 1.25);
    assert_eq!(body["input"], "Hello there.");
    allow("tts", "cloud-elevenlabs");
    let r = run(a, &call(a, "tts.speak", json!({"text": "Hi."}))).unwrap();
    assert_eq!(r["seconds"], 0.25);
    let q = fx.mock.hits("/v1/text-to-speech/").pop().unwrap();
    assert!(q
        .url
        .starts_with("/v1/text-to-speech/21m00Tcm4TlvDq8ikWAM?output_format=pcm_24000"));
    assert_eq!(q.auth.as_deref(), Some("k-FLUX_ELEVENLABS_KEY"));
    call(
        a,
        "ai.engine.set",
        json!({"role": "tts", "engine": "cloud-free-kokoro", "allow_cloud": true, "voice": "bf_emma"}),
    );
    let r = run(a, &call(a, "tts.speak", json!({"text": "Free speech."}))).unwrap();
    assert_eq!(r["seconds"], 0.75);
    let q = fx
        .mock
        .hits("/spaces/remsky-kokoro-tts-zero/gradio_api/call/generate_speech_from_ui")
        .into_iter()
        .find(|r| r.method == "POST")
        .unwrap();
    let body: Value = serde_json::from_slice(&q.body).unwrap();
    assert_eq!(body["data"], json!(["Free speech.", ["bf_emma"], 1.0]));
    let p = call(a, "project.get", json!({}));
    let media = p["project"]["media"]
        .as_array()
        .or(p["media"].as_array())
        .unwrap();
    let synth: Vec<&Value> = media.iter().filter(|m| m["synthetic"] == true).collect();
    assert_eq!(synth.len(), 3);
    for m in synth {
        let b = std::fs::read(m["path"].as_str().unwrap()).unwrap();
        assert!(String::from_utf8_lossy(&b).contains("Synthetic speech made by Flux with"));
    }

    // Stems: Replicate (upload, prediction, poll, fetch each stem) and the free two-stem split.
    // Not turned on: refused before anything is sent.
    let (code, msg) = fails(a, "tool.stems", json!({"engine": "cloud-replicate-demucs"}));
    assert_eq!(code, "consent_required");
    assert!(msg.contains("hasn't been turned on"), "{msg}");
    assert!(fx.mock.hits("/v1/files").is_empty());
    allow("stems", "cloud-replicate-demucs");
    let r = run(a, &call(a, "tool.stems", json!({}))).unwrap();
    assert_eq!(r["method"], "cloud:cloud-replicate-demucs");
    let names: Vec<&str> = r["stems"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Vocals", "Drums", "Bass", "Other"]);
    let lufs = |n: &str| {
        r["stems"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["name"] == n)
            .unwrap()["lufs"]
            .as_f64()
            .unwrap()
    };
    assert!(
        (lufs("Vocals") - lufs("Drums") - 6.02).abs() < 0.2,
        "the stems Flux got back, at their levels"
    );
    let pred = fx.mock.hits("/v1/predictions");
    let create: Value = serde_json::from_slice(&pred[0].body).unwrap();
    assert_eq!(
        create["input"]["audio"],
        format!("{}/v1/files/f1", fx.mock.base())
    );
    assert!(
        pred.iter()
            .filter(|r| r.url == "/v1/predictions/p1")
            .count()
            >= 2,
        "polled until done"
    );
    assert!(
        fx.mock.hits("/delivery/").iter().all(|r| r.auth.is_none()),
        "the key only goes to the API"
    );
    call(a, "history.undo", json!({}));
    allow("stems", "cloud-free-separation");
    let r = run(a, &call(a, "tool.stems", json!({"mode": "2"}))).unwrap();
    let names: Vec<&str> = r["stems"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Vocals", "Instrumental"]);

    for v in [
        "FLUX_OPENAI_KEY",
        "FLUX_DEEPGRAM_KEY",
        "FLUX_ELEVENLABS_KEY",
        "FLUX_REPLICATE_KEY",
    ] {
        std::env::remove_var(v);
    }
}

#[test]
fn failures_are_clear_and_carry_no_text_audio_or_key() {
    let fx = fixture();
    let a = &fx.a;
    std::env::set_var("FLUX_OPENAI_KEY", "sk-very-secret");
    call(a, "project.new", json!({"name": "Cloud"}));
    call(
        a,
        "media.import",
        json!({"path": song(&fx.d), "wait": true}),
    );
    call(
        a,
        "ai.engine.set",
        json!({"role": "tts", "engine": "cloud-openai-tts", "allow_cloud": true}),
    );
    let text = "my private sentence about the launch";

    fx.mock
        .fail
        .lock()
        .unwrap()
        .push(("/v1/audio/speech".into(), 401));
    let e = run(a, &call(a, "tts.speak", json!({"text": text}))).unwrap_err();
    assert!(
        e.contains("refused the key") && e.contains("nope from the mock"),
        "{e}"
    );
    assert!(!e.contains(text) && !e.contains("sk-very-secret"), "{e}");
    fx.mock.fail.lock().unwrap().clear();
    fx.mock
        .fail
        .lock()
        .unwrap()
        .push(("/v1/audio/speech".into(), 429));
    let e = run(a, &call(a, "tts.speak", json!({"text": text}))).unwrap_err();
    assert!(e.contains("busy") && e.contains("Try again later"), "{e}");
    fx.mock.fail.lock().unwrap().clear();

    // A busy free demo.
    call(
        a,
        "ai.engine.set",
        json!({"role": "transcript", "engine": "cloud-free-whisper", "allow_cloud": true}),
    );
    *fx.mock.gradio_error.lock().unwrap() = true;
    let e = run(a, &call(a, "transcript.make", json!({}))).unwrap_err();
    assert!(
        e.contains("ZeroGPU quota exceeded") && e.contains("choose another engine"),
        "{e}"
    );
    *fx.mock.gradio_error.lock().unwrap() = false;

    // No connection.
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = closed.local_addr().unwrap().port();
    drop(closed);
    call(
        a,
        "ai.cloud.set",
        json!({"provider": "openai", "base_url": format!("http://127.0.0.1:{port}/v1")}),
    );
    let e = run(a, &call(a, "tts.speak", json!({"text": text}))).unwrap_err();
    assert!(e.contains("Couldn't reach OpenAI"), "{e}");
    assert!(!e.contains(text));
    // Back to the default address.
    let r = call(
        a,
        "ai.cloud.set",
        json!({"provider": "openai", "base_url": ""}),
    );
    assert_eq!(r["base_url"], "https://api.openai.com/v1");

    // Nothing of the text went anywhere but the speech requests.
    let others: Vec<Req> = fx
        .mock
        .reqs
        .lock()
        .unwrap()
        .iter()
        .filter(|r| !r.url.starts_with("/v1/audio/speech"))
        .cloned()
        .collect();
    assert!(others.iter().all(|r| !contains(&r.body, text)));
    assert!(contains(&fx.mock.all_bodies(), text));
    std::env::remove_var("FLUX_OPENAI_KEY");
}
