/**
 * Timeline, Waveform, Spectrogram, and Transport Viewport Models.
 */

export type WaveformScale = "linear" | "db";
export type ChannelDisplayMode = "mono" | "stereo" | "midside";
export type FrequencyScale = "linear" | "logarithmic" | "mel";
export type SpectrogramColorMap = "magma" | "viridis" | "inferno" | "theme";

export interface SpectralSelection {
  startTime: number; // in seconds
  endTime: number; // in seconds
  minFreq: number; // in Hz
  maxFreq: number; // in Hz
  active?: boolean;
}

export interface Marker {
  id: string;
  time: number; // in seconds
  label: string;
  color?: string; // CSS variable e.g. '--t1', '--t2'
}

export interface Region {
  id: string;
  startTime: number;
  endTime: number;
  label: string;
  color?: string;
}

export interface TimelineTransportState {
  playheadTime: number; // in seconds
  isPlaying: boolean;
  isScrubbing: boolean;
  scrubSpeed: number;
  loopEnabled: boolean;
  loopStartTime: number;
  loopEndTime: number;
}

/**
 * Mathematical conversions between Hertz and Mel scale.
 */
export function hzToMel(hz: number): number {
  return 2595 * Math.log10(1 + Math.max(0, hz) / 700);
}

export function melToHz(mel: number): number {
  return 700 * (Math.pow(10, Math.max(0, mel) / 2595) - 1);
}

/**
 * Mid-Side decomposition and reconstruction.
 * M = (L + R) / sqrt(2)
 * S = (L - R) / sqrt(2)
 */
export function encodeMidSide(
  left: Float32Array,
  right: Float32Array,
): { mid: Float32Array; side: Float32Array } {
  const n = Math.min(left.length, right.length);
  const mid = new Float32Array(n);
  const side = new Float32Array(n);
  const invSqrt2 = 1 / Math.SQRT2;

  for (let i = 0; i < n; i++) {
    mid[i] = (left[i] + right[i]) * invSqrt2;
    side[i] = (left[i] - right[i]) * invSqrt2;
  }
  return { mid, side };
}

export function decodeMidSide(
  mid: Float32Array,
  side: Float32Array,
): { left: Float32Array; right: Float32Array } {
  const n = Math.min(mid.length, side.length);
  const left = new Float32Array(n);
  const right = new Float32Array(n);
  const invSqrt2 = 1 / Math.SQRT2;

  for (let i = 0; i < n; i++) {
    left[i] = (mid[i] + side[i]) * invSqrt2;
    right[i] = (mid[i] - side[i]) * invSqrt2;
  }
  return { left, right };
}

/**
 * Vertical scale mapping: maps normalized amplitude in [-1, 1] to [0, 1] range
 * supporting both linear and logarithmic (dB) scale.
 */
export function mapAmplitudeToHeight(
  amplitude: number,
  scale: WaveformScale,
  minDbfs = -60.0,
): number {
  const absVal = Math.min(1.0, Math.abs(amplitude));
  if (scale === "linear") {
    return absVal;
  }
  // dB scale: 20 * log10(absVal)
  if (absVal < 1e-5) {
    return 0.0;
  }
  const db = 20 * Math.log10(absVal);
  if (db <= minDbfs) {
    return 0.0;
  }
  return Math.min(1.0, Math.max(0.0, (db - minDbfs) / -minDbfs));
}

/**
 * Formats spectral selection as human-readable badge text (e.g. "2.1 to 4.8 kHz · 1.42 s").
 */
export function formatSpectralSelection(selection: SpectralSelection): string {
  const fMin =
    selection.minFreq >= 1000
      ? (selection.minFreq / 1000).toFixed(1)
      : Math.round(selection.minFreq).toString();
  const fMax =
    selection.maxFreq >= 1000
      ? `${(selection.maxFreq / 1000).toFixed(1)} kHz`
      : `${Math.round(selection.maxFreq)} Hz`;
  const dur = Math.max(0, selection.endTime - selection.startTime).toFixed(2);
  return `${fMin} to ${fMax} · ${dur} s`;
}

/**
 * Viewport model handling sample-accurate zoom, scroll, and coordinate conversion.
 */
export class TimelineViewport {
  public scrollLeftSeconds: number;
  public pixelsPerSecond: number;
  public sampleRate: number;
  public durationSeconds: number;

  constructor(
    pixelsPerSecond = 100,
    sampleRate = 48000,
    durationSeconds = 180,
    scrollLeftSeconds = 0,
  ) {
    this.pixelsPerSecond = Math.max(1, pixelsPerSecond);
    this.sampleRate = sampleRate;
    this.durationSeconds = durationSeconds;
    this.scrollLeftSeconds = Math.max(0, scrollLeftSeconds);
  }

  public timeToPixel(time: number): number {
    return (time - this.scrollLeftSeconds) * this.pixelsPerSecond;
  }

  public pixelToTime(pixel: number): number {
    return this.scrollLeftSeconds + pixel / this.pixelsPerSecond;
  }

  public sampleToPixel(sampleIndex: number): number {
    const time = sampleIndex / this.sampleRate;
    return this.timeToPixel(time);
  }

  public pixelToSample(pixel: number): number {
    const time = this.pixelToTime(pixel);
    return Math.round(time * this.sampleRate);
  }

  /**
   * Returns whether current zoom level allows rendering individual sample dots.
   */
  public isSampleLevel(pixelsPerSampleThreshold = 4): boolean {
    const pixelsPerSample = this.pixelsPerSecond / this.sampleRate;
    return pixelsPerSample >= pixelsPerSampleThreshold;
  }

  /**
   * Zooms in centered at a specific pixel position on the canvas.
   */
  public zoomIn(factor = 1.5, centerPixel = 0): void {
    this.setZoom(this.pixelsPerSecond * factor, centerPixel);
  }

  /**
   * Zooms out centered at a specific pixel position on the canvas.
   */
  public zoomOut(factor = 1.5, centerPixel = 0): void {
    this.setZoom(this.pixelsPerSecond / factor, centerPixel);
  }

  /**
   * Sets zoom with pivot point preservation.
   */
  public setZoom(newPxPerSec: number, pivotPixel = 0): void {
    const maxPxPerSec = this.sampleRate * 32; // up to 32 pixels per sample
    const minPxPerSec = 1;
    const clampedPx = Math.max(minPxPerSec, Math.min(maxPxPerSec, newPxPerSec));

    const pivotTime = this.pixelToTime(pivotPixel);
    this.pixelsPerSecond = clampedPx;
    this.scrollLeftSeconds = Math.max(0, pivotTime - pivotPixel / this.pixelsPerSecond);
  }

  /**
   * Fits whole file within the available canvas width.
   */
  public zoomToFit(widthPixels: number): void {
    if (this.durationSeconds <= 0 || widthPixels <= 0) return;
    this.pixelsPerSecond = widthPixels / this.durationSeconds;
    this.scrollLeftSeconds = 0;
  }
}
