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
