/**
 * Musical tempo map, beat grid calculator, and tap tempo detector.
 */

export interface TempoPoint {
  samplePosition: number;
  bpm: number;
  timeSignature: [number, number];
}

export type StretchMode = "Music" | "Voice" | "Varispeed";
export type StretchQuality = "Fast" | "High" | "Mastering";

export interface SpeedPitchSettings {
  speedRatio: number; // 0.25 to 4.0, default 1.0 (or 0.85 in prototype)
  pitchSemitones: number; // -24.0 to +24.0, default 0.0 (or -2.0 in prototype)
  keepPitch: boolean; // default true
  mode: StretchMode; // default "Music"
  preserveFormants: boolean; // default true
  tempoBpm: number; // default 84.0
  showBeatGrid: boolean; // default true
  renderQuality: StretchQuality; // default "High"
}

export const DEFAULT_SPEED_PITCH_SETTINGS: SpeedPitchSettings = {
  speedRatio: 0.85,
  pitchSemitones: -2.0,
  keepPitch: true,
  mode: "Music",
  preserveFormants: true,
  tempoBpm: 84.0,
  showBeatGrid: true,
  renderQuality: "High",
};

/**
 * Calculates sample-accurate tempo mapping and beat grid subdivision.
 */
export class TempoMap {
  private readonly sampleRate: number;
  private points: TempoPoint[];

  constructor(sampleRate = 48000, initialBpm = 120, timeSignature: [number, number] = [4, 4]) {
    this.sampleRate = Math.max(1, sampleRate);
    this.points = [
      {
        samplePosition: 0,
        bpm: Math.min(280, Math.max(40, initialBpm)),
        timeSignature,
      },
    ];
  }

  public bpmAtSample(sample: number): number {
    let activeBpm = this.points[0].bpm;
    for (const p of this.points) {
      if (p.samplePosition <= sample) {
        activeBpm = p.bpm;
      } else {
        break;
      }
    }
    return activeBpm;
  }

  public setTempoPoint(
    samplePosition: number,
    bpm: number,
    timeSignature: [number, number] = [4, 4],
  ): void {
    const cleanBpm = Math.min(280, Math.max(40, bpm));
    const existingIdx = this.points.findIndex((p) => p.samplePosition === samplePosition);
    if (existingIdx >= 0) {
      this.points[existingIdx] = { samplePosition, bpm: cleanBpm, timeSignature };
    } else {
      this.points.push({ samplePosition, bpm: cleanBpm, timeSignature });
      this.points.sort((a, b) => a.samplePosition - b.samplePosition);
    }
  }

  public sampleToBeat(sample: number): number {
    let accumulatedBeats = 0;
    let prevSample = 0;
    let activeBpm = this.points[0].bpm;

    for (const p of this.points) {
      if (p.samplePosition > sample) break;
      if (p.samplePosition > prevSample) {
        const deltaSamples = p.samplePosition - prevSample;
        const samplesPerBeat = (60.0 * this.sampleRate) / activeBpm;
        accumulatedBeats += deltaSamples / samplesPerBeat;
        prevSample = p.samplePosition;
      }
      activeBpm = p.bpm;
    }

    const remainingSamples = sample - prevSample;
    const samplesPerBeat = (60.0 * this.sampleRate) / activeBpm;
    return accumulatedBeats + remainingSamples / samplesPerBeat;
  }

  public beatToSample(beat: number): number {
    let remainingBeats = Math.max(0, beat);
    let currentSample = 0;
    let activeBpm = this.points[0].bpm;

    for (let i = 0; i < this.points.length; i++) {
      const nextPoint = this.points[i + 1];
      const samplesPerBeat = (60.0 * this.sampleRate) / activeBpm;

      if (nextPoint) {
        const segSamples = nextPoint.samplePosition - currentSample;
        const segBeats = segSamples / samplesPerBeat;

        if (remainingBeats <= segBeats) {
          return Math.round(currentSample + remainingBeats * samplesPerBeat);
        }

        remainingBeats -= segBeats;
        currentSample = nextPoint.samplePosition;
        activeBpm = nextPoint.bpm;
      } else {
        return Math.round(currentSample + remainingBeats * samplesPerBeat);
      }
    }

    return currentSample;
  }

  public snapToSubdivision(sample: number, subdivisionBeats = 1.0): number {
    const sub = Math.max(0.0625, subdivisionBeats);
    const currentBeat = this.sampleToBeat(sample);
    const snappedBeat = Math.round(currentBeat / sub) * sub;
    return this.beatToSample(snappedBeat);
  }

  public generateGrid(startSample: number, endSample: number, subdivisionBeats = 1.0): number[] {
    const grid: number[] = [];
    const sub = Math.max(0.0625, subdivisionBeats);
    const startBeat = Math.floor(this.sampleToBeat(startSample) / sub) * sub;
    const endBeat = this.sampleToBeat(endSample);

    for (let b = startBeat; b <= endBeat; b += sub) {
      const s = this.beatToSample(b);
      if (s >= startSample && s <= endSample) {
        grid.push(s);
      }
    }
    return grid;
  }
}

/**
 * Tap tempo calculator tracking real-time tap events and returning smoothed BPM.
 */
export class TapTempoCalculator {
  private lastTapMs = 0;
  private intervalsMs: number[] = [];
  private readonly maxHistory: number;
  private readonly timeoutMs: number;
  private currentBpm = 120;

  constructor(maxHistory = 5, timeoutMs = 2500) {
    this.maxHistory = maxHistory;
    this.timeoutMs = timeoutMs;
  }

  public tap(timestampMs: number = Date.now()): number | null {
    if (this.lastTapMs > 0) {
      const delta = timestampMs - this.lastTapMs;
      if (delta > this.timeoutMs || delta < 150) {
        this.intervalsMs = [];
      } else {
        this.intervalsMs.push(delta);
        if (this.intervalsMs.length > this.maxHistory) {
          this.intervalsMs.shift();
        }
      }
    }
    this.lastTapMs = timestampMs;

    if (this.intervalsMs.length === 0) {
      return null;
    }

    const avgMs = this.intervalsMs.reduce((a, b) => a + b, 0) / this.intervalsMs.length;
    this.currentBpm = Math.min(280, Math.max(40, Math.round((60000.0 / avgMs) * 10) / 10));
    return this.currentBpm;
  }

  public get bpm(): number {
    return this.currentBpm;
  }

  public reset(): void {
    this.lastTapMs = 0;
    this.intervalsMs = [];
  }
}

/**
 * Utility helpers for calculating speed and pitch parameters.
 */
export function calculateVarispeedPitch(speedRatio: number): number {
  return 12.0 * Math.log2(Math.max(0.25, Math.min(4.0, speedRatio)));
}

export function calculateScaledDuration(
  originalDurationSeconds: number,
  speedRatio: number,
): number {
  return originalDurationSeconds / Math.max(0.01, speedRatio);
}

export function formatDurationMinutesSeconds(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}
