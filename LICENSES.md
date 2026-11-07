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

External tools Flux *calls* when present but does not ship: `ffmpeg` (AAC, M4R, Opus export) and the operating
system's speech engine (System.Speech on Windows, `say` on macOS, `espeak-ng` on Linux). They run as separate
processes, so their licences do not apply to Flux's code.

---

## 4. Web & UI Dependencies

Scanned and verified by `license-checker-rseidelsohn`.

| Package / Library | Role | Licence | Status | Notes |
|---|---|---|---|---|
| `typescript` | Build tooling / compiler | Apache-2.0 | Allowed | Microsoft Corporation |
| `esbuild` | Fast web bundler | MIT | Allowed | Evan Wallace |
| `react` / `react-dom` | UI component runtime & DOM renderer | MIT | Allowed | Meta Platforms, Inc. |
| `@types/react` / `@types/react-dom` | React TypeScript definitions | MIT | Allowed | DefinitelyTyped contributors |
| `eslint` / `typescript-eslint` | Code quality & static analysis | MIT / Apache-2.0 | Allowed | Development gate |
| `prettier` | Code formatting | MIT | Allowed | Development gate |
| `license-checker-rseidelsohn` | License compliance auditing | MIT | Allowed | Development gate |
| `onnxruntime-web` | In-browser ML inference runtime (WebGPU / WASM) | MIT | Allowed | Microsoft Corporation |
| `GSAP` *(Landing prototype)* | Kinetic animations & scroll scenes | GreenSock standard licence | **Review Queued** | Terms review queued before production release |
| `Lenis` *(Landing prototype)* | Smooth scrolling | MIT | Allowed | Studio Freight |

---

## 5. Machine Learning Models & Weights Policy

Model code and trained weights have distinct licenses and must be audited separately.

| Model Pipeline | Intended Implementation | Weight Licence Policy | Status |
|---|---|---|---|
| Stem Separation | HTDemucs / BS-RoFormer | Audit weights licence per model card; non-commercial weights prohibited | Planned P6 |
| Neural Denoise | DeepFilterNet / RNNoise | Permissive (MIT / Apache / CC-BY) only | Planned P6 |
| Speech Recognition / VAD | Whisper / Silero VAD | MIT / Permissive weights only | Planned P6 |
| Text-to-Speech | Permissive models | Coqui XTTS prohibited (non-commercial); permissive weights only | Planned P7 |

---

## 6. Audio Codecs, Patents & Containers

- **MP3:** Patents have expired worldwide; decoding via Symphonia and encoding via LAME/platform encoder.
- **AAC:** Use platform encoders (`WebCodecs` in browser, Media Foundation in desktop) to avoid patent indemnification issues with custom FDK-AAC builds.
- **FLAC, Vorbis, Opus:** Unencumbered open formats.
- **WAV, AIFF:** Standard uncompressed formats.

---

## 7. Verification Commands

To run all automated license scans across the monorepo:

```bash
# Rust workspace dependency scan
cargo deny check licenses

# JavaScript/TypeScript dependency scan
npm run license:check

# Run full local CI suite including license scan
just ci
```
