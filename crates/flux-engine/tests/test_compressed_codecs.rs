//! Comprehensive integration tests for lossy compressed decoders (P1.1-c).

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
fn test_decode_mp3_cbr_and_id3v2() {
    let path = fixture_path("test_cbr.mp3");
    assert!(path.exists(), "MP3 CBR fixture must exist");

    let file = File::open(&path).expect("Failed to open MP3 CBR file");
    let mut decoder = open_decoder(file).expect("Failed to create MP3 decoder");

    let info = decoder.info();
    assert_eq!(info.container_format, ContainerFormat::Mp3);
    assert_eq!(info.sample_rate, 48000);
    assert_eq!(info.channels, 2);

    // Verify ID3v2 metadata
    let meta = &info.metadata;
    assert_eq!(meta.title.as_deref(), Some("Flux MP3 Test"));
    assert_eq!(meta.artist.as_deref(), Some("Flux Engineer"));
    assert_eq!(meta.album.as_deref(), Some("Test Suite"));
    assert_eq!(meta.track_number, Some(2));
    assert_eq!(meta.genre.as_deref(), Some("DSP Test"));

    // Verify embedded cover artwork
    assert!(
        meta.cover_art.is_some(),
        "Cover art must be extracted from ID3v2 APIC"
    );
    let art = meta.cover_art.as_ref().unwrap();
    assert!(!art.data.is_empty(), "Cover art bytes must be present");
    assert!(art.mime_type.contains("png") || art.mime_type.contains("image"));

    // Verify audio stream decoding
    let audio = decoder.decode_all().expect("decode_all failed for MP3");
    assert_eq!(audio.channels.len(), 2);
    assert!(audio.num_frames() > 10000);
}

#[test]
fn test_decode_mp3_vbr_and_seek() {
    let path = fixture_path("test_vbr.mp3");
    assert!(path.exists(), "MP3 VBR fixture must exist");

    let file = File::open(&path).expect("Failed to open MP3 VBR file");
    let mut decoder = open_decoder(file).expect("Failed to create MP3 VBR decoder");

    let info = decoder.info();
    assert_eq!(info.container_format, ContainerFormat::Mp3);
    assert_eq!(info.metadata.title.as_deref(), Some("VBR Track"));

    // Seek test in VBR
    decoder.seek_frame(5000).expect("Seek in VBR MP3 failed");
    assert_eq!(decoder.current_frame(), 5000);

    let chunk = decoder
        .decode_chunk(1024)
        .expect("Decode after VBR seek failed")
        .expect("Non-empty chunk");
    assert_eq!(chunk.num_frames(), 1024);
    assert_eq!(decoder.current_frame(), 6024);
}

#[test]
fn test_decode_aac_adts() {
    let path = fixture_path("test.aac");
    assert!(path.exists(), "AAC fixture must exist");

    let file = File::open(&path).expect("Failed to open AAC file");
    let mut decoder = open_decoder(file).expect("Failed to create AAC decoder");

    let info = decoder.info();
    assert_eq!(info.container_format, ContainerFormat::Aac);
    assert_eq!(info.sample_rate, 48000);
    assert_eq!(info.channels, 2);

    let audio = decoder.decode_all().expect("decode_all failed for AAC");
    assert_eq!(audio.channels.len(), 2);
    assert!(audio.num_frames() > 10000);
}

#[test]
fn test_decode_ogg_vorbis() {
    let path = fixture_path("test_vorbis.ogg");
    assert!(path.exists(), "OGG Vorbis fixture must exist");

    let file = File::open(&path).expect("Failed to open Vorbis file");
    let mut decoder = open_decoder(file).expect("Failed to create Vorbis decoder");

    let info = decoder.info();
    assert_eq!(info.container_format, ContainerFormat::Vorbis);
    assert_eq!(info.sample_rate, 48000);
    assert_eq!(info.channels, 2);
    assert_eq!(info.metadata.title.as_deref(), Some("Vorbis Stream"));

    let audio = decoder.decode_all().expect("decode_all failed for Vorbis");
    assert_eq!(audio.channels.len(), 2);
    assert!(audio.num_frames() > 10000);
}

#[test]
fn test_decode_ogg_opus_pure_rust() {
    let path = fixture_path("test_opus.opus");
    assert!(path.exists(), "OGG Opus fixture must exist");

    let file = File::open(&path).expect("Failed to open Opus file");
    let mut decoder = open_decoder(file).expect("Failed to create Opus decoder");

    let info = decoder.info();
    assert_eq!(info.container_format, ContainerFormat::Opus);
    assert_eq!(info.sample_rate, 48000);
    assert_eq!(info.channels, 2);

    // Verify OpusTags comments
    let meta = &info.metadata;
    assert_eq!(meta.title.as_deref(), Some("Opus Stream"));
    assert_eq!(meta.artist.as_deref(), Some("RFC 6716 Test"));

    // Verify preskip priming information
    assert!(meta.gapless_info.is_some());
    let gapless = meta.gapless_info.unwrap();
    assert!(gapless.encoder_delay > 0, "Opus preskip must be > 0");

    // Decode and verify samples
    let mut total_frames = 0;
    while let Some(chunk) = decoder.decode_chunk(960).expect("Opus decode_chunk failed") {
        total_frames += chunk.num_frames();
        assert_eq!(chunk.channels.len(), 2);
        for ch in &chunk.channels {
            for &s in ch {
                assert!((-1.5..=1.5).contains(&s));
            }
        }
    }
    assert!(
        total_frames > 10000,
        "Decoded frames {total_frames} must be non-trivial"
    );

    // Test sample-accurate seeking in Opus
    let file2 = File::open(&path).expect("Failed to open Opus file");
    let mut decoder2 = open_decoder(file2).expect("Failed to create Opus decoder");
    decoder2.seek_frame(4800).expect("Seek in Opus failed");
    assert_eq!(decoder2.current_frame(), 4800);
    let chunk2 = decoder2
        .decode_chunk(960)
        .expect("Decode after seek failed")
        .expect("Expected chunk");
    assert_eq!(chunk2.num_frames(), 960);
    assert_eq!(decoder2.current_frame(), 4800 + 960);
}
