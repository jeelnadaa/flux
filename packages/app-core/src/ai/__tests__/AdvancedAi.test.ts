import { describe, it } from "node:test";
import assert from "node:assert";
import {
  DEFAULT_TEMPO_KEY_ANALYSIS,
  DEFAULT_MASTER_METRICS,
  DEFAULT_MASTER_CHANGES,
  DEFAULT_TTS_VOICES,
  DEFAULT_DUB_REVIEW_ITEMS,
  DEFAULT_MAGIC_DIAGNOSIS,
  DEFAULT_MAGIC_CHAIN,
} from "../AdvancedAiModels.js";

describe("Phase 7: Advanced AI Domain Models", () => {
  it("provides valid tempo and key analysis defaults", () => {
    assert.strictEqual(DEFAULT_TEMPO_KEY_ANALYSIS.bpm, 84.0);
    assert.strictEqual(DEFAULT_TEMPO_KEY_ANALYSIS.key, "D min");
    assert.strictEqual(DEFAULT_TEMPO_KEY_ANALYSIS.camelotNotation, "7A Camelot");
    assert.strictEqual(DEFAULT_TEMPO_KEY_ANALYSIS.openKeyNotation, "6m Open Key");
    assert.strictEqual(DEFAULT_TEMPO_KEY_ANALYSIS.tuningHz, 440);
    assert.strictEqual(DEFAULT_TEMPO_KEY_ANALYSIS.timelineSegments.length, 3);
  });

  it("configures auto-mastering broadcast targets and explanations", () => {
    assert.strictEqual(DEFAULT_MASTER_METRICS.length, 4);
    assert.strictEqual(DEFAULT_MASTER_METRICS[0].value, "−14.0");
    assert.strictEqual(DEFAULT_MASTER_METRICS[0].unit, "LUFS");

    assert.strictEqual(DEFAULT_MASTER_CHANGES.length, 6);
    assert.ok(DEFAULT_MASTER_CHANGES.some((c) => c.title.includes("250 Hz")));
  });

  it("provides licensed TTS voices with narration descriptions", () => {
    assert.strictEqual(DEFAULT_TTS_VOICES.length, 4);
    const ash = DEFAULT_TTS_VOICES.find((v) => v.id === "ash");
    assert.ok(ash);
    assert.strictEqual(ash.name, "Ash");
    assert.strictEqual(ash.selected, true);
  });

  it("provides dubbing review items with per-speaker length validation", () => {
    assert.strictEqual(DEFAULT_DUB_REVIEW_ITEMS.length, 3);
    const tom = DEFAULT_DUB_REVIEW_ITEMS.find((d) => d.speaker === "Tom");
    assert.ok(tom);
    assert.strictEqual(tom.status, "warn");
    assert.strictEqual(tom.durationPercent, "+22%");
  });

  it("provides complete Magic diagnosis and proposed audio chain", () => {
    assert.strictEqual(DEFAULT_MAGIC_DIAGNOSIS.length, 5);
    const hum = DEFAULT_MAGIC_DIAGNOSIS.find((d) => d.label === "Hum");
    assert.ok(hum);
    assert.strictEqual(hum.status, "50 Hz");

    assert.strictEqual(DEFAULT_MAGIC_CHAIN.length, 7);
    assert.strictEqual(DEFAULT_MAGIC_CHAIN[0].title, "Hum removal");
    assert.strictEqual(DEFAULT_MAGIC_CHAIN[6].title, "Limiter");
  });
});
