# ADR-0009: Automated CI Gates with Human & Hardware Queues

- **Status:** Accepted
- **Deciders:** Flux Engineering
- **Date:** 2026-11-03
- **Traceability:** PLAN.md §1 (D9), §5.4, AGENTS.md

## Context
Antigravity and autonomous coding agents can write, test, and verify code at massive speeds. However, certain validation gates cannot be genuinely completed by an AI agent or in virtual environments alone: listening panels (ABX/MUSHRA perceptual checks), real-device audio hardware checks (physical sound cards, iOS latency, hardware Bluetooth codec latency), full benchmark runs on non-redistributable corpora (MUSDB18, LibriSpeech), and formal legal/trademark clearance.

## Decision
1. Classify all verification criteria into four strict gate types:
   - **[A] Automated:** Unit, property, golden, null, integration, E2E, axe-core, license scan, WASM build. Cleared autonomously by the agent in `just ci` and blocks the commit if failing.
   - **[D] Dataset:** Large or restricted corpora. The agent executes on synthetic or open subsets, then queues the full evaluation in `PROGRESS.md`.
   - **[R] Real device:** Physical hardware audio tests. The agent validates against throttled browser proxies, then queues the real-device test in `PROGRESS.md`.
   - **[H] Human judgment:** Listening panels, legal review, usability studies. Queued in `PROGRESS.md`.
2. A phase reaches **Agent-complete** when all [A] gates pass and remaining [D], [R], [H] gates are formally queued. A phase reaches **Released** only when real humans clear the queued items.
3. Agents are strictly forbidden from simulating ears, estimating perceptual metrics, or self-certifying non-automated gates.

## Consequences
- **Positive:** Uncompromising technical honesty; prevents fake test passes; maintains clear audit trails between automated development and human oversight.
- **Negative:** Human gates require manual attention before official releases can be tagged.
