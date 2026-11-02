/**
 * EngineController - High-level TypeScript controller for Flux Audio Engine:
 * graph orchestration, transport controls, AudioWorklet execution, resampling, and dither.
 */

export enum TransportState {
  Stopped = "stopped",
  Playing = "playing",
  Paused = "paused",
  Scrubbing = "scrubbing",
}

export enum ResamplerQuality {
  Fast = 0,
  High = 1,
  Mastering = 2,
}

export enum DitherType {
  None = 0,
  Tpdf = 1,
  NoiseShaped = 2,
}

export interface LoopRegion {
  enabled: boolean;
  startFrame: number;
  endFrame: number;
}

export interface TransportPosition {
  state: TransportState;
  playheadFrame: number;
  timeSeconds: number;
  loop: LoopRegion;
}

export class EngineController {
  private sampleRate: number;
  private state: TransportState = TransportState.Stopped;
  private playheadFrame = 0;
  private totalTimelineFrames = 0;
  private loopRegion: LoopRegion = {
    enabled: false,
    startFrame: 0,
    endFrame: 0,
  };
  private scrubSpeed = 1.0;

  constructor(sampleRate = 48000) {
    this.sampleRate = sampleRate;
  }

  /**
   * Set total length of project timeline in sample frames.
   */
  public setTimelineLength(totalFrames: number): void {
    this.totalTimelineFrames = totalFrames;
  }

  /**
   * Play transport.
   */
  public play(): void {
    this.state = TransportState.Playing;
  }

  /**
   * Pause transport.
   */
  public pause(): void {
    this.state = TransportState.Paused;
  }

  /**
   * Stop transport and return to zero or loop start.
   */
  public stop(): void {
    this.state = TransportState.Stopped;
    this.playheadFrame = this.loopRegion.enabled ? this.loopRegion.startFrame : 0;
  }

  /**
   * Sample-accurate seek.
   */
  public seek(frame: number): void {
    if (this.totalTimelineFrames > 0) {
      this.playheadFrame = Math.max(0, Math.min(frame, this.totalTimelineFrames));
    } else {
      this.playheadFrame = Math.max(0, frame);
    }
  }

  /**
   * Configure seamless loop region.
   */
  public setLoop(enabled: boolean, startFrame: number, endFrame: number): void {
    this.loopRegion = {
      enabled,
      startFrame: Math.min(startFrame, endFrame),
      endFrame: Math.max(startFrame, endFrame),
    };
  }

  /**
   * Set variable-speed scrub playback speed (-4.0 to +4.0).
   */
  public setScrubSpeed(speed: number): void {
    this.scrubSpeed = Math.max(-4.0, Math.min(4.0, speed));
    if (Math.abs(speed) > 1e-4) {
      this.state = TransportState.Scrubbing;
    } else {
      this.state = TransportState.Paused;
    }
  }

  /**
   * Current transport position and loop status.
   */
  public getPosition(): TransportPosition {
    return {
      state: this.state,
      playheadFrame: this.playheadFrame,
      timeSeconds: this.playheadFrame / this.sampleRate,
      loop: { ...this.loopRegion },
    };
  }

  /**
   * Advance playhead by quantum frames (seamless loop wrap-around).
   */
  public advance(frames: number): { fromFrame: number; toFrame: number } {
    if (this.state !== TransportState.Playing && this.state !== TransportState.Scrubbing) {
      return { fromFrame: this.playheadFrame, toFrame: this.playheadFrame };
    }

    const start = this.playheadFrame;
    const step =
      this.state === TransportState.Scrubbing ? Math.round(frames * this.scrubSpeed) : frames;

    if (this.loopRegion.enabled && this.loopRegion.endFrame > this.loopRegion.startFrame) {
      const loopLen = this.loopRegion.endFrame - this.loopRegion.startFrame;
      let next = start + step;

      if (next >= this.loopRegion.endFrame) {
        const over = next - this.loopRegion.endFrame;
        next = this.loopRegion.startFrame + (over % loopLen);
      } else if (next < this.loopRegion.startFrame) {
        const under = this.loopRegion.startFrame - next;
        next = this.loopRegion.endFrame - (under % loopLen);
      }

      this.playheadFrame = Math.max(0, next);
    } else {
      const next = Math.max(0, start + step);
      if (this.totalTimelineFrames > 0) {
        this.playheadFrame = Math.min(next, this.totalTimelineFrames);
        if (this.playheadFrame >= this.totalTimelineFrames) {
          this.state = TransportState.Stopped;
        }
      } else {
        this.playheadFrame = next;
      }
    }

    return { fromFrame: start, toFrame: this.playheadFrame };
  }

  /**
   * Applies triangular PDF dither to a 32-bit float audio buffer for 16-bit or 24-bit export.
   */
  public static applyDither(
    samples: Float32Array,
    ditherType: DitherType,
    targetBits: 16 | 24 | 32,
  ): Float32Array {
    if (targetBits === 32 || ditherType === DitherType.None) {
      return new Float32Array(samples);
    }

    const out = new Float32Array(samples.length);
    const lsb = 1.0 / (targetBits === 16 ? 32768.0 : 8388608.0);
    const invLsb = 1.0 / lsb;

    // PRNG state
    let prng = 0x85432198;
    function nextUniform(): number {
      prng ^= prng << 13;
      prng ^= prng >>> 7;
      prng ^= prng << 17;
      return ((prng >>> 0) % 65536) / 65536.0 - 0.5;
    }

    // Lipshitz noise-shaping error feedback history
    const hist = [0, 0, 0, 0, 0];
    const coeffs = [2.033, -2.165, 1.959, -1.369, 0.468];

    for (let i = 0; i < samples.length; i++) {
      let ditherNoise = 0.0;
      if (ditherType === DitherType.Tpdf || ditherType === DitherType.NoiseShaped) {
        ditherNoise = (nextUniform() + nextUniform()) * lsb;
      }

      let shaped = samples[i];
      if (ditherType === DitherType.NoiseShaped) {
        let feedback = 0;
        for (let j = 0; j < 5; j++) {
          feedback += coeffs[j] * hist[j];
        }
        shaped += feedback;
      }

      const qLsb = Math.round((shaped + ditherNoise) * invLsb);
      const qSample = Math.max(-1.0, Math.min(1.0 - lsb, qLsb * lsb));

      const err = shaped - qSample;
      for (let j = 4; j > 0; j--) {
        hist[j] = hist[j - 1];
      }
      hist[0] = err;

      out[i] = qSample;
    }

    return out;
  }
}
