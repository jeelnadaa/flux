/**
 * Audio export and batch format conversion manager.
 */

import { NativeBridge } from "../platform/NativeBridge.js";

export type ExportTargetFormat =
  | "wav_16"
  | "wav_24"
  | "wav_32f"
  | "flac_16"
  | "flac_24"
  | "mp3_128"
  | "mp3_192"
  | "mp3_256"
  | "mp3_320"
  | "mp3_v0"
  | "aac_96"
  | "aac_128"
  | "aac_192"
  | "aac_256"
  | "vorbis_q3"
  | "vorbis_q6"
  | "vorbis_q9"
  | "opus_64"
  | "opus_128"
  | "opus_192"
  | "opus_voice";

export type ExportPresetId =
  "streaming" | "podcast_stereo" | "podcast_mono" | "youtube" | "cd_audio" | "archival";

export interface ExportPresetInfo {
  id: ExportPresetId;
  name: string;
  description: string;
  sampleRate?: number;
  bitDepth?: number;
  targetLufs?: number;
  truePeakCeiling?: number;
  targetPeakDbfs?: number;
  fileExtension: string;
}

export interface ExportOptions {
  format?: ExportTargetFormat;
  preset?: ExportPresetId;
  sampleRate?: number;
  channelMode?: "keep" | "mono" | "stereo";
  regionStart?: number;
  regionEnd?: number;
  tailFrames?: number;
  fadeOutFrames?: number;
  normalization?: "off" | string;
  dither?: "none" | "tpdf" | "noise_shaped";
  title?: string;
  artist?: string;
  album?: string;
  date?: string;
  genre?: string;
  trackNumber?: number;
  comment?: string;
  forceRf64?: boolean;
}

export interface ExportProgress {
  fraction: number;
  stage: string;
  framesProcessed: number;
  totalFrames: number;
}

export type ExportProgressCallback = (progress: ExportProgress) => void;

/**
 * Standard destination presets matching PLAN.md §1.4.
 */
export const EXPORT_PRESETS: ExportPresetInfo[] = [
  {
    id: "streaming",
    name: "Streaming (Spotify / Apple Music / Tidal)",
    description: "Target -14 LUFS with -1.0 dBTP true-peak ceiling for streaming platforms",
    targetLufs: -14.0,
    truePeakCeiling: -1.0,
    fileExtension: "wav",
  },
  {
    id: "podcast_stereo",
    name: "Podcast (Stereo)",
    description: "Target -16 LUFS with -1.0 dBTP true-peak ceiling for stereo podcast release",
    targetLufs: -16.0,
    truePeakCeiling: -1.0,
    fileExtension: "mp3",
  },
  {
    id: "podcast_mono",
    name: "Podcast (Mono)",
    description: "Target -19 LUFS with -1.0 dBTP true-peak ceiling and mono downmix",
    targetLufs: -19.0,
    truePeakCeiling: -1.0,
    fileExtension: "mp3",
  },
  {
    id: "youtube",
    name: "YouTube",
    description: "Target -14 LUFS with -1.0 dBTP ceiling at 48 kHz",
    sampleRate: 48000,
    targetLufs: -14.0,
    truePeakCeiling: -1.0,
    fileExtension: "wav",
  },
  {
    id: "cd_audio",
    name: "CD Audio (Red Book)",
    description: "16-bit / 44.1 kHz with TPDF dither and -0.3 dBFS peak ceiling",
    sampleRate: 44100,
    bitDepth: 16,
    targetPeakDbfs: -0.3,
    fileExtension: "wav",
  },
  {
    id: "archival",
    name: "Archival Master (24-bit FLAC)",
    description: "24-bit / 48 kHz FLAC lossless archival master with -0.1 dBFS margin",
    sampleRate: 48000,
    bitDepth: 24,
    targetPeakDbfs: -0.1,
    fileExtension: "flac",
  },
];

/**
 * Manages background export jobs, preset selection, and batch format conversion.
 */
export class ExportManager {
  /**
   * Returns list of predefined export presets.
   */
  public getPresets(): ExportPresetInfo[] {
    return EXPORT_PRESETS;
  }

  /**
   * Encodes audio samples to target format and specifications.
   */
  public async exportAudio(
    samples: Float32Array,
    numChannels: number,
    sampleRate: number,
    options: ExportOptions = {},
    onProgress?: ExportProgressCallback,
    abortSignal?: AbortSignal,
  ): Promise<Uint8Array> {
    if (abortSignal?.aborted) {
      throw new DOMException("Export was cancelled", "AbortError");
    }

    const totalFrames = Math.floor(samples.length / numChannels);
    onProgress?.({
      fraction: 0.1,
      stage: "preparing",
      framesProcessed: 0,
      totalFrames,
    });

    if (abortSignal?.aborted) {
      throw new DOMException("Export was cancelled", "AbortError");
    }

    // 1. Fast-path: Native Rust SIMD export when running on desktop
    try {
      const nativeBytes = await NativeBridge.exportAudio(
        samples,
        numChannels,
        sampleRate,
        options,
      );
      if (nativeBytes && nativeBytes.byteLength > 0) {
        onProgress?.({
          fraction: 1.0,
          stage: "finished",
          framesProcessed: totalFrames,
          totalFrames,
        });
        return nativeBytes;
      }
    } catch {
      // Continue to WASM or JS fallback
    }

    // 2. Call WASM export function if available in global environment
    const wasm = (
      globalThis as unknown as {
        wasmExportAudio?: (
          samples: Float32Array,
          ch: number,
          sr: number,
          config: string,
        ) => Uint8Array;
      }
    ).wasmExportAudio;

    if (typeof wasm === "function") {
      const configJson = JSON.stringify({
        format: options.format,
        preset: options.preset,
        sample_rate: options.sampleRate,
        channel_mode: options.channelMode,
        region_start: options.regionStart,
        region_end: options.regionEnd,
        tail_frames: options.tailFrames,
        fade_out_frames: options.fadeOutFrames,
        normalization: options.normalization,
        dither: options.dither,
        title: options.title,
        artist: options.artist,
        album: options.album,
        date: options.date,
        genre: options.genre,
        track_number: options.trackNumber,
        comment: options.comment,
        force_rf64: options.forceRf64,
      });

      onProgress?.({
        fraction: 0.5,
        stage: "encoding",
        framesProcessed: Math.floor(totalFrames / 2),
        totalFrames,
      });

      if (abortSignal?.aborted) {
        throw new DOMException("Export was cancelled", "AbortError");
      }

      const out = wasm(samples, numChannels, sampleRate, configJson);

      onProgress?.({
        fraction: 1.0,
        stage: "finished",
        framesProcessed: totalFrames,
        totalFrames,
      });

      return out;
    }

    // Fallback simple RIFF WAV generator for pure JS environments
    onProgress?.({
      fraction: 0.6,
      stage: "encoding_wav",
      framesProcessed: totalFrames,
      totalFrames,
    });

    const wavBytes = this.createFallbackWav(samples, numChannels, sampleRate);
    onProgress?.({
      fraction: 1.0,
      stage: "finished",
      framesProcessed: totalFrames,
      totalFrames,
    });

    return wavBytes;
  }

  /**
   * Converts existing audio container bytes into the target format.
   */
  public async convertAudioFormat(
    inputBytes: Uint8Array,
    options: ExportOptions = {},
    onProgress?: ExportProgressCallback,
    abortSignal?: AbortSignal,
  ): Promise<Uint8Array> {
    if (abortSignal?.aborted) {
      throw new DOMException("Conversion was cancelled", "AbortError");
    }

    onProgress?.({
      fraction: 0.2,
      stage: "decoding",
      framesProcessed: 0,
      totalFrames: 100,
    });

    const wasmConvert = (
      globalThis as unknown as {
        wasmConvertAudioFormat?: (data: Uint8Array, config: string) => Uint8Array;
      }
    ).wasmConvertAudioFormat;

    if (typeof wasmConvert === "function") {
      const configJson = JSON.stringify({
        format: options.format,
        sample_rate: options.sampleRate,
        channel_mode: options.channelMode,
        normalization: options.normalization,
        dither: options.dither,
      });

      onProgress?.({
        fraction: 0.7,
        stage: "converting",
        framesProcessed: 70,
        totalFrames: 100,
      });

      if (abortSignal?.aborted) {
        throw new DOMException("Conversion was cancelled", "AbortError");
      }

      const out = wasmConvert(inputBytes, configJson);

      onProgress?.({
        fraction: 1.0,
        stage: "finished",
        framesProcessed: 100,
        totalFrames: 100,
      });

      return out;
    }

    return inputBytes;
  }

  private createFallbackWav(
    samples: Float32Array,
    channels: number,
    sampleRate: number,
  ): Uint8Array {
    const numFrames = Math.floor(samples.length / channels);
    const bytesPerSample = 2; // 16-bit PCM
    const dataLen = numFrames * channels * bytesPerSample;
    const buffer = new ArrayBuffer(44 + dataLen);
    const view = new DataView(buffer);

    // RIFF identifier
    this.writeAscii(view, 0, "RIFF");
    view.setUint32(4, 36 + dataLen, true);
    this.writeAscii(view, 8, "WAVE");

    // fmt subchunk
    this.writeAscii(view, 12, "fmt ");
    view.setUint32(16, 16, true); // Subchunk1Size (16 for PCM)
    view.setUint16(20, 1, true); // AudioFormat 1 = PCM
    view.setUint16(22, channels, true);
    view.setUint32(24, sampleRate, true);
    view.setUint32(28, sampleRate * channels * bytesPerSample, true); // ByteRate
    view.setUint16(32, channels * bytesPerSample, true); // BlockAlign
    view.setUint16(34, 16, true); // BitsPerSample

    // data subchunk
    this.writeAscii(view, 36, "data");
    view.setUint32(40, dataLen, true);

    // Write 16-bit PCM samples
    let offset = 44;
    for (let i = 0; i < samples.length; i++) {
      const s = Math.max(-1.0, Math.min(1.0, samples[i]));
      const pcm16 = s < 0 ? s * 0x8000 : s * 0x7fff;
      view.setInt16(offset, pcm16, true);
      offset += 2;
    }

    return new Uint8Array(buffer);
  }

  private writeAscii(view: DataView, offset: number, text: string): void {
    for (let i = 0; i < text.length; i++) {
      view.setUint8(offset + i, text.charCodeAt(i));
    }
  }
}
