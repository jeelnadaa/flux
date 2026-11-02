//! Flux testkit: signal generators, analyzers, and invariant validation harness.
#![deny(missing_docs)]

pub mod analyzers;
pub mod compare;
pub mod corpus;
pub mod generators;
pub mod invariants;
pub mod sha256;
pub mod wav;

pub use corpus::{
    generate_synthetic_corpus, parse_manifest_yaml, serialize_manifest_yaml,
    verify_corpus_integrity, CorpusEntry, CorpusManifest, IntegrityReport,
};
pub use invariants::{run_dsp_invariants, InvariantConfig, InvariantSuiteResult};
pub use sha256::sha256_digest;
pub use wav::{decode_wav, encode_wav, read_wav_file, write_wav_file, WavData, WavFormat};

#[cfg(test)]
mod tests {
    use crate::analyzers::*;
    use crate::compare::*;
    use crate::generators::*;
    use crate::invariants::{run_dsp_invariants, InvariantConfig};
    use flux_dsp::{BiquadChain, BiquadType, GainNode, PassthroughNode};

    #[test]
    fn test_biquad_chain_universal_invariants() {
        let config = InvariantConfig::default();
        let result = run_dsp_invariants(
            || {
                let mut chain = BiquadChain::new(4);
                chain.configure_stage(0, BiquadType::Highpass, 40.0, 0.707, 0.0);
                chain.configure_stage(1, BiquadType::Peaking, 1000.0, 1.0, 3.0);
                chain.configure_stage(2, BiquadType::Peaking, 4000.0, 1.5, -2.0);
                chain.configure_stage(3, BiquadType::Lowpass, 18000.0, 0.707, 0.0);
                Box::new(chain)
            },
            &config,
        );
        assert!(
            result.all_passed(),
            "Biquad chain failed invariants: {:?}",
            result
        );
    }

    #[test]
    fn test_passthrough_node_universal_invariants() {
        let config = InvariantConfig::default();
        let result = run_dsp_invariants(|| Box::new(PassthroughNode::new()), &config);
        assert!(
            result.all_passed(),
            "Passthrough node failed invariants: {:?}",
            result
        );
    }

    #[test]
    fn test_gain_node_universal_invariants() {
        let config = InvariantConfig::default();
        let result = run_dsp_invariants(|| Box::new(GainNode::new(0.8)), &config);
        assert!(
            result.all_passed(),
            "Gain node failed invariants: {:?}",
            result
        );
    }

    #[test]
    fn test_binaural_node_universal_invariants() {
        let config = InvariantConfig::default();
        let result = run_dsp_invariants(|| Box::new(flux_dsp::BinauralNode::new()), &config);
        assert!(
            result.all_passed(),
            "Binaural node failed invariants: {:?}",
            result
        );
    }

    #[test]
    fn test_stretch_node_universal_invariants() {
        let config = InvariantConfig::default();
        let result = run_dsp_invariants(|| Box::new(flux_dsp::StretchNode::new()), &config);
        assert!(
            result.all_passed(),
            "Stretch node failed invariants: {:?}",
            result
        );
    }

    #[test]
    fn test_sine_generator_and_spectrum() {
        let sample_rate = 48000.0;
        let freq = 1000.0;
        let duration = 0.5;
        let sig = sine(freq, sample_rate, duration, 1.0, 0.0);

        assert_eq!(sig.len(), 24000);

        // Sine RMS should be 1 / sqrt(2) ~ -3.0103 dBFS
        let r_db = rms_db(&sig);
        assert!((r_db - (-3.0103)).abs() < 0.05, "RMS was {}", r_db);

        // Peak should be 1.0 = 0.0 dBFS
        let p_db = peak_db(&sig);
        assert!((p_db - 0.0).abs() < 0.01, "Peak was {}", p_db);

        // Dominant frequency should be 1000 Hz
        let (dom_freq, _dom_mag) = dominant_frequency(&sig, sample_rate);
        assert!(
            (dom_freq - freq).abs() < 5.0,
            "Dominant freq was {}",
            dom_freq
        );

        // Crest factor of a pure sine is sqrt(2) ~ 1.4142
        let cf = crest_factor(&sig);
        assert!(
            (cf - std::f32::consts::SQRT_2).abs() < 0.02,
            "Crest factor was {}",
            cf
        );
    }

    #[test]
    fn test_pure_sine_thd_plus_n() {
        let sample_rate = 48000.0;
        let freq = 1000.0;
        let sig = sine(freq, sample_rate, 0.2, 1.0, 0.0);
        let thd = thd_plus_n(&sig, freq, sample_rate);

        // Pure mathematical sine should have THD+N below -100 dBFS
        assert!(thd < -100.0, "THD+N was {}", thd);
    }

    #[test]
    fn test_impulse_and_group_delay() {
        let len = 1024;
        let delay_samples = 8;
        let imp = impulse(len, delay_samples);

        assert_eq!(imp[delay_samples], 1.0);
        assert_eq!(imp[0], 0.0);

        let sample_rate = 48000.0;
        let gd = group_delay(&imp, sample_rate);
        assert!(!gd.is_empty());

        // Expected group delay is delay_samples / sample_rate seconds
        let expected_delay = (delay_samples as f32) / sample_rate;
        // Verify in mid-band
        for &(_freq, d) in gd.iter().skip(10).take(200) {
            assert!(
                (d - expected_delay).abs() < 1e-4,
                "Delay was {}, expected {}",
                d,
                expected_delay
            );
        }
    }

    #[test]
    fn test_white_and_pink_noise_determinism() {
        let n1 = white_noise(1000, 42, 0.5);
        let n2 = white_noise(1000, 42, 0.5);
        let n3 = white_noise(1000, 99, 0.5);

        assert_eq!(n1, n2, "Seeded white noise must be deterministic");
        assert_ne!(n1, n3, "Different seeds must produce different noise");

        let p1 = pink_noise(1000, 42, 0.5);
        let p2 = pink_noise(1000, 42, 0.5);
        assert_eq!(p1, p2, "Seeded pink noise must be deterministic");
    }

    #[test]
    fn test_chirp_generators() {
        let lin = chirp_linear(20.0, 20000.0, 48000.0, 0.1, 0.8);
        let log = chirp_log(20.0, 20000.0, 48000.0, 0.1, 0.8);

        assert_eq!(lin.len(), 4800);
        assert_eq!(log.len(), 4800);
        assert!(peak(&lin) > 0.7);
        assert!(peak(&log) > 0.7);
    }

    #[test]
    fn test_multitone_and_dc() {
        let freqs = [100.0, 1000.0, 10000.0];
        let mt = multitone(&freqs, 48000.0, 0.1, 0.9);
        assert_eq!(mt.len(), 4800);
        assert!(peak(&mt) <= 1.0);

        let dc_sig = dc(0.5, 100);
        assert_eq!(dc_sig.len(), 100);
        assert_eq!(dc_sig[50], 0.5);

        let sil = silence(100);
        assert_eq!(peak(&sil), 0.0);
        assert_eq!(rms_db(&sil), -140.0);
    }

    #[test]
    fn test_true_peak_intersample_detection() {
        // High frequency pattern at Nyquist/2 where sample values are <= 1.0
        // but true reconstructed analog waveform peaks above 1.0 (inter-sample peak)
        let sample_rate = 48000.0;
        let freq = 12000.0; // Fs / 4
        let sig = sine(freq, sample_rate, 0.05, 1.0, std::f32::consts::PI / 4.0);

        let (tp, tp_db) = true_peak(&sig);
        assert!(tp >= 0.99, "True peak was {}", tp);
        assert!(tp_db >= -0.1, "True peak dB was {}", tp_db);
    }

    #[test]
    fn test_lufs_reference_calibration() {
        let sample_rate = 48000.0;
        let freq = 1000.0;
        let duration = 1.0;

        // In BS.1770, verify that scaling tone amplitude scales measured LUFS linearly,
        // and that a calibrated 1 kHz reference tone satisfies target loudness within 0.1 LU.
        let full_scale_tone = sine(freq, sample_rate, duration, 1.0, 0.0);
        let full_scale_lufs = lufs(&full_scale_tone, sample_rate);

        let target_lufs = -23.0f32;
        let amplitude = 10.0f32.powf((target_lufs - full_scale_lufs) / 20.0);
        let calibrated_tone = sine(freq, sample_rate, duration, amplitude, 0.0);

        let measured = lufs(&calibrated_tone, sample_rate);
        assert!(
            (measured - target_lufs).abs() <= 0.05,
            "LUFS measurement was {}, target was {}",
            measured,
            target_lufs
        );
    }

    #[test]
    fn test_comparison_null_and_snr() {
        let sig_a = sine(1000.0, 48000.0, 0.1, 1.0, 0.0);
        let sig_b = sig_a.clone();

        // Null test on identical signals
        let null_res = null_residual(&sig_a, &sig_b);
        assert!(null_res.is_bit_exact);
        assert_eq!(null_res.max_error_db, -140.0);
        assert_eq!(null_res.rms_error_db, -140.0);

        // SNR on identical signals
        let snr_val = snr(&sig_a, &sig_b);
        assert!(snr_val >= 140.0);

        // Slightly degraded signal (add small noise)
        let noise = white_noise(sig_a.len(), 123, 0.001); // -60 dBFS noise
        let mut degraded = sig_a.clone();
        for i in 0..sig_a.len() {
            degraded[i] += noise[i];
        }

        let degraded_snr = snr(&sig_a, &degraded);
        assert!(
            degraded_snr > 50.0 && degraded_snr < 70.0,
            "SNR was {}",
            degraded_snr
        );

        let spec_dist = spectral_distance(&sig_a, &degraded, 48000.0);
        assert!(spec_dist < 5.0, "Spectral distance was {}", spec_dist);
    }

    #[test]
    fn test_corpus_manifest_integrity() {
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("Failed to locate repo root");

        let manifest_path = repo_root.join("corpus").join("corpus.yaml");
        assert!(
            manifest_path.exists(),
            "corpus.yaml must exist at {}",
            manifest_path.display()
        );

        let yaml_text =
            std::fs::read_to_string(&manifest_path).expect("Failed to read corpus.yaml");
        let manifest =
            crate::corpus::parse_manifest_yaml(&yaml_text).expect("Failed to parse corpus.yaml");

        assert_eq!(manifest.version, 1);
        assert!(
            !manifest.entries.is_empty(),
            "Manifest entries must not be empty"
        );

        let report = crate::corpus::verify_corpus_integrity(&manifest, repo_root);
        assert!(
            report.failures.is_empty(),
            "Corpus integrity failures: {:?}",
            report.failures
        );
        assert!(report.is_valid(), "Corpus report invalid: {:?}", report);
        assert_eq!(
            report.synthetic_entries, 15,
            "Expected 15 synthetic entries, found {}",
            report.synthetic_entries
        );
        assert_eq!(
            report.files_verified, 15,
            "Expected 15 verified fixtures on disk, verified {}",
            report.files_verified
        );
    }

    #[test]
    fn test_manifest_yaml_parse_and_serialize() {
        let sample_yaml = r#"
version: 1
description: "Test manifest"

corpus:
  - id: syn_test_sine
    path: corpus/synthetic/test_sine.wav
    source: synthetic:test
    licence: CC0-1.0
    sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
    duration: 1.000
    sample_rate: 48000
    channels: 1
    synthetic: true
    tags:
      - test
      - sine
    expected_test_uses:
      - dsp-unit
"#;

        let parsed = crate::corpus::parse_manifest_yaml(sample_yaml).expect("Failed to parse");
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0].id, "syn_test_sine");
        assert_eq!(parsed.entries[0].channels, 1);
        assert_eq!(parsed.entries[0].sample_rate, 48000);
        assert!(parsed.entries[0].synthetic);

        let serialized = crate::corpus::serialize_manifest_yaml(&parsed);
        let parsed_again =
            crate::corpus::parse_manifest_yaml(&serialized).expect("Failed to re-parse");
        assert_eq!(parsed, parsed_again);
    }
}
