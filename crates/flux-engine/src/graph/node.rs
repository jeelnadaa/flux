//! Graph node identifiers and wrapping structure.

use flux_dsp::DspNode;

/// Unique identifier for a node within an audio graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub usize);

/// A wrapper around a DSP processing node in the graph.
pub struct EngineNode {
    /// Unique identifier in graph.
    pub id: NodeId,
    /// Human-readable name.
    pub name: String,
    /// Underlying DSP processing implementation.
    pub dsp: Box<dyn DspNode>,
    /// Number of audio input channels.
    pub input_channels: usize,
    /// Number of audio output channels.
    pub output_channels: usize,
    /// Pre-allocated latency compensation delay (samples).
    pub compensation_delay: usize,
}

impl EngineNode {
    /// Creates a new engine node wrapping a `DspNode`.
    pub fn new(
        id: NodeId,
        name: impl Into<String>,
        dsp: Box<dyn DspNode>,
        channels: usize,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            dsp,
            input_channels: channels,
            output_channels: channels,
            compensation_delay: 0,
        }
    }

    /// Prepares node with sample rate and buffer dimensions.
    pub fn prepare(&mut self, sample_rate: f32) {
        self.dsp
            .prepare(sample_rate, self.input_channels.max(self.output_channels));
    }

    /// Returns processing latency reported by the DSP node plus compensation delay.
    pub fn total_latency(&self) -> usize {
        self.dsp.latency_samples() + self.compensation_delay
    }
}
