# ADR 0002: Live presets are modular graphs, not fixed effect chains

Date: 2026-11-07. Status: accepted.

**Context.** The user asked for live presets on computer sound and the microphone that are "insanely
customizable": a person should be able to build almost anything.

**Decision.** A preset is parallel lanes of effects, a post chain and a safety limiter. Modulators (10 shapes
including a step sequencer and an envelope follower, tempo-synced or free) drive any parameter; macros move many
parameters with ranges and curves. Presets are JSON (`.fluxlive`), validated as hostile input on import. Built-ins
are read-only and copy on first edit. Updates reach the audio thread through a lock-free queue; the graph is
rebuilt off the audio thread and swapped in.

**Capture.** WASAPI loopback (Windows), CoreAudio process tap (macOS 14.6+), Monitor inputs (Linux). Flux does not
install a virtual audio driver; to send processed sound into a call app, the user routes through a virtual cable.

**Consequences.** One preset format serves the UI, MCP and import/export. `live.render` lets agents audition a
preset offline, so presets can be designed without speakers.
