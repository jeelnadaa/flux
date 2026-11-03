import test from "node:test";
import assert from "node:assert/strict";
import { ProjectJournal, computeCrc32 } from "../ProjectJournal.js";
import type { SerializedCommand } from "../Command.js";

test("Project Journal: calculates exact CRC32 checksums", () => {
  const text = "Flux deterministic audio editor";
  const crc1 = computeCrc32(text);
  const crc2 = computeCrc32(text);
  assert.equal(crc1, crc2);
  assert.equal(typeof crc1, "number");
  assert.ok(crc1 > 0);

  // Difference check
  const diff = computeCrc32("Flux deterministic audio editor!");
  assert.notEqual(diff, crc1);
});

test("Project Journal: appends commands and recovers full command history intact", async () => {
  const journal = new ProjectJournal("TestProject_Append");
  await journal.clear();

  const cmd1: SerializedCommand = {
    id: "cmd_1",
    type: "clip.move",
    name: 'Move "Chorus" to 0:42.318',
    timestamp: Date.now() - 5000,
    params: { clipId: "c1", pos: 42.318 },
  };

  const cmd2: SerializedCommand = {
    id: "cmd_2",
    type: "clip.split",
    name: 'Split "Lead take 3" at 1:14.002',
    timestamp: Date.now() - 2000,
    params: { clipId: "c2", pos: 74.002 },
  };

  await journal.appendCommand(cmd1);
  await journal.appendCommand(cmd2);

  const report = await journal.recover();
  assert.equal(report.recoveredEditsCount, 2);
  assert.equal(report.lastEditName, 'Split "Lead take 3" at 1:14.002');
  assert.equal(report.commands.length, 2);
  assert.equal(report.commands[0].id, "cmd_1");
  assert.equal(report.commands[1].id, "cmd_2");
  assert.equal(report.hasCorruptedTrailingData, false);
});

test("Project Journal: recovers in-flight export state on unexpected shutdown", async () => {
  const journal = new ProjectJournal("TestProject_Export");
  await journal.clear();

  await journal.appendCommand({
    id: "cmd_1",
    type: "clip.move",
    name: 'Move "Chorus"',
    timestamp: Date.now(),
    params: {},
  });

  await journal.recordExportState("master.flac", 62);

  const report = await journal.recover();
  assert.equal(report.recoveredEditsCount, 1);
  assert.ok(report.interruptedExport !== undefined);
  assert.equal(report.interruptedExport?.name, "master.flac");
  assert.equal(report.interruptedExport?.progress, 62);
});

test("Project Journal Chaos Test: 100 trials of simulated kill-mid-write recover to last intact committed command", async () => {
  for (let trial = 0; trial < 100; trial++) {
    const trialJournal = new ProjectJournal(`ChaosTrial_${trial}`);
    await trialJournal.clear();

    const numValid = 3 + (trial % 8);
    for (let j = 0; j < numValid; j++) {
      await trialJournal.appendCommand({
        id: `cmd_trial_${trial}_${j}`,
        type: "clip.edit",
        name: `Edit ${j}`,
        timestamp: 1000 + j,
        params: { idx: j },
      });
    }

    const corruptPayloads = [
      '{"seq":99,"timestamp":12345,"cmd":{"id":"corrupted',
      '{"seq":99,"timestamp":12345,"cmd":{"id":"bad_crc"}}|99999999\n',
      '{"seq":99,"timestamp":12345,"cmd":{',
      "partial_broken_line_with_no_json\n",
      '{"seq":99}|0\n',
    ];
    const corruptData = corruptPayloads[trial % corruptPayloads.length];
    trialJournal.injectRawData(corruptData);

    const recovery = await trialJournal.recover();
    assert.equal(recovery.recoveredEditsCount, numValid);
    assert.equal(recovery.lastEditName, `Edit ${numValid - 1}`);
    assert.equal(recovery.hasCorruptedTrailingData, true);

    await trialJournal.clear();
  }
});
