# ADR-0006: Offline-First Architecture via Service Workers and OPFS

- **Status:** Accepted
- **Deciders:** Flux Engineering
- **Date:** 2026-11-03
- **Traceability:** PLAN.md §1 (D6), §2

## Context
Standard browser web storage (`localStorage`, basic `IndexedDB`) lacks high-performance random-access I/O for multi-gigabyte audio files and can be evicted by the browser under storage pressure. Web users expect audio editing to work reliably even without network connectivity.

## Decision
1. Implement a complete offline-first Progressive Web App (PWA) architecture using Service Workers to precache all application assets (WASM modules, fonts, UI scripts, styles).
2. Use the Origin Private File System (OPFS) with synchronous access handles in Web Workers for high-throughput, chunked streaming audio I/O.
3. On Chromium browsers, support the File System Access API so users can directly open and edit local folders on disk.
4. Request `navigator.storage.persist()` on project initialization to prevent browser eviction, and provide regular warnings if storage quota falls below safety thresholds.

## Consequences
- **Positive:** Full functionality in airplanes, studios, and offline environments; non-destructive project safety.
- **Negative:** Storage quotas and persistence policies differ across browser engines (notably Safari 7-day eviction heuristics without installation), requiring prominent `.flux` package export prompts.
