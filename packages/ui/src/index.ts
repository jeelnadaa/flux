/**
 * Flux UI package - design tokens and components.
 */

export const FLUX_DEFAULT_THEME = "cassette";
export const FLUX_DEFAULT_FONT = "bricolage";

export interface ThemeConfig {
  id: string;
  name: string;
  ground: string;
  ink: string;
}

export const CASSETTE_THEME: ThemeConfig = {
  id: "cassette",
  name: "Cassette",
  ground: "#DCCBA3",
  ink: "#1E1A10",
};
