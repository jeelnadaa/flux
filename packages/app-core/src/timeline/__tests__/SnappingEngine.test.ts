import test from "node:test";
import assert from "node:assert/strict";
import { SnappingEngine } from "../SnappingEngine.js";

test("SnappingEngine: Grid snapping", () => {
  const pxPerSec = 100;
  // Grid interval = 0.5s = 50px
  // Query at 0.52s (2px from 0.5s grid line, within 8px threshold)
  const res = SnappingEngine.snap(0.52, pxPerSec, {
    thresholdPx: 8,
    gridIntervalSeconds: 0.5,
  });

  assert.equal(res.snappedTime, 0.5);
  assert.equal(res.candidate?.targetType, "grid");

  // Query at 0.65s (15px from 0.5s, 15px from 1.0s, outside 8px threshold)
  const outside = SnappingEngine.snap(0.65, pxPerSec, {
    thresholdPx: 8,
    gridIntervalSeconds: 0.5,
  });
  assert.equal(outside.snappedTime, 0.65);
  assert.equal(outside.candidate, null);
});

test("SnappingEngine: Marker and clip boundary snapping", () => {
  const pxPerSec = 100;
  const markers = [
    { time: 1.2, label: "Verse" },
    { time: 3.5, label: "Chorus" },
  ];
  const clips = [
    { startTime: 0.0, endTime: 2.0, label: "Clip 1" },
    { startTime: 2.5, endTime: 5.0, label: "Clip 2" },
  ];

  // Near marker at 1.2s (query 1.23s = 3px distance)
  const snapMarker = SnappingEngine.snap(1.23, pxPerSec, {
    thresholdPx: 8,
    markers,
    clips,
  });
  assert.equal(snapMarker.snappedTime, 1.2);
  assert.equal(snapMarker.candidate?.targetType, "marker");
  assert.equal(snapMarker.candidate?.label, "Verse");

  // Near clip boundary at 2.0s (query 1.96s = 4px distance)
  const snapClip = SnappingEngine.snap(1.96, pxPerSec, {
    thresholdPx: 8,
    markers,
    clips,
  });
  assert.equal(snapClip.snappedTime, 2.0);
  assert.equal(snapClip.candidate?.targetType, "clip_boundary");
});

test("SnappingEngine: Zero-crossing snapping", () => {
  const sr = 1000;
  const samples = new Float32Array(sr);
  // Zero crossing at sample 500 (time = 0.5s)
  samples[499] = -0.1;
  samples[500] = 0.1;

  const res = SnappingEngine.snap(0.504, 1000, {
    thresholdPx: 8,
    samples,
    sampleRate: sr,
  });

  assert.equal(res.candidate?.targetType, "zero_crossing");
  assert.ok(Math.abs(res.snappedTime - 0.5) < 0.002);
});

test("SnappingEngine: Deterministic tie-breaking priority", () => {
  const pxPerSec = 100;
  // Exactly at same distance (0px) from grid (1.0s), marker (1.0s), and zero crossing (1.0s)
  const sr = 100;
  const samples = new Float32Array(200);
  samples[99] = -0.5;
  samples[100] = 0.0; // exact zero crossing at sample 100 = 1.0s
  samples[101] = 0.5;

  const res = SnappingEngine.snap(1.0, pxPerSec, {
    thresholdPx: 8,
    gridIntervalSeconds: 1.0,
    markers: [{ time: 1.0, label: "Marker at 1.0" }],
    clips: [{ startTime: 1.0, endTime: 2.0, label: "Clip at 1.0" }],
    samples,
    sampleRate: sr,
  });

  // Zero-crossing has highest priority weight (1) over marker (2), clip (3), and grid (4)
  assert.equal(res.candidate?.targetType, "zero_crossing");
});
