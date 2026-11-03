/**
 * Project template system for Flux per PLAN.md §8.3 & §Tests (Phase 8).
 * Sets up initial tracks, bus routing, sample rates, and default effect chains.
 */

export interface TrackTemplate {
  name: string;
  colorVar: string;
  pan: number;
  gainDb: number;
  initialEffects?: string[];
}

export interface ProjectTemplate {
  id: string;
  name: string;
  subtitle: string;
  icon: string;
  defaultSampleRate: number;
  tracks: TrackTemplate[];
  masterChain?: string[];
  targetLufs?: number;
}

export const PROJECT_TEMPLATES: ProjectTemplate[] = [
  {
    id: "blank",
    name: "Blank",
    subtitle: "One empty track at 48 kHz",
    icon: "wave",
    defaultSampleRate: 48000,
    tracks: [{ name: "Audio 1", colorVar: "--t4", pan: 0, gainDb: 0 }],
  },
  {
    id: "podcast",
    name: "Podcast",
    subtitle: "Two voices, music bed, −16 LUFS",
    icon: "mic",
    defaultSampleRate: 48000,
    targetLufs: -16.0,
    tracks: [
      {
        name: "Host",
        colorVar: "--t4",
        pan: -0.2,
        gainDb: -1.0,
        initialEffects: ["De-esser", "EQ"],
      },
      {
        name: "Guest",
        colorVar: "--t1",
        pan: 0.2,
        gainDb: -1.0,
        initialEffects: ["De-esser", "EQ"],
      },
      { name: "Music Bed", colorVar: "--t3", pan: 0, gainDb: -12.0 },
    ],
    masterChain: ["Compressor", "Limiter"],
  },
  {
    id: "music",
    name: "Music",
    subtitle: "Eight tracks, buses, master chain",
    icon: "mix",
    defaultSampleRate: 48000,
    targetLufs: -14.0,
    tracks: [
      { name: "Vocals", colorVar: "--t4", pan: 0, gainDb: -3.0 },
      { name: "Drums", colorVar: "--t3", pan: 0, gainDb: -2.0 },
      { name: "Bass", colorVar: "--t2", pan: 0, gainDb: -4.0 },
      { name: "Keys", colorVar: "--t1", pan: -0.3, gainDb: -6.0 },
      { name: "Guitar Lead", colorVar: "--t5", pan: 0.3, gainDb: -5.0 },
      { name: "Guitar Rhythm", colorVar: "--t6", pan: -0.4, gainDb: -8.0 },
      { name: "Synths", colorVar: "--t7", pan: 0.4, gainDb: -7.0 },
      { name: "FX / Room", colorVar: "--t8", pan: 0, gainDb: -10.0 },
    ],
    masterChain: ["EQ", "Multiband", "Limiter"],
  },
  {
    id: "voiceover",
    name: "Voiceover",
    subtitle: "Voice, room tone, de-esser",
    icon: "voice",
    defaultSampleRate: 48000,
    targetLufs: -16.0,
    tracks: [
      {
        name: "Voice",
        colorVar: "--t4",
        pan: 0,
        gainDb: 0,
        initialEffects: ["Noise reduction", "De-esser", "EQ"],
      },
      { name: "Room Tone", colorVar: "--t6", pan: 0, gainDb: -24.0 },
    ],
    masterChain: ["Limiter"],
  },
  {
    id: "lecture",
    name: "Lecture",
    subtitle: "Long recording, silence removal",
    icon: "script",
    defaultSampleRate: 44100,
    targetLufs: -16.0,
    tracks: [
      {
        name: "Speaker",
        colorVar: "--t1",
        pan: 0,
        gainDb: 0,
        initialEffects: ["Silence remover", "Leveler"],
      },
    ],
  },
  {
    id: "interview",
    name: "Interview",
    subtitle: "Two speakers, transcript ready",
    icon: "text",
    defaultSampleRate: 48000,
    targetLufs: -16.0,
    tracks: [
      { name: "Interviewer", colorVar: "--t4", pan: -0.15, gainDb: 0 },
      { name: "Interviewee", colorVar: "--t5", pan: 0.15, gainDb: 0 },
    ],
    masterChain: ["Compressor", "Limiter"],
  },
  {
    id: "8d-music",
    name: "8D music",
    subtitle: "Stereo in, orbit on, reverb room",
    icon: "orbit",
    defaultSampleRate: 48000,
    targetLufs: -14.0,
    tracks: [
      {
        name: "Stereo Input",
        colorVar: "--t3",
        pan: 0,
        gainDb: 0,
        initialEffects: ["3D and 8D", "Reverb"],
      },
    ],
  },
];

export interface InstantiatedProject {
  id: string;
  name: string;
  templateId: string;
  sampleRate: number;
  tracks: TrackTemplate[];
  masterChain: string[];
  targetLufs?: number;
  createdAt: number;
}

export class TemplateRegistry {
  public static getAll(): ProjectTemplate[] {
    return [...PROJECT_TEMPLATES];
  }

  public static getById(id: string): ProjectTemplate | undefined {
    return PROJECT_TEMPLATES.find((t) => t.id === id);
  }

  public static createProject(
    templateId: string,
    projectName: string,
    sampleRate?: number,
  ): InstantiatedProject {
    const tmpl = TemplateRegistry.getById(templateId) ?? PROJECT_TEMPLATES[0];
    return {
      id: `proj-${Math.random().toString(36).substring(2, 9)}`,
      name: projectName || tmpl.name,
      templateId: tmpl.id,
      sampleRate: sampleRate ?? tmpl.defaultSampleRate,
      tracks: tmpl.tracks.map((tr) => ({ ...tr })),
      masterChain: tmpl.masterChain ? [...tmpl.masterChain] : [],
      targetLufs: tmpl.targetLufs,
      createdAt: Date.now(),
    };
  }
}
