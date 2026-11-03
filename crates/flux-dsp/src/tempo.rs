//! Tempo mapping, tap tempo detection, and beat grid alignment.
//!
//! Provides sample-accurate conversion between audio sample positions and musical
//! beat subdivisions, dynamic tempo curves, tap tempo tracking, and rhythmic grid generation.

/// A point along the timeline defining tempo and meter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TempoPoint {
    /// Sample position where this tempo point takes effect.
    pub sample_position: usize,
    /// Tempo in beats per minute (40.0 to 280.0 BPM).
    pub bpm: f32,
    /// Time signature numerator (e.g. 4 in 4/4).
    pub time_signature_num: u8,
    /// Time signature denominator (e.g. 4 in 4/4).
    pub time_signature_den: u8,
}

impl Default for TempoPoint {
    fn default() -> Self {
        Self {
            sample_position: 0,
            bpm: 120.0,
            time_signature_num: 4,
            time_signature_den: 4,
        }
    }
}

/// Sample-accurate musical tempo map.
#[derive(Debug, Clone)]
pub struct TempoMap {
    sample_rate: f32,
    points: Vec<TempoPoint>,
}

impl TempoMap {
    /// Creates a new tempo map with a constant default tempo (120 BPM, 4/4).
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate: sample_rate.max(1.0),
            points: vec![TempoPoint::default()],
        }
    }

    /// Creates a tempo map with a specific constant BPM and time signature.
    pub fn with_bpm(sample_rate: f32, bpm: f32, num: u8, den: u8) -> Self {
        Self {
            sample_rate: sample_rate.max(1.0),
            points: vec![TempoPoint {
                sample_position: 0,
                bpm: bpm.clamp(40.0, 280.0),
                time_signature_num: num.max(1),
                time_signature_den: den.max(1),
            }],
        }
    }

    /// Adds or updates a tempo point in the map.
    pub fn set_tempo_point(&mut self, point: TempoPoint) {
        let mut clean_point = point;
        clean_point.bpm = clean_point.bpm.clamp(40.0, 280.0);
        clean_point.time_signature_num = clean_point.time_signature_num.max(1);
        clean_point.time_signature_den = clean_point.time_signature_den.max(1);

        if let Some(pos) = self
            .points
            .iter()
            .position(|p| p.sample_position == clean_point.sample_position)
        {
            self.points[pos] = clean_point;
        } else {
            self.points.push(clean_point);
            self.points.sort_by_key(|p| p.sample_position);
        }
    }

    /// Returns the tempo in BPM at the specified sample position.
    pub fn bpm_at_sample(&self, sample: usize) -> f32 {
        let mut active_bpm = self.points[0].bpm;
        for p in &self.points {
            if p.sample_position <= sample {
                active_bpm = p.bpm;
            } else {
                break;
            }
        }
        active_bpm
    }

    /// Converts sample position to fractional musical beat index (0.0 = first beat).
    pub fn sample_to_beat(&self, sample: usize) -> f64 {
        let mut accumulated_beats = 0.0f64;
        let mut prev_sample = 0usize;
        let mut active_bpm = self.points[0].bpm;

        for p in &self.points {
            if p.sample_position > sample {
                break;
            }
            if p.sample_position > prev_sample {
                let delta_samples = (p.sample_position - prev_sample) as f64;
                let samples_per_beat = (60.0 * (self.sample_rate as f64)) / (active_bpm as f64);
                accumulated_beats += delta_samples / samples_per_beat;
                prev_sample = p.sample_position;
            }
            active_bpm = p.bpm;
        }

        let remaining_samples = (sample - prev_sample) as f64;
        let samples_per_beat = (60.0 * (self.sample_rate as f64)) / (active_bpm as f64);
        accumulated_beats + (remaining_samples / samples_per_beat)
    }

    /// Converts fractional musical beat index back to sample position.
    pub fn beat_to_sample(&self, beat: f64) -> usize {
        let mut remaining_beats = beat.max(0.0);
        let mut current_sample = 0usize;
        let mut active_bpm = self.points[0].bpm;

        for i in 0..self.points.len() {
            let next_point = self.points.get(i + 1);
            let samples_per_beat = (60.0 * (self.sample_rate as f64)) / (active_bpm as f64);

            if let Some(next_p) = next_point {
                let seg_samples = (next_p.sample_position - current_sample) as f64;
                let seg_beats = seg_samples / samples_per_beat;

                if remaining_beats <= seg_beats {
                    return current_sample + (remaining_beats * samples_per_beat).round() as usize;
                }

                remaining_beats -= seg_beats;
                current_sample = next_p.sample_position;
                active_bpm = next_p.bpm;
            } else {
                return current_sample + (remaining_beats * samples_per_beat).round() as usize;
            }
        }

        current_sample
    }

    /// Snaps a sample position to the nearest beat subdivision (e.g. 1.0 for whole beats, 0.25 for 1/16th notes).
    pub fn snap_to_subdivision(&self, sample: usize, subdivision_beats: f64) -> usize {
        let sub = subdivision_beats.max(0.0625);
        let current_beat = self.sample_to_beat(sample);
        let snapped_beat = (current_beat / sub).round() * sub;
        self.beat_to_sample(snapped_beat)
    }

    /// Generates beat grid sample positions within the specified sample range.
    pub fn generate_grid(
        &self,
        start_sample: usize,
        end_sample: usize,
        subdivision_beats: f64,
    ) -> Vec<usize> {
        let mut grid = Vec::new();
        let sub = subdivision_beats.max(0.0625);
        let start_beat = (self.sample_to_beat(start_sample) / sub).floor() * sub;
        let end_beat = self.sample_to_beat(end_sample);

        let mut b = start_beat;
        while b <= end_beat {
            let s = self.beat_to_sample(b);
            if s >= start_sample && s <= end_sample {
                grid.push(s);
            }
            b += sub;
        }

        grid
    }
}

/// Tap tempo detector computing moving average BPM with outlier filtering.
#[derive(Debug, Clone)]
pub struct TapTempo {
    last_tap_time_ms: f64,
    intervals_ms: Vec<f64>,
    max_history: usize,
    timeout_ms: f64,
}

impl Default for TapTempo {
    fn default() -> Self {
        Self::new()
    }
}

impl TapTempo {
    /// Creates a new tap tempo detector.
    pub fn new() -> Self {
        Self {
            last_tap_time_ms: 0.0,
            intervals_ms: Vec::with_capacity(8),
            max_history: 5,
            timeout_ms: 2500.0, // Reset after 2.5s of inactivity
        }
    }

    /// Registers a tap at the given millisecond timestamp and returns the computed BPM if enough taps exist.
    pub fn tap(&mut self, timestamp_ms: f64) -> Option<f32> {
        if self.last_tap_time_ms > 0.0 {
            let delta = timestamp_ms - self.last_tap_time_ms;
            if delta > self.timeout_ms || delta < 150.0 {
                // Timeout or double-tap bounce: reset interval history
                self.intervals_ms.clear();
            } else {
                self.intervals_ms.push(delta);
                if self.intervals_ms.len() > self.max_history {
                    self.intervals_ms.remove(0);
                }
            }
        }
        self.last_tap_time_ms = timestamp_ms;

        if self.intervals_ms.is_empty() {
            return None;
        }

        // Calculate average interval
        let avg_interval_ms: f64 =
            self.intervals_ms.iter().sum::<f64>() / (self.intervals_ms.len() as f64);
        let bpm = (60_000.0 / avg_interval_ms) as f32;
        Some(bpm.clamp(40.0, 280.0))
    }

    /// Resets the tap history.
    pub fn reset(&mut self) {
        self.last_tap_time_ms = 0.0;
        self.intervals_ms.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tempo_map_constant_bpm_exact_roundtrip() {
        let tm = TempoMap::with_bpm(48000.0, 120.0, 4, 4);

        // At 120 BPM and 48 kHz, 1 beat is 0.5 s = 24,000 samples
        assert_eq!(tm.beat_to_sample(0.0), 0);
        assert_eq!(tm.beat_to_sample(1.0), 24000);
        assert_eq!(tm.beat_to_sample(4.0), 96000);

        assert!((tm.sample_to_beat(24000) - 1.0).abs() < 1e-6);
        assert!((tm.sample_to_beat(96000) - 4.0).abs() < 1e-6);
    }

    #[test]
    fn test_tempo_map_snap_to_quarter_and_sixteenth_beats() {
        let tm = TempoMap::with_bpm(48000.0, 120.0, 4, 4);

        // Sample near beat 1.0 (e.g. 23,800 samples) snaps to 24,000
        let snapped_beat = tm.snap_to_subdivision(23800, 1.0);
        assert_eq!(snapped_beat, 24000);

        // Snap to 1/16th note (0.25 beat = 6,000 samples)
        // Sample near 6,100 snaps to 6,000
        let snapped_16th = tm.snap_to_subdivision(6100, 0.25);
        assert_eq!(snapped_16th, 6000);
    }

    #[test]
    fn test_tap_tempo_detector_computes_accurate_bpm() {
        let mut tap = TapTempo::new();

        // Simulate 4 taps at 500 ms intervals = 120 BPM
        assert_eq!(tap.tap(1000.0), None); // First tap
        let bpm1 = tap.tap(1500.0).expect("Should have BPM after 2 taps");
        assert!((bpm1 - 120.0).abs() < 0.1);

        let bpm2 = tap.tap(2000.0).unwrap();
        assert!((bpm2 - 120.0).abs() < 0.1);

        let bpm3 = tap.tap(2500.0).unwrap();
        assert!((bpm3 - 120.0).abs() < 0.1);
    }

    #[test]
    fn test_tap_tempo_detector_resets_after_timeout() {
        let mut tap = TapTempo::new();
        tap.tap(1000.0);
        tap.tap(1500.0);

        // Wait 3 seconds (> 2.5s timeout)
        let new_bpm = tap.tap(4500.0);
        assert_eq!(new_bpm, None, "Should reset after timeout");
    }
}
