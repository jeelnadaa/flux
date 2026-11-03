# STRIDE Threat Model & Security Architecture

- **Product:** Flux Web Audio Editor
- **Version:** 1.0.0-rc1
- **Date:** 2026-11-03
- **Traceability:** PLAN.md §9.3, AGENTS.md, ADR-0004, ADR-0010, ADR-0014

## Executive Summary
Flux is built on a "local-first, privacy-by-design" architecture. Audio processing, ML inference, and project persistence execute on-device inside sandboxed browser environments (WebAssembly and WebGPU) with zero telemetric surveillance of user audio. Cloud features are strictly opt-in, ephemeral, and protected by end-to-end TLS and automated 24-hour time-to-live (TTL) hard deletion.

---

## 1. STRIDE Threat Matrix

| Threat Category | Potential Attack Vector | Impact | Mitigation in Flux |
|---|---|---|---|
| **Spoofing (S)** | Malicious third-party masquerading as an official model or synthetic speech impersonation. | User executes tampered weights or unauthorized voice impersonation. | - Signed model manifests (`SignedModelManifest`) enforce SHA-256 integrity checks before execution.<br>- Voice cloning requires 3-tier attestation, explicit spoken consent validation, and embeds an inaudible spread-spectrum watermark (ADR-0014). |
| **Tampering (T)** | Corrupted or hostile audio/project bitstreams (e.g. decompression bombs, malformed `.flux` packages, path traversal). | Memory exhaustion, crash, or data corruption. | - CRC32 checksums on all append-only journal entries.<br>- Hostile parser fuzzing (`fuzz_parsers`) and strict 4 GB / 300 MB bounds checking.<br>- Path sanitization disallows `../` in archive unpackers. |
| **Repudiation (R)** | User claims unauthorized cloud billing or denies generating synthetic voice content. | Disputes over generated content or resource consumption. | - Audit trail in local journal logs actions with monotonic sequence IDs.<br>- Imperceptible high-frequency watermarking in synthetic voice outputs provides cryptographically verifiable attribution. |
| **Information Disclosure (I)** | Leaking user audio recordings, transcripts, or project filenames to third parties or host servers. | Severe user privacy violation. | - Zero cloud telemetry of audio or transcripts.<br>- Ephemeral cloud processing with 24-hour auto-deletion TTL.<br>- Full on-device execution default (WebGPU + WASM SIMD). |
| **Denial of Service (D)** | AudioWorklet thread starvation, memory bloat, decompression bombs, or audio feedback loops. | Browser freeze, tab crash, speaker blowout, hearing damage. | - Hard safety limiter clamping signals at +0.0 dBFS with soft-start ramp ($\ge 10$ ms).<br>- `AdaptiveQualityGovernor` monitors xruns and automatically steps down processing tiers.<br>- Memory pool caps at 1 GB total / 300 MB audio heap. |
| **Elevation of Privilege (E)** | XSS injecting malicious scripts into the web editor or AudioWorklet scope escaping sandbox. | Arbitrary code execution in browser context. | - Strict Content Security Policy (CSP) blocking unauthorized scripts, eval, and cross-origin frames.<br>- Sandboxed WebWorkers and zero third-party tracking scripts. |

---

## 2. Content Security Policy (CSP)

Flux serves the following strict CSP header:

```http
Content-Security-Policy: default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; worker-src 'self' blob:; connect-src 'self' data: blob:; img-src 'self' data: blob:; font-src 'self'; object-src 'none'; base-uri 'self';
```

- `'wasm-unsafe-eval'` is permitted strictly for WebAssembly compilation while blocking JavaScript `eval()`.
- `object-src 'none'` blocks Flash and plugin embedding.
- Font requests are restricted to `'self'` (all 6 fonts are locally bundled).

---

## 3. Cross-Origin Isolation (COOP & COEP)

To enable high-performance `SharedArrayBuffer` lock-free ring buffers between the UI thread and `AudioWorklet`:

```http
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

When headers are unavailable (e.g. static local file execution), `PlatformCapabilitiesDetector` smoothly falls back to standard `MessagePort` chunk streaming without blocking the UI.
