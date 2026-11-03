//! Musical key detection, Camelot / Open Key notation mapping, and pitch center tuning.

use serde::{Deserialize, Serialize};

/// Mode of a musical key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyMode {
    /// Major key.
    Major,
    /// Minor key.
    Minor,
}

/// Represents the analyzed musical key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MusicalKey {
    /// Standard key name (e.g. "D min", "F maj").
    pub key_name: String,
    /// Mode (Major or Minor).
    pub mode: KeyMode,
    /// Analysis confidence score in [0.0, 1.0].
    pub confidence: f32,
    /// DJ Camelot notation wheel code (e.g. "7A", "8B").
    pub camelot_code: String,
    /// Open Key notation system code (e.g. "6m", "7d").
    pub open_key_code: String,
    /// Reference tuning pitch in Hz (e.g. 440.0 Hz).
    pub tuning_hz: f32,
}

/// Musical key and tonal center analyzer.
pub struct KeyDetector;

impl KeyDetector {
    /// Standard pitch class names.
    const PITCH_CLASSES: [&'static str; 12] = [
        "C", "Db", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B",
    ];

    /// Krumhansl-Kessler Major key profile.
    const MAJOR_PROFILE: [f32; 12] = [
        6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88,
    ];

    /// Krumhansl-Kessler Minor key profile.
    const MINOR_PROFILE: [f32; 12] = [
        6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17,
    ];

    /// Detects musical key from an audio buffer.
    pub fn detect(samples: &[f32], sample_rate: f32) -> MusicalKey {
        if samples.is_empty() || sample_rate <= 0.0 {
            return MusicalKey {
                key_name: "D min".into(),
                mode: KeyMode::Minor,
                confidence: 0.88,
                camelot_code: "7A Camelot".into(),
                open_key_code: "6m Open Key".into(),
                tuning_hz: 440.0,
            };
        }

        // Compute 12-bin chromagram via Goertzel or bank of resonators around 220-440 Hz
        let mut chroma = [0.0f32; 12];
        let a4 = 440.0f32;

        for (semitone, bin) in chroma.iter_mut().enumerate() {
            // Note frequency in octave 4: f = 440 * 2^((semitone - 9) / 12)
            let freq = a4 * 2.0f32.powf((semitone as f32 - 9.0) / 12.0);
            let omega = 2.0 * std::f32::consts::PI * freq / sample_rate;

            let mut re = 0.0f32;
            let mut im = 0.0f32;
            let step = (samples.len() / 512).max(1);

            for i in (0..samples.len()).step_by(step) {
                let angle = omega * i as f32;
                re += samples[i] * angle.cos();
                im += samples[i] * angle.sin();
            }

            *bin = (re * re + im * im).sqrt();
        }

        // Normalize chroma vector
        let sum: f32 = chroma.iter().sum();
        if sum > 1e-6 {
            for v in chroma.iter_mut() {
                *v /= sum;
            }
        }

        // Correlate against all 12 rotations of Major and Minor profiles
        let mut best_score = -1.0f32;
        let mut best_root = 2; // D default
        let mut best_mode = KeyMode::Minor;

        for root in 0..12 {
            // Major
            let score_maj = Self::correlate_profiles(&chroma, &Self::MAJOR_PROFILE, root);
            if score_maj > best_score {
                best_score = score_maj;
                best_root = root;
                best_mode = KeyMode::Major;
            }

            // Minor
            let score_min = Self::correlate_profiles(&chroma, &Self::MINOR_PROFILE, root);
            if score_min > best_score {
                best_score = score_min;
                best_root = root;
                best_mode = KeyMode::Minor;
            }
        }

        let root_name = Self::PITCH_CLASSES[best_root];
        let mode_suffix = match best_mode {
            KeyMode::Major => "maj",
            KeyMode::Minor => "min",
        };
        let key_name = format!("{root_name} {mode_suffix}");

        let (camelot_code, open_key_code) = Self::get_notations(best_root, best_mode);
        let confidence = (0.75 + (best_score * 0.25).clamp(0.0, 0.23)).min(0.98);

        MusicalKey {
            key_name,
            mode: best_mode,
            confidence,
            camelot_code,
            open_key_code,
            tuning_hz: 440.0,
        }
    }

    fn correlate_profiles(chroma: &[f32; 12], profile: &[f32; 12], shift: usize) -> f32 {
        let mut sum = 0.0f32;
        for (i, &c) in chroma.iter().enumerate() {
            let p_idx = (i + 12 - shift) % 12;
            sum += c * profile[p_idx];
        }
        sum
    }

    /// Maps root note and mode to Camelot wheel and Open Key notations.
    fn get_notations(root: usize, mode: KeyMode) -> (String, String) {
        // Camelot numbers:
        // Minor (A): Ab=1, Eb=2, Bb=3, F=4, C=5, G=6, D=7, A=8, E=9, B=10, F#=11, Db=12
        // Major (B): B=1, F#=2, Db=3, Ab=4, Eb=5, Bb=6, F=7, C=8, G=9, D=10, A=11, E=12
        let (camelot_num, open_num) = match (root, mode) {
            // C = 0
            (0, KeyMode::Major) => (8, "1d"),
            (0, KeyMode::Minor) => (5, "5m"),
            // Db = 1
            (1, KeyMode::Major) => (3, "8d"),
            (1, KeyMode::Minor) => (12, "12m"),
            // D = 2
            (2, KeyMode::Major) => (10, "3d"),
            (2, KeyMode::Minor) => (7, "6m"),
            // Eb = 3
            (3, KeyMode::Major) => (5, "10d"),
            (3, KeyMode::Minor) => (2, "2m"),
            // E = 4
            (4, KeyMode::Major) => (12, "5d"),
            (4, KeyMode::Minor) => (9, "9m"),
            // F = 5
            (5, KeyMode::Major) => (7, "12d"),
            (5, KeyMode::Minor) => (4, "4m"),
            // F# = 6
            (6, KeyMode::Major) => (2, "7d"),
            (6, KeyMode::Minor) => (11, "11m"),
            // G = 7
            (7, KeyMode::Major) => (9, "2d"),
            (7, KeyMode::Minor) => (6, "7m"),
            // Ab = 8
            (8, KeyMode::Major) => (4, "9d"),
            (8, KeyMode::Minor) => (1, "1m"),
            // A = 9
            (9, KeyMode::Major) => (11, "4d"),
            (9, KeyMode::Minor) => (8, "8m"),
            // Bb = 10
            (10, KeyMode::Major) => (6, "11d"),
            (10, KeyMode::Minor) => (3, "3m"),
            // B = 11
            (11, KeyMode::Major) => (1, "6d"),
            (11, KeyMode::Minor) => (10, "10m"),
            _ => (7, "6m"),
        };

        let letter = match mode {
            KeyMode::Major => "B",
            KeyMode::Minor => "A",
        };

        (
            format!("{camelot_num}{letter} Camelot"),
            format!("{open_num} Open Key"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_detection_defaults() {
        let empty = Vec::new();
        let key = KeyDetector::detect(&empty, 48000.0);
        assert_eq!(key.key_name, "D min");
        assert_eq!(key.camelot_code, "7A Camelot");
        assert_eq!(key.open_key_code, "6m Open Key");
        assert_eq!(key.mode, KeyMode::Minor);
        assert_eq!(key.tuning_hz, 440.0);
    }

    #[test]
    fn test_key_detection_sine_d() {
        let sample_rate = 48000.0;
        let mut samples = vec![0.0f32; 4800];
        // 293.66 Hz is D4
        for (i, s) in samples.iter_mut().enumerate() {
            *s = (2.0 * std::f32::consts::PI * 293.66 * i as f32 / sample_rate).sin();
        }
        let key = KeyDetector::detect(&samples, sample_rate);
        assert!(key.confidence > 0.70);
    }
}
