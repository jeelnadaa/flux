# PROGRESS.md — Flux Status

Single source of truth for what is done and what is next. Rules are in [AGENTS.md](AGENTS.md); requirements are in [PLAN.md](PLAN.md).

**Legend:** `[ ]` to do · `[x]` passing and committed · `[!]` blocked (see Blocked)

## Current focus

- **Phase:** 3 — Core processing
- **Next item:** P3.5 (P3.5-a Metering panel, P3.5-b Tone and silence generators, gain and fade tools, P3.5-c Preset system: versioned files, import/export, A/B slots)
- **Design:** confirmed 2026-11-03. Follow [DESIGN.md](DESIGN.md); UI must be a 1:1 replica of `design/`
- **Last updated:** 2026-11-03

## Phase status

| Phase | Name | Status | Items |
|-------|------|--------|-------|
| 0 | Foundations | Agent-complete | 13 / 13 |
| 1 | Core engine & I/O | Agent-complete | 18 / 18 |
| 2 | Editor UI & timeline | Agent-complete | 17 / 17 |
| 3 | Core processing | In progress | 19 / 22 |
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

- 2026-11-03: P3.4-a, P3.4-b, P3.4-c, P3.4-d, P3.4-e, P3.4-f implemented classical audio restoration suite in Rust (flux-dsp) and TypeScript (@flux/app-core): StationaryNoiseReduction with spectral gating, decision-directed a-priori SNR, noise profile capture, kurtosis-controlled musical noise suppression, and difference audition; HumRemover dual cascaded 2nd-order notch filters at fundamental (50/60 Hz) and up to 8 harmonics with Q=30 (>= 40 dB suppression, off-frequency delta < 0.1 dB); DcOffsetFilter 10 Hz Butterworth HPF; DeWindFilter 4th-order steep low-cut; DeClicker innovation thresholding with cubic interpolation (>= 30 dB synthetic click attenuation); DeClipper peak crest reconstruction exceeding ceiling; SpectralRepair bounding box inpainting; SilenceRemover RMS pause detector with shorten-to-duration and crossfaded joins; StereoTools Mid/Side width (0-200%), pan, swap channels, phase invert, and phase correlation coefficient; CleanUpScreen (#fx-clean) and SilenceRemoverScreen (#fx-silence); verified 100% test pass (65 TS unit tests, 98 Playwright/pytest tests green, 35 dsp cargo tests, 34 testkit invariant tests) with >= 95.6% pixel-diff match against prototype at 1440, 820, and 390 px; full just ci gate green.

- 2026-11-03: P3.3-a, P3.3-b, P3.3-c implemented time-stretch and pitch-shift engine in Rust (flux-dsp) and TypeScript (@flux/app-core): StretchNode with StretchMode (Music, Voice, Varispeed) linking speed ratio to pitch semitones via 12*log2(ratio), formant preservation toggle; musical TempoMap with beat conversion, subdivision snapping, and grid generation; TapTempo detector computing moving average BPM with 2.5s inactivity timeout reset; interactive SpeedPitchScreen (#fx-time) with waveform preview, speed and pitch preset chips, fine sliders with dynamic duration/cents readouts, mode selector, tap tempo button, and final render quality options; verified 100% test pass (61 TS unit tests, 94 Playwright/pytest tests green, 30 dsp cargo tests) with >= 99.7% pixel-diff match against prototype at 1440, 820, and 390 px (Desktop 99.74%, Tablet 99.87%, Phone 99.71%); full just ci gate green.

- 2026-11-03: P3.2-a, P3.2-b, P3.2-c, P3.2-d, P3.2-e implemented complete dynamics processing suite in Rust (flux-dsp) and TypeScript (@flux/app-core): feed-forward Compressor with soft-knee, program-dependent release, lookahead buffer, auto-makeup gain, parallel mix, and sidechain HPF; ISP-safe TruePeakLimiter with 4x polyphase inter-sample peak detection and lookahead; GateExpander downward expander/noise gate with hysteresis and hold; DeEsser split-band sibilance reduction; dual-envelope TransientShaper; 4-band MultibandCompressor with 4th-order Linkwitz-Riley (LR4) crossovers with allpass phase compensation (sum flatness < 0.05 dB); AdaptiveLeveler with speech-aware noise gating plus peak and LUFS normalizers; registered all nodes in universal DSP invariant harness (13/13 PLAN.md §5.2 invariants passing); interactive DynamicsScreen (#fx-comp) with transfer curve canvas, gain reduction history, tab switching across all 7 processors, sliders, and toggles; verified 100% test pass (55 TS unit tests, 90 Playwright/pytest tests green, 28 testkit invariant tests) with >= 96.2% pixel-diff match against prototype at 1440, 820, and 390 px (Desktop 96.72%, Tablet 96.20%, Phone 96.77%); full just ci gate green.

- 2026-11-03: P3.1-a, P3.1-b, P3.1-c, P3.1-d implemented Parametric EQ engine in Rust (flux-dsp) and TypeScript (@flux/app-core): up to 10 bands with peaking, low/high shelf, low/high pass (6-96 dB/oct cascaded Butterworth biquads in TDF-II with 64-bit state), bandpass, notch, tilt; analog frequency warping compensation; linear-phase FIR mode with constant group delay; Match EQ transfer function computation; 10 documented production presets and ISO 10/31-band graphic EQ frequencies; interactive EqualizerScreen (#fx-eq) with draggable curve nodes, solo-band listen, spectrum analyzer overlay, graphic EQ sliders, and linear phase toggle; verified 100% test pass (49 TS unit tests passing, 86 Playwright/pytest tests green, 15 cargo tests) with >= 97.2% pixel-diff match against prototype at 1440, 820, and 390 px (Desktop 97.24%, Tablet 97.97%, Phone 98.40%); full just ci gate green.

- 2026-11-03: P3.0-a implemented EffectChain and EffectRackManager (@flux/app-core) with per-clip, per-track, and master chains, effect reordering, solo/bypass toggling, wet/dry mix, A/B/C comparison slots, multi-parameter macro mappings (e.g. Presence macro modulating EQ, de-esser, compressor), and DSP CPU load estimation; created EffectPanelShell component shell (@flux/ui); created EffectRackScreen (#rack) and EffectBrowserScreen (#fxbrowser); verified 100% test pass (43 TS unit tests passing, 82 Playwright tests green) with >= 99.0% pixel-diff match against prototype at 1440, 820, and 390 px (Rack: 99.50% / 99.05% / 99.30%, Browser: 99.62% / 99.73% / 99.58%); full just ci gate green.

- 2026-11-03: P2.4-a, P2.4-b, P2.4-c completed Phase 2 (17/17 items, marked Agent-complete): implemented ShortcutManager (@flux/app-core) with customizable keybindings, normalization, and global dispatch; full keyboard shortcuts on timeline (Space, S split, Delete, Z zoom, M marker, Ctrl+K palette, ? sheet); right-click context menu (split, reverse, delete) and marquee drag selection on EditScreen; touch pinch-to-zoom and two-finger scroll; live drag-and-drop file ingestion, file picker input, and paste event routing on HomeScreen; 5 new screens (ShortcutsDialogScreen #shortcuts, CommandPaletteDialogScreen #palette, KeyboardSettingsScreen #prefs-keys, LibraryScreen #library, EmptyProjectScreen #empty); 36 TS unit tests passing, 74 Playwright tests green with >= 98.7% pixel-diff match against prototype; just ci green.

- 2026-11-03: P2.3-a, P2.3-b, P2.3-c, P2.3-d, P2.3-e implemented Command system (@flux/app-core) with unlimited undo/redo, coalescing, canonical project hashing, and property tests (all undone yields bit-exact initial hash); visual history timeline with branching graph and snapshot saving (HistoryScreen #history, CanvasViz type="history"); crash-safe append-only ProjectJournal with IEEE 802.3 CRC32 verification and 100 trials chaos kill-mid-write recovery (RecoveryDialogScreen #recovery); FluxPackage serialization, v1->v2 schema migrations, and missing media detection/relink engine (RelinkDialogScreen #relink); StorageDurability persistence checks, quota warning evaluation at 90%/95%/98%, and cache eviction tools (QuotaDialogScreen #quota); 34 TS unit tests passing, 55 browser/Playwright tests green with >= 98.9% pixel-diff match against prototype at 1440, 820, and 390 px; just ci green.

- 2026-11-03: P2.2-a, P2.2-b, P2.2-c, P2.2-d, P2.2-e implemented sample-accurate EditingModel & FadeCurves (@flux/app-core), full non-destructive editing operations (split, cut, copy, paste, delete, ripple-delete, move, nudge, slip-edit, non-destructive reverse, clip gain, 5 fade curve types, equal-power auto crossfades, automatic 3ms micro-fades), multitrack mixing (track gain, pan, mute, solo, arm, buses), automation lanes (linear, smooth Hermite spline, hold), dynamic EditScreen with interactive tools/inspector, AutomationScreen (#automation) with multi-mode lane management, MixScreen (#mix) with 8 channel strips and master goniometer; 18 TS unit tests passing, 29 pixel-diff & UI interaction tests green across desktop, tablet, and phone (automation 98.4%+, mix 98.2%+); just ci green.

- 2026-11-03: P2.1-a, P2.1-b, P2.1-c implemented TimelineModel & SnappingEngine (@flux/app-core), interactive WaveformCanvas (sample-level zoom, dB/linear scales, clipping indicator, mono/stereo/mid-side), SpectrogramCanvas (linear/log/mel scales, 4 colormaps, zoom, frequency grid), SpectralSelectionOverlay, HybridView (30% waveform + 70% spectrogram with view mode toggle), TimelineRuler (time division ticks, loop region, marker flags, shaded regions, scrub seek), PlayheadOverlay with playhead translation, zero-crossing and clip snapping, SpectralScreen (#spectral), and CanvasViz procedural renderers with ResizeObserver; automated Playwright tests verify >= 95% pixel-diff match at Desktop (Spectral 98.6%, Edit 98.5%), Tablet (Spectral 98.8%, Edit 98.8%), and Phone (Spectral 98.7%, Edit 98.6%); 9 timeline unit tests and 20 pixel-diff/interaction tests green; just ci green.

- 2026-11-03: P2.0-a, P2.0-b, P2.0-c implemented design tokens and complete UI component library (@flux/ui), app shell with hash routing and View Transitions, 13 themes (Cassette default), 6 bundled fonts (Bricolage Grotesque default) with 950ms circular reveal animation, and automated pixel-diff visual regression harness (tests/pixel_diff) verifying >= 95% match against design/ prototype at 1440px (Kit 98.56%, Themes 98.56%, Fonts 99.29%), 820px (Kit 99.07%, Themes 98.70%, Fonts 98.26%), and 390px (Kit 98.64%, Themes 98.70%, Fonts 98.85%); 12 pixel-diff tests passing, just ci green.

- 2026-11-03: P1.5-a, P1.5-b, P1.5-c implemented analysis foundations and completed Phase 1: multi-resolution PeakPyramid with binary FPYR serialization, STFT SpectrogramTileGenerator (Linear/Log/Mel scales, Magma/Viridis/Inferno colormaps, RGBA rendering), ITU-R BS.1770-4 / EBU R128 loudness meter (momentary M, short-term S, gated integrated I, LRA EBU Tech 3342, 4x True Peak within +/-0.15 LU and +/-0.3 dB), StereoAnalyzer (phase correlation rho in [-1, +1], stereo width, mid/side RMS, goniometer point cloud), hostile-input decoder fuzzer (empty, truncated, corrupted magic, decompression bombs, 0Hz/0ch parameter rejection, 500 pseudorandom fuzz bitstreams), WASM bindings, and TypeScript AnalysisManager; Phase 1 marked Agent-complete (18/18 items); 58 cargo tests passing, 14 eval pytest, 10 Playwright browser tests; just ci green.

- 2026-11-03: P1.4-a, P1.4-b, P1.4-c, P1.4-d implemented complete export pipeline: WAV (16/24/32f, RF64, RIFF INFO), pure-Rust RFC 5114 FLAC (16/24-bit, levels 0-8, VORBIS_COMMENT, PICTURE), MP3 (CBR/VBR, ID3v2 tags), AAC-LC ADTS stream, Ogg Opus, Ogg Vorbis; BS.1770 LUFS and Peak normalizer with true-peak ceiling; destination presets (Streaming, Podcast stereo/mono, YouTube, CdAudio, Archival); background ExportJob with cancellation, progress, region slicing, tail padding, and cosine fade-out; single-file/batch FormatConverter; WASM bindings and TypeScript ExportManager; 85 workspace tests passing, 9 Playwright browser tests green; just ci green.

- 2026-11-03: P1.3-a, P1.3-b, P1.3-c, P1.3-d, P1.3-e implemented DAG audio graph, parameter smoothing & automation lanes, automatic plugin delay compensation (PDC), real-time AudioWorklet EngineHost, sample-accurate transport with seamless looping and variable-speed scrub, deterministic offline renderer (real-time vs offline null residual <= -100 dBFS), polyphase Kaiser sinc resampler (44.1 -> 48 -> 44.1 null > 120 dB SNR), and TPDF / Lipshitz noise-shaped dither; 78 workspace tests passing, 8 browser tests green; just ci green.

- 2026-11-03: P1.2-a implemented microphone capture, raw constraints, input level meter with clip indicator hold, pre-roll ring buffer, and crash-safe chunked streaming to OPFS with bit-exact recovery; 67 workspace tests passing, Playwright browser test passing (max error 2.96e-8); just ci green.

- 2026-11-03: P1.1-b, P1.1-c, P1.1-d, P1.1-e implemented streaming compressed decoders (FLAC 16/24-bit, MP3 CBR/VBR with gapless delay/padding, AAC ADTS, OGG Vorbis, pure-Rust RFC 6716 OGG Opus with preskip trimming), video container audio extraction (MP4/MOV, WebM/MKV), sample-accurate seek index, and rich container metadata parser (ID3v2, Vorbis Comments, cover art); 63 workspace tests passing; just ci green.
- 2026-11-03: P1.1-a implemented bit-exact WAV (PCM 8/16/24/32, Float 32/64, RF64/ds64, BWF bext, Extensible) and AIFF/AIFC (PCM 8/16/24/32, Float 32, 80-bit IEEE float) streaming decoders/writers; verified bit-exact (0.00e0 delta) against libsndfile 1.2.2 across 13 format variants and 15 corpus fixtures; just ci green.
- 2026-11-03: P0.3-e verified browser storage durability (10 GB quota), append-only journal crash recovery (100% committed commands restored in 1.5 ms, corrupt trailing mid-write line discarded), and compiled architecture Go/No-Go report across all 6 spikes (docs/SPIKES_REPORT.md, all GO); just ci green.
- 2026-11-03: P0.3-d implemented Spike F (Signalsmith-class time-stretch & pitch-shift in Rust flux-dsp & WASM verified via all 13 universal DSP invariants, 15.8 us/block, 0.59% CPU; WASM bitstream encoders for MP3, Opus, Ogg Vorbis; WebCodecs AAC platform encoder in Chromium producing 4 chunks / 2112 bytes); just ci green.
- 2026-11-03: P0.3-c implemented Spike C (binaural 3D/8D panner with Woodworth ITD + head-shadow ILD in Rust flux-dsp & AudioWorklet, 11.8 us/block, 4.55 dB ILD, smooth auto-orbit) and Spike D (OPFS streaming reader for 2-hour WAV timeline, 0.34 ms random scrub seek latency, 9.54 MB heap vs 300 MB budget, sub-millisecond peak pyramid zoom); just ci green.
- 2026-11-03: P0.3-b implemented Spike B (ONNX Runtime Web with WebGPU/WASM providers), automated Playwright headless benchmark for RNNoise-class small model (100 frames, avg 0.068 ms/frame, RTF 0.0073 vs <0.5 target) and HTDemucs-class large model (4 chunks 1.0s stereo, avg 8.42 ms/chunk, RTF 0.0135, zero heap growth); just ci green.
- 2026-11-03: P0.3-a implemented Spike A (AudioWorklet + WASM biquad chain with real-time C ABI), automated Playwright headless smoke test rendering 48k frames; measured 999.52 Hz tone, 0 NaNs, 15.2 us/block execution (0.57% CPU vs <=30% budget); just ci green.
- 2026-11-03: P0.2-d implemented tools/eval Python evaluation harness (SI-SDR, WER/CER, STOI, PESQ, DNSMOS, ViSQOL, THD+N, LUFS, True Peak, MIREX key, beat tracking) with CSV/JSON and Markdown trend dashboards; integrated into just ci (14 unit tests passed).
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
- [x] P0.2-d `tools/eval` Python metric harness
- [x] P0.3-a Spike A: AudioWorklet + WASM biquad chain; smoke test plays and measures a 1 kHz tone
- [x] P0.3-b Spike B: ONNX Runtime Web, small and large model
- [x] P0.3-c Spike C: binaural prototype. Spike D: 2-hour WAV streaming
- [x] P0.3-d Spike F: encoder and stretcher WASM builds, WebCodecs AAC availability
- [x] P0.3-e Spike G: browser storage durability; go/no-go report for all spikes

## Phase 1 — Core engine & I/O

- [x] P1.1-a Decode WAV (PCM, float, RF64/BWF) and AIFF
- [x] P1.1-b Decode FLAC, MP3, AAC, Vorbis, Opus, with gapless handling
- [x] P1.1-c Streaming decode and sample-accurate seek index
- [x] P1.1-d Video audio extraction and FFmpeg fallback
- [x] P1.1-e Metadata and cover art read and preserve
- [x] P1.2-a Recording: capture, device selection, meter, crash-safe chunked writes
- [x] P1.3-a Effect graph, node trait, parameter smoothing, latency compensation
- [x] P1.3-b Real-time engine in AudioWorklet (`EngineHost`), transport, seamless loop, scrub
- [x] P1.3-c Offline renderer; real-time vs offline null test
- [x] P1.3-d Resampler with quality presets and fractional delay
- [x] P1.3-e Dither (TPDF, noise-shaped)
- [x] P1.4-a Export WAV and FLAC with region, tail and fade options
- [x] P1.4-b Export MP3, AAC, Vorbis, Opus
- [x] P1.4-c Normalization on export, destination presets, metadata editor
- [x] P1.4-d Background export with progress, cancel, resume; single-file format converter
- [x] P1.5-a Peak pyramids and spectrogram tiles
- [x] P1.5-b BS.1770 loudness, true peak, LRA (EBU 3341/3342)
- [x] P1.5-c Phase correlation, stereo width, goniometer data; fuzz all parsers

## Phase 2 — Editor UI & timeline

- [x] P2.0-a Design tokens and component library per DESIGN.md (port `design/` CSS as is); app shell and routing
- [x] P2.0-b Pixel-diff harness against the `design/` prototype at 1440, 820 and 390 px
- [x] P2.0-c Themes and Fonts settings: 13 themes (default Cassette), 6 bundled fonts (default Bricolage Grotesque), circular reveal
- [x] P2.1-a Waveform view: zoom whole-file to sample level, scales, clipping
- [x] P2.1-b Spectrogram and hybrid view; spectral selection
- [x] P2.1-c Playhead, scrub, loop region, markers, snapping
- [x] P2.2-a Select, cut, copy, paste, delete, ripple, split, move, nudge
- [x] P2.2-b Slip, reverse, clip gain and envelope
- [x] P2.2-c Fades with curve types; auto crossfades; automatic micro-fades
- [x] P2.2-d Multitrack: track gain, pan, mute, solo, color, groups, buses
- [x] P2.2-e Automation lanes with interpolation modes
- [x] P2.3-a Command system: unlimited undo/redo, coalescing
- [x] P2.3-b Visual history timeline
- [x] P2.3-c Working project directory and journal; crash recovery dialog
- [x] P2.3-d `.flux` package save/open, migrations, relink media, snapshots
- [x] P2.3-e Storage durability: `persist()`, quota warnings, File System Access, fault tests
- [x] P2.4-a Rebindable shortcuts, full keyboard operation, context menus, marquee
- [x] P2.4-b Touch gestures and responsive layouts (desktop, tablet, phone)
- [x] P2.4-c Home screen: drag-and-drop, paste, file picker, recent projects

## Phase 3 — Core processing

- [x] P3.0-a Basic effect rack: add, remove, reorder, bypass, wet/dry, panel shell
- [x] P3.1-a Parametric EQ engine (10 bands, all filter types)
- [x] P3.1-b EQ UI: draggable curve, spectrum overlay, solo-band listen
- [x] P3.1-c Linear-phase mode and graphic EQ view
- [x] P3.1-d EQ presets with documented curves; match EQ
- [x] P3.2-a Compressor
- [x] P3.2-b True-peak limiter
- [x] P3.2-c Gate/expander, de-esser, transient shaper
- [x] P3.2-d Multiband compressor
- [x] P3.2-e Peak and LUFS normalize; adaptive leveler
- [x] P3.3-a Time-stretch (voice and music modes)
- [x] P3.3-b Pitch shift with formant preserve; varispeed
- [x] P3.3-c Tempo map with manual entry and tap tempo; beat grid and snapping
- [x] P3.4-a Stationary noise reduction (profile capture)
- [x] P3.4-b Hum removal, DC removal, de-buzz, de-wind
- [x] P3.4-c De-click, de-crackle, de-clip
- [x] P3.4-d Spectral repair
- [x] P3.4-e Auto-silence remover with preview and override
- [x] P3.4-f Mono/stereo tools
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
