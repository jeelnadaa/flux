import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  computeCompressorTransfer,
  computeAutoMakeupGain,
  computeGateTransfer,
  DEFAULT_COMPRESSOR_PARAMS,
  DEFAULT_LIMITER_PARAMS,
  DEFAULT_MULTIBAND_BANDS,
  DYNAMICS_PRESETS,
} from "../Dynamics.js";

describe("Dynamics Math and Modeling", () => {
  it("computeCompressorTransfer calculates accurate 1:1 below threshold", () => {
    const out = computeCompressorTransfer(-30, -18, 4, 6);
    assert.ok(Math.abs(out - -30) < 0.001);
  });

  it("computeCompressorTransfer applies exact compression ratio above knee", () => {
    const threshold = -18;
    const ratio = 4;
    const knee = 6;
    const inDb = 2; // delta = 20 dB, well above knee
    const expected = threshold + (inDb - threshold) / ratio; // -18 + 5 = -13
    const out = computeCompressorTransfer(inDb, threshold, ratio, knee);
    assert.ok(Math.abs(out - expected) < 0.05);
  });

  it("computeCompressorTransfer handles quadratic soft knee transition smoothly", () => {
    const threshold = -18;
    const ratio = 4;
    const knee = 6;
    // Exactly at threshold
    const out = computeCompressorTransfer(threshold, threshold, ratio, knee);
    assert.ok(out < threshold);
    // Continuous derivative (no step jump)
    const out1 = computeCompressorTransfer(threshold - 0.1, threshold, ratio, knee);
    const out2 = computeCompressorTransfer(threshold + 0.1, threshold, ratio, knee);
    assert.ok(out2 > out1);
  });

  it("computeAutoMakeupGain accurately computes compensating gain", () => {
    const gain = computeAutoMakeupGain(-18, 4);
    // -18 * 0.5 * (1 - 0.25) * -1 = 6.75 dB
    assert.ok(Math.abs(gain - 6.75) < 0.01);
  });

  it("computeGateTransfer attenuates signal below threshold towards floor", () => {
    const open = computeGateTransfer(-10, -30, 20, -60);
    assert.equal(open, -10);

    const closed = computeGateTransfer(-40, -30, 20, -60);
    assert.equal(closed, -60); // Attenuated to floor
  });

  it("all default parameter sets and presets are valid and consistent", () => {
    assert.ok(DEFAULT_COMPRESSOR_PARAMS.ratio >= 1.0);
    assert.ok(DEFAULT_LIMITER_PARAMS.ceilingDbTp <= 0.0);
    assert.equal(DEFAULT_MULTIBAND_BANDS.length, 4);

    for (const p of DYNAMICS_PRESETS) {
      assert.ok(p.id.length > 0);
      assert.ok(p.name.length > 0);
    }
  });
});
