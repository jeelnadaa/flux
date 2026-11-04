import React, { useState } from "react";
import { Input, Tile } from "@flux/ui";
import { HelpLayout } from "./HelpLayout.js";

const GUIDES: readonly [string, string][] = [
  ["Your first 8D track", "90 seconds"],
  ["Edit by deleting words", "3 minutes"],
  ["Clean up a noisy recording", "4 minutes"],
  ["Splitting stems", "2 minutes"],
  ["Understanding LUFS", "5 minutes"],
  ["Drawing paths on the stage", "3 minutes"],
  ["Backing up your projects", "2 minutes"],
  ["Keyboard-only editing", "6 minutes"],
  ["Preset file format", "For developers"],
];

const GUIDE_ROUTES: Record<string, string> = {
  "Your first 8D track": "stage",
  "Edit by deleting words": "script",
  "Clean up a noisy recording": "cleanup",
  "Splitting stems": "stems-setup",
  "Understanding LUFS": "meters",
  "Drawing paths on the stage": "stage",
  "Backing up your projects": "prefs-storage",
  "Keyboard-only editing": "prefs-keys",
  "Preset file format": "presets",
};

interface HelpScreenProps {
  onNavigate?: (route: string) => void;
}

export const HelpScreen: React.FC<HelpScreenProps> = ({ onNavigate }) => {
  const [query, setQuery] = useState("");

  const navigate = (r: string) => {
    if (onNavigate) {
      onNavigate(r);
    } else {
      window.location.hash = r;
    }
  };

  const filtered = GUIDES.filter(
    ([title, sub]) =>
      title.toLowerCase().includes(query.toLowerCase()) ||
      sub.toLowerCase().includes(query.toLowerCase()),
  );

  return (
    <HelpLayout currentTab="help" title="How can we help?" onNavigate={navigate}>
      <Input placeholder="Search help" value={query} onChange={(e) => setQuery(e.target.value)} />
      <div className="g3">
        {filtered.map(([title, subtitle]) => {
          const route = GUIDE_ROUTES[title] || "help";
          return (
            <Tile
              key={title}
              title={title}
              subtitle={subtitle}
              dataGo={route}
              href={`#${route}`}
              onClick={() => navigate(route)}
            />
          );
        })}
      </div>
    </HelpLayout>
  );
};
