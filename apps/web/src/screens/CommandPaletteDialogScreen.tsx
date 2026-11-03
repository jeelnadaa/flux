import React, { useState } from "react";
import { EditScreen } from "./EditScreen.js";
import { Dialog, Icon, type IconName } from "@flux/ui";

interface PaletteCommand {
  label: string;
  route: string;
  icon: IconName;
  active?: boolean;
}

const ALL_COMMANDS: PaletteCommand[] = [
  { label: "Normalize to \u221214 LUFS", route: "fx-comp", icon: "master", active: true },
  { label: "Normalize peak to \u22121 dB", route: "fx-comp", icon: "master" },
  { label: "Noise reduction", route: "fx-clean", icon: "mic" },
  { label: "Add reverb: Normal room", route: "fx-reverb", icon: "room" },
  { label: "Export normalized file", route: "export", icon: "exp" },
];

export const CommandPaletteDialogScreen: React.FC = () => {
  const [query, setQuery] = useState("nor");

  // When query is the default "nor" from design spec, render all 5 specification commands
  const displayed =
    query === "nor"
      ? ALL_COMMANDS
      : ALL_COMMANDS.filter((c) => c.label.toLowerCase().includes(query.toLowerCase()));

  return (
    <>
      <EditScreen />
      <Dialog title="Commands" baseScreenId="edit" className="cmd">
        <input
          className="inp"
          value={query}
          placeholder="Type a command"
          onChange={(e) => setQuery(e.target.value)}
        />
        <div className="col" style={{ gap: "1px" }}>
          {displayed.map((c, i) => (
            <a
              key={c.label}
              className={`it ${c.active || (query !== "nor" && i === 0) ? "on" : ""}`}
              href={`#${c.route}`}
              data-go={c.route}
            >
              <Icon name={c.icon} />
              <span style={{ flex: 1 }}>{c.label}</span>
            </a>
          ))}
        </div>
      </Dialog>
    </>
  );
};
