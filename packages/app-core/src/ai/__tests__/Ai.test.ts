import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  DEFAULT_MODELS,
  DEFAULT_STEMS,
  DEFAULT_VOICE_CLEANUP,
  DEFAULT_TRANSCRIPT,
  DEFAULT_FILLERS,
  DEFAULT_CLOUD_JOBS,
  DEFAULT_PRIVACY_SETTINGS,
  calculateTimeSaved,
  formatTimeSaved,
} from "../AiModels.js";

describe("AI Domain Models and Utilities", () => {
  it("provides valid default ML models catalog", () => {
    assert.equal(DEFAULT_MODELS.length, 7);
    const downloaded = DEFAULT_MODELS.filter((m) => m.isDownloaded);
    assert.equal(downloaded.length, 4);

    const totalDownloadedBytes = downloaded.reduce((sum, m) => sum + m.sizeBytes, 0);
    // 820 + 64 + 240 + 18 = 1142 MB
    assert.equal(totalDownloadedBytes, 1142 * 1024 * 1024);
  });

  it("defines default 4-stem configuration with color tokens", () => {
    assert.equal(DEFAULT_STEMS.length, 4);
    assert.equal(DEFAULT_STEMS[0].type, "Vocals");
    assert.equal(DEFAULT_STEMS[0].colorToken, "--t4");
    assert.equal(DEFAULT_STEMS[1].type, "Drums");
    assert.equal(DEFAULT_STEMS[1].colorToken, "--t3");
    assert.equal(DEFAULT_STEMS[2].type, "Bass");
    assert.equal(DEFAULT_STEMS[2].colorToken, "--t2");
    assert.equal(DEFAULT_STEMS[3].type, "Other");
    assert.equal(DEFAULT_STEMS[3].colorToken, "--t5");
  });

  it("configures voice cleanup studio sound defaults", () => {
    assert.equal(DEFAULT_VOICE_CLEANUP.studioSoundAmount, 60);
    assert.equal(DEFAULT_VOICE_CLEANUP.removeNoise, true);
    assert.equal(DEFAULT_VOICE_CLEANUP.removeEcho, true);
    assert.equal(DEFAULT_VOICE_CLEANUP.matchStudioMic, true);
    assert.equal(DEFAULT_VOICE_CLEANUP.keepRoomToneDb, -20.0);
  });

  it("provides default transcript dialogue with speakers and word tokens", () => {
    assert.equal(DEFAULT_TRANSCRIPT.blocks.length, 2);
    assert.equal(DEFAULT_TRANSCRIPT.blocks[0].speakerName, "Maya");
    assert.equal(DEFAULT_TRANSCRIPT.blocks[1].speakerName, "Tom");

    const mayaWords = DEFAULT_TRANSCRIPT.blocks[0].words;
    assert.ok(mayaWords.length >= 25);
    const lowConf = mayaWords.find((w) => w.isLowConfidence);
    assert.ok(lowConf);
    assert.equal(lowConf?.text, "hear");

    const tomWords = DEFAULT_TRANSCRIPT.blocks[1].words;
    const cutWords = tomWords.filter((w) => w.isCut);
    assert.equal(cutWords.length, 2);
    assert.equal(cutWords[0].text, "the");
    assert.equal(cutWords[1].text, "the");
  });

  it("calculates time saved from selected filler occurrences", () => {
    assert.equal(DEFAULT_FILLERS.length, 6);
    const saved = calculateTimeSaved(DEFAULT_FILLERS);
    // 0.3 + 0.42 + 0.45 + 0.28 + 0.55 = 2.0s
    assert.ok(Math.abs(saved - 2.0) < 1e-4);
    assert.equal(formatTimeSaved(74), "1:14");
    assert.equal(formatTimeSaved(125), "2:05");
  });

  it("tracks default cloud jobs and privacy settings", () => {
    assert.equal(DEFAULT_CLOUD_JOBS.length, 4);
    const activeJobs = DEFAULT_CLOUD_JOBS.filter((j) => j.isActive);
    assert.equal(activeJobs.length, 2);

    assert.equal(DEFAULT_PRIVACY_SETTINGS.stemsBestCloud, false);
    assert.equal(DEFAULT_PRIVACY_SETTINGS.dubbingCloud, false);
  });
});
