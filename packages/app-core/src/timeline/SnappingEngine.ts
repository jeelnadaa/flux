/**
 * Deterministic timeline snapping engine:
 * - Snap to zero-crossings (click-free audio cuts)
 * - Snap to markers & regions
 * - Snap to clip start & end boundaries
 * - Snap to time/musical grid
 * - Enforces deterministic tie-breaking per PLAN.md §5.
 */

export type SnapTargetType = "zero_crossing" | "marker" | "clip_boundary" | "grid";

export interface SnapCandidate {
  targetTime: number; // in seconds
  targetType: SnapTargetType;
  distancePx: number; // absolute distance in screen pixels
  label?: string;
}

export interface ClipBoundary {
  startTime: number;
  endTime: number;
  label?: string;
}

export interface SnappingOptions {
  thresholdPx?: number; // default 8 pixels
  gridIntervalSeconds?: number;
  markers?: Array<{ time: number; label: string }>;
  clips?: ClipBoundary[];
  samples?: Float32Array; // raw audio samples for zero-crossing analysis
  sampleRate?: number;
}

export class SnappingEngine {
  public static readonly PRIORITY_WEIGHTS: Record<SnapTargetType, number> = {
    zero_crossing: 1, // highest priority
    marker: 2,
    clip_boundary: 3,
    grid: 4, // lowest priority
  };

  /**
   * Evaluates all snapping candidates near queryTime and returns the best snap target,
   * or the original queryTime if no candidate is within thresholdPx.
   */
  public static snap(
    queryTime: number,
    pixelsPerSecond: number,
    options: SnappingOptions = {},
  ): { snappedTime: number; candidate: SnapCandidate | null } {
    const thresholdPx = options.thresholdPx ?? 8;
    const candidates: SnapCandidate[] = [];

    // 1. Grid candidates
    if (options.gridIntervalSeconds && options.gridIntervalSeconds > 0) {
      const interval = options.gridIntervalSeconds;
      const lowerGrid = Math.floor(queryTime / interval) * interval;
      const upperGrid = lowerGrid + interval;

      for (const gridTime of [lowerGrid, upperGrid]) {
        const distPx = Math.abs(gridTime - queryTime) * pixelsPerSecond;
        if (distPx <= thresholdPx) {
          candidates.push({
            targetTime: gridTime,
            targetType: "grid",
            distancePx: distPx,
            label: `Grid ${interval.toFixed(3)}s`,
          });
        }
      }
    }

    // 2. Marker candidates
    if (options.markers && options.markers.length > 0) {
      for (const m of options.markers) {
        const distPx = Math.abs(m.time - queryTime) * pixelsPerSecond;
        if (distPx <= thresholdPx) {
          candidates.push({
            targetTime: m.time,
            targetType: "marker",
            distancePx: distPx,
            label: m.label,
          });
        }
      }
    }

    // 3. Clip boundary candidates
    if (options.clips && options.clips.length > 0) {
      for (const clip of options.clips) {
        for (const t of [clip.startTime, clip.endTime]) {
          const distPx = Math.abs(t - queryTime) * pixelsPerSecond;
          if (distPx <= thresholdPx) {
            candidates.push({
              targetTime: t,
              targetType: "clip_boundary",
              distancePx: distPx,
              label: clip.label,
            });
          }
        }
      }
    }

    // 4. Zero-crossing candidates
    if (options.samples && options.sampleRate && options.sampleRate > 0) {
      const sr = options.sampleRate;
      const centerSample = Math.round(queryTime * sr);
      const searchRadiusSamples = Math.ceil((thresholdPx / pixelsPerSecond) * sr);
      const startSample = Math.max(1, centerSample - searchRadiusSamples);
      const endSample = Math.min(options.samples.length - 1, centerSample + searchRadiusSamples);

      let bestCrossingSample: number | null = null;
      let minCrossingDistPx = Infinity;

      for (let s = startSample; s <= endSample; s++) {
        const s0 = options.samples[s - 1];
        const s1 = options.samples[s];

        // Check for sign flip (must not be flat silence where both are zero)
        if ((s0 < 0 && s1 >= 0) || (s0 > 0 && s1 <= 0)) {
          const frac = s0 / (s0 - s1 || 1e-9);
          const crossingSample = s - 1 + frac;
          const crossingTime = crossingSample / sr;
          const distPx = Math.abs(crossingTime - queryTime) * pixelsPerSecond;
          if (distPx <= thresholdPx && distPx < minCrossingDistPx) {
            minCrossingDistPx = distPx;
            bestCrossingSample = crossingSample;
          }
        }
      }

      if (bestCrossingSample !== null) {
        const t = bestCrossingSample / sr;
        candidates.push({
          targetTime: t,
          targetType: "zero_crossing",
          distancePx: Math.abs(t - queryTime) * pixelsPerSecond,
          label: "Zero crossing",
        });
      }
    }

    if (candidates.length === 0) {
      return { snappedTime: queryTime, candidate: null };
    }

    // Deterministic tie-breaking:
    // 1. Minimum distancePx
    // 2. Primary target type weight (zero_crossing > marker > clip_boundary > grid)
    // 3. Earlier target time
    candidates.sort((a, b) => {
      const distDiff = a.distancePx - b.distancePx;
      if (Math.abs(distDiff) > 1e-4) {
        return distDiff;
      }
      const weightA = SnappingEngine.PRIORITY_WEIGHTS[a.targetType];
      const weightB = SnappingEngine.PRIORITY_WEIGHTS[b.targetType];
      if (weightA !== weightB) {
        return weightA - weightB;
      }
      return a.targetTime - b.targetTime;
    });

    const best = candidates[0];
    return { snappedTime: best.targetTime, candidate: best };
  }
}
