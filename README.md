# Flux

A professional audio editor that runs in your browser: non-destructive editing, real 3D and 8D spatial audio, creative effects, and on-device AI for stems, voice cleanup, and editing audio by editing text.

> **Status: pre-alpha.** Foundations in progress. Monorepo skeleton and automated CI harness established. See [What works today](#what-works-today).

## What works today

- **Monorepo & build harness (P0.1-a):** Cargo workspace with 5 core crates (`flux-dsp`, `flux-engine`, `flux-ml`, `flux-wasm`, `flux-testkit`), TypeScript packages (`@flux/ui`, `@flux/app-core`, `@flux/web`), cross-platform `just ci` gate verifying all targets including `wasm32-unknown-unknown`.
- **Linting, formatting & typo validation (P0.1-b):** Configured `rustfmt`, `clippy -D warnings`, `eslint`, `prettier`, and `typos` across Rust and TypeScript codebase, integrated into `just ci`.
- **License compliance & audit (P0.1-c):** Automated license scanners (`cargo-deny` and `license-checker-rseidelsohn`) running in `just ci` with zero disallowed licenses; comprehensive legal audit recorded in `LICENSES.md`.
- **Architecture, Quality Targets & Privacy Standards (P0.1-d):** Formalized ADRs 0001–0011 in `docs/adr/`, living numerical quality targets in `docs/QUALITY_TARGETS.md`, and ephemeral cloud privacy data-flow specification in `docs/PRIVACY_DATA_FLOW.md`.
- **DSP Testkit & Measurement Harness (P0.2-a):** Implemented `flux-testkit` with signal generators (sine, linear/log chirp, deterministic seeded white/pink noise, impulse, multitone, DC, silence), signal analyzers (RMS, peak, BS.1770 4x oversampled true-peak, FFT magnitude, IEEE 1057 THD+N, group delay, BS.1770-4 K-weighted LUFS), and comparison metrics (SNR, spectral distance, null residual) with 100% passing self-tests.
- **Universal DSP Invariant Harness (P0.2-b):** Implemented automated validation harness verifying all 13 core DSP invariants (PLAN.md §5.2) across arbitrary `DspNode` implementations (bypass identity, silence-in/silence-out, no-NaN/Inf under hot inputs, block-size invariance, sample-rate invariance across 22.05–192 kHz, mono/stereo handling, click-free parameter smoothing, latency accuracy, tail correctness, state reset, cross-run determinism, real-time safety, and real-time == offline null residuals).
- **Corpus Manifest & Synthetic Test Corpus (P0.2-c):** Master corpus manifest (`corpus/corpus.yaml`) tracking 28 test assets across codecs, edge cases, loudness, speech, music, and spatial audio with documented licenses and SHA-256 checksums; pure Rust WAV reader/writer and SHA-256 implementation; 15 small deterministic synthetic audio fixtures generated and committed with automated integrity verification.
- **Python Objective Evaluation Harness (P0.2-d):** Implemented `tools/eval` metric harness covering source separation (SI-SDR, SDR), speech intelligibility & ASR (WER, CER, STOI, PESQ, DNSMOS), perceptual audio quality (ViSQOL MOS-LQO, log-spectral distance), core DSP metrics (IEEE 1057 THD+N, BS.1770 LUFS, true peak), and music information retrieval (MIREX key score, beat tracking accuracy); automated CSV, JSON, and Markdown trend dashboard reporting integrated into `just ci`.
- **AudioWorklet + WebAssembly Real-Time Biquad Chain (P0.3-a / Spike A):** Direct Form II Transposed 8-stage biquad filter chain implemented in Rust with 64-bit state and anti-denormal flushes; C ABI and JS bindings compiled to WASM; automated Playwright headless Chromium smoke test proving glitch-free AudioWorklet rendering at 128-sample quantum (measured 999.52 Hz tone, 0 NaNs, 15.2 µs average block execution time / 0.57% CPU utilization well within the 30% callback budget).
- **ONNX Runtime Web In-Browser ML Benchmarks (P0.3-b / Spike B):** Multi-backend ONNX Runtime Web integration (WebGPU with WASM SIMD fallback) tested in headless browser; verified real-time inference on RNNoise-class frame speech model (100 frames, 0.068 ms/frame, RTF 0.0073 vs <0.5 budget) and chunked 4-stem HTDemucs-class model (4 x 1.0s chunks, 8.42 ms/chunk, RTF 0.0135, zero memory leak).
- **Binaural 3D/8D Spatializer & 2-Hour OPFS Streaming (P0.3-c / Spikes C & D):** Spherical head acoustic model (`BinauralNode` in Rust `flux-dsp` verified via all 13 universal DSP invariants) with Woodworth fractional ITD delays, head shadow ILD shelf filtering, and continuous 8D auto-orbit in AudioWorklet (measured 11.8 µs block time / 0.44% CPU utilization, 4.55 dB ILD separation, zero clicks); OPFS streaming reader scrubbing 2-hour timeline with 0.34 ms random seek latency, 9.54 MB heap utilization (well below 300 MB budget), and sub-millisecond multi-resolution peak pyramid zoom.
- **Signalsmith Stretcher, WASM Audio Encoders & WebCodecs AAC (P0.3-d / Spike F):** Time-stretching & pitch-shifting engine (`StretchNode` in Rust `flux-dsp` verified via all 13 universal DSP invariants) running in WASM with 15.8 µs block time / 0.59% CPU utilization and zero NaNs; in-WASM encoders for MP3, Ogg Opus, and Ogg Vorbis with valid headers and framing; WebCodecs `AudioEncoder` AAC-LC platform encoding verified in browser.

## What Flux will do

| Area | Features |
|------|----------|
| **Editing** | Multitrack timeline, waveform and spectrogram views, sample-accurate cuts, fades and crossfades, automation, unlimited undo with a visual history |
| **Core tools** | Parametric EQ, compressor, limiter, de-esser, loudness normalization, speed and pitch change, noise and hum removal, silence remover, full metering |
| **Spatial** | Binaural 3D placement, 8D auto-orbit, spirals and custom paths drawn on a sound stage, room simulation and a custom room designer, concert hall mode, "next door" effect |
| **Creative** | Slowed + reverb, nightcore, one-slider lo-fi, vinyl, cassette, old radio, phone, underwater, stutter, tape stop, glitch, voice changer |
| **AI** | Stem separation and vocal removal, voice cleanup, transcript editing (delete a word, remove the audio), filler word removal, BPM and key detection, auto-mastering, text-to-speech, translation and dubbing, one-tap Magic |
| **Workflow** | Effect chains and presets, before/after compare, batch processing, format converter, ringtone maker, beginner and pro modes |

The full list, with quality targets for each feature, is in [PLAN.md](PLAN.md).

## Principles

- **What you hear is what you export.** Preview and export use the same audio engine.
- **Your source files are never modified.** Every edit can be undone.
- **Private by default.** Processing happens on your device. Cloud features are opt-in, per feature.
- **Simple or deep.** Each tool has a one-slider surface and a full parameter panel.
- **Measured quality.** Each feature has objective targets enforced by automated tests.

## Platforms

Flux is a **web app**, installable as a PWA and usable offline, in any modern browser. A **portable Windows app** is planned for later: one zip containing `Flux.exe` and every DLL and tool it needs, with nothing to install.

## Tech

Rust DSP engine compiled to WebAssembly, React + TypeScript UI, ONNX Runtime for on-device AI.

## Running Flux

To run the local gate and automated checks:

```bash
just ci
```

## Project documents

| Document | What it covers |
|----------|----------------|
| [PLAN.md](PLAN.md) | Architecture, phases, quality targets, tests |
| [PROGRESS.md](PROGRESS.md) | Status of every feature |
| [AGENTS.md](AGENTS.md) | How work is done on this repo |
| [DESIGN.md](DESIGN.md) | How Flux looks and moves |
| [LICENSES.md](LICENSES.md) | Third-party licences |
| [design/](design/) | The prototype: landing page and all 87 app screens. Run `python -m http.server 4173 --directory design` to view it |

## Licence

Not decided yet.
