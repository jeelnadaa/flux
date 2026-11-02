# Flux Objective Quality Dashboard — Run `20261102_211728`

**Commit:** `bd4f6e4`  
**Date:** 2026-11-02 21:17:28 UTC  
**Total Metrics Evaluated:** 12  

| Category | Fixture | Metric | Measured | Target | Status | Notes |
|----------|---------|--------|----------|--------|--------|-------|
| DSP / Audio | `syn_sine_1k_0dbfs_100ms` | THD+N | -97.9203 dBFS | -96.0 dBFS | PASS | 16-bit PCM quantization floor target <= -96 dBFS |
| DSP / Audio | `syn_sine_1k_0dbfs_100ms` | True Peak | -0.0003 dBTP | 0.1 dBTP | PASS | True peak ceiling target |
| DSP / Audio | `syn_sine_1k_0dbfs_100ms` | LUFS | -11.561 LUFS | — | INFO | Integrated loudness |
| DSP / Audio | `syn_silence_100ms` | Null Residual RMS | -140.0 dBFS | -120.0 dBFS | PASS | Silence in / silence out |
| DSP / Hot Float | `syn_hot_float_plusminus4_50ms` | Peak Amplitude | 4.0 FS | 4.0 FS | PASS | Hot float +/-4.0 representation preservation |
| Speech / ASR | `synthetic_transcript_pair_01` | WER | 22.2222 % | 25.0 % | PASS | Levenshtein word error rate benchmark |
| Speech / ASR | `synthetic_transcript_pair_01` | CER | 11.6279 % | 15.0 % | PASS | Character error rate benchmark |
| Restoration / Stems | `syn_chirp_with_noise_snr60` | SI-SDR | 54.992 dB | 50.0 dB | PASS | Scale-invariant separation ratio benchmark (target >= 50 dB) |
| Restoration / Stems | `syn_chirp_with_noise_snr60` | LSD | 0.0444 dB | 2.0 dB | PASS | Log-spectral distance distortion benchmark (target <= 2 dB) |
| Perceptual Quality | `syn_chirp_with_noise_snr60` | ViSQOL MOS-LQO | 4.9479 MOS | 4.0 MOS | PASS | Perceptual quality objective estimator (target >= 4.0 MOS) |
| MIR / Key | `c_major_vs_a_minor_relative` | MIREX Key Score | 0.3 score | 0.3 score | PASS | Relative major/minor credit = 0.3 |
| MIR / Beat | `synthetic_isochronous_beats_120bpm` | Beat F-Measure | 1.0 score | 0.85 score | PASS | Beat tracking accuracy target >= 0.85 |
