//! Delay lines with integer and fractional interpolation.

use crate::node::DspNode;

/// Delay line with fractional sample interpolation.
#[derive(Debug, Clone)]
pub struct DelayLine {
    buffer: Vec<f32>,
    write_pos: usize,
    delay_samples: f32,
    max_delay: usize,
    bypassed: bool,
}

impl DelayLine {
    /// Creates a new delay line with a maximum capacity.
    pub fn new(max_delay_samples: usize) -> Self {
        let capacity = (max_delay_samples + 4).max(16);
        Self {
            buffer: vec![0.0; capacity],
            write_pos: 0,
            delay_samples: 0.0,
            max_delay: max_delay_samples,
            bypassed: false,
        }
    }

    /// Sets the target delay in fractional samples.
    pub fn set_delay(&mut self, delay: f32) {
        self.delay_samples = delay.clamp(0.0, self.max_delay as f32);
    }

    /// Returns the current delay in samples.
    pub fn delay(&self) -> f32 {
        self.delay_samples
    }

    /// Processes a single sample and returns the delayed sample using cubic Hermite interpolation.
    #[inline]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        let cap = self.buffer.len();
        self.buffer[self.write_pos] = input;

        if self.delay_samples <= 0.0 {
            self.write_pos = (self.write_pos + 1) % cap;
            return input;
        }

        // Fractional delay calculation
        let int_delay = self.delay_samples.floor() as usize;
        let frac = self.delay_samples - (int_delay as f32);

        // Indices for 4-point cubic Hermite interpolation
        let read_pos = (self.write_pos + cap - int_delay) % cap;
        let y0 = self.buffer[(read_pos + cap - 1) % cap];
        let y1 = self.buffer[read_pos];
        let y2 = self.buffer[(read_pos + 1) % cap];
        let y3 = self.buffer[(read_pos + 2) % cap];

        // 4-point, 3rd-order Hermite x-form
        let c0 = y1;
        let c1 = 0.5 * (y2 - y0);
        let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
        let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);

        let out = ((c3 * frac + c2) * frac + c1) * frac + c0;

        self.write_pos = (self.write_pos + 1) % cap;
        out
    }

    /// Writes a sample to the delay line buffer and advances the write position.
    #[inline]
    pub fn write(&mut self, input: f32) {
        let cap = self.buffer.len();
        self.buffer[self.write_pos] = input;
        self.write_pos = (self.write_pos + 1) % cap;
    }

    /// Reads a sample at a given fractional delay relative to the current write position.
    #[inline]
    pub fn read_samples(&self, delay_samples: f32) -> f32 {
        let cap = self.buffer.len();
        let clamped = delay_samples.clamp(0.0, self.max_delay as f32);
        let int_delay = clamped.floor() as usize;
        let frac = clamped - (int_delay as f32);

        let read_pos = (self.write_pos + cap * 2 - 1 - (int_delay % cap)) % cap;
        let y0 = self.buffer[(read_pos + cap - 1) % cap];
        let y1 = self.buffer[read_pos];
        let y2 = self.buffer[(read_pos + 1) % cap];
        let y3 = self.buffer[(read_pos + 2) % cap];

        let c0 = y1;
        let c1 = 0.5 * (y2 - y0);
        let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
        let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);

        ((c3 * frac + c2) * frac + c1) * frac + c0
    }
}

impl DspNode for DelayLine {
    fn name(&self) -> &str {
        "DelayLine"
    }

    fn prepare(&mut self, _sample_rate: f32, _channels: usize) {
        self.reset();
    }

    fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]]) {
        if self.bypassed {
            let chs = input.len().min(output.len());
            for ch in 0..chs {
                let frames = input[ch].len().min(output[ch].len());
                output[ch][..frames].copy_from_slice(&input[ch][..frames]);
            }
            return;
        }

        if input.is_empty() || output.is_empty() {
            return;
        }

        let in_ch = input[0];
        let out_ch = &mut output[0];
        let frames = in_ch.len().min(out_ch.len());

        for i in 0..frames {
            out_ch[i] = self.process_sample(in_ch[i]);
        }
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
    }

    fn latency_samples(&self) -> usize {
        self.delay_samples.round() as usize
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        if index == 0 {
            self.set_delay(value);
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        if index == 0 {
            self.delay_samples
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
