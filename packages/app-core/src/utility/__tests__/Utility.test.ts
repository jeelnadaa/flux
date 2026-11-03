import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  computeLoudnessDifference,
  formatDbtp,
  formatLufs,
  DEFAULT_LOUDNESS_METRICS,
} from "../Metering.js";
import {
  FACTORY_PRESETS,
  validatePreset,
  exportPresetJson,
  importPresetJson,
  ComparisonSlotManager,
} from "../PresetSystem.js";

describe("Utility Metering and Targets", () => {
  it("computes difference against broadcast targets correctly", () => {
    // Measured -14.2 LUFS against -14.0 target
    const diff14 = computeLoudnessDifference(-14.2, "-14 LUFS");
    assert.equal(diff14, -0.2);

    // Measured -14.2 LUFS against -16.0 target
    const diff16 = computeLoudnessDifference(-14.2, "-16 LUFS");
    assert.equal(diff16, 1.8);

    // Measured -14.2 LUFS against -23.0 target
    const diff23 = computeLoudnessDifference(-14.2, "-23 LUFS");
    assert.equal(diff23, 8.8);
  });

  it("formats metrics display strings properly", () => {
    assert.equal(formatDbtp(-1.1), "-1.1 dBTP");
    assert.equal(formatDbtp(0.5), "+0.5 dBTP");
    assert.equal(formatLufs(-14.2), "-14.2 LUFS");
    assert.ok(DEFAULT_LOUDNESS_METRICS.integratedLufs < 0);
  });
});

describe("Preset System and A/B Comparison Slots", () => {
  it("validates all factory presets schema and properties", () => {
    assert.equal(FACTORY_PRESETS.length, 8);
    for (const preset of FACTORY_PRESETS) {
      assert.ok(validatePreset(preset), `Preset ${preset.name} should be valid`);
      assert.equal(preset.format, "fluxpreset");
      assert.equal(preset.version, 1);
    }
  });

  it("exports and imports presets cleanly with round-trip fidelity", () => {
    const original = FACTORY_PRESETS[0];
    const jsonStr = exportPresetJson(original);
    const imported = importPresetJson(jsonStr);

    assert.deepEqual(imported, original);
  });

  it("rejects corrupted, malformed or wrong-version presets", () => {
    assert.throws(() => importPresetJson("not json"), /Failed to parse/);

    const wrongFormat = {
      format: "otherformat",
      version: 1,
      name: "Bad",
      category: "Effects",
      data: {},
    };
    assert.throws(
      () => importPresetJson(JSON.stringify(wrongFormat)),
      /Invalid \.fluxpreset file format/,
    );

    const wrongVersion = {
      format: "fluxpreset",
      version: 99,
      name: "Future",
      category: "Effects",
      data: {},
    };
    assert.throws(
      () => importPresetJson(JSON.stringify(wrongVersion)),
      /Invalid \.fluxpreset file format/,
    );
  });

  it("ComparisonSlotManager switches, updates, and copies states seamlessly", () => {
    interface EqState {
      gain: number;
      freq: number;
    }
    const stateA: EqState = { gain: 3.0, freq: 1000 };
    const stateB: EqState = { gain: -2.0, freq: 500 };

    const manager = new ComparisonSlotManager<EqState>(stateA, stateB);
    assert.equal(manager.getActiveSlot(), "A");
    assert.deepEqual(manager.getActiveState(), stateA);

    // Switch to B
    manager.selectSlot("B");
    assert.equal(manager.getActiveSlot(), "B");
    assert.deepEqual(manager.getActiveState(), stateB);

    // Update B
    manager.updateActiveState({ gain: -4.0, freq: 400 });
    assert.deepEqual(manager.getActiveState(), { gain: -4.0, freq: 400 });

    // Switch to A and verify it was preserved
    manager.selectSlot("A");
    assert.deepEqual(manager.getActiveState(), stateA);

    // Copy B to C
    manager.copySlot("B", "C");
    manager.selectSlot("C");
    assert.deepEqual(manager.getActiveState(), { gain: -4.0, freq: 400 });
  });
});
