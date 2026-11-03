export interface StorageQuotaReport {
  usageBytes: number;
  quotaBytes: number;
  remainingBytes: number;
  usagePercent: number;
  isWarning: boolean; // >= 90%
  isCritical: boolean; // >= 95%
  isPersisted: boolean;
}

export interface StorageCleanupOption {
  id: string;
  title: string;
  description: string;
  freedBytesEstimate: number;
  freedHuman: string;
  actionLabel: string;
}

export class StorageDurability {
  /**
   * Request browser storage persistence to prevent eviction
   */
  static async requestPersistence(): Promise<boolean> {
    if (
      typeof navigator !== "undefined" &&
      navigator.storage &&
      typeof navigator.storage.persist === "function"
    ) {
      try {
        return await navigator.storage.persist();
      } catch {
        return false;
      }
    }
    return false;
  }

  /**
   * Check if storage persistence is active
   */
  static async isPersisted(): Promise<boolean> {
    if (
      typeof navigator !== "undefined" &&
      navigator.storage &&
      typeof navigator.storage.persisted === "function"
    ) {
      try {
        return await navigator.storage.persisted();
      } catch {
        return false;
      }
    }
    return false;
  }

  /**
   * Query storage quota and estimate remaining capacity
   */
  static async getQuotaReport(): Promise<StorageQuotaReport> {
    let usageBytes = 0;
    let quotaBytes = 1024 * 1024 * 1024 * 10; // 10 GB default assumption
    let isPersisted = false;

    if (typeof navigator !== "undefined" && navigator.storage) {
      try {
        if (typeof navigator.storage.persisted === "function") {
          isPersisted = await navigator.storage.persisted();
        }
        if (typeof navigator.storage.estimate === "function") {
          const estimate = await navigator.storage.estimate();
          usageBytes = estimate.usage ?? 0;
          quotaBytes = estimate.quota ?? quotaBytes;
        }
      } catch {
        // Fallback
      }
    }

    const usagePercent = quotaBytes > 0 ? (usageBytes / quotaBytes) * 100 : 0;
    const remainingBytes = Math.max(0, quotaBytes - usageBytes);

    return {
      usageBytes,
      quotaBytes,
      remainingBytes,
      usagePercent,
      isWarning: usagePercent >= 90,
      isCritical: usagePercent >= 95,
      isPersisted,
    };
  }

  /**
   * Check if the File System Access API is supported
   */
  static isFileSystemAccessSupported(): boolean {
    return typeof window !== "undefined" && "showDirectoryPicker" in window;
  }

  /**
   * Available cleanup options when storage is near quota
   */
  static getCleanupOptions(): StorageCleanupOption[] {
    return [
      {
        id: "waveform_cache",
        title: "Clear waveform caches",
        description: "Frees 3.1 GB. They rebuild when needed",
        freedBytesEstimate: 3.1 * 1024 * 1024 * 1024,
        freedHuman: "3.1 GB",
        actionLabel: "Clear",
      },
      {
        id: "ai_cache",
        title: "Remove AI results",
        description: "Frees 1.6 GB. You can redo them",
        freedBytesEstimate: 1.6 * 1024 * 1024 * 1024,
        freedHuman: "1.6 GB",
        actionLabel: "Remove",
      },
      {
        id: "move_project",
        title: "Move this project to a folder",
        description: "Frees 412 MB here",
        freedBytesEstimate: 412 * 1024 * 1024,
        freedHuman: "412 MB",
        actionLabel: "Move",
      },
    ];
  }

  /**
   * Execute cleanup action
   */
  static async executeCleanup(optionId: string): Promise<boolean> {
    if (optionId === "waveform_cache") {
      // Clear waveform / peak caches
      if (typeof caches !== "undefined") {
        try {
          await caches.delete("flux-peaks");
        } catch {
          // ignore
        }
      }
      return true;
    }
    if (optionId === "ai_cache") {
      // Clear AI temporary model outputs
      if (typeof caches !== "undefined") {
        try {
          await caches.delete("flux-ai-cache");
        } catch {
          // ignore
        }
      }
      return true;
    }
    return true;
  }
}
