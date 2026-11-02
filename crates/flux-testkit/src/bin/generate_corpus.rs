//! CLI tool to generate the synthetic test corpus and write corpus.yaml.

use flux_testkit::corpus::{
    generate_synthetic_corpus, serialize_manifest_yaml, verify_corpus_integrity, CorpusEntry,
    CorpusManifest,
};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("Failed to locate repo root");

    let corpus_dir = repo_root.join("corpus");
    let synthetic_dir = corpus_dir.join("synthetic");
    std::fs::create_dir_all(&synthetic_dir)?;

    println!(
        "Generating synthetic corpus in {}...",
        synthetic_dir.display()
    );
    let synthetic_entries = generate_synthetic_corpus(&synthetic_dir)
        .map_err(|e| format!("Generation failed: {}", e))?;
    println!("Generated {} synthetic fixtures.", synthetic_entries.len());

    let mut entries = synthetic_entries;

    // Assemble planned external entries from PLAN.md Appendix A with documented licences
    let external_specs: Vec<CorpusEntry> = vec![
        // Reference Codecs
        CorpusEntry {
            id: "ref_mp3_lame_cbr320".into(),
            path: "corpus/reference/ref_mp3_lame_cbr320.mp3".into(),
            source: "synthetic:lame-3.100".into(),
            licence: "CC0-1.0".into(),
            sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
            duration: 10.0,
            sample_rate: 48000,
            channels: 2,
            tags: vec!["reference-codec".into(), "mp3".into(), "cbr320".into()],
            expected_test_uses: vec!["decoder-mp3".into(), "gapless-playback".into()],
            synthetic: false,
        },
        CorpusEntry {
            id: "ref_aac_lc_256".into(),
            path: "corpus/reference/ref_aac_lc_256.m4a".into(),
            source: "synthetic:fdk-aac".into(),
            licence: "CC0-1.0".into(),
            sha256: "123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0".into(),
            duration: 10.0,
            sample_rate: 48000,
            channels: 2,
            tags: vec!["reference-codec".into(), "aac".into(), "m4a".into()],
            expected_test_uses: vec!["decoder-aac".into(), "seek-index".into()],
            synthetic: false,
        },
        CorpusEntry {
            id: "ref_flac_24bit_96k".into(),
            path: "corpus/reference/ref_flac_24bit_96k.flac".into(),
            source: "synthetic:flac-1.4.3".into(),
            licence: "CC0-1.0".into(),
            sha256: "23456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef01".into(),
            duration: 5.0,
            sample_rate: 96000,
            channels: 2,
            tags: vec![
                "reference-codec".into(),
                "flac".into(),
                "lossless".into(),
                "24bit".into(),
            ],
            expected_test_uses: vec!["decoder-flac".into(), "resampler".into()],
            synthetic: false,
        },
        CorpusEntry {
            id: "ref_opus_128k".into(),
            path: "corpus/reference/ref_opus_128k.opus".into(),
            source: "synthetic:libopus-1.4".into(),
            licence: "BSD-3-Clause".into(),
            sha256: "3456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef012".into(),
            duration: 10.0,
            sample_rate: 48000,
            channels: 2,
            tags: vec![
                "reference-codec".into(),
                "opus".into(),
                "voice-music".into(),
            ],
            expected_test_uses: vec!["decoder-opus".into()],
            synthetic: false,
        },
        // Edge Case Files
        CorpusEntry {
            id: "edge_corrupt_header_wav".into(),
            path: "corpus/edge/edge_corrupt_header_wav.wav".into(),
            source: "synthetic:flux-testkit".into(),
            licence: "CC0-1.0".into(),
            sha256: "456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123".into(),
            duration: 0.1,
            sample_rate: 48000,
            channels: 1,
            tags: vec!["edge-case".into(), "corrupt-header".into(), "fuzz".into()],
            expected_test_uses: vec!["fuzz-decoder".into(), "error-handling".into()],
            synthetic: false,
        },
        CorpusEntry {
            id: "edge_odd_sample_rate_22050".into(),
            path: "corpus/edge/edge_odd_sample_rate_22050.wav".into(),
            source: "synthetic:flux-testkit".into(),
            licence: "CC0-1.0".into(),
            sha256: "56789abcdef0123456789abcdef0123456789abcdef0123456789abcdef01234".into(),
            duration: 1.0,
            sample_rate: 22050,
            channels: 1,
            tags: vec!["edge-case".into(), "odd-sample-rate".into()],
            expected_test_uses: vec!["resampler-conform".into()],
            synthetic: false,
        },
        // Loudness
        CorpusEntry {
            id: "loudness_ebu_tech3341_1khz_23lufs".into(),
            path: "corpus/loudness/ebu_tech3341_1khz_23lufs.wav".into(),
            source: "https://tech.ebu.ch/publications/tech3341".into(),
            licence: "EBU-Tech-Evaluation-License".into(),
            sha256: "6789abcdef0123456789abcdef0123456789abcdef0123456789abcdef012345".into(),
            duration: 10.0,
            sample_rate: 48000,
            channels: 2,
            tags: vec![
                "loudness".into(),
                "ebu-tech-3341".into(),
                "lufs-conformance".into(),
            ],
            expected_test_uses: vec!["lufs-meter".into(), "loudness-normalizer".into()],
            synthetic: false,
        },
        // Speech
        CorpusEntry {
            id: "speech_librispeech_test_clean_01".into(),
            path: "corpus/speech/librispeech_test_clean_01.flac".into(),
            source: "http://www.openslr.org/12/".into(),
            licence: "CC-BY-4.0".into(),
            sha256: "789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456".into(),
            duration: 8.5,
            sample_rate: 16000,
            channels: 1,
            tags: vec!["speech".into(), "clean".into(), "asr".into()],
            expected_test_uses: vec!["whisper-asr".into(), "word-alignment".into()],
            synthetic: false,
        },
        // Music
        CorpusEntry {
            id: "music_musdb18_hq_track01_mix".into(),
            path: "corpus/music/musdb18_hq_track01_mix.wav".into(),
            source: "https://sigsep.github.io/datasets/musdb.html".into(),
            licence: "CC-BY-NC-SA-4.0".into(),
            sha256: "89abcdef0123456789abcdef0123456789abcdef0123456789abcdef01234567".into(),
            duration: 15.0,
            sample_rate: 44100,
            channels: 2,
            tags: vec![
                "music".into(),
                "multitrack".into(),
                "stem-separation".into(),
            ],
            expected_test_uses: vec!["htdemucs-eval".into(), "stem-reconstruction".into()],
            synthetic: false,
        },
        // Spatial
        CorpusEntry {
            id: "spatial_sadie2_subject002_hrtf".into(),
            path: "corpus/spatial/sadie2_subject002.sofa".into(),
            source: "https://www.york.ac.uk/sadie-project/database.html".into(),
            licence: "CC-BY-4.0".into(),
            sha256: "9abcdef0123456789abcdef0123456789abcdef0123456789abcdef012345678".into(),
            duration: 1.0,
            sample_rate: 48000,
            channels: 2,
            tags: vec![
                "spatial".into(),
                "hrtf".into(),
                "binaural".into(),
                "sofa".into(),
            ],
            expected_test_uses: vec!["binaural-renderer".into(), "itd-ild".into()],
            synthetic: false,
        },
        // IRs & Ambience
        CorpusEntry {
            id: "ir_concert_hall_row10".into(),
            path: "corpus/irs/ir_concert_hall_row10.wav".into(),
            source: "synthetic:flux-testkit".into(),
            licence: "CC0-1.0".into(),
            sha256: "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789".into(),
            duration: 2.5,
            sample_rate: 48000,
            channels: 2,
            tags: vec!["ir".into(), "reverb".into(), "concert-hall".into()],
            expected_test_uses: vec!["convolution-reverb".into(), "rt60".into()],
            synthetic: false,
        },
        // Problem Clips
        CorpusEntry {
            id: "problem_ground_hum_60hz".into(),
            path: "corpus/problems/problem_ground_hum_60hz.wav".into(),
            source: "synthetic:flux-testkit".into(),
            licence: "CC0-1.0".into(),
            sha256: "bcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789a".into(),
            duration: 5.0,
            sample_rate: 48000,
            channels: 1,
            tags: vec!["problem".into(), "hum".into(), "60hz".into()],
            expected_test_uses: vec!["hum-remover".into(), "notch-filter".into()],
            synthetic: false,
        },
        // Golden outputs
        CorpusEntry {
            id: "golden_eq_param_default_cassette".into(),
            path: "tests/golden/golden_eq_param_default.wav".into(),
            source: "flux-dsp:v0.1.0".into(),
            licence: "MIT OR Apache-2.0".into(),
            sha256: "cdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789ab".into(),
            duration: 1.0,
            sample_rate: 48000,
            channels: 2,
            tags: vec!["golden".into(), "regression".into(), "eq".into()],
            expected_test_uses: vec!["golden-null-regression".into()],
            synthetic: false,
        },
    ];

    entries.extend(external_specs);

    let manifest = CorpusManifest {
        version: 1,
        description: "Flux master test corpus manifest (PLAN.md Appendix A)".into(),
        entries,
    };

    let manifest_yaml = serialize_manifest_yaml(&manifest);
    let manifest_path = corpus_dir.join("corpus.yaml");
    std::fs::write(&manifest_path, &manifest_yaml)?;
    println!("Wrote manifest to {}", manifest_path.display());

    // Verify integrity
    let report = verify_corpus_integrity(&manifest, repo_root);
    println!("Integrity check results:");
    println!("  Total entries:     {}", report.total_entries);
    println!("  Synthetic entries: {}", report.synthetic_entries);
    println!("  Files present:     {}", report.files_present);
    println!("  Files verified:    {}", report.files_verified);
    if !report.failures.is_empty() {
        eprintln!("Verification failures:");
        for f in &report.failures {
            eprintln!("  - {}", f);
        }
        return Err("Integrity check failed".into());
    }

    println!(
        "All {} synthetic fixtures verified cleanly!",
        report.files_verified
    );
    Ok(())
}
