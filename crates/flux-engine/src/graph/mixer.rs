//! Bus mixing, panning laws, and track summing junctions.

use flux_dsp::param::{SmoothedParameter, SmoothingMode};
use std::f32::consts::FRAC_PI_2;

/// Panning law determining center attenuation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanLaw {
    /// Linear panning (-6 dB center dip).
    Linear,
    /// Constant power (-3 dB center dip, maintaining constant total acoustic power).
    ConstantPower,
    /// Compromise (-4.5 dB center dip).
    ConstantPower45,
}

/// A mixer bus with gain, panning, mute, and solo controls.
#[derive(Debug, Clone)]
pub struct Bus {
    /// Bus identifier / name.
    pub name: String,
    /// Smoothed linear volume gain.
    pub gain: SmoothedParameter,
    /// Smoothed pan position (-1.0 hard left to +1.0 hard right, 0.0 center).
    pub pan: SmoothedParameter,
    /// Mute toggle.
    pub mute: bool,
    /// Solo toggle.
    pub solo: bool,
    /// Pan law.
    pub pan_law: PanLaw,
    sample_rate: f32,
}

impl Bus {
    /// Creates a new mixer bus.
    pub fn new(name: impl Into<String>, sample_rate: f32) -> Self {
        Self {
            name: name.into(),
            gain: SmoothedParameter::new(1.0, sample_rate),
            pan: SmoothedParameter::new(0.0, sample_rate),
            mute: false,
            solo: false,
            pan_law: PanLaw::ConstantPower,
            sample_rate,
        }
    }

    /// Returns the sample rate of the bus.
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Sets volume in decibels with smoothing.
    pub fn set_gain_db(&mut self, gain_db: f32, duration_sec: f32) {
        let linear = if gain_db <= -100.0 {
            0.0
        } else {
            10.0f32.powf(gain_db / 20.0)
        };
        self.gain
            .set_target(linear, duration_sec, SmoothingMode::Linear);
    }

    /// Sets pan position (-1.0 to 1.0) with smoothing.
    pub fn set_pan(&mut self, pan: f32, duration_sec: f32) {
        let clamped = pan.clamp(-1.0, 1.0);
        self.pan
            .set_target(clamped, duration_sec, SmoothingMode::Linear);
    }

    /// Computes left and right gain coefficients for a given pan value.
    #[inline]
    pub fn compute_pan_gains(&self, pan_val: f32) -> (f32, f32) {
        // Map pan [-1.0, 1.0] to [0.0, 1.0]
        let p = (pan_val + 1.0) * 0.5;

        match self.pan_law {
            PanLaw::Linear => (1.0 - p, p),
            PanLaw::ConstantPower => {
                let angle = p * FRAC_PI_2;
                (angle.cos(), angle.sin())
            }
            PanLaw::ConstantPower45 => {
                let angle = p * FRAC_PI_2;
                let scale = 1.0 / (2.0f32.powf(0.25));
                (angle.cos() * scale, angle.sin() * scale)
            }
        }
    }

    /// Mixes an input buffer (stereo or mono) into the destination stereo buffer with gain and pan applied.
    pub fn mix_into(&mut self, input: &[&[f32]], output: &mut [&mut [f32]]) {
        if self.mute || output.len() < 2 || input.is_empty() {
            return;
        }

        let num_frames = output[0].len().min(output[1].len());
        let in_left = input[0];
        let in_right = if input.len() > 1 { input[1] } else { input[0] };
        let frames = num_frames.min(in_left.len()).min(in_right.len());

        let (left_slice, right_slice) = output.split_at_mut(1);
        let out_left = &mut left_slice[0];
        let out_right = &mut right_slice[0];

        for i in 0..frames {
            let g = self.gain.next();
            let p = self.pan.next();
            let (pan_l, pan_r) = self.compute_pan_gains(p);

            let l_sample = in_left[i] * g * pan_l;
            let r_sample = in_right[i] * g * pan_r;

            out_left[i] += l_sample;
            out_right[i] += r_sample;
        }
    }
}
