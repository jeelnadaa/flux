# ADR-0014: Advanced AI - Tempo/Key Tracking, Auto-Mastering, TTS, Multilingual Dubbing, and Magic Assistant

- **Status:** Accepted
- **Deciders:** Flux Engineering
- **Date:** 2026-11-03
- **Traceability:** PLAN.md Phase 7, AGENTS.md, DESIGN.md

## Context
Phase 7 specifies high-level AI and music intelligence modules:
1. Tempo & key detection: BPM estimation, beat grid snapping, Camelot and Open Key notations, tuning Hz estimation ($A = 440$ Hz), and warp clip to grid.
2. Auto-mastering: adaptive corrective EQ, genre-specific target curves (Streaming -14.0 LUFS, Podcast -16.0 LUFS, Club -9.0 LUFS), multiband compression, mono-safe width enhancement, true-peak ceiling limiting, loudness-matched before/after comparison hold toggle, and explanation audit trail.
3. Neural text-to-speech: licensed narration voices (`Ash`, `Reed`, `Marin`, `Oren`), SSML control tags (pauses, emphasis, rate, pitch), word token alignment, and synthetic metadata labeling.
4. Voice cloning safeguards: mandatory user attestations, spoken consent phrase validation, inaudible spread-spectrum watermark embedding and detection, daily rate capping, disabled by default.
5. Multilingual dubbing: script translation with glossary constraints, duration fitting with shortening heuristics, per-speaker voice synthesis, and remixing with background instrumental stems.
6. One-tap Magic assistant: acoustic diagnosis of flaws (noise, echo, clipping, mains hum, loudness) and synthesis of an editable, reversible effect chain with goal presets.

## Decision
1. Implement the DSP and machine learning pipelines in `crates/flux-ml`:
   - `tempo_detect.rs`: Energy novelty onset curve and autocorrelation lag search over 40–240 BPM; clip warping to grid.
   - `key_detect.rs`: Goertzel 12-bin chromagram and Krumhansl-Schmuckler profile correlation; Camelot and Open Key notation lookup.
   - `mastering.rs`: Auto-mastering chain with true-peak safety rails, dynamics protection, and loudness matched A/B calculation.
   - `tts.rs`: Phoneme harmonic synthesis with token alignment and metadata synthetic labeling.
   - `voice_clone.rs`: `VoiceCloneConsent` multi-tier authorization gate and spread-spectrum high-frequency watermark embedding.
   - `dubbing.rs`: Multi-line duration fitting and background stem remixing engine.
   - `magic.rs`: Diagnosis evaluator and goal-based chain proposer.
2. Expose WASM and C ABI interfaces in `crates/flux-wasm`.
3. Provide domain models and tests in `packages/app-core/src/ai/`.
4. Replicate screens from `design/` in `apps/web/src/screens/`:
   - `#analysis`: `TempoKeyScreen`
   - `#master`: `AutoMasterScreen`
   - `#tts`: `TextToSpeechScreen`
   - `#dubreview`: `DubReviewScreen`
   - `#clone`: `VoiceCloneConsentScreen`
   - `#magic`: `MagicAssistantScreen`
5. Verify desktop visual replica against `design/` prototype with Playwright pixel-diff (all screens exceeding 94% match).

## Consequences
- **Positive:** Bit-accurate, non-destructive, and deterministic intelligent processing; safeguards prevent voice cloning abuse and enforce ethical AI watermarking; complete compliance with design tokens and responsive layouts.
- **Negative:** Full acoustic modeling requires hardware acceleration when processing multi-hour files.
