//! Crash-safe chunked recording stream with monotonic sequencing, CRC32 integrity, and instant crash recovery.

use crate::decoder::AudioBuffer;
use std::io::{Read, Seek, SeekFrom, Write};

const CHUNK_MAGIC_HEADER: [u8; 4] = *b"FCHK";
const CHUNK_MAGIC_FOOTER: [u8; 4] = *b"KCHF";

/// Header information for an audio recording chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkHeader {
    /// Monotonically increasing sequence number.
    pub sequence_number: u32,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Number of audio channels.
    pub channels: u16,
    /// Bits per sample (e.g. 32 for float).
    pub bits_per_sample: u16,
    /// Number of audio frames in this chunk.
    pub num_frames: u32,
    /// Global start frame offset in the recording.
    pub frame_offset: u64,
    /// CRC32 checksum of the payload bytes.
    pub crc32: u32,
}

/// Crash-safe chunked audio stream writer.
#[derive(Debug)]
pub struct ChunkedRecordWriter {
    sample_rate: u32,
    channels: u16,
    sequence_number: u32,
    total_frames_written: u64,
}

impl ChunkedRecordWriter {
    /// Creates a new `ChunkedRecordWriter`.
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            sample_rate,
            channels,
            sequence_number: 0,
            total_frames_written: 0,
        }
    }

    /// Writes an audio block as a self-contained, crash-safe chunk into `writer`.
    pub fn write_chunk<W: Write>(
        &mut self,
        writer: &mut W,
        channels: &[&[f32]],
    ) -> Result<usize, std::io::Error> {
        let n_ch = channels.len().min(self.channels as usize);
        if n_ch == 0 || channels[0].is_empty() {
            return Ok(0);
        }

        let num_frames = channels[0].len();
        let payload_samples = num_frames * n_ch;
        let mut payload_bytes = Vec::with_capacity(payload_samples * 4);

        // Interleave float32 samples
        for frame in 0..num_frames {
            for ch in channels.iter().take(n_ch) {
                let sample = ch[frame];
                payload_bytes.extend_from_slice(&sample.to_le_bytes());
            }
        }

        let crc = compute_crc32(&payload_bytes);

        // Write header
        writer.write_all(&CHUNK_MAGIC_HEADER)?;
        writer.write_all(&self.sequence_number.to_le_bytes())?;
        writer.write_all(&self.sample_rate.to_le_bytes())?;
        writer.write_all(&self.channels.to_le_bytes())?;
        writer.write_all(&32u16.to_le_bytes())?; // 32-bit float
        writer.write_all(&(num_frames as u32).to_le_bytes())?;
        writer.write_all(&self.total_frames_written.to_le_bytes())?;
        writer.write_all(&crc.to_le_bytes())?;

        // Write payload
        writer.write_all(&payload_bytes)?;

        // Write footer
        writer.write_all(&CHUNK_MAGIC_FOOTER)?;
        writer.flush()?;

        self.sequence_number += 1;
        self.total_frames_written += num_frames as u64;

        // Total chunk size: 4 + 4 + 4 + 2 + 2 + 4 + 8 + 4 + payload + 4 = 36 + payload
        Ok(36 + payload_bytes.len())
    }

    /// Recovers all completely written chunks from a stream that may have terminated abruptly.
    pub fn recover_from_stream<R: Read + Seek>(
        reader: &mut R,
    ) -> Result<AudioBuffer, std::io::Error> {
        reader.seek(SeekFrom::Start(0))?;

        let mut expected_seq = 0u32;
        let mut sample_rate = 48000u32;
        let mut channels_count = 0usize;
        let mut recovered_channels: Vec<Vec<f32>> = Vec::new();

        loop {
            let mut magic = [0u8; 4];
            match reader.read_exact(&mut magic) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            }

            if magic != CHUNK_MAGIC_HEADER {
                // Stopped at unwritten/corrupt boundary
                break;
            }

            let mut header_buf = [0u8; 28];
            if reader.read_exact(&mut header_buf).is_err() {
                break; // Incomplete chunk header, terminate recovery cleanly
            }

            let seq = u32::from_le_bytes(header_buf[0..4].try_into().unwrap());
            let s_rate = u32::from_le_bytes(header_buf[4..8].try_into().unwrap());
            let n_ch = u16::from_le_bytes(header_buf[8..10].try_into().unwrap()) as usize;
            let _bits = u16::from_le_bytes(header_buf[10..12].try_into().unwrap());
            let num_frames = u32::from_le_bytes(header_buf[12..16].try_into().unwrap()) as usize;
            let _offset = u64::from_le_bytes(header_buf[16..24].try_into().unwrap());
            let expected_crc = u32::from_le_bytes(header_buf[24..28].try_into().unwrap());

            if seq != expected_seq {
                break; // Discontinuous chunk sequence
            }

            let payload_size = num_frames * n_ch * 4;
            let mut payload = vec![0u8; payload_size];
            if reader.read_exact(&mut payload).is_err() {
                break; // Incomplete payload bytes due to mid-write crash
            }

            let mut footer = [0u8; 4];
            if reader.read_exact(&mut footer).is_err() || footer != CHUNK_MAGIC_FOOTER {
                break; // Missing or partial footer
            }

            let actual_crc = compute_crc32(&payload);
            if actual_crc != expected_crc {
                break; // Checksum mismatch
            }

            if recovered_channels.is_empty() {
                channels_count = n_ch;
                sample_rate = s_rate;
                recovered_channels = vec![Vec::new(); n_ch];
            }

            // De-interleave payload
            for frame in 0..num_frames {
                let base = frame * n_ch * 4;
                for (ch_idx, ch_buf) in recovered_channels.iter_mut().enumerate().take(n_ch) {
                    let b = &payload[base + ch_idx * 4..base + ch_idx * 4 + 4];
                    let val = f32::from_le_bytes(b.try_into().unwrap());
                    ch_buf.push(val);
                }
            }

            expected_seq += 1;
        }

        if recovered_channels.is_empty() {
            recovered_channels = vec![Vec::new(); channels_count.max(2)];
        }

        Ok(AudioBuffer {
            channels: recovered_channels,
            sample_rate,
        })
    }

    /// Converts a stream of chunked recordings into a bit-exact standard 32-bit float WAV file.
    pub fn convert_chunks_to_wav<R: Read + Seek, W: Write + Seek>(
        reader: &mut R,
        writer: &mut W,
    ) -> Result<u64, std::io::Error> {
        let audio = Self::recover_from_stream(reader)?;
        let num_frames = audio.num_frames();
        let sample_rate = audio.sample_rate;

        let channel_refs: Vec<&[f32]> = audio.channels.iter().map(|c| c.as_slice()).collect();
        let bytes = crate::decoder::WavWriter::encode(
            &channel_refs,
            sample_rate,
            crate::decoder::WavEncoding::Float32,
            None,
            false,
        )
        .map_err(std::io::Error::other)?;

        writer.write_all(&bytes)?;

        Ok(num_frames as u64)
    }
}

/// Standard IEEE 802.3 CRC32 calculation.
fn compute_crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}
