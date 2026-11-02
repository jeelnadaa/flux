//! Symphonia-based streaming decoder for FLAC, MP3, AAC, OGG Vorbis, OGG Opus, and video containers.

use super::metadata::{ContainerMetadata, CoverArt, GaplessInfo};
use super::seek_index::SeekIndex;
use super::{AudioBuffer, AudioDecoder, AudioFileInfo, ContainerFormat, DecodeError, SampleFormat};
use std::io::{Read, Seek, SeekFrom};
use symphonia::core::audio::{
    AudioBuffer as SymphoniaAudioBuffer, AudioMut, AudioSpec, GenericAudioBufferRef,
};
use symphonia::core::codecs::audio::{
    well_known, AudioCodecParameters, AudioDecoder as SymphoniaAudioDecoder, AudioDecoderOptions,
    FinalizeResult, CODEC_ID_NULL_AUDIO,
};
use symphonia::core::codecs::CodecInfo;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatReader, SeekMode, SeekTo};
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::{StandardTag, StandardVisualKey};
use symphonia::core::packet::PacketRef;
use symphonia::core::units::Timestamp;

/// Adaptor that implements Symphonia `MediaSource` for any standard `Read + Seek` stream.
pub struct ReadSeekMediaSource<R: Read + Seek + Send + Sync> {
    inner: R,
    len: Option<u64>,
}

impl<R: Read + Seek + Send + Sync> ReadSeekMediaSource<R> {
    /// Creates a new `ReadSeekMediaSource`.
    pub fn new(mut inner: R) -> Self {
        let len = inner.seek(SeekFrom::End(0)).ok();
        let _ = inner.seek(SeekFrom::Start(0));
        Self { inner, len }
    }
}

impl<R: Read + Seek + Send + Sync> Read for ReadSeekMediaSource<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<R: Read + Seek + Send + Sync> Seek for ReadSeekMediaSource<R> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

impl<R: Read + Seek + Send + Sync + 'static> MediaSource for ReadSeekMediaSource<R> {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        self.len
    }
}

/// Pure-Rust RFC 6716 Opus audio decoder plugging into Symphonia's demuxer pipeline.
pub struct OpusAudioDecoder {
    params: AudioCodecParameters,
    inner: opus1::OpusDecoder,
    buf: SymphoniaAudioBuffer<f32>,
    raw_pcm: Vec<f32>,
    num_channels: usize,
}

impl OpusAudioDecoder {
    /// Creates a new `OpusAudioDecoder` from audio codec parameters.
    pub fn try_new(params: &AudioCodecParameters) -> Result<Self, DecodeError> {
        let channels_count = params.channels.as_ref().map_or(2, |c| c.count());
        let opus_channels = if channels_count == 1 {
            opus1::Channels::Mono
        } else {
            opus1::Channels::Stereo
        };

        let inner =
            opus1::OpusDecoder::new(opus1::SampleRate::Hz48000, opus_channels).map_err(|e| {
                DecodeError::UnsupportedFormat(format!("Failed to create Opus decoder: {e:?}"))
            })?;

        let channels =
            params
                .channels
                .clone()
                .unwrap_or(symphonia::core::audio::Channels::Discrete(
                    channels_count as u16,
                ));
        let spec = AudioSpec::new(48000, channels);

        let buf = SymphoniaAudioBuffer::<f32>::new(spec, 5760);

        Ok(Self {
            params: params.clone(),
            inner,
            buf,
            raw_pcm: vec![0.0f32; 5760 * channels_count],
            num_channels: channels_count,
        })
    }
}

impl SymphoniaAudioDecoder for OpusAudioDecoder {
    fn reset(&mut self) {
        self.inner.reset_state();
        self.buf.clear();
    }

    fn codec_info(&self) -> &CodecInfo {
        static OPUS_CODEC_INFO: CodecInfo = CodecInfo {
            short_name: "opus",
            long_name: "Opus Interactive Audio Codec",
            profiles: &[],
        };
        &OPUS_CODEC_INFO
    }

    fn codec_params(&self) -> &AudioCodecParameters {
        &self.params
    }

    fn decode_ref(
        &mut self,
        packet: &PacketRef<'_>,
    ) -> symphonia::core::errors::Result<GenericAudioBufferRef<'_>> {
        self.buf.clear();
        let num_samples = match self.inner.decode(packet.data, 5760, &mut self.raw_pcm) {
            Ok(s) => s,
            Err(_) => {
                return Ok(GenericAudioBufferRef::F32(&self.buf));
            }
        };

        if num_samples == 0 {
            return Ok(GenericAudioBufferRef::F32(&self.buf));
        }

        self.buf.render_uninit(Some(num_samples));

        for ch in 0..self.num_channels {
            if let Some(plane) = self.buf.plane_mut(ch) {
                for (frame, item) in plane.iter_mut().enumerate().take(num_samples) {
                    *item = self.raw_pcm[frame * self.num_channels + ch];
                }
            }
        }

        Ok(GenericAudioBufferRef::F32(&self.buf))
    }

    fn finalize(&mut self) -> FinalizeResult {
        Default::default()
    }

    fn last_decoded(&self) -> GenericAudioBufferRef<'_> {
        GenericAudioBufferRef::F32(&self.buf)
    }
}

/// Multi-format streaming decoder powered by Symphonia.
pub struct SymphoniaDecoder {
    format_reader: Box<dyn FormatReader>,
    decoder: Box<dyn SymphoniaAudioDecoder>,
    track_id: u32,
    info: AudioFileInfo,
    current_frame: u64,
    seek_index: SeekIndex,
    gapless_delay: u32,
    gapless_padding: u32,
    delay_skipped: u32,
    interleaved_buf: Vec<f32>,
    leftover_samples: Vec<Vec<f32>>,
}

impl SymphoniaDecoder {
    /// Opens and probes an audio stream from any seekable reader.
    pub fn new<R: Read + Seek + Send + Sync + 'static>(
        reader: R,
        hint_ext: Option<&str>,
    ) -> Result<Self, DecodeError> {
        let source = ReadSeekMediaSource::new(reader);
        let mss = MediaSourceStream::new(Box::new(source), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = hint_ext {
            hint.with_extension(ext);
        }

        let mut format_reader = symphonia::default::get_probe()
            .probe(&hint, mss, Default::default(), Default::default())
            .map_err(|e| {
                DecodeError::InvalidHeader(format!("Symphonia format probe failed: {e}"))
            })?;

        // Find the primary audio track
        let track = format_reader
            .tracks()
            .iter()
            .find(|t| {
                t.codec_params
                    .as_ref()
                    .and_then(|p| p.audio())
                    .is_some_and(|ap| ap.codec != CODEC_ID_NULL_AUDIO)
            })
            .ok_or_else(|| {
                DecodeError::UnsupportedFormat("No supported audio tracks found".to_string())
            })?
            .clone();

        let track_id = track.id;
        let audio_params = track
            .codec_params
            .as_ref()
            .and_then(|p| p.audio())
            .ok_or_else(|| {
                DecodeError::UnsupportedFormat("Missing audio parameters".to_string())
            })?;

        let sample_rate = audio_params
            .sample_rate
            .ok_or_else(|| DecodeError::CorruptData("Missing sample rate".to_string()))?;
        let channels = audio_params
            .channels
            .as_ref()
            .map_or(2, |c| c.count() as u16);
        let bits_per_sample = audio_params.bits_per_sample.unwrap_or(16) as u16;

        let delay = track.delay.unwrap_or(0);
        let padding = track.padding.unwrap_or(0);

        let total_frames = track.num_frames.unwrap_or_else(|| {
            if let (Some(dur), Some(tb)) = (track.duration, track.time_base) {
                if let Some(time) = tb.calc_duration(dur) {
                    (time.as_secs_f64() * sample_rate as f64).round() as u64
                } else {
                    0
                }
            } else {
                0
            }
        });

        let duration_seconds = if sample_rate > 0 && total_frames > 0 {
            total_frames as f64 / sample_rate as f64
        } else if let (Some(dur), Some(tb)) = (track.duration, track.time_base) {
            tb.calc_duration(dur).map_or(0.0, |t| t.as_secs_f64())
        } else {
            0.0
        };

        // Determine container format
        let fmt_short = format_reader.format_info().short_name.to_ascii_lowercase();
        let container_format = if fmt_short.contains("mp4") || fmt_short.contains("isomp4") {
            ContainerFormat::Mp4
        } else if fmt_short.contains("mkv")
            || fmt_short.contains("matroska")
            || fmt_short.contains("webm")
        {
            ContainerFormat::Mkv
        } else {
            match audio_params.codec {
                well_known::CODEC_ID_FLAC => ContainerFormat::Flac,
                well_known::CODEC_ID_MP3 => ContainerFormat::Mp3,
                well_known::CODEC_ID_AAC => ContainerFormat::Aac,
                well_known::CODEC_ID_VORBIS => ContainerFormat::Vorbis,
                well_known::CODEC_ID_OPUS => ContainerFormat::Opus,
                _ => ContainerFormat::Flac,
            }
        };

        // Extract metadata and cover art
        let mut metadata = ContainerMetadata::default();
        if delay > 0 || padding > 0 {
            metadata.gapless_info = Some(GaplessInfo {
                encoder_delay: delay,
                encoder_padding: padding,
                original_samples: track.num_frames,
            });
        }

        extract_metadata(&mut *format_reader, &mut metadata);

        let decoder: Box<dyn SymphoniaAudioDecoder> = if audio_params.codec
            == well_known::CODEC_ID_OPUS
        {
            Box::new(OpusAudioDecoder::try_new(audio_params)?)
        } else {
            let decoder_opts = AudioDecoderOptions::default();
            symphonia::default::get_codecs()
                .make_audio_decoder(audio_params, &decoder_opts)
                .map_err(|e| {
                    DecodeError::UnsupportedFormat(format!("Failed to create audio decoder: {e}"))
                })?
        };

        let info = AudioFileInfo {
            container_format,
            sample_format: SampleFormat::Float32Le,
            channels,
            sample_rate,
            bits_per_sample,
            total_frames,
            duration_seconds,
            channel_mask: None,
            metadata,
            bwf_info: None,
        };

        Ok(Self {
            format_reader,
            decoder,
            track_id,
            info,
            current_frame: 0,
            seek_index: SeekIndex::new(),
            gapless_delay: delay,
            gapless_padding: padding,
            delay_skipped: 0,
            interleaved_buf: Vec::new(),
            leftover_samples: vec![Vec::new(); channels as usize],
        })
    }

    /// Returns a reference to the streaming seek index.
    pub fn seek_index(&self) -> &SeekIndex {
        &self.seek_index
    }

    /// Returns the gapless priming and padding information if available.
    pub fn gapless_info(&self) -> Option<&GaplessInfo> {
        self.info.metadata.gapless_info.as_ref()
    }
}

impl AudioDecoder for SymphoniaDecoder {
    fn info(&self) -> &AudioFileInfo {
        &self.info
    }

    fn current_frame(&self) -> u64 {
        self.current_frame
    }

    fn seek_frame(&mut self, frame: u64) -> Result<(), DecodeError> {
        let target_ts = frame + self.gapless_delay as u64;
        let seek_to = SeekTo::Timestamp {
            ts: Timestamp::new(target_ts as i64),
            track_id: self.track_id,
        };

        let sought = self
            .format_reader
            .seek(SeekMode::Accurate, seek_to)
            .map_err(|e| DecodeError::IoError(format!("Seek failed: {e}")))?;

        self.decoder.reset();
        self.leftover_samples.iter_mut().for_each(|ch| ch.clear());

        let actual_ts = sought.actual_ts.get() as u64;
        let frames_to_skip = target_ts.saturating_sub(actual_ts);

        let mut skipped = 0u64;
        while skipped < frames_to_skip {
            let to_read = (frames_to_skip - skipped).min(1024) as usize;
            match self.decode_chunk(to_read)? {
                Some(chunk) => {
                    let n = chunk.num_frames() as u64;
                    if n == 0 {
                        break;
                    }
                    skipped += n;
                }
                None => break,
            }
        }

        self.current_frame = frame;
        Ok(())
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

        loop {
            let packet = match self.format_reader.next_packet() {
                Ok(Some(pkt)) => pkt,
                Ok(None) => break,
                Err(symphonia::core::errors::Error::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                Err(symphonia::core::errors::Error::ResetRequired) => {
                    self.decoder.reset();
                    continue;
                }
                Err(_) => break,
            };

            if packet.track_id != self.track_id {
                continue;
            }

            // Index packet position for sample-accurate seeking
            self.seek_index.add_entry(
                self.current_frame + collected_frames as u64,
                0,
                packet.pts.get() as u64,
            );

            let decoded_audio = match self.decoder.decode(&packet) {
                Ok(ab) => ab,
                Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
                Err(e) => {
                    return Err(DecodeError::CorruptData(format!(
                        "Decode packet error: {e}"
                    )))
                }
            };

            let n_frames = decoded_audio.frames();
            let n_ch = decoded_audio.spec().channels().count();
            if n_frames == 0 || n_ch == 0 {
                continue;
            }

            self.interleaved_buf
                .resize(decoded_audio.samples_interleaved(), 0.0);
            decoded_audio.copy_to_slice_interleaved(&mut self.interleaved_buf[..]);

            // De-interleave into planar channels
            let mut planar = vec![Vec::with_capacity(n_frames); n_ch];
            for frame in 0..n_frames {
                let base = frame * n_ch;
                for (ch, plane) in planar.iter_mut().enumerate().take(n_ch) {
                    plane.push(self.interleaved_buf[base + ch]);
                }
            }

            // Handle gapless delay skipping
            let mut start_frame = 0;
            if self.delay_skipped < self.gapless_delay {
                let remaining_delay = (self.gapless_delay - self.delay_skipped) as usize;
                let skip = remaining_delay.min(n_frames);
                self.delay_skipped += skip as u32;
                start_frame = skip;
            }

            let valid_frames = n_frames - start_frame;
            if valid_frames == 0 {
                continue;
            }

            let needed = max_frames - collected_frames;
            let mut take = valid_frames.min(needed);
            if self.gapless_padding > 0 && self.info.total_frames > 0 {
                let current_total = self.current_frame + collected_frames as u64;
                if current_total + take as u64 > self.info.total_frames {
                    take = (self.info.total_frames.saturating_sub(current_total)) as usize;
                }
            }

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

/// Helper to extract standard ID3, Vorbis, and container tags into `ContainerMetadata`.
fn extract_metadata(reader: &mut dyn FormatReader, meta: &mut ContainerMetadata) {
    if let Some(metadata) = reader.metadata().current() {
        for tag in &metadata.media.tags {
            if let Some(std_tag) = &tag.std {
                match std_tag {
                    StandardTag::TrackTitle(s) => meta.title = Some(s.to_string()),
                    StandardTag::Artist(s) => meta.artist = Some(s.to_string()),
                    StandardTag::Album(s) => meta.album = Some(s.to_string()),
                    StandardTag::AlbumArtist(s) => meta.album_artist = Some(s.to_string()),
                    StandardTag::Composer(s) => meta.composer = Some(s.to_string()),
                    StandardTag::Genre(s) => meta.genre = Some(s.to_string()),
                    StandardTag::ReleaseDate(s) | StandardTag::RecordingDate(s) => {
                        if meta.date.is_none() {
                            meta.date = Some(s.to_string());
                        }
                    }
                    StandardTag::Comment(s) => meta.comment = Some(s.to_string()),
                    StandardTag::Copyright(s) => meta.copyright = Some(s.to_string()),
                    StandardTag::Encoder(s) => meta.software = Some(s.to_string()),
                    StandardTag::TrackNumber(num) => meta.track_number = Some(*num as u32),
                    StandardTag::TrackTotal(num) => meta.track_total = Some(*num as u32),
                    StandardTag::DiscNumber(num) => meta.disc_number = Some(*num as u32),
                    StandardTag::DiscTotal(num) => meta.disc_total = Some(*num as u32),
                    _ => {}
                }
            } else {
                let key_upper = tag.raw.key.to_ascii_uppercase();
                let val_str = tag.raw.value.to_string();
                match key_upper.as_str() {
                    "TITLE" if meta.title.is_none() => meta.title = Some(val_str),
                    "ARTIST" if meta.artist.is_none() => meta.artist = Some(val_str),
                    "ALBUM" if meta.album.is_none() => meta.album = Some(val_str),
                    "GENRE" if meta.genre.is_none() => meta.genre = Some(val_str),
                    "DATE" | "YEAR" if meta.date.is_none() => meta.date = Some(val_str),
                    "COMMENT" if meta.comment.is_none() => meta.comment = Some(val_str),
                    "TRACKNUMBER" if meta.track_number.is_none() => {
                        if let Ok(n) = val_str.parse::<u32>() {
                            meta.track_number = Some(n);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Extract primary cover artwork if present
        if let Some(visual) = metadata.media.visuals.first() {
            let mime = visual
                .media_type
                .clone()
                .unwrap_or_else(|| "image/jpeg".to_string());
            let picture_type = match visual.usage {
                Some(StandardVisualKey::FrontCover) => "CoverFront".to_string(),
                Some(StandardVisualKey::BackCover) => "CoverBack".to_string(),
                Some(StandardVisualKey::LeadArtistPerformerSoloist) => "Artist".to_string(),
                Some(StandardVisualKey::Illustration) => "Illustration".to_string(),
                Some(ref k) => format!("{k:?}"),
                None => "Other".to_string(),
            };
            let description = visual.tags.first().map(|t| t.raw.value.to_string());
            meta.cover_art = Some(CoverArt {
                mime_type: mime,
                picture_type,
                description,
                data: visual.data.to_vec(),
            });
        }
    }
}
