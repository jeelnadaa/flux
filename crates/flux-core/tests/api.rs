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
