# Flux User Guide & Production Manual

Welcome to **Flux**, a professional, non-destructive web audio editor with 3D/8D spatial audio, creative effects, and on-device AI.

---

## 1. Quick Start

1. Open Flux in your browser (or install it as a PWA via the browser address bar icon).
2. Choose a project template from the **New Project** screen (`#new`) or drag audio files directly onto the **Home** screen (`#home`).
3. Press **Space** to play/pause. Use the transport controls at the bottom to scrub, loop, or record.

---

## 2. Multitrack Timeline & Editing

- **Splitting Clips:** Move the playhead to the desired position and press `S` (or right-click and select **Split**).
- **Moving Clips:** Click and drag any clip horizontally along the timeline or vertically to another track.
- **Trimming & Fades:** Drag the edge handles of any clip to trim non-destructively. Drag the upper corner fade handles to apply smooth fadeIn/fadeOut curves (Linear, Equal Power, S-Curve, Logarithmic, Exponential).
- **Slip Editing:** Alt/Option-drag inside a clip to adjust the source audio offset without moving its timeline position.
- **Undo / Redo:** Press `Ctrl+Z` to undo and `Ctrl+Shift+Z` / `Ctrl+Y` to redo. View the branching history tree on the **History** screen (`#history`).

---

## 3. Spatial Audio & 8D Sound Stage

- Navigate to the **Sound Stage** (`#stage`) or use the **Task: 8D Audio** shortcut (`#task`).
- **3D Placement:** Drag the source circle on the circular radar canvas to position audio in 3D space (azimuth, elevation, distance).
- **8D Auto-Orbit:** Select **Orbit**, choose direction (Clockwise, Counter-Clockwise, or Ping-Pong), and set orbit speed (e.g. 0.15 Hz). Toggle **Tempo Sync** to lock revolutions to project BPM.
- **16D Trajectories:** Switch to **Path** mode to choose from advanced 3D motion paths: *Spiral*, *Figure-8 / Lemniscate*, *Vertical Loop*, *Swarm*, or *3D Lissajous*.
- **Headphone Check:** Open `#hpcheck` to verify binaural stereo imaging, HRTF profile alignment, and phase orientation before final export.

---

## 4. Creative Transformations

- **Slowed + Reverb (`#task-slowed`, `#creative`):** Adjust speed ratio ($0.5\times\text{--}1.0\times$) for authentic chopped-and-screwed textures, paired with warm lowpass filtering and hall reverberation.
- **Nightcore (`#creative`):** Speed up audio ($1.0\times\text{--}1.6\times$) with pitch elevation, high-shelf sparkle, and shimmer chorus.
- **Lo-Fi Media Textures (`#textures`):** Add authentic vinyl surface crackle, cassette tape bias/hiss, 12-bit vintage sampler downsampling, or old telephone/megaphone bandpass filters.
- **16-Character Voice Changer (`#voice`):** Audition character presets (*Deep, Chipmunk, Villain, Robot, Alien, Monster, Radio DJ, etc.*) with real-time mic monitoring, ring modulation, and pitch/formant warping.

---

## 5. On-Device AI Tools

- **Stem Separation (`#stems`):** Decompose any mixed song into 4 isolated stems: *Vocals*, *Drums*, *Bass*, and *Other*. Generate Karaoke (instrumental backing) or Acapella (isolated lead vocals) with one click.
- **Voice Cleanup / Studio Sound (`#cleanup`):** Remove background HVAC noise, traffic rumble, room reflections, and acoustic reverberation while preserving natural speech dynamics.
- **Transcript Audio Editing (`#script`):** View automatic speech-to-text transcript. Select and delete unwanted words; Flux automatically cuts the underlying audio with 3ms micro-crossfades and room-tone fill.
- **Filler Word Removal (`#fillers`):** Scan for verbal hesitations (*"um", "uh", "like"*); review detections with context playback and remove them in bulk.
- **Auto-Mastering (`#master`):** Select broadcast targets (-14.0 LUFS Streaming, -16.0 LUFS Podcast, -9.0 LUFS Club); audition loudness-matched before/after comparisons with the hold-to-compare button.

---

## 6. Pro Workflow & Automation

- **Batch Processor (`#batch`):** Queue up to 200 files for automated multi-step processing (format conversion, loudness normalization, stem separation, silence trimming).
- **Ringtone Maker (`#ringtone`):** Automatically scan for peak energy ("catchiest section"), fit strict 30-second iOS M4R or Android limits, and apply phone speaker leveling.
- **Action Macros (`#macros`):** Record sequences of edits and parameter changes into reusable macros that replay deterministically.
- **Keyboard Shortcuts:** Press `?` anywhere to open the shortcuts cheat sheet (`#shortcuts`), or press `Ctrl+K` to open the quick Command Palette (`#palette`).
