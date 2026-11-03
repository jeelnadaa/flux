import test from "node:test";
import assert from "node:assert/strict";
import { Clip, TimelineEditor } from "../EditingModel.js";

test("TimelineEditor: splitClip produces sample-accurate non-destructive sub-clips", () => {
  const editor = new TimelineEditor();
  const track = editor.addTrack("Vocals");
  const clip = editor.addClip({
    trackId: track.id,
    name: "Verse 1",
    sourceId: "src-1",
    sourceDurationSeconds: 60.0,
    sourceOffsetSeconds: 10.0,
    startSeconds: 5.0,
    durationSeconds: 20.0,
    gainDb: 1.5,
    reversed: false,
    fadeInDurationSeconds: 1.0,
    fadeInCurve: "equalPower",
    fadeOutDurationSeconds: 2.0,
    fadeOutCurve: "sCurve",
  });

  const splitResult = editor.splitClip(clip.id, 12.0); // 7s into the 20s clip
  assert.ok(splitResult !== null);
  const [left, right] = splitResult;

  // Left sub-clip
  assert.equal(left.startSeconds, 5.0);
  assert.equal(left.durationSeconds, 7.0);
  assert.equal(left.sourceOffsetSeconds, 10.0);
  assert.equal(left.gainDb, 1.5);

  // Right sub-clip
  assert.equal(right.startSeconds, 12.0);
  assert.equal(right.durationSeconds, 13.0);
  assert.equal(right.sourceOffsetSeconds, 17.0); // 10.0 + 7.0
  assert.equal(right.gainDb, 1.5);

  // Sum of durations must exactly equal original duration
  assert.equal(left.durationSeconds + right.durationSeconds, 20.0);
});

test("TimelineEditor: cut, copy, paste operations preserve clip data", () => {
  const editor = new TimelineEditor();
  const t1 = editor.addTrack("Lead");
  const t2 = editor.addTrack("Backing");
  const clip = editor.addClip({
    trackId: t1.id,
    name: "Chorus",
    sourceId: "src-2",
    sourceDurationSeconds: 30.0,
    sourceOffsetSeconds: 5.0,
    startSeconds: 15.0,
    durationSeconds: 10.0,
    gainDb: -2.0,
    reversed: true,
    fadeInDurationSeconds: 0.5,
    fadeInCurve: "linear",
    fadeOutDurationSeconds: 0.5,
    fadeOutCurve: "linear",
  });

  // Copy clip
  const copied = editor.copyClip(clip.id);
  assert.ok(copied !== null);
  assert.equal(copied.name, "Chorus");

  // Paste onto track 2 at 40.0s
  const pasted = editor.pasteClip(t2.id, 40.0);
  assert.ok(pasted !== null);
  assert.equal(pasted.trackId, t2.id);
  assert.equal(pasted.startSeconds, 40.0);
  assert.equal(pasted.durationSeconds, 10.0);
  assert.equal(pasted.reversed, true);
  assert.equal(pasted.gainDb, -2.0);

  // Cut clip from track 1
  const cutClip = editor.cutClip(clip.id);
  assert.ok(cutClip !== null);
  assert.equal(
    editor.getState().clips.find((c: Clip) => c.id === clip.id),
    undefined,
  );
});

test("TimelineEditor: delete with ripple shifts following clips on the same track", () => {
  const editor = new TimelineEditor();
  const t = editor.addTrack("Drums");
  const c1 = editor.addClip({
    trackId: t.id,
    name: "Beat 1",
    sourceId: "src-3",
    sourceDurationSeconds: 100,
    sourceOffsetSeconds: 0,
    startSeconds: 0,
    durationSeconds: 10,
    gainDb: 0,
    reversed: false,
    fadeInDurationSeconds: 0,
    fadeInCurve: "linear",
    fadeOutDurationSeconds: 0,
    fadeOutCurve: "linear",
  });
  const c2 = editor.addClip({
    trackId: t.id,
    name: "Beat 2",
    sourceId: "src-3",
    sourceDurationSeconds: 100,
    sourceOffsetSeconds: 10,
    startSeconds: 10,
    durationSeconds: 15,
    gainDb: 0,
    reversed: false,
    fadeInDurationSeconds: 0,
    fadeInCurve: "linear",
    fadeOutDurationSeconds: 0,
    fadeOutCurve: "linear",
  });
  const c3 = editor.addClip({
    trackId: t.id,
    name: "Beat 3",
    sourceId: "src-3",
    sourceDurationSeconds: 100,
    sourceOffsetSeconds: 25,
    startSeconds: 25,
    durationSeconds: 20,
    gainDb: 0,
    reversed: false,
    fadeInDurationSeconds: 0,
    fadeInCurve: "linear",
    fadeOutDurationSeconds: 0,
    fadeOutCurve: "linear",
  });

  // Delete c2 with ripple: c3 should shift from 25s to 10s (shifted by c2 duration = 15s)
  editor.deleteClip(c2.id, true);

  const state = editor.getState();
  assert.equal(state.clips.length, 2);
  const remainingC1 = state.clips.find((c: Clip) => c.id === c1.id);
  const remainingC3 = state.clips.find((c: Clip) => c.id === c3.id);
  assert.equal(remainingC1?.startSeconds, 0);
  assert.equal(remainingC3?.startSeconds, 10); // 25 - 15 = 10
});

test("TimelineEditor: slip-edit and reverse operations", () => {
  const editor = new TimelineEditor();
  const t = editor.addTrack("Guitar");
  const clip = editor.addClip({
    trackId: t.id,
    name: "Riff",
    sourceId: "src-4",
    sourceDurationSeconds: 60.0,
    sourceOffsetSeconds: 10.0,
    startSeconds: 20.0,
    durationSeconds: 15.0,
    gainDb: 0,
    reversed: false,
    fadeInDurationSeconds: 0,
    fadeInCurve: "linear",
    fadeOutDurationSeconds: 0,
    fadeOutCurve: "linear",
  });

  // Slip forward by +5s: source offset increases from 10s to 15s, start stays 20s
  editor.slipClip(clip.id, 5.0);
  const state1 = editor.getState();
  const c1 = state1.clips.find((c: Clip) => c.id === clip.id);
  assert.equal(c1?.sourceOffsetSeconds, 15.0);
  assert.equal(c1?.startSeconds, 20.0);
  assert.equal(c1?.durationSeconds, 15.0);

  // Reverse clip: non-destructive toggle
  editor.reverseClip(clip.id);
  const state2 = editor.getState();
  const c2 = state2.clips.find((c: Clip) => c.id === clip.id);
  assert.equal(c2?.reversed, true);
});

test("TimelineEditor: automation lanes with linear, smooth, and hold interpolation", () => {
  const editor = new TimelineEditor();
  const t = editor.addTrack("Synth");
  const lane = editor.addAutomationLane(t.id, "volume", "linear");

  // Add automation points: at 0s = 0.0, at 10s = 1.0
  editor.addAutomationPoint(lane.id, 0.0, 0.0);
  editor.addAutomationPoint(lane.id, 10.0, 1.0);

  // Linear evaluation at midpoint (5s) -> 0.5
  assert.equal(editor.evaluateAutomation(lane, 5.0), 0.5);

  // Hold interpolation at midpoint (5s) -> 0.0
  lane.interpolation = "hold";
  assert.equal(editor.evaluateAutomation(lane, 5.0), 0.0);
  assert.equal(editor.evaluateAutomation(lane, 9.99), 0.0);
  assert.equal(editor.evaluateAutomation(lane, 10.0), 1.0);

  // Smooth interpolation at midpoint (5s) -> 0.5
  lane.interpolation = "smooth";
  assert.equal(editor.evaluateAutomation(lane, 5.0), 0.5);
  // At 2.5s (t=0.25): 3*(0.25)^2 - 2*(0.25)^3 = 3*0.0625 - 2*0.015625 = 0.1875 - 0.03125 = 0.15625
  assert.ok(Math.abs(editor.evaluateAutomation(lane, 2.5) - 0.15625) < 1e-4);
});
