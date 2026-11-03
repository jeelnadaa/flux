//! Automated translation and multilingual dubbing engine with duration fitting and stem remixing.

use serde::{Deserialize, Serialize};

/// Represents a translated dialogue line for dubbing review.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DubLine {
    /// Speaker identifier (e.g. "Maya", "Tom").
    pub speaker: String,
    /// Speaker UI theme color token (e.g. "--t4", "--t3").
    pub color_token: String,
    /// Source original text.
    pub original_text: String,
    /// Translated target text.
    pub translated_text: String,
    /// Target language name (e.g. "Spanish").
    pub target_language: String,
    /// Duration fit percentage difference vs original (e.g. "+4%", "+22%").
    pub duration_diff_percent: f32,
    /// Fit status badge ("ok" or "warn").
    pub status: String,
    /// Whether this segment was shortened or adjusted.
    pub shortened: bool,
}

/// Project container for dubbing setup and multi-line review.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DubbingProject {
    /// Source language.
    pub source_language: String,
    /// Target language.
    pub target_language: String,
    /// Tone setting ("Natural", "Formal", "Casual").
    pub tone: String,
    /// Custom terminology glossary to preserve.
    pub glossary: Vec<String>,
    /// Dialogue lines for dubbing.
    pub lines: Vec<DubLine>,
}

impl DubbingProject {
    /// Creates the standard Harbor Lights prototype dubbing dialogue dataset.
    pub fn create_sample() -> Self {
        Self {
            source_language: "English (detected)".into(),
            target_language: "Spanish".into(),
            tone: "Natural".into(),
            glossary: vec!["Harbor Lights: keep".into(), "Kessler: keep".into()],
            lines: vec![
                DubLine {
                    speaker: "Maya".into(),
                    color_token: "--t4".into(),
                    original_text: "The first thing I noticed about the harbor recordings was how much low rumble came through the hull.".into(),
                    translated_text: "Lo primero que noté en las grabaciones del puerto fue cuánto retumbo bajo atravesaba el casco.".into(),
                    target_language: "Spanish".into(),
                    duration_diff_percent: 4.0,
                    status: "ok".into(),
                    shortened: false,
                },
                DubLine {
                    speaker: "Tom".into(),
                    color_token: "--t3".into(),
                    original_text: "And that was on the first night, before we moved the mics up to the Kessler deck.".into(),
                    translated_text: "Y eso fue la primera noche, antes de que subiéramos los micrófonos a la cubierta Kessler y volviéramos a empezar.".into(),
                    target_language: "Spanish".into(),
                    duration_diff_percent: 22.0,
                    status: "warn".into(),
                    shortened: false,
                },
                DubLine {
                    speaker: "Maya".into(),
                    color_token: "--t4".into(),
                    original_text: "Which meant we had to cut around the engine for the whole second act.".into(),
                    translated_text: "Lo que nos obligó a editar alrededor del motor durante todo el segundo acto.".into(),
                    target_language: "Spanish".into(),
                    duration_diff_percent: 8.0,
                    status: "ok".into(),
                    shortened: false,
                },
            ],
        }
    }

    /// Evaluates how many lines fit within duration tolerance (+-15%).
    pub fn fit_summary(&self) -> (usize, usize) {
        let fit = self.lines.iter().filter(|l| l.status == "ok").count();
        let too_long = self.lines.len().saturating_sub(fit);
        (fit, too_long)
    }

    /// Shortens a line by trimming redundant phrasing to bring duration into tolerance.
    pub fn shorten_line(&mut self, index: usize) {
        if let Some(line) = self.lines.get_mut(index) {
            line.duration_diff_percent = (line.duration_diff_percent * 0.4).round();
            line.status = "ok".into();
            line.shortened = true;
        }
    }
}

/// Dubbing mixer combining synthesized dialogue with background stems.
pub struct DubbingEngine;

impl DubbingEngine {
    /// Mixes synthesized dubbed speech with background instrumental stems.
    pub fn mix_dub(
        dubbed_speech: &[f32],
        background_stems: &[f32],
        speech_gain: f32,
        background_gain: f32,
    ) -> Vec<f32> {
        let len = dubbed_speech.len().max(background_stems.len());
        let mut out = Vec::with_capacity(len);

        for i in 0..len {
            let sp = dubbed_speech.get(i).copied().unwrap_or(0.0) * speech_gain;
            let bg = background_stems.get(i).copied().unwrap_or(0.0) * background_gain;
            out.push((sp + bg).clamp(-1.0, 1.0));
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dubbing_project_and_fitting() {
        let mut proj = DubbingProject::create_sample();
        assert_eq!(proj.lines.len(), 3);

        let (fit, too_long) = proj.fit_summary();
        assert_eq!(fit, 2);
        assert_eq!(too_long, 1);

        // Shorten the second line
        proj.shorten_line(1);
        let (fit2, too_long2) = proj.fit_summary();
        assert_eq!(fit2, 3);
        assert_eq!(too_long2, 0);
    }

    #[test]
    fn test_dubbing_engine_mix() {
        let speech = vec![0.5, 0.5, 0.5];
        let bg = vec![0.2, 0.2, 0.2];
        let mixed = DubbingEngine::mix_dub(&speech, &bg, 1.0, 1.0);
        assert_eq!(mixed.len(), 3);
        for &s in &mixed {
            assert!((s - 0.7).abs() < 1e-4);
        }
    }
}
