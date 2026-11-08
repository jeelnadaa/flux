# Flux

A non-destructive audio editor for Windows, macOS (Apple silicon and Intel) and Linux, with an MCP server so
AI agents can drive every feature.

> **Status: v2 agent-complete.** The desktop app (engine, every screen, Live, MCP, auto-update), the website and
> the release pipeline are built and tested on this machine. Voice models download and run from the app; Chatterbox Turbo has run
> for real on an NVIDIA chip and on a processor. Not yet done: listening checks and the other models on real hardware
> (queued). See
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
- **First run.** A short setup shows once: mode, headphone check, a head shape for 3D sound, and where exports go.
  Settings > Audio > Sound profile runs the headphone and head-shape steps again.
- **Live (desktop app, Live page).** Put a preset on everything the computer plays (Windows and macOS capture an
  output device; Linux uses Monitor inputs) or on a microphone, and play the result to any output, with in and out
  meters and a bypass. 41 built-in live presets (21 for computer sound, 20 for a microphone). The designer builds
  anything from 39 effects in parallel lanes and a post chain, moved by modulators (LFOs, random, step sequencer,
  envelope follower, tempo sync) and macros; export and import `.fluxlive` files. A built-in preset is never
  changed: the first edit makes your own copy. Editing happens in a dialog: Save keeps the changes; Discard or Esc
  asks, then puts the preset back. Built-ins play 3 LU below their input by default. Every control has a tooltip.
- **Tooltips.** Every control in the app explains itself on hover or keyboard focus (2784 controls on 92 screens,
  checked by the end-to-end test).
- **Command API.** 179 methods in one registry, shared by the UI, the MCP server and the tests.
- **MCP server (`crates/flux-mcp`).** Every method is an MCP tool over stdio. It drives the running app through a
  token-protected localhost port, or runs headless when the app is closed. See [docs/MCP.md](docs/MCP.md).

- **Voice cloning, with consent.** Four local models, all MIT (Resemble AI): Chatterbox Turbo (2.99 GB), which
  performs `[laugh]`, `[sigh]`, `[cough]` and other sounds written in a script and says hesitations in the cloned
  voice; Chatterbox (3.19 GB, English, with an emotion control); Chatterbox Multilingual (3.21 GB, 23 languages); and
  the Chatterbox voice converter (1.06 GB), which turns your own recording into the voice, keeping your timing, laughs
  and hesitations exactly. Flux checks the computer (graphics chip, video memory, memory) and suggests one.
  **Download in the app:** the Download dialog shows the size, licence, source and folder first; the first download
  also installs the voice engine (portable Python 3.11, PyTorch 2.6 for NVIDIA, Apple or processor, and Chatterbox
  0.1.7, all pinned). Downloads show progress, can be cancelled and resume where they stopped, and every file is
  checked against its SHA-256 before use. Delete keeps files another model shares.
  **Your choice:** any downloaded model for speech or conversion, the device, the language, a seed for repeatable
  takes, each model's own settings (sliders), or a folder of your own compatible model.
  A voice is only made with the owner's consent recording and a clip, kept on this computer. Scripts take three
  kinds of placeholder, inserted by typing or from chips: sounds (`[laugh]`, `(sighs)`, `*chuckles*`), hesitations
  said in the voice (`[hmm]`, `[um]`, `[uh]`, or plain words) and exact silences (`[pause]`, `[long pause]`,
  `[pause 2s]`). Speech lands on a "Generated speech" track (one undo step), or in the export folder with no project
  open, and the WAV says it is synthetic, whose voice, and which model made it. Models run with the Hugging Face Hub
  switched off; nothing is sent anywhere.

- **Website (`site/`).** A landing page and a download page with no web app. The download page reads the latest
  GitHub release in the visitor's browser and offers the right installer for their computer. `npm run build:static`
  writes `dist-static/` for Cloudflare Pages, with a content security policy.

- **Smart tools: pick the model.** Stems, transcripts and text to speech each have a built-in or light engine
  and downloadable models, and Flux suggests one for this computer (graphics chip, memory, cores) with the reason.
  Stems: the built-in split (instant, no download), Demucs v4 (balanced), Demucs fine-tuned (best) or Demucs 6 stems
  (adds guitar and piano). Transcripts: Whisper Base, Small or Large v3 Turbo through faster-whisper, with every word
  at an exact sample position, filler words and repeats found, and a stale flag once the project changes. Text to
  speech: the system voices, Kokoro (6 voices, speed) or Chatterbox Turbo's own voice; model speech is labelled
  synthetic in the WAV. All MIT or Apache-2.0; each model installs only the packages it needs, once, and runs on
  this computer. Choose with `ai.engine.set`; see ADR 0005.

- **Online engines, only when you choose them.** Free ones with no key: Whisper Large v3 (text only), Kokoro (6
  voices) and a vocal/instrumental split, all public Hugging Face Spaces that can be slow or busy. With your key:
  Whisper on Groq or OpenAI and Deepgram Nova-3 (with word timing), OpenAI speech and ElevenLabs, and Demucs on
  Replicate. Flux never suggests them; choosing one asks first and says what is sent where. Keys come from an
  environment variable (`OPENAI_API_KEY` and the like) or the system keychain, and are never shown, logged or saved
  in Flux's files. The OpenAI engines also work with any OpenAI-compatible server. See ADR 0006.

- **Flux speaker and Flux microphone.** Pick the Flux speaker as the output in any app and Live plays it through a
  preset to your headphones, so you hear only the processed sound. Pick the Flux microphone in a call app to send it
  your processed voice. On Linux (PipeWire or PulseAudio) Flux adds both itself, with no administrator password. On
  Windows and macOS, install a free virtual audio cable and Flux finds it and uses it (Live says when none is installed
  and has a step-by-step guide): a device of Flux's own needs a paid, signed driver (ADR 0007). The Linux path has only been tested against a stand-in so far.

- **Smart tools in Settings.** One page lists every engine for stems, transcripts and speech with its size,
  licence and tier, marks the one suggested for this computer, and downloads, uses or removes it. Online engines
  are turned on there, with what is sent and where, and keys are added or forgotten. Split into stems shows the
  installed engines as tiles; Text to speech has an engine menu. The Transcript editor shows every word: click
  one to hear it or cut it from the audio, and Filler review removes um, uh and stutters in one go.

Not available yet: dubbing (`dub.start` returns `unavailable`). The Smart tools picker in Settings is next.

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

Everything runs on your computer unless you turn on an online engine for a job; then only that job's audio or
text goes to the service you chose. Flux never logs audio, transcripts, keys or file names, and diagnostics never
include them.

## Licence

MIT OR Apache-2.0. Third-party licences are listed in [LICENSES.md](LICENSES.md).
