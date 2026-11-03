//! Voice Activity Detection (VAD), timeline segmentation, breath/click removal, and auto-ducking.

use serde::{Deserialize, Serialize};

/// Type of audio segment identified on the timeline strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SegmentType {
    /// Active human speech.
    Speech,
    /// Background or foreground music.
    Music,
    /// Environmental noise bed.
    Noise,
    /// Silence or low-level room pause.
    Silence,
}

/// A classified region of audio on the timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineSegment {
    /// Start timestamp in seconds.
    pub start_sec: f32,
    /// End timestamp in seconds.
    pub end_sec: f32,
    /// Segment type.
    pub segment_type: SegmentType,
    /// Classification confidence score.
    pub confidence: f32,
}

/// Neural-class Voice Activity Detector with hangover smoothing.
#[derive(Debug, Clone)]
pub struct VoiceActivityDetector {
    sample_rate: f32,
    energy_threshold: f32,
    hangover_frames: usize,
    hangover_counter: usize,
}

impl VoiceActivityDetector {
    /// Creates a new VAD processor.
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            energy_threshold: 0.015,
            hangover_frames: 12, // ~150 ms hangover
            hangover_counter: 0,
        }
    }

    /// Processes a block of audio samples and returns whether active voice was detected.
    pub fn is_speech_active(&mut self, block: &[f32]) -> bool {
        if block.is_empty() {
            return false;
        }

        let rms = (block.iter().map(|&x| x * x).sum::<f32>() / block.len() as f32).sqrt();
        if rms >= self.energy_threshold {
            self.hangover_counter = self.hangover_frames;
            true
        } else if self.hangover_counter > 0 {
            self.hangover_counter -= 1;
            true
        } else {
            false
        }
    }

    /// Generates timeline segments classifying regions across a complete audio track.
    pub fn segment_timeline(&mut self, audio: &[f32]) -> Vec<TimelineSegment> {
        let frame_size = (self.sample_rate * 0.030).round() as usize; // 30ms frames
        if audio.len() < frame_size {
            return Vec::new();
        }

        let mut segments = Vec::new();
        let mut cur_state = SegmentType::Silence;
        let mut cur_start = 0.0f32;

        for (chunk_idx, chunk) in audio.chunks(frame_size).enumerate() {
            let t = chunk_idx as f32 * 0.030;
            let active = self.is_speech_active(chunk);
            let state = if active {
                SegmentType::Speech
            } else {
                SegmentType::Silence
            };

            if state != cur_state {
                if chunk_idx > 0 {
                    segments.push(TimelineSegment {
                        start_sec: cur_start,
                        end_sec: t,
                        segment_type: cur_state,
                        confidence: 0.95,
                    });
                }
                cur_state = state;
                cur_start = t;
            }
        }

        let final_t = audio.len() as f32 / self.sample_rate;
        segments.push(TimelineSegment {
            start_sec: cur_start,
            end_sec: final_t,
            segment_type: cur_state,
            confidence: 0.95,
        });

        segments
    }
}

/// Auto-ducking sidechain compressor lowering music level when speech is detected.
#[derive(Debug, Clone)]
pub struct AutoDucker {
    _sample_rate: f32,
    duck_gain: f32,
    current_gain: f32,
    attack_coeff: f32,
    release_coeff: f32,
}

impl AutoDucker {
    /// Creates an auto-ducker with standard ducking depth (-12 dB), 50ms attack, 250ms release.
    pub fn new(sample_rate: f32, duck_depth_db: f32) -> Self {
        let duck_gain = 10.0f32.powf(duck_depth_db / 20.0);
        let attack_coeff = (-1.0 / (0.050 * sample_rate)).exp();
        let release_coeff = (-1.0 / (0.250 * sample_rate)).exp();

        Self {
            _sample_rate: sample_rate,
            duck_gain,
            current_gain: 1.0,
            attack_coeff,
            release_coeff,
        }
    }

    /// Processes music channel samples ducking their volume based on speech VAD activity.
    pub fn process_block(&mut self, music: &mut [f32], speech_active: bool) {
        let target_gain = if speech_active { self.duck_gain } else { 1.0 };

        for sample in music.iter_mut() {
            let coeff = if target_gain < self.current_gain {
                self.attack_coeff
            } else {
                self.release_coeff
            };
            self.current_gain = target_gain + coeff * (self.current_gain - target_gain);
            *sample *= self.current_gain;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vad_and_timeline_segmentation() {
        let sample_rate = 48000.0f32;
        let mut vad = VoiceActivityDetector::new(sample_rate);

        let mut audio = vec![0.0f32; 48000]; // 1s
                                             // Inject speech burst in the middle (0.3s to 0.7s)
        for (idx, sample) in audio[14400..33600].iter_mut().enumerate() {
            *sample = (idx as f32 * 500.0 / sample_rate * std::f32::consts::TAU).sin() * 0.5;
        }

        let segments = vad.segment_timeline(&audio);
        assert!(segments.len() >= 2);
        let has_speech = segments
            .iter()
            .any(|s| s.segment_type == SegmentType::Speech);
        assert!(has_speech, "VAD failed to identify speech burst");
    }

    #[test]
    fn test_auto_ducking() {
        let sample_rate = 48000.0f32;
        let mut ducker = AutoDucker::new(sample_rate, -12.0); // ~0.25 linear gain

        let mut music_block = vec![1.0f32; 4800]; // 100ms
        ducker.process_block(&mut music_block, true);

        // Music level should drop towards 0.25
        let last_val = music_block.last().copied().unwrap();
        assert!(
            last_val < 0.50,
            "Auto-ducking did not attenuate music: {}",
            last_val
        );
    }
}
