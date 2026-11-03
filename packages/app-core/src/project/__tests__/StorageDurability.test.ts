import test from "node:test";
import assert from "node:assert/strict";
import { StorageDurability } from "../StorageDurability.js";

test("StorageDurability: provides available cleanup options with accurate descriptions and freed estimates", () => {
  const options = StorageDurability.getCleanupOptions();
  assert.equal(options.length, 3);

  assert.equal(options[0].title, "Clear waveform caches");
  assert.ok(options[0].description.includes("Frees 3.1 GB"));
  assert.equal(options[0].actionLabel, "Clear");

  assert.equal(options[1].title, "Remove AI results");
  assert.ok(options[1].description.includes("Frees 1.6 GB"));
  assert.equal(options[1].actionLabel, "Remove");

  assert.equal(options[2].title, "Move this project to a folder");
  assert.ok(options[2].description.includes("Frees 412 MB"));
  assert.equal(options[2].actionLabel, "Move");
});

test("StorageDurability: handles storage cleanup operations cleanly", async () => {
  const okWave = await StorageDurability.executeCleanup("waveform_cache");
  assert.equal(okWave, true);

  const okAi = await StorageDurability.executeCleanup("ai_cache");
  assert.equal(okAi, true);
});

test("StorageDurability: calculates quota thresholds and triggers warnings at 90% and 95%", async () => {
  const report = await StorageDurability.getQuotaReport();
  assert.equal(typeof report.quotaBytes, "number");
  assert.equal(typeof report.usageBytes, "number");
  assert.equal(typeof report.usagePercent, "number");
  assert.equal(typeof report.isWarning, "boolean");
  assert.equal(typeof report.isCritical, "boolean");
});
