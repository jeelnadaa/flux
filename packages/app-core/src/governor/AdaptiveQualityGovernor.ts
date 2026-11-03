/**
 * Adaptive Quality Governor
 * Monitors AudioWorklet xruns and dynamic processing load, stepping down quality
 * tiers to prevent buffer underruns and pops, and recovering with hysteresis.
 */

export type QualityTier = "Mastering" | "High" | "Balanced" | "PowerSave";

export interface QualityProfile {
  tier: QualityTier;
  limiterOversampling: number; // 4x, 2x, 1x
  reverbDelays: number; // 8, 4, 2
  stftWindowSize: number; // 4096, 2048, 1024
  resamplerQuality: "mastering" | "high" | "fast";
  spatialResolution: "16d" | "8d" | "2d";
  backgroundAnalysisEnabled: boolean;
}

export interface GovernorMetrics {
  totalFramesProcessed: number;
  xrunCount: number;
  recentXrunsInWindow: number;
  estimatedCpuPercent: number;
  currentTier: QualityTier;
  isLocked: boolean;
  timeInCurrentTierSeconds: number;
}

export class AdaptiveQualityGovernor {
  private currentTier: QualityTier = "High";
  private isLocked = false;
  private xrunHistory: number[] = []; // Timestamps of xrun events
  private cpuHistory: number[] = [];
  private timeInTier = 0; // seconds
  private totalFrames = 0;
  private totalXruns = 0;

  private static readonly PROFILES: Record<QualityTier, QualityProfile> = {
    Mastering: {
      tier: "Mastering",
      limiterOversampling: 4,
      reverbDelays: 8,
      stftWindowSize: 4096,
      resamplerQuality: "mastering",
      spatialResolution: "16d",
      backgroundAnalysisEnabled: true,
    },
    High: {
      tier: "High",
      limiterOversampling: 2,
      reverbDelays: 4,
      stftWindowSize: 2048,
      resamplerQuality: "high",
      spatialResolution: "8d",
      backgroundAnalysisEnabled: true,
    },
    Balanced: {
      tier: "Balanced",
      limiterOversampling: 1,
      reverbDelays: 2,
      stftWindowSize: 1024,
      resamplerQuality: "fast",
      spatialResolution: "2d",
      backgroundAnalysisEnabled: true,
    },
    PowerSave: {
      tier: "PowerSave",
      limiterOversampling: 1,
      reverbDelays: 1,
      stftWindowSize: 512,
      resamplerQuality: "fast",
      spatialResolution: "2d",
      backgroundAnalysisEnabled: false,
    },
  };

  constructor(initialTier: QualityTier = "High") {
    this.currentTier = initialTier;
  }

  public getProfile(): QualityProfile {
    return { ...AdaptiveQualityGovernor.PROFILES[this.currentTier] };
  }

  public getTier(): QualityTier {
    return this.currentTier;
  }

  public setTierOverride(tier: QualityTier, lock = true): void {
    this.currentTier = tier;
    this.isLocked = lock;
    this.timeInTier = 0;
  }

  public unlock(): void {
    this.isLocked = false;
  }

  /**
   * Reports an audio buffer underrun/xrun event with the given timestamp (seconds).
   */
  public recordXrun(timestampSeconds: number): void {
    this.totalXruns += 1;
    this.xrunHistory.push(timestampSeconds);
    this.pruneHistory(timestampSeconds);

    if (!this.isLocked && this.xrunHistory.length >= 2) {
      this.evaluateStepDown();
      this.xrunHistory = [];
    }
  }

  /**
   * Updates CPU load measurement and quantum frame count, evaluating step-up or step-down.
   */
  public update(
    cpuPercent: number,
    quantumFrames: number,
    timestampSeconds: number,
  ): QualityProfile {
    this.totalFrames += quantumFrames;
    this.cpuHistory.push(cpuPercent);
    if (this.cpuHistory.length > 30) {
      this.cpuHistory.shift();
    }
    this.timeInTier += 0.1; // simulated interval
    this.pruneHistory(timestampSeconds);

    if (!this.isLocked) {
      const avgCpu = this.getAverageCpu();
      if (avgCpu > 80.0 || this.getRecentXruns(timestampSeconds) >= 2) {
        this.evaluateStepDown();
      } else if (
        avgCpu < 40.0 &&
        this.timeInTier >= 15.0 &&
        this.getRecentXruns(timestampSeconds) === 0
      ) {
        this.evaluateStepUp();
      }
    }

    return this.getProfile();
  }

  public getMetrics(): GovernorMetrics {
    return {
      totalFramesProcessed: this.totalFrames,
      xrunCount: this.totalXruns,
      recentXrunsInWindow: this.xrunHistory.length,
      estimatedCpuPercent: this.getAverageCpu(),
      currentTier: this.currentTier,
      isLocked: this.isLocked,
      timeInCurrentTierSeconds: Math.round(this.timeInTier),
    };
  }

  private evaluateStepDown(): void {
    if (this.currentTier === "Mastering") {
      this.currentTier = "High";
      this.timeInTier = 0;
    } else if (this.currentTier === "High") {
      this.currentTier = "Balanced";
      this.timeInTier = 0;
    } else if (this.currentTier === "Balanced") {
      this.currentTier = "PowerSave";
      this.timeInTier = 0;
    }
  }

  private evaluateStepUp(): void {
    if (this.currentTier === "PowerSave") {
      this.currentTier = "Balanced";
      this.timeInTier = 0;
    } else if (this.currentTier === "Balanced") {
      this.currentTier = "High";
      this.timeInTier = 0;
    } else if (this.currentTier === "High") {
      this.currentTier = "Mastering";
      this.timeInTier = 0;
    }
  }

  private pruneHistory(nowSeconds: number): void {
    const windowStart = nowSeconds - 5.0; // 5-second sliding window
    this.xrunHistory = this.xrunHistory.filter((t) => t >= windowStart);
  }

  private getRecentXruns(nowSeconds: number): number {
    this.pruneHistory(nowSeconds);
    return this.xrunHistory.length;
  }

  private getAverageCpu(): number {
    if (this.cpuHistory.length === 0) return 0.0;
    const sum = this.cpuHistory.reduce((acc, v) => acc + v, 0);
    return sum / this.cpuHistory.length;
  }
}
