//! Effect registry and the processor chain. The registry is the single source of truth for
//! every effect kind and parameter: the UI, the MCP tool schemas and the renderer all read it.

pub mod character;
pub mod clean;
pub mod dynamics;
pub mod eq;
pub mod glitch;
pub mod pitch;
pub mod space;
pub mod spatial;
pub mod tips;

use serde::Serialize;

pub const MAXP: usize = 32;
pub type P = [f32; MAXP];
pub const BLOCK: usize = 512;

/// Context for one processing block.
#[derive(Clone, Copy, Debug)]
pub struct Ctx {
    pub sr: f32,
    /// Timeline sample at the start of the block.
    pub pos: i64,
    /// Project length in samples.
    pub len: i64,
    pub tempo: f32,
}

pub trait Processor: Send {
    fn set(&mut self, p: &P);
    fn process(&mut self, l: &mut [f32], r: &mut [f32], ctx: &Ctx);
    fn latency(&self) -> usize {
        0
    }
    fn reset(&mut self) {}
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct ParamSpec {
    pub key: &'static str,
    pub label: &'static str,
    pub min: f32,
    pub max: f32,
    pub def: f32,
    /// Unit. Special values: "bool", "enum:a|b|c" (value is the index), "int".
    pub unit: &'static str,
    pub log: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct FxSpec {
    pub kind: &'static str,
    pub name: &'static str,
    pub group: &'static str,
    pub master_only: bool,
    pub default_mix: f32,
    pub params: &'static [ParamSpec],
}

macro_rules! p {
    ($k:expr, $l:expr, $min:expr, $max:expr, $d:expr, $u:expr) => {
        ParamSpec {
            key: $k,
            label: $l,
            min: $min as f32,
            max: $max as f32,
            def: $d as f32,
            unit: $u,
            log: false,
        }
    };
    ($k:expr, $l:expr, $min:expr, $max:expr, $d:expr, $u:expr, log) => {
        ParamSpec {
            key: $k,
            label: $l,
            min: $min as f32,
            max: $max as f32,
            def: $d as f32,
            unit: $u,
            log: true,
        }
    };
}
macro_rules! fx {
    ($k:expr, $n:expr, $g:expr, $mix:expr, [$($p:expr),* $(,)?]) => {
        FxSpec { kind: $k, name: $n, group: $g, master_only: false, default_mix: $mix, params: &[$($p),*] }
    };
}

const BT: &str = "enum:bell|lowshelf|highshelf|lowpass|highpass|notch";
const NOTES: &str = "enum:C|C#|D|D#|E|F|F#|G|G#|A|A#|B";

pub static SPECS: &[FxSpec] = &[
    fx!(
        "gain",
        "Gain",
        "Utility",
        1.0,
        [
            p!("gain", "Gain", -36, 24, 0, "dB"),
            p!("pan", "Pan", -100, 100, 0, "%")
        ]
    ),
    fx!(
        "eq",
        "Parametric EQ",
        "Tone",
        1.0,
        [
            p!("b1_type", "Band 1 type", 0, 5, 1, BT),
            p!("b1_freq", "Band 1 frequency", 20, 20000, 70, "Hz", log),
            p!("b1_gain", "Band 1 gain", -18, 18, 0, "dB"),
            p!("b1_q", "Band 1 Q", 0.1, 10, 0.8, "", log),
            p!("b2_type", "Band 2 type", 0, 5, 0, BT),
            p!("b2_freq", "Band 2 frequency", 20, 20000, 280, "Hz", log),
            p!("b2_gain", "Band 2 gain", -18, 18, 0, "dB"),
            p!("b2_q", "Band 2 Q", 0.1, 10, 1.3, "", log),
            p!("b3_type", "Band 3 type", 0, 5, 0, BT),
            p!("b3_freq", "Band 3 frequency", 20, 20000, 1200, "Hz", log),
            p!("b3_gain", "Band 3 gain", -18, 18, 0, "dB"),
            p!("b3_q", "Band 3 Q", 0.1, 10, 1.0, "", log),
            p!("b4_type", "Band 4 type", 0, 5, 0, BT),
            p!("b4_freq", "Band 4 frequency", 20, 20000, 3400, "Hz", log),
            p!("b4_gain", "Band 4 gain", -18, 18, 0, "dB"),
            p!("b4_q", "Band 4 Q", 0.1, 10, 1.5, "", log),
            p!("b5_type", "Band 5 type", 0, 5, 2, BT),
            p!("b5_freq", "Band 5 frequency", 20, 20000, 11000, "Hz", log),
            p!("b5_gain", "Band 5 gain", -18, 18, 0, "dB"),
            p!("b5_q", "Band 5 Q", 0.1, 10, 0.7, "", log),
            p!("output", "Output", -24, 24, 0, "dB"),
        ]
    ),
    fx!(
        "geq",
        "Graphic EQ",
        "Tone",
        1.0,
        [
            p!("g31", "31 Hz", -12, 12, 0, "dB"),
            p!("g63", "63 Hz", -12, 12, 0, "dB"),
            p!("g125", "125 Hz", -12, 12, 0, "dB"),
            p!("g250", "250 Hz", -12, 12, 0, "dB"),
            p!("g500", "500 Hz", -12, 12, 0, "dB"),
            p!("g1k", "1 kHz", -12, 12, 0, "dB"),
            p!("g2k", "2 kHz", -12, 12, 0, "dB"),
            p!("g4k", "4 kHz", -12, 12, 0, "dB"),
            p!("g8k", "8 kHz", -12, 12, 0, "dB"),
            p!("g16k", "16 kHz", -12, 12, 0, "dB"),
        ]
    ),
    fx!(
        "compressor",
        "Compressor",
        "Dynamics",
        1.0,
        [
            p!("threshold", "Threshold", -60, 0, -18, "dB"),
            p!("ratio", "Ratio", 1, 20, 3.2, ":1", log),
            p!("attack", "Attack", 0.1, 200, 12, "ms", log),
            p!("release", "Release", 10, 2000, 180, "ms", log),
            p!("knee", "Knee", 0, 24, 6, "dB"),
            p!("makeup", "Makeup", 0, 24, 0, "dB"),
            p!("lookahead", "Lookahead", 0, 10, 0, "ms"),
            p!("auto_makeup", "Auto makeup", 0, 1, 1, "bool"),
            p!("sc_lowcut", "Sidechain low cut", 20, 500, 20, "Hz", log),
        ]
    ),
    fx!(
        "limiter",
        "Limiter",
        "Dynamics",
        1.0,
        [
            p!("ceiling", "Ceiling", -12, 0, -1, "dB"),
            p!("input", "Input gain", -24, 24, 0, "dB"),
            p!("release", "Release", 1, 1000, 80, "ms", log),
            p!("lookahead", "Lookahead", 0.5, 10, 3, "ms")
        ]
    ),
    fx!(
        "gate",
        "Gate",
        "Dynamics",
        1.0,
        [
            p!("threshold", "Threshold", -90, 0, -50, "dB"),
            p!("range", "Range", -90, 0, -60, "dB"),
            p!("attack", "Attack", 0.1, 50, 1, "ms", log),
            p!("hold", "Hold", 0, 500, 50, "ms"),
            p!("release", "Release", 5, 2000, 150, "ms", log)
        ]
    ),
    fx!(
        "deesser",
        "De-esser",
        "Tone",
        1.0,
        [
            p!("freq", "Frequency", 2000, 12000, 6800, "Hz", log),
            p!("threshold", "Threshold", -60, 0, -30, "dB"),
            p!("amount", "Max reduction", 0, 24, 6, "dB")
        ]
    ),
    fx!(
        "transient",
        "Transient shaper",
        "Dynamics",
        1.0,
        [
            p!("attack", "Attack", -100, 100, 0, "%"),
            p!("sustain", "Sustain", -100, 100, 0, "%")
        ]
    ),
    fx!(
        "leveler",
        "Leveler",
        "Dynamics",
        1.0,
        [
            p!("target", "Target", -40, -6, -20, "dB"),
            p!("max_gain", "Max gain", 0, 24, 12, "dB"),
            p!("speed", "Speed", 100, 5000, 1000, "ms", log)
        ]
    ),
    fx!(
        "multiband",
        "Multiband",
        "Dynamics",
        1.0,
        [
            p!("low_x", "Low crossover", 40, 500, 200, "Hz", log),
            p!("high_x", "High crossover", 1000, 10000, 3000, "Hz", log),
            p!("threshold", "Threshold", -60, 0, -20, "dB"),
            p!("ratio", "Ratio", 1, 10, 2.5, ":1", log),
            p!("attack", "Attack", 0.5, 100, 10, "ms", log),
            p!("release", "Release", 20, 1000, 150, "ms", log),
            p!("low", "Low gain", -12, 12, 0, "dB"),
            p!("mid", "Mid gain", -12, 12, 0, "dB"),
            p!("high", "High gain", -12, 12, 0, "dB"),
        ]
    ),
    fx!(
        "reverb",
        "Reverb",
        "Space",
        0.3,
        [
            p!("decay", "Decay", 0.1, 12, 3.2, "s", log),
            p!("predelay", "Predelay", 0, 250, 24, "ms"),
            p!("damping", "Damping", 1000, 20000, 6000, "Hz", log),
            p!("lowcut", "Low cut", 20, 1000, 120, "Hz", log),
            p!("width", "Width", 0, 100, 100, "%"),
            p!("size", "Size", 0.3, 1.5, 1.0, ""),
            p!("reverse", "Reverse", 0, 1, 0, "bool"),
        ]
    ),
    fx!(
        "delay",
        "Delay",
        "Space",
        0.3,
        [
            p!("time", "Time", 1, 2000, 350, "ms", log),
            p!("feedback", "Feedback", 0, 95, 35, "%"),
            p!("pingpong", "Ping-pong", 0, 1, 0, "bool"),
            p!("tone", "Tone", 500, 20000, 8000, "Hz", log)
        ]
    ),
    fx!(
        "convolver",
        "Impulse response",
        "Space",
        0.3,
        [p!("gain", "Gain", -24, 12, 0, "dB")]
    ),
    FxSpec {
        kind: "speed",
        name: "Speed",
        group: "Time and pitch",
        master_only: true,
        default_mix: 1.0,
        params: &[
            p!("rate", "Speed", 0.5, 2, 1, "x", log),
            p!("keep_pitch", "Keep pitch", 0, 1, 0, "bool"),
            p!("formant", "Preserve formants", 0, 1, 1, "bool"),
            p!("mode", "Mode", 0, 2, 0, "enum:music|voice|varispeed"),
        ],
    },
    fx!(
        "pitch",
        "Pitch",
        "Time and pitch",
        1.0,
        [
            p!("semitones", "Pitch", -12, 12, 0, "st"),
            p!("cents", "Fine", -100, 100, 0, "cents"),
            p!("formant", "Preserve formants", 0, 1, 1, "bool")
        ]
    ),
    fx!(
        "spatial",
        "3D and 8D",
        "Space",
        1.0,
        [
            p!(
                "mode",
                "Movement",
                0,
                5,
                0,
                "enum:orbit|spiral|eight|swarm|place|path"
            ),
            p!("rate", "Rotation speed", 0, 2, 0.1, "Hz"),
            p!(
                "direction",
                "Direction",
                0,
                2,
                0,
                "enum:clockwise|counter|pingpong"
            ),
            p!("distance", "Distance", 0.3, 10, 1.8, "m", log),
            p!("elevation", "Elevation", -90, 90, 0, "deg"),
            p!("depth", "Depth", 0, 100, 80, "%"),
            p!("smoothing", "Smoothing", 0, 100, 60, "%"),
            p!("azimuth", "Position", -180, 180, 0, "deg"),
            p!("bass_mono", "Keep bass centered", 0, 1, 1, "bool"),
            p!("bass_freq", "Bass below", 40, 300, 120, "Hz", log),
            p!("doppler", "Doppler", 0, 1, 0, "bool"),
            p!("input", "Treat stereo as", 0, 1, 0, "enum:speakers|midside"),
            p!("head_width", "Head width", 12, 22, 17.5, "cm"),
            p!("room", "Room", 0, 100, 0, "%"),
            p!("lite", "Lite renderer", 0, 1, 0, "bool"),
        ]
    ),
    fx!(
        "noise",
        "Noise reduction",
        "Fix",
        1.0,
        [
            p!("amount", "Amount", 0, 40, 18, "dB"),
            p!("sensitivity", "Sensitivity", 0, 100, 45, "%"),
            p!("smoothing", "Smoothing", 0, 500, 120, "ms"),
            p!("keep", "Keep room tone", -60, 0, -24, "dB"),
            p!("listen", "Listen to what is removed", 0, 1, 0, "bool")
        ]
    ),
    fx!(
        "hum",
        "Hum removal",
        "Fix",
        1.0,
        [
            p!("freq", "Mains", 0, 2, 0, "enum:auto|50 Hz|60 Hz"),
            p!("harmonics", "Harmonics", 1, 10, 6, "int"),
            p!("q", "Narrowness", 2, 100, 30, "", log)
        ]
    ),
    fx!(
        "declick",
        "De-click",
        "Fix",
        1.0,
        [p!("sensitivity", "Sensitivity", 0, 100, 50, "%")]
    ),
    fx!(
        "declip",
        "De-clip",
        "Fix",
        1.0,
        [p!("threshold", "Clip level", 50, 100, 98, "%")]
    ),
    fx!(
        "dewind",
        "De-wind",
        "Fix",
        1.0,
        [p!("amount", "Amount", 0, 100, 60, "%")]
    ),
    fx!(
        "stereo",
        "Stereo tools",
        "Fix",
        1.0,
        [
            p!("width", "Width", 0, 200, 100, "%"),
            p!("balance", "Balance", -100, 100, 0, "%"),
            p!("swap", "Swap left and right", 0, 1, 0, "bool"),
            p!("mono", "Mono", 0, 1, 0, "bool"),
            p!("invert_l", "Invert left", 0, 1, 0, "bool"),
            p!("invert_r", "Invert right", 0, 1, 0, "bool")
        ]
    ),
    fx!(
        "lofi",
        "Lo-fi",
        "Character",
        1.0,
        [
            p!("amount", "Amount", 0, 100, 64, "%"),
            p!("lowpass", "Low-pass", 1000, 20000, 6400, "Hz", log),
            p!("highpass", "High-pass", 20, 500, 90, "Hz", log),
            p!("drive", "Drive", 0, 24, 4, "dB"),
            p!("bits", "Bit depth", 4, 16, 12, "bit"),
            p!("rate", "Sample rate", 4000, 48000, 22000, "Hz", log),
            p!("crackle", "Crackle", 0, 20, 4, "/s"),
            p!("surface", "Surface", -80, -20, -48, "dB"),
            p!("wow", "Wow", 0, 2, 0.6, "Hz"),
            p!("flutter", "Flutter", 0, 12, 7.5, "Hz"),
            p!("pump", "Pump", 0, 100, 35, "%"),
            p!("room", "Room", 0, 3, 1.1, "s"),
            p!("noise", "Noise", -80, -20, -54, "dB"),
            p!("seed", "Seed", 0, 9999, 4821, "int"),
        ]
    ),
    fx!(
        "vinyl",
        "Vinyl",
        "Character",
        1.0,
        [
            p!("crackle", "Crackle", 0, 20, 4, "/s"),
            p!("pops", "Pops", 0, 5, 0.6, "/s"),
            p!("surface", "Surface", -80, -20, -48, "dB"),
            p!("wow", "Wow", 0, 2, 0.5, "Hz"),
            p!("flutter", "Flutter", 0, 12, 6, "Hz"),
            p!("warp", "Warp", 0, 100, 10, "%"),
            p!("rolloff", "Roll-off", 2000, 20000, 11000, "Hz", log),
            p!("mono_lows", "Mono lows", 20, 500, 200, "Hz", log),
            p!("seed", "Seed", 0, 9999, 4821, "int"),
        ]
    ),
    fx!(
        "cassette",
        "Cassette",
        "Character",
        1.0,
        [
            p!("hiss", "Hiss", -80, -30, -56, "dB"),
            p!("wow", "Wow", 0, 2, 0.6, "Hz"),
            p!("flutter", "Flutter", 0, 12, 7, "Hz"),
            p!("saturation", "Saturation", 0, 100, 40, "%"),
            p!("rolloff", "Roll-off", 4000, 18000, 12000, "Hz", log),
            p!("seed", "Seed", 0, 9999, 4821, "int")
        ]
    ),
    fx!(
        "radio",
        "Old radio",
        "Character",
        1.0,
        [
            p!("low", "Low edge", 100, 800, 400, "Hz", log),
            p!("high", "High edge", 1500, 6000, 3000, "Hz", log),
            p!("drive", "Drive", 0, 100, 40, "%"),
            p!("noise", "Static", -80, -20, -46, "dB"),
            p!("seed", "Seed", 0, 9999, 77, "int")
        ]
    ),
    fx!(
        "phone",
        "Phone call",
        "Character",
        1.0,
        [
            p!("low", "Low edge", 200, 600, 300, "Hz", log),
            p!("high", "High edge", 2500, 4000, 3400, "Hz", log),
            p!("drive", "Drive", 0, 100, 20, "%")
        ]
    ),
    fx!(
        "megaphone",
        "Megaphone",
        "Character",
        1.0,
        [
            p!("drive", "Drive", 0, 100, 70, "%"),
            p!("tone", "Tone", 500, 3000, 1500, "Hz", log)
        ]
    ),
    fx!(
        "walkie",
        "Walkie-talkie",
        "Character",
        1.0,
        [
            p!("drive", "Drive", 0, 100, 50, "%"),
            p!("squelch", "Squelch", 0, 1, 1, "bool"),
            p!("noise", "Noise", -80, -20, -40, "dB"),
            p!("seed", "Seed", 0, 9999, 12, "int")
        ]
    ),
    fx!(
        "underwater",
        "Underwater",
        "Character",
        1.0,
        [
            p!("depth", "Depth", 0, 100, 60, "%"),
            p!("wobble", "Wobble", 0, 100, 40, "%")
        ]
    ),
    fx!(
        "distortion",
        "Distortion",
        "Character",
        1.0,
        [
            p!("drive", "Drive", 0, 48, 12, "dB"),
            p!("tone", "Tone", 500, 12000, 4000, "Hz", log),
            p!("output", "Output", -24, 0, -6, "dB"),
            p!("type", "Type", 0, 2, 0, "enum:soft|hard|fuzz")
        ]
    ),
    fx!(
        "bitcrusher",
        "Bitcrusher",
        "Character",
        1.0,
        [
            p!("bits", "Bits", 2, 16, 8, "bit"),
            p!("downsample", "Downsample", 1, 32, 4, "x")
        ]
    ),
    fx!(
        "chorus",
        "Chorus",
        "Character",
        0.5,
        [
            p!("rate", "Rate", 0.05, 5, 0.8, "Hz", log),
            p!("depth", "Depth", 0, 100, 40, "%"),
            p!("delay", "Delay", 5, 30, 15, "ms"),
            p!("voices", "Voices", 1, 4, 2, "int")
        ]
    ),
    fx!(
        "flanger",
        "Flanger",
        "Character",
        0.5,
        [
            p!("rate", "Rate", 0.05, 5, 0.25, "Hz", log),
            p!("depth", "Depth", 0, 100, 70, "%"),
            p!("feedback", "Feedback", -95, 95, 50, "%"),
            p!("delay", "Delay", 0.5, 10, 2, "ms")
        ]
    ),
    fx!(
        "phaser",
        "Phaser",
        "Character",
        0.5,
        [
            p!("rate", "Rate", 0.05, 5, 0.4, "Hz", log),
            p!("depth", "Depth", 0, 100, 70, "%"),
            p!("stages", "Stages", 2, 12, 6, "int"),
            p!("feedback", "Feedback", 0, 90, 40, "%")
        ]
    ),
    fx!(
        "rotary",
        "Rotary",
        "Character",
        1.0,
        [
            p!("speed", "Speed", 0, 1, 0, "enum:slow|fast"),
            p!("depth", "Depth", 0, 100, 70, "%")
        ]
    ),
    fx!(
        "stutter",
        "Stutter",
        "Glitch",
        1.0,
        [
            p!(
                "division",
                "Division",
                0,
                4,
                2,
                "enum:1/4|1/8|1/16|1/32|1/64"
            ),
            p!("probability", "Probability", 0, 100, 72, "%"),
            p!("pitch_ramp", "Pitch ramp", -12, 12, 3, "st"),
            p!("gate", "Gate", 0, 100, 80, "%"),
            p!("seed", "Seed", 0, 9999, 77, "int")
        ]
    ),
    fx!(
        "tapestop",
        "Tape stop",
        "Glitch",
        1.0,
        [
            p!("at", "At", 0, 36000, 0, "s"),
            p!("time", "Stop time", 0.1, 4, 1.4, "s"),
            p!("mode", "Mode", 0, 2, 0, "enum:stop|start|both")
        ]
    ),
    fx!(
        "granular",
        "Granular",
        "Glitch",
        1.0,
        [
            p!("grain", "Grain", 10, 500, 80, "ms", log),
            p!("spray", "Spray", 0, 100, 50, "%"),
            p!("freeze", "Freeze", 0, 1, 0, "bool"),
            p!("seed", "Seed", 0, 9999, 5, "int")
        ]
    ),
    fx!(
        "trancegate",
        "Trance gate",
        "Glitch",
        1.0,
        [
            p!("steps", "Pattern", 0, 65535, 18761, "int"),
            p!("depth", "Depth", 0, 100, 80, "%"),
            p!("division", "Division", 0, 1, 0, "enum:1/16|1/8"),
            p!("smooth", "Smoothing", 0, 20, 3, "ms")
        ]
    ),
    fx!(
        "glitch",
        "Glitch",
        "Glitch",
        1.0,
        [
            p!("chaos", "Chaos", 0, 100, 34, "%"),
            p!("seed", "Seed", 0, 9999, 77, "int")
        ]
    ),
    fx!(
        "voice",
        "Voice changer",
        "Voice",
        1.0,
        [
            p!("pitch", "Pitch", -12, 12, 0, "st"),
            p!("formant", "Formant", -100, 100, 0, "%"),
            p!("ring", "Ring mod", 0, 100, 0, "%"),
            p!("ring_freq", "Ring frequency", 1, 1000, 30, "Hz", log),
            p!("comb", "Comb", 0, 100, 0, "%"),
            p!("vocoder", "Vocoder", 0, 100, 0, "%"),
            p!(
                "carrier",
                "Vocoder carrier",
                0,
                2,
                0,
                "enum:saw|noise|chord"
            ),
            p!("bands", "Bands", 8, 32, 24, "int"),
            p!("tune", "Hard tune", 0, 1, 0, "bool"),
            p!("key", "Key", 0, 11, 2, NOTES),
            p!("scale", "Scale", 0, 2, 1, "enum:major|minor|chromatic"),
            p!("retune", "Retune speed", 1, 200, 10, "ms", log),
            p!("tremor", "Tremor", 0, 100, 0, "%"),
        ]
    ),
    fx!(
        "row10",
        "Row 10",
        "Space",
        1.0,
        [
            p!("row", "Row", 1, 40, 10, "int"),
            p!("crowd", "Crowd", -60, 0, -18, "dB"),
            p!("pa_color", "PA color", 0, 100, 30, "%"),
            p!("applause", "Applause at the start and end", 0, 1, 1, "bool"),
            p!("seed", "Seed", 0, 9999, 10, "int")
        ]
    ),
    fx!(
        "nextdoor",
        "Next door",
        "Space",
        1.0,
        [
            p!("wall", "Wall thickness", 200, 2000, 620, "Hz", log),
            p!("bass", "Bass through wall", 0, 12, 4, "dB"),
            p!("room", "Their room", 0.1, 2, 0.6, "s"),
            p!("drift", "Level drifts over time", 0, 1, 1, "bool"),
            p!("seed", "Seed", 0, 9999, 3, "int")
        ]
    ),
];

pub fn spec(kind: &str) -> Option<&'static FxSpec> {
    SPECS.iter().find(|s| s.kind == kind)
}

/// Resolve a parameter map into the fixed array, filling defaults and clamping.
pub fn resolve(spec: &FxSpec, params: &std::collections::BTreeMap<String, f32>) -> P {
    let mut p = [0.0; MAXP];
    for (i, ps) in spec.params.iter().enumerate() {
        let v = params.get(ps.key).copied().unwrap_or(ps.def);
        p[i] = if v.is_finite() {
            v.clamp(ps.min, ps.max)
        } else {
            ps.def
        };
    }
    p
}

pub fn defaults(spec: &FxSpec) -> std::collections::BTreeMap<String, f32> {
    spec.params
        .iter()
        .map(|p| (p.key.to_string(), p.def))
        .collect()
}

/// Effect-specific extra data (noise profile, impulse response, motion path).
#[derive(Clone, Default)]
pub struct Extra {
    pub profile: Option<Vec<f32>>,
    pub ir: Option<(Vec<f32>, Vec<f32>)>,
    pub path: Vec<[f32; 2]>,
}

pub fn build(kind: &str, sr: f32, extra: &Extra) -> Option<Box<dyn Processor>> {
    use self::{character as c, clean as cl, dynamics as d, glitch as g, pitch as pt, space as s};
    Some(match kind {
        "gain" => Box::new(d::Gain::default()),
        "eq" => Box::new(eq::Eq::new(sr)),
        "geq" => Box::new(eq::Geq::new(sr)),
        "compressor" => Box::new(d::Compressor::new(sr)),
        "limiter" => Box::new(d::Limiter::new(sr)),
        "gate" => Box::new(d::Gate::new(sr)),
        "deesser" => Box::new(d::DeEsser::new(sr)),
        "transient" => Box::new(d::Transient::new(sr)),
        "leveler" => Box::new(d::Leveler::new(sr)),
        "multiband" => Box::new(d::Multiband::new(sr)),
        "reverb" => Box::new(s::Reverb::new(sr)),
        "delay" => Box::new(s::DelayFx::new(sr)),
        "convolver" => Box::new(s::Convolver::new(extra.ir.clone())),
        "pitch" => Box::new(pt::PitchFx::new(sr)),
        "spatial" => Box::new(spatial::Spatial::new(sr, extra.path.clone())),
        "noise" => Box::new(cl::NoiseReduce::new(sr, extra.profile.clone())),
        "hum" => Box::new(cl::Hum::new(sr)),
        "declick" => Box::new(cl::DeClick::new(sr)),
        "declip" => Box::new(cl::DeClip::new()),
        "dewind" => Box::new(cl::DeWind::new(sr)),
        "stereo" => Box::new(cl::Stereo::default()),
        "lofi" => Box::new(c::Lofi::new(sr)),
        "vinyl" => Box::new(c::Vinyl::new(sr)),
        "cassette" => Box::new(c::Cassette::new(sr)),
        "radio" | "phone" | "megaphone" | "walkie" => Box::new(c::Band::new(sr, kind)),
        "underwater" => Box::new(c::Underwater::new(sr)),
        "distortion" => Box::new(c::Distortion::new(sr)),
        "bitcrusher" => Box::new(c::Crusher::default()),
        "chorus" | "flanger" => Box::new(c::Modulated::new(sr, kind == "flanger")),
        "phaser" => Box::new(c::Phaser::new(sr)),
        "rotary" => Box::new(c::Rotary::new(sr)),
        "stutter" => Box::new(g::Stutter::new(sr)),
        "tapestop" => Box::new(g::TapeStop::new(sr)),
        "granular" => Box::new(g::Granular::new(sr)),
        "trancegate" => Box::new(g::TranceGate::new(sr)),
        "glitch" => Box::new(g::Glitch::new(sr)),
        "voice" => Box::new(pt::Voice::new(sr)),
        "row10" => Box::new(s::Row10::new(sr)),
        "nextdoor" => Box::new(s::NextDoor::new(sr)),
        // speed is applied by the renderer, not as an in-chain processor
        "speed" => Box::new(d::Gain::default()),
        _ => return None,
    })
}

/// One effect instance inside a chain, with dry/wet mix and latency-compensated dry path.
pub struct Slot {
    pub id: u32,
    pub kind: &'static str,
    pub enabled: bool,
    pub mix: f32,
    mix_s: f32,
    pub proc: Box<dyn Processor>,
    lat: usize,
    dl: crate::dsp::Delay,
    dr: crate::dsp::Delay,
    bl: Vec<f32>,
    br: Vec<f32>,
    /// Parameter smoothing: what the processor has now and where it is heading.
    cur: P,
    target: P,
    /// Continuous parameters glide; enums, switches and integers jump.
    glide: [bool; MAXP],
    primed: bool,
}

/// Parameter glide time constant.
pub const GLIDE_MS: f32 = 20.0;
/// Samples between parameter updates while gliding.
const GLIDE_STEP: usize = 16;

impl Slot {
    pub fn new(
        id: u32,
        kind: &'static str,
        enabled: bool,
        mix: f32,
        proc: Box<dyn Processor>,
    ) -> Self {
        let lat = proc.latency();
        Slot {
            id,
            kind,
            enabled,
            mix,
            mix_s: mix,
            proc,
            lat,
            dl: crate::dsp::Delay::new(lat + 4),
            dr: crate::dsp::Delay::new(lat + 4),
            bl: vec![0.0; BLOCK],
            br: vec![0.0; BLOCK],
            cur: [0.0; MAXP],
            target: [0.0; MAXP],
            glide: spec(kind).map(glide_mask).unwrap_or([false; MAXP]),
            primed: false,
        }
    }
    /// New parameter values. The first call applies them at once; later calls glide.
    pub fn set_params(&mut self, p: &P) {
        if !self.primed {
            self.cur = *p;
            self.target = *p;
            self.primed = true;
            self.proc.set(p);
            return;
        }
        self.target = *p;
        let mut jumped = false;
        for ((c, &want), &glides) in self.cur.iter_mut().zip(p.iter()).zip(self.glide.iter()) {
            if !glides && *c != want {
                *c = want;
                jumped = true;
            }
        }
        if jumped {
            self.proc.set(&self.cur);
        }
    }
    /// The values the processor is using right now (tests).
    pub fn current(&self) -> &P {
        &self.cur
    }
    fn glide_step(&mut self, n: usize, sr: f32) {
        if self.cur == self.target {
            return;
        }
        let k = 1.0 - (-(n as f32) / (sr * GLIDE_MS * 0.001)).exp();
        for i in 0..MAXP {
            let d = self.target[i] - self.cur[i];
            if d == 0.0 {
                continue;
            }
            if d.abs() <= 1e-4 * (1.0 + self.target[i].abs()) {
                self.cur[i] = self.target[i];
            } else {
                self.cur[i] += d * k;
            }
        }
        self.proc.set(&self.cur);
    }
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32], ctx: &Ctx) {
        if !self.enabled || self.kind == "speed" {
            return;
        }
        let n = l.len();
        self.bl[..n].copy_from_slice(l);
        self.br[..n].copy_from_slice(r);
        if self.cur == self.target {
            self.proc.process(l, r, ctx);
        } else {
            // glide in short steps so the change has no audible staircase
            let mut i = 0;
            while i < n {
                let e = (i + GLIDE_STEP).min(n);
                self.glide_step(e - i, ctx.sr);
                let c = Ctx {
                    pos: ctx.pos + i as i64,
                    ..*ctx
                };
                self.proc.process(&mut l[i..e], &mut r[i..e], &c);
                i = e;
            }
        }
        let target = self.mix.clamp(0.0, 1.0);
        let full = target >= 0.999 && self.mix_s >= 0.999;
        if full && self.lat == 0 {
            return;
        }
        for i in 0..n {
            self.dl.push(self.bl[i]);
            self.dr.push(self.br[i]);
            self.mix_s += (target - self.mix_s) * 0.002;
            let m = self.mix_s;
            let (dl, dr) = (self.dl.tap(self.lat), self.dr.tap(self.lat));
            l[i] = dl + (l[i] - dl) * m;
            r[i] = dr + (r[i] - dr) * m;
        }
    }
}

fn glide_mask(s: &FxSpec) -> [bool; MAXP] {
    let mut m = [false; MAXP];
    for (i, p) in s.params.iter().enumerate().take(MAXP) {
        m[i] = !(p.unit == "bool" || p.unit == "int" || p.unit.starts_with("enum:"));
    }
    m
}

/// A chain of slots processed in order.
#[derive(Default)]
pub struct Chain {
    pub slots: Vec<Slot>,
}
impl Chain {
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32], ctx: &Ctx) {
        for s in &mut self.slots {
            s.process(l, r, ctx);
        }
    }
    pub fn find(&mut self, id: u32) -> Option<&mut Slot> {
        self.slots.iter_mut().find(|s| s.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Universal invariants that every effect must pass: finite output, silence in gives
    /// bounded output, no NaN under extreme parameters, deterministic for the same seed.
    #[test]
    fn every_effect_is_finite_bounded_and_deterministic() {
        let sr = 48000.0;
        for s in SPECS {
            for extreme in 0..3 {
                let mut p = [0.0; MAXP];
                for (i, ps) in s.params.iter().enumerate() {
                    p[i] = match extreme {
                        0 => ps.def,
                        1 => ps.min,
                        _ => ps.max,
                    };
                }
                let run = || {
                    let mut pr = build(s.kind, sr, &Extra::default()).unwrap();
                    pr.set(&p);
                    let mut rng = crate::dsp::Rng::new(1);
                    let mut out = Vec::new();
                    for b in 0..60 {
                        let mut l: Vec<f32> = (0..BLOCK).map(|_| rng.bi() * 0.5).collect();
                        let mut r = l.clone();
                        let ctx = Ctx {
                            sr,
                            pos: (b * BLOCK) as i64,
                            len: 48000 * 10,
                            tempo: 120.0,
                        };
                        pr.process(&mut l, &mut r, &ctx);
                        out.extend(l);
                        out.extend(r);
                    }
                    out
                };
                let a = run();
                assert!(
                    a.iter().all(|v| v.is_finite()),
                    "{} produced non-finite output (case {extreme})",
                    s.kind
                );
                let peak = a.iter().fold(0f32, |m, v| m.max(v.abs()));
                assert!(
                    peak < 64.0,
                    "{} exploded: peak {peak} (case {extreme})",
                    s.kind
                );
                let b = run();
                assert_eq!(a, b, "{} is not deterministic (case {extreme})", s.kind);
            }
        }
    }

    #[test]
    fn every_kind_in_registry_builds() {
        for s in SPECS {
            assert!(
                build(s.kind, 48000.0, &Extra::default()).is_some(),
                "{}",
                s.kind
            );
            assert!(s.params.len() <= MAXP);
        }
    }
}
