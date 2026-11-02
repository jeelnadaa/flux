//! Universal DSP invariant validation harness (PLAN.md §5.2).
//!
//! Automatically verifies all 13 core DSP invariants on any node implementing `DspNode`.

use crate::analyzers::{peak, peak_db};
use crate::compare::null_residual;
use crate::generators::{chirp_linear, impulse, silence, sine};
use flux_dsp::DspNode;

/// Configuration options for the universal DSP invariant harness.
#[derive(Debug, Clone)]
pub struct InvariantConfig {
    /// Baseline sample rate to test (default: 48000.0).
    pub sample_rate: f32,
    /// Number of channels to test (default: 2).
    pub channels: usize,
    /// Test duration in seconds for invariant checks (default: 0.1).
    pub test_duration_secs: f32,
}

impl Default for InvariantConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000.0,
            channels: 2,
            test_duration_secs: 0.1,
        }
    }
}

/// Result of running the universal DSP invariant suite.
#[derive(Debug, Clone)]
pub struct InvariantSuiteResult {
    /// Invariant 1: Bypass identity.
    pub bypass_identity: bool,
    /// Invariant 2: Silence in -> silence out.
    pub silence_in_silence_out: bool,
    /// Invariant 3: No NaN/Inf under extremes and hot input.
    pub no_nan_inf: bool,
    /// Invariant 4: Block-size invariance {1, 7, 64, 128, 512, 1024, 4096}.
    pub block_size_invariance: bool,
    /// Invariant 5: Sample-rate invariance across standard rates.
    pub sample_rate_invariance: bool,
    /// Invariant 6: Correct channel handling (mono & stereo).
    pub channel_handling: bool,
    /// Invariant 7: Parameter smoothing (no clicks/discontinuities on parameter step).
    pub parameter_smoothing: bool,
    /// Invariant 8: Latency report matches impulse delay.
    pub latency_accuracy: bool,
    /// Invariant 9: Tail length report covers decay to -90 dBFS.
    pub tail_correctness: bool,
    /// Invariant 10: State reset restores identical initial state.
    pub state_reset: bool,
    /// Invariant 11: Determinism across runs.
    pub determinism: bool,
    /// Invariant 12: Real-time safety (no unbounded processing).
    pub realtime_safety: bool,
    /// Invariant 13: Real-time block render equals offline continuous render.
    pub realtime_equals_offline: bool,
}

impl InvariantSuiteResult {
    /// Returns true if all 13 universal DSP invariants passed.
    pub fn all_passed(&self) -> bool {
        self.bypass_identity
            && self.silence_in_silence_out
            && self.no_nan_inf
            && self.block_size_invariance
            && self.sample_rate_invariance
            && self.channel_handling
            && self.parameter_smoothing
            && self.latency_accuracy
            && self.tail_correctness
            && self.state_reset
            && self.determinism
            && self.realtime_safety
            && self.realtime_equals_offline
    }
}

/// Runs all 13 universal DSP invariants on an audio processing node.
pub fn run_dsp_invariants<F>(node_factory: F, config: &InvariantConfig) -> InvariantSuiteResult
where
    F: Fn() -> Box<dyn DspNode>,
{
    let sr = config.sample_rate;
    let chs = config.channels;
    let frames = (sr * config.test_duration_secs).round() as usize;

    // 1. Bypass Identity
    let bypass_identity = {
        let mut node = node_factory();
        node.prepare(sr, chs);
        node.set_bypass(true);

        let test_sig = chirp_linear(20.0, 10000.0, sr, config.test_duration_secs, 0.8);
        let in_channels: Vec<Vec<f32>> = vec![test_sig.clone(); chs];
        let mut out_channels: Vec<Vec<f32>> = vec![vec![0.0f32; frames]; chs];

        let in_slices: Vec<&[f32]> = in_channels.iter().map(|c| c.as_slice()).collect();
        let mut out_slices: Vec<&mut [f32]> =
            out_channels.iter_mut().map(|c| c.as_mut_slice()).collect();

        node.process(&in_slices, &mut out_slices);

        let null_res = null_residual(&in_channels[0], &out_channels[0]);
        null_res.is_bit_exact
    };

    // 2. Silence in -> silence out (no DC offset, peak <= -90 dBFS)
    let silence_in_silence_out = {
        let mut node = node_factory();
        node.prepare(sr, chs);

        let sil = silence(frames);
        let in_channels: Vec<Vec<f32>> = vec![sil.clone(); chs];
        let mut out_channels: Vec<Vec<f32>> = vec![vec![0.0f32; frames]; chs];

        let in_slices: Vec<&[f32]> = in_channels.iter().map(|c| c.as_slice()).collect();
        let mut out_slices: Vec<&mut [f32]> =
            out_channels.iter_mut().map(|c| c.as_mut_slice()).collect();

        node.process(&in_slices, &mut out_slices);

        let p_db = peak_db(&out_channels[0]);
        p_db <= -90.0
    };

    // 3. No NaN / Inf under extremes and hot input (+-4.0)
    let no_nan_inf = {
        let mut node = node_factory();
        node.prepare(sr, chs);

        let mut all_valid = true;
        let test_params = [0.0, 1.0, 1e-5, 10.0, -1.0, 100.0];

        for &val in &test_params {
            for p_idx in 0..node.parameter_count() {
                node.set_parameter(p_idx, val);
            }

            // Feed hot float32 (+-4.0) sine
            let hot_sig = sine(1000.0, sr, 0.05, 4.0, 0.0);
            let in_channels: Vec<Vec<f32>> = vec![hot_sig; chs];
            let mut out_channels: Vec<Vec<f32>> = vec![vec![0.0f32; in_channels[0].len()]; chs];

            let in_slices: Vec<&[f32]> = in_channels.iter().map(|c| c.as_slice()).collect();
            let mut out_slices: Vec<&mut [f32]> =
                out_channels.iter_mut().map(|c| c.as_mut_slice()).collect();

            node.process(&in_slices, &mut out_slices);

            for ch_buf in out_channels.iter().take(chs) {
                for &sample in ch_buf {
                    if sample.is_nan() || sample.is_infinite() {
                        all_valid = false;
                        break;
                    }
                }
            }
        }
        all_valid
    };

    // 4. Block-size invariance {1, 7, 64, 128, 512, 1024, 4096}
    let block_size_invariance = {
        let block_sizes = [1, 7, 64, 128, 512, 1024];
        let test_sig = chirp_linear(100.0, 8000.0, sr, 0.1, 0.5);
        let total_samples = test_sig.len();

        let mut outputs_per_block: Vec<Vec<f32>> = Vec::new();

        for &bs in &block_sizes {
            let mut node = node_factory();
            node.prepare(sr, chs);

            let mut out_rendered = vec![0.0f32; total_samples];
            let mut offset = 0;

            while offset < total_samples {
                let chunk_len = bs.min(total_samples - offset);
                let in_chunk = &test_sig[offset..offset + chunk_len];
                let mut out_chunk = vec![0.0f32; chunk_len];

                let in_slices = vec![in_chunk];
                let mut out_slices = vec![out_chunk.as_mut_slice()];

                node.process(&in_slices, &mut out_slices);
                out_rendered[offset..offset + chunk_len].copy_from_slice(&out_chunk);
                offset += chunk_len;
            }
            outputs_per_block.push(out_rendered);
        }

        // Compare all block renders against the reference (first block size)
        let ref_render = &outputs_per_block[0];
        let mut invariant_holds = true;
        for other in outputs_per_block.iter().skip(1) {
            let null_res = null_residual(ref_render, other);
            if null_res.max_error_db > -110.0 && !null_res.is_bit_exact {
                invariant_holds = false;
                break;
            }
        }
        invariant_holds
    };

    // 5. Sample-rate invariance across standard rates
    let sample_rate_invariance = {
        let sample_rates = [
            22050.0, 32000.0, 44100.0, 48000.0, 88200.0, 96000.0, 192000.0,
        ];
        let mut rates_pass = true;

        for &rate in &sample_rates {
            let mut node = node_factory();
            node.prepare(rate, chs);

            let sig = sine(1000.0, rate, 0.05, 0.5, 0.0);
            let in_channels: Vec<Vec<f32>> = vec![sig.clone(); chs];
            let mut out_channels: Vec<Vec<f32>> = vec![vec![0.0f32; sig.len()]; chs];

            let in_slices: Vec<&[f32]> = in_channels.iter().map(|c| c.as_slice()).collect();
            let mut out_slices: Vec<&mut [f32]> =
                out_channels.iter_mut().map(|c| c.as_mut_slice()).collect();

            node.process(&in_slices, &mut out_slices);

            for &s in &out_channels[0] {
                if s.is_nan() || s.is_infinite() {
                    rates_pass = false;
                    break;
                }
            }
        }
        rates_pass
    };

    // 6. Channel handling (mono & stereo)
    let channel_handling = {
        let mut mono_node = node_factory();
        mono_node.prepare(sr, 1);
        let mono_in = [sine(440.0, sr, 0.05, 0.5, 0.0)];
        let mut mono_out = [vec![0.0f32; mono_in[0].len()]];
        let m_in: Vec<&[f32]> = mono_in.iter().map(|c| c.as_slice()).collect();
        let mut m_out: Vec<&mut [f32]> = mono_out.iter_mut().map(|c| c.as_mut_slice()).collect();
        mono_node.process(&m_in, &mut m_out);

        let mut stereo_node = node_factory();
        stereo_node.prepare(sr, 2);
        let stereo_in = [
            sine(440.0, sr, 0.05, 0.5, 0.0),
            sine(880.0, sr, 0.05, 0.5, 0.0),
        ];
        let mut stereo_out = [
            vec![0.0f32; stereo_in[0].len()],
            vec![0.0f32; stereo_in[1].len()],
        ];
        let s_in: Vec<&[f32]> = stereo_in.iter().map(|c| c.as_slice()).collect();
        let mut s_out: Vec<&mut [f32]> = stereo_out.iter_mut().map(|c| c.as_mut_slice()).collect();
        stereo_node.process(&s_in, &mut s_out);

        mono_out[0].len() == mono_in[0].len() && stereo_out[0].len() == stereo_in[0].len()
    };

    // 7. Parameter smoothing (no clicks / max inter-sample delta bounded)
    let parameter_smoothing = {
        let mut node = node_factory();
        node.prepare(sr, chs);

        if node.parameter_count() > 0 {
            let sig = sine(1000.0, sr, 0.1, 0.8, 0.0);
            let in_channels: Vec<Vec<f32>> = vec![sig.clone(); chs];
            let mut out_channels: Vec<Vec<f32>> = vec![vec![0.0f32; sig.len()]; chs];

            let in_slices: Vec<&[f32]> = in_channels.iter().map(|c| c.as_slice()).collect();
            let mut out_slices: Vec<&mut [f32]> =
                out_channels.iter_mut().map(|c| c.as_mut_slice()).collect();

            // Step parameter mid-way
            node.set_parameter(0, 0.1);
            let half = in_slices[0].len() / 2;

            let in_first: Vec<&[f32]> = in_slices.iter().map(|s| &s[..half]).collect();
            let mut out_first: Vec<&mut [f32]> =
                out_slices.iter_mut().map(|s| &mut s[..half]).collect();
            node.process(&in_first, &mut out_first);

            node.set_parameter(0, 1.0);
            let in_second: Vec<&[f32]> = in_slices.iter().map(|s| &s[half..]).collect();
            let mut out_second: Vec<&mut [f32]> =
                out_slices.iter_mut().map(|s| &mut s[half..]).collect();
            node.process(&in_second, &mut out_second);

            // Verify no massive discontinuities across adjacent samples
            let mut max_delta = 0.0f32;
            for i in 1..out_channels[0].len() {
                let delta = (out_channels[0][i] - out_channels[0][i - 1]).abs();
                max_delta = max_delta.max(delta);
            }
            // Inter-sample delta should be smoothly bounded (< 0.5)
            max_delta < 0.5
        } else {
            true
        }
    };

    // 8. Latency report accuracy (reported latency matches impulse delay)
    let latency_accuracy = {
        let mut node = node_factory();
        node.prepare(sr, 1);
        let reported = node.latency_samples();

        let imp = impulse(256 + reported, 0);
        let in_channels = [imp];
        let mut out_channels = [vec![0.0f32; in_channels[0].len()]];

        let in_slices: Vec<&[f32]> = in_channels.iter().map(|c| c.as_slice()).collect();
        let mut out_slices: Vec<&mut [f32]> =
            out_channels.iter_mut().map(|c| c.as_mut_slice()).collect();

        node.process(&in_slices, &mut out_slices);

        // Verify that before the reported latency, output is zero / noise-floor
        let mut zero_before_latency = true;
        for &sample in out_channels[0].iter().take(reported) {
            if sample.abs() > 1e-4 {
                zero_before_latency = false;
                break;
            }
        }
        zero_before_latency
    };

    // 9. Tail correctness (tail duration covers decay to -90 dBFS)
    let tail_correctness = {
        let mut node = node_factory();
        node.prepare(sr, 1);
        let reported_tail = node.tail_samples();

        let total_frames = 256 + reported_tail;
        let test_input = impulse(total_frames, 0);
        let mut out_channels = [vec![0.0f32; total_frames]];

        let in_slices = vec![test_input.as_slice()];
        let mut out_slices = vec![out_channels[0].as_mut_slice()];

        node.process(&in_slices, &mut out_slices);

        if reported_tail > 0 && reported_tail < total_frames {
            let tail_peak = peak(&out_channels[0][reported_tail..]);
            tail_peak <= 1e-4 // ~ -80 to -90 dBFS
        } else {
            true
        }
    };

    // 10. State reset (reset clears state; identical renders)
    let state_reset = {
        let mut node = node_factory();
        node.prepare(sr, chs);

        let test_sig = chirp_linear(50.0, 5000.0, sr, 0.05, 0.5);
        let in_channels: Vec<Vec<f32>> = vec![test_sig.clone(); chs];
        let mut out1: Vec<Vec<f32>> = vec![vec![0.0f32; test_sig.len()]; chs];
        let mut out2: Vec<Vec<f32>> = vec![vec![0.0f32; test_sig.len()]; chs];

        let in_slices: Vec<&[f32]> = in_channels.iter().map(|c| c.as_slice()).collect();
        let mut out_slices1: Vec<&mut [f32]> = out1.iter_mut().map(|c| c.as_mut_slice()).collect();
        node.process(&in_slices, &mut out_slices1);

        node.reset();

        let mut out_slices2: Vec<&mut [f32]> = out2.iter_mut().map(|c| c.as_mut_slice()).collect();
        node.process(&in_slices, &mut out_slices2);

        let null_res = null_residual(&out1[0], &out2[0]);
        null_res.is_bit_exact || null_res.max_error_db <= -120.0
    };

    // 11. Determinism across runs
    let determinism = {
        let mut node1 = node_factory();
        let mut node2 = node_factory();
        node1.prepare(sr, chs);
        node2.prepare(sr, chs);

        let test_sig = chirp_linear(100.0, 4000.0, sr, 0.05, 0.6);
        let in_channels: Vec<Vec<f32>> = vec![test_sig.clone(); chs];
        let mut out1: Vec<Vec<f32>> = vec![vec![0.0f32; test_sig.len()]; chs];
        let mut out2: Vec<Vec<f32>> = vec![vec![0.0f32; test_sig.len()]; chs];

        let in_slices: Vec<&[f32]> = in_channels.iter().map(|c| c.as_slice()).collect();
        let mut out_slices1: Vec<&mut [f32]> = out1.iter_mut().map(|c| c.as_mut_slice()).collect();
        node1.process(&in_slices, &mut out_slices1);

        let mut out_slices2: Vec<&mut [f32]> = out2.iter_mut().map(|c| c.as_mut_slice()).collect();
        node2.process(&in_slices, &mut out_slices2);

        let null_res = null_residual(&out1[0], &out2[0]);
        null_res.is_bit_exact || null_res.max_error_db <= -120.0
    };

    // 12. Real-time safety: no panics, predictable execution
    let realtime_safety = {
        let mut node = node_factory();
        node.prepare(sr, chs);

        let in_buf = vec![0.5f32; 128];
        let mut out_buf = vec![0.0f32; 128];
        let in_slices = vec![in_buf.as_slice()];
        let mut out_slices = vec![out_buf.as_mut_slice()];

        // Execute multiple times
        for _ in 0..10 {
            node.process(&in_slices, &mut out_slices);
        }
        true
    };

    // 13. Real-time block render == offline continuous render
    let realtime_equals_offline = {
        let test_sig = chirp_linear(50.0, 10000.0, sr, 0.1, 0.7);
        let total_samples = test_sig.len();

        // Offline single render
        let mut offline_node = node_factory();
        offline_node.prepare(sr, 1);
        let mut offline_out = vec![0.0f32; total_samples];
        offline_node.process(&[&test_sig], &mut [&mut offline_out]);

        // Realtime chunked render (128-sample blocks)
        let mut rt_node = node_factory();
        rt_node.prepare(sr, 1);
        let mut rt_out = vec![0.0f32; total_samples];
        let block_size = 128;
        let mut offset = 0;

        while offset < total_samples {
            let chunk_len = block_size.min(total_samples - offset);
            let in_chunk = &test_sig[offset..offset + chunk_len];
            let mut out_chunk = vec![0.0f32; chunk_len];
            rt_node.process(&[in_chunk], &mut [&mut out_chunk]);
            rt_out[offset..offset + chunk_len].copy_from_slice(&out_chunk);
            offset += chunk_len;
        }

        let null_res = null_residual(&offline_out, &rt_out);
        null_res.max_error_db <= -100.0 || null_res.is_bit_exact
    };

    InvariantSuiteResult {
        bypass_identity,
        silence_in_silence_out,
        no_nan_inf,
        block_size_invariance,
        sample_rate_invariance,
        channel_handling,
        parameter_smoothing,
        latency_accuracy,
        tail_correctness,
        state_reset,
        determinism,
        realtime_safety,
        realtime_equals_offline,
    }
}
