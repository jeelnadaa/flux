import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { ProjectStore } from "../ProjectStore.js";
import { TransportState } from "../../engine/EngineController.js";

describe("ProjectStore", () => {
  it("initializes with demo tracks and sample-accurate timeline bounds", () => {
    const store = ProjectStore.get();
    const state = store.getState();

    assert.equal(state.name, "Harbor Lights");
    assert.equal(state.sampleRate, 48000);
    assert.ok(state.tracks.length >= 4);
    assert.ok(state.totalFrames > 0);
    assert.equal(state.transportState, TransportState.Stopped);
  });

  it("handles seeking and looping boundaries accurately", () => {
    const store = ProjectStore.get();
    store.seek(24000); // 0.5s at 48k
    assert.equal(store.getState().playheadFrame, 24000);

    store.setLoop(true, 10000, 50000);
    assert.equal(store.getState().loopEnabled, true);
    assert.equal(store.getState().loopStartFrame, 10000);
    assert.equal(store.getState().loopEndFrame, 50000);

    store.stop();
    assert.equal(store.getState().playheadFrame, 10000); // returns to loop start
  });

  it("splits selected clip cleanly without loss", () => {
    const store = ProjectStore.get();
    const track = store.getState().tracks[0];
    const initialClips = track.clips.length;
    const clip = track.clips[0];

    store.selectClip(clip.id);
    const splitPoint = clip.startFrame + Math.floor(clip.durationFrames / 2);
    store.seek(splitPoint);
    store.splitSelectedClip();

    assert.equal(track.clips.length, initialClips + 1);
    assert.equal(track.clips[0].durationFrames + track.clips[1].durationFrames, clip.durationFrames);
  });

  it("renders offline mixdown correctly into interleaved float buffer", () => {
    const store = ProjectStore.get();
    const mix = store.renderMixdown(0, 48000);

    assert.equal(mix.numChannels, 2);
    assert.equal(mix.sampleRate, 48000);
    assert.equal(mix.interleaved.length, 48000 * 2);

    let hasNonZero = false;
    for (let i = 0; i < mix.interleaved.length; i++) {
      if (Math.abs(mix.interleaved[i]) > 1e-4) {
        hasNonZero = true;
        break;
      }
    }
    assert.ok(hasNonZero, "Mixdown should contain synthesized audio");
  });

  it("supports non-destructive undo and redo for editing commands", () => {
    const store = ProjectStore.get();
    const track = store.getState().tracks[0];
    const initialClips = track.clips.length;
    const clip = track.clips[0];

    store.selectClip(clip.id);
    const splitPoint = clip.startFrame + Math.floor(clip.durationFrames / 2);
    store.seek(splitPoint);
    store.splitSelectedClip();

    assert.equal(store.getState().tracks[0].clips.length, initialClips + 1);
    assert.ok(store.canUndo());

    // Undo the split
    const undone = store.undo();
    assert.ok(undone);
    assert.equal(store.getState().tracks[0].clips.length, initialClips);
    assert.ok(store.canRedo());

    // Redo the split
    const redone = store.redo();
    assert.ok(redone);
    assert.equal(store.getState().tracks[0].clips.length, initialClips + 1);
  });

  it("duplicates and moves clips accurately", () => {
    const store = ProjectStore.get();
    const track = store.getState().tracks[0];
    const initialClips = track.clips.length;
    const clip = track.clips[0];

    store.selectClip(clip.id);
    const dup = store.duplicateSelectedClip();
    assert.ok(dup);
    assert.equal(store.getState().tracks[0].clips.length, initialClips + 1);

    store.moveClip(dup.id, 96000);
    const moved = store.getState().tracks[0].clips.find((c) => c.id === dup.id);
    assert.equal(moved?.startFrame, 96000);
  });
});
