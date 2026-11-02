//! Integration tests for WAV, RF64, and Broadcast Wave Format (BWF) decoding and encoding.
#![allow(clippy::needless_range_loop)]

use flux_engine::decoder::{
    decode_buffer, open_decoder, AudioDecoder, BwfInfo, ContainerFormat, SampleFormat, WavDecoder,
    WavEncoding, WavWriter,
};
use proptest::prelude::*;
use std::io::Cursor;

#[test]
fn test_wav_pcm16_stereo_roundtrip() {
    let ch0 = vec![0.0, 0.5, -0.5, 0.999, -1.0];
    let ch1 = vec![0.1, -0.2, 0.3, -0.4, 0.0];
    let original = vec![ch0.as_slice(), ch1.as_slice()];

    let encoded = WavWriter::encode(&original, 48000, WavEncoding::Pcm16, None, false).unwrap();
    let mut decoder = WavDecoder::new(Cursor::new(&encoded)).unwrap();

    let info = decoder.info();
    assert_eq!(info.container_format, ContainerFormat::Wav);
    assert_eq!(info.sample_format, SampleFormat::PcmS16Le);
    assert_eq!(info.channels, 2);
    assert_eq!(info.sample_rate, 48000);
    assert_eq!(info.total_frames, 5);

    let decoded = decoder.decode_all().unwrap();
    assert_eq!(decoded.num_channels(), 2);
    assert_eq!(decoded.num_frames(), 5);

    // 16-bit PCM quantization error bound <= 1/32768
    for ch in 0..2 {
        for frame in 0..5 {
            let diff = (original[ch][frame] - decoded.channel(ch)[frame]).abs();
            assert!(diff <= 1.0 / 32767.0 + 1e-6);
        }
    }
}

#[test]
fn test_wav_pcm24_roundtrip() {
    let ch0 = vec![0.0, 0.123456, -0.654321, 0.999999, -1.0];
    let original = vec![ch0.as_slice()];

    let encoded = WavWriter::encode(&original, 96000, WavEncoding::Pcm24, None, false).unwrap();
    let mut decoder = WavDecoder::new(Cursor::new(&encoded)).unwrap();

    let info = decoder.info();
    assert_eq!(info.sample_format, SampleFormat::PcmS24Le);
    assert_eq!(info.channels, 1);
    assert_eq!(info.sample_rate, 96000);
    assert_eq!(info.total_frames, 5);

    let decoded = decoder.decode_all().unwrap();
    // 24-bit PCM quantization error bound <= 1/8388608
    for frame in 0..5 {
        let diff = (original[0][frame] - decoded.channel(0)[frame]).abs();
        assert!(diff <= 1.0 / 8388607.0 + 1e-7);
    }
}

#[test]
fn test_wav_pcm32_roundtrip() {
    let ch0 = vec![0.0, 0.12345678, -0.8765432, 0.5];
    let original = vec![ch0.as_slice()];

    let encoded = WavWriter::encode(&original, 44100, WavEncoding::Pcm32, None, false).unwrap();
    let mut decoder = WavDecoder::new(Cursor::new(&encoded)).unwrap();

    let info = decoder.info();
    assert_eq!(info.sample_format, SampleFormat::PcmS32Le);
    assert_eq!(info.channels, 1);
    assert_eq!(info.sample_rate, 44100);

    let decoded = decoder.decode_all().unwrap();
    for frame in 0..4 {
        let diff = (original[0][frame] - decoded.channel(0)[frame]).abs();
        assert!(diff <= 1e-6);
    }
}

#[test]
fn test_wav_float32_exact_roundtrip() {
    let ch0 = vec![0.0f32, 0.125f32, -0.875f32, 1.5f32, -2.5f32];
    let original = vec![ch0.as_slice()];

    let encoded = WavWriter::encode(&original, 192000, WavEncoding::Float32, None, false).unwrap();
    let decoded = decode_buffer(&encoded).unwrap();

    assert_eq!(decoded.sample_rate, 192000);
    assert_eq!(decoded.num_frames(), 5);
    // 32-bit float round-trip must be bit-exact!
    for frame in 0..5 {
        assert_eq!(decoded.channel(0)[frame], original[0][frame]);
    }
}

#[test]
fn test_wav_bwf_bext_chunk() {
    let ch0 = vec![0.0, 0.5, -0.5];
    let original = vec![ch0.as_slice()];

    let bwf = BwfInfo {
        description: "Take 1 Dialogue Scene 4".to_string(),
        originator: "Flux Audio Workstation".to_string(),
        originator_reference: "FLUX-REF-2026-001".to_string(),
        origination_date: "2026-11-03".to_string(),
        origination_time: "03:45:00".to_string(),
        time_reference: 48000 * 3600, // 1 hour timecode
        version: 1,
        umid: Some([0xAA; 64]),
        loudness_value: Some(-2300),     // -23.00 LUFS
        loudness_range: Some(600),       // 6.00 LU
        max_true_peak_level: Some(-100), // -1.00 dBTP
        coding_history: "A=PCM,F=48000,W=24,M=mono,T=Flux".to_string(),
    };

    let encoded =
        WavWriter::encode(&original, 48000, WavEncoding::Pcm24, Some(&bwf), false).unwrap();
    let decoder = WavDecoder::new(Cursor::new(&encoded)).unwrap();

    let info = decoder.info();
    assert_eq!(info.container_format, ContainerFormat::Bwf);
    let parsed_bwf = info.bwf_info.as_ref().unwrap();

    assert_eq!(parsed_bwf.description, bwf.description);
    assert_eq!(parsed_bwf.originator, bwf.originator);
    assert_eq!(parsed_bwf.originator_reference, bwf.originator_reference);
    assert_eq!(parsed_bwf.origination_date, bwf.origination_date);
    assert_eq!(parsed_bwf.origination_time, bwf.origination_time);
    assert_eq!(parsed_bwf.time_reference, bwf.time_reference);
    assert_eq!(parsed_bwf.loudness_value, Some(-2300));
    assert_eq!(parsed_bwf.loudness_range, Some(600));
    assert_eq!(parsed_bwf.max_true_peak_level, Some(-100));
    assert_eq!(parsed_bwf.coding_history, bwf.coding_history);
}

#[test]
fn test_rf64_ds64_parsing() {
    let ch0 = vec![0.25, -0.25, 0.75, -0.75];
    let original = vec![ch0.as_slice()];

    let encoded = WavWriter::encode(&original, 48000, WavEncoding::Pcm16, None, true).unwrap();
    let decoder = WavDecoder::new(Cursor::new(&encoded)).unwrap();

    let info = decoder.info();
    assert_eq!(info.container_format, ContainerFormat::Rf64);
    assert_eq!(info.total_frames, 4);
    assert_eq!(info.sample_rate, 48000);
}

#[test]
fn test_wav_streaming_chunks_and_seek() {
    // Generate 1000 frames of a test signal
    let num_frames = 1000;
    let mut ch0 = Vec::with_capacity(num_frames);
    for i in 0..num_frames {
        ch0.push((i as f32 / 100.0).sin());
    }
    let original = vec![ch0.as_slice()];

    let encoded = WavWriter::encode(&original, 48000, WavEncoding::Float32, None, false).unwrap();
    let mut decoder = WavDecoder::new(Cursor::new(&encoded)).unwrap();

    // Decode in small chunks of 128 frames
    let mut decoded_frames = 0;
    let mut streamed_samples = Vec::new();
    while let Some(chunk) = decoder.decode_chunk(128).unwrap() {
        decoded_frames += chunk.num_frames();
        streamed_samples.extend_from_slice(chunk.channel(0));
    }

    assert_eq!(decoded_frames, num_frames);
    assert_eq!(streamed_samples.len(), num_frames);
    for frame in 0..num_frames {
        assert_eq!(streamed_samples[frame], ch0[frame]);
    }

    // Test sample-accurate seeking
    let seek_targets = [0, 50, 128, 500, 999];
    for &target in &seek_targets {
        decoder.seek_frame(target).unwrap();
        assert_eq!(decoder.current_frame(), target);
        let chunk = decoder.decode_chunk(1).unwrap().unwrap();
        assert_eq!(chunk.channel(0)[0], ch0[target as usize]);
    }
}

#[test]
fn test_corpus_synthetic_fixtures_decode() {
    let corpus_dir = std::path::Path::new("../../corpus/synthetic");
    let fallback_dir = std::path::Path::new("corpus/synthetic");
    let actual_dir = if corpus_dir.exists() {
        corpus_dir
    } else {
        fallback_dir
    };

    assert!(actual_dir.exists(), "Corpus directory exists");
    for entry in std::fs::read_dir(actual_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("wav") {
            let file = std::fs::File::open(&path).unwrap();
            let mut decoder = open_decoder(file).unwrap();
            let (sample_rate, channels, total_frames) = {
                let info = decoder.info();
                (info.sample_rate, info.channels, info.total_frames)
            };
            assert_eq!(sample_rate, 48000);
            assert!(channels >= 1);
            let decoded = decoder.decode_all().unwrap();
            assert_eq!(decoded.num_frames(), total_frames as usize);
            println!(
                "Successfully decoded corpus fixture: {:?}",
                path.file_name().unwrap()
            );
        }
    }
}

#[test]
fn test_wav_malformed_inputs() {
    // Empty data
    assert!(WavDecoder::new(Cursor::new(vec![])).is_err());

    // Truncated header
    assert!(WavDecoder::new(Cursor::new(b"RIFF\x00\x00\x00\x00WAV".to_vec())).is_err());

    // Non-WAVE magic
    assert!(WavDecoder::new(Cursor::new(
        b"RIFF\x10\x00\x00\x00AVI \x00\x00\x00\x00".to_vec()
    ))
    .is_err());

    // Seek out of bounds
    let ch0 = vec![0.0, 0.5];
    let encoded =
        WavWriter::encode(&[ch0.as_slice()], 48000, WavEncoding::Float32, None, false).unwrap();
    let mut decoder = WavDecoder::new(Cursor::new(&encoded)).unwrap();
    assert!(decoder.seek_frame(10).is_err());
}

proptest! {
    #[test]
    fn prop_wav_float32_lossless_roundtrip(
        signal in proptest::collection::vec(-10.0f32..10.0f32, 1..256)
    ) {
        let original = vec![signal.as_slice()];
        let encoded = WavWriter::encode(&original, 48000, WavEncoding::Float32, None, false).unwrap();
        let decoded = decode_buffer(&encoded).unwrap();

        prop_assert_eq!(decoded.num_frames(), signal.len());
        for (a, b) in signal.iter().zip(decoded.channel(0).iter()) {
            prop_assert_eq!(a, b);
        }
    }
}
