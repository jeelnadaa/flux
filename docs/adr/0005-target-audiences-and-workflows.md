# ADR-0005: Target User Personas and Dual-Mode UX (Simple vs Pro)

- **Status:** Accepted
- **Deciders:** Flux Engineering
- **Date:** 2026-11-03
- **Traceability:** PLAN.md §1 (D5), §0 principle 8

## Context
Audio editors frequently struggle with user interface complexity: they are either oversimplified "toy" apps with no depth, or dense Digital Audio Workstations (DAWs) overwhelming non-professionals. Flux targets three distinct groups:
1. Music listeners and enthusiasts (e.g. slowed + reverb, 8D audio, nightcore, ringtones).
2. Content creators and podcasters (voice cleanup, transcript editing, auto-ducking, loudness normalization).
3. Musicians and DJs (multitrack editing, stems, EQ, dynamics, tempo/key detection).

## Decision
1. Implement a dual-mode UX architecture:
   - **Simple Mode (Task-First):** One-slider macros, preset-first workflows, and immediate creative transformations without jargon.
   - **Pro Mode (Deep Engineering):** Full multitrack timeline, sample-accurate editing, 10-band parametric EQ, visual routing, metering (LUFS, true peak, goniometer), and explicit numerical parameters.
2. A single toggle in the header transitions seamlessly between Simple and Pro views without altering audio engine state or project data.

## Consequences
- **Positive:** Broad market appeal; progressive disclosure of technical depth; creators can begin simply and adopt pro tools as their needs grow.
- **Negative:** UI design must maintain coherent design tokens and strict visual consistency across both modes.
