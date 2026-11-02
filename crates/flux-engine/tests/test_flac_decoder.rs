//! Comprehensive integration tests for FLAC decoder (P1.1-b).

use flux_engine::decoder::{open_decoder, ContainerFormat};
use std::fs::File;
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
        .join("compressed")
        .join(name)
}

#[test]
fn test_decode_flac_16bit() {
    let path = fixture_path("sine_16.flac");
    assert!(path.exists(), "FLAC 16 fixture must exist");

    let file = File::open(&path).expect("Failed to open FLAC 16 file");
    let mut decoder = open_decoder(file).expect("Failed to create FLAC decoder");

    let info = decoder.info();
    assert_eq!(info.container_format, ContainerFormat::Flac);
    assert_eq!(info.sample_rate, 48000);
    assert_eq!(info.channels, 2);
    assert_eq!(info.bits_per_sample, 16);

    let mut total_decoded_frames = 0;
    while let Some(chunk) = decoder.decode_chunk(1024).expect("Decode chunk failed") {
        total_decoded_frames += chunk.num_frames();
        assert_eq!(chunk.channels.len(), 2);
        for ch in &chunk.channels {
            for &sample in ch {
                assert!((-1.05..=1.05).contains(&sample));
            }
        }
    }

    assert!(
        (24000 - 50..=24000 + 50).contains(&total_decoded_frames),
        "Decoded frame count {total_decoded_frames} must match ~0.5s at 48kHz"
    );
}

#[test]
fn test_decode_flac_24bit() {
    let path = fixture_path("sine_24.flac");
    assert!(path.exists(), "FLAC 24 fixture must exist");

    let file = File::open(&path).expect("Failed to open FLAC 24 file");
    let mut decoder = open_decoder(file).expect("Failed to create FLAC decoder");

    let info = decoder.info();
    assert_eq!(info.container_format, ContainerFormat::Flac);
    assert_eq!(info.sample_rate, 48000);
    assert_eq!(info.channels, 2);
    assert_eq!(info.bits_per_sample, 24);

    let audio = decoder.decode_all().expect("decode_all failed for FLAC 24");
    assert_eq!(audio.channels.len(), 2);
    assert!(audio.num_frames() >= 23900);
}

#[test]
fn test_flac_vorbis_comments_metadata() {
    let path = fixture_path("sine_16_tagged.flac");
    assert!(path.exists(), "Tagged FLAC fixture must exist");

    let file = File::open(&path).expect("Failed to open tagged FLAC file");
    let decoder = open_decoder(file).expect("Failed to create FLAC decoder");

    let meta = &decoder.info().metadata;
    assert_eq!(meta.title.as_deref(), Some("Flux FLAC Test"));
    assert_eq!(meta.artist.as_deref(), Some("Flux Audio"));
    assert_eq!(meta.album.as_deref(), Some("Lossless Suite"));
    assert_eq!(meta.track_number, Some(1));
}

#[test]
fn test_flac_sample_accurate_seek() {
    let path = fixture_path("sine_16.flac");
    let file = File::open(&path).expect("Failed to open FLAC file");
    let mut decoder = open_decoder(file).expect("Failed to create FLAC decoder");

    // Seek to frame 12000 (~0.25s)
    decoder.seek_frame(12000).expect("Seek to 12000 failed");
    assert_eq!(decoder.current_frame(), 12000);

    let chunk = decoder
        .decode_chunk(1024)
        .expect("Decode after seek failed")
        .expect("Chunk should not be empty");
    assert_eq!(chunk.num_frames(), 1024);
    assert_eq!(decoder.current_frame(), 12000 + 1024);

    // Seek backwards to frame 2000
    decoder.seek_frame(2000).expect("Seek back to 2000 failed");
    assert_eq!(decoder.current_frame(), 2000);

    let chunk2 = decoder
        .decode_chunk(512)
        .expect("Decode after rewind failed")
        .expect("Chunk should not be empty");
    assert_eq!(chunk2.num_frames(), 512);
    assert_eq!(decoder.current_frame(), 2512);
}
