import { test } from "node:test";
import assert from "node:assert/strict";
import { EffectChain, EffectRackManager } from "../EffectRack.js";

test("EffectChain: adds, moves, reorders and removes effects", () => {
  const chain = new EffectChain("track", "Test chain");
  assert.equal(chain.effects.length, 0);

  const fx1 = chain.addEffect({
    pluginId: "parametric-eq",
    name: "EQ",
    subtitle: "5 bands",
    colorVar: "--t1",
    route: "fx-eq",
    bypassed: false,
    soloed: false,
    mix: 100,
    parameters: {},
  });

  const fx2 = chain.addEffect({
    pluginId: "compressor",
    name: "Compressor",
    subtitle: "Dynamics",
    colorVar: "--t2",
    route: "fx-comp",
    bypassed: false,
    soloed: false,
    mix: 80,
    parameters: {},
  });

  assert.equal(chain.effects.length, 2);
  assert.equal(chain.effects[0].id, fx1.id);
  assert.equal(chain.effects[1].id, fx2.id);

  // Insert at index 0
  const fx3 = chain.addEffect(
    {
      pluginId: "de-esser",
      name: "De-esser",
      subtitle: "Sibilance",
      colorVar: "--t5",
      route: "fx-comp",
      bypassed: false,
      soloed: false,
      mix: 100,
      parameters: {},
    },
    0,
  );

  assert.equal(chain.effects.length, 3);
  assert.equal(chain.effects[0].id, fx3.id);
  assert.equal(chain.effects[1].id, fx1.id);
  assert.equal(chain.effects[2].id, fx2.id);

  // Move effect from index 0 to 2
  const moved = chain.moveEffect(0, 2);
  assert.equal(moved, true);
  assert.equal(chain.effects[0].id, fx1.id);
  assert.equal(chain.effects[1].id, fx2.id);
  assert.equal(chain.effects[2].id, fx3.id);

  // Reorder by ID
  const reordered = chain.reorderEffect(fx3.id, 0);
  assert.equal(reordered, true);
  assert.equal(chain.effects[0].id, fx3.id);

  // Remove effect
  const removed = chain.removeEffect(fx1.id);
  assert.equal(removed, true);
  assert.equal(chain.effects.length, 2);
  assert.equal(
    chain.effects.find((e) => e.id === fx1.id),
    undefined,
  );
});

test("EffectChain: bypass, solo, and wet/dry mix controls", () => {
  const chain = EffectChain.createDefaultVocalsChain();
  const eq = chain.effects.find((e) => e.pluginId === "parametric-eq")!;
  assert.ok(eq);
  assert.equal(eq.bypassed, false);

  // Toggle bypass
  const newBypass = chain.toggleBypass(eq.id);
  assert.equal(newBypass, true);
  assert.equal(eq.bypassed, true);

  chain.setBypass(eq.id, false);
  assert.equal(eq.bypassed, false);

  // Toggle solo
  assert.equal(eq.soloed, false);
  const newSolo = chain.toggleSolo(eq.id);
  assert.equal(newSolo, true);
  assert.equal(eq.soloed, true);

  // Wet/Dry mix clamping
  chain.setMix(eq.id, 150);
  assert.equal(eq.mix, 100);

  chain.setMix(eq.id, -20);
  assert.equal(eq.mix, 0);

  chain.setMix(eq.id, 45);
  assert.equal(eq.mix, 45);
});

test("EffectChain: A/B/C comparison slots preserve and restore independent states", () => {
  const chain = EffectChain.createDefaultVocalsChain();
  const initialEq = chain.effects.find((e) => e.pluginId === "parametric-eq")!;

  // Slot A has initial state
  chain.saveToCompareSlot("A");

  // Modify chain in slot B
  chain.switchCompareSlot("B");
  chain.setMix(initialEq.id, 30);
  chain.toggleBypass(initialEq.id);
  assert.equal(chain.effects.find((e) => e.pluginId === "parametric-eq")!.mix, 30);
  assert.equal(chain.effects.find((e) => e.pluginId === "parametric-eq")!.bypassed, true);

  // Modify chain in slot C
  chain.switchCompareSlot("C");
  chain.removeEffect(initialEq.id);
  assert.equal(
    chain.effects.find((e) => e.pluginId === "parametric-eq"),
    undefined,
  );

  // Switch back to A: original state restored intact
  chain.switchCompareSlot("A");
  const restoredEq = chain.effects.find((e) => e.pluginId === "parametric-eq")!;
  assert.ok(restoredEq);
  assert.equal(restoredEq.mix, 100);
  assert.equal(restoredEq.bypassed, false);

  // Switch back to B: modified state in B restored
  chain.switchCompareSlot("B");
  const bEq = chain.effects.find((e) => e.pluginId === "parametric-eq")!;
  assert.equal(bEq.mix, 30);
  assert.equal(bEq.bypassed, true);
});

test("EffectChain: Macro system computes and maps control parameters", () => {
  const chain = EffectChain.createDefaultVocalsChain();
  const eq = chain.effects.find((e) => e.name === "Parametric EQ")!;
  const comp = chain.effects.find((e) => e.name === "Compressor")!;

  // Macro at 0%
  chain.setMacroValue("Presence", 0);
  assert.equal(chain.macros[0].value, 0);
  assert.equal(chain.macros[0].displayValue, "0%");
  assert.equal(Number(eq.parameters["EQ band 4 gain"]), 0);
  assert.equal(comp.mix, 60);

  // Macro at 100%
  chain.setMacroValue("Presence", 100);
  assert.equal(chain.macros[0].value, 100);
  assert.equal(chain.macros[0].displayValue, "100%");
  assert.equal(Number(eq.parameters["EQ band 4 gain"]), 5);
  assert.equal(comp.mix, 100);

  // Macro at 62% (default prototype value)
  chain.setMacroValue("Presence", 62);
  assert.equal(chain.macros[0].value, 62);
  assert.equal(chain.macros[0].displayValue, "62%");
  assert.equal(Number(Number(eq.parameters["EQ band 4 gain"]).toFixed(1)), 3.1);
  assert.equal(comp.mix, 85); // 60 + 40 * 0.62 = 84.8 -> 85
});

test("EffectChain: CPU load estimation matches active DSP processors", () => {
  const chain = EffectChain.createDefaultVocalsChain();
  // 5 active effects: de-esser(0.8) + eq(1.2) + comp(1.5) + reverb(3.5) + limiter(1.0) = 8.0
  assert.equal(chain.estimateCpuPercent(), 8);

  // Bypassing reverb reduces CPU from 8% to 5% (4.5 rounded)
  const reverb = chain.effects.find((e) => e.pluginId === "reverb")!;
  chain.setBypass(reverb.id, true);
  assert.equal(chain.estimateCpuPercent(), 5);

  // Soloing de-esser leaves only 1% CPU
  const deesser = chain.effects.find((e) => e.pluginId === "de-esser")!;
  chain.setSolo(deesser.id, true);
  assert.equal(chain.estimateCpuPercent(), 1);
});

test("EffectChain: serialization round-trip preserves complete configuration", () => {
  const chain = EffectChain.createDefaultVocalsChain();
  const serialized = chain.serialize();
  const deserialized = EffectChain.deserialize(serialized);

  assert.equal(deserialized.name, chain.name);
  assert.equal(deserialized.effects.length, chain.effects.length);
  assert.equal(deserialized.macros.length, chain.macros.length);
  assert.deepEqual(deserialized.effects[0], chain.effects[0]);
  assert.deepEqual(deserialized.macros[0], chain.macros[0]);
});

test("EffectRackManager: isolates clip, track, and master rack chains", () => {
  const manager = new EffectRackManager();

  // Track chain default
  manager.setTarget("track", "trk_1");
  const trackChain = manager.getActiveChain();
  assert.equal(trackChain.target, "track");
  assert.equal(trackChain.effects.length, 5);

  // Master chain
  manager.setTarget("master");
  const masterChain = manager.getActiveChain();
  assert.equal(masterChain.target, "master");
  assert.equal(masterChain.name, "Master bus");
  assert.equal(masterChain.effects.length, 0);

  // Add limiter to master
  masterChain.addEffect({
    pluginId: "limiter",
    name: "Master Limiter",
    subtitle: "Ceiling −0.1 dBTP",
    colorVar: "--t4",
    route: "fx-comp",
    bypassed: false,
    soloed: false,
    mix: 100,
    parameters: {},
  });
  assert.equal(masterChain.effects.length, 1);
  assert.equal(trackChain.effects.length, 5); // Unchanged

  // Clip chain
  manager.setTarget("clip", "clip_vocal_take1");
  const clipChain = manager.getActiveChain();
  assert.equal(clipChain.target, "clip");
  assert.equal(clipChain.targetId, "clip_vocal_take1");
  assert.equal(clipChain.effects.length, 0);
});
