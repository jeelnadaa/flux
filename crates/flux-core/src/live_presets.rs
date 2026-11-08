//! Built-in live presets and the `live.*` API (the UI's Live page and MCP tools both use it).

use crate::api::{err, Api};
use crate::live::{
    self, Lane, LivePreset, Macro, Modulator, Module, Source, StartOpts, Target, SHAPES, SYNCS,
};
use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::path::PathBuf;

// ---------------------------------------------------------------- builders

fn m(id: &str, kind: &str, params: &[(&str, f32)]) -> Module {
    Module {
        id: id.into(),
        kind: kind.into(),
        params: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        ..Default::default()
    }
}
fn mx(id: &str, kind: &str, params: &[(&str, f32)], mix: f32) -> Module {
    Module {
        mix,
        ..m(id, kind, params)
    }
}
fn lane(id: &str, name: &str, gain_db: f32, pan: f32, modules: Vec<Module>) -> Lane {
    Lane {
        id: id.into(),
        name: name.into(),
        gain_db,
        pan,
        mute: false,
        modules,
    }
}
#[allow(clippy::too_many_arguments)]
fn lfo(
    id: &str,
    name: &str,
    shape: &str,
    rate_hz: f32,
    sync: &str,
    depth: f32,
    bipolar: bool,
    targets: &[(&str, &str, f32)],
) -> Modulator {
    Modulator {
        id: id.into(),
        name: name.into(),
        shape: shape.into(),
        rate_hz,
        sync: sync.into(),
        depth,
        bipolar,
        targets: targets
            .iter()
            .map(|(mo, pa, a)| Target {
                module: mo.to_string(),
                param: pa.to_string(),
                amount: *a,
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}
fn mac(id: &str, name: &str, value: f32, targets: &[(&str, &str, f32, f32)]) -> Macro {
    Macro {
        id: id.into(),
        name: name.into(),
        value,
        targets: targets
            .iter()
            .map(|(mo, pa, f, t)| Target {
                module: mo.to_string(),
                param: pa.to_string(),
                from: *f,
                to: *t,
                ..Default::default()
            })
            .collect(),
    }
}
fn p(id: &str, name: &str, kind: &str, desc: &str, tags: &[&str]) -> LivePreset {
    LivePreset {
        id: id.into(),
        name: name.into(),
        kind: kind.into(),
        description: desc.into(),
        tags: tags.iter().map(|t| t.to_string()).collect(),
        builtin: true,
        ..Default::default()
    }
}

/// Spatial movement indexes: orbit, spiral, eight, swarm, place, path.
const ORBIT: f32 = 0.0;
const SPIRAL: f32 = 1.0;
const EIGHT: f32 = 2.0;
const SWARM: f32 = 3.0;

/// Every built-in live preset. Each one uses the same modules, modulators and macros a user can.
/// Output level of each built-in preset, measured so each one plays 3 LU quieter than the sound going in (BS.1770,
/// on the demo music or voice). Without it, presets ranged from 17 LU quieter to 18 LU louder than the input.
const LEVEL: &[(&str, f32)] = &[
    ("out-8d-orbit", -4.6),
    ("out-8d-spiral", -7.2),
    ("out-8d-swarm", -5.6),
    ("out-8d-eight", -8.2),
    ("out-slowed-reverb", 0.4),
    ("out-nightcore", -3.1),
    ("out-lofi", -1.6),
    ("out-vinyl", -2.5),
    ("out-cassette", -5.8),
    ("out-concert", -2.7),
    ("out-next-door", -0.8),
    ("out-underwater", -6.1),
    ("out-bass", -7.1),
    ("out-night", 0.5),
    ("out-clarity", -0.6),
    ("out-wide", -3.2),
    ("out-mono", -2.9),
    ("out-gate", -1.3),
    ("out-cathedral", -3.9),
    ("out-psych", -0.9),
    ("out-radio", -2.2),
    ("in-broadcast", -10.2),
    ("in-clean", -13.1),
    ("in-deep", -1.8),
    ("in-chipmunk", 0.9),
    ("in-helium", -2.2),
    ("in-robot", 3.2),
    ("in-choir", 11.0),
    ("in-hardtune", -0.0),
    ("in-host", -13.0),
    ("in-walkie", -10.2),
    ("in-megaphone", -23.2),
    ("in-phone", -5.3),
    ("in-cave", 1.8),
    ("in-stadium", -9.9),
    ("in-alien", 6.1),
    ("in-monster", -3.1),
    ("in-space", -4.5),
    ("in-ghost", 13.8),
    ("in-octaves", -1.4),
    ("in-warm", -8.9),
];

/// Every built-in preset, at its matched output level.
pub fn builtin() -> Vec<LivePreset> {
    let mut all = presets();
    for p in &mut all {
        if let Some((_, db)) = LEVEL.iter().find(|(id, _)| *id == p.id) {
            p.output_gain_db = *db;
        }
    }
    all
}

fn presets() -> Vec<LivePreset> {
    vec![
        // ---------------- for sound from the computer
        LivePreset {
            post: vec![
                m("sp", "spatial", &[("mode", ORBIT), ("rate", 0.12), ("depth", 85.0), ("room", 20.0), ("bass_mono", 1.0)]),
                mx("rv", "reverb", &[("decay", 1.8), ("predelay", 20.0), ("lowcut", 200.0)], 0.12),
            ],
            macros: vec![
                mac("speed", "Speed", 0.3, &[("sp", "rate", 0.03, 0.45)]),
                mac("room", "Room", 0.25, &[("rv", "mix", 0.0, 0.45), ("sp", "room", 0.0, 60.0)]),
            ],
            ..p("out-8d-orbit", "8D orbit", "output", "The sound circles your head. Use headphones.", &["8D", "spatial", "headphones"])
        },
        LivePreset {
            post: vec![m("sp", "spatial", &[("mode", SPIRAL), ("rate", 0.08), ("depth", 90.0), ("room", 25.0)])],
            mods: vec![lfo("breathe", "Breathe", "sine", 0.05, "", 0.6, true, &[("sp", "distance", 0.25)])],
            macros: vec![mac("speed", "Speed", 0.3, &[("sp", "rate", 0.02, 0.3)])],
            ..p("out-8d-spiral", "8D spiral", "output", "Spirals around and slowly drifts closer and further away.", &["8D", "spatial"])
        },
        LivePreset {
            post: vec![
                m("sp", "spatial", &[("mode", SWARM), ("rate", 0.35), ("depth", 80.0), ("room", 30.0)]),
                mx("ch", "chorus", &[("rate", 0.4), ("depth", 35.0)], 0.3),
            ],
            macros: vec![mac("chaos", "Chaos", 0.4, &[("sp", "rate", 0.1, 1.2), ("ch", "mix", 0.0, 0.6)])],
            ..p("out-8d-swarm", "8D swarm", "output", "The sound darts around you like a swarm.", &["8D", "spatial"])
        },
        LivePreset {
            post: vec![m("sp", "spatial", &[("mode", EIGHT), ("rate", 0.1), ("depth", 85.0), ("elevation", 20.0)])],
            mods: vec![lfo("rise", "Rise and fall", "triangle", 0.0, "4/1", 1.0, true, &[("sp", "elevation", 0.35)])],
            ..p("out-8d-eight", "8D figure eight", "output", "Loops in a figure eight that rises and dips in time with the music.", &["8D", "spatial"])
        },
        LivePreset {
            tempo: 90.0,
            post: vec![
                m("pt", "pitch", &[("semitones", -3.0), ("formant", 0.0)]),
                m("eq", "eq", &[("b1_type", 1.0), ("b1_freq", 120.0), ("b1_gain", 3.0), ("b5_type", 2.0), ("b5_freq", 6000.0), ("b5_gain", -4.0)]),
                mx("rv", "reverb", &[("decay", 4.5), ("predelay", 40.0), ("damping", 5000.0), ("width", 100.0)], 0.38),
            ],
            macros: vec![mac("depth", "Slower", 0.5, &[("pt", "semitones", -1.0, -5.0), ("rv", "decay", 2.5, 7.0)])],
            ..p("out-slowed-reverb", "Slowed + reverb", "output", "Live sound can't really slow down, so this lowers the pitch and adds a long, dark hall.", &["mood", "reverb"])
        },
        LivePreset {
            post: vec![
                m("pt", "pitch", &[("semitones", 4.0)]),
                m("eq", "eq", &[("b5_type", 2.0), ("b5_freq", 8000.0), ("b5_gain", 3.0)]),
                m("cp", "compressor", &[("threshold", -18.0), ("ratio", 3.0), ("auto_makeup", 1.0)]),
            ],
            ..p("out-nightcore", "Nightcore", "output", "Higher and brighter, with a little extra punch.", &["mood", "pitch"])
        },
        LivePreset {
            post: vec![m("lf", "lofi", &[("amount", 60.0), ("crackle", 4.0), ("wow", 0.4), ("pump", 20.0)])],
            mods: vec![lfo("drift", "Tape drift", "smooth", 0.2, "", 0.3, true, &[("lf", "lowpass", -0.15)])],
            macros: vec![mac("age", "Age", 0.5, &[("lf", "amount", 20.0, 100.0), ("lf", "crackle", 0.0, 12.0)])],
            ..p("out-lofi", "Lo-fi", "output", "Dusty, wobbly and warm, like a beat tape.", &["vintage", "lofi"])
        },
        LivePreset {
            post: vec![
                m("vn", "vinyl", &[("crackle", 5.0), ("pops", 0.6), ("wow", 0.3), ("warp", 20.0)]),
                m("eq", "eq", &[("b1_type", 1.0), ("b1_freq", 200.0), ("b1_gain", 2.0), ("b5_type", 2.0), ("b5_freq", 9000.0), ("b5_gain", -3.0)]),
            ],
            ..p("out-vinyl", "Vinyl evening", "output", "A record on a well-loved turntable.", &["vintage"])
        },
        LivePreset {
            post: vec![m("cs", "cassette", &[("hiss", -52.0), ("wow", 0.5), ("flutter", 6.0), ("saturation", 45.0)])],
            mods: vec![lfo("stretch", "Stretched tape", "smooth", 0.3, "", 0.5, true, &[("cs", "wow", 0.3)])],
            ..p("out-cassette", "Cassette", "output", "A tape that has been played a hundred times.", &["vintage"])
        },
        LivePreset {
            post: vec![m("hall", "row10", &[("row", 12.0), ("crowd", -30.0), ("pa_color", 50.0), ("applause", 0.0)])],
            macros: vec![mac("seat", "Seat", 0.3, &[("hall", "row", 2.0, 40.0)])],
            ..p("out-concert", "Concert seat", "output", "As if you were sitting in the hall. Move your seat with the macro.", &["space"])
        },
        LivePreset {
            post: vec![m("nd", "nextdoor", &[("wall", 600.0), ("bass", 6.0), ("drift", 1.0)])],
            ..p("out-next-door", "Next door", "output", "The party is in the next room.", &["space"])
        },
        LivePreset {
            post: vec![m("uw", "underwater", &[("depth", 60.0), ("wobble", 50.0)])],
            mods: vec![lfo("tide", "Tide", "sine", 0.07, "", 0.5, true, &[("uw", "depth", 0.3)])],
            ..p("out-underwater", "Underwater", "output", "Heard from under the surface, rising and sinking.", &["space"])
        },
        LivePreset {
            post: vec![
                m("eq", "eq", &[("b1_type", 1.0), ("b1_freq", 90.0), ("b1_gain", 8.0), ("b1_q", 0.7)]),
                m("cp", "compressor", &[("threshold", -16.0), ("ratio", 2.5), ("attack", 20.0), ("auto_makeup", 1.0)]),
            ],
            macros: vec![mac("boost", "Boost", 0.5, &[("eq", "b1_gain", 0.0, 14.0)])],
            ..p("out-bass", "Bass boost", "output", "More low end without blowing up.", &["tone"])
        },
        LivePreset {
            post: vec![
                m("cp", "compressor", &[("threshold", -32.0), ("ratio", 6.0), ("attack", 5.0), ("release", 300.0), ("auto_makeup", 1.0)]),
                m("lv", "leveler", &[("target", -22.0), ("max_gain", 12.0), ("speed", 1500.0)]),
            ],
            output_gain_db: -4.0,
            ..p("out-night", "Night mode", "output", "Quiet parts up, loud parts down, so you can listen softly late at night.", &["dynamics", "useful"])
        },
        LivePreset {
            post: vec![
                m("eq", "eq", &[("b2_type", 0.0), ("b2_freq", 250.0), ("b2_gain", -3.0), ("b3_type", 0.0), ("b3_freq", 3000.0), ("b3_gain", 4.0), ("b3_q", 0.8)]),
                m("ds", "deesser", &[("freq", 6500.0), ("threshold", -30.0)]),
            ],
            ..p("out-clarity", "Voice clarity", "output", "Clearer speech in videos, calls and podcasts.", &["useful", "voice"])
        },
        LivePreset {
            post: vec![m("st", "stereo", &[("width", 170.0)])],
            width: 1.2,
            ..p("out-wide", "Extra wide", "output", "Pushes the sound out past your ears.", &["space"])
        },
        LivePreset {
            post: vec![m("st", "stereo", &[("mono", 1.0)])],
            ..p("out-mono", "Mono", "output", "Both ears hear the same thing. Useful with one earbud.", &["useful"])
        },
        LivePreset {
            tempo: 128.0,
            lanes: vec![
                lane("dry", "Gated", 0.0, 0.0, vec![m("tg", "trancegate", &[("steps", 0b1011_0110_1101_1010 as f32), ("depth", 90.0)])]),
                lane("tail", "Echo tail", -6.0, 0.0, vec![m("dl", "delay", &[("time", 351.0), ("feedback", 45.0), ("pingpong", 1.0)]), mx("rv", "reverb", &[("decay", 3.0)], 0.5)]),
            ],
            macros: vec![mac("chop", "Chop", 0.8, &[("tg", "depth", 0.0, 100.0)])],
            ..p("out-gate", "Trance gate", "output", "Chops the sound into a rhythm, with echoes in the gaps.", &["rhythm"])
        },
        LivePreset {
            lanes: vec![
                lane("dry", "Dry", -2.0, 0.0, vec![]),
                lane("wet", "Cathedral", -3.0, 0.0, vec![m("rv", "reverb", &[("decay", 8.0), ("size", 1.5), ("predelay", 60.0), ("damping", 6000.0), ("lowcut", 250.0)])]),
            ],
            macros: vec![mac("space", "Space", 0.5, &[("rv", "mix", 0.4, 1.0), ("rv", "decay", 4.0, 12.0)])],
            ..p("out-cathedral", "Cathedral", "output", "A huge stone space around everything you hear.", &["space", "reverb"])
        },
        LivePreset {
            post: vec![
                mx("ph", "phaser", &[("rate", 0.2), ("depth", 70.0), ("stages", 8.0), ("feedback", 50.0)], 0.6),
                mx("fl", "flanger", &[("rate", 0.1), ("depth", 60.0), ("feedback", 40.0)], 0.35),
                m("sp", "spatial", &[("mode", ORBIT), ("rate", 0.07), ("depth", 70.0)]),
            ],
            mods: vec![
                lfo("swirl", "Swirl", "triangle", 0.0, "2/1", 0.8, true, &[("ph", "rate", 0.2), ("fl", "mix", 0.3)]),
                lfo("dream", "Dream", "smooth", 0.15, "", 0.6, true, &[("sp", "rate", 0.1)]),
            ],
            ..p("out-psych", "Psychedelic", "output", "Swirling, sweeping and slowly orbiting.", &["creative", "spatial"])
        },
        LivePreset {
            post: vec![m("rd", "radio", &[("low", 400.0), ("high", 3200.0), ("drive", 30.0), ("noise", -42.0)])],
            mods: vec![lfo("tune", "Tuning", "smooth", 0.4, "", 0.5, true, &[("rd", "high", 0.2), ("rd", "noise", 0.25)])],
            ..p("out-radio", "Old radio", "output", "A crackly AM radio that drifts off the station.", &["vintage"])
        },
        // ---------------- for a microphone
        LivePreset {
            post: vec![
                m("gt", "gate", &[("threshold", -50.0), ("range", -30.0), ("hold", 120.0)]),
                m("eq", "eq", &[("b1_type", 4.0), ("b1_freq", 80.0), ("b2_type", 0.0), ("b2_freq", 300.0), ("b2_gain", -3.0), ("b4_type", 2.0), ("b4_freq", 5000.0), ("b4_gain", 3.0)]),
                m("cp", "compressor", &[("threshold", -22.0), ("ratio", 4.0), ("attack", 8.0), ("release", 150.0), ("auto_makeup", 1.0)]),
                m("ds", "deesser", &[("freq", 6500.0), ("threshold", -28.0)]),
            ],
            ..p("in-broadcast", "Broadcast voice", "input", "Rich, close and even, like a radio presenter.", &["voice", "clean"])
        },
        LivePreset {
            post: vec![
                m("gt", "gate", &[("threshold", -48.0), ("range", -40.0), ("hold", 150.0)]),
                m("ds", "deesser", &[]),
                m("lv", "leveler", &[("target", -18.0)]),
            ],
            ..p("in-clean", "Clean mic", "input", "Quiets the room between words and keeps your level steady.", &["voice", "clean"])
        },
        LivePreset {
            post: vec![m("vc", "voice", &[("pitch", -5.0), ("formant", -40.0)])],
            macros: vec![mac("depth", "Depth", 0.5, &[("vc", "pitch", 0.0, -10.0), ("vc", "formant", 0.0, -80.0)])],
            ..p("in-deep", "Deep voice", "input", "Lower and bigger.", &["voice changer"])
        },
        LivePreset {
            post: vec![m("pt", "pitch", &[("semitones", 8.0), ("formant", 0.0)])],
            ..p("in-chipmunk", "Chipmunk", "input", "Small and squeaky.", &["voice changer", "fun"])
        },
        LivePreset {
            post: vec![m("vc", "voice", &[("formant", 80.0)])],
            ..p("in-helium", "Helium", "input", "Same notes, tiny throat, like after a balloon.", &["voice changer", "fun"])
        },
        LivePreset {
            post: vec![
                m("vc", "voice", &[("ring", 75.0), ("ring_freq", 55.0), ("comb", 30.0)]),
                mx("bc", "bitcrusher", &[("bits", 8.0), ("downsample", 2.0)], 0.4),
            ],
            mods: vec![lfo("motor", "Motor", "steps", 0.0, "1/8", 0.4, true, &[("vc", "ring_freq", 0.05)])],
            ..p("in-robot", "Robot", "input", "A metal voice with a buzzing motor.", &["voice changer", "sci-fi"])
        },
        LivePreset {
            post: vec![m("vc", "voice", &[("vocoder", 100.0), ("carrier", 2.0), ("bands", 24.0)]), mx("rv", "reverb", &[("decay", 2.5)], 0.3)],
            ..p("in-choir", "Vocoder choir", "input", "Your words, sung by a synth chord.", &["voice changer", "music"])
        },
        LivePreset {
            post: vec![m("vc", "voice", &[("tune", 1.0), ("retune", 1.0), ("scale", 0.0)])],
            macros: vec![mac("snap", "Snap", 1.0, &[("vc", "retune", 120.0, 1.0)])],
            ..p("in-hardtune", "Hard tune", "input", "Snaps your voice to the notes of a scale.", &["voice changer", "music"])
        },
        LivePreset {
            post: vec![m("rd", "radio", &[]), m("cp", "compressor", &[("threshold", -20.0), ("ratio", 4.0), ("auto_makeup", 1.0)]), mx("rv", "reverb", &[("decay", 0.6)], 0.12)],
            ..p("in-host", "Radio host", "input", "A late-night host on a small station.", &["voice", "vintage"])
        },
        LivePreset {
            post: vec![m("wk", "walkie", &[("squelch", 1.0), ("drive", 40.0)])],
            ..p("in-walkie", "Walkie-talkie", "input", "Over. With the squelch when you stop talking.", &["voice changer"])
        },
        LivePreset {
            post: vec![m("mg", "megaphone", &[("drive", 50.0)])],
            ..p("in-megaphone", "Megaphone", "input", "Loud, narrow and honky.", &["voice changer"])
        },
        LivePreset {
            post: vec![m("ph", "phone", &[])],
            ..p("in-phone", "Phone call", "input", "A voice down a phone line.", &["voice changer"])
        },
        LivePreset {
            post: vec![
                mx("dl", "delay", &[("time", 420.0), ("feedback", 40.0), ("tone", 2500.0)], 0.3),
                mx("rv", "reverb", &[("decay", 5.0), ("damping", 3000.0), ("predelay", 30.0)], 0.4),
            ],
            ..p("in-cave", "Cave", "input", "Echoes off wet stone walls.", &["space"])
        },
        LivePreset {
            lanes: vec![
                lane("dry", "Voice", 0.0, 0.0, vec![m("cp", "compressor", &[("threshold", -20.0), ("ratio", 3.0), ("auto_makeup", 1.0)])]),
                lane("slap", "Slap", -9.0, 0.0, vec![m("dl", "delay", &[("time", 140.0), ("feedback", 20.0)])]),
                lane("hall", "Stadium", -6.0, 0.0, vec![m("rv", "reverb", &[("decay", 6.0), ("predelay", 90.0), ("size", 1.5)])]),
            ],
            ..p("in-stadium", "Stadium announcer", "input", "Your voice bouncing around a stadium.", &["space", "fun"])
        },
        LivePreset {
            post: vec![
                m("vc", "voice", &[("pitch", 3.0), ("formant", 60.0), ("ring", 30.0), ("ring_freq", 220.0)]),
                mx("ph", "phaser", &[("rate", 1.5), ("depth", 80.0), ("stages", 10.0)], 0.5),
            ],
            mods: vec![lfo("signal", "Signal", "random", 0.0, "1/16", 0.6, true, &[("vc", "ring_freq", 0.3)])],
            ..p("in-alien", "Alien", "input", "A transmission from somewhere else.", &["voice changer", "sci-fi"])
        },
        LivePreset {
            post: vec![
                m("vc", "voice", &[("pitch", -9.0), ("formant", -80.0), ("tremor", 30.0)]),
                mx("ds", "distortion", &[("drive", 12.0), ("type", 0.0), ("tone", 3000.0)], 0.5),
                mx("rv", "reverb", &[("decay", 2.0), ("damping", 2500.0)], 0.2),
            ],
            ..p("in-monster", "Monster", "input", "Huge, growling and dark.", &["voice changer", "fun"])
        },
        LivePreset {
            post: vec![
                m("rd", "radio", &[("low", 300.0), ("high", 3500.0), ("noise", -50.0)]),
                mx("fl", "flanger", &[("rate", 0.3), ("depth", 50.0), ("feedback", 50.0)], 0.4),
                mx("rv", "reverb", &[("decay", 1.2)], 0.2),
            ],
            mods: vec![lfo("orbit", "Orbit", "sine", 0.0, "1/1", 0.5, true, &[("fl", "rate", 0.2)])],
            ..p("in-space", "Space station", "input", "Mission control, from orbit.", &["voice changer", "sci-fi"])
        },
        LivePreset {
            post: vec![
                m("vc", "voice", &[("vocoder", 100.0), ("carrier", 1.0)]),
                mx("rv", "reverb", &[("decay", 4.0), ("reverse", 0.0)], 0.5),
            ],
            mods: vec![lfo("haunt", "Haunt", "smooth", 0.3, "", 0.6, true, &[("rv", "mix", 0.3)])],
            ..p("in-ghost", "Ghost whisper", "input", "Your words as a breathy whisper in an empty room.", &["voice changer", "fun"])
        },
        LivePreset {
            lanes: vec![
                lane("mid", "Voice", 0.0, 0.0, vec![]),
                lane("low", "Octave down", -6.0, -0.4, vec![m("pt1", "pitch", &[("semitones", -12.0), ("formant", 1.0)])]),
                lane("high", "Octave up", -10.0, 0.4, vec![m("pt2", "pitch", &[("semitones", 12.0), ("formant", 1.0)])]),
            ],
            post: vec![mx("rv", "reverb", &[("decay", 2.2)], 0.25)],
            macros: vec![mac("stack", "Stack", 0.6, &[("pt1", "mix", 0.0, 1.0), ("pt2", "mix", 0.0, 1.0)])],
            ..p("in-octaves", "Octave stack", "input", "Three of you, an octave apart.", &["voice changer", "music"])
        },
        LivePreset {
            post: vec![
                m("eq", "eq", &[("b1_type", 4.0), ("b1_freq", 70.0), ("b2_type", 1.0), ("b2_freq", 180.0), ("b2_gain", 2.0)]),
                m("cp", "compressor", &[("threshold", -20.0), ("ratio", 3.0), ("attack", 15.0), ("auto_makeup", 1.0)]),
                mx("cs", "cassette", &[("hiss", -78.0), ("saturation", 35.0), ("wow", 0.0), ("flutter", 0.0)], 0.6),
            ],
            ..p("in-warm", "Studio warmth", "input", "A warm, smooth vocal chain.", &["voice", "clean"])
        },
    ]
}

// ---------------------------------------------------------------- storage

fn dir(api: &Api) -> PathBuf {
    api.lock().store.root.join("LivePresets")
}
fn state_path(api: &Api) -> PathBuf {
    api.lock().store.root.join("live.json")
}

fn user(api: &Api) -> Vec<LivePreset> {
    let mut v = vec![];
    if let Ok(rd) = std::fs::read_dir(dir(api)) {
        for e in rd.flatten() {
            let path = e.path();
            if path.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            // treat every file as hostile: size cap, parse, validate
            if e.metadata().map(|m| m.len() > 512 * 1024).unwrap_or(true) {
                continue;
            }
            if let Ok(b) = std::fs::read(&path) {
                if let Ok(mut p) = serde_json::from_slice::<LivePreset>(&b) {
                    p.builtin = false;
                    if p.validate().is_ok() {
                        v.push(p);
                    }
                }
            }
        }
    }
    v.sort_by_key(|a| a.name.to_lowercase());
    v
}

pub fn all(api: &Api) -> Vec<LivePreset> {
    let mut v = builtin();
    v.extend(user(api));
    v
}

pub fn get(api: &Api, id: &str) -> Result<LivePreset> {
    all(api).into_iter().find(|p| p.id == id).ok_or_else(|| {
        err(
            "not_found",
            format!("No live preset `{id}`. Call live.presets for the list."),
        )
    })
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let t = out.trim_matches('-');
    if t.is_empty() {
        "preset".into()
    } else {
        t.chars().take(32).collect()
    }
}

fn new_user_id(api: &Api, name: &str) -> String {
    let taken: Vec<String> = all(api).into_iter().map(|p| p.id).collect();
    let base = format!("user-{}", slug(name));
    if !taken.contains(&base) {
        return base;
    }
    (2..)
        .map(|i| format!("{base}-{i}"))
        .find(|c| !taken.contains(c))
        .unwrap()
}

fn write(api: &Api, p: &LivePreset) -> Result<()> {
    let d = dir(api);
    std::fs::create_dir_all(&d)?;
    crate::store::atomic_write(
        &d.join(format!("{}.json", p.id)),
        serde_json::to_string_pretty(p)?.as_bytes(),
    )
}

/// Push a changed preset into any route that is playing it.
fn refresh(api: &Api, p: &LivePreset) {
    let mut s = api.lock();
    for r in s.live.routes.values_mut() {
        if r.info.preset == p.id {
            r.update(p);
        }
    }
}

/// Edit a user preset: apply `f`, validate, save, and update any route playing it.
fn edit(api: &Api, v: &Value, f: impl FnOnce(&mut LivePreset) -> Result<Value>) -> Result<Value> {
    let id = req(v, "preset")?;
    let mut p = get(api, &id)?;
    if p.builtin {
        bail!(err("read_only", "Built-in presets can't be changed. Call live.preset.create with `from` to make your own copy."));
    }
    let out = f(&mut p)?;
    p.validate()?;
    write(api, &p)?;
    refresh(api, &p);
    Ok(json!({ "preset": p, "result": out }))
}

fn req(v: &Value, k: &str) -> Result<String> {
    v.get(k)
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| err("bad_params", format!("`{k}` is required")))
}
fn os(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.as_str()).map(|s| s.to_string())
}
fn of(v: &Value, k: &str) -> Option<f32> {
    v.get(k).and_then(|x| x.as_f64()).map(|x| x as f32)
}
fn ob(v: &Value, k: &str) -> Option<bool> {
    v.get(k).and_then(|x| x.as_bool())
}

fn summary(p: &LivePreset) -> Value {
    json!({
        "id": p.id, "name": p.name, "kind": p.kind, "description": p.description, "tags": p.tags,
        "builtin": p.builtin, "modules": p.modules().count(), "lanes": p.lanes.len(),
        "mods": p.mods.len(), "macros": p.macros.iter().map(|m| json!({"id": m.id, "name": m.name, "value": m.value})).collect::<Vec<_>>(),
        "kinds": p.modules().map(|m| m.kind.clone()).collect::<Vec<_>>(),
    })
}

// ---------------------------------------------------------------- handlers

pub fn h_kinds(_: &Api, _: &Value) -> Result<Value> {
    let kinds: Vec<Value> = crate::fx::SPECS
        .iter()
        .filter(|s| live::live_kind(s.kind))
        .map(crate::methods::spec_json)
        .collect();
    Ok(json!({
        "kinds": kinds,
        "not_live": live::NOT_LIVE,
        "shapes": SHAPES.iter().map(|(k, t)| json!({"id": k, "tip": t})).collect::<Vec<_>>(),
        "syncs": &SYNCS[1..],
        "tips": live::tips(),
        "limits": {"lanes": live::MAX_LANES, "modules": live::MAX_MODULES, "mods": live::MAX_MODS, "targets": live::MAX_TARGETS, "macros": live::MAX_MACROS, "steps": live::MAX_STEPS},
    }))
}

pub fn h_presets(api: &Api, v: &Value) -> Result<Value> {
    let kind = os(v, "kind");
    Ok(Value::Array(
        all(api)
            .iter()
            .filter(|p| match kind.as_deref() {
                None | Some("all") => true,
                Some(k) => p.kind == k || p.kind == "any",
            })
            .map(summary)
            .collect(),
    ))
}

pub fn h_preset_get(api: &Api, v: &Value) -> Result<Value> {
    Ok(serde_json::to_value(get(api, &req(v, "id")?)?)?)
}

pub fn h_preset_create(api: &Api, v: &Value) -> Result<Value> {
    let name = os(v, "name").unwrap_or_else(|| "My preset".into());
    let mut p = match os(v, "from") {
        Some(from) => {
            let mut p = get(api, &from)?;
            if os(v, "name").is_none() {
                p.name = format!("{} (mine)", p.name);
            } else {
                p.name = name.clone();
            }
            p
        }
        None => LivePreset {
            name: name.clone(),
            kind: os(v, "kind").unwrap_or_else(|| "any".into()),
            ..Default::default()
        },
    };
    p.builtin = false;
    p.id = new_user_id(api, &p.name);
    p.validate()?;
    write(api, &p)?;
    Ok(serde_json::to_value(p)?)
}

pub fn h_preset_save(api: &Api, v: &Value) -> Result<Value> {
    let raw = v.get("preset").cloned().unwrap_or(Value::Null);
    let mut p: LivePreset = serde_json::from_value(raw)
        .map_err(|e| err("bad_params", format!("that isn't a live preset: {e}")))?;
    p.builtin = false;
    let exists = user(api).iter().any(|u| u.id == p.id);
    if p.id.is_empty()
        || !exists && all(api).iter().any(|a| a.id == p.id)
        || !p.id.starts_with("user-")
    {
        p.id = new_user_id(api, &p.name);
    }
    p.validate()?;
    write(api, &p)?;
    refresh(api, &p);
    Ok(serde_json::to_value(p)?)
}

pub fn h_preset_update(api: &Api, v: &Value) -> Result<Value> {
    edit(api, v, |p| {
        if let Some(x) = os(v, "name") {
            p.name = x;
        }
        if let Some(x) = os(v, "description") {
            p.description = x;
        }
        if let Some(x) = os(v, "kind") {
            p.kind = x;
        }
        if let Some(t) = v.get("tags").and_then(|x| x.as_array()) {
            p.tags = t
                .iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect();
        }
        if let Some(x) = of(v, "tempo") {
            p.tempo = x;
        }
        if let Some(x) = of(v, "input_gain_db") {
            p.input_gain_db = x;
        }
        if let Some(x) = of(v, "output_gain_db") {
            p.output_gain_db = x;
        }
        if let Some(x) = of(v, "width") {
            p.width = x;
        }
        if let Some(x) = of(v, "balance") {
            p.balance = x;
        }
        if let Some(x) = ob(v, "safety_limiter") {
            p.safety_limiter = x;
        }
        Ok(Value::Null)
    })
}

pub fn h_preset_delete(api: &Api, v: &Value) -> Result<Value> {
    let id = req(v, "id")?;
    let p = get(api, &id)?;
    if p.builtin {
        bail!(err("read_only", "Built-in presets can't be deleted."));
    }
    if api.lock().live.routes.values().any(|r| r.info.preset == id) {
        bail!(err(
            "busy",
            "That preset is playing. Pick another preset for the route first."
        ));
    }
    std::fs::remove_file(dir(api).join(format!("{id}.json")))?;
    Ok(json!({"deleted": id}))
}

pub fn h_preset_export(api: &Api, v: &Value) -> Result<Value> {
    let mut p = get(api, &req(v, "id")?)?;
    p.builtin = false;
    let text = serde_json::to_string_pretty(&json!({"flux_live_preset": 1, "preset": p}))?;
    if let Some(path) = os(v, "path") {
        let path = PathBuf::from(path);
        if path.is_dir() {
            bail!(err("bad_params", "`path` must be a file, not a folder"));
        }
        crate::store::atomic_write(&path, text.as_bytes())?;
        return Ok(json!({"path": path.display().to_string(), "bytes": text.len()}));
    }
    Ok(json!({"json": text}))
}

pub fn h_preset_import(api: &Api, v: &Value) -> Result<Value> {
    let text = match (os(v, "path"), os(v, "json")) {
        (Some(path), _) => {
            let meta = std::fs::metadata(&path)
                .map_err(|e| err("not_found", format!("can't read that file: {e}")))?;
            if meta.len() > 512 * 1024 {
                bail!(err("bad_file", "That file is too big to be a live preset."));
            }
            std::fs::read_to_string(&path)
                .map_err(|e| err("bad_file", format!("can't read that file: {e}")))?
        }
        (None, Some(j)) => j,
        _ => bail!(err(
            "bad_params",
            "live.preset.import needs `path` or `json`"
        )),
    };
    if text.len() > 512 * 1024 {
        bail!(err("bad_file", "That is too big to be a live preset."));
    }
    let raw: Value = serde_json::from_str(&text)
        .map_err(|e| err("bad_file", format!("That isn't a Flux live preset: {e}")))?;
    let inner = raw.get("preset").cloned().unwrap_or(raw);
    let mut p: LivePreset = serde_json::from_value(inner)
        .map_err(|e| err("bad_file", format!("That isn't a Flux live preset: {e}")))?;
    p.builtin = false;
    p.id = new_user_id(api, &p.name);
    p.validate()
        .map_err(|e| err("bad_file", format!("That preset has a problem: {e}")))?;
    write(api, &p)?;
    Ok(serde_json::to_value(p)?)
}

fn module_from(v: &Value) -> Result<Module> {
    let kind = req(v, "kind")?;
    let mut md = Module {
        kind,
        ..Default::default()
    };
    if let Some(ps) = v.get("params").and_then(|x| x.as_object()) {
        for (k, x) in ps {
            let x = x
                .as_f64()
                .ok_or_else(|| err("bad_params", format!("parameter `{k}` must be a number")))?;
            md.params.insert(k.clone(), x as f32);
        }
    }
    if let Some(x) = of(v, "mix") {
        md.mix = x;
    }
    if let Some(x) = ob(v, "enabled") {
        md.enabled = x;
    }
    Ok(md)
}

pub fn h_module_add(api: &Api, v: &Value) -> Result<Value> {
    edit(api, v, |p| {
        let mut md = module_from(v)?;
        md.id = p.new_id("m");
        let id = md.id.clone();
        let index = v.get("index").and_then(|x| x.as_u64()).map(|x| x as usize);
        let list = match os(v, "lane").as_deref() {
            None | Some("post") => &mut p.post,
            Some(l) => {
                &mut p
                    .lanes
                    .iter_mut()
                    .find(|x| x.id == l)
                    .ok_or_else(|| err("not_found", format!("no lane `{l}`")))?
                    .modules
            }
        };
        let at = index.unwrap_or(list.len()).min(list.len());
        list.insert(at, md);
        Ok(json!({"module": id}))
    })
}

pub fn h_module_set(api: &Api, v: &Value) -> Result<Value> {
    edit(api, v, |p| {
        let id = req(v, "module")?;
        let md = p
            .module_mut(&id)
            .ok_or_else(|| err("not_found", format!("no module `{id}`")))?;
        if let Some(ps) = v.get("params").and_then(|x| x.as_object()) {
            for (k, x) in ps {
                let x = x.as_f64().ok_or_else(|| {
                    err("bad_params", format!("parameter `{k}` must be a number"))
                })?;
                md.params.insert(k.clone(), x as f32);
            }
        }
        if let Some(x) = of(v, "mix") {
            md.mix = x;
        }
        if let Some(x) = ob(v, "enabled") {
            md.enabled = x;
        }
        if let Some(path) = v.get("path").and_then(|x| x.as_array()) {
            md.path = path
                .iter()
                .filter_map(|pt| {
                    let a = pt.as_array()?;
                    Some([a.first()?.as_f64()? as f32, a.get(1)?.as_f64()? as f32])
                })
                .collect();
        }
        Ok(json!({"module": id}))
    })
}

fn remove_refs(p: &mut LivePreset, id: &str) {
    for m in &mut p.mods {
        m.targets.retain(|t| t.module != id);
    }
    for m in &mut p.macros {
        m.targets.retain(|t| t.module != id);
    }
}

pub fn h_module_remove(api: &Api, v: &Value) -> Result<Value> {
    edit(api, v, |p| {
        let id = req(v, "module")?;
        let before = p.modules().count();
        for l in &mut p.lanes {
            l.modules.retain(|m| m.id != id);
        }
        p.post.retain(|m| m.id != id);
        if p.modules().count() == before {
            bail!(err("not_found", format!("no module `{id}`")));
        }
        remove_refs(p, &id);
        Ok(json!({"removed": id}))
    })
}

pub fn h_module_move(api: &Api, v: &Value) -> Result<Value> {
    edit(api, v, |p| {
        let id = req(v, "module")?;
        let mut taken = None;
        for l in &mut p.lanes {
            if let Some(i) = l.modules.iter().position(|m| m.id == id) {
                taken = Some(l.modules.remove(i));
            }
        }
        if taken.is_none() {
            if let Some(i) = p.post.iter().position(|m| m.id == id) {
                taken = Some(p.post.remove(i));
            }
        }
        let md = taken.ok_or_else(|| err("not_found", format!("no module `{id}`")))?;
        let list = match os(v, "lane").as_deref() {
            None | Some("post") => &mut p.post,
            Some(l) => {
                &mut p
                    .lanes
                    .iter_mut()
                    .find(|x| x.id == l)
                    .ok_or_else(|| err("not_found", format!("no lane `{l}`")))?
                    .modules
            }
        };
        let at = v
            .get("index")
            .and_then(|x| x.as_u64())
            .map(|x| x as usize)
            .unwrap_or(list.len())
            .min(list.len());
        list.insert(at, md);
        Ok(json!({"module": id}))
    })
}

pub fn h_lane_add(api: &Api, v: &Value) -> Result<Value> {
    edit(api, v, |p| {
        let id = p.new_id("lane");
        let n = p.lanes.len() + 1;
        // the first lane added to a preset with only a post chain starts as the dry signal
        p.lanes.push(Lane {
            id: id.clone(),
            name: os(v, "name").unwrap_or_else(|| format!("Lane {n}")),
            gain_db: of(v, "gain_db").unwrap_or(0.0),
            pan: of(v, "pan").unwrap_or(0.0),
            ..Default::default()
        });
        Ok(json!({"lane": id}))
    })
}

pub fn h_lane_set(api: &Api, v: &Value) -> Result<Value> {
    edit(api, v, |p| {
        let id = req(v, "lane")?;
        let l = p
            .lanes
            .iter_mut()
            .find(|l| l.id == id)
            .ok_or_else(|| err("not_found", format!("no lane `{id}`")))?;
        if let Some(x) = os(v, "name") {
            l.name = x;
        }
        if let Some(x) = of(v, "gain_db") {
            l.gain_db = x;
        }
        if let Some(x) = of(v, "pan") {
            l.pan = x;
        }
        if let Some(x) = ob(v, "mute") {
            l.mute = x;
        }
        Ok(json!({"lane": id}))
    })
}

pub fn h_lane_remove(api: &Api, v: &Value) -> Result<Value> {
    edit(api, v, |p| {
        let id = req(v, "lane")?;
        let i = p
            .lanes
            .iter()
            .position(|l| l.id == id)
            .ok_or_else(|| err("not_found", format!("no lane `{id}`")))?;
        let l = p.lanes.remove(i);
        for m in &l.modules {
            remove_refs(p, &m.id);
        }
        Ok(json!({"removed": id}))
    })
}

fn apply_mod_fields(m: &mut Modulator, v: &Value) -> Result<()> {
    if let Some(x) = os(v, "name") {
        m.name = x;
    }
    if let Some(x) = os(v, "shape") {
        m.shape = x;
    }
    if let Some(x) = of(v, "rate_hz") {
        m.rate_hz = x;
    }
    if let Some(x) = os(v, "sync") {
        m.sync = x;
    }
    if let Some(x) = of(v, "depth") {
        m.depth = x;
    }
    if let Some(x) = of(v, "offset") {
        m.offset = x;
    }
    if let Some(x) = of(v, "phase") {
        m.phase = x;
    }
    if let Some(x) = ob(v, "bipolar") {
        m.bipolar = x;
    }
    if let Some(x) = of(v, "smooth") {
        m.smooth = x;
    }
    if let Some(x) = of(v, "attack_ms") {
        m.attack_ms = x;
    }
    if let Some(x) = of(v, "release_ms") {
        m.release_ms = x;
    }
    if let Some(x) = v.get("seed").and_then(|x| x.as_u64()) {
        m.seed = x as u32;
    }
    if let Some(s) = v.get("steps").and_then(|x| x.as_array()) {
        m.steps = s
            .iter()
            .filter_map(|x| x.as_f64().map(|f| f as f32))
            .collect();
    }
    if let Some(t) = v.get("targets") {
        m.targets = serde_json::from_value(t.clone())
            .map_err(|e| err("bad_params", format!("targets: {e}")))?;
    }
    Ok(())
}

pub fn h_mod_add(api: &Api, v: &Value) -> Result<Value> {
    edit(api, v, |p| {
        let mut m = Modulator {
            id: p.new_id("mod"),
            name: format!("Modulator {}", p.mods.len() + 1),
            ..Default::default()
        };
        apply_mod_fields(&mut m, v)?;
        let id = m.id.clone();
        p.mods.push(m);
        Ok(json!({"mod": id}))
    })
}

pub fn h_mod_set(api: &Api, v: &Value) -> Result<Value> {
    edit(api, v, |p| {
        let id = req(v, "mod")?;
        let m = p
            .mods
            .iter_mut()
            .find(|m| m.id == id)
            .ok_or_else(|| err("not_found", format!("no modulator `{id}`")))?;
        apply_mod_fields(m, v)?;
        Ok(json!({"mod": id}))
    })
}

pub fn h_mod_remove(api: &Api, v: &Value) -> Result<Value> {
    edit(api, v, |p| {
        let id = req(v, "mod")?;
        let n = p.mods.len();
        p.mods.retain(|m| m.id != id);
        if p.mods.len() == n {
            bail!(err("not_found", format!("no modulator `{id}`")));
        }
        Ok(json!({"removed": id}))
    })
}

/// Connect a modulator to a parameter, change the amount, or disconnect with amount 0.
pub fn h_mod_target(api: &Api, v: &Value) -> Result<Value> {
    edit(api, v, |p| {
        let id = req(v, "mod")?;
        let module = req(v, "module")?;
        let param = req(v, "param")?;
        let amount = of(v, "amount").unwrap_or(0.5);
        let m = p
            .mods
            .iter_mut()
            .find(|m| m.id == id)
            .ok_or_else(|| err("not_found", format!("no modulator `{id}`")))?;
        m.targets
            .retain(|t| !(t.module == module && t.param == param));
        if amount != 0.0 {
            m.targets.push(Target {
                module,
                param,
                amount,
                ..Default::default()
            });
        }
        Ok(json!({"targets": m.targets.len()}))
    })
}

pub fn h_macro_add(api: &Api, v: &Value) -> Result<Value> {
    edit(api, v, |p| {
        let id = p.new_id("macro");
        p.macros.push(Macro {
            id: id.clone(),
            name: os(v, "name").unwrap_or_else(|| format!("Macro {}", p.macros.len() + 1)),
            value: of(v, "value").unwrap_or(0.5),
            targets: vec![],
        });
        Ok(json!({"macro": id}))
    })
}

pub fn h_macro_set(api: &Api, v: &Value) -> Result<Value> {
    edit(api, v, |p| {
        let id = req(v, "macro")?;
        let m = p
            .macros
            .iter_mut()
            .find(|m| m.id == id)
            .ok_or_else(|| err("not_found", format!("no macro `{id}`")))?;
        if let Some(x) = os(v, "name") {
            m.name = x;
        }
        if let Some(x) = of(v, "value") {
            m.value = x;
        }
        if let Some(t) = v.get("targets") {
            m.targets = serde_json::from_value(t.clone())
                .map_err(|e| err("bad_params", format!("targets: {e}")))?;
        }
        Ok(json!({"macro": id}))
    })
}

pub fn h_macro_remove(api: &Api, v: &Value) -> Result<Value> {
    edit(api, v, |p| {
        let id = req(v, "macro")?;
        let n = p.macros.len();
        p.macros.retain(|m| m.id != id);
        if p.macros.len() == n {
            bail!(err("not_found", format!("no macro `{id}`")));
        }
        Ok(json!({"removed": id}))
    })
}

/// Connect a macro to a parameter (with its range and curve), or disconnect with `remove`.
pub fn h_macro_target(api: &Api, v: &Value) -> Result<Value> {
    edit(api, v, |p| {
        let id = req(v, "macro")?;
        let module = req(v, "module")?;
        let param = req(v, "param")?;
        // default range: the parameter's current value to its maximum
        let (cur, max) = match p.module(&module) {
            Some(md) if param == "mix" => (md.mix, 1.0),
            Some(md) => {
                let spec =
                    crate::fx::spec(&md.kind).ok_or_else(|| err("not_found", "unknown kind"))?;
                let ps = spec.params.iter().find(|x| x.key == param).ok_or_else(|| {
                    err(
                        "bad_params",
                        format!("{} has no parameter `{param}`", md.kind),
                    )
                })?;
                (md.params.get(&param).copied().unwrap_or(ps.def), ps.max)
            }
            None => bail!(err("not_found", format!("no module `{module}`"))),
        };
        let m = p
            .macros
            .iter_mut()
            .find(|m| m.id == id)
            .ok_or_else(|| err("not_found", format!("no macro `{id}`")))?;
        m.targets
            .retain(|t| !(t.module == module && t.param == param));
        if !ob(v, "remove").unwrap_or(false) {
            m.targets.push(Target {
                module,
                param,
                from: of(v, "from").unwrap_or(cur),
                to: of(v, "to").unwrap_or(max),
                curve: of(v, "curve").unwrap_or(0.0),
                ..Default::default()
            });
        }
        Ok(json!({"targets": m.targets.len()}))
    })
}

/// Randomize a user preset's parameters around their current values. Creative starting points.
pub fn h_randomize(api: &Api, v: &Value) -> Result<Value> {
    edit(api, v, |p| {
        let amount = of(v, "amount").unwrap_or(0.3).clamp(0.0, 1.0);
        let mut rng = crate::dsp::Rng::new(v.get("seed").and_then(|x| x.as_u64()).unwrap_or(1));
        let ids: Vec<String> = p.modules().map(|m| m.id.clone()).collect();
        for id in ids {
            let md = p.module_mut(&id).unwrap();
            let spec = crate::fx::spec(&md.kind).unwrap();
            for ps in spec.params {
                if ps.key == "seed" || ps.unit == "bool" || ps.unit.starts_with("enum:") {
                    continue;
                }
                let cur = md.params.get(ps.key).copied().unwrap_or(ps.def);
                let span = ps.max - ps.min;
                let x = (cur + rng.bi() * amount * span * 0.5).clamp(ps.min, ps.max);
                md.params.insert(
                    ps.key.to_string(),
                    if ps.unit == "int" { x.round() } else { x },
                );
            }
        }
        Ok(json!({"amount": amount}))
    })
}

fn source_of(v: &Value) -> Result<Source> {
    let dev = os(v, "source_device");
    Ok(match os(v, "source").as_deref().unwrap_or("system") {
        "system" => Source::System(dev),
        // the Flux speaker; h_start turns it into the device to record
        "flux" => Source::System(None),
        "input" | "mic" => Source::Input(dev),
        "demo-music" => Source::DemoMusic,
        "demo-voice" => Source::DemoVoice,
        o => bail!(err(
            "bad_params",
            format!("unknown source `{o}`. Use flux, system, input, demo-music or demo-voice.")
        )),
    })
}

fn route_of(v: &Value) -> Result<String> {
    let r = req(v, "route")?;
    if r != "output" && r != "input" {
        bail!(err(
            "bad_params",
            "route must be `output` (sound from the computer) or `input` (a microphone)"
        ));
    }
    Ok(r)
}

fn remember(api: &Api, route: &str, v: &Value) {
    let path = state_path(api);
    let mut st: Value = std::fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or(json!({}));
    if !st.is_object() {
        st = json!({});
    }
    let e = st[route].as_object().cloned().unwrap_or_default();
    let mut e = Value::Object(e);
    for k in ["preset", "source", "source_device", "output"] {
        if let Some(x) = v.get(k) {
            if !x.is_null() {
                e[k] = x.clone();
            }
        }
    }
    st[route] = e;
    let _ = crate::store::atomic_write(&path, st.to_string().as_bytes());
}

pub fn h_start(api: &Api, v: &Value) -> Result<Value> {
    let route = route_of(v)?;
    let preset_id = os(v, "preset").unwrap_or_else(|| {
        if route == "input" {
            "in-clean".into()
        } else {
            "out-8d-orbit".into()
        }
    });
    let p = get(api, &preset_id)?;
    let source = match v.get("source") {
        Some(_) => source_of(v)?,
        None => {
            if route == "input" {
                Source::Input(os(v, "source_device"))
            } else {
                Source::System(os(v, "source_device"))
            }
        }
    };
    // stop the old route first so the devices are free
    let old = api.lock().live.routes.remove(&route);
    if let Some(r) = old {
        r.stop();
    }
    if route == "output" {
        crate::vdev::restore(api);
    }
    let (source, output, take) = virtual_devices(api, &route, v, source)?;
    let r = live::start(
        StartOpts {
            route: route.clone(),
            source,
            output,
        },
        &p,
    )?;
    if take {
        // Apps that follow the system output now play into the Flux speaker; the old output comes back on stop.
        match crate::vdev::take_default() {
            Ok(old) => api.lock().live.restore_sink = old,
            Err(e) => {
                r.stop();
                return Err(e);
            }
        }
    }
    let st = r.status();
    api.lock().live.routes.insert(route.clone(), r);
    remember(
        api,
        &route,
        &json!({"preset": preset_id, "source": v.get("source"), "source_device": v.get("source_device"), "output": v.get("output")}),
    );
    Ok(st)
}

/// Turn the Flux speaker (`source: flux`) and the Flux microphone (`output: flux`) into real devices: what to
/// record, where to play, and whether to make the Flux speaker the system output while the route runs (Linux).
pub fn virtual_devices(
    api: &Api,
    route: &str,
    v: &Value,
    source: Source,
) -> Result<(Source, Option<String>, bool)> {
    use crate::vdev;
    let output = os(v, "output");
    let speaker = route == "output" && os(v, "source").as_deref() == Some("flux");
    let mic = route == "input" && output.as_deref() == Some("flux");
    if !speaker && !mic {
        return Ok((source, output, false));
    }
    let (mut f, mut d) = vdev::found();
    let missing = if speaker {
        f.speaker.is_none()
    } else {
        f.mic.is_none()
    };
    if missing && vdev::can_create() {
        vdev::create()?;
        (f, d) = vdev::found();
    }
    let what = if speaker { vdev::SPEAKER } else { vdev::MIC };
    let pair = if speaker {
        f.speaker.clone()
    } else {
        f.mic.clone()
    };
    let pair = pair.ok_or_else(|| {
        err(
            "no_virtual_device",
            format!("There is no {what} on this computer yet. Install a virtual audio cable and Flux uses it (live.virtual says more)."),
        )
    })?;
    if f.shared {
        let other = if speaker { "input" } else { "output" };
        if api.lock().live.routes.contains_key(other) {
            bail!(err(
                "busy",
                format!("{} is both the Flux speaker and the Flux microphone, so only one can run at a time. Stop the other route, or install a second virtual cable.", pair.label),
            ));
        }
    }
    if mic {
        return Ok((source, Some(pair.play.id), false));
    }
    // The Flux speaker: record its other side, and play somewhere real.
    let real = |id: &str| !vdev::is_virtual(&f, id);
    if let Some(o) = output.as_deref().filter(|o| *o != "flux") {
        if o != "none" && !real(o) {
            bail!(err(
                "feedback",
                "That output is a Flux device, so Flux would hear itself. Pick your headphones or speakers.",
            ));
        }
        return Ok((Source::Input(Some(pair.rec.id)), output, false));
    }
    let own = pair.kind == "flux" && vdev::can_create();
    // Linux: the output the system uses now; elsewhere the default output, unless it is the cable itself.
    let now = if own {
        vdev::default_sink()
            .filter(|s| s != "flux_speaker")
            .and_then(|s| {
                d.outputs
                    .iter()
                    .find(|o| vdev::sink_of(&o.id) == s)
                    .map(|o| o.id.clone())
            })
    } else {
        None
    };
    let out = now
        .or_else(|| {
            d.outputs
                .iter()
                .find(|o| o.default && real(&o.id))
                .map(|o| o.id.clone())
        })
        .or_else(|| {
            d.outputs
                .iter()
                .find(|o| real(&o.id) && !o.name.to_lowercase().contains("flux"))
                .map(|o| o.id.clone())
        })
        .ok_or_else(|| {
            err(
                "no_device",
                "Flux needs headphones or speakers to play the processed sound on, and found none besides the Flux devices.",
            )
        })?;
    Ok((Source::Input(Some(pair.rec.id)), Some(out), own))
}

pub fn h_stop(api: &Api, v: &Value) -> Result<Value> {
    let route = route_of(v)?;
    if route == "output" {
        crate::vdev::restore(api);
    }
    let r = api.lock().live.routes.remove(&route);
    match r {
        Some(r) => {
            r.stop();
            Ok(json!({"stopped": route}))
        }
        None => Ok(json!({"stopped": null})),
    }
}

pub fn h_set(api: &Api, v: &Value) -> Result<Value> {
    let route = route_of(v)?;
    let p = match os(v, "preset") {
        Some(id) => Some(get(api, &id)?),
        None => None,
    };
    let mut s = api.lock();
    let r = s.live.routes.get_mut(&route).ok_or_else(|| {
        err(
            "not_running",
            format!("The {route} route isn't running. Call live.start first."),
        )
    })?;
    if let Some(p) = &p {
        r.update(p);
    }
    if let Some(b) = ob(v, "bypass") {
        r.set_bypass(b);
    }
    let st = r.status();
    drop(s);
    if let Some(p) = p {
        remember(api, &route, &json!({"preset": p.id}));
    }
    Ok(st)
}

pub fn h_status(api: &Api, _: &Value) -> Result<Value> {
    let s = api.lock();
    let mut routes = serde_json::Map::new();
    for name in ["output", "input"] {
        routes.insert(
            name.into(),
            s.live
                .routes
                .get(name)
                .map(|r| r.status())
                .unwrap_or(Value::Null),
        );
    }
    drop(s);
    let remembered: Value = std::fs::read(state_path(api))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or(json!({}));
    Ok(json!({"routes": routes, "capture": live::capture_support(), "last": remembered}))
}

pub fn h_devices(_: &Api, _: &Value) -> Result<Value> {
    // the same audio host Live records and plays with (PulseAudio on Linux)
    let d = crate::vdev::devices();
    let dv = |x: &Vec<crate::device::DeviceInfo>| {
        x.iter()
            .map(|i| crate::vdev::Dev {
                id: i.id.clone(),
                name: i.name.clone(),
            })
            .collect::<Vec<_>>()
    };
    let f = crate::vdev::detect(&dv(&d.outputs), &dv(&d.inputs));
    let loopback = !cfg!(target_os = "linux");
    let outputs: Vec<Value> = d.outputs.iter().map(|o| json!({"id": o.id, "name": o.name, "default": o.default, "channels": o.channels, "sample_rate": o.sample_rate, "capture": loopback, "virtual": crate::vdev::is_virtual(&f, &o.id)})).collect();
    let inputs: Vec<Value> = d.inputs.iter().map(|i| json!({"id": i.id, "name": i.name, "default": i.default, "channels": i.channels, "sample_rate": i.sample_rate, "monitor": i.name.to_lowercase().contains("monitor"), "virtual": crate::vdev::is_virtual(&f, &i.id)})).collect();
    Ok(
        json!({"host": d.host, "outputs": outputs, "inputs": inputs, "capture": live::capture_support(), "flux": crate::vdev::summary(&f)}),
    )
}

/// Render a preset offline: through a test source or an audio file, to a WAV, with measurements.
pub fn h_render(api: &Api, v: &Value) -> Result<Value> {
    let p = get(api, &req(v, "preset")?)?;
    let seconds = of(v, "seconds").unwrap_or(8.0).clamp(0.5, 120.0);
    let sr = 48000.0;
    let src = os(v, "source").unwrap_or_else(|| {
        if p.kind == "input" {
            "demo-voice".into()
        } else {
            "demo-music".into()
        }
    });
    let (il, ir, ol, or) = match src.as_str() {
        "demo-music" | "demo-voice" => {
            let mut d = live::Demo::new(src == "demo-voice", sr);
            live::render(&p, sr, &mut |l, r| d.fill(l, r), seconds)
        }
        path => {
            let m = crate::media::decode_file(std::path::Path::new(path))
                .map_err(|e| err("bad_file", format!("can't read that audio: {e:#}")))?;
            let a = crate::dsp::resample(&m.data[0], m.rate, 48000);
            let b = m
                .data
                .get(1)
                .map(|c| crate::dsp::resample(c, m.rate, 48000))
                .unwrap_or_else(|| a.clone());
            let mut at = 0;
            live::render(
                &p,
                sr,
                &mut |l, r| {
                    for k in 0..l.len() {
                        l[k] = *a.get(at).unwrap_or(&0.0);
                        r[k] = *b.get(at).unwrap_or(&0.0);
                        at += 1;
                    }
                },
                seconds.min(a.len() as f32 / sr + 0.5),
            )
        }
    };
    let db = |x: f32| (crate::dsp::lin_to_db(x.max(1e-9)) * 10.0).round() / 10.0;
    let peak = |a: &[f32], b: &[f32]| a.iter().chain(b).fold(0f32, |m, x| m.max(x.abs()));
    let rms = |a: &[f32], b: &[f32]| {
        (a.iter().chain(b).map(|x| x * x).sum::<f32>() / (a.len() + b.len()).max(1) as f32).sqrt()
    };
    let path = match os(v, "path") {
        Some(p) => PathBuf::from(p),
        None => {
            let d = api
                .lock()
                .settings
                .export_dir
                .clone()
                .map(PathBuf::from)
                .unwrap_or_else(crate::store::default_export_dir);
            std::fs::create_dir_all(&d)?;
            d.join(format!("Live preview - {}.wav", slug(&p.name)))
        }
    };
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: sr as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w = hound::WavWriter::create(&path, spec)?;
    for k in 0..ol.len() {
        w.write_sample(ol[k])?;
        w.write_sample(or[k])?;
    }
    w.finalize()?;
    Ok(json!({
        "path": path.display().to_string(), "seconds": ol.len() as f32 / sr, "source": src,
        "in_peak_db": db(peak(&il, &ir)), "in_rms_db": db(rms(&il, &ir)),
        "out_peak_db": db(peak(&ol, &or)), "out_rms_db": db(rms(&ol, &or)),
        "changed": il.iter().zip(&ol).any(|(a, b)| (a - b).abs() > 1e-4),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_preset_is_valid_unique_and_renders_cleanly() {
        let all = builtin();
        assert!(all.iter().filter(|p| p.kind == "output").count() >= 15);
        assert!(all.iter().filter(|p| p.kind == "input").count() >= 15);
        let mut ids = std::collections::HashSet::new();
        for p in &all {
            assert!(ids.insert(p.id.clone()), "duplicate id {}", p.id);
            let mut q = p.clone();
            q.validate().unwrap_or_else(|e| panic!("{}: {e:#}", p.id));
            assert_eq!(&q.modules().count(), &p.modules().count());
            let mut d = live::Demo::new(p.kind == "input", 48000.0);
            let (il, _, ol, or) = live::render(&q, 48000.0, &mut |l, r| d.fill(l, r), 3.0);
            assert!(
                ol.iter()
                    .chain(&or)
                    .all(|v| v.is_finite() && v.abs() <= 1.0),
                "{}",
                p.id
            );
            let e = |x: &[f32]| x.iter().map(|v| v * v).sum::<f32>();
            assert!(e(&ol) + e(&or) > 1e-3, "{} is silent", p.id);
            assert!(
                il.iter().zip(&ol).any(|(a, b)| (a - b).abs() > 1e-3),
                "{} changes nothing",
                p.id
            );
        }
    }

    #[test]
    fn builtin_presets_play_a_little_quieter_than_the_input() {
        for p in builtin() {
            let mut q = p.clone();
            q.validate().unwrap();
            let mut d = live::Demo::new(p.kind == "input", 48000.0);
            let (il, ir, ol, or) = live::render(&q, 48000.0, &mut |l, r| d.fill(l, r), 12.0);
            let delta = crate::analysis::loudness(&ol, &or, 48000.0).integrated
                - crate::analysis::loudness(&il, &ir, 48000.0).integrated;
            assert!(
                (-4.0..=-2.0).contains(&delta),
                "{} plays {delta:+.1} LU against its input",
                p.id
            );
        }
        assert_eq!(
            LEVEL.len(),
            presets().len(),
            "every built-in preset has a measured level"
        );
    }

    #[test]
    fn builtin_macro_targets_all_resolve() {
        for p in builtin() {
            for m in &p.macros {
                assert!(
                    !m.targets.is_empty(),
                    "{} macro {} does nothing",
                    p.id,
                    m.id
                );
            }
            for m in &p.mods {
                assert!(
                    !m.targets.is_empty(),
                    "{} modulator {} does nothing",
                    p.id,
                    m.id
                );
            }
        }
    }
}
