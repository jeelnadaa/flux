# ADR-0010: Cloud Provider Interface with Local Mocking

- **Status:** Accepted
- **Deciders:** Flux Engineering
- **Date:** 2026-11-03
- **Traceability:** PLAN.md §1 (D10), §3

## Context
Cloud GPU backends incur immediate operational hosting and API costs. Developing features directly against live paid cloud infrastructure wastes budget during early development cycles and blocks offline development.

## Decision
1. Decouple all cloud services behind a strict typed client provider interface (`CloudJobClient`).
2. Implement and test cloud functionality initially against a lightweight local mock service (`services/cloud` mock running in development/CI).
3. Defer provisioning and funding of production GPU cloud clusters until on-device paths are fully stabilized and user demand warrants cloud scaling.
4. The local mock must simulate real network behavior: presigned upload URLs, asynchronous processing status polling, simulated retries, simulated failures, and automated TTL asset deletion.

## Consequences
- **Positive:** Zero cloud spending during Phases 0 through 8; full offline testability of cloud client UX and failure paths.
- **Negative:** Cloud deployment latency and real GPU cold-start metrics must be verified later during Phase 9 hardening.
