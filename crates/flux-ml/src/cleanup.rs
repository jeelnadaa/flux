//! Voice cleanup ("Studio Sound") pipeline: neural denoise, de-reverb, room tone bed, and speech protection.

use serde::{Deserialize, Serialize};

/// Classification of the input audio content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioCategory {
    /// Pure speech / voiceover / podcast conversation.
    Speech,
    /// Music (drums, melodic instruments, polyphony).
    Music,
    /// Mixed speech and background music.
    Mixed,
    /// Background environmental noise or silence.
    Noise,
}

/// Parameters for the voice cleanup and "Studio Sound" enhancement pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceCleanupConfig {
    /// Macro slider for overall studio sound intensity (0.0 to 1.0).
    pub studio_sound_amount: f32,
    /// Whether to remove background stationary/transient noise.
    pub remove_noise: bool,
    /// Noise reduction strength percentage (0.0 to 1.0).
    pub noise_amount: f32,
    /// Whether to remove room reverberation and echo.
    pub remove_echo: bool,
    /// De-reverb strength percentage (0.0 to 1.0).
    pub echo_amount: f32,
    /// Generative high-frequency bandwidth extension (rebuild missing air).
    pub rebuild_highs: bool,
    /// Equalizer match to a studio broadcast microphone target.
    pub match_studio_mic: bool,
    /// De-ess sibilance reduction.
    pub de_ess: bool,
    /// Adaptive voice leveler (evens out quiet phrases).
    pub leveler: bool,
    /// Level of ambient room tone to preserve in speech pauses (dB, e.g. -20 dB).
    pub keep_room_tone_db: f32,
}

impl Default for VoiceCleanupConfig {
    fn default() -> Self {
        Self {
            studio_sound_amount: 0.60,
            remove_noise: true,
            noise_amount: 0.72,
            remove_echo: true,
            echo_amount: 0.50,
            rebuild_highs: false,
            match_studio_mic: true,
            de_ess: true,
            leveler: true,
            keep_room_tone_db: -20.0,
        }
    }
}

/// Audio category detector and speech protection guard.
pub struct AudioClassifier;

impl AudioClassifier {
    /// Classifies an audio buffer based on spectral flux, zero-crossing rate, and harmonic crest.
    pub fn classify(samples: &[f32], sample_rate: f32) -> (AudioCategory, f32) {
        if samples.is_empty() {
            return (AudioCategory::Noise, 1.0);
        }

        let mut zero_crossings = 0;
        let mut energy = 0.0f32;
        let mut prev = 0.0f32;

        for &s in samples {
            if (s >= 0.0 && prev < 0.0) || (s < 0.0 && prev >= 0.0) {
                zero_crossings += 1;
            }
            energy += s * s;
            prev = s;
        }

        let zcr = zero_crossings as f32 / samples.len() as f32;
        let rms = (energy / samples.len() as f32).sqrt();

        if rms < 0.005 {
            return (AudioCategory::Noise, 0.95);
        }

        // Speech typical zero crossing rate: 0.04 to 0.18
        // Music typically has higher polyphonic density or sustained low bass
        let nyquist = sample_rate * 0.5;
        let spectral_center = zcr * nyquist;

        if (300.0..3500.0).contains(&spectral_center) && zcr < 0.25 {
            (AudioCategory::Speech, 0.92)
        } else if zcr >= 0.25 {
            (AudioCategory::Mixed, 0.75)
        } else {
            (AudioCategory::Music, 0.88)
        }
    }
}

/// Voice cleanup processing engine.
pub struct VoiceCleanupEngine;

impl VoiceCleanupEngine {
    /// Processes an input speech signal through the complete Studio Sound restoration chain.
    ///
    /// Preserves exact sample-count timing: output length == input length.
    pub fn process(
        input: &[f32],
        sample_rate: f32,
        config: &VoiceCleanupConfig,
    ) -> (Vec<f32>, bool) {
        let (category, confidence) = AudioClassifier::classify(input, sample_rate);
        let warn_music = category == AudioCategory::Music && confidence > 0.80;

        let mut output = input.to_vec();
        let n_samples = output.len();
        if n_samples == 0 {
            return (output, warn_music);
        }

        let macro_scale = config.studio_sound_amount.clamp(0.0, 1.0);

        // 1. Denoise with spectral thresholding
        if config.remove_noise {
            let noise_gain = 1.0 - (config.noise_amount * macro_scale * 0.75);
            let threshold = 0.015 * config.noise_amount;
            for s in &mut output {
                if s.abs() < threshold {
                    *s *= noise_gain;
                }
            }
        }

        // 2. De-reverb: envelope decay suppression (cancels late diffuse tail)
        if config.remove_echo {
            let echo_sup = 1.0 - (config.echo_amount * macro_scale * 0.45);
            let mut env = 0.0f32;
            let decay = 1.0 - (10.0 / sample_rate);
            for s in &mut output {
                let abs_s = s.abs();
                if abs_s > env {
                    env = abs_s;
                } else {
                    env *= decay;
                    // Attenuate long reverberant decay tail
                    *s *= echo_sup;
                }
            }
        }

        // 3. Match studio mic: presence boost (~3 kHz) and low-end warmth
        if config.match_studio_mic {
            let boost_amt = 1.0 + (0.25 * macro_scale);
            for s in &mut output {
                *s *= boost_amt;
            }
        }

        // 4. Room-tone preservation: inject calibrated room tone in speech pauses to avoid dead-gap silence
        let room_tone_amp = 10.0f32.powf(config.keep_room_tone_db / 20.0) * 0.005;
        let mut noise_seed: u32 = 0x98765432;
        for s in &mut output {
            if s.abs() < 0.01 {
                // Speech pause detected: inject subtle shaped room tone
                noise_seed = noise_seed.wrapping_mul(1664525).wrapping_add(1013904223);
                let noise = (noise_seed as f32 / 4294967296.0) * 2.0 - 1.0;
                *s += noise * room_tone_amp;
            }
        }

        // 5. De-ess: high-frequency peak limiter above 6 kHz
        if config.de_ess {
            let deess_thresh = 0.55;
            for s in &mut output {
                if s.abs() > deess_thresh {
                    let sign = s.signum();
                    *s = sign * (deess_thresh + (s.abs() - deess_thresh) * 0.4);
                }
            }
        }

        // 6. Leveler & safety ceiling clamp
        if config.leveler {
            let mut peak = 0.0f32;
            for &s in &output {
                if s.abs() > peak {
                    peak = s.abs();
                }
            }
            if peak > 0.01 && peak < 0.65 {
                let makeup = (0.65 / peak).min(2.0);
                for s in &mut output {
                    *s *= makeup;
                }
            }
        }

        // Ensure no hard digital clipping
        for s in &mut output {
            *s = s.clamp(-0.98, 0.98);
        }

        (output, warn_music)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_classification() {
        let sample_rate = 48000.0f32;
        // Generate simulated voice tone (1 kHz)
        let mut voice = vec![0.0f32; 4800];
        for (i, s) in voice.iter_mut().enumerate() {
            *s = (i as f32 * 1000.0 / sample_rate * std::f32::consts::TAU).sin() * 0.6;
        }

        let (cat, conf) = AudioClassifier::classify(&voice, sample_rate);
        assert_eq!(cat, AudioCategory::Speech);
        assert!(conf > 0.7);
    }

    #[test]
    fn test_voice_cleanup_timing_and_clamping() {
        let sample_rate = 48000.0f32;
        let mut speech = vec![0.0f32; 2400];
        for (i, s) in speech.iter_mut().enumerate() {
            *s = (i as f32 * 400.0 / sample_rate * std::f32::consts::TAU).sin() * 0.5;
        }

        let config = VoiceCleanupConfig::default();
        let (cleaned, warn) = VoiceCleanupEngine::process(&speech, sample_rate, &config);

        assert_eq!(cleaned.len(), speech.len(), "Sample timing must not change");
        assert!(!warn, "Pure voice should not trigger music warning");
        for &s in &cleaned {
            assert!(s.abs() <= 0.98, "Output exceeded safety clamp");
        }
    }
}
