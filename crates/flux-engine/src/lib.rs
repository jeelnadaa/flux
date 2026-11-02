//! Flux engine graph and playback orchestration.
#![deny(missing_docs)]

pub mod codec;
pub use codec::{AudioMetadata, ExportFormat, Mp3Encoder, OpusEncoder, VorbisEncoder};
pub use flux_dsp;

/// Engine host placeholder.
pub struct EngineHost;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_init() {
        let _ = EngineHost;
    }
}
