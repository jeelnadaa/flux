//! The Flux speaker and microphone: made with a stand-in pactl (tests/voice_stub/pactl_stub.py keeps the sound
//! server's state and refuses what a real one would), and chosen from fake device lists for each kind of computer.
//! Real PipeWire, PulseAudio and virtual cables are in the human and hardware queue.

use flux_core::api::Api;
use flux_core::live::Source;
use flux_core::live_presets::virtual_devices;
use flux_core::store::Store;
use flux_core::vdev;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn tmp() -> PathBuf {
    let d = std::env::temp_dir().join(format!("flux-vdev-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn python() -> &'static str {
    ["python3", "python"]
        .into_iter()
        .find(|p| {
            std::process::Command::new(p)
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        })
        .expect("these tests need Python 3 on PATH")
}

fn dev(id: &str, name: &str, default: bool) -> Value {
    json!({"id": id, "name": name, "default": default, "channels": 2, "sample_rate": 48000})
}

fn devices(path: &Path, outputs: Vec<Value>, inputs: Vec<Value>) {
    std::fs::write(
        path,
        serde_json::to_vec(
            &json!({"host": "Test", "hosts": [], "outputs": outputs, "inputs": inputs}),
        )
        .unwrap(),
    )
    .unwrap();
}

fn state(p: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

fn code(e: anyhow::Error) -> String {
    format!("{e:#}")
}

#[test]
fn makes_chooses_and_restores_the_devices() {
    let d = tmp();
    let stub = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/voice_stub/pactl_stub.py");
    let (st, log, fake) = (
        d.join("pactl.json"),
        d.join("pactl.log"),
        d.join("devices.json"),
    );
    std::env::set_var("FLUX_NULL_AUDIO", "1");
    std::env::set_var("FLUX_PACTL", format!("{} {}", python(), stub.display()));
    std::env::set_var("FLUX_PACTL_STATE", &st);
    std::env::set_var("FLUX_PACTL_LOG", &log);
    std::env::set_var("FLUX_FAKE_DEVICES", &fake);
    let a = Api::new(Store::at(d.join("data")).unwrap(), true);
    let call = |m: &str, p: Value| a.call(m, p).unwrap_or_else(|e| panic!("{m}: {e:#}"));
    let v = |s: Value| s;

    // Linux, before setup: nothing yet, but Flux can make them.
    let real_out = dev("pulseaudio:alsa_output.real", "Built-in Audio", true);
    devices(
        &fake,
        vec![real_out.clone()],
        vec![dev("pulseaudio:alsa_input.real", "Mic", true)],
    );
    let s = call("live.virtual", json!({}));
    assert_eq!(
        (s["speaker"].clone(), s["can_create"].clone()),
        (Value::Null, json!(true)),
        "{s}"
    );

    // Setup makes three modules; the stub refuses badly quoted properties and an unknown master. Twice adds nothing.
    call("live.virtual.setup", json!({}));
    let m = state(&st)["modules"].as_array().unwrap().clone();
    assert_eq!(m.len(), 3, "{m:?}");
    assert_eq!(
        m[0]["args"][1],
        "sink_properties='device.description=\"Flux speaker\"'"
    );
    assert_eq!(m[2]["name"], "module-remap-source");
    assert_eq!(m[2]["args"][0], "master=flux_mic_feed.monitor");
    let r = call("live.virtual.setup", json!({}));
    assert_eq!(r["made"], json!([]));
    assert_eq!(state(&st)["modules"].as_array().unwrap().len(), 3);

    // Linux, after setup. The sound comes from the speaker's monitor and goes to the real output, and the Flux
    // speaker becomes the system output only after a successful start.
    let flux_out = dev("pulseaudio:flux_speaker", "Flux speaker", false);
    let feed = dev("pulseaudio:flux_mic_feed", "Flux microphone feed", false);
    devices(
        &fake,
        vec![real_out.clone(), flux_out.clone(), feed.clone()],
        vec![
            dev(
                "pulseaudio:alsa_output.real.monitor",
                "Monitor of Built-in Audio",
                false,
            ),
            dev(
                "pulseaudio:flux_speaker.monitor",
                "Monitor of Flux speaker",
                false,
            ),
            dev(
                "pulseaudio:flux_mic_feed.monitor",
                "Monitor of Flux microphone feed",
                false,
            ),
            dev("pulseaudio:flux_mic", "Flux microphone", false),
        ],
    );
    let (src, out, take) = virtual_devices(
        &a,
        "output",
        &json!({"source": "flux"}),
        Source::System(None),
    )
    .unwrap();
    assert_eq!(
        src,
        Source::Input(Some("pulseaudio:flux_speaker.monitor".into()))
    );
    assert_eq!(out.as_deref(), Some("pulseaudio:alsa_output.real"));
    assert!(take);
    assert_eq!(state(&st)["default"], "alsa_output.real");
    // No sound device under test here, so the start fails, and the system output is left alone.
    a.call("live.start", json!({"route": "output", "source": "flux"}))
        .unwrap_err();
    assert_eq!(
        state(&st)["default"],
        "alsa_output.real",
        "a failed start must not take the system output"
    );

    // The microphone plays into the feed; call apps record the remapped source.
    let (_, out, take) =
        virtual_devices(&a, "input", &json!({"output": "flux"}), Source::Input(None)).unwrap();
    assert_eq!(
        (out.as_deref(), take),
        (Some("pulseaudio:flux_mic_feed"), false)
    );

    // Choosing the Flux speaker as the place to listen would feed back.
    let e = virtual_devices(
        &a,
        "output",
        &json!({"source": "flux", "output": "pulseaudio:flux_speaker"}),
        Source::System(None),
    )
    .unwrap_err();
    assert!(code(e).contains("hear itself"));
    // A chosen real output is respected, without taking the system output.
    let (_, out, take) = virtual_devices(
        &a,
        "output",
        &json!({"source": "flux", "output": "pulseaudio:alsa_output.real"}),
        Source::System(None),
    )
    .unwrap();
    assert_eq!(
        (out.as_deref(), take),
        (Some("pulseaudio:alsa_output.real"), false)
    );

    // Take and put back; Stop restores it, once.
    assert_eq!(
        vdev::take_default().unwrap().as_deref(),
        Some("alsa_output.real")
    );
    assert_eq!(state(&st)["default"], "flux_speaker");
    assert_eq!(
        vdev::take_default().unwrap(),
        None,
        "already the system output: nothing to put back"
    );
    a.lock().live.restore_sink = Some("alsa_output.real".into());
    call("live.stop", json!({"route": "output"}));
    assert_eq!(state(&st)["default"], "alsa_output.real");
    assert!(a.lock().live.restore_sink.is_none());

    // Remove takes out the microphone first (the stub refuses a sink in use) and leaves other modules.
    let r = call("live.virtual.remove", json!({}));
    assert_eq!(r["removed"], 3);
    assert!(state(&st)["modules"].as_array().unwrap().is_empty());

    // Windows with a cable, set as the system output: Flux records its other side and plays on a real output.
    let cable_in = dev("ca", "CABLE Input (VB-Audio Virtual Cable)", true);
    let spk = dev("spk", "Speakers (Realtek(R) Audio)", false);
    devices(
        &fake,
        vec![cable_in, spk],
        vec![
            dev("cb", "CABLE Output (VB-Audio Virtual Cable)", false),
            dev("m", "Microphone", true),
        ],
    );
    let (src, out, take) = virtual_devices(
        &a,
        "output",
        &json!({"source": "flux"}),
        Source::System(None),
    )
    .unwrap();
    assert_eq!(src, Source::Input(Some("cb".into())));
    assert_eq!((out.as_deref(), take), (Some("spk"), false));
    let (_, out, _) =
        virtual_devices(&a, "input", &json!({"output": "flux"}), Source::Input(None)).unwrap();
    assert_eq!(out.as_deref(), Some("ca"));
    let s = call("live.devices", json!({}));
    assert_eq!(s["flux"]["shared"], true);
    assert_eq!(s["outputs"][0]["virtual"], true);
    assert_eq!(s["outputs"][1]["virtual"], false);

    // One cable can't be both at once.
    v(call(
        "live.start",
        json!({"route": "input", "source": "demo-voice", "output": "none"}),
    ));
    let e = virtual_devices(
        &a,
        "output",
        &json!({"source": "flux"}),
        Source::System(None),
    )
    .unwrap_err();
    assert!(code(e).contains("only one can run"));
    call("live.stop", json!({"route": "input"}));

    // Only the Flux devices and nowhere real to play: says so.
    devices(
        &fake,
        vec![dev("ca", "CABLE Input (VB-Audio Virtual Cable)", true)],
        vec![dev("cb", "CABLE Output (VB-Audio Virtual Cable)", false)],
    );
    let e = virtual_devices(
        &a,
        "output",
        &json!({"source": "flux"}),
        Source::System(None),
    )
    .unwrap_err();
    assert!(code(e).contains("headphones or speakers"));

    // Anything else passes through untouched.
    let (src, out, take) = virtual_devices(
        &a,
        "output",
        &json!({"output": "x"}),
        Source::System(Some("y".into())),
    )
    .unwrap();
    assert_eq!(
        (src, out.as_deref(), take),
        (Source::System(Some("y".into())), Some("x"), false)
    );
    let _ = std::fs::remove_dir_all(&d);
}
