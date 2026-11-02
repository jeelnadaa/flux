# ADR-0004: Accountless On-Device Usage and Opt-In Cloud Features

- **Status:** Accepted
- **Deciders:** Flux Engineering
- **Date:** 2026-11-03
- **Traceability:** PLAN.md §1 (D4)

## Context
Many web applications force mandatory user sign-up and authentication before granting access to basic client-side functionality. This introduces friction, telemetry tracking, and privacy concerns for creators working with sensitive or confidential audio.

## Decision
1. No user account or sign-in is required for any on-device capability in Flux (recording, timeline editing, DSP rack, spatial audio, local export, and on-device AI).
2. User authentication and accounts exist solely for opt-in cloud GPU processing tiers and paid subscription plans.
3. The editor shell loads instantly in guest mode with full features available immediately.

## Consequences
- **Positive:** Zero friction for first-time visitors; enhanced user trust; zero telemetry or audio data collection on default paths.
- **Negative:** Cloud tier authentication state must be cleanly isolated so guest users are never prompted unexpectedly.
