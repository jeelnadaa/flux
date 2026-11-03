/**
 * Effect Rack and Effect Chain Management for Flux.
 * Non-destructive processing model per PLAN.md §3.0.
 */

export type RackTarget = "clip" | "track" | "master";

export interface EffectInstance {
  id: string;
  pluginId: string;
  name: string;
  subtitle: string;
  colorVar: string;
  route: string;
  bypassed: boolean;
  soloed: boolean;
  mix: number; // 0 to 100%
  parameters: Record<string, number | string | boolean>;
}

export interface MacroTargetMapping {
  targetEffectId?: string;
  targetEffectName?: string;
  paramName: string;
  min: number;
  max: number;
  unit: string;
  format?: (val: number) => string;
}

export interface MacroDefinition {
  name: string;
  value: number; // 0 to 100%
  displayValue: string;
  mappings: MacroTargetMapping[];
}

export interface SerializedEffectChain {
  target: RackTarget;
  targetId?: string;
  name: string;
  effects: EffectInstance[];
  activeCompareSlot: "A" | "B" | "C";
  compareSlots: Record<"A" | "B" | "C", EffectInstance[]>;
  macros: MacroDefinition[];
}

/**
 * Standard estimated CPU load for built-in effects.
 */
export const EFFECT_CPU_ESTIMATES: Record<string, number> = {
  "de-esser": 0.8,
  "parametric-eq": 1.2,
  "graphic-eq": 1.5,
  compressor: 1.5,
  limiter: 1.0,
  reverb: 3.5,
  delay: 1.0,
  multiband: 2.8,
  gate: 0.6,
  "transient-shaper": 0.9,
  leveler: 1.1,
  speed: 2.0,
  pitch: 2.5,
  "noise-reduction": 3.2,
  "hum-removal": 1.4,
};

export class EffectChain {
  public target: RackTarget;
  public targetId?: string;
  public name: string;
  public effects: EffectInstance[];
  public activeCompareSlot: "A" | "B" | "C";
  public compareSlots: Record<"A" | "B" | "C", EffectInstance[]>;
  public macros: MacroDefinition[];

  constructor(
    target: RackTarget = "track",
    name = "Vocals chain",
    effects: EffectInstance[] = [],
    targetId?: string,
  ) {
    this.target = target;
    this.targetId = targetId;
    this.name = name;
    this.effects = effects.map((fx) => ({ ...fx, parameters: { ...fx.parameters } }));
    this.activeCompareSlot = "A";
    this.compareSlots = {
      A: this.cloneEffects(this.effects),
      B: this.cloneEffects(this.effects),
      C: this.cloneEffects(this.effects),
    };
    this.macros = [];
  }

  private cloneEffects(effects: EffectInstance[]): EffectInstance[] {
    return effects.map((fx) => ({
      ...fx,
      parameters: { ...fx.parameters },
    }));
  }

  /**
   * Adds an effect to the chain at the specified index, or appends to the end.
   */
  public addEffect(
    effect: Omit<EffectInstance, "id"> & { id?: string },
    index?: number,
  ): EffectInstance {
    const id = effect.id || `fx_${Date.now()}_${Math.random().toString(36).substring(2, 7)}`;
    const newEffect: EffectInstance = {
      ...effect,
      id,
      mix: Math.min(100, Math.max(0, effect.mix ?? 100)),
      bypassed: Boolean(effect.bypassed),
      soloed: Boolean(effect.soloed),
      parameters: { ...(effect.parameters || {}) },
    };

    if (index !== undefined && index >= 0 && index < this.effects.length) {
      this.effects.splice(index, 0, newEffect);
    } else {
      this.effects.push(newEffect);
    }

    this.saveToCompareSlot(this.activeCompareSlot);
    return newEffect;
  }

  /**
   * Removes an effect by its unique instance ID.
   */
  public removeEffect(effectId: string): boolean {
    const idx = this.effects.findIndex((e) => e.id === effectId);
    if (idx === -1) return false;
    this.effects.splice(idx, 1);
    this.saveToCompareSlot(this.activeCompareSlot);
    return true;
  }

  /**
   * Reorders an effect to a new index in the chain.
   */
  public reorderEffect(effectId: string, toIndex: number): boolean {
    const fromIndex = this.effects.findIndex((e) => e.id === effectId);
    if (fromIndex === -1) return false;
    return this.moveEffect(fromIndex, toIndex);
  }

  /**
   * Moves an effect from one index to another.
   */
  public moveEffect(fromIndex: number, toIndex: number): boolean {
    if (
      fromIndex < 0 ||
      fromIndex >= this.effects.length ||
      toIndex < 0 ||
      toIndex >= this.effects.length ||
      fromIndex === toIndex
    ) {
      return false;
    }

    const [item] = this.effects.splice(fromIndex, 1);
    this.effects.splice(toIndex, 0, item);
    this.saveToCompareSlot(this.activeCompareSlot);
    return true;
  }

  /**
   * Sets the bypass state of an effect.
   */
  public setBypass(effectId: string, bypassed: boolean): void {
    const effect = this.effects.find((e) => e.id === effectId);
    if (effect) {
      effect.bypassed = bypassed;
      this.saveToCompareSlot(this.activeCompareSlot);
    }
  }

  /**
   * Toggles the bypass state of an effect.
   */
  public toggleBypass(effectId: string): boolean {
    const effect = this.effects.find((e) => e.id === effectId);
    if (effect) {
      effect.bypassed = !effect.bypassed;
      this.saveToCompareSlot(this.activeCompareSlot);
      return effect.bypassed;
    }
    return false;
  }

  /**
   * Sets the solo state of an effect.
   */
  public setSolo(effectId: string, soloed: boolean): void {
    const effect = this.effects.find((e) => e.id === effectId);
    if (effect) {
      effect.soloed = soloed;
      this.saveToCompareSlot(this.activeCompareSlot);
    }
  }

  /**
   * Toggles the solo state of an effect.
   */
  public toggleSolo(effectId: string): boolean {
    const effect = this.effects.find((e) => e.id === effectId);
    if (effect) {
      effect.soloed = !effect.soloed;
      this.saveToCompareSlot(this.activeCompareSlot);
      return effect.soloed;
    }
    return false;
  }

  /**
   * Sets the wet/dry mix percentage (0 to 100).
   */
  public setMix(effectId: string, mixPercent: number): void {
    const effect = this.effects.find((e) => e.id === effectId);
    if (effect) {
      effect.mix = Math.min(100, Math.max(0, mixPercent));
      this.saveToCompareSlot(this.activeCompareSlot);
    }
  }

  /**
   * Sets an individual parameter value for an effect.
   */
  public setParam(effectId: string, paramKey: string, value: number | string | boolean): void {
    const effect = this.effects.find((e) => e.id === effectId);
    if (effect) {
      effect.parameters[paramKey] = value;
      this.saveToCompareSlot(this.activeCompareSlot);
    }
  }

  /**
   * Saves the current chain state into a compare slot.
   */
  public saveToCompareSlot(slot: "A" | "B" | "C"): void {
    this.compareSlots[slot] = this.cloneEffects(this.effects);
  }

  /**
   * Switches to a compare slot ('A', 'B', or 'C') and restores its saved state.
   */
  public switchCompareSlot(slot: "A" | "B" | "C"): void {
    this.activeCompareSlot = slot;
    if (this.compareSlots[slot]) {
      this.effects = this.cloneEffects(this.compareSlots[slot]);
    }
  }

  /**
   * Modulates a macro control, recalculating parameter values for all mapped targets.
   */
  public setMacroValue(macroName: string, valuePercent: number): void {
    const macro = this.macros.find((m) => m.name.toLowerCase() === macroName.toLowerCase());
    if (!macro) return;

    const clampedVal = Math.min(100, Math.max(0, valuePercent));
    macro.value = clampedVal;
    macro.displayValue = `${Math.round(clampedVal)}%`;

    const norm = clampedVal / 100;
    for (const mapping of macro.mappings) {
      const computedValue = mapping.min + (mapping.max - mapping.min) * norm;
      const targetEffect = this.effects.find(
        (e) =>
          (mapping.targetEffectId && e.id === mapping.targetEffectId) ||
          (mapping.targetEffectName &&
            e.name.toLowerCase() === mapping.targetEffectName.toLowerCase()),
      );

      if (targetEffect) {
        if (mapping.paramName.toLowerCase().includes("mix")) {
          targetEffect.mix = Math.round(computedValue);
        } else {
          targetEffect.parameters[mapping.paramName] = computedValue;
        }
      }
    }
    this.saveToCompareSlot(this.activeCompareSlot);
  }

  /**
   * Estimates total CPU consumption percentage based on active plugins.
   */
  public estimateCpuPercent(): number {
    let total = 0;
    const hasSolo = this.effects.some((e) => e.soloed);

    for (const fx of this.effects) {
      if (fx.bypassed) continue;
      if (hasSolo && !fx.soloed) continue;
      const baseEstimate = EFFECT_CPU_ESTIMATES[fx.pluginId] ?? 1.0;
      total += baseEstimate;
    }

    return Math.round(total);
  }

  /**
   * Clones this effect chain.
   */
  public clone(): EffectChain {
    const chain = new EffectChain(this.target, this.name, this.effects, this.targetId);
    chain.activeCompareSlot = this.activeCompareSlot;
    chain.compareSlots = {
      A: this.cloneEffects(this.compareSlots.A),
      B: this.cloneEffects(this.compareSlots.B),
      C: this.cloneEffects(this.compareSlots.C),
    };
    chain.macros = this.macros.map((m) => ({
      ...m,
      mappings: m.mappings.map((mp) => ({ ...mp })),
    }));
    return chain;
  }

  /**
   * Serializes the chain to a plain object.
   */
  public serialize(): SerializedEffectChain {
    return {
      target: this.target,
      targetId: this.targetId,
      name: this.name,
      effects: this.cloneEffects(this.effects),
      activeCompareSlot: this.activeCompareSlot,
      compareSlots: {
        A: this.cloneEffects(this.compareSlots.A),
        B: this.cloneEffects(this.compareSlots.B),
        C: this.cloneEffects(this.compareSlots.C),
      },
      macros: this.macros.map((m) => ({
        ...m,
        mappings: m.mappings.map((mp) => ({ ...mp })),
      })),
    };
  }

  /**
   * Deserializes a chain from a plain object.
   */
  public static deserialize(data: SerializedEffectChain): EffectChain {
    const chain = new EffectChain(data.target, data.name, data.effects, data.targetId);
    chain.activeCompareSlot = data.activeCompareSlot;
    chain.compareSlots = {
      A: chain.cloneEffects(data.compareSlots.A || []),
      B: chain.cloneEffects(data.compareSlots.B || []),
      C: chain.cloneEffects(data.compareSlots.C || []),
    };
    chain.macros = (data.macros || []).map((m) => ({
      ...m,
      mappings: (m.mappings || []).map((mp) => ({ ...mp })),
    }));
    return chain;
  }

  /**
   * Creates the standard default Vocals Chain matching design/screens-b.js:14.
   */
  public static createDefaultVocalsChain(): EffectChain {
    const defaultEffects: EffectInstance[] = [
      {
        id: "fx_deesser_1",
        pluginId: "de-esser",
        name: "De-esser",
        subtitle: "−3.2 dB at 6.8 kHz",
        route: "fx-comp",
        colorVar: "--t5",
        bypassed: false,
        soloed: false,
        mix: 100,
        parameters: { thresholdDb: -18, frequencyHz: 6800, reductionDb: -3.2 },
      },
      {
        id: "fx_eq_1",
        pluginId: "parametric-eq",
        name: "Parametric EQ",
        subtitle: "Clear voice, 5 bands",
        route: "fx-eq",
        colorVar: "--t1",
        bypassed: false,
        soloed: false,
        mix: 100,
        parameters: { bands: 5, preset: "Clear voice", band4GainDb: 3.1 },
      },
      {
        id: "fx_comp_1",
        pluginId: "compressor",
        name: "Compressor",
        subtitle: "3.2:1, 12 ms, 180 ms",
        route: "fx-comp",
        colorVar: "--t2",
        bypassed: false,
        soloed: false,
        mix: 100,
        parameters: { ratio: 3.2, attackMs: 12, releaseMs: 180 },
      },
      {
        id: "fx_reverb_1",
        pluginId: "reverb",
        name: "Reverb",
        subtitle: "Plate 1.4 s",
        route: "fx-reverb",
        colorVar: "--t3",
        bypassed: false,
        soloed: false,
        mix: 22,
        parameters: { rt60Seconds: 1.4, type: "Plate" },
      },
      {
        id: "fx_limiter_1",
        pluginId: "limiter",
        name: "Limiter",
        subtitle: "Ceiling −1.0 dBTP",
        route: "fx-comp",
        colorVar: "--t4",
        bypassed: false,
        soloed: false,
        mix: 100,
        parameters: { ceilingDbTp: -1.0 },
      },
    ];

    const chain = new EffectChain("track", "Vocals chain", defaultEffects);
    chain.macros = [
      {
        name: "Presence",
        value: 62,
        displayValue: "62%",
        mappings: [
          {
            targetEffectName: "Parametric EQ",
            paramName: "EQ band 4 gain",
            min: 0,
            max: 5,
            unit: "dB",
          },
          {
            targetEffectName: "De-esser",
            paramName: "De-esser amount",
            min: -1,
            max: -5,
            unit: "dB",
          },
          {
            targetEffectName: "Compressor",
            paramName: "Compressor mix",
            min: 60,
            max: 100,
            unit: "%",
          },
        ],
      },
    ];

    return chain;
  }
}

/**
 * Manages all effect chains across clips, tracks, and master bus.
 */
export class EffectRackManager {
  public clipChains: Map<string, EffectChain> = new Map();
  public trackChains: Map<string, EffectChain> = new Map();
  public masterChain: EffectChain;
  public activeTarget: RackTarget = "track";
  public activeTargetId?: string;

  constructor() {
    this.masterChain = new EffectChain("master", "Master bus");
    // Pre-populate default track vocals chain
    const defaultTrackChain = EffectChain.createDefaultVocalsChain();
    this.trackChains.set("trk_1", defaultTrackChain);
    this.activeTargetId = "trk_1";
  }

  public getActiveChain(): EffectChain {
    if (this.activeTarget === "master") {
      return this.masterChain;
    }
    if (this.activeTarget === "track") {
      const id = this.activeTargetId || "trk_1";
      let chain = this.trackChains.get(id);
      if (!chain) {
        chain = EffectChain.createDefaultVocalsChain();
        chain.targetId = id;
        this.trackChains.set(id, chain);
      }
      return chain;
    }
    // clip target
    const id = this.activeTargetId || "clip_1";
    let chain = this.clipChains.get(id);
    if (!chain) {
      chain = new EffectChain("clip", "Clip chain", [], id);
      this.clipChains.set(id, chain);
    }
    return chain;
  }

  public setTarget(target: RackTarget, targetId?: string): void {
    this.activeTarget = target;
    this.activeTargetId = targetId;
  }
}
