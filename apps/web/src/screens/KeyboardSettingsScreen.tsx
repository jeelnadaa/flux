import React, { useState } from "react";
import { SettingsLayout, Button } from "@flux/ui";

interface ShortcutRow {
  command: string;
  keys: string;
}

const DEFAULT_ROWS: ShortcutRow[] = [
  { command: "Play or pause", keys: "Space" },
  { command: "Split at playhead", keys: "S" },
  { command: "Delete and close gap", keys: "Shift Del" },
  { command: "Nudge left 1 ms", keys: "Alt \u2190" },
  { command: "Undo", keys: "Ctrl Z" },
  { command: "Redo", keys: "Ctrl Shift Z" },
  { command: "Hold for original", keys: "A" },
  { command: "All commands", keys: "Ctrl K" },
  { command: "Zoom to selection", keys: "Z" },
  { command: "Add marker", keys: "M" },
];

export const KeyboardSettingsScreen: React.FC = () => {
  const [search, setSearch] = useState("");
  const [rows, setRows] = useState<ShortcutRow[]>(DEFAULT_ROWS);
  const [editingIndex, setEditingIndex] = useState<number | null>(null);

  const filtered = rows.filter((r) => r.command.toLowerCase().includes(search.toLowerCase()));

  const handleRebind = (index: number) => {
    setEditingIndex(index);
    const newKey = window.prompt(`Press key chord for "${rows[index].command}":`, rows[index].keys);
    if (newKey && newKey.trim()) {
      setRows((prev) => {
        const copy = [...prev];
        copy[index] = { ...copy[index], keys: newKey.trim() };
        return copy;
      });
    }
    setEditingIndex(null);
  };

  return (
    <SettingsLayout currentTab="keys" title="Shortcuts">
      <input
        className="inp"
        value={search}
        placeholder="Search commands"
        onChange={(e) => setSearch(e.target.value)}
      />
      <div className="tw">
        <table>
          <thead>
            <tr>
              <th>Command</th>
              <th>Keys</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {filtered.map((r, i) => (
              <tr key={r.command}>
                <td>{r.command}</td>
                <td>
                  <span className="kbd">{r.keys}</span>
                </td>
                <td>
                  <Button
                    variant="ghost"
                    label={editingIndex === i ? "Press key..." : "Change"}
                    onClick={() => handleRebind(i)}
                  />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </SettingsLayout>
  );
};
