//! Circular pre-roll buffer storing the last N seconds of audio prior to recording.

/// Circular pre-roll buffer for capturing audio right before record punch-in.
#[derive(Debug, Clone)]
pub struct PreRollBuffer {
    channels: Vec<Vec<f32>>,
    capacity_frames: usize,
    write_pos: usize,
    total_buffered: usize,
}

impl PreRollBuffer {
    /// Creates a new `PreRollBuffer` for `num_channels` with `duration_seconds` at `sample_rate`.
    pub fn new(num_channels: usize, sample_rate: u32, duration_seconds: f64) -> Self {
        let capacity_frames = ((sample_rate as f64) * duration_seconds.max(0.1)).round() as usize;
        let channels = vec![vec![0.0f32; capacity_frames]; num_channels];

        Self {
            channels,
            capacity_frames,
            write_pos: 0,
            total_buffered: 0,
        }
    }

    /// Appends incoming audio frames to the circular pre-roll buffer.
    pub fn append(&mut self, inputs: &[&[f32]]) {
        if inputs.is_empty() || self.capacity_frames == 0 {
            return;
        }

        let num_frames = inputs[0].len();
        let num_channels = inputs.len().min(self.channels.len());

        for frame in 0..num_frames {
            let pos = (self.write_pos + frame) % self.capacity_frames;
            for (ch, input_ch) in inputs.iter().enumerate().take(num_channels) {
                self.channels[ch][pos] = input_ch[frame];
            }
        }

        self.write_pos = (self.write_pos + num_frames) % self.capacity_frames;
        self.total_buffered = (self.total_buffered + num_frames).min(self.capacity_frames);
    }

    /// Returns the number of frames currently buffered in pre-roll.
    pub fn len(&self) -> usize {
        self.total_buffered
    }

    /// Returns whether the pre-roll buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.total_buffered == 0
    }

    /// Returns the maximum capacity of the pre-roll buffer in frames.
    pub fn capacity(&self) -> usize {
        self.capacity_frames
    }

    /// Drains all buffered pre-roll audio chronologically (oldest to newest) and clears the buffer.
    pub fn drain(&mut self) -> Vec<Vec<f32>> {
        let n_ch = self.channels.len();
        let count = self.total_buffered;
        if count == 0 {
            return vec![Vec::new(); n_ch];
        }

        let mut output = vec![Vec::with_capacity(count); n_ch];
        let start_pos = if self.total_buffered == self.capacity_frames {
            self.write_pos
        } else {
            0
        };

        for i in 0..count {
            let pos = (start_pos + i) % self.capacity_frames;
            for (ch, out_ch) in output.iter_mut().enumerate().take(n_ch) {
                out_ch.push(self.channels[ch][pos]);
            }
        }

        self.total_buffered = 0;
        self.write_pos = 0;

        output
    }

    /// Clears the pre-roll buffer without returning data.
    pub fn clear(&mut self) {
        self.total_buffered = 0;
        self.write_pos = 0;
    }
}
