/**
 * Platform Capabilities & Runtime Probing
 * Detects COOP/COEP isolation, SharedArrayBuffer, WebGPU, OPFS, and configures graceful fallbacks.
 */

export interface PlatformCapabilitiesReport {
  crossOriginIsolated: boolean;
  sharedArrayBuffer: boolean;
  audioWorklet: boolean;
  webGpu: boolean;
  opfs: boolean;
  webCodecs: boolean;
  wasmSimd: boolean;
  recommendedAudioPath: "shared-array-buffer" | "audio-worklet-message" | "script-processor";
  recommendedMlBackend: "webgpu" | "wasm-simd" | "wasm-scalar" | "cloud";
  recommendedStorageBackend: "opfs" | "indexeddb" | "memory";
}

export class PlatformCapabilitiesDetector {
  /**
   * Evaluates execution environment capabilities with safe feature checks.
   */
  public static probe(
    envOverride?: Partial<PlatformCapabilitiesReport>,
  ): PlatformCapabilitiesReport {
    const hasWindow = typeof window !== "undefined";
    const hasNavigator = typeof navigator !== "undefined";

    const crossOriginIsolated =
      envOverride?.crossOriginIsolated ?? (hasWindow && Boolean(window.crossOriginIsolated));
    const sharedArrayBuffer =
      envOverride?.sharedArrayBuffer ??
      (typeof SharedArrayBuffer !== "undefined" && crossOriginIsolated);
    const audioWorklet =
      envOverride?.audioWorklet ?? (hasWindow && typeof AudioWorkletNode !== "undefined");
    const webGpu =
      envOverride?.webGpu ?? (hasNavigator && "gpu" in navigator && Boolean(navigator.gpu));
    const opfs =
      envOverride?.opfs ??
      (hasNavigator &&
        "storage" in navigator &&
        typeof navigator.storage?.getDirectory === "function");
    const webCodecs = envOverride?.webCodecs ?? (hasWindow && "AudioEncoder" in window);
    const wasmSimd = envOverride?.wasmSimd ?? this.checkWasmSimdSupport();

    let recommendedAudioPath: "shared-array-buffer" | "audio-worklet-message" | "script-processor";
    if (envOverride?.recommendedAudioPath) {
      recommendedAudioPath = envOverride.recommendedAudioPath;
    } else if (sharedArrayBuffer && audioWorklet) {
      recommendedAudioPath = "shared-array-buffer";
    } else if (audioWorklet) {
      recommendedAudioPath = "audio-worklet-message";
    } else {
      recommendedAudioPath = "script-processor";
    }

    let recommendedMlBackend: "webgpu" | "wasm-simd" | "wasm-scalar" | "cloud";
    if (envOverride?.recommendedMlBackend) {
      recommendedMlBackend = envOverride.recommendedMlBackend;
    } else if (webGpu) {
      recommendedMlBackend = "webgpu";
    } else if (wasmSimd) {
      recommendedMlBackend = "wasm-simd";
    } else {
      recommendedMlBackend = "wasm-scalar";
    }

    let recommendedStorageBackend: "opfs" | "indexeddb" | "memory";
    if (envOverride?.recommendedStorageBackend) {
      recommendedStorageBackend = envOverride.recommendedStorageBackend;
    } else if (opfs) {
      recommendedStorageBackend = "opfs";
    } else if (hasWindow && "indexedDB" in window) {
      recommendedStorageBackend = "indexeddb";
    } else {
      recommendedStorageBackend = "memory";
    }

    return {
      crossOriginIsolated,
      sharedArrayBuffer,
      audioWorklet,
      webGpu,
      opfs,
      webCodecs,
      wasmSimd,
      recommendedAudioPath,
      recommendedMlBackend,
      recommendedStorageBackend,
    };
  }

  /**
   * Quick heuristic check for WebAssembly SIMD support.
   */
  private static checkWasmSimdSupport(): boolean {
    if (typeof WebAssembly !== "object" || typeof WebAssembly.validate !== "function") {
      return false;
    }
    // Minimal 128-bit SIMD instruction opcode sequence test
    const simdTestBytes = new Uint8Array([
      0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7b,
      0x03, 0x02, 0x01, 0x00, 0x0a, 0x0a, 0x01, 0x08, 0x00, 0xfd, 0x0c, 0x00, 0x00, 0x00, 0x00,
      0x0b,
    ]);
    return WebAssembly.validate(simdTestBytes);
  }
}
