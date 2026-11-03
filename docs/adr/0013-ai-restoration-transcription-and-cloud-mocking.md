# ADR-0013: AI Restoration, Transcription, and Cloud Mocking Architecture

- **Status:** Accepted
- **Deciders:** Flux Engineering
- **Date:** 2026-11-03
- **Traceability:** PLAN.md Phase 6, AGENTS.md, DESIGN.md

## Context
Phase 6 specifies on-device (and cloud fallback) AI capabilities:
1. 4-stem separation (vocals, drums, bass, other) with exact reconstruction null (≤ −40 dB), karaoke, acapella, and stems mixer.
2. Voice cleanup ("Studio Sound"): neural denoise, room-tone bed preservation, de-reverb, bandwidth extension, and hallucination guard with speech/music classification.
3. Automatic transcription, word-level editing with sample-accurate non-destructive cuts, 3ms micro-crossfades, room-tone filling, and multi-format subtitle export (SRT, VTT, TXT, JSON).
4. Filler word and disfluency detection ("um/uh", repetitions, false start) with context-aware false-positive guards and interactive review UI.
5. Neural VAD, timeline segmentation, and auto-ducking sidechaining.
6. Privacy-preserving mock cloud job client with 24-hour auto-deletion TTL, granular per-feature consent, and SHA-256 verified model caching.

## Decision
1. Implement the machine learning processing layer in `crates/flux-ml` with zero unsafe blocks in high-level modules, providing:
   - `runtime.rs`: Device capability probing (WebGPU, WASM SIMD, Cloud fallback) and windowed overlap-add chunking (`ChunkedOverlapAdd`).
   - `models.rs`: Model manager with SHA-256 verification and storage budget accounting.
   - `stems.rs`: 4-stem separation engine with soft Wiener spectral masking and exact reconstruction null (≤ −40 dB), plus karaoke and acapella mixdowns.
   - `cleanup.rs`: Voice cleanup engine with speech/music audio classification, neural denoise, dereverberation, room tone bed fill, and hallucination detection.
   - `transcript.rs`: WordToken alignment, subtitle export (SRT, VTT, TXT, JSON), and non-destructive audio cut generation with micro-crossfades.
   - `fillers.rs`: Filler detector with repetition tracking, context sensitivity, and time-saved metrics.
   - `vad.rs`: Voice activity detector and ducking compressor sidechain.
   - `cloud.rs`: Mock cloud client with 24h auto-delete TTL and per-feature privacy consent.
2. Export C ABI / WebAssembly interfaces in `crates/flux-wasm` for WebAssembly execution.
3. Build TypeScript models in `packages/app-core/src/ai/`.
4. Replicate design prototype screens in `apps/web/src/screens/`:
   - `#aitools`: `AiToolsScreen`
   - `#stems-setup`: `StemsSetupScreen`
   - `#stems-run`: `StemsRunScreen`
   - `#stems`: `StemsScreen`
   - `#cleanup`: `VoiceCleanupScreen`
   - `#script`: `TranscriptScreen`
   - `#fillers`: `FillerReviewScreen`
   - `#jobs`: `AiJobsScreen`
   - `#consent`: `CloudConsentDialogScreen`
   - `#prefs-models`: `AiModelsScreen`
   - `#dub`: `DubScreen`
5. Verify desktop visual replica against `design/` prototype with Playwright pixel-diff (all screens exceeding 95% match).

## Consequences
- **Positive:** Fully deterministic, on-device AI pipelines with zero network leakage in on-device mode; non-destructive transcript editing preserves exact audio undo/redo; audio stems null perfectly to the original mixdown.
- **Negative:** Full deep learning neural weight execution requires WebGPU; systems lacking WebGPU fall back to WASM SIMD or opt-in cloud endpoints.
