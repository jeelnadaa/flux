/**
 * Flux UI Design Tokens, Themes, Fonts, and Motion.
 * Verbatim alignment with DESIGN.md and design/prototype.
 */

export interface ThemeDefinition {
  id: string;
  name: string;
  ground: string;
  ink: string;
  effectClass: string;
  word: string;
  meta: string;
}

export interface FontDefinition {
  id: string;
  name: string;
  description: string;
}

export const THEMES: readonly ThemeDefinition[] = [
  {
    id: "cassette",
    name: "Cassette",
    ground: "#DCCBA3",
    ink: "#1E1A10",
    effectClass: "tape",
    word: "Cassette",
    meta: "Default",
  },
  {
    id: "slowed",
    name: "Slowed",
    ground: "#142A6E",
    ink: "#CFE0FF",
    effectClass: "slow",
    word: "Slowed",
    meta: "0.85×",
  },
  {
    id: "nightcore",
    name: "Nightcore",
    ground: "#FF2E78",
    ink: "#17030B",
    effectClass: "night",
    word: "Nightcore",
    meta: "1.25×",
  },
  {
    id: "lofi",
    name: "Lo-fi",
    ground: "#C8A179",
    ink: "#2A1A0C",
    effectClass: "lofi",
    word: "Lo-fi",
    meta: "64%",
  },
  {
    id: "vinyl",
    name: "Vinyl",
    ground: "#111013",
    ink: "#EDE6D6",
    effectClass: "vinyl",
    word: "Vinyl",
    meta: "33⅓",
  },
  {
    id: "underwater",
    name: "Underwater",
    ground: "#04424C",
    ink: "#9CF0E4",
    effectClass: "water",
    word: "Underwater",
    meta: "12 m",
  },
  {
    id: "radio",
    name: "Old radio",
    ground: "#E8A321",
    ink: "#1B1000",
    effectClass: "radio",
    word: "Old radio",
    meta: "3 kHz",
  },
  {
    id: "stutter",
    name: "Stutter",
    ground: "#F4F1E6",
    ink: "#0B0B10",
    effectClass: "stut",
    word: "S-S-Stutter",
    meta: "1/16",
  },
  {
    id: "tapestop",
    name: "Tape stop",
    ground: "#D4342A",
    ink: "#FFF6F1",
    effectClass: "stop",
    word: "Tape stop",
    meta: "1.4 s",
  },
  {
    id: "reverse",
    name: "Reverse",
    ground: "#E7E3FF",
    ink: "#21115C",
    effectClass: "rev",
    word: "Reverse",
    meta: "Region",
  },
  {
    id: "cathedral",
    name: "Cathedral",
    ground: "#17140F",
    ink: "#F2E7CF",
    effectClass: "cath",
    word: "Cathedral",
    meta: "6.8 s",
  },
  {
    id: "nextdoor",
    name: "Next door",
    ground: "#8B8C86",
    ink: "#111210",
    effectClass: "next",
    word: "Next door",
    meta: "620 Hz",
  },
  {
    id: "robot",
    name: "Robot",
    ground: "#0D2B21",
    ink: "#7CFFB2",
    effectClass: "bot",
    word: "R0B0T",
    meta: "30 Hz",
  },
] as const;

export const FONTS: readonly FontDefinition[] = [
  {
    id: "bricolage",
    name: "Bricolage Grotesque",
    description: "Rough, analog, a little condensed.",
  },
  { id: "satoshi", name: "Satoshi", description: "Smooth and friendly." },
  { id: "archivo", name: "Archivo", description: "Industrial, like hardware labels." },
  { id: "schibsted", name: "Schibsted Grotesk", description: "Sturdy, like a newspaper." },
  { id: "geist", name: "Geist", description: "Clean and technical." },
  { id: "anybody", name: "Anybody", description: "Wide and loud." },
] as const;

export const DEFAULT_THEME = "cassette";
export const DEFAULT_FONT = "bricolage";
export const FLUX_DEFAULT_THEME = DEFAULT_THEME;
export const FLUX_DEFAULT_FONT = DEFAULT_FONT;

export function getStoredTheme(): string {
  try {
    if (typeof localStorage !== "undefined") {
      const stored = localStorage.getItem("flux-theme");
      if (stored && THEMES.some((t) => t.id === stored)) return stored;
    }
  } catch {
    // Ignore storage errors
  }
  return DEFAULT_THEME;
}

export function getStoredFont(): string {
  try {
    if (typeof localStorage !== "undefined") {
      const stored = localStorage.getItem("flux-font");
      if (stored && FONTS.some((f) => f.id === stored)) return stored;
    }
  } catch {
    // Ignore storage errors
  }
  return DEFAULT_FONT;
}

export function canViewTransition(): boolean {
  if (typeof document === "undefined") return false;
  const calm =
    typeof window !== "undefined" &&
    window.matchMedia &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  return Boolean(
    "startViewTransition" in document && !calm && document.visibilityState === "visible",
  );
}

/**
 * Performs 950ms circular reveal animation per DESIGN.md §Motion using View Transitions API.
 */
export function circularReveal(
  applyFn: () => void,
  event?: { clientX?: number; clientY?: number },
): void {
  if (typeof document === "undefined") {
    applyFn();
    return;
  }

  const doc = document as Document & {
    startViewTransition?: (cb: () => void) => { ready: Promise<void>; finished: Promise<void> };
  };
  if (!canViewTransition() || typeof doc.startViewTransition !== "function") {
    applyFn();
    return;
  }

  const x = event?.clientX ?? window.innerWidth / 2;
  const y = event?.clientY ?? window.innerHeight / 2;
  const r = Math.hypot(Math.max(x, window.innerWidth - x), Math.max(y, window.innerHeight - y));
  const root = document.documentElement;

  root.classList.add("vt-theme");
  const vt = doc.startViewTransition(applyFn);

  vt.ready
    .then(() => {
      root.animate(
        {
          clipPath: [`circle(0px at ${x}px ${y}px)`, `circle(${r}px at ${x}px ${y}px)`],
        },
        {
          duration: 950,
          easing: "cubic-bezier(.16,1,.3,1)",
          pseudoElement: "::view-transition-new(root)",
        },
      );
    })
    .catch(() => {});

  vt.finished
    .catch(() => {})
    .finally(() => {
      root.classList.remove("vt-theme");
    });
}

export function applyTheme(themeId: string, event?: { clientX?: number; clientY?: number }): void {
  const target = THEMES.find((t) => t.id === themeId) ? themeId : DEFAULT_THEME;
  const apply = () => {
    if (typeof document !== "undefined") {
      document.documentElement.dataset.flux = target;
      try {
        localStorage.setItem("flux-theme", target);
      } catch {
        // Storage restricted
      }
    }
  };

  circularReveal(apply, event);
}

export function applyFont(fontId: string, event?: { clientX?: number; clientY?: number }): void {
  const fontObj = FONTS.find((f) => f.id === fontId) || FONTS[0];
  const apply = () => {
    if (typeof document !== "undefined") {
      document.documentElement.dataset.font = fontObj.id;
      try {
        localStorage.setItem("flux-font", fontObj.id);
      } catch {
        // Storage restricted
      }
    }
  };

  if (typeof document !== "undefined" && "fonts" in document) {
    document.fonts.load(`800 40px '${fontObj.name}'`).then(
      () => circularReveal(apply, event),
      () => circularReveal(apply, event),
    );
  } else {
    circularReveal(apply, event);
  }
}

export function initThemeAndFont(): { theme: string; font: string } {
  const theme = getStoredTheme();
  const font = getStoredFont();
  if (typeof document !== "undefined") {
    document.documentElement.dataset.flux = theme;
    document.documentElement.dataset.font = font;
  }
  return { theme, font };
}
