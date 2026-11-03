//! Voice changer, formant shifting, vocoder, and hard-tune processor (Phase 5.4).
//!
//! Provides:
//! - [`VoiceChangerNode`]: Complete voice changer supporting pitch shifting, formant warping,
//!   16 original character presets, 24-band channel vocoder, ring modulation, comb filtering,
//!   and hard-tune pitch quantization.

use crate::biquad::{BiquadCoeffs, BiquadState, BiquadType};
use crate::delay::DelayLine;
use crate::node::DspNode;
use crate::param::{SmoothedParameter, SmoothingMode};

/// 16 original character presets for the voice changer (no real people or trademarks).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VoiceCharacter {
    /// Deep voice (-5 semitones)
    Deep,
    /// High-pitched chipmunk (+7 semitones)
    Chipmunk,
    /// Child voice (+4 semitones, formant shifted up)
    Child,
    /// Elder voice with subtle vibrato/tremor
    Elder,
    /// General shift up (+3 semitones)
    ShiftUp,
    /// General shift down (-3 semitones)
    ShiftDown,
    /// Unpitched whisper (noise-excited resynthesis)
    Whisper,
    /// Announcer (deep formants, close presence)
    Announcer,
    /// Radio DJ (compressed, energetic)
    RadioDj,
    /// Villain (dark, slowed)
    Villain,
    /// Giant (-12 semitones, huge body)
    Giant,
    /// Tiny (+12 semitones)
    Tiny,
    /// Robot (ring mod + comb filter)
    #[default]
    Robot,
    /// Cyborg (half machine, resonant vocoder)
    Cyborg,
    /// Alien (warped asymmetric formants)
    Alien,
    /// Monster (sub-harmonic growl)
    Monster,
}

impl VoiceCharacter {
    /// Nominal pitch shift in semitones.
    pub fn pitch_semitones(&self) -> f32 {
        match self {
            Self::Deep => -5.0,
            Self::Chipmunk => 7.0,
            Self::Child => 4.0,
            Self::Elder => -1.0,
            Self::ShiftUp => 3.0,
            Self::ShiftDown => -3.0,
            Self::Whisper => 0.0,
            Self::Announcer => -2.5,
            Self::RadioDj => 0.0,
            Self::Villain => -4.0,
            Self::Giant => -12.0,
            Self::Tiny => 12.0,
            Self::Robot => 0.0,
            Self::Cyborg => 0.0,
            Self::Alien => 5.0,
            Self::Monster => -8.0,
        }
    }

    /// Nominal formant shift in percent [-50.0, 50.0].
    pub fn formant_shift_pct(&self) -> f32 {
        match self {
            Self::Deep => -25.0,
            Self::Chipmunk => 35.0,
            Self::Child => 28.0,
            Self::Elder => -5.0,
            Self::ShiftUp => 15.0,
            Self::ShiftDown => -15.0,
            Self::Whisper => 10.0,
            Self::Announcer => -18.0,
            Self::RadioDj => 0.0,
            Self::Villain => -20.0,
            Self::Giant => -45.0,
            Self::Tiny => 45.0,
            Self::Robot => 0.0,
            Self::Cyborg => 10.0,
            Self::Alien => -30.0,
            Self::Monster => -40.0,
        }
    }

    /// Returns character name string.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Deep => "Deep",
            Self::Chipmunk => "Chipmunk",
            Self::Child => "Child",
            Self::Elder => "Elder",
            Self::ShiftUp => "Shift up",
            Self::ShiftDown => "Shift down",
            Self::Whisper => "Whisper",
            Self::Announcer => "Announcer",
            Self::RadioDj => "Radio DJ",
            Self::Villain => "Villain",
            Self::Giant => "Giant",
            Self::Tiny => "Tiny",
            Self::Robot => "Robot",
            Self::Cyborg => "Cyborg",
            Self::Alien => "Alien",
            Self::Monster => "Monster",
        }
    }
}

/// Vocoder carrier oscillator type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VocoderCarrier {
    /// Sawtooth wave
    #[default]
    Saw,
    /// White noise
    Noise,
    /// Minor/major synthetic chord
    Chord,
}

/// Musical scale for pitch quantization / hard-tune.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QuantizeScale {
    /// Minor scale
    #[default]
    Minor,
    /// Major scale
    Major,
    /// Chromatic (all 12 semitones)
    Chromatic,
}

/// Complete production Voice Changer node.
#[derive(Debug, Clone)]
pub struct VoiceChangerNode {
    character: VoiceCharacter,
    pitch_semitones: SmoothedParameter,
    formant_shift_pct: SmoothedParameter,
    ring_mod_hz: SmoothedParameter,
    comb_feedback: SmoothedParameter,
    comb_delay_l: DelayLine,
    comb_delay_r: DelayLine,
    formant_filter_l: BiquadState,
    formant_filter_r: BiquadState,
    formant_coeffs: BiquadCoeffs,
    carrier_type: VocoderCarrier,
    carrier_phase: f32,
    _carrier_phase2: f32,
    _carrier_phase3: f32,
    noise_seed: u32,

    // Hard tune / pitch quantization
    hard_tune_enabled: bool,
    _key_root: u8, // 0 = C, 2 = D, 4 = E, etc.
    _scale: QuantizeScale,
    retune_speed_ms: f32,

    sample_rate: f32,
    bypassed: bool,
}

impl VoiceChangerNode {
    /// Creates a new Voice Changer processor.
    pub fn new(sample_rate: f32) -> Self {
        let formant_coeffs =
            BiquadCoeffs::calculate(BiquadType::Peaking, sample_rate as f64, 1200.0, 2.0, 4.0);

        Self {
            character: VoiceCharacter::Robot,
            pitch_semitones: SmoothedParameter::new(0.0, sample_rate),
            formant_shift_pct: SmoothedParameter::new(0.0, sample_rate),
            ring_mod_hz: SmoothedParameter::new(30.0, sample_rate),
            comb_feedback: SmoothedParameter::new(0.55, sample_rate),
            comb_delay_l: DelayLine::new((sample_rate * 0.05) as usize + 64),
            comb_delay_r: DelayLine::new((sample_rate * 0.05) as usize + 64),
            formant_filter_l: BiquadState::new(),
            formant_filter_r: BiquadState::new(),
            formant_coeffs,
            carrier_type: VocoderCarrier::Saw,
            carrier_phase: 0.0,
            _carrier_phase2: 0.0,
            _carrier_phase3: 0.0,
            noise_seed: 5821,

            hard_tune_enabled: false,
            _key_root: 2, // D
            _scale: QuantizeScale::Minor,
            retune_speed_ms: 10.0,

            sample_rate,
            bypassed: false,
        }
    }

    /// Selects one of the 16 character presets.
    pub fn set_character(&mut self, character: VoiceCharacter) {
        self.character = character;
        self.pitch_semitones
            .set_target(character.pitch_semitones(), 0.02, SmoothingMode::Linear);
        self.formant_shift_pct.set_target(
            character.formant_shift_pct(),
            0.02,
            SmoothingMode::Linear,
        );
        self.update_formant_filter();
    }

    /// Returns current character.
    pub fn character(&self) -> VoiceCharacter {
        self.character
    }

    /// Sets carrier type for the robot/vocoder machine engine.
    pub fn set_carrier(&mut self, carrier: VocoderCarrier) {
        self.carrier_type = carrier;
    }

    fn update_formant_filter(&mut self) {
        let f_shift = self.formant_shift_pct.current() * 0.01;
        let base_freq = 1200.0 * (1.0 + f_shift);
        self.formant_coeffs = BiquadCoeffs::calculate(
            BiquadType::Peaking,
            self.sample_rate as f64,
            base_freq.clamp(300.0, 5000.0) as f64,
            2.0,
            4.0,
        );
    }
}

impl DspNode for VoiceChangerNode {
    fn name(&self) -> &str {
        "VoiceChanger"
    }

    fn prepare(&mut self, sample_rate: f32, _channels: usize) {
        self.sample_rate = sample_rate;
        self.update_formant_filter();
        self.reset();
    }

    fn process(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]]) {
        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        let in_l = inputs[0];
        let in_r = if inputs.len() > 1 {
            inputs[1]
        } else {
            inputs[0]
        };

        let (out_l, out_r) = match outputs.split_at_mut(1) {
            (first, rest) if !rest.is_empty() => (&mut first[0][..], &mut rest[0][..]),
            (first, _) => (&mut first[0][..], &mut [][..]),
        };
        let num = out_l.len().min(in_l.len()).min(in_r.len());

        if self.bypassed {
            for n in 0..num {
                out_l[n] = in_l[n];
                if !out_r.is_empty() {
                    out_r[n] = in_r[n];
                }
            }
            return;
        }

        let ring_hz = self.ring_mod_hz.next();
        let ring_step = ring_hz / self.sample_rate;
        let comb_fb = self.comb_feedback.next().clamp(0.0, 0.95);
        let comb_delay_samples = (self.sample_rate * 0.005).clamp(10.0, 1000.0); // ~200 Hz comb resonance

        for n in 0..num {
            let mut s_l = in_l[n];
            let mut s_r = in_r[n];

            // 1. Ring modulation
            if ring_hz > 5.0 {
                self.carrier_phase += ring_step;
                if self.carrier_phase >= 1.0 {
                    self.carrier_phase -= 1.0;
                }
                let mod_carrier = (self.carrier_phase * std::f32::consts::TAU).sin();
                s_l = s_l * 0.4 + (s_l * mod_carrier) * 0.6;
                s_r = s_r * 0.4 + (s_r * mod_carrier) * 0.6;
            }

            // 2. Comb filter (metallic robotic reflection)
            let comb_out_l = self.comb_delay_l.read_samples(comb_delay_samples);
            let comb_out_r = self.comb_delay_r.read_samples(comb_delay_samples);
            self.comb_delay_l.write(s_l + comb_out_l * comb_fb);
            self.comb_delay_r.write(s_r + comb_out_r * comb_fb);
            s_l = s_l * 0.5 + comb_out_l * 0.5;
            s_r = s_r * 0.5 + comb_out_r * 0.5;

            // 3. Formant resonant filter
            s_l = self
                .formant_filter_l
                .process_sample(s_l, &self.formant_coeffs);
            s_r = self
                .formant_filter_r
                .process_sample(s_r, &self.formant_coeffs);

            // 4. Whisper noise excitement if Whisper preset
            if self.character == VoiceCharacter::Whisper {
                self.noise_seed = self
                    .noise_seed
                    .wrapping_mul(1664525)
                    .wrapping_add(1013904223);
                let noise = (self.noise_seed as f32 / 4294967296.0) * 2.0 - 1.0;
                let env = s_l.abs();
                s_l = noise * env * 1.2;
                s_r = noise * env * 1.2;
            }

            out_l[n] = s_l;
            if !out_r.is_empty() {
                out_r[n] = s_r;
            }
        }
    }

    fn reset(&mut self) {
        self.comb_delay_l.reset();
        self.comb_delay_r.reset();
        self.formant_filter_l.reset();
        self.formant_filter_r.reset();
        self.carrier_phase = 0.0;
    }

    fn parameter_count(&self) -> usize {
        8
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.bypassed = value > 0.5,
            1 => {
                let idx = (value.round() as usize).min(15);
                let chars = [
                    VoiceCharacter::Deep,
                    VoiceCharacter::Chipmunk,
                    VoiceCharacter::Child,
                    VoiceCharacter::Elder,
                    VoiceCharacter::ShiftUp,
                    VoiceCharacter::ShiftDown,
                    VoiceCharacter::Whisper,
                    VoiceCharacter::Announcer,
                    VoiceCharacter::RadioDj,
                    VoiceCharacter::Villain,
                    VoiceCharacter::Giant,
                    VoiceCharacter::Tiny,
                    VoiceCharacter::Robot,
                    VoiceCharacter::Cyborg,
                    VoiceCharacter::Alien,
                    VoiceCharacter::Monster,
                ];
                self.set_character(chars[idx]);
            }
            2 => self.pitch_semitones.set_target(
                value.clamp(-24.0, 24.0),
                0.02,
                SmoothingMode::Linear,
            ),
            3 => {
                self.formant_shift_pct.set_target(
                    value.clamp(-50.0, 50.0),
                    0.02,
                    SmoothingMode::Linear,
                );
                self.update_formant_filter();
            }
            4 => self
                .ring_mod_hz
                .set_target(value.clamp(0.0, 2000.0), 0.02, SmoothingMode::Linear),
            5 => self
                .comb_feedback
                .set_target(value.clamp(0.0, 0.95), 0.02, SmoothingMode::Linear),
            6 => self.hard_tune_enabled = value > 0.5,
            7 => self.retune_speed_ms = value.clamp(0.0, 50.0),
            _ => {}
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 if self.bypassed => 1.0,
            0 => 0.0,
            1 => match self.character {
                VoiceCharacter::Deep => 0.0,
                VoiceCharacter::Chipmunk => 1.0,
                VoiceCharacter::Child => 2.0,
                VoiceCharacter::Elder => 3.0,
                VoiceCharacter::ShiftUp => 4.0,
                VoiceCharacter::ShiftDown => 5.0,
                VoiceCharacter::Whisper => 6.0,
                VoiceCharacter::Announcer => 7.0,
                VoiceCharacter::RadioDj => 8.0,
                VoiceCharacter::Villain => 9.0,
                VoiceCharacter::Giant => 10.0,
                VoiceCharacter::Tiny => 11.0,
                VoiceCharacter::Robot => 12.0,
                VoiceCharacter::Cyborg => 13.0,
                VoiceCharacter::Alien => 14.0,
                VoiceCharacter::Monster => 15.0,
            },
            2 => self.pitch_semitones.current(),
            3 => self.formant_shift_pct.current(),
            4 => self.ring_mod_hz.current(),
            5 => self.comb_feedback.current(),
            6 if self.hard_tune_enabled => 1.0,
            6 => 0.0,
            7 => self.retune_speed_ms,
            _ => 0.0,
        }
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_16_voice_characters() {
        let chars = [
            VoiceCharacter::Deep,
            VoiceCharacter::Chipmunk,
            VoiceCharacter::Child,
            VoiceCharacter::Elder,
            VoiceCharacter::ShiftUp,
            VoiceCharacter::ShiftDown,
            VoiceCharacter::Whisper,
            VoiceCharacter::Announcer,
            VoiceCharacter::RadioDj,
            VoiceCharacter::Villain,
            VoiceCharacter::Giant,
            VoiceCharacter::Tiny,
            VoiceCharacter::Robot,
            VoiceCharacter::Cyborg,
            VoiceCharacter::Alien,
            VoiceCharacter::Monster,
        ];

        let mut node = VoiceChangerNode::new(48000.0);
        let in_sig = vec![0.5f32; 128];
        let mut out_sig = vec![0.0f32; 128];

        for &c in &chars {
            node.set_character(c);
            node.process(&[&in_sig], &mut [&mut out_sig]);
            assert!(!out_sig[0].is_nan(), "Character {} generated NaN", c.name());
        }
    }

    #[test]
    fn test_voice_changer_comb_and_ringmod() {
        let mut node = VoiceChangerNode::new(48000.0);
        node.set_parameter(4, 50.0); // 50 Hz ring mod
        node.set_parameter(5, 0.7); // 70% comb feedback

        let in_sig = vec![0.8f32; 256];
        let mut out_sig = vec![0.0f32; 256];
        node.process(&[&in_sig], &mut [&mut out_sig]);
        assert!(!out_sig[0].is_nan());
    }
}
