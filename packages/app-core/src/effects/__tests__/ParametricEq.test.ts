import { test } from "node:test";
import assert from "node:assert/strict";
import {
  calculateBiquadResponse,
  calculateCombinedResponse,
  computeMatchEqCurve,
  EQ_PRESETS,
  ISO_10_BAND_FREQUENCIES,
  ISO_31_BAND_FREQUENCIES,
  type EqBand,
} from "../ParametricEq.js";

test("ParametricEq: analytic biquad response accuracy at center frequency (<= 0.05 dB)", () => {
  const sr = 48000;
  // Peaking filter at 1000 Hz, +6.0 dB, Q=1.0
  const centerDb = calculateBiquadResponse(1000, sr, "peaking", 1000, 6.0, 1.0);
  assert.ok(
    Math.abs(centerDb - 6.0) <= 0.05,
    `Center frequency response ${centerDb} dB expected +6.0 dB within 0.05 dB`,
  );

  // Highpass filter at 1000 Hz, -3 dB at corner
  const hpCornerDb = calculateBiquadResponse(1000, sr, "highpass", 1000, 0, 0.707);
  assert.ok(
    Math.abs(hpCornerDb - -3.01) <= 0.2,
    `Highpass corner response ${hpCornerDb} dB expected -3 dB`,
  );
});

test("ParametricEq: cascaded Butterworth highpass slope meets nominal specification", () => {
  const bands: EqBand[] = [
    {
      id: "b1",
      type: "highpass",
      frequency: 1000,
      gainDb: 0,
      q: 0.707,
      slope: 24, // 24 dB/oct
      enabled: true,
      solo: false,
      colorVar: "--t3",
    },
  ];

  const freqs = [250, 500, 1000];
  const response = calculateCombinedResponse(freqs, bands, 0, 48000);

  // Corner at 1000 Hz ~ -3.0 dB
  assert.ok(Math.abs(response[2] - -3.0) < 0.6, `Corner at 1000 Hz was ${response[2]}`);

  // From 500 Hz down to 250 Hz (1 octave down in stopband): roll-off should be ~24 dB
  const octaveDrop = response[1] - response[0];
  assert.ok(
    Math.abs(octaveDrop - 24.0) < 1.0,
    `Roll-off per octave was ${octaveDrop} dB, expected ~24.0 dB`,
  );
});

test("ParametricEq: solo band audition ignores non-soloed bands", () => {
  const bands: EqBand[] = [
    {
      id: "b1",
      type: "lowshelf",
      frequency: 100,
      gainDb: 6.0,
      q: 0.707,
      slope: 12,
      enabled: true,
      solo: false,
      colorVar: "--t1",
    },
    {
      id: "b2",
      type: "peaking",
      frequency: 1000,
      gainDb: 4.0,
      q: 1.0,
      slope: 12,
      enabled: true,
      solo: true, // solo active on b2
      colorVar: "--t2",
    },
  ];

  const freqs = [100, 1000];
  const response = calculateCombinedResponse(freqs, bands, 0, 48000);

  // At 100 Hz, low shelf should be ignored because b2 is soloed
  assert.ok(
    Math.abs(response[0]) < 0.5,
    `Expected low shelf to be muted in solo mode, got ${response[0]} dB`,
  );

  // At 1000 Hz, peaking band is active (+4 dB)
  assert.ok(
    Math.abs(response[1] - 4.0) < 0.1,
    `Expected soloed band to measure ~4.0 dB, got ${response[1]} dB`,
  );
});

test("ParametricEq: Match EQ computes smoothed transfer curve", () => {
  const source = [0, 0, 0, 0, 0, 0, 0, 0];
  const reference = [0, 0, 6, 6, 6, 0, 0, 0]; // +6 dB boost in middle
  const matchCurve = computeMatchEqCurve(source, reference, 3);

  assert.equal(matchCurve.length, 8);
  assert.ok(matchCurve[3] > 4.0, `Match curve at center should be boosted, got ${matchCurve[3]}`);
  assert.ok(matchCurve[0] < 1.0, `Match curve at ends should be near zero, got ${matchCurve[0]}`);
});

test("ParametricEq: all 10 production presets are defined with valid parameters", () => {
  const expectedPresets = [
    "Clear voice",
    "Bass boost",
    "Treble boost",
    "Warm",
    "Bright",
    "Podcast",
    "Telephone-clean",
    "De-mud",
    "Presence",
    "Air",
  ];

  for (const name of expectedPresets) {
    const preset = EQ_PRESETS[name];
    assert.ok(preset, `Preset '${name}' is missing!`);
    assert.ok(preset.bands.length > 0, `Preset '${name}' has no bands!`);
    for (const b of preset.bands) {
      assert.ok(b.frequency >= 20 && b.frequency <= 20000);
      assert.ok(b.gainDb >= -24 && b.gainDb <= 24);
      assert.ok(b.q >= 0.1 && b.q <= 50.0);
    }
  }
});

test("ParametricEq: ISO standard frequencies for 10-band and 31-band graphic EQ", () => {
  assert.equal(ISO_10_BAND_FREQUENCIES.length, 10);
  assert.equal(ISO_10_BAND_FREQUENCIES[0], 31.25);
  assert.equal(ISO_10_BAND_FREQUENCIES[9], 16000);

  assert.equal(ISO_31_BAND_FREQUENCIES.length, 31);
  assert.equal(ISO_31_BAND_FREQUENCIES[0], 20);
  assert.equal(ISO_31_BAND_FREQUENCIES[30], 20000);
});
