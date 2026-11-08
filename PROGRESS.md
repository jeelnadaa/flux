# PROGRESS.md

Rebuild started 2026-11-07 at the user's request. The old web/WASM code was deleted; the old plan and log are in
[docs/archive/](docs/archive/). The design prototype in `design/` is unchanged and remains the specification.

## Current focus

Rebuild v2: native desktop app (Windows, macOS Apple + Intel, Linux), Flux MCP server, landing and download site,
CI/CD that publishes a GitHub release on every push to `main`. Added 2026-11-07: in-app auto-update, Live
(computer sound and microphone through modular live presets, app + MCP), tooltips on every control, voice cloning
research, launch video.

## Checklist (v2 rebuild)

Legend: `[x]` done and tested · `[ ]` not started · `[~]` in progress · `[!]` blocked

### R0 Repo
- [x] R0.1 Delete old code and brag output; archive old PLAN/PROGRESS
- [x] R0.2 New workspace, PLAN.md (v2), ADRs 0001-0004 (AGENTS.md still describes v1's web-only rules; left for the user to change)

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
- [x] R3.1 UI shell ported from design (kit, viz, routing, themes, fonts, motion)
- [x] R3.2 All screens wired to real data (90 render against the real engine without errors)
- [x] R3.3 Tauri shell, file dialogs, drag and drop, icons, bundles (signed NSIS + MSI built locally)
- [x] R3.4 In-app auto-update: background check (5 s after launch, then every 6 h), signed download, install on restart or quit, toggle in Settings > About (user request 2026-11-07)

### R7 Live (user request 2026-11-07)
- [x] R7.1 Live engine: capture computer sound (WASAPI loopback; CoreAudio tap on macOS 14.6+; Monitor inputs on Linux) or a microphone, process, play to any output; drift-correcting resampler; feedback guard; lock-free updates
- [x] R7.2 Modular live presets: parallel lanes + post chain of any of 39 live effects, 10 modulator shapes (LFOs, random, steps, envelope follower) with tempo sync, macros with ranges and curves, safety limiter
- [x] R7.3 41 built-in live presets (21 for computer sound, 20 for a microphone); create, copy, edit, randomize, export/import (.fluxlive), hostile-input checks
- [x] R7.4 31 `live.*` methods = 31 MCP tools, including `live.render` so agents can audition presets offline
- [x] R7.5 Live page in the app (composed from the kit; added to design/LIVE.md), with tooltips on every control
- [x] R7.6 Tooltips: every effect and parameter (`fx.catalog`, `live.kinds`), enforced by a test
- [x] R7.7 Tooltips shown on every control in the UI (tips.js; engine tips for effects and parameters)
- [x] R7.8 Voice cloning, part 1: model picked for the computer (Chatterbox Turbo MIT first: performs [laugh], [sigh] and other script sounds; Chatterbox MIT for converting your own recording; OpenVoice V2 MIT lightest), hardware probe, consent-gated voice profiles, script parser for sounds and hesitations, clone screen, 8 MCP tools
- [x] R7.11 Smart tool models, part 1: choose the engine for stems (built-in split, Demucs v4, Demucs fine-tuned, Demucs 6 stems), transcripts (Whisper Base, Small, Large v3 Turbo via faster-whisper) and text to speech (system voices, Kokoro, Chatterbox Turbo's own voice); suggestion per computer with a reason; per-model pip packages installed once and shared; LGPL/GPL dependencies avoided with --no-deps and MIT stand-ins (ADR 0005); transcript.make/get/fillers with sample positions; 5 new MCP tools (173) (user request 2026-11-08)
- [x] R7.12 Smart tool models, part 2: online engines. Free, no key (public Hugging Face Spaces): Whisper Large v3 text, Kokoro speech, vocal/instrumental split. With a key: Groq and OpenAI Whisper, Deepgram Nova-3, OpenAI speech, ElevenLabs, Demucs on Replicate. Opt-in per engine (`allow_cloud`), keys from env vars or the system keychain, never shown or saved, HTTPS or this computer only, OpenAI-compatible address override (ADR 0006); 3 new MCP tools (176)
- [x] R7.13 Smart tool models, part 3: Settings > Smart tools (every engine by role with tier, suggestion, Download/Use/Remove, Turn on with what is sent, keys added and forgotten, voice and language), engine tiles on Split into stems, an engine menu on Text to speech, and the Transcript editor and Filler review screens ported from the prototype (design/LIVE.md)
- [x] R7.14 Flux speaker and Flux microphone: devices other apps can pick, so only the processed sound is heard. Linux: Flux makes them (PipeWire or PulseAudio, no root) and makes the speaker the system output while Live runs. Windows and macOS: an installed virtual cable is found and used; Flux tells people how to use it (ADR 0007); 3 new MCP tools (179) (user request 2026-11-08)
- [x] R7.15 Virtual cable guide: Live says when no virtual cable is installed (on the panel and both route cards) and why it matters; a step-by-step guide for Windows, macOS and Linux with the maker's download page, copyable commands and Check again; `live.virtual` reports status and where to get one (user request 2026-11-08)
- [x] R7.10 Cloning scripts: hesitation placeholders ([hmm], [um], [uh], [mm-hmm] and more, said in the voice) and exact pauses ([pause], [long pause], [pause 2s]); the script splits into spoken pieces and silences; chips for all three kinds (user request 2026-11-08)
- [x] R7.9 Voice cloning, part 2: in-app model downloader and runner. 4 Chatterbox models (MIT) pinned with SHA-256 per file, plus a voice engine (portable Python 3.11, PyTorch 2.6, chatterbox-tts 0.1.7) installed on first download; confirm dialog with size, licence and folder; progress, cancel, resume (Range), checksum, free-space check, shared files, delete; choose any model for speech or conversion, device, language, seed and each model's settings; add your own model folder; clone.speak and clone.convert run as jobs, label the WAV synthetic and add it to the project. 6 new MCP tools (168). Tested against a local mirror and stub runtime; the real download and listening are queued

### R4 Site (`site/`, deployed by Cloudflare Pages from `dist-static/`)
- [x] R4.1 Landing improved for the desktop app (no web app; Live and agents sections; OS-aware download button)
- [x] R4.2 Download page reads the latest GitHub release (release.js; `npm run build:static` writes dist-static with CSP `_headers` and `_redirects`)

### R5 CI/CD
- [x] R5.1 CI: fmt, typos, clippy, cargo deny, Rust tests on Linux/Windows/macOS, check:js, site tests, UI E2E with screenshots
- [x] R5.2 Release on every push to main that changes the app: draft v2.0.<run>, Windows (NSIS, MSI), macOS arm64 + x64 (dmg), Linux (AppImage, deb, rpm), flux-mcp per target, latest.json when signed; published only when every build passes
- [x] R5.3 Build caches (rust-cache + sccache + npm) so a push only rebuilds what changed (user request 2026-11-07)

### R6 Verification and docs
- [x] R6.1 Full local test run, screenshots checked
- [x] R6.2 README, MCP guide (Live and cloning sections), in-app agent guide, LICENSES (stale v1 rows removed; cloning models listed)

## Human and hardware queue

- First CI and Release runs on GitHub (needs the user to push). Add secrets TAURI_SIGNING_PRIVATE_KEY and TAURI_SIGNING_PRIVATE_KEY_PASSWORD (empty) first, or releases ship without self-update files. Installers are not code-signed for Windows SmartScreen or macOS Gatekeeper (needs paid certificates).

- Voice cloning with the real models (R7.9): downloaded and run on Windows with an RTX 3070 Ti (CUDA) and on its processor, 2026-11-08. Still to do: Apple silicon and Linux, the Chatterbox, Multilingual and voice converter models, and listening checks of cloned speech, laughs, sighs and hesitations (agents can't judge sound).
- Smart tool models (R7.11): first real download and run of Demucs, Whisper and Kokoro on this machine (needs the user's yes to download), then Apple silicon and Linux; listening checks of stems and Kokoro speech; word timing checked against a real recording.
- Online engines (R7.12): first real call to each service (needs the user's keys and their yes to send audio): Groq, OpenAI, Deepgram, ElevenLabs, Replicate, and the 3 free Spaces; check word timing against a real recording; macOS Keychain and Linux Secret Service storage on real machines.
- Licence audit of the voice engine's resolved pip dependency set (transformers, diffusers, librosa, the Perth watermarker and the rest).

- Listen to all 41 built-in live presets on headphones and speakers (agents can't judge sound).
- Live on macOS (14.6+ system audio permission) and Linux (PipeWire/Pulse monitor inputs): only built in CI so far.

- Listening checks for every effect (no agent can judge sound).
- Real-device playback and recording on WASAPI, CoreAudio and ALSA (tests use the null device).
- macOS and Linux builds are only exercised in CI; run them on real machines.
- Code signing: Windows (Authenticode) and macOS (Developer ID + notarization) need paid certificates.
- Satoshi font licence review (carried over).

- Flux speaker and microphone (R7.14): real run on Linux with PipeWire and with PulseAudio (never compiled or run on Linux in this session; cpal's PulseAudio host is new to Flux); real virtual cable on Windows and macOS; listen for double sound and dropouts.

## Blocked

None.

## Decision log

- 2026-11-08: The user decided: no audio driver of Flux's own. Use the existing virtual cables and tell people how to use them.
- 2026-11-08: Flux speaker and microphone (ADR 0007, replaces the 2026-11-07 note that Flux can't be an output): Linux makes its own devices with pactl; Windows and macOS use an installed virtual cable, and Flux only tells people how to use it. Cable software is not bundled.
- 2026-11-07: Voice cloning models limited to MIT: Chatterbox Turbo (default; inline sound tags), Chatterbox (23 languages, voice conversion), OpenVoice V2 (lightest). Dia skipped: its README limits it to research and educational use. Weights are never committed and are downloaded only with the user's permission.

- 2026-11-07: Live can't make Flux appear as an output device for other apps: that needs a signed kernel audio driver. Instead Flux captures an output device (loopback) and plays to another. To route a processed mic into call apps the user installs a virtual cable; Flux says so in the app and over MCP.
- 2026-11-07: Live presets exclude speed, convolver, noise reduction and tape stop (they need a file or move time). "Slowed + reverb" therefore lowers pitch and says so.
- 2026-11-07: Updater signing key lives outside the repo (`%USERPROFILE%\.flux-signing`); CI reads `TAURI_SIGNING_PRIVATE_KEY`. Builds are not code-signed (queued).

- 2026-11-07: `.gitignore` `Flux-*/` matched `crates/flux-core/` on case-insensitive Windows; anchored to the repo root.
- 2026-11-07: Without bundled models, stems/cleanup/master/magic use DSP and say so; transcript, fillers, dub and clone return `unavailable`. TTS uses the OS speech engine and is labelled synthetic.

- 2026-11-07: User asked for a full rebuild as a native desktop app with MCP and per-push releases. Old code deleted.

## Session log

- 2026-11-08: R7.9 test fix: test_clone_models timed out on Linux CI waiting for Cancel because the stub download could finish between two polls of the page. Reproduced on Windows by removing the mirror's delay (same error, same line). The stub install now takes a set time; passes with the delay removed too. Prints the page and engine state if it ever fails.
- 2026-11-08: R5.3 fix: a 504 downloading NSIS's plugin failed the Windows release. The bundler's tools are now cached between runs and the build retries 3 times. Workflow parsed and checked by script; not run on GitHub (needs a push).
- 2026-11-08: R5.3 fix: a 504 downloading sccache failed every CI and release job, because RUSTC_WRAPPER was set job-wide and the action's post step read stats from the missing tool. sccache is now optional per job. Workflow files parsed and checked by script (3 places); not run on GitHub (needs a push).
- 2026-11-08: R7.15 virtual cable guide. Install steps checked against the makers' pages (VB-CABLE, BlackHole). test_flux_devices.py now 5 checks: not installed shown on panel and both cards; guide steps for each OS at 1440, 820 and 390 with no overflow; maker's link; Check again says still not found, then finds a cable added while it is open and Live switches to it. The guide found a missing kit import. Detection now also accepts cables named the same on both sides (VB-Cable on macOS) and two BlackHole sizes. The real install was not run: it adds a driver to this computer and needs a restart.

- 2026-11-08: R7.14 Flux speaker and microphone. New tests/flux_devices.rs (stand-in pactl that keeps server state and refuses bad quoting, unknown masters and removing a sink in use; fake device lists): 3 modules made once, not twice; speaker records flux_speaker.monitor and plays on the real sink; microphone plays into the feed; a failed start leaves the system output alone; Stop restores it; Remove unloads in order; a cable is found and the real output chosen even when the cable is the default; one cable can't run both; a Flux device as the listening output is refused. 3 unit tests (detection for Linux, one and two cables, macOS, nothing). New tests/e2e/test_flux_devices.py (4 checks: no device, one cable, Linux add and remove, 1440/820/390). Both suites fail when broken on purpose (cable offered as output; remove order reversed). Not run on real PipeWire, PulseAudio or a real cable: no Linux or cable here (queued). The Live suite caught a 30 px overflow in the new panel at 390 px (fixed; the new test now measures the scrolling area too). The Escape-hides-tooltip check that flaked in R7.12 was a real bug: a redraw put focus back and re-showed a dismissed tooltip (fixed separately; 6 of 6 runs pass).
- 2026-11-08: R7.13 Smart tools UI. New tests/e2e/test_smart_tools.py (10 checks, local mirror + stub models, no network): engines listed by role and tier with the built-in split in use out of the box; Download asks first and Cancel fetches nothing; free online engine dialog says what is sent and asks no key; keyed engine has a password field, refuses an empty key, stores it in the keychain (never in any file under the data folder) and Forget removes it (cmdkey shows nothing left); transcript made with the chosen engine shows So um I I think uh it works. with 3 fillers marked; keyboard selects a word and Delete cuts it; filler review filters and removes 3; stem tiles and the speech engine menu follow the installed engines; no sideways scroll at 1440, 820 and 390 or in 13 themes x 6 fonts; every control named. The test found 2 bugs: transcript messages leaked onto Smart tools (shared message slot), and after a cut the out-of-date filler list could be removed again at the wrong positions (Remove and Delete now wait for an updated transcript; the check fails with the old code). E2E 20 + 9 + 7 + 10 green.
- 2026-11-08: R7.12 online engines. New tests/cloud_engines.rs (3 tests, a local mock of every service): nothing sent until allowed (consent_required before any request, 0 hits), keyed engine without a key says which variable and where to get one, a key in Windows Credential Manager is used as `Bearer <key>` and never appears in answers or any file under the data folder, deleted on request; env keys win; each service asked the right way (OpenAI multipart with model, verbose_json and 16 kHz mono WAV; Deepgram `Token` header with the WAV as body; ElevenLabs `xi-api-key` and pcm_24000; Replicate upload, prediction, 2 polls, stems fetched without the key; Gradio upload, call, event stream, file); words at exact samples (um at 12000, repeat at 24000; Deepgram uh at 24000); speech 0.5 / 0.25 / 0.75 s, all labelled synthetic; Replicate stems 6.02 dB apart as sent; 401, 429, a busy free demo and no connection give clear errors without the text or key. The test found 1 bug: online speech ignored the saved voice. 4 new unit tests (chunk offsets, OpenAI and Deepgram answers, Gradio streams, HTTPS only). cargo deny licences ok with keyring. 106 Rust tests; E2E 20 + 9 + 7 (test_live's Escape-hides-tooltip check failed once in 3 runs, timing; passed twice after).

- 2026-11-08: R7.11 smart tool models, part 1. 7 models pinned with SHA-256 per file (Whisper x3 (CTranslate2 conversions by Systran and Mobius Labs), Demucs x3 from the authors' release URLs, Kokoro v1.0 with 6 voices). New tests/smart_engines.rs (2 tests, mock server + stub faster_whisper/demucs/kokoro that check the audio they get): engines listed with one choice each, the 6-stem model never suggested, choice validated and saved; not_installed points to Smart tools; download asks first, installs torch then the model's 2 packages (--no-deps), the definition comes from its own URL; 4 stems in order at their share of the input (vocals -6.02 dB, drums -12.04, bass -18.06, within 0.3 LU) and 2-stem instrumental; the 6-stem model reuses the packages (0 new pip calls) and gives 6 stems; transcript words at 0, 12000, ... 84000 samples (48 kHz), fillers um@12000, repeat I@24000, uh@60000, stale after an edit; Kokoro speaks in the chosen voice, falls back to its first voice, WAV labelled synthetic; work folder left empty. The test found 3 bugs, all fixed: tool.stems and ai.model.download refused the engine and device_kind parameters, runner failures said "Speaking failed" for every job, and the runner could not read Flux's own float WAVs (WAVE_FORMAT_EXTENSIBLE). The full run found 2 more: app.info deadlocked (it held the API lock while listing engines, which locks again; the server test hung), and with nothing chosen Stems defaulted to a suggested model that wasn't downloaded, so it failed out of the box (the default is now the suggestion once downloaded, else the built-in engine). System voices are counted once per run instead of starting PowerShell 3 times per app.info. 99 Rust tests; E2E 20 + 9 + 7 green. 3 new unit tests (suggestions for 6 computers, filler positions, sentence split).

- 2026-11-08: Live preset editor is a dialog. The page shows the preset and its actions; Edit opens the editor over the page, changes play as they are made, Save keeps them, and Discard, the close button or Escape ask first ("Discard changes?", Escape there keeps editing) and then restore the preset as it was when opened (for a built-in, the copy Flux made is deleted and the route goes back to the built-in). Footer stays in view while scrolling. test_live.py 20/20 (new: editor controls named and tipped, Save, Escape asks, Keep editing, Discard restores, unchanged closes without asking, built-in copy deleted, fits 390 px).
- 2026-11-08: Live presets play quieter by default. Measured every built-in on the demo music or voice (BS.1770): they ranged from 16.8 LU quieter to 18.4 LU louder than the input (8D figure eight +3.1, Megaphone +18.4). Each built-in now has a measured output level so it plays 3 LU below the input; a test renders all 41 and asserts -4..-2 LU. Your own presets keep their levels.
- 2026-11-08: First run shows once: it is marked seen when it opens, so closing Flux halfway no longer brings it back. Settings > Audio has a Sound profile section (current profile and head width) with Set up again, which runs the headphone check and head shape as 2 steps and returns to where it was opened from (Settings, or the 3D screens). New E2E tests/e2e/test_first_run.py: 7 checks, fails without the change (first run not marked seen); fits 1440/820/390.
- 2026-11-08: Fix: first-run sound checks. The headphone and sound-profile steps failed with "no_project" because test tones needed an open project; the engine now starts without one (48 kHz) and stops after. The chosen sound profile did nothing; it now sets 3D depth and room (A/D 65, B 80, C 100+8 room, E/F 90+15 room), head width sets the far-ear delay (impulse at 90 degrees: 13 samples at 12 cm, 24 at 22 cm), and picking a profile plays the moving tone. 1 new API test, 2 new unit tests; checked in the app with no project open.
- 2026-11-08: R7.9 real run (user approved the download): voice engine (Python 3.11.17, torch 2.6.0+cu124, chatterbox 0.1.7) and Chatterbox Turbo (2.99 GB, 9 files, all SHA-256 checked) downloaded in the app in about 9 min. Found and fixed 2 bugs the stubs could not: setuptools 84 has no pkg_resources so the watermarker failed to load (pinned setuptools 80.9.0; the install check now tests the watermarker), and top_k reached the model as 1000.0 (the runner now casts by the model's int defaults). Speak on CUDA: 8.68 s of speech in 42 s including load, peak -7.6 dBFS, 0 clipped samples, 1.38 s silence at the 1 s pause, synthetic label in the WAV; on the processor: 3.04 s in 53 s. Also: install progress now moves through pip steps (was stuck at 10.5% for 7 min), and FD.lvl moved to flux-api.js (viz.js could call it before app.js loaded; seen once in E2E). Stub tests reproduce both model bugs and fail without the fixes. 90 Rust tests, E2E 16/16 and 9/9.
- 2026-11-08: R7.9: model downloader and runner. 2 new Rust tests against a mock server and stub Python packages (voice_models.rs): confirm before download, resume after a cut connection (2nd request is a Range) and after cancel (resumed from the .part), bad checksum thrown away, shared files kept on delete, pip asked for pinned packages with the CPU index, runner spoke [say, 1.5 s silence, say] with settings applied and the voice given once, WAV labelled synthetic, runner errors carry no script text; the assertions failed before fixes (kept_shared counted 3 not 1). New E2E tests/e2e/test_clone_models.py: 9 checks (dialog, progress, install, device/slider/seed saved, spoke onto the project, 50 controls named and tipped, 1440/820/390 fit, delete). test_live.py 16/16. Totals: 86 Rust tests.
- 2026-11-08: R0.2, R6.1-R6.2: PLAN.md written for v2 (Live and voice cloning specs), 4 ADRs, MCP guide covers live_* and clone_*, LICENSES cleaned. Full run: 85 Rust tests, 16 E2E checks, 3 site tests, check-js 16 files, clippy, typos, cargo deny all green.

- 2026-11-08: R5.1-R5.3: CI and Release workflows rewritten for the desktop app (old ones built the removed web app). Checked locally: YAML parses, version stamping dry run (2.0.77 into Cargo.toml and tauri.conf.json, updater files off without the key), typos clean (6 identifier words allowed), cargo deny ok. Not yet run on GitHub.

- 2026-11-08: R7.10: hesitation and pause placeholders. New unit test fails when the hesitation comma rule is removed; API test checks segments [say, silence 1500, say]; E2E inserts [hmm] from a chip and sees 1 sound, 2 hesitations, 1.5 s of pauses. 85 Rust tests, 16 E2E checks green; no overflow at 1440/390.

- 2026-11-07: R4.1-R4.2: desktop-only site. tests/site: 3 green (installer classification for 7 kinds, OS detection, no /app links, CSP allows only api.github.com). build:static fails on /app links or missing files. No horizontal overflow at 390/820/1440 on both pages (headless Chromium).

- 2026-11-07: R7.8: voice cloning part 1. Model picker tested for 9 hardware cases; on this machine it detects an RTX 3070 Ti Laptop GPU (8 GB) and picks Chatterbox Turbo on CUDA, with Chatterbox for voice conversion. Consent gate refuses missing consent, missing statement recording, unknown owner, too-short and non-audio clips. Script parser: [laugh] (sighs) *giggles* {clear throat} become sounds; hmm and um stay words; unknown [tags] refused. E2E: a voice made through the UI with real file picks, script sounds shown, speak reports why it can't yet. 84 Rust tests and 16 E2E checks green. Two live timing tests now wait for levels instead of a fixed sleep (they failed under full parallel load; same tolerances).

- 2026-11-07: R7.7: tooltips on every control. tests/e2e/test_live.py now 15 checks; all 2784 controls on all 92 screens have a tooltip (failed with 3 bare before the fix). Effect and parameter tips come from the engine; actions, screens and settings from apps/desktop/ui/tips.js.

- 2026-11-07: R7.5: Live page (live.js) + tooltip component. tests/e2e/test_live.py (Playwright, real engine): 14 checks green - both routes run on test sources (out -10.2 / -18.1 dB RMS, 0 dropouts), bypass, built-in edit makes a copy and the live route follows, modules/steps modulator/macro built through the UI, preview render, export/import identical, 156 controls all named and tooltipped, no overflow at 1440/820/390 and in all 13 themes x 6 fonts, all 92 screens render without errors. Fails when the slider binding is removed.

- 2026-11-07: R7.1-R7.4, R7.6: live engine + presets + 31 MCP tools + tooltips. 82 Rust tests green (14 new live unit tests, 1 end-to-end API test, tooltip coverage test); tests fail when modulation or in-place updates are broken on purpose. Real hardware on this Windows machine: WASAPI loopback of the headphones read -33.0 dB RMS for a -30 dBFS sine (expected -33.0), 0 dropouts; microphone route ran; a route played into the Bluetooth headphones and switched presets live; capture-and-play on one device refused as feedback.
- 2026-11-07: R3.3-R3.4: Tauri shell; signed NSIS (8.2 MB) and MSI (11.4 MB) built; release app launched and flux-mcp attached (headless=false).

- 2026-11-07: R1.7-R1.10, R2.1-R2.3: session API (123 methods, one registry for UI/MCP/tests), jobs, localhost control server (token, Host/Origin checks), flux-mcp stdio (headless + attach) and `serve`. 62 Rust tests green; clippy -D warnings clean; cargo deny ok. Auto-master lands within 0.25 LU of target (iterative limiter trim). Glide test: largest sample step 0.0078 on a -24 dB change (was 0.34 before smoothing).

- 2026-11-07: R1.1-R1.6 engine modules (dsp, fx 45 kinds, project, media, render, analysis, export): 38 tests green; render bit-exact; RT vs offline null < 1e-5; BS.1770 within 0.1 LU; all 4 built-in export formats verified by decoding back.
- 2026-11-07: R0.1 done. 460 old files removed; design/ kept; landing/download pages moved to site/.
