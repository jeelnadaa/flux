//! Pure-Rust streaming Ogg Opus decoder with gapless preskip trimming.

use super::metadata::{ContainerMetadata, GaplessInfo};
use super::seek_index::SeekIndex;
use super::{AudioBuffer, AudioDecoder, AudioFileInfo, ContainerFormat, DecodeError, SampleFormat};
use opus1::{Channels, OpusDecoder as InnerOpusDecoder, SampleRate};
use std::io::{Read, Seek, SeekFrom};

/// Pure-Rust streaming Ogg Opus decoder.
pub struct OpusDecoder<R: Read + Seek> {
    reader: R,
    inner: InnerOpusDecoder,
    info: AudioFileInfo,
    current_frame: u64,
    seek_index: SeekIndex,
    preskip: u32,
    preskip_skipped: u32,
    leftover_samples: Vec<Vec<f32>>,
}

impl<R: Read + Seek> OpusDecoder<R> {
    /// Opens and parses an Ogg Opus stream from `reader`.
    pub fn new(mut reader: R) -> Result<Self, DecodeError> {
        let (head_pkt, tags_pkt, first_audio_offset) = read_initial_ogg_packets(&mut reader)?;

        if head_pkt.len() < 19 || &head_pkt[0..8] != b"OpusHead" {
            return Err(DecodeError::InvalidHeader(
                "Missing OpusHead magic".to_string(),
            ));
        }

        let channels_num = head_pkt[9];
        if channels_num != 1 && channels_num != 2 {
            return Err(DecodeError::UnsupportedFormat(format!(
                "Unsupported Opus channel count: {channels_num}"
            )));
        }

        let preskip = u16::from_le_bytes([head_pkt[10], head_pkt[11]]) as u32;
        let _original_rate =
            u32::from_le_bytes([head_pkt[12], head_pkt[13], head_pkt[14], head_pkt[15]]);

        let opus_channels = if channels_num == 1 {
            Channels::Mono
        } else {
            Channels::Stereo
        };

        let inner = InnerOpusDecoder::new(SampleRate::Hz48000, opus_channels).map_err(|e| {
            DecodeError::UnsupportedFormat(format!("Failed to create Opus decoder: {e:?}"))
        })?;

        let mut metadata = ContainerMetadata::default();
        if tags_pkt.len() >= 8 && &tags_pkt[0..8] == b"OpusTags" {
            parse_opus_tags(&tags_pkt[8..], &mut metadata);
        }

        metadata.gapless_info = Some(GaplessInfo {
            encoder_delay: preskip,
            encoder_padding: 0,
            original_samples: None,
        });

        // Seek back to first audio packet offset
        reader.seek(SeekFrom::Start(first_audio_offset))?;

        let info = AudioFileInfo {
            container_format: ContainerFormat::Opus,
            sample_format: SampleFormat::Float32Le,
            channels: channels_num as u16,
            sample_rate: 48000,
            bits_per_sample: 16,
            total_frames: 0, // Determined as stream is read or via duration
            duration_seconds: 0.0,
            channel_mask: None,
            metadata,
            bwf_info: None,
        };

        Ok(Self {
            reader,
            inner,
            info,
            current_frame: 0,
            seek_index: SeekIndex::new(),
            preskip,
            preskip_skipped: 0,
            leftover_samples: vec![Vec::new(); channels_num as usize],
        })
    }

    /// Returns a reference to the streaming seek index.
    pub fn seek_index(&self) -> &SeekIndex {
        &self.seek_index
    }
}

impl<R: Read + Seek> AudioDecoder for OpusDecoder<R> {
    fn info(&self) -> &AudioFileInfo {
        &self.info
    }

    fn current_frame(&self) -> u64 {
        self.current_frame
    }

    fn seek_frame(&mut self, frame: u64) -> Result<(), DecodeError> {
        // Find nearest sync frame
        if let Some(entry) = self.seek_index.find_sync_frame(frame) {
            let offset = entry.byte_offset;
            let sync_frame = entry.frame_index;
            self.reader.seek(SeekFrom::Start(offset))?;
            self.inner.reset_state();
            self.leftover_samples.iter_mut().for_each(|c| c.clear());
            self.current_frame = sync_frame;

            let diff = frame.saturating_sub(sync_frame);
            if diff > 0 {
                let _ = self.decode_chunk(diff as usize)?;
            }
            self.current_frame = frame;
            Ok(())
        } else {
            // Seek to start
            self.reader.seek(SeekFrom::Start(0))?;
            let _ = read_initial_ogg_packets(&mut self.reader)?;
            self.inner.reset_state();
            self.leftover_samples.iter_mut().for_each(|c| c.clear());
            self.preskip_skipped = 0;
            self.current_frame = 0;

            if frame > 0 {
                let _ = self.decode_chunk(frame as usize)?;
            }
            self.current_frame = frame;
            Ok(())
        }
    }

    fn decode_chunk(&mut self, max_frames: usize) -> Result<Option<AudioBuffer>, DecodeError> {
        let num_channels = self.info.channels as usize;
        let mut out_channels = vec![Vec::with_capacity(max_frames); num_channels];
        let mut collected_frames = 0;

        // Drain leftovers first
        let leftover_avail = self.leftover_samples.first().map_or(0, |c| c.len());
        if leftover_avail > 0 {
            let to_drain = leftover_avail.min(max_frames);
            for (ch_idx, ch_out) in out_channels.iter_mut().enumerate() {
                ch_out.extend(self.leftover_samples[ch_idx].drain(0..to_drain));
            }
            collected_frames += to_drain;
            if collected_frames >= max_frames {
                self.current_frame += collected_frames as u64;
                return Ok(Some(AudioBuffer {
                    channels: out_channels,
                    sample_rate: self.info.sample_rate,
                }));
            }
        }

        let mut pcm_out = [0.0f32; 5760 * 2]; // Max 120 ms stereo frame

        loop {
            let stream_pos = self.reader.stream_position().unwrap_or(0);
            let packet_bytes = match read_next_ogg_packet(&mut self.reader)? {
                Some(bytes) => bytes,
                None => break,
            };

            // Estimate frame size or decode
            let decoded_samples = match self.inner.decode(&packet_bytes, 5760, &mut pcm_out) {
                Ok(samples) => samples,
                Err(_) => continue,
            };

            if decoded_samples == 0 {
                continue;
            }

            self.seek_index
                .add_entry(self.current_frame + collected_frames as u64, stream_pos, 0);

            // De-interleave
            let mut planar = vec![Vec::with_capacity(decoded_samples); num_channels];
            for frame in 0..decoded_samples {
                for ch in 0..num_channels {
                    planar[ch].push(pcm_out[frame * num_channels + ch]);
                }
            }

            // Handle preskip delay trimming
            let mut start_frame = 0;
            if self.preskip_skipped < self.preskip {
                let remaining_delay = (self.preskip - self.preskip_skipped) as usize;
                let skip = remaining_delay.min(decoded_samples);
                self.preskip_skipped += skip as u32;
                start_frame = skip;
            }

            let valid_frames = decoded_samples - start_frame;
            if valid_frames == 0 {
                continue;
            }

            let needed = max_frames - collected_frames;
            let take = valid_frames.min(needed);

            for ch in 0..num_channels {
                out_channels[ch].extend_from_slice(&planar[ch][start_frame..start_frame + take]);
                if valid_frames > take {
                    self.leftover_samples[ch].extend_from_slice(&planar[ch][start_frame + take..]);
                }
            }

            collected_frames += take;
            if collected_frames >= max_frames {
                break;
            }
        }

        if collected_frames == 0 {
            return Ok(None);
        }

        self.current_frame += collected_frames as u64;
        Ok(Some(AudioBuffer {
            channels: out_channels,
            sample_rate: self.info.sample_rate,
        }))
    }
}

/// Helper to read initial OpusHead and OpusTags packets from Ogg bitstream.
fn read_initial_ogg_packets<R: Read + Seek>(
    reader: &mut R,
) -> Result<(Vec<u8>, Vec<u8>, u64), DecodeError> {
    let head = read_next_ogg_packet(reader)?
        .ok_or_else(|| DecodeError::UnexpectedEof("Missing OpusHead packet".to_string()))?;
    let tags = read_next_ogg_packet(reader)?
        .ok_or_else(|| DecodeError::UnexpectedEof("Missing OpusTags packet".to_string()))?;
    let audio_start = reader.stream_position()?;
    Ok((head, tags, audio_start))
}

/// Reads the next logical packet from an Ogg bitstream (RFC 3533).
fn read_next_ogg_packet<R: Read>(reader: &mut R) -> Result<Option<Vec<u8>>, DecodeError> {
    let mut packet = Vec::new();

    loop {
        let mut magic = [0u8; 4];
        match reader.read_exact(&mut magic) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                if packet.is_empty() {
                    return Ok(None);
                } else {
                    return Ok(Some(packet));
                }
            }
            Err(e) => return Err(DecodeError::IoError(e.to_string())),
        }

        if &magic != b"OggS" {
            return Err(DecodeError::InvalidHeader(
                "Invalid Ogg page magic".to_string(),
            ));
        }

        let mut header = [0u8; 23];
        reader.read_exact(&mut header)?;

        let num_segments = header[22] as usize;
        let mut segment_table = vec![0u8; num_segments];
        reader.read_exact(&mut segment_table)?;

        for &seg_len in &segment_table {
            let mut seg_data = vec![0u8; seg_len as usize];
            reader.read_exact(&mut seg_data)?;
            packet.extend_from_slice(&seg_data);

            if seg_len < 255 {
                // Packet boundary
                return Ok(Some(packet));
            }
        }
    }
}

/// Parses standard Vorbis comments inside `OpusTags` packet into `ContainerMetadata`.
fn parse_opus_tags(mut data: &[u8], metadata: &mut ContainerMetadata) {
    if data.len() < 4 {
        return;
    }
    let vendor_len = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
    data = &data[4..];
    if data.len() < vendor_len + 4 {
        return;
    }
    let vendor = String::from_utf8_lossy(&data[..vendor_len]).to_string();
    metadata.software = Some(vendor);
    data = &data[vendor_len..];

    let count = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
    data = &data[4..];

    for _ in 0..count {
        if data.len() < 4 {
            break;
        }
        let tag_len = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
        data = &data[4..];
        if data.len() < tag_len {
            break;
        }
        let tag_str = String::from_utf8_lossy(&data[..tag_len]).to_string();
        data = &data[tag_len..];

        if let Some((k, v)) = tag_str.split_once('=') {
            let key = k.to_uppercase();
            let val = v.to_string();
            match key.as_str() {
                "TITLE" => metadata.title = Some(val),
                "ARTIST" => metadata.artist = Some(val),
                "ALBUM" => metadata.album = Some(val),
                "ALBUMARTIST" => metadata.album_artist = Some(val),
                "GENRE" => metadata.genre = Some(val),
                "DATE" => metadata.date = Some(val),
                "COMMENT" => metadata.comment = Some(val),
                "COPYRIGHT" => metadata.copyright = Some(val),
                "TRACKNUMBER" => {
                    if let Ok(n) = val.parse::<u32>() {
                        metadata.track_number = Some(n);
                    }
                }
                _ => {}
            }
        }
    }
}
