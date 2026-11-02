//! Real-time input level meter with true-peak approximation, RMS, and clip hold.

/// Channel meter reading with instantaneous and held values.
#[derive(Debug, Clone, PartialEq)]
pub struct ChannelMeter {
    /// Instantaneous peak linear amplitude [0.0, 1.0+].
    pub peak_linear: f32,
    /// Instantaneous peak in dBFS (-inf to 0.0+ dB).
    pub peak_db: f32,
    /// Root-mean-square (RMS) linear amplitude.
    pub rms_linear: f32,
    /// RMS level in dBFS.
    pub rms_db: f32,
    /// Whether any sample in the current block clipped (>= 0.9999).
    pub clipped: bool,
    /// Whether a clip occurred and is currently being held.
    pub clip_held: bool,
    /// Current peak-hold level in dBFS.
    pub peak_hold_db: f32,
}

/// Input level meter for multi-channel audio capture.
#[derive(Debug, Clone)]
pub struct InputLevelMeter {
    num_channels: usize,
    sample_rate: u32,
    clip_hold_samples: u32,
    clip_counters: Vec<u32>,
    peak_hold_db: Vec<f32>,
    decay_rate_db_per_sec: f32,
    threshold_clip: f32,
}

impl InputLevelMeter {
    /// Creates a new `InputLevelMeter` for `num_channels` at `sample_rate`.
    pub fn new(num_channels: usize, sample_rate: u32) -> Self {
        let hold_ms = 1500; // Hold clip indicator for 1.5s
        let clip_hold_samples = (sample_rate as f64 * hold_ms as f64 / 1000.0) as u32;

        Self {
            num_channels,
            sample_rate,
            clip_hold_samples,
            clip_counters: vec![0; num_channels],
            peak_hold_db: vec![-120.0; num_channels],
            decay_rate_db_per_sec: 20.0, // standard 20 dB/s peak decay
            threshold_clip: 0.9999,
        }
    }

    /// Sets the clip hold duration in milliseconds.
    pub fn set_clip_hold_ms(&mut self, ms: u32) {
        self.clip_hold_samples = (self.sample_rate as f64 * ms as f64 / 1000.0) as u32;
    }

    /// Manually clears all held clip indicators.
    pub fn clear_clips(&mut self) {
        self.clip_counters.fill(0);
    }

    /// Processes an audio frame block and returns the updated meter readings per channel.
    pub fn process(&mut self, channels: &[&[f32]]) -> Vec<ChannelMeter> {
        let n_ch = channels.len().min(self.num_channels);
        let mut readings = Vec::with_capacity(n_ch);

        let dt_sec = if n_ch > 0 && !channels[0].is_empty() {
            channels[0].len() as f32 / self.sample_rate as f32
        } else {
            0.0
        };
        let decay_amount = self.decay_rate_db_per_sec * dt_sec;

        for (ch, &samples) in channels.iter().enumerate().take(n_ch) {
            if samples.is_empty() {
                readings.push(ChannelMeter {
                    peak_linear: 0.0,
                    peak_db: -120.0,
                    rms_linear: 0.0,
                    rms_db: -120.0,
                    clipped: false,
                    clip_held: self.clip_counters[ch] > 0,
                    peak_hold_db: self.peak_hold_db[ch],
                });
                continue;
            }

            let mut peak_lin = 0.0f32;
            let mut sum_sq = 0.0f64;
            let mut clipped = false;

            for &s in samples {
                let abs_s = s.abs();
                if abs_s > peak_lin {
                    peak_lin = abs_s;
                }
                if abs_s >= self.threshold_clip {
                    clipped = true;
                }
                sum_sq += (s as f64) * (s as f64);
            }

            let rms_lin = (sum_sq / samples.len() as f64).sqrt() as f32;

            let peak_db = if peak_lin > 1e-6 {
                20.0 * peak_lin.log10()
            } else {
                -120.0
            };

            let rms_db = if rms_lin > 1e-6 {
                20.0 * rms_lin.log10()
            } else {
                -120.0
            };

            // Clip hold handling
            if clipped {
                self.clip_counters[ch] = self.clip_hold_samples;
            } else if self.clip_counters[ch] > 0 {
                self.clip_counters[ch] =
                    self.clip_counters[ch].saturating_sub(samples.len() as u32);
            }
            let clip_held = self.clip_counters[ch] > 0;

            // Peak hold decay
            let prev_hold = self.peak_hold_db[ch] - decay_amount;
            let new_hold = prev_hold.max(peak_db);
            self.peak_hold_db[ch] = new_hold;

            readings.push(ChannelMeter {
                peak_linear: peak_lin,
                peak_db,
                rms_linear: rms_lin,
                rms_db,
                clipped,
                clip_held,
                peak_hold_db: new_hold,
            });
        }

        readings
    }
}
