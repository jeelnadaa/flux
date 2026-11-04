/**
 * Reactive Project Store & Real-time Web Audio Engine for Flux.
 * Manages multitrack timeline data, non-destructive editing, sample-accurate transport,
 * file decoding, and mixdown rendering.
 */

import { EngineController, TransportState } from "../engine/EngineController.js";
import { AnalysisManager } from "../analysis/AnalysisManager.js";
import { NativeBridge } from "../platform/NativeBridge.js";

export interface AudioClip {
  id: string;
  name: string;
  trackId: string;
  startFrame: number;
  durationFrames: number;
  colorVar: string;
  channels: Float32Array[];
  fadeInFrames: number;
  fadeOutFrames: number;
  gain: number;
  seed: number;
  kind: string;
}

export interface AudioTrack {
  id: string;
  name: string;
  colorVar: string;
  volume: number; // 0.0 to 1.5 (1.0 = 0dB)
  pan: number; // -1.0 (left) to +1.0 (right)
  muted: boolean;
  solo: boolean;
  tall?: boolean;
  clips: AudioClip[];
}

export interface FluxProjectState {
  name: string;
  sampleRate: number;
  tracks: AudioTrack[];
  totalFrames: number;
  playheadFrame: number;
  transportState: TransportState;
  loopEnabled: boolean;
  loopStartFrame: number;
  loopEndFrame: number;
  selectedClipId: string | null;
  selectedTrackId: string | null;
  meterLevels: { leftPeak: number; rightPeak: number; leftRms: number; rightRms: number };
}

export type StateListener = (state: FluxProjectState) => void;

/**
 * Generates synthetic multi-track audio for initial project demo ("Harbor Lights").
 */
function createDemoTracks(sampleRate: number): AudioTrack[] {
  const durationSec = 16.0;
  const totalFrames = Math.floor(sampleRate * durationSec);

  // Track 1: Vocals / Lead Melody
  const vocalFrames = totalFrames;
  const vocalLeft = new Float32Array(vocalFrames);
  const vocalRight = new Float32Array(vocalFrames);
  const vocalFreqs = [440, 493.88, 523.25, 587.33, 659.25, 587.33, 523.25, 493.88];
  for (let i = 0; i < vocalFrames; i++) {
    const t = i / sampleRate;
    const noteIdx = Math.floor((t * 2) % vocalFreqs.length);
    const freq = vocalFreqs[noteIdx];
    const env = 0.5 * (1 - Math.cos((2 * Math.PI * (t % 0.5)) / 0.5));
    const s = Math.sin(2 * Math.PI * freq * t) * 0.25 * env;
    vocalLeft[i] = s;
    vocalRight[i] = s;
  }

  // Track 2: Drums / Rhythm
  const drumFrames = totalFrames;
  const drumLeft = new Float32Array(drumFrames);
  const drumRight = new Float32Array(drumFrames);
  for (let i = 0; i < drumFrames; i++) {
    const t = i / sampleRate;
    const beatPhase = (t * 2) % 1.0;
    // Kick on beat
    let kick = 0;
    if (beatPhase < 0.15) {
      const kf = 120 * Math.exp(-beatPhase * 25);
      kick = Math.sin(2 * Math.PI * kf * beatPhase) * 0.45 * (1 - beatPhase / 0.15);
    }
    // Snare on off-beat
    let snare = 0;
    const snarePhase = (t * 2 + 0.5) % 1.0;
    if (snarePhase < 0.12) {
      const noise = (Math.random() * 2 - 1) * 0.25;
      snare = (Math.sin(2 * Math.PI * 220 * snarePhase) * 0.2 + noise) * (1 - snarePhase / 0.12);
    }
    const s = kick + snare;
    drumLeft[i] = s;
    drumRight[i] = s;
  }

  // Track 3: Bass
  const bassFrames = totalFrames;
  const bassLeft = new Float32Array(bassFrames);
  const bassRight = new Float32Array(bassFrames);
  const bassFreqs = [110, 110, 130.81, 146.83, 164.81, 146.83, 130.81, 110];
  for (let i = 0; i < bassFrames; i++) {
    const t = i / sampleRate;
    const noteIdx = Math.floor((t * 2) % bassFreqs.length);
    const freq = bassFreqs[noteIdx];
    const s = Math.sin(2 * Math.PI * freq * t) * 0.35 + Math.sin(2 * Math.PI * freq * 2 * t) * 0.1;
    bassLeft[i] = s;
    bassRight[i] = s;
  }

  // Track 4: Keys / Ambient Pad
  const padFrames = totalFrames;
  const padLeft = new Float32Array(padFrames);
  const padRight = new Float32Array(padFrames);
  for (let i = 0; i < padFrames; i++) {
    const t = i / sampleRate;
    const s1 = Math.sin(2 * Math.PI * 220 * t) * 0.12;
    const s2 = Math.sin(2 * Math.PI * 277.18 * t) * 0.12;
    const s3 = Math.sin(2 * Math.PI * 329.63 * t) * 0.12;
    padLeft[i] = s1 + s2;
    padRight[i] = s2 + s3;
  }

  return [
    {
      id: "track-vocals",
      name: "Vocals",
      colorVar: "--t4",
      volume: 1.0,
      pan: 0.0,
      muted: false,
      solo: false,
      tall: true,
      clips: [
        {
          id: "clip-v1",
          name: "Verse 1",
          trackId: "track-vocals",
          startFrame: Math.floor(sampleRate * 0.5),
          durationFrames: Math.floor(sampleRate * 4.5),
          colorVar: "--t4",
          channels: [
            vocalLeft.slice(0, Math.floor(sampleRate * 4.5)),
            vocalRight.slice(0, Math.floor(sampleRate * 4.5)),
          ],
          fadeInFrames: Math.floor(sampleRate * 0.05),
          fadeOutFrames: Math.floor(sampleRate * 0.05),
          gain: 1.0,
          seed: 7,
          kind: "voice",
        },
        {
          id: "clip-v2",
          name: "Chorus",
          trackId: "track-vocals",
          startFrame: Math.floor(sampleRate * 5.5),
          durationFrames: Math.floor(sampleRate * 4.0),
          colorVar: "--t4",
          channels: [
            vocalLeft.slice(0, Math.floor(sampleRate * 4.0)),
            vocalRight.slice(0, Math.floor(sampleRate * 4.0)),
          ],
          fadeInFrames: Math.floor(sampleRate * 0.05),
          fadeOutFrames: Math.floor(sampleRate * 0.05),
          gain: 1.0,
          seed: 8,
          kind: "voice",
        },
        {
          id: "clip-v3",
          name: "Verse 2",
          trackId: "track-vocals",
          startFrame: Math.floor(sampleRate * 10.0),
          durationFrames: Math.floor(sampleRate * 5.0),
          colorVar: "--t4",
          channels: [
            vocalLeft.slice(0, Math.floor(sampleRate * 5.0)),
            vocalRight.slice(0, Math.floor(sampleRate * 5.0)),
          ],
          fadeInFrames: Math.floor(sampleRate * 0.05),
          fadeOutFrames: Math.floor(sampleRate * 0.05),
          gain: 1.0,
          seed: 9,
          kind: "voice",
        },
      ],
    },
    {
      id: "track-drums",
      name: "Drums",
      colorVar: "--t3",
      volume: 0.9,
      pan: 0.0,
      muted: false,
      solo: false,
      clips: [
        {
          id: "clip-d1",
          name: "Drum loop",
          trackId: "track-drums",
          startFrame: 0,
          durationFrames: totalFrames,
          colorVar: "--t3",
          channels: [drumLeft, drumRight],
          fadeInFrames: 0,
          fadeOutFrames: 0,
          gain: 1.0,
          seed: 3,
          kind: "drums",
        },
      ],
    },
    {
      id: "track-bass",
      name: "Bass",
      colorVar: "--t2",
      volume: 0.85,
      pan: 0.0,
      muted: false,
      solo: false,
      clips: [
        {
          id: "clip-b1",
          name: "Bass stem",
          trackId: "track-bass",
          startFrame: Math.floor(sampleRate * 1.0),
          durationFrames: totalFrames - Math.floor(sampleRate * 1.0),
          colorVar: "--t2",
          channels: [
            bassLeft.slice(0, totalFrames - Math.floor(sampleRate * 1.0)),
            bassRight.slice(0, totalFrames - Math.floor(sampleRate * 1.0)),
          ],
          fadeInFrames: Math.floor(sampleRate * 0.02),
          fadeOutFrames: Math.floor(sampleRate * 0.02),
          gain: 1.0,
          seed: 4,
          kind: "bass",
        },
      ],
    },
    {
      id: "track-keys",
      name: "Keys",
      colorVar: "--t1",
      volume: 0.75,
      pan: 0.0,
      muted: false,
      solo: false,
      clips: [
        {
          id: "clip-k1",
          name: "Pad",
          trackId: "track-keys",
          startFrame: 0,
          durationFrames: Math.floor(sampleRate * 8.0),
          colorVar: "--t1",
          channels: [
            padLeft.slice(0, Math.floor(sampleRate * 8.0)),
            padRight.slice(0, Math.floor(sampleRate * 8.0)),
          ],
          fadeInFrames: Math.floor(sampleRate * 0.1),
          fadeOutFrames: Math.floor(sampleRate * 0.1),
          gain: 1.0,
          seed: 5,
          kind: "pad",
        },
      ],
    },
  ];
}

export class ProjectStore {
  private static instance: ProjectStore | null = null;

  public static get(): ProjectStore {
    if (!ProjectStore.instance) {
      ProjectStore.instance = new ProjectStore();
    }
    return ProjectStore.instance;
  }

  public static getInstance(): ProjectStore {
    return ProjectStore.get();
  }

  private state: FluxProjectState;
  private listeners = new Set<StateListener>();
  private audioCtx: AudioContext | null = null;
  private activeSources: AudioBufferSourceNode[] = [];
  private playbackStartTime = 0;
  private playbackStartFrame = 0;
  private animFrameId: number | null = null;
  private undoStack: FluxProjectState[] = [];
  private redoStack: FluxProjectState[] = [];
  private readonly maxHistory = 50;
  private engineController: EngineController;
  public readonly analysis: AnalysisManager;

  constructor() {
    const sr = 48000;
    const demoTracks = createDemoTracks(sr);
    let maxFrames = 0;
    for (const t of demoTracks) {
      for (const c of t.clips) {
        maxFrames = Math.max(maxFrames, c.startFrame + c.durationFrames);
      }
    }

    this.engineController = new EngineController(sr);
    this.engineController.setTimelineLength(maxFrames);
    this.analysis = new AnalysisManager();

    this.state = {
      name: "Harbor Lights",
      sampleRate: sr,
      tracks: demoTracks,
      totalFrames: maxFrames,
      playheadFrame: 0,
      transportState: TransportState.Stopped,
      loopEnabled: false,
      loopStartFrame: 0,
      loopEndFrame: maxFrames,
      selectedClipId: "clip-v2",
      selectedTrackId: "track-vocals",
      meterLevels: { leftPeak: -60, rightPeak: -60, leftRms: -60, rightRms: -60 },
    };
  }

  public getState(): FluxProjectState {
    return this.state;
  }

  public subscribe(fn: StateListener): () => void {
    this.listeners.add(fn);
    fn(this.state);
    return () => this.listeners.delete(fn);
  }

  private notify(): void {
    for (const fn of this.listeners) {
      fn(this.state);
    }
  }

  private ensureAudioContext(): AudioContext {
    if (!this.audioCtx) {
      const AudioContextClass =
        window.AudioContext ||
        (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
      this.audioCtx = new AudioContextClass({ sampleRate: this.state.sampleRate });
    }
    if (this.audioCtx.state === "suspended") {
      void this.audioCtx.resume();
    }
    return this.audioCtx;
  }

  // ==========================================
  // Transport Operations
  // ==========================================

  public async play(): Promise<void> {
    if (this.state.transportState === TransportState.Playing) return;

    const ctx = this.ensureAudioContext();
    if (ctx.state === "suspended") {
      await ctx.resume();
    }

    this.stopAudioNodes();

    const startFrame = this.state.playheadFrame;
    if (startFrame >= this.state.totalFrames) {
      this.state.playheadFrame = 0;
    }

    const currentFrame = this.state.playheadFrame;
    this.playbackStartTime = ctx.currentTime;
    this.playbackStartFrame = currentFrame;

    // Check solo states
    const hasSolo = this.state.tracks.some((t) => t.solo);

    for (const track of this.state.tracks) {
      if (track.muted || (hasSolo && !track.solo)) continue;

      for (const clip of track.clips) {
        const clipEnd = clip.startFrame + clip.durationFrames;
        if (currentFrame >= clipEnd) continue;

        const offsetInClipFrames = Math.max(0, currentFrame - clip.startFrame);
        const remainingClipFrames = clip.durationFrames - offsetInClipFrames;
        if (remainingClipFrames <= 0) continue;

        // Build Web Audio Buffer
        const numCh = clip.channels.length;
        const ab = ctx.createBuffer(numCh, remainingClipFrames, this.state.sampleRate);
        for (let ch = 0; ch < numCh; ch++) {
          const chData = ab.getChannelData(ch);
          const srcData = clip.channels[ch];
          for (let i = 0; i < remainingClipFrames; i++) {
            chData[i] = srcData[offsetInClipFrames + i] * clip.gain;
          }
        }

        const srcNode = ctx.createBufferSource();
        srcNode.buffer = ab;

        // Gain node for track volume & pan
        const gainNode = ctx.createGain();
        gainNode.gain.value = track.volume;

        const panner = ctx.createStereoPanner ? ctx.createStereoPanner() : null;
        if (panner) {
          panner.pan.value = track.pan;
          srcNode.connect(gainNode);
          gainNode.connect(panner);
          panner.connect(ctx.destination);
        } else {
          srcNode.connect(gainNode);
          gainNode.connect(ctx.destination);
        }

        const when =
          currentFrame < clip.startFrame
            ? ctx.currentTime + (clip.startFrame - currentFrame) / this.state.sampleRate
            : ctx.currentTime;

        srcNode.start(when);
        this.activeSources.push(srcNode);
      }
    }

    this.state.transportState = TransportState.Playing;
    this.engineController.play();
    this.startPlayheadRaf();
    this.notify();
  }

  public pause(): void {
    if (this.state.transportState !== TransportState.Playing) return;

    this.stopAudioNodes();
    this.cancelPlayheadRaf();
    this.state.transportState = TransportState.Paused;
    this.engineController.pause();
    this.notify();
  }

  public stop(): void {
    this.stopAudioNodes();
    this.cancelPlayheadRaf();
    this.state.transportState = TransportState.Stopped;
    this.state.playheadFrame = this.state.loopEnabled ? this.state.loopStartFrame : 0;
    this.engineController.stop();
    this.state.meterLevels = { leftPeak: -60, rightPeak: -60, leftRms: -60, rightRms: -60 };
    this.notify();
  }

  public seek(frame: number): void {
    const wasPlaying = this.state.transportState === TransportState.Playing;
    if (wasPlaying) {
      this.pause();
    }
    this.state.playheadFrame = Math.max(0, Math.min(frame, this.state.totalFrames));
    this.engineController.seek(this.state.playheadFrame);
    this.notify();

    if (wasPlaying) {
      void this.play();
    }
  }

  public togglePlay(): void {
    if (this.state.transportState === TransportState.Playing) {
      this.pause();
    } else {
      void this.play();
    }
  }

  public setLoop(enabled: boolean, startFrame?: number, endFrame?: number): void {
    this.state.loopEnabled = enabled;
    if (startFrame !== undefined) this.state.loopStartFrame = startFrame;
    if (endFrame !== undefined) this.state.loopEndFrame = endFrame;
    this.engineController.setLoop(enabled, this.state.loopStartFrame, this.state.loopEndFrame);
    this.notify();
  }

  private stopAudioNodes(): void {
    for (const src of this.activeSources) {
      try {
        src.stop();
        src.disconnect();
      } catch {
        // Ignored
      }
    }
    this.activeSources = [];
  }

  private startPlayheadRaf(): void {
    this.cancelPlayheadRaf();

    const tick = () => {
      if (this.state.transportState !== TransportState.Playing) return;

      if (this.audioCtx) {
        const elapsed = this.audioCtx.currentTime - this.playbackStartTime;
        const currentFrame = this.playbackStartFrame + Math.floor(elapsed * this.state.sampleRate);

        if (this.state.loopEnabled && currentFrame >= this.state.loopEndFrame) {
          this.seek(this.state.loopStartFrame);
          return;
        } else if (currentFrame >= this.state.totalFrames) {
          this.stop();
          return;
        }

        this.state.playheadFrame = currentFrame;

        // Compute simulated VU meter values from active audio
        const peak = Math.min(0, -6 + Math.sin(elapsed * 8) * 4);
        this.state.meterLevels = {
          leftPeak: peak,
          rightPeak: peak - 1.2,
          leftRms: peak - 8,
          rightRms: peak - 9,
        };

        this.notify();
      }

      this.animFrameId = requestAnimationFrame(tick);
    };

    this.animFrameId = requestAnimationFrame(tick);
  }

  private cancelPlayheadRaf(): void {
    if (this.animFrameId !== null) {
      cancelAnimationFrame(this.animFrameId);
      this.animFrameId = null;
    }
  }

  // ==========================================
  // Undo / Redo & State History
  // ==========================================

  private cloneStateSnapshot(st: FluxProjectState): FluxProjectState {
    return {
      ...st,
      tracks: st.tracks.map((t) => ({
        ...t,
        clips: t.clips.map((c) => ({
          ...c,
          channels: [...c.channels],
        })),
      })),
      meterLevels: { ...st.meterLevels },
    };
  }

  public pushUndo(): void {
    this.undoStack.push(this.cloneStateSnapshot(this.state));
    if (this.undoStack.length > this.maxHistory) {
      this.undoStack.shift();
    }
    this.redoStack = [];
  }

  public undo(): boolean {
    if (this.undoStack.length === 0) return false;
    this.redoStack.push(this.cloneStateSnapshot(this.state));
    const prev = this.undoStack.pop()!;
    this.state = prev;
    this.recalculateTotalFrames();
    this.notify();
    return true;
  }

  public redo(): boolean {
    if (this.redoStack.length === 0) return false;
    this.undoStack.push(this.cloneStateSnapshot(this.state));
    const next = this.redoStack.pop()!;
    this.state = next;
    this.recalculateTotalFrames();
    this.notify();
    return true;
  }

  public canUndo(): boolean {
    return this.undoStack.length > 0;
  }

  public canRedo(): boolean {
    return this.redoStack.length > 0;
  }

  // ==========================================
  // Track and Clip Editing Operations
  // ==========================================

  public selectClip(clipId: string | null): void {
    this.state.selectedClipId = clipId;
    this.notify();
  }

  public duplicateSelectedClip(): AudioClip | null {
    if (!this.state.selectedClipId) return null;
    this.pushUndo();
    for (const track of this.state.tracks) {
      const clip = track.clips.find((c) => c.id === this.state.selectedClipId);
      if (clip) {
        const dup: AudioClip = {
          ...clip,
          id: `clip-${Date.now()}`,
          name: `${clip.name} (Copy)`,
          startFrame: clip.startFrame + clip.durationFrames,
          channels: [...clip.channels],
        };
        track.clips.push(dup);
        this.state.selectedClipId = dup.id;
        this.recalculateTotalFrames();
        this.notify();
        return dup;
      }
    }
    return null;
  }

  public moveClip(clipId: string, newStartFrame: number, targetTrackId?: string): void {
    this.pushUndo();
    let foundClip: AudioClip | null = null;
    for (const track of this.state.tracks) {
      const idx = track.clips.findIndex((c) => c.id === clipId);
      if (idx !== -1) {
        foundClip = track.clips.splice(idx, 1)[0];
        break;
      }
    }
    if (!foundClip) return;

    foundClip.startFrame = Math.max(0, newStartFrame);
    const destTrackId = targetTrackId || foundClip.trackId;
    const destTrack = this.state.tracks.find((t) => t.id === destTrackId) || this.state.tracks[0];
    foundClip.trackId = destTrack.id;
    destTrack.clips.push(foundClip);
    this.recalculateTotalFrames();
    this.notify();
  }

  public splitSelectedClip(): void {
    if (!this.state.selectedClipId) return;

    for (const track of this.state.tracks) {
      const clipIdx = track.clips.findIndex((c) => c.id === this.state.selectedClipId);
      if (clipIdx === -1) continue;

      const clip = track.clips[clipIdx];
      const ph = this.state.playheadFrame;
      if (ph <= clip.startFrame || ph >= clip.startFrame + clip.durationFrames) return;

      this.pushUndo();
      const splitOffset = ph - clip.startFrame;

      // First part
      const leftPart: AudioClip = {
        ...clip,
        id: `${clip.id}-a`,
        name: `${clip.name} (Part 1)`,
        durationFrames: splitOffset,
        channels: clip.channels.map((ch) => ch.slice(0, splitOffset)),
      };

      // Second part
      const rightPart: AudioClip = {
        ...clip,
        id: `${clip.id}-b`,
        name: `${clip.name} (Part 2)`,
        startFrame: ph,
        durationFrames: clip.durationFrames - splitOffset,
        channels: clip.channels.map((ch) => ch.slice(splitOffset)),
      };

      track.clips.splice(clipIdx, 1, leftPart, rightPart);
      this.state.selectedClipId = rightPart.id;
      this.notify();
      return;
    }
  }

  public deleteSelectedClip(): void {
    if (!this.state.selectedClipId) return;

    for (const track of this.state.tracks) {
      const clipIdx = track.clips.findIndex((c) => c.id === this.state.selectedClipId);
      if (clipIdx !== -1) {
        this.pushUndo();
        track.clips.splice(clipIdx, 1);
        this.state.selectedClipId = null;
        this.recalculateTotalFrames();
        this.notify();
        return;
      }
    }
  }

  public togglePlayback(): void {
    this.togglePlay();
  }

  public toggleTrackMute(trackId: string): void {
    const t = this.state.tracks.find((trk) => trk.id === trackId);
    if (t) {
      this.setTrackMute(trackId, !t.muted);
    }
  }

  public toggleTrackSolo(trackId: string): void {
    const t = this.state.tracks.find((trk) => trk.id === trackId);
    if (t) {
      this.setTrackSolo(trackId, !t.solo);
    }
  }

  public formatTimecode(frame: number): string {
    const totalSeconds = Math.max(0, frame / this.state.sampleRate);
    const mins = Math.floor(totalSeconds / 60);
    const secs = Math.floor(totalSeconds % 60);
    const ms = Math.floor((totalSeconds % 1) * 1000);
    return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}.${ms.toString().padStart(3, "0")}`;
  }

  public setTrackMute(trackId: string, muted: boolean): void {
    const t = this.state.tracks.find((trk) => trk.id === trackId);
    if (t) {
      t.muted = muted;
      this.notify();
    }
  }

  public setTrackSolo(trackId: string, solo: boolean): void {
    const t = this.state.tracks.find((trk) => trk.id === trackId);
    if (t) {
      t.solo = solo;
      this.notify();
    }
  }

  public setTrackVolume(trackId: string, vol: number): void {
    const t = this.state.tracks.find((trk) => trk.id === trackId);
    if (t) {
      t.volume = Math.max(0, Math.min(2, vol));
      this.notify();
    }
  }

  public setTrackPan(trackId: string, pan: number): void {
    const t = this.state.tracks.find((trk) => trk.id === trackId);
    if (t) {
      t.pan = Math.max(-1, Math.min(1, pan));
      this.notify();
    }
  }

  // ==========================================
  // File Ingestion (Audio & Video)
  // ==========================================

  public async importAudioFile(file: File | Blob, trackName?: string): Promise<AudioTrack> {
    const arrayBuf = await file.arrayBuffer();
    let channels: Float32Array[] = [];
    let frames = 0;

    // Fast-path: Native Rust Symphonia decoder when running on desktop
    const nativeDecoded = await NativeBridge.decodeAudio(arrayBuf);
    if (nativeDecoded && nativeDecoded.channels.length > 0) {
      channels = nativeDecoded.channels;
      frames = nativeDecoded.totalFrames;
    } else {
      // Web browser fallback: Web Audio API decodeAudioData
      const ctx = this.ensureAudioContext();
      const decodedBuf = await ctx.decodeAudioData(arrayBuf);
      const numCh = decodedBuf.numberOfChannels;
      frames = decodedBuf.length;
      for (let c = 0; c < numCh; c++) {
        channels.push(new Float32Array(decodedBuf.getChannelData(c)));
      }
    }

    const tColors = ["--t1", "--t2", "--t3", "--t4"];
    const chosenColor = tColors[this.state.tracks.length % tColors.length];
    const name =
      trackName || (file as File).name || `Imported Track ${this.state.tracks.length + 1}`;

    const newClip: AudioClip = {
      id: `clip-${Date.now()}`,
      name: name.replace(/\.[^/.]+$/, ""),
      trackId: `track-${Date.now()}`,
      startFrame: 0,
      durationFrames: frames,
      colorVar: chosenColor,
      channels,
      fadeInFrames: 0,
      fadeOutFrames: 0,
      gain: 1.0,
      seed: Math.floor(Math.random() * 20),
      kind: "voice",
    };

    const newTrack: AudioTrack = {
      id: newClip.trackId,
      name,
      colorVar: chosenColor,
      volume: 1.0,
      pan: 0.0,
      muted: false,
      solo: false,
      tall: true,
      clips: [newClip],
    };

    this.pushUndo();
    this.state.tracks.push(newTrack);
    this.recalculateTotalFrames();
    this.state.selectedClipId = newClip.id;
    this.state.selectedTrackId = newTrack.id;
    this.notify();

    return newTrack;
  }

  private recalculateTotalFrames(): void {
    let max = 0;
    for (const t of this.state.tracks) {
      for (const c of t.clips) {
        max = Math.max(max, c.startFrame + c.durationFrames);
      }
    }
    this.state.totalFrames = Math.max(max, this.state.sampleRate * 2);
    this.state.loopEndFrame = this.state.totalFrames;
    this.engineController.setTimelineLength(this.state.totalFrames);
  }

  // ==========================================
  // Mixdown & Render for Export
  // ==========================================

  public renderMixdown(
    startFrame = 0,
    endFrame = this.state.totalFrames,
  ): { interleaved: Float32Array; numChannels: number; sampleRate: number } {
    const len = Math.max(0, endFrame - startFrame);
    const left = new Float32Array(len);
    const right = new Float32Array(len);

    const hasSolo = this.state.tracks.some((t) => t.solo);

    for (const track of this.state.tracks) {
      if (track.muted || (hasSolo && !track.solo)) continue;

      const pan = track.pan;
      const leftGain = track.volume * Math.cos(((pan + 1) * Math.PI) / 4);
      const rightGain = track.volume * Math.sin(((pan + 1) * Math.PI) / 4);

      for (const clip of track.clips) {
        const clipStart = clip.startFrame;
        const clipEnd = clip.startFrame + clip.durationFrames;

        const overlapStart = Math.max(startFrame, clipStart);
        const overlapEnd = Math.min(endFrame, clipEnd);

        if (overlapStart >= overlapEnd) continue;

        const numCh = clip.channels.length;
        const clipOffset = overlapStart - clipStart;
        const mixOffset = overlapStart - startFrame;
        const copyLen = overlapEnd - overlapStart;

        for (let i = 0; i < copyLen; i++) {
          const sL = clip.channels[0][clipOffset + i] * clip.gain;
          const sR = (numCh > 1 ? clip.channels[1][clipOffset + i] : sL) * clip.gain;

          left[mixOffset + i] += sL * leftGain;
          right[mixOffset + i] += sR * rightGain;
        }
      }
    }

    // Interleave to L, R, L, R...
    const interleaved = new Float32Array(len * 2);
    for (let i = 0; i < len; i++) {
      interleaved[i * 2] = Math.max(-1.0, Math.min(1.0, left[i]));
      interleaved[i * 2 + 1] = Math.max(-1.0, Math.min(1.0, right[i]));
    }

    return {
      interleaved,
      numChannels: 2,
      sampleRate: this.state.sampleRate,
    };
  }
}
