import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  TempoMap,
  TapTempoCalculator,
  calculateVarispeedPitch,
  calculateScaledDuration,
  formatDurationMinutesSeconds,
  DEFAULT_SPEED_PITCH_SETTINGS,
} from "../TempoMap.js";

describe("Tempo Map and Beat Grid Modeling", () => {
  it("TempoMap converts beats to samples and back with exact precision", () => {
    const tm = new TempoMap(48000, 120, [4, 4]);

    // At 120 BPM, 1 beat is 0.5s = 24,000 samples
    assert.equal(tm.beatToSample(0), 0);
    assert.equal(tm.beatToSample(1), 24000);
    assert.equal(tm.beatToSample(4), 96000);

    assert.ok(Math.abs(tm.sampleToBeat(24000) - 1.0) < 1e-5);
    assert.ok(Math.abs(tm.sampleToBeat(96000) - 4.0) < 1e-5);
  });

  it("TempoMap snaps to quarter note and sixteenth note subdivisions", () => {
    const tm = new TempoMap(48000, 120, [4, 4]);

    // Near beat 1 (sample 23800) -> snaps to 24000
    const snappedQuarter = tm.snapToSubdivision(23800, 1.0);
    assert.equal(snappedQuarter, 24000);

    // Near 1/16th (sample 6100) -> snaps to 6000
    const snapped16th = tm.snapToSubdivision(6100, 0.25);
    assert.equal(snapped16th, 6000);
  });

  it("TempoMap generates exact grid markers across time range", () => {
    const tm = new TempoMap(48000, 120, [4, 4]);
    const grid = tm.generateGrid(0, 96000, 1.0);
    assert.deepEqual(grid, [0, 24000, 48000, 72000, 96000]);
  });

  it("TapTempoCalculator computes moving average BPM and resets on timeout", () => {
    const tap = new TapTempoCalculator();

    assert.equal(tap.tap(1000), null); // First tap
    const bpm1 = tap.tap(1500); // 500ms interval -> 120 BPM
    assert.ok(bpm1 !== null && Math.abs(bpm1 - 120.0) < 0.2);

    const bpm2 = tap.tap(2000);
    assert.ok(bpm2 !== null && Math.abs(bpm2 - 120.0) < 0.2);

    // Wait 3000ms (> 2500ms timeout)
    const resetResult = tap.tap(5000);
    assert.equal(resetResult, null);
  });

  it("calculateVarispeedPitch adheres to octave log2 formula", () => {
    const doubleSpeedPitch = calculateVarispeedPitch(2.0);
    assert.ok(Math.abs(doubleSpeedPitch - 12.0) < 0.01);

    const halfSpeedPitch = calculateVarispeedPitch(0.5);
    assert.ok(Math.abs(halfSpeedPitch - -12.0) < 0.01);
  });

  it("duration formatting works accurately", () => {
    assert.equal(formatDurationMinutesSeconds(261), "4:21");
    const scaled = calculateScaledDuration(261, 0.85);
    assert.ok(scaled > 261);
    assert.ok(DEFAULT_SPEED_PITCH_SETTINGS.speedRatio === 0.85);
  });
});
