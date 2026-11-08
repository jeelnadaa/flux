# ADR 0005: Choosing models for stems, transcripts and text to speech

Date: 2026-11-08. Status: accepted.

## Context

The user asked for the best local models and lighter ones for stems, transcripts and text to speech, a suggestion
based on the computer, and the freedom to pick any of them. Flux already downloads and runs voice cloning models
through a pinned Python runtime (ADR 0003). The licence policy (PLAN.md §4) allows permissive licences only.

## Decision

The same catalog, downloader and runner serve all three jobs. Each model has a `role` (stems, transcript, tts), a
`tier` (light, balanced, best; or extra for a specialist that is offered but never suggested) and its own pip
`install` steps, which run once and are shared between models that need the same packages.

| Job | Light | Balanced | Best | Also |
|---|---|---|---|---|
| Stems | Built-in split (DSP, no download) | Demucs v4 hybrid transformer (MIT) | Demucs fine-tuned (MIT, 4 models in a bag) | Demucs 6 stems (guitar, piano) |
| Transcripts | Whisper Base (MIT) | Whisper Small (MIT) | Whisper Large v3 Turbo (MIT) | |
| Text to speech | System voices (no download), Kokoro 82M (Apache-2.0) | | Chatterbox Turbo's own voice (MIT) | |

Whisper runs through faster-whisper with CTranslate2 (both MIT). Kokoro is the light text to speech model because
it is 82M parameters and fast on a processor.

The suggestion: on an NVIDIA graphics chip, the best model that fits its memory; on a processor, balanced at most
(light for text to speech), and only with 8 or more cores beyond light; on Apple silicon with 16 GB or more,
the best stems model. Intel Macs get the built-in engines (no PyTorch 2.6 build). The chosen engine is saved per job
in the model settings and can be changed any time (`ai.engine.set`, Settings > Smart tools).

### Avoiding copyleft dependencies

Several packages pull in LGPL or GPL code that Flux never calls:

- faster-whisper depends on PyAV, which bundles FFmpeg (LGPL/GPL). Flux sends Whisper 16 kHz float WAV it decoded
  itself, so faster-whisper is installed with `--no-deps` and a stub `av` module stands in.
- Demucs depends on lameenc (LGPL) for MP3 output and torchaudio file loading; Flux reads and writes WAV itself,
  so Demucs is installed with `--no-deps` and its pure-Python dependencies are listed explicitly.
- misaki (Kokoro's text front end) imports num2words (LGPL), espeakng-loader and phonemizer (GPL, espeak-ng).
  Flux supplies MIT stand-ins in `py_shims/`: a num2words for English cardinals, ordinals, years and decimals, and
  phonemizer/espeak modules that raise if called (English goes through misaki's own lexicon; out-of-vocabulary
  words are spelled out).

The stand-ins go last on the module path, so a real package the user installed would win.

## Consequences

- Real runs need the user's yes to download weights and packages; the first real run of each model is in the
  human queue. Until then tests use stub packages that check what Flux hands them.
- Without espeak-ng, Kokoro reads only English well. Other languages wait for a permissive phonemizer.
- Cloud engines (keys from the system keychain or an environment variable, and free ones without a key) are the
  next part; they plug into the same per-job choice.
