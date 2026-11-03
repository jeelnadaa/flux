# ADR 0004: Spatial Reverb, Early Reflections, and Acoustic Environment Simulation

## Context
Phase 4 requires immersive acoustic environments for spatialized audio:
1. Reverb engines: FDN (algorithmic) with unitary Householder scattering and partitioned convolution for impulse responses.
2. 12 room presets matching physical spaces (Cathedral, Bathroom, Concert Hall, Cave, etc.).
3. Custom room designer using Sabine RT60 formula and image-source reflection timing.
4. "Row 10" concert hall acoustics with PA coloration, slapback delay, and procedural crowd ambience.
5. "Next Door" through-the-wall acoustic transmission loss with steep 24 dB/oct low-pass filtering, structural flanking bass boost, and level drift.

## Decision
1. **Algorithmic Reverb**: Implemented an 8-delay Feedback Delay Network (FDN) using mutually-prime delay lengths at 48 kHz to prevent resonant modal buildup. Utilized a lossless unitary Householder scattering matrix $H = I - \frac{2}{N} \mathbf{1}\mathbf{1}^T$ for stable, smooth energy recirculation. Added 4 cascaded Schroeder allpass diffusers at the input for instant echo density without coloration, and 1-pole low-pass damping filters in each feedback loop.
2. **Room Presets & Custom Room Designer**: Implemented analytical Sabine equation ($RT_{60} = \frac{0.161 \cdot V}{\sum S_i \alpha_i}$) with 10 calibrated surface materials (plaster, concrete, brick, wood, glass, curtain, carpet, stone, tile, acoustic tile). First reflection arrival times are estimated from shortest boundary bounce deltas.
3. **Concert Hall**: Coupled the FDN reverb with a 2.4 kHz peaking horn filter (+3 dB) modeling PA coloration, a 82 ms slapback delay for back-wall reflections, and a deterministic PRNG crowd murmur bed.
4. **Next Door**: Modeled transmission loss with cascaded 2nd-order lowpass filters (24 dB/oct slope, 200–2000 Hz cutoff), a low-shelf flanking bass boost at 90 Hz, and secondary diffuse neighbor reflections.

## Consequences
- Zero dynamic memory allocations during real-time quantum processing on the audio thread.
- Exact analytic verification in cargo unit tests, DSP invariant tests, and WebAssembly C ABI bindings.
- Fully synchronized UI visualizers (`V.room` perspective wireframe and `V.decay` exponential decay curve).
