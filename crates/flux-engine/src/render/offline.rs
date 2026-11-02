//! Deterministic offline audio renderer using the unified DSP graph.

use crate::decoder::AudioBuffer;
use crate::graph::AudioGraph;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Progress and cancellation state for an offline render job.
pub struct RenderProgress {
    /// Cancellation flag.
    pub is_cancelled: Arc<AtomicBool>,
    /// Last reported progress ratio (0.0 to 1.0).
    pub current_progress: f32,
}

/// Offline audio renderer that runs the graph deterministically without real-time constraints.
pub struct OfflineRenderer;

impl OfflineRenderer {
    /// Renders an audio input through the graph to an output buffer.
    ///
    /// # Arguments
    /// * `graph`: Audio graph to execute.
    /// * `input`: Input multi-channel audio buffer.
    /// * `output_channels`: Desired output channel count.
    /// * `block_size`: Frame block size for internal chunk processing.
    /// * `cancel_token`: Optional cancellation flag.
    /// * `progress_callback`: Optional callback receiving progress ratio in `0.0..=1.0`.
    pub fn render(
        graph: &mut AudioGraph,
        input: &AudioBuffer,
        output_channels: usize,
        block_size: usize,
        cancel_token: Option<&AtomicBool>,
        mut progress_callback: Option<&mut dyn FnMut(f32)>,
    ) -> Result<AudioBuffer, &'static str> {
        let total_frames = input.num_frames();
        let in_channels = input.channels.len();
        graph.reset();

        let mut output_channels_vec = vec![Vec::with_capacity(total_frames); output_channels];
        let mut frame_cursor = 0;

        let mut in_block = vec![vec![0.0f32; block_size]; in_channels];
        let mut out_block = vec![vec![0.0f32; block_size]; output_channels];

        while frame_cursor < total_frames {
            if let Some(token) = cancel_token {
                if token.load(Ordering::Relaxed) {
                    return Err("Render cancelled by user");
                }
            }

            let frames_this_block = (total_frames - frame_cursor).min(block_size);

            // Copy input slice
            for (ch, in_ch) in in_block.iter_mut().enumerate().take(in_channels) {
                in_ch[..frames_this_block].copy_from_slice(
                    &input.channels[ch][frame_cursor..frame_cursor + frames_this_block],
                );
            }

            // Prepare slice refs
            let in_refs: Vec<&[f32]> = in_block.iter().map(|ch| &ch[..frames_this_block]).collect();
            let mut out_refs: Vec<&mut [f32]> = out_block
                .iter_mut()
                .map(|ch| &mut ch[..frames_this_block])
                .collect();

            // Execute graph
            graph.process(&in_refs, &mut out_refs);

            // Append to output
            for ch in 0..output_channels {
                output_channels_vec[ch].extend_from_slice(&out_block[ch][..frames_this_block]);
            }

            frame_cursor += frames_this_block;

            if let Some(cb) = progress_callback.as_deref_mut() {
                let progress = frame_cursor as f32 / total_frames as f32;
                cb(progress);
            }
        }

        Ok(AudioBuffer {
            channels: output_channels_vec,
            sample_rate: input.sample_rate,
        })
    }
}
