/**
 * FadeCurves.ts - Mathematical fade and crossfade curves for non-destructive audio editing.
 *
 * Implements:
 * - Linear: y = t
 * - Equal-Power (Constant Energy for uncorrelated audio): y = sin(t * pi / 2), crossfade power = sin^2 + cos^2 = 1.0 (0.0 dB)
 * - Equal-Gain (Linear amplitude for correlated audio): y_in + y_out = 1.0
 * - S-Curve (Smooth Hermite): y = 3t^2 - 2t^3
 * - Logarithmic: y = log10(1 + 9t)
 * - Exponential: y = (e^(3t) - 1) / (e^3 - 1)
 * - Automatic micro-fades (2-5ms) to prevent boundary clicks.
 */

export type FadeCurveType = "linear" | "equalPower" | "sCurve" | "logarithmic" | "exponential";

export class FadeCurves {
  /**
   * Evaluates a fade curve for a normalized progress t in [0.0, 1.0].
   * @param type The mathematical curve type.
   * @param progress Progress in [0, 1].
   * @param isFadeIn True for fade-in (0 -> 1), false for fade-out (1 -> 0).
   * @returns Gain multiplier in [0.0, 1.0].
   */
  static evaluate(type: FadeCurveType, progress: number, isFadeIn: boolean = true): number {
    const t = Math.max(0.0, Math.min(1.0, progress));
    const factor = isFadeIn ? t : 1.0 - t;

    let gain: number;
    switch (type) {
      case "linear":
        gain = factor;
        break;

      case "equalPower":
        // y = sin(t * pi / 2) -> when t=0 gain=0, when t=1 gain=1
        gain = Math.sin((factor * Math.PI) / 2);
        break;

      case "sCurve":
        // Hermite smoothstep: 3t^2 - 2t^3 (zero derivative at endpoints)
        gain = factor * factor * (3.0 - 2.0 * factor);
        break;

      case "logarithmic":
        // Base-10 perceptual curve: rapid rise, flattening at top
        gain = Math.log10(1.0 + 9.0 * factor);
        break;

      case "exponential": {
        // Slow rise, steep rise near end
        const k = 3.0;
        gain = (Math.exp(k * factor) - 1.0) / (Math.exp(k) - 1.0);
        break;
      }

      default:
        gain = factor;
    }

    return Math.max(0.0, Math.min(1.0, gain));
  }

  /**
   * Evaluates equal-power crossfade gain pair for overlapping clips.
   * Power conservation: gainIn^2 + gainOut^2 == 1.0 within numerical precision.
   * @param progress Progress in [0.0, 1.0] across crossfade region.
   */
  static evaluateEqualPowerCrossfade(progress: number): {
    gainFadeIn: number;
    gainFadeOut: number;
  } {
    const t = Math.max(0.0, Math.min(1.0, progress));
    const gainFadeIn = Math.sin((t * Math.PI) / 2);
    const gainFadeOut = Math.cos((t * Math.PI) / 2);
    return { gainFadeIn, gainFadeOut };
  }

  /**
   * Evaluates equal-gain (linear amplitude) crossfade pair for correlated signals.
   * Amplitude conservation: gainIn + gainOut == 1.0.
   */
  static evaluateEqualGainCrossfade(progress: number): { gainFadeIn: number; gainFadeOut: number } {
    const t = Math.max(0.0, Math.min(1.0, progress));
    return { gainFadeIn: t, gainFadeOut: 1.0 - t };
  }

  /**
   * Applies an automatic micro-fade (typically 2ms to 5ms) to an audio buffer boundary
   * to eliminate discontinuity clicks.
   * @param buffer Float32Array of samples.
   * @param sampleRate Audio sample rate in Hz.
   * @param microFadeDurationMs Duration in milliseconds (default 3.0 ms).
   * @param atStart Whether to fade in at the start of buffer.
   * @param atEnd Whether to fade out at the end of buffer.
   */
  static applyMicroFade(
    buffer: Float32Array,
    sampleRate: number,
    microFadeDurationMs: number = 3.0,
    atStart: boolean = true,
    atEnd: boolean = true,
  ): void {
    const fadeSamples = Math.min(
      Math.floor(buffer.length / 2),
      Math.max(1, Math.round((microFadeDurationMs / 1000.0) * sampleRate)),
    );

    if (atStart) {
      for (let i = 0; i < fadeSamples; i++) {
        const t = i / fadeSamples;
        // Half-cosine raised sine smoothing: 0.5 * (1 - cos(pi * t))
        const gain = 0.5 * (1.0 - Math.cos(Math.PI * t));
        buffer[i] *= gain;
      }
    }

    if (atEnd) {
      const len = buffer.length;
      for (let i = 0; i < fadeSamples; i++) {
        const t = i / fadeSamples;
        const gain = 0.5 * (1.0 + Math.cos(Math.PI * t));
        buffer[len - fadeSamples + i] *= gain;
      }
    }
  }
}
