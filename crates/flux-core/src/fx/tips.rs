//! Tooltips for every effect and every parameter. The UI shows them on hover and focus, and MCP
//! clients read them from `fx.catalog` and `live.kinds`. A test makes sure none is missing.

/// What an effect does, in one or two plain sentences.
pub fn effect(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "gain" => "Turns the level up or down and places the sound left or right.",
        "eq" => "Five bands you can shape freely. Boost or cut any frequency, or filter the ends away.",
        "geq" => "Ten fixed bands, one slider each. Quick broad tone changes.",
        "compressor" => "Evens out loud and quiet moments by turning down whatever goes over the threshold.",
        "limiter" => "A ceiling nothing gets past. Keeps peaks from clipping, with lookahead so it reacts in time.",
        "gate" => "Mutes the signal when it falls below the threshold. Removes background noise between words.",
        "deesser" => "Softens sharp s and t sounds by turning down only the hissy band when it gets loud.",
        "transient" => "Makes the start of each hit punchier or softer, and the tail longer or shorter.",
        "leveler" => "Rides the volume slowly toward a target, like an engineer on the fader.",
        "multiband" => "A compressor split into low, mid and high bands, each with its own gain.",
        "reverb" => "Puts the sound in a room. From a small booth to a long hall.",
        "delay" => "Repeats the sound after a set time. Feedback makes more repeats.",
        "convolver" => "Reverb from a recording of a real space (an impulse response).",
        "speed" => "Changes playback speed, with or without changing pitch.",
        "pitch" => "Moves the pitch up or down without changing the speed.",
        "spatial" => "Binaural 3D sound. Moves the sound around your head. Best on headphones.",
        "noise" => "Removes steady background noise using a learned noise profile.",
        "hum" => "Removes mains hum at 50 or 60 Hz and its harmonics.",
        "declick" => "Finds and repairs short clicks and pops.",
        "declip" => "Rebuilds the tops of waves that were clipped flat.",
        "dewind" => "Reduces low rumble from wind hitting the microphone.",
        "stereo" => "Width, balance, swap, mono and polarity tools for the stereo image.",
        "lofi" => "Everything that makes a sound old and cheap: filters, crushed bits, wobble, crackle, noise.",
        "vinyl" => "A record player: crackle, pops, surface noise, warp and wow.",
        "cassette" => "A tape deck: hiss, wow and flutter, saturation and dull highs.",
        "radio" => "An AM radio: a narrow band, drive and static.",
        "phone" => "A phone call: a narrow band and a little grit.",
        "megaphone" => "A loud hailer: very narrow, honky and distorted.",
        "walkie" => "A walkie-talkie: narrow band, drive, noise and an optional squelch.",
        "underwater" => "Muffled and wobbling, as if heard from under the water.",
        "distortion" => "Overdrive, from warm saturation to hard clipping and fuzz.",
        "bitcrusher" => "Lowers bit depth and sample rate for crunchy digital grit.",
        "chorus" => "Several slightly detuned copies for a thick, shimmering sound.",
        "flanger" => "A short, sweeping delay that makes a jet-plane whoosh.",
        "phaser" => "Sweeping notches that make the sound swirl.",
        "rotary" => "A rotating speaker, slow or fast.",
        "stutter" => "Repeats tiny slices in time with the tempo.",
        "tapestop" => "Slows the sound to a halt like a tape machine losing power, or speeds it up from a stop.",
        "granular" => "Breaks the sound into tiny grains and scatters them. Freeze holds one moment forever.",
        "trancegate" => "Chops the sound on and off in a rhythm you draw.",
        "glitch" => "Random repeats, reverses and dropouts. Chaos sets how often.",
        "voice" => "A voice changer: pitch, formant, robot ring mod, comb, vocoder and hard tune.",
        "row10" => "As if heard from a seat in a concert hall, with crowd and PA color.",
        "nextdoor" => "As if the music is playing in the room next door, through the wall.",
        _ => return None,
    })
}

/// What one parameter does.
pub fn param(kind: &str, key: &str) -> Option<&'static str> {
    // Bands of the parametric EQ share their wording.
    if kind == "eq" {
        if let Some(rest) = key.strip_prefix('b') {
            let what = rest.split_once('_').map(|x| x.1).unwrap_or("");
            return Some(match what {
                "freq" => "Center (or corner) frequency of this band.",
                "gain" => {
                    "How much this band boosts or cuts. Ignored by the filter and notch shapes."
                }
                "q" => "How narrow the band is. Higher is narrower and more surgical.",
                "type" => "Band shape: bell, low or high shelf, low or high pass filter, or notch.",
                "on" => "Turns this band on or off.",
                _ => return None,
            });
        }
    }
    if kind == "geq" && key.starts_with('g') {
        return Some("Boost or cut around this frequency.");
    }
    Some(match (kind, key) {
        (_, "seed") => "Starting point for the random parts. The same seed always sounds the same.",
        (_, "mix") => "Balance between the untouched sound and the effect. 100% is all effect.",

        ("gain", "gain") => "Level change in decibels. +6 dB is about twice as loud.",
        ("gain", "pan") => "Moves the sound left (negative) or right (positive).",
        ("eq", "output") => "Overall level after the EQ, to make up for boosts or cuts.",

        ("compressor", "threshold") => "Level above which the compressor starts turning things down.",
        ("compressor", "ratio") => "How hard it turns down. 4:1 means 4 dB over the threshold comes out as 1 dB.",
        ("compressor", "attack") => "How fast it reacts once the threshold is crossed. Slow lets the punch through.",
        ("compressor", "release") => "How fast it lets go after the level drops.",
        ("compressor", "knee") => "How gently compression starts around the threshold. Wider is smoother.",
        ("compressor", "makeup") => "Gain added after compression, to win back level.",
        ("compressor", "lookahead") => "Lets it see peaks coming, so it can catch them cleanly. Adds a little delay.",
        ("compressor", "auto_makeup") => "Adds makeup gain automatically from the threshold and ratio.",
        ("compressor", "sc_lowcut") => "Ignores bass below this when deciding how much to compress, so kick and rumble don't pump.",

        ("limiter", "ceiling") => "The highest level that can come out.",
        ("limiter", "input") => "Pushes the signal into the limiter. More is louder and more limited.",
        ("limiter", "release") => "How fast the limiter recovers after a peak.",
        ("limiter", "lookahead") => "How far ahead it looks for peaks. Longer is cleaner, with more delay.",

        ("gate", "threshold") => "Below this level, the gate closes.",
        ("gate", "range") => "How much the gate turns down when closed. -90 dB is silence.",
        ("gate", "attack") => "How fast the gate opens.",
        ("gate", "hold") => "How long it stays open after the level drops, so word endings aren't cut.",
        ("gate", "release") => "How fast the gate closes.",

        ("deesser", "freq") => "Where the hiss lives. Most voices have it between 5 and 8 kHz.",
        ("deesser", "threshold") => "How loud the hiss must be before it gets turned down.",
        ("deesser", "amount") => "The most it will turn the hiss down.",

        ("transient", "attack") => "More makes the start of each hit punchier. Less softens it.",
        ("transient", "sustain") => "More brings up the tail and room. Less makes it tighter and drier.",

        ("leveler", "target") => "The level it aims for.",
        ("leveler", "max_gain") => "The most it will turn quiet parts up.",
        ("leveler", "speed") => "How quickly it moves toward the target. Slower is more natural.",

        ("multiband", "low_x") => "Where the low band ends and the mid band starts.",
        ("multiband", "high_x") => "Where the mid band ends and the high band starts.",
        ("multiband", "threshold") => "Level above which each band is compressed.",
        ("multiband", "ratio") => "How hard each band is compressed.",
        ("multiband", "attack") => "How fast each band reacts.",
        ("multiband", "release") => "How fast each band lets go.",
        ("multiband", "low") => "Level of the low band after compression.",
        ("multiband", "mid") => "Level of the mid band after compression.",
        ("multiband", "high") => "Level of the high band after compression.",

        ("reverb", "decay") => "How long the room keeps ringing, in seconds.",
        ("reverb", "predelay") => "A gap before the reverb starts. Keeps the dry sound clear in front.",
        ("reverb", "damping") => "Highs above this fade faster, like soft furnishings. Lower is darker.",
        ("reverb", "lowcut") => "Removes bass from the reverb so it doesn't get muddy.",
        ("reverb", "width") => "How wide the reverb spreads across left and right.",
        ("reverb", "size") => "Size of the room. Changes the spacing of early reflections.",
        ("reverb", "reverse") => "Plays the reverb backwards, swelling into the sound.",

        ("delay", "time") => "Time between repeats.",
        ("delay", "feedback") => "How much of each repeat feeds the next. Higher means more repeats.",
        ("delay", "pingpong") => "Bounces repeats between left and right.",
        ("delay", "tone") => "Repeats lose highs above this, like an old tape echo.",

        ("convolver", "gain") => "Level of the convolution reverb.",

        ("speed", "rate") => "Playback speed. 2x is twice as fast.",
        ("speed", "keep_pitch") => "Changes speed without changing pitch.",
        ("speed", "formant") => "Keeps voices sounding natural when the pitch changes.",
        ("speed", "mode") => "Music, voice (tuned for speech) or varispeed (pitch follows speed like a turntable).",

        ("pitch", "semitones") => "Pitch shift in semitones. 12 is one octave.",
        ("pitch", "cents") => "Fine tuning in hundredths of a semitone.",
        ("pitch", "formant") => "Keeps voices natural. Off gives the classic chipmunk or monster sound.",

        ("spatial", "mode") => "The movement: orbit, spiral, figure eight, swarm, a fixed place, or a path you draw.",
        ("spatial", "direction") => "Which way it travels: clockwise, counterclockwise, or back and forth.",
        ("stutter", "division") => "Length of each slice, in notes of the tempo.",
        ("voice", "carrier") => "What the vocoder plays through your voice: a saw synth, noise (whisper), or a chord.",
        ("voice", "key") => "Key that hard tune snaps to.",
        ("spatial", "rate") => "How fast the sound travels around your head. 0 holds it still.",
        ("spatial", "distance") => "How far away the sound seems.",
        ("spatial", "elevation") => "Above (positive) or below (negative) your ears.",
        ("spatial", "depth") => "How much of the head effect is applied. Less is subtler.",
        ("spatial", "smoothing") => "Rounds off sudden moves so the motion glides.",
        ("spatial", "azimuth") => "Where the sound sits around you when it isn't moving. 0 is in front.",
        ("spatial", "bass_mono") => "Keeps the bass in the middle so it stays solid.",
        ("spatial", "bass_freq") => "Bass below this stays in the center.",
        ("spatial", "doppler") => "Bends the pitch slightly as the sound moves toward and away from you.",
        ("spatial", "input") => "How a stereo source is placed: as two speakers, or as a center and sides.",
        ("spatial", "head_width") => "Distance between the ears. Match your head for sharper placement.",
        ("spatial", "room") => "Adds a little room so the sound seems outside your head.",
        ("spatial", "lite") => "A lighter renderer that uses less processing.",

        ("noise", "amount") => "How much noise to remove.",
        ("noise", "sensitivity") => "How eagerly it decides something is noise.",
        ("noise", "smoothing") => "Smooths the reduction over time to avoid watery artifacts.",
        ("noise", "keep") => "Leaves a little room tone so silence doesn't sound dead.",
        ("noise", "listen") => "Plays only what is being removed, so you can check nothing important goes.",

        ("hum", "freq") => "Mains frequency. Auto finds it. 50 Hz in most of the world, 60 Hz in the Americas.",
        ("hum", "harmonics") => "How many multiples of the mains frequency to remove.",
        ("hum", "q") => "How narrow each notch is. Higher touches less of the music.",

        ("declick", "sensitivity") => "How small a click it will repair.",
        ("declip", "threshold") => "Level at which waves count as clipped.",
        ("dewind", "amount") => "How much wind rumble to remove.",

        ("stereo", "width") => "0% is mono, 100% unchanged, 200% extra wide.",
        ("stereo", "balance") => "Shifts the whole image left or right.",
        ("stereo", "swap") => "Swaps the left and right channels.",
        ("stereo", "mono") => "Sums to mono. Good for checking a mix.",
        ("stereo", "invert_l") => "Flips the polarity of the left channel.",
        ("stereo", "invert_r") => "Flips the polarity of the right channel.",

        ("lofi", "amount") => "One slider for everything below, scaled together.",
        ("lofi", "lowpass") => "Removes highs above this.",
        ("lofi", "highpass") => "Removes lows below this.",
        ("lofi", "drive") => "Warm saturation before the filters.",
        ("lofi", "bits") => "Bit depth. Fewer bits are grittier.",
        ("lofi", "rate") => "Sample rate. Lower adds aliasing and a crunchy top.",
        ("lofi", "crackle") => "Vinyl crackles per second.",
        ("lofi", "surface") => "Level of the record's surface noise.",
        ("lofi", "wow") => "Slow pitch wobble, like a warped record or stretched tape.",
        ("lofi", "flutter") => "Fast pitch wobble.",
        ("lofi", "pump") => "Rhythmic ducking, like a sidechained beat.",
        ("lofi", "room") => "A small, dull room around the sound.",
        ("lofi", "noise") => "Level of tape or amp hiss.",

        ("vinyl", "crackle") => "Small crackles per second.",
        ("vinyl", "pops") => "Bigger pops per second.",
        ("vinyl", "surface") => "Level of the steady surface noise.",
        ("vinyl", "wow") => "Slow pitch wobble from an off-center record.",
        ("vinyl", "flutter") => "Fast pitch wobble from the motor.",
        ("vinyl", "warp") => "How bent the record is. Adds deeper, slower wobble.",
        ("vinyl", "rolloff") => "Highs fade above this, like a worn stylus.",
        ("vinyl", "mono_lows") => "Bass below this is made mono, as on a real record.",

        ("cassette", "hiss") => "Level of the tape hiss.",
        ("cassette", "wow") => "Slow pitch wobble from a stretched tape.",
        ("cassette", "flutter") => "Fast pitch wobble from the capstan.",
        ("cassette", "saturation") => "Tape warmth and compression when driven hard.",
        ("cassette", "rolloff") => "Highs fade above this.",

        ("radio" | "phone", "low") => "Lowest frequency that gets through.",
        ("radio" | "phone", "high") => "Highest frequency that gets through.",
        ("radio" | "phone" | "megaphone" | "walkie", "drive") => "How hard the little speaker is pushed. More is dirtier.",
        ("radio", "noise") => "Level of the static between stations.",
        ("megaphone", "tone") => "Center of the honk.",
        ("walkie", "squelch") => "Adds the burst of noise when someone stops talking.",
        ("walkie", "noise") => "Level of the radio noise.",

        ("underwater", "depth") => "How deep under the water. Deeper is more muffled.",
        ("underwater", "wobble") => "How much the sound wavers.",

        ("distortion", "drive") => "How hard the signal is pushed into distortion.",
        ("distortion", "tone") => "Brightness after distortion. Lower is darker.",
        ("distortion", "output") => "Level after distortion, to keep it from getting too loud.",
        ("distortion", "type") => "Soft is warm, hard is aggressive, fuzz is buzzy and square.",

        ("bitcrusher", "bits") => "Bit depth. 16 is clean, 4 is very crunchy.",
        ("bitcrusher", "downsample") => "Holds each sample for this many steps. Higher is harsher.",

        ("chorus", "rate") => "Speed of the shimmer.",
        ("chorus", "depth") => "How far the copies drift in pitch.",
        ("chorus", "delay") => "Base delay of the copies. Longer sounds more like a doubled part.",
        ("chorus", "voices") => "Number of copies.",
        ("flanger", "rate") => "Speed of the sweep.",
        ("flanger", "depth") => "How wide the sweep is.",
        ("flanger", "feedback") => "Makes the sweep sharper and more metallic. Negative gives a hollow tone.",
        ("flanger", "delay") => "Base delay around which it sweeps.",
        ("phaser", "rate") => "Speed of the swirl.",
        ("phaser", "depth") => "How wide the swirl is.",
        ("phaser", "stages") => "Number of notches. More is deeper and more vocal.",
        ("phaser", "feedback") => "Makes the notches sharper and more resonant.",
        ("rotary", "speed") => "Slow (chorale) or fast (tremolo) rotation.",
        ("rotary", "depth") => "How strong the rotating effect is.",

        ("stutter", "probability") => "Chance that each beat slice stutters.",
        ("stutter", "pitch_ramp") => "Bends the pitch across each stutter, up or down.",
        ("stutter", "gate") => "Shortens each repeat for a choppier sound.",
        ("tapestop", "at") => "When the stop happens, in seconds.",
        ("tapestop", "time") => "How long the slowdown takes.",
        ("tapestop", "mode") => "Stop slows to a halt, start spins up from silence, both does one then the other.",
        ("granular", "grain") => "Length of each grain.",
        ("granular", "spray") => "How randomly grains are picked from around the playhead.",
        ("granular", "freeze") => "Holds the current moment and keeps playing grains from it.",
        ("trancegate", "steps") => "The rhythm pattern. Each of the 16 bits is one step, on or off.",
        ("trancegate", "depth") => "How far the off steps turn down. 100% is silence.",
        ("trancegate", "division") => "Length of each step: a sixteenth or an eighth note.",
        ("trancegate", "smooth") => "Rounds the edges of each step to avoid clicks.",
        ("glitch", "chaos") => "How often something glitches.",

        ("voice", "pitch") => "Moves the voice up or down in semitones.",
        ("voice", "formant") => "Changes the size of the throat. Negative is bigger and deeper, positive smaller.",
        ("voice", "ring") => "Robot sound from a ring modulator.",
        ("voice", "ring_freq") => "Pitch of the robot buzz.",
        ("voice", "comb") => "Metallic, tube-like resonance.",
        ("voice", "vocoder") => "Turns the voice into a synth-like vocoder.",
        ("voice", "bands") => "Vocoder detail. More bands is clearer speech.",
        ("voice", "tune") => "Snaps the pitch to notes of the scale.",
        ("voice", "scale") => "Which notes hard tune snaps to.",
        ("voice", "retune") => "How fast it snaps. 1 ms is the robotic effect, slower sounds natural.",
        ("voice", "tremor") => "Adds a shaky, wavering quality.",

        ("row10", "row") => "Which row you sit in. Further back is more room, less direct sound.",
        ("row10", "crowd") => "Level of the audience around you.",
        ("row10", "pa_color") => "How much the PA speakers color the sound.",
        ("row10", "applause") => "Adds applause at the start and end.",
        ("nextdoor", "wall") => "How thick the wall is. Lower lets less of the highs through.",
        ("nextdoor", "bass") => "How much bass thumps through the wall.",
        ("nextdoor", "room") => "Reverb of the neighbor's room.",
        ("nextdoor", "drift") => "Lets the level wander, as if people move around.",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_effect_and_parameter_has_a_tooltip() {
        let mut missing = vec![];
        for s in super::super::SPECS {
            if super::effect(s.kind).is_none() {
                missing.push(s.kind.to_string());
            }
            for p in s.params {
                if super::param(s.kind, p.key).is_none() {
                    missing.push(format!("{}.{}", s.kind, p.key));
                }
            }
        }
        assert!(missing.is_empty(), "missing tooltips: {missing:?}");
    }
}
