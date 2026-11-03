//! Phase correlation, stereo width ratio, mid/side balance, and goniometer vectorscope data.

/// Goniometer 2D vectorscope coordinate point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GoniometerPoint {
    /// Horizontal axis: Side component `(L - R) / sqrt(2)`.
    pub x: f32,
    /// Vertical axis: Mid component `(L + R) / sqrt(2)`.
    pub y: f32,
}

/// Stereo field spatial analysis metrics.
#[derive(Debug, Clone, PartialEq)]
pub struct StereoAnalysisResult {
    /// Phase correlation in [-1.0, +1.0] (+1.0 mono in-phase, -1.0 anti-phase, 0.0 uncorrelated).
    pub phase_correlation: f32,
    /// Stereo width ratio RMS(Side) / RMS(Mid) (0.0 = mono, 1.0 = standard stereo balance).
    pub stereo_width: f32,
    /// Mid channel RMS amplitude.
    pub mid_rms: f32,
    /// Side channel RMS amplitude.
    pub side_rms: f32,
    /// Decimated goniometer vectorscope points for visual display.
    pub goniometer_points: Vec<GoniometerPoint>,
}

/// Spatial stereo analyzer.
pub struct StereoAnalyzer;

impl StereoAnalyzer {
    /// Analyzes a stereo pair of channels (`left` and `right`).
    pub fn analyze(
        left: &[f32],
        right: &[f32],
        max_goniometer_points: usize,
    ) -> StereoAnalysisResult {
        let n = left.len().min(right.len());
        if n == 0 {
            return StereoAnalysisResult {
                phase_correlation: 1.0,
                stereo_width: 0.0,
                mid_rms: 0.0,
                side_rms: 0.0,
                goniometer_points: Vec::new(),
            };
        }

        let mut sum_l_sq = 0.0f64;
        let mut sum_r_sq = 0.0f64;
        let mut sum_lr = 0.0f64;
        let mut sum_mid_sq = 0.0f64;
        let mut sum_side_sq = 0.0f64;

        let inv_sqrt2 = 1.0f32 / std::f32::consts::SQRT_2;

        for i in 0..n {
            let l = left[i] as f64;
            let r = right[i] as f64;

            sum_l_sq += l * l;
            sum_r_sq += r * r;
            sum_lr += l * r;

            let m = 0.5 * (l + r);
            let s = 0.5 * (l - r);
            sum_mid_sq += m * m;
            sum_side_sq += s * s;
        }

        // Phase correlation
        let denom = (sum_l_sq * sum_r_sq).sqrt();
        let phase_correlation = if denom > 1e-12 {
            (sum_lr / denom).clamp(-1.0, 1.0) as f32
        } else {
            1.0 // Silence default
        };

        // Mid and Side RMS
        let mid_rms = (sum_mid_sq / (n as f64)).sqrt() as f32;
        let side_rms = (sum_side_sq / (n as f64)).sqrt() as f32;

        // Stereo width
        let stereo_width = if mid_rms > 1e-7 {
            side_rms / mid_rms
        } else if side_rms > 1e-7 {
            2.0 // Pure side signal (out of phase)
        } else {
            0.0 // Silence
        };

        // Goniometer point cloud (decimated if necessary)
        let target_pts = max_goniometer_points.max(16);
        let step = (n / target_pts).max(1);
        let mut goniometer_points = Vec::with_capacity(n / step);

        for i in (0..n).step_by(step) {
            if goniometer_points.len() >= target_pts {
                break;
            }
            let l = left[i];
            let r = right[i];
            let x = (l - r) * inv_sqrt2;
            let y = (l + r) * inv_sqrt2;
            goniometer_points.push(GoniometerPoint { x, y });
        }

        StereoAnalysisResult {
            phase_correlation,
            stereo_width,
            mid_rms,
            side_rms,
            goniometer_points,
        }
    }
}
