# PROGRESS.md — Flux Status

Single source of truth for what is done and what is next. Rules are in [AGENTS.md](AGENTS.md); requirements are in [PLAN.md](PLAN.md).

**Legend:** `[ ]` to do · `[x]` passing and committed · `[!]` blocked (see Blocked)

## Current focus

- **Phase:** 0 — Foundations
- **Next item:** P0.2-d
- **Design:** confirmed 2026-11-03. Follow [DESIGN.md](DESIGN.md); UI must be a 1:1 replica of `design/`
- **Last updated:** 2026-11-03

## Phase status

| Phase | Name | Status | Items |
|-------|------|--------|-------|
| 0 | Foundations | In progress | 7 / 13 |
| 1 | Core engine & I/O | Not started | 0 / 18 |
| 2 | Editor UI & timeline | Not started | 0 / 17 |
| 3 | Core processing | Not started | 0 / 22 |
| 4 | Spatial audio | Not started | 0 / 18 |
| 5 | Creative FX & voice | Not started | 0 / 17 |
| 6 | AI restoration & transcript | Not started | 0 / 14 |
| 7 | Advanced AI | Not started | 0 / 11 |
| 8 | Pro workflow & polish | Not started | 0 / 15 |
| 9 | Hardening & launch (web) | Not started | 0 / 10 |
| 10 | Windows portable app | **On request only.** Do not start until the user asks | 0 / 9 |

Statuses: Not started → In progress → Agent-complete (only human, data or device gates remain) → Released.

## Human & hardware queue

Gates an agent cannot clear (PLAN.md §5.4). Add a line whenever one is deferred.

| Item | Gate | What is needed | Added | Result |
|------|------|----------------|-------|--------|
| — | [H] | Trademark search for the name "Flux" before public launch (PLAN.md §4) | 2026-11-03 | Pending |
| — | [H] | Name conflict check: "FLUX::" is an existing audio software brand | 2026-11-03 | Pending |
| — | [H] | Read the Satoshi ITF Free Font License and confirm it allows bundling, or remove Satoshi | 2026-11-03 | Pending |
| — | [H] | Confirm GSAP licence terms for the landing page | 2026-11-03 | Pending |

## Blocked

| Item | What was tried | Exact error or missing thing | What would unblock it | Stash |
|------|----------------|------------------------------|-----------------------|-------|
| — | — | — | — | — |

## Decision log

Short pointers only; full reasoning goes in `docs/adr/`.

- 2026-11-03: Product named Flux (PLAN.md D11). Scope is the web app, then a portable Windows zip only when the user asks. No native macOS, Linux or mobile apps (PLAN.md D1).
- 2026-11-03: Design confirmed. Default theme Cassette, default font Bricolage Grotesque. All 13 themes and 6 fonts ship, and the fonts are bundled. UI is a 1:1 port of `design/` (DESIGN.md).

## Session log

Newest first. One line per commit: item, what changed, measured result.

- 2026-11-03: P0.2-c created corpus manifest (corpus/corpus.yaml), SHA-256 and WAV encoders/decoders, generate-corpus binary generating 15 synthetic fixtures; automated integrity verification test passed; just ci green (23 unit tests passed).
- 2026-11-03: P0.2-b implemented universal DSP invariant harness in flux-testkit verifying all 13 PLAN.md §5.2 invariants; defined DspNode trait in flux-dsp with PassthroughNode and GainNode; just ci green (16 unit tests passed).
- 2026-11-03: P0.2-a completed flux-testkit signal generators, analyzers (RMS, peak, 4x True Peak, FFT, IEEE 1057 THD+N, group delay, BS.1770 LUFS), and comparison helpers (SNR, spectral distance, null residual); just ci green (13 unit tests passed).
- 2026-11-03: P0.1-d written ADRs 0001-0011 (D1-D11), docs/QUALITY_TARGETS.md, and docs/PRIVACY_DATA_FLOW.md; just ci green.
- 2026-11-03: P0.1-c configured license scanners (cargo-deny, license-checker-rseidelsohn) in deny.toml and just ci; completed LICENSES.md audit; just ci green (licenses ok, 0 violations).
- 2026-11-03: P0.1-b configured lint, format, and typo tooling (rustfmt, clippy, eslint, prettier, typos); just ci green (fmt, clippy -D warnings, eslint, typos, 5 unit tests, WASM build, TS typecheck).
- 2026-11-03: P0.1-a initialized git repository, .gitignore, .gitattributes, monorepo workspace (5 Rust crates, 3 TS packages/apps, skeleton dirs), and justfile; just ci green (5 unit tests, WASM target build, TS typecheck).
- 2026-11-03: Initial repository setup with specs and design prototype.
- 2026-11-03: DESIGN.md and LICENSES.md written; prototype finished at 87 screens plus landing. Sweep: 0 overflows across 6 fonts x 3 widths x 87 screens (1,566 renders), Cassette theme only. Not committed: no git repo yet (P0 creates it).
- 2026-11-03: Plan revised to revision 2; AGENTS.md, PROGRESS.md and README.md created. No code yet.

---

# Feature checklist

## Phase 0 — Foundations (PLAN.md Phase 0)

- [x] P0.1-a Initialize git, `.gitignore`, `.gitattributes`, monorepo skeleton, `just ci`
- [x] P0.1-b Lint, format and typo tooling for Rust and TypeScript
- [x] P0.1-c License scan (`cargo-deny`, JS checker), `LICENSES.md`, licence audit
- [x] P0.1-d ADRs for D1–D11, `docs/QUALITY_TARGETS.md`, privacy data-flow diagram
- [x] P0.2-a `flux-testkit`: signal generators and analyzers, with self-tests
- [x] P0.2-b Universal DSP invariant harness (§5.2)
- [x] P0.2-c Corpus manifest and integrity test; synthetic corpus generated
- [ ] P0.2-d `tools/eval` Python metric harness
- [ ] P0.3-a Spike A: AudioWorklet + WASM biquad chain; smoke test plays and measures a 1 kHz tone
- [ ] P0.3-b Spike B: ONNX Runtime Web, small and large model
- [ ] P0.3-c Spike C: binaural prototype. Spike D: 2-hour WAV streaming
- [ ] P0.3-d Spike F: encoder and stretcher WASM builds, WebCodecs AAC availability
- [ ] P0.3-e Spike G: browser storage durability; go/no-go report for all spikes

## Phase 1 — Core engine & I/O

- [ ] P1.1-a Decode WAV (PCM, float, RF64/BWF) and AIFF
- [ ] P1.1-b Decode FLAC, MP3, AAC, Vorbis, Opus, with gapless handling
- [ ] P1.1-c Streaming decode and sample-accurate seek index
- [ ] P1.1-d Video audio extraction and FFmpeg fallback
- [ ] P1.1-e Metadata and cover art read and preserve
- [ ] P1.2-a Recording: capture, device selection, meter, crash-safe chunked writes
- [ ] P1.3-a Effect graph, node trait, parameter smoothing, latency compensation
- [ ] P1.3-b Real-time engine in AudioWorklet (`EngineHost`), transport, seamless loop, scrub
- [ ] P1.3-c Offline renderer; real-time vs offline null test
- [ ] P1.3-d Resampler with quality presets and fractional delay
- [ ] P1.3-e Dither (TPDF, noise-shaped)
- [ ] P1.4-a Export WAV and FLAC with region, tail and fade options
- [ ] P1.4-b Export MP3, AAC, Vorbis, Opus
- [ ] P1.4-c Normalization on export, destination presets, metadata editor
- [ ] P1.4-d Background export with progress, cancel, resume; single-file format converter
- [ ] P1.5-a Peak pyramids and spectrogram tiles
- [ ] P1.5-b BS.1770 loudness, true peak, LRA (EBU 3341/3342)
- [ ] P1.5-c Phase correlation, stereo width, goniometer data; fuzz all parsers

## Phase 2 — Editor UI & timeline

- [ ] P2.0-a Design tokens and component library per DESIGN.md (port `design/` CSS as is); app shell and routing
- [ ] P2.0-b Pixel-diff harness against the `design/` prototype at 1440, 820 and 390 px
- [ ] P2.0-c Themes and Fonts settings: 13 themes (default Cassette), 6 bundled fonts (default Bricolage Grotesque), circular reveal
- [ ] P2.1-a Waveform view: zoom whole-file to sample level, scales, clipping
- [ ] P2.1-b Spectrogram and hybrid view; spectral selection
- [ ] P2.1-c Playhead, scrub, loop region, markers, snapping
- [ ] P2.2-a Select, cut, copy, paste, delete, ripple, split, move, nudge
- [ ] P2.2-b Slip, reverse, clip gain and envelope
- [ ] P2.2-c Fades with curve types; auto crossfades; automatic micro-fades
- [ ] P2.2-d Multitrack: track gain, pan, mute, solo, color, groups, buses
- [ ] P2.2-e Automation lanes with interpolation modes
- [ ] P2.3-a Command system: unlimited undo/redo, coalescing
- [ ] P2.3-b Visual history timeline
- [ ] P2.3-c Working project directory and journal; crash recovery dialog
- [ ] P2.3-d `.flux` package save/open, migrations, relink media, snapshots
- [ ] P2.3-e Storage durability: `persist()`, quota warnings, File System Access, fault tests
- [ ] P2.4-a Rebindable shortcuts, full keyboard operation, context menus, marquee
- [ ] P2.4-b Touch gestures and responsive layouts (desktop, tablet, phone)
- [ ] P2.4-c Home screen: drag-and-drop, paste, file picker, recent projects

## Phase 3 — Core processing

- [ ] P3.0-a Basic effect rack: add, remove, reorder, bypass, wet/dry, panel shell
- [ ] P3.1-a Parametric EQ engine (10 bands, all filter types)
- [ ] P3.1-b EQ UI: draggable curve, spectrum overlay, solo-band listen
- [ ] P3.1-c Linear-phase mode and graphic EQ view
- [ ] P3.1-d EQ presets with documented curves; match EQ
- [ ] P3.2-a Compressor
- [ ] P3.2-b True-peak limiter
- [ ] P3.2-c Gate/expander, de-esser, transient shaper
- [ ] P3.2-d Multiband compressor
- [ ] P3.2-e Peak and LUFS normalize; adaptive leveler
- [ ] P3.3-a Time-stretch (voice and music modes)
- [ ] P3.3-b Pitch shift with formant preserve; varispeed
- [ ] P3.3-c Tempo map with manual entry and tap tempo; beat grid and snapping
- [ ] P3.4-a Stationary noise reduction (profile capture)
- [ ] P3.4-b Hum removal, DC removal, de-buzz, de-wind
- [ ] P3.4-c De-click, de-crackle, de-clip
- [ ] P3.4-d Spectral repair
- [ ] P3.4-e Auto-silence remover with preview and override
- [ ] P3.4-f Mono/stereo tools
- [ ] P3.5-a Metering panel (LUFS, LRA, true peak, spectrum, goniometer)
- [ ] P3.5-b Tone and silence generators, gain and fade tools
- [ ] P3.5-c Preset system: versioned files, import/export, A/B slots

## Phase 4 — Spatial audio

- [ ] P4.1-a HRTF pipeline: SOFA conversion, minimum-phase + ITD
- [ ] P4.1-b Renderer A: direct HRTF with interpolation, crossfaded convolvers
- [ ] P4.1-c Partitioned FFT convolution engine
- [ ] P4.1-d Renderer B: ambisonic encode, rotate, MagLS decode
- [ ] P4.1-e Distance cues, Doppler, level stability, headphone compensation
- [ ] P4.1-f Head-width control and "pick your HRTF" test
- [ ] P4.2-a Stereo inputs (virtual speakers, mid/side), mono bass
- [ ] P4.3-a 3D static placement and presets
- [ ] P4.3-b 8D auto-orbit with all controls, tempo sync
- [ ] P4.3-c 16D paths: spiral, figure-eight, vertical loop, swarm, Lissajous
- [ ] P4.3-d Sound stage UI: top and side views, drag to place
- [ ] P4.3-e Draw-a-path, record dragging to automation, multiple sources
- [ ] P4.3-f Headphone check wizard; speaker warning and mono-compatibility check
- [ ] P4.4-a Convolution reverb with IR tools
- [ ] P4.4-b Algorithmic reverb (FDN, plate, hall)
- [ ] P4.4-c Room presets; custom room designer; binaural early reflections
- [ ] P4.4-d Concert hall "row 10" mode
- [ ] P4.4-e "Next door" effect; renderer choice ADR (queue the perceptual tests)

## Phase 5 — Creative FX & voice

- [ ] P5.1-a Slowed + reverb, sped-up / nightcore, keep-pitch toggle
- [ ] P5.1-b Lo-fi one-slider macro
- [ ] P5.2-a Vinyl
- [ ] P5.2-b Cassette/tape
- [ ] P5.2-c Old radio, phone, megaphone, walkie-talkie, PA
- [ ] P5.2-d Underwater
- [ ] P5.2-e Bitcrusher, distortion, waveshaper (anti-aliased)
- [ ] P5.2-f Chorus, flanger, phaser, tremolo, vibrato, auto-pan, ring mod, rotary
- [ ] P5.2-g Delay and echo with tempo sync
- [ ] P5.3-a Reverse and reverse-reverb
- [ ] P5.3-b Stutter and beat repeat
- [ ] P5.3-c Tape stop and start
- [ ] P5.3-d Granular freeze, glitch, trance gate
- [ ] P5.4-a Voice engine: independent pitch and formant shift
- [ ] P5.4-b Voice presets (deep, chipmunk, child, elder, whisper, announcer, others)
- [ ] P5.4-c Robot, cyborg, alien, monster; channel vocoder; hard-tune
- [ ] P5.4-d Live mic preview, latency measured

## Phase 6 — AI restoration & transcript

- [ ] P6.0-a ONNX Runtime wrapper, backend selection, device probing, chunked overlap-add
- [ ] P6.0-b Model manager: verified download, versioning, eviction UI
- [ ] P6.0-c Cloud job client against a local mock; consent toggle; derived-asset cache
- [ ] P6.1-a Stem separation: 4-stem pipeline and quality modes
- [ ] P6.1-b Stem results UI: karaoke, acapella, per-stem mixer, stem export
- [ ] P6.2-a Neural denoise with room-tone preservation
- [ ] P6.2-b De-reverb and echo removal
- [ ] P6.2-c Studio Sound chain with hallucination guard; speech/music routing
- [ ] P6.3-a Transcription: word timestamps, diarization, language ID, glossary
- [ ] P6.3-b Transcript editor: click to seek, delete word removes audio, reorder
- [ ] P6.3-c Boundary refinement, room-tone fill; SRT/VTT/TXT/JSON export
- [ ] P6.4-a Filler detection and stutter detection
- [ ] P6.4-b Filler review UI
- [ ] P6.5-a Neural VAD upgrade, segmentation strip, breath and mouth-click tools, voice isolation, auto-ducking

## Phase 7 — Advanced AI

- [ ] P7.1-a BPM, beat and downbeat tracking; fills tempo map; warp to grid
- [ ] P7.1-b Key detection, Camelot notation, tuning; tag writing
- [ ] P7.2-a Auto-master: analysis and adaptive chain with safety rails
- [ ] P7.2-b Reference matching, speech mode
- [ ] P7.2-c Master UI: loudness-matched A/B, explanation panel
- [ ] P7.3-a TTS: engine, voices, controls, pronunciation dictionary, timeline insert
- [ ] P7.3-b Voice cloning safeguards (ask the user first; off by default)
- [ ] P7.4-a Translation with glossary; editable translated script
- [ ] P7.4-b Dub render: per-speaker TTS, duration fit, background mix
- [ ] P7.4-c Dub review UI, per-segment regenerate, failure paths
- [ ] P7.5-a Magic: analyzer, issue detectors, editable proposed chain, goal presets

## Phase 8 — Pro workflow & polish

- [ ] P8.1-a Full rack: chain presets, A/B/C slots, hold-to-compare with loudness match
- [ ] P8.1-b Effect browser: search, favorites, animated previews, preview without commit
- [ ] P8.1-c Macro controls
- [ ] P8.2-a Simple/Pro switch and task-first Simple mode
- [ ] P8.3-a Batch processing queue
- [ ] P8.3-b Ringtone maker
- [ ] P8.3-c Recorded macros/actions
- [ ] P8.3-d Templates; project library with folders and search; storage manager
- [ ] P8.4-a Onboarding tour, contextual help, design system audit
- [ ] P8.5-a Accessibility audit: NVDA, VoiceOver, TalkBack, high contrast, reduced motion
- [ ] P8.5-b Translations, pseudo-localization, RTL
- [ ] P8.6-a Settings: audio, privacy, hearing safety, data deletion
- [ ] P8.6-b Help center, troubleshooting, diagnostic bundle
- [ ] P8.6-c Landing page (port `design/landing.html` and `design/landing.js` 1:1) and legal pages (privacy, terms, attributions)
- [ ] P8.7-a Performance budgets: INP, cold start, memory

## Phase 9 — Hardening & launch

- [ ] P9.1-a PWA: offline, install, COOP/COEP, service worker, graceful fallbacks
- [ ] P9.2-a Adaptive quality governor and profile-driven optimization
- [ ] P9.2-b Chaos suite: 1,000 fault injections with no data loss
- [ ] P9.2-c 4-hour soak; 72-hour cumulative fuzz
- [ ] P9.3-a Threat model, CSP, signed model manifests, dependency audit
- [ ] P9.3-b Real cloud backend, deletion and load tests (ask the user first: costs money)
- [ ] P9.4-a Attributions screen, privacy policy, terms, store disclosures
- [ ] P9.4-b User guide, tutorials, developer preset docs
- [ ] P9.4-c Release tooling: staged rollout, kill switches, rollback rehearsal
- [ ] P9.4-d Launch readiness review against PLAN.md Appendix C; list everything still queued

## Phase 10 — Windows portable app

**On request only. Do not start until the user explicitly asks, after Phase 9.** PLAN.md Phase 10 has the full requirements.

- [ ] P10.1-a Tauri shell, same UI, static C runtime, WebView2 fixed-version runtime in the package
- [ ] P10.1-b Native `EngineHost` with `cpal`: WASAPI, optional ASIO, device selection; native vs WASM null test
- [ ] P10.1-c Native file access, real project folders, portable settings location
- [ ] P10.1-d Native ONNX Runtime with DirectML, CUDA and CPU fallback
- [ ] P10.1-e Bundled FFmpeg and encoders, located relative to the exe
- [ ] P10.2-a `just package-windows` builds `Flux-<version>-win-x64.zip`
- [ ] P10.2-b Dependency audit fails the build on any missing DLL; launch test with empty `PATH`
- [ ] P10.2-c Licences folder and LGPL offers for every bundled component
- [ ] P10.2-d Clean-machine test on Windows 10 and 11 (queue as [R] if no VM is available)
