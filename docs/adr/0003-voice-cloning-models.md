# ADR 0003: Voice cloning uses local MIT models, downloaded in the app and chosen by the person

Date: 2026-11-07, updated 2026-11-08 (twice). Status: accepted. Real-model download and listening checks are queued.

**Context.** The user asked for voice cloning with a local model chosen for their computer, and for scripts where
hmm, laughs and other sounds are cloned too, both from text with placeholders and from their own recorded voice.
Then: an in-app download for every model Flux uses, and flexibility in which model runs.

**Decision.**
- **Models (all MIT, Resemble AI), each pinned to a Hugging Face commit with every file's size and SHA-256 in
  `crates/flux-core/src/voice_models.json`:**
  - Chatterbox Turbo (2.99 GB) is suggested first. It clones from one short clip, performs paralinguistic tags
    ([laugh], [chuckle], [sigh], [cough] ...) in the cloned voice, and adds an inaudible watermark.
  - Chatterbox (3.19 GB), English with an emotion control.
  - Chatterbox Multilingual (3.21 GB), 23 languages.
  - Chatterbox voice converter (1.06 GB), turns the user's own recording into the voice, keeping timing, laughs and
    hesitations exactly. It shares files with the multilingual model, and files are stored once per repository.
- **OpenVoice V2 is dropped.** Its MeloTTS front end needs a second Python stack; every Chatterbox model also runs on a
  processor, more slowly, so one runtime covers every computer except Intel Macs.
- **One voice engine:** a portable CPython 3.11 (python-build-standalone, pinned archive and SHA-256) unpacked into
  Flux's data folder, then pip installs pinned `torch==2.6.0`/`torchaudio==2.6.0` (CUDA 12.4 index with an NVIDIA
  chip, CPU index otherwise, default index on Apple silicon) and `chatterbox-tts==0.1.7`. Intel Macs are refused
  with a reason: PyTorch 2.6 has no Intel Mac builds. `flux_voice.py` (embedded in the engine) loads models only
  from Flux's verified folders with the Hugging Face Hub switched off, and speaks a line protocol
  (`PROGRESS`, `RESULT`, `ERROR`) that never contains audio, text or file names.
- **Downloads:** only from `clone.model.download` with `confirm: true`. The app sends it from a dialog that shows
  the size, licence, source and folder; an agent gets `confirm_required` with the same facts and is told to ask.
  HTTPS only (localhost allowed for mirrors and tests). Files resume from a `.part` with a Range request after a
  cancel or a dropped connection, are retried, checked against their SHA-256 before use (a mismatch is thrown away),
  and free space is checked first. Delete keeps files another downloaded model shares.
- **Choice:** any downloaded or added model for speech or conversion (`clone.settings.set`), the device (auto,
  NVIDIA, Apple, processor), language, seed, and each model's own generation settings, clamped to their ranges. A
  folder of a compatible model (for example a fine-tune) can be added by hand and is checked for the files its kind
  needs. A per-call `model` overrides the choice.
- **Output:** a 16-bit WAV with a LIST/INFO chunk saying it is synthetic speech, in whose voice, with consent, and
  which model made it; placed on a "Generated speech" track as one undoable step, or saved to the export folder
  when no project is open. Models that cannot perform sounds leave the tags out and report how many.
- Scripts take three placeholder kinds. Sounds map to the model's tags. Hesitations ([hmm], [um], [uh] ...) are
  written into the text as words with natural punctuation, so the model says them in the voice. Pauses split the
  script into pieces and become exact silence between them.
- Consent: the owner's recorded consent statement plus a clip and an agreement, before any voice exists.
- Not used: Dia (research and educational use only), XTTS (non-commercial licence).

**Testing without the real weights.** The Rust tests and the E2E test point a `Models/catalog.override.json` at a
local mock server and at stub `pip`, `torch` and `chatterbox` packages (`crates/flux-core/tests/voice_stub`), so the
whole path runs (download, resume, checksum, cancel, runtime install, runner, labels) with nothing fetched from the
internet. The real 3 to 6 GB download, real inference and listening checks are in the human and hardware queue.

**Consequences.** Weights are never committed. The voice engine (about 3 GB with CUDA, about 0.9 GB for processors)
is downloaded once, on the user's say-so, and its pip dependency set needs a licence audit (queued).
