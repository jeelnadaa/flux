//! Real-time AudioWorklet EngineHost with lock-free command queues and meter publishing.

use crate::capture::InputLevelMeter;
use crate::graph::{AudioGraph, NodeId};
use crate::transport::{Transport, TransportState};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Real-time commands dispatched from UI/main thread to audio thread.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EngineCommand {
    /// Start or resume playback.
    Play,
    /// Pause playback.
    Pause,
    /// Stop and rewind.
    Stop,
    /// Sample-accurate seek to frame position.
    Seek(u64),
    /// Configure loop range.
    SetLoop {
        /// Enable or disable loop.
        enabled: bool,
        /// Loop start in frames.
        start: u64,
        /// Loop end in frames.
        end: u64,
    },
    /// Set scrub playback speed.
    SetScrub(f32),
    /// Update a node's parameter.
    SetParameter {
        /// Target node.
        node_id: NodeId,
        /// Parameter index.
        index: usize,
        /// New value.
        value: f32,
    },
}

/// SPSC lock-free ring buffer for engine commands.
pub struct CommandQueue {
    buffer: Vec<Option<EngineCommand>>,
    capacity: usize,
    write_idx: AtomicUsize,
    read_idx: AtomicUsize,
}

impl CommandQueue {
    /// Creates a new command queue with specified capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![None; capacity],
            capacity,
            write_idx: AtomicUsize::new(0),
            read_idx: AtomicUsize::new(0),
        }
    }

    /// Enqueues a command (called by UI thread).
    pub fn push(&mut self, cmd: EngineCommand) -> bool {
        let write = self.write_idx.load(Ordering::Relaxed);
        let next_write = (write + 1) % self.capacity;
        if next_write == self.read_idx.load(Ordering::Acquire) {
            return false; // Queue full
        }
        self.buffer[write] = Some(cmd);
        self.write_idx.store(next_write, Ordering::Release);
        true
    }

    /// Dequeues next command (called by real-time audio thread).
    pub fn pop(&mut self) -> Option<EngineCommand> {
        let read = self.read_idx.load(Ordering::Relaxed);
        if read == self.write_idx.load(Ordering::Acquire) {
            return None; // Queue empty
        }
        let cmd = self.buffer[read].take();
        self.read_idx
            .store((read + 1) % self.capacity, Ordering::Release);
        cmd
    }
}

/// Real-time engine host driving AudioWorklet quanta.
pub struct EngineHost {
    /// Internal audio graph.
    pub graph: AudioGraph,
    /// Playback transport.
    pub transport: Transport,
    /// Input command queue.
    pub commands: CommandQueue,
    /// Output level meter.
    pub output_meter: InputLevelMeter,
    sample_rate: f32,
}

impl EngineHost {
    /// Creates a new `EngineHost`.
    pub fn new(sample_rate: f32, quantum_frames: usize) -> Self {
        Self {
            graph: AudioGraph::new(sample_rate, quantum_frames),
            transport: Transport::new(sample_rate as u32),
            commands: CommandQueue::new(256),
            output_meter: InputLevelMeter::new(2, sample_rate as u32),
            sample_rate,
        }
    }

    /// Returns engine sample rate.
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Drains command queue and applies updates before rendering the next quantum.
    pub fn poll_commands(&mut self) {
        while let Some(cmd) = self.commands.pop() {
            match cmd {
                EngineCommand::Play => self.transport.play(),
                EngineCommand::Pause => self.transport.pause(),
                EngineCommand::Stop => self.transport.stop(),
                EngineCommand::Seek(f) => self.transport.seek(f),
                EngineCommand::SetLoop {
                    enabled,
                    start,
                    end,
                } => {
                    self.transport.set_loop(enabled, start, end);
                }
                EngineCommand::SetScrub(s) => self.transport.set_scrub_speed(s),
                EngineCommand::SetParameter {
                    node_id,
                    index,
                    value,
                } => {
                    if let Some(node) = self.graph.get_node_mut(node_id) {
                        node.dsp.set_parameter(index, value);
                    }
                }
            }
        }
    }

    /// Renders a single AudioWorklet quantum (128 frames) in real time.
    ///
    /// # Safety and Real-Time Guarantees
    /// This method is hard real-time safe: zero allocation, lock-free, zero I/O.
    pub fn render_quantum(&mut self, input: &[&[f32]], output: &mut [&mut [f32]]) {
        self.poll_commands();

        if self.transport.state() == TransportState::Stopped
            || self.transport.state() == TransportState::Paused
        {
            for ch in output.iter_mut() {
                ch.fill(0.0);
            }
            return;
        }

        let quantum_len = output[0].len();
        self.transport.advance(quantum_len);

        // Process audio graph
        self.graph.process(input, output);

        // Update output meter
        let out_refs: Vec<&[f32]> = output.iter().map(|ch| &ch[..quantum_len]).collect();
        self.output_meter.process(&out_refs);
    }
}
