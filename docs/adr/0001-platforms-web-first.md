# ADR-0001: Target Platform: Web App (PWA) First

- **Status:** Accepted
- **Deciders:** Flux Engineering
- **Date:** 2026-11-03
- **Traceability:** PLAN.md §1 (D1)

## Context
Audio editing software has historically been distributed as platform-specific native desktop applications (C++/JUCE). However, building native applications for macOS, Linux, Windows, iOS, and Android creates fragmented codebases, slower iteration speed, and high installation friction for users. Modern web capabilities (WebAssembly SIMD, AudioWorklet, WebGPU, Origin Private File System, WebCodecs) enable desktop-grade digital signal processing directly in browser engines.

## Decision
1. Build Flux exclusively as a Web Application (Progressive Web App - PWA) through Phase 9.
2. Ensure full desktop browser support for Chrome, Edge, Firefox, and Safari, and responsive support for mobile browsers (simple tasks, playback, export).
3. Defer desktop packaging (Phase 10: portable Windows x64 ZIP containing WebView2 and bundled dependencies) until Phase 9 is complete and the user explicitly requests it.
4. Exclude native macOS, native Linux, and native mobile apps from the project scope.

## Consequences
- **Positive:** Single unified UI and DSP engine codebase; zero-friction user onboarding with no installation; instant cross-platform updates.
- **Negative:** Dependent on browser support for cutting-edge APIs (`AudioWorklet`, `WebGPU`, `SharedArrayBuffer` via COOP/COEP); AudioWorklet quantum size is fixed at 128 frames.
- **Mitigations:** Graceful fallbacks for browsers lacking WebGPU (WASM-SIMD CPU fallback) and SharedArrayBuffer (structured clone message passing).
