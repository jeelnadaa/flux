/**
 * Flux Web App entry.
 */

import React from "react";
import { createRoot } from "react-dom/client";
import { FLUX_DEFAULT_THEME, FLUX_DEFAULT_FONT, initThemeAndFont } from "@flux/ui";
import { createProjectMetadata, type ProjectMetadata } from "@flux/app-core";
import { App } from "./App.js";

export interface AppState {
  theme: string;
  font: string;
  currentProject: ProjectMetadata | null;
}

export function initializeAppState(): AppState {
  const { theme, font } = initThemeAndFont();
  return {
    theme: theme || FLUX_DEFAULT_THEME,
    font: font || FLUX_DEFAULT_FONT,
    currentProject: createProjectMetadata("Untitled Session"),
  };
}

export function mountApp(container: HTMLElement): void {
  initializeAppState();
  const root = createRoot(container);
  root.render(React.createElement(App));
}

// Auto-mount if running in browser with element #root
if (typeof document !== "undefined") {
  const rootEl = document.getElementById("root");
  if (rootEl) {
    mountApp(rootEl);
  }
}

export { App };
