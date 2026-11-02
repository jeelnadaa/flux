//! Integration tests for seek index and rich metadata parser (P1.1-e).

use flux_engine::decoder::{ContainerMetadata, CoverArt, GaplessInfo, SeekIndex};

#[test]
fn test_seek_index_binary_search_sync_lookup() {
    let mut index = SeekIndex::new();
    assert!(index.is_empty());
    assert_eq!(index.len(), 0);

    // Populate sparse keyframe sync points
    index.add_entry(0, 1024, 0);
    index.add_entry(1152, 2048, 1152);
    index.add_entry(2304, 3072, 2304);
    index.add_entry(3456, 4096, 3456);
    index.add_entry(4608, 5120, 4608);

    assert_eq!(index.len(), 5);
    assert!(!index.is_empty());

    // Query exact point
    let entry = index.find_sync_frame(2304).expect("Should find sync entry");
    assert_eq!(entry.frame_index, 2304);
    assert_eq!(entry.byte_offset, 3072);

    // Query in between points (must return greatest frame <= target)
    let entry_between = index.find_sync_frame(3000).expect("Should find entry");
    assert_eq!(entry_between.frame_index, 2304);
    assert_eq!(entry_between.byte_offset, 3072);

    let entry_near_end = index.find_sync_frame(5000).expect("Should find entry");
    assert_eq!(entry_near_end.frame_index, 4608);

    // Query before first point
    assert!(index.find_sync_frame(0).is_some());

    // Monotonic addition rule
    index.add_entry(2000, 9999, 0); // out of order, must be ignored
    assert_eq!(index.len(), 5);
}

#[test]
fn test_metadata_merging_and_fields() {
    let mut primary = ContainerMetadata {
        title: Some("Primary Title".to_string()),
        artist: None,
        album: Some("Primary Album".to_string()),
        album_artist: None,
        composer: None,
        date: Some("2026".to_string()),
        genre: None,
        track_number: Some(1),
        track_total: Some(10),
        disc_number: None,
        disc_total: None,
        comment: None,
        copyright: None,
        software: Some("Flux Audio v0.1".to_string()),
        gapless_info: Some(GaplessInfo {
            encoder_delay: 576,
            encoder_padding: 1100,
            original_samples: Some(24000),
        }),
        cover_art: None,
    };

    let secondary = ContainerMetadata {
        title: Some("Overridden Title".to_string()),
        artist: Some("Secondary Artist".to_string()),
        album: None,
        album_artist: Some("Secondary Album Artist".to_string()),
        composer: Some("Beethoven".to_string()),
        date: None,
        genre: Some("Classical".to_string()),
        track_number: None,
        track_total: None,
        disc_number: Some(1),
        disc_total: Some(1),
        comment: Some("Test Comment".to_string()),
        copyright: Some("2026 Flux Team".to_string()),
        software: None,
        gapless_info: None,
        cover_art: Some(CoverArt {
            mime_type: "image/jpeg".to_string(),
            picture_type: "CoverFront".to_string(),
            description: Some("Front Cover".to_string()),
            data: vec![0xFF, 0xD8, 0xFF, 0xE0],
        }),
    };

    primary.merge(secondary);

    // Existing fields preserved
    assert_eq!(primary.title.as_deref(), Some("Primary Title"));
    assert_eq!(primary.album.as_deref(), Some("Primary Album"));
    assert_eq!(primary.date.as_deref(), Some("2026"));
    assert_eq!(primary.track_number, Some(1));
    assert_eq!(primary.track_total, Some(10));
    assert_eq!(primary.software.as_deref(), Some("Flux Audio v0.1"));

    // Missing fields merged
    assert_eq!(primary.artist.as_deref(), Some("Secondary Artist"));
    assert_eq!(
        primary.album_artist.as_deref(),
        Some("Secondary Album Artist")
    );
    assert_eq!(primary.composer.as_deref(), Some("Beethoven"));
    assert_eq!(primary.genre.as_deref(), Some("Classical"));
    assert_eq!(primary.disc_number, Some(1));
    assert_eq!(primary.disc_total, Some(1));
    assert_eq!(primary.comment.as_deref(), Some("Test Comment"));
    assert_eq!(primary.copyright.as_deref(), Some("2026 Flux Team"));
    assert!(primary.cover_art.is_some());
    assert_eq!(
        primary.cover_art.as_ref().unwrap().picture_type,
        "CoverFront"
    );
    assert_eq!(primary.gapless_info.unwrap().encoder_delay, 576);
}
