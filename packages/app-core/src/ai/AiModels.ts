/**
 * Domain models, configurations, and utilities for AI restoration,
 * stem separation, voice cleanup, transcription, fillers, and cloud management.
 */

export type ModelTier = "Fast" | "Balanced" | "Best";

export interface MlModel {
  id: string;
  title: string;
  sizeBytes: number;
  sizeFormatted: string;
  sha256Hash: string;
  isDownloaded: boolean;
  tier: ModelTier;
  domain: "stems" | "cleanup" | "transcript" | "analysis" | "tts";
}

export type StemType = "Vocals" | "Drums" | "Bass" | "Other";

export interface SeparatedStem {
  type: StemType;
  colorToken: string;
  seed: number;
  kind: "voice" | "drums" | "bass" | "mix";
  gainDb: number;
  defaultLevel: number;
  isMuted: boolean;
  isSoloed: boolean;
}

export interface StemSeparationConfig {
  mode: "Vocals and music" | "4 stems" | "6 stems";
  quality: ModelTier;
  previewFirst30Sec: boolean;
}

export interface VoiceCleanupConfig {
  studioSoundAmount: number; // 0 to 100
  removeNoise: boolean;
  noiseAmount: number; // 0 to 100
  removeEcho: boolean;
  echoAmount: number; // 0 to 100
  rebuildMissingHighs: boolean;
  matchStudioMic: boolean;
  deEss: boolean;
  evenOutLevel: boolean;
  keepRoomToneDb: number; // e.g. -20 dB
}

export interface WordToken {
  text: string;
  startSec: number;
  endSec: number;
  confidence: number;
  speakerId: number;
  isFiller: boolean;
  isCut: boolean;
  isLowConfidence: boolean;
}

export interface TranscriptBlock {
  speakerName: string;
  speakerColor: string;
  timestampStr: string;
  words: WordToken[];
}

export interface TranscriptDocument {
  language: string;
  blocks: TranscriptBlock[];
  customVocabulary: string[];
}

export type FillerType = "UmUh" | "Like" | "YouKnow" | "Stutter" | "Custom";

export interface FillerOccurrence {
  id: string;
  contextHtml: string;
  timestampStr: string;
  timestampSec: number;
  durationSec: number;
  fillerType: FillerType;
  label: string;
  selected: boolean;
  confidence: number;
}

export type CloudJobStatus = "Queued" | "Running" | "Completed" | "Failed";

export interface CloudJob {
  id: string;
  title: string;
  details: string;
  progressPct: number;
  status: CloudJobStatus;
  isActive: boolean;
  autoDeleteHours: number;
}

export interface PrivacySettings {
  stemsBestCloud: boolean;
  ttsCloud: boolean;
  dubbingCloud: boolean;
  crashReports: boolean;
  usageStats: boolean;
}

export const DEFAULT_MODELS: MlModel[] = [
  {
    id: "stems_balanced",
    title: "Stems: Balanced",
    sizeBytes: 820 * 1024 * 1024,
    sizeFormatted: "820 MB",
    sha256Hash: "a3f5c9e1b2d4f6a80c2e4f6a80c2e4f6a80c2e4f6a80c2e4f6a80c2e4f6a80c2",
    isDownloaded: true,
    tier: "Balanced",
    domain: "stems",
  },
  {
    id: "voice_cleanup",
    title: "Voice cleanup",
    sizeBytes: 64 * 1024 * 1024,
    sizeFormatted: "64 MB",
    sha256Hash: "b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2",
    isDownloaded: true,
    tier: "Balanced",
    domain: "cleanup",
  },
  {
    id: "transcript_whisper_small",
    title: "Transcript: Whisper small",
    sizeBytes: 240 * 1024 * 1024,
    sizeFormatted: "240 MB",
    sha256Hash: "c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3",
    isDownloaded: true,
    tier: "Fast",
    domain: "transcript",
  },
  {
    id: "tempo_key",
    title: "Tempo and key",
    sizeBytes: 18 * 1024 * 1024,
    sizeFormatted: "18 MB",
    sha256Hash: "d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4",
    isDownloaded: true,
    tier: "Fast",
    domain: "analysis",
  },
  {
    id: "stems_best",
    title: "Stems: Best",
    sizeBytes: 1945 * 1024 * 1024,
    sizeFormatted: "1.9 GB",
    sha256Hash: "e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5",
    isDownloaded: false,
    tier: "Best",
    domain: "stems",
  },
  {
    id: "transcript_whisper_medium",
    title: "Transcript: Whisper medium",
    sizeBytes: 780 * 1024 * 1024,
    sizeFormatted: "780 MB",
    sha256Hash: "f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6",
    isDownloaded: false,
    tier: "Balanced",
    domain: "transcript",
  },
  {
    id: "speech_voices_en",
    title: "Speech voices (English)",
    sizeBytes: 310 * 1024 * 1024,
    sizeFormatted: "310 MB",
    sha256Hash: "a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7",
    isDownloaded: false,
    tier: "Fast",
    domain: "tts",
  },
];

export const DEFAULT_STEMS: SeparatedStem[] = [
  {
    type: "Vocals",
    colorToken: "--t4",
    seed: 7,
    kind: "voice",
    gainDb: 0.0,
    defaultLevel: 78,
    isMuted: false,
    isSoloed: false,
  },
  {
    type: "Drums",
    colorToken: "--t3",
    seed: 3,
    kind: "drums",
    gainDb: -1.0,
    defaultLevel: 70,
    isMuted: false,
    isSoloed: false,
  },
  {
    type: "Bass",
    colorToken: "--t2",
    seed: 4,
    kind: "bass",
    gainDb: -2.0,
    defaultLevel: 66,
    isMuted: false,
    isSoloed: false,
  },
  {
    type: "Other",
    colorToken: "--t5",
    seed: 5,
    kind: "mix",
    gainDb: -3.0,
    defaultLevel: 60,
    isMuted: false,
    isSoloed: false,
  },
];

export const DEFAULT_VOICE_CLEANUP: VoiceCleanupConfig = {
  studioSoundAmount: 60,
  removeNoise: true,
  noiseAmount: 72,
  removeEcho: true,
  echoAmount: 50,
  rebuildMissingHighs: false,
  matchStudioMic: true,
  deEss: true,
  evenOutLevel: true,
  keepRoomToneDb: -20.0,
};

export const DEFAULT_TRANSCRIPT: TranscriptDocument = {
  language: "en",
  customVocabulary: ["Kessler", "Harbor Lights"],
  blocks: [
    {
      speakerName: "Maya",
      speakerColor: "--t4",
      timestampStr: "00:12:04",
      words: [
        {
          text: "So,",
          startSec: 724.0,
          endSec: 724.3,
          confidence: 0.94,
          speakerId: 1,
          isFiller: true,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "um,",
          startSec: 724.35,
          endSec: 724.65,
          confidence: 0.92,
          speakerId: 1,
          isFiller: true,
          isCut: true,
          isLowConfidence: false,
        },
        {
          text: "the",
          startSec: 724.7,
          endSec: 724.85,
          confidence: 0.98,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "first",
          startSec: 724.88,
          endSec: 725.15,
          confidence: 0.97,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "thing",
          startSec: 725.18,
          endSec: 725.42,
          confidence: 0.96,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "I",
          startSec: 725.45,
          endSec: 725.55,
          confidence: 0.99,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "noticed",
          startSec: 725.58,
          endSec: 725.95,
          confidence: 0.95,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "about",
          startSec: 725.98,
          endSec: 726.25,
          confidence: 0.98,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "the",
          startSec: 726.28,
          endSec: 726.4,
          confidence: 0.99,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "harbor",
          startSec: 726.42,
          endSec: 726.85,
          confidence: 0.96,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "recordings",
          startSec: 726.88,
          endSec: 727.45,
          confidence: 0.97,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "was",
          startSec: 727.48,
          endSec: 727.68,
          confidence: 0.98,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "how",
          startSec: 727.7,
          endSec: 727.9,
          confidence: 0.99,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "much",
          startSec: 727.92,
          endSec: 728.18,
          confidence: 0.97,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "low",
          startSec: 728.2,
          endSec: 728.48,
          confidence: 0.96,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "rumble",
          startSec: 728.5,
          endSec: 728.85,
          confidence: 0.95,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "came",
          startSec: 728.88,
          endSec: 729.12,
          confidence: 0.98,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "through",
          startSec: 729.15,
          endSec: 729.4,
          confidence: 0.99,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "the",
          startSec: 729.42,
          endSec: 729.58,
          confidence: 0.99,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "hull.",
          startSec: 729.6,
          endSec: 729.98,
          confidence: 0.96,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "You",
          startSec: 730.02,
          endSec: 730.2,
          confidence: 0.99,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "could",
          startSec: 730.22,
          endSec: 730.45,
          confidence: 0.97,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "feel",
          startSec: 730.48,
          endSec: 730.75,
          confidence: 0.98,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "it",
          startSec: 730.78,
          endSec: 730.95,
          confidence: 0.99,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "before",
          startSec: 730.98,
          endSec: 731.32,
          confidence: 0.97,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "you",
          startSec: 731.35,
          endSec: 731.52,
          confidence: 0.99,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "could",
          startSec: 731.55,
          endSec: 731.78,
          confidence: 0.98,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "hear",
          startSec: 731.8,
          endSec: 732.1,
          confidence: 0.65,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: true,
        },
        {
          text: "it.",
          startSec: 732.12,
          endSec: 732.35,
          confidence: 0.98,
          speakerId: 1,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
      ],
    },
    {
      speakerName: "Tom",
      speakerColor: "--t3",
      timestampStr: "00:12:31",
      words: [
        {
          text: "And",
          startSec: 751.0,
          endSec: 751.2,
          confidence: 0.98,
          speakerId: 2,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "that",
          startSec: 751.22,
          endSec: 751.45,
          confidence: 0.99,
          speakerId: 2,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "was",
          startSec: 751.48,
          endSec: 751.68,
          confidence: 0.98,
          speakerId: 2,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "on",
          startSec: 751.7,
          endSec: 751.88,
          confidence: 0.97,
          speakerId: 2,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "the",
          startSec: 751.9,
          endSec: 752.1,
          confidence: 0.95,
          speakerId: 2,
          isFiller: true,
          isCut: true,
          isLowConfidence: false,
        },
        {
          text: "the",
          startSec: 752.12,
          endSec: 752.32,
          confidence: 0.95,
          speakerId: 2,
          isFiller: true,
          isCut: true,
          isLowConfidence: false,
        },
        {
          text: "first",
          startSec: 752.35,
          endSec: 752.65,
          confidence: 0.98,
          speakerId: 2,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "night,",
          startSec: 752.68,
          endSec: 753.05,
          confidence: 0.97,
          speakerId: 2,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "you know,",
          startSec: 753.1,
          endSec: 753.55,
          confidence: 0.92,
          speakerId: 2,
          isFiller: true,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "before",
          startSec: 753.6,
          endSec: 753.95,
          confidence: 0.98,
          speakerId: 2,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "we",
          startSec: 753.98,
          endSec: 754.12,
          confidence: 0.99,
          speakerId: 2,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "moved",
          startSec: 754.15,
          endSec: 754.48,
          confidence: 0.97,
          speakerId: 2,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "the",
          startSec: 754.5,
          endSec: 754.65,
          confidence: 0.99,
          speakerId: 2,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "mics",
          startSec: 754.68,
          endSec: 755.02,
          confidence: 0.98,
          speakerId: 2,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "up",
          startSec: 755.05,
          endSec: 755.22,
          confidence: 0.99,
          speakerId: 2,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "to",
          startSec: 755.25,
          endSec: 755.4,
          confidence: 0.99,
          speakerId: 2,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "the",
          startSec: 755.42,
          endSec: 755.58,
          confidence: 0.98,
          speakerId: 2,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
        {
          text: "Kessler",
          startSec: 755.6,
          endSec: 756.02,
          confidence: 0.62,
          speakerId: 2,
          isFiller: false,
          isCut: false,
          isLowConfidence: true,
        },
        {
          text: "deck.",
          startSec: 756.05,
          endSec: 756.4,
          confidence: 0.97,
          speakerId: 2,
          isFiller: false,
          isCut: false,
          isLowConfidence: false,
        },
      ],
    },
  ],
};

export const DEFAULT_FILLERS: FillerOccurrence[] = [
  {
    id: "f1",
    contextHtml: 'So <b class="sig">um</b> the first thing',
    timestampStr: "00:12:04",
    timestampSec: 724.35,
    durationSec: 0.3,
    fillerType: "UmUh",
    label: "Um",
    selected: true,
    confidence: 0.96,
  },
  {
    id: "f2",
    contextHtml: 'on <b class="sig">the the</b> first night',
    timestampStr: "00:12:33",
    timestampSec: 753.12,
    durationSec: 0.42,
    fillerType: "Stutter",
    label: "Stutter",
    selected: true,
    confidence: 0.98,
  },
  {
    id: "f3",
    contextHtml: 'first night, <b class="sig">you know,</b> before we',
    timestampStr: "00:12:36",
    timestampSec: 756.1,
    durationSec: 0.45,
    fillerType: "YouKnow",
    label: "You know",
    selected: true,
    confidence: 0.91,
  },
  {
    id: "f4",
    contextHtml: 'it was <b class="sig">uh</b> around two',
    timestampStr: "00:14:02",
    timestampSec: 842.2,
    durationSec: 0.28,
    fillerType: "UmUh",
    label: "Uh",
    selected: true,
    confidence: 0.95,
  },
  {
    id: "f5",
    contextHtml: 'we were <b class="sig">like</b> forty feet away',
    timestampStr: "00:15:18",
    timestampSec: 918.4,
    durationSec: 0.25,
    fillerType: "Like",
    label: "Like, not sure",
    selected: false,
    confidence: 0.62,
  },
  {
    id: "f6",
    contextHtml: 'and <b class="sig">I I I</b> think that',
    timestampStr: "00:17:44",
    timestampSec: 1064.15,
    durationSec: 0.55,
    fillerType: "Stutter",
    label: "Stutter",
    selected: true,
    confidence: 0.99,
  },
];

export const DEFAULT_CLOUD_JOBS: CloudJob[] = [
  {
    id: "job1",
    title: "Stems: Harbor Lights",
    details: "On device · 3 min left",
    progressPct: 62,
    status: "Running",
    isActive: true,
    autoDeleteHours: 0,
  },
  {
    id: "job2",
    title: "Transcript: Interview ep. 12",
    details: "On device · Waiting",
    progressPct: 0,
    status: "Queued",
    isActive: false,
    autoDeleteHours: 0,
  },
  {
    id: "job3",
    title: "Dub to Spanish",
    details: "Cloud · 1 min left · Deletes in 24 h",
    progressPct: 84,
    status: "Running",
    isActive: true,
    autoDeleteHours: 24,
  },
  {
    id: "job4",
    title: "Export: Harbor Lights master.flac",
    details: "Done 4 min ago",
    progressPct: 100,
    status: "Completed",
    isActive: false,
    autoDeleteHours: 0,
  },
];

export const DEFAULT_PRIVACY_SETTINGS: PrivacySettings = {
  stemsBestCloud: false,
  ttsCloud: false,
  dubbingCloud: false,
  crashReports: false,
  usageStats: false,
};

export function calculateTimeSaved(fillers: FillerOccurrence[]): number {
  return fillers.filter((f) => f.selected).reduce((sum, f) => sum + f.durationSec, 0);
}

export function formatTimeSaved(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = Math.round(seconds % 60);
  return `${mins}:${secs < 10 ? "0" : ""}${secs}`;
}
