import test from "node:test";
import assert from "node:assert/strict";
import { ShortcutManager } from "../ShortcutManager.js";

test("ShortcutManager: registers default shortcuts and normalizes key chords", () => {
  const sm = new ShortcutManager();
  const all = sm.getAll();
  assert.ok(all.length >= 10);

  const split = all.find((s) => s.id === "split");
  assert.equal(split?.keys, "S");
  assert.equal(split?.category, "Editing");

  assert.equal(sm.normalizeKey("Ctrl Z"), "ctrl+z");
  assert.equal(sm.normalizeKey("Ctrl Shift Z"), "ctrl+shift+z");
  assert.equal(sm.normalizeKey("Shift Del"), "shift+del");
  assert.equal(sm.normalizeKey("Space"), "space");
});

test("ShortcutManager: allows rebinding shortcuts and dispatches actions", () => {
  const sm = new ShortcutManager();
  let actionFired = false;

  sm.setAction("split", () => {
    actionFired = true;
  });

  // Rebind split to 'X'
  sm.rebind("split", "X");

  // Simulate keydown event with 'x'
  const mockEvent = {
    key: "x",
    ctrlKey: false,
    shiftKey: false,
    altKey: false,
    metaKey: false,
    preventDefault: () => {},
  } as unknown as KeyboardEvent;

  const handled = sm.handleKeyDown(mockEvent);
  assert.equal(handled, true);
  assert.equal(actionFired, true);
});
