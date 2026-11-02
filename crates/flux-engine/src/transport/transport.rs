//! Playback transport, sample-accurate loop points, and variable-speed scrubbing.

/// Playback transport state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportState {
    /// Playback is stopped and playhead rests at start.
    Stopped,
    /// Real-time playback is active.
    Playing,
    /// Playback is paused at current position.
    Paused,
    /// Scrubbing is active (hear while dragging playhead).
    Scrubbing,
}

/// Sample-accurate playback transport controller.
#[derive(Debug, Clone)]
pub struct Transport {
    state: TransportState,
    playhead_frame: u64,
    sample_rate: u32,
    // Looping
    loop_enabled: bool,
    loop_start_frame: u64,
    loop_end_frame: u64,
    loop_crossfade_frames: usize,
    // Scrubbing
    scrub_speed: f32,
    // Total project duration bounds
    total_frames: u64,
}

impl Transport {
    /// Creates a new transport controller.
    pub fn new(sample_rate: u32) -> Self {
        Self {
            state: TransportState::Stopped,
            playhead_frame: 0,
            sample_rate,
            loop_enabled: false,
            loop_start_frame: 0,
            loop_end_frame: 0,
            loop_crossfade_frames: 128, // ~2.6 ms at 48 kHz micro-fade
            scrub_speed: 1.0,
            total_frames: 0,
        }
    }

    /// Returns the sample rate.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Sets the loop crossfade length in frames.
    pub fn set_loop_crossfade_frames(&mut self, frames: usize) {
        self.loop_crossfade_frames = frames;
    }

    /// Returns the loop crossfade length in frames.
    pub fn loop_crossfade_frames(&self) -> usize {
        self.loop_crossfade_frames
    }

    /// Sets the total timeline length in frames.
    pub fn set_total_frames(&mut self, frames: u64) {
        self.total_frames = frames;
    }

    /// Returns current state.
    pub fn state(&self) -> TransportState {
        self.state
    }

    /// Starts or resumes playback.
    pub fn play(&mut self) {
        self.state = TransportState::Playing;
    }

    /// Pauses playback.
    pub fn pause(&mut self) {
        self.state = TransportState::Paused;
    }

    /// Stops playback and returns playhead to start or loop start.
    pub fn stop(&mut self) {
        self.state = TransportState::Stopped;
        self.playhead_frame = if self.loop_enabled {
            self.loop_start_frame
        } else {
            0
        };
    }

    /// Moves playhead to a sample-accurate frame.
    pub fn seek(&mut self, frame: u64) {
        if self.total_frames > 0 {
            self.playhead_frame = frame.min(self.total_frames);
        } else {
            self.playhead_frame = frame;
        }
    }

    /// Configures the loop region.
    pub fn set_loop(&mut self, enabled: bool, start: u64, end: u64) {
        self.loop_enabled = enabled;
        if start < end {
            self.loop_start_frame = start;
            self.loop_end_frame = end;
        } else {
            self.loop_start_frame = end;
            self.loop_end_frame = start;
        }
    }

    /// Enables variable-speed scrubbing.
    pub fn set_scrub_speed(&mut self, speed: f32) {
        self.scrub_speed = speed.clamp(-4.0, 4.0);
        if (speed).abs() > 1e-4 {
            self.state = TransportState::Scrubbing;
        } else {
            self.state = TransportState::Paused;
        }
    }

    /// Returns the current playhead frame.
    pub fn playhead(&self) -> u64 {
        self.playhead_frame
    }

    /// Whether loop playback is currently active.
    pub fn is_looping(&self) -> bool {
        self.loop_enabled
    }

    /// Loop start frame.
    pub fn loop_start(&self) -> u64 {
        self.loop_start_frame
    }

    /// Loop end frame.
    pub fn loop_end(&self) -> u64 {
        self.loop_end_frame
    }

    /// Advances the transport playhead by `frames`, seamlessly wrapping around loop boundaries.
    /// Returns the slice of frames rendered `(from_frame, to_frame)`.
    pub fn advance(&mut self, frames: usize) -> (u64, u64) {
        if self.state != TransportState::Playing && self.state != TransportState::Scrubbing {
            return (self.playhead_frame, self.playhead_frame);
        }

        let start = self.playhead_frame;
        let step = if self.state == TransportState::Scrubbing {
            (frames as f32 * self.scrub_speed).round() as i64
        } else {
            frames as i64
        };

        if self.loop_enabled && self.loop_end_frame > self.loop_start_frame {
            let loop_len = (self.loop_end_frame - self.loop_start_frame) as i64;
            let mut next = start as i64 + step;

            if next >= self.loop_end_frame as i64 {
                let over = next - self.loop_end_frame as i64;
                next = self.loop_start_frame as i64 + (over % loop_len);
            } else if next < self.loop_start_frame as i64 {
                let under = self.loop_start_frame as i64 - next;
                next = self.loop_end_frame as i64 - (under % loop_len);
            }

            self.playhead_frame = next.max(0) as u64;
        } else {
            let next = (start as i64 + step).max(0) as u64;
            if self.total_frames > 0 {
                self.playhead_frame = next.min(self.total_frames);
                if self.playhead_frame >= self.total_frames {
                    self.state = TransportState::Stopped;
                }
            } else {
                self.playhead_frame = next;
            }
        }

        (start, self.playhead_frame)
    }
}
