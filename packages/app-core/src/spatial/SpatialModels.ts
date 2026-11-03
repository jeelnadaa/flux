/**
//! Spatial audio domain models, coordinate transforms, and acoustic calculations.
 */

export type HrtfProfileId = "A" | "B" | "C" | "D" | "E" | "F";

export interface HrtfProfileInfo {
  id: HrtfProfileId;
  name: string;
  description: string;
  headWidthMultiplier: number;
  pinnaRearCueFactor: number;
  trebleBalance: number;
}

export const HRTF_PROFILES: Record<HrtfProfileId, HrtfProfileInfo> = {
  A: {
    id: "A",
    name: "Profile A",
    description: "Wide head, bright",
    headWidthMultiplier: 1.1,
    pinnaRearCueFactor: 0.8,
    trebleBalance: 1.2,
  },
  B: {
    id: "B",
    name: "Profile B",
    description: "Medium, neutral",
    headWidthMultiplier: 1.0,
    pinnaRearCueFactor: 1.0,
    trebleBalance: 1.0,
  },
  C: {
    id: "C",
    name: "Profile C",
    description: "Medium, strong rear cue",
    headWidthMultiplier: 1.0,
    pinnaRearCueFactor: 1.35,
    trebleBalance: 1.05,
  },
  D: {
    id: "D",
    name: "Profile D",
    description: "Narrow, bright",
    headWidthMultiplier: 0.9,
    pinnaRearCueFactor: 0.9,
    trebleBalance: 1.25,
  },
  E: {
    id: "E",
    name: "Profile E",
    description: "Wide, warm",
    headWidthMultiplier: 1.1,
    pinnaRearCueFactor: 1.0,
    trebleBalance: 0.85,
  },
  F: {
    id: "F",
    name: "Profile F",
    description: "Narrow, warm",
    headWidthMultiplier: 0.9,
    pinnaRearCueFactor: 0.95,
    trebleBalance: 0.9,
  },
};

export type SpatialStageMode = "Orbit" | "Spiral" | "Figure 8" | "Swarm" | "Place";
export type SpatialTool = "Drag" | "Draw path";
export type OrbitDirection = "Clockwise" | "Counter" | "Ping-pong";
export type StereoInputMode = "Two speakers" | "Mid and side" | "Stems";
export type SpatialRendererMode = "Best" | "Lite";

export interface CartesianCoord {
  x: number;
  y: number;
  z: number;
}

export interface SphericalCoord {
  azimuthDeg: number;
  elevationDeg: number;
  distanceM: number;
}

export interface ItdResult {
  delayLeftSec: number;
  delayRightSec: number;
  itdSec: number;
}

export const SPEED_OF_SOUND = 343.0; // m/s
export const DEFAULT_HEAD_WIDTH_CM = 17.5;

/**
 * Converts spherical coordinates (azimuth, elevation, distance) to 3D Cartesian (x, y, z).
 * Right-handed coordinate system: X is right, Y is forward, Z is up.
 */
export function sphericalToCartesian(
  azimuthDeg: number,
  elevationDeg: number,
  distanceM: number,
): CartesianCoord {
  const azRad = (azimuthDeg * Math.PI) / 180.0;
  const elRad = (elevationDeg * Math.PI) / 180.0;

  const cosEl = Math.cos(elRad);
  return {
    x: distanceM * cosEl * Math.sin(azRad),
    y: distanceM * cosEl * Math.cos(azRad),
    z: distanceM * Math.sin(elRad),
  };
}

/**
 * Converts 3D Cartesian coordinates to spherical coordinates.
 */
export function cartesianToSpherical(x: number, y: number, z: number): SphericalCoord {
  const distanceM = Math.sqrt(x * x + y * y + z * z);
  if (distanceM < 1e-6) {
    return { azimuthDeg: 0, elevationDeg: 0, distanceM: 0 };
  }

  const elevationDeg = (Math.asin(Math.max(-1.0, Math.min(1.0, z / distanceM))) * 180.0) / Math.PI;
  let azimuthDeg = (Math.atan2(x, y) * 180.0) / Math.PI;
  if (azimuthDeg < 0) {
    azimuthDeg += 360.0;
  }

  return { azimuthDeg, elevationDeg, distanceM };
}

/**
 * Computes theoretical Woodworth-Schlosser Interaural Time Difference (ITD).
 */
export function computeItdWoodworth(
  azimuthDeg: number,
  headWidthCm = DEFAULT_HEAD_WIDTH_CM,
): ItdResult {
  const headRadiusM = (headWidthCm * 0.01) / 2.0;
  const azRad = (azimuthDeg * Math.PI) / 180.0;
  const sinAz = Math.sin(azRad);

  if (sinAz >= 0) {
    // Source on right: sound hits right ear first
    const theta = Math.asin(Math.min(1.0, sinAz));
    const delayLeft = (headRadiusM / SPEED_OF_SOUND) * (sinAz + theta);
    return {
      delayLeftSec: delayLeft,
      delayRightSec: 0,
      itdSec: delayLeft,
    };
  } else {
    // Source on left: sound hits left ear first
    const absSin = Math.min(1.0, -sinAz);
    const theta = Math.asin(absSin);
    const delayRight = (headRadiusM / SPEED_OF_SOUND) * (absSin + theta);
    return {
      delayLeftSec: 0,
      delayRightSec: delayRight,
      itdSec: delayRight,
    };
  }
}

/**
 * Computes 1/r distance attenuation factor with reference distance and near-field clamping.
 */
export function computeDistanceGain(
  distanceM: number,
  refDistanceM = 1.0,
  minDistanceM = 0.2,
): number {
  const clampedDist = Math.max(minDistanceM, distanceM);
  return Math.min(3.0, refDistanceM / clampedDist);
}

/**
 * Generates continuous 3D coordinate on an Orbit trajectory around listener.
 */
export function generateOrbitPosition(
  tSec: number,
  speedHz: number,
  direction: OrbitDirection,
  distanceM: number,
  elevationDeg: number,
): SphericalCoord {
  let phase = (tSec * speedHz) % 1.0;
  if (direction === "Counter") {
    phase = 1.0 - phase;
  } else if (direction === "Ping-pong") {
    const cycle = (tSec * speedHz * 2.0) % 2.0;
    phase = cycle > 1.0 ? 2.0 - cycle : cycle;
  }

  const azimuthDeg = (phase * 360.0) % 360.0;
  return {
    azimuthDeg,
    elevationDeg,
    distanceM,
  };
}

/**
 * Generates continuous 3D coordinate on a Figure-8 (Lemniscate) trajectory.
 */
export function generateFigureEightPosition(
  tSec: number,
  speedHz: number,
  radiusM: number,
  elevationDeg: number,
): CartesianCoord {
  const omega = 2.0 * Math.PI * speedHz * tSec;
  // Gerono lemniscate
  const x = radiusM * Math.sin(omega);
  const y = radiusM * Math.sin(omega) * Math.cos(omega);
  const z = (elevationDeg / 90.0) * radiusM * 0.5;

  return { x, y, z };
}

/**
 * Generates continuous 3D coordinate on a Spiral trajectory (varying distance and elevation).
 */
export function generateSpiralPosition(
  tSec: number,
  speedHz: number,
  minRadiusM: number,
  maxRadiusM: number,
  minElevDeg: number,
  maxElevDeg: number,
): SphericalCoord {
  const phase = (tSec * speedHz) % 1.0;
  const az = (phase * 360.0 * 2.0) % 360.0;
  const cycle = Math.sin(phase * 2.0 * Math.PI) * 0.5 + 0.5;

  const dist = minRadiusM + (maxRadiusM - minRadiusM) * cycle;
  const el = minElevDeg + (maxElevDeg - minElevDeg) * cycle;

  return {
    azimuthDeg: az,
    elevationDeg: el,
    distanceM: dist,
  };
}

/**
 * Calculates 3D virtual speaker coordinates for Left and Right channels given a central azimuth
 * and angular spread (default 60 deg, +/-30 deg).
 */
export function calculateVirtualSpeakerPositions(
  azimuthDeg: number,
  spreadDeg = 60.0,
  distanceM = 1.0,
  elevationDeg = 0.0,
): { left: SphericalCoord; right: SphericalCoord } {
  const halfSpread = spreadDeg * 0.5;
  let leftAz = (azimuthDeg - halfSpread) % 360.0;
  if (leftAz < 0) {
    leftAz += 360.0;
  }
  let rightAz = (azimuthDeg + halfSpread) % 360.0;
  if (rightAz < 0) {
    rightAz += 360.0;
  }

  return {
    left: { azimuthDeg: leftAz, elevationDeg, distanceM },
    right: { azimuthDeg: rightAz, elevationDeg, distanceM },
  };
}

/**
 * Encodes stereo Left and Right channels into Mid (sum) and Side (difference) channels.
 */
export function calculateMidSideSignals(
  left: Float32Array,
  right: Float32Array,
): { mid: Float32Array; side: Float32Array } {
  const n = Math.min(left.length, right.length);
  const mid = new Float32Array(n);
  const side = new Float32Array(n);

  for (let i = 0; i < n; i++) {
    mid[i] = 0.5 * (left[i] + right[i]);
    side[i] = 0.5 * (left[i] - right[i]);
  }

  return { mid, side };
}

/**
 * Decodes Mid and Side channels back into Left and Right stereo channels.
 */
export function reconstructFromMidSide(
  mid: Float32Array,
  side: Float32Array,
): { left: Float32Array; right: Float32Array } {
  const n = Math.min(mid.length, side.length);
  const left = new Float32Array(n);
  const right = new Float32Array(n);

  for (let i = 0; i < n; i++) {
    left[i] = mid[i] + side[i];
    right[i] = mid[i] - side[i];
  }

  return { left, right };
}

/**
 * Configuration for the stereo spatializer.
 */
export interface StereoSpatialConfig {
  mode: StereoInputMode;
  azimuthDeg: number;
  elevationDeg: number;
  distanceM: number;
  spreadDeg: number;
  monoBassEnabled: boolean;
  monoBassCrossoverHz: number;
  headWidthCm: number;
  profile: HrtfProfileId;
}

/**
 * Default stereo spatializer configuration.
 */
export const DEFAULT_STEREO_SPATIAL_CONFIG: StereoSpatialConfig = {
  mode: "Two speakers",
  azimuthDeg: 0,
  elevationDeg: 0,
  distanceM: 1.0,
  spreadDeg: 60.0,
  monoBassEnabled: true,
  monoBassCrossoverHz: 120.0,
  headWidthCm: DEFAULT_HEAD_WIDTH_CM,
  profile: "C",
};

export type SpatialPresetName =
  | "Front"
  | "Behind"
  | "Above"
  | "Below"
  | "FrontLeft"
  | "FrontRight"
  | "RearLeft"
  | "RearRight"
  | "InsideRoom";

export interface SpatialPresetInfo {
  name: SpatialPresetName;
  label: string;
  azimuthDeg: number;
  elevationDeg: number;
  distanceM: number;
}

export const SPATIAL_PRESETS: Record<SpatialPresetName, SpatialPresetInfo> = {
  Front: { name: "Front", label: "Front center", azimuthDeg: 0, elevationDeg: 0, distanceM: 1.5 },
  Behind: {
    name: "Behind",
    label: "Directly behind",
    azimuthDeg: 180,
    elevationDeg: 0,
    distanceM: 1.5,
  },
  Above: { name: "Above", label: "Above head", azimuthDeg: 0, elevationDeg: 60, distanceM: 1.5 },
  Below: { name: "Below", label: "Below head", azimuthDeg: 0, elevationDeg: -60, distanceM: 1.5 },
  FrontLeft: {
    name: "FrontLeft",
    label: "Front left",
    azimuthDeg: 315,
    elevationDeg: 0,
    distanceM: 1.8,
  },
  FrontRight: {
    name: "FrontRight",
    label: "Front right",
    azimuthDeg: 45,
    elevationDeg: 0,
    distanceM: 1.8,
  },
  RearLeft: {
    name: "RearLeft",
    label: "Rear left",
    azimuthDeg: 225,
    elevationDeg: 0,
    distanceM: 1.8,
  },
  RearRight: {
    name: "RearRight",
    label: "Rear right",
    azimuthDeg: 135,
    elevationDeg: 0,
    distanceM: 1.8,
  },
  InsideRoom: {
    name: "InsideRoom",
    label: "Inside a room",
    azimuthDeg: 30,
    elevationDeg: 15,
    distanceM: 2.5,
  },
};

/**
 * Generates continuous 3D coordinate on a Vertical Loop trajectory.
 */
export function generateVerticalLoopPosition(
  tSec: number,
  speedHz: number,
  distanceM: number,
): SphericalCoord {
  const phase = (tSec * speedHz) % 1.0;
  const omega = phase * 2.0 * Math.PI;
  const el = 85.0 * Math.sin(omega);
  const az = (Math.cos(omega) * 15.0 + 360.0) % 360.0;
  return { azimuthDeg: az, elevationDeg: el, distanceM };
}

/**
 * Generates continuous 3D coordinate on a deterministic seeded random walk / Swarm trajectory.
 */
export function generateSwarmPosition(
  tSec: number,
  speedHz: number,
  distanceM: number,
  seed = 42,
): CartesianCoord {
  const seedOff = (seed % 1000) * 0.1;
  const b = tSec * 1.3 * speedHz + seedOff;
  const r = distanceM;
  const x = r * (0.7 * Math.sin(b) + 0.2 * Math.sin(2.9 * b));
  const y = r * (0.7 * Math.cos(1.7 * b));
  const z = r * (0.4 * Math.sin(0.8 * b));
  return { x, y, z };
}

/**
 * Generates continuous 3D coordinate on a 3D Lissajous trajectory knot.
 */
export function generateLissajousPosition(
  tSec: number,
  speedHz: number,
  distanceM: number,
): CartesianCoord {
  const phase = (tSec * speedHz) % 1.0;
  const omega = phase * 2.0 * Math.PI;
  const r = distanceM;
  const x = r * Math.sin(3.0 * omega);
  const y = r * Math.cos(2.0 * omega);
  const z = r * Math.sin(4.0 * omega) * 0.5;
  return { x, y, z };
}

/**
 * Evaluates Catmull-Rom spline interpolation along a sequence of 2D control points.
 */
export function interpolateCatmullRomSpline(
  points: [number, number][],
  t: number,
): [number, number] {
  if (points.length === 0) return [0, 0];
  if (points.length === 1) return points[0];

  const n = points.length;
  const u = Math.max(0, Math.min(n - 1, Math.max(0, Math.min(1, t)) * (n - 1)));
  const idx = Math.floor(u);
  const s = u - idx;

  const i0 = Math.max(0, idx - 1);
  const i1 = idx;
  const i2 = Math.min(n - 1, idx + 1);
  const i3 = Math.min(n - 1, idx + 2);

  const p0 = points[i0];
  const p1 = points[i1];
  const p2 = points[i2];
  const p3 = points[i3];

  const s2 = s * s;
  const s3 = s2 * s;

  const x =
    0.5 *
    (2.0 * p1[0] +
      (-p0[0] + p2[0]) * s +
      (2.0 * p0[0] - 5.0 * p1[0] + 4.0 * p2[0] - p3[0]) * s2 +
      (-p0[0] + 3.0 * p1[0] - 3.0 * p2[0] + p3[0]) * s3);

  const y =
    0.5 *
    (2.0 * p1[1] +
      (-p0[1] + p2[1]) * s +
      (2.0 * p0[1] - 5.0 * p1[1] + 4.0 * p2[1] - p3[1]) * s2 +
      (-p0[1] + 3.0 * p1[1] - 3.0 * p2[1] + p3[1]) * s3);

  return [x, y];
}

export interface MonoCompatibilityResult {
  correlation: number;
  monoLossDb: number;
  isCompatible: boolean;
}

/**
 * Evaluates mono compatibility (phase correlation and level drop) of stereo buffers.
 */
export function checkMonoCompatibility(
  left: Float32Array | number[],
  right: Float32Array | number[],
): MonoCompatibilityResult {
  const n = Math.min(left.length, right.length);
  if (n === 0) {
    return { correlation: 1.0, monoLossDb: 0.0, isCompatible: true };
  }
  let sumL2 = 0;
  let sumR2 = 0;
  let sumLR = 0;
  let sumM2 = 0;
  for (let i = 0; i < n; i++) {
    const l = left[i];
    const r = right[i];
    const m = 0.5 * (l + r);
    sumL2 += l * l;
    sumR2 += r * r;
    sumLR += l * r;
    sumM2 += m * m;
  }
  const denom = Math.sqrt(sumL2 * sumR2);
  const correlation = denom > 1e-12 ? Math.max(-1.0, Math.min(1.0, sumLR / denom)) : 1.0;
  const stereoPwr = Math.max(1e-12, (0.5 * (sumL2 + sumR2)) / n);
  const monoPwr = Math.max(1e-12, sumM2 / n);
  const ratio = monoPwr / stereoPwr;
  const monoLossDb = -10.0 * Math.log10(ratio);
  const isCompatible = correlation >= 0.0 && monoLossDb <= 6.0;

  return { correlation, monoLossDb, isCompatible };
}

export interface StageStemSource {
  id: string;
  name: string;
  colorVar: string;
  azimuthDeg: number;
  distanceRatio: number;
  mode: "fixed" | "orbit" | "reverse";
  fixed: boolean;
}

export const DEFAULT_STAGE_STEMS: StageStemSource[] = [
  {
    id: "vocals",
    name: "Vocals",
    colorVar: "--t4",
    azimuthDeg: 0,
    distanceRatio: 0.72,
    mode: "fixed",
    fixed: true,
  },
  {
    id: "drums",
    name: "Drums",
    colorVar: "--t3",
    azimuthDeg: 126,
    distanceRatio: 0.85,
    mode: "orbit",
    fixed: false,
  },
  {
    id: "bass",
    name: "Bass",
    colorVar: "--t2",
    azimuthDeg: 235,
    distanceRatio: 0.55,
    mode: "fixed",
    fixed: true,
  },
  {
    id: "other",
    name: "Other",
    colorVar: "--t5",
    azimuthDeg: 309,
    distanceRatio: 0.9,
    mode: "reverse",
    fixed: false,
  },
];
