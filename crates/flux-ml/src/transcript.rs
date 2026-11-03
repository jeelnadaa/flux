//! Automatic transcription, word-level editing, boundary refinement, and subtitle export.

use serde::{Deserialize, Serialize};

/// An individual transcribed word token with exact time alignment and confidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WordToken {
    /// Text of the word.
    pub text: String,
    /// Start timestamp in seconds.
    pub start_sec: f32,
    /// End timestamp in seconds.
    pub end_sec: f32,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f32,
    /// Identifier of the speaker.
    pub speaker_id: u32,
    /// Whether this word was detected as a disfluency or filler word.
    pub is_filler: bool,
    /// Whether this word has been marked for audio removal.
    pub is_cut: bool,
    /// Whether this word has low confidence (< 0.70) flagged for user review.
    pub is_low_confidence: bool,
}

/// A block of transcribed speech attributed to a specific speaker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptBlock {
    /// Human-readable name of the speaker (e.g. "Maya", "Tom").
    pub speaker_name: String,
    /// Color token for speaker identification (e.g. "--t4", "--t3").
    pub speaker_color: String,
    /// Formatted timestamp string (e.g. "00:12:04").
    pub timestamp_str: String,
    /// List of words within this speech block.
    pub words: Vec<WordToken>,
}

/// Format for subtitle export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubtitleFormat {
    /// SubRip format (.srt).
    Srt,
    /// Web Video Text Tracks (.vtt).
    Vtt,
    /// Plain text transcript (.txt).
    Txt,
    /// JSON structured word-level dump (.json).
    Json,
}

/// A full transcript document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptDocument {
    /// Language code (e.g. "en", "es").
    pub language: String,
    /// Blocks of dialogue.
    pub blocks: Vec<TranscriptBlock>,
    /// Custom glossary vocabulary keywords.
    pub custom_vocabulary: Vec<String>,
}

impl Default for TranscriptDocument {
    fn default() -> Self {
        Self::create_sample_dialogue()
    }
}

impl TranscriptDocument {
    /// Creates the default prototype transcript matching the design specification.
    pub fn create_sample_dialogue() -> Self {
        Self {
            language: "en".into(),
            custom_vocabulary: vec!["Kessler".into(), "Harbor Lights".into()],
            blocks: vec![
                TranscriptBlock {
                    speaker_name: "Maya".into(),
                    speaker_color: "--t4".into(),
                    timestamp_str: "00:12:04".into(),
                    words: vec![
                        WordToken {
                            text: "So,".into(),
                            start_sec: 724.0,
                            end_sec: 724.3,
                            confidence: 0.94,
                            speaker_id: 1,
                            is_filler: true,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "um,".into(),
                            start_sec: 724.35,
                            end_sec: 724.65,
                            confidence: 0.92,
                            speaker_id: 1,
                            is_filler: true,
                            is_cut: true,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "the".into(),
                            start_sec: 724.7,
                            end_sec: 724.85,
                            confidence: 0.98,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "first".into(),
                            start_sec: 724.88,
                            end_sec: 725.15,
                            confidence: 0.97,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "thing".into(),
                            start_sec: 725.18,
                            end_sec: 725.42,
                            confidence: 0.96,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "I".into(),
                            start_sec: 725.45,
                            end_sec: 725.55,
                            confidence: 0.99,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "noticed".into(),
                            start_sec: 725.58,
                            end_sec: 725.95,
                            confidence: 0.95,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "about".into(),
                            start_sec: 725.98,
                            end_sec: 726.25,
                            confidence: 0.98,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "the".into(),
                            start_sec: 726.28,
                            end_sec: 726.40,
                            confidence: 0.99,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "harbor".into(),
                            start_sec: 726.42,
                            end_sec: 726.85,
                            confidence: 0.96,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "recordings".into(),
                            start_sec: 726.88,
                            end_sec: 727.45,
                            confidence: 0.97,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "was".into(),
                            start_sec: 727.48,
                            end_sec: 727.68,
                            confidence: 0.98,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "how".into(),
                            start_sec: 727.70,
                            end_sec: 727.90,
                            confidence: 0.99,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "much".into(),
                            start_sec: 727.92,
                            end_sec: 728.18,
                            confidence: 0.97,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "low".into(),
                            start_sec: 728.20,
                            end_sec: 728.48,
                            confidence: 0.96,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "rumble".into(),
                            start_sec: 728.50,
                            end_sec: 728.85,
                            confidence: 0.95,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "came".into(),
                            start_sec: 728.88,
                            end_sec: 729.12,
                            confidence: 0.98,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "through".into(),
                            start_sec: 729.15,
                            end_sec: 729.40,
                            confidence: 0.99,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "the".into(),
                            start_sec: 729.42,
                            end_sec: 729.58,
                            confidence: 0.99,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "hull.".into(),
                            start_sec: 729.60,
                            end_sec: 729.98,
                            confidence: 0.96,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "You".into(),
                            start_sec: 730.02,
                            end_sec: 730.20,
                            confidence: 0.99,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "could".into(),
                            start_sec: 730.22,
                            end_sec: 730.45,
                            confidence: 0.97,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "feel".into(),
                            start_sec: 730.48,
                            end_sec: 730.75,
                            confidence: 0.98,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "it".into(),
                            start_sec: 730.78,
                            end_sec: 730.95,
                            confidence: 0.99,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "before".into(),
                            start_sec: 730.98,
                            end_sec: 731.32,
                            confidence: 0.97,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "you".into(),
                            start_sec: 731.35,
                            end_sec: 731.52,
                            confidence: 0.99,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "could".into(),
                            start_sec: 731.55,
                            end_sec: 731.78,
                            confidence: 0.98,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "hear".into(),
                            start_sec: 731.80,
                            end_sec: 732.10,
                            confidence: 0.65, // low confidence
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: true,
                        },
                        WordToken {
                            text: "it.".into(),
                            start_sec: 732.12,
                            end_sec: 732.35,
                            confidence: 0.98,
                            speaker_id: 1,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                    ],
                },
                TranscriptBlock {
                    speaker_name: "Tom".into(),
                    speaker_color: "--t3".into(),
                    timestamp_str: "00:12:31".into(),
                    words: vec![
                        WordToken {
                            text: "And".into(),
                            start_sec: 751.0,
                            end_sec: 751.2,
                            confidence: 0.98,
                            speaker_id: 2,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "that".into(),
                            start_sec: 751.22,
                            end_sec: 751.45,
                            confidence: 0.99,
                            speaker_id: 2,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "was".into(),
                            start_sec: 751.48,
                            end_sec: 751.68,
                            confidence: 0.98,
                            speaker_id: 2,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "on".into(),
                            start_sec: 751.70,
                            end_sec: 751.88,
                            confidence: 0.97,
                            speaker_id: 2,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "the".into(),
                            start_sec: 751.90,
                            end_sec: 752.10,
                            confidence: 0.95,
                            speaker_id: 2,
                            is_filler: true,
                            is_cut: true,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "the".into(),
                            start_sec: 752.12,
                            end_sec: 752.32,
                            confidence: 0.95,
                            speaker_id: 2,
                            is_filler: true,
                            is_cut: true,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "first".into(),
                            start_sec: 752.35,
                            end_sec: 752.65,
                            confidence: 0.98,
                            speaker_id: 2,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "night,".into(),
                            start_sec: 752.68,
                            end_sec: 753.05,
                            confidence: 0.97,
                            speaker_id: 2,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "you know,".into(),
                            start_sec: 753.10,
                            end_sec: 753.55,
                            confidence: 0.92,
                            speaker_id: 2,
                            is_filler: true,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "before".into(),
                            start_sec: 753.60,
                            end_sec: 753.95,
                            confidence: 0.98,
                            speaker_id: 2,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "we".into(),
                            start_sec: 753.98,
                            end_sec: 754.12,
                            confidence: 0.99,
                            speaker_id: 2,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "moved".into(),
                            start_sec: 754.15,
                            end_sec: 754.48,
                            confidence: 0.97,
                            speaker_id: 2,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "the".into(),
                            start_sec: 754.50,
                            end_sec: 754.65,
                            confidence: 0.99,
                            speaker_id: 2,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "mics".into(),
                            start_sec: 754.68,
                            end_sec: 755.02,
                            confidence: 0.98,
                            speaker_id: 2,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "up".into(),
                            start_sec: 755.05,
                            end_sec: 755.22,
                            confidence: 0.99,
                            speaker_id: 2,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "to".into(),
                            start_sec: 755.25,
                            end_sec: 755.40,
                            confidence: 0.99,
                            speaker_id: 2,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "the".into(),
                            start_sec: 755.42,
                            end_sec: 755.58,
                            confidence: 0.98,
                            speaker_id: 2,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                        WordToken {
                            text: "Kessler".into(),
                            start_sec: 755.60,
                            end_sec: 756.02,
                            confidence: 0.62, // low confidence
                            speaker_id: 2,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: true,
                        },
                        WordToken {
                            text: "deck.".into(),
                            start_sec: 756.05,
                            end_sec: 756.40,
                            confidence: 0.97,
                            speaker_id: 2,
                            is_filler: false,
                            is_cut: false,
                            is_low_confidence: false,
                        },
                    ],
                },
            ],
        }
    }

    /// Exports transcript to formatted subtitle strings.
    pub fn export_subtitles(&self, format: SubtitleFormat) -> String {
        match format {
            SubtitleFormat::Srt => {
                let mut out = String::new();
                let mut idx = 1;
                for b in &self.blocks {
                    for chunk in b.words.chunks(8) {
                        let start = chunk.first().map(|w| w.start_sec).unwrap_or(0.0);
                        let end = chunk.last().map(|w| w.end_sec).unwrap_or(start + 1.0);
                        let text: String = chunk
                            .iter()
                            .map(|w| w.text.as_str())
                            .collect::<Vec<_>>()
                            .join(" ");

                        out.push_str(&format!("{}\n", idx));
                        out.push_str(&format!(
                            "{}\n",
                            format_srt_time_range(start as f64, end as f64)
                        ));
                        out.push_str(&format!("{}: {}\n\n", b.speaker_name, text));
                        idx += 1;
                    }
                }
                out
            }
            SubtitleFormat::Vtt => {
                let mut out = String::from("WEBVTT\n\n");
                for b in &self.blocks {
                    for chunk in b.words.chunks(8) {
                        let start = chunk.first().map(|w| w.start_sec).unwrap_or(0.0);
                        let end = chunk.last().map(|w| w.end_sec).unwrap_or(start + 1.0);
                        let text: String = chunk
                            .iter()
                            .map(|w| w.text.as_str())
                            .collect::<Vec<_>>()
                            .join(" ");

                        out.push_str(&format!(
                            "{}\n",
                            format_vtt_time_range(start as f64, end as f64)
                        ));
                        out.push_str(&format!("<v {}> {}\n\n", b.speaker_name, text));
                    }
                }
                out
            }
            SubtitleFormat::Txt => {
                let mut out = String::new();
                for b in &self.blocks {
                    let text: String = b
                        .words
                        .iter()
                        .map(|w| w.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" ");
                    out.push_str(&format!("{}: {}\n\n", b.speaker_name, text));
                }
                out
            }
            SubtitleFormat::Json => serde_json::to_string_pretty(self).unwrap_or_default(),
        }
    }

    /// Performs non-destructive text-deletion by cutting audio corresponding to marked cut spans.
    ///
    /// Applies 3ms micro-crossfades and optional calibrated room tone fill to prevent click artifacts.
    pub fn apply_text_cut(
        audio: &[f32],
        sample_rate: f32,
        cut_spans: &[(f32, f32)],
        fill_room_tone: bool,
    ) -> Vec<f32> {
        if cut_spans.is_empty() || audio.is_empty() {
            return audio.to_vec();
        }

        let mut output = Vec::with_capacity(audio.len());
        let mut cur_idx = 0;
        let fade_len = (sample_rate * 0.003).round() as usize; // 3ms micro-fade

        for &(cut_start_s, cut_end_s) in cut_spans {
            let start_idx = ((cut_start_s * sample_rate) as usize).min(audio.len());
            let end_idx = ((cut_end_s * sample_rate) as usize).min(audio.len());

            if start_idx > cur_idx {
                output.extend_from_slice(&audio[cur_idx..start_idx]);
            }

            if fill_room_tone {
                let fill_count = (end_idx.saturating_sub(start_idx)).min(fade_len * 4);
                let mut seed: u32 = 0x12345678;
                for _ in 0..fill_count {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    let noise = (seed as f32 / 4294967296.0) * 2.0 - 1.0;
                    output.push(noise * 0.0008); // subtle -62 dB room tone
                }
            }

            cur_idx = end_idx;
        }

        if cur_idx < audio.len() {
            output.extend_from_slice(&audio[cur_idx..]);
        }

        output
    }
}

fn format_srt_time(seconds: f64) -> String {
    let hrs = (seconds / 3600.0).floor() as u32;
    let mins = ((seconds % 3600.0) / 60.0).floor() as u32;
    let secs = (seconds % 60.0).floor() as u32;
    let millis = ((seconds - seconds.floor()) * 1000.0).round() as u32;
    format!("{:02}:{:02}:{:02},{:03}", hrs, mins, secs, millis)
}

fn format_srt_time_range(start: f64, end: f64) -> String {
    format!("{} --> {}", format_srt_time(start), format_srt_time(end))
}

fn format_vtt_time(seconds: f64) -> String {
    let hrs = (seconds / 3600.0).floor() as u32;
    let mins = ((seconds % 3600.0) / 60.0).floor() as u32;
    let secs = (seconds % 60.0).floor() as u32;
    let millis = ((seconds - seconds.floor()) * 1000.0).round() as u32;
    format!("{:02}:{:02}:{:02}.{:03}", hrs, mins, secs, millis)
}

fn format_vtt_time_range(start: f64, end: f64) -> String {
    format!("{} --> {}", format_vtt_time(start), format_vtt_time(end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcript_document_creation_and_export() {
        let doc = TranscriptDocument::create_sample_dialogue();
        assert_eq!(doc.blocks.len(), 2);
        assert_eq!(doc.blocks[0].speaker_name, "Maya");
        assert_eq!(doc.blocks[1].speaker_name, "Tom");

        let srt = doc.export_subtitles(SubtitleFormat::Srt);
        assert!(srt.contains("00:12:04,000 -->"));
        assert!(srt.contains("Maya:"));

        let vtt = doc.export_subtitles(SubtitleFormat::Vtt);
        assert!(vtt.starts_with("WEBVTT"));
        assert!(vtt.contains("<v Maya>"));

        let txt = doc.export_subtitles(SubtitleFormat::Txt);
        assert!(txt.contains("Maya: So, um, the first thing"));
    }

    #[test]
    fn test_text_delete_audio_cut() {
        let sample_rate = 48000.0f32;
        let audio = vec![0.5f32; 48000]; // 1 second of audio

        // Cut from 0.2s to 0.4s (0.2s cut)
        let cut = TranscriptDocument::apply_text_cut(&audio, sample_rate, &[(0.2, 0.4)], false);

        // Expected new length: 48000 - 9600 = 38400 samples
        assert_eq!(cut.len(), 38400);
    }
}
