//! The Flux speaker and the Flux microphone: audio devices other apps can pick.
//!
//! The Flux speaker is where apps play the sound Live should process. Flux records it, runs it through a preset
//! and plays the result on a real output, so only the processed sound is heard. The Flux microphone is where Live
//! plays a processed microphone; call apps record it like any microphone.
//!
//! - Linux: Flux makes both itself with PipeWire or PulseAudio (`pactl`, no root): a null sink "Flux speaker",
//!   and a null sink "Flux microphone feed" whose monitor is remapped as the source "Flux microphone". While the
//!   computer-sound route runs, Flux makes the Flux speaker the system output and puts the old one back after.
//! - Windows and macOS: Flux uses an existing virtual
//!   audio cable the person installed: apps play into one side, and the other side records it.
//!
//! Detection works on device names only, so it is tested with plain lists on every OS.

use crate::api::{err, Api};
use anyhow::{bail, Result};
use serde::Serialize;
use serde_json::{json, Value};

pub const SPEAKER: &str = "Flux speaker";
pub const MIC: &str = "Flux microphone";
const SINK: &str = "flux_speaker";
const MIC_SINK: &str = "flux_mic_feed";
const MIC_SRC: &str = "flux_mic";

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Dev {
    pub id: String,
    pub name: String,
}

/// One virtual device: `play` is the output side, `rec` the input side that records what is played into it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Pair {
    pub play: Dev,
    pub rec: Dev,
    /// `flux` (made by Flux) or `cable` (a virtual cable someone installed)
    pub kind: &'static str,
    /// What to pick in other apps.
    pub label: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Found {
    pub speaker: Option<Pair>,
    pub mic: Option<Pair>,
    /// One cable serves as both, so only one of the two can be used at a time.
    pub shared: bool,
}

fn lc(s: &str) -> String {
    s.to_lowercase()
}

/// Pairs of a virtual cable's two sides, in the order the outputs are listed.
fn cables(outputs: &[Dev], inputs: &[Dev]) -> Vec<Pair> {
    let mut out = vec![];
    for o in outputs {
        let n = lc(&o.name);
        if n.contains("flux") {
            continue;
        }
        // "CABLE Input (VB-Audio Virtual Cable)" plays into "CABLE Output (VB-Audio Virtual Cable)", and the
        // same for CABLE-A, CABLE-B and so on.
        let rec = if n.starts_with("cable") && n.contains(" input") {
            let want = n.replacen(" input", " output", 1);
            inputs.iter().find(|i| lc(&i.name) == want)
        } else if n.contains("blackhole")
            || n.contains("loopback")
            || n.contains("virtual")
            || n.contains("soundflower")
            || n.contains("cable")
        {
            // Devices that are the same on both sides (BlackHole 2ch, Loopback Audio, ...).
            inputs.iter().find(|i| i.name == o.name)
        } else {
            None
        };
        if let Some(r) = rec {
            let label = o
                .name
                .split(" (")
                .next()
                .unwrap_or(&o.name)
                .replace(" Input", "")
                .trim()
                .to_string();
            out.push(Pair {
                play: o.clone(),
                rec: r.clone(),
                kind: "cable",
                label,
            });
        }
    }
    out
}

/// Find the Flux speaker and microphone among the devices: Flux's own first, then installed cables.
pub fn detect(outputs: &[Dev], inputs: &[Dev]) -> Found {
    let own = |play_has: &str, rec_ok: &dyn Fn(&str) -> bool, label: &str| -> Option<Pair> {
        let play = outputs.iter().find(|o| lc(&o.name).contains(play_has))?;
        let rec = inputs.iter().find(|i| rec_ok(&lc(&i.name)))?;
        Some(Pair {
            play: play.clone(),
            rec: rec.clone(),
            kind: "flux",
            label: label.into(),
        })
    };
    // Linux: "Flux speaker" and its monitor, "Monitor of Flux speaker".
    let speaker = own("flux speaker", &|n| n.contains("flux speaker"), SPEAKER);
    // Linux: "Flux microphone feed" plays into the source "Flux microphone".
    let mic = own(
        "flux microphone",
        &|n| n.contains("flux microphone") && !n.contains("monitor of"),
        MIC,
    );
    let mut c = cables(outputs, inputs).into_iter();
    let speaker = speaker.or_else(|| c.next());
    let (mic, shared) = match mic {
        Some(m) => (Some(m), false),
        None => match c.next() {
            Some(m) => (Some(m), false),
            None => match &speaker {
                Some(s) if s.kind == "cable" => (Some(s.clone()), true),
                _ => (None, false),
            },
        },
    };
    Found {
        speaker,
        mic,
        shared,
    }
}

/// True when a device is one of the virtual devices (so it isn't a place to listen).
pub fn is_virtual(f: &Found, id: &str) -> bool {
    [&f.speaker, &f.mic]
        .iter()
        .filter_map(|p| p.as_ref())
        .any(|p| p.play.id == id || p.rec.id == id)
}

// ---------------------------------------------------------------- Linux: pactl

/// The `pactl` command: `FLUX_PACTL` (a command line, for tests) or `pactl`.
fn pactl(args: &[&str]) -> Result<String> {
    let cmd = std::env::var("FLUX_PACTL").unwrap_or_else(|_| "pactl".into());
    let mut parts = cmd.split_whitespace();
    let prog = parts.next().unwrap_or("pactl");
    let out = std::process::Command::new(prog)
        .args(parts)
        .args(args)
        .output()
        .map_err(|_| {
            err(
                "unavailable",
                "Flux needs PipeWire or PulseAudio (the pactl command) to make the Flux speaker and microphone. Install pipewire-pulse or pulseaudio-utils.",
            )
        })?;
    if !out.status.success() {
        bail!(err(
            "device",
            format!(
                "The sound server refused: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// True where Flux makes the devices itself.
/// Checked once per run: Live's status asks often.
pub fn can_create() -> bool {
    static C: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *C.get_or_init(|| {
        (cfg!(target_os = "linux") || std::env::var_os("FLUX_PACTL").is_some())
            && pactl(&["info"]).is_ok()
    })
}

fn names(kind: &str) -> Vec<String> {
    pactl(&["list", "short", kind])
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split('\t').nth(1).map(str::to_string))
        .collect()
}

/// Make the Flux speaker and microphone (only the ones missing). Returns what was made.
pub fn create() -> Result<Vec<&'static str>> {
    let sinks = names("sinks");
    let sources = names("sources");
    let mut made = vec![];
    if !sinks.iter().any(|s| s == SINK) {
        pactl(&[
            "load-module",
            "module-null-sink",
            &format!("sink_name={SINK}"),
            &format!("sink_properties='device.description=\"{SPEAKER}\"'"),
        ])?;
        made.push(SPEAKER);
    }
    if !sinks.iter().any(|s| s == MIC_SINK) {
        pactl(&[
            "load-module",
            "module-null-sink",
            &format!("sink_name={MIC_SINK}"),
            &format!("sink_properties='device.description=\"{MIC} feed\"'"),
        ])?;
    }
    if !sources.iter().any(|s| s == MIC_SRC) {
        pactl(&[
            "load-module",
            "module-remap-source",
            &format!("master={MIC_SINK}.monitor"),
            &format!("source_name={MIC_SRC}"),
            &format!("source_properties='device.description=\"{MIC}\"'"),
        ])?;
        made.push(MIC);
    }
    Ok(made)
}

/// Remove the devices Flux made. Apps using them move to the system's own devices.
pub fn remove() -> Result<usize> {
    let mods = pactl(&["list", "short", "modules"])?;
    // the remapped microphone first, then the sinks it hangs off
    let mut ids: Vec<(u8, String)> = mods
        .lines()
        .filter_map(|l| {
            let mut f = l.split('\t');
            let id = f.next()?.trim().to_string();
            let _name = f.next()?;
            let args = f.next().unwrap_or("");
            let order = if args.contains(&format!("source_name={MIC_SRC}")) {
                0
            } else if args.contains(&format!("sink_name={SINK}"))
                || args.contains(&format!("sink_name={MIC_SINK}"))
            {
                1
            } else {
                return None;
            };
            Some((order, id))
        })
        .collect();
    ids.sort();
    for (_, id) in &ids {
        pactl(&["unload-module", id])?;
    }
    Ok(ids.len())
}

/// The system output now (`pactl info`, which every version has).
pub fn default_sink() -> Option<String> {
    pactl(&["info"]).ok()?.lines().find_map(|l| {
        l.strip_prefix("Default Sink:")
            .map(|v| v.trim().to_string())
    })
}

pub fn set_default_sink(name: &str) -> Result<()> {
    pactl(&["set-default-sink", name]).map(|_| ())
}

/// Make the Flux speaker the system output while the computer-sound route runs. Returns the output to put back.
pub fn take_default() -> Result<Option<String>> {
    let old = default_sink();
    if old.as_deref() == Some(SINK) {
        return Ok(None);
    }
    set_default_sink(SINK)?;
    Ok(old)
}

/// The sink name in a Linux device id ("pulseaudio:alsa_output..." -> "alsa_output...").
pub fn sink_of(id: &str) -> &str {
    id.rsplit_once(':').map(|x| x.1).unwrap_or(id)
}

// ---------------------------------------------------------------- devices and the API

/// The audio host Live uses: PulseAudio on Linux when a server is running (it lists every output, monitor and
/// Flux's own devices by name), else the platform default.
#[cfg(feature = "device")]
pub fn host() -> cpal::Host {
    #[cfg(target_os = "linux")]
    {
        if let Ok(h) = cpal::host_from_id(cpal::HostId::PulseAudio) {
            return h;
        }
    }
    cpal::default_host()
}

/// The devices Live can use. `FLUX_FAKE_DEVICES` (a JSON file, read every time) stands in for the hardware in tests.
pub fn devices() -> crate::device::DeviceList {
    if let Some(l) = std::env::var_os("FLUX_FAKE_DEVICES")
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
    {
        return l;
    }
    #[cfg(feature = "device")]
    {
        crate::device::list_devices_on(&host())
    }
    #[cfg(not(feature = "device"))]
    crate::device::DeviceList::default()
}

fn devs(d: &crate::device::DeviceList) -> (Vec<Dev>, Vec<Dev>) {
    let m = |v: &Vec<crate::device::DeviceInfo>| {
        v.iter()
            .map(|x| Dev {
                id: x.id.clone(),
                name: x.name.clone(),
            })
            .collect::<Vec<_>>()
    };
    (m(&d.outputs), m(&d.inputs))
}

pub fn found() -> (Found, crate::device::DeviceList) {
    let d = devices();
    let (o, i) = devs(&d);
    (detect(&o, &i), d)
}

fn platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

/// `ready`, `can_add` (Flux can make them here) or `missing` (something has to be installed first).
fn state(f: &Found, create: bool) -> &'static str {
    if f.speaker.is_some() || f.mic.is_some() {
        "ready"
    } else if create {
        "can_add"
    } else {
        "missing"
    }
}

/// Where to get what's needed on this OS, for the guide and for agents.
fn install(platform: &str) -> Value {
    match platform {
        "windows" => json!({"name": "VB-CABLE", "url": "https://vb-audio.com/Cable/"}),
        "macos" => {
            json!({"name": "BlackHole", "url": "https://existential.audio/blackhole/", "command": "brew install blackhole-2ch"})
        }
        _ => json!({"name": "pactl", "command": "sudo apt install pulseaudio-utils"}),
    }
}

/// What the person sees about the two devices.
fn note(f: &Found, create: bool, platform: &str) -> String {
    if f.speaker.as_ref().is_some_and(|p| p.kind == "flux") {
        return format!("Pick {SPEAKER} as the output in any app (or as the system output) and {MIC} as the microphone in call apps.");
    }
    if let Some(s) = &f.speaker {
        return if f.shared {
            format!("Using the virtual cable {} as the Flux speaker and microphone, one at a time. Add a second cable to use both together.", s.label)
        } else {
            format!(
                "Using the virtual cable {} as the Flux speaker{}.",
                s.label,
                f.mic
                    .as_ref()
                    .map(|m| format!(" and {} as the Flux microphone", m.label))
                    .unwrap_or_default()
            )
        };
    }
    if create {
        return format!("Flux can add {SPEAKER} and {MIC} to this computer. Apps can then play into Flux, and call apps can use your processed voice.");
    }
    if platform == "linux" {
        return "Flux couldn't reach PipeWire or PulseAudio with the pactl command, so it can't add the Flux speaker and microphone. Install pactl, then check again.".into();
    }
    "No virtual audio cable is installed on this computer, so other apps can't pick Flux yet, and the original sound keeps playing alongside the processed sound.".into()
}

fn pair_json(p: &Option<Pair>) -> Value {
    match p {
        Some(p) => json!({"label": p.label, "kind": p.kind, "play": p.play, "rec": p.rec}),
        None => Value::Null,
    }
}

/// What Live and `live.virtual` report about the two devices.
pub fn summary(f: &Found) -> Value {
    let create = can_create();
    json!({
        "platform": platform(),
        "speaker": pair_json(&f.speaker),
        "mic": pair_json(&f.mic),
        "shared": f.shared,
        "can_create": create,
        "status": state(f, create),
        "install": install(platform()),
        "note": note(f, create, platform()),
    })
}

/// `live.virtual`: the Flux speaker and microphone on this computer, and whether Flux can make them.
pub fn h_status(_: &Api, _: &Value) -> Result<Value> {
    Ok(summary(&found().0))
}

/// `live.virtual.setup`: make the Flux speaker and microphone (Linux).
pub fn h_setup(api: &Api, v: &Value) -> Result<Value> {
    if !can_create() {
        bail!(err(
            "unavailable",
            "On this computer the Flux speaker and microphone use a virtual audio cable. Install one and Flux finds it (live.virtual says where to get one).",
        ));
    }
    let made = create()?;
    let mut r = h_status(api, v)?;
    r["made"] = json!(made);
    Ok(r)
}

/// `live.virtual.remove`: remove the devices Flux made (Linux).
pub fn h_remove(api: &Api, v: &Value) -> Result<Value> {
    if !can_create() {
        return h_status(api, v);
    }
    restore(api);
    let n = remove()?;
    let mut r = h_status(api, v)?;
    r["removed"] = json!(n);
    Ok(r)
}

/// Put back the system output Flux replaced, if any.
pub fn restore(api: &Api) {
    let old = api.lock().live.restore_sink.take();
    if let Some(o) = old {
        let _ = set_default_sink(&o);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(id: &str, name: &str) -> Dev {
        Dev {
            id: id.into(),
            name: name.into(),
        }
    }

    #[test]
    fn finds_flux_devices_first_then_cables() {
        // Linux after setup: Flux's own devices win over a cable.
        let outs = [
            d("o1", "Built-in Audio Analog Stereo"),
            d("o2", "Flux speaker"),
            d("o3", "Flux microphone feed"),
            d("o4", "CABLE Input (VB-Audio Virtual Cable)"),
        ];
        let ins = [
            d("i1", "Monitor of Built-in Audio Analog Stereo"),
            d("i2", "Monitor of Flux speaker"),
            d("i3", "Monitor of Flux microphone feed"),
            d("i4", "Flux microphone"),
            d("i5", "CABLE Output (VB-Audio Virtual Cable)"),
        ];
        let f = detect(&outs, &ins);
        let s = f.speaker.unwrap();
        assert_eq!(
            (s.play.id.as_str(), s.rec.id.as_str(), s.kind),
            ("o2", "i2", "flux")
        );
        let m = f.mic.unwrap();
        assert_eq!((m.play.id.as_str(), m.rec.id.as_str()), ("o3", "i4"));
        assert!(!f.shared);
    }

    #[test]
    fn cables_pair_by_name_and_one_cable_is_shared() {
        let outs = [
            d("spk", "Speakers (Realtek(R) Audio)"),
            d("ca", "CABLE Input (VB-Audio Virtual Cable)"),
        ];
        let ins = [
            d("mic", "Microphone Array (Realtek(R) Audio)"),
            d("cb", "CABLE Output (VB-Audio Virtual Cable)"),
        ];
        let f = detect(&outs, &ins);
        let s = f.speaker.clone().unwrap();
        assert_eq!(
            (
                s.play.id.as_str(),
                s.rec.id.as_str(),
                s.kind,
                s.label.as_str()
            ),
            ("ca", "cb", "cable", "CABLE")
        );
        assert_eq!(f.mic.as_ref().unwrap().play.id, "ca");
        assert!(f.shared);
        assert!(is_virtual(&f, "cb") && !is_virtual(&f, "spk"));

        // Two cables: one each, not shared.
        let outs2 = [
            d("a", "CABLE-A Input (VB-Audio Cable A)"),
            d("b", "CABLE-B Input (VB-Audio Cable B)"),
        ];
        let ins2 = [
            d("ar", "CABLE-A Output (VB-Audio Cable A)"),
            d("br", "CABLE-B Output (VB-Audio Cable B)"),
        ];
        let f2 = detect(&outs2, &ins2);
        assert_eq!(f2.speaker.unwrap().rec.id, "ar");
        assert_eq!(f2.mic.unwrap().play.id, "b");
        assert!(!f2.shared);

        // macOS: same name on both sides.
        let f3 = detect(
            &[d("bh", "BlackHole 2ch"), d("mb", "MacBook Pro Speakers")],
            &[d("bh-in", "BlackHole 2ch")],
        );
        assert_eq!(f3.speaker.unwrap().rec.id, "bh-in");

        // Nothing virtual: nothing found, and a real output is never taken for one.
        let f4 = detect(&[d("spk", "Speakers")], &[d("m", "Microphone")]);
        assert_eq!(f4, Found::default());
        let none = note(&f4, false, "windows");
        assert!(
            none.contains("No virtual audio cable is installed"),
            "{none}"
        );
        assert_eq!(state(&f4, false), "missing");
        assert!(note(&f4, false, "macos").contains("No virtual audio cable"));
        assert!(note(&f4, false, "linux").contains("pactl"));
        assert!(note(&f4, true, "linux").contains("can add"));
        assert_eq!(state(&f4, true), "can_add");

        // macOS with two loopback devices, or the same maker's cable named alike on both sides.
        let f5 = detect(
            &[d("b2", "BlackHole 2ch"), d("b16", "BlackHole 16ch")],
            &[d("i2", "BlackHole 2ch"), d("i16", "BlackHole 16ch")],
        );
        assert_eq!(state(&f5, false), "ready");
        assert_eq!(
            (
                f5.speaker.unwrap().rec.id.as_str(),
                f5.mic.unwrap().rec.id.as_str(),
                f5.shared
            ),
            ("i2", "i16", false)
        );
        let f6 = detect(&[d("v", "VB-Cable")], &[d("vi", "VB-Cable")]);
        assert_eq!(f6.speaker.unwrap().rec.id, "vi");
    }

    #[test]
    fn sink_names_from_ids() {
        assert_eq!(
            sink_of("pulseaudio:alsa_output.pci.analog-stereo"),
            "alsa_output.pci.analog-stereo"
        );
        assert_eq!(sink_of("flux_speaker"), "flux_speaker");
    }
}
