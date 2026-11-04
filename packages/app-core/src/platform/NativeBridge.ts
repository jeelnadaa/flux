/**
 * NativeBridge - High-performance bridge between Flux frontend and native Rust engine.
 * Automatically detects whether running inside the native desktop app (Windows, macOS, Linux)
 * or a standard web browser, routing decoding, DSP, and export to the native engine when present.
 */

import type { ExportOptions } from "../export/ExportManager.js";

export interface NativeEngineStatus {
  status: string;
  backend: string;
  version: string;
  sampleRate: number;
  platform: string;
  arch: string;
  simd: boolean;
}

export interface DecodedAudioResult {
  channels: Float32Array[];
  sampleRate: number;
  totalFrames: number;
}

export interface NativeLoudnessResult {
  integratedLufs: number;
  momentaryMaxLufs: number;
  shortTermMaxLufs: number;
  loudnessRangeLu: number;
  maxTruePeakDbtp: number;
}

export class NativeBridge {
  private static cachedStatus: NativeEngineStatus | null = null;
  private static checkAttempted = false;

  /**
   * Probes whether the native Rust engine backend is reachable on loopback.
   */
  public static async isNativeAvailable(): Promise<boolean> {
    if (this.cachedStatus !== null) {
      return true;
    }
    if (this.checkAttempted && this.cachedStatus === null) {
      return false;
    }

    this.checkAttempted = true;
    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 600);

      const res = await fetch("/api/status", {
        method: "GET",
        headers: { Accept: "application/json" },
        signal: controller.signal,
      });
      clearTimeout(timeoutId);

      if (res.ok) {
        const json = (await res.json()) as NativeEngineStatus;
        if (json.backend === "flux-native") {
          this.cachedStatus = json;
          return true;
        }
      }
    } catch {
      // In web browser without native backend running
    }
    return false;
  }

  /**
   * Returns cached native engine details or null.
   */
  public static getStatus(): NativeEngineStatus | null {
    return this.cachedStatus;
  }

  /**
   * Decodes any audio container (WAV, FLAC, MP3, AAC, Vorbis, Opus, AIFF) using native Symphonia.
   * Returns binary audio channel buffers with zero string serialization overhead.
   */
  public static async decodeAudio(data: ArrayBuffer): Promise<DecodedAudioResult | null> {
    const available = await this.isNativeAvailable();
    if (!available) return null;

    try {
      const res = await fetch("/api/decode", {
        method: "POST",
        headers: {
          "Content-Type": "application/octet-stream",
          Accept: "application/octet-stream",
        },
        body: data,
      });

      if (!res.ok) {
        console.warn("Native decode failed with status:", res.status);
        return null;
      }

      const buf = await res.arrayBuffer();
      if (buf.byteLength < 16) return null;

      const view = new DataView(buf);
      const sampleRate = view.getUint32(0, true);
      const numChannels = view.getUint32(4, true);
      const totalFrames = Number(view.getBigUint64(8, true));

      const channels: Float32Array[] = [];
      let offset = 16;
      for (let ch = 0; ch < numChannels; ch++) {
        const chSlice = buf.slice(offset, offset + totalFrames * 4);
        channels.push(new Float32Array(chSlice));
        offset += totalFrames * 4;
      }

      return {
        channels,
        sampleRate,
        totalFrames,
      };
    } catch (e) {
      console.warn("Native decode exception, falling back:", e);
      return null;
    }
  }

  /**
   * Encodes interleaved 32-bit float audio using native Rust SIMD encoders.
   */
  public static async exportAudio(
    samples: Float32Array,
    numChannels: number,
    sampleRate: number,
    options: ExportOptions = {},
  ): Promise<Uint8Array | null> {
    const available = await this.isNativeAvailable();
    if (!available) return null;

    try {
      const query = new URLSearchParams({
        format: options.format || "wav_16",
        sr: String(sampleRate),
        ch: String(numChannels),
        dither: options.dither || "none",
        norm: options.normalization || "off",
      });

      const res = await fetch(`/api/export?${query.toString()}`, {
        method: "POST",
        headers: {
          "Content-Type": "application/octet-stream",
        },
        body: samples.buffer as ArrayBuffer,
      });

      if (!res.ok) {
        console.warn("Native export failed with status:", res.status);
        return null;
      }

      const encodedBuffer = await res.arrayBuffer();
      return new Uint8Array(encodedBuffer);
    } catch (e) {
      console.warn("Native export exception, falling back:", e);
      return null;
    }
  }

  /**
   * Computes ITU-R BS.1770-4 loudness metrics using native DSP.
   */
  public static async measureLoudness(
    samples: Float32Array,
    numChannels: number,
    sampleRate: number,
  ): Promise<NativeLoudnessResult | null> {
    const available = await this.isNativeAvailable();
    if (!available) return null;

    try {
      const query = new URLSearchParams({
        sr: String(sampleRate),
        ch: String(numChannels),
      });

      const res = await fetch(`/api/dsp/loudness?${query.toString()}`, {
        method: "POST",
        headers: {
          "Content-Type": "application/octet-stream",
          Accept: "application/json",
        },
        body: samples.buffer as ArrayBuffer,
      });

      if (!res.ok) return null;
      return (await res.json()) as NativeLoudnessResult;
    } catch {
      return null;
    }
  }
}
