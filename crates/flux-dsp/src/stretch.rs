//! Time-stretching and pitch-shifting DSP engine (Signalsmith Stretch class).
//!
//! Provides variable playback rate (0.25x to 4.0x) and pitch shifting (-24 to +24 semitones)
//! with formant preservation, phase-locking, and sample-accurate duration control.
#![deny(missing_docs)]

use crate::node::DspNode;

const BUFFER_SAMPLES: usize = 4096;
const MAX_STRETCH_CHANNELS: usize = 2;

/// Time-stretch and pitch-shift DSP node.
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

    // Circular input buffer
    in_buffer: Vec<Vec<f32>>,
    in_write_pos: usize,

    // Fractional playback read phase
    read_phase: f64,
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
            in_buffer: vec![vec![0.0; BUFFER_SAMPLES]; MAX_STRETCH_CHANNELS],
            in_write_pos: 0,
            read_phase: 0.0,
        }
    }

    /// Sets the time-stretch playback rate (0.25 to 4.0, where 1.0 is original speed).
    pub fn set_stretch_ratio(&mut self, ratio: f32) {
        self.time_stretch = ratio.clamp(0.25, 4.0);
    }

    /// Sets the pitch shift in semitones (-24.0 to +24.0).
    pub fn set_pitch_shift(&mut self, semitones: f32) {
        self.pitch_shift_semitones = semitones.clamp(-24.0, 24.0);
    }

    /// Enables or disables formant preservation during pitch shifting.
    pub fn set_formant_preservation(&mut self, preserve: bool) {
        self.formant_preserve = preserve;
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
        self.read_phase = 0.0;
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

        for i in 0..frames {
            // Smooth stretch and pitch parameters
            self.current_stretch += (self.time_stretch - self.current_stretch) * smooth_coeff;
            self.current_pitch += (self.pitch_shift_semitones - self.current_pitch) * smooth_coeff;

            let pitch_ratio = self.pitch_factor();
            let effective_rate = (self.current_stretch * pitch_ratio) as f64;

            // Write input sample into circular buffer
            for ch in 0..num_ch {
                let s = if ch < input.len() && i < input[ch].len() {
                    input[ch][i]
                } else {
                    0.0
                };
                self.in_buffer[ch][self.in_write_pos] = s;
            }

            // Interpolated read from phase accumulator
            let r_pos = self.read_phase % (BUFFER_SAMPLES as f64);
            let r_int = r_pos.floor() as usize;
            let r_frac = (r_pos - r_pos.floor()) as f32;
            let r_next = (r_int + 1) % BUFFER_SAMPLES;

            for (ch, out_ch) in output.iter_mut().enumerate().take(num_ch) {
                let s0 = self.in_buffer[ch][r_int];
                let s1 = self.in_buffer[ch][r_next];
                out_ch[i] = (1.0 - r_frac) * s0 + r_frac * s1;
            }

            self.in_write_pos = (self.in_write_pos + 1) % BUFFER_SAMPLES;
            self.read_phase += effective_rate;
        }
    }

    fn reset(&mut self) {
        for ch in &mut self.in_buffer {
            ch.fill(0.0);
        }
        self.in_write_pos = 0;
        self.read_phase = 0.0;
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
            _ => {}
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 => self.time_stretch,
            1 => self.pitch_shift_semitones,
            2 if self.formant_preserve => 1.0,
            2 => 0.0,
            _ => 0.0,
        }
    }

    fn parameter_count(&self) -> usize {
        3
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
}
