import test from "node:test";
import assert from "node:assert/strict";
import { FadeCurves } from "../FadeCurves.js";

test("FadeCurves: mathematical curve formulas evaluate accurately", () => {
  // Test endpoints for all curve types
  const curveTypes = ["linear", "equalPower", "sCurve", "logarithmic", "exponential"] as const;

  for (const curve of curveTypes) {
    const fadeIn0 = FadeCurves.evaluate(curve, 0.0, true);
    const fadeIn1 = FadeCurves.evaluate(curve, 1.0, true);
    const fadeOut0 = FadeCurves.evaluate(curve, 0.0, false);
    const fadeOut1 = FadeCurves.evaluate(curve, 1.0, false);

    assert.ok(Math.abs(fadeIn0 - 0.0) < 1e-5, `${curve} fadeIn(0) should be 0.0`);
    assert.ok(Math.abs(fadeIn1 - 1.0) < 1e-5, `${curve} fadeIn(1) should be 1.0`);
    assert.ok(Math.abs(fadeOut0 - 1.0) < 1e-5, `${curve} fadeOut(0) should be 1.0`);
    assert.ok(Math.abs(fadeOut1 - 0.0) < 1e-5, `${curve} fadeOut(1) should be 0.0`);
  }

  // Linear midpoint
  assert.equal(FadeCurves.evaluate("linear", 0.5, true), 0.5);

  // Equal power midpoint: sin(pi/4) = 1/sqrt(2) approx 0.7071
  const eqMid = FadeCurves.evaluate("equalPower", 0.5, true);
  assert.ok(Math.abs(eqMid - Math.SQRT1_2) < 1e-4);

  // S-Curve midpoint: 3*(0.5)^2 - 2*(0.5)^3 = 0.75 - 0.25 = 0.5
  assert.equal(FadeCurves.evaluate("sCurve", 0.5, true), 0.5);
});

test("FadeCurves: equal-power crossfade conserves power (gainIn^2 + gainOut^2 == 1.0)", () => {
  const steps = 100;
  for (let i = 0; i <= steps; i++) {
    const t = i / steps;
    const { gainFadeIn, gainFadeOut } = FadeCurves.evaluateEqualPowerCrossfade(t);
    const totalPower = gainFadeIn * gainFadeIn + gainFadeOut * gainFadeOut;

    // Power must equal 1.0 within +-0.0001 (constant energy)
    assert.ok(
      Math.abs(totalPower - 1.0) < 1e-5,
      `Equal-power crossfade power violation at t=${t}: power=${totalPower}`,
    );
  }
});

test("FadeCurves: equal-gain crossfade conserves linear amplitude", () => {
  const steps = 50;
  for (let i = 0; i <= steps; i++) {
    const t = i / steps;
    const { gainFadeIn, gainFadeOut } = FadeCurves.evaluateEqualGainCrossfade(t);
    assert.ok(
      Math.abs(gainFadeIn + gainFadeOut - 1.0) < 1e-6,
      `Equal-gain crossfade sum violation at t=${t}: ${gainFadeIn + gainFadeOut}`,
    );
  }
});

test("FadeCurves: automatic micro-fade eliminates boundary clicks without distortion", () => {
  const sampleRate = 48000;
  const bufferLen = 480; // 10ms
  const buffer = new Float32Array(bufferLen).fill(1.0); // sharp DC step

  FadeCurves.applyMicroFade(buffer, sampleRate, 2.0, true, true);

  // Start must begin at 0.0 and ramp up
  assert.equal(buffer[0], 0.0);
  assert.ok(buffer[1] > 0.0);
  assert.ok(buffer[1] < buffer[10]);

  // Middle must remain unattenuated (1.0)
  assert.equal(buffer[Math.floor(bufferLen / 2)], 1.0);

  // End must ramp down to 0.0
  const fadeSamples = Math.round((2.0 / 1000.0) * sampleRate);
  assert.ok(buffer[bufferLen - 1] < 0.05);
  assert.ok(buffer[bufferLen - fadeSamples] > 0.9);
});
