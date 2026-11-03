/**
 * Internationalization, RTL support, and pseudo-localization engine for Flux
 * per PLAN.md §8.5 & §Tests (Phase 8).
 */

export type SupportedLocale = "en" | "hi" | "es" | "fr" | "de" | "pt" | "ja";

export interface LocaleDefinition {
  code: SupportedLocale;
  name: string;
  nativeName: string;
  isRtl: boolean;
}

export const SUPPORTED_LOCALES: LocaleDefinition[] = [
  { code: "en", name: "English", nativeName: "English", isRtl: false },
  { code: "hi", name: "Hindi", nativeName: "हिन्दी", isRtl: false },
  { code: "es", name: "Spanish", nativeName: "Español", isRtl: false },
  { code: "fr", name: "French", nativeName: "Français", isRtl: false },
  { code: "de", name: "German", nativeName: "Deutsch", isRtl: false },
  { code: "pt", name: "Portuguese", nativeName: "Português", isRtl: false },
  { code: "ja", name: "Japanese", nativeName: "日本語", isRtl: false },
];

const ACCENT_MAP: Record<string, string> = {
  a: "à",
  e: "é",
  i: "î",
  o: "ô",
  u: "û",
  A: "À",
  E: "É",
  I: "Î",
  O: "Ô",
  U: "Û",
};

export class I18nManager {
  private currentLocale: SupportedLocale = "en";
  private isPseudoEnabled = false;

  constructor(defaultLocale: SupportedLocale = "en") {
    this.currentLocale = defaultLocale;
  }

  public setLocale(locale: SupportedLocale): void {
    this.currentLocale = locale;
  }

  public getLocale(): SupportedLocale {
    return this.currentLocale;
  }

  public setPseudoLocalization(enabled: boolean): void {
    this.isPseudoEnabled = enabled;
  }

  public isPseudoLocalizationEnabled(): boolean {
    return this.isPseudoEnabled;
  }

  /**
   * Generates pseudo-localized text that expands strings by ~30% and adds accented
   * characters and brackets to test UI layout clipping and truncation resilience.
   */
  public static pseudoLocalize(text: string): string {
    let transformed = "";
    for (const char of text) {
      transformed += ACCENT_MAP[char] || char;
    }
    // Expand by ~30%
    const extraChars = Math.max(1, Math.floor(text.length * 0.3));
    const padding = "!".repeat(extraChars);
    return `[${transformed} ${padding}]`;
  }

  public formatTimecode(seconds: number): string {
    const totalMs = Math.round(seconds * 1000);
    const ms = totalMs % 1000;
    const totalSec = Math.floor(totalMs / 1000);
    const s = totalSec % 60;
    const m = Math.floor(totalSec / 60) % 60;
    const h = Math.floor(totalSec / 3600);

    const pad = (n: number, z = 2) => n.toString().padStart(z, "0");
    return `${pad(h)}:${pad(m)}:${pad(s)}.${pad(ms, 3)}`;
  }

  public formatLufs(lufs: number): string {
    return `${lufs > 0 ? "+" : ""}${lufs.toFixed(1)} LUFS`;
  }

  public isRtl(): boolean {
    return false; // Arabic/Hebrew in extended tier
  }
}
