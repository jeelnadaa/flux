/**
 * EditingModel.ts - Non-destructive Multitrack Timeline & Clip Editing Engine.
 *
 * Implements sample-accurate non-destructive editing operations:
 * - Select, Cut, Copy, Paste, Delete, Ripple-Delete, Split at Playhead, Move, Nudge
 * - Slip editing (change source offset within clip boundaries)
 * - Reverse clip toggle (non-destructive playback reversal)
 * - Clip gain and fade parameters with curve selection
 * - Multitrack mixing (gain, pan, mute, solo, arm, buses)
 * - Automation lanes with Linear, Smooth (Hermite spline), and Hold interpolation.
 */

import { FadeCurveType } from "./FadeCurves.js";

export interface AutomationPoint {
  timeSeconds: number;
  value: number;
}

export type AutomationInterpolation = "linear" | "smooth" | "hold";

export interface AutomationLane {
  id: string;
  trackId: string;
  parameter: string; // e.g. "volume", "pan", "reverb_mix", "azimuth", "distance"
  interpolation: AutomationInterpolation;
  points: AutomationPoint[];
}

export interface Clip {
  id: string;
  trackId: string;
  name: string;
  sourceId: string;
  sourceDurationSeconds: number;
  sourceOffsetSeconds: number; // offset in source audio file
  startSeconds: number; // position on timeline
  durationSeconds: number; // length on timeline
  gainDb: number; // default 0.0 dB
  reversed: boolean; // default false
  fadeInDurationSeconds: number;
  fadeInCurve: FadeCurveType;
  fadeOutDurationSeconds: number;
  fadeOutCurve: FadeCurveType;
  selected?: boolean;
}

export interface Track {
  id: string;
  name: string;
  color: string; // token var e.g. "--t1", "--t4"
  gainDb: number; // track volume fader
  pan: number; // -1.0 (hard left) to +1.0 (hard right)
  muted: boolean;
  soloed: boolean;
  armed: boolean;
  tall?: boolean;
  busId?: string;
  automationLanes: AutomationLane[];
}

export interface Bus {
  id: string;
  name: string;
  gainDb: number;
  pan: number;
  muted: boolean;
  soloed: boolean;
}

export interface ProjectState {
  sampleRate: number;
  durationSeconds: number;
  tracks: Track[];
  clips: Clip[];
  buses: Bus[];
  master: {
    gainDb: number;
    pan: number;
    muted: boolean;
  };
  selection: {
    selectedClipIds: string[];
    selectedTrackId: string | null;
    rangeStart?: number;
    rangeEnd?: number;
  };
  clipboard: Clip | null;
}

export class TimelineEditor {
  private state: ProjectState;

  constructor(initialState?: Partial<ProjectState>) {
    this.state = {
      sampleRate: initialState?.sampleRate ?? 48000,
      durationSeconds: initialState?.durationSeconds ?? 240,
      tracks: initialState?.tracks ?? [],
      clips: initialState?.clips ?? [],
      buses: initialState?.buses ?? [],
      master: initialState?.master ?? { gainDb: 0, pan: 0, muted: false },
      selection: initialState?.selection ?? { selectedClipIds: [], selectedTrackId: null },
      clipboard: initialState?.clipboard ?? null,
    };
  }

  public getState(): ProjectState {
    return JSON.parse(JSON.stringify(this.state));
  }

  // ----------------------------------------------------
  // Track Management
  // ----------------------------------------------------

  public addTrack(name: string, color: string = "--t1"): Track {
    const track: Track = {
      id: `trk-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
      name,
      color,
      gainDb: 0.0,
      pan: 0.0,
      muted: false,
      soloed: false,
      armed: false,
      automationLanes: [],
    };
    this.state.tracks.push(track);
    return track;
  }

  public setTrackControls(
    trackId: string,
    controls: Partial<Pick<Track, "gainDb" | "pan" | "muted" | "soloed" | "armed" | "tall">>,
  ): void {
    const trk = this.state.tracks.find((t) => t.id === trackId);
    if (!trk) return;
    Object.assign(trk, controls);
  }

  // ----------------------------------------------------
  // Clip Editing Operations (Non-Destructive)
  // ----------------------------------------------------

  public addClip(clipData: Omit<Clip, "id">): Clip {
    const clip: Clip = {
      id: `clip-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
      ...clipData,
    };
    this.state.clips.push(clip);
    return clip;
  }

  public selectClip(clipId: string, multiSelect: boolean = false): void {
    if (!multiSelect) {
      this.state.selection.selectedClipIds = [clipId];
      this.state.clips.forEach((c) => (c.selected = c.id === clipId));
    } else {
      if (!this.state.selection.selectedClipIds.includes(clipId)) {
        this.state.selection.selectedClipIds.push(clipId);
      }
      const c = this.state.clips.find((x) => x.id === clipId);
      if (c) c.selected = true;
    }
  }

  public clearSelection(): void {
    this.state.selection.selectedClipIds = [];
    this.state.clips.forEach((c) => (c.selected = false));
  }

  /**
   * Split a clip at an exact timeline position (sample-accurate).
   * Generates two independent non-destructive clips referencing the same source.
   */
  public splitClip(clipId: string, splitTimeSeconds: number): [Clip, Clip] | null {
    const clipIdx = this.state.clips.findIndex((c) => c.id === clipId);
    if (clipIdx === -1) return null;

    const clip = this.state.clips[clipIdx];
    if (
      splitTimeSeconds <= clip.startSeconds ||
      splitTimeSeconds >= clip.startSeconds + clip.durationSeconds
    ) {
      return null;
    }

    const leftDuration = splitTimeSeconds - clip.startSeconds;
    const rightDuration = clip.durationSeconds - leftDuration;
    const rightSourceOffset = clip.sourceOffsetSeconds + (clip.reversed ? 0 : leftDuration);

    const leftClip: Clip = {
      ...clip,
      id: `clip-${Date.now()}-L`,
      durationSeconds: leftDuration,
      fadeOutDurationSeconds: Math.min(clip.fadeOutDurationSeconds, leftDuration),
    };

    const rightClip: Clip = {
      ...clip,
      id: `clip-${Date.now()}-R`,
      startSeconds: splitTimeSeconds,
      sourceOffsetSeconds: rightSourceOffset,
      durationSeconds: rightDuration,
      fadeInDurationSeconds: Math.min(clip.fadeInDurationSeconds, rightDuration),
    };

    // Replace original clip with left and right sub-clips
    this.state.clips.splice(clipIdx, 1, leftClip, rightClip);
    return [leftClip, rightClip];
  }

  /**
   * Cut clip: copies clip to clipboard and removes it from project.
   */
  public cutClip(clipId: string): Clip | null {
    const clip = this.copyClip(clipId);
    if (clip) {
      this.deleteClip(clipId, false);
    }
    return clip;
  }

  /**
   * Copy clip to internal clipboard.
   */
  public copyClip(clipId: string): Clip | null {
    const clip = this.state.clips.find((c) => c.id === clipId);
    if (!clip) return null;
    this.state.clipboard = JSON.parse(JSON.stringify(clip));
    return this.state.clipboard;
  }

  /**
   * Paste clip from clipboard onto track at given timeline position.
   */
  public pasteClip(trackId: string, timeSeconds: number): Clip | null {
    if (!this.state.clipboard) return null;
    const pasted: Clip = {
      ...this.state.clipboard,
      id: `clip-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
      trackId,
      startSeconds: timeSeconds,
      selected: true,
    };
    this.clearSelection();
    this.state.clips.push(pasted);
    this.state.selection.selectedClipIds = [pasted.id];
    return pasted;
  }

  /**
   * Delete a clip. If ripple is true, shifts all subsequent clips on the same track to the left.
   */
  public deleteClip(clipId: string, ripple: boolean = false): void {
    const clipIdx = this.state.clips.findIndex((c) => c.id === clipId);
    if (clipIdx === -1) return;

    const clip = this.state.clips[clipIdx];
    this.state.clips.splice(clipIdx, 1);
    this.state.selection.selectedClipIds = this.state.selection.selectedClipIds.filter(
      (id) => id !== clipId,
    );

    if (ripple) {
      const shift = clip.durationSeconds;
      for (const other of this.state.clips) {
        if (
          other.trackId === clip.trackId &&
          other.startSeconds >= clip.startSeconds + clip.durationSeconds
        ) {
          other.startSeconds -= shift;
        }
      }
    }
  }

  /**
   * Ripple-delete a time range on a track.
   */
  public rippleDeleteRange(trackId: string, startTime: number, endTime: number): void {
    const duration = endTime - startTime;
    if (duration <= 0) return;

    const remainingClips: Clip[] = [];

    for (const clip of this.state.clips) {
      if (clip.trackId !== trackId) {
        remainingClips.push(clip);
        continue;
      }

      const clipEnd = clip.startSeconds + clip.durationSeconds;

      // Entirely before range
      if (clipEnd <= startTime) {
        remainingClips.push(clip);
      }
      // Entirely after range -> shift left by duration
      else if (clip.startSeconds >= endTime) {
        clip.startSeconds -= duration;
        remainingClips.push(clip);
      }
      // Overlapping range: trim or split
      else {
        // Left preserved part
        if (clip.startSeconds < startTime) {
          const preservedLeft = {
            ...clip,
            durationSeconds: startTime - clip.startSeconds,
          };
          remainingClips.push(preservedLeft);
        }
        // Right preserved part
        if (clipEnd > endTime) {
          const cutInOffset = endTime - clip.startSeconds;
          const preservedRight = {
            ...clip,
            id: `clip-${Date.now()}-cut`,
            startSeconds: startTime, // rippled to range start
            sourceOffsetSeconds: clip.sourceOffsetSeconds + cutInOffset,
            durationSeconds: clipEnd - endTime,
          };
          remainingClips.push(preservedRight);
        }
      }
    }

    this.state.clips = remainingClips;
  }

  /**
   * Move clip to a new start time and optionally a new track.
   */
  public moveClip(clipId: string, newStartSeconds: number, newTrackId?: string): void {
    const clip = this.state.clips.find((c) => c.id === clipId);
    if (!clip) return;
    clip.startSeconds = Math.max(0, newStartSeconds);
    if (newTrackId) {
      clip.trackId = newTrackId;
    }
  }

  /**
   * Nudge clip by delta seconds (e.g. 1ms or 1 frame).
   */
  public nudgeClip(clipId: string, deltaSeconds: number): void {
    const clip = this.state.clips.find((c) => c.id === clipId);
    if (!clip) return;
    clip.startSeconds = Math.max(0, clip.startSeconds + deltaSeconds);
  }

  /**
   * Slip-edit: modifies source offset within clip boundaries while leaving start & duration intact.
   */
  public slipClip(clipId: string, deltaSourceOffsetSeconds: number): void {
    const clip = this.state.clips.find((c) => c.id === clipId);
    if (!clip) return;
    const maxOffset = Math.max(0, clip.sourceDurationSeconds - clip.durationSeconds);
    clip.sourceOffsetSeconds = Math.max(
      0,
      Math.min(maxOffset, clip.sourceOffsetSeconds + deltaSourceOffsetSeconds),
    );
  }

  /**
   * Toggle non-destructive clip reverse playback.
   */
  public reverseClip(clipId: string): void {
    const clip = this.state.clips.find((c) => c.id === clipId);
    if (!clip) return;
    clip.reversed = !clip.reversed;
  }

  /**
   * Set clip volume gain in dB.
   */
  public setClipGain(clipId: string, gainDb: number): void {
    const clip = this.state.clips.find((c) => c.id === clipId);
    if (!clip) return;
    clip.gainDb = gainDb;
  }

  /**
   * Configure clip fade-in and fade-out parameters.
   */
  public setClipFades(
    clipId: string,
    fades: {
      fadeInDurationSeconds?: number;
      fadeInCurve?: FadeCurveType;
      fadeOutDurationSeconds?: number;
      fadeOutCurve?: FadeCurveType;
    },
  ): void {
    const clip = this.state.clips.find((c) => c.id === clipId);
    if (!clip) return;
    if (fades.fadeInDurationSeconds !== undefined) {
      clip.fadeInDurationSeconds = Math.max(
        0,
        Math.min(clip.durationSeconds, fades.fadeInDurationSeconds),
      );
    }
    if (fades.fadeInCurve !== undefined) clip.fadeInCurve = fades.fadeInCurve;
    if (fades.fadeOutDurationSeconds !== undefined) {
      clip.fadeOutDurationSeconds = Math.max(
        0,
        Math.min(clip.durationSeconds, fades.fadeOutDurationSeconds),
      );
    }
    if (fades.fadeOutCurve !== undefined) clip.fadeOutCurve = fades.fadeOutCurve;
  }

  /**
   * Automatically crossfades two adjacent or overlapping clips with an equal-power curve.
   */
  public autoCrossfade(
    clipAId: string,
    clipBId: string,
    crossfadeDurationSec: number = 0.05,
  ): void {
    const clipA = this.state.clips.find((c) => c.id === clipAId);
    const clipB = this.state.clips.find((c) => c.id === clipBId);
    if (!clipA || !clipB || clipA.trackId !== clipB.trackId) return;

    clipA.fadeOutDurationSeconds = crossfadeDurationSec;
    clipA.fadeOutCurve = "equalPower";
    clipB.fadeInDurationSeconds = crossfadeDurationSec;
    clipB.fadeInCurve = "equalPower";
  }

  // ----------------------------------------------------
  // Automation Lanes & Curves
  // ----------------------------------------------------

  public addAutomationLane(
    trackId: string,
    parameter: string,
    interpolation: AutomationInterpolation = "linear",
  ): AutomationLane {
    const trk = this.state.tracks.find((t) => t.id === trackId);
    if (!trk) throw new Error(`Track ${trackId} not found`);

    const lane: AutomationLane = {
      id: `lane-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
      trackId,
      parameter,
      interpolation,
      points: [],
    };
    trk.automationLanes.push(lane);
    return lane;
  }

  public addAutomationPoint(laneId: string, timeSeconds: number, value: number): void {
    for (const trk of this.state.tracks) {
      const lane = trk.automationLanes.find((l) => l.id === laneId);
      if (lane) {
        lane.points.push({ timeSeconds, value });
        lane.points.sort((a, b) => a.timeSeconds - b.timeSeconds);
        return;
      }
    }
  }

  public removeAutomationPoint(laneId: string, pointIndex: number): void {
    for (const trk of this.state.tracks) {
      const lane = trk.automationLanes.find((l) => l.id === laneId);
      if (lane && pointIndex >= 0 && pointIndex < lane.points.length) {
        lane.points.splice(pointIndex, 1);
        return;
      }
    }
  }

  /**
   * Evaluates automation curve at given timeline time.
   */
  public evaluateAutomation(
    lane: AutomationLane,
    timeSeconds: number,
    defaultValue: number = 0.0,
  ): number {
    const pts = lane.points;
    if (pts.length === 0) return defaultValue;
    if (timeSeconds <= pts[0].timeSeconds) return pts[0].value;
    if (timeSeconds >= pts[pts.length - 1].timeSeconds) return pts[pts.length - 1].value;

    // Binary search for surrounding points
    let low = 0;
    let high = pts.length - 1;
    while (low <= high) {
      const mid = (low + high) >> 1;
      if (pts[mid].timeSeconds <= timeSeconds) {
        low = mid + 1;
      } else {
        high = mid - 1;
      }
    }

    const p0 = pts[high];
    const p1 = pts[low];
    const dt = p1.timeSeconds - p0.timeSeconds;
    if (dt <= 1e-6) return p0.value;

    const t = (timeSeconds - p0.timeSeconds) / dt;

    switch (lane.interpolation) {
      case "hold":
        return p0.value;

      case "linear":
        return p0.value + t * (p1.value - p0.value);

      case "smooth": {
        // Hermite smoothstep spline: 3t^2 - 2t^3
        const smoothT = t * t * (3.0 - 2.0 * t);
        return p0.value + smoothT * (p1.value - p0.value);
      }

      default:
        return p0.value + t * (p1.value - p0.value);
    }
  }
}
