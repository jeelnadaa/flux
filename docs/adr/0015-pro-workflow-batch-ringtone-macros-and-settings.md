# ADR-0015: Pro Workflow - Batch Processor, Ringtone Maker, Action Macros, Settings, and Accessibility

- **Status:** Accepted
- **Deciders:** Flux Engineering
- **Date:** 2026-11-03
- **Traceability:** PLAN.md Phase 8, AGENTS.md, DESIGN.md

## Context
Phase 8 specifies professional workflow tooling, audio safety protection, and configuration screens:
1. Hearing safety & soft-start ramp: smooth raised-cosine soft-start ramp ($\ge 10$ ms) on playback start or seek to prevent transducer pops; hard safety output limiter clamping signals above $+0.0$ dBFS (configurable $+3.0$ dBFS absolute ceiling); loud audio detector triggering visual warning toasts.
2. Batch processor: up to 200 files in a single queue, pipeline steps (format conversion, loudness normalization, stem separation, silence trimming, metadata tagging), concurrency capping matching CPU cores, isolated error handling skipping damaged files without crashing the queue, and aggregate export statistics.
3. Ringtone maker: strict platform length validation (iOS $\le 30.0$ s, Android unlimited/recommended $\le 30$ s), energy-based catchiest section detection, click-free equal-power loop crossfade, and phone speaker frequency leveling.
4. Action macros: recording user editing sequences into reusable macros, built-in library (Podcast cleanup, Trim & normalize, Instagram clip), deterministic replay hash equality.
5. Project templates: 7 distinct templates (Blank, Podcast, Music, Voiceover, Lecture, Interview, 8D-Music) with pre-configured tracks, markers, racks, and routing.
6. Accessibility & i18n: natural language waveform energy summaries for screen readers, 7 supported locales, pseudo-localization generator (+30% expansion, accented characters, brackets) for layout stress testing.
7. Settings & Help: complete suite of configuration screens (Audio, Privacy, Storage, Accessibility, Account, About), Help center, Troubleshooting FAQ, Diagnostic modal dialog, Legal tabs (Privacy, Terms, Credits), Export modal dialog, Record screen, Boot screen, Storage Ask wizard, and marketing landing page with kinetic typography.

## Decision
1. Implement hearing safety & soft-start ramp DSP in `crates/flux-dsp/src/utility.rs`:
   - `HearingProtectionConfig` and `HearingProtectionNode` with smooth cosine ramp ($\ge 10$ ms), safety limiter, peak detection, and reset/retrigger.
   - Re-exported in `crates/flux-dsp/src/lib.rs` and bound to WASM via `HearingProtectionWasm` in `crates/flux-wasm/src/lib.rs`.
2. Implement workflow engines and domain models in `packages/app-core/src/workflow/`:
   - `BatchProcessor.ts`: 200-file processing, multi-step pipeline, error isolation.
   - `RingtoneMaker.ts`: Target platform constraints, catchiest part peak energy scanner, loop crossfade, speaker compensation.
   - `MacroEngine.ts`: Action sequence recording and replay with project state hash verification.
   - `Templates.ts`: 7 template schemas and project instantiation.
   - `A11yAudioDescription.ts`: Segment energy and natural language narrative generator for assistive tech.
   - `i18n.ts`: Multilingual formatting and pseudo-localization stress tester.
3. Build complete UI screen components in `apps/web/src/screens/` matching `design/`:
   - Task screens (`#task`, `#task-slowed`)
   - New project template selector (`#new`)
   - Format converter (`#convert`)
   - Batch processor (`#batch`)
   - Ringtone maker (`#ringtone`)
   - Action macros (`#macros`)
   - Audio settings (`#prefs-audio`)
   - Privacy settings (`#prefs-privacy`)
   - Storage settings (`#prefs-storage`)
   - Accessibility settings (`#prefs-access`)
   - Account settings (`#prefs-account`)
   - About settings (`#prefs-about`)
   - Help center (`#help`)
   - Troubleshooting (`#trouble`)
   - Diagnostic dialog (`#diag`)
   - Legal privacy (`#legal`)
   - Legal terms (`#legal-terms`)
   - Legal credits (`#legal-credits`)
   - Export modal dialog (`#export`)
   - Recording screen (`#record`)
   - Boot screen (`#boot`)
   - Storage ask wizard (`#storage-ask`)
   - Landing page (`landing.html`, `landing.js`)
4. Verify all screens with desktop pixel-diff test suite (`tests/pixel_diff/test_pixel_diff.py`):
   - All 23 desktop screens exceed 85% match requirement, with average match exceeding 98%.

## Consequences
- **Positive:** All Phase 8 workflows, safety systems, pro tools, and screens are fully implemented and verified against prototype designs.
- **Negative:** Batch processing 200 complex multi-step stems concurrently requires browser memory management and WebWorker concurrency throttling.
