//! Voice model downloads and the voice engine, end to end through the JSON API, against a local mock server and a
//! stub Python runtime (tests/voice_stub: pip, torch and chatterbox stand-ins). Nothing leaves this computer.
//!
//! Covers: confirm before download, checksums, resume after a dropped connection and after cancel, shared files,
//! delete, the runtime install (archive unpack, pip steps with the right index, check), settings and their limits,
//! added model folders, speaking and converting through the runner, synthetic labels, and runner errors.

use flux_core::api::Api;
use flux_core::store::Store;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

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

fn tone(dir: &Path, name: &str, secs: f32) -> String {
    let p = dir.join(name);
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 24000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(&p, spec).unwrap();
    for i in 0..(24000.0 * secs) as usize {
        let t = i as f32 / 24000.0;
        w.write_sample(((std::f32::consts::TAU * 200.0 * t).sin() * 9000.0) as i16)
            .unwrap();
    }
    w.finalize().unwrap();
    p.display().to_string()
}

fn sha(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}

/// Seeded bytes, so files are different and checksums mean something.
fn bytes(n: usize, seed: u32) -> Vec<u8> {
    let mut s = seed;
    (0..n)
        .map(|_| {
            s = s.wrapping_mul(1664525).wrapping_add(1013904223);
            (s >> 24) as u8
        })
        .collect()
}

// ---------------------------------------------------------------- the mock server

/// (path, Range header) of one request.
type Hit = (String, Option<String>);

#[derive(Clone, Default)]
struct Mock {
    port: u16,
    /// (path, Range header) for every request.
    hits: Arc<Mutex<Vec<Hit>>>,
    /// Paths whose next full response is cut off half way.
    cut_next: Arc<Mutex<Vec<String>>>,
    slow: Arc<AtomicBool>,
}

struct Body {
    data: Vec<u8>,
    pos: usize,
    stop: Option<usize>,
    slow: Arc<AtomicBool>,
}
impl Read for Body {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        if let Some(s) = self.stop {
            if self.pos >= s {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::ConnectionReset,
                    "cut",
                ));
            }
        }
        if self.slow.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(15));
        }
        let end = (self.pos + out.len().min(32 * 1024)).min(self.data.len());
        let end = self.stop.map(|s| end.min(s)).unwrap_or(end);
        let n = end - self.pos;
        out[..n].copy_from_slice(&self.data[self.pos..end]);
        self.pos = end;
        Ok(n)
    }
}

fn serve(files: HashMap<String, Vec<u8>>) -> Mock {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let port = server.server_addr().to_ip().unwrap().port();
    let m = Mock {
        port,
        ..Default::default()
    };
    let m2 = m.clone();
    std::thread::spawn(move || {
        for req in server.incoming_requests() {
            let path = req.url().to_string();
            let range = req
                .headers()
                .iter()
                .find(|h| h.field.equiv("Range"))
                .map(|h| h.value.to_string());
            m2.hits.lock().unwrap().push((path.clone(), range.clone()));
            let Some(data) = files.get(&path).cloned() else {
                let _ = req.respond(tiny_http::Response::empty(404));
                continue;
            };
            let from = range
                .as_deref()
                .and_then(|r| r.strip_prefix("bytes="))
                .and_then(|r| r.trim_end_matches('-').parse::<usize>().ok())
                .unwrap_or(0);
            let total = data.len();
            let cut = {
                let mut c = m2.cut_next.lock().unwrap();
                let i = c.iter().position(|x| *x == path);
                i.map(|i| c.remove(i)).is_some()
            };
            let slow = m2.slow.clone();
            std::thread::spawn(move || {
                let body = data[from..].to_vec();
                let len = body.len();
                let stop = cut.then_some(len / 2);
                let mut headers = vec![];
                let status = if from > 0 {
                    headers.push(
                        tiny_http::Header::from_bytes(
                            "Content-Range",
                            format!("bytes {from}-{}/{total}", total - 1),
                        )
                        .unwrap(),
                    );
                    206
                } else {
                    200
                };
                let r = tiny_http::Response::new(
                    tiny_http::StatusCode(status),
                    headers,
                    Body {
                        data: body,
                        pos: 0,
                        stop,
                        slow,
                    },
                    Some(len),
                    None,
                );
                let _ = req.respond(r);
            });
        }
    });
    m
}

impl Mock {
    fn ranges_for(&self, name: &str) -> Vec<Option<String>> {
        self.hits
            .lock()
            .unwrap()
            .iter()
            .filter(|h| h.0.ends_with(name))
            .map(|h| h.1.clone())
            .collect()
    }
}

// ---------------------------------------------------------------- the fixture

fn python() -> String {
    for p in ["python3", "python"] {
        if std::process::Command::new(p)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return p.to_string();
        }
    }
    panic!("these tests need Python 3 on PATH to run the voice runner against its stub packages");
}

fn platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

fn targz(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut b = tar::Builder::new(flate2::write::GzEncoder::new(
        vec![],
        flate2::Compression::fast(),
    ));
    for (name, data) in files {
        let mut h = tar::Header::new_gnu();
        h.set_size(data.len() as u64);
        h.set_mode(0o644);
        h.set_cksum();
        b.append_data(&mut h, name, *data).unwrap();
    }
    b.into_inner().unwrap().finish().unwrap()
}

const REV: &str = "0123456789abcdef0123456789abcdef01234567";

struct Fx {
    _one_at_a_time: std::sync::MutexGuard<'static, ()>,
    a: Api,
    d: PathBuf,
    mock: Mock,
    log: PathBuf,
}

/// The stub log path is an environment variable, so these tests take turns.
static SERIAL: Mutex<()> = Mutex::new(());

fn fixture() -> Fx {
    let guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("FLUX_NULL_AUDIO", "1");
    let d = tmp("voicemodels");
    let shared = bytes(400_000, 1);
    let turbo = br#"{"kind":"turbo"}"#.to_vec();
    let english = br#"{"kind":"english"}"#.to_vec();
    let big = bytes(3_000_000, 7);
    let pyarc = targz(&[("python/README.txt", b"a portable Python would be here")]);
    let base = format!("/test/voice/resolve/{REV}/");
    let mut files = HashMap::new();
    files.insert(format!("{base}a.bin"), shared.clone());
    files.insert(format!("{base}turbo.json"), turbo.clone());
    files.insert(format!("{base}english.json"), english.clone());
    files.insert(format!("{base}big.bin"), big.clone());
    files.insert(format!("{base}bad.bin"), bytes(1000, 3));
    files.insert("/py.tar.gz".to_string(), pyarc.clone());
    let mock = serve(files);
    let hub = format!("http://127.0.0.1:{}", mock.port);
    let f = |p: &str, b: &[u8]| json!({"path": p, "size": b.len(), "sha256": sha(b)});
    let stub = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("voice_stub");
    let catalog = json!({
        "hub": hub,
        "repos": {"r": {"repo": "test/voice", "rev": REV}},
        "models": [
            {"id": "t-turbo", "name": "Test Turbo", "role": "speech", "loader": "turbo", "repo": "r", "licence": "MIT",
             "languages": ["en"], "sounds": true, "min_vram_gb": 4.0, "note": "n",
             "files": [f("a.bin", &shared), f("turbo.json", &turbo)],
             "params": [{"key": "temperature", "label": "Variety", "min": 0.05, "max": 2.0, "step": 0.05, "default": 0.8, "tip": "t"}]},
            {"id": "t-english", "name": "Test English", "role": "speech", "loader": "english", "repo": "r", "licence": "MIT",
             "languages": ["en"], "sounds": false, "min_vram_gb": 6.0, "note": "n",
             "files": [f("a.bin", &shared), f("english.json", &english)],
             "params": [{"key": "exaggeration", "label": "Emotion", "min": 0.25, "max": 2.0, "step": 0.05, "default": 0.5, "tip": "t"}]},
            {"id": "t-vc", "name": "Test Converter", "role": "convert", "loader": "vc", "repo": "r", "licence": "MIT",
             "languages": [], "sounds": false, "min_vram_gb": 6.0, "note": "n", "files": [f("a.bin", &shared)]},
            {"id": "t-big", "name": "Test Big", "role": "speech", "loader": "turbo", "repo": "r", "licence": "MIT",
             "languages": ["en"], "sounds": true, "min_vram_gb": 4.0, "note": "n", "files": [f("big.bin", &big)]},
            {"id": "t-bad", "name": "Test Bad", "role": "speech", "loader": "turbo", "repo": "r", "licence": "MIT",
             "languages": ["en"], "sounds": true, "min_vram_gb": 4.0, "note": "n",
             "files": [{"path": "bad.bin", "size": 1000, "sha256": "0".repeat(64)}]}
        ],
        "runtime": {
            "id": "voice-engine", "name": "Voice engine", "version": "test-1", "note": "n",
            "python": {platform(): {"url": format!("{hub}/py.tar.gz"), "size": pyarc.len(), "sha256": sha(&pyarc), "exe": "python/README.txt"}},
            "steps": [
                {"name": "Installing PyTorch", "args": ["torch==2.6.0"], "index": {"cpu": "https://example.invalid/cpu", "cuda": "https://example.invalid/cu124"}},
                {"name": "Installing Chatterbox", "args": ["chatterbox-tts==0.1.7"]}
            ],
            "approx_mb": {"cpu": 1, "cuda": 1, "mps": 1},
            "python_override": python(),
            "pythonpath": stub.display().to_string()
        }
    });
    let data = d.join("data");
    std::fs::create_dir_all(data.join("Models")).unwrap();
    std::fs::write(
        data.join("Models").join("catalog.override.json"),
        serde_json::to_vec_pretty(&catalog).unwrap(),
    )
    .unwrap();
    let log = d.join("stub.log");
    std::env::set_var("FLUX_STUB_LOG", &log);
    let a = Api::new(Store::at(data).unwrap(), true);
    Fx {
        _one_at_a_time: guard,
        a,
        d,
        mock,
        log,
    }
}

fn log_lines(fx: &Fx) -> Vec<Value> {
    std::fs::read_to_string(&fx.log)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

fn model<'a>(list: &'a Value, id: &str) -> &'a Value {
    list["models"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == id)
        .unwrap()
}

fn profile(fx: &Fx) {
    let r = tone(&fx.d, "ref.wav", 8.0);
    let c = tone(&fx.d, "consent.wav", 4.0);
    call(
        &fx.a,
        "clone.profile.create",
        json!({"name": "Robin", "owner": "me", "consent": true, "consent_audio": c, "reference": r}),
    );
}

#[test]
fn models_download_verified_resume_share_and_delete() {
    let fx = fixture();
    let a = &fx.a;
    let m = call(a, "clone.models", json!({}));
    let t = model(&m, "t-turbo");
    assert_eq!(t["installed"], false);
    assert_eq!(t["size"], 400_000 + 16);
    assert_eq!(m["runtime"]["installed"], false);

    // Nothing happens without the person's yes, and the refusal says what and how much.
    let (code, msg) = fails(a, "clone.model.download", json!({"model": "t-turbo"}));
    assert_eq!(code, "confirm_required");
    assert!(
        msg.contains("Test Turbo") && msg.contains("MIT") && msg.contains("voice engine"),
        "{msg}"
    );
    assert!(fx.mock.hits.lock().unwrap().is_empty());
    assert_eq!(
        fails(
            a,
            "clone.model.download",
            json!({"model": "nope", "confirm": true})
        )
        .0,
        "not_found"
    );
    assert_eq!(
        fails(
            a,
            "clone.model.download",
            json!({"model": "t-turbo", "confirm": true, "device_kind": "rocm"})
        )
        .0,
        "bad_params"
    );

    // The first download installs the engine, then the files; a dropped connection resumes with a Range request.
    fx.mock
        .cut_next
        .lock()
        .unwrap()
        .push(format!("/test/voice/resolve/{REV}/a.bin"));
    let r = call(
        a,
        "clone.model.download",
        json!({"model": "t-turbo", "confirm": true, "device_kind": "cpu", "wait": true}),
    );
    assert_eq!(r["result"]["model"], "t-turbo");
    assert_eq!(r["result"]["runtime"]["torch"], "2.6.0+stub");
    let ranges = fx.mock.ranges_for("a.bin");
    assert_eq!(ranges.len(), 2, "{ranges:?}");
    assert_eq!(ranges[0], None);
    assert!(
        ranges[1].as_deref().unwrap().starts_with("bytes=")
            && ranges[1].as_deref() != Some("bytes=0-")
    );
    // pip was asked for pinned packages, with the processor build's index.
    let pip: Vec<Vec<String>> = log_lines(&fx)
        .iter()
        .filter_map(|l| serde_json::from_value(l["pip"].clone()).ok())
        .collect();
    assert_eq!(pip.len(), 2);
    assert!(pip[0].contains(&"torch==2.6.0".to_string()));
    assert!(pip[0]
        .windows(2)
        .any(|w| w[0] == "--index-url" && w[1] == "https://example.invalid/cpu"));
    assert!(
        pip[1].contains(&"chatterbox-tts==0.1.7".to_string())
            && !pip[1].contains(&"--index-url".to_string())
    );
    let m = call(a, "clone.models", json!({}));
    assert_eq!(model(&m, "t-turbo")["installed"], true);
    assert_eq!(m["runtime"]["installed"], true);
    assert_eq!(m["runtime"]["info"]["kind"], "cpu");
    // The converter needs only the shared file, so it is already there; English needs one more small file.
    assert_eq!(model(&m, "t-vc")["installed"], true);
    assert_eq!(model(&m, "t-english")["have"], 400_000);
    assert_eq!(
        model(&m, "t-english")["partial"],
        false,
        "shared files are not a stopped download"
    );
    let again = call(
        a,
        "clone.model.download",
        json!({"model": "t-vc", "confirm": true}),
    );
    assert_eq!(again["installed"], true);
    let before = fx.mock.hits.lock().unwrap().len();
    call(
        a,
        "clone.model.download",
        json!({"model": "t-english", "confirm": true, "wait": true}),
    );
    let new_hits: Vec<String> = fx.mock.hits.lock().unwrap()[before..]
        .iter()
        .map(|h| h.0.clone())
        .collect();
    assert_eq!(
        new_hits,
        vec![format!("/test/voice/resolve/{REV}/english.json")],
        "only the missing file, no engine again"
    );

    // A file that doesn't match its checksum is thrown away and never used.
    let (_, msg) = fails(
        a,
        "clone.model.download",
        json!({"model": "t-bad", "confirm": true, "wait": true}),
    );
    assert!(msg.contains("checksum"), "{msg}");
    let files = fx.d.join("data").join("Models").join("files");
    let repo = std::fs::read_dir(&files)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert!(!repo.join("bad.bin").exists() && !repo.join("bad.bin.part").exists());
    assert_eq!(
        model(&call(a, "clone.models", json!({})), "t-bad")["installed"],
        false
    );

    // Cancel half way, then download again: it carries on from where it stopped.
    fx.mock.slow.store(true, Ordering::Relaxed);
    let job = call(
        a,
        "clone.model.download",
        json!({"model": "t-big", "confirm": true}),
    )["job"]
        .as_str()
        .unwrap()
        .to_string();
    let t0 = std::time::Instant::now();
    loop {
        let j = call(a, "job.get", json!({"id": job}));
        if j["progress"].as_f64().unwrap() > 0.1 {
            break;
        }
        assert!(
            t0.elapsed() < Duration::from_secs(20),
            "download never started: {j}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let listed = model(&call(a, "clone.models", json!({})), "t-big")["downloading"].clone();
    assert_eq!(listed["job"], job.as_str());
    assert_eq!(
        fails(a, "clone.model.delete", json!({"model": "t-big"})).0,
        "busy"
    );
    call(a, "job.cancel", json!({"id": job}));
    let j = call(a, "job.get", json!({"id": job, "wait_s": 20}));
    assert_eq!(j["state"], "cancelled");
    let part = std::fs::metadata(repo.join("big.bin.part")).unwrap().len();
    assert!(part > 0 && part < 3_000_000, "{part}");
    assert_eq!(
        model(&call(a, "clone.models", json!({})), "t-big")["partial"],
        true
    );
    fx.mock.slow.store(false, Ordering::Relaxed);
    call(
        a,
        "clone.model.download",
        json!({"model": "t-big", "confirm": true, "wait": true}),
    );
    let ranges = fx.mock.ranges_for("big.bin");
    let resumed_from: usize = ranges
        .last()
        .unwrap()
        .as_deref()
        .unwrap()
        .trim_start_matches("bytes=")
        .trim_end_matches('-')
        .parse()
        .unwrap();
    assert!(
        resumed_from > 0 && resumed_from as u64 <= part,
        "{ranges:?}"
    );
    assert_eq!(
        std::fs::read(repo.join("big.bin")).unwrap(),
        bytes(3_000_000, 7)
    );

    // Deleting keeps files another downloaded model still uses.
    let del = call(a, "clone.model.delete", json!({"model": "t-turbo"}));
    assert_eq!(del["kept_shared"], 1);
    assert!(repo.join("a.bin").exists() && !repo.join("turbo.json").exists());
    let m = call(a, "clone.models", json!({}));
    assert_eq!(model(&m, "t-turbo")["installed"], false);
    assert_eq!(model(&m, "t-english")["installed"], true);
    call(a, "clone.model.delete", json!({"model": "voice-engine"}));
    assert_eq!(
        call(a, "clone.models", json!({}))["runtime"]["installed"],
        false
    );
}

#[test]
fn chosen_models_speak_and_convert_with_their_settings_labelled_synthetic() {
    let fx = fixture();
    let a = &fx.a;
    profile(&fx);
    // Not downloaded yet: says what to download.
    let (code, msg) = fails(a, "clone.speak", json!({"voice": "robin", "text": "hi"}));
    assert_eq!(code, "not_installed");
    assert!(msg.contains("Test Turbo isn't downloaded"), "{msg}");

    // Settings: roles, ranges and values are checked.
    assert_eq!(
        fails(a, "clone.settings.set", json!({"speech": "t-vc"})).0,
        "bad_params"
    );
    assert_eq!(
        fails(a, "clone.settings.set", json!({"convert": "t-turbo"})).0,
        "bad_params"
    );
    assert_eq!(
        fails(a, "clone.settings.set", json!({"device": "gpu"})).0,
        "bad_params"
    );
    assert_eq!(
        fails(a, "clone.settings.set", json!({"language": "xx"})).0,
        "bad_params"
    );
    assert_eq!(
        fails(a, "clone.settings.set", json!({"seed": -1})).0,
        "bad_params"
    );
    assert_eq!(
        fails(
            a,
            "clone.settings.set",
            json!({"params": {"t-turbo": {"nope": 1}}})
        )
        .0,
        "bad_params"
    );
    let s = call(
        a,
        "clone.settings.set",
        json!({"speech": "t-turbo", "convert": "t-vc", "device": "cpu", "seed": 42, "params": {"t-turbo": {"temperature": 9}}}),
    );
    assert_eq!(
        s["params"]["t-turbo"]["temperature"], 2.0,
        "clamped to the model's range"
    );
    let s = call(
        a,
        "clone.settings.set",
        json!({"params": {"t-turbo": {"temperature": 0.6}}}),
    );
    assert_eq!(s["params"]["t-turbo"]["temperature"], 0.6);
    assert_eq!(
        s["params"]["t-english"]["exaggeration"], 0.5,
        "defaults show for untouched models"
    );
    assert_eq!(s["settings"]["seed"], 42);

    let (code, msg) = fails(a, "clone.speak", json!({"voice": "robin", "text": "hi"}));
    assert_eq!(code, "not_installed");
    assert!(msg.contains("Test Turbo isn't downloaded"), "{msg}");
    call(
        a,
        "clone.model.download",
        json!({"model": "t-turbo", "confirm": true, "device_kind": "cpu", "wait": true}),
    );

    // Speaking onto the open project: one clip on Generated speech, labelled synthetic, pauses kept as silence.
    call(a, "project.new", json!({"name": "Cloning"}));
    let r = call(
        a,
        "clone.speak",
        json!({"voice": "robin", "text": "[hmm] So [laugh] [pause 1.5s] (um) yes", "wait": true}),
    )["result"]
        .clone();
    assert_eq!(r["synthetic"], true);
    assert_eq!(r["model"], "t-turbo");
    assert_eq!(r["device"], "cpu");
    let said = ["Hmm, So [laugh]", "Um, yes"];
    let want = said
        .iter()
        .map(|t| (0.02 * t.len() as f64 * 24000.0) as usize)
        .sum::<usize>() as f64
        / 24000.0
        + 1.5;
    assert!(
        (r["seconds"].as_f64().unwrap() - want).abs() < 0.01,
        "{r} vs {want}"
    );
    let gens: Vec<Value> = log_lines(&fx)
        .into_iter()
        .filter(|l| l["generate"] == "turbo")
        .collect();
    assert_eq!(
        gens.iter()
            .map(|g| g["text"].as_str().unwrap())
            .collect::<Vec<_>>(),
        said
    );
    assert!(gens.iter().all(|g| g["temperature"] == 0.6));
    assert!(gens[0]["audio_prompt_path"]
        .as_str()
        .unwrap()
        .ends_with("reference.wav"));
    assert!(
        gens[1]["audio_prompt_path"].is_null(),
        "the voice is given once"
    );
    assert!(log_lines(&fx)
        .iter()
        .filter(|l| l["load"].is_string())
        .all(|l| l["offline"] == "1" && l["device"] == "cpu"));
    let p = call(a, "project.get", json!({}));
    let tr = p["project"]["tracks"]
        .as_array()
        .or(p["tracks"].as_array())
        .unwrap();
    assert!(tr.iter().any(|t| t["name"] == "Generated speech"));
    let media = p["project"]["media"]
        .as_array()
        .or(p["media"].as_array())
        .unwrap();
    let m = media.iter().find(|m| m["synthetic"] == true).unwrap();
    let wavb = std::fs::read(m["path"].as_str().unwrap()).unwrap();
    let text = String::from_utf8_lossy(&wavb);
    assert!(text.contains("Synthetic speech in the voice of Robin") && text.contains("Test Turbo"));
    // Same seed and script, same take.
    let r2 = call(
        a,
        "clone.speak",
        json!({"voice": "robin", "text": "[hmm] So [laugh] [pause 1.5s] (um) yes", "wait": true}),
    )["result"]
        .clone();
    assert_eq!(r2["seconds"], r["seconds"]);
    // Undo removes the generated clip like any edit.
    call(a, "history.undo", json!({}));

    // A model that can't perform sounds leaves them out and says so; it is picked per call.
    call(
        a,
        "clone.model.download",
        json!({"model": "t-english", "confirm": true, "wait": true}),
    );
    let r = call(a, "clone.speak", json!({"voice": "robin", "text": "So [laugh] fine (sighs)", "model": "t-english", "wait": true}))["result"].clone();
    assert_eq!(r["sounds_left_out"], 2);
    let last = log_lines(&fx)
        .into_iter()
        .rfind(|l| l["generate"] == "english")
        .unwrap();
    assert_eq!(last["text"], "So fine");
    assert_eq!(last["exaggeration"], 0.5);
    assert_eq!(
        fails(
            a,
            "clone.speak",
            json!({"voice": "robin", "text": "hi", "model": "t-vc"})
        )
        .0,
        "bad_params"
    );

    // Converting your own take, kept exactly; with no project open it goes to the export folder.
    call(a, "project.close", json!({}));
    let out = fx.d.join("exports");
    call(
        a,
        "settings.set",
        json!({"values": {"export_dir": out.display().to_string()}}),
    );
    let take = tone(&fx.d, "take.wav", 2.0);
    let r = call(
        a,
        "clone.convert",
        json!({"voice": "robin", "path": take, "wait": true}),
    )["result"]
        .clone();
    let path = PathBuf::from(r["path"].as_str().unwrap());
    assert!(path.starts_with(&out));
    let rd = hound::WavReader::open(&path).unwrap();
    assert_eq!(rd.len(), 36000);
    assert!(String::from_utf8_lossy(&std::fs::read(&path).unwrap()).contains("Test Converter"));

    // A runner error comes back as a clear failure, with no audio or text in it.
    let (code, msg) = fails(
        a,
        "clone.speak",
        json!({"voice": "robin", "text": "please explode", "wait": true}),
    );
    assert_eq!(code, "failed");
    assert!(
        msg.contains("stub model was asked to fail") && !msg.contains("please explode"),
        "{msg}"
    );

    // Folders of your own: checked for the files their kind needs, usable, removable.
    let mine = fx.d.join("my-model");
    std::fs::create_dir_all(&mine).unwrap();
    std::fs::write(mine.join("a.bin"), b"x").unwrap();
    assert_eq!(
        fails(
            a,
            "clone.model.add_local",
            json!({"path": mine.display().to_string(), "loader": "turbo"})
        )
        .0,
        "bad_file"
    );
    assert_eq!(
        fails(
            a,
            "clone.model.add_local",
            json!({"path": mine.display().to_string(), "loader": "bark"})
        )
        .0,
        "bad_params"
    );
    std::fs::write(mine.join("turbo.json"), b"{}").unwrap();
    let id = call(
        a,
        "clone.model.add_local",
        json!({"path": mine.display().to_string(), "loader": "turbo", "name": "My tuned voice"}),
    )["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(id, "local-my-tuned-voice");
    let listed = call(a, "clone.models", json!({}));
    assert_eq!(model(&listed, &id)["local"], true);
    call(
        a,
        "clone.settings.set",
        json!({"speech": id, "params": {&id: {"temperature": 1.1}}}),
    );
    let r = call(
        a,
        "clone.speak",
        json!({"voice": "robin", "text": "mine", "wait": true}),
    )["result"]
        .clone();
    assert_eq!(r["model"], id.as_str());
    let last = log_lines(&fx)
        .into_iter()
        .rfind(|l| l["generate"] == "turbo")
        .unwrap();
    assert_eq!(last["temperature"], 1.1);
    call(a, "clone.model.remove_local", json!({"id": id}));
    assert!(
        mine.join("turbo.json").exists(),
        "your files are left alone"
    );
    assert_eq!(
        call(a, "clone.settings.get", json!({}))["settings"]["speech"],
        "t-turbo",
        "back to the first speech model this catalog has"
    );
}
