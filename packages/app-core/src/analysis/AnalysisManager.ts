/**
 * Audio analysis foundations: Peak pyramids, Spectrogram tiles, BS.1770 loudness, and Stereo analysis.
 */

export interface PeakBinData {
  min: number;
  max: number;
  rms: number;
}

export interface LoudnessResult {
  integratedLufs: number;
  maxMomentaryLufs: number;
  maxShortTermLufs: number;
  loudnessRangeLu: number;
  maxTruePeakDbtp: number;
  maxTruePeakLinear: number;
}

export interface GoniometerCoord {
  x: number;
  y: number;
}

export interface StereoResult {
  phaseCorrelation: number;
  stereoWidth: number;
  midRms: number;
  sideRms: number;
  goniometerPoints: GoniometerCoord[];
}

export interface SpectrogramOptions {
  windowSize?: number;
  hopSize?: number;
  numBins?: number;
  scale?: "linear" | "logarithmic" | "mel";
  colormap?: "magma" | "viridis" | "inferno";
  minDbfs?: number;
  maxDbfs?: number;
}

/**
 * Audio analysis manager providing peak summaries, spectrogram tiles, BS.1770 loudness, and stereo vectorscopes.
 */
export class AnalysisManager {
  /**
   * Generates a multi-resolution peak pyramid for fast waveform rendering.
   */
  public async buildPeakPyramid(
    samples: Float32Array,
    numChannels: number,
    baseBinSize = 64,
  ): Promise<Uint8Array> {
    const wasm = (
      globalThis as unknown as {
        wasmBuildPeakPyramid?: (
          samples: Float32Array,
          channels: number,
          binSize: number,
        ) => Uint8Array;
      }
    ).wasmBuildPeakPyramid;

    if (typeof wasm === "function") {
      return wasm(samples, numChannels, baseBinSize);
    }

    // Pure JS fallback pyramid builder
    return this.createFallbackPyramid(samples, numChannels, baseBinSize);
  }

  /**
   * Queries a timeline range from serialized pyramid bytes at the optimal zoom level.
   */
  public queryPeakPyramid(
    pyramidBytes: Uint8Array,
    channel: number,
    startFrame: number,
    endFrame: number,
    maxBins = 1024,
  ): PeakBinData[] {
    const wasmQuery = (
      globalThis as unknown as {
        wasmQueryPeakPyramid?: (
          bytes: Uint8Array,
          ch: number,
          start: number,
          end: number,
          bins: number,
        ) => Float32Array | number[];
      }
    ).wasmQueryPeakPyramid;

    if (typeof wasmQuery === "function") {
      const raw = wasmQuery(pyramidBytes, channel, startFrame, endFrame, maxBins);
      const bins: PeakBinData[] = [];
      for (let i = 0; i < raw.length; i += 3) {
        bins.push({
          min: raw[i],
          max: raw[i + 1],
          rms: raw[i + 2],
        });
      }
      return bins;
    }

    return [];
  }

  /**
   * Measures ITU-R BS.1770-4 / EBU R128 integrated loudness, momentary, short-term, LRA, and true peak.
   */
  public async measureLoudness(
    samples: Float32Array,
    numChannels: number,
    sampleRate: number,
  ): Promise<LoudnessResult> {
    const wasmLoudness = (
      globalThis as unknown as {
        wasmMeasureLoudness?: (samples: Float32Array, channels: number, sr: number) => string;
      }
    ).wasmMeasureLoudness;

    if (typeof wasmLoudness === "function") {
      const json = wasmLoudness(samples, numChannels, sampleRate);
      const raw = JSON.parse(json);
      return {
        integratedLufs: raw.integrated_lufs,
        maxMomentaryLufs: raw.max_momentary_lufs,
        maxShortTermLufs: raw.max_short_term_lufs,
        loudnessRangeLu: raw.loudness_range_lu,
        maxTruePeakDbtp: raw.max_true_peak_dbtp,
        maxTruePeakLinear: raw.max_true_peak_linear,
      };
    }

    // Fallback simple RMS estimate
    let sumSq = 0;
    for (let i = 0; i < samples.length; i++) {
      sumSq += samples[i] * samples[i];
    }
    const rms = Math.sqrt(sumSq / samples.length);
    const lufs = rms > 1e-7 ? -0.691 + 20 * Math.log10(rms) : -140.0;

    return {
      integratedLufs: lufs,
      maxMomentaryLufs: lufs,
      maxShortTermLufs: lufs,
      loudnessRangeLu: 0.0,
      maxTruePeakDbtp: lufs,
      maxTruePeakLinear: rms,
    };
  }

  /**
   * Computes stereo spatial analysis: phase correlation, stereo width ratio, and goniometer vectorscope points.
   */
  public async analyzeStereo(
    left: Float32Array,
    right: Float32Array,
    maxGoniometerPoints = 512,
  ): Promise<StereoResult> {
    const wasmStereo = (
      globalThis as unknown as {
        wasmAnalyzeStereo?: (l: Float32Array, r: Float32Array, pts: number) => string;
      }
    ).wasmAnalyzeStereo;

    if (typeof wasmStereo === "function") {
      const json = wasmStereo(left, right, maxGoniometerPoints);
      const raw = JSON.parse(json);
      const points: GoniometerCoord[] = [];
      const coords = raw.goniometer_coords as number[];
      for (let i = 0; i < coords.length; i += 2) {
        points.push({ x: coords[i], y: coords[i + 1] });
      }
      return {
        phaseCorrelation: raw.phase_correlation,
        stereoWidth: raw.stereo_width,
        midRms: raw.mid_rms,
        sideRms: raw.side_rms,
        goniometerPoints: points,
      };
    }

    // Pure JS fallback
    let sumL2 = 0;
    let sumR2 = 0;
    let sumLR = 0;
    const n = Math.min(left.length, right.length);
    for (let i = 0; i < n; i++) {
      sumL2 += left[i] * left[i];
      sumR2 += right[i] * right[i];
      sumLR += left[i] * right[i];
    }
    const denom = Math.sqrt(sumL2 * sumR2);
    const phase = denom > 1e-12 ? Math.max(-1.0, Math.min(1.0, sumLR / denom)) : 1.0;

    return {
      phaseCorrelation: phase,
      stereoWidth: 1.0,
      midRms: Math.sqrt(sumL2 / n),
      sideRms: Math.sqrt(sumR2 / n),
      goniometerPoints: [],
    };
  }

  /**
   * Generates a 2D spectrogram tile returning packed RGBA pixel bytes.
   */
  public async generateSpectrogramTile(
    samples: Float32Array,
    sampleRate: number,
    options: SpectrogramOptions = {},
  ): Promise<Uint8Array> {
    const wasmSpectrogram = (
      globalThis as unknown as {
        wasmGenerateSpectrogramTile?: (
          samples: Float32Array,
          sr: number,
          configJson: string,
        ) => Uint8Array;
      }
    ).wasmGenerateSpectrogramTile;

    if (typeof wasmSpectrogram === "function") {
      const configJson = JSON.stringify({
        window_size: options.windowSize ?? 1024,
        hop_size: options.hopSize ?? 512,
        num_bins: options.numBins ?? 256,
        scale: options.scale ?? "linear",
        colormap: options.colormap ?? "magma",
        min_dbfs: options.minDbfs ?? -100.0,
        max_dbfs: options.maxDbfs ?? 0.0,
      });

      return wasmSpectrogram(samples, sampleRate, configJson);
    }

    return new Uint8Array(0);
  }

  private createFallbackPyramid(
    samples: Float32Array,
    channels: number,
    baseBinSize: number,
  ): Uint8Array {
    const numFrames = Math.floor(samples.length / channels);
    const binsCount = Math.ceil(numFrames / baseBinSize);
    const headerSize = 24 + 8;
    const bodySize = channels * binsCount * 12;
    const buffer = new ArrayBuffer(headerSize + bodySize);
    const view = new DataView(buffer);

    // "FPYR" magic
    view.setUint8(0, 0x46);
    view.setUint8(1, 0x50);
    view.setUint8(2, 0x59);
    view.setUint8(3, 0x52);
    view.setUint16(4, 1, true); // Version
    view.setUint16(6, channels, true);
    view.setUint32(8, baseBinSize, true);
    view.setBigUint64(12, BigInt(numFrames), true);
    view.setUint32(20, 1, true); // 1 level

    // Level 0 header
    view.setUint32(24, baseBinSize, true);
    view.setUint32(28, binsCount, true);

    let offset = 32;
    for (let c = 0; c < channels; c++) {
      for (let b = 0; b < binsCount; b++) {
        let minVal = 1.0;
        let maxVal = -1.0;
        let sumSq = 0;
        const start = b * baseBinSize;
        const end = Math.min(numFrames, start + baseBinSize);
        const count = end - start;

        for (let f = start; f < end; f++) {
          const s = samples[f * channels + c];
          if (s < minVal) minVal = s;
          if (s > maxVal) maxVal = s;
          sumSq += s * s;
        }

        const rms = count > 0 ? Math.sqrt(sumSq / count) : 0;
        view.setFloat32(offset, minVal, true);
        view.setFloat32(offset + 4, maxVal, true);
        view.setFloat32(offset + 8, rms, true);
        offset += 12;
      }
    }

    return new Uint8Array(buffer);
  }
}
