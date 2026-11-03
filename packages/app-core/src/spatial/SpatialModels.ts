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
