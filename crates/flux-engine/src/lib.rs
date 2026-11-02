//! Flux engine graph and playback orchestration.
#![deny(missing_docs)]

pub mod codec;
pub mod decoder;

pub use codec::{AudioMetadata, ExportFormat, Mp3Encoder, OpusEncoder, VorbisEncoder};
pub use decoder::{
    decode_buffer, open_decoder, AiffDecoder, AiffEncoding, AiffWriter, AudioBuffer, AudioDecoder,
    AudioFileInfo, BwfInfo, ContainerFormat, ContainerMetadata, DecodeError, SampleFormat,
    WavDecoder, WavEncoding, WavWriter,
};
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
