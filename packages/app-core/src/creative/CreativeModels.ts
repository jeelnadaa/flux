/**
 * Creative Effects & Voice Changer Domain Models (Phase 5).
 */

export interface SlowedReverbConfig {
  speedRatio: number; // 0.5 to 1.0 (default 0.85)
  reverbDecaySeconds: number; // 0.5 to 10.0 (default 3.2)
  brightnessDb: number; // -12 to +6 (default -3)
  keepPitch: boolean;
}

export const DEFAULT_SLOWED_REVERB: SlowedReverbConfig = {
  speedRatio: 0.85,
  reverbDecaySeconds: 3.2,
  brightnessDb: -3.0,
  keepPitch: false,
};

export interface NightcoreConfig {
  speedRatio: number; // 1.0 to 1.5 (default 1.25)
  trebleLiftDb: number; // 0 to 8 (default +2)
  chorusDepthPct: number; // 0 to 100 (default 18)
  keepPitch: boolean;
}

export const DEFAULT_NIGHTCORE: NightcoreConfig = {
  speedRatio: 1.25,
  trebleLiftDb: 2.0,
  chorusDepthPct: 18,
  keepPitch: false,
};

export interface LofiMacroConfig {
  amount: number; // 0.0 to 1.0 (default 0.64)
  lowpassHz: number; // 1000 to 20000 (default 6400)
  highpassHz: number; // 20 to 1000 (default 90)
  driveDb: number; // 0 to 12 (default +4)
  bitDepth: number; // 4 to 16 (default 12)
  sampleRateKhz: number; // 8 to 48 (default 22)
  cracklePerSec: number; // 0 to 20 (default 4)
  surfaceDb: number; // -60 to -24 (default -48)
  wowHz: number; // 0.1 to 2.0 (default 0.6)
  flutterHz: number; // 3 to 15 (default 7.5)
  pumpPct: number; // 0 to 100 (default 35)
  roomDecaySeconds: number; // 0.2 to 4.0 (default 1.1)
  noiseDb: number; // -70 to -30 (default -54)
}

export const DEFAULT_LOFI_MACRO: LofiMacroConfig = {
  amount: 0.64,
  lowpassHz: 6400,
  highpassHz: 90,
  driveDb: 4.0,
  bitDepth: 12,
  sampleRateKhz: 22,
  cracklePerSec: 4,
  surfaceDb: -48,
  wowHz: 0.6,
  flutterHz: 7.5,
  pumpPct: 35,
  roomDecaySeconds: 1.1,
  noiseDb: -54,
};

export interface VinylConfig {
  cracklePerSec: number; // default 4
  popsPerSec: number; // default 0.6
  surfaceDb: number; // default -48
  wowHz: number; // default 0.5
  flutterHz: number; // default 6.0
  warpPct: number; // default 10
  rolloffKhz: number; // default 11
  monoLowsHz: number; // default 200
  seed: number; // default 4821
}

export const DEFAULT_VINYL: VinylConfig = {
  cracklePerSec: 4,
  popsPerSec: 0.6,
  surfaceDb: -48,
  wowHz: 0.5,
  flutterHz: 6.0,
  warpPct: 10,
  rolloffKhz: 11,
  monoLowsHz: 200,
  seed: 4821,
};

export type VoiceCharacterId =
  | "deep"
  | "chipmunk"
  | "child"
  | "elder"
  | "shift_up"
  | "shift_down"
  | "whisper"
  | "announcer"
  | "radio_dj"
  | "villain"
  | "giant"
  | "tiny"
  | "robot"
  | "cyborg"
  | "alien"
  | "monster";

export interface VoiceCharacterInfo {
  id: VoiceCharacterId;
  name: string;
  subtitle: string;
  paletteClass: string;
  iconSuffix: string;
  pitchSemitones: number;
  formantShiftPct: number;
}

export const VOICE_CHARACTERS: VoiceCharacterInfo[] = [
  {
    id: "deep",
    name: "Deep",
    subtitle: "−5 st",
    paletteClass: "pal-nightcore",
    iconSuffix: "",
    pitchSemitones: -5,
    formantShiftPct: -25,
  },
  {
    id: "chipmunk",
    name: "Chipmunk",
    subtitle: "+7 st",
    paletteClass: "pal-stutter",
    iconSuffix: "",
    pitchSemitones: 7,
    formantShiftPct: 35,
  },
  {
    id: "child",
    name: "Child",
    subtitle: "+4 st, formant up",
    paletteClass: "pal-radio",
    iconSuffix: "",
    pitchSemitones: 4,
    formantShiftPct: 28,
  },
  {
    id: "elder",
    name: "Elder",
    subtitle: "Tremor, breath",
    paletteClass: "pal-lofi",
    iconSuffix: "",
    pitchSemitones: -1,
    formantShiftPct: -5,
  },
  {
    id: "shift_up",
    name: "Shift up",
    subtitle: "Higher voice",
    paletteClass: "pal-reverse",
    iconSuffix: "",
    pitchSemitones: 3,
    formantShiftPct: 15,
  },
  {
    id: "shift_down",
    name: "Shift down",
    subtitle: "Lower voice",
    paletteClass: "pal-slowed",
    iconSuffix: "slow",
    pitchSemitones: -3,
    formantShiftPct: -15,
  },
  {
    id: "whisper",
    name: "Whisper",
    subtitle: "No pitch",
    paletteClass: "pal-nextdoor",
    iconSuffix: "",
    pitchSemitones: 0,
    formantShiftPct: 10,
  },
  {
    id: "announcer",
    name: "Announcer",
    subtitle: "Low, close",
    paletteClass: "pal-cassette",
    iconSuffix: "",
    pitchSemitones: -2.5,
    formantShiftPct: -18,
  },
  {
    id: "radio_dj",
    name: "Radio DJ",
    subtitle: "Compressed",
    paletteClass: "pal-tapestop",
    iconSuffix: "",
    pitchSemitones: 0,
    formantShiftPct: 0,
  },
  {
    id: "villain",
    name: "Villain",
    subtitle: "Dark, slow",
    paletteClass: "pal-vinyl",
    iconSuffix: "",
    pitchSemitones: -4,
    formantShiftPct: -20,
  },
  {
    id: "giant",
    name: "Giant",
    subtitle: "−12 st, huge",
    paletteClass: "pal-cathedral",
    iconSuffix: "",
    pitchSemitones: -12,
    formantShiftPct: -45,
  },
  {
    id: "tiny",
    name: "Tiny",
    subtitle: "+12 st",
    paletteClass: "pal-underwater",
    iconSuffix: "",
    pitchSemitones: 12,
    formantShiftPct: 45,
  },
  {
    id: "robot",
    name: "Robot",
    subtitle: "Ring mod, comb",
    paletteClass: "pal-robot",
    iconSuffix: "bot",
    pitchSemitones: 0,
    formantShiftPct: 0,
  },
  {
    id: "cyborg",
    name: "Cyborg",
    subtitle: "Half machine",
    paletteClass: "pal-robot",
    iconSuffix: "",
    pitchSemitones: 0,
    formantShiftPct: 10,
  },
  {
    id: "alien",
    name: "Alien",
    subtitle: "Warped formants",
    paletteClass: "pal-underwater",
    iconSuffix: "",
    pitchSemitones: 5,
    formantShiftPct: -30,
  },
  {
    id: "monster",
    name: "Monster",
    subtitle: "Growl, sub",
    paletteClass: "pal-tapestop",
    iconSuffix: "",
    pitchSemitones: -8,
    formantShiftPct: -40,
  },
];
