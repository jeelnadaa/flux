//! Flux testkit: signal generators, analyzers, and invariant validation harness.
#![deny(missing_docs)]

/// Signal generation primitives.
pub mod signal {
    /// Generates a test impulse of specified length.
    pub fn impulse(len: usize) -> Vec<f32> {
        let mut out = vec![0.0; len];
        if len > 0 {
            out[0] = 1.0;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::signal::*;

    #[test]
    fn test_impulse() {
        let imp = impulse(4);
        assert_eq!(imp, vec![1.0, 0.0, 0.0, 0.0]);
    }
}
