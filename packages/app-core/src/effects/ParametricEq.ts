/**
 * Parametric Equalizer Model, Presets, and Analytic Frequency Response.
 * Complies with PLAN.md §3.1:
 * - Up to 10 parametric bands
 * - Filter types: Peaking, LowShelf, HighShelf, Lowpass, Highpass, Bandpass, Notch, Tilt
 * - Linear-phase and Analog-matched modes
 * - 10-band and 31-band Graphic EQ views
 * - All 10 documented production presets
 * - Match EQ spectral transfer matching
 */

export type EqFilterType =
  "peaking" | "lowshelf" | "highshelf" | "lowpass" | "highpass" | "bandpass" | "notch" | "tilt";

export type EqMode = "natural" | "analog-match" | "linear-phase";

export type GraphicEqViewMode = "parametric" | "10-band" | "31-band";

export interface EqBand {
  id: string;
  type: EqFilterType;
  frequency: number; // Hz
  gainDb: number; // dB (-24 to +24)
  q: number; // Q factor (0.1 to 50.0)
  slope: number; // dB/oct (6 to 96)
  enabled: boolean;
  solo: boolean;
  colorVar: string;
}

export interface EqPreset {
  name: string;
  bands: Omit<EqBand, "id">[];
}

export const ISO_10_BAND_FREQUENCIES: readonly number[] = [
  31.25, 62.5, 125, 250, 500, 1000, 2000, 4000, 8000, 16000,
] as const;

export const ISO_31_BAND_FREQUENCIES: readonly number[] = [
  20, 25, 31.5, 40, 50, 63, 80, 100, 125, 160, 200, 250, 315, 400, 500, 630, 800, 1000, 1250, 1600,
  2000, 2500, 3150, 4000, 5000, 6300, 8000, 10000, 12500, 16000, 20000,
] as const;

/**
 * Standard documented EQ presets per PLAN.md §3.1.
 */
export const EQ_PRESETS: Record<string, EqPreset> = {
  "Clear voice": {
    name: "Clear voice",
    bands: [
      {
        type: "highpass",
        frequency: 80,
        gainDb: 0,
        q: 0.707,
        slope: 18,
        enabled: true,
        solo: false,
        colorVar: "--t3",
      },
      {
        type: "peaking",
        frequency: 280,
        gainDb: -3.0,
        q: 1.3,
        slope: 12,
        enabled: true,
        solo: false,
        colorVar: "--t2",
      },
      {
        type: "peaking",
        frequency: 1200,
        gainDb: 1.6,
        q: 1.0,
        slope: 12,
        enabled: true,
        solo: false,
        colorVar: "--t4",
      },
      {
        type: "peaking",
        frequency: 3400,
        gainDb: 4.0,
        q: 1.5,
        slope: 12,
        enabled: true,
        solo: false,
        colorVar: "--t5",
      },
      {
        type: "highshelf",
        frequency: 11000,
        gainDb: 3.0,
        q: 0.707,
        slope: 12,
        enabled: true,
        solo: false,
        colorVar: "--t1",
      },
    ],
  },
  "Bass boost": {
    name: "Bass boost",
    bands: [
      {
        type: "highpass",
        frequency: 25,
        gainDb: 0,
        q: 0.707,
        slope: 24,
        enabled: true,
        solo: false,
        colorVar: "--t3",
      },
      {
        type: "lowshelf",
        frequency: 90,
        gainDb: 5.5,
        q: 0.8,
        slope: 12,
        enabled: true,
        solo: false,
        colorVar: "--t1",
      },
      {
        type: "peaking",
        frequency: 350,
        gainDb: -1.5,
        q: 1.2,
        slope: 12,
        enabled: true,
        solo: false,
        colorVar: "--t2",
      },
    ],
  },
  "Treble boost": {
    name: "Treble boost",
    bands: [
      {
        type: "highshelf",
        frequency: 6500,
        gainDb: 4.5,
        q: 0.707,
        slope: 12,
        enabled: true,
        solo: false,
        colorVar: "--t1",
      },
      {
        type: "peaking",
        frequency: 12000,
        gainDb: 2.5,
        q: 1.0,
        slope: 12,
        enabled: true,
        solo: false,
        colorVar: "--t5",
      },
    ],
  },
  Warm: {
    name: "Warm",
    bands: [
      {
        type: "lowshelf",
        frequency: 150,
        gainDb: 2.5,
        q: 0.707,
        slope: 12,
        enabled: true,
        solo: false,
        colorVar: "--t1",
      },
      {
        type: "peaking",
        frequency: 450,
        gainDb: 1.5,
        q: 1.0,
        slope: 12,
        enabled: true,
        solo: false,
        colorVar: "--t4",
      },
      {
        type: "highshelf",
        frequency: 7000,
        gainDb: -2.5,
        q: 0.707,
        slope: 12,
        enabled: true,
        solo: false,
        colorVar: "--t2",
      },
    ],
  },
  Bright: {
    name: "Bright",
    bands: [
      {
        type: "highpass",
        frequency: 40,
        gainDb: 0,
        q: 0.707,
        slope: 12,
        enabled: true,
        solo: false,
        colorVar: "--t3",
      },
      {
        type: "peaking",
        frequency: 4500,
        gainDb: 3.5,
        q: 1.2,
        slope: 12,
        enabled: true,
        solo: false,
        colorVar: "--t5",
      },
      {
        type: "highshelf",
        frequency: 10000,
        gainDb: 4.0,
        q: 0.707,
        slope: 12,
        enabled: true,
        solo: false,
        colorVar: "--t1",
      },
    ],
  },
  Podcast: {
    name: "Podcast",
    bands: [
      {
        type: "highpass",
        frequency: 75,
        gainDb: 0,
        q: 0.707,
        slope: 18,
        enabled: true,
        solo: false,
        colorVar: "--t3",
      },
      {
        type: "peaking",
        frequency: 220,
        gainDb: -2.5,
        q: 1.4,
        slope: 12,
        enabled: true,
        solo: false,
        colorVar: "--t2",
      },
      {
        type: "peaking",
        frequency: 2800,
        gainDb: 3.0,
        q: 1.2,
        slope: 12,
        enabled: true,
        solo: false,
        colorVar: "--t4",
      },
      {
        type: "peaking",
        frequency: 5500,
        gainDb: 2.0,
        q: 1.5,
        slope: 12,
        enabled: true,
        solo: false,
        colorVar: "--t5",
      },
    ],
  },
  "Telephone-clean": {
    name: "Telephone-clean",
    bands: [
      {
        type: "highpass",
        frequency: 320,
        gainDb: 0,
        q: 0.707,
        slope: 24,
        enabled: true,
        solo: false,
        colorVar: "--t3",
      },
      {
        type: "lowpass",
        frequency: 3400,
        gainDb: 0,
        q: 0.707,
        slope: 24,
        enabled: true,
        solo: false,
        colorVar: "--t2",
      },
      {
        type: "peaking",
        frequency: 1500,
        gainDb: 3.0,
        q: 1.0,
        slope: 12,
        enabled: true,
        solo: false,
        colorVar: "--t4",
      },
    ],
  },
  "De-mud": {
    name: "De-mud",
    bands: [
      {
        type: "peaking",
        frequency: 260,
        gainDb: -4.5,
        q: 1.6,
        slope: 12,
        enabled: true,
        solo: false,
        colorVar: "--t2",
      },
      {
        type: "peaking",
        frequency: 420,
        gainDb: -2.0,
        q: 2.0,
        slope: 12,
        enabled: true,
        solo: false,
        colorVar: "--t3",
      },
    ],
  },
  Presence: {
    name: "Presence",
    bands: [
      {
        type: "peaking",
        frequency: 3200,
        gainDb: 4.5,
        q: 1.3,
        slope: 12,
        enabled: true,
        solo: false,
        colorVar: "--t5",
      },
      {
        type: "highshelf",
        frequency: 8500,
        gainDb: 2.0,
        q: 0.707,
        slope: 12,
        enabled: true,
        solo: false,
        colorVar: "--t1",
      },
    ],
  },
  Air: {
    name: "Air",
    bands: [
      {
        type: "highshelf",
        frequency: 12500,
        gainDb: 4.8,
        q: 0.65,
        slope: 12,
        enabled: true,
        solo: false,
        colorVar: "--t1",
      },
    ],
  },
};

/**
 * Calculates analytic dB magnitude response for a single biquad band at frequency f.
 */
export function calculateBiquadResponse(
  f: number,
  sr: number,
  type: EqFilterType,
  f0: number,
  gainDb: number,
  q: number,
): number {
  if (f <= 0 || f >= sr / 2) return 0;
  const omega = (2 * Math.PI * f) / sr;
  const omega0 = (2 * Math.PI * f0) / sr;
  const alpha = Math.sin(omega0) / (2 * q);
  const A = Math.pow(10, gainDb / 40);
  const cos0 = Math.cos(omega0);

  let b0 = 1,
    b1 = 0,
    b2 = 0,
    a0 = 1,
    a1 = 0,
    a2 = 0;

  switch (type) {
    case "peaking":
      b0 = 1 + alpha * A;
      b1 = -2 * cos0;
      b2 = 1 - alpha * A;
      a0 = 1 + alpha / A;
      a1 = -2 * cos0;
      a2 = 1 - alpha / A;
      break;
    case "lowshelf": {
      const twoSqrtAAlpha = 2 * Math.sqrt(A) * alpha;
      b0 = A * (A + 1 - (A - 1) * cos0 + twoSqrtAAlpha);
      b1 = 2 * A * (A - 1 - (A + 1) * cos0);
      b2 = A * (A + 1 - (A - 1) * cos0 - twoSqrtAAlpha);
      a0 = A + 1 + (A - 1) * cos0 + twoSqrtAAlpha;
      a1 = -2 * (A - 1 + (A + 1) * cos0);
      a2 = A + 1 + (A - 1) * cos0 - twoSqrtAAlpha;
      break;
    }
    case "highshelf": {
      const twoSqrtAAlpha = 2 * Math.sqrt(A) * alpha;
      b0 = A * (A + 1 + (A - 1) * cos0 + twoSqrtAAlpha);
      b1 = -2 * A * (A - 1 + (A + 1) * cos0);
      b2 = A * (A + 1 + (A - 1) * cos0 - twoSqrtAAlpha);
      a0 = A + 1 - (A - 1) * cos0 + twoSqrtAAlpha;
      a1 = 2 * (A - 1 - (A + 1) * cos0);
      a2 = A + 1 - (A - 1) * cos0 - twoSqrtAAlpha;
      break;
    }
    case "lowpass":
      b0 = (1 - cos0) / 2;
      b1 = 1 - cos0;
      b2 = (1 - cos0) / 2;
      a0 = 1 + alpha;
      a1 = -2 * cos0;
      a2 = 1 - alpha;
      break;
    case "highpass":
      b0 = (1 + cos0) / 2;
      b1 = -(1 + cos0);
      b2 = (1 + cos0) / 2;
      a0 = 1 + alpha;
      a1 = -2 * cos0;
      a2 = 1 - alpha;
      break;
    case "bandpass":
      b0 = alpha;
      b1 = 0;
      b2 = -alpha;
      a0 = 1 + alpha;
      a1 = -2 * cos0;
      a2 = 1 - alpha;
      break;
    case "notch":
      b0 = 1;
      b1 = -2 * cos0;
      b2 = 1;
      a0 = 1 + alpha;
      a1 = -2 * cos0;
      a2 = 1 - alpha;
      break;
    case "tilt": {
      // Tilt = LowShelf(-gain/2) + HighShelf(+gain/2)
      const low = calculateBiquadResponse(f, sr, "lowshelf", f0, -gainDb / 2, 0.707);
      const high = calculateBiquadResponse(f, sr, "highshelf", f0, gainDb / 2, 0.707);
      return low + high;
    }
  }

  // Evaluate transfer function on unit circle z = e^(j*omega)
  const cosW = Math.cos(omega);
  const sinW = Math.sin(omega);
  const cos2W = Math.cos(2 * omega);
  const sin2W = Math.sin(2 * omega);

  const numRe = b0 / a0 + (b1 / a0) * cosW + (b2 / a0) * cos2W;
  const numIm = -(b1 / a0) * sinW - (b2 / a0) * sin2W;
  const denRe = 1 + (a1 / a0) * cosW + (a2 / a0) * cos2W;
  const denIm = -(a1 / a0) * sinW - (a2 / a0) * sin2W;

  const numMagSq = numRe * numRe + numIm * numIm;
  const denMagSq = denRe * denRe + denIm * denIm;
  if (denMagSq <= 1e-12) return 0;

  const mag = Math.sqrt(numMagSq / denMagSq);
  return 20 * Math.log10(Math.max(1e-9, mag));
}

/**
 * Calculates the combined dB magnitude response of all active EQ bands.
 */
export function calculateCombinedResponse(
  frequencies: readonly number[] | Float32Array,
  bands: readonly EqBand[],
  outputGainDb = 0,
  sampleRate = 48000,
): Float32Array {
  const result = new Float32Array(frequencies.length);
  const hasSolo = bands.some((b) => b.enabled && b.solo);

  for (let i = 0; i < frequencies.length; i++) {
    const f = frequencies[i];
    let sumDb = outputGainDb;

    for (const b of bands) {
      if (!b.enabled) continue;
      if (hasSolo && !b.solo) continue;

      if (b.type === "highpass" || b.type === "lowpass") {
        const numBiquads = Math.min(8, Math.max(1, Math.ceil(b.slope / 12)));
        for (let k = 0; k < numBiquads; k++) {
          const order = numBiquads * 2;
          const qButter =
            1 / (2 * Math.abs(Math.cos(((2 * k + 1) * Math.PI) / (2 * order)))) || 0.707;
          sumDb += calculateBiquadResponse(f, sampleRate, b.type, b.frequency, 0, qButter);
        }
      } else {
        sumDb += calculateBiquadResponse(f, sampleRate, b.type, b.frequency, b.gainDb, b.q);
      }
    }
    result[i] = sumDb;
  }

  return result;
}

/**
 * Match EQ: Computes the spectral transfer correction needed to match source to reference.
 */
export function computeMatchEqCurve(
  sourceSpectrumDb: readonly number[] | Float32Array,
  referenceSpectrumDb: readonly number[] | Float32Array,
  smoothingWindow = 5,
): Float32Array {
  const len = Math.min(sourceSpectrumDb.length, referenceSpectrumDb.length);
  const diff = new Float32Array(len);

  for (let i = 0; i < len; i++) {
    diff[i] = Math.min(18, Math.max(-18, referenceSpectrumDb[i] - sourceSpectrumDb[i]));
  }

  if (smoothingWindow <= 1) return diff;

  const smoothed = new Float32Array(len);
  const half = Math.floor(smoothingWindow / 2);
  for (let i = 0; i < len; i++) {
    const start = Math.max(0, i - half);
    const end = Math.min(len, i + half + 1);
    let sum = 0;
    for (let k = start; k < end; k++) {
      sum += diff[k];
    }
    smoothed[i] = sum / (end - start);
  }

  return smoothed;
}
