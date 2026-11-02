//! WAV, RF64, and Broadcast Wave Format (BWF) decoder and writer.

use super::{
    AudioBuffer, AudioDecoder, AudioFileInfo, BwfInfo, ContainerFormat, ContainerMetadata,
    DecodeError, SampleFormat,
};
use std::io::{Read, Seek, SeekFrom};

const WAVE_FORMAT_PCM: u16 = 0x0001;
const WAVE_FORMAT_IEEE_FLOAT: u16 = 0x0003;
const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;

// Subformat GUID for PCM: {00000001-0000-0010-8000-00AA00389B71}
const SUBTYPE_PCM: [u8; 16] = [
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];
// Subformat GUID for IEEE Float: {00000003-0000-0010-8000-00AA00389B71}
const SUBTYPE_IEEE_FLOAT: [u8; 16] = [
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];

/// Streaming decoder for WAV, RF64, and BWF files.
pub struct WavDecoder<R: Read + Seek> {
    reader: R,
    info: AudioFileInfo,
    data_start: u64,
    _data_bytes: u64,
    current_frame: u64,
    block_align: usize,
}

impl<R: Read + Seek> WavDecoder<R> {
    /// Opens and parses the WAV/RF64/BWF header from `reader`.
    pub fn new(mut reader: R) -> Result<Self, DecodeError> {
        let mut header = [0u8; 12];
        reader
            .read_exact(&mut header)
            .map_err(|e| DecodeError::UnexpectedEof(e.to_string()))?;

        let magic = &header[0..4];
        let is_rf64 = magic == b"RF64" || magic == b"BW64";
        let is_riff = magic == b"RIFF";

        if !is_riff && !is_rf64 {
            return Err(DecodeError::InvalidHeader(format!(
                "Expected RIFF, RF64, or BW64 header, found {:?}",
                magic
            )));
        }

        if &header[8..12] != b"WAVE" {
            return Err(DecodeError::InvalidHeader(format!(
                "Expected WAVE format tag, found {:?}",
                &header[8..12]
            )));
        }

        let mut _ds64_riff_size: Option<u64> = None;
        let mut ds64_data_size: Option<u64> = None;
        let mut ds64_sample_count: Option<u64> = None;

        let mut fmt_found = false;
        let mut channels = 0u16;
        let mut sample_rate = 0u32;
        let mut bits_per_sample = 0u16;
        let mut block_align = 0usize;
        let mut channel_mask: Option<u32> = None;
        let mut sample_format = SampleFormat::PcmS16Le;

        let mut data_start: Option<u64> = None;
        let mut data_bytes: Option<u64> = None;

        let mut bwf_info: Option<BwfInfo> = None;
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
            let raw_size =
                u32::from_le_bytes([chunk_hdr[4], chunk_hdr[5], chunk_hdr[6], chunk_hdr[7]]) as u64;

            if is_rf64 && chunk_id == b"ds64" {
                if raw_size < 28 {
                    return Err(DecodeError::CorruptData(
                        "ds64 chunk size too small".to_string(),
                    ));
                }
                let mut ds_buf = vec![0u8; raw_size as usize];
                reader
                    .read_exact(&mut ds_buf)
                    .map_err(|e| DecodeError::UnexpectedEof(e.to_string()))?;
                cur_pos += raw_size;

                let riff_sz = u64::from_le_bytes(ds_buf[0..8].try_into().unwrap());
                let data_sz = u64::from_le_bytes(ds_buf[8..16].try_into().unwrap());
                let sample_cnt = u64::from_le_bytes(ds_buf[16..24].try_into().unwrap());

                _ds64_riff_size = Some(riff_sz);
                ds64_data_size = Some(data_sz);
                ds64_sample_count = Some(sample_cnt);

                if !raw_size.is_multiple_of(2) {
                    let mut pad = [0u8; 1];
                    let _ = reader.read_exact(&mut pad);
                    cur_pos += 1;
                }
                continue;
            }

            let effective_size = if raw_size == 0xFFFFFFFF && is_rf64 && chunk_id == b"data" {
                ds64_data_size.ok_or_else(|| {
                    DecodeError::CorruptData(
                        "data chunk has 0xFFFFFFFF size but ds64 chunk missing".to_string(),
                    )
                })?
            } else {
                raw_size
            };

            match chunk_id {
                b"fmt " => {
                    if effective_size < 16 {
                        return Err(DecodeError::CorruptData("fmt chunk too small".to_string()));
                    }
                    let mut fmt_buf = vec![0u8; effective_size as usize];
                    reader
                        .read_exact(&mut fmt_buf)
                        .map_err(|e| DecodeError::UnexpectedEof(e.to_string()))?;
                    cur_pos += effective_size;

                    let audio_format_code = u16::from_le_bytes([fmt_buf[0], fmt_buf[1]]);
                    channels = u16::from_le_bytes([fmt_buf[2], fmt_buf[3]]);
                    sample_rate =
                        u32::from_le_bytes([fmt_buf[4], fmt_buf[5], fmt_buf[6], fmt_buf[7]]);
                    let reported_block_align =
                        u16::from_le_bytes([fmt_buf[12], fmt_buf[13]]) as usize;
                    bits_per_sample = u16::from_le_bytes([fmt_buf[14], fmt_buf[15]]);
                    block_align = reported_block_align;

                    if channels == 0 {
                        return Err(DecodeError::CorruptData(
                            "Channel count is zero".to_string(),
                        ));
                    }
                    if sample_rate == 0 {
                        return Err(DecodeError::CorruptData("Sample rate is zero".to_string()));
                    }

                    if audio_format_code == WAVE_FORMAT_PCM {
                        sample_format = match bits_per_sample {
                            8 => SampleFormat::PcmU8,
                            16 => SampleFormat::PcmS16Le,
                            24 => SampleFormat::PcmS24Le,
                            32 => SampleFormat::PcmS32Le,
                            other => {
                                return Err(DecodeError::UnsupportedFormat(format!(
                                    "Unsupported PCM bit depth: {other}"
                                )))
                            }
                        };
                    } else if audio_format_code == WAVE_FORMAT_IEEE_FLOAT {
                        sample_format = match bits_per_sample {
                            32 => SampleFormat::Float32Le,
                            64 => SampleFormat::Float64Le,
                            other => {
                                return Err(DecodeError::UnsupportedFormat(format!(
                                    "Unsupported float bit depth: {other}"
                                )))
                            }
                        };
                    } else if audio_format_code == WAVE_FORMAT_EXTENSIBLE {
                        if fmt_buf.len() < 40 {
                            return Err(DecodeError::CorruptData(
                                "WAVE_FORMAT_EXTENSIBLE fmt chunk too small".to_string(),
                            ));
                        }
                        channel_mask = Some(u32::from_le_bytes([
                            fmt_buf[20],
                            fmt_buf[21],
                            fmt_buf[22],
                            fmt_buf[23],
                        ]));
                        let sub_guid = &fmt_buf[24..40];

                        if sub_guid == SUBTYPE_PCM {
                            sample_format = match bits_per_sample {
                                8 => SampleFormat::PcmU8,
                                16 => SampleFormat::PcmS16Le,
                                24 => SampleFormat::PcmS24Le,
                                32 => SampleFormat::PcmS32Le,
                                other => {
                                    return Err(DecodeError::UnsupportedFormat(format!(
                                        "Unsupported extensible PCM bit depth: {other}"
                                    )))
                                }
                            };
                        } else if sub_guid == SUBTYPE_IEEE_FLOAT {
                            sample_format = match bits_per_sample {
                                32 => SampleFormat::Float32Le,
                                64 => SampleFormat::Float64Le,
                                other => {
                                    return Err(DecodeError::UnsupportedFormat(format!(
                                        "Unsupported extensible float bit depth: {other}"
                                    )))
                                }
                            };
                        } else {
                            return Err(DecodeError::UnsupportedFormat(format!(
                                "Unsupported extensible sub-format GUID: {:02x?}",
                                sub_guid
                            )));
                        }
                    } else {
                        return Err(DecodeError::UnsupportedFormat(format!(
                            "Unsupported WAV format tag: {audio_format_code:#06x}"
                        )));
                    }

                    let expected_align = (channels as usize) * sample_format.bytes_per_sample();
                    if block_align == 0 {
                        block_align = expected_align;
                    }

                    fmt_found = true;
                }
                b"bext" => {
                    let mut bext_buf = vec![0u8; effective_size as usize];
                    reader
                        .read_exact(&mut bext_buf)
                        .map_err(|e| DecodeError::UnexpectedEof(e.to_string()))?;
                    cur_pos += effective_size;

                    if bext_buf.len() >= 602 {
                        let desc = read_null_terminated_ascii(&bext_buf[0..256]);
                        let orig = read_null_terminated_ascii(&bext_buf[256..288]);
                        let orig_ref = read_null_terminated_ascii(&bext_buf[288..320]);
                        let date = read_null_terminated_ascii(&bext_buf[320..330]);
                        let time = read_null_terminated_ascii(&bext_buf[330..338]);
                        let time_ref_low =
                            u32::from_le_bytes(bext_buf[338..342].try_into().unwrap()) as u64;
                        let time_ref_high =
                            u32::from_le_bytes(bext_buf[342..346].try_into().unwrap()) as u64;
                        let time_reference = (time_ref_high << 32) | time_ref_low;
                        let version = u16::from_le_bytes([bext_buf[346], bext_buf[347]]);

                        let mut umid = [0u8; 64];
                        umid.copy_from_slice(&bext_buf[348..412]);

                        let loudness_val = i16::from_le_bytes([bext_buf[412], bext_buf[413]]);
                        let loudness_range = i16::from_le_bytes([bext_buf[414], bext_buf[415]]);
                        let max_tp = i16::from_le_bytes([bext_buf[416], bext_buf[417]]);

                        let coding_history = if bext_buf.len() > 602 {
                            read_null_terminated_ascii(&bext_buf[602..])
                        } else {
                            String::new()
                        };

                        bwf_info = Some(BwfInfo {
                            description: desc,
                            originator: orig,
                            originator_reference: orig_ref,
                            origination_date: date,
                            origination_time: time,
                            time_reference,
                            version,
                            umid: Some(umid),
                            loudness_value: Some(loudness_val),
                            loudness_range: Some(loudness_range),
                            max_true_peak_level: Some(max_tp),
                            coding_history,
                        });
                    }
                }
                b"LIST" => {
                    let mut list_buf = vec![0u8; effective_size as usize];
                    reader
                        .read_exact(&mut list_buf)
                        .map_err(|e| DecodeError::UnexpectedEof(e.to_string()))?;
                    cur_pos += effective_size;

                    if list_buf.len() >= 4 && &list_buf[0..4] == b"INFO" {
                        parse_info_list(&list_buf[4..], &mut metadata);
                    }
                }
                b"data" => {
                    data_start = Some(cur_pos);
                    data_bytes = Some(effective_size);
                    // Do not read the entire data chunk into RAM! Just seek past it.
                    reader.seek(SeekFrom::Current(effective_size as i64))?;
                    cur_pos += effective_size;
                }
                _ => {
                    // Skip unknown chunk
                    reader.seek(SeekFrom::Current(effective_size as i64))?;
                    cur_pos += effective_size;
                }
            }

            // RIFF chunks are padded to 2-byte word boundaries
            if !effective_size.is_multiple_of(2) {
                let mut pad = [0u8; 1];
                let _ = reader.read_exact(&mut pad);
                cur_pos += 1;
            }
        }

        if !fmt_found {
            return Err(DecodeError::CorruptData(
                "Missing 'fmt ' chunk in WAV".to_string(),
            ));
        }

        let data_start = data_start
            .ok_or_else(|| DecodeError::CorruptData("Missing 'data' chunk in WAV".to_string()))?;
        let data_bytes = data_bytes.unwrap_or(0);

        let total_frames = if let Some(cnt) = ds64_sample_count {
            cnt
        } else if block_align > 0 {
            data_bytes / (block_align as u64)
        } else {
            0
        };

        let duration_seconds = if sample_rate > 0 {
            total_frames as f64 / sample_rate as f64
        } else {
            0.0
        };

        let container_format = if is_rf64 {
            ContainerFormat::Rf64
        } else if bwf_info.is_some() {
            ContainerFormat::Bwf
        } else {
            ContainerFormat::Wav
        };

        let info = AudioFileInfo {
            container_format,
            sample_format,
            channels,
            sample_rate,
            bits_per_sample,
            total_frames,
            duration_seconds,
            channel_mask,
            metadata,
            bwf_info,
        };

        // Rewind reader to start of audio data
        reader.seek(SeekFrom::Start(data_start))?;

        Ok(Self {
            reader,
            info,
            data_start,
            _data_bytes: data_bytes,
            current_frame: 0,
            block_align,
        })
    }
}

impl<R: Read + Seek> AudioDecoder for WavDecoder<R> {
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
        let _bytes_per_sample = self.info.sample_format.bytes_per_sample();
        let total_bytes = frames_to_read * self.block_align;

        let mut byte_buf = vec![0u8; total_bytes];
        self.reader
            .read_exact(&mut byte_buf)
            .map_err(|e| DecodeError::UnexpectedEof(e.to_string()))?;
        self.current_frame += frames_to_read as u64;

        let mut channel_data = vec![Vec::with_capacity(frames_to_read); num_channels];

        match self.info.sample_format {
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
            SampleFormat::Float32Le => {
                let mut idx = 0;
                for _ in 0..frames_to_read {
                    for ch_vec in channel_data.iter_mut().take(num_channels) {
                        let sample_f32 = f32::from_le_bytes([
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
            SampleFormat::Float64Le => {
                let mut idx = 0;
                for _ in 0..frames_to_read {
                    for ch_vec in channel_data.iter_mut().take(num_channels) {
                        let sample_f64 = f64::from_le_bytes([
                            byte_buf[idx],
                            byte_buf[idx + 1],
                            byte_buf[idx + 2],
                            byte_buf[idx + 3],
                            byte_buf[idx + 4],
                            byte_buf[idx + 5],
                            byte_buf[idx + 6],
                            byte_buf[idx + 7],
                        ]);
                        idx += 8;
                        ch_vec.push(sample_f64 as f32);
                    }
                }
            }
            _ => {
                return Err(DecodeError::UnsupportedFormat(format!(
                    "Unsupported WAV sample format: {:?}",
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

/// Helper to read null-terminated ASCII string from byte slice.
fn read_null_terminated_ascii(bytes: &[u8]) -> String {
    let null_idx = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..null_idx])
        .trim()
        .to_string()
}

/// Parses standard RIFF `INFO` list chunks into `ContainerMetadata`.
fn parse_info_list(mut data: &[u8], metadata: &mut ContainerMetadata) {
    while data.len() >= 8 {
        let tag = &data[0..4];
        let len = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
        data = &data[8..];
        if data.len() < len {
            break;
        }
        let text = read_null_terminated_ascii(&data[..len]);
        match tag {
            b"INAM" => metadata.title = Some(text),
            b"IART" => metadata.artist = Some(text),
            b"IPRD" => metadata.album = Some(text),
            b"ICRD" => metadata.date = Some(text),
            b"IGNR" => metadata.genre = Some(text),
            b"ICMT" => metadata.comment = Some(text),
            b"ICOP" => metadata.copyright = Some(text),
            b"ISFT" => metadata.software = Some(text),
            _ => {}
        }
        let advance = (len + 1) & !1;
        if data.len() >= advance {
            data = &data[advance..];
        } else {
            break;
        }
    }
}

/// Output format options for writing WAV files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavEncoding {
    /// 16-bit signed integer PCM.
    Pcm16,
    /// 24-bit signed integer packed PCM.
    Pcm24,
    /// 32-bit signed integer PCM.
    Pcm32,
    /// 32-bit IEEE 754 floating point.
    Float32,
}

/// Writer for standard WAV, BWF, and RF64 audio streams.
pub struct WavWriter;

impl WavWriter {
    /// Encodes multi-channel planar audio to standard WAV, BWF, or RF64 bytes.
    pub fn encode(
        channels: &[&[f32]],
        sample_rate: u32,
        encoding: WavEncoding,
        bwf_info: Option<&BwfInfo>,
        force_rf64: bool,
    ) -> Result<Vec<u8>, String> {
        if channels.is_empty() {
            return Err("Cannot encode WAV with 0 channels".to_string());
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

        let (format_tag, bits_per_sample) = match encoding {
            WavEncoding::Pcm16 => (WAVE_FORMAT_PCM, 16u16),
            WavEncoding::Pcm24 => (WAVE_FORMAT_PCM, 24u16),
            WavEncoding::Pcm32 => (WAVE_FORMAT_PCM, 32u16),
            WavEncoding::Float32 => (WAVE_FORMAT_IEEE_FLOAT, 32u16),
        };

        let bytes_per_sample = (bits_per_sample / 8) as usize;
        let block_align = (num_channels * bytes_per_sample) as u16;
        let byte_rate = sample_rate * (block_align as u32);
        let data_bytes = (num_frames * num_channels * bytes_per_sample) as u64;

        let is_rf64 = force_rf64 || (data_bytes + 256 >= 0xFFFFFFFF);

        let mut out = Vec::new();

        if is_rf64 {
            out.extend_from_slice(b"RF64");
            out.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes());
            out.extend_from_slice(b"WAVE");

            // ds64 chunk
            out.extend_from_slice(b"ds64");
            out.extend_from_slice(&28u32.to_le_bytes());
            let riff_size64 = data_bytes + 120; // approximate total RF64 size
            out.extend_from_slice(&riff_size64.to_le_bytes());
            out.extend_from_slice(&data_bytes.to_le_bytes());
            out.extend_from_slice(&(num_frames as u64).to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes()); // tableLength = 0
        } else {
            out.extend_from_slice(b"RIFF");
            let mut riff_size = 36 + data_bytes as u32;
            if bwf_info.is_some() {
                riff_size += 610; // bext chunk header + payload
            }
            out.extend_from_slice(&riff_size.to_le_bytes());
            out.extend_from_slice(b"WAVE");
        }

        // bext chunk if provided
        if let Some(b) = bwf_info {
            out.extend_from_slice(b"bext");
            let bext_len = 602 + b.coding_history.len() as u32;
            out.extend_from_slice(&bext_len.to_le_bytes());

            let mut desc_buf = [0u8; 256];
            let desc_bytes = b.description.as_bytes();
            desc_buf[..desc_bytes.len().min(256)]
                .copy_from_slice(&desc_bytes[..desc_bytes.len().min(256)]);
            out.extend_from_slice(&desc_buf);

            let mut orig_buf = [0u8; 32];
            let orig_bytes = b.originator.as_bytes();
            orig_buf[..orig_bytes.len().min(32)]
                .copy_from_slice(&orig_bytes[..orig_bytes.len().min(32)]);
            out.extend_from_slice(&orig_buf);

            let mut ref_buf = [0u8; 32];
            let ref_bytes = b.originator_reference.as_bytes();
            ref_buf[..ref_bytes.len().min(32)]
                .copy_from_slice(&ref_bytes[..ref_bytes.len().min(32)]);
            out.extend_from_slice(&ref_buf);

            let mut date_buf = [0u8; 10];
            let date_bytes = b.origination_date.as_bytes();
            date_buf[..date_bytes.len().min(10)]
                .copy_from_slice(&date_bytes[..date_bytes.len().min(10)]);
            out.extend_from_slice(&date_buf);

            let mut time_buf = [0u8; 8];
            let time_bytes = b.origination_time.as_bytes();
            time_buf[..time_bytes.len().min(8)]
                .copy_from_slice(&time_bytes[..time_bytes.len().min(8)]);
            out.extend_from_slice(&time_buf);

            let time_ref_low = (b.time_reference & 0xFFFFFFFF) as u32;
            let time_ref_high = (b.time_reference >> 32) as u32;
            out.extend_from_slice(&time_ref_low.to_le_bytes());
            out.extend_from_slice(&time_ref_high.to_le_bytes());

            out.extend_from_slice(&b.version.to_le_bytes());
            out.extend_from_slice(&b.umid.unwrap_or([0u8; 64]));
            out.extend_from_slice(&b.loudness_value.unwrap_or(0).to_le_bytes());
            out.extend_from_slice(&b.loudness_range.unwrap_or(0).to_le_bytes());
            out.extend_from_slice(&b.max_true_peak_level.unwrap_or(0).to_le_bytes());
            out.extend_from_slice(&0i16.to_le_bytes()); // max momentary
            out.extend_from_slice(&0i16.to_le_bytes()); // max short term
            out.extend_from_slice(&[0u8; 180]); // 180 reserved bytes per EBU Tech 3285
            out.extend_from_slice(b.coding_history.as_bytes());

            if !bext_len.is_multiple_of(2) {
                out.push(0);
            }
        }

        // fmt chunk
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&format_tag.to_le_bytes());
        out.extend_from_slice(&(num_channels as u16).to_le_bytes());
        out.extend_from_slice(&sample_rate.to_le_bytes());
        out.extend_from_slice(&byte_rate.to_le_bytes());
        out.extend_from_slice(&block_align.to_le_bytes());
        out.extend_from_slice(&bits_per_sample.to_le_bytes());

        // data chunk
        out.extend_from_slice(b"data");
        if is_rf64 {
            out.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes());
        } else {
            out.extend_from_slice(&(data_bytes as u32).to_le_bytes());
        }

        match encoding {
            WavEncoding::Pcm16 => {
                for frame in 0..num_frames {
                    for ch_slice in channels.iter().take(num_channels) {
                        let sample = ch_slice[frame];
                        let scaled = (sample * 32768.0).round().clamp(-32768.0, 32767.0) as i16;
                        out.extend_from_slice(&scaled.to_le_bytes());
                    }
                }
            }
            WavEncoding::Pcm24 => {
                for frame in 0..num_frames {
                    for ch_slice in channels.iter().take(num_channels) {
                        let sample = ch_slice[frame];
                        let scaled =
                            (sample * 8388608.0).round().clamp(-8388608.0, 8388607.0) as i32;
                        let bytes = scaled.to_le_bytes();
                        out.push(bytes[0]);
                        out.push(bytes[1]);
                        out.push(bytes[2]);
                    }
                }
            }
            WavEncoding::Pcm32 => {
                for frame in 0..num_frames {
                    for ch_slice in channels.iter().take(num_channels) {
                        let sample = ch_slice[frame];
                        let scaled = (sample as f64 * 2147483648.0)
                            .round()
                            .clamp(-2147483648.0, 2147483647.0)
                            as i32;
                        out.extend_from_slice(&scaled.to_le_bytes());
                    }
                }
            }
            WavEncoding::Float32 => {
                for frame in 0..num_frames {
                    for ch_slice in channels.iter().take(num_channels) {
                        let sample = ch_slice[frame];
                        out.extend_from_slice(&sample.to_le_bytes());
                    }
                }
            }
        }

        if !data_bytes.is_multiple_of(2) {
            out.push(0);
        }

        Ok(out)
    }
}
