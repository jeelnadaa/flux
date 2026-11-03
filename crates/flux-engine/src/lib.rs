//! Flux engine graph and playback orchestration.
#![deny(missing_docs)]

pub mod capture;
pub mod codec;
pub mod decoder;
pub mod export;
pub mod graph;
pub mod render;
pub mod transport;

pub use capture::{ChannelMeter, ChunkHeader, ChunkedRecordWriter, InputLevelMeter, PreRollBuffer};

pub use codec::{AudioMetadata, ExportFormat, Mp3Encoder, OpusEncoder, VorbisEncoder};
pub use decoder::{
    decode_buffer, is_ffmpeg_available, open_decoder, AiffDecoder, AiffEncoding, AiffWriter,
    AudioBuffer, AudioDecoder, AudioFileInfo, BwfInfo, ContainerFormat, ContainerMetadata,
    CoverArt, DecodeError, FfmpegDecoder, GaplessInfo, OpusDecoder, SampleFormat, SeekEntry,
    SeekIndex, SymphoniaDecoder, WavDecoder, WavEncoding, WavWriter,
};
pub use flux_dsp;
pub use graph::{AudioGraph, Bus, EngineNode, NodeId, PanLaw, PluginDelayCompensation};
pub use render::{CommandQueue, EngineCommand, EngineHost, OfflineRenderer};
pub use transport::{Transport, TransportState};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_init() {
        let host = EngineHost::new(48000.0, 128);
        assert_eq!(host.transport.state(), TransportState::Stopped);
        let _ = symphonia::default::get_probe();
        let _ = symphonia::default::get_codecs();
    }
}
