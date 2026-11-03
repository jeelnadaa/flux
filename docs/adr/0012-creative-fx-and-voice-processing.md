# ADR-0012: Creative Effects and Real-Time Voice Processing

- **Status:** Accepted
- **Deciders:** Flux Engineering
- **Date:** 2026-11-03
- **Traceability:** PLAN.md Phase 5, AGENTS.md, DESIGN.md

## Context
Phase 5 introduces creative and media-texture transformations (Slowed + Reverb, Nightcore, Lo-Fi macro, Vinyl, Cassette, Old Radio, Phone, Megaphone, Walkie-talkie, Underwater, Bitcrusher, Waveshaper, Glitch/Rhythm) and a 16-character real-time voice changer.
These effects need to run seamlessly both in the real-time AudioWorklet graph (zero allocations, click-free parameter smoothing) and in offline export rendering, matching sample-for-sample.

## Decision
1. Implement all creative DSP nodes (`SlowedReverbNode`, `NightcoreNode`, `LofiMacroNode`, `VinylNode`, `CassetteTapeNode`, `MediumFilterNode`, `UnderwaterNode`, `BitcrusherNode`, `GlitchRhythmNode`, and `VoiceChangerNode`) in Rust (`crates/flux-dsp/src/creative.rs` and `crates/flux-dsp/src/voice.rs`).
2. Implement 16 original character voice presets (Deep, Chipmunk, Child, Elder, ShiftUp, ShiftDown, Whisper, Announcer, RadioDj, Villain, Giant, Tiny, Robot, Cyborg, Alien, Monster) adhering strictly to ADR-0008 (no celebrity impersonations or trademarked names).
3. Integrate pitch shifting, formant preservation, ring modulation, comb filtering, vocoder synthesis, and hard-tune pitch quantization into the voice engine.
4. Export C ABI entry points in `crates/flux-wasm` for WebAssembly AudioWorklet execution.
5. Create UI screens in `@flux/web` (`CreativeScreen`, `TexturesScreen`, `GlitchScreen`, `VoiceScreen`) replicating the prototype screens `#creative`, `#textures`, `#glitch`, and `#voice` with interactive visual controls, responsive layouts, and visual diff test suites.

## Consequences
- **Positive:** Bit-exact deterministic DSP matching between real-time worklet and offline export; zero-allocation processing loops during audio execution; complete compliance with privacy and legal guidelines.
- **Negative:** Ring modulation and comb filtering require careful gain compensation to avoid saturation on extreme parameter settings.
