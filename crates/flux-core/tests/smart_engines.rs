//! Engines for stems, transcripts and text to speech, end to end through the JSON API: choosing an engine, the
//! suggestion for this computer, downloading a model with its own packages, and running it. Against a local mock
//! server and stub Python packages (tests/voice_stub: faster_whisper, demucs and kokoro stand-ins that check what
//! Flux hands them). Nothing leaves this computer.

use flux_core::api::Api;
use flux_core::store::Store;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
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

fn sha(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}

fn bytes(n: usize, seed: u32) -> Vec<u8> {
    let mut s = seed;
    (0..n)
        .map(|_| {
            s = s.wrapping_mul(1664525).wrapping_add(1013904223);
            (s >> 24) as u8
        })
        .collect()
}

/// Two seconds of stereo at 48 kHz, different tones left and right.
fn music(dir: &Path) -> String {
    let p = dir.join("song.wav");
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 48000,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w = hound::WavWriter::create(&p, spec).unwrap();
    for i in 0..96000 {
        let t = i as f32 / 48000.0;
        w.write_sample((std::f32::consts::TAU * 220.0 * t).sin() * 0.4)
            .unwrap();
        w.write_sample((std::f32::consts::TAU * 330.0 * t).sin() * 0.3)
            .unwrap();
    }
    w.finalize().unwrap();
    p.display().to_string()
}

/// A file server that records every path asked for.
fn serve(files: HashMap<String, Vec<u8>>) -> (u16, Arc<Mutex<Vec<String>>>) {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let port = server.server_addr().to_ip().unwrap().port();
    let hits = Arc::new(Mutex::new(vec![]));
    let h2 = hits.clone();
    std::thread::spawn(move || {
        for req in server.incoming_requests() {
            let path = req.url().to_string();
            h2.lock().unwrap().push(path.clone());
            let r = match files.get(&path) {
                Some(d) => req.respond(tiny_http::Response::from_data(d.clone())),
                None => req.respond(tiny_http::Response::empty(404)),
            };
            let _ = r;
        }
    });
    (port, hits)
}

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
    panic!("these tests need Python 3 on PATH to run the model runner against its stub packages");
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
    hits: Arc<Mutex<Vec<String>>>,
    log: PathBuf,
}

/// The stub log path is an environment variable, so these tests take turns.
static SERIAL: Mutex<()> = Mutex::new(());

fn fixture() -> Fx {
    let guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("FLUX_NULL_AUDIO", "1");
    let d = tmp("smart");
    let pyarc = targz(&[("python/README.txt", b"a portable Python would be here")]);
    // (repo, path, bytes); the Demucs definition comes from somewhere else, by its own URL.
    let blobs: Vec<(&str, &str, Vec<u8>)> = vec![
        ("w", "config.json", br#"{"whisper":1}"#.to_vec()),
        ("w", "model.bin", bytes(50_000, 1)),
        ("d", "w.th", bytes(80_000, 2)),
        ("d", "w6.th", bytes(80_000, 3)),
        ("k", "config.json", br#"{"kokoro":1}"#.to_vec()),
        ("k", "kokoro-v1_0.pth", bytes(60_000, 4)),
        ("k", "voices/af_heart.pt", bytes(500, 5)),
        ("k", "voices/bm_george.pt", bytes(500, 6)),
    ];
    let yaml = b"models: ['w']\n".to_vec();
    let yaml6 = b"models: ['w6']\n".to_vec();
    let mut files = HashMap::new();
    for (repo, path, b) in &blobs {
        files.insert(format!("/test/{repo}/resolve/{REV}/{path}"), b.clone());
    }
    files.insert("/raw/tdemucs.yaml".into(), yaml.clone());
    files.insert("/raw/tdemucs_6s.yaml".into(), yaml6.clone());
    files.insert("/py.tar.gz".to_string(), pyarc.clone());
    let (port, hits) = serve(files);
    let hub = format!("http://127.0.0.1:{port}");
    let f = |repo: &str, p: &str| {
        let b = &blobs.iter().find(|x| x.0 == repo && x.1 == p).unwrap().2;
        json!({"path": p, "size": b.len(), "sha256": sha(b)})
    };
    let url = |p: &str, b: &[u8]| json!({"path": p, "size": b.len(), "sha256": sha(b), "url": format!("{hub}/raw/{p}")});
    let stub = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("voice_stub");
    let demucs_pkgs = json!([
        {"name": "Installing Demucs", "args": ["--no-deps", "demucs==4.0.1"]},
        {"name": "Installing support", "args": ["einops==0.8.2"]}
    ]);
    let catalog = json!({
        "hub": hub,
        "repos": {
            "w": {"repo": "test/w", "rev": REV},
            "d": {"repo": "test/d", "rev": REV},
            "k": {"repo": "test/k", "rev": REV}
        },
        "models": [
            {"id": "t-whisper", "name": "Test Whisper", "role": "transcript", "loader": "whisper", "repo": "w",
             "licence": "MIT", "tier": "light", "languages": ["en", "fr"], "sounds": false, "min_vram_gb": 0.0,
             "note": "n", "files": [f("w", "config.json"), f("w", "model.bin")],
             "install": [{"name": "Installing Whisper", "args": ["--no-deps", "faster-whisper==1.2.1"]}]},
            {"id": "t-demucs", "name": "Test Demucs", "role": "stems", "loader": "demucs", "repo": "d",
             "licence": "MIT", "tier": "balanced", "languages": [], "sounds": false, "min_vram_gb": 0.0, "note": "n",
             "files": [url("tdemucs.yaml", &yaml), f("d", "w.th")], "install": demucs_pkgs},
            {"id": "t-demucs-6s", "name": "Test Demucs 6", "role": "stems", "loader": "demucs", "repo": "d",
             "licence": "MIT", "tier": "extra", "languages": [], "sounds": false, "min_vram_gb": 0.0, "note": "n",
             "files": [url("tdemucs_6s.yaml", &yaml6), f("d", "w6.th")], "install": demucs_pkgs},
            {"id": "t-kokoro", "name": "Test Kokoro", "role": "tts", "loader": "kokoro", "repo": "k",
             "licence": "Apache-2.0", "tier": "light", "languages": ["en"], "sounds": false, "min_vram_gb": 0.0,
             "note": "n",
             "files": [f("k", "config.json"), f("k", "kokoro-v1_0.pth"), f("k", "voices/af_heart.pt"), f("k", "voices/bm_george.pt")],
             "voices": [{"id": "af_heart", "name": "Heart", "language": "en-US"}, {"id": "bm_george", "name": "George", "language": "en-GB"}],
             "params": [{"key": "speed", "label": "Speed", "min": 0.5, "max": 2.0, "step": 0.05, "default": 1.0, "tip": "t"}],
             "install": [
                {"name": "Installing Kokoro", "args": ["--no-deps", "kokoro==0.9.4"]},
                {"name": "Installing support", "args": ["einops==0.8.2"]}
             ]}
        ],
        "runtime": {
            "id": "voice-engine", "name": "Voice engine", "version": "test-1", "note": "n",
            "python": {format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH): {"url": format!("{hub}/py.tar.gz"), "size": pyarc.len(), "sha256": sha(&pyarc), "exe": "python/README.txt"}},
            "steps": [{"name": "Installing PyTorch", "args": ["torch==2.6.0"]}],
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
    std::env::set_var("FLUX_STUB_PIP_SLEEP", "0");
    let a = Api::new(Store::at(data).unwrap(), true);
    Fx {
        _one_at_a_time: guard,
        a,
        d,
        hits,
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

fn pips(fx: &Fx) -> Vec<Vec<String>> {
    log_lines(fx)
        .iter()
        .filter_map(|l| serde_json::from_value(l["pip"].clone()).ok())
        .collect()
}

fn role<'a>(e: &'a Value, r: &str) -> &'a Value {
    e["roles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["role"] == r)
        .unwrap_or_else(|| panic!("no {r} in {e}"))
}

fn engine<'a>(e: &'a Value, r: &str, id: &str) -> &'a Value {
    role(e, r)["engines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"] == id)
        .unwrap_or_else(|| panic!("no {id} in {r}"))
}

fn finish(a: &Api, r: &Value) -> Value {
    let job = r["job"].as_str().unwrap_or_else(|| panic!("{r}"));
    let j = call(a, "job.get", json!({"id": job, "wait_s": 60}));
    assert_eq!(j["state"], "done", "{j}");
    j["result"].clone()
}

fn download(a: &Api, id: &str) {
    finish(
        a,
        &call(
            a,
            "ai.model.download",
            json!({"model": id, "confirm": true, "device_kind": "cpu"}),
        ),
    );
}

#[test]
fn engines_are_listed_suggested_and_chosen() {
    let fx = fixture();
    let a = &fx.a;
    let e = call(a, "ai.engines", json!({}));
    let roles: Vec<&str> = e["roles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["role"].as_str().unwrap())
        .collect();
    assert_eq!(roles, ["stems", "transcript", "tts"]);
    for r in &roles {
        let x = role(&e, r);
        // Nothing chosen yet: the suggestion once downloaded, until then the built-in engine (none for transcripts).
        let sug = x["engines"]
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["id"] == x["suggested"])
            .unwrap();
        let base = x["engines"]
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["kind"] == "builtin" && g["installed"] == true);
        let want = match (sug["installed"] == true, base) {
            (false, Some(b)) => b["id"].clone(),
            _ => x["suggested"].clone(),
        };
        assert_eq!(x["choice"], want, "nothing chosen yet: {x}");
        assert!(!x["why"].as_str().unwrap().is_empty());
        let chosen: Vec<_> = x["engines"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|g| g["chosen"] == true)
            .collect();
        assert_eq!(chosen.len(), 1, "{x}");
    }
    assert_eq!(
        role(&e, "stems")["choice"],
        "spectral",
        "works before any download"
    );
    // The built-in split needs nothing; the specialist six-stem model is offered but never suggested.
    assert_eq!(engine(&e, "stems", "spectral")["installed"], true);
    assert_eq!(engine(&e, "stems", "t-demucs-6s")["suggested"], false);
    assert_eq!(engine(&e, "stems", "t-demucs")["installed"], false);
    assert_eq!(
        engine(&e, "tts", "t-kokoro")["voices"][1]["id"],
        "bm_george"
    );
    // Only the transcript model is offered for transcripts; voice cloning's list keeps to its own models.
    assert_eq!(
        role(&e, "transcript")["engines"].as_array().unwrap().len(),
        1
    );
    let clone = call(a, "clone.models", json!({}));
    assert!(clone["models"].as_array().unwrap().iter().all(|m| ![
        "t-whisper",
        "t-demucs",
        "t-kokoro"
    ]
    .contains(&m["id"].as_str().unwrap())));

    // Choosing: checked against the job's engines, saved, and kept.
    let (code, msg) = fails(
        a,
        "ai.engine.set",
        json!({"role": "stems", "engine": "t-kokoro"}),
    );
    assert_eq!(code, "bad_params");
    assert!(
        msg.contains("spectral") && msg.contains("t-demucs"),
        "{msg}"
    );
    assert_eq!(
        fails(a, "ai.engine.set", json!({"role": "mixing", "engine": "x"})).0,
        "bad_params"
    );
    let s = call(
        a,
        "ai.engine.set",
        json!({"role": "stems", "engine": "t-demucs-6s"}),
    );
    assert_eq!(role(&s, "stems")["choice"], "t-demucs-6s");
    call(
        a,
        "ai.engine.set",
        json!({"role": "tts", "engine": "t-kokoro", "voice": "bm_george"}),
    );
    let e = call(a, "ai.engines", json!({"role": "tts"}));
    assert_eq!(e["roles"].as_array().unwrap().len(), 1);
    assert_eq!(role(&e, "tts")["choice"], "t-kokoro");
    assert_eq!(role(&e, "tts")["options"]["voice"], "bm_george");
    assert_eq!(engine(&e, "tts", "t-kokoro")["chosen"], true);
}

#[test]
fn models_download_with_their_packages_and_run_at_exact_positions() {
    let fx = fixture();
    let a = &fx.a;
    call(a, "project.new", json!({"name": "Smart"}));
    call(
        a,
        "media.import",
        json!({"path": music(&fx.d), "wait": true}),
    );
    let input = call(a, "analysis.loudness", json!({}))["loudness"]["integrated"]
        .as_f64()
        .unwrap();

    // Not downloaded: says where to get it, and nothing is fetched.
    let (code, msg) = fails(a, "tool.stems", json!({"engine": "t-demucs"}));
    assert_eq!(code, "not_installed");
    assert!(
        msg.contains("Test Demucs") && msg.contains("Smart tools"),
        "{msg}"
    );
    assert_eq!(
        fails(a, "transcript.make", json!({"engine": "t-whisper"})).0,
        "not_installed"
    );
    let (code, msg) = fails(a, "ai.model.download", json!({"model": "t-demucs"}));
    assert_eq!(code, "confirm_required");
    assert!(msg.contains("Test Demucs") && msg.contains("MIT"), "{msg}");
    assert!(fx.hits.lock().unwrap().is_empty());

    // The first model brings the engine and its own packages; the definition comes from its own address.
    download(a, "t-demucs");
    let p = pips(&fx);
    assert_eq!(p.len(), 3, "{p:?}");
    assert!(p[0].contains(&"torch==2.6.0".to_string()));
    assert!(
        p[1].contains(&"--no-deps".to_string()) && p[1].contains(&"demucs==4.0.1".to_string()),
        "{p:?}"
    );
    assert!(p[2].contains(&"einops==0.8.2".to_string()));
    assert!(fx
        .hits
        .lock()
        .unwrap()
        .contains(&"/raw/tdemucs.yaml".to_string()));
    let e = call(a, "ai.engines", json!({"role": "stems"}));
    assert_eq!(engine(&e, "stems", "t-demucs")["installed"], true);
    assert_eq!(engine(&e, "stems", "t-demucs")["packages"], true);

    // Four stems, vocals first, each at its share of the input's level, then muted source.
    let r = finish(a, &call(a, "tool.stems", json!({"engine": "t-demucs"})));
    assert_eq!(r["method"], "model:t-demucs");
    let names: Vec<&str> = r["stems"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Vocals", "Drums", "Bass", "Other"]);
    let level = |name: &str, r: &Value| {
        r["stems"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["name"] == name)
            .unwrap()["lufs"]
            .as_f64()
            .unwrap()
    };
    for (name, gain) in [("Vocals", 0.5f64), ("Drums", 0.25), ("Bass", 0.125)] {
        let want = input + 20.0 * gain.log10();
        let got = level(name, &r);
        assert!((got - want).abs() < 0.3, "{name}: {got} vs {want}");
    }
    let run = log_lines(&fx)
        .into_iter()
        .rfind(|l| l["stems"].is_string())
        .unwrap();
    assert_eq!(run["stems"], "tdemucs");
    assert_eq!(run["frames"], 88200, "two seconds at 44.1 kHz");
    // Two stems: everything that isn't the voice comes back as one instrumental.
    let r = finish(
        a,
        &call(a, "tool.stems", json!({"engine": "t-demucs", "mode": "2"})),
    );
    let names: Vec<&str> = r["stems"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Vocals", "Instrumental"]);
    assert!((level("Instrumental", &r) - (input + 20.0 * 0.5f64.log10())).abs() < 0.3);
    call(a, "history.undo", json!({}));
    call(a, "history.undo", json!({}));

    // The six-stem model shares the packages: only its files come down.
    let before = pips(&fx).len();
    download(a, "t-demucs-6s");
    assert_eq!(pips(&fx).len(), before, "no packages again");
    call(
        a,
        "ai.engine.set",
        json!({"role": "stems", "engine": "t-demucs-6s"}),
    );
    let r = finish(a, &call(a, "tool.stems", json!({})));
    assert_eq!(r["method"], "model:t-demucs-6s");
    assert_eq!(r["stems"].as_array().unwrap().len(), 6);
    call(a, "history.undo", json!({}));

    // Transcript: words at exact sample positions of the project, fillers and repeats found, stale after an edit.
    assert_eq!(fails(a, "transcript.get", json!({})).0, "not_found");
    download(a, "t-whisper");
    let r = finish(
        a,
        &call(
            a,
            "transcript.make",
            json!({"engine": "t-whisper", "language": "en"}),
        ),
    );
    assert_eq!(r["words"], 8);
    let heard = log_lines(&fx)
        .into_iter()
        .rfind(|l| l["transcribe"] == true)
        .unwrap();
    assert_eq!(heard["language"], "en");
    assert_eq!(heard["seconds"], 2.0);
    let t = call(a, "transcript.get", json!({}));
    assert_eq!(t["stale"], false);
    assert_eq!(t["sample_rate"], 48000);
    assert_eq!(t["engine"], "t-whisper");
    let at: Vec<i64> = t["words"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["at"].as_i64().unwrap())
        .collect();
    assert_eq!(at, (0..8).map(|i| i * 12000).collect::<Vec<i64>>());
    assert_eq!(t["words"][1]["word"], "um");
    let f = call(a, "transcript.fillers", json!({}));
    let found: Vec<(String, i64)> = f["fillers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| {
            (
                x["kind"].as_str().unwrap().into(),
                x["at"].as_i64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        found,
        [
            ("filler".to_string(), 12000),
            ("repeat".to_string(), 24000),
            ("filler".to_string(), 60000)
        ]
    );
    call(a, "track.add", json!({"name": "Notes"}));
    assert_eq!(call(a, "transcript.get", json!({}))["stale"], true);

    // Text to speech with a downloaded voice: the chosen voice, labelled synthetic, on the timeline.
    let before = pips(&fx).len();
    download(a, "t-kokoro");
    let p = pips(&fx);
    assert_eq!(
        p.len(),
        before + 1,
        "the shared package isn't installed twice: {p:?}"
    );
    assert!(p.last().unwrap().contains(&"kokoro==0.9.4".to_string()));
    call(
        a,
        "ai.engine.set",
        json!({"role": "tts", "engine": "t-kokoro", "voice": "bm_george"}),
    );
    let r = finish(a, &call(a, "tts.speak", json!({"text": "Hello there."})));
    assert_eq!(r["engine"], "t-kokoro");
    assert_eq!(r["seconds"], 0.6);
    let said = log_lines(&fx)
        .into_iter()
        .rfind(|l| l["tts"] == "kokoro")
        .unwrap();
    assert_eq!(said["voice"], "bm_george");
    assert_eq!(said["speed"], 1.0);
    // A voice the model doesn't have falls back to its first.
    finish(
        a,
        &call(a, "tts.speak", json!({"text": "Again.", "voice": "nobody"})),
    );
    let said = log_lines(&fx)
        .into_iter()
        .rfind(|l| l["tts"] == "kokoro")
        .unwrap();
    assert_eq!(said["voice"], "af_heart");
    let pj = call(a, "project.get", json!({}));
    let media = pj["project"]["media"]
        .as_array()
        .or(pj["media"].as_array())
        .unwrap();
    let m = media.iter().find(|m| m["synthetic"] == true).unwrap();
    let text =
        String::from_utf8_lossy(&std::fs::read(m["path"].as_str().unwrap()).unwrap()).to_string();
    assert!(
        text.contains("Synthetic speech made by Flux with Test Kokoro"),
        "labelled"
    );

    // The work folder is left clean: no copies of the audio stay behind.
    let work = fx.d.join("data").join("Models").join("work");
    let left: Vec<_> = std::fs::read_dir(&work)
        .map(|r| r.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    assert!(left.is_empty(), "{left:?}");
}
