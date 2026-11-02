# Flux

A professional audio editor that runs in your browser: non-destructive editing, real 3D and 8D spatial audio, creative effects, and on-device AI for stems, voice cleanup, and editing audio by editing text.

> **Status: pre-alpha.** Foundations in progress. Monorepo skeleton and automated CI harness established. See [What works today](#what-works-today).

## What works today

- **Monorepo & build harness (P0.1-a):** Cargo workspace with 5 core crates (`flux-dsp`, `flux-engine`, `flux-ml`, `flux-wasm`, `flux-testkit`), TypeScript packages (`@flux/ui`, `@flux/app-core`, `@flux/web`), cross-platform `just ci` gate verifying all targets including `wasm32-unknown-unknown`.
- **Linting, formatting & typo validation (P0.1-b):** Configured `rustfmt`, `clippy -D warnings`, `eslint`, `prettier`, and `typos` across Rust and TypeScript codebase, integrated into `just ci`.

## What Flux will do

| Area | Features |
|------|----------|
| **Editing** | Multitrack timeline, waveform and spectrogram views, sample-accurate cuts, fades and crossfades, automation, unlimited undo with a visual history |
| **Core tools** | Parametric EQ, compressor, limiter, de-esser, loudness normalization, speed and pitch change, noise and hum removal, silence remover, full metering |
| **Spatial** | Binaural 3D placement, 8D auto-orbit, spirals and custom paths drawn on a sound stage, room simulation and a custom room designer, concert hall mode, "next door" effect |
| **Creative** | Slowed + reverb, nightcore, one-slider lo-fi, vinyl, cassette, old radio, phone, underwater, stutter, tape stop, glitch, voice changer |
| **AI** | Stem separation and vocal removal, voice cleanup, transcript editing (delete a word, remove the audio), filler word removal, BPM and key detection, auto-mastering, text-to-speech, translation and dubbing, one-tap Magic |
| **Workflow** | Effect chains and presets, before/after compare, batch processing, format converter, ringtone maker, beginner and pro modes |

The full list, with quality targets for each feature, is in [PLAN.md](PLAN.md).

## Principles

- **What you hear is what you export.** Preview and export use the same audio engine.
- **Your source files are never modified.** Every edit can be undone.
- **Private by default.** Processing happens on your device. Cloud features are opt-in, per feature.
- **Simple or deep.** Each tool has a one-slider surface and a full parameter panel.
- **Measured quality.** Each feature has objective targets enforced by automated tests.

## Platforms

Flux is a **web app**, installable as a PWA and usable offline, in any modern browser. A **portable Windows app** is planned for later: one zip containing `Flux.exe` and every DLL and tool it needs, with nothing to install.

## Tech

Rust DSP engine compiled to WebAssembly, React + TypeScript UI, ONNX Runtime for on-device AI.

## Running Flux

To run the local gate and automated checks:

```bash
just ci
```

## Project documents

| Document | What it covers |
|----------|----------------|
| [PLAN.md](PLAN.md) | Architecture, phases, quality targets, tests |
| [PROGRESS.md](PROGRESS.md) | Status of every feature |
| [AGENTS.md](AGENTS.md) | How work is done on this repo |
| [DESIGN.md](DESIGN.md) | How Flux looks and moves |
| [LICENSES.md](LICENSES.md) | Third-party licences |
| [design/](design/) | The prototype: landing page and all 87 app screens. Run `python -m http.server 4173 --directory design` to view it |

## Licence

Not decided yet.
