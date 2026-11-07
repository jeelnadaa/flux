# PLAN.md — Flux v2 (desktop rebuild)

Flux v2 is a non-destructive audio editor for **Windows, macOS (Apple silicon and Intel) and Linux**, with an MCP
server so an AI agent can do anything a person can. It replaces the v1 browser app (archived in
[docs/archive/PLAN-v1-web.md](docs/archive/PLAN-v1-web.md)). The design prototype in `design/` is the specification
for every screen. Progress and the checklist live in [PROGRESS.md](PROGRESS.md).

Section numbers match the ones AGENTS.md refers to (§0, §1, §2, §4, §5).

## 0. Principles

1. **One engine.** The same DSP code renders playback and export, so they null (≤ 1e-5).
2. **Non-destructive.** Source audio is never modified. Every edit is an undoable command.
3. **32-bit float inside.** Dither only at export. Positions are samples, never pixels or milliseconds.
4. **Audio thread safety.** No allocation, locks, I/O or logging on the audio thread. Parameters arrive through
   lock-free queues and are smoothed (20 ms glide).
5. **Never lose work.** Every committed command is journalled. Every opened file is hostile input.
6. **On-device and private.** No cloud features. Audio, transcripts and file names are never logged or sent.
7. **Everything over MCP.** The UI, the MCP server and the tests call one command registry (`Api::call`). A feature
   isn't done until an agent can use it.
8. **Every control explains itself.** Every control in the app has a tooltip; every effect and parameter has a
   description in the engine, enforced by tests.
9. **Honest AI.** Synthetic speech is labelled. Voice cloning needs the owner's recorded consent. Features whose
   models aren't installed say so and return `unavailable`; they never fake a result.
10. **Measure, don't guess.** Audio tests measure rendered samples. Listening checks go to the human queue.

## 1. Decisions

| Decision | Choice | Record |
|---|---|---|
| Platforms | Windows x64, macOS arm64 and x64, Linux x64. No mobile, no web app. | [ADR 0001](docs/adr/0001-desktop-with-tauri.md) |
| Shell | Tauri 2; the UI is the `design/` prototype ported to plain JS; the engine runs in-process | [ADR 0001](docs/adr/0001-desktop-with-tauri.md) |
| Engine | Rust (`crates/flux-core`): symphonia decode, cpal devices, rustfft, own DSP | — |
| Agents | `flux-mcp` (stdio MCP): drives the open app over a local port with a token, or runs headless | — |
| Updates | Tauri updater, signed with minisign; checks 5 s after launch and every 6 h | — |
| Releases | Every push to `main` that changes the app publishes `v2.0.<run>` on GitHub | [ADR 0004](docs/adr/0004-release-on-every-push.md) |
| Site | Static landing and download pages on Cloudflare Pages; the download page reads the latest release | — |
| Live | Modular live presets on captured computer sound or a microphone | [ADR 0002](docs/adr/0002-live-presets.md) |
| Voice cloning | Local MIT models chosen per computer; consent-gated; placeholders for sounds, hesitations and pauses | [ADR 0003](docs/adr/0003-voice-cloning-models.md) |

Changing a decision in this table needs the user's approval.

## 2. Architecture

```
apps/desktop/ui          plain-JS UI ported from design/ (kit, screens, live.js, clone.js, tips.js)
apps/desktop/src-tauri   Tauri shell: window, file dialogs, drag and drop, updater, bundles flux-mcp
crates/flux-core         engine + command API (methods.rs registry, live.rs, voiceclone.rs, ...)
crates/flux-mcp          MCP server (stdio), `serve` HTTP mode for tests
site/                    landing.html, download.html, release.js -> dist-static/ (npm run build:static)
tests/e2e, tests/site    Playwright UI tests against the real engine; site tests
```

- The UI calls the engine through `FD.call(method, params)`; Tauri forwards it to `Api::call`. `flux-mcp serve`
  exposes the same call over HTTP so the UI runs in a browser for tests.
- The app writes `control.json` (port and random token) to its data folder; `flux-mcp` attaches through it.
- Long tasks are jobs (`job.get`). Errors carry a code: `bad_params`, `not_found`, `empty`, `unavailable`,
  `consent_required`, `internal`.

## 3. Features

R0 to R5 cover the rebuild of what v1 had (engine, effects, analysis, export, projects, stems, mastering, TTS,
desktop shell, MCP, site, CI). Their checklist is in PROGRESS.md. The two features added after the rebuild started:

### 3.1 Live (R7.1 to R7.7)

**Goal.** Whatever the computer plays, or a microphone, goes through a preset and out to any output device, live.

- **Routes.** Two independent routes: *computer sound* (WASAPI loopback on Windows; CoreAudio process tap on macOS
  14.6+; Monitor inputs on Linux) and *microphone*. Each picks a source, a preset and an output. Bypass compares.
  A route that would capture and play on the same device is refused (feedback guard). A drift-correcting resampler
  joins devices with different clocks.
- **Presets are modular.** Parallel lanes of effects, then a post chain, then a safety limiter. 39 live effect kinds.
  Modulators: sine, triangle, saw up/down, square, random steps, smooth random, step sequencer (up to 32 steps),
  envelope follower, and a constant; free rate or tempo-synced. Any modulator can drive any parameter with depth.
  Macros move several parameters with their own ranges and curves.
- **Built-ins.** 41 presets (21 for computer sound, 20 for a microphone). They are read-only; editing one makes a
  copy. User presets can be created, copied, randomized, exported and imported (`.fluxlive`, checked as hostile).
- **MCP.** 31 `live.*` tools, including `live.render` so agents can audition a preset offline.
- **Tests.** Routes run on the null device and meters move; real-hardware loopback reads −33.0 dB RMS for a
  −30 dBFS sine (expected −33.0); modulation and in-place updates fail their tests when broken on purpose; E2E
  builds a preset through the UI; every control tooltipped; no overflow at 1440, 820, 390 px in 13 themes × 6 fonts.
- **Human queue.** Listen to every preset; Live on real macOS and Linux machines.

### 3.2 Voice cloning (R7.8 to R7.10)

**Goal.** Make a voice from a short clip of someone who agreed, and speak scripts in it, with laughs, hesitations
and pauses where the script puts them. Or re-voice your own recording, keeping its timing exactly.

- **Model picked for the computer.** A probe reads memory, graphics chip and video memory. Chatterbox Turbo (MIT,
  about 1.4 GB) is the default: it clones from 5 to 20 s and performs paralinguistic tags. Chatterbox (MIT, about
  3.2 GB, 23 languages) does voice conversion when there is an NVIDIA chip with ≥ 6 GB or Apple silicon with
  ≥ 16 GB. OpenVoice V2 (MIT) is the lightest. Below 7.5 GB of memory nothing is offered.
- **Consent.** A voice needs the owner (you, or someone with permission), a recording of them reading a consent
  statement, a reference clip, and a ticked agreement. Clips are checked for length and level. Voices stay in the
  data folder and can be deleted with everything they hold.
- **Scripts.** Three kinds of placeholder, in `[]`, `()`, `{}`, `<>` or `**`:
  - sounds the voice performs: laugh, chuckle, sigh, gasp, cough, clear throat, sniff, groan, shush;
  - hesitations said in the voice: hmm, um, uh, mm-hmm, ah, oh, huh, uh-oh, well (plain words work too);
  - pauses as exact silence: beat (0.3 s), pause (0.5 s), long pause (1.2 s), or any length up to 5 s.
  Unknown tags are refused with the list. `clone.script` returns the parts, the text the model reads, and the
  spoken pieces and silences the generator will produce.
- **Your own voice as input.** `clone.convert` turns your recording into the cloned voice and keeps every laugh,
  hmm and pause where you made it.
- **Labelling.** Output is marked synthetic; Chatterbox adds an inaudible watermark.
- **Status.** Part 1 (picker, consent, profiles, parser, screen, 9 tools) is done. Part 2 (R7.9: download the
  weights with the user's permission and run them) is blocked on that permission.
- **Tests.** Picker for 9 hardware cases; consent refusals; parser cases incl. hesitations, pauses, clamping and
  refusals (fail when broken on purpose); E2E makes a voice through the UI and checks the script read-out.
- **Human queue.** Listening checks of cloned speech, laughs, sighs and hesitations once the model runs.

## 4. Licences

Only permissive licences (MIT, Apache-2.0, BSD, ISC, Zlib, OFL for fonts) without asking the user. Every dependency
or model goes in [LICENSES.md](LICENSES.md); `cargo deny check` runs in CI. Model weights are checked on their own
model cards, never committed, and downloaded only with the user's permission. Models limited to research or
non-commercial use are not used (for example Dia, XTTS).

## 5. Testing

The v1 test strategy still applies: [docs/archive/PLAN-v1-web.md §5](docs/archive/PLAN-v1-web.md) defines the test
layers (§5.1), the 13 universal DSP invariants (§5.2), gate types A/D/R/H (§5.4) and threshold discipline (§5.5).
For v2 the gate is:

| Gate | Command |
|---|---|
| Format, spelling, lint | `cargo fmt --all --check`, `typos`, `cargo clippy --workspace --all-targets -- -D warnings` |
| Licences | `cargo deny check` |
| Engine, API, MCP | `cargo test --workspace` |
| UI end to end | `cargo build -p flux-mcp --release` then `npm run test:ui` |
| Site | `npm run build:static` then `npm run test:site` |
| Scripts parse | `npm run check:js` |

CI (`.github/workflows/ci.yml`) runs all of these on Linux and the Rust tests on Windows and macOS.
