//! Integration tests for Apple AIFF and AIFF-C (AIFC) decoding and encoding.
#![allow(clippy::needless_range_loop)]

use flux_engine::decoder::aiff::{decode_extended_80, encode_extended_80};
use flux_engine::decoder::{
    decode_buffer, AiffDecoder, AiffEncoding, AiffWriter, AudioDecoder, ContainerFormat,
    SampleFormat,
};
use proptest::prelude::*;
use std::io::Cursor;

#[test]
fn test_aiff_extended_80_conversion() {
    let test_rates = [
        8000.0, 11025.0, 16000.0, 22050.0, 32000.0, 44100.0, 48000.0, 88200.0, 96000.0, 192000.0,
    ];
    for &rate in &test_rates {
        let bytes = encode_extended_80(rate);
        let recovered = decode_extended_80(&bytes);
        assert!(
            (rate - recovered).abs() < 1e-4,
            "Rate {rate} failed to recover: {recovered}"
        );
    }
}

#[test]
fn test_aiff_pcm16_stereo_roundtrip() {
    let ch0 = vec![0.0, 0.5, -0.5, 0.999, -1.0];
    let ch1 = vec![0.1, -0.2, 0.3, -0.4, 0.0];
    let original = vec![ch0.as_slice(), ch1.as_slice()];

    let encoded = AiffWriter::encode(
        &original,
        44100,
        AiffEncoding::Pcm16,
        Some("Aiff Title"),
        Some("Aiff Artist"),
    )
    .unwrap();
    let mut decoder = AiffDecoder::new(Cursor::new(&encoded)).unwrap();

    let info = decoder.info();
    assert_eq!(info.container_format, ContainerFormat::Aiff);
    assert_eq!(info.sample_format, SampleFormat::PcmS16Be);
    assert_eq!(info.channels, 2);
    assert_eq!(info.sample_rate, 44100);
    assert_eq!(info.total_frames, 5);
    assert_eq!(info.metadata.title.as_deref(), Some("Aiff Title"));
    assert_eq!(info.metadata.artist.as_deref(), Some("Aiff Artist"));

    let decoded = decoder.decode_all().unwrap();
    assert_eq!(decoded.num_channels(), 2);
    assert_eq!(decoded.num_frames(), 5);

    for ch in 0..2 {
        for frame in 0..5 {
            let diff = (original[ch][frame] - decoded.channel(ch)[frame]).abs();
            assert!(diff <= 1.0 / 32767.0 + 1e-6);
        }
    }
}

#[test]
fn test_aiff_pcm24_roundtrip() {
    let ch0 = vec![0.0, 0.25, -0.75, 0.9999];
    let original = vec![ch0.as_slice()];

    let encoded = AiffWriter::encode(&original, 48000, AiffEncoding::Pcm24, None, None).unwrap();
    let mut decoder = AiffDecoder::new(Cursor::new(&encoded)).unwrap();

    let info = decoder.info();
    assert_eq!(info.sample_format, SampleFormat::PcmS24Be);
    assert_eq!(info.channels, 1);
    assert_eq!(info.sample_rate, 48000);

    let decoded = decoder.decode_all().unwrap();
    for frame in 0..4 {
        let diff = (original[0][frame] - decoded.channel(0)[frame]).abs();
        assert!(diff <= 1.0 / 8388607.0 + 1e-7);
    }
}

#[test]
fn test_aifc_float32_exact_roundtrip() {
    let ch0 = vec![0.0f32, 0.125f32, -0.875f32, 1.5f32];
    let original = vec![ch0.as_slice()];

    let encoded = AiffWriter::encode(&original, 96000, AiffEncoding::Float32, None, None).unwrap();
    let decoded = decode_buffer(&encoded).unwrap();

    assert_eq!(decoded.sample_rate, 96000);
    assert_eq!(decoded.num_frames(), 4);
    for frame in 0..4 {
        assert_eq!(decoded.channel(0)[frame], original[0][frame]);
    }
}

#[test]
fn test_aiff_streaming_chunks_and_seek() {
    let num_frames = 500;
    let mut ch0 = Vec::with_capacity(num_frames);
    for i in 0..num_frames {
        ch0.push((i as f32 / 50.0).sin());
    }
    let original = vec![ch0.as_slice()];

    let encoded = AiffWriter::encode(&original, 44100, AiffEncoding::Float32, None, None).unwrap();
    let mut decoder = AiffDecoder::new(Cursor::new(&encoded)).unwrap();

    let mut decoded_frames = 0;
    let mut streamed_samples = Vec::new();
    while let Some(chunk) = decoder.decode_chunk(64).unwrap() {
        decoded_frames += chunk.num_frames();
        streamed_samples.extend_from_slice(chunk.channel(0));
    }

    assert_eq!(decoded_frames, num_frames);
    for frame in 0..num_frames {
        assert_eq!(streamed_samples[frame], ch0[frame]);
    }

    // Test sample-accurate seeking
    let seek_targets = [0, 10, 64, 250, 499];
    for &target in &seek_targets {
        decoder.seek_frame(target).unwrap();
        assert_eq!(decoder.current_frame(), target);
        let chunk = decoder.decode_chunk(1).unwrap().unwrap();
        assert_eq!(chunk.channel(0)[0], ch0[target as usize]);
    }
}

#[test]
fn test_aiff_malformed_inputs() {
    assert!(AiffDecoder::new(Cursor::new(vec![])).is_err());
    assert!(AiffDecoder::new(Cursor::new(b"FORM\x00\x00\x00\x00WAVE".to_vec())).is_err());

    let ch0 = vec![0.0, 0.5];
    let encoded =
        AiffWriter::encode(&[ch0.as_slice()], 44100, AiffEncoding::Float32, None, None).unwrap();
    let mut decoder = AiffDecoder::new(Cursor::new(&encoded)).unwrap();
    assert!(decoder.seek_frame(10).is_err());
}

proptest! {
    #[test]
    fn prop_aiff_float32_lossless_roundtrip(
        signal in proptest::collection::vec(-10.0f32..10.0f32, 1..256)
    ) {
        let original = vec![signal.as_slice()];
        let encoded = AiffWriter::encode(&original, 44100, AiffEncoding::Float32, None, None).unwrap();
        let decoded = decode_buffer(&encoded).unwrap();

        prop_assert_eq!(decoded.num_frames(), signal.len());
        for (a, b) in signal.iter().zip(decoded.channel(0).iter()) {
            prop_assert_eq!(a, b);
        }
    }
}
