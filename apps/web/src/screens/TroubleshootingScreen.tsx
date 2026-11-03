import React from "react";
import { Button } from "@flux/ui";
import { HelpLayout } from "./HelpLayout.js";

const TROUBLE_ITEMS: readonly [string, string][] = [
  [
    "I hear nothing",
    "Check the output device and that this tab is not muted. Press Play once: browsers need a click before they make sound.",
  ],
  [
    "Clicks or dropouts while playing",
    "Set Quality to Balanced in Settings, under Audio. Close other heavy tabs.",
  ],
  [
    "8D sounds flat or inside my head",
    "Run the headphone check, then try another head shape. Add some room. It will not work on speakers.",
  ],
  ["Stems are slow", "Your browser may not have WebGPU. Use Fast mode, or try Chrome or Edge."],
  [
    "My AAC export option is missing",
    "Your browser has no AAC encoder. Use Chrome or Edge, or export FLAC or Opus.",
  ],
  [
    "Project won't open",
    "Flux keeps earlier snapshots. Open History, or restore from your .flux backup.",
  ],
];

interface TroubleshootingScreenProps {
  onNavigate?: (route: string) => void;
}

export const TroubleshootingScreen: React.FC<TroubleshootingScreenProps> = ({ onNavigate }) => {
  const navigate = (r: string) => {
    if (onNavigate) {
      onNavigate(r);
    } else {
      window.location.hash = r;
    }
  };

  return (
    <HelpLayout currentTab="trouble" title="Something wrong?" onNavigate={onNavigate}>
      <div className="lst box">
        {TROUBLE_ITEMS.map(([q, a]) => (
          <div
            key={q}
            style={{
              alignItems: "flex-start",
              flexDirection: "column",
              gap: "3px",
            }}
          >
            <b className="h3">{q}</b>
            <span className="dim">{a}</span>
          </div>
        ))}
      </div>

      <div className="row">
        <Button
          label="Create a diagnostic report"
          icon="bug"
          dataGo="diag"
          href="#diag"
          onClick={(e) => {
            e.preventDefault();
            navigate("diag");
          }}
        />
      </div>
    </HelpLayout>
  );
};
