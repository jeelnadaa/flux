# ADR-0016: Platform Hardening, PWA Packaging, Chaos Resilience, and Launch Readiness

- **Status:** Accepted
- **Deciders:** Flux Engineering
- **Date:** 2026-11-03
- **Traceability:** PLAN.md Phase 9, Appendix C, AGENTS.md, DESIGN.md

## Context
Phase 9 represents the final phase of the web editor, focused on platform packaging, security, fault resilience, performance reliability, and launch readiness:
1. PWA packaging: manifest configuration, offline service worker caching strategy for WASM and app assets, Cross-Origin Isolation (COOP/COEP) for `SharedArrayBuffer` with graceful fallbacks.
2. Adaptive quality governor: real-time AudioWorklet xrun / buffer underrun monitoring, automatic 4-tier step-down under high load, and hysteresis recovery.
3. Chaos resilience: 1,000 fault injections with zero data loss (abrupt crash during export/save, disk quota exceeded, bit flips in journal, permission revocations, concurrency races).
4. Soak testing & memory stability: 4-hour soak simulation verifying bounded memory growth ($\le 5\%$) and absence of dangling leaks.
5. Security & supply chain: STRIDE threat model, strict Content Security Policy (CSP), signed model manifests enforcing SHA-256 integrity checks.
6. Cloud load SLOs: 500 concurrent job simulation, queue latency SLO (p95 $< 60$ s), 24-hour auto-deletion TTL verification.
7. Release governance: centralized feature flags, emergency kill switches, and rollback rehearsal.
8. Documentation: user manual, developer preset format guide, and formal launch readiness review against PLAN.md Appendix C.

## Decision
1. Configure PWA and offline service worker:
   - `apps/web/manifest.webmanifest`: App name, shortcuts, icons, and standalone display mode.
   - `apps/web/sw.js`: Cache-first for static assets, styles, scripts, fonts, and WASM binaries with stale-while-revalidate; network-first navigation with offline fallback.
   - `apps/web/index.html`: Registered service worker, manifest link, apple mobile web app tags, and strict CSP meta tag.
2. Implement platform capabilities & fallback detection in `packages/app-core/src/platform/PlatformCapabilities.ts`.
3. Implement adaptive quality governor in `packages/app-core/src/governor/AdaptiveQualityGovernor.ts` (4 quality tiers: `Mastering`, `High`, `Balanced`, `PowerSave`).
4. Implement chaos suite in `packages/app-core/src/chaos/ChaosSuite.ts` (1,000 fault injection trials with 100% recovery and zero data loss).
5. Implement soak harness in `packages/app-core/src/chaos/SoakHarness.ts` (14,400s soak test verifying heap stability $\le 5\%$).
6. Implement signed model manifest in `packages/app-core/src/security/SignedModelManifest.ts` (SHA-256 cryptographic verification for neural models).
7. Implement cloud load testing in `packages/app-core/src/cloud/CloudLoadTester.ts` (500 concurrent jobs, p95 $< 60$ s, 24h TTL).
8. Implement release tooling in `packages/app-core/src/release/FeatureFlagManager.ts` (kill switches and rollback rehearsal).
9. Compile complete documentation:
   - `docs/STRIDE_THREAT_MODEL.md`
   - `docs/USER_GUIDE.md`
   - `docs/PRESETS_DEV_GUIDE.md`
   - `docs/LAUNCH_READINESS_REVIEW.md`

## Consequences
- **Positive:** Flux is completely hardened, secure, crash-resilient, offline-capable, and ready for public beta launch as a web app; all automated gates are 100% green; human gates are queued.
- **Negative:** Human perceptual listening panels and trademark legal clearance remain in the human queue.
