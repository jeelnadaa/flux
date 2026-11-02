# QUALITY_TARGETS.md — Flux Measured Quality Standards

> **Living Document:** Every DSP algorithm, effect, parser, UI performance target, and AI model in Flux must satisfy objective, automated numerical thresholds. 
> 
> **Threshold Discipline (PLAN.md §5.5):** Never loosen a threshold, tolerance, or golden reference to make a failing test pass without an Architecture Decision Record (ADR) in `docs/adr/` documenting measured empirical evidence.

---

## 1. Core Engine & DSP Precision

| Domain | Characteristic | Metric | Target | Verification Method |
|---|---|---|---|---|
| **Internal Precision** | Signal Path | Float width | 32-bit float (`f32`) standard; 64-bit (`f64`) for recursive IIR/biquad states | Type checking & bit-depth assertions |
| **Clean Passthrough** | Digital Noise Floor | THD+N | $\le -110\text{ dBFS}$ | 1 kHz sine @ 0 dBFS input through bypassed engine |
| **Passthrough Identity** | Bypass / Null | Residual peak | $-\infty\text{ dBFS}$ (bit-exact passthrough) | Subtraction of input from bypass output |
| **Realtime vs Offline** | Engine Parity | Null residual | $\le -100\text{ dBFS}$ | Null subtraction between realtime capture and offline bounce |
| **Block Size Invariance** | Block Independence | Max difference | Bit-exact for LTI; $\le -120\text{ dBFS}$ for float reordering | Render with block sizes {1, 7, 64, 128, 512, 1024, 4096} |
| **Sample Rate Invariance**| Rate Independence | Spectral variance | $\le 0.1\text{ dB}$ across shared audible band | Test across 22.05, 32, 44.1, 48, 88.2, 96, 192 kHz |
| **Denormal Flushing** | Underflow Handling | CPU variance | $\le 5\%$ CPU increase on sub-normal signals | Test decay to $-300\text{ dBFS}$ on WASM build |
| **Memory Allocation** | Real-Time Safety | Allocations in audio thread | **0 allocations** | Instrumented allocator assertion (`assert_no_alloc`) |

---

## 2. Filters, Equalization & Resampling

| Domain | Characteristic | Metric | Target | Verification Method |
|---|---|---|---|---|
| **Biquad Filters / EQ** | Magnitude Response | Error vs Analytic | $\le 0.05\text{ dB}$ across $20\text{ Hz} - 20\text{ kHz}$ ($\ge 44.1\text{ kHz}$) | Impulse FFT comparison against analytical formula |
| **Resampler (High)** | Passband Ripple | Peak-to-peak ripple | $\le 0.01\text{ dB}$ ($0\text{ Hz} - 20\text{ kHz}$) | Chirp / multitone sweep through resampler |
| **Resampler (High)** | Stopband Attenuation | Aliasing / image suppression | $\ge 120\text{ dB}$ (images $< -120\text{ dBFS}$) | High-frequency sine transposition test |
| **Resampler (Master)** | Stopband Attenuation | Aliasing / image suppression | $\ge 140\text{ dB}$ | High-frequency sine transposition test |
| **Filter Stability** | Parameter Traversal | Bounded output | No NaN, no Inf, output bounded within $\pm 4.0$ | Automated random parameter sweeps (`proptest`) |
| **Parameter Smoothing** | Zipper Noise | Max inter-sample $\Delta$ | Bound within smoothing slew rate (no clicks) | Transient detector on stepped parameter output |

---

## 3. Time, Pitch & Dynamics

| Domain | Characteristic | Metric | Target | Verification Method |
|---|---|---|---|---|
| **Pitch Shift** | Fundamental Accuracy | Frequency error | $\le \pm 3\text{ cents}$ across $\pm 12\text{ semitones}$ | Autocorrelation / YIN f0 detector on shifted sine/voice |
| **Formant Preservation**| Formant Stability | Spectral envelope shift | $\le 3\%$ shift in formant peaks (LPC) | LPC spectral envelope matching on vowel phonemes |
| **Time Stretch** | Duration Accuracy | Sample drift | $\le \pm 1\text{ sample}$ per $10\text{ s}$ ($\le 0.01\%$) | Zero-crossing and length verification |
| **Limiter** | True Peak Inter-Sample| Max overshot | $\le 0.0\text{ dBTP}$ with lookahead enabled | 4x oversampled true-peak detector (BS.1770) |
| **Compressor** | Gain Reduction Curve | Curve error vs formula | $\le 0.1\text{ dB}$ steady state | Static level response measurement |

---

## 4. Metering & Loudness Compliance

| Standard | Metric | Target | Reference Test Suite |
|---|---|---|---|
| **ITU-R BS.1770-4** | Integrated Loudness | $\pm 0.1\text{ LU}$ | EBU Tech 3341 / 3342 test signals |
| **EBU R128** | Loudness Range (LRA) | $\pm 0.1\text{ LU}$ | EBU Tech 3342 synthetic test vectors |
| **ITU-R BS.1770-4** | True Peak | $\pm 0.3\text{ dBTP}$ | EBU Tech 3341 inter-sample peak signals |
| **Phase Correlation** | Stereo Correlation | $\pm 0.01$ range $[-1.0, +1.0]$ | In-phase, anti-phase, and decorrelated noise tests |

---

## 5. Performance, Latency & System Budgets

| Metric | Reference Device Class | Budget | Verification Method |
|---|---|---|---|
| **Audio Callback CPU** | Mid-range phone proxy (Chromium 6x throttle) | $\le 30\%$ of block budget ($128\text{ frames} \approx 2.67\text{ ms}$) | High-resolution performance timer in AudioWorklet |
| **Audio Callback CPU** | Low-end laptop proxy (Chromium 4x throttle) | $\le 20\%$ of block budget | Benchmark 8-node rack in headless harness |
| **End-to-End Latency** | Web Audio (round-trip) | $\le 100\text{ ms}$ buffer-to-ear | Automated latency test using loopback pulse |
| **Cold Start Time** | Desktop browser | $\le 1.5\text{ s}$ to interactive canvas | Playwright performance metrics |
| **Memory Footprint** | Active 1-Hour Multitrack Session | $\le 450\text{ MB}$ client RAM | Chrome DevTools Memory Heap snapshot |
| **UI Frame Rate** | Canvas timeline & meters | $\ge 60\text{ fps}$ continuous | `requestAnimationFrame` delta tracking under playback |
| **Interaction to Next Paint (INP)** | General editing gestures | $\le 50\text{ ms}$ | Chrome Web Vitals profiler |

---

## 6. Artificial Intelligence & Audio Restoration

| Task | Metric | Baseline Target | Dataset / Test Method | Gate Type |
|---|---|---|---|---|
| **Stem Separation** | SDR (Vocals) | $\ge 8.0\text{ dB}$ median | MUSDB18-HQ test set | [D] |
| **Stem Separation** | SDR (Drums / Bass) | $\ge 7.0\text{ dB}$ median | MUSDB18-HQ test set | [D] |
| **Neural Denoising** | DNSMOS (OVRL) | Improve by $\ge +0.5\text{ MOS}$ | Interspeech DNS Challenge test set | [D] |
| **Speech Preservation**| PESQ / STOI | PESQ $\ge 3.0$, STOI $\ge 0.90$ | Clean speech + added additive noise | [A] |
| **Speech-to-Text (ASR)**| Word Error Rate (WER) | $\le 5.0\%$ | LibriSpeech `test-clean` | [D] |
| **Beat Tracking** | F-measure | $\ge 0.85$ | Ballroom & GTZAN beat datasets | [D] |
| **Key Detection** | MIREX Weighted Score | $\ge 0.70$ | GiantSteps Key dataset | [D] |

---

## 7. Storage Durability & Fault Tolerance

| Scenario | Requirement | Verification Method |
|---|---|---|
| **Crash Mid-Write** | 0 committed operations lost; restore to last valid journal command | Process kill simulation during active command journal append |
| **Disk / Quota Exhaustion**| Graceful warning at 80% and 95% quota; block new audio recording safely without corrupting project | Virtual quota limit simulation |
| **Hostile Package (.flux)** | Path traversal (zip-slip) blocked; decompression bomb blocked ($>10\text{ GB}$ or ratio $>100:1$ rejected) | Hostile archive fuzzing in `flux-engine` |
