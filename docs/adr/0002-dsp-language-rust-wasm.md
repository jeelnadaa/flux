# ADR-0002: DSP Engine in Rust with WebAssembly and Native Compilation

- **Status:** Accepted
- **Deciders:** Flux Engineering
- **Date:** 2026-11-03
- **Traceability:** PLAN.md §1 (D2), §0 principle 1 & 2

## Context
Production-grade audio processing requires 32-bit floating-point precision, sample-accurate scheduling, zero heap allocations on the audio render thread, and lock-free concurrency. The DSP engine must run with identical numerical output in real-time within an `AudioWorklet` and in offline export workers.

## Decision
1. Implement the DSP engine (`flux-dsp`) and engine orchestration (`flux-engine`) in Rust using the stable toolchain.
2. Compile the DSP core to WebAssembly (`wasm32-unknown-unknown` with `simd128`) for the web application via `wasm-bindgen`.
3. Retain native target compilation support so that the future Windows desktop shell (Phase 10) reuses the exact same DSP code.
4. Enforce identical rendering behavior: the same DSP code runs in real time (in `AudioWorklet`) and offline (in Web Workers or native threads), verifiable by automated null tests (≤ −100 dBFS residual).

## Consequences
- **Positive:** Memory safety, strict concurrency guarantees, lock-free primitives without garbage collection pauses, high performance via WASM SIMD.
- **Negative:** WASM lacks native Flush-To-Zero / Denormals-Are-Zero (FTZ/DAZ) CPU flags, requiring explicit software anti-denormal offsets or state flushing in recursive biquad/IIR filters.
