//! Flux DSP core processing library.
#![deny(missing_docs)]

/// Sample processing constants and primitives.
pub mod sample {
    /// Audio sample type used across Flux: 32-bit floating point.
    pub type Sample = f32;
}

#[cfg(test)]
mod tests {
    use super::sample::Sample;

    #[test]
    fn test_sample_type_precision() {
        let s: Sample = 0.0;
        assert_eq!(s, 0.0);
    }
}
