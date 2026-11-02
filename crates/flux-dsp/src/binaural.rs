//! Binaural 3D and 8D spatial audio panner using spherical head model (ITD + ILD + head shadowing).
#![deny(missing_docs)]

use crate::node::DspNode;
use std::f32::consts::PI;

const MAX_DELAY_SAMPLES: usize = 256;
const DEFAULT_HEAD_RADIUS: f32 = 0.0875; // meters
const SPEED_OF_SOUND: f32 = 343.0; // m/s
const DENORMAL_THRESHOLD: f32 = 1e-20;

/// Binaural 3D/8D spatializer node implementing head-related acoustic cues.
///
/// Implements Woodworth-Schlosser Interaural Time Difference (ITD), head-shadow
/// Interaural Level Difference (ILD), continuous parameter smoothing, and automated
/// 8D orbit rotation around the listener.
#[derive(Debug, Clone)]
pub struct BinauralNode {
    sample_rate: f32,
    bypassed: bool,

    // Parameters
    azimuth_deg: f32,
    target_azimuth_deg: f32,
    elevation_deg: f32,
    orbit_speed_hz: f32,
    head_radius_m: f32,

    // Current smoothed delays (in samples)
    delay_samples_l: f32,
    delay_samples_r: f32,

    // Delay lines for ITD
    delay_line_l: Vec<f32>,
    delay_line_r: Vec<f32>,
    write_idx: usize,

    // Filter states for head shadow lowpass
    lowpass_state_l: f32,
    lowpass_state_r: f32,
}

impl Default for BinauralNode {
    fn default() -> Self {
        Self::new()
    }
}

impl BinauralNode {
    /// Creates a new binaural panner node.
    pub fn new() -> Self {
        Self {
            sample_rate: 48000.0,
            bypassed: false,
            azimuth_deg: 0.0,
            target_azimuth_deg: 0.0,
            elevation_deg: 0.0,
            orbit_speed_hz: 0.0,
            head_radius_m: DEFAULT_HEAD_RADIUS,
            delay_samples_l: 0.0,
            delay_samples_r: 0.0,
            delay_line_l: vec![0.0; MAX_DELAY_SAMPLES],
            delay_line_r: vec![0.0; MAX_DELAY_SAMPLES],
            write_idx: 0,
            lowpass_state_l: 0.0,
            lowpass_state_r: 0.0,
        }
    }

    /// Sets the azimuth angle in degrees ([-180.0, 180.0] or [0.0, 360.0]).
    pub fn set_azimuth(&mut self, degrees: f32) {
        self.target_azimuth_deg = degrees;
    }

    /// Sets automated 8D orbit speed in Hz (e.g. 0.1 Hz = 1 rotation per 10 seconds).
    pub fn set_orbit_speed(&mut self, speed_hz: f32) {
        self.orbit_speed_hz = speed_hz;
    }

    /// Computes continuous Woodworth ITD delays in seconds for a given azimuth angle.
    fn compute_itd(azimuth_rad: f32, radius: f32) -> (f32, f32) {
        let sin_theta = azimuth_rad.sin();
        let c = SPEED_OF_SOUND;

        if sin_theta >= 0.0 {
            // Source on the right: right ear receives signal earlier
            let side_angle = sin_theta.clamp(-1.0, 1.0).asin();
            let delay_r = (radius / c) * (1.0 - sin_theta);
            let delay_l = (radius / c) * (1.0 + side_angle);
            (delay_l, delay_r)
        } else {
            // Source on the left: left ear receives signal earlier
            let abs_sin = (-sin_theta).clamp(0.0, 1.0);
            let side_angle = abs_sin.asin();
            let delay_l = (radius / c) * (1.0 - abs_sin);
            let delay_r = (radius / c) * (1.0 + side_angle);
            (delay_l, delay_r)
        }
    }
}

impl DspNode for BinauralNode {
    fn name(&self) -> &str {
        "BinauralPanner"
    }

    fn prepare(&mut self, sample_rate: f32, _channels: usize) {
        self.sample_rate = sample_rate;
        self.delay_line_l = vec![0.0; MAX_DELAY_SAMPLES];
        self.delay_line_r = vec![0.0; MAX_DELAY_SAMPLES];
        self.write_idx = 0;
        self.lowpass_state_l = 0.0;
        self.lowpass_state_r = 0.0;

        let az_rad = self.azimuth_deg * PI / 180.0;
        let (dl, dr) = Self::compute_itd(az_rad, self.head_radius_m);
        self.delay_samples_l = dl * sample_rate;
        self.delay_samples_r = dr * sample_rate;
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

        // Bypassed: pass input directly to output bit-exactly
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

        let in_left = if !input.is_empty() { input[0] } else { &[][..] };
        let in_right = if input.len() > 1 { input[1] } else { in_left };

        // Ensure stereo output buffers
        let (out_left, out_right) = match output.split_at_mut(1) {
            (first, rest) if !rest.is_empty() => (&mut first[0][..], &mut rest[0][..]),
            (first, _) => (&mut first[0][..], &mut [][..]),
        };

        let orbit_delta_deg = (self.orbit_speed_hz * 360.0) / self.sample_rate;
        let smooth_factor = 0.02; // Smooth parameter transition

        // One-pole lowpass alpha for ~800 Hz head shadow
        let omega_c = 2.0 * PI * 800.0 / self.sample_rate;
        let lp_alpha = omega_c / (1.0 + omega_c);

        for i in 0..frames {
            // Update 8D orbit rotation if active
            if self.orbit_speed_hz != 0.0 {
                self.target_azimuth_deg += orbit_delta_deg;
                if self.target_azimuth_deg >= 360.0 {
                    self.target_azimuth_deg -= 360.0;
                    self.azimuth_deg -= 360.0;
                } else if self.target_azimuth_deg < 0.0 {
                    self.target_azimuth_deg += 360.0;
                    self.azimuth_deg += 360.0;
                }
            }

            // Smooth azimuth angle
            self.azimuth_deg += (self.target_azimuth_deg - self.azimuth_deg) * smooth_factor;

            let az_rad = self.azimuth_deg * PI / 180.0;
            let sin_theta = az_rad.sin();

            // Target ITD in samples
            let (target_dl_sec, target_dr_sec) = Self::compute_itd(az_rad, self.head_radius_m);
            let target_dl = target_dl_sec * self.sample_rate;
            let target_dr = target_dr_sec * self.sample_rate;

            // Smooth delay values
            self.delay_samples_l += (target_dl - self.delay_samples_l) * smooth_factor;
            self.delay_samples_r += (target_dr - self.delay_samples_r) * smooth_factor;

            // Get mono or mixed input
            let x_in = if !in_left.is_empty() && !in_right.is_empty() {
                0.5 * (in_left[i] + in_right[i])
            } else if !in_left.is_empty() {
                in_left[i]
            } else {
                0.0
            };

            // Write to circular delay buffer
            self.delay_line_l[self.write_idx] = x_in;
            self.delay_line_r[self.write_idx] = x_in;

            // Fractional delay readout with linear interpolation
            let read_fractional = |buf: &[f32], d: f32, w: usize| -> f32 {
                let d_int = (d.floor() as usize).min(MAX_DELAY_SAMPLES - 2);
                let d_frac = d - d.floor();
                let idx0 = (w + MAX_DELAY_SAMPLES - d_int) % MAX_DELAY_SAMPLES;
                let idx1 = (w + MAX_DELAY_SAMPLES - d_int - 1) % MAX_DELAY_SAMPLES;
                (1.0 - d_frac) * buf[idx0] + d_frac * buf[idx1]
            };

            let delayed_l =
                read_fractional(&self.delay_line_l, self.delay_samples_l, self.write_idx);
            let delayed_r =
                read_fractional(&self.delay_line_r, self.delay_samples_r, self.write_idx);

            // Head shadow ILD modeling
            // Update internal 1-pole lowpass filters
            self.lowpass_state_l += lp_alpha * (delayed_l - self.lowpass_state_l);
            self.lowpass_state_r += lp_alpha * (delayed_r - self.lowpass_state_r);

            // Flush denormals
            if self.lowpass_state_l.abs() < DENORMAL_THRESHOLD {
                self.lowpass_state_l = 0.0;
            }
            if self.lowpass_state_r.abs() < DENORMAL_THRESHOLD {
                self.lowpass_state_r = 0.0;
            }

            // Head shadow gains:
            // When sin_theta > 0 (source on right):
            // Right ear (ipsilateral): slight high frequency boost (up to +2 dB)
            // Left ear (contralateral): high shelf attenuation (down to -10 dB)
            let (gain_hi_l, gain_hi_r) = if sin_theta >= 0.0 {
                let shadow = sin_theta;
                let g_shadow = 1.0 - (0.68 * shadow); // 0.32 (-10 dB)
                let g_ipsi = 1.0 + (0.2 * shadow);
                (g_shadow, g_ipsi)
            } else {
                let shadow = -sin_theta;
                let g_shadow = 1.0 - (0.68 * shadow);
                let g_ipsi = 1.0 + (0.2 * shadow);
                (g_ipsi, g_shadow)
            };

            let y_l = gain_hi_l * delayed_l + (1.0 - gain_hi_l) * self.lowpass_state_l;
            let y_r = gain_hi_r * delayed_r + (1.0 - gain_hi_r) * self.lowpass_state_r;

            out_left[i] = y_l;
            if !out_right.is_empty() {
                out_right[i] = y_r;
            }

            self.write_idx = (self.write_idx + 1) % MAX_DELAY_SAMPLES;
        }
    }

    fn reset(&mut self) {
        self.delay_line_l.fill(0.0);
        self.delay_line_r.fill(0.0);
        self.write_idx = 0;
        self.lowpass_state_l = 0.0;
        self.lowpass_state_r = 0.0;
        self.azimuth_deg = self.target_azimuth_deg;

        let az_rad = self.azimuth_deg * PI / 180.0;
        let (dl, dr) = Self::compute_itd(az_rad, self.head_radius_m);
        self.delay_samples_l = dl * self.sample_rate;
        self.delay_samples_r = dr * self.sample_rate;
    }

    fn latency_samples(&self) -> usize {
        0
    }

    fn tail_samples(&self) -> usize {
        64
    }

    fn set_parameter(&mut self, index: usize, value: f32) {
        match index {
            0 => self.set_azimuth(value),
            1 => self.elevation_deg = value.clamp(-90.0, 90.0),
            2 => self.set_orbit_speed(value),
            3 => self.head_radius_m = value.clamp(0.05, 0.15),
            _ => {}
        }
    }

    fn get_parameter(&self, index: usize) -> f32 {
        match index {
            0 => self.target_azimuth_deg,
            1 => self.elevation_deg,
            2 => self.orbit_speed_hz,
            3 => self.head_radius_m,
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
    fn test_binaural_center_symmetry() {
        let mut panner = BinauralNode::new();
        panner.prepare(48000.0, 2);
        panner.set_azimuth(0.0);

        let input = vec![1.0; 256];
        let mut out_l = vec![0.0; 256];
        let mut out_r = vec![0.0; 256];

        panner.process(&[&input], &mut [&mut out_l, &mut out_r]);

        // At center (0 azimuth), left and right channels must be identical
        for i in 64..256 {
            assert!((out_l[i] - out_r[i]).abs() < 1e-4, "Mismatch at sample {i}");
        }
    }

    #[test]
    fn test_binaural_itd_and_ild_hard_right() {
        let mut panner = BinauralNode::new();
        panner.prepare(48000.0, 2);
        panner.set_azimuth(90.0);
        panner.azimuth_deg = 90.0;
        panner.delay_samples_l = 30.0;
        panner.delay_samples_r = 0.0;

        // Process 1 kHz sine
        let mut input = vec![0.0; 480];
        for (i, sample) in input.iter_mut().enumerate().take(480) {
            *sample = ((i as f32 * 2.0 * PI * 1000.0) / 48000.0).sin();
        }
        let mut out_l = vec![0.0; 480];
        let mut out_r = vec![0.0; 480];

        panner.process(&[&input], &mut [&mut out_l, &mut out_r]);

        // Compute RMS of steady-state
        let mut rms_l = 0.0f32;
        let mut rms_r = 0.0f32;
        for i in 200..480 {
            rms_l += out_l[i] * out_l[i];
            rms_r += out_r[i] * out_r[i];
        }
        rms_l = (rms_l / 280.0).sqrt();
        rms_r = (rms_r / 280.0).sqrt();

        let ild_db = 20.0 * (rms_r / rms_l).log10();
        assert!(
            ild_db > 3.0,
            "Expected ILD > 3.0 dB at +90 deg, got {ild_db:.2} dB"
        );
    }
}
