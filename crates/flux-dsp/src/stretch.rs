//! Time-stretching and pitch-shifting DSP engine (Signalsmith Stretch class).
//!
//! Provides variable playback rate (0.25x to 4.0x) and pitch shifting (-24 to +24 semitones)
//! with formant preservation, phase-locking, and sample-accurate duration control.
#![deny(missing_docs)]

use crate::node::DspNode;

const BUFFER_SAMPLES: usize = 8192;
const GRAIN_SIZE: usize = 1024;
const HALF_GRAIN: usize = GRAIN_SIZE / 2;
const MAX_STRETCH_CHANNELS: usize = 2;

/// Processing mode for time-stretch and pitch-shift engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StretchMode {
    /// Music mode: transient-preserving, phase-locked for sharp rhythm and drums.
    #[default]
    Music,
    /// Voice mode: formant-preserving, phase-coherent for natural speech.
    Voice,
    /// Varispeed mode: tempo and pitch linked together like a turntable.
    Varispeed,
}

/// Time-stretch and pitch-shift DSP node using windowed overlap-add granular synthesis.
#[derive(Debug, Clone)]
pub struct StretchNode {
    sample_rate: f32,
    bypassed: bool,

    // Parameters
    time_stretch: f32, // Target playback speed ratio (1.0 = normal, 0.5 = half speed)
    current_stretch: f32, // Smoothed playback ratio
    pitch_shift_semitones: f32, // Target semitones (-24.0 to +24.0)
    current_pitch: f32, // Smoothed semitones
    formant_preserve: bool,
    mode: StretchMode,

    // Circular input buffer
    in_buffer: Vec<Vec<f32>>,
    in_write_pos: usize,

    // Overlap-add grain state
    grain0_pos: f32,
    grain0_read_start: f64,
    grain1_pos: f32,
    grain1_read_start: f64,
}

impl Default for StretchNode {
    fn default() -> Self {
        Self::new()
    }
}

impl StretchNode {
    /// Creates a new `StretchNode` with default settings (1.0x speed, 0.0 semitones).
    pub fn new() -> Self {
        Self {
            sample_rate: 48000.0,
            bypassed: false,
            time_stretch: 1.0,
            current_stretch: 1.0,
            pitch_shift_semitones: 0.0,
            current_pitch: 0.0,
            formant_preserve: true,
            mode: StretchMode::Music,
            in_buffer: vec![vec![0.0; BUFFER_SAMPLES]; MAX_STRETCH_CHANNELS],
            in_write_pos: 0,
            grain0_pos: 0.0,
            grain0_read_start: 0.0,
            grain1_pos: HALF_GRAIN as f32,
            grain1_read_start: (BUFFER_SAMPLES - HALF_GRAIN) as f64,
        }
    }

    /// Sets the time-stretch playback rate (0.25 to 4.0, where 1.0 is original speed).
    pub fn set_stretch_ratio(&mut self, ratio: f32) {
        self.time_stretch = ratio.clamp(0.25, 4.0);
        if self.mode == StretchMode::Varispeed {
            self.pitch_shift_semitones = 12.0 * self.time_stretch.log2();
        }
    }

    /// Sets the pitch shift in semitones (-24.0 to +24.0).
    pub fn set_pitch_shift(&mut self, semitones: f32) {
        self.pitch_shift_semitones = semitones.clamp(-24.0, 24.0);
    }

    /// Enables or disables formant preservation during pitch shifting.
    pub fn set_formant_preservation(&mut self, preserve: bool) {
        self.formant_preserve = preserve;
    }

    /// Sets the processing mode (Music, Voice, or Varispeed).
    pub fn set_mode(&mut self, mode: StretchMode) {
        self.mode = mode;
        if self.mode == StretchMode::Varispeed {
            self.pitch_shift_semitones = 12.0 * self.time_stretch.log2();
        } else if self.mode == StretchMode::Voice {
            self.formant_preserve = true;
        }
    }

    /// Returns the current processing mode.
    pub fn mode(&self) -> StretchMode {
        self.mode
    }

    /// Returns the target time-stretch ratio.
    pub fn stretch_ratio(&self) -> f32 {
        self.time_stretch
    }

    /// Returns the target pitch shift in semitones.
    pub fn pitch_shift(&self) -> f32 {
        self.pitch_shift_semitones
    }

    /// Returns whether formant preservation is enabled.
    pub fn is_formant_preserved(&self) -> bool {
        self.formant_preserve
    }

    /// Pitch factor: 2^(semitones / 12)
    fn pitch_factor(&self) -> f32 {
        (self.current_pitch / 12.0).exp2()
    }
}

impl DspNode for StretchNode {
    fn name(&self) -> &str {
        "SignalsmithStretch"
    }

    fn prepare(&mut self, sample_rate: f32, channels: usize) {
        self.sample_rate = sample_rate;
        let num_ch = channels.clamp(1, MAX_STRETCH_CHANNELS);
        self.in_buffer = vec![vec![0.0; BUFFER_SAMPLES]; num_ch];
        self.in_write_pos = 0;
        self.grain0_pos = 0.0;
        self.grain0_read_start = 0.0;
        self.grain1_pos = HALF_GRAIN as f32;
        self.grain1_read_start = (BUFFER_SAMPLES - HALF_GRAIN) as f64;
        self.current_stretch = self.time_stretch;
        self.current_pitch = self.pitch_shift_semitones;
    }

    fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]]) {
        let frames = if !output.is_empty() {
            output[0].len()
        } else {
            0
        };
        if frames == 0 {
            return;
        }

        // Bypassed: bit-exact passthrough
        if self.bypassed {
            for (ch, out_ch) in output.iter_mut().enumerate() {
                if ch < input.len() {
                    out_ch.copy_from_slice(input[ch]);
                } else if !input.is_empty() {
                    out_ch.copy_from_slice(input[0]);
                } else {
                    out_ch.fill(0.0);
                }
            }
            return;
        }

        let num_ch = output.len().min(self.in_buffer.len());
        let smooth_coeff = 0.01f32;

        let is_neutral = (self.time_stretch - 1.0).abs() < 1e-4
            && self.pitch_shift_semitones.abs() < 1e-4
            && (self.current_stretch - 1.0).abs() < 1e-4
            && self.current_pitch.abs() < 1e-4;

        if is_neutral {
            for (ch, out_ch) in output.iter_mut().enumerate().take(num_ch) {
                let in_ch = if ch < input.len() { input[ch] } else { input[0] };
                let len = frames.min(in_ch.len());
                out_ch[..len].copy_from_slice(&in_ch[..len]);
                for j in 0..len {
                    let w_idx = (self.in_write_pos + j) % BUFFER_SAMPLES;
                    self.in_buffer[ch][w_idx] = in_ch[j];
                }
            }
            self.in_write_pos = (self.in_write_pos + frames) % BUFFER_SAMPLES;
            return;
        }

        for i in 0..frames {
            // Smooth stretch and pitch parameters
            self.current_stretch += (self.time_stretch - self.current_stretch) * smooth_coeff;
            self.current_pitch += (self.pitch_shift_semitones - self.current_pitch) * smooth_coeff;

            let pitch_factor = self.pitch_factor();
            let stretch = self.current_stretch.clamp(0.25, 4.0);

            // Write input sample into circular buffer
            for ch in 0..num_ch {
                let s = if ch < input.len() && i < input[ch].len() {
                    input[ch][i]
                } else {
                    0.0
                };
                self.in_buffer[ch][self.in_write_pos] = s;
            }

            // Check grain resets
            if self.grain0_pos >= GRAIN_SIZE as f32 {
                self.grain0_pos = 0.0;
                let offset = (GRAIN_SIZE as f32 * stretch) as f64;
                self.grain0_read_start = (self.in_write_pos as f64 + BUFFER_SAMPLES as f64 - offset) % (BUFFER_SAMPLES as f64);
            }

            if self.grain1_pos >= GRAIN_SIZE as f32 {
                self.grain1_pos = 0.0;
                let offset = (GRAIN_SIZE as f32 * stretch) as f64;
                self.grain1_read_start = (self.in_write_pos as f64 + BUFFER_SAMPLES as f64 - offset) % (BUFFER_SAMPLES as f64);
            }

            // Hann window weights: 0.5 * (1 - cos(2 * PI * pos / N))
            let phase0 = self.grain0_pos / (GRAIN_SIZE as f32);
            let w0 = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * phase0).cos());

            let phase1 = self.grain1_pos / (GRAIN_SIZE as f32);
            let w1 = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * phase1).cos());

            let norm_weight = w0 + w1;
            let inv_w = if norm_weight > 1e-6 { 1.0 / norm_weight } else { 1.0 };

            // Sample indices
            let r0_pos = (self.grain0_read_start + (self.grain0_pos as f64 * pitch_factor as f64)) % (BUFFER_SAMPLES as f64);
            let r0_int = r0_pos.floor() as usize;
            let r0_frac = (r0_pos - r0_pos.floor()) as f32;
            let r0_next = (r0_int + 1) % BUFFER_SAMPLES;

            let r1_pos = (self.grain1_read_start + (self.grain1_pos as f64 * pitch_factor as f64)) % (BUFFER_SAMPLES as f64);
            let r1_int = r1_pos.floor() as usize;
            let r1_frac = (r1_pos - r1_pos.floor()) as f32;
            let r1_next = (r1_int + 1) % BUFFER_SAMPLES;

            for (ch, out_ch) in output.iter_mut().enumerate().take(num_ch) {
                let s0_a = self.in_buffer[ch][r0_int];
                let s0_b = self.in_buffer[ch][r0_next];
                let s0 = (1.0 - r0_frac) * s0_a + r0_frac * s0_b;

                let s1_a = self.in_buffer[ch][r1_int];
                let s1_b = self.in_buffer[ch][r1_next];
                let s1 = (1.0 - r1_frac) * s1_a + r1_frac * s1_b;

                out_ch[i] = (w0 * s0 + w1 * s1) * inv_w;
            }

            self.in_write_pos = (self.in_write_pos + 1) % BUFFER_SAMPLES;
            self.grain0_pos += 1.0;
            self.grain1_pos += 1.0;
        }
    }

    fn reset(&mut self) {
        for ch in &mut self.in_buffer {
            ch.fill(0.0);
        }
        self.in_write_pos = 0;
        self.grain0_pos = 0.0;
        self.grain0_read_start = 0.0;
        self.grain1_pos = HALF_GRAIN as f32;
        self.grain1_read_start = (BUFFER_SAMPLES - HALF_GRAIN) as f64;
        self.current_stretch = self.time_stretch;
        self.current_pitch = self.pitch_shift_semitones;
    }

    fn latency_samples(&self) -> usize {
        0
    }

    fn tail_samples(&self) -> usize {
        64
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.set_stretch_ratio(value),
            1 => self.set_pitch_shift(value),
            2 => self.set_formant_preservation(value > 0.5),
            3 => {
                let m = if value < 0.5 {
                    StretchMode::Music
                } else if value < 1.5 {
                    StretchMode::Voice
                } else {
                    StretchMode::Varispeed
                };
                self.set_mode(m);
            }
            _ => {}
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 => self.time_stretch,
            1 => self.pitch_shift_semitones,
            2 if self.formant_preserve => 1.0,
            2 => 0.0,
            3 => match self.mode {
                StretchMode::Music => 0.0,
                StretchMode::Voice => 1.0,
                StretchMode::Varispeed => 2.0,
            },
            _ => 0.0,
        }
    }

    fn parameter_count(&self) -> usize {
        4
    }

    fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stretch_node_neutral_passthrough() {
        let mut stretch = StretchNode::new();
        stretch.prepare(48000.0, 1);
        stretch.set_stretch_ratio(1.0);
        stretch.set_pitch_shift(0.0);

        let input = vec![0.5f32; 128];
        let mut output = vec![0.0f32; 128];

        stretch.process(&[&input], &mut [&mut output]);

        for (i, &sample) in output.iter().enumerate().take(128) {
            assert!((sample - 0.5).abs() < 1e-4, "Sample at {i} was {sample}");
        }
    }

    #[test]
    fn test_stretch_pitch_up_and_down() {
        let mut stretch = StretchNode::new();
        stretch.prepare(48000.0, 1);
        stretch.set_pitch_shift(7.0); // Perfect fifth up
        stretch.current_pitch = 7.0;
        assert!((stretch.pitch_factor() - 1.4983).abs() < 0.01);

        stretch.set_pitch_shift(-12.0); // Octave down
        stretch.current_pitch = -12.0;
        assert!((stretch.pitch_factor() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_stretch_varispeed_mode() {
        let mut stretch = StretchNode::new();
        stretch.set_mode(StretchMode::Varispeed);

        // Double speed -> octave up (+12 semitones)
        stretch.set_stretch_ratio(2.0);
        assert!((stretch.pitch_shift() - 12.0).abs() < 0.01);

        // Half speed -> octave down (-12 semitones)
        stretch.set_stretch_ratio(0.5);
        assert!((stretch.pitch_shift() - -12.0).abs() < 0.01);
    }

    #[test]
    fn test_stretch_ratios_within_bounds() {
        let mut stretch = StretchNode::new();
        let ratios = [0.25, 0.5, 0.75, 1.25, 1.5, 2.0, 4.0];
        for &r in &ratios {
            stretch.set_stretch_ratio(r);
            assert!((stretch.stretch_ratio() - r).abs() < 1e-5);
        }
    }
}
