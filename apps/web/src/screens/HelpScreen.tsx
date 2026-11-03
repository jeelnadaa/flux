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

interface HelpScreenProps {
  onNavigate?: (route: string) => void;
}

export const HelpScreen: React.FC<HelpScreenProps> = ({ onNavigate }) => {
  const [query, setQuery] = useState("");

  const filtered = GUIDES.filter(
    ([title, sub]) =>
      title.toLowerCase().includes(query.toLowerCase()) ||
      sub.toLowerCase().includes(query.toLowerCase()),
  );

  return (
    <HelpLayout currentTab="help" title="How can we help?" onNavigate={onNavigate}>
      <Input placeholder="Search help" value={query} onChange={(e) => setQuery(e.target.value)} />
      <div className="g3">
        {filtered.map(([title, subtitle]) => (
          <Tile key={title} title={title} subtitle={subtitle} />
        ))}
      </div>
    </HelpLayout>
  );
};
