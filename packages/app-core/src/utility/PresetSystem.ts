/**
 * Versioned Preset System and A/B Comparison Slots per PLAN.md §3.5.
 */

export type PresetCategory = "Effects" | "Chains" | "Stage paths" | "Export";

export interface FluxPresetFile {
  format: "fluxpreset";
  version: 1;
  id: string;
  name: string;
  category: PresetCategory;
  subtitle: string;
  createdAt: string;
  target: string;
  data: Record<string, unknown>;
}

export const FACTORY_PRESETS: FluxPresetFile[] = [
  {
    format: "fluxpreset",
    version: 1,
    id: "clear-voice-eq",
    name: "Clear voice",
    category: "Effects",
    subtitle: "EQ",
    createdAt: "2026-11-03T00:00:00Z",
    target: "fx-eq",
    data: {
      bands: [
        { type: "lowshelf", freq: 70, gain: 3.5, q: 0.71 },
        { type: "bell", freq: 280, gain: -3.0, q: 1.2 },
        { type: "bell", freq: 1200, gain: 1.6, q: 1.0 },
        { type: "bell", freq: 3400, gain: 4.0, q: 1.4 },
        { type: "highshelf", freq: 11000, gain: 3.0, q: 0.71 },
      ],
    },
  },
  {
    format: "fluxpreset",
    version: 1,
    id: "late-night-radio",
    name: "Late night radio",
    category: "Chains",
    subtitle: "Chain, 4 effects",
    createdAt: "2026-11-03T00:00:00Z",
    target: "chain",
    data: {
      effects: ["fx-eq", "fx-comp", "fx-tape", "fx-limiter"],
    },
  },
  {
    format: "fluxpreset",
    version: 1,
    id: "tape-warmth",
    name: "Tape warmth",
    category: "Effects",
    subtitle: "Cassette",
    createdAt: "2026-11-03T00:00:00Z",
    target: "fx-tape",
    data: {
      saturation: 45,
      hiss: 12,
      flutter: 18,
    },
  },
  {
    format: "fluxpreset",
    version: 1,
    id: "podcast-ready",
    name: "Podcast ready",
    category: "Chains",
    subtitle: "Chain, 6 effects",
    createdAt: "2026-11-03T00:00:00Z",
    target: "chain",
    data: {
      effects: ["fx-clean", "fx-eq", "fx-comp", "fx-deesser", "fx-leveler", "fx-limiter"],
    },
  },
  {
    format: "fluxpreset",
    version: 1,
    id: "slow-orbit",
    name: "Slow orbit",
    category: "Stage paths",
    subtitle: "Stage path",
    createdAt: "2026-11-03T00:00:00Z",
    target: "stage",
    data: {
      path: "orbit",
      speedRpm: 4.0,
      elevation: 0.0,
      distance: 1.5,
    },
  },
  {
    format: "fluxpreset",
    version: 1,
    id: "tight-drums",
    name: "Tight drums",
    category: "Effects",
    subtitle: "Compressor",
    createdAt: "2026-11-03T00:00:00Z",
    target: "fx-comp",
    data: {
      threshold: -18.0,
      ratio: 4.0,
      attackMs: 15.0,
      releaseMs: 80.0,
      kneeDb: 3.0,
    },
  },
  {
    format: "fluxpreset",
    version: 1,
    id: "dream-hall",
    name: "Dream hall",
    category: "Effects",
    subtitle: "Reverb",
    createdAt: "2026-11-03T00:00:00Z",
    target: "fx-reverb",
    data: {
      decaySec: 4.2,
      mix: 40,
      dampingHz: 4500,
      predelayMs: 35,
    },
  },
  {
    format: "fluxpreset",
    version: 1,
    id: "spoken-word",
    name: "Spoken word",
    category: "Chains",
    subtitle: "Chain, 5 effects",
    createdAt: "2026-11-03T00:00:00Z",
    target: "chain",
    data: {
      effects: ["fx-clean", "fx-eq", "fx-comp", "fx-silence", "fx-limiter"],
    },
  },
];

/**
 * Validates a parsed object as a valid Flux preset.
 */
export function validatePreset(obj: unknown): obj is FluxPresetFile {
  if (!obj || typeof obj !== "object") {
    return false;
  }
  const candidate = obj as Record<string, unknown>;
  if (candidate.format !== "fluxpreset") {
    return false;
  }
  if (candidate.version !== 1) {
    return false;
  }
  if (typeof candidate.name !== "string" || candidate.name.trim().length === 0) {
    return false;
  }
  if (typeof candidate.category !== "string") {
    return false;
  }
  if (!candidate.data || typeof candidate.data !== "object") {
    return false;
  }
  return true;
}

/**
 * Serializes a preset into formatted JSON for local export.
 */
export function exportPresetJson(preset: FluxPresetFile): string {
  return JSON.stringify(preset, null, 2);
}

/**
 * Parses and validates an imported JSON preset string.
 */
export function importPresetJson(jsonStr: string): FluxPresetFile {
  let parsed: unknown;
  try {
    parsed = JSON.parse(jsonStr);
  } catch (err) {
    throw new Error(`Failed to parse preset JSON: ${(err as Error).message}`);
  }

  if (!validatePreset(parsed)) {
    throw new Error("Invalid .fluxpreset file format or incompatible version.");
  }

  return parsed;
}

/**
 * Manages A/B comparison slots for click-free state evaluation.
 */
export class ComparisonSlotManager<T> {
  private slots: {
    A: T;
    B: T;
    C?: T;
  };
  private activeSlot: "A" | "B" | "C";

  constructor(initialState: T, slotBState?: T) {
    this.slots = {
      A: JSON.parse(JSON.stringify(initialState)),
      B: JSON.parse(JSON.stringify(slotBState ?? initialState)),
    };
    this.activeSlot = "A";
  }

  public getActiveSlot(): "A" | "B" | "C" {
    return this.activeSlot;
  }

  public getActiveState(): T {
    return JSON.parse(JSON.stringify(this.slots[this.activeSlot]!));
  }

  public updateActiveState(state: T): void {
    this.slots[this.activeSlot] = JSON.parse(JSON.stringify(state));
  }

  public selectSlot(slot: "A" | "B" | "C"): T {
    this.activeSlot = slot;
    if (!this.slots[slot]) {
      this.slots[slot] = JSON.parse(JSON.stringify(this.slots.A));
    }
    return this.getActiveState();
  }

  public copySlot(from: "A" | "B" | "C", to: "A" | "B" | "C"): void {
    if (!this.slots[from]) {
      return;
    }
    this.slots[to] = JSON.parse(JSON.stringify(this.slots[from]!));
  }
}
