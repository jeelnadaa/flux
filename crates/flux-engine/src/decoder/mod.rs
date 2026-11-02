//! Audio decoders and metadata parsing.

use std::fmt;
use std::io::{Read, Seek};

pub mod aiff;
pub mod wav;

pub use aiff::{AiffDecoder, AiffEncoding, AiffWriter};
pub use wav::{WavDecoder, WavEncoding, WavWriter};

/// Audio container format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerFormat {
    /// Standard Microsoft RIFF/WAVE.
    Wav,
    /// RF64 (64-bit RIFF/WAVE for files > 4 GB).
    Rf64,
    /// Broadcast Wave Format (BWF with `bext` chunk).
    Bwf,
    /// Apple Audio Interchange File Format.
    Aiff,
    /// Apple AIFF Compressed (or float/little-endian variant).
    Aifc,
}

/// Sample encoding format in the container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFormat {
    /// 8-bit unsigned PCM (standard WAV).
    PcmU8,
    /// 8-bit signed PCM (standard AIFF).
    PcmS8,
    /// 16-bit signed PCM little-endian.
    PcmS16Le,
    /// 16-bit signed PCM big-endian.
    PcmS16Be,
    /// 24-bit signed PCM packed little-endian.
    PcmS24Le,
    /// 24-bit signed PCM packed big-endian.
    PcmS24Be,
    /// 32-bit signed PCM little-endian.
    PcmS32Le,
    /// 32-bit signed PCM big-endian.
    PcmS32Be,
    /// 32-bit IEEE 754 floating-point little-endian.
    Float32Le,
    /// 32-bit IEEE 754 floating-point big-endian.
    Float32Be,
    /// 64-bit IEEE 754 floating-point little-endian.
    Float64Le,
}

impl SampleFormat {
    /// Returns the number of bytes required per sample in this format.
    pub fn bytes_per_sample(self) -> usize {
        match self {
            SampleFormat::PcmU8 | SampleFormat::PcmS8 => 1,
            SampleFormat::PcmS16Le | SampleFormat::PcmS16Be => 2,
            SampleFormat::PcmS24Le | SampleFormat::PcmS24Be => 3,
            SampleFormat::PcmS32Le
            | SampleFormat::PcmS32Be
            | SampleFormat::Float32Le
            | SampleFormat::Float32Be => 4,
            SampleFormat::Float64Le => 8,
        }
    }
}

/// Broadcast Audio Extension (BWF `bext` chunk) metadata.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BwfInfo {
    /// Description of the sound sequence (up to 256 chars).
    pub description: String,
    /// Name of the originator (up to 32 chars).
    pub originator: String,
    /// Reference of the originator (up to 32 chars).
    pub originator_reference: String,
    /// Origination date (YYYY-MM-DD).
    pub origination_date: String,
    /// Origination time (HH:MM:SS).
    pub origination_time: String,
    /// Time reference: sample count from midnight.
    pub time_reference: u64,
    /// BWF version.
    pub version: u16,
    /// Unique Material Identifier (UMID, 64 bytes).
    pub umid: Option<[u8; 64]>,
    /// Integrated loudness value in hundredths of LUFS (-0.01 LUFS).
    pub loudness_value: Option<i16>,
    /// Loudness range in hundredths of LU (0.01 LU).
    pub loudness_range: Option<i16>,
    /// Maximum true peak level in hundredths of dBTP (0.01 dBTP).
    pub max_true_peak_level: Option<i16>,
    /// Coding history string.
    pub coding_history: String,
}

/// Metadata extracted from container tags (INFO, ID3, AIFF tags).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContainerMetadata {
    /// Track title.
    pub title: Option<String>,
    /// Artist / Performer.
    pub artist: Option<String>,
    /// Album title.
    pub album: Option<String>,
    /// Date / Year.
    pub date: Option<String>,
    /// Genre.
    pub genre: Option<String>,
    /// Comment / Annotation.
    pub comment: Option<String>,
    /// Copyright notice.
    pub copyright: Option<String>,
    /// Software / encoder used.
    pub software: Option<String>,
}

/// Information about an opened audio file.
#[derive(Debug, Clone)]
pub struct AudioFileInfo {
    /// Container format.
    pub container_format: ContainerFormat,
    /// Sample encoding format.
    pub sample_format: SampleFormat,
    /// Number of audio channels.
    pub channels: u16,
    /// Sampling rate in Hz.
    pub sample_rate: u32,
    /// Bit depth per sample.
    pub bits_per_sample: u16,
    /// Total audio frames in the stream.
    pub total_frames: u64,
    /// Total duration in seconds.
    pub duration_seconds: f64,
    /// Optional channel mask (e.g. from WAVE_FORMAT_EXTENSIBLE).
    pub channel_mask: Option<u32>,
    /// Container metadata tags.
    pub metadata: ContainerMetadata,
    /// Optional BWF broadcast metadata.
    pub bwf_info: Option<BwfInfo>,
}

/// Multi-channel planar 32-bit floating-point audio buffer.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioBuffer {
    /// Channels of audio data: `channels[ch][frame]`.
    pub channels: Vec<Vec<f32>>,
    /// Sampling rate in Hz.
    pub sample_rate: u32,
}

impl AudioBuffer {
    /// Creates a new silent buffer with `num_channels` channels and `num_frames` frames.
    pub fn new(num_channels: usize, num_frames: usize, sample_rate: u32) -> Self {
        Self {
            channels: vec![vec![0.0; num_frames]; num_channels],
            sample_rate,
        }
    }

    /// Creates an `AudioBuffer` from de-interleaved channel vectors.
    pub fn from_channels(channels: Vec<Vec<f32>>, sample_rate: u32) -> Self {
        Self {
            channels,
            sample_rate,
        }
    }

    /// Creates an `AudioBuffer` from interleaved samples.
    pub fn from_interleaved(interleaved: &[f32], num_channels: usize, sample_rate: u32) -> Self {
        if num_channels == 0 || interleaved.is_empty() {
            return Self::new(num_channels, 0, sample_rate);
        }
        let num_frames = interleaved.len() / num_channels;
        let mut channels = vec![Vec::with_capacity(num_frames); num_channels];
        for frame in 0..num_frames {
            let base = frame * num_channels;
            for ch in 0..num_channels {
                channels[ch].push(interleaved[base + ch]);
            }
        }
        Self {
            channels,
            sample_rate,
        }
    }

    /// Number of audio channels.
    pub fn num_channels(&self) -> usize {
        self.channels.len()
    }

    /// Number of frames per channel.
    pub fn num_frames(&self) -> usize {
        self.channels.first().map_or(0, |c| c.len())
    }

    /// Returns a slice of samples for the specified channel index.
    pub fn channel(&self, ch: usize) -> &[f32] {
        &self.channels[ch]
    }

    /// Returns a mutable slice of samples for the specified channel index.
    pub fn channel_mut(&mut self, ch: usize) -> &mut [f32] {
        &mut self.channels[ch]
    }

    /// Converts planar channels into an interleaved slice.
    pub fn to_interleaved(&self) -> Vec<f32> {
        let n_ch = self.num_channels();
        let n_frames = self.num_frames();
        let mut out = Vec::with_capacity(n_ch * n_frames);
        for frame in 0..n_frames {
            for ch in 0..n_ch {
                out.push(self.channels[ch][frame]);
            }
        }
        out
    }
}

/// Errors that can occur during audio decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// Unexpected end of file / stream.
    UnexpectedEof(String),
    /// Unrecognized file header or magic signature.
    InvalidHeader(String),
    /// Unsupported container or audio format.
    UnsupportedFormat(String),
    /// Corrupt chunk or malformed payload.
    CorruptData(String),
    /// Underlying I/O error.
    IoError(String),
    /// Seek out of bounds.
    SeekOutOfBounds {
        /// Requested frame index.
        requested: u64,
        /// Maximum total frames available.
        total: u64,
    },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::UnexpectedEof(s) => write!(f, "Unexpected EOF: {s}"),
            DecodeError::InvalidHeader(s) => write!(f, "Invalid header: {s}"),
            DecodeError::UnsupportedFormat(s) => write!(f, "Unsupported format: {s}"),
            DecodeError::CorruptData(s) => write!(f, "Corrupt data: {s}"),
            DecodeError::IoError(s) => write!(f, "I/O error: {s}"),
            DecodeError::SeekOutOfBounds { requested, total } => {
                write!(f, "Seek frame {requested} out of bounds (total {total})")
            }
        }
    }
}

impl std::error::Error for DecodeError {}

impl From<std::io::Error> for DecodeError {
    fn from(err: std::io::Error) -> Self {
        DecodeError::IoError(err.to_string())
    }
}

/// Trait implemented by streaming audio decoders.
pub trait AudioDecoder {
    /// Returns audio file metadata and format properties.
    fn info(&self) -> &AudioFileInfo;

    /// Returns the current read frame position.
    fn current_frame(&self) -> u64;

    /// Seeks the stream to the specified frame index.
    fn seek_frame(&mut self, frame: u64) -> Result<(), DecodeError>;

    /// Decodes the next chunk of up to `max_frames` audio frames.
    /// Returns `Ok(None)` when the end of stream is reached.
    fn decode_chunk(&mut self, max_frames: usize) -> Result<Option<AudioBuffer>, DecodeError>;

    /// Decodes all remaining audio frames in the stream into an `AudioBuffer`.
    fn decode_all(&mut self) -> Result<AudioBuffer, DecodeError> {
        let total = self.info().total_frames;
        let channels = self.info().channels as usize;
        let sample_rate = self.info().sample_rate;
        let mut out_channels =
            vec![Vec::with_capacity((total - self.current_frame()) as usize); channels];

        while let Some(chunk) = self.decode_chunk(8192)? {
            for (ch_idx, ch_vec) in out_channels.iter_mut().enumerate() {
                ch_vec.extend_from_slice(chunk.channel(ch_idx));
            }
        }

        Ok(AudioBuffer {
            channels: out_channels,
            sample_rate,
        })
    }
}

/// Opens an audio decoder from any seekable byte stream, auto-detecting the container.
pub fn open_decoder<'a, R: Read + Seek + 'a>(
    mut reader: R,
) -> Result<Box<dyn AudioDecoder + 'a>, DecodeError> {
    let mut magic = [0u8; 12];
    reader
        .read_exact(&mut magic)
        .map_err(|e| DecodeError::UnexpectedEof(e.to_string()))?;
    reader.seek(std::io::SeekFrom::Start(0))?;

    if &magic[0..4] == b"RIFF" || &magic[0..4] == b"RF64" || &magic[0..4] == b"BW64" {
        let decoder = WavDecoder::new(reader)?;
        Ok(Box::new(decoder))
    } else if &magic[0..4] == b"FORM" {
        let decoder = AiffDecoder::new(reader)?;
        Ok(Box::new(decoder))
    } else {
        Err(DecodeError::InvalidHeader(format!(
            "Unrecognized magic signature: {:?}",
            &magic[0..4]
        )))
    }
}

/// Decodes an entire in-memory audio byte slice into an `AudioBuffer`.
pub fn decode_buffer(data: &[u8]) -> Result<AudioBuffer, DecodeError> {
    let cursor = std::io::Cursor::new(data);
    let mut decoder = open_decoder(cursor)?;
    decoder.decode_all()
}
