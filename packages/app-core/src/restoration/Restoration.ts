/**
 * Classical Audio Restoration domain models, configurations, and algorithms per PLAN.md §3.4.
 */

export interface NoiseReductionConfig {
  amountDb: number; // e.g. -18.0 dB
  sensitivity: number; // 0 to 100
  smoothingMs: number; // e.g. 120 ms
  keepRoomToneDb: number; // e.g. -24 dB
  listenDifference: boolean;
  profile: {
    label: string;
    startSec: number;
    endSec: number;
    learned: boolean;
  };
}

export interface HumRemovalConfig {
  fundamentalHz: 50 | 60 | number;
  harmonics: number; // 1 to 8
  q: number; // 5 to 50
  depthDb: number;
}

export interface ClicksConfig {
  sensitivity: number; // 0 to 100
  maxWidthSamples: number;
  deCrackle: boolean;
}

export interface ClippingConfig {
  thresholdDbfs: number; // e.g. -0.2 dBFS
  reconstructionMethod: "Cubic" | "AR" | "Spline";
}

export interface WindConfig {
  cutoffHz: number; // 20 to 200 Hz
  reductionDb: number;
}

export interface StereoToolsConfig {
  widthPercent: number; // 0 to 200%
  pan: number; // -100 to +100
  panLaw: "Linear" | "EqualPower3dB" | "EqualPower4p5dB";
  swapChannels: boolean;
  invertLeft: boolean;
  invertRight: boolean;
}

export interface PauseItem {
  id: string;
  timestampFormatted: string;
  durationSec: number;
  action: "Shorten" | "Keep" | "Delete";
  shortenToMs: number;
  enabled: boolean;
}

export interface SilenceRemoverConfig {
  longerThanSec: number; // default 0.6 s
  shortenToMs: number; // default 300 ms
  paddingMs: number; // default 150 ms
  thresholdDbfs: number; // default -42 dBFS
  detectMode: "Speech only" | "Any audio";
  protectBreaths: boolean;
  crossfadeJoins: boolean;
  pauses: PauseItem[];
}

export const DEFAULT_NOISE_REDUCTION: NoiseReductionConfig = {
  amountDb: -18.0,
  sensitivity: 45,
  smoothingMs: 120,
  keepRoomToneDb: -24.0,
  listenDifference: false,
  profile: {
    label: "Room noise",
    startSec: 0.4,
    endSec: 2.1,
    learned: true,
  },
};

export const DEFAULT_HUM_REMOVAL: HumRemovalConfig = {
  fundamentalHz: 50,
  harmonics: 8,
  q: 30,
  depthDb: -40.0,
};

export const DEFAULT_CLICKS: ClicksConfig = {
  sensitivity: 50,
  maxWidthSamples: 32,
  deCrackle: true,
};

export const DEFAULT_CLIPPING: ClippingConfig = {
  thresholdDbfs: -0.2,
  reconstructionMethod: "Cubic",
};

export const DEFAULT_WIND: WindConfig = {
  cutoffHz: 80,
  reductionDb: -24.0,
};

export const DEFAULT_STEREO_TOOLS: StereoToolsConfig = {
  widthPercent: 100,
  pan: 0,
  panLaw: "EqualPower3dB",
  swapChannels: false,
  invertLeft: false,
  invertRight: false,
};

export const DEFAULT_SILENCE_REMOVER: SilenceRemoverConfig = {
  longerThanSec: 0.6,
  shortenToMs: 300,
  paddingMs: 150,
  thresholdDbfs: -42.0,
  detectMode: "Speech only",
  protectBreaths: true,
  crossfadeJoins: true,
  pauses: [
    {
      id: "p1",
      timestampFormatted: "0:42.1",
      durationSec: 1.9,
      action: "Shorten",
      shortenToMs: 300,
      enabled: true,
    },
    {
      id: "p2",
      timestampFormatted: "1:18.4",
      durationSec: 0.8,
      action: "Shorten",
      shortenToMs: 300,
      enabled: true,
    },
    {
      id: "p3",
      timestampFormatted: "2:06.7",
      durationSec: 3.4,
      action: "Shorten",
      shortenToMs: 300,
      enabled: true,
    },
    {
      id: "p4",
      timestampFormatted: "2:44.0",
      durationSec: 1.1,
      action: "Keep",
      shortenToMs: 300,
      enabled: false,
    },
  ],
};

/**
 * Calculates stereo phase correlation between left and right channels in [-1.0, +1.0].
 */
export function calculatePhaseCorrelation(
  left: Float32Array | number[],
  right: Float32Array | number[],
): number {
  const len = Math.min(left.length, right.length);
  if (len === 0) return 1.0;

  let sumXY = 0;
  let sumXX = 0;
  let sumYY = 0;

  for (let i = 0; i < len; i++) {
    const x = left[i];
    const y = right[i];
    sumXY += x * y;
    sumXX += x * x;
    sumYY += y * y;
  }

  const denom = Math.sqrt(sumXX * sumYY);
  if (denom < 1e-9) return 1.0;
  return Math.min(1.0, Math.max(-1.0, sumXY / denom));
}

/**
 * Applies Mid/Side widening to a stereo frame.
 */
export function applyMidSideWidth(
  left: number,
  right: number,
  widthRatio: number,
): [number, number] {
  const invSqrt2 = Math.SQRT1_2;
  const mid = (left + right) * invSqrt2;
  const side = (left - right) * invSqrt2 * widthRatio;

  const outLeft = (mid + side) * invSqrt2;
  const outRight = (mid - side) * invSqrt2;
  return [outLeft, outRight];
}

/**
 * Formats seconds into M:SS.S display string.
 */
export function formatTimeMs(sec: number): string {
  const m = Math.floor(sec / 60);
  const s = sec % 60;
  return `${m}:${s.toFixed(1).padStart(4, "0")}`;
}
