# Licences & Legal Audit

Every third-party dependency, font, model, asset, and codec specification used by Flux is recorded here.
This document is continuously maintained and enforced by automated scans in CI per [PLAN.md §4](PLAN.md#4-licensing--legal-audit).

---

## 1. Licensing Policy & Automated Enforcement

Flux requires permissive, commercially viable open-source licenses for all core distribution targets:
- **Permitted by Default:** MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, 0BSD, BlueOak-1.0.0, CC0-1.0, Unlicense, SIL OFL 1.1.
- **Restricted / Requires Isolation:** LGPL (e.g. dynamically linked FFmpeg/LAME with written offer and replaceable binary).
- **Prohibited:** AGPL (e.g. Essentia), GPL (e.g. Rubber Band without commercial license), non-commercial licenses (CC-BY-NC, non-commercial model weights).

### Automated CI Scanners
- **Rust Dependencies:** `cargo deny check licenses` validates all workspace crates against allowed SPDX identifiers in [deny.toml](deny.toml).
- **JavaScript / Web Dependencies:** `npm run license:check` runs `license-checker-rseidelsohn` enforcing permitted licenses across `node_modules`.

---

## 2. Interface Fonts

Bundled in `design/fonts/` for complete offline functionality without third-party network requests.

| Font | Role | Licence | Status | Notes |
|---|---|---|---|---|
| **Bricolage Grotesque** | Default UI font & brand logo | SIL OFL 1.1 | Allowed | In `design/fonts/licenses/bricolage-grotesque-OFL.txt` |
| **Archivo** | UI font option | SIL OFL 1.1 | Allowed | In `design/fonts/licenses/archivo-OFL.txt` |
| **Schibsted Grotesk** | UI font option | SIL OFL 1.1 | Allowed | In `design/fonts/licenses/schibsted-grotesk-OFL.txt` |
| **Geist** | UI font option | SIL OFL 1.1 | Allowed | In `design/fonts/licenses/geist-OFL.txt` |
| **Geist Mono** | Numeric readouts, meters, timecode | SIL OFL 1.1 | Allowed | In `design/fonts/licenses/geist-mono-OFL.txt` |
| **Anybody** | UI font option | SIL OFL 1.1 | Allowed | In `design/fonts/licenses/anybody-OFL.txt` |
| **Satoshi** | UI font option | ITF Free Font License | **Review Queued** | Human verification queued in PROGRESS.md before release |

---

## 3. Rust Engine Crates

Scanned and verified by `cargo deny check licenses` ([deny.toml](deny.toml)). Direct dependencies only; the
scanner covers the full tree.

| Crate | Role | Licence | Status |
|---|---|---|---|
| `flux-core`, `flux-mcp`, `flux-desktop` | First-party engine, MCP server, desktop shell | MIT OR Apache-2.0 | First-party |
| `symphonia` 0.6 | Decoding WAV, AIFF, FLAC, MP3, OGG, M4A/AAC, ALAC | MPL-2.0 | Allowed (file-level copyleft; used unmodified) |
| `hound` 3.5 | WAV writing and test fixtures | Apache-2.0 | Allowed |
| `flacenc` 0.5 | FLAC encoding | Apache-2.0 | Allowed |
| `rusty_mp3` 1.0 | MP3 encoding (pure Rust) | Apache-2.0 | Allowed |
| `vorbis_rs` 0.5 (+ `aotuv_lancer_vorbis_sys`, `ogg_next_sys`) | Ogg Vorbis encoding | BSD-3-Clause | Allowed |
| `rustfft` 6.4 | FFT for spectra, convolution, pitch, stems | MIT OR Apache-2.0 | Allowed |
| `rtrb` 0.3 | Lock-free queues to the audio thread | MIT OR Apache-2.0 | Allowed |
| `cpal` 0.18 | Audio devices (WASAPI, CoreAudio, ALSA) | Apache-2.0 | Allowed |
| `tiny_http` 0.12 | Localhost control server for MCP attach | MIT OR Apache-2.0 | Allowed |
| `zip` 9 | `.flux` project packages | MIT | Allowed |
| `dirs` 6 | Platform data and music folders | MIT OR Apache-2.0 | Allowed |
| `serde`, `serde_json`, `anyhow` | Serialization and errors | MIT OR Apache-2.0 | Allowed |
| `tauri` 2, `tauri-build` 2 | Desktop shell: native window and webview | MIT OR Apache-2.0 | Allowed |
| `tauri-plugin-dialog`, `-opener`, `-updater`, `-process` 2 | File dialogs, reveal and links, signed auto-update, restart | MIT OR Apache-2.0 | Allowed |
| `reqwest` 0.13 (rustls, no default features) | Voice model downloads over HTTPS, with resume | MIT OR Apache-2.0 | Allowed |
| `pulseaudio` 0.3 (Linux, through cpal's PulseAudio host) | Lists and opens every sink and monitor for Live | MIT | Allowed |
| `keyring` 4.2 (with its Windows, Apple and zbus Secret Service stores) | Keeps online-service keys in the system keychain | MIT OR Apache-2.0 | Allowed |
| `rustls` 0.23 (`ring` provider) | TLS for model downloads | Apache-2.0 OR ISC OR MIT (`ring`: Apache-2.0 AND ISC) | Allowed |
| `sha2` 0.10 | Checking every downloaded file's SHA-256 | MIT OR Apache-2.0 | Allowed |
| `fs4` 0.13 | Free disk space before a download | MIT OR Apache-2.0 | Allowed |
| `flate2` 1, `tar` 0.4 | Unpacking the portable Python archive | MIT OR Apache-2.0 | Allowed |

External tools Flux *calls* when present but does not ship: `ffmpeg` (AAC, M4R, Opus export) and the operating
system's speech engine (System.Speech on Windows, `say` on macOS, `espeak-ng` on Linux). They run as separate
processes, so their licences do not apply to Flux's code.

---

## 4. Web & UI Dependencies

| Package / Library | Role | Licence | Status | Notes |
|---|---|---|---|---|
| `@tauri-apps/cli` | Builds and bundles the desktop app (dev only) | MIT OR Apache-2.0 | Allowed | Not shipped |
| `GSAP` 3.12 *(website only, from cdnjs)* | Scroll scenes on the landing and download pages | GreenSock standard licence | **Review Queued** | Not shipped in the app |
| `Lenis` 1.1 *(website only, from jsDelivr)* | Smooth scrolling on the website | MIT | Allowed | Not shipped in the app |

The desktop UI (`apps/desktop/ui`) has no third-party JavaScript.

---

## 5. Machine Learning Models & Weights Policy

Model code and trained weights have distinct licenses and must be audited separately.

| Model | Use in Flux | Code and weights licence | Status |
|---|---|---|---|
| Chatterbox Turbo (Resemble AI), `ResembleAI/chatterbox-turbo` | Default speech model: performs [laugh], [sigh] and other script sounds; watermark | MIT | Downloadable in the app (2.99 GB), pinned to a commit with SHA-256 per file |
| Chatterbox (Resemble AI), `ResembleAI/chatterbox` | English speech with an emotion control | MIT | Downloadable (3.19 GB), pinned |
| Chatterbox Multilingual (Resemble AI), same repository | Speech in 23 languages | MIT | Downloadable (3.21 GB), pinned; shares files with the converter |
| Chatterbox voice converter (Resemble AI), same repository | Turns the user's own recording into the voice | MIT | Downloadable (1.06 GB), pinned |
| Demucs v4 `htdemucs`, `htdemucs_ft`, `htdemucs_6s` (Meta Research) | Stems (balanced, best, 6 stems) | MIT (code and weights) | Downloadable (84 MB, 336 MB, 55 MB), pinned to the release files with SHA-256 |
| Whisper Base, Small, Large v3 Turbo (OpenAI); CTranslate2 conversions by Systran (Base, Small) and Mobius Labs (Turbo) | Transcripts | MIT | Downloadable (148 MB, 486 MB, 1.62 GB), pinned to a commit with SHA-256 per file |
| Kokoro 82M v1.0 (hexgrad) | Light text to speech, 6 voices | Apache-2.0 | Downloadable (about 330 MB), pinned |
| OpenVoice V2 (MyShell) | Was the light fallback | MIT | Dropped: its MeloTTS front end needs a second runtime; Chatterbox runs on a processor instead (ADR 0003) |
| Dia (Nari Labs) | Not used | Its README limits use to research and education | Rejected |
| Coqui XTTS | Not used | Coqui Public Model Licence (non-commercial) | Rejected |

**Online engines (ADR 0006):** off unless the person turns one on. They are services, not code or weights Flux
ships: OpenAI, Groq, Deepgram, ElevenLabs and Replicate under the person's own account and terms, and three public
Hugging Face Spaces (Whisper Large v3, MIT weights; Kokoro, Apache-2.0 weights; a vocal split) under each Space's
terms.

**Voice engine (downloaded on the user's computer when they first download a model; not shipped in the installer):**

| Component | Role | Licence | Status |
|---|---|---|---|
| CPython 3.11 (python-build-standalone builds) | Runs the models | PSF-2.0 (permissive); build scripts MPL-2.0 / BSD | Allowed; pinned archive with SHA-256 |
| PyTorch 2.6.0, torchaudio 2.6.0 | Model runtime (CUDA 12.4, CPU or Apple builds) | BSD-3-Clause / BSD-2-Clause | Allowed; pinned |
| `chatterbox-tts` 0.1.7 | Model code | MIT | Allowed; pinned |
| `resemble-perth` 1.0.1 | Chatterbox's inaudible watermark | MIT | Allowed (pulled in by chatterbox-tts) |
| `setuptools` 80.9.0 | Pinned so `pkg_resources` exists for the watermarker (removed in setuptools 81+) | MIT | Allowed; pinned |
| `faster-whisper` 1.2.1, `ctranslate2` 4.8.5, `onnxruntime` 1.31.1 | Runs Whisper | MIT / MIT / MIT | Allowed; faster-whisper installed with `--no-deps` so PyAV (FFmpeg, LGPL/GPL) is not installed |
| `demucs` 4.0.1, `dora-search` 0.1.13, `einops` 0.8.2, `julius` 0.2.8, `openunmix` 1.3.0 | Runs Demucs | MIT | Allowed; demucs installed with `--no-deps` so lameenc (LGPL) is not installed |
| `kokoro` 0.9.4, `misaki` 0.9.4, `loguru` 0.7.3, `addict` 2.4.0, `spacy` 3.8.18, `en_core_web_sm` 3.8.0 | Runs Kokoro and reads English text | Apache-2.0 / Apache-2.0 / MIT / MIT / MIT / MIT | Allowed; kokoro and misaki installed with `--no-deps` so num2words (LGPL), phonemizer and espeak-ng (GPL) are not installed |
| Flux stand-ins in `crates/flux-core/src/py_shims` (`av`, `num2words`, `espeakng_loader`, `phonemizer`) | Replace the copyleft modules above; written by Flux | MIT (Flux) | Allowed (ADR 0005) |
| Their pip dependencies (transformers, diffusers, librosa, numpy, safetensors, the Perth watermarker and others) | Installed by pip at their versions | Mostly Apache-2.0 / BSD / MIT / ISC | **Review queued**: a full audit of the resolved set is in the human queue |

Clean-up and mastering use Flux's own DSP. Stems, transcripts and text to speech can also use the downloadable
models above, chosen by the user; no model weights ship with Flux.
Weights are never committed to git. Check each model card again before downloading.

---

## 6. Audio Codecs, Patents & Containers

- **MP3:** Patents have expired worldwide; decoded by Symphonia and encoded by the `rusty_mp3` crate.
- **AAC, Opus, M4R:** exported only through an `ffmpeg` already on the computer; Flux ships no AAC encoder.
- **FLAC, Vorbis, Opus:** Unencumbered open formats.
- **WAV, AIFF:** Standard uncompressed formats.

---

## 7. Verification Commands

```bash
cargo deny check licenses
```

The website's two scripts load from cdnjs and jsDelivr and are listed in section 4. CI runs `cargo deny check` on
every push.
