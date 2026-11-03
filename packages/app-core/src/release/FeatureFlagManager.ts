/**
 * Feature Flag & Kill Switch Manager
 * Supports remote feature toggles, staged rollout percentiles, and instant
 * emergency kill switches with verified rollback rehearsal paths.
 */

export type FeatureFlagKey =
  | "cloud_processing"
  | "voice_cloning"
  | "experimental_ml"
  | "high_sample_rates_192k"
  | "spatial_16d"
  | "neural_tts";

export interface FeatureFlagConfig {
  key: FeatureFlagKey;
  description: string;
  defaultEnabled: boolean;
  killSwitchActive: boolean;
  rolloutPercentage: number; // 0 to 100
}

export class FeatureFlagManager {
  private flags: Map<FeatureFlagKey, FeatureFlagConfig> = new Map();

  constructor() {
    this.registerDefaultFlags();
  }

  private registerDefaultFlags(): void {
    this.flags.set("cloud_processing", {
      key: "cloud_processing",
      description: "Opt-in offloading to remote cloud GPU clusters",
      defaultEnabled: false, // Off by default per privacy principles
      killSwitchActive: false,
      rolloutPercentage: 100,
    });

    this.flags.set("voice_cloning", {
      key: "voice_cloning",
      description: "Personal voice cloning with spoken consent and spread-spectrum watermark",
      defaultEnabled: false, // Strict consent required
      killSwitchActive: false,
      rolloutPercentage: 100,
    });

    this.flags.set("experimental_ml", {
      key: "experimental_ml",
      description: "Experimental neural models and on-device WebGPU acceleration",
      defaultEnabled: true,
      killSwitchActive: false,
      rolloutPercentage: 100,
    });

    this.flags.set("high_sample_rates_192k", {
      key: "high_sample_rates_192k",
      description: "96 kHz and 192 kHz high-resolution timeline playback and rendering",
      defaultEnabled: true,
      killSwitchActive: false,
      rolloutPercentage: 100,
    });

    this.flags.set("spatial_16d", {
      key: "spatial_16d",
      description: "Complex 16D spatial trajectories (Spiral, Figure-8, Swarm, Lissajous)",
      defaultEnabled: true,
      killSwitchActive: false,
      rolloutPercentage: 100,
    });

    this.flags.set("neural_tts", {
      key: "neural_tts",
      description: "Licensed neural text-to-speech synthesis with SSML markers",
      defaultEnabled: true,
      killSwitchActive: false,
      rolloutPercentage: 100,
    });
  }

  /**
   * Evaluates if a feature is enabled for a given client UUID.
   */
  public isEnabled(key: FeatureFlagKey, clientUuid?: string): boolean {
    const config = this.flags.get(key);
    if (!config) return false;
    if (config.killSwitchActive) return false;
    if (!config.defaultEnabled && config.rolloutPercentage === 0) return false;

    if (config.rolloutPercentage >= 100) {
      return config.defaultEnabled;
    }

    if (!clientUuid) {
      return config.defaultEnabled;
    }

    // Hash clientUuid to 0-99 bucket
    let hash = 0;
    for (let i = 0; i < clientUuid.length; i++) {
      hash = (hash * 31 + clientUuid.charCodeAt(i)) % 100;
    }
    return hash < config.rolloutPercentage;
  }

  /**
   * Activates emergency kill switch for a feature.
   */
  public activateKillSwitch(key: FeatureFlagKey): void {
    const config = this.flags.get(key);
    if (config) {
      config.killSwitchActive = true;
    }
  }

  /**
   * Deactivates kill switch, returning to configured rollout state.
   */
  public deactivateKillSwitch(key: FeatureFlagKey): void {
    const config = this.flags.get(key);
    if (config) {
      config.killSwitchActive = false;
    }
  }

  /**
   * Rehearses rollback execution by triggering kill switch and returning fallback action.
   */
  public rehearseRollback(key: FeatureFlagKey): { rolledBack: boolean; fallbackStrategy: string } {
    this.activateKillSwitch(key);
    const activeAfterKill = this.isEnabled(key);

    let fallbackStrategy = "standard-local-fallback";
    if (key === "cloud_processing") {
      fallbackStrategy = "local-wasm-fallback";
    } else if (key === "voice_cloning") {
      fallbackStrategy = "preset-character-voices-only";
    } else if (key === "spatial_16d") {
      fallbackStrategy = "standard-8d-orbit";
    } else if (key === "experimental_ml") {
      fallbackStrategy = "classical-dsp-filter-fallback";
    }

    return {
      rolledBack: !activeAfterKill,
      fallbackStrategy,
    };
  }
}
