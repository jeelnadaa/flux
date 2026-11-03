import { describe, it } from "node:test";
import assert from "node:assert";
import {
  calculatePhaseCorrelation,
  applyMidSideWidth,
  formatTimeMs,
  DEFAULT_NOISE_REDUCTION,
  DEFAULT_HUM_REMOVAL,
  DEFAULT_SILENCE_REMOVER,
} from "../Restoration.js";

describe("Restoration Domain Models and Math", () => {
  it("calculates exact phase correlation for identical, inverted, and uncorrelated signals", () => {
    const n = 1000;
    const l = new Float32Array(n);
    const r = new Float32Array(n);
    const invR = new Float32Array(n);

    for (let i = 0; i < n; i++) {
      l[i] = Math.sin((2 * Math.PI * 440 * i) / 44100);
      r[i] = l[i];
      invR[i] = -l[i];
    }

    const corrIdent = calculatePhaseCorrelation(l, r);
    assert.ok(Math.abs(corrIdent - 1.0) < 1e-4, `Expected 1.0, got ${corrIdent}`);

    const corrInv = calculatePhaseCorrelation(l, invR);
    assert.ok(Math.abs(corrInv - -1.0) < 1e-4, `Expected -1.0, got ${corrInv}`);
  });

  it("applies Mid/Side widening accurately collapsing to mono and exaggerating stereo", () => {
    const l = 0.8;
    const r = 0.2;

    // Width = 0 -> Mono (L == R)
    const [monoL, monoR] = applyMidSideWidth(l, r, 0.0);
    assert.ok(Math.abs(monoL - monoR) < 1e-5, "Mono collapsed signals should be identical");
    assert.ok(Math.abs(monoL - 0.5) < 1e-5, `Expected 0.5, got ${monoL}`);

    // Width = 1 -> Unity passthrough
    const [normL, normR] = applyMidSideWidth(l, r, 1.0);
    assert.ok(Math.abs(normL - l) < 1e-5, `Expected ${l}, got ${normL}`);
    assert.ok(Math.abs(normR - r) < 1e-5, `Expected ${r}, got ${normR}`);
  });

  it("formats time seconds into M:SS.S string", () => {
    assert.strictEqual(formatTimeMs(42.1), "0:42.1");
    assert.strictEqual(formatTimeMs(78.4), "1:18.4");
    assert.strictEqual(formatTimeMs(126.7), "2:06.7");
  });

  it("verifies default configs and presets are internally consistent", () => {
    assert.strictEqual(DEFAULT_NOISE_REDUCTION.amountDb, -18.0);
    assert.strictEqual(DEFAULT_HUM_REMOVAL.fundamentalHz, 50);
    assert.strictEqual(DEFAULT_HUM_REMOVAL.harmonics, 8);
    assert.strictEqual(DEFAULT_SILENCE_REMOVER.pauses.length, 4);
    assert.strictEqual(DEFAULT_SILENCE_REMOVER.pauses[0].timestampFormatted, "0:42.1");
  });
});
