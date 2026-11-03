//! Integration tests for export pipeline, codecs, presets, normalization, and converter (P1.4).

use flux_dsp::dither::DitherType;
use flux_engine::decoder::{open_decoder, AudioBuffer, ContainerMetadata, CoverArt};
use flux_engine::export::{
    AacBitrate, ExportChannelMode, ExportError, ExportJob, ExportOptions, ExportPresetId,
    ExportTargetFormat, FlacCompressionLevel, FormatConvertOptions, FormatConverter, Mp3Bitrate,
    NormalizationMode, Normalizer, OpusApplication, VorbisQuality, WavBitsPerSample, WavEncoder,
};
use std::io::Cursor;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

fn create_test_buffer(frames: usize, sample_rate: u32, freq: f32) -> AudioBuffer {
    let mut left = Vec::with_capacity(frames);
    let mut right = Vec::with_capacity(frames);
    let omega = 2.0 * std::f32::consts::PI * freq / (sample_rate as f32);

    for i in 0..frames {
        let s = (i as f32 * omega).sin() * 0.7;
        left.push(s);
        right.push(-s * 0.8);
    }

    AudioBuffer::from_channels(vec![left, right], sample_rate)
}

fn create_test_metadata() -> ContainerMetadata {
    ContainerMetadata {
        title: Some("Flux Master Track".to_string()),
        artist: Some("Antigravity Engine".to_string()),
        album: Some("Flux Suite Vol 1".to_string()),
        date: Some("2026".to_string()),
        genre: Some("Electronic".to_string()),
        track_number: Some(1),
        comment: Some("Mastered with Flux Audio Engine".to_string()),
        cover_art: Some(CoverArt {
            mime_type: "image/png".to_string(),
            picture_type: "Cover (front)".to_string(),
            description: Some("Front Cover".to_string()),
            data: vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A], // PNG magic
        }),
        ..Default::default()
    }
}

#[test]
fn test_wav_export_16_24_32f_and_rf64() {
    let buf = create_test_buffer(4800, 48000, 440.0);
    let meta = create_test_metadata();

    // 1. WAV 16-bit
    let wav16 = WavEncoder::encode(&buf, WavBitsPerSample::Bits16, Some(&meta), false)
        .expect("WAV 16-bit encode failed");
    assert!(wav16.starts_with(b"RIFF"));
    assert!(wav16.windows(4).any(|w| w == b"INAM")); // INFO title chunk

    // Round-trip decode WAV 16-bit
    let mut dec16 = open_decoder(Cursor::new(wav16)).expect("Failed to open WAV 16 decoder");
    let decoded16 = dec16.decode_all().expect("Failed to decode WAV 16");
    assert_eq!(decoded16.channels.len(), 2);
    assert_eq!(decoded16.num_frames(), 4800);
    assert_eq!(dec16.info().sample_rate, 48000);
    assert_eq!(dec16.info().bits_per_sample, 16);

    // 2. WAV 24-bit
    let wav24 = WavEncoder::encode(&buf, WavBitsPerSample::Bits24, Some(&meta), false)
        .expect("WAV 24-bit encode failed");
    assert!(wav24.starts_with(b"RIFF"));

    let mut dec24 = open_decoder(Cursor::new(wav24)).expect("Failed to open WAV 24 decoder");
    let decoded24 = dec24.decode_all().expect("Failed to decode WAV 24");
    assert_eq!(decoded24.channels.len(), 2);
    assert_eq!(decoded24.num_frames(), 4800);
    assert_eq!(dec24.info().bits_per_sample, 24);

    // 3. WAV 32-bit Float
    let wav32 = WavEncoder::encode(&buf, WavBitsPerSample::Float32, Some(&meta), false)
        .expect("WAV 32f encode failed");
    assert!(wav32.starts_with(b"RIFF"));

    let mut dec32 = open_decoder(Cursor::new(wav32)).expect("Failed to open WAV 32f decoder");
    let decoded32 = dec32.decode_all().expect("Failed to decode WAV 32f");
    assert_eq!(decoded32.channels.len(), 2);
    assert_eq!(decoded32.num_frames(), 4800);

    // Float32 round-trip should be bit-exact
    for ch in 0..2 {
        for f in 0..4800 {
            assert_eq!(decoded32.channels[ch][f], buf.channels[ch][f]);
        }
    }

    // 4. RF64 chunk generation
    let rf64_bytes = WavEncoder::encode(&buf, WavBitsPerSample::Float32, Some(&meta), true)
        .expect("RF64 encode failed");
    assert!(rf64_bytes.starts_with(b"RF64"));
    assert!(rf64_bytes.windows(4).any(|w| w == b"ds64"));
}

#[test]
fn test_flac_export_roundtrip_and_metadata() {
    let buf = create_test_buffer(4096 * 2, 48000, 1000.0);
    let meta = create_test_metadata();

    let flac_bytes = flux_engine::export::FlacEncoder::encode(
        &buf,
        16,
        FlacCompressionLevel::Level5,
        Some(&meta),
    )
    .expect("FLAC encode failed");

    // Check FLAC header magic
    assert!(flac_bytes.starts_with(b"fLaC"));

    // Verify Vorbis comments and Picture block exist in stream
    assert!(flac_bytes.windows(6).any(|w| w == b"TITLE="));
    assert!(flac_bytes.windows(7).any(|w| w == b"ARTIST="));
    assert!(flac_bytes.windows(9).any(|w| w == b"image/png"));

    // Decode FLAC stream using symphonia decoder
    let mut dec = open_decoder(Cursor::new(flac_bytes)).expect("Failed to open FLAC decoder");
    let decoded = dec.decode_all().expect("Failed to decode FLAC stream");
    assert_eq!(decoded.channels.len(), 2);
    assert_eq!(decoded.num_frames(), 4096 * 2);
    assert_eq!(dec.info().sample_rate, 48000);

    // Lossless check: 16-bit quantized error <= 1 LSB (1 / 32767)
    for ch in 0..2 {
        for f in 0..1000 {
            let orig = buf.channels[ch][f];
            let recon = decoded.channels[ch][f];
            assert!((orig - recon).abs() < (1.5 / 32767.0));
        }
    }
}

#[test]
fn test_compressed_encoders_mp3_aac_vorbis_opus() {
    let buf = create_test_buffer(2304, 48000, 800.0);
    let meta = create_test_metadata();

    // 1. MP3 Encoder
    let mp3_bytes = flux_engine::export::Mp3Encoder::encode(&buf, Mp3Bitrate::Cbr320, Some(&meta))
        .expect("MP3 encode failed");
    assert!(mp3_bytes.starts_with(b"ID3"));
    assert!(mp3_bytes.windows(4).any(|w| w == b"TIT2"));
    assert!(mp3_bytes.windows(4).any(|w| w == b"APIC"));

    // 2. AAC Encoder (ADTS stream)
    let aac_bytes = flux_engine::export::AacEncoder::encode(&buf, AacBitrate::Kbps192, Some(&meta))
        .expect("AAC encode failed");
    assert_eq!(aac_bytes[0], 0xFF);
    assert_eq!(aac_bytes[1] & 0xF0, 0xF0); // ADTS syncword 0xFFF

    // 3. Ogg Opus Encoder
    let opus_bytes =
        flux_engine::export::OpusEncoder::encode(&buf, OpusApplication::Audio, 128, Some(&meta))
            .expect("Opus encode failed");
    assert!(opus_bytes.starts_with(b"OggS"));
    assert!(opus_bytes.windows(8).any(|w| w == b"OpusHead"));
    assert!(opus_bytes.windows(8).any(|w| w == b"OpusTags"));

    // 4. Ogg Vorbis Encoder
    let vorbis_bytes =
        flux_engine::export::VorbisEncoder::encode(&buf, VorbisQuality::Q6, Some(&meta))
            .expect("Vorbis encode failed");
    assert!(vorbis_bytes.starts_with(b"OggS"));
    assert!(vorbis_bytes.windows(7).any(|w| w == b"\x01vorbis"));
    assert!(vorbis_bytes.windows(7).any(|w| w == b"\x03vorbis"));
}

#[test]
fn test_normalization_peak_and_lufs_with_ceiling() {
    // 1. Peak Normalization
    let mut buf_peak = create_test_buffer(4800, 48000, 440.0);
    Normalizer::apply(&mut buf_peak, NormalizationMode::Peak { target_dbfs: -0.3 });

    let mut measured_peak = 0.0f32;
    for ch in &buf_peak.channels {
        for &s in ch {
            measured_peak = measured_peak.max(s.abs());
        }
    }
    let measured_dbfs = 20.0 * measured_peak.log10();
    assert!(
        (measured_dbfs - (-0.3)).abs() < 0.01,
        "Expected -0.3 dBFS, got {}",
        measured_dbfs
    );

    // 2. LUFS Normalization with True-Peak Ceiling
    let mut buf_lufs = create_test_buffer(48000, 48000, 1000.0); // 1 second 1 kHz tone
    Normalizer::apply(
        &mut buf_lufs,
        NormalizationMode::Lufs {
            target_lufs: -14.0,
            true_peak_ceiling_dbtp: -1.0,
        },
    );

    let lufs_after = Normalizer::measure_integrated_lufs(&buf_lufs, 48000.0);
    let tp_after = Normalizer::measure_max_true_peak(&buf_lufs);

    assert!(
        tp_after <= -0.95,
        "True-peak ceiling violated: got {} dBTP",
        tp_after
    );
    assert!(
        (lufs_after - (-14.0)).abs() < 1.0,
        "Expected ~-14 LUFS, got {}",
        lufs_after
    );
}

#[test]
fn test_export_presets() {
    for preset_id in ExportPresetId::all() {
        let cfg = preset_id.config();
        assert!(!cfg.name.is_empty());
        assert!(!cfg.description.is_empty());
        assert!(!cfg.file_extension.is_empty());

        match preset_id {
            ExportPresetId::Streaming => {
                assert_eq!(
                    cfg.normalization,
                    NormalizationMode::Lufs {
                        target_lufs: -14.0,
                        true_peak_ceiling_dbtp: -1.0
                    }
                );
            }
            ExportPresetId::PodcastStereo => {
                assert_eq!(
                    cfg.normalization,
                    NormalizationMode::Lufs {
                        target_lufs: -16.0,
                        true_peak_ceiling_dbtp: -1.0
                    }
                );
                assert!(!cfg.force_mono);
            }
            ExportPresetId::PodcastMono => {
                assert!(cfg.force_mono);
            }
            ExportPresetId::CdAudio => {
                assert_eq!(cfg.sample_rate, Some(44100));
                assert_eq!(cfg.bit_depth, Some(16));
                assert_eq!(cfg.dither, DitherType::Tpdf);
            }
            ExportPresetId::Archival => {
                assert_eq!(cfg.sample_rate, Some(48000));
                assert_eq!(cfg.bit_depth, Some(24));
                assert_eq!(cfg.file_extension, "flac");
            }
            ExportPresetId::YouTube => {
                assert_eq!(cfg.sample_rate, Some(48000));
            }
        }
    }
}

#[test]
fn test_export_pipeline_region_tail_fade_cancel() {
    let buf = create_test_buffer(4800, 48000, 440.0);
    let mut progress_stages = Vec::new();

    // 1. Region (1000..3000), tail (500 frames), fade-out (200 frames)
    let opts = ExportOptions {
        format: ExportTargetFormat::Wav(WavBitsPerSample::Bits16),
        sample_rate: Some(48000),
        channel_mode: ExportChannelMode::Keep,
        region: Some(1000..3000),
        tail_frames: 500,
        fade_out_frames: 200,
        normalization: NormalizationMode::Peak { target_dbfs: -1.0 },
        dither: DitherType::Tpdf,
        metadata: Some(create_test_metadata()),
        force_rf64: false,
    };

    let wav_bytes = ExportJob::run(&buf, &opts, None, |p| {
        progress_stages.push(p.stage);
    })
    .expect("ExportJob failed");

    assert!(wav_bytes.starts_with(b"RIFF"));
    assert!(progress_stages.contains(&"finished"));

    // Verify length of exported audio: 2000 frames + 500 tail = 2500 frames
    let mut dec = open_decoder(Cursor::new(wav_bytes)).expect("Failed to open decoder");
    let decoded = dec.decode_all().expect("Failed to decode");
    assert_eq!(decoded.num_frames(), 2500);

    // 2. Cancellation Test
    let cancel_flag = Arc::new(AtomicBool::new(true)); // Already cancelled
    let cancel_res = ExportJob::run(&buf, &opts, Some(&cancel_flag), |_| {});
    assert_eq!(cancel_res, Err(ExportError::Cancelled));
}

#[test]
fn test_format_converter_metadata_preservation() {
    let buf = create_test_buffer(2400, 48000, 600.0);
    let meta = create_test_metadata();

    let opts = FormatConvertOptions {
        target_format: ExportTargetFormat::Flac(FlacCompressionLevel::Level5, 16),
        target_sample_rate: Some(44100),
        target_channel_mode: ExportChannelMode::ForceMono,
        override_metadata: None,
        normalization: NormalizationMode::Peak { target_dbfs: -0.5 },
        dither: DitherType::Tpdf,
    };

    let flac_bytes = FormatConverter::convert(&buf, Some(&meta), &opts, None, |_| {})
        .expect("FormatConverter failed");

    assert!(flac_bytes.starts_with(b"fLaC"));
    assert!(flac_bytes.windows(6).any(|w| w == b"TITLE="));
    assert!(flac_bytes.windows(7).any(|w| w == b"ARTIST="));
}
