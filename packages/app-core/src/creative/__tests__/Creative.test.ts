import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  DEFAULT_SLOWED_REVERB,
  DEFAULT_NIGHTCORE,
  DEFAULT_LOFI_MACRO,
  DEFAULT_VINYL,
  VOICE_CHARACTERS,
} from "../CreativeModels.js";

describe("Creative Effects & Voice Domain Models", () => {
  it("defines default Slowed + Reverb configuration", () => {
    assert.equal(DEFAULT_SLOWED_REVERB.speedRatio, 0.85);
    assert.equal(DEFAULT_SLOWED_REVERB.reverbDecaySeconds, 3.2);
    assert.equal(DEFAULT_SLOWED_REVERB.brightnessDb, -3.0);
    assert.equal(DEFAULT_SLOWED_REVERB.keepPitch, false);
  });

  it("defines default Nightcore configuration", () => {
    assert.equal(DEFAULT_NIGHTCORE.speedRatio, 1.25);
    assert.equal(DEFAULT_NIGHTCORE.trebleLiftDb, 2.0);
    assert.equal(DEFAULT_NIGHTCORE.chorusDepthPct, 18);
    assert.equal(DEFAULT_NIGHTCORE.keepPitch, false);
  });

  it("defines default Lo-Fi macro with 12 controls", () => {
    assert.equal(DEFAULT_LOFI_MACRO.amount, 0.64);
    assert.equal(DEFAULT_LOFI_MACRO.bitDepth, 12);
    assert.equal(DEFAULT_LOFI_MACRO.sampleRateKhz, 22);
    assert.equal(DEFAULT_LOFI_MACRO.cracklePerSec, 4);
    assert.equal(DEFAULT_LOFI_MACRO.surfaceDb, -48);
  });

  it("defines default Vinyl configuration with seeded reproducibility", () => {
    assert.equal(DEFAULT_VINYL.seed, 4821);
    assert.equal(DEFAULT_VINYL.cracklePerSec, 4);
    assert.equal(DEFAULT_VINYL.popsPerSec, 0.6);
  });

  it("provides 16 original character voice presets", () => {
    assert.equal(VOICE_CHARACTERS.length, 16);
    const chars = Object.fromEntries(VOICE_CHARACTERS.map((c) => [c.id, c]));
    assert.equal(chars.deep.pitchSemitones, -5);
    assert.equal(chars.chipmunk.pitchSemitones, 7);
    assert.equal(chars.giant.pitchSemitones, -12);
    assert.equal(chars.tiny.pitchSemitones, 12);
    assert.equal(chars.robot.name, "Robot");
  });
});
