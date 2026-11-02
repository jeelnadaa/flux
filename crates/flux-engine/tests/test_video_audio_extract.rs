//! Integration tests for container & video audio extraction (P1.1-d).

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
fn test_extract_audio_from_mp4_video() {
    let path = fixture_path("test_video.mp4");
    assert!(path.exists(), "MP4 video fixture must exist");

    let file = File::open(&path).expect("Failed to open MP4 video file");
    let mut decoder = open_decoder(file).expect("Failed to create decoder for MP4 video");

    let info = decoder.info();
    assert_eq!(info.container_format, ContainerFormat::Mp4);
    assert_eq!(info.sample_rate, 48000);
    assert_eq!(info.channels, 2);

    let audio = decoder
        .decode_all()
        .expect("decode_all failed on MP4 video audio track");
    assert_eq!(audio.channels.len(), 2);
    assert!(
        audio.num_frames() > 10000,
        "Audio frames extracted from video container must be non-empty"
    );

    // Verify sample amplitudes are valid audio samples
    for ch in &audio.channels {
        for &sample in ch {
            assert!((-1.05..=1.05).contains(&sample));
        }
    }
}

#[test]
fn test_extract_audio_from_mkv_video() {
    let path = fixture_path("test_video.mkv");
    assert!(path.exists(), "MKV video fixture must exist");

    let file = File::open(&path).expect("Failed to open MKV video file");
    let mut decoder = open_decoder(file).expect("Failed to create decoder for MKV video");

    let info = decoder.info();
    assert_eq!(info.container_format, ContainerFormat::Mkv);
    assert_eq!(info.sample_rate, 48000);
    assert_eq!(info.channels, 2);

    let audio = decoder
        .decode_all()
        .expect("decode_all failed on MKV video audio track");
    assert_eq!(audio.channels.len(), 2);
    assert!(
        audio.num_frames() > 10000,
        "Audio frames extracted from MKV container must be non-empty"
    );
}
