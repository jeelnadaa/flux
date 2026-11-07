# PLAN.md — Flux: Production-Grade Audio Editor (Spatial, Creative, AI)

> **Flux** is a professional-quality, non-destructive audio editor with 3D/8D spatial audio, creative effects, restoration tools, and AI features (stem separation, transcription-based editing, dubbing, auto-mastering).
>
> **Scope:** everything brainstormed *except* social/sharing features (no community presets, challenges, share-as-video, or accounts-for-social). Local preset import/export files are in scope.
>
> **Quality bar:** "production level" means: 32-bit float internal processing, sample-accurate editing, no audible artifacts, deterministic offline rendering that matches real-time preview, measurable quality targets for every feature, and automated tests that enforce them.
>
> **Platform scope:** a **web app**, and later a **portable Windows app** delivered as one zip with everything bundled. There is no native macOS, Linux, iOS or Android app. **Build the web app only.** Phase 10 (Windows) starts only when the user explicitly asks, after Phase 9.
>
> **Companion documents:** [AGENTS.md](AGENTS.md) (how work is executed), [PROGRESS.md](PROGRESS.md) (feature-by-feature status) and [README.md](README.md) (what works today). This file defines *what* to build and *how good* it must be.

---

## Table of Contents

0. [Guiding Principles](#0-guiding-principles)
1. [Assumptions & Open Decisions](#1-assumptions--open-decisions)
2. [Architecture Overview](#2-architecture-overview)
3. [Tech Stack](#3-tech-stack)
4. [Licensing & Legal Audit](#4-licensing--legal-audit)
5. [Testing Strategy (applies to every phase)](#5-testing-strategy-applies-to-every-phase)
6. [Phase 0 — Foundations](#phase-0--foundations)
7. [Phase 1 — Core Audio Engine & I/O](#phase-1--core-audio-engine--io)
8. [Phase 2 — Editor UI & Non-Destructive Timeline](#phase-2--editor-ui--non-destructive-timeline)
9. [Phase 3 — Core Processing Toolkit](#phase-3--core-processing-toolkit)
10. [Phase 4 — Spatial Audio (3D, 8D, Orbit, Rooms)](#phase-4--spatial-audio-3d-8d-orbit-rooms)
11. [Phase 5 — Creative Effects & Voice Changer](#phase-5--creative-effects--voice-changer)
12. [Phase 6 — AI Restoration & Transcript Editing](#phase-6--ai-restoration--transcript-editing)
13. [Phase 7 — Advanced AI: Analysis, Mastering, TTS, Dubbing, Magic Button](#phase-7--advanced-ai-analysis-mastering-tts-dubbing-magic-button)
14. [Phase 8 — Pro Workflow, UX Polish & Accessibility](#phase-8--pro-workflow-ux-polish--accessibility)
15. [Phase 9 — Hardening, Packaging & Launch](#phase-9--hardening-packaging--launch)
16. [Feature → Phase Traceability Matrix](#feature--phase-traceability-matrix)
17. [Risks & Mitigations](#risks--mitigations)
18. [Rough Timeline & Team](#rough-timeline--team)
19. [Appendix A — Test Corpus](#appendix-a--test-corpus)
20. [Appendix B — Quality Metrics Glossary](#appendix-b--quality-metrics-glossary)
21. [Appendix C — Definition of Done](#appendix-c--definition-of-done)
22. [Appendix D — Revision History](#appendix-d--revision-history)
23. [Phase 10 — Windows Portable App (on request only)](#phase-10--windows-portable-app-on-request-only)

---

## 0. Guiding Principles

1. **One DSP engine, two modes.** The exact same compiled DSP code runs for real-time preview and for offline export. This is the only way to guarantee "what you hear is what you export."
2. **Non-destructive by default.** Source audio is never modified. Edits are a list of operations; effects are a graph; heavy results (AI outputs) are cached as derived assets linked to their source.
3. **32-bit float everywhere internally.** Convert to integer only at export, with proper dithering. Never clip internally; handle clipping only at the output stage.
4. **Sample-accurate.** Cuts, fades, automation, and effect parameters resolve to sample positions, not UI pixels or milliseconds.
5. **Measure, don't guess.** Every feature has an objective metric (dB error, SI-SDR, WER, cents, LUFS, etc.) and a perceptual check (ABX/MUSHRA) where objective metrics are insufficient.
6. **Privacy-first.** Default to on-device processing. Server-side GPU processing is opt-in per feature, with clear disclosure, encryption in transit, and automatic deletion.
7. **Graceful degradation.** If a device can't run a model (no WebGPU, low RAM), offer the cloud path or a lighter model, never a crash.
8. **Beginner-simple, pro-deep.** Every feature has a one-slider "simple" surface and an "advanced" panel with real parameters.
9. **No dark patterns around AI.** Voice cloning and voice conversion require explicit consent attestations; all generated or converted speech (TTS, dubbing, voice conversion, cloning) is labeled in machine-readable metadata.
10. **The user's work is sacred.** No crash, browser storage eviction, full disk, or failed migration may lose committed work. Durability beats every other convenience.
11. **Every phase ends runnable.** The app builds, launches, and does something useful at the end of every phase. There is no long stretch where nothing can be heard or clicked.

---

## 1. Assumptions & Open Decisions

These were not specified, so this plan makes a default choice. Change them in Phase 0 if needed; the architecture is designed so most changes only affect the UI shell.

| # | Decision | Default in this plan | Why / Alternatives |
|---|----------|---------------------|--------------------|
| D1 | Platforms | **Web app (PWA) only, through Phase 9.** Then, **only when the user asks**, a portable Windows x64 app (Phase 10): one zip containing `Flux.exe` and every dependency it needs. No native macOS, Linux or mobile apps. | Fastest iteration, zero-install, one UI codebase. The web app still runs in macOS, Linux and phone browsers (§1.1). |
| D2 | DSP language | **Rust** compiled to WASM (web), and also kept compiling natively so the Windows app in Phase 10 reuses it | Memory safe, fast, one codebase. Alternative: C++ (JUCE-style). |
| D3 | AI execution | **Hybrid**: on-device where feasible (ONNX Runtime Web/WebGPU, native ORT), cloud GPU for heavy jobs (dubbing, large stem models, TTS cloning) | Balance privacy, cost, quality. |
| D4 | Business model | Not decided (free tier + Pro assumed). Affects licensing (see §4). **No account is needed for anything that runs on-device**; sign-in exists only for opt-in cloud features and paid plans. | Commercial-use licences matter for models and libraries. |
| D5 | Target users | Music fans, podcasters/creators, musicians/DJs | Drives preset content and default workflows. |
| D6 | Offline support | Core editor + on-device AI works offline after models are downloaded | PWA service worker + OPFS storage. |
| D7 | Languages (UI) | English first; i18n-ready from day one | |
| D8 | Celebrity-style voice presets | **Dropped.** Replaced with original character voices (robot, monster, alien, "announcer", etc.) | Imitating real people's voices carries legal/ethical risk (right of publicity, impersonation). Applies to UI copy and preset names too: no real people or third-party trademarks. |
| D9 | Delivery model | **Small team or solo developer working with AI coding agents** (see [AGENTS.md](AGENTS.md)). Gates an agent cannot perform are tracked as human gates (§5.4). | The original estimates assumed 8–10 people; the listening panels, device labs and legal reviews still need real people. |
| D10 | Cloud timing | **Built last, behind a provider interface.** On-device paths ship first; cloud-only tiers (e.g. "Best" stems, dubbing voices) are tested against a local mock until a real backend is funded. | GPU backends cost money from day one. The interface keeps the choice open without blocking the product. |
| D11 | Product name | **Flux.** Trademark clearance is required before public launch (§4). | FLUX:: is an existing pro-audio software brand, and FLUX is a well-known AI image model. Keep the name in one config constant so a rename stays cheap. |

### 1.1 Platform support tiers & reference devices

| Tier | Targets | Commitment |
|------|---------|------------|
| **Tier 1** | Chrome and Edge (desktop, latest 2) | Every feature; full E2E on every commit |
| **Tier 2** | Firefox and Safari (desktop, latest 2) | Core editing and DSP; AI degrades gracefully where WebGPU or codecs are missing; nightly E2E |
| **Tier 3** | iOS Safari, Android Chrome, installed PWA | Simple-mode tasks and playback/export in the phone browser. No native mobile app is planned |

**Reference devices** (all "reference device" budgets in this plan mean these; adjust in an ADR after Phase 0 if a device is unobtainable):

| Class | Device | Agent-run proxy |
|-------|--------|-----------------|
| Low-end laptop | 4-core x86, 8 GB RAM, integrated GPU | Chromium with 4× CPU throttle |
| Mid-range phone | Pixel 7a or Galaxy A54 class, iPhone 12 class | Chromium mobile emulation with 6× CPU throttle |
| Pro desktop | 8-core or better, 16 GB+, discrete GPU or Apple Silicon | Development machine |

Proxies catch regressions; only the real device signs off a budget (§5.4).

---

## 2. Architecture Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                           UI Layer (TypeScript)                      │
│  React + Zustand · Canvas/WebGL waveform & spectrogram · Spatial     │
│  stage view · Effect rack · Transcript editor · Export dialog        │
└───────────────▲──────────────────────────────────────▲───────────────┘
                │ commands / state (typed messages)    │ meters, peaks
┌───────────────┴──────────────────────────────────────┴───────────────┐
│                      Project & Command Layer (TS + Rust)             │
│  Project model · Command pattern (undo/redo) · Autosave · Asset      │
│  store (OPFS/IndexedDB or FS) · Peak/spectrogram cache · Migrations  │
└───────────────▲──────────────────────────────────────▲───────────────┘
                │                                      │
┌───────────────┴──────────────┐      ┌────────────────┴──────────────┐
│  Real-time Engine (Rust→WASM │      │  Offline Render Engine        │
│  in AudioWorklet / native    │      │  (same Rust DSP crate, run in │
│  audio thread)               │      │  Web Worker / native thread)  │
│  Lock-free param queues,     │      │  Export, bounce, analysis,    │
│  no allocation in callback   │      │  AI pre/post-processing       │
└───────────────▲──────────────┘      └────────────────▲──────────────┘
                │                                      │
        ┌───────┴──────────────────────────────────────┴───────┐
        │                 DSP Crate (`flux-dsp`)               │
        │ filters · dynamics · resampler · time/pitch ·        │
        │ convolution · spatial · reverb · modulation ·        │
        │ loudness · analysis · effect graph                   │
        └───────────────────────────┬──────────────────────────┘
                                    │
        ┌───────────────────────────┴──────────────────────────┐
        │           ML Runtime Layer (`flux-ml`)               │
        │ ONNX Runtime (WebGPU/WASM/native) · model manager ·  │
        │ chunking + overlap-add · cloud job client            │
        └───────────────────────────┬──────────────────────────┘
                                    │ (optional, opt-in)
        ┌───────────────────────────┴──────────────────────────┐
        │             Cloud Services (GPU workers)             │
        │ FastAPI gateway · Redis queue · GPU workers (Demucs, │
        │ Whisper, TTS, MT) · Object storage · Auto-delete     │
        └──────────────────────────────────────────────────────┘
```

### Key architectural rules

- **Audio thread rules:** no heap allocation, no locks, no I/O, no logging in the audio callback. Parameter changes arrive via a lock-free SPSC ring buffer and are smoothed (per-sample or per-block ramps) to avoid zipper noise.
- **Effect graph:** directed acyclic graph of nodes (`Source → Effect… → Bus → Master`). Each node declares: latency (samples), tail length, channel layout in/out, and parameter schema (with ranges, units, smoothing, automation capability). Plugin delay compensation (PDC) is automatic.
- **Block-size independence:** every DSP node must produce identical output regardless of block size (tested).
- **Sample-rate independence:** every node reads sample rate at prepare-time; coefficients are recalculated, never hard-coded.
- **Determinism:** randomness (vinyl crackle, noise, dither) uses seeded PRNGs stored in the project, so exports are bit-reproducible.
- **Derived assets:** AI outputs (stems, denoised audio, transcripts, translations) are stored as assets keyed by `hash(source + params + model version)` so they cache and invalidate correctly.
- **Denormals:** WebAssembly has no FTZ/DAZ mode, so denormals cannot be disabled with CPU flags the way they can natively. Every recursive DSP node must flush them explicitly (state flushing or an inaudible anti-denormal offset) and pass a denormal-CPU test on the WASM build.
- **AudioWorklet constraints:** the render quantum is fixed at 128 frames; there is no `fetch` and no blocking (`Atomics.wait` is forbidden) in the worklet scope, and `wasm-bindgen` glue may need `TextDecoder`/`TextEncoder` shims. Compile the module on the main thread and transfer it to the worklet. The DSP module in the worklet is single-threaded.
- **Engine host abstraction:** the UI talks to a single `EngineHost` interface with two implementations: AudioWorklet (web, built now) and native `cpal` (Windows, built in Phase 10). Never let browser audio APIs leak into project or UI logic.
- **Device sample rate:** the browser or OS may refuse the requested rate. The engine always renders at the project rate and resamples at the output stage if the device differs, and the UI shows the mismatch.
- **Cross-origin isolation:** `SharedArrayBuffer` needs COOP/COEP, so every same-page resource (CDN models, fonts, WASM) must send CORS or CORP headers. Test this against the production header configuration, not only the dev server.
- **Autoplay policy:** the `AudioContext` starts only from a user gesture; every playback entry point handles the suspended state.

### Project file format (`.flux`)

A ZIP cannot be rewritten every few seconds on a multi-gigabyte project, so working state and the portable file are separate:

- **Working project (directory in OPFS or on disk):** `project.json` snapshot, `journal.jsonl` (append-only committed commands), `assets/` (original audio untouched, derived audio, IRs/HRTFs used), `cache/` (peaks, spectrogram tiles — optional, regenerable), `thumbs/`.
- **Portable `.flux` package (Save As / Export Project):** ZIP64 of the above with audio stored uncompressed and `cache/` omitted by default. Option to reference external media instead of embedding it, with a relink flow when files move.
- Schema versioning with forward migrations; unit-tested with archived fixtures from every previous version. Migrations never modify the input; they write a new snapshot and keep the old one until the new one validates.
- Every committed command is appended and flushed to the journal immediately; a crash loses at most the in-progress gesture. Snapshots are written atomically (temp file + rename) every N commands.
- **Browser storage is not durable by default.** Request `navigator.storage.persist()`, show the result, monitor quota, warn before storage runs out, and on Chromium prefer the File System Access API so projects live in real folders. Where storage may be evicted (notably Safari), prompt for regular `.flux` backups.
- Package loading treats input as hostile: path-traversal (zip-slip) rejection, decompression-bomb limits, schema validation before use.

---

## 3. Tech Stack

| Area | Choice | Notes |
|------|--------|-------|
| DSP core | Rust (`flux-dsp`) | `no_std`-friendly core where possible; pinned **stable** toolchain; SIMD via `wide` or `core::arch` with WASM `simd128` (`std::simd` is nightly-only); `proptest`, `criterion` for testing/bench; `assert_no_alloc` in tests |
| Decoding | `symphonia` (Rust) for MP3/AAC/FLAC/OGG/WAV; `libopus` (BSD) for Opus, which Symphonia does not decode; WebCodecs `AudioDecoder` where available; FFmpeg (LGPL build) as fallback for exotic containers & video audio extraction | Verify codec licensing per target (see §4) |
| Resampling | `rubato` (Rust) or libsamplerate (BSD) | Target ≥120 dB stopband attenuation in "high quality" mode |
| Time-stretch / pitch | `signalsmith-stretch` (MIT) as default; Rubber Band R3 only if a commercial licence is purchased | C++ library, so it needs a WASM + native binding layer proven in Spike F. Formant-preserving mode required |
| Convolution | Custom uniform/non-uniform partitioned FFT convolver (`realfft`/`rustfft`) | For HRTFs, IRs, cabinet/room sims |
| Encoding | LAME (MP3), FLAC, Opus/Vorbis (libopus/libvorbis), AAC (WebCodecs `AudioEncoder` on web, Media Foundation in the Windows app; FDK-AAC has licence caveats) | Browsers without AAC encoding hide or explain AAC and `.m4r` options. Never fail silently. |
| Web audio plumbing | AudioWorklet + WASM, `SharedArrayBuffer` (needs COOP/COEP headers), OffscreenCanvas | Fallback path without SAB (message passing) |
| UI | React + TypeScript (`strict`) + Zustand; Canvas2D/WebGL (or `regl`/PixiJS) for waveform, spectrogram | Virtualized rendering for long files |
| Build & tooling | Vite, pnpm workspaces, `wasm-bindgen`, `just` as the single task runner (`just ci` runs every local gate) | One command for everything keeps humans and agents on identical checks |
| ML runtime | ONNX Runtime (Web: WebGPU/WebNN/WASM-SIMD; Windows app: DirectML, CUDA, CPU) | Model files hosted on CDN, downloaded on demand, integrity-checked. WASM32 memory is capped at 4 GB and browser buffer limits apply, so large models need external-data files and streaming |
| Storage | OPFS + IndexedDB, File System Access where available (web); filesystem (Windows app) | Streaming access; never load full long files into RAM. Sync OPFS handles run in a worker |
| Cloud | FastAPI, Redis + worker queue, S3-compatible storage, GPU nodes (A10/L4 class) | Presigned URLs, TTL deletion |
| Windows app (Phase 10 only) | Tauri + WebView2 fixed-version runtime | Native file access and audio devices (WASAPI, optional ASIO, via `cpal`). Bundled in one zip; see Phase 10 |
| CI/CD | GitHub Actions (or equivalent), self-hosted GPU runner for ML regression | |
| Observability | Sentry (errors), privacy-respecting analytics (opt-in), performance telemetry (real-time factor, xruns) | Never transmit audio, transcripts or file names. Session replay stays off |

---

## 4. Licensing & Legal Audit

**Do this in Phase 0 and re-check at every dependency/model addition.** Licences below are what I believe to be accurate at time of writing and **must be verified** against current repos/model cards before shipping, especially for commercial use.

| Component | Concern |
|-----------|---------|
| Rubber Band | GPL (commercial licence available). Using it in a closed-source app requires a purchased licence. Default plan uses Signalsmith Stretch (MIT) instead. |
| FFmpeg | LGPL or GPL depending on build flags. Use an LGPL-only build, dynamically linked, and document it. |
| LAME | LGPL. Fine if relinkable. On web that means a separate, replaceable WASM module with source and a written offer (or use a platform encoder). Same rule for an FFmpeg WASM build. |
| FDK-AAC | Non-standard licence with patent caveats; prefer platform AAC encoders. |
| Essentia | AGPL (commercial licence available) — avoid or license; the plan uses permissive alternatives for BPM/key. |
| Model weights (Demucs/HTDemucs, BS-RoFormer, DeepFilterNet, Whisper, etc.) | Code licence ≠ weights licence. Check every model card. Some separation/enhancement models are non-commercial. |
| Coqui XTTS-style models | Historically non-commercial licence; **do not ship** in a commercial product without a clear commercial licence. |
| Training data of third-party models | Some models carry data-provenance restrictions. Document each. |
| HRTF datasets (SADIE II, MIT KEMAR, CIPIC, ARI, …) | Research-friendly licences differ in commercial terms and attribution. Verify and include attributions in-app. |
| Impulse responses (rooms, halls) | Use only IRs with clear commercial licences, or record/commission your own. |
| Vinyl/tape/crowd samples | Record your own or license. |
| Loudness standards (ITU-R BS.1770, EBU R128) | Implementing the published algorithm is fine; use official EBU test signals only under their terms. |
| Voice cloning / voice conversion | Require consent attestation, watermark/metadata flag, ToS prohibition on impersonation/fraud, abuse reporting. Consult counsel for jurisdiction-specific rules (e.g., right of publicity, biometric/voice-data laws, EU AI Act transparency obligations). |
| Evaluation datasets | Many benchmark sets restrict use: MUSDB18 and MedleyDB are non-commercial, GTZAN's rights are unclear. Check whether commercial-product evaluation is allowed, keep them out of the repo and out of any shipped artifact, and prefer openly licensed or self-recorded sets for committed golden files. |
| Silero VAD, pyannote and similar | Verify code, weights and any gated-access terms per model card; some diarization pipelines require accepting extra terms. |
| ASIO | Steinberg's SDK has its own licence terms; verify before shipping the Windows ASIO path. |
| AI-generated audio labelling | Mark all generated or converted speech in machine-readable form (metadata, plus C2PA or watermark where feasible). EU AI Act transparency obligations and similar laws apply; confirm current scope with counsel. |
| Product name | FLUX:: is an established pro-audio software brand (including spatial audio products). Run a trademark search in target markets before public launch and take advice. Do not register domains or print assets until cleared. |
| Web fonts | Six interface fonts and Geist Mono are self-hosted so the app works offline (see DESIGN.md). Bricolage Grotesque is the default. All are SIL OFL 1.1 except Satoshi, which is under the ITF Free Font License; verify its terms before release, and drop it if they don't allow bundling. |
| User content | Users must own or have rights to uploaded audio; cloud features need a clear DPA/privacy policy, retention limits, and GDPR/DPDP-style deletion. |
| MP3/AAC patents | MP3 patents have expired; AAC licensing depends on distribution/encoder — verify with counsel for your market. |

Deliverable: `LICENSES.md` + automated license scan (`cargo-deny`, `license-checker`) in CI that fails on disallowed licences.

---

## 5. Testing Strategy (applies to every phase)

Every feature in every phase ships with tests from **all applicable layers** below. A feature is not "done" until its tests are in CI.

### 5.1 Test layers

| Layer | Tools | What it proves |
|-------|-------|----------------|
| **Unit (DSP math)** | `cargo test`, `proptest` | Filters, coefficients, interpolators, envelope followers behave per spec |
| **Property-based** | `proptest` | Invariants hold for random inputs (no NaN/Inf, bounded output, block-size invariance, sample-rate invariance, parameter extremes stable) |
| **Golden/regression** | Reference WAVs + tolerance (SNR/spectral distance) | Output didn't change unintentionally; changes require reviewed golden updates |
| **Null tests** | Subtract two renders | Bypass == input (bit-exact); offline == real-time render; refactors don't change sound |
| **Objective quality** | Python harness (librosa/scipy/`mir_eval`/`museval`/`pesq`/`pystoi`/ViSQOL/DNSMOS/jiwer) | Metrics hit targets on standard datasets |
| **Perceptual** | ABX / MUSHRA sessions (internal panel ≥8 listeners incl. trained ears; headphones + speakers) | Things metrics can't catch: artifacts, "phasiness", naturalness, externalization |
| **Integration** | Rust integration tests, TS + WASM tests (Vitest/Jest + headless browser) | Engine ↔ UI ↔ storage contracts |
| **E2E** | Playwright (Chromium/Firefox/WebKit, plus mobile emulation) | Real user flows end-to-end |
| **Performance** | `criterion` benches, real-time-factor (RTF) tracking, xrun counters | Meets CPU/latency budgets; regressions caught in CI (fail if >10% slower) |
| **Soak/stability** | 1–4h playback & editing loops, memory profiling | No leaks, no drift, no crashes |
| **Fuzzing** | `cargo-fuzz` on decoders/project loader/model loader | Malformed files can't crash or exploit the app |
| **Security** | SAST, dependency audit, pen-test for cloud endpoints | No data leaks, SSRF, path traversal |
| **Accessibility** | axe-core, manual screen-reader passes, keyboard-only runs | WCAG 2.2 AA |
| **Visual regression** | Playwright screenshots / Chromatic | UI doesn't unintentionally change |
| **Compatibility** | Browser/OS/device matrix (BrowserStack or device lab) | Works on target devices |

### 5.2 Universal DSP invariants (auto-applied to every effect via a shared harness)

Every effect registers with a test harness that automatically runs:

1. **Bypass identity:** wet=0 / bypassed → bit-exact passthrough (after latency compensation).
2. **Silence in → silence out** (or only intended tail/noise-floor), no DC offset, no denormal CPU spikes.
3. **No NaN/Inf** for any parameter combination, including extremes, rapid automation, and full-scale ±1.0 / ±4.0 (hot float) input.
4. **Block-size invariance:** render with block sizes {1, 7, 64, 128, 512, 1024, 4096}; outputs identical (bit-exact for linear/time-invariant, ≤ −120 dBFS difference for those with float-order differences).
5. **Sample-rate invariance:** characteristic metrics (frequency response, time constants) match across 22.05/32/44.1/48/88.2/96/192 kHz within tolerance.
6. **Channel handling:** mono, stereo, and (where supported) 5.1 inputs behave per spec; channel count never silently changes.
7. **Parameter smoothing:** stepping any parameter produces no click (max inter-sample delta bound) — verified by transient detector on output.
8. **Latency report accuracy:** reported latency matches measured impulse delay (±0 samples).
9. **Tail correctness:** reported tail length ≥ time for output to fall below −90 dBFS.
10. **State reset:** `reset()` clears all state; two renders of the same input separated by reset are identical.
11. **Determinism:** same project + seed → identical output across runs and platforms (within float tolerance documented per effect).
12. **Real-time safety:** instrumented allocator asserts zero allocations in `process()`.
13. **Realtime == offline:** null test between real-time capture and offline render (≤ −100 dBFS residual).

### 5.3 CI gates

- Every PR: lint, format, unit, property, golden, null, integration, license scan, WASM build, bundle size budget.
- Nightly: full objective-quality suite on datasets, perf benchmarks, soak (1h), cross-browser E2E, fuzz (time-boxed).
- Pre-release: perceptual listening session sign-off, device matrix, security scan, accessibility audit.
- Until a hosted CI exists, `just ci` run locally is the "every PR" gate and must pass before each commit.

### 5.4 Gate types: what an agent can verify

Every test in this plan belongs to one of four gate types. Agents must not simulate, estimate, or self-certify gates they cannot run.

| Gate | Examples | Who clears it | If it cannot run now |
|------|----------|---------------|----------------------|
| **[A] Automated** | Unit, property, golden, null, integration, E2E, axe-core, license scan, proxy performance runs | The agent or CI | Blocks the commit |
| **[D] Dataset** | MUSDB18, LibriSpeech and other large or licence-restricted corpora | The agent, once a human has provided the data locally | Run on the synthetic or open subset, commit, and queue the full run in PROGRESS.md |
| **[R] Real device** | Reference-device CPU budgets, Safari/iOS behavior, headphone variety | A human with the hardware | Commit on proxy results and queue it |
| **[H] Human judgment** | ABX/MUSHRA panels, usability studies, mastering engineer review, legal sign-off, pen-test | Real people | Commit, queue it, and never mark the feature "released" |

A phase can reach **Agent-complete** with [D], [R] and [H] gates queued. It reaches **Released** only when the queue for that phase is empty.

### 5.5 Threshold discipline

- Numeric targets marked "initial" are starting points. Recalibrate a target only through an ADR that records measured evidence, and record it in `docs/QUALITY_TARGETS.md`.
- **Never loosen a threshold, tolerance, or golden file to make a failing test pass** without that ADR.
- Any threshold still undefined when its test is written must be set from a baseline measurement and recorded in `docs/QUALITY_TARGETS.md` first.
- Audio tests prove behavior with measured signals. Asserting that a function ran or a file exists is not an audio test.

---

# PHASES

Each phase lists: **Goal → Features & implementation → Tests → Exit criteria.**

---

## Phase 0 — Foundations

**Goal:** Make the technical and legal decisions, set up the repo, CI, test corpus, and benchmarks, so that all later work is measurable.

### 0.1 Decisions & documents
- Finalize D1–D8 (§1). Write ADRs (architecture decision records) in `/docs/adr/`.
- Define the **quality targets table** (below) as a living doc `docs/QUALITY_TARGETS.md`.
- Licensing audit (§4); set up `cargo-deny` and JS license checker.
- Define privacy policy skeleton and data-flow diagram for cloud features.

### 0.2 Repo & tooling
- Initialize the git repository with `.gitignore` (models, datasets, build output, large audio) and `.gitattributes`.
- Monorepo: `crates/flux-dsp`, `crates/flux-engine`, `crates/flux-ml`, `crates/flux-wasm`, `crates/flux-testkit`, `packages/ui` (design tokens and components), `packages/app-core` (project model, commands), `apps/web`, `services/cloud`, `tools/eval` (Python), `tests/golden`, `docs/`.
- `just ci` (local gate) and a CI pipeline definition with caching, WASM build, Playwright, Python eval harness; GPU runner for ML regression when available.
- Pre-commit: `rustfmt`, `clippy -D warnings`, `eslint`, `prettier`, `typos`.
- Feature flags & remote config for staged rollout of AI features.

### 0.3 Test corpus & harness
- Assemble the corpus in **Appendix A** with manifest (`corpus.yaml`) listing licence, source, SHA-256, intended tests.
- Build the **DSP test harness** (§5.2) as a reusable Rust crate (`flux-testkit`): signal generators (sine, sweep, noise, impulse, chirp, multitone), analyzers (FFT, THD+N, group delay, RMS, true peak, LUFS), comparison helpers (SNR, spectral distance, null residual).
- Build **`tools/eval`** Python harness with metric wrappers (SI-SDR, PESQ, STOI, DNSMOS, WER, MIR metrics, ViSQOL) and CSV/JSON reporting + trend dashboards.

### 0.4 Prototypes (de-risking spikes, 1–2 weeks each, throwaway code)
- **Spike A:** AudioWorklet + WASM: confirm glitch-free playback with a Rust biquad chain at 128-sample blocks on mid-range phone and low-end laptop.
- **Spike B:** ONNX Runtime Web + WebGPU running a small model (RNNoise-class) in real time; and a large model (HTDemucs-class) chunked — measure RTF and memory.
- **Spike C:** Binaural rendering prototype (HRIR convolution + rotating azimuth) to validate the 8D approach by ear before committing.
- **Spike D:** 2-hour WAV scrubbing with OPFS streaming and peak-file zoom.
- **Spike F:** Encoder and stretcher builds: LAME, libopus, Vorbis and Signalsmith Stretch in WASM; WebCodecs AAC availability per browser.
- **Spike G:** Storage durability: `persist()`, quota behavior, eviction, and File System Access per Tier 1–2 browser; verify the journal survives a killed tab.

### Quality targets (initial — refine after spikes)

| Area | Metric | Target |
|------|--------|--------|
| Internal precision | Processing path | 32-bit float (64-bit for filter state where needed) |
| Biquad/EQ | Magnitude error vs analytic | ≤ 0.05 dB across 20 Hz–20 kHz (at ≥44.1 kHz) |
| Resampler (high) | Passband ripple / stopband | ≤ 0.01 dB / ≥ 120 dB (aliasing images < −120 dBFS) |
| Digital noise floor | THD+N of clean path | ≤ −110 dBFS |
| Pitch shift | Pitch accuracy | ≤ ±3 cents (±12 st range) |
| Time stretch | Duration accuracy | ≤ ±1 sample per 10 s (or ≤0.01%) |
| Loudness | LUFS accuracy vs EBU Tech 3341/3342 test set | ±0.1 LU; true-peak ±0.3 dB |
| Real-time | Callback CPU budget | ≤ 30% of block time for a typical 8-effect chain on mid-range phone |
| Latency (monitoring) | End-to-end click-to-sound | ≤ 100 ms web, ≤ 40 ms Windows app (Phase 10) |
| Stem separation | SDR (MUSDB18-HQ) | Vocals ≥ target set by chosen model (e.g., ≥ 8 dB median) — see Phase 6 |
| Denoise | DNSMOS OVRL / PESQ | Improve over input by ≥ 0.5 MOS-ish; no speech distortion regressions |
| ASR | WER (LibriSpeech test-clean / clean-ish podcasts) | ≤ 5% EN (model-dependent) |
| Beat tracking | F-measure (Ballroom/GTZAN) | ≥ 0.85 |
| Key detection | MIREX weighted score (Giant Steps / GTZAN-Key) | ≥ 0.70 |

### Tests (Phase 0)
- Harness self-tests: generators produce expected spectra; analyzers verified against known signals (e.g., LUFS of 1 kHz −23 LUFS reference tone).
- CI smoke test: build → WASM → load in headless browser → play 1 kHz tone → verify captured frequency & level.
- Corpus integrity test: hashes match manifest.

### Exit criteria
- Spikes A–D report with go/no-go and measured numbers.
- CI green on skeleton; corpus + harness usable; ADRs and licence audit complete.

---

## Phase 1 — Core Audio Engine & I/O

**Goal:** A rock-solid engine: decode anything, play glitch-free, render offline identically, export high-quality files.

### 1.1 Decoding & import
- **Formats:** WAV (PCM 8/16/24/32/float, RF64/BWF for >4 GB), AIFF, FLAC, MP3, AAC/M4A, OGG Vorbis, Opus, WMA (via FFmpeg fallback); audio extraction from common video containers (MP4/MOV/MKV/WebM).
- **Implementation:** `symphonia` first; FFmpeg fallback; metadata (ID3, Vorbis comments, cover art) preserved to export where applicable.
- **Streaming decode:** chunked, seekable, never loads whole file in RAM. Build a **sample-accurate seek index** for VBR MP3/AAC (decode-to-WAV-cache option for perfect seek).
- Handle gapless info (LAME/iTunSMPB), encoder delay/padding trimming, odd sample rates, and corrupt frames (skip gracefully with warning).
- **Import normalization policy:** keep the original sample rate/bit depth in the asset; project has its own sample rate (default 48 kHz, configurable); sources are resampled on the fly (high-quality SRC) or pre-conformed to a cached project-rate asset.

### 1.2 Recording
- Microphone capture (`getUserMedia` with echoCancellation/noiseSuppression/autoGainControl **off** by default for raw capture, toggle available); input device selection; input level meter with clip indicator; monitoring with latency warning; pre-roll buffer; recording straight into OPFS (crash-safe chunked writes).
- Windows app (Phase 10): `cpal` low-latency capture.

### 1.3 Engine core
- Effect graph, node trait, parameter system (smoothing, automation lanes, units), plugin delay compensation, bus/mixer, master section.
- Transport: play/pause/seek/loop (sample-accurate loop points with seamless wrap), variable-speed scrub with anti-aliased resampling, loop-region crossfade option.
- Real-time engine: lock-free command/param queues, meter publishing (peak, RMS, LUFS-M/S/I, true peak, spectrum) via shared ring buffers.
- Offline renderer: same graph, no real-time constraint, supports multi-threaded rendering of independent branches, progress/cancel, deterministic output.
- **Resampler module** with quality presets (fast / high / mastering) and fractional-delay primitives (used later by spatial & modulation).
- **Dither:** TPDF default, optional noise-shaped (e.g., high-order shaped for 16-bit), applied only when reducing bit depth.

### 1.4 Export / encode
- **Targets:** WAV (16/24/32f, mono/stereo), FLAC (levels 0–8, 16/24-bit), MP3 (LAME VBR V0–V9 / CBR 128–320), AAC/M4A (platform encoder; VBR/CBR), OGG Vorbis, Opus (voice/music modes), ALAC (stretch).
- **Format converter mode:** drag file(s) in → pick format/quality → batch convert with metadata/cover preserved, optional sample-rate/bit-depth/channel conversion.
- Export options: sample rate, bit depth, dither type, channel layout, normalization (off / peak / LUFS target with true-peak ceiling), tail handling (include reverb tail), start/end region, fade out on end, metadata editor.
- Export presets for common destinations (podcast −16 LUFS stereo / −19 mono, streaming −14 LUFS/−1 dBTP, YouTube, CD 16/44.1 with dither, archival 24-bit WAV). *Verify current platform loudness specs when implementing—they change.*
- Background export with progress, cancel, resume-after-crash for long jobs; export in Web Worker so UI stays responsive.

### 1.5 Analysis foundations (needed by later phases)
- Peak/min-max pyramids (multi-resolution, cached) for waveform display.
- STFT/spectrogram tile generator (linear/log/mel scales, selectable window size).
- Loudness meters per BS.1770-4 (K-weighting, gating, LRA, true peak with 4× oversampling).
- Phase correlation, stereo width, goniometer data.

### Tests (Phase 1)

**Unit / property**
- Decoder bit-exactness: PCM WAV/AIFF/FLAC decode == reference decode (bit-exact vs `libsndfile`/`flac` CLI).
- MP3/AAC/Opus decode vs reference decoder: residual ≤ −90 dBFS (or bit-exact where spec allows).
- Gapless: decode of LAME-encoded test with known delay yields exact original length ±0 samples.
- Seek accuracy: random seeks in VBR MP3 land within ±1 sample of linear decode (with seek index); fuzzed 10k seeks.
- Resampler: sweep & multitone tests → passband ripple, aliasing/imaging below −120 dBFS ("high"), phase linearity; compare vs SoX VHQ and `libsamplerate` best; round-trip 44.1→48→44.1 null ≤ −110 dB (in band).
- Dither: TPDF spectrum is flat, error uncorrelated with signal (correlation ≈ 0); noise-shaped matches design curve.
- Loudness: EBU Tech 3341/3342 test signals pass within spec tolerances; K-weighting filter verified vs published coefficients.
- True peak: inter-sample-peak test files measure within ±0.3 dB of reference.
- Parameter smoothing & automation: ramp reaches target exactly at specified sample.
- Graph: PDC test — parallel branches with differing latencies stay time-aligned (impulse test).

**Integration**
- Offline == real-time null test (≤ −100 dBFS) on a complex graph.
- Loop playback: 1-hour loop of a click track shows zero drift/dropouts (xrun counter = 0, click interval jitter < 1 sample).
- Recording → file: recorded 1 kHz loopback has no gaps/duplicated samples (continuity check).
- Export → re-import round trip: WAV/FLAC lossless exact; MP3/AAC/Opus within codec tolerance (spectral distance bound) and correct duration, metadata, cover art.
- Large files: 2-hour 48 kHz stereo 24-bit WAV (~2 GB) opens < 3 s to first waveform, memory < 300 MB.
- RF64/BWF >4 GB read/write.

**Performance**
- Real-time factor benchmarks per node; engine with 32 tracks × 4 effects within CPU budget on reference low-end device.
- Callback deadline misses: 0 over 1 h on reference devices.

**Fuzz/Security**
- `cargo-fuzz` on all decoders, metadata parsers, project loader (1 CPU-hour per target nightly); no panics/crashes/UB; max memory bound enforced against decompression bombs; `.flux` packages with path-traversal entries are rejected.

**E2E**
- Import 20 sample formats → play → export to 8 formats → verify via ffprobe + audio comparison.

### Exit criteria
- All Phase-1 tests green; 2-hour file handled; offline == real-time; export formats verified; fuzz clean; perf budgets met on reference devices.

---

## Phase 2 — Editor UI & Non-Destructive Timeline

**Goal:** A fast, precise, pleasant editing surface: waveform editor, multitrack timeline, selections, undo/redo — everything that doesn't involve effects yet.

### 2.1 Waveform & spectrogram views
- GPU/Canvas-rendered waveform from peak pyramids; smooth pinch/scroll zoom from whole-file to individual samples (sample-level view draws interpolated dots/lines); stereo/mono/mid-side display; dB or linear scale toggle; clipping highlighted.
- Spectrogram view (and hybrid waveform+spectrogram); frequency range zoom; color maps (perceptually uniform); **spectral selection** (rectangle on spectrogram — foundation for spectral repair later).
- Playhead follows with smooth scrolling; scrub (hear while dragging); loop region; markers & regions with labels/colors; snap to markers, zero-crossings, grid, beats (once BPM exists).

### 2.2 Editing operations (all non-destructive)
- Select, trim, cut, copy, paste, delete, ripple-delete, split at playhead, duplicate, move, nudge (sample/ms/frame), slip-edit (change source offset inside a clip), reverse clip, gain per clip (+ clip envelope/automation).
- **Fades:** per-clip fade in/out with curve types (linear, equal-power, S-curve, logarithmic, exponential), draggable handles. **Crossfades** between adjacent/overlapping clips with equal-power/equal-gain choice, auto-applied on overlap.
- **Merge/concatenate** multiple files; multitrack arrangement (unlimited tracks target 128+), track gain/pan/mute/solo, track colors, groups/buses.
- Zero-crossing snapping for click-free cuts; automatic micro-fades (≈2–5 ms) at every edit boundary to prevent clicks (toggleable, default on).
- Clip-level and track-level **automation lanes** (volume, pan, any effect parameter) with curve interpolation modes.

### 2.3 Command system
- Every edit is a serializable **Command** (do/undo/merge); unlimited undo/redo; coalescing for drags; branching **visual history timeline** (undo panel with thumbnails) — "undo history with a visual timeline".
- Autosave/journal; crash-recovery dialog on relaunch; project open/save/save-as; version snapshots.

### 2.4 Input & UX fundamentals
- Keyboard shortcut system (rebindable), full keyboard operation, context menus, multi-select, marquee select, touch gestures (pinch, two-finger scroll, long-press, drag handles ≥ 44px).
- Responsive layouts: desktop (multi-pane), tablet, phone (single-pane with bottom sheets).
- Drag-and-drop files; paste audio; file picker; recent projects.

### Tests (Phase 2)

**Unit**
- Command invariants: for random sequences of commands, `undo` then `redo` reproduces identical project JSON hash; `undo` all returns to initial hash (property-based).
- Clip math: split/trim/slip/ripple operations produce exact sample positions (no off-by-one), verified against a reference model implemented naively.
- Fade curves: numeric values vs analytic functions; equal-power crossfade keeps summed power constant within ±0.1 dB for uncorrelated signals; equal-gain keeps amplitude for correlated.
- Snap logic: snaps to nearest candidate within pixel threshold; deterministic tie-breaking.
- Peak pyramid: min/max at every level equals brute-force computation.

**Integration**
- Render of an edit list == manual construction of expected samples (bit-exact for cuts; tolerance for fades).
- Auto micro-fade: no sample-to-sample jump at any edit boundary exceeds the local signal's own maximum derivative by more than 3 dB, in a corpus of random edits.
- Autosave/recovery: kill process mid-edit (chaos test) → on restart, project recovers to last committed command (100 trials).
- Project schema migration: open projects saved by every prior version.
- Storage faults: quota exceeded, eviction, and revoked file permission each produce a clear prompt and no loss of committed work.
- `.flux` round trip: package → open → identical project hash and audio; missing external media triggers relink, not failure.

**Performance**
- Waveform rendering stays 60 fps while zooming/scrolling a 2-hour file; first paint < 200 ms after zoom change using cached tiles.
- Timeline with 64 tracks × 500 clips remains interactive (frame time < 16 ms for scroll/zoom).

**E2E (Playwright)**
- Flows: import → trim → split → move → fade → crossfade → undo ×10 → redo ×10 → export; verify output samples vs expected.
- Keyboard-only editing flow.
- Touch gesture simulation on emulated mobile devices.
- Visual regression on waveform/timeline states.

**Accessibility**
- Screen-reader announcements for transport state, selection range (time), and operations; focus order; contrast.

### Exit criteria
- A user can fully edit a multitrack project non-destructively with unlimited undo; crash recovery works; 60 fps on 2-hour file; E2E suite green.

---

## Phase 3 — Core Processing Toolkit

**Goal:** The "useful editing tools" — EQ, dynamics, loudness, speed/pitch, silence removal, classical noise reduction — all at professional quality, real-time capable, with simple presets and advanced controls.

### 3.0 Basic effect rack (prerequisite)
- Minimal per-clip, per-track and master effect chain UI: add, remove, reorder, bypass, wet/dry, and the shared simple/advanced panel shell. Every effect in Phases 3–7 plugs into this; Phase 8 adds the browser, previews and compare slots.

### 3.1 Equalizer
- **Parametric EQ:** up to 10 bands; filter types: peak/bell, low/high shelf, low/high-pass (6–96 dB/oct via cascaded biquads / Butterworth, Linkwitz-Riley option), band-pass, notch, tilt. RBJ-cookbook biquads in **TDF-II transposed** with 64-bit state; **bilinear-transform frequency warping compensation** near Nyquist (e.g., analog-matched / oversampled option) so high-frequency bells don't "cramp."
- **Linear-phase mode** (FIR via FFT convolution with pre-ringing control; latency reported).
- **Graphic EQ** (10/31-band) view on top of the same engine.
- **Real-time spectrum analyzer overlay** (pre/post), draggable curve nodes, Q via scroll/pinch, solo-band listen.
- **Match EQ** (match spectrum of a reference; required by auto-mastering in Phase 7) and **dynamic EQ (stretch)**.
- **Simple presets:** "Clear Voice," "Bass Boost," "Treble Boost," "Warm," "Bright," "Podcast," "Telephone-clean," "De-mud," "Vocal Presence," "Air." Each preset is a documented set of band parameters; simple mode exposes 1–3 sliders mapping to those bands.

### 3.2 Dynamics
- **Compressor** (feed-forward + program-dependent release, soft knee, sidechain filter, lookahead, auto-makeup, parallel mix), **limiter** (lookahead, true-peak safe, oversampled 4–8×, release smoothing, ISP-safe), **gate/expander**, **de-esser** (split-band, adaptive), **transient shaper**, **multiband compressor** (3–5 bands, LR4 crossover, phase-aligned).
- **Volume normalizer / leveler:** peak normalize; LUFS normalize (BS.1770) with true-peak ceiling; **adaptive leveler** (slow gain-riding on short-term loudness with speech-aware gating so breaths aren't boosted) — "evens out loud and quiet parts."

### 3.3 Speed, tempo & pitch
- **Speed change without pitch change** (time-stretch): ratio 0.25×–4×, voice mode (formant preserving, phase-coherent) and music mode (transient-preserving, polyphonic); real-time preview and offline high-quality render; sample-accurate duration.
- **Pitch shift** without tempo change: ±24 semitones, cents precision, formant-preserve toggle.
- **Varispeed/resample** mode (tempo and pitch together) — basis for "nightcore" and "slowed."
- **Tempo map with manual entry and tap tempo.** Phases 4–5 tempo sync works from this; automatic detection in Phase 7 fills the same map.

### 3.4 Restoration — classical (non-ML) baseline
- **Stationary noise reduction:** noise-profile capture + spectral gating/Wiener filtering with temporal smoothing and musical-noise suppression (e.g., minimum-statistics noise tracking, decision-directed a-priori SNR).
- **Hum removal:** auto-detect 50/60 Hz + harmonics; adaptive notch comb with depth control.
- **De-click / de-crackle**, **de-clip** (AR-model interpolation), **DC offset removal**, **de-wind** (adaptive high-pass + spectral subtraction), **de-buzz.**
- **Spectral repair:** select a region on the spectrogram → interpolate from surroundings (inpainting).
- **Auto-silence remover:** energy + spectral VAD (the neural upgrade arrives in 6.5 because the ML runtime does not exist yet) → detect pauses longer than T (default 0.6 s), keep configurable padding (default 150 ms) and shorten rather than delete ("shorten pauses to X ms" option), crossfade joins, preview markers before applying, per-region override, protects breaths/music beds with "speech-only" vs "any-audio" mode.
- **Mono/stereo tools:** mono-compat check, mid/side widener, channel swap, phase invert, pan law selection.

### 3.5 Utility
- **Metering suite:** LUFS (M/S/I), LRA, true peak, RMS/peak, spectrum, spectrogram, goniometer/phase correlation, stereo width. All meters verified (see tests).
- **Tone generator, silence generator**, **DC filter**, **gain/trim**, **fade tool**.
- **Preset system:** versioned JSON preset files (local import/export), A/B slots.

### Tests (Phase 3)

**EQ**
- Magnitude/phase response vs analytic for 1,000 random band configs: ≤ 0.05 dB (≤ 0.2° phase for IIR vs analytic at ≥ 44.1 kHz, within the stated warping limits at 192 kHz/48 kHz comparisons).
- Cascaded high-pass slope: measured slope within ±0.5 dB/oct of nominal; −3 dB at corner ±0.1 dB.
- Linear-phase mode: group delay constant; impulse response symmetric; pre-ringing matches design.
- Stability: extreme settings (Q=0.1–50, 20 Hz–Nyquist, ±24 dB) produce no NaN, bounded output.
- Parameter sweep automation: no zipper noise (spectral flux of sidebands < −80 dB relative).
- Preset tests: each preset's measured response matches documented curve within tolerance; golden files.

**Dynamics**
- Static transfer curve measured with stepped sine = theoretical (threshold, ratio, knee) within ±0.1 dB.
- Attack/release timing measured via step response within ±5% of spec (using standard definitions: 10–90% or time-constant, documented).
- Limiter: output never exceeds ceiling (sample peak and **true peak** with 8× oversampling) over 100 hours of stress signals; THD at moderate GR < −60 dB; no pumping artifacts on "difficult" corpus (kick+bass).
- Multiband: LR4 crossover sum is flat ±0.05 dB, phase-aligned (null test sum vs original with unity gains).
- De-esser: reduces sibilant band energy on test speech by target dB while non-sibilant energy change < 0.5 dB.
- Normalize: LUFS-normalized output measures target ±0.1 LU; true-peak ceiling respected; leveler reduces short-term loudness standard deviation by ≥ 40% (initial) on uneven speech corpus without raising noise floor between words > 3 dB.

**Time/pitch**
- Pitch accuracy: sine and speech tones shifted by ±1…±12 semitones → measured f0 (YIN/pYIN) within ±3 cents.
- Duration accuracy: stretch ratios {0.5, 0.75, 1.25, 1.5, 2.0, 4.0} → output length exact to ≤ 1 sample per 10 s.
- Transient preservation: drum loop attack energy ratio vs original ≥ 0.9 in music mode; no "double hit" pre-echo above −30 dB (onset detector check).
- Formant preservation: speech formants (LPC) shift ≤ 3% when enabled.
- Perceptual: ABX/MUSHRA vs reference stretcher (Rubber Band R3 commercial as benchmark in lab only) — score ≥ 80/100 on speech and music.
- Objective: ViSQOL MOS-LQO vs the reference stretcher's output ≥ 3.5 (initial).
- Block-size invariance and real-time vs offline null for streaming stretcher.

**Restoration**
- Noise reduction: on corpus with added stationary noise at SNR {0, 5, 10, 20 dB}, SNR improvement ≥ target (e.g., ≥ 8 dB at 10 dB input) with PESQ/STOI not degraded relative to input beyond bounds; **musical noise** measured as kurtosis ratio (noise-only regions, output vs input) ≤ 1.5 (initial).
- Hum removal: 50/60 Hz and first 8 harmonics reduced ≥ 40 dB; off-frequency program energy change < 0.3 dB.
- De-click: synthetic clicks (known positions) removed ≥ 30 dB with fricative speech unchanged (null test outside click regions < −60 dB).
- De-clip: clipped speech corpus → peak reconstruction error (NMSE in clipped regions) ≤ −15 dB (initial).
- Spectral repair: removed synthetic tone burst in music → energy in selected region falls ≥ 25 dB, surrounding unaffected (< −60 dB residual).
- Silence remover: precision/recall of pause detection vs labeled corpus ≥ 0.95/0.95; no word clipping (label-aligned; zero speech samples removed in 99.9% of utterances); joins are click-free.

**Meters**
- LUFS/LRA/true-peak vs EBU test sets (pass all in Tech 3341/3342); spectrum analyzer amplitude accuracy ±0.5 dB for sines at bin centers; correlation meter on L=R (+1), L=−R (−1), uncorrelated (≈0).

**Integration / E2E**
- Chain test: EQ→Comp→Limiter→Normalize on 20 corpus files; export meets target LUFS and ceiling.
- UI: parameter drag → audible/measured change within 1 block; A/B toggle is click-free.

**Performance**
- Each effect < documented CPU budget (e.g., parametric EQ 10 bands stereo < 1% core at 48 kHz on reference phone); linear-phase EQ latency reported & correct.

### Exit criteria
- All metrics at target; perceptual sign-off on EQ, limiter, stretcher; noise reduction does not introduce musical noise artifacts on corpus; meters pass EBU suite.

---

## Phase 4 — Spatial Audio (3D, 8D, Orbit, Rooms)

**Goal:** Best-in-class headphone spatialization: convincing externalization, smooth motion, no coloration disasters, and a delightful interactive stage.

> **Note on "8D audio":** it's not a real format — it's binaural audio with the source moving around the head (automated azimuth), usually plus reverb. The goal here is to implement it *correctly* (good HRTFs, smooth interpolation, proper distance cues) so it sounds substantially better than typical pan-and-reverb imitations.

### 4.1 Binaural rendering core
Two interchangeable renderers, A/B-tested by ear in Phase 4 before choosing a default:

**Renderer A — Direct HRTF with interpolation**
- Use a high-quality HRTF set (SADIE II / MIT KEMAR / CIPIC / ARI, stored as SOFA → converted to compact internal format). Convert HRIRs to **minimum-phase + separate ITD** (fractional delay) so interpolation doesn't comb-filter.
- Spatial interpolation of magnitude responses over a triangulated sphere mesh (barycentric / VBAP-style weights); ITD interpolated separately; per-block filter changes use **crossfaded dual convolvers** (or per-sample coefficient interpolation) to avoid clicks.
- Partitioned FFT convolution (low latency head + FFT tail).

**Renderer B — Ambisonic intermediate**
- Encode to **3rd-order (or higher) ambisonics** → rotate (cheap & exact for orbits) → **binaural decode with MagLS** (magnitude least-squares) + diffuse-field EQ. Smooth motion by construction.
- Benefits: orbit rotation, multi-source mixing, and room reverb all share one pipeline.

**Common**
- **ITD/ILD personalization:** head-width slider (scales ITD), plus a quick **"pick your HRTF"** test (play moving sound with 4–6 candidate HRTFs; user chooses the most externalized). Persist choice.
- **Distance cues:** gain (1/r with configurable rolloff), air absorption (distance-dependent high-shelf/low-pass), direct-to-reverberant ratio, near-field ILD boost (<1 m), optional **Doppler** (off by default for orbit; on for fly-by).
- **Elevation cues:** HRTF-based pinna notches; optional elevation exaggeration for stronger "above/below" effect.
- **Headphone compensation:** diffuse-field/free-field equalization; optional headphone-model correction curves (AutoEq-style data where licence permits).
- **Loudness stability:** compensate level variations across the orbit (target ±1 dB perceived), auto limiter after spatializer.

### 4.2 Input handling
- **Mono source:** direct placement.
- **Stereo source:** options — (a) *Virtual speakers* (L/R as sources at ±30°, spread adjustable) rotated together; (b) *Mid/Side* (center stays frontal, sides orbit); (c) *Stem mode* (after Phase 6) — each stem gets its own position/orbit/radius (e.g., vocals front, drums wide, bass center, "other" orbiting).
- Bass management: keep low frequencies (< ~120 Hz, adjustable) centered/mono to avoid thin, wobbling bass (mono-bass option).

### 4.3 Feature modes
- **3D audio (static placement):** place sound at azimuth/elevation/distance; presets (front-left, behind, above, "inside a room").
- **8D audio (auto-orbit):** circle around the head; controls: **rotation speed** (default ~0.08–0.15 Hz, i.e., ~7–12 s/rev; range 0.01–2 Hz), **direction** (CW/CCW/ping-pong), **radius/distance** (affects level/reverb), **elevation offset**, **smoothness/easing**, **start phase**, **sync to tempo** (revolution = N beats/bars once BPM detection exists), **depth of movement**.
- **16D / Orbit mode:** extended motion paths — spirals (vary radius/elevation), figure-eight, vertical loops, random-walk "swarm" (seeded), Lissajous patterns.
- **Draggable sound stage:** top-down head view + side elevation view; drag a source dot to place; **draw a path** with finger/mouse (smoothed with spline, speed profile adjustable) and the source follows it (loop / ping-pong / one-shot); record live dragging as an automation lane; multiple sources/stems with colors; snap-to-grid and angle guides; visual trail.
- **Automation:** all spatial parameters are automatable on the timeline.
- **Headphone check:** L/R channel test tone, "swap detected?" prompt, and a short **binaural sanity test** (a sound jumping front/left/right/behind) with explicit "if it sounds inside your head / flat, here's why" tips. (Browsers can't reliably detect headphones; don't claim to.)
- **Speaker warning:** binaural on speakers collapses; offer **crosstalk-cancelled / speaker mode** (stretch) and a mono-compatibility check of the binaural render.

### 4.4 Rooms & ambience
- **Convolution reverb engine** (partitioned FFT, up to 10+ s IRs, stereo & true-stereo, predelay, EQ on IR, decay-time scaling via IR envelope manipulation, time-reverse option).
- **Algorithmic reverb** (FDN / Dattorro plate / Lexicon-style hall) with size, decay, damping, diffusion, modulation, predelay, early/late balance — low CPU for mobile.
- **Room simulation presets:** Cathedral, Bathroom, Concert hall, Cave, Empty parking garage, Studio booth, Stairwell, Living room, Car interior, Stadium, Forest clearing, Subway tunnel. Built from licensed/recorded IRs and/or **image-source + FDN simulator** with absorption by material.
- **Custom room designer:** width/height/depth + wall materials → generates IR via **image-source method (early) + FDN/ray-based late tail**; source/listener placement ties to the sound stage. 
- **Binaural early reflections:** each early reflection spatialized through HRTF for real externalization.
- **Concert hall mode ("row 10"):** concert-hall IR (listener at 10th row position), plus **crowd ambience bed** (licensed/recorded applause, murmur, whoops — procedurally varied, loop-free via granular/stitching with random seeded variation), slight PA-coloration EQ, and delay-based "slapback" from back wall; crowd level control, applause triggers at start/end (optional).
- **"Next door" effect:** through-the-wall simulation: steep low-pass (~300–1200 Hz adjustable), mass-law-like transmission loss curve, comb-filter coloration, bass emphasis (flanking), diffuse low-level room reverb, slight random level fluctuation, optional muffled neighbor ambience.

### Tests (Phase 4)

**Unit / objective (renderer)**
- **ITD accuracy:** for the selected HRTF, measure interaural time difference at azimuths 0–180° via cross-correlation; compare to dataset ground truth within ±1 sample at 48 kHz; for default head (~17.5 cm width) ITD at 90° ≈ 600–700 µs.
- **ILD/spectral accuracy:** magnitude response of rendered L/R at 24 directions vs raw HRTF magnitude within ±1 dB (1/3-octave bands, 200 Hz–12 kHz).
- **Interpolation quality:** render sources at positions between measured HRTF points; spectral distortion vs "ground truth" (measured finer grid held out) ≤ 2 dB LSD; no comb-filter artifacts (check phase continuity).
- **Symmetry:** azimuth θ and −θ produce L/R swapped outputs (null ≤ −100 dB) for symmetric HRTF sets; azimuth 0 mono → L == R exactly.
- **Rotation invariance (Renderer B):** ambisonic rotation by Δ then decode equals direct encode at θ+Δ within ≤ 0.5 dB LSD; order truncation error report.
- **Cross-renderer agreement:** A vs B on static positions within ≤ 3 dB LSD below 8 kHz.
- **Continuity/click test:** during orbit (all speeds), maximum sample-to-sample derivative and spectral flux show no impulsive events (detector finds 0 clicks in 10 min renders); crossfade of filters verified by smoothness metric.
- **Level stability:** RMS across a full orbit varies ≤ ±1.5 dB for pink noise (excluding intentional distance changes).
- **Mono-bass:** content < crossover is identical in L/R (null ≤ −60 dB) when enabled.
- **Distance model:** gain vs distance follows configured law ±0.1 dB; air-absorption filter response matches formula.
- **Doppler:** pitch shift ratio matches v/c formula within ±2 cents when enabled; off → exactly none.
- **Orbit timing:** N revolutions complete at exactly N/f seconds (azimuth trajectory analytic vs measured via ITD tracking within ±2°).
- **Path following:** drawn path → rendered azimuth/elevation/time trajectory matches spline within ±1° and speed profile within ±2%.
- **Block-size & real-time vs offline invariance** for all spatial nodes.
- **Spatial reverb:** RT60 of generated IRs vs target (Schroeder integration) ±5%; early reflection times/levels match image-source theory; FDN stability (poles inside unit circle) across parameter space; normalized echo density ≥ 0.9 within 100 ms of onset (initial); no metallic ringing (late-tail spectral flatness baseline recorded per §5.5).
- **Convolution engine:** output equals direct time-domain convolution (null ≤ −120 dB) for random IR lengths up to 10 s; latency as reported; partition scheduling doesn't glitch under CPU load.
- **Next-door:** measured transmission curve matches design (slope, cutoff) ±1 dB.

**Perceptual (required — objective metrics aren't enough for spatial audio)**
- **Localization test** (≥ 12 listeners, headphones): play noise bursts at 12 azimuths × 3 elevations; measure angular error, **front-back confusion rate**, up-down confusion; targets: mean azimuth error ≤ 25° (generic HRTF), front-back confusion ≤ 35% (lower with chosen-HRTF personalization), benchmarked against a reference renderer (e.g., open-source SPARTA/IEM binaural plugins) — must be **no worse** than reference.
- **Externalization rating** (MUSHRA-style): "inside head ↔ outside head" on music & speech; target mean ≥ 60/100 with reverb on; must beat naive pan+delay baseline significantly (p < 0.05).
- **Motion smoothness** ABX: orbit at 3 speeds judged "smooth, no zipper/clicks" by ≥ 90% of listeners.
- **Timbre naturalness:** coloration rating vs bypass at ≥ "slightly different" but not "unpleasant"; check on speech & music.
- **Headphone variety:** test on ≥ 6 headphone/IEM models + phone speakers (speaker mode) for regression notes.
- **A/B choose default renderer** based on these results (document in ADR).

**Integration / E2E**
- Sound stage UI → engine: dragging a dot changes rendered azimuth within one audio block; path recording → automation lane → playback matches.
- 8D export: exported file contains correct binaural stereo; loudness/true-peak within preset; re-import and analyze orbit period via ITD tracking = configured.
- Room designer: change dimensions → IR regenerated < 500 ms (background) with crossfade to new IR, no clicks.
- Stem mode integration test (after Phase 6): vocals fixed front while drums orbit — verify via per-stem analysis.
- Mono-compatibility check meter correct on known signals.

**Performance**
- 8D with stereo input, 3rd-order ambisonics + early reflections + convolution reverb: < 25% of one core on reference phone; fallback "lite" mode (shorter HRIR, algorithmic reverb) < 8%.
- Memory for HRTF + IR sets < 50 MB loaded.

### Exit criteria
- All objective tests pass; perceptual tests meet targets with documented listener data; default renderer chosen; stage UI feels fluid (≤ 1 block parameter latency); mobile performance budget met.

---

## Phase 5 — Creative Effects & Voice Changer

**Goal:** All the fun effects — convincing, characterful, and controllable — built on Phase 1–4 primitives.

### 5.1 Tempo/pitch culture presets
- **Slowed + Reverb:** varispeed slowdown (default ~0.80–0.90×, which lowers pitch) + lush reverb (hall/plate) + gentle low-pass/shelf; sliders: speed, reverb size/mix, brightness; **"keep pitch"** toggle (time-stretch instead).
- **Sped-up / Nightcore:** varispeed up (default ~1.15–1.30×) + optional treble lift + light chorus; "keep pitch" toggle.
- **Lo-fi maker (one-slider "Lo-Fi Amount"):** macro controlling: low-pass/high-pass band limiting, soft saturation, bit-crush/sample-rate reduction (subtle), vinyl crackle, tape wow/flutter, gentle sidechain-style ducking "pump," optional room reverb, subtle noise bed. Advanced panel exposes each parameter.

### 5.2 Texture / medium simulations
- **Vinyl:** crackle/pops (sampled + procedural, seeded), surface noise, RIAA-style roll-off & rumble filter, wow & flutter (slow and fast pitch modulation with realistic spectra), mono-ization of low end, optional warp (slow pitch drift).
- **Cassette/tape:** hiss, HF loss, saturation (tape curve), wow/flutter, azimuth/stereo-phase misalignment, dropouts, compression "tape squash."
- **Old radio / AM:** band-pass (~300–3000 Hz), carrier hum/hiss, tube-style saturation, tuning drift/static, optional station-tuning sweep.
- **Phone call:** narrowband (300–3400 Hz) + codec simulation (AMR-NB/G.711 style), packet-loss artifacts option, line noise.
- **Megaphone / walkie-talkie / PA:** band-pass + hard saturation + compression + squelch tail + roger-beep option; distance-based feedback whine optional.
- **Underwater:** progressive low-pass (modulated cutoff), pitch/time wobble (slow chorus), muffled reverb, bubble textures (sampled, seeded random events), depth control.
- **Bitcrusher / sample-rate reducer / distortion/ overdrive / fuzz / waveshaper** primitives with anti-aliasing (oversampled saturation).
- **Modulation:** chorus, flanger, phaser, tremolo, vibrato, auto-pan, ring modulator, rotary speaker — with tempo sync.
- **Delay/echo:** stereo/ping-pong/tape delay with feedback filtering, tempo sync; diffusion.

### 5.3 Glitch & rhythm effects
- **Reverse** (clip or region), **reverse-reverb** (swell into hit), **stutter / beat repeat** (tempo-synced slice repeat with 1/4…1/64 divisions, probability, pitch ramp, gate), **tape stop / tape start** (smooth speed ramp to 0 with realistic curve), **scratch** (stretch), **granular freeze/time-smear**, **glitch** (random slice rearrangement, seeded), **gate/trance-gate patterns**.
- All glitch effects snap to the tempo grid when BPM is known; free-time mode otherwise.

### 5.4 Voice changer
- **Core engine:** high-quality **pitch + formant shifting** independently (e.g., PSOLA / phase-vocoder with spectral-envelope preservation; WORLD-vocoder-style analysis/synthesis for best quality on voice).
- **Presets:** Deep, Chipmunk, Child, Elder, Gender-shift (up/down: coordinated f0 + formant shift), Whisper (noise-excited resynthesis), Announcer (low formants + compression/EQ), Radio DJ, Villain, Giant, Tiny, Telephone Operator.
- **Robot / Cyborg:** ring modulation + comb filtering + vocoder; **channel vocoder** (16–32 bands) with carrier selection (saw, noise, chord); autotune-style **pitch quantization** to key/scale (hard-tune effect with retune-speed control).
- **Alien / Monster:** formant warp + pitch down + sub-harmonic generator + formant-modulated growl + distortion + reverb.
- **Neural voice conversion (optional stretch):** on-device or cloud model that converts timbre to *original* licensed synthetic voices (not real people), requires consent flow if user's own voice-clone features exist.
- **Live preview** with low-latency mode for talking into mic (≤ 100 ms web; ≤ 40 ms in the Windows app, Phase 10).
- *(No celebrity/real-person impersonation presets — see D8.)*

### Tests (Phase 5)

**Objective / unit**
- **Slowed/nightcore:** speed ratio accurate (output duration = input/ratio ±1 sample); pitch shift in cents equals 1200·log2(ratio) ±2 cents when varispeed; "keep pitch" mode pitch error ≤ ±3 cents.
- **Wow & flutter:** measured modulation spectrum matches design (e.g., wow 0.3–1 Hz, flutter 5–15 Hz; deviation in % as parameterized ±5%) using FM demodulation of a pilot tone; weighted W&F meter value within spec.
- **Vinyl/tape:** noise spectra vs target curves (hiss tilt ±1.5 dB); crackle event density (events/sec) within ±10% of setting; seeded determinism (same seed → bit-identical); loop-free over 1 h (no detectable period via autocorrelation).
- **Filters (radio/phone/megaphone):** passband −3 dB points within ±5% of nominal; phone band measured vs ITU-T P.48-style mask tolerance.
- **Distortion/bitcrush:** alias level of waveshapers with oversampling < −60 dBFS (sweep test) in "high quality"; bitcrush effective bit depth measured equals setting (histogram of quantization levels).
- **Modulation FX:** LFO rate accurate ±0.1%; tempo sync exactly equals musical division; delay times accurate ±1 sample; flanger notch frequencies match theory.
- **Reverse / tape stop:** reversed clip null-test vs `reverse(original)` bit-exact; tape stop speed curve continuous, final speed 0 reached at specified time ±1 block; no discontinuities.
- **Stutter/beat repeat:** slice boundaries land on grid within ±1 sample; repeated slices identical (bit-exact) when pitch ramp = 0; edge crossfades remove clicks (click detector = 0).
- **Granular/glitch:** seeded determinism; grain envelope sum = constant overlap (COLA check).
- **Underwater/next-door:** cutoff modulation range & rate measured ±5%.
- **Voice changer:** f0 shift accuracy ±5 cents (pitch-only); formant shift measured via LPC peaks ±3%; **formant/pitch independence**: pitch change with formant locked moves formants < 2%; intelligibility (STOI ≥ 0.85 for ±4 semitone shifts; ASR WER increase ≤ 10% absolute on speech corpus for mild presets); autotune quantization: ≥ 95% of voiced frames land within ±10 cents of target scale notes; vocoder intelligibility: ASR WER on vocoded speech ≤ 25% (initial).
- **Safety:** all effects pass universal invariants (§5.2) via the shared harness.

**Perceptual**
- MUSHRA for each presets category vs commercial references (lo-fi, tape, vinyl, voice changers): authenticity & artifact ratings ≥ 70/100; "robotic artifacts/phasiness" in voice changer rated acceptable (≥ 3.5/5) on male/female/child voices.
- Preset sanity listening on 30 diverse clips (speech, EDM, acoustic, rap, classical) — no preset allowed to produce clipping, harsh resonance, or silence (automated checks + human review).

**Integration / E2E**
- Macro slider ("Lo-Fi Amount") 0→100% produces monotonic change in measured features (HF cutoff ↓, noise ↑, bits ↓) and no clicks while dragging.
- Effect chain reorder test: moving effects changes output predictably (null vs expected order render).
- Real-time voice monitoring latency measured on reference devices ≤ target.

**Performance**
- Each effect CPU budget; worst-case chain (vinyl+tape+reverb+vocoder) within real-time budget on reference phone, else "quality: lite" auto-switch triggered and tested.

### Exit criteria
- Every effect passes invariants + specific tests; perceptual panel signs off presets; voice changer intelligibility metrics hit targets; zero clicks in glitch tools.

---

## Phase 6 — AI Restoration & Transcript Editing

**Goal:** On-device (and cloud fallback) AI that delivers professional results: stem separation, voice cleanup, de-reverb, transcription, and text-based editing.

### 6.0 ML runtime & model management (prerequisite)
- ONNX Runtime wrapper with backend selection (WebGPU → WASM-SIMD-threads → cloud), model quantization (fp16/int8) with quality-verified variants, **chunked inference with overlap-add and cross-fading** (avoid boundary artifacts), progress/cancel, memory caps, and device capability probing.
- Model manager: CDN download with SHA-256 verification, versioning, cache eviction UI ("Models: 1.2 GB"), offline availability, signed manifest.
- **Cloud job client:** upload (presigned, encrypted), job queue, polling/WebSocket progress, results download, **auto-deletion** after completion/TTL (e.g., 24 h), clear UI consent toggle per feature.
- Derived-asset caching (hash keyed); non-destructive "AI layer" per clip: toggleable, adjustable amount (wet/dry) where it makes sense.

### 6.1 Stem separation / vocal remover
- **Model:** HTDemucs-class (hybrid transformer demucs) and/or BS-RoFormer / MDX-class models; offer **4-stem** (vocals, drums, bass, other) and, if licence/quality allow, **6-stem** (+ guitar, piano). Pick by licensing (§4) and measured SDR.
- Pipeline: decode → resample to model rate → chunk with overlap (e.g., 10 s window, 25–50% overlap, windowed overlap-add) → **test-time augmentation** option ("shifts" / equivariant stabilization) for high-quality mode → post-process (phase-consistent Wiener-style soft masks, residual add-back so stems sum to the mix) → write stems at original sample rate and bit depth.
- **Outputs:** karaoke (instrumental), acapella, drumless, bassless, any stem combination with per-stem gain/mute/solo, export stems individually or as multi-track project.
- **Quality modes:** Fast (small model, quantized), Balanced, Best (ensemble/TTA, cloud).
- UX: preview the first 30 s before processing the full file; progress per stem.

### 6.2 Voice cleanup ("studio sound")
- **Denoise:** neural speech enhancement (DeepFilterNet-class for real-time; larger model for offline), strength slider, preserve-room-tone option (retain bed at an adjustable level, default −20 dB, to avoid "dead" gaps), music/non-speech protection via classifier.
- **De-reverb:** neural/dereverberation (WPE + neural postfilter) with amount control; **echo removal** (long-tail room echo).
- **Studio Sound (generative enhancement):** denoise → de-reverb → bandwidth extension → EQ matching to a "studio mic" target → de-ess → leveling; **warn when generative step might hallucinate** (flag, allow disable); adjustable amount; keep original timing exactly.
- **Speech/music-aware routing:** classifier (speech / music / mixed) auto-selects model; never apply speech enhancement blindly to music (offer "vocals-only" via stem separation → enhance → remix).
- Real-time preview on small chunk; full process offline.

### 6.3 Automatic transcription & word-level editing
- **ASR:** Whisper-class model (on-device small/medium via ONNX/whisper.cpp-like; cloud large-v3-class for accuracy) → **word-level timestamps** via forced alignment (WhisperX-style wav2vec2 alignment) → speaker diarization (pyannote-class, licence check) → punctuation/casing → language ID (≥ 50 languages supported by model).
- **Custom vocabulary / glossary** biasing for names/terms.
- **Transcript editor:** transcript pane synced with waveform; click word → seek; select text → highlights audio region; **delete words → removes audio** (non-destructive edit commands with auto crossfades); reorder paragraphs (moves audio); find/replace; speaker labels; confidence highlighting (low-confidence words flagged for review).
- **Boundary refinement:** snap cuts to nearest zero-crossing + low-energy point; room-tone fill to keep ambience continuous; optional **prosody smoothing** on joins (micro time-stretch/pitch fix across join).
- **Subtitle/caption export:** SRT, VTT, TXT, JSON (not "social" — just file outputs).

### 6.4 Filler word & disfluency removal
- Detect "um/uh/er/ah/like/you know/so/I mean" + stutters/repeats ("I I I think") using **verbatim-style ASR** (prompting/forced alignment preserving fillers) + acoustic filler classifier (to catch fillers ASR normalizes away).
- **Review UI:** list of detections with context; one-click remove all/each; sensitivity & custom word list; "shorten" vs "remove" for pauses; preview before apply.
- Joins processed with micro crossfades, room-tone patch, and breath-preserve option.
- Language-aware filler lexicons (EN first; extensible).

### 6.5 Additional AI utilities
- **Neural VAD upgrade** (Silero-class) for the Phase 3 silence remover, keeping the same UI and thresholds. **Speech/music/noise segmentation** (timeline strip), **breath detection/reduction**, **mouth-click removal**, **voice isolation** (single-speaker extraction), **loudness-aware auto-ducking** (music under speech via VAD sidechain).

### Tests (Phase 6)

**Runtime / infra**
- Model integrity: tampered file rejected (hash mismatch test); version upgrade path; corrupted download resumes.
- **Chunking invariance:** output with chunk sizes {5, 10, 30 s} and overlaps {25, 50%} differs by ≤ −40 dB (SI-SDR between outputs ≥ 40 dB) in steady state; boundary artifact detector finds none (spectral discontinuity score).
- Backend parity: WebGPU vs WASM vs cloud outputs agree (SI-SDR ≥ 35 dB between backends for same weights/precision; quantized vs fp32 within ≤ 0.3 dB SDR loss for chosen variant).
- Memory ceilings: 1-hour file processed within memory cap via streaming (peak memory excluding the model ≤ 1.5 GB desktop, ≤ 600 MB phone; initial), no tab crash; cancel frees memory within 2 s.
- Cloud: upload/encrypt/process/download round trip; deletion verified (object absent after TTL); auth/rate-limit tests; network drop & resume; no PII in logs; pen-test passes.
- Graceful degradation: simulate no-WebGPU / low-RAM → fallback chosen, user informed.

**Stem separation**
- **Objective:** MUSDB18-HQ test set with `museval`: median SDR per stem ≥ model-card baseline − 0.3 dB (regression gate) — e.g., target vocals ≥ ~8 dB, drums ≥ ~8, bass ≥ ~8, other ≥ ~5 for the HTDemucs class; record SI-SDR, SIR, SAR.
- **Reconstruction:** sum of stems vs original mix null ≤ −40 dB (with residual add-back ≤ −60 dB).
- **Length/alignment:** output length & time alignment exact (cross-correlation lag = 0 samples).
- **Sample-rate/bit-depth fidelity:** stems export at original rate, no clipping (peak check; apply safe gain/limiter if >0 dBFS and warn).
- **Edge cases:** mono files, very short (<1 s), 3-hour files, silence, heavily compressed MP3 128 kbps (quality degrade report), a-cappella/ instrumental-only input (no hallucinated stems).
- **Perceptual:** MUSHRA vs commercial separators (internal benchmark) — "vocal bleed in instrumental" & "artifacts in vocals" rated ≥ 3.5/5 on 20 diverse songs; ensure karaoke mode usable.

**Voice cleanup**
- **Denoise:** DNS Challenge / VoiceBank-DEMAND style test: PESQ, STOI, DNSMOS (SIG/BAK/OVRL) improve vs noisy input by targets (ΔOVRL ≥ +0.5, STOI not lower by > 0.01); **speech preservation** — no more than 1 dB log-spectral distortion in clean speech passed through (clean input → output SI-SDR ≥ 30 dB at strength "low").
- **Room-tone preservation** mode: noise floor in gaps stays within ±2 dB of target.
- **De-reverb:** measured RT60 of reverberant test (simulated RIRs) reduced ≥ 50%; speech clarity C50 improves ≥ 5 dB; WER improves vs reverberant input.
- **Music safety:** classifier accuracy on speech/music ≥ 95%; running denoiser on music corpus triggers warning/skip; if forced, objective degradation recorded.
- **Hallucination guard (Studio Sound):** ASR WER of enhanced vs original (on clear speech) must not increase > 1% absolute; phoneme-alignment check (forced-aligner timings change < 10 ms median).
- **Timing:** output sample-count identical to input.

**Transcription & editing**
- **WER** on LibriSpeech test-clean/other, TED-LIUM, noisy podcast clips, accented English, plus ≥ 5 other languages (Fleurs/CommonVoice subsets): targets per model card; regression gate (≤ +0.5% abs vs baseline).
- **Word timestamp accuracy:** vs forced-aligned ground truth (TIMIT/Buckeye-style): median error ≤ 40 ms; 90th percentile ≤ 120 ms.
- **Diarization:** DER ≤ target (e.g., ≤ 15% on AMI/VoxConverse-like data).
- **Text-delete → audio-edit:** deleting word w removes audio span [start,end] ±10 ms; listening tests: no clipped phonemes in ≥ 98% of edits (automated: energy at boundaries; manual spot-check 200 edits).
- **Undo/redo of transcript edits** returns exact audio.
- **Subtitle export:** SRT/VTT validity (parsed by validator), timing monotonic, line-length rules.
- **Large transcripts:** 3-hour podcast transcript editor stays at 60 fps (virtualized).
- **Privacy:** on-device mode makes zero network requests (verified by network-interception test).

**Filler removal**
- Detection precision/recall on labeled corpora (SwitchBoard-style disfluency labels + own labeled podcasts): precision ≥ 0.90, recall ≥ 0.85 for "um/uh"; stutter/repeat detection F1 ≥ 0.75.
- **False-positive guard:** legitimate words ("like" as verb, "so" connective) preserved in ≥ 95% of occurrences at default sensitivity.
- **Edit quality:** after removal, join naturalness MUSHRA ≥ 75/100; no rhythm "jump" (inter-word gap distribution after edit within natural range).
- Review UI E2E: apply all / reject some / undo.

**Performance**
- RTF targets: denoise real-time on mid phones (RTF < 0.5), stem separation (fast mode) RTF < 1.0 on laptop WebGPU, Best mode via cloud < 0.3×duration queue+process; Whisper small RTF < 0.5 on laptop.
- Thermal tests on a phone browser and a thin laptop (30-min processing doesn't throttle to unusable).

### Exit criteria
- Objective metrics at/above model-card baselines in the app pipeline (not just in research code); perceptual panel approval; privacy test passes; cloud fallback & deletion verified.

---

## Phase 7 — Advanced AI: Analysis, Mastering, TTS, Dubbing, Magic Button

**Goal:** Finish the feature set: key/BPM, auto-mastering, text-to-speech, translation + dubbing, and the one-tap "Magic" assistant.

### 7.1 Key & BPM detection
- **BPM/beat tracking:** neural beat/downbeat tracker (e.g., Beat This!-class, or TCN-based) + tempo curve estimation (handles tempo changes), half/double-time disambiguation, confidence score, manual tap-tempo override, **tempo map** stored in project, beat grid overlay, snap-to-beat editing, "warp clip to grid."
- **Key detection:** CQT/chroma → CNN/Krumhansl-profile ensemble; outputs key + mode + confidence + Camelot/Open Key notation; handles modulation (segment-level key timeline); tuning-frequency estimation (e.g., A=432/440/442).
- **Section detection (stretch):** intro/verse/chorus boundaries for quick navigation.
- Uses: sync-to-BPM for 8D/stutter, DJ-friendly metadata export (tag write).

### 7.2 Auto-mastering ("one tap to sound polished")
- **Analysis:** genre/loudness/spectral balance/dynamics/stereo-width/ true-peak/ vocal-prominence analysis.
- **Processing chain (adaptive):** corrective EQ (resonance/mud detection) → tonal-balance EQ toward target curve per genre (or **reference-track matching**: user supplies reference; match EQ + loudness + width) → multiband compression → stereo enhancement (mono-safe) → saturation (optional) → **true-peak limiter** to target LUFS (−14 default streaming; selectable) → dithered export.
- **Controls:** intensity (Gentle/Balanced/Loud), loudness target, "preserve dynamics" toggle, before/after **loudness-matched A/B** (essential to avoid "louder = better" bias), explanation panel ("boosted 3 kHz +1.5 dB, reduced 250 Hz −2 dB, GR avg 2.1 dB").
- **Safety rails:** never exceed true-peak ceiling; avoid over-compression (DR floor); skip/lighten if input already loud/crushed; warn about clipping/inter-sample peaks; mono-compat check.
- Mastering for speech mode (podcast/voice) with its own chain & loudness spec.

### 7.3 Text-to-speech (voiceover)
- **Engine:** neural TTS (on-device lightweight model + cloud high-quality voices); multi-language; multiple *licensed* voices; SSML-like controls (pauses, emphasis, rate, pitch); pronunciation dictionary; long-form chunking with prosody continuity.
- **Insert into timeline** as a clip; regenerate segments independently; word timestamps returned for transcript sync; output labeled as synthetic in metadata.
- **Voice cloning (optional, gated):** requires verified consent (recorded consent phrase + attestation), liveness/consent check where feasible, rate limits, ToS enforcement, **inaudible watermark + metadata tag** on generated audio, abuse-report channel. Disabled by default; legal review required before launch (see §4).

### 7.4 Auto-translation & dubbing
- **Pipeline:** transcribe (Phase 6) + diarize → machine translation (NMT/LLM, with glossary & tone settings) → **editable translated script** → TTS per speaker (voice-matched or user-chosen voice) → **duration fitting** (adjust speaking rate via TTS rate/time-stretch within ±15%; propose script shortening via MT when over) → lip-sync not required (audio-only) → **mix with original background** (use stem separation: replace vocals, keep music/effects; keep room-tone) → loudness/EQ match to original → export.
- Speaker consistency across segments; non-speech sounds (laughter, breaths) preserved from original where possible.
- **Quality gates:** per-segment review UI with original/translated waveform and text; re-generate individual lines; human-in-the-loop editing before final render.
- Supported language pairs launched incrementally; clear "machine translation may be inaccurate" notice.

### 7.5 One-tap "Magic" button
- **Analyzer** classifies content (speech/music/mixed/noisy/phone recording/etc.), measures issues (noise, reverb, clipping, loudness, sibilance, clicks, imbalance), and **proposes an effect chain** (as an editable chain, never a black box): e.g., for noisy phone interview → hum/noise removal → de-reverb → EQ "Clear Voice" → de-ess → leveler → limiter −16 LUFS.
- Shows **before/after with loudness-matched toggle** and a "why" list; each step can be toggled.
- Goal presets: "Make it clear," "Make it loud," "Make it dreamy (slowed+reverb)," "Make it 8D," "Podcast-ready," "Music-ready."
- Learns from nothing without consent; optional on-device preference memory (local only).

### Tests (Phase 7)

**BPM / key**
- **Tempo:** Acc1/Acc2 on Ballroom/GTZAN-Tempo/SMC ≥ published baselines (e.g., Acc1 ≥ 80%, Acc2 ≥ 92%); beat F-measure ≥ 0.85 (±70 ms window) and CMLt/AMLt reported; downbeat F ≥ 0.75; tempo-change files tracked within ±2 BPM.
- **Synthetic ground truth:** click tracks at 40–240 BPM with swing → error ≤ 0.05 BPM; time-signature variants (3/4, 6/8, 7/8).
- **Key:** MIREX weighted score ≥ 0.70 on GiantSteps/GTZAN-Key; tuned-shifted (±50 cents) detection stable; modulation timeline accuracy on synthetic modulating progressions.
- Beat grid UI snap tests; warp-to-grid output beats land within ±5 ms of grid.

**Auto-mastering**
- **Loudness/peak compliance:** output within ±0.2 LU of target & true-peak ≤ ceiling on 200-track diverse set (100%); never clips.
- **Spectral balance:** post-master tonal-balance distance to target curve decreases ≥ 30% vs input on "dull/boomy" test tracks.
- **Dynamics guard:** DR (PLR/LRA) loss ≤ configured limit; tracks already at −8 LUFS are not made louder.
- **Reference matching:** 1/3-octave spectrum distance to reference ≤ 1.5 dB RMS after match; loudness matched to reference ±0.3 LU.
- **Idempotence-ish:** mastering a mastered track (default settings) changes spectrum < 1 dB and DR < 0.5 dB (no runaway).
- **Perceptual:** MUSHRA vs original (loudness-matched) and vs 2 commercial auto-mastering references: preferred or equal ≥ 70% on 30 tracks; mastering engineer review of 20 tracks (pass/fail checklist).
- Explanation panel values match actual measured changes (±0.5 dB).
- Mono-compat: summed mono vs stereo loudness difference < 3 dB (unless source already wide).

**TTS**
- **Intelligibility:** ASR WER on TTS output vs input text ≤ 3% (EN) / language-specific targets; MOS/UTMOS estimate ≥ 4.0 (neural MOS predictor) + human MOS ≥ 4.0 on 50 sentences.
- **Control accuracy:** pause durations ±50 ms; rate control within ±5% of requested.
- **Long-form stability:** 30-minute script — no drift, no repeated/skipped sentences (forced-alignment check), consistent loudness (±1 LU).
- **Pronunciation dictionary** overrides applied (test cases incl. acronyms, numbers, dates, currencies).
- **Voice-clone safeguards:** consent flow cannot be bypassed (automated test with spoofed attestation), watermark detection ≥ 99% after MP3 128 k re-encode/resample/noise 20 dB SNR, metadata tag present; rate limits enforced; abuse-report E2E.
- Latency: first-audio-chunk streaming < 2 s cloud.

**Translation & dubbing**
- **Translation quality:** BLEU/COMET on FLORES-style set per language pair ≥ baseline; named entity & glossary adherence 100% on glossary test set.
- **Duration fit:** ≥ 95% of segments within ±15% of source duration after fitting (or flagged); total drift < 1 s per 10 min through re-timing.
- **Mix quality:** dubbed speech vs background loudness ratio matches original within ±2 LU; no vocal bleed from original speech > −35 dB below dubbed track (measured via source separation residual).
- **Speaker consistency:** speaker-embedding cosine similarity within a speaker's segments ≥ 0.8 (voice-matched mode).
- **E2E:** upload 5-min bilingual sample → translated, edited script → rendered dub → human bilingual review pass/fail rubric (accuracy, naturalness, timing, mix).
- Failure paths: unsupported language, missing stems, API timeout → clear UX, partial results retained.

**Magic button**
- **Diagnosis accuracy:** issue detectors (noise, reverb, clipping, hum, sibilance, low loudness) precision/recall ≥ 0.9 on labeled problem corpus.
- **Outcome tests:** after Magic on 100 flawed clips: objective improvement on targeted metrics (DNSMOS, LUFS in range, clipping removed) in ≥ 90%; **no regressions** (no case worse than input on STOI/DNSMOS beyond tolerance); perceptual test: loudness-matched preference for processed ≥ 75%.
- Chain is editable & reversible (undo restores original exactly).
- Idempotence guard: running Magic twice does not over-process (second pass applies ≤ minor changes).

### Exit criteria
- Feature complete; each AI feature meets objective & perceptual targets; consent/watermark/safety controls verified; legal sign-off on voice features.

---

## Phase 8 — Pro Workflow, UX Polish & Accessibility

**Goal:** Turn the feature set into a cohesive, fast, delightful product that beginners enjoy and pros trust.

### 8.1 Effect chains & rack
- Drag-reorder effect chain per clip/track/master; bypass, solo-effect, wet/dry per effect; chain presets (local save/import/export `.fluxpreset`); **A/B/C compare slots**; **before/after hold-to-compare** with optional loudness-matching; macro controls (one slider mapped to multiple parameters); parameter randomize/morph (stretch).
- **Effect browser** with categories, search, favorites, recent; each effect has an **animated icon / mini waveform preview** (rendering a 2-second demo through the effect, cached) — "visual feedback for every effect."
- Preview-without-commit: hover/press-and-hold previews an effect on the selection.

### 8.2 Beginner vs Pro modes
- Every effect since Phase 3 already has simple and advanced surfaces (Principle 8). This phase adds the global switch and the task-first home.
- **Simple mode:** big task-oriented actions ("Remove noise," "Make 8D," "Slowed + reverb," "Vocal remover," "Clean up voice," "Convert format"), single-slider controls, plain-language labels, built-in tips.
- **Pro mode:** full parameter access, spectral views, routing, automation, numerical entry, meters, shortcuts, multi-select, track groups.
- One-toggle switch preserving state; progressive disclosure within each effect.

### 8.3 Workflow tools
- **Batch processing:** apply a chain/preset to many files (normalize, convert, denoise, trim silence), queue with progress, error report; **watch-folder** (desktop stretch).
- **Ringtone maker:** select region (with preview & fade), enforce platform limits (iOS m4r ≤ 30 s; Android MP3/OGG/M4A with length options), loudness-normalize for ringtone use, export & guided install instructions (platform-specific; share-sheet / save-to-files — **local only**, not social).
- **Macros/Actions:** record sequence of operations → reusable action.
- **Templates:** Podcast, Music, Voiceover, Lecture, Interview, 8D-Music project templates.
- **Project management:** recent projects, search, folders, storage-usage manager, model manager.
- **Import/export interchange (stretch):** stems pack, multitrack WAV bounce, AAF/OMF/EDL (pro), CUE sheet, iXML/BWF metadata.

### 8.4 Visual design & UX
- Audit and finish the design system per DESIGN.md, which is built from Phase 0 and used by every UI since then (it cannot wait until now): tokens, all 13 themes, all 6 fonts, motion guidelines (60 fps), consistent iconography, responsive layouts. Add the onboarding tour (interactive, skippable), contextual help, empty states, error messages that explain and offer actions.
- Touch-friendly handles, one-handed reachability for bottom sheets.
- **Headphone check** wizard (from Phase 4) integrated in onboarding and when enabling spatial effects.
- Undo-history timeline view (from Phase 2) polished with thumbnails.

### 8.5 Accessibility & localization
These apply from the first UI commit (axe-core runs in E2E from Phase 2, strings are externalized from day one). This phase is the full audit and the translation work.

- WCAG 2.2 AA: keyboard operability, focus management, ARIA live regions for transport/meters (throttled), high-contrast theme, reduced-motion, scalable text, color-blind-safe palettes, **audio descriptions of waveforms (text summary)**, screen-reader-friendly transcript editor.
- i18n: ICU message formats, RTL support, locale-aware number/time formatting; initial languages beyond English (e.g., Hindi, Spanish, French, German, Portuguese, Japanese) as scoped.

### 8.6 Settings & safety
- Audio device selection, buffer size, sample rate, quality-vs-battery mode, privacy controls (per-feature cloud consent, data deletion), telemetry opt-in, **hearing-safety features:** output limiter option, "loud export" warnings, volume-spike protection on playback start (soft-start ramp).

### Tests (Phase 8)

**Functional / E2E**
- Effect chain: reorder/bypass/wet-dry produce expected renders (null vs reference graph), A/B switching click-free (click detector = 0), hold-to-compare is level-matched within ±0.2 LU when enabled.
- Effect previews: demo renders deterministic; cache invalidates on effect version change.
- Beginner ↔ Pro toggle preserves project state exactly (hash equality).
- Batch: 200 files processed with 0 failures on valid input; invalid files reported not fatal; output metrics (LUFS etc.) correct; parallelism doesn't exceed memory cap.
- Ringtone maker: output meets iOS/Android constraints (length, format, sample rate) — validated by platform tools; loop point click-free; E2E on real devices.
- Macros: recorded action replays identically (project hash).
- Templates open with expected structure.

**UX / usability**
- **Moderated usability tests** (≥ 12 participants: 4 beginners, 4 creators, 4 pros) on 8 core tasks; target: task success ≥ 90%, SUS ≥ 80, median time-to-first-8D-export < 90 s for beginners.
- Onboarding completion rate and drop-off instrumented (opt-in) and reviewed.
- First-run experience on slow network/old device.

**Accessibility**
- axe-core: 0 critical/serious violations on all main screens; manual screen reader tests (NVDA, VoiceOver, TalkBack) of 6 core flows; keyboard-only completion of all core tasks; contrast ≥ 4.5:1; reduced-motion honored.

**Localization**
- Pseudo-localization test (expanded strings, RTL mirror) — no clipped/overlapping UI; all strings externalized (lint check); locale formatting tests.

**Visual regression**
- Snapshot tests for all screens × themes × breakpoints.

**Safety**
- Volume-spike protection: playback start ramp measured (ramp ≥ 10 ms with no step above −60 dBFS relative to target; initial); output limiter prevents > ceiling in worst-case test.

**Performance**
- UI interaction latency (INP) < 200 ms; app cold start < 3 s on mid-range phone (cached); JS + WASM heap ≤ 1 GB excluding loaded models and with no upward trend in a 30-minute editing session; no layout thrash.

### Exit criteria
- Usability and accessibility targets met; all pro/beginner flows complete; batch/ringtone features verified on real devices.

---

## Phase 9 — Hardening, Packaging & Launch

**Goal:** Stability, security, compatibility, performance on real devices, and a controlled release.

### 9.1 Platform packaging
- **Web:** PWA (offline, installable), COOP/COEP headers for SharedArrayBuffer, service worker caching strategy for WASM/models, graceful fallback without SAB/WebGPU.

### 9.2 Performance & reliability
- Profile & optimize hotspots (SIMD, cache layout, FFT plan reuse), adaptive quality governor (auto-lowers quality on xruns), memory pooling, startup time, model load time.
- Chaos testing: kill during export, disk full, permission revoked, device sleep/wake, audio device hot-plug, browser tab throttling/backgrounding, low-memory kills.

### 9.3 Security & privacy
- Threat model (STRIDE) for cloud, project files, model downloads; dependency scanning; CSP; sandboxed workers; signed model manifests; upload size/type limits; SSRF/path-traversal protection; rate limiting; secrets management.
- Privacy review: data-flow audit, retention TTLs, DSAR/deletion flows, regional hosting options, DPAs; third-party model/API data-handling terms checked.
- External penetration test + fixes before public beta.

### 9.4 Release program
- **Alpha** (internal dogfooding with weekly listening sessions) → **closed beta** (200–1000 users; crash/xrun telemetry opt-in; feedback in-app, not social) → **public beta** → GA with staged rollout + kill switches (remote feature flags, especially for AI/voice features).
- Documentation: user guide, tutorials per feature, FAQ, troubleshooting ("no sound," "clicks while playing," "spatial audio sounds flat"), developer docs for preset format.
- Support tooling: diagnostic bundle export (logs without audio), version/build info, issue templates.

### Tests (Phase 9)

- **Device/browser matrix:** latest-2 versions of Chrome/Edge/Firefox/Safari on Windows 10/11, macOS and Ubuntu, plus iOS Safari and Android Chrome (low-end, mid, flagship); verify playback, edit, export, spatial render, ML on each; known-limitations table maintained.
- **Soak:** 4 h continuous playback + edits + autosave; memory growth < 5%; 0 xruns on reference devices; 24 h batch run.
- **Large-scale regression:** run the complete nightly objective suite on release candidate; compare against previous release; any metric regression > tolerance blocks release.
- **Golden-master listening:** engineers/listeners compare RC vs previous release renders of 50 presets/clips (blind).
- **Chaos suite** results: 0 data-loss incidents across 1,000 randomized fault injections.
- **Fuzzing:** 72 h cumulative per parser; 0 open crashes.
- **Security:** pen-test report with 0 critical/high open; dependency audit clean; model-supply-chain test (tampered model blocked).
- **Load tests (cloud):** 500 concurrent jobs; queue latency SLO (p95 < 60 s start); autoscaling; cost per job tracked; failure injection (worker crash → job retried, no data loss).
- **Upgrade tests:** project compatibility across app versions; model version upgrades don't invalidate user projects silently.
- **Legal/compliance checklist:** licences, attributions screen (HRTF datasets, IRs, models), privacy policy, ToS (voice cloning).
- **Accessibility re-audit** on release build.
- **Launch readiness review** against Appendix C.

### Exit criteria
- All release gates green; beta crash-free sessions ≥ 99.5%; xrun rate below threshold; security & legal sign-off; rollback & kill-switch rehearsed.

---

## Phase 10 — Native Desktop Ecosystem (Windows, macOS & Linux)

> **Updated per User Direction (2026-11-03):** Transition Flux to a **100% native desktop application suite** across Windows, macOS, and Linux. The web version of the editor is retired; the web domain (`flux.solarquack.in`) serves strictly as the product showcase landing page and download portal. The editor runs natively with zero web-view/browser-wrapper overhead, utilizing operating system audio APIs and hardware acceleration directly.
>
> **Pending User Confirmation:** This phase will begin execution once the user reviews and confirms this updated specification.

**Goal:** Standalone, self-contained native desktop applications. For Windows and macOS: unpack a single zip and run the app with all dependencies, codecs, and runtimes bundled. For Linux: compile natively from source code via a single `make` command.

### 10.1 The Desktop Packages

#### Windows Portable Package
```
Flux-<version>-win-x64.zip
└── Flux/
    ├── Flux.exe               (Native OS application, static C runtime)
    ├── *.dll                  (DirectML, ONNX Runtime, native audio codecs)
    ├── tools/ffmpeg.exe       (LGPL build bundled locally)
    ├── data/                  (bundled models and assets)
    ├── licenses/              (notices and source offers)
    ├── portable.flag          (portable mode indicator)
    └── README.txt
```

#### macOS Universal Package
```
Flux-<version>-macos-universal.zip
└── Flux.app/
    ├── Contents/
    │   ├── MacOS/Flux         (Universal Mach-O binary: arm64 + x86_64)
    │   ├── Frameworks/        (Native dylibs & Metal shaders)
    │   ├── Resources/         (Icons, fonts, audio assets)
    │   └── Info.plist
    ├── licenses/
    └── README.txt
```

#### Linux Native Source Build (Makefile)
```bash
git clone https://github.com/solarquack/flux.git
cd flux
make
./bin/flux
sudo make install # Optional: installs to /usr/local/bin with desktop entry
```

### 10.2 Engineering Rules & Native Capabilities

- **Zero Web Packaging Overhead:** The application is native to the host operating system, binding directly to native windowing, input events, and hardware audio subsystems rather than a browser sandbox.
- **Self-Contained Zips (Windows & macOS):** No external installer, no administrator rights, no system dependencies on `PATH`. Everything needed to run is located relative to the application binary.
- **Native Audio Subsystems:**
  - **Windows:** WASAPI exclusive & shared mode via `cpal`, optional low-latency ASIO driver support; device hot-plug.
  - **macOS:** CoreAudio low-latency HAL stream integration; automatic aggregation and Bluetooth/AirPlay routing.
  - **Linux:** Native PipeWire, JACK, and ALSA drivers with real-time thread scheduling (`SCHED_FIFO`).
- **Native Hardware ML Acceleration:**
  - **Windows:** DirectML GPU acceleration and CUDA support with CPU fallback.
  - **macOS:** Apple Neural Engine (ANE) and Metal Performance Shaders (MPS).
  - **Linux:** OpenVINO and AVX-512 / AVX2 SIMD CPU kernels.
- **Native File Access & Durability:** Real operating system folders, file descriptor streaming, no browser storage quotas, zero cloud data transfer.
- **GitHub Releases Automation:** `.github/workflows/release.yml` automatically compiles, packages, and publishes the Windows and macOS ZIPs to GitHub Releases on tag push.

### Tests (Phase 10)

**Automated**
- Native DSP engine passes every §5.2 universal invariant on host operating systems.
- Build scripts produce standalone ZIPs (`just package-windows`, `just package-macos`, `make`).
- Dependency audit verifies that zero unbundled dynamic libraries are required.
- Launch test with sanitized empty `PATH`: all tools and codecs are located relative to the executable.

**Real Device [R]**
- **Clean-machine test (Windows):** Fresh Windows 10 & 11 VM (no VC++ redistributable): unzip, double-click `Flux.exe`, verify edit and export.
- **Clean-machine test (macOS):** Fresh macOS 13–15 machine: unzip, double-click `Flux.app`, verify CoreAudio and Metal rendering.
- **Clean-machine test (Linux):** Fresh Ubuntu/Arch/Fedora install: run `make`, verify binary runs and binds to PipeWire/ALSA.

### Exit Criteria
- Windows portable zip verified on clean Windows VM.
- macOS universal zip verified on Apple Silicon and Intel machines.
- Linux `make` compiles cleanly from fresh git checkout.
- GitHub Actions release pipeline verified.
- User confirmation received.

---

## Feature → Phase Traceability Matrix

| # | Feature (from brainstorm) | Phase |
|---|---------------------------|-------|
| 1 | 3D audio (static placement) | 4 |
| 2 | 8D audio (auto-orbit) | 4 |
| 3 | 16D / orbit mode (spirals, figure-eights) | 4 |
| 4 | Draggable sound stage + draw-a-path | 4 (UI polish in 8) |
| 5 | Room simulation (cathedral, bathroom, cave, garage…) | 4 |
| 6 | Concert hall "row 10" mode | 4 |
| 7 | Underwater | 5 |
| 8 | Phone / old radio / megaphone / walkie-talkie | 5 |
| 9 | Vinyl / cassette | 5 |
| 10 | Robot / alien / monster voices | 5 |
| 11 | Slowed + reverb, sped-up (nightcore) | 5 |
| 12 | Lo-fi maker (one slider) | 5 |
| 13 | Reverse, stutter, tape-stop, glitch | 5 |
| 14 | "Next door" effect | 4 |
| 15 | Voice changer (deep, chipmunk, gender-shift; *no celebrity*) | 5 |
| 16 | Trim, cut, split, merge, fades, crossfades | 2 |
| 17 | Noise removal | 3 (classical), 6 (AI) |
| 18 | Auto-silence remover | 3 (neural upgrade in 6) |
| 19 | Volume normalizer & leveler | 3 |
| 20 | Echo/reverb removal | 6 |
| 21 | Speed change without pitch change | 3 |
| 22 | Equalizer + simple presets | 3 |
| 23 | Format converter (MP3/WAV/M4A…) | 1 (batch in 8) |
| 24 | Waveform editor with pinch-to-zoom | 2 |
| 25 | Vocal remover / stem separation | 6 |
| 26 | Auto-transcription + delete-words-to-edit-audio | 6 |
| 27 | Filler word removal | 6 |
| 28 | Voice cleanup ("studio sound") | 6 |
| 29 | Auto-translation & dubbing | 7 |
| 30 | Key & BPM detection | 7 (manual tempo and tap in 3) |
| 31 | Auto-mastering | 7 |
| 32 | Text-to-speech voiceover | 7 |
| 33 | Ringtone maker | 8 |
| 34 | Preset packs (local files only; *community sharing excluded*) | 8 |
| 35 | Before/after toggle | 8 (A/B engine in 3) |
| 36 | One-tap "magic" button | 7 (UX in 8) |
| 37 | Effect chains (stack + reorder) | 1 (graph) / 3 (basic rack) / 8 (full UX) |
| 38 | Visual feedback / animated icons per effect | 8 |
| 39 | Undo history with visual timeline | 2 (polish 8) |
| 40 | Headphone check | 4 / 8 |
| 41 | Beginner & Pro modes | 8 |
| — | *Excluded:* share-as-video, community filters, challenges | Out of scope |

---

## Risks & Mitigations

| Risk | Likelihood / Impact | Mitigation |
|------|--------------------|------------|
| Real-time glitches in AudioWorklet on low-end phones | High / High | Spike A early; adaptive quality governor; "lite" DSP paths; honest Tier 3 scope (§1.1) |
| Large ML models too heavy on-device | High / Med | Hybrid cloud fallback; quantized variants with quality gates; chunked streaming |
| Model/dataset licences block commercial use | Med / **Critical** | Licence audit in Phase 0; prefer permissive/retrainable models; budget for commercial licences; be ready to swap models behind a stable interface |
| Spatial audio sounds flat with generic HRTF | High / High | HRTF selection test, ambisonic renderer, binaural early reflections, headphone compensation; set honest expectations in UI |
| Musical-noise / artifacts in denoise & stems | Med / High | Objective + perceptual gates; wet/dry controls; "Best" cloud mode |
| Voice cloning misuse / legal exposure | Med / **Critical** | Consent flow, watermark, ToS, rate limits, legal review; ship disabled by default; drop real-person presets |
| Browser limits (SAB/WebGPU/memory) | Med / Med | Progressive enhancement, feature detection, Windows app later (Phase 10) |
| Scope too large for the team | **High** / High | Strict phase gates; each phase independently shippable; AI features behind flags; cut stretch items first |
| Cloud costs spiral (GPU) | Med / Med | On-device first, quotas, queue prioritization, cost-per-job dashboards, caching of derived assets |
| Quality regressions unnoticed | Med / High | Golden/null/metric gates in CI; nightly dataset evals; listening sessions each release |
| Time-stretch/pitch library licence (Rubber Band GPL) | Med / Med | Default to permissive Signalsmith Stretch or purchase licence |
| Project file corruption / data loss | Low / **Critical** | Journaling, autosave, migrations tested, chaos tests |
| Browser evicts OPFS projects | Med / **Critical** | `persist()` request, File System Access where available, backup prompts, Spike G |
| Agent marks unverifiable quality as done | Med / High | Gate types (§5.4), threshold discipline (§5.5), human-gate queue in PROGRESS.md |
| Name conflict with FLUX:: audio software | Med / High | Trademark search before launch (§4); name kept in one constant |
| Web lacks a codec on some browsers (AAC encode) | High / Low | Feature detection, honest UI, Spike F |
| Hearing damage from loud output | Low / High | Output limiter, spike protection, loud-export warnings |

---

## Rough Timeline & Team

*Estimates assume a team of ~8–10 (2 DSP/Rust, 2 frontend, 1–2 ML/audio-ML, 1 backend/infra, 1 designer, 1 QA/test-automation, 1 product/audio-domain lead) and are intentionally rough. Re-estimate after Phase 0 spikes. With AI agents (D9) calendar time will differ; treat this table as relative effort and judge progress by PROGRESS.md.*

**Recommended release milestones:** **M1 Flux Alpha** after Phase 2 (non-destructive editor). **M2 "8D & Slowed + Reverb"** after Phase 4 + §5.1 (first public release, validates market fit). **M3 Flux AI** after Phase 7. **M4 GA** (web) after Phase 9. **Windows portable app** after that, only on request (Phase 10).


| Phase | Duration | Cumulative | Shippable milestone |
|-------|---------|------------|---------------------|
| 0 Foundations | 3–5 weeks | ~1 mo | Harness + spikes + decisions |
| 1 Core engine & I/O | 8–10 weeks | ~3.5 mo | Playback/record/export engine (CLI + minimal UI) |
| 2 Editor UI & timeline | 8–10 weeks | ~6 mo | **Alpha:** non-destructive editor |
| 3 Core processing | 8–10 weeks | ~8.5 mo | Pro basics: EQ, dynamics, time/pitch, denoise, silence remover |
| 4 Spatial audio | 8–10 weeks | ~11 mo | **Beta-1:** 3D/8D + rooms (headline feature) |
| 5 Creative FX & voice | 6–8 weeks | ~13 mo | Fun effects + voice changer |
| 6 AI restoration & transcript | 10–12 weeks | ~16 mo | Stems, studio sound, transcript editing |
| 7 Advanced AI | 10–12 weeks | ~19 mo | Mastering, TTS, dubbing, Magic |
| 8 Pro workflow & polish | 6–8 weeks | ~21 mo | Feature complete |
| 9 Hardening & launch | 8–10 weeks | ~23 mo | **GA (web)** |
| 10 Windows portable (on request) | 4–6 weeks | n/a | **`Flux-win-x64.zip`** |

**Parallelization tips:** Phases 4–5 (DSP team) can overlap with Phase 6 (ML team) after Phase 3; UI polish (Phase 8) can start incrementally from Phase 2. Consider shipping an early public **"8D & Slowed+Reverb" focused release** after Phase 4 to validate market fit (with Phase 3's basics).

---

## Appendix A — Test Corpus

Assemble with documented licences (`corpus.yaml`: id, source, licence, SHA-256, duration, SR, channels, tags, expected-test-uses). Corpus audio stays outside git except small, self-generated synthetic files; the manifest is committed. See §4 for dataset use restrictions.

| Set | Contents | Used for |
|-----|----------|----------|
| **Synthetic** | Sines, log sweeps, multitones, impulses, white/pink noise, clicks, chirps, square/saw (aliasing tests), DC, full-scale, hot-float (±4.0), silence, denormal-trigger signals | DSP unit/property tests |
| **Reference codecs** | Same program encoded at many settings: MP3 (CBR/VBR, LAME gapless), AAC, Opus, Vorbis, FLAC, WAV variants (8/16/24/32f, RF64) | Decode/gapless/seek/export tests |
| **Edge-case files** | Truncated, corrupt headers, odd sample rates (8k–384k), 1-sample, 0-length, >4 GB, huge metadata, 7.1 channel, mono, DSD (reject gracefully) | Fuzz/robustness |
| **Loudness** | EBU Tech 3341/3342 conformance signals (use per licence), ISP test files | Meters, normalize, limiter |
| **Speech** | LibriSpeech, Common Voice, VCTK, TED-LIUM, noisy podcasts, accented English, ≥ 5 other languages (Fleurs), disfluency-labeled data, phone-quality, reverberant | ASR, VAD, denoise, filler, voice changer, TTS eval |
| **Noise/Reverb** | DNS Challenge noise sets, DEMAND, RIR datasets (e.g., OpenAIR/BUT ReverbDB — licence check) | Denoise/de-reverb |
| **Music** | MUSDB18-HQ, MedleyDB (licence check), GTZAN, Ballroom, GiantSteps, SMC, own-licensed multi-genre set (EDM, rock, hip-hop, classical, acoustic, jazz, low-bitrate, loud-masters) | Stems, BPM/key, mastering, effects |
| **Spatial** | HRTF sets (SADIE II, KEMAR, CIPIC, ARI), reference binaural renders from SPARTA/IEM, localization stimuli (noise bursts, speech) | Spatial tests |
| **IRs & ambiences** | Licensed or self-recorded IRs (halls, rooms, spaces), crowd/applause/vinyl/tape/bubble sample sets | Reverb/room/FX |
| **Problem clips** | Hand-curated flawed recordings (hum, wind, clipping, echo, sibilance, phone call, uneven levels) with labels | Magic button, restoration |
| **Golden outputs** | Reviewed reference renders per effect/preset with metadata of settings & seed | Regression |

---

## Appendix B — Quality Metrics Glossary

| Metric | Meaning |
|--------|---------|
| **LUFS / LU / LRA** | Integrated loudness (BS.1770), loudness units, loudness range |
| **True peak (dBTP)** | Peak level of the reconstructed (oversampled) signal; prevents inter-sample clipping |
| **THD+N** | Total harmonic distortion plus noise |
| **SI-SDR / SDR / SIR / SAR** | (Scale-invariant) signal-to-distortion/interference/artifact ratio for separation |
| **PESQ / STOI / DNSMOS / ViSQOL / UTMOS** | Speech/audio quality & intelligibility estimators |
| **WER / CER** | Word/character error rate (ASR) |
| **DER** | Diarization error rate |
| **BLEU / COMET** | Machine-translation quality metrics |
| **Acc1 / Acc2 / F-measure / CMLt / AMLt** | Tempo and beat-tracking metrics |
| **MIREX key score** | Weighted score for key detection (exact/fifth/relative/parallel credits) |
| **LSD** | Log-spectral distance |
| **ITD / ILD** | Interaural time / level differences |
| **RT60 / C50** | Reverberation time / speech clarity index |
| **RTF** | Real-time factor = processing time / audio duration (lower is faster) |
| **Xrun** | Audio buffer underrun/overrun (glitch) |
| **MUSHRA / ABX** | Standard perceptual listening-test methodologies |
| **Null test** | Subtract two signals; residual level shows how identical they are |
| **PDC** | Plugin delay compensation |
| **COLA** | Constant overlap-add (windows sum to constant) |

---

## Appendix C — Definition of Done

A feature is **Done** only when **all** are true. For agent work, items that need a person or real hardware (§5.4) are queued in PROGRESS.md: the feature can be committed as agent-complete, but it is not released until the queue clears.

1. **Spec & UX:** Requirements, simple + advanced UI, empty/error states, and copy are approved.
2. **Implementation:** Code reviewed; real-time safe (no allocs/locks in audio path); runs in both real-time and offline modes with identical output.
3. **Universal invariants (§5.2)** pass for every DSP node it adds.
4. **Feature-specific tests** from its phase section are implemented and green in CI (unit, property, golden, null, objective, integration, E2E as applicable).
5. **Quality targets** met and recorded (metric values stored in the trend dashboard).
6. **Perceptual sign-off** completed where the phase requires it (ABX/MUSHRA results archived).
7. **Performance:** CPU/memory/latency within budget on reference devices; "lite" fallback exists where needed.
8. **Accessibility** checks pass; strings localized-ready.
9. **Security/privacy/legal** reviewed (licences, data flow, consent, retention).
10. **Docs:** user doc, tooltip text, preset docs, ADR updates, changelog.
11. **Observability:** errors and key performance counters instrumented (opt-in telemetry), feature flag in place.
12. **Rollback plan** exists (flag/kill-switch) and was tested for AI and cloud features.

---

## Appendix D — Revision History

**Revision 2 (2026-11-03).** Named the product Flux and hardened the plan:

- **Dependency-order fixes:** basic rack moved to Phase 3 (effects had no UI until Phase 8); neural VAD moved after the ML runtime; manual tempo added before tempo-synced effects; match EQ promoted from stretch because mastering needs it; design system, accessibility and i18n moved from Phase 8 to the start.
- **Technical corrections:** working project directory plus journal replaces rewriting a ZIP on autosave; `std::simd` (nightly-only) replaced; Opus decoding gap closed; web AAC path via WebCodecs; deprecated NNAPI dropped; React Native removed; WASM denormal, AudioWorklet, storage eviction, cross-origin isolation and memory-limit risks documented.
- **Testability:** gate types separate what an agent can verify from what needs people, hardware or data (§5.4); threshold discipline (§5.5); placeholder thresholds replaced with initial numbers; reference devices and browser tiers defined (§1.1).
- **Legal and policy:** dataset restrictions, LGPL in WASM, labelling of all synthetic speech, name clearance; third-party names removed from presets (D8).
- **Delivery:** decisions D9–D11, Spikes F–G, release milestones.

**Revision 3 (2026-11-03).** Scope narrowed to the web app and a later Windows portable app. Native macOS, Linux and mobile apps removed. Windows moved to Phase 10, which starts only when the user asks, and defined as one zip with all DLLs and tools such as FFmpeg bundled.

---

*End of PLAN.md. Treat this as a living document: update the quality-targets table and ADRs at the end of each phase using measured numbers rather than estimates.*
