# PROGRESS.md

Rebuild started 2026-11-07 at the user's request. The old web/WASM code was deleted; the old plan and log are in
[docs/archive/](docs/archive/). The design prototype in `design/` is unchanged and remains the specification.

## Current focus

Rebuild v2: native desktop app (Windows, macOS Apple + Intel, Linux), Flux MCP server, landing and download site,
CI/CD that publishes a GitHub release on every push to `main`.

## Checklist (v2 rebuild)

Legend: `[x]` done and tested · `[ ]` not started · `[~]` in progress · `[!]` blocked

### R0 Repo
- [x] R0.1 Delete old code and brag output; archive old PLAN/PROGRESS
- [ ] R0.2 New workspace, PLAN.md, AGENTS.md, ADRs

### R1 Engine (`crates/flux-core`)
- [x] R1.1 Project model, edit commands, undo/redo (snapshot), project hash
- [x] R1.2 Decode (WAV, AIFF, FLAC, MP3, OGG, M4A/AAC via symphonia), media pool, safe copy
- [x] R1.3 Renderer (one engine for real time and offline), clip gain/fades, pan, mute/solo
- [x] R1.4 Effects: EQ, dynamics, reverb/delay, clean-up, speed/pitch, spatial 8D, creative, textures, glitch, voice
- [x] R1.5 Analysis: peaks, spectrogram, LUFS/true peak, tempo, key, silence, hum, noise floor, clipping
- [x] R1.6 Export: WAV, FLAC, MP3, OGG, plus AAC/Opus/M4R through a system encoder when present
- [x] R1.7 Playback + recording (cpal), meters, hold-for-original; 20 ms parameter/gain glide (no zipper)
- [x] R1.8 Projects library, .flux package, journal + crash recovery, presets, macros, batch, ringtone, convert
- [x] R1.9 Stems (DSP split), auto-master, magic, voice cleanup, TTS (system voices)
- [x] R1.10 Session API: one command registry used by UI, MCP and tests

### R2 MCP (`crates/flux-mcp`)
- [x] R2.1 stdio MCP server: every API command is a tool
- [x] R2.2 Attach to the running app (local control port + token), headless fallback
- [x] R2.3 `flux-mcp serve` (HTTP API + UI) for E2E tests

### R3 Desktop app (`apps/desktop`)
- [ ] R3.1 UI shell ported from design (kit, viz, routing, themes, fonts, motion)
- [ ] R3.2 All 87 screens wired to real data
- [ ] R3.3 Tauri shell, file dialogs, drag and drop, icons, bundles

### R4 Site (`site/`, deployed by Cloudflare Pages from `dist-static/`)
- [ ] R4.1 Landing improved for the desktop app
- [ ] R4.2 Download page reads the latest GitHub release

### R5 CI/CD
- [ ] R5.1 CI: fmt, clippy, tests, UI E2E
- [ ] R5.2 Release on every push to main: Windows, macOS arm64 + x64, Linux; flux-mcp binaries
- [ ] R5.3 Build caches (rust-cache + sccache + npm) so a push only rebuilds what changed (user request 2026-11-07)

### R6 Verification and docs
- [ ] R6.1 Full local test run, screenshots checked
- [ ] R6.2 README, MCP guide, in-app agent guide, LICENSES

## Human and hardware queue

- Listening checks for every effect (no agent can judge sound).
- Real-device playback and recording on WASAPI, CoreAudio and ALSA (tests use the null device).
- macOS and Linux builds are only exercised in CI; run them on real machines.
- Code signing: Windows (Authenticode) and macOS (Developer ID + notarization) need paid certificates.
- Satoshi font licence review (carried over).

## Blocked

None.

## Decision log

- 2026-11-07: `.gitignore` `Flux-*/` matched `crates/flux-core/` on case-insensitive Windows; anchored to the repo root.
- 2026-11-07: Without bundled models, stems/cleanup/master/magic use DSP and say so; transcript, fillers, dub and clone return `unavailable`. TTS uses the OS speech engine and is labelled synthetic.

- 2026-11-07: User asked for a full rebuild as a native desktop app with MCP and per-push releases. Old code deleted.

## Session log

- 2026-11-07: R1.7-R1.10, R2.1-R2.3: session API (123 methods, one registry for UI/MCP/tests), jobs, localhost control server (token, Host/Origin checks), flux-mcp stdio (headless + attach) and `serve`. 62 Rust tests green; clippy -D warnings clean; cargo deny ok. Auto-master lands within 0.25 LU of target (iterative limiter trim). Glide test: largest sample step 0.0078 on a -24 dB change (was 0.34 before smoothing).

- 2026-11-07: R1.1-R1.6 engine modules (dsp, fx 45 kinds, project, media, render, analysis, export): 38 tests green; render bit-exact; RT vs offline null < 1e-5; BS.1770 within 0.1 LU; all 4 built-in export formats verified by decoding back.
- 2026-11-07: R0.1 done. 460 old files removed; design/ kept; landing/download pages moved to site/.
