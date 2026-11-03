/**
 * Studio-grade dynamics processing definitions, transfer functions, and presets.
 * Covers: Compressor, Limiter, Multiband, Gate, De-Esser, Transient Shaper, and Leveler.
 */

export type DynamicsMode =
  "Compressor" | "Limiter" | "Multiband" | "Gate" | "De-esser" | "Transient" | "Leveler";

export interface CompressorParameters {
  thresholdDb: number; // -60 to 0 dB, default -18
  ratio: number; // 1 to 20, default 3.2
  attackMs: number; // 0.1 to 200 ms, default 12
  releaseMs: number; // 5 to 2000 ms, default 180
  kneeDb: number; // 0 to 24 dB, default 6
  makeupDb: number; // 0 to 36 dB, default 3.5
  autoMakeup: boolean; // default true
  programDependentRelease: boolean; // default true
  lookaheadMs: number; // 0 to 20 ms, default 3
  mix: number; // 0 to 100%, default 100
  sidechainSource: "This track" | "Drums" | "Music bus";
  sidechainLowCutHz: number; // 20 to 500 Hz, default 80
}

export interface LimiterParameters {
  ceilingDbTp: number; // -30 to -0.1 dBTP, default -1.0
  releaseMs: number; // 5 to 1000 ms, default 50
  lookaheadMs: number; // 1 to 10 ms, default 3
}

export interface GateParameters {
  thresholdDb: number; // -80 to 0 dB, default -40
  ratio: number; // 1 to 100, default 20
  attackMs: number; // 0.1 to 50 ms, default 2
  holdMs: number; // 0 to 500 ms, default 20
  releaseMs: number; // 5 to 2000 ms, default 100
  hysteresisDb: number; // 0 to 12 dB, default 3
  floorDb: number; // -80 to -6 dB, default -60
}

export interface DeEsserParameters {
  thresholdDb: number; // -60 to 0 dB, default -24
  ratio: number; // 1.5 to 10, default 4
  frequencyHz: number; // 4000 to 10000 Hz, default 6500
  bandwidthOctaves: number; // 0.5 to 2.0, default 1.0
  splitBand: boolean; // default true
}

export interface TransientParameters {
  attackGainDb: number; // -12 to +12 dB, default 0
  sustainGainDb: number; // -12 to +12 dB, default 0
}

export interface MultibandBandParameters {
  name: string;
  lowCutoffHz: number;
  highCutoffHz: number;
  thresholdDb: number;
  ratio: number;
  attackMs: number;
  releaseMs: number;
  makeupGainDb: number;
  mute: boolean;
  solo: boolean;
  bypassed: boolean;
}

export interface LevelerParameters {
  targetLufs: number; // -36 to -6 LUFS, default -16
  windowSeconds: number; // 1 to 5 s, default 3
  maxBoostDb: number; // 0 to 18 dB, default 8
  maxCutDb: number; // -24 to 0 dB, default -12
  gateThresholdDbfs: number; // -60 to -30 dBFS, default -45
}

/**
 * Computes static output dB level for a compressor with soft knee.
 */
export function computeCompressorTransfer(
  inputDb: number,
  thresholdDb: number,
  ratio: number,
  kneeDb: number,
): number {
  const halfKnee = kneeDb * 0.5;
  const delta = inputDb - thresholdDb;
  const r = Math.max(1.0, ratio);

  if (delta <= -halfKnee) {
    return inputDb;
  }
  if (delta >= halfKnee || kneeDb <= 0.001) {
    return thresholdDb + delta / r;
  }
  const k = delta + halfKnee;
  return inputDb + ((1.0 / r - 1.0) * k * k) / (2.0 * kneeDb);
}

/**
 * Computes automatic makeup gain in dB based on threshold and ratio.
 */
export function computeAutoMakeupGain(thresholdDb: number, ratio: number): number {
  const r = Math.max(1.0, ratio);
  return Math.max(0.0, -thresholdDb * 0.5 * (1.0 - 1.0 / r));
}

/**
 * Computes downward expander / noise gate static transfer function.
 */
export function computeGateTransfer(
  inputDb: number,
  thresholdDb: number,
  ratio: number,
  floorDb: number,
): number {
  if (inputDb >= thresholdDb) {
    return inputDb;
  }
  const delta = thresholdDb - inputDb;
  const attenuation = delta * (ratio - 1.0);
  const outDb = inputDb - attenuation;
  return Math.max(floorDb, outDb);
}

/**
 * Default production dynamics settings.
 */
export const DEFAULT_COMPRESSOR_PARAMS: CompressorParameters = {
  thresholdDb: -18,
  ratio: 3.2,
  attackMs: 12,
  releaseMs: 180,
  kneeDb: 6,
  makeupDb: 3.5,
  autoMakeup: true,
  programDependentRelease: true,
  lookaheadMs: 3,
  mix: 100,
  sidechainSource: "This track",
  sidechainLowCutHz: 80,
};

export const DEFAULT_LIMITER_PARAMS: LimiterParameters = {
  ceilingDbTp: -1.0,
  releaseMs: 50,
  lookaheadMs: 3,
};

export const DEFAULT_GATE_PARAMS: GateParameters = {
  thresholdDb: -40,
  ratio: 20,
  attackMs: 2,
  holdMs: 20,
  releaseMs: 100,
  hysteresisDb: 3,
  floorDb: -60,
};

export const DEFAULT_DEESSER_PARAMS: DeEsserParameters = {
  thresholdDb: -24,
  ratio: 4,
  frequencyHz: 6500,
  bandwidthOctaves: 1.0,
  splitBand: true,
};

export const DEFAULT_TRANSIENT_PARAMS: TransientParameters = {
  attackGainDb: 0,
  sustainGainDb: 0,
};

export const DEFAULT_MULTIBAND_BANDS: MultibandBandParameters[] = [
  {
    name: "Low",
    lowCutoffHz: 20,
    highCutoffHz: 200,
    thresholdDb: -18,
    ratio: 3.0,
    attackMs: 30,
    releaseMs: 200,
    makeupGainDb: 1.5,
    mute: false,
    solo: false,
    bypassed: false,
  },
  {
    name: "Low-Mid",
    lowCutoffHz: 200,
    highCutoffHz: 1500,
    thresholdDb: -16,
    ratio: 2.8,
    attackMs: 15,
    releaseMs: 160,
    makeupGainDb: 1.0,
    mute: false,
    solo: false,
    bypassed: false,
  },
  {
    name: "High-Mid",
    lowCutoffHz: 1500,
    highCutoffHz: 6000,
    thresholdDb: -14,
    ratio: 2.5,
    attackMs: 8,
    releaseMs: 120,
    makeupGainDb: 1.0,
    mute: false,
    solo: false,
    bypassed: false,
  },
  {
    name: "High",
    lowCutoffHz: 6000,
    highCutoffHz: 20000,
    thresholdDb: -12,
    ratio: 2.2,
    attackMs: 4,
    releaseMs: 90,
    makeupGainDb: 0.5,
    mute: false,
    solo: false,
    bypassed: false,
  },
];

export const DEFAULT_LEVELER_PARAMS: LevelerParameters = {
  targetLufs: -16.0,
  windowSeconds: 3.0,
  maxBoostDb: 8.0,
  maxCutDb: -12.0,
  gateThresholdDbfs: -45.0,
};

/**
 * Curated production presets for dynamics processing.
 */
export interface DynamicsPreset {
  id: string;
  name: string;
  category: DynamicsMode;
  description: string;
  params: Partial<CompressorParameters> &
    Partial<LimiterParameters> &
    Partial<GateParameters> &
    Partial<DeEsserParameters> &
    Partial<TransientParameters> &
    Partial<LevelerParameters>;
}

export const DYNAMICS_PRESETS: DynamicsPreset[] = [
  {
    id: "vocal-opt",
    name: "Smooth Optical Vocal",
    category: "Compressor",
    description: "Gentle leveling with program-dependent release and transparent knee",
    params: {
      thresholdDb: -16,
      ratio: 2.5,
      attackMs: 20,
      releaseMs: 250,
      kneeDb: 8,
      autoMakeup: true,
      programDependentRelease: true,
    },
  },
  {
    id: "punchy-drums",
    name: "Punchy Drum Bus",
    category: "Compressor",
    description: "Slower attack to preserve transient crack with fast recovery",
    params: {
      thresholdDb: -14,
      ratio: 4.0,
      attackMs: 30,
      releaseMs: 80,
      kneeDb: 4,
      mix: 80,
      sidechainLowCutHz: 90,
    },
  },
  {
    id: "brickwall-master",
    name: "Master Brickwall Limiter",
    category: "Limiter",
    description: "ISP-safe true-peak ceiling for streaming distribution",
    params: {
      ceilingDbTp: -1.0,
      releaseMs: 40,
      lookaheadMs: 3.5,
    },
  },
  {
    id: "speech-gate",
    name: "Podcast Room Noise Gate",
    category: "Gate",
    description: "Smooth gating with hysteresis preventing chattering on breaths",
    params: {
      thresholdDb: -38,
      ratio: 15,
      attackMs: 1.5,
      holdMs: 30,
      releaseMs: 120,
      hysteresisDb: 4,
      floorDb: -48,
    },
  },
  {
    id: "gentle-deess",
    name: "Natural Sibilance Tamer",
    category: "De-esser",
    description: "Split-band ducking centered at 6.8 kHz for clean modern vocals",
    params: {
      thresholdDb: -22,
      ratio: 3.5,
      frequencyHz: 6800,
      bandwidthOctaves: 1.0,
      splitBand: true,
    },
  },
  {
    id: "snare-transient",
    name: "Snare Attack Boost",
    category: "Transient",
    description: "Boosts initial transient crack while tightening ringing sustain",
    params: {
      attackGainDb: 4.5,
      sustainGainDb: -2.0,
    },
  },
  {
    id: "broadcast-leveler",
    name: "EBU Broadcast Leveler",
    category: "Leveler",
    description: "Rides gain smoothly towards -16 LUFS with speech-aware noise freeze",
    params: {
      targetLufs: -16.0,
      windowSeconds: 3.0,
      maxBoostDb: 6.0,
      maxCutDb: -8.0,
      gateThresholdDbfs: -42.0,
    },
  },
];
