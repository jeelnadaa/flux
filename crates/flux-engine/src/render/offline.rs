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
        progress_callback: Option<&mut dyn FnMut(f32)>,
    ) -> Result<AudioBuffer, &'static str> {
        Self::render_with_tail(
            graph,
            input,
            output_channels,
            block_size,
            0,
            cancel_token,
            progress_callback,
        )
    }

    /// Renders an audio input through the graph to an output buffer, including tail frames for reverb/delay decay.
    pub fn render_with_tail(
        graph: &mut AudioGraph,
        input: &AudioBuffer,
        output_channels: usize,
        block_size: usize,
        tail_frames: usize,
        cancel_token: Option<&AtomicBool>,
        mut progress_callback: Option<&mut dyn FnMut(f32)>,
    ) -> Result<AudioBuffer, &'static str> {
        let total_frames = input.num_frames();
        let grand_total = total_frames + tail_frames;
        let in_channels = input.channels.len();
        graph.reset();

        let mut output_channels_vec = vec![Vec::with_capacity(grand_total); output_channels];
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
                let progress = frame_cursor as f32 / grand_total.max(1) as f32;
                cb(progress);
            }
        }

        // Render tail frames if requested (feeds silence into graph for natural ring-out)
        let mut tail_rendered = 0;
        while tail_rendered < tail_frames {
            if let Some(token) = cancel_token {
                if token.load(Ordering::Relaxed) {
                    return Err("Render cancelled by user");
                }
            }

            let frames_this_block = (tail_frames - tail_rendered).min(block_size);
            for ch in in_block.iter_mut().take(in_channels) {
                ch[..frames_this_block].fill(0.0);
            }

            let in_refs: Vec<&[f32]> = in_block.iter().map(|ch| &ch[..frames_this_block]).collect();
            let mut out_refs: Vec<&mut [f32]> = out_block
                .iter_mut()
                .map(|ch| &mut ch[..frames_this_block])
                .collect();

            graph.process(&in_refs, &mut out_refs);

            for ch in 0..output_channels {
                output_channels_vec[ch].extend_from_slice(&out_block[ch][..frames_this_block]);
            }

            tail_rendered += frames_this_block;

            if let Some(cb) = progress_callback.as_deref_mut() {
                let progress = (total_frames + tail_rendered) as f32 / grand_total.max(1) as f32;
                cb(progress);
            }
        }

        Ok(AudioBuffer {
            channels: output_channels_vec,
            sample_rate: input.sample_rate,
        })
    }
}
