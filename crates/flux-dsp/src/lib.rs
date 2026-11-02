//! Flux DSP core processing library.
#![deny(missing_docs)]

pub mod binaural;
pub mod biquad;
pub mod node;
pub mod sample;

pub use binaural::BinauralNode;
pub use biquad::{BiquadChain, BiquadCoeffs, BiquadState, BiquadType};
pub use node::{DspNode, GainNode, PassthroughNode};
pub use sample::Sample;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_type_precision() {
        let s: Sample = 0.0;
        assert_eq!(s, 0.0);
    }

    #[test]
    fn test_gain_node_basic() {
        let mut gain = GainNode::new(0.5);
        gain.prepare(48000.0, 1);
        let in_buf = vec![1.0; 128];
        let mut out_buf = vec![0.0; 128];

        gain.process(&[&in_buf], &mut [&mut out_buf]);
        // With smoothing from 0.5 to 0.5, gain stays ~0.5
        assert!((out_buf[127] - 0.5).abs() < 0.01);
    }
}
