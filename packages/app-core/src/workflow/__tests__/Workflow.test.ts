import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  BatchProcessor,
  RingtoneMaker,
  MacroEngine,
  TemplateRegistry,
  A11yAudioDescription,
  I18nManager,
} from "../index.js";

describe("Phase 8 Workflow & Pro Tools Domain Models", () => {
  // --------------------------------------------------------------------------
  // 1. Batch Processing Engine
  // --------------------------------------------------------------------------
  describe("BatchProcessor", () => {
    it("processes 200 files with 0 unhandled failures and isolates damaged files", async () => {
      const processor = new BatchProcessor({ maxConcurrency: 4 });

      // Queue 200 files, with 5 synthetic corrupt/damaged files interspersed
      for (let i = 1; i <= 200; i++) {
        const isDamaged = i % 40 === 0; // 5 damaged files
        processor.addFile(
          `episode-${i.toString().padStart(3, "0")}.wav`,
          10_000_000 + i * 1000,
          180.0,
          isDamaged,
        );
      }

      assert.equal(processor.files.length, 200);

      const result = await processor.processAll();

      assert.equal(result.totalFiles, 200);
      assert.equal(result.completedCount, 195);
      assert.equal(result.skippedCount, 5);
      assert.equal(result.failedCount, 0);

      // Verify metrics on completed files
      const completed = processor.files.filter((f) => f.status === "done");
      for (const item of completed) {
        assert.ok(item.metrics);
        assert.equal(item.metrics.integratedLufs, -16.0);
        assert.equal(item.metrics.truePeakDbtp, -1.0);
        assert.ok(item.metrics.outputSizeBytes > 0);
        assert.ok(item.metrics.processedDurationSec > 0);
      }

      // Verify damaged files were reported cleanly
      const skipped = processor.files.filter((f) => f.status === "skipped");
      assert.equal(skipped.length, 5);
      for (const item of skipped) {
        assert.equal(item.statusText, "Skipped: file is damaged");
      }
    });

    it("allows adding and removing processing steps", () => {
      const processor = new BatchProcessor();
      const initialStepCount = processor.steps.length;

      processor.addStep({
        id: "step-custom",
        type: "normalize",
        label: "Normalize to −14 LUFS",
      });
      assert.equal(processor.steps.length, initialStepCount + 1);

      processor.removeStep("step-custom");
      assert.equal(processor.steps.length, initialStepCount);
    });
  });

  // --------------------------------------------------------------------------
  // 2. Ringtone Maker Engine
  // --------------------------------------------------------------------------
  describe("RingtoneMaker", () => {
    it("validates strict platform constraints (iOS <= 30.0s, Android)", () => {
      // Valid iOS ringtone
      const validIos = RingtoneMaker.validate({
        platform: "iPhone",
        startTimeSec: 15.0,
        durationSec: 29.5,
        fadeInSec: 0.3,
        fadeOutSec: 1.0,
        levelForSpeaker: true,
        loopSmoothly: true,
      });
      assert.equal(validIos.valid, true);
      assert.equal(validIos.errors.length, 0);

      // Invalid iOS ringtone (> 30.0s)
      const invalidIos = RingtoneMaker.validate({
        platform: "iPhone",
        startTimeSec: 0.0,
        durationSec: 35.0,
        fadeInSec: 0.3,
        fadeOutSec: 1.0,
        levelForSpeaker: true,
        loopSmoothly: true,
      });
      assert.equal(invalidIos.valid, false);
      assert.ok(
        invalidIos.errors.some((e) =>
          e.includes("iOS ringtones (.m4r) must not exceed 30 seconds"),
        ),
      );

      // Overlapping fades
      const invalidFades = RingtoneMaker.validate({
        platform: "Android",
        startTimeSec: 0.0,
        durationSec: 2.0,
        fadeInSec: 1.5,
        fadeOutSec: 1.5,
        levelForSpeaker: false,
        loopSmoothly: false,
      });
      assert.equal(invalidFades.valid, false);
      assert.ok(
        invalidFades.errors.some((e) =>
          e.includes("Combined fade durations cannot exceed ringtone length"),
        ),
      );
    });

    it("scans and detects the catchiest section based on peak energy", () => {
      const sampleRate = 1000; // 1kHz synthetic SR
      const totalSeconds = 90;
      const samples = new Float32Array(totalSeconds * sampleRate);

      // Create a quiet track with a high-energy "chorus" at 40s to 70s
      for (let i = 0; i < samples.length; i++) {
        samples[i] = 0.05 * Math.sin((2 * Math.PI * 440 * i) / sampleRate);
      }
      // Boost chorus (40s to 70s = 30s)
      for (let i = 40 * sampleRate; i < 70 * sampleRate; i++) {
        samples[i] = 0.9 * Math.sin((2 * Math.PI * 440 * i) / sampleRate);
      }

      const catchy = RingtoneMaker.findCatchiestPart(samples, sampleRate, 30.0);
      assert.equal(catchy.durationSec, 30.0);
      assert.ok(
        Math.abs(catchy.startTimeSec - 40.0) <= 1.0,
        `Expected start near 40s, got ${catchy.startTimeSec}`,
      );
    });

    it("makes loop click-free with equal power crossfade", () => {
      const sampleRate = 48000;
      const samples = new Float32Array(sampleRate * 2); // 2 seconds
      for (let i = 0; i < samples.length; i++) {
        samples[i] = Math.sin((2 * Math.PI * 220 * i) / sampleRate);
      }

      const smoothed = RingtoneMaker.makeLoopClickFree(samples, sampleRate, 50.0);
      assert.equal(smoothed.length, samples.length);
      // The boundary samples should not produce NaN or infinite jumps
      for (let i = 0; i < smoothed.length; i++) {
        assert.ok(!Number.isNaN(smoothed[i]));
      }
    });

    it("applies speaker frequency leveling and saturation", () => {
      const input = new Float32Array([0.0, 0.5, 1.0, 2.0, -2.0]);
      const leveled = RingtoneMaker.applySpeakerLeveling(input);
      assert.equal(leveled.length, input.length);
      for (let i = 0; i < leveled.length; i++) {
        assert.ok(Math.abs(leveled[i]) <= 1.0, "Speaker leveling should softly clamp to <= 1.0");
      }
    });
  });

  // --------------------------------------------------------------------------
  // 3. Macro & Action Recording Engine
  // --------------------------------------------------------------------------
  describe("MacroEngine", () => {
    it("loads built-in macros including Podcast cleanup and Trim & normalize", () => {
      const engine = new MacroEngine();
      const all = engine.getAll();
      assert.ok(all.length >= 3);

      const podcast = engine.getById("macro-podcast-cleanup");
      assert.ok(podcast);
      assert.equal(podcast.steps.length, 6);
      assert.equal(podcast.shortcut, "Ctrl Shift 1");

      const trim = engine.getById("macro-trim-normalize");
      assert.ok(trim);
      assert.equal(trim.steps.length, 3);
    });

    it("records custom macro action and re-plays deterministically with identical hash", () => {
      const engine = new MacroEngine();
      engine.startRecording("My Vocal Prep");
      assert.equal(engine.isRecordingActive(), true);

      engine.recordStep("highpass", "Highpass at 80 Hz", { freq: 80 });
      engine.recordStep("compressor", "Opto compression", { ratio: 3.5, thresholdDb: -18 });
      engine.recordStep("air_boost", "Air band +2.5 dB", { freq: 12000, gainDb: 2.5 });

      const custom = engine.stopRecording("Ctrl Shift 2");
      assert.equal(engine.isRecordingActive(), false);
      assert.ok(custom);
      assert.equal(custom.name, "My Vocal Prep");
      assert.equal(custom.steps.length, 3);
      assert.equal(custom.shortcut, "Ctrl Shift 2");

      // Replay must be 100% deterministic (identical hash)
      const replay1 = engine.replay(custom.id, 999);
      const replay2 = engine.replay(custom.id, 999);
      assert.equal(replay1.stepsExecuted, 3);
      assert.equal(replay1.finalHash, replay2.finalHash);
    });
  });

  // --------------------------------------------------------------------------
  // 4. Project Templates
  // --------------------------------------------------------------------------
  describe("TemplateRegistry", () => {
    it("provides all standard templates from DESIGN.md", () => {
      const templates = TemplateRegistry.getAll();
      assert.ok(templates.length >= 7);

      const expectedIds = [
        "blank",
        "podcast",
        "music",
        "voiceover",
        "lecture",
        "interview",
        "8d-music",
      ];
      for (const id of expectedIds) {
        assert.ok(
          templates.some((t) => t.id === id),
          `Missing template: ${id}`,
        );
      }
    });

    it("creates an instantiated project matching template structure", () => {
      const proj = TemplateRegistry.createProject("podcast", "My Weekly Show", 48000);
      assert.equal(proj.name, "My Weekly Show");
      assert.equal(proj.templateId, "podcast");
      assert.equal(proj.sampleRate, 48000);
      assert.equal(proj.tracks.length, 3);
      assert.equal(proj.tracks[0].name, "Host");
      assert.equal(proj.tracks[1].name, "Guest");
      assert.equal(proj.tracks[2].name, "Music Bed");
      assert.ok(proj.masterChain.includes("Limiter"));
      assert.equal(proj.targetLufs, -16.0);
    });
  });

  // --------------------------------------------------------------------------
  // 5. Accessibility Audio Description
  // --------------------------------------------------------------------------
  describe("A11yAudioDescription", () => {
    it("generates natural language waveform summary for screen readers", () => {
      const sampleRate = 1000;
      const durationSec = 60;
      const samples = new Float32Array(durationSec * sampleRate);

      // Intro (0 to 10s): quiet
      for (let i = 0; i < 10 * sampleRate; i++) {
        samples[i] = 0.02 * Math.sin((2 * Math.PI * 440 * i) / sampleRate);
      }
      // Chorus (10 to 40s): loud
      for (let i = 10 * sampleRate; i < 40 * sampleRate; i++) {
        samples[i] = 0.85 * Math.sin((2 * Math.PI * 440 * i) / sampleRate);
      }
      // Outro (40 to 60s): moderate
      for (let i = 40 * sampleRate; i < 60 * sampleRate; i++) {
        samples[i] = 0.25 * Math.sin((2 * Math.PI * 440 * i) / sampleRate);
      }

      const description = A11yAudioDescription.describeWaveform(samples, sampleRate);
      assert.ok(description.includes("duration"));
      assert.ok(description.includes("dBFS"));
      assert.ok(description.length > 20);
    });

    it("handles empty audio gracefully", () => {
      const desc = A11yAudioDescription.describeWaveform(new Float32Array(0), 48000);
      assert.ok(desc.includes("Empty"));
    });
  });

  // --------------------------------------------------------------------------
  // 6. Internationalization & Pseudo-localization
  // --------------------------------------------------------------------------
  describe("I18nManager", () => {
    it("provides pseudo-localization with +30% expansion and accented vowels", () => {
      const original = "Format converter";
      const pseudo = I18nManager.pseudoLocalize(original);

      assert.ok(pseudo.startsWith("["));
      assert.ok(pseudo.endsWith("]"));
      assert.ok(pseudo.length > original.length * 1.25);
      // Contains accented characters
      assert.ok(pseudo.includes("ô") || pseudo.includes("à") || pseudo.includes("é"));
    });

    it("formats timecode and LUFS consistently across locales", () => {
      const i18n = new I18nManager("en");
      const tc = i18n.formatTimecode(42.318);
      assert.equal(tc, "00:00:42.318");

      const lufs = i18n.formatLufs(-14.2);
      assert.equal(lufs, "-14.2 LUFS");
    });
  });
});
