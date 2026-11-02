/**
 * Flux Web App entry.
 */

import { FLUX_DEFAULT_THEME, FLUX_DEFAULT_FONT } from "@flux/ui";
import { createProjectMetadata, type ProjectMetadata } from "@flux/app-core";

export interface AppState {
  theme: string;
  font: string;
  currentProject: ProjectMetadata | null;
}

export function initializeAppState(): AppState {
  return {
    theme: FLUX_DEFAULT_THEME,
    font: FLUX_DEFAULT_FONT,
    currentProject: createProjectMetadata("Untitled Session"),
  };
}
