# Flux

A non-destructive audio editor for Windows, macOS (Apple silicon and Intel) and Linux, with an MCP server so
AI agents can drive every feature.

> **Status: rebuild in progress (v2).** The old browser app was removed on 2026-11-07. The engine and the MCP server
> are done and tested; the desktop app, site and release pipeline are next. See [PROGRESS.md](PROGRESS.md).

## What works today

- **Engine (`crates/flux-core`).** One renderer for playback and export, so real-time output nulls against offline
  output (< 1e-5). An unprocessed render is bit-exact with the source. Everything is 32-bit float.
  - Decodes WAV, AIFF, FLAC, MP3, OGG, M4A/AAC and ALAC. Source files are never modified.
  - Edits: split, trim, move, fades, clip gain, cut/copy/paste ranges, markers and automation lanes. Every edit is
    undoable, and undo followed by redo gives an identical project hash.
  - 45 effects: EQ, dynamics, reverb, delay, clean-up, speed/pitch, 8D spatial, creative and voice. Parameter
    changes glide over 20 ms, so there are no clicks.
  - Analysis: BS.1770 loudness and true peak, tempo, key, spectrum, spectrogram, hum, noise floor and clipping.
  - Export: WAV, FLAC, MP3 and OGG built in; AAC, M4R and Opus when `ffmpeg` is installed. Each export is measured
    by decoding the written file.
  - Tools: DSP stem split, auto-master (lands within 0.25 LU of the target), Magic fix, voice clean-up, silence
    removal, spectral repair, match EQ, and text to speech with system voices (always labelled synthetic).
  - Projects are saved atomically and journaled, with crash recovery, `.flux` packages, presets, recorded actions,
    batch, convert and ringtone.
- **Live (desktop app, Live page).** Put a preset on everything the computer plays (Windows and macOS capture an
  output device; Linux uses Monitor inputs) or on a microphone, and play the result to any output, with in and out
  meters and a bypass. 41 built-in live presets (21 for computer sound, 20 for a microphone). The designer builds
  anything from 39 effects in parallel lanes and a post chain, moved by modulators (LFOs, random, step sequencer,
  envelope follower, tempo sync) and macros; export and import `.fluxlive` files. A built-in preset is never
  changed: the first edit makes your own copy. Every control has a tooltip.
- **Tooltips.** Every control in the app explains itself on hover or keyboard focus (2784 controls on 92 screens,
  checked by the end-to-end test).
- **Command API.** 154 methods in one registry, shared by the UI, the MCP server and the tests.
- **MCP server (`crates/flux-mcp`).** Every method is an MCP tool over stdio. It drives the running app through a
  token-protected localhost port, or runs headless when the app is closed. See [docs/MCP.md](docs/MCP.md).

Not available without bundled models: transcript editing, filler-word removal, dubbing and voice cloning. These
methods return `unavailable` instead of fake results.

## Build and test

```bash
cargo test --workspace
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
cargo run -p flux-mcp -- --help
```

## Privacy

Everything runs on your computer. Flux never uploads audio, transcripts or file names. Diagnostics never include them.

## Licence

MIT OR Apache-2.0. Third-party licences are listed in [LICENSES.md](LICENSES.md).
