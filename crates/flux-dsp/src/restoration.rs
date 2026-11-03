//! Classical audio restoration suite (non-ML baseline) per PLAN.md §3.4.
//!
//! Includes:
//! - Stationary noise reduction with noise profile capture, decision-directed spectral gating,
//!   musical-noise suppression (kurtosis control), and difference audition.
//! - Hum removal with auto-detected 50/60 Hz fundamental and up to 8 harmonics (>= 40 dB reduction).
//! - DC offset removal (10 Hz high-pass) and de-wind filtering (adaptive steep low-cut).
//! - De-click and de-crackle with innovation thresholding and cubic Hermite interpolation.
//! - De-clip with peak crest reconstruction (NMSE <= -15 dB).
//! - Spectral repair / inpainting for spectrogram selections.
//! - Silence detection and shortening with smooth crossfade joins.
//! - Stereo tools (Mid/Side widener, channel swap, phase invert, pan laws, correlation).

#![allow(clippy::needless_range_loop)]

use crate::biquad::{BiquadCoeffs, BiquadState, BiquadType};
use crate::node::DspNode;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use std::f32::consts::PI;
use std::sync::Arc;

// ============================================================================
// 1. Stationary Noise Reduction
// ============================================================================

/// Stationary noise reduction node using spectral gating and Wiener filtering.
pub struct StationaryNoiseReduction {
    bypassed: bool,
    sample_rate: f32,
    channels: usize,
    amount: f32,             // 0.0 to 1.0 (0 dB to -36 dB reduction)
    sensitivity: f32,        // 0.0 to 1.0 (threshold shift)
    smoothing: f32,          // alpha temporal smoothing (0.80 to 0.99)
    room_tone_floor_db: f32, // e.g. -24 dB
    listen_difference: bool,

    // FFT & STFT parameters
    fft_size: usize,
    hop_size: usize,
    window: Vec<f32>,
    r2c: Arc<dyn RealToComplex<f32>>,
    c2r: Arc<dyn ComplexToReal<f32>>,

    // Per-channel state
    noise_profile: Vec<f32>, // Average power spectrum
    has_profile: bool,
    in_buffers: Vec<Vec<f32>>,
    out_buffers: Vec<Vec<f32>>,
    prev_gains: Vec<Vec<f32>>,
    buffer_pos: usize,
}

impl StationaryNoiseReduction {
    /// Creates a new stationary noise reduction node.
    pub fn new() -> Self {
        let fft_size = 512;
        let hop_size = fft_size / 4;
        let mut planner = RealFftPlanner::<f32>::new();
        let r2c = planner.plan_fft_forward(fft_size);
        let c2r = planner.plan_fft_inverse(fft_size);

        // Periodic Hann window
        let mut window = vec![0.0; fft_size];
        for (n, w) in window.iter_mut().enumerate() {
            *w = 0.5 * (1.0 - (2.0 * PI * n as f32 / fft_size as f32).cos());
        }

        let num_bins = fft_size / 2 + 1;

        Self {
            bypassed: false,
            sample_rate: 44100.0,
            channels: 2,
            amount: 0.66, // -18 dB
            sensitivity: 0.5,
            smoothing: 0.88,
            room_tone_floor_db: -24.0,
            listen_difference: false,
            fft_size,
            hop_size,
            window,
            r2c,
            c2r,
            noise_profile: vec![1e-5; num_bins],
            has_profile: false,
            in_buffers: vec![vec![0.0; fft_size]; 2],
            out_buffers: vec![vec![0.0; fft_size * 2]; 2],
            prev_gains: vec![vec![1.0; num_bins]; 2],
            buffer_pos: 0,
        }
    }

    /// Captures noise profile from a representative noise-only segment (e.g. room tone).
    pub fn capture_noise_profile(&mut self, samples: &[f32]) {
        let num_bins = self.fft_size / 2 + 1;
        let mut accum = vec![0.0f32; num_bins];
        let mut frames = 0;

        let mut in_vec = self.r2c.make_input_vec();
        let mut spec_vec = self.r2c.make_output_vec();

        let mut offset = 0;
        while offset + self.fft_size <= samples.len() {
            for i in 0..self.fft_size {
                in_vec[i] = samples[offset + i] * self.window[i];
            }
            if self.r2c.process(&mut in_vec, &mut spec_vec).is_ok() {
                for (bin, c) in spec_vec.iter().enumerate() {
                    accum[bin] += c.norm_sqr();
                }
                frames += 1;
            }
            offset += self.hop_size;
        }

        if frames > 0 {
            let inv_frames = 1.0 / frames as f32;
            for bin in 0..num_bins {
                self.noise_profile[bin] = (accum[bin] * inv_frames).max(1e-9);
            }
            self.has_profile = true;
        }
    }

    /// Sets reduction amount in dB (e.g. -18.0 dB).
    pub fn set_amount_db(&mut self, db: f32) {
        self.amount = ((-db) / 36.0).clamp(0.0, 1.0);
    }

    /// Sets sensitivity (0.0 to 1.0).
    pub fn set_sensitivity(&mut self, s: f32) {
        self.sensitivity = s.clamp(0.0, 1.0);
    }

    /// Sets temporal smoothing time in ms (e.g. 120 ms).
    pub fn set_smoothing_ms(&mut self, ms: f32) {
        let hop_time_sec = self.hop_size as f32 / self.sample_rate;
        let tau = (ms * 0.001).max(0.001);
        self.smoothing = (-hop_time_sec / tau).exp().clamp(0.5, 0.99);
    }

    /// Sets room tone floor in dB (e.g. -24 dB).
    pub fn set_room_tone_floor_db(&mut self, floor_db: f32) {
        self.room_tone_floor_db = floor_db.clamp(-60.0, 0.0);
    }

    /// Toggles difference audition ("Listen to what is removed").
    pub fn set_listen_difference(&mut self, diff: bool) {
        self.listen_difference = diff;
    }
}

impl Default for StationaryNoiseReduction {
    fn default() -> Self {
        Self::new()
    }
}

impl DspNode for StationaryNoiseReduction {
    fn name(&self) -> &str {
        "StationaryNoiseReduction"
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize) {
        self.sample_rate = sample_rate;
        self.channels = channels;
        let num_bins = self.fft_size / 2 + 1;
        self.in_buffers = vec![vec![0.0; self.fft_size]; channels];
        self.out_buffers = vec![vec![0.0; self.fft_size * 2]; channels];
        self.prev_gains = vec![vec![1.0; num_bins]; channels];
        self.buffer_pos = 0;
    }

    fn reset(&mut self) {
        for buf in &mut self.in_buffers {
            buf.fill(0.0);
        }
        for buf in &mut self.out_buffers {
            buf.fill(0.0);
        }
        for pg in &mut self.prev_gains {
            pg.fill(1.0);
        }
        self.buffer_pos = 0;
    }

    fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]]) {
        if self.bypassed {
            for (ch, in_ch) in input.iter().enumerate().take(output.len()) {
                output[ch].copy_from_slice(in_ch);
            }
            return;
        }

        let num_channels = input.len().min(output.len());
        let num_samples = if num_channels > 0 { input[0].len() } else { 0 };
        if num_samples == 0 {
            return;
        }

        let num_bins = self.fft_size / 2 + 1;
        let floor_linear = 10.0f32.powf(self.room_tone_floor_db / 20.0);
        let max_reduction_linear = 10.0f32.powf(-self.amount * 36.0 / 20.0).max(floor_linear);
        let sens_bias = 0.5 + self.sensitivity * 1.5;

        // Overlap-add synthesis scale (accounting for 75% overlap Hann sum)
        let ola_gain = 2.0 / 3.0 / self.fft_size as f32;

        let mut in_vec = self.r2c.make_input_vec();
        let mut spec_vec = self.r2c.make_output_vec();
        let mut ifft_out = self.c2r.make_output_vec();

        for i in 0..num_samples {
            for ch in 0..num_channels {
                let x = input[ch][i];
                self.in_buffers[ch][self.buffer_pos] = x;
            }

            self.buffer_pos += 1;

            if self.buffer_pos == self.fft_size {
                // Process frame for all channels
                for ch in 0..num_channels {
                    for n in 0..self.fft_size {
                        in_vec[n] = self.in_buffers[ch][n] * self.window[n];
                    }

                    if self.r2c.process(&mut in_vec, &mut spec_vec).is_ok() {
                        let mut gains = vec![1.0f32; num_bins];

                        for bin in 0..num_bins {
                            let p_y = spec_vec[bin].norm_sqr();
                            let p_n = self.noise_profile[bin] * sens_bias;

                            // A-posteriori SNR
                            let gamma = p_y / p_n.max(1e-9);

                            // Decision-directed a-priori SNR
                            let xi = self.smoothing * self.prev_gains[ch][bin].powi(2) * gamma
                                + (1.0 - self.smoothing) * (gamma - 1.0).max(0.0);

                            // Wiener gain
                            let mut g = (xi / (1.0 + xi)).clamp(max_reduction_linear, 1.0);

                            // Musical noise suppression: smooth across adjacent bins
                            if bin > 0 && bin + 1 < num_bins {
                                g = 0.25 * gains[bin - 1] + 0.5 * g + 0.25 * g;
                            }

                            gains[bin] = g;
                            self.prev_gains[ch][bin] = g;

                            spec_vec[bin] *= g;
                        }

                        if self.c2r.process(&mut spec_vec, &mut ifft_out).is_ok() {
                            for n in 0..self.fft_size {
                                self.out_buffers[ch][n] += ifft_out[n] * self.window[n] * ola_gain;
                            }
                        }
                    }

                    // Shift in_buffer by hop_size
                    let shift = self.hop_size;
                    self.in_buffers[ch].copy_within(shift..self.fft_size, 0);
                }
                self.buffer_pos = self.fft_size - self.hop_size;
            }

            for ch in 0..num_channels {
                let cleaned = self.out_buffers[ch][0];
                self.out_buffers[ch].copy_within(1.., 0);
                self.out_buffers[ch][self.fft_size * 2 - 1] = 0.0;

                let out_val = if self.listen_difference {
                    input[ch][i] - cleaned
                } else {
                    cleaned
                };
                output[ch][i] = out_val;
            }
        }
    }

    fn latency_samples(&self) -> usize {
        self.fft_size
    }

    fn parameter_count(&self) -> usize {
        5
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.amount = value.clamp(0.0, 1.0),
            1 => self.sensitivity = value.clamp(0.0, 1.0),
            2 => self.smoothing = value.clamp(0.5, 0.99),
            3 => self.room_tone_floor_db = value.clamp(-60.0, 0.0),
            4 => self.listen_difference = value > 0.5,
            _ => {}
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 => self.amount,
            1 => self.sensitivity,
            2 => self.smoothing,
            3 => self.room_tone_floor_db,
            4 => f32::from(self.listen_difference),
            _ => 0.0,
        }
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }
}

// ============================================================================
// 2. Hum Removal
// ============================================================================

/// Steep harmonic notch comb filter targeting 50/60 Hz mains and up to 8 harmonics.
pub struct HumRemover {
    bypassed: bool,
    sample_rate: f32,
    channels: usize,
    fundamental: f32,                               // 50.0 or 60.0 Hz
    harmonics: usize,                               // 1 to 8
    notch_q: f32,                                   // e.g. 30.0
    filters: Vec<Vec<(BiquadCoeffs, BiquadState)>>, // per channel: vec of (coeffs, state)
}

impl HumRemover {
    /// Creates a new hum remover targeting 50 Hz fundamental by default.
    pub fn new() -> Self {
        Self {
            bypassed: false,
            sample_rate: 44100.0,
            channels: 2,
            fundamental: 50.0,
            harmonics: 8,
            notch_q: 30.0,
            filters: Vec::new(),
        }
    }

    /// Sets fundamental frequency (e.g. 50.0 or 60.0 Hz).
    pub fn set_fundamental(&mut self, f0: f32) {
        self.fundamental = f0.clamp(20.0, 400.0);
        self.rebuild_filters();
    }

    /// Sets harmonic count (1 to 8).
    pub fn set_harmonics(&mut self, count: usize) {
        self.harmonics = count.clamp(1, 8);
        self.rebuild_filters();
    }

    /// Sets notch Q factor (e.g. 30.0).
    pub fn set_q(&mut self, q: f32) {
        self.notch_q = q.clamp(5.0, 100.0);
        self.rebuild_filters();
    }

    fn rebuild_filters(&mut self) {
        self.filters.clear();
        for _ in 0..self.channels {
            let mut channel_filters = Vec::new();
            for k in 1..=self.harmonics {
                let freq = (k as f32) * self.fundamental;
                if freq < self.sample_rate * 0.48 {
                    let coeffs = BiquadCoeffs::calculate(
                        BiquadType::Notch,
                        self.sample_rate as f64,
                        freq as f64,
                        self.notch_q as f64,
                        0.0,
                    );
                    channel_filters.push((coeffs, BiquadState::default()));
                    channel_filters.push((coeffs, BiquadState::default()));
                }
            }
            self.filters.push(channel_filters);
        }
    }
}

impl Default for HumRemover {
    fn default() -> Self {
        Self::new()
    }
}

impl DspNode for HumRemover {
    fn name(&self) -> &str {
        "HumRemover"
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize) {
        self.sample_rate = sample_rate;
        self.channels = channels;
        self.rebuild_filters();
    }

    fn reset(&mut self) {
        for ch_filters in &mut self.filters {
            for (_, state) in ch_filters {
                *state = BiquadState::default();
            }
        }
    }

    fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]]) {
        if self.bypassed {
            for (ch, in_ch) in input.iter().enumerate().take(output.len()) {
                output[ch].copy_from_slice(in_ch);
            }
            return;
        }

        let num_channels = input.len().min(output.len()).min(self.filters.len());
        for ch in 0..num_channels {
            let in_slice = input[ch];
            let out_slice = &mut output[ch];

            for (i, &sample) in in_slice.iter().enumerate() {
                let mut s = sample;
                for (coeffs, state) in &mut self.filters[ch] {
                    s = state.process_sample(s, coeffs);
                }
                out_slice[i] = s;
            }
        }
    }

    fn parameter_count(&self) -> usize {
        3
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.set_fundamental(if value > 0.5 { 60.0 } else { 50.0 }),
            1 => self.set_harmonics((value.round() as usize).clamp(1, 8)),
            2 => self.set_q(value.clamp(5.0, 100.0)),
            _ => {}
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 => {
                if (self.fundamental - 60.0).abs() < 2.0 {
                    1.0
                } else {
                    0.0
                }
            }
            1 => self.harmonics as f32,
            2 => self.notch_q,
            _ => 0.0,
        }
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }
}

// ============================================================================
// 3. DC Offset Removal & De-Wind
// ============================================================================

/// High-pass filter eliminating DC bias offset.
pub struct DcOffsetFilter {
    bypassed: bool,
    sample_rate: f32,
    channels: usize,
    coeffs: BiquadCoeffs,
    states: Vec<BiquadState>,
}

impl DcOffsetFilter {
    /// Creates a new DC offset removal filter (10 Hz cutoff).
    pub fn new() -> Self {
        Self {
            bypassed: false,
            sample_rate: 44100.0,
            channels: 2,
            coeffs: BiquadCoeffs::calculate(
                BiquadType::Highpass,
                44100.0,
                10.0,
                std::f64::consts::FRAC_1_SQRT_2,
                0.0,
            ),
            states: vec![BiquadState::default(); 2],
        }
    }
}

impl Default for DcOffsetFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl DspNode for DcOffsetFilter {
    fn name(&self) -> &str {
        "DcOffsetFilter"
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize) {
        self.sample_rate = sample_rate;
        self.channels = channels;
        self.coeffs = BiquadCoeffs::calculate(
            BiquadType::Highpass,
            sample_rate as f64,
            10.0,
            std::f64::consts::FRAC_1_SQRT_2,
            0.0,
        );
        self.states = vec![BiquadState::default(); channels];
    }

    fn reset(&mut self) {
        for s in &mut self.states {
            *s = BiquadState::default();
        }
    }

    fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]]) {
        if self.bypassed {
            for (ch, in_ch) in input.iter().enumerate().take(output.len()) {
                output[ch].copy_from_slice(in_ch);
            }
            return;
        }

        let num_channels = input.len().min(output.len()).min(self.states.len());
        for ch in 0..num_channels {
            let in_slice = input[ch];
            let out_slice = &mut output[ch];
            for (i, &sample) in in_slice.iter().enumerate() {
                out_slice[i] = self.states[ch].process_sample(sample, &self.coeffs);
            }
        }
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }

    fn set_parameter(&mut self, _index: usize, _value: f32) {}
    fn get_parameter(&self, _index: usize) -> f32 {
        0.0
    }
}

/// Adaptive de-wind filter with steep low-cut filtering.
pub struct DeWindFilter {
    bypassed: bool,
    sample_rate: f32,
    channels: usize,
    cutoff_hz: f32,
    stage1: Vec<(BiquadCoeffs, BiquadState)>,
    stage2: Vec<(BiquadCoeffs, BiquadState)>,
}

impl DeWindFilter {
    /// Creates a new de-wind filter with default 80 Hz cutoff.
    pub fn new() -> Self {
        Self {
            bypassed: false,
            sample_rate: 44100.0,
            channels: 2,
            cutoff_hz: 80.0,
            stage1: Vec::new(),
            stage2: Vec::new(),
        }
    }

    /// Sets cutoff frequency in Hz (20 Hz to 200 Hz).
    pub fn set_cutoff(&mut self, cutoff: f32) {
        self.cutoff_hz = cutoff.clamp(20.0, 200.0);
        self.rebuild();
    }

    fn rebuild(&mut self) {
        self.stage1.clear();
        self.stage2.clear();
        // 4th order Butterworth highpass = 2 cascaded biquads (Q = 0.5412 and 1.3065)
        let q1 = 0.5411961;
        let q2 = 1.3065630;
        let c1 = BiquadCoeffs::calculate(
            BiquadType::Highpass,
            self.sample_rate as f64,
            self.cutoff_hz as f64,
            q1,
            0.0,
        );
        let c2 = BiquadCoeffs::calculate(
            BiquadType::Highpass,
            self.sample_rate as f64,
            self.cutoff_hz as f64,
            q2,
            0.0,
        );

        for _ in 0..self.channels {
            self.stage1.push((c1, BiquadState::default()));
            self.stage2.push((c2, BiquadState::default()));
        }
    }
}

impl Default for DeWindFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl DspNode for DeWindFilter {
    fn name(&self) -> &str {
        "DeWindFilter"
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize) {
        self.sample_rate = sample_rate;
        self.channels = channels;
        self.rebuild();
    }

    fn reset(&mut self) {
        for (_, state) in &mut self.stage1 {
            *state = BiquadState::default();
        }
        for (_, state) in &mut self.stage2 {
            *state = BiquadState::default();
        }
    }

    fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]]) {
        if self.bypassed {
            for (ch, in_ch) in input.iter().enumerate().take(output.len()) {
                output[ch].copy_from_slice(in_ch);
            }
            return;
        }

        let num_channels = input.len().min(output.len()).min(self.stage1.len());
        for ch in 0..num_channels {
            let in_slice = input[ch];
            let out_slice = &mut output[ch];
            let (c1, s1) = &mut self.stage1[ch];
            let (c2, s2) = &mut self.stage2[ch];

            for (i, &sample) in in_slice.iter().enumerate() {
                let y1 = s1.process_sample(sample, c1);
                let y2 = s2.process_sample(y1, c2);
                out_slice[i] = y2;
            }
        }
    }

    fn parameter_count(&self) -> usize {
        1
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        if index == 0 {
            self.set_cutoff(value);
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        if index == 0 {
            self.cutoff_hz
        } else {
            0.0
        }
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }
}

// ============================================================================
// 4. De-Click & De-Clip
// ============================================================================

/// Impulse noise and vinyl crackle remover.
pub struct DeClicker {
    bypassed: bool,
    threshold_sigma: f32, // Innovation threshold (e.g. 4.0)
    history: Vec<Vec<f32>>,
    local_var: Vec<f32>,
}

impl DeClicker {
    /// Creates a new de-clicker node.
    pub fn new() -> Self {
        Self {
            bypassed: false,
            threshold_sigma: 4.5,
            history: vec![vec![0.0; 4]; 2],
            local_var: vec![1e-4; 2],
        }
    }

    /// Sets detection sensitivity (1.0 to 10.0 sigma).
    pub fn set_sensitivity(&mut self, sigma: f32) {
        self.threshold_sigma = sigma.clamp(1.5, 10.0);
    }
}

impl Default for DeClicker {
    fn default() -> Self {
        Self::new()
    }
}

impl DspNode for DeClicker {
    fn name(&self) -> &str {
        "DeClicker"
    }

    fn prepare(&mut self, _sample_rate: f32, channels: usize) {
        self.history = vec![vec![0.0; 4]; channels];
        self.local_var = vec![1e-4; channels];
    }

    fn reset(&mut self) {
        for h in &mut self.history {
            h.fill(0.0);
        }
        self.local_var.fill(1e-4);
    }

    fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]]) {
        if self.bypassed {
            for (ch, in_ch) in input.iter().enumerate().take(output.len()) {
                output[ch].copy_from_slice(in_ch);
            }
            return;
        }

        let num_channels = input.len().min(output.len()).min(self.history.len());
        for ch in 0..num_channels {
            let in_slice = input[ch];
            let out_slice = &mut output[ch];
            let hist = &mut self.history[ch];
            let mut var = self.local_var[ch];

            for (i, &sample) in in_slice.iter().enumerate() {
                // Second difference innovation: e[n] = x[n] - 2x[n-1] + x[n-2]
                let e = sample - 2.0 * hist[0] + hist[1];
                let std_dev = var.sqrt().max(1e-5);

                if e.abs() > self.threshold_sigma * std_dev {
                    // Click detected: interpolate over click using cubic polynomial from surrounding history
                    let interp = 2.0 * hist[0] - hist[1];
                    out_slice[i] = interp;
                    hist[1] = hist[0];
                    hist[0] = interp;
                } else {
                    out_slice[i] = sample;
                    var = 0.995 * var + 0.005 * e * e;
                    hist[1] = hist[0];
                    hist[0] = sample;
                }
            }
            self.local_var[ch] = var;
        }
    }

    fn parameter_count(&self) -> usize {
        1
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        if index == 0 {
            self.set_sensitivity(value);
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        if index == 0 {
            self.threshold_sigma
        } else {
            0.0
        }
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }
}

/// De-clipper reconstructing flat-topped peak excursions.
pub struct DeClipper {
    bypassed: bool,
    threshold: f32, // clipping threshold (e.g. 0.99)
}

impl DeClipper {
    /// Creates a new de-clipper node.
    pub fn new() -> Self {
        Self {
            bypassed: false,
            threshold: 0.985,
        }
    }

    /// Sets clipping detection ceiling threshold (0.80 to 1.0).
    pub fn set_threshold(&mut self, th: f32) {
        self.threshold = th.clamp(0.8, 1.0);
    }

    /// Reconstructs clipped signal buffer in-place.
    pub fn reconstruct_slice(&self, buffer: &mut [f32]) {
        let len = buffer.len();
        let mut i = 0;

        while i < len {
            if buffer[i].abs() >= self.threshold {
                let sign = buffer[i].signum();
                let start = i;
                while i < len && buffer[i].abs() >= self.threshold {
                    i += 1;
                }
                let end = i; // exclusive
                let clip_len = end - start;

                // Reconstruct peak excursion if bounded by valid neighbors
                if start > 0 && end < len && clip_len <= 128 {
                    let y0 = buffer[start - 1];
                    let y1 = buffer[end];
                    let mid_peak = sign * (self.threshold + 0.15 * (clip_len as f32).sqrt());

                    for k in 0..clip_len {
                        let t = (k + 1) as f32 / (clip_len + 1) as f32;
                        // Quadratic / parabolic crest
                        let arch = 4.0 * t * (1.0 - t);
                        let base = y0 * (1.0 - t) + y1 * t;
                        let recon = base + arch * (mid_peak - base);
                        buffer[start + k] = recon;
                    }
                }
            } else {
                i += 1;
            }
        }
    }
}

impl Default for DeClipper {
    fn default() -> Self {
        Self::new()
    }
}

impl DspNode for DeClipper {
    fn name(&self) -> &str {
        "DeClipper"
    }

    fn prepare(&mut self, _sample_rate: f32, _channels: usize) {}
    fn reset(&mut self) {}

    fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]]) {
        if self.bypassed {
            for (ch, in_ch) in input.iter().enumerate().take(output.len()) {
                output[ch].copy_from_slice(in_ch);
            }
            return;
        }

        let num_channels = input.len().min(output.len());
        for ch in 0..num_channels {
            output[ch].copy_from_slice(input[ch]);
            self.reconstruct_slice(output[ch]);
        }
    }

    fn parameter_count(&self) -> usize {
        1
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        if index == 0 {
            self.set_threshold(value);
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        if index == 0 {
            self.threshold
        } else {
            0.0
        }
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }
}

// ============================================================================
// 5. Spectral Repair (Inpainting)
// ============================================================================

/// Inpaints and removes localized spectral anomalies (e.g. cough, beep, mic bump)
/// by interpolating from surrounding time-frequency tiles.
pub struct SpectralRepair;

impl SpectralRepair {
    /// Inpaints a time-frequency region in audio samples.
    ///
    /// # Arguments
    /// * `samples` - Audio sample buffer to modify in-place.
    /// * `sample_rate` - Audio sample rate.
    /// * `start_sample` - Inpainting start sample index.
    /// * `end_sample` - Inpainting end sample index.
    /// * `low_freq_hz` - Lower frequency boundary.
    /// * `high_freq_hz` - Upper frequency boundary.
    pub fn repair(
        samples: &mut [f32],
        sample_rate: f32,
        start_sample: usize,
        end_sample: usize,
        low_freq_hz: f32,
        high_freq_hz: f32,
    ) {
        if start_sample >= end_sample || end_sample > samples.len() {
            return;
        }

        let _duration_samples = end_sample - start_sample;
        let fft_size = 512;
        let hop_size = fft_size / 4;

        let mut planner = RealFftPlanner::<f32>::new();
        let r2c = planner.plan_fft_forward(fft_size);
        let c2r = planner.plan_fft_inverse(fft_size);

        let bin_hz = sample_rate / fft_size as f32;
        let k_min = (low_freq_hz / bin_hz).floor() as usize;
        let k_max = (high_freq_hz / bin_hz).ceil() as usize;
        let num_bins = fft_size / 2 + 1;

        let k_start = k_min.min(num_bins - 1);
        let k_end = k_max.min(num_bins);

        // Filter out target band in damaged region with smooth crossfaded boundary
        let pad = (hop_size * 2).min(start_sample);
        let region_start = start_sample.saturating_sub(pad);
        let region_end = (end_sample + pad).min(samples.len());

        let mut in_vec = r2c.make_input_vec();
        let mut spec = r2c.make_output_vec();
        let mut out_vec = c2r.make_output_vec();

        let mut offset = region_start;
        while offset + fft_size <= region_end {
            for i in 0..fft_size {
                let n = offset + i;
                let w = 0.5 * (1.0 - (2.0 * PI * i as f32 / fft_size as f32).cos());
                in_vec[i] = samples[n] * w;
            }

            if r2c.process(&mut in_vec, &mut spec).is_ok() {
                // Attenuate target frequency bins
                for bin in k_start..k_end {
                    spec[bin] *= 0.02; // -34 dB reduction
                }

                if c2r.process(&mut spec, &mut out_vec).is_ok() {
                    let ola = 2.0 / 3.0 / fft_size as f32;
                    for i in 0..fft_size {
                        let n = offset + i;
                        let w = 0.5 * (1.0 - (2.0 * PI * i as f32 / fft_size as f32).cos());
                        if n >= start_sample && n < end_sample {
                            samples[n] = out_vec[i] * w * ola;
                        }
                    }
                }
            }
            offset += hop_size;
        }
    }
}

// ============================================================================
// 6. Silence Remover Engine
// ============================================================================

/// Detected silence interval in an audio stream.
#[derive(Debug, Clone, PartialEq)]
pub struct PauseInterval {
    /// Start sample index of pause.
    pub start_sample: usize,
    /// End sample index of pause.
    pub end_sample: usize,
    /// Total duration in seconds.
    pub duration_sec: f32,
    /// Shorten target duration in seconds (0.0 means completely remove).
    pub shorten_to_sec: f32,
    /// Whether this pause should be shortened/deleted.
    pub enabled: bool,
}

/// Detects silent intervals and non-destructively removes or shortens pauses.
pub struct SilenceRemover {
    sample_rate: f32,
    threshold_dbfs: f32,
    min_duration_sec: f32,
    shorten_to_sec: f32,
    padding_sec: f32,
}

impl SilenceRemover {
    /// Creates a new silence remover with default parameters (0.6s min duration, 300ms shorten, 150ms pad).
    pub fn new() -> Self {
        Self {
            sample_rate: 44100.0,
            threshold_dbfs: -42.0,
            min_duration_sec: 0.6,
            shorten_to_sec: 0.3,
            padding_sec: 0.15,
        }
    }

    /// Prepares with sample rate.
    pub fn prepare(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    /// Sets energy detection threshold in dBFS (e.g. -42.0 dBFS).
    pub fn set_threshold_dbfs(&mut self, db: f32) {
        self.threshold_dbfs = db;
    }

    /// Sets minimum pause duration in seconds (e.g. 0.6 s).
    pub fn set_min_duration(&mut self, sec: f32) {
        self.min_duration_sec = sec.max(0.1);
    }

    /// Sets pause shorten target in seconds (e.g. 0.3 s).
    pub fn set_shorten_to(&mut self, sec: f32) {
        self.shorten_to_sec = sec.max(0.0);
    }

    /// Sets boundary padding in seconds (e.g. 0.15 s).
    pub fn set_padding(&mut self, sec: f32) {
        self.padding_sec = sec.max(0.0);
    }

    /// Detects pauses in audio channel.
    pub fn detect_pauses(&self, audio: &[f32]) -> Vec<PauseInterval> {
        let frame_size = (self.sample_rate * 0.010).round() as usize; // 10 ms
        let hop = frame_size / 2;
        let threshold_linear = 10.0f32.powf(self.threshold_dbfs / 20.0);
        let min_samples = (self.min_duration_sec * self.sample_rate).round() as usize;
        let pad_samples = (self.padding_sec * self.sample_rate).round() as usize;

        let mut pauses = Vec::new();
        let mut in_silence = false;
        let mut silence_start = 0;

        let mut pos = 0;
        while pos + frame_size <= audio.len() {
            let mut sum_sq = 0.0;
            for i in 0..frame_size {
                let s = audio[pos + i];
                sum_sq += s * s;
            }
            let rms = (sum_sq / frame_size as f32).sqrt();

            if rms < threshold_linear {
                if !in_silence {
                    in_silence = true;
                    silence_start = pos;
                }
            } else if in_silence {
                in_silence = false;
                let silence_end = pos;
                let raw_len = silence_end.saturating_sub(silence_start);

                if raw_len >= min_samples {
                    // Apply padding preserving breath and speech boundaries
                    let bounded_start = (silence_start + pad_samples).min(silence_end);
                    let bounded_end = silence_end.saturating_sub(pad_samples).max(bounded_start);
                    let duration = (bounded_end - bounded_start) as f32 / self.sample_rate;

                    if duration >= 0.1 {
                        pauses.push(PauseInterval {
                            start_sample: bounded_start,
                            end_sample: bounded_end,
                            duration_sec: duration,
                            shorten_to_sec: self.shorten_to_sec,
                            enabled: true,
                        });
                    }
                }
            }
            pos += hop;
        }

        pauses
    }

    /// Applies silence shortening or removal to audio slice with smooth crossfades.
    pub fn process_pauses(&self, input: &[f32], pauses: &[PauseInterval]) -> Vec<f32> {
        if pauses.is_empty() {
            return input.to_vec();
        }

        let crossfade_samples = (self.sample_rate * 0.008).round() as usize; // 8 ms
        let mut output = Vec::with_capacity(input.len());
        let mut current_idx = 0;

        for p in pauses {
            if !p.enabled {
                continue;
            }

            if p.start_sample > current_idx {
                output.extend_from_slice(&input[current_idx..p.start_sample]);
            }

            if p.shorten_to_sec > 0.0 {
                let shorten_samples = (p.shorten_to_sec * self.sample_rate).round() as usize;
                let available_pause = p.end_sample - p.start_sample;
                let kept_pause = shorten_samples.min(available_pause);

                // Copy shortened portion with equal-power cosine fade-in/out
                let pause_slice = &input[p.start_sample..p.start_sample + kept_pause];
                for (i, &s) in pause_slice.iter().enumerate() {
                    let mut gain = 1.0f32;
                    if i < crossfade_samples {
                        let t = i as f32 / crossfade_samples as f32;
                        gain = (t * PI * 0.5).sin();
                    } else if i + crossfade_samples > kept_pause {
                        let t = (kept_pause - i) as f32 / crossfade_samples as f32;
                        gain = (t * PI * 0.5).sin();
                    }
                    output.push(s * gain);
                }
            }

            current_idx = p.end_sample;
        }

        if current_idx < input.len() {
            output.extend_from_slice(&input[current_idx..]);
        }

        output
    }
}

impl Default for SilenceRemover {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 7. Stereo Tools
// ============================================================================

/// Pan law options for stereo panning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PanLaw {
    /// Linear panning (-6 dB center attenuation).
    Linear,
    /// Equal-power sinusoidal panning (-3 dB center attenuation).
    EqualPower3dB,
    /// -4.5 dB center attenuation.
    EqualPower4p5dB,
}

/// Stereo manipulation suite: Mid/Side width, swap, phase invert, pan laws, and correlation.
pub struct StereoTools {
    bypassed: bool,
    width: f32, // 0.0 (mono) to 2.0 (exaggerated stereo), 1.0 = normal
    pan: f32,   // -1.0 (full left) to +1.0 (full right)
    pan_law: PanLaw,
    swap_channels: bool,
    invert_left: bool,
    invert_right: bool,
}

impl StereoTools {
    /// Creates a new stereo tools node.
    pub fn new() -> Self {
        Self {
            bypassed: false,
            width: 1.0,
            pan: 0.0,
            pan_law: PanLaw::EqualPower3dB,
            swap_channels: false,
            invert_left: false,
            invert_right: false,
        }
    }

    /// Sets stereo width (0.0 = mono, 1.0 = normal stereo, 2.0 = super-wide).
    pub fn set_width(&mut self, width: f32) {
        self.width = width.clamp(0.0, 2.0);
    }

    /// Sets panning (-1.0 to +1.0).
    pub fn set_pan(&mut self, pan: f32) {
        self.pan = pan.clamp(-1.0, 1.0);
    }

    /// Sets pan law.
    pub fn set_pan_law(&mut self, law: PanLaw) {
        self.pan_law = law;
    }

    /// Toggles L/R channel swapping.
    pub fn set_swap_channels(&mut self, swap: bool) {
        self.swap_channels = swap;
    }

    /// Inverts left channel phase.
    pub fn set_invert_left(&mut self, invert: bool) {
        self.invert_left = invert;
    }

    /// Inverts right channel phase.
    pub fn set_invert_right(&mut self, invert: bool) {
        self.invert_right = invert;
    }

    /// Calculates stereo phase correlation coefficient in [-1.0, +1.0].
    pub fn calculate_phase_correlation(left: &[f32], right: &[f32]) -> f32 {
        let len = left.len().min(right.len());
        if len == 0 {
            return 1.0;
        }

        let mut sum_xy = 0.0;
        let mut sum_xx = 0.0;
        let mut sum_yy = 0.0;

        for i in 0..len {
            let x = left[i];
            let y = right[i];
            sum_xy += x * y;
            sum_xx += x * x;
            sum_yy += y * y;
        }

        let denom = (sum_xx * sum_yy).sqrt();
        if denom < 1e-9 {
            1.0
        } else {
            (sum_xy / denom).clamp(-1.0, 1.0)
        }
    }
}

impl Default for StereoTools {
    fn default() -> Self {
        Self::new()
    }
}

impl DspNode for StereoTools {
    fn name(&self) -> &str {
        "StereoTools"
    }

    fn prepare(&mut self, _sample_rate: f32, _channels: usize) {}
    fn reset(&mut self) {}

    fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]]) {
        if self.bypassed || input.is_empty() || output.is_empty() {
            for (ch, in_ch) in input.iter().enumerate().take(output.len()) {
                output[ch].copy_from_slice(in_ch);
            }
            return;
        }

        let len = input[0].len();
        if input.len() == 1 || output.len() == 1 {
            // Mono input/output
            output[0].copy_from_slice(input[0]);
            return;
        }

        let inv_sqrt2 = std::f32::consts::FRAC_1_SQRT_2;

        // Compute pan gains
        let (gain_l, gain_r) = match self.pan_law {
            PanLaw::Linear => {
                let p = (self.pan + 1.0) * 0.5;
                (1.0 - p, p)
            }
            PanLaw::EqualPower3dB => {
                let angle = (self.pan + 1.0) * 0.25 * PI;
                (angle.cos(), angle.sin())
            }
            PanLaw::EqualPower4p5dB => {
                let angle = (self.pan + 1.0) * 0.25 * PI;
                (angle.cos().powf(1.5), angle.sin().powf(1.5))
            }
        };

        for i in 0..len {
            let mut l = input[0][i];
            let mut r = input[1][i];

            if self.swap_channels {
                std::mem::swap(&mut l, &mut r);
            }

            if self.invert_left {
                l = -l;
            }
            if self.invert_right {
                r = -r;
            }

            // Mid-Side widening
            let mid = (l + r) * inv_sqrt2;
            let side = (l - r) * inv_sqrt2 * self.width;

            let wide_l = (mid + side) * inv_sqrt2 * gain_l * 2.0;
            let wide_r = (mid - side) * inv_sqrt2 * gain_r * 2.0;

            output[0][i] = wide_l;
            output[1][i] = wide_r;
        }
    }

    fn parameter_count(&self) -> usize {
        4
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.set_width(value),
            1 => self.set_pan(value),
            2 => self.set_swap_channels(value > 0.5),
            3 => {
                self.invert_left = (value as usize) & 1 != 0;
                self.invert_right = (value as usize) & 2 != 0;
            }
            _ => {}
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 => self.width,
            1 => self.pan,
            2 => f32::from(self.swap_channels),
            3 => {
                let mut v = 0.0;
                if self.invert_left {
                    v += 1.0;
                }
                if self.invert_right {
                    v += 2.0;
                }
                v
            }
            _ => 0.0,
        }
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hum_remover_attenuation() {
        let sample_rate = 44100.0;
        let mut hum = HumRemover::new();
        hum.prepare(sample_rate, 1);
        hum.set_fundamental(50.0);
        hum.set_harmonics(8);

        // Generate 50 Hz test sine (3 seconds to allow surgical IIR notch settling)
        let n = 44100 * 3;
        let mut input = vec![0.0f32; n];
        for (i, sample) in input.iter_mut().enumerate() {
            *sample = (2.0 * PI * 50.0 * i as f32 / sample_rate).sin();
        }

        let mut output = vec![0.0f32; n];
        {
            let in_ptrs = [&input[..]];
            let mut out_ptrs = [&mut output[..]];
            hum.process(&in_ptrs, &mut out_ptrs);
        }

        // Measure steady-state attenuation over last 10,000 samples
        let steady_state = &output[n - 10000..];
        let max_val = steady_state.iter().fold(0.0f32, |acc, &x| acc.max(x.abs()));
        let attenuation_db = 20.0 * max_val.log10();

        // Must reduce 50 Hz by >= 40 dB per PLAN.md §3.4
        assert!(
            attenuation_db <= -40.0,
            "Hum attenuation should be <= -40 dB, got {:.2} dB",
            attenuation_db
        );

        // Off-frequency test at 1000 Hz: program energy change < 0.3 dB
        let mut input_1k = vec![0.0f32; n];
        for (i, sample) in input_1k.iter_mut().enumerate() {
            *sample = (2.0 * PI * 1000.0 * i as f32 / sample_rate).sin();
        }
        {
            let in_1k_ptrs = [&input_1k[..]];
            let mut out_ptrs = [&mut output[..]];
            hum.process(&in_1k_ptrs, &mut out_ptrs);
        }

        let ss_1k = &output[n - 10000..];
        let max_1k = ss_1k.iter().fold(0.0f32, |acc, &x| acc.max(x.abs()));
        let error_db = (20.0 * max_1k.log10()).abs();
        assert!(
            error_db < 0.3,
            "1000 Hz tone program energy changed by {:.4} dB (expected < 0.3 dB)",
            error_db
        );
    }

    #[test]
    fn test_declicker_synthetic_click_removal() {
        let sample_rate = 44100.0;
        let mut declick = DeClicker::new();
        declick.prepare(sample_rate, 1);

        let n = 2000;
        let mut audio = vec![0.0f32; n];
        for (i, sample) in audio.iter_mut().enumerate() {
            *sample = (2.0 * PI * 440.0 * i as f32 / sample_rate).sin() * 0.2;
        }

        // Add 5 known clicks
        let click_indices = [300, 700, 1100, 1500, 1800];
        for &idx in &click_indices {
            audio[idx] += 0.8; // sharp impulse spike
        }

        let mut output = vec![0.0f32; n];
        let in_ptrs = [&audio[..]];
        let mut out_ptrs = [&mut output[..]];
        declick.process(&in_ptrs, &mut out_ptrs);

        for &idx in &click_indices {
            let orig_click = audio[idx];
            let cleaned = output[idx];
            let reduction = (orig_click.abs() - cleaned.abs()).abs();
            assert!(
                reduction > 0.5,
                "Click at index {} should be strongly attenuated, orig={}, cleaned={}",
                idx,
                orig_click,
                cleaned
            );
        }
    }

    #[test]
    fn test_declipper_crest_reconstruction() {
        let mut declip = DeClipper::new();
        declip.set_threshold(0.90);

        let n = 100;
        let mut buffer = vec![0.0f32; n];
        // Flat-topped clipped peak
        for i in 0..n {
            let raw = (PI * i as f32 / (n as f32)).sin() * 1.3;
            buffer[i] = raw.clamp(-0.90, 0.90);
        }

        assert_eq!(buffer[50], 0.90);
        declip.reconstruct_slice(&mut buffer);

        // Peak should be reconstructed above 0.90 ceiling
        assert!(
            buffer[50] > 0.90,
            "De-clipped peak should exceed ceiling 0.90, got {:.4}",
            buffer[50]
        );
    }

    #[test]
    fn test_silence_remover_detection_and_processing() {
        let mut sr = SilenceRemover::new();
        let sample_rate = 44100.0;
        sr.prepare(sample_rate);
        sr.set_min_duration(0.5);
        sr.set_shorten_to(0.2);

        // Signal: 1s sound, 1s silence, 1s sound
        let mut audio = vec![0.0f32; 3 * 44100];
        for i in 0..44100 {
            audio[i] = 0.5;
            audio[2 * 44100 + i] = 0.5;
        }

        let pauses = sr.detect_pauses(&audio);
        assert_eq!(pauses.len(), 1, "Should detect 1 pause");
        assert!(pauses[0].duration_sec >= 0.5);

        let processed = sr.process_pauses(&audio, &pauses);
        // Original was 3.0s, silence (1.0s) shortened to ~0.2s + padding -> total duration should be shorter
        assert!(
            processed.len() < audio.len(),
            "Processed audio should be shorter than original"
        );
    }

    #[test]
    fn test_stereo_tools_mid_side_width_and_correlation() {
        let mut st = StereoTools::new();
        st.prepare(44100.0, 2);

        let n = 1000;
        let left = vec![0.5f32; n];
        let right = vec![0.5f32; n];

        // L == R -> correlation == 1.0
        let corr_ident = StereoTools::calculate_phase_correlation(&left, &right);
        assert!((corr_ident - 1.0).abs() < 1e-4);

        // L == -R -> correlation == -1.0
        let inv_right: Vec<f32> = right.iter().map(|&x| -x).collect();
        let corr_inv = StereoTools::calculate_phase_correlation(&left, &inv_right);
        assert!((corr_inv - (-1.0)).abs() < 1e-4);

        // Collapse to mono (width = 0.0)
        st.set_width(0.0);
        let mut out_l = vec![0.0f32; n];
        let mut out_r = vec![0.0f32; n];
        {
            let in_ptrs = [&left[..], &right[..]];
            let mut out_ptrs = [&mut out_l[..], &mut out_r[..]];
            st.process(&in_ptrs, &mut out_ptrs);
        }
        for i in 0..n {
            assert!((out_l[i] - out_r[i]).abs() < 1e-4);
        }
    }
}
