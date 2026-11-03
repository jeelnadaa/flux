export interface ShortcutDefinition {
  id: string;
  name: string;
  keys: string;
  category: "Transport" | "Editing" | "Navigation" | "System";
  action: () => void | Promise<void>;
}

export class ShortcutManager {
  private bindings: Map<string, ShortcutDefinition> = new Map();
  private keyMap: Map<string, string> = new Map(); // normalized key string -> shortcut id
  private listener: ((e: KeyboardEvent) => void) | null = null;

  constructor() {
    this.registerDefaults();
  }

  private registerDefaults(): void {
    const defaults: Omit<ShortcutDefinition, "action">[] = [
      { id: "play_pause", name: "Play or pause", keys: "Space", category: "Transport" },
      { id: "split", name: "Split at playhead", keys: "S", category: "Editing" },
      { id: "ripple_delete", name: "Delete and close gap", keys: "Shift Del", category: "Editing" },
      { id: "nudge_left", name: "Nudge left 1 ms", keys: "Alt \u2190", category: "Editing" },
      { id: "undo", name: "Undo", keys: "Ctrl Z", category: "System" },
      { id: "redo", name: "Redo", keys: "Ctrl Shift Z", category: "System" },
      { id: "hold_original", name: "Hold for original", keys: "A", category: "Transport" },
      { id: "command_palette", name: "All commands", keys: "Ctrl K", category: "System" },
      { id: "zoom_selection", name: "Zoom to selection", keys: "Z", category: "Navigation" },
      { id: "add_marker", name: "Add marker", keys: "M", category: "Navigation" },
      { id: "shortcut_sheet", name: "This sheet", keys: "?", category: "System" },
    ];

    for (const d of defaults) {
      this.register({
        ...d,
        action: () => {},
      });
    }
  }

  public register(shortcut: ShortcutDefinition): void {
    this.bindings.set(shortcut.id, shortcut);
    const normalized = this.normalizeKey(shortcut.keys);
    this.keyMap.set(normalized, shortcut.id);
  }

  public setAction(id: string, action: () => void): void {
    const item = this.bindings.get(id);
    if (item) {
      item.action = action;
    }
  }

  public rebind(id: string, newKeys: string): void {
    const item = this.bindings.get(id);
    if (!item) return;

    // Remove old mapping
    const oldNorm = this.normalizeKey(item.keys);
    this.keyMap.delete(oldNorm);

    // Set new
    item.keys = newKeys;
    const newNorm = this.normalizeKey(newKeys);
    this.keyMap.set(newNorm, id);
  }

  public getAll(): ShortcutDefinition[] {
    return Array.from(this.bindings.values());
  }

  public normalizeKey(keysStr: string): string {
    const parts = keysStr.toLowerCase().split(/[\s+]+/);
    const ctrl = parts.includes("ctrl") || parts.includes("control");
    const shift = parts.includes("shift");
    const alt = parts.includes("alt");
    const meta = parts.includes("meta") || parts.includes("cmd");

    const mainKey =
      parts.find((p) => !["ctrl", "control", "shift", "alt", "meta", "cmd"].includes(p)) || "";
    return `${ctrl ? "ctrl+" : ""}${shift ? "shift+" : ""}${alt ? "alt+" : ""}${meta ? "meta+" : ""}${mainKey}`;
  }

  public eventToKeyString(e: KeyboardEvent): string {
    const ctrl = e.ctrlKey || e.metaKey;
    const shift = e.shiftKey;
    const alt = e.altKey;

    let key = e.key.toLowerCase();
    if (key === " ") key = "space";
    if (key === "delete") key = "del";
    if (key === "arrowleft") key = "\u2190";
    if (key === "arrowright") key = "\u2192";

    if (["control", "shift", "alt", "meta"].includes(key)) {
      key = "";
    }

    return `${ctrl ? "ctrl+" : ""}${shift ? "shift+" : ""}${alt ? "alt+" : ""}${key}`;
  }

  public handleKeyDown(e: KeyboardEvent): boolean {
    const keyStr = this.eventToKeyString(e);
    const id = this.keyMap.get(keyStr);
    if (id) {
      const def = this.bindings.get(id);
      if (def && def.action) {
        e.preventDefault();
        def.action();
        return true;
      }
    }
    return false;
  }

  public attach(target: EventTarget = window): void {
    if (this.listener) return;
    this.listener = (e: Event) => this.handleKeyDown(e as KeyboardEvent);
    target.addEventListener("keydown", this.listener as EventListener);
  }

  public detach(target: EventTarget = window): void {
    if (!this.listener) return;
    target.removeEventListener("keydown", this.listener as EventListener);
    this.listener = null;
  }
}

export const globalShortcuts = new ShortcutManager();
