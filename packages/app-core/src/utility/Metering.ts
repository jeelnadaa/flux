/**
 * Utility Metering Domain Models and Calculations per PLAN.md §3.5.
 */

export interface LoudnessMetrics {
  integratedLufs: number;
  shortTermLufs: number;
  momentaryLufs: number;
  loudnessRangeLu: number;
  truePeakDbtp: number;
}

export type LoudnessTarget = "-14 LUFS" | "-16 LUFS" | "-23 LUFS";

export interface LoudnessTargetConfig {
  name: LoudnessTarget;
  targetLufs: number;
  maxTruePeakDbtp: number;
  description: string;
}

export const LOUDNESS_TARGETS: Record<LoudnessTarget, LoudnessTargetConfig> = {
  "-14 LUFS": {
    name: "-14 LUFS",
    targetLufs: -14.0,
    maxTruePeakDbtp: -1.0,
    description: "Streaming standard (Spotify, YouTube, Tidal)",
  },
  "-16 LUFS": {
    name: "-16 LUFS",
    targetLufs: -16.0,
    maxTruePeakDbtp: -1.0,
    description: "Podcast Stereo standard (Apple Podcasts, AES TD1004)",
  },
  "-23 LUFS": {
    name: "-23 LUFS",
    targetLufs: -23.0,
    maxTruePeakDbtp: -1.0,
    description: "European Broadcast Union standard (EBU R128)",
  },
};

export const DEFAULT_LOUDNESS_METRICS: LoudnessMetrics = {
  integratedLufs: -14.2,
  shortTermLufs: -12.8,
  loudnessRangeLu: 6.4,
  truePeakDbtp: -1.1,
  momentaryLufs: -13.5,
};

/**
 * Computes difference in Loudness Units (LU) between measured integrated LUFS and target.
 */
export function computeLoudnessDifference(measuredLufs: number, target: LoudnessTarget): number {
  const targetVal = LOUDNESS_TARGETS[target].targetLufs;
  return Number((measuredLufs - targetVal).toFixed(2));
}

/**
 * Formats a dBTP value to string with sign and single decimal point.
 */
export function formatDbtp(val: number): string {
  if (val > 0) {
    return `+${val.toFixed(1)} dBTP`;
  }
  return `${val.toFixed(1)} dBTP`;
}

/**
 * Formats a LUFS value to string with single decimal point.
 */
export function formatLufs(val: number): string {
  return `${val.toFixed(1)} LUFS`;
}
