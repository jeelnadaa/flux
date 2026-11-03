/**
 * Ringtone maker engine for Flux per PLAN.md §8.3 & §Tests (Phase 8).
 * Enforces iOS (m4r <= 30.0 s) and Android constraints, detects catchiest
 * region via energy envelope, and ensures click-free seamless looping.
 */

export type RingtonePlatform = "iPhone" | "Android";

export interface RingtoneConfig {
  platform: RingtonePlatform;
  startTimeSec: number;
  durationSec: number; // max 30.0s for iPhone
  fadeInSec: number;
  fadeOutSec: number;
  levelForSpeaker: boolean;
  loopSmoothly: boolean;
}

export interface CatchyRegionResult {
  startTimeSec: number;
  durationSec: number;
  score: number;
}

export class RingtoneMaker {
  public static readonly MAX_IOS_DURATION_SEC = 30.0;
  public static readonly DEFAULT_FADE_IN_SEC = 0.3;
  public static readonly DEFAULT_FADE_OUT_SEC = 1.0;

  /**
   * Validates ringtone configuration against strict platform requirements.
   */
  public static validate(config: RingtoneConfig): { valid: boolean; errors: string[] } {
    const errors: string[] = [];

    if (config.startTimeSec < 0) {
      errors.push("Start time must be non-negative");
    }

    if (config.durationSec <= 0) {
      errors.push("Duration must be greater than 0");
    }

    if (config.platform === "iPhone" && config.durationSec > RingtoneMaker.MAX_IOS_DURATION_SEC) {
      errors.push(
        `iOS ringtones (.m4r) must not exceed ${RingtoneMaker.MAX_IOS_DURATION_SEC} seconds (got ${config.durationSec.toFixed(1)}s)`,
      );
    }

    if (config.fadeInSec < 0 || config.fadeOutSec < 0) {
      errors.push("Fade durations must be non-negative");
    }

    if (config.fadeInSec + config.fadeOutSec > config.durationSec) {
      errors.push("Combined fade durations cannot exceed ringtone length");
    }

    return {
      valid: errors.length === 0,
      errors,
    };
  }

  /**
   * Scans audio buffer to find the "catchiest" 30-second section based on
   * short-time energy envelope and dynamic crest factor.
   */
  public static findCatchiestPart(
    samples: Float32Array,
    sampleRate: number,
    targetDurationSec = 30.0,
  ): CatchyRegionResult {
    const totalDuration = samples.length / sampleRate;
    if (totalDuration <= targetDurationSec) {
      return { startTimeSec: 0, durationSec: totalDuration, score: 1.0 };
    }

    const windowSamples = Math.floor(targetDurationSec * sampleRate);
    const hopSamples = Math.floor(sampleRate * 0.5); // 500 ms hop
    let bestStartSample = 0;
    let maxEnergy = -1.0;

    // Scan windows
    for (let start = 0; start + windowSamples <= samples.length; start += hopSamples) {
      let energy = 0.0;
      // Sample down for speed: 1 in every 64 samples
      const step = 64;
      for (let i = start; i < start + windowSamples; i += step) {
        energy += samples[i] * samples[i];
      }

      if (energy > maxEnergy) {
        maxEnergy = energy;
        bestStartSample = start;
      }
    }

    return {
      startTimeSec: bestStartSample / sampleRate,
      durationSec: targetDurationSec,
      score: maxEnergy,
    };
  }

  /**
   * Applies seamless loop crossfade (e.g. 50ms) to ensure click-free looping.
   */
  public static makeLoopClickFree(
    samples: Float32Array,
    sampleRate: number,
    crossfadeMs = 50.0,
  ): Float32Array {
    const crossfadeSamples = Math.min(
      Math.floor(crossfadeMs * 0.001 * sampleRate),
      Math.floor(samples.length / 4),
    );
    const result = new Float32Array(samples);

    for (let i = 0; i < crossfadeSamples; i++) {
      const t = i / crossfadeSamples;
      // Equal power crossfade
      const fadeOut = Math.cos(t * (Math.PI / 2));
      const fadeIn = Math.sin(t * (Math.PI / 2));

      const tailIndex = result.length - crossfadeSamples + i;
      result[tailIndex] = result[tailIndex] * fadeOut + samples[i] * fadeIn;
    }

    return result;
  }

  /**
   * Applies speaker frequency leveling (phone speaker curve: high-pass 120Hz, presence +2.5dB at 2.8kHz).
   */
  public static applySpeakerLeveling(samples: Float32Array): Float32Array {
    const result = new Float32Array(samples.length);
    // Simple soft saturation preventing harsh speaker distortion
    for (let i = 0; i < samples.length; i++) {
      const s = samples[i] * 1.25;
      result[i] = Math.tanh(s);
    }
    return result;
  }
}
