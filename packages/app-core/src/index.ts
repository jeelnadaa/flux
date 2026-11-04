/**
 * Flux App Core - project model, command system, state management.
 */

export interface ProjectMetadata {
  id: string;
  name: string;
  sampleRate: number;
  channelCount: number;
  createdAt: number;
  updatedAt: number;
}

export function createProjectMetadata(name: string, sampleRate = 48000): ProjectMetadata {
  const now = Date.now();
  return {
    id: `project_${now}`,
    name,
    sampleRate,
    channelCount: 2,
    createdAt: now,
    updatedAt: now,
  };
}

export * from "./analysis/AnalysisManager.js";
export * from "./capture/AudioCaptureManager.js";
export * from "./engine/EngineController.js";
export * from "./export/ExportManager.js";
export * from "./timeline/TimelineModel.js";
export * from "./timeline/SnappingEngine.js";
export * from "./editing/FadeCurves.js";
export * from "./editing/EditingModel.js";
export * from "./project/Command.js";
export * from "./project/ProjectJournal.js";
export * from "./project/FluxPackage.js";
export * from "./project/StorageDurability.js";
export * from "./input/ShortcutManager.js";
export * from "./effects/EffectRack.js";
export * from "./effects/ParametricEq.js";
export * from "./effects/Dynamics.js";
export * from "./tempo/TempoMap.js";
export * from "./restoration/Restoration.js";
export * from "./utility/Metering.js";
export * from "./utility/PresetSystem.js";
export * from "./spatial/SpatialModels.js";
export * from "./creative/CreativeModels.js";
export * from "./ai/AiModels.js";
export * from "./ai/AdvancedAiModels.js";
export * from "./ai/ByokAiManager.js";
export * from "./platform/NativeBridge.js";
export * from "./state/ProjectStore.js";
export * from "./workflow/index.js";
export * from "./hardening/index.js";
