import React, { useState } from "react";
import { ProjectLayout, Button, Chip, Callout, EffectCard, Input } from "@flux/ui";

export interface EffectItem {
  name: string;
  route: string;
}

export const FX_CATEGORIES: readonly [string, readonly [string, string][]][] = [
  [
    "Fix",
    [
      ["Noise reduction", "fx-clean"],
      ["Hum removal", "fx-clean"],
      ["De-click", "fx-clean"],
      ["De-clip", "fx-clean"],
      ["De-wind", "fx-clean"],
      ["Silence remover", "fx-silence"],
      ["Spectral repair", "spectral"],
    ],
  ],
  [
    "Tone",
    [
      ["Parametric EQ", "fx-eq"],
      ["Graphic EQ", "fx-eq"],
      ["Match EQ", "fx-eq"],
      ["De-esser", "fx-comp"],
    ],
  ],
  [
    "Dynamics",
    [
      ["Compressor", "fx-comp"],
      ["Limiter", "fx-comp"],
      ["Multiband", "fx-comp"],
      ["Gate", "fx-comp"],
      ["Transient shaper", "fx-comp"],
      ["Leveler", "fx-comp"],
    ],
  ],
  [
    "Time and pitch",
    [
      ["Speed", "fx-time"],
      ["Pitch", "fx-time"],
      ["Varispeed", "fx-time"],
    ],
  ],
  [
    "Space",
    [
      ["3D and 8D", "stage"],
      ["Reverb", "fx-reverb"],
      ["Rooms", "rooms"],
      ["Row 10", "hall"],
      ["Next door", "hall"],
      ["Delay", "fx-reverb"],
    ],
  ],
  [
    "Character",
    [
      ["Slowed + reverb", "creative"],
      ["Nightcore", "creative"],
      ["Lo-fi", "creative"],
      ["Vinyl", "textures"],
      ["Cassette", "textures"],
      ["Old radio", "textures"],
      ["Phone", "textures"],
      ["Megaphone", "textures"],
      ["Underwater", "textures"],
      ["Distortion", "textures"],
      ["Bitcrusher", "textures"],
      ["Chorus", "textures"],
      ["Flanger", "textures"],
      ["Phaser", "textures"],
    ],
  ],
  [
    "Glitch",
    [
      ["Stutter", "glitch"],
      ["Tape stop", "glitch"],
      ["Reverse", "glitch"],
      ["Granular freeze", "glitch"],
      ["Trance gate", "glitch"],
    ],
  ],
  [
    "Voice",
    [
      ["Voice changer", "voice"],
      ["Robot", "voice"],
      ["Vocoder", "voice"],
      ["Hard tune", "voice"],
    ],
  ],
] as const;

export const FX_GROUP_STYLE: Record<string, [string]> = {
  Fix: ["stutter"],
  Tone: ["reverse"],
  Dynamics: ["radio"],
  "Time and pitch": ["slowed"],
  Space: ["cathedral"],
  Character: ["lofi"],
  Glitch: ["tapestop"],
  Voice: ["robot"],
};

export const FX_STYLE: Record<string, [string, string, string]> = {
  "Slowed + reverb": ["slowed", "slow", "0.85×"],
  Nightcore: ["nightcore", "night", "1.25×"],
  "Lo-fi": ["lofi", "lofi", "64%"],
  Vinyl: ["vinyl", "vinyl", "33⅓"],
  Cassette: ["cassette", "tape", "Wow 0.6 Hz"],
  "Old radio": ["radio", "radio", "3 kHz"],
  Underwater: ["underwater", "water", "12 m"],
  Stutter: ["stutter", "stut", "1/16"],
  "Tape stop": ["tapestop", "stop", "1.4 s"],
  Reverse: ["reverse", "rev", ""],
  Reverb: ["cathedral", "cath", "6.8 s"],
  "Next door": ["nextdoor", "next", "620 Hz"],
  Robot: ["robot", "bot", "30 Hz"],
};

export const EffectBrowserScreen: React.FC = () => {
  const [searchQuery, setSearchQuery] = useState("");
  const [activeChip, setActiveChip] = useState("All");

  const query = searchQuery.trim().toLowerCase();

  const filteredCategories = FX_CATEGORIES.map(([group, items]) => {
    let matched = items;
    if (query) {
      matched = items.filter(
        ([name, _go]) => name.toLowerCase().includes(query) || group.toLowerCase().includes(query),
      );
    }
    return [group, matched] as const;
  }).filter(([_group, items]) => items.length > 0);

  return (
    <ProjectLayout currentTab="fxbrowser">
      <div className="pg in">
        <div className="pgh">
          <h1 className="h1">Effects</h1>
          <div className="row">
            <div style={{ width: "min(300px, 100%)" }}>
              <Input
                value={searchQuery}
                placeholder="Search effects"
                onChange={(e) => setSearchQuery(e.target.value)}
              />
            </div>
            <Button label="Your chain" icon="fx" dataGo="rack" href="#rack" />
          </div>
        </div>

        <div className="row">
          <Chip label="All" active={activeChip === "All"} onClick={() => setActiveChip("All")} />
          <Chip
            label="Favorites"
            active={activeChip === "Favorites"}
            onClick={() => setActiveChip("Favorites")}
          />
          <Chip
            label="Recent"
            active={activeChip === "Recent"}
            onClick={() => setActiveChip("Recent")}
          />
          <Chip
            label="Runs live"
            active={activeChip === "Runs live"}
            onClick={() => setActiveChip("Runs live")}
          />
          <Chip
            label="Needs rendering"
            active={activeChip === "Needs rendering"}
            onClick={() => setActiveChip("Needs rendering")}
          />
        </div>

        {filteredCategories.map(([group, items]) => (
          <section key={group} className="sec">
            <h2 className="h2">{group}</h2>
            <div className="g5">
              {items.map(([name, go]) => {
                const styleDef = FX_STYLE[name] || FX_GROUP_STYLE[group];
                const palette = styleDef[0];
                const effect = styleDef[1] || "";
                const rightMeta = styleDef[2] || "";
                return (
                  <EffectCard
                    key={name}
                    palette={palette}
                    effect={effect}
                    word={name}
                    leftMeta={group}
                    rightMeta={rightMeta}
                    dataGo={go}
                    href={`#${go}`}
                  />
                );
              })}
            </div>
          </section>
        ))}

        <Callout icon="info">
          Press and hold any tile to hear it on your selection before you add it. Previews never
          change your project.
        </Callout>
      </div>
    </ProjectLayout>
  );
};
