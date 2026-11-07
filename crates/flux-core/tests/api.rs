//! End-to-end tests of the JSON command API: the same surface the UI and MCP use.

use flux_core::api::Api;
use flux_core::store::Store;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
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

fn api(name: &str) -> (Api, PathBuf) {
    std::env::set_var("FLUX_NULL_AUDIO", "1");
    let d = tmp(name);
    (Api::new(Store::at(d.join("data")).unwrap(), true), d)
}

/// Stereo sines plus a little seeded noise, 16-bit WAV.
fn wav(dir: &Path, name: &str, sr: u32, secs: f32) -> PathBuf {
    let p = dir.join(name);
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: sr,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(&p, spec).unwrap();
    let n = (sr as f32 * secs) as usize;
    let mut seed = 1u32;
    for i in 0..n {
        let t = i as f32 / sr as f32;
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let noise = (seed >> 9) as f32 / (1u32 << 23) as f32 - 0.5;
        let x = 0.4 * (std::f32::consts::TAU * 440.0 * t).sin() + 0.02 * noise;
        let y = 0.4 * (std::f32::consts::TAU * 660.0 * t).sin() + 0.02 * noise;
        w.write_sample((x * 32767.0) as i16).unwrap();
        w.write_sample((y * 32767.0) as i16).unwrap();
    }
    w.finalize().unwrap();
    p
}

fn call(a: &Api, m: &str, p: Value) -> Value {
    a.call(m, p.clone())
        .unwrap_or_else(|e| panic!("{m} {p}: {e:#}"))
}

fn hash(a: &Api) -> String {
    call(a, "project.get", json!({}))["hash"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn every_method_has_valid_schema() {
    let list = flux_core::methods::METHODS;
    assert_eq!(
        flux_core::methods::list().as_array().unwrap().len(),
        list.len()
    );
    assert!(list.len() >= 100, "only {} methods", list.len());
    let mut names = std::collections::HashSet::new();
    for m in list {
        assert!(names.insert(m.name), "duplicate {}", m.name);
        assert!(!m.desc.is_empty(), "{} has no description", m.name);
        let s = flux_core::methods::schema(m);
        assert_eq!(s["type"], "object", "{}", m.name);
        assert!(s["properties"].is_object(), "{}", m.name);
        if let Some(req) = s["required"].as_array() {
            for r in req {
                assert!(
                    s["properties"].get(r.as_str().unwrap()).is_some(),
                    "{} requires unknown {r}",
                    m.name
                );
            }
        }
        assert!(flux_core::methods::find(&m.name.replace('.', "_")).is_some());
    }
}

#[test]
fn hostile_params_are_rejected_not_panics() {
    let (a, _d) = api("hostile");
    assert!(a.call("no.such.method", json!({})).is_err());
    assert!(a.call("project.new", json!([1, 2])).is_err());
    assert!(a.call("project.new", json!({"bogus": 1})).is_err());
    call(&a, "project.new", json!({"name": "x"}));
    for (m, p) in [
        ("clip.move", json!({"clip": 99999, "start": 0})),
        ("fx.add", json!({"kind": "nope"})),
        ("fx.set", json!({"fx": "a"})),
        ("media.import", json!({"path": "Z:/does/not/exist.wav"})),
        ("project.open", json!({"id": "../../etc"})),
        ("track.set", json!({"track": 99999, "gain": 1.0})),
        ("export.start", json!({"format": "exe", "wait": true})),
    ] {
        assert!(a.call(m, p.clone()).is_err(), "{m} {p} should fail");
    }
    // the session is still usable
    call(&a, "track.add", json!({}));
}

#[test]
fn import_edit_undo_redo_export_roundtrip() {
    let (a, d) = api("roundtrip");
    let f = wav(&d, "tone.wav", 48000, 2.0);
    let r = call(
        &a,
        "media.import",
        json!({"path": f.display().to_string(), "wait": true}),
    );
    assert!(r.get("job").is_some(), "{r}");
    let p = call(&a, "project.get", json!({}));
    assert_eq!(p["length"], 96000, "{}", p["length"]);
    let clip = p["tracks"][0]["clips"][0]["id"].as_u64().unwrap();

    let h0 = hash(&a);
    call(&a, "clip.split", json!({"at": 48000}));
    let p = call(&a, "project.get", json!({}));
    let clips = p["tracks"][0]["clips"].as_array().unwrap();
    assert_eq!(clips.len(), 2);
    assert_eq!(clips[1]["start"], 48000);
    call(&a, "clip.move", json!({"clip": clip, "start_s": 0.5}));
    let p = call(&a, "project.get", json!({}));
    let moved = p["tracks"][0]["clips"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == clip)
        .unwrap()
        .clone();
    assert_eq!(moved["start"], 24000);
    let fx = call(&a, "fx.add", json!({"kind": "eq", "target": "master"}));
    let fxid = fx["id"]
        .as_u64()
        .or(fx["fx"].as_u64())
        .unwrap_or_else(|| panic!("{fx}"));
    call(
        &a,
        "fx.set",
        json!({"fx": fxid, "params": {"b2_gain": -3.0}}),
    );
    let h1 = hash(&a);
    assert_ne!(h0, h1);
    for _ in 0..4 {
        call(&a, "history.undo", json!({}));
    }
    assert_eq!(hash(&a), h0, "undo all gives the original");
    for _ in 0..4 {
        call(&a, "history.redo", json!({}));
    }
    assert_eq!(hash(&a), h1, "redo all gives an identical hash");

    for fmt in ["wav", "flac", "mp3", "ogg"] {
        let out = d.join(format!("out.{fmt}"));
        let r = call(
            &a,
            "export.start",
            json!({"format": fmt, "path": out.display().to_string(), "wait": true, "tail": false}),
        );
        assert!(out.is_file(), "{fmt}: {r}");
        let res = &r["result"];
        let secs = res["seconds"]
            .as_f64()
            .or(res["duration"].as_f64())
            .unwrap_or_else(|| panic!("{fmt}: {res}"));
        assert!((secs - 2.0).abs() < 0.1, "{fmt} {secs}");
    }
    // the source file is never modified
    let h = hound::WavReader::open(&f).unwrap();
    assert_eq!(h.duration(), 96000);
}

#[test]
fn analysis_and_tools_produce_measured_results() {
    let (a, d) = api("tools");
    let f = wav(&d, "tone.wav", 44100, 3.0);
    call(
        &a,
        "media.import",
        json!({"path": f.display().to_string(), "wait": true}),
    );
    let l = call(&a, "analysis.loudness", json!({}));
    let lufs = l["loudness"]["integrated"]
        .as_f64()
        .unwrap_or_else(|| panic!("{l}"));
    assert!(lufs < -5.0 && lufs > -20.0, "{lufs}");
    let m = call(&a, "tool.master.apply", json!({"target": -16.0}));
    let after = m["after"]["integrated"]
        .as_f64()
        .or(m["integrated"].as_f64())
        .unwrap_or_else(|| panic!("{m}"));
    assert!((after + 16.0).abs() < 1.5, "mastered to {after}");
    let s = call(&a, "tool.stems", json!({"mode": "2"}));
    let j = a
        .jobs
        .wait(
            s["job"].as_str().unwrap_or_else(|| panic!("{s}")),
            Duration::from_secs(120),
        )
        .unwrap();
    assert_eq!(j.state, "done", "{:?}", j.error);
    let p = call(&a, "project.get", json!({}));
    assert!(
        p["tracks"].as_array().unwrap().len() >= 3,
        "{}",
        p["tracks"]
    );
}

#[test]
fn actions_record_and_replay() {
    let (a, _d) = api("actions");
    call(&a, "project.new", json!({"name": "a"}));
    call(&a, "action.record", json!({"on": true}));
    call(&a, "track.add", json!({}));
    call(&a, "track.add", json!({}));
    let r = call(
        &a,
        "action.record",
        json!({"on": false, "name": "Two tracks"}),
    );
    assert!(r.to_string().contains("Two"), "{r}");
    let n0 = call(&a, "project.get", json!({}))["tracks"]
        .as_array()
        .unwrap()
        .len();
    let list = call(&a, "action.list", json!({}));
    let arr = list
        .as_array()
        .or(list["actions"].as_array())
        .unwrap_or_else(|| panic!("{list}"))
        .clone();
    let id = arr
        .iter()
        .find(|x| x["name"] == "Two tracks")
        .unwrap_or_else(|| panic!("{list}"))["id"]
        .as_str()
        .unwrap()
        .to_string();
    call(&a, "action.run", json!({"id": id}));
    assert_eq!(
        call(&a, "project.get", json!({}))["tracks"]
            .as_array()
            .unwrap()
            .len(),
        n0 + 2
    );
}

#[test]
fn a_gesture_is_one_undo_step() {
    let (a, d) = api("gesture");
    let f = wav(&d, "g.wav", 48000, 1.0);
    call(&a, "media.import", json!({"path": f, "wait": true}));
    let fx = call(&a, "fx.add", json!({"kind": "gain", "target": "master"}))["fx"]
        .as_u64()
        .unwrap();
    let start = hash(&a);
    let steps = |a: &Api| {
        call(a, "history.list", json!({}))["items"]
            .as_array()
            .unwrap()
            .len()
    };
    let n0 = steps(&a);
    // Ten moves in one drag land as one step, and undo returns to the state before the drag.
    for i in 1..=10 {
        call(
            &a,
            "fx.set",
            json!({"fx": fx, "params": {"gain": -(i as f64)}, "gesture": "drag-1"}),
        );
    }
    assert_eq!(steps(&a), n0 + 1);
    let after = hash(&a);
    call(&a, "history.undo", json!({}));
    assert_eq!(hash(&a), start);
    call(&a, "history.redo", json!({}));
    assert_eq!(hash(&a), after);
    // A new gesture, or no gesture, starts a new step.
    call(
        &a,
        "fx.set",
        json!({"fx": fx, "params": {"gain": -3.0}, "gesture": "drag-2"}),
    );
    call(&a, "fx.set", json!({"fx": fx, "params": {"gain": -4.0}}));
    call(&a, "fx.set", json!({"fx": fx, "params": {"gain": -5.0}}));
    assert_eq!(steps(&a), n0 + 4);
    // A bad gesture is rejected, not ignored.
    let r = a.call(
        "fx.set",
        json!({"fx": fx, "params": {"gain": -1.0}, "gesture": 5}),
    );
    assert!(r.is_err());
}

fn code(a: &Api, m: &str, p: Value) -> &'static str {
    flux_core::api::error_code(&a.call(m, p).expect_err(m))
}

#[test]
fn live_presets_build_edit_route_render_export_and_import() {
    let (a, d) = api("live");
    // catalogue: kinds with tooltips, shapes, many built-ins of each kind
    let k = call(&a, "live.kinds", json!({}));
    let kinds = k["kinds"].as_array().unwrap();
    assert!(kinds.len() >= 35);
    assert!(kinds.iter().all(|x| x["tip"].is_string()
        && x["params"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["tip"].is_string())));
    assert!(kinds.iter().all(|x| x["kind"] != "speed"));
    assert_eq!(k["shapes"].as_array().unwrap().len(), 10);
    let outs = call(&a, "live.presets", json!({"kind": "output"}));
    let ins = call(&a, "live.presets", json!({"kind": "input"}));
    assert!(outs.as_array().unwrap().len() >= 15 && ins.as_array().unwrap().len() >= 15);

    // built-ins are read-only; copying one makes an editable preset
    assert_eq!(
        code(
            &a,
            "live.module.add",
            json!({"preset": "out-8d-orbit", "kind": "gain"})
        ),
        "read_only"
    );
    let mine = call(&a, "live.preset.create", json!({"from": "out-8d-orbit"}));
    let id = mine["id"].as_str().unwrap().to_string();
    assert!(id.starts_with("user-") && mine["builtin"] == false);
    assert_eq!(mine["post"].as_array().unwrap().len(), 2);

    // build a rig: a parallel lane with an octave, an LFO on reverb, a macro on spatial rate
    let r = call(&a, "live.lane.add", json!({"preset": id, "name": "Octave"}));
    let lane = r["result"]["lane"].as_str().unwrap().to_string();
    let r = call(
        &a,
        "live.module.add",
        json!({"preset": id, "kind": "pitch", "lane": lane, "params": {"semitones": -12}}),
    );
    let pitch = r["result"]["module"].as_str().unwrap().to_string();
    let r = call(
        &a,
        "live.mod.add",
        json!({"preset": id, "shape": "steps", "sync": "1/8", "steps": [0, 1, 0.5, 1]}),
    );
    let lfo = r["result"]["mod"].as_str().unwrap().to_string();
    call(
        &a,
        "live.mod.target",
        json!({"preset": id, "mod": lfo, "module": "rv", "param": "mix", "amount": 0.4}),
    );
    call(
        &a,
        "live.macro.target",
        json!({"preset": id, "macro": "speed", "module": pitch, "param": "semitones", "from": -12, "to": 0}),
    );
    let p = call(
        &a,
        "live.macro.set",
        json!({"preset": id, "macro": "speed", "value": 0.9}),
    )["preset"]
        .clone();
    assert_eq!(p["lanes"][0]["modules"][0]["kind"], "pitch");
    assert_eq!(p["mods"][0]["targets"][0]["param"], "mix");
    // bad edits are refused with clear codes, and leave the preset as it was
    assert_eq!(
        code(
            &a,
            "live.module.add",
            json!({"preset": id, "kind": "speed"})
        ),
        "bad_params"
    );
    assert_eq!(
        code(
            &a,
            "live.module.set",
            json!({"preset": id, "module": pitch, "params": {"loudness": 3}})
        ),
        "bad_params"
    );
    assert_eq!(
        code(
            &a,
            "live.mod.target",
            json!({"preset": id, "mod": lfo, "module": "ghost", "param": "gain"})
        ),
        "bad_params"
    );
    assert_eq!(
        code(
            &a,
            "live.mod.set",
            json!({"preset": id, "mod": lfo, "sync": "1/7"})
        ),
        "bad_params"
    );
    let again = call(&a, "live.preset.get", json!({"id": id}));
    assert_eq!(again, p);

    // removing a module also removes what pointed at it
    call(
        &a,
        "live.module.remove",
        json!({"preset": id, "module": "rv"}),
    );
    let p = call(&a, "live.preset.get", json!({"id": id}));
    assert!(p["mods"][0]["targets"].as_array().unwrap().is_empty());

    // a route on the test source plays the preset; edits reach it live
    let st = call(
        &a,
        "live.start",
        json!({"route": "output", "preset": id, "source": "demo-music", "output": "none"}),
    );
    assert_eq!(st["running"], true);
    // Levels are measured over a window: wait (up to 3 s on a busy machine) for it to fill.
    let out = |s: &Value| s["routes"]["output"]["out_rms_db"].as_f64().unwrap();
    let mut s1 = Value::Null;
    for _ in 0..60 {
        std::thread::sleep(Duration::from_millis(50));
        s1 = call(&a, "live.status", json!({}));
        if s1["routes"]["output"]["frames"].as_u64().unwrap() > 9600 && out(&s1) > -40.0 {
            break;
        }
    }
    assert!(s1["routes"]["output"]["frames"].as_u64().unwrap() > 9000);
    assert!(s1["routes"]["output"]["out_rms_db"].as_f64().unwrap() > -40.0);
    assert_eq!(code(&a, "live.preset.delete", json!({"id": id})), "busy");
    call(
        &a,
        "live.preset.update",
        json!({"preset": id, "output_gain_db": -20}),
    );
    let mut drop_db = 0.0;
    for _ in 0..60 {
        std::thread::sleep(Duration::from_millis(50));
        drop_db = out(&s1) - out(&call(&a, "live.status", json!({})));
        if drop_db > 12.0 {
            break;
        }
    }
    assert!(drop_db > 12.0, "output gain edit dropped only {drop_db} dB");
    call(&a, "live.set", json!({"route": "output", "bypass": true}));
    call(&a, "live.stop", json!({"route": "output"}));
    assert!(call(&a, "live.status", json!({}))["routes"]["output"].is_null());
    // devices are off in tests: a real source is refused, not faked
    assert_eq!(
        code(
            &a,
            "live.start",
            json!({"route": "input", "preset": "in-robot"})
        ),
        "no_device"
    );
    assert_eq!(
        code(&a, "live.start", json!({"route": "sideways"})),
        "bad_params"
    );

    // offline render through the test voice
    let out = d.join("robot.wav");
    let r = call(
        &a,
        "live.render",
        json!({"preset": "in-robot", "seconds": 2, "path": out.display().to_string()}),
    );
    assert_eq!(r["changed"], true);
    assert!(
        r["out_peak_db"].as_f64().unwrap() <= -0.9,
        "safety limiter: {}",
        r["out_peak_db"]
    );
    assert_eq!(hound::WavReader::open(&out).unwrap().duration(), 96000);

    // export, import as a new preset, identical content
    let file = d.join("mine.fluxlive");
    call(
        &a,
        "live.preset.export",
        json!({"id": id, "path": file.display().to_string()}),
    );
    let imp = call(
        &a,
        "live.preset.import",
        json!({"path": file.display().to_string()}),
    );
    assert_ne!(imp["id"], p["id"]);
    let p = call(&a, "live.preset.get", json!({"id": id}));
    let (mut x, mut y) = (imp.clone(), p.clone());
    x["id"] = json!(null);
    y["id"] = json!(null);
    assert_eq!(x, y);
    // hostile imports are refused
    assert_eq!(
        code(&a, "live.preset.import", json!({"json": "{not json"})),
        "bad_file"
    );
    assert_eq!(
        code(
            &a,
            "live.preset.import",
            json!({"json": "{\"post\":[{\"kind\":\"rm -rf\"}]}"})
        ),
        "bad_file"
    );
    assert_eq!(
        code(
            &a,
            "live.preset.import",
            json!({"json": "x".repeat(600 * 1024)})
        ),
        "bad_file"
    );

    // randomize stays in range and is seeded
    let r1 = call(
        &a,
        "live.randomize",
        json!({"preset": imp["id"], "amount": 1, "seed": 4}),
    )["preset"]
        .clone();
    for m in r1["lanes"][0]["modules"].as_array().unwrap() {
        let st = m["params"]["semitones"].as_f64().unwrap();
        assert!((-12.0..=12.0).contains(&st));
    }
    call(&a, "live.preset.delete", json!({"id": imp["id"]}));
    assert_eq!(
        code(&a, "live.preset.get", json!({"id": imp["id"]})),
        "not_found"
    );
}

#[test]
fn voice_cloning_needs_consent_and_reports_the_model_for_this_computer() {
    let (a, d) = api("clone");
    let st = call(&a, "clone.status", json!({}));
    assert!(st["hardware"]["cores"].as_u64().unwrap() >= 1);
    assert!(st["models"]
        .as_array()
        .unwrap()
        .iter()
        .all(|m| m["licence"] == "MIT"));
    assert!(st["pick"]["reason"].as_str().unwrap().len() > 20);
    assert_eq!(st["available"], false);
    let r = wav(&d, "ref.wav", 48000, 8.0);
    let c = wav(&d, "consent.wav", 44100, 4.0);
    let short = wav(&d, "short.wav", 48000, 1.0);
    let (rp, cp, sp) = (
        r.display().to_string(),
        c.display().to_string(),
        short.display().to_string(),
    );
    // No consent, no profile.
    assert_eq!(
        code(
            &a,
            "clone.profile.create",
            json!({"name": "Sam", "owner": "me", "consent": false, "consent_audio": cp, "reference": rp})
        ),
        "consent_required"
    );
    assert_eq!(
        code(
            &a,
            "clone.profile.create",
            json!({"name": "Sam", "owner": "me", "consent": true, "reference": rp})
        ),
        "consent_required"
    );
    assert_eq!(
        code(
            &a,
            "clone.profile.create",
            json!({"name": "Sam", "owner": "nobody", "consent": true, "consent_audio": cp, "reference": rp})
        ),
        "consent_required"
    );
    // Hostile or unsuitable audio.
    assert_eq!(
        code(
            &a,
            "clone.profile.create",
            json!({"name": "Sam", "owner": "me", "consent": true, "consent_audio": cp, "reference": sp})
        ),
        "bad_params"
    );
    let junk = d.join("junk.wav");
    std::fs::write(&junk, b"RIFF nonsense").unwrap();
    assert_eq!(
        code(
            &a,
            "clone.profile.create",
            json!({"name": "Sam", "owner": "me", "consent": true, "consent_audio": cp, "reference": junk.display().to_string()})
        ),
        "bad_file"
    );
    // With consent.
    let p = call(
        &a,
        "clone.profile.create",
        json!({"name": "Sam", "owner": "me", "consent": true, "consent_audio": cp, "reference": rp}),
    );
    assert_eq!(p["id"], "sam");
    assert!((p["reference_seconds"].as_f64().unwrap() - 8.0).abs() < 0.1);
    assert!(p["statement"].as_str().unwrap().contains("I am Sam"));
    assert_eq!(
        call(&a, "clone.profiles", json!({}))
            .as_array()
            .unwrap()
            .len(),
        1
    );
    // Speaking says why it can't yet, and never for a voice without a profile.
    assert_eq!(
        code(&a, "clone.speak", json!({"voice": "nobody", "text": "hi"})),
        "consent_required"
    );
    assert_eq!(
        code(&a, "clone.speak", json!({"voice": "sam", "text": "hi"})),
        "not_installed"
    );
    assert_eq!(
        code(&a, "clone.profile.delete", json!({"id": "../x"})),
        "bad_params"
    );
    let sc = call(
        &a,
        "clone.script",
        json!({"text": "Well [laugh] hmm, (sighs) okay"}),
    );
    assert_eq!(sc["model_text"], "Well [laugh] hmm, [sigh] okay");
    assert_eq!(sc["sounds"], 2);
    assert_eq!(sc["hesitations"], 1);
    let sc = call(
        &a,
        "clone.script",
        json!({"text": "[hmm] So [laugh] [pause 1.5s] (um) yes"}),
    );
    assert_eq!(
        sc["segments"],
        json!([
            {"type": "say", "value": "Hmm, So [laugh]"},
            {"type": "silence", "value": 1500},
            {"type": "say", "value": "Um, yes"}
        ])
    );
    assert_eq!(
        (sc["hesitations"].as_u64(), sc["silence_ms"].as_u64()),
        (Some(2), Some(1500))
    );
    let t = call(&a, "clone.tags", json!({}));
    assert!(t["hesitations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|h| h["tag"] == "hmm"));
    assert!(t["pauses"].as_array().unwrap().len() >= 3);
    assert_eq!(
        code(
            &a,
            "clone.speak",
            json!({"voice": "sam", "text": "hi [yodel]"})
        ),
        "bad_params"
    );
    assert!(
        call(&a, "clone.tags", json!({}))["tags"]
            .as_array()
            .unwrap()
            .len()
            >= 9
    );
    assert_eq!(
        code(&a, "clone.convert", json!({"voice": "sam", "path": rp})),
        "not_installed"
    );
    assert_eq!(
        code(&a, "clone.convert", json!({"voice": "nobody", "path": rp})),
        "consent_required"
    );
    call(&a, "clone.profile.delete", json!({"id": "sam"}));
    assert!(call(&a, "clone.profiles", json!({}))
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn first_run_sound_checks_work_without_a_project_and_3d_uses_the_head_profile() {
    let (a, _d) = api("hpcheck");
    // The headphone check runs before any project exists.
    for k in ["left", "right", "moving"] {
        let r = call(&a, "audio.test_tone", json!({"kind": k}));
        assert_eq!(r["playing"], k);
    }
    call(
        &a,
        "settings.set",
        json!({"values": {"hrtf_profile": "C", "head_width_cm": 20.5}}),
    );
    call(&a, "project.new", json!({"name": "3d"}));
    let fx = call(&a, "fx.add", json!({"kind": "spatial", "target": "master"}))["fx"]
        .as_u64()
        .unwrap();
    let p = call(&a, "project.get", json!({}));
    let e = p["master"]["fx"]
        .as_array()
        .or(p["master_fx"].as_array())
        .unwrap()
        .iter()
        .find(|e| e["id"].as_u64() == Some(fx))
        .unwrap()
        .clone();
    assert_eq!(e["params"]["head_width"], 20.5);
    assert_eq!(e["params"]["depth"], 100.0);
    // A width given by the caller wins.
    let fx2 = call(
        &a,
        "fx.add",
        json!({"kind": "spatial", "target": "master", "params": {"head_width": 14}}),
    )["fx"]
        .as_u64()
        .unwrap();
    let p = call(&a, "project.get", json!({}));
    let all = p["master"]["fx"]
        .as_array()
        .or(p["master_fx"].as_array())
        .unwrap();
    let e2 = all.iter().find(|e| e["id"].as_u64() == Some(fx2)).unwrap();
    assert_eq!(e2["params"]["head_width"], 14.0);
}
