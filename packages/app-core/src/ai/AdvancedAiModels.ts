/**
 * Advanced AI domain models for Tempo/Key, Auto-Mastering, TTS, Dubbing, Voice Cloning, and Magic Assistant.
 * PLAN.md Phase 7 specification.
 */

export interface TempoKeyAnalysis {
  bpm: number;
  bpmConfidence: number;
  key: string;
  keyConfidence: number;
  camelotNotation: string;
  openKeyNotation: string;
  tuningHz: number;
  meter: string;
  timelineSegments: Array<{
    key: string;
    widthPercent: number;
    colorToken: string;
  }>;
}

export const DEFAULT_TEMPO_KEY_ANALYSIS: TempoKeyAnalysis = {
  bpm: 84.0,
  bpmConfidence: 96,
  key: "D min",
  keyConfidence: 88,
  camelotNotation: "7A Camelot",
  openKeyNotation: "6m Open Key",
  tuningHz: 440,
  meter: "4/4",
  timelineSegments: [
    { key: "D min", widthPercent: 62, colorToken: "--t1" },
    { key: "F maj", widthPercent: 16, colorToken: "--t3" },
    { key: "D min", widthPercent: 22, colorToken: "--t1" },
  ],
};

export interface MasterMetric {
  value: string;
  unit: string;
  label: string;
  status: "ok" | "warn" | "bad";
}

export const DEFAULT_MASTER_METRICS: MasterMetric[] = [
  { value: "−14.0", unit: "LUFS", label: "Target met", status: "ok" },
  { value: "−1.0", unit: "dBTP", label: "Below ceiling", status: "ok" },
  { value: "8.2", unit: "LRA", label: "Dynamics kept", status: "ok" },
  { value: "+0.71", unit: "Mono", label: "Safe", status: "ok" },
];

export interface MasterChangeItem {
  title: string;
  details: string;
  enabled: boolean;
}

export const DEFAULT_MASTER_CHANGES: MasterChangeItem[] = [
  { title: "Cut 250 Hz", details: "−2.0 dB, mud", enabled: true },
  { title: "Lift 3 kHz", details: "+1.5 dB, presence", enabled: true },
  { title: "Air at 12 kHz", details: "+1.0 dB", enabled: true },
  { title: "Multiband", details: "Average −2.1 dB", enabled: true },
  { title: "Width", details: "+6%, lows stay mono", enabled: true },
  { title: "Limiter", details: "Average −1.4 dB", enabled: true },
];

export interface TtsVoiceOption {
  id: string;
  name: string;
  description: string;
  selected: boolean;
}

export const DEFAULT_TTS_VOICES: TtsVoiceOption[] = [
  { id: "ash", name: "Ash", description: "Warm, narration", selected: true },
  { id: "reed", name: "Reed", description: "Clear, neutral", selected: false },
  { id: "marin", name: "Marin", description: "Bright, friendly", selected: false },
  { id: "oren", name: "Oren", description: "Deep, slow", selected: false },
];

export interface DubReviewItem {
  speaker: string;
  colorToken: string;
  originalText: string;
  translatedText: string;
  durationPercent: string;
  status: "ok" | "warn";
}

export const DEFAULT_DUB_REVIEW_ITEMS: DubReviewItem[] = [
  {
    speaker: "Maya",
    colorToken: "--t4",
    originalText:
      "The first thing I noticed about the harbor recordings was how much low rumble came through the hull.",
    translatedText:
      "Lo primero que noté en las grabaciones del puerto fue cuánto retumbo bajo atravesaba el casco.",
    durationPercent: "+4%",
    status: "ok",
  },
  {
    speaker: "Tom",
    colorToken: "--t3",
    originalText:
      "And that was on the first night, before we moved the mics up to the Kessler deck.",
    translatedText:
      "Y eso fue la primera noche, antes de que subiéramos los micrófonos a la cubierta Kessler y volviéramos a empezar.",
    durationPercent: "+22%",
    status: "warn",
  },
  {
    speaker: "Maya",
    colorToken: "--t4",
    originalText: "Which meant we had to cut around the engine for the whole second act.",
    translatedText: "Lo que nos obligó a editar alrededor del motor durante todo el segundo acto.",
    durationPercent: "+8%",
    status: "ok",
  },
];

export interface MagicDiagnosisItem {
  label: string;
  status: string;
  badge: "ok" | "warn" | "bad";
}

export const DEFAULT_MAGIC_DIAGNOSIS: MagicDiagnosisItem[] = [
  { label: "Noise", status: "Moderate", badge: "warn" },
  { label: "Echo", status: "Slight", badge: "warn" },
  { label: "Clipping", status: "None", badge: "ok" },
  { label: "Hum", status: "50 Hz", badge: "warn" },
  { label: "Loudness", status: "-21.3 LUFS", badge: "warn" },
];

export interface MagicChainItem {
  title: string;
  details: string;
  enabled: boolean;
}

export const DEFAULT_MAGIC_CHAIN: MagicChainItem[] = [
  { title: "Hum removal", details: "Found 50 Hz and 6 harmonics", enabled: true },
  { title: "Noise reduction", details: "Room noise at -52 dB", enabled: true },
  { title: "De-reverb", details: "Short room echo", enabled: true },
  { title: "EQ: Clear voice", details: "Cut 250 Hz -2 dB, lift 3 kHz +1.5 dB", enabled: true },
  { title: "De-esser", details: "Sibilance at 6.8 kHz", enabled: true },
  { title: "Leveler", details: "Evens out quiet phrases", enabled: true },
  { title: "Limiter", details: "Target -16 LUFS, -1 dBTP", enabled: true },
];
