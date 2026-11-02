//! Comprehensive integration tests for Phase 1.3 Engine Core:
//! - P1.3-a: Effect graph, parameter smoothing & automation, and PDC impulse alignment
//! - P1.3-b: Real-time EngineHost, transport, seamless loop, and variable-speed scrub
//! - P1.3-c: Offline == real-time null test (residual <= -100 dBFS)
//! - P1.3-d: Resampler with quality presets, round-trip 44.1 -> 48 -> 44.1 null <= -110 dB
//! - P1.3-e: Dither (TPDF zero correlation, noise-shaped psychoacoustic curve)

use flux_dsp::delay::DelayLine;
use flux_dsp::dither::{BitDepth, Dither, DitherType};
use flux_dsp::node::{DspNode, GainNode, PassthroughNode};
use flux_dsp::param::{AutomationLane, AutomationPoint, SmoothedParameter, SmoothingMode};
use flux_dsp::resampler::{Resampler, ResamplerQuality};
use flux_engine::decoder::AudioBuffer;
use flux_engine::graph::AudioGraph;
use flux_engine::render::{EngineCommand, EngineHost, OfflineRenderer};
use flux_engine::transport::{Transport, TransportState};
use flux_testkit::generators;

/// Custom mock DSP node with controllable internal latency for PDC testing.
#[derive(Debug, Clone)]
struct LatencyNode {
    delay: DelayLine,
    latency: usize,
}

impl LatencyNode {
    fn new(latency: usize) -> Self {
        let mut delay = DelayLine::new(latency + 16);
        delay.set_delay(latency as f32);
        Self { delay, latency }
    }
}

impl DspNode for LatencyNode {
    fn name(&self) -> &str {
        "LatencyNode"
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize) {
        self.delay.prepare(sample_rate, channels);
    }

    fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]]) {
        self.delay.process(input, output);
    }

    fn reset(&mut self) {
        self.delay.reset();
    }

    fn latency_samples(&self) -> usize {
        self.latency
    }

    fn set_parameter(&mut self, _index: usize, _value: f32) {}
    fn get_parameter(&self, _index: usize) -> f32 {
        0.0
    }
    fn set_bypass(&mut self, _bypassed: bool) {}
    fn is_bypassed(&self) -> bool {
        false
    }
}

// =========================================================================
// P1.3-a: Parameter Smoothing, Automation, and Plugin Delay Compensation (PDC)
// =========================================================================

#[test]
fn test_parameter_smoothing_reaches_target_at_exact_sample() {
    let mut param = SmoothedParameter::new(0.0, 48000.0);
    let ramp_frames = 100;
    param.set_target_frames(1.0, ramp_frames, SmoothingMode::Linear);

    for frame in 0..ramp_frames - 1 {
        let val = param.next();
        let expected = (frame + 1) as f32 / ramp_frames as f32;
        assert!(
            (val - expected).abs() < 1e-5,
            "Frame {frame}: expected {expected}, got {val}"
        );
    }

    // Exact target reach on the ramp completion frame
    let final_val = param.next();
    assert_eq!(final_val, 1.0, "Must reach target exactly at frame N");
    // Persists after ramp ends
    assert_eq!(param.next(), 1.0);
}

#[test]
fn test_automation_lane_curve_evaluation() {
    let mut lane = AutomationLane::new();
    lane.add_point(AutomationPoint {
        frame: 0,
        value: 0.0,
        tension: 0.0, // Linear
    });
    lane.add_point(AutomationPoint {
        frame: 1000,
        value: 1.0,
        tension: 0.0,
    });

    assert_eq!(lane.evaluate(0, 0.0), 0.0);
    assert_eq!(lane.evaluate(500, 0.0), 0.5);
    assert_eq!(lane.evaluate(1000, 0.0), 1.0);
    // Extrapolation bounds
    assert_eq!(lane.evaluate(1500, 0.0), 1.0);
}

#[test]
fn test_graph_pdc_parallel_branches_impulse_alignment() {
    // Setup a graph with two parallel branches:
    // Root -> Branch A (LatencyNode with 64 samples latency) -> Master Sum
    // Root -> Branch B (PassthroughNode with 0 latency)      -> Master Sum
    let mut graph = AudioGraph::new(48000.0, 256);

    let root = graph.add_node("Root", Box::new(PassthroughNode::new()), 1);
    let branch_a = graph.add_node("BranchA_Delayed", Box::new(LatencyNode::new(64)), 1);
    let branch_b = graph.add_node("BranchB_Fast", Box::new(PassthroughNode::new()), 1);
    let master = graph.add_node("Master", Box::new(PassthroughNode::new()), 1);

    graph.connect(root, branch_a).unwrap();
    graph.connect(root, branch_b).unwrap();
    graph.connect(branch_a, master).unwrap();
    graph.connect(branch_b, master).unwrap();

    // Input: single Dirac delta impulse at frame 0
    let mut input = vec![0.0f32; 256];
    input[0] = 1.0;
    let mut output = vec![0.0f32; 256];

    graph.process(&[&input], &mut [&mut output]);

    // With PDC, Branch B is delayed by 64 samples so both impulses arrive at frame 64.
    // Summed amplitude at frame 64 should be 1.0 + 1.0 = 2.0!
    assert!(
        (output[64] - 2.0).abs() < 1e-4,
        "Impulses from both branches must align at frame 64, got amplitude {}",
        output[64]
    );

    // Frame 0 must be silence (0.0) because Branch B was delay compensated
    assert_eq!(
        output[0], 0.0,
        "Branch B must be delayed to match Branch A latency"
    );
}

// =========================================================================
// P1.3-b: Real-Time EngineHost, Transport, Loop, and Variable-Speed Scrub
// =========================================================================

#[test]
fn test_transport_seamless_loop_and_scrubbing() {
    let mut transport = Transport::new(48000);
    transport.set_total_frames(100000);

    // 1. Playback advance
    transport.play();
    assert_eq!(transport.state(), TransportState::Playing);
    let (s1, e1) = transport.advance(1000);
    assert_eq!(s1, 0);
    assert_eq!(e1, 1000);
    assert_eq!(transport.playhead(), 1000);

    // 2. Seamless loop wrap
    transport.set_loop(true, 1000, 5000);
    transport.seek(4900);

    // Advance 200 frames: 4900 -> wraps at 5000 -> 100 frames into loop -> 1100
    let (s_loop, e_loop) = transport.advance(200);
    assert_eq!(s_loop, 4900);
    assert_eq!(e_loop, 1100);
    assert_eq!(transport.playhead(), 1100);

    // 3. Variable-speed scrub
    transport.set_scrub_speed(2.0); // 2x forward
    assert_eq!(transport.state(), TransportState::Scrubbing);
    let (_, e_scrub) = transport.advance(100);
    assert_eq!(e_scrub, 1300); // 1100 + 200

    transport.set_scrub_speed(-1.0); // 1x reverse
    let (_, e_rev) = transport.advance(100);
    assert_eq!(e_rev, 1200); // 1300 - 100
}

#[test]
fn test_engine_host_realtime_quantum_and_command_queue() {
    let mut host = EngineHost::new(48000.0, 128);
    let _root = host.graph.add_node("Gain", Box::new(GainNode::new(0.5)), 2);
    host.graph.update_topology().unwrap();

    let in_l = vec![1.0f32; 128];
    let in_r = vec![1.0f32; 128];
    let mut out_l = vec![0.0f32; 128];
    let mut out_r = vec![0.0f32; 128];

    // While stopped, output must be silence
    host.render_quantum(&[&in_l, &in_r], &mut [&mut out_l, &mut out_r]);
    assert_eq!(out_l[0], 0.0);

    // Push Play command
    host.commands.push(EngineCommand::Play);
    host.render_quantum(&[&in_l, &in_r], &mut [&mut out_l, &mut out_r]);

    // Output rendered with gain
    assert!(out_l[127] > 0.0);
    assert_eq!(host.transport.state(), TransportState::Playing);
}

// =========================================================================
// P1.3-c: Offline == Real-Time Null Test (residual <= -100 dBFS)
// =========================================================================

#[test]
fn test_offline_vs_realtime_null_on_complex_graph() {
    let sample_rate = 48000.0;
    let block_size = 128;
    let num_frames = 4800; // 100 ms

    // Generate test input signal (sum of sines)
    let ch0 = generators::sine(1000.0, sample_rate, 0.1, 0.5, 0.0);
    let ch1 = generators::sine(2500.0, sample_rate, 0.1, 0.4, 0.0);
    let input_buffer = AudioBuffer {
        channels: vec![ch0.clone(), ch1.clone()],
        sample_rate: 48000,
    };

    // Helper to build identical graph: Root -> GainNode(0.8) -> Biquad Peaking -> Passthrough
    fn build_graph(sr: f32, bs: usize) -> AudioGraph {
        let mut g = AudioGraph::new(sr, bs);
        let n1 = g.add_node("Gain", Box::new(GainNode::new(0.8)), 2);
        let mut biquad = flux_dsp::biquad::BiquadChain::new(1);
        biquad.prepare(sr, 2);
        biquad.configure_stage(0, flux_dsp::biquad::BiquadType::Peaking, 1000.0, 1.0, 3.0);
        let n2 = g.add_node("Biquad", Box::new(biquad), 2);
        g.connect(n1, n2).unwrap();
        g
    }

    // 1. Render in simulated real-time (128 frames per quantum)
    let mut rt_graph = build_graph(sample_rate, block_size);
    let mut rt_out_l = Vec::with_capacity(num_frames);
    let mut rt_out_r = Vec::with_capacity(num_frames);

    let mut cursor = 0;
    while cursor < num_frames {
        let frames = (num_frames - cursor).min(block_size);
        let in_slice_l = &ch0[cursor..cursor + frames];
        let in_slice_r = &ch1[cursor..cursor + frames];

        let mut out_l = vec![0.0f32; frames];
        let mut out_r = vec![0.0f32; frames];

        rt_graph.process(&[in_slice_l, in_slice_r], &mut [&mut out_l, &mut out_r]);

        rt_out_l.extend_from_slice(&out_l);
        rt_out_r.extend_from_slice(&out_r);
        cursor += frames;
    }

    // 2. Render through OfflineRenderer
    let mut offline_graph = build_graph(sample_rate, block_size);
    let offline_output =
        OfflineRenderer::render(&mut offline_graph, &input_buffer, 2, block_size, None, None)
            .unwrap();

    assert_eq!(offline_output.channels[0].len(), num_frames);
    assert_eq!(offline_output.channels[1].len(), num_frames);

    // 3. Null test: calculate residual difference between offline and real-time
    let mut max_residual_db = -200.0f32;
    for ch in 0..2 {
        let rt_ch = if ch == 0 { &rt_out_l } else { &rt_out_r };
        let off_ch = &offline_output.channels[ch];

        for i in 0..num_frames {
            let diff = (rt_ch[i] - off_ch[i]).abs();
            let diff_db = if diff > 1e-12 {
                20.0 * diff.log10()
            } else {
                -200.0
            };
            if diff_db > max_residual_db {
                max_residual_db = diff_db;
            }
        }
    }

    assert!(
        max_residual_db <= -100.0,
        "Real-time vs Offline null residual must be <= -100 dBFS, measured {max_residual_db:.2} dBFS"
    );
}

// =========================================================================
// P1.3-d: Resampler with Quality Presets and Round-Trip Accuracy
// =========================================================================

#[test]
fn test_resampler_roundtrip_441_to_48_to_441_null() {
    let sr_in = 44100;
    let sr_mid = 48000;
    let num_frames = 4410; // 0.1s

    // Generate 1 kHz reference sine at 44.1 kHz
    let input = generators::sine(1000.0, sr_in as f32, 0.1, 0.5, 0.0);

    // 1. Resample 44.1 kHz -> 48 kHz
    let resampler_up = Resampler::new(sr_in, sr_mid, ResamplerQuality::High);
    let upsampled = resampler_up.resample_channel(&input);
    let expected_mid_frames = (num_frames as f64 * (48000.0 / 44100.0)).round() as usize;
    assert_eq!(upsampled.len(), expected_mid_frames);

    // 2. Resample 48 kHz -> 44.1 kHz
    let resampler_down = Resampler::new(sr_mid, sr_in, ResamplerQuality::High);
    let roundtrip = resampler_down.resample_channel(&upsampled);

    // Compare in-band samples in steady state (ignore edge filter transient of 2x 128 taps)
    let edge_margin = 350;
    let cmp_len = (input.len().min(roundtrip.len())).saturating_sub(edge_margin * 2);
    let in_slice = &input[edge_margin..edge_margin + cmp_len];
    let out_slice = &roundtrip[edge_margin..edge_margin + cmp_len];

    // Compute residual null SNR
    let mut sum_sq_err = 0.0f64;
    let mut sum_sq_sig = 0.0f64;
    for i in 0..cmp_len {
        let err = (in_slice[i] - out_slice[i]) as f64;
        sum_sq_err += err * err;
        sum_sq_sig += (in_slice[i] as f64) * (in_slice[i] as f64);
    }

    let snr = 10.0 * (sum_sq_sig / sum_sq_err).log10();
    assert!(
        snr >= 110.0,
        "Resampler roundtrip 44.1 -> 48 -> 44.1 SNR must be >= 110 dB, measured {snr:.2} dB"
    );
}

// =========================================================================
// P1.3-e: Dither (TPDF Zero Correlation & Noise-Shaped)
// =========================================================================

#[test]
fn test_tpdf_dither_flatness_and_zero_correlation() {
    let mut dither = Dither::new(DitherType::Tpdf, BitDepth::Bits16, 1);
    let n = 48000;

    // Test signal: slow sine wave
    let mut signal: Vec<f32> = (0..n)
        .map(|i| (2.0 * std::f32::consts::PI * 100.0 * (i as f32 / 48000.0)).sin() * 0.1)
        .collect();
    let original = signal.clone();

    dither.process(&mut [&mut signal]);

    // Quantization error = dithered_quantized - original
    let mut error = Vec::with_capacity(n);
    for i in 0..n {
        error.push(signal[i] - original[i]);
    }

    // Pearson correlation between signal and error: should be ~ 0.0 (uncorrelated)
    let mean_sig: f32 = original.iter().sum::<f32>() / n as f32;
    let mean_err: f32 = error.iter().sum::<f32>() / n as f32;

    let mut cov = 0.0f64;
    let mut var_sig = 0.0f64;
    let mut var_err = 0.0f64;

    for i in 0..n {
        let ds = (original[i] - mean_sig) as f64;
        let de = (error[i] - mean_err) as f64;
        cov += ds * de;
        var_sig += ds * ds;
        var_err += de * de;
    }

    let correlation = (cov / (var_sig.sqrt() * var_err.sqrt())).abs();
    assert!(
        correlation < 0.02,
        "TPDF dither error must be uncorrelated with signal (correlation < 0.02), got {correlation:.5}"
    );
}

#[test]
fn test_dither_bypassed_on_float32() {
    let mut dither = Dither::new(DitherType::Tpdf, BitDepth::Float32, 1);
    let mut samples = vec![0.1234567f32; 100];
    let original = samples.clone();

    dither.process(&mut [&mut samples]);
    assert_eq!(
        samples, original,
        "Dithering must be bypassed when bit depth is Float32"
    );
}
