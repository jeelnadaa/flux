import test from "node:test";
import assert from "node:assert/strict";
import {
  CommandManager,
  MoveClipCommand,
  SetClipGainCommand,
  SetTrackControlsCommand,
  computeProjectHash,
} from "../Command.js";
import { TimelineEditor } from "../../editing/EditingModel.js";
import type { ProjectState } from "../../editing/EditingModel.js";

function createTestState(): ProjectState {
  const editor = new TimelineEditor({ sampleRate: 48000 });
  const t1 = editor.addTrack("Vocals", "--t4");
  t1.id = "trk_1";
  const t2 = editor.addTrack("Drums", "--t3");
  t2.id = "trk_2";
  let state = editor.getState();
  state = {
    ...state,
    clips: [
      {
        id: "clip_vocals_1",
        trackId: t1.id,
        name: "Vocals Lead",
        sourceId: "src_1",
        sourceDurationSeconds: 120,
        sourceOffsetSeconds: 0,
        startSeconds: 10.0,
        durationSeconds: 30.0,
        gainDb: 0.0,
        reversed: false,
        fadeInDurationSeconds: 0.05,
        fadeInCurve: "equalPower",
        fadeOutDurationSeconds: 0.1,
        fadeOutCurve: "equalPower",
      },
      {
        id: "clip_drums_1",
        trackId: t2.id,
        name: "Drums Loop",
        sourceId: "src_2",
        sourceDurationSeconds: 60,
        sourceOffsetSeconds: 0,
        startSeconds: 0.0,
        durationSeconds: 40.0,
        gainDb: -1.5,
        reversed: false,
        fadeInDurationSeconds: 0.01,
        fadeInCurve: "equalPower",
        fadeOutDurationSeconds: 0.01,
        fadeOutCurve: "equalPower",
      },
    ],
  };
  return state;
}

test("Command System: maintains project state hash across do and undo (single command)", () => {
  const initialState = createTestState();
  const initialHash = computeProjectHash(initialState);
  const mgr = new CommandManager(initialState);

  assert.equal(mgr.currentHash, initialHash);
  assert.equal(mgr.canUndo, false);

  // Execute MoveClipCommand
  const cmd = new MoveClipCommand("clip_vocals_1", "trk_1", 10.0, "trk_1", 15.5, "Vocals Lead");
  mgr.execute(cmd);

  assert.equal(mgr.canUndo, true);
  assert.notEqual(mgr.currentHash, initialHash);
  assert.equal(mgr.state.clips[0].startSeconds, 15.5);

  // Undo
  const undone = mgr.undo();
  assert.equal(undone, true);
  assert.equal(mgr.state.clips[0].startSeconds, 10.0);
  assert.equal(mgr.currentHash, initialHash);

  // Redo
  const redone = mgr.redo();
  assert.equal(redone, true);
  assert.equal(mgr.state.clips[0].startSeconds, 15.5);
});

test("Command System Property Test: 25 edits undone all returns bit-exact initial hash; undo then redo gives identical hash", () => {
  const initialState = createTestState();
  const initialHash = computeProjectHash(initialState);
  const mgr = new CommandManager(initialState);

  const commandHistory: string[] = [];

  // Apply 25 various operations
  for (let i = 0; i < 25; i++) {
    const opType = i % 4;
    if (opType === 0) {
      mgr.execute(
        new SetClipGainCommand("clip_vocals_1", mgr.state.clips[0].gainDb, (i % 6) - 3.0),
      );
    } else if (opType === 1) {
      mgr.execute(
        new MoveClipCommand(
          "clip_vocals_1",
          "trk_1",
          mgr.state.clips[0].startSeconds,
          "trk_1",
          10.0 + i * 0.5,
        ),
      );
    } else if (opType === 2) {
      mgr.execute(
        new SetTrackControlsCommand(
          "trk_1",
          { pan: mgr.state.tracks[0].pan },
          {
            pan: ((i % 10) - 5) / 5,
          },
        ),
      );
    } else {
      mgr.execute(
        new SetTrackControlsCommand(
          "trk_2",
          { gainDb: mgr.state.tracks[1].gainDb },
          {
            gainDb: -2.0 - (i % 5),
          },
        ),
      );
    }
    commandHistory.push(mgr.currentHash);
  }

  const stateAtEndHash = mgr.currentHash;

  // Undo 5 times and redo 5 times
  const hashesBefore: string[] = [];
  for (let i = 0; i < 5; i++) {
    hashesBefore.push(mgr.currentHash);
    mgr.undo();
  }
  for (let i = 4; i >= 0; i--) {
    mgr.redo();
    assert.equal(mgr.currentHash, hashesBefore[i]);
  }
  assert.equal(mgr.currentHash, stateAtEndHash);

  // Undo all operations
  while (mgr.canUndo) {
    mgr.undo();
  }

  // Invariant: all operations undone must produce bit-exact initial hash
  assert.equal(mgr.currentHash, initialHash);
});

test("Command System: coalesces rapid repeated commands (e.g. continuous slider/drag)", () => {
  const initialState = createTestState();
  const mgr = new CommandManager(initialState);

  // Execute 3 gain changes in rapid succession
  mgr.execute(new SetClipGainCommand("clip_vocals_1", 0.0, 1.0));
  mgr.execute(new SetClipGainCommand("clip_vocals_1", 1.0, 2.0));
  mgr.execute(new SetClipGainCommand("clip_vocals_1", 2.0, 3.5));

  // Because of coalescing, undo stack has 1 entry, not 3
  assert.equal(mgr.state.clips[0].gainDb, 3.5);
  const undone = mgr.undo();
  assert.equal(undone, true);
  assert.equal(mgr.state.clips[0].gainDb, 0.0);
  assert.equal(mgr.canUndo, false);
});

test("Command System: creates and preserves named snapshots with accurate hashes", () => {
  const initialState = createTestState();
  const mgr = new CommandManager(initialState);

  mgr.execute(new SetClipGainCommand("clip_vocals_1", 0.0, 2.5));
  const snap1 = mgr.createSnapshot("Before Chorus EQ");
  assert.equal(snap1.name, "Before Chorus EQ");
  assert.equal(snap1.stateHash, mgr.currentHash);

  mgr.execute(new MoveClipCommand("clip_vocals_1", "trk_1", 10.0, "trk_1", 20.0));
  const snap2 = mgr.createSnapshot("Moved Chorus");
  assert.equal(snap2.stateHash, mgr.currentHash);
  assert.notEqual(snap2.stateHash, snap1.stateHash);

  const allSnaps = mgr.getSnapshots();
  assert.equal(allSnaps.length, 2);
  assert.equal(allSnaps[0].name, "Before Chorus EQ");
  assert.equal(allSnaps[1].name, "Moved Chorus");
});

test("Command System: builds visual history nodes and tracks recent edits list", () => {
  const initialState = createTestState();
  const mgr = new CommandManager(initialState);

  mgr.execute(new MoveClipCommand("clip_vocals_1", "trk_1", 10.0, "trk_1", 42.318, "Chorus"));
  mgr.execute(new SetClipGainCommand("clip_vocals_1", 0.0, -2.0, "Chorus"));

  const recent = mgr.getRecentEditsList();
  assert.equal(recent.length, 2);
  assert.equal(recent[0].isCurrent, true);
  assert.ok(recent[0].name.includes('Gain -2.0 dB on "Chorus"'));
  assert.ok(recent[1].name.includes('Move "Chorus" to 42.318s'));
});
