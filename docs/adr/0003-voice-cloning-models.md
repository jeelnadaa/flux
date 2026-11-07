# ADR 0003: Voice cloning uses local MIT models picked for the computer

Date: 2026-11-07, updated 2026-11-08. Status: accepted; inference blocked on permission to download weights.

**Context.** The user asked for voice cloning with a local model chosen for their computer, and for scripts where
hmm, laughs and other sounds are cloned too, both from text with placeholders and from their own recorded voice.

**Decision.**
- **Chatterbox Turbo (MIT)** is the default. It clones from one short clip and performs paralinguistic tags
  ([laugh], [chuckle], [sigh], [cough] ...) in the cloned voice, and adds an inaudible watermark.
- **Chatterbox (MIT)** converts the user's own recording into the cloned voice, keeping timing, laughs and
  hesitations exactly; offered with an NVIDIA chip with ≥ 6 GB video memory or Apple silicon with ≥ 16 GB.
- **OpenVoice V2 (MIT)** is the lightest fallback.
- Not used: Dia (research and educational use only), XTTS (non-commercial licence).
- Scripts take three placeholder kinds. Sounds map to the model's tags. Hesitations ([hmm], [um], [uh] ...) are
  written into the text as words with natural punctuation, so the model says them in the voice rather than pasting
  a sound effect. Pauses split the script into pieces and become exact silence between them, because models do not
  hold a requested pause length.
- Consent: the owner's recorded consent statement plus a clip and an agreement, before any voice exists.

**Consequences.** Weights (1.4 GB and 3.2 GB) are never committed and are downloaded only when the user agrees.
Until then `clone.speak` and `clone.convert` return `unavailable` with the model they would use and why.
