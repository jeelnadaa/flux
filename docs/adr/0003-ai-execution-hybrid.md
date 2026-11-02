# ADR-0003: Hybrid AI Execution (On-Device WebGPU / Cloud GPU Fallback)

- **Status:** Accepted
- **Deciders:** Flux Engineering
- **Date:** 2026-11-03
- **Traceability:** PLAN.md §1 (D3), §2

## Context
AI-powered audio tools range from lightweight models (real-time voice activity detection, small denoisers) to heavy multi-gigabyte neural networks (full 4-stem separation, multilingual whisper transcription, text-to-speech, neural dubbing). Running all models purely on client hardware causes out-of-memory crashes on low-end devices, while running all models in the cloud incurs severe privacy loss and high recurring server GPU costs.

## Decision
1. Adopt a hybrid execution architecture:
   - **On-Device by Default:** Run models via ONNX Runtime Web using WebGPU when available, falling back to WASM-SIMD CPU execution.
   - **Cloud GPU Optional & Opt-in:** Heavy jobs (e.g. 4-stem HTDemucs separation, dubbing synthesis, large TTS) offer an opt-in cloud path behind clear user consent.
2. Store derived AI outputs locally in the project cache keyed by `hash(source_audio + model_version + inference_parameters)` to prevent unnecessary re-computation.
3. Cap client-side WASM memory allocation safely within browser buffer limits using chunked overlap-add streaming.

## Consequences
- **Positive:** Privacy by default; zero server costs for basic AI features; high accessibility on varying device tiers.
- **Negative:** Increased complexity in handling backend selection, model asset downloading, caching, and graceful failure paths.
