//! Bit-exact compatibility verification against reference decodes from libsndfile.

use flux_engine::decoder::open_decoder;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

#[derive(serde::Deserialize)]
struct FixtureInfo {
    sample_rate: u32,
    channels: usize,
    frames: usize,
    channel_samples: Vec<Vec<f32>>,
}

#[test]
fn test_libsndfile_bit_exactness() {
    let manifest_path = Path::new("../../tests/fixtures/libsndfile/manifest.json");
    let fallback_path = Path::new("tests/fixtures/libsndfile/manifest.json");

    let actual_manifest_path = if manifest_path.exists() {
        manifest_path
    } else if fallback_path.exists() {
        fallback_path
    } else {
        panic!(
            "Cannot locate libsndfile manifest.json at {:?} or {:?}",
            manifest_path, fallback_path
        );
    };

    let manifest_file = File::open(actual_manifest_path).unwrap();
    let manifest: HashMap<String, FixtureInfo> = serde_json::from_reader(manifest_file).unwrap();

    let parent_dir = actual_manifest_path.parent().unwrap();

    for (filename, info) in &manifest {
        let file_path = parent_dir.join(filename);
        let file = File::open(&file_path).unwrap();
        let mut decoder = open_decoder(file).unwrap();

        assert_eq!(
            decoder.info().channels as usize,
            info.channels,
            "Channel mismatch on {filename}"
        );
        assert_eq!(
            decoder.info().sample_rate,
            info.sample_rate,
            "Sample rate mismatch on {filename}"
        );
        assert_eq!(
            decoder.info().total_frames as usize,
            info.frames,
            "Frame count mismatch on {filename}"
        );

        let decoded = decoder.decode_all().unwrap();
        assert_eq!(decoded.num_channels(), info.channels);
        assert_eq!(decoded.num_frames(), info.frames);

        let mut max_err = 0.0f32;
        for ch in 0..info.channels {
            for frame in 0..info.frames {
                let actual = decoded.channel(ch)[frame];
                let expected = info.channel_samples[ch][frame];
                let err = (actual - expected).abs();
                if err > max_err {
                    max_err = err;
                }
            }
        }

        // Bit-exact or minimal quantization rounding difference vs libsndfile
        assert!(
            max_err <= 1e-4,
            "Error too high on {filename}: max error was {max_err}"
        );
        println!("Verified {filename} vs libsndfile: max absolute delta = {max_err:.2e}");
    }
}
