# Flux

A non-destructive audio editor for Windows, macOS (Apple silicon and Intel) and Linux, with an MCP server so
AI agents can drive every feature.

> **Status: v2 agent-complete.** The desktop app (engine, every screen, Live, MCP, auto-update), the website and
> the release pipeline are built and tested on this machine. Not yet done: running the voice cloning model (waits
> for permission to download it), the first GitHub release run (waits for a push), and listening checks. See
> [PLAN.md](PLAN.md) and [PROGRESS.md](PROGRESS.md).

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
- **Command API.** 162 methods in one registry, shared by the UI, the MCP server and the tests.
- **MCP server (`crates/flux-mcp`).** Every method is an MCP tool over stdio. It drives the running app through a
  token-protected localhost port, or runs headless when the app is closed. See [docs/MCP.md](docs/MCP.md).

- **Voice cloning, with consent.** Flux checks the computer (graphics chip, video memory, memory) and picks a local
  model: Chatterbox Turbo (MIT), which performs `[laugh]`, `[sigh]`, `[cough]` and other sounds written in a script
  and says hesitations in the cloned voice; Chatterbox (MIT) to turn your own recording into the voice, keeping your
  timing, laughs and hesitations exactly;
  OpenVoice V2 (MIT) where memory is tight. A voice is only made with the owner's consent recording and a clip, kept
  on this computer. Scripts take three kinds of placeholder, inserted by typing or from chips: sounds (`[laugh]`,
  `(sighs)`, `*chuckles*`), hesitations said in the voice (`[hmm]`, `[um]`, `[uh]`, or plain words) and exact
  silences (`[pause]`, `[long pause]`, `[pause 2s]`). `clone.script` shows the pieces the generator will speak and the
  silences between them. **Speaking in the voice is not working yet:** the model download waits for permission, so
  `clone.speak` and `clone.convert` say exactly what they would use and why they can't yet.

- **Website (`site/`).** A landing page and a download page with no web app. The download page reads the latest
  GitHub release in the visitor's browser and offers the right installer for their computer. `npm run build:static`
  writes `dist-static/` for Cloudflare Pages, with a content security policy.

Not available without bundled models: transcript editing, filler-word removal and dubbing. These methods return
`unavailable` instead of fake results.

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

```bash
npm run test:site
```

## Privacy

Everything runs on your computer. Flux never uploads audio, transcripts or file names. Diagnostics never include them.

## Licence

MIT OR Apache-2.0. Third-party licences are listed in [LICENSES.md](LICENSES.md).
