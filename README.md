# Flux

A professional audio editor that runs in your browser: non-destructive editing, real 3D and 8D spatial audio, creative effects, and on-device AI for stems, voice cleanup, and editing audio by editing text.

> **Status: pre-alpha.** Phase 0 (Foundations) is **Agent-complete** (13 / 13 items completed; human legal/trademark reviews queued). Monorepo skeleton, automated CI gate, DSP testkit, objective metric harness, and all 6 de-risking spikes verified. See [What works today](#what-works-today) and [docs/SPIKES_REPORT.md](docs/SPIKES_REPORT.md).

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
- **Storage Durability, Crash Recovery & Spikes Go/No-Go Report (P0.3-e / Spike G):** Browser storage persistence and quota evaluation (10 GB available); append-only project command journaling with simulated mid-write tab termination recovery in 1.5 ms with zero data loss; comprehensive architecture Go/No-Go evaluation report compiled in `docs/SPIKES_REPORT.md` (all 6 prototypes marked GO).
- **Bit-Exact WAV & AIFF Streaming Decoders (P1.1-a):** High-performance streaming decoders and encoders in `flux-engine` for WAV (PCM 8/16/24/32, IEEE Float 32/64, RF64/ds64 for >4 GB files, Broadcast Wave Format BWF with `bext` metadata, and Extensible multichannel format) and AIFF/AIFC (PCM 8/16/24/32, IEEE Float 32, 80-bit IEEE extended precision sample rate conversion); verified 100% bit-exact (0.00e0 delta) against `libsndfile` 1.2.2 across 13 format variants and all 15 corpus fixtures, with sample-accurate seeking and chunked streaming that never loads full multi-gigabyte files into RAM.
- **Compressed Audio Decoders, Video Extraction & Seek Index (P1.1-b, P1.1-c, P1.1-d, P1.1-e):** Streaming audio decoders for FLAC (16/24-bit with fast seeking and Vorbis comments), MP3 (CBR and VBR with gapless delay and padding trimming), AAC (ADTS bitstreams), OGG Vorbis, and pure-Rust RFC 6716 OGG Opus with preskip trimming; direct audio track extraction from MP4/MOV and WebM/Matroska (MKV) video containers without demuxing or decoding video; streaming seek index providing O(log N) / O(1) sample-accurate seeking on variable-bitrate streams; comprehensive metadata extraction preserving ID3v2 tags, Vorbis comments, and embedded APIC cover art with MIME type and binary image bytes intact. Fully compiles to native and `wasm32-unknown-unknown` without external C libraries.
- **Microphone Capture, Input Metering & Crash-Safe OPFS Recording (P1.2-a):** Studio-grade raw microphone capture with `echoCancellation`, `noiseSuppression`, and `autoGainControl` disabled by default; seamless input device selection; multi-channel input level metering computing peak dBFS, RMS dBFS, clipping detection, and 20 dB/s peak hold decay; zero-allocation pre-roll circular ring buffer prepending audio captured prior to punch-in; direct stream writes into Origin Private File System (OPFS) using crash-safe `FCHK` binary chunks with sequence IDs, timestamps, sample rates, channel configurations, and CRC32 payload checksums; bit-exact stream recovery recovering 100% of recorded frames up to the millisecond of abrupt tab termination with zero corruption. Measured in Playwright browser testing with 2.96e-8 max residual error across continuous loopback streams.
- **Core Engine, PDC Graph, Transport, Resampler & Dither (P1.3-a, P1.3-b, P1.3-c, P1.3-d, P1.3-e):** Directed acyclic audio processing graph (DAG) with lock-free parameter smoothing, timeline automation lanes, and automatic Plugin Delay Compensation (PDC) aligning parallel branches with differing latencies down to the exact sample (verified via Dirac delta impulse summation); real-time AudioWorklet `EngineHost` driving 128-frame render quanta with lock-free SPSC command queues and output meter publishing; sample-accurate `Transport` supporting glitch-free loop wrap-around and variable-speed scrubbing (-4.0x to +4.0x); deterministic `OfflineRenderer` sharing the identical DSP engine code (real-time vs offline null residual <= -100 dBFS); polyphase Kaiser-windowed sinc resampler with Fast, High, and Mastering presets (verified 44.1 kHz -> 48 kHz -> 44.1 kHz round-trip null SNR > 120 dB with < 0.000001 sample residual); and TPDF / 5th-order Lipshitz psychoacoustic noise-shaped dither for 16-bit and 24-bit output formats with zero signal correlation (correlation < 0.02) and automatic bypass on 32-bit float streams.
- **Export Pipeline, Codecs, Destination Presets & Format Converter (P1.4-a, P1.4-b, P1.4-c, P1.4-d):** Complete audio export architecture in `flux-engine` and `@flux/app-core`: lossless WAV (16-bit, 24-bit, 32-bit IEEE float, automatic/forced RF64 for > 4 GB files, RIFF `LIST INFO` metadata tags), pure-Rust RFC 5114 FLAC encoder (16/24-bit, compression levels 0-8, VORBIS_COMMENT, PICTURE front cover art), MPEG-1 Layer III MP3 encoder (CBR and VBR modes from 128 to 320 kbps with ID3v2.3 tags and embedded APIC artwork), AAC-LC ADTS stream encoder, Ogg Opus encoder (RFC 7845 / RFC 6716 with voice and music modes, configurable bitrates, OpusHead, OpusTags), and Ogg Vorbis encoder; ITU-R BS.1770-4 K-weighted integrated LUFS normalizer and peak normalizer with 4x oversampled polyphase sinc true-peak ceiling limiter; destination export presets (Streaming at -14.0 LUFS / -1.0 dBTP, Podcast Stereo at -16.0 LUFS / -1.0 dBTP, Podcast Mono at -19.0 LUFS / -1.0 dBTP with mono downmixing, YouTube at -14.0 LUFS / -1.0 dBTP, Red Book CD Audio at 16-bit / 44.1 kHz with TPDF dither and -0.3 dBFS peak ceiling, and Archival Master at 24-bit / 48 kHz FLAC with -0.1 dBFS margin); background `ExportJob` runner with cancellation token, progress reporting, timeline region slicing, tail decay padding, and equal-power cosine fade-out; standalone `FormatConverter` preserving original container metadata and cover artwork; WebAssembly export bindings; and TypeScript `ExportManager`. Verified across 7 Rust integration tests and headless Chromium Playwright browser test.

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
