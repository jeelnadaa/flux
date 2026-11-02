//! Core DSP node interfaces and reference node implementations.

/// Common trait implemented by all audio processing nodes and effects in Flux.
pub trait DspNode: Send {
    /// Name or type identifier of the node.
    fn name(&self) -> &str;

    /// Prepares the node for processing with a specific sample rate and channel layout.
    fn prepare(&mut self, sample_rate: f32, channels: usize);

    /// Processes audio frames from multi-channel input to output buffers.
    ///
    /// # Safety and Real-Time Guarantees
    /// This method runs in the real-time audio thread callback:
    /// - MUST NOT allocate or deallocate memory on the heap.
    /// - MUST NOT acquire blocking locks, access disk I/O, or perform logging.
    fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]]);

    /// Resets all internal delay buffers, filter memories, and state machines.
    fn reset(&mut self);

    /// Reports processing latency in samples (used for automatic delay compensation).
    fn latency_samples(&self) -> usize {
        0
    }

    /// Reports effect tail duration in samples (e.g. reverb or delay decay).
    fn tail_samples(&self) -> usize {
        0
    }

    /// Sets parameter value by index.
    fn set_parameter(&mut self, index: usize, value: f32);

    /// Gets parameter value by index.
    fn get_parameter(&self, index: usize) -> f32;

    /// Total number of exposed automatable parameters.
    fn parameter_count(&self) -> usize {
        0
    }

    /// Toggles bypass state. When bypassed, the node passes input to output bit-exactly.
    fn set_bypass(&mut self, bypassed: bool);

    /// Queries current bypass state.
    fn is_bypassed(&self) -> bool;
}

/// Bit-exact passthrough DSP node used for testing and baseline nulls.
#[derive(Debug, Default, Clone)]
pub struct PassthroughNode {
    bypassed: bool,
}

impl PassthroughNode {
    /// Creates a new passthrough node.
    pub fn new() -> Self {
        Self::default()
    }
}

impl DspNode for PassthroughNode {
    fn name(&self) -> &str {
        "Passthrough"
    }

    fn prepare(&mut self, _sample_rate: f32, _channels: usize) {}

    fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]]) {
        let chs = input.len().min(output.len());
        for ch in 0..chs {
            let frames = input[ch].len().min(output[ch].len());
            output[ch][..frames].copy_from_slice(&input[ch][..frames]);
        }
    }

    fn reset(&mut self) {}

    fn set_parameter(&mut self, _index: usize, _value: f32) {}

    fn get_parameter(&self, _index: usize) -> f32 {
        0.0
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }
}

/// Gain node with smoothed linear volume parameter and bypass control.
#[derive(Debug, Clone)]
pub struct GainNode {
    target_gain: f32,
    current_gain: f32,
    smoothing_factor: f32,
    bypassed: bool,
}

impl Default for GainNode {
    fn default() -> Self {
        Self {
            target_gain: 1.0,
            current_gain: 1.0,
            smoothing_factor: 0.05,
            bypassed: false,
        }
    }
}

impl GainNode {
    /// Creates a gain node with specified initial linear gain.
    pub fn new(gain: f32) -> Self {
        Self {
            target_gain: gain,
            current_gain: gain,
            ..Self::default()
        }
    }
}

impl DspNode for GainNode {
    fn name(&self) -> &str {
        "Gain"
    }

    fn prepare(&mut self, sample_rate: f32, _channels: usize) {
        // Approximate 10 ms smoothing time constant
        let time_constant = 0.01;
        self.smoothing_factor = 1.0 - (-1.0 / (time_constant * sample_rate)).exp();
    }

    fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]]) {
        let chs = input.len().min(output.len());
        if chs == 0 {
            return;
        }

        let frames = input[0].len().min(output[0].len());
        if self.bypassed {
            for ch in 0..chs {
                output[ch][..frames].copy_from_slice(&input[ch][..frames]);
            }
            return;
        }

        for i in 0..frames {
            // Parameter smoothing ramp per sample
            self.current_gain += (self.target_gain - self.current_gain) * self.smoothing_factor;
            let g = self.current_gain;

            for ch in 0..chs {
                output[ch][i] = input[ch][i] * g;
            }
        }
    }

    fn reset(&mut self) {
        self.current_gain = self.target_gain;
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        if index == 0 {
            self.target_gain = value.max(0.0);
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        if index == 0 {
            self.target_gain
        } else {
            0.0
        }
    }

    fn parameter_count(&self) -> usize {
        1
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }
}
