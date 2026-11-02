//! Parameter smoothing and automation evaluation.

/// Parameter interpolation curve mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmoothingMode {
    /// Linear ramp from current to target value.
    Linear,
    /// Exponential curve (best for frequencies and decibels).
    Exponential,
    /// Smooth S-curve (cubic Hermite / smoothstep).
    SCurve,
}

/// A smoothed audio parameter supporting sample-accurate automation ramps.
#[derive(Debug, Clone)]
pub struct SmoothedParameter {
    current_value: f32,
    target_value: f32,
    start_value: f32,
    ramp_frames: usize,
    frame_index: usize,
    mode: SmoothingMode,
    sample_rate: f32,
}

impl SmoothedParameter {
    /// Creates a new parameter with an initial value.
    pub fn new(initial_value: f32, sample_rate: f32) -> Self {
        Self {
            current_value: initial_value,
            target_value: initial_value,
            start_value: initial_value,
            ramp_frames: 0,
            frame_index: 0,
            mode: SmoothingMode::Linear,
            sample_rate,
        }
    }

    /// Returns the current parameter value.
    pub fn current(&self) -> f32 {
        self.current_value
    }

    /// Returns the target parameter value.
    pub fn target(&self) -> f32 {
        self.target_value
    }

    /// Sets the value immediately without smoothing.
    pub fn set_immediate(&mut self, value: f32) {
        self.current_value = value;
        self.target_value = value;
        self.start_value = value;
        self.ramp_frames = 0;
        self.frame_index = 0;
    }

    /// Initiates a smooth ramp towards `target` over `duration_seconds`.
    pub fn set_target(&mut self, target: f32, duration_seconds: f32, mode: SmoothingMode) {
        self.start_value = self.current_value;
        self.target_value = target;
        self.mode = mode;
        self.ramp_frames = (duration_seconds * self.sample_rate).round().max(1.0) as usize;
        self.frame_index = 0;
    }

    /// Sets target value to reach exactly over a specified number of frames.
    pub fn set_target_frames(&mut self, target: f32, frames: usize, mode: SmoothingMode) {
        self.start_value = self.current_value;
        self.target_value = target;
        self.mode = mode;
        self.ramp_frames = frames.max(1);
        self.frame_index = 0;
    }

    /// Advances the parameter by one frame and returns the smoothed value.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> f32 {
        if self.frame_index >= self.ramp_frames {
            self.current_value = self.target_value;
            return self.current_value;
        }

        self.frame_index += 1;
        if self.frame_index >= self.ramp_frames {
            self.current_value = self.target_value;
            return self.current_value;
        }

        let t = self.frame_index as f32 / self.ramp_frames as f32;
        match self.mode {
            SmoothingMode::Linear => {
                self.current_value = self.start_value + (self.target_value - self.start_value) * t;
            }
            SmoothingMode::SCurve => {
                let smooth_t = t * t * (3.0 - 2.0 * t);
                self.current_value =
                    self.start_value + (self.target_value - self.start_value) * smooth_t;
            }
            SmoothingMode::Exponential => {
                if self.start_value > 0.0 && self.target_value > 0.0 {
                    let ratio = self.target_value / self.start_value;
                    self.current_value = self.start_value * ratio.powf(t);
                } else {
                    self.current_value =
                        self.start_value + (self.target_value - self.start_value) * t;
                }
            }
        }

        self.current_value
    }

    /// Fills a slice of frames with the smoothed parameter values.
    pub fn process_block(&mut self, output: &mut [f32]) {
        for sample in output.iter_mut() {
            *sample = self.next();
        }
    }
}

/// A point on an automation curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AutomationPoint {
    /// Frame offset in timeline.
    pub frame: u64,
    /// Normalized or unit value.
    pub value: f32,
    /// Curve tension / mode to next point (-1.0 to 1.0, 0.0 = linear).
    pub tension: f32,
}

/// Timeline automation lane for a parameter.
#[derive(Debug, Clone, Default)]
pub struct AutomationLane {
    points: Vec<AutomationPoint>,
}

impl AutomationLane {
    /// Creates a new empty automation lane.
    pub fn new() -> Self {
        Self { points: Vec::new() }
    }

    /// Adds an automation point (keeps points sorted by frame).
    pub fn add_point(&mut self, point: AutomationPoint) {
        let idx = self.points.partition_point(|p| p.frame < point.frame);
        if idx < self.points.len() && self.points[idx].frame == point.frame {
            self.points[idx] = point;
        } else {
            self.points.insert(idx, point);
        }
    }

    /// Evaluates parameter value at a given sample frame.
    pub fn evaluate(&self, frame: u64, default_val: f32) -> f32 {
        if self.points.is_empty() {
            return default_val;
        }
        if frame <= self.points[0].frame {
            return self.points[0].value;
        }
        let last = self.points.last().unwrap();
        if frame >= last.frame {
            return last.value;
        }

        let idx = self.points.partition_point(|p| p.frame <= frame);
        let p0 = self.points[idx - 1];
        let p1 = self.points[idx];

        let span = (p1.frame - p0.frame) as f32;
        if span <= 0.0 {
            return p0.value;
        }
        let t = (frame - p0.frame) as f32 / span;

        // Apply tension curve
        let curved_t = if (p0.tension).abs() < 1e-4 {
            t
        } else if p0.tension > 0.0 {
            t.powf(1.0 + p0.tension * 3.0)
        } else {
            1.0 - (1.0 - t).powf(1.0 - p0.tension * 3.0)
        };

        p0.value + (p1.value - p0.value) * curved_t
    }
}
