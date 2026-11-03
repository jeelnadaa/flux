import test, { describe } from "node:test";
import assert from "node:assert";
import {
  PlatformCapabilitiesDetector,
  AdaptiveQualityGovernor,
  ChaosSuite,
  SoakHarness,
  SignedModelManifest,
  CloudLoadTester,
  FeatureFlagManager,
  ModelManifestEntry,
} from "../index.js";

describe("Phase 9 Hardening & Platform Quality", () => {
  describe("Platform Capabilities & Fallbacks", () => {
    test("probes capabilities and determines optimal audio, ML, and storage backends", () => {
      // Test isolated environment
      const isolatedReport = PlatformCapabilitiesDetector.probe({
        crossOriginIsolated: true,
        sharedArrayBuffer: true,
        audioWorklet: true,
        webGpu: true,
        opfs: true,
        webCodecs: true,
        wasmSimd: true,
      });

      assert.strictEqual(isolatedReport.crossOriginIsolated, true);
      assert.strictEqual(isolatedReport.sharedArrayBuffer, true);
      assert.strictEqual(isolatedReport.recommendedAudioPath, "shared-array-buffer");
      assert.strictEqual(isolatedReport.recommendedMlBackend, "webgpu");
      assert.strictEqual(isolatedReport.recommendedStorageBackend, "opfs");

      // Test fallback environment (no crossOriginIsolated, no WebGPU, no OPFS)
      const fallbackReport = PlatformCapabilitiesDetector.probe({
        crossOriginIsolated: false,
        sharedArrayBuffer: false,
        audioWorklet: true,
        webGpu: false,
        opfs: false,
        webCodecs: false,
        wasmSimd: true,
        recommendedAudioPath: "audio-worklet-message",
        recommendedMlBackend: "wasm-simd",
        recommendedStorageBackend: "indexeddb",
      });

      assert.strictEqual(fallbackReport.crossOriginIsolated, false);
      assert.strictEqual(fallbackReport.sharedArrayBuffer, false);
      assert.strictEqual(fallbackReport.recommendedAudioPath, "audio-worklet-message");
      assert.strictEqual(fallbackReport.recommendedMlBackend, "wasm-simd");
      assert.strictEqual(fallbackReport.recommendedStorageBackend, "indexeddb");
    });
  });

  describe("Adaptive Quality Governor", () => {
    test("steps down quality tiers under high CPU load or xruns", () => {
      const governor = new AdaptiveQualityGovernor("High");
      assert.strictEqual(governor.getTier(), "High");
      assert.strictEqual(governor.getProfile().limiterOversampling, 2);

      // Simulate 2 xruns within window
      governor.recordXrun(1.0);
      governor.recordXrun(1.5);
      assert.strictEqual(governor.getTier(), "Balanced");
      assert.strictEqual(governor.getProfile().reverbDelays, 2);

      // Two more xruns force PowerSave tier
      governor.recordXrun(2.0);
      governor.recordXrun(2.5);
      assert.strictEqual(governor.getTier(), "PowerSave");
      assert.strictEqual(governor.getProfile().backgroundAnalysisEnabled, false);
    });

    test("recovers quality tier with hysteresis under low CPU load", () => {
      const governor = new AdaptiveQualityGovernor("Balanced");

      // Feed low CPU (<40%) over 160 ticks (16 virtual seconds)
      let profile = governor.getProfile();
      for (let i = 0; i < 160; i++) {
        profile = governor.update(25.0, 128, 10.0 + i * 0.1);
      }

      assert.strictEqual(governor.getTier(), "High");
      assert.strictEqual(profile.reverbDelays, 4);
    });

    test("manual lock prevents automatic tier alterations", () => {
      const governor = new AdaptiveQualityGovernor("Mastering");
      governor.setTierOverride("Mastering", true);

      // Bombard with xruns
      governor.recordXrun(1.0);
      governor.recordXrun(2.0);
      governor.recordXrun(3.0);

      // Must remain locked to Mastering
      assert.strictEqual(governor.getTier(), "Mastering");
      assert.strictEqual(governor.getMetrics().isLocked, true);
    });
  });

  describe("Chaos Suite & Resilience", () => {
    test("executes 100 fault injections with zero data loss and 100% recovery", async () => {
      const summary = await ChaosSuite.run(100, 12345);

      assert.strictEqual(summary.totalTrials, 100);
      assert.strictEqual(summary.dataLossIncidents, 0, "Zero data loss incidents allowed");
      assert.strictEqual(summary.unhandledExceptions, 0, "Zero unhandled exceptions allowed");
      assert.strictEqual(summary.passedTrials, 100, "All trials must pass and recover");
    });
  });

  describe("Soak Simulator & Memory Stability", () => {
    test("4-hour soak simulation demonstrates memory growth <= 5% and zero leaks", () => {
      const metrics = SoakHarness.simulate(14400, 48000, 128);

      assert.strictEqual(metrics.durationSecondsSimulated, 14400);
      assert.strictEqual(metrics.status, "passed");
      assert.strictEqual(metrics.memoryLeakDetected, false);
      assert.ok(
        metrics.memoryGrowthRatio <= 0.05,
        `Memory growth ratio ${metrics.memoryGrowthRatio} exceeds 5% threshold`,
      );
    });
  });

  describe("Signed Model Manifest & Supply Chain Security", () => {
    test("verifies official models and detects tampering", () => {
      const models = SignedModelManifest.getOfficialModels();
      assert.ok(models.length >= 5);

      // Valid check
      const htdemucs = models.find((m: ModelManifestEntry) => m.modelId === "htdemucs-4stems")!;
      assert.ok(htdemucs);
      const isValid = SignedModelManifest.verifyModelIntegrity(
        htdemucs.modelId,
        htdemucs.expectedSha256,
      );
      assert.strictEqual(isValid, true);

      // Tampered check (altered hash)
      assert.throws(
        () => {
          SignedModelManifest.verifyModelIntegrity(
            htdemucs.modelId,
            "0000000000000000000000000000000000000000000000000000000000000000",
          );
        },
        /tampering/i,
        "Must reject tampered weights",
      );

      // Untrusted check
      assert.throws(
        () => {
          SignedModelManifest.verifyModelIntegrity("malicious-model-xyz", "abc");
        },
        /not registered/i,
        "Must reject unlisted model",
      );
    });
  });

  describe("Cloud Load Testing & SLA SLOs", () => {
    test("simulates 500 concurrent cloud jobs and validates p95 latency and 24h TTL", () => {
      const report = CloudLoadTester.simulate(500, 50);

      assert.strictEqual(report.totalJobs, 500);
      assert.strictEqual(report.successfulJobs, 500);
      assert.strictEqual(report.failedJobs, 0);
      assert.strictEqual(report.sloMet, true, "p95 queue latency must be < 60s");
      assert.ok(report.p95QueueLatencySeconds < 60.0);
      assert.strictEqual(
        report.ephemeralDeletionVerified,
        true,
        "All jobs must enforce 24h deletion TTL",
      );
    });
  });

  describe("Feature Flag Manager & Rollback Rehearsal", () => {
    test("defaults match privacy rules and kill switches allow instant rollback", () => {
      const flags = new FeatureFlagManager();

      // Privacy checks: cloud and cloning off by default
      assert.strictEqual(flags.isEnabled("cloud_processing"), false);
      assert.strictEqual(flags.isEnabled("voice_cloning"), false);

      // Standard tools on
      assert.strictEqual(flags.isEnabled("spatial_16d"), true);
      assert.strictEqual(flags.isEnabled("neural_tts"), true);

      // Rollback rehearsal test
      const rollbackResult = flags.rehearseRollback("spatial_16d");
      assert.strictEqual(rollbackResult.rolledBack, true);
      assert.strictEqual(rollbackResult.fallbackStrategy, "standard-8d-orbit");
      assert.strictEqual(flags.isEnabled("spatial_16d"), false);
    });
  });
});
