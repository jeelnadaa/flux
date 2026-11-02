//! Apple AIFF and AIFF-C (AIFC) decoder and writer.

use super::{
    AudioBuffer, AudioDecoder, AudioFileInfo, ContainerFormat, ContainerMetadata, DecodeError,
    SampleFormat,
};
use std::io::{Read, Seek, SeekFrom};

/// Streaming decoder for Apple AIFF and AIFF-C files.
pub struct AiffDecoder<R: Read + Seek> {
    reader: R,
    info: AudioFileInfo,
    data_start: u64,
    _data_bytes: u64,
    current_frame: u64,
    block_align: usize,
    _is_little_endian: bool,
}

impl<R: Read + Seek> AiffDecoder<R> {
    /// Opens and parses an AIFF or AIFC file from `reader`.
    pub fn new(mut reader: R) -> Result<Self, DecodeError> {
        let mut header = [0u8; 12];
        reader
            .read_exact(&mut header)
            .map_err(|e| DecodeError::UnexpectedEof(e.to_string()))?;

        if &header[0..4] != b"FORM" {
            return Err(DecodeError::InvalidHeader(format!(
                "Expected FORM header, found {:?}",
                &header[0..4]
            )));
        }

        let form_type = &header[8..12];
        let is_aiff = form_type == b"AIFF";
        let is_aifc = form_type == b"AIFC";

        if !is_aiff && !is_aifc {
            return Err(DecodeError::InvalidHeader(format!(
                "Expected AIFF or AIFC form type, found {:?}",
                form_type
            )));
        }

        let mut comm_found = false;
        let mut channels = 0u16;
        let mut num_sample_frames = 0u64;
        let mut sample_size = 0u16;
        let mut sample_rate = 0u32;
        let mut compression_type = [0u8; 4];

        let mut data_start: Option<u64> = None;
        let mut data_bytes: Option<u64> = None;
        let mut metadata = ContainerMetadata::default();

        let mut cur_pos = 12u64;

        loop {
            let mut chunk_hdr = [0u8; 8];
            match reader.read_exact(&mut chunk_hdr) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(DecodeError::IoError(e.to_string())),
            }
            cur_pos += 8;

            let chunk_id = &chunk_hdr[0..4];
            let chunk_size =
                u32::from_be_bytes([chunk_hdr[4], chunk_hdr[5], chunk_hdr[6], chunk_hdr[7]]) as u64;

            match chunk_id {
                b"COMM" => {
                    if chunk_size < 18 {
                        return Err(DecodeError::CorruptData("COMM chunk too small".to_string()));
                    }
                    let mut comm_buf = vec![0u8; chunk_size as usize];
                    reader
                        .read_exact(&mut comm_buf)
                        .map_err(|e| DecodeError::UnexpectedEof(e.to_string()))?;
                    cur_pos += chunk_size;

                    channels = u16::from_be_bytes([comm_buf[0], comm_buf[1]]);
                    num_sample_frames =
                        u32::from_be_bytes([comm_buf[2], comm_buf[3], comm_buf[4], comm_buf[5]])
                            as u64;
                    sample_size = u16::from_be_bytes([comm_buf[6], comm_buf[7]]);
                    let rate_extended = &comm_buf[8..18];
                    sample_rate = decode_extended_80(rate_extended).round() as u32;

                    if is_aifc {
                        if chunk_size >= 22 {
                            compression_type.copy_from_slice(&comm_buf[18..22]);
                        } else {
                            compression_type = *b"NONE";
                        }
                    } else {
                        compression_type = *b"NONE";
                    }

                    comm_found = true;
                }
                b"SSND" => {
                    if chunk_size < 8 {
                        return Err(DecodeError::CorruptData("SSND chunk too small".to_string()));
                    }
                    let mut ssnd_hdr = [0u8; 8];
                    reader
                        .read_exact(&mut ssnd_hdr)
                        .map_err(|e| DecodeError::UnexpectedEof(e.to_string()))?;
                    cur_pos += 8;

                    let offset =
                        u32::from_be_bytes([ssnd_hdr[0], ssnd_hdr[1], ssnd_hdr[2], ssnd_hdr[3]])
                            as u64;
                    // block_size is in ssnd_hdr[4..8], usually 0

                    let sound_bytes = chunk_size - 8 - offset;
                    data_start = Some(cur_pos + offset);
                    data_bytes = Some(sound_bytes);

                    // Skip the sound data chunk
                    reader.seek(SeekFrom::Current((chunk_size - 8) as i64))?;
                    cur_pos += chunk_size - 8;
                }
                b"NAME" => {
                    let mut buf = vec![0u8; chunk_size as usize];
                    reader
                        .read_exact(&mut buf)
                        .map_err(|e| DecodeError::UnexpectedEof(e.to_string()))?;
                    cur_pos += chunk_size;
                    metadata.title = Some(String::from_utf8_lossy(&buf).trim().to_string());
                }
                b"AUTH" => {
                    let mut buf = vec![0u8; chunk_size as usize];
                    reader
                        .read_exact(&mut buf)
                        .map_err(|e| DecodeError::UnexpectedEof(e.to_string()))?;
                    cur_pos += chunk_size;
                    metadata.artist = Some(String::from_utf8_lossy(&buf).trim().to_string());
                }
                b"(c) " => {
                    let mut buf = vec![0u8; chunk_size as usize];
                    reader
                        .read_exact(&mut buf)
                        .map_err(|e| DecodeError::UnexpectedEof(e.to_string()))?;
                    cur_pos += chunk_size;
                    metadata.copyright = Some(String::from_utf8_lossy(&buf).trim().to_string());
                }
                b"ANNO" => {
                    let mut buf = vec![0u8; chunk_size as usize];
                    reader
                        .read_exact(&mut buf)
                        .map_err(|e| DecodeError::UnexpectedEof(e.to_string()))?;
                    cur_pos += chunk_size;
                    metadata.comment = Some(String::from_utf8_lossy(&buf).trim().to_string());
                }
                _ => {
                    reader.seek(SeekFrom::Current(chunk_size as i64))?;
                    cur_pos += chunk_size;
                }
            }

            // AIFF chunks are padded to 2-byte boundaries
            if !chunk_size.is_multiple_of(2) {
                let mut pad = [0u8; 1];
                let _ = reader.read_exact(&mut pad);
                cur_pos += 1;
            }
        }

        if !comm_found {
            return Err(DecodeError::CorruptData(
                "Missing COMM chunk in AIFF".to_string(),
            ));
        }

        let data_start = data_start
            .ok_or_else(|| DecodeError::CorruptData("Missing SSND chunk in AIFF".to_string()))?;
        let data_bytes = data_bytes.unwrap_or(0);

        let is_sowt = &compression_type == b"sowt";
        let is_float = &compression_type == b"fl32" || &compression_type == b"FL32";

        let sample_format = if is_float {
            if sample_size == 32 {
                SampleFormat::Float32Be
            } else {
                return Err(DecodeError::UnsupportedFormat(format!(
                    "Unsupported AIFF float bit depth: {sample_size}"
                )));
            }
        } else if is_sowt {
            match sample_size {
                8 => SampleFormat::PcmS8,
                16 => SampleFormat::PcmS16Le,
                24 => SampleFormat::PcmS24Le,
                32 => SampleFormat::PcmS32Le,
                other => {
                    return Err(DecodeError::UnsupportedFormat(format!(
                        "Unsupported AIFF sowt bit depth: {other}"
                    )))
                }
            }
        } else {
            // standard big-endian PCM (NONE / twos / raw)
            match sample_size {
                8 => {
                    if &compression_type == b"raw " {
                        SampleFormat::PcmU8
                    } else {
                        SampleFormat::PcmS8
                    }
                }
                16 => SampleFormat::PcmS16Be,
                24 => SampleFormat::PcmS24Be,
                32 => SampleFormat::PcmS32Be,
                other => {
                    return Err(DecodeError::UnsupportedFormat(format!(
                        "Unsupported AIFF PCM bit depth: {other}"
                    )))
                }
            }
        };

        let block_align = (channels as usize) * sample_format.bytes_per_sample();
        let calculated_frames = if block_align > 0 {
            data_bytes / (block_align as u64)
        } else {
            0
        };

        let total_frames = if num_sample_frames > 0 {
            num_sample_frames
        } else {
            calculated_frames
        };

        let duration_seconds = if sample_rate > 0 {
            total_frames as f64 / sample_rate as f64
        } else {
            0.0
        };

        let info = AudioFileInfo {
            container_format: if is_aifc {
                ContainerFormat::Aifc
            } else {
                ContainerFormat::Aiff
            },
            sample_format,
            channels,
            sample_rate,
            bits_per_sample: sample_size,
            total_frames,
            duration_seconds,
            channel_mask: None,
            metadata,
            bwf_info: None,
        };

        reader.seek(SeekFrom::Start(data_start))?;

        Ok(Self {
            reader,
            info,
            data_start,
            _data_bytes: data_bytes,
            current_frame: 0,
            block_align,
            _is_little_endian: is_sowt,
        })
    }
}

impl<R: Read + Seek> AudioDecoder for AiffDecoder<R> {
    fn info(&self) -> &AudioFileInfo {
        &self.info
    }

    fn current_frame(&self) -> u64 {
        self.current_frame
    }

    fn seek_frame(&mut self, frame: u64) -> Result<(), DecodeError> {
        if frame > self.info.total_frames {
            return Err(DecodeError::SeekOutOfBounds {
                requested: frame,
                total: self.info.total_frames,
            });
        }
        let target_pos = self.data_start + frame * (self.block_align as u64);
        self.reader.seek(SeekFrom::Start(target_pos))?;
        self.current_frame = frame;
        Ok(())
    }

    fn decode_chunk(&mut self, max_frames: usize) -> Result<Option<AudioBuffer>, DecodeError> {
        if self.current_frame >= self.info.total_frames {
            return Ok(None);
        }

        let remaining = (self.info.total_frames - self.current_frame) as usize;
        let frames_to_read = max_frames.min(remaining);
        if frames_to_read == 0 {
            return Ok(None);
        }

        let num_channels = self.info.channels as usize;
        let total_bytes = frames_to_read * self.block_align;

        let mut byte_buf = vec![0u8; total_bytes];
        self.reader
            .read_exact(&mut byte_buf)
            .map_err(|e| DecodeError::UnexpectedEof(e.to_string()))?;
        self.current_frame += frames_to_read as u64;

        let mut channel_data = vec![Vec::with_capacity(frames_to_read); num_channels];

        match self.info.sample_format {
            SampleFormat::PcmS8 => {
                let mut idx = 0;
                for _ in 0..frames_to_read {
                    for ch_vec in channel_data.iter_mut().take(num_channels) {
                        let sample_i8 = byte_buf[idx] as i8;
                        idx += 1;
                        let sample_f32 = sample_i8 as f32 / 128.0;
                        ch_vec.push(sample_f32);
                    }
                }
            }
            SampleFormat::PcmU8 => {
                let mut idx = 0;
                for _ in 0..frames_to_read {
                    for ch_vec in channel_data.iter_mut().take(num_channels) {
                        let sample_u8 = byte_buf[idx];
                        idx += 1;
                        let sample_f32 = (sample_u8 as f32 - 128.0) / 128.0;
                        ch_vec.push(sample_f32);
                    }
                }
            }
            SampleFormat::PcmS16Be => {
                let mut idx = 0;
                for _ in 0..frames_to_read {
                    for ch_vec in channel_data.iter_mut().take(num_channels) {
                        let sample_i16 = i16::from_be_bytes([byte_buf[idx], byte_buf[idx + 1]]);
                        idx += 2;
                        let sample_f32 = sample_i16 as f32 / 32768.0;
                        ch_vec.push(sample_f32);
                    }
                }
            }
            SampleFormat::PcmS16Le => {
                let mut idx = 0;
                for _ in 0..frames_to_read {
                    for ch_vec in channel_data.iter_mut().take(num_channels) {
                        let sample_i16 = i16::from_le_bytes([byte_buf[idx], byte_buf[idx + 1]]);
                        idx += 2;
                        let sample_f32 = sample_i16 as f32 / 32768.0;
                        ch_vec.push(sample_f32);
                    }
                }
            }
            SampleFormat::PcmS24Be => {
                let mut idx = 0;
                for _ in 0..frames_to_read {
                    for ch_vec in channel_data.iter_mut().take(num_channels) {
                        let b0 = byte_buf[idx];
                        let b1 = byte_buf[idx + 1];
                        let b2 = byte_buf[idx + 2];
                        idx += 3;
                        let sign_ext = if b0 & 0x80 != 0 { 0xFF } else { 0x00 };
                        let sample_i32 = i32::from_be_bytes([sign_ext, b0, b1, b2]);
                        let sample_f32 = sample_i32 as f32 / 8388608.0;
                        ch_vec.push(sample_f32);
                    }
                }
            }
            SampleFormat::PcmS24Le => {
                let mut idx = 0;
                for _ in 0..frames_to_read {
                    for ch_vec in channel_data.iter_mut().take(num_channels) {
                        let b0 = byte_buf[idx];
                        let b1 = byte_buf[idx + 1];
                        let b2 = byte_buf[idx + 2];
                        idx += 3;
                        let sign_ext = if b2 & 0x80 != 0 { 0xFF } else { 0x00 };
                        let sample_i32 = i32::from_le_bytes([b0, b1, b2, sign_ext]);
                        let sample_f32 = sample_i32 as f32 / 8388608.0;
                        ch_vec.push(sample_f32);
                    }
                }
            }
            SampleFormat::PcmS32Be => {
                let mut idx = 0;
                for _ in 0..frames_to_read {
                    for ch_vec in channel_data.iter_mut().take(num_channels) {
                        let sample_i32 = i32::from_be_bytes([
                            byte_buf[idx],
                            byte_buf[idx + 1],
                            byte_buf[idx + 2],
                            byte_buf[idx + 3],
                        ]);
                        idx += 4;
                        let sample_f32 = (sample_i32 as f64 / 2147483648.0) as f32;
                        ch_vec.push(sample_f32);
                    }
                }
            }
            SampleFormat::PcmS32Le => {
                let mut idx = 0;
                for _ in 0..frames_to_read {
                    for ch_vec in channel_data.iter_mut().take(num_channels) {
                        let sample_i32 = i32::from_le_bytes([
                            byte_buf[idx],
                            byte_buf[idx + 1],
                            byte_buf[idx + 2],
                            byte_buf[idx + 3],
                        ]);
                        idx += 4;
                        let sample_f32 = (sample_i32 as f64 / 2147483648.0) as f32;
                        ch_vec.push(sample_f32);
                    }
                }
            }
            SampleFormat::Float32Be => {
                let mut idx = 0;
                for _ in 0..frames_to_read {
                    for ch_vec in channel_data.iter_mut().take(num_channels) {
                        let sample_f32 = f32::from_be_bytes([
                            byte_buf[idx],
                            byte_buf[idx + 1],
                            byte_buf[idx + 2],
                            byte_buf[idx + 3],
                        ]);
                        idx += 4;
                        ch_vec.push(sample_f32);
                    }
                }
            }
            _ => {
                return Err(DecodeError::UnsupportedFormat(format!(
                    "Unsupported AIFF sample format: {:?}",
                    self.info.sample_format
                )));
            }
        }

        Ok(Some(AudioBuffer {
            channels: channel_data,
            sample_rate: self.info.sample_rate,
        }))
    }
}

/// Converts an 80-bit IEEE 754 extended precision floating point byte array into f64.
pub fn decode_extended_80(bytes: &[u8]) -> f64 {
    if bytes.len() < 10 {
        return 0.0;
    }
    let expon = (((bytes[0] as u16) & 0x7F) << 8) | (bytes[1] as u16);
    let sign = (bytes[0] & 0x80) != 0;
    let mantissa = u64::from_be_bytes(bytes[2..10].try_into().unwrap());

    if expon == 0 && mantissa == 0 {
        return 0.0;
    }
    if expon == 0x7FFF {
        return if sign {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }

    // Bias is 16383. Mantissa has explicit 1.xxxx in bit 63.
    let shift = expon as i32 - 16383 - 63;
    let val = (mantissa as f64) * 2.0f64.powi(shift);
    if sign {
        -val
    } else {
        val
    }
}

/// Encodes an f64 sample rate into an 80-bit IEEE 754 extended precision byte array.
pub fn encode_extended_80(val: f64) -> [u8; 10] {
    let mut out = [0u8; 10];
    if val == 0.0 {
        return out;
    }
    let sign = val < 0.0;
    let abs_val = val.abs();

    let mut exp = (abs_val.log2().floor()) as i32;
    let mut mantissa_f = abs_val / 2.0f64.powi(exp);

    // Normalize so 1.0 <= mantissa_f < 2.0
    while mantissa_f < 1.0 {
        mantissa_f *= 2.0;
        exp -= 1;
    }
    while mantissa_f >= 2.0 {
        mantissa_f /= 2.0;
        exp += 1;
    }

    let biased_exp = (exp + 16383) as u16;
    let mut e0 = ((biased_exp >> 8) & 0x7F) as u8;
    if sign {
        e0 |= 0x80;
    }
    let e1 = (biased_exp & 0xFF) as u8;

    let mantissa_u64 = (mantissa_f * (1u64 << 63) as f64) as u64;
    let m_bytes = mantissa_u64.to_be_bytes();

    out[0] = e0;
    out[1] = e1;
    out[2..10].copy_from_slice(&m_bytes);
    out
}

/// Output format options for writing AIFF files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiffEncoding {
    /// 16-bit big-endian signed integer PCM.
    Pcm16,
    /// 24-bit big-endian signed integer PCM.
    Pcm24,
    /// 32-bit big-endian signed integer PCM.
    Pcm32,
    /// 32-bit big-endian IEEE float (AIFC).
    Float32,
}

/// Writer for standard AIFF and AIFF-C audio streams.
pub struct AiffWriter;

impl AiffWriter {
    /// Encodes multi-channel planar audio to AIFF or AIFC byte vector.
    pub fn encode(
        channels: &[&[f32]],
        sample_rate: u32,
        encoding: AiffEncoding,
        title: Option<&str>,
        artist: Option<&str>,
    ) -> Result<Vec<u8>, String> {
        if channels.is_empty() {
            return Err("Cannot encode AIFF with 0 channels".to_string());
        }
        let num_channels = channels.len();
        let num_frames = channels[0].len();
        for (i, ch) in channels.iter().enumerate() {
            if ch.len() != num_frames {
                return Err(format!(
                    "Channel {i} length {} != channel 0 length {num_frames}",
                    ch.len()
                ));
            }
        }

        let is_aifc = encoding == AiffEncoding::Float32;
        let bits_per_sample = match encoding {
            AiffEncoding::Pcm16 => 16u16,
            AiffEncoding::Pcm24 => 24u16,
            AiffEncoding::Pcm32 | AiffEncoding::Float32 => 32u16,
        };

        let bytes_per_sample = (bits_per_sample / 8) as usize;
        let sound_data_len = (num_frames * num_channels * bytes_per_sample) as u32;

        let mut out = Vec::new();
        out.extend_from_slice(b"FORM");
        out.extend_from_slice(&0u32.to_be_bytes()); // placeholder for form size
        out.extend_from_slice(if is_aifc { b"AIFC" } else { b"AIFF" });

        // COMM chunk
        out.extend_from_slice(b"COMM");
        let aifc_desc = b"32-bit floating point";
        let comm_size = if is_aifc {
            let mut extra = 4 + 1 + aifc_desc.len();
            if !extra.is_multiple_of(2) {
                extra += 1;
            }
            (18 + extra) as u32
        } else {
            18u32
        };
        out.extend_from_slice(&comm_size.to_be_bytes());
        out.extend_from_slice(&(num_channels as u16).to_be_bytes());
        out.extend_from_slice(&(num_frames as u32).to_be_bytes());
        out.extend_from_slice(&bits_per_sample.to_be_bytes());
        out.extend_from_slice(&encode_extended_80(sample_rate as f64));

        if is_aifc {
            out.extend_from_slice(b"fl32");
            // Pascal string "32-bit floating point" padded to even size
            out.push(aifc_desc.len() as u8);
            out.extend_from_slice(aifc_desc);
            if !(aifc_desc.len() + 1).is_multiple_of(2) {
                out.push(0);
            }
        }

        // Optional NAME chunk
        if let Some(t) = title {
            out.extend_from_slice(b"NAME");
            let t_bytes = t.as_bytes();
            out.extend_from_slice(&(t_bytes.len() as u32).to_be_bytes());
            out.extend_from_slice(t_bytes);
            if !t_bytes.len().is_multiple_of(2) {
                out.push(0);
            }
        }

        // Optional AUTH chunk
        if let Some(a) = artist {
            out.extend_from_slice(b"AUTH");
            let a_bytes = a.as_bytes();
            out.extend_from_slice(&(a_bytes.len() as u32).to_be_bytes());
            out.extend_from_slice(a_bytes);
            if !a_bytes.len().is_multiple_of(2) {
                out.push(0);
            }
        }

        // SSND chunk
        out.extend_from_slice(b"SSND");
        let ssnd_chunk_size = 8 + sound_data_len;
        out.extend_from_slice(&ssnd_chunk_size.to_be_bytes());
        out.extend_from_slice(&0u32.to_be_bytes()); // offset = 0
        out.extend_from_slice(&0u32.to_be_bytes()); // block size = 0

        match encoding {
            AiffEncoding::Pcm16 => {
                for frame in 0..num_frames {
                    for ch_slice in channels.iter().take(num_channels) {
                        let sample = ch_slice[frame];
                        let scaled = (sample * 32768.0).round().clamp(-32768.0, 32767.0) as i16;
                        out.extend_from_slice(&scaled.to_be_bytes());
                    }
                }
            }
            AiffEncoding::Pcm24 => {
                for frame in 0..num_frames {
                    for ch_slice in channels.iter().take(num_channels) {
                        let sample = ch_slice[frame];
                        let scaled =
                            (sample * 8388608.0).round().clamp(-8388608.0, 8388607.0) as i32;
                        let bytes = scaled.to_be_bytes();
                        out.push(bytes[1]);
                        out.push(bytes[2]);
                        out.push(bytes[3]);
                    }
                }
            }
            AiffEncoding::Pcm32 => {
                for frame in 0..num_frames {
                    for ch_slice in channels.iter().take(num_channels) {
                        let sample = ch_slice[frame];
                        let scaled = (sample as f64 * 2147483648.0)
                            .round()
                            .clamp(-2147483648.0, 2147483647.0)
                            as i32;
                        out.extend_from_slice(&scaled.to_be_bytes());
                    }
                }
            }
            AiffEncoding::Float32 => {
                for frame in 0..num_frames {
                    for ch_slice in channels.iter().take(num_channels) {
                        let sample = ch_slice[frame];
                        out.extend_from_slice(&sample.to_be_bytes());
                    }
                }
            }
        }

        if !sound_data_len.is_multiple_of(2) {
            out.push(0);
        }

        // Fill in total FORM size (excluding 8 bytes of FORM header)
        let total_form_size = (out.len() - 8) as u32;
        let form_size_bytes = total_form_size.to_be_bytes();
        out[4..8].copy_from_slice(&form_size_bytes);

        Ok(out)
    }
}
