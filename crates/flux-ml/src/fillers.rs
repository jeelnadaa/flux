//! Filler word and disfluency detection (um, uh, like, you know, stutters).

use serde::{Deserialize, Serialize};

/// Category of vocal filler or disfluency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FillerType {
    /// Hesitation sounds (um, uh, er, ah).
    UmUh,
    /// Discourse marker "like" (contextually distinguished from verb/preposition).
    Like,
    /// Conversational tag "you know" / "I mean".
    YouKnow,
    /// Syllable or word repetitions ("the the", "I I I").
    Stutter,
    /// User-defined custom filler word.
    Custom,
}

/// A detected filler word occurrence with contextual transcript excerpt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillerOccurrence {
    /// Unique identifier for the occurrence.
    pub id: String,
    /// Highlighted context excerpt (e.g. "So <b class=\"sig\">um</b> the first thing").
    pub context_html: String,
    /// Formatted timestamp (e.g. "00:12:04").
    pub timestamp_str: String,
    /// Time position in seconds.
    pub timestamp_sec: f32,
    /// Duration of the filler in seconds.
    pub duration_sec: f32,
    /// Type of filler.
    pub filler_type: FillerType,
    /// Category label string (e.g. "Um", "Stutter", "Like, not sure").
    pub label: String,
    /// Whether this item is currently selected for removal.
    pub selected: bool,
    /// Confidence score of the detection.
    pub confidence: f32,
}

/// Configurable detector for disfluencies and filler words.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillerDetectorConfig {
    /// Sensitivity threshold (0.0 to 1.0, default 0.60).
    pub sensitivity: f32,
    /// Whether to preserve natural speech breaths across cuts.
    pub keep_natural_breaths: bool,
    /// Additional user-specified custom filler words.
    pub custom_words: Vec<String>,
}

impl Default for FillerDetectorConfig {
    fn default() -> Self {
        Self {
            sensitivity: 0.60,
            keep_natural_breaths: true,
            custom_words: Vec::new(),
        }
    }
}

/// Engine detecting disfluencies and repetitions across transcript tokens.
pub struct FillerDetector;

impl FillerDetector {
    /// Detects filler occurrences matching the prototype review list in `#fillers`.
    pub fn get_sample_detections() -> Vec<FillerOccurrence> {
        vec![
            FillerOccurrence {
                id: "f1".into(),
                context_html: "So <b class=\"sig\">um</b> the first thing".into(),
                timestamp_str: "00:12:04".into(),
                timestamp_sec: 724.35,
                duration_sec: 0.30,
                filler_type: FillerType::UmUh,
                label: "Um".into(),
                selected: true,
                confidence: 0.96,
            },
            FillerOccurrence {
                id: "f2".into(),
                context_html: "on <b class=\"sig\">the the</b> first night".into(),
                timestamp_str: "00:12:33".into(),
                timestamp_sec: 753.12,
                duration_sec: 0.42,
                filler_type: FillerType::Stutter,
                label: "Stutter".into(),
                selected: true,
                confidence: 0.98,
            },
            FillerOccurrence {
                id: "f3".into(),
                context_html: "first night, <b class=\"sig\">you know,</b> before we".into(),
                timestamp_str: "00:12:36".into(),
                timestamp_sec: 756.10,
                duration_sec: 0.45,
                filler_type: FillerType::YouKnow,
                label: "You know".into(),
                selected: true,
                confidence: 0.91,
            },
            FillerOccurrence {
                id: "f4".into(),
                context_html: "it was <b class=\"sig\">uh</b> around two".into(),
                timestamp_str: "00:14:02".into(),
                timestamp_sec: 842.20,
                duration_sec: 0.28,
                filler_type: FillerType::UmUh,
                label: "Uh".into(),
                selected: true,
                confidence: 0.95,
            },
            FillerOccurrence {
                id: "f5".into(),
                context_html: "we were <b class=\"sig\">like</b> forty feet away".into(),
                timestamp_str: "00:15:18".into(),
                timestamp_sec: 918.40,
                duration_sec: 0.25,
                filler_type: FillerType::Like,
                label: "Like, not sure".into(),
                selected: false, // kept by default because "like" carries approximate quantifier meaning
                confidence: 0.62,
            },
            FillerOccurrence {
                id: "f6".into(),
                context_html: "and <b class=\"sig\">I I I</b> think that".into(),
                timestamp_str: "00:17:44".into(),
                timestamp_sec: 1064.15,
                duration_sec: 0.55,
                filler_type: FillerType::Stutter,
                label: "Stutter".into(),
                selected: true,
                confidence: 0.99,
            },
        ]
    }

    /// Computes total time saved in seconds by removing selected filler occurrences.
    pub fn calculate_time_saved(occurrences: &[FillerOccurrence]) -> f32 {
        occurrences
            .iter()
            .filter(|o| o.selected)
            .map(|o| o.duration_sec)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filler_detections_and_time_saved() {
        let detections = FillerDetector::get_sample_detections();
        assert_eq!(detections.len(), 6);

        let time_saved = FillerDetector::calculate_time_saved(&detections);
        // 0.30 + 0.42 + 0.45 + 0.28 + 0.55 = 2.00s (f5 is not selected)
        assert!((time_saved - 2.00).abs() < 1e-3);
    }
}
