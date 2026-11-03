import test from "node:test";
import assert from "node:assert/strict";
import {
  decodeMidSide,
  encodeMidSide,
  formatSpectralSelection,
  hzToMel,
  mapAmplitudeToHeight,
  melToHz,
  TimelineViewport,
} from "../TimelineModel.js";

test("TimelineViewport coordinate and zoom math", () => {
  const vp = new TimelineViewport(100, 48000, 120, 0);

  // Time <-> Pixel roundtrip
  assert.equal(vp.timeToPixel(0), 0);
  assert.equal(vp.timeToPixel(1.5), 150);
  assert.equal(vp.pixelToTime(150), 1.5);

  // Sample <-> Pixel roundtrip
  assert.equal(vp.sampleToPixel(48000), 100);
  assert.equal(vp.pixelToSample(100), 48000);

  // isSampleLevel check
  // 100 px/sec / 48000 samples/sec = 0.00208 px/sample (not sample level)
  assert.equal(vp.isSampleLevel(), false);

  // Zoom in to sample level (e.g. 5 pixels per sample = 240,000 px/sec)
  vp.setZoom(240000, 0);
  assert.equal(vp.isSampleLevel(), true);

  // Zoom to fit
  vp.zoomToFit(1200);
  // 1200 px / 120 sec = 10 px/sec
  assert.equal(vp.pixelsPerSecond, 10);
  assert.equal(vp.scrollLeftSeconds, 0);
});

test("Mid-Side decomposition and reconstruction exact roundtrip", () => {
  const n = 1000;
  const left = new Float32Array(n);
  const right = new Float32Array(n);

  for (let i = 0; i < n; i++) {
    left[i] = Math.sin((i / n) * 40);
    right[i] = Math.cos((i / n) * 35);
  }

  const { mid, side } = encodeMidSide(left, right);
  const { left: recL, right: recR } = decodeMidSide(mid, side);

  for (let i = 0; i < n; i++) {
    const errL = Math.abs(recL[i] - left[i]);
    const errR = Math.abs(recR[i] - right[i]);
    assert.ok(errL < 1e-6, `Left channel error at ${i}: ${errL}`);
    assert.ok(errR < 1e-6, `Right channel error at ${i}: ${errR}`);
  }
});

test("Waveform amplitude vertical scale mapping (Linear vs dB)", () => {
  // Linear scale
  assert.equal(mapAmplitudeToHeight(0.0, "linear"), 0.0);
  assert.equal(mapAmplitudeToHeight(0.5, "linear"), 0.5);
  assert.equal(mapAmplitudeToHeight(1.0, "linear"), 1.0);
  assert.equal(mapAmplitudeToHeight(-0.8, "linear"), 0.8);

  // dB scale (-60 dBFS to 0 dBFS)
  assert.equal(mapAmplitudeToHeight(1.0, "db", -60), 1.0); // 0 dBFS -> 1.0
  assert.equal(mapAmplitudeToHeight(0.001, "db", -60), 0.0); // -60 dBFS -> 0.0
  assert.equal(mapAmplitudeToHeight(0.0001, "db", -60), 0.0); // Below floor -> 0.0

  // -20 dBFS (amplitude 0.1) should map to (-20 - (-60)) / 60 = 40/60 = 0.6667
  const h20 = mapAmplitudeToHeight(0.1, "db", -60);
  assert.ok(Math.abs(h20 - 40 / 60) < 1e-3);
});

test("Mel scale conversions roundtrip", () => {
  const testFreqs = [20, 100, 440, 1000, 5000, 10000, 20000];
  for (const f of testFreqs) {
    const mel = hzToMel(f);
    const recF = melToHz(mel);
    assert.ok(Math.abs(recF - f) < 1e-2, `Mel roundtrip failed for ${f} Hz: got ${recF}`);
  }
});

test("Spectral selection badge formatting", () => {
  const badge = formatSpectralSelection({
    startTime: 1.0,
    endTime: 2.42,
    minFreq: 2100,
    maxFreq: 4800,
  });
  assert.equal(badge, "2.1 to 4.8 kHz · 1.42 s");

  const badgeLow = formatSpectralSelection({
    startTime: 0,
    endTime: 5.0,
    minFreq: 80,
    maxFreq: 800,
  });
  assert.equal(badgeLow, "80 to 800 Hz · 5.00 s");
});
