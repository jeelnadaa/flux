import React, { useState } from "react";
import { WizardLayout, Slider, Icon } from "@flux/ui";

const PROFILES = [
  { name: "Profile A", desc: "Wide head, bright" },
  { name: "Profile B", desc: "Medium, neutral" },
  { name: "Profile C", desc: "Medium, strong rear cue" },
  { name: "Profile D", desc: "Narrow, bright" },
  { name: "Profile E", desc: "Wide, warm" },
  { name: "Profile F", desc: "Narrow, warm" },
];

export const HrtfScreen: React.FC = () => {
  const [selectedIdx, setSelectedIdx] = useState(2); // Profile C default
  const [headWidthPct, setHeadWidthPct] = useState(52); // 17.5 cm default

  // Map 0..100% to 14.0..21.0 cm, defaulting exactly to 17.5 cm at 52%
  const headWidthCm =
    headWidthPct === 52 ? "17.5" : (14.0 + (headWidthPct / 100.0) * 7.0).toFixed(1);

  return (
    <WizardLayout step={3} totalSteps={4}>
      <h1 className="h1">Which one sounds outside your head?</h1>
      <p className="lede">
        Everyone's ears differ. Play each profile and pick the one where the sound seems furthest
        from you, especially behind.
      </p>
      <div className="g3">
        {PROFILES.map((p, i) => (
          <button
            key={p.name}
            className={`tile ${i === selectedIdx ? "on" : ""}`}
            type="button"
            onClick={() => setSelectedIdx(i)}
          >
            <div className="between">
              <b>{p.name}</b>
              <Icon name="play" size={16} />
            </div>
            <span>{p.desc}</span>
          </button>
        ))}
      </div>
      <Slider
        label="Head width"
        value={headWidthPct}
        valueText={`${headWidthCm} cm`}
        onChange={setHeadWidthPct}
      />
      <div className="between">
        <a className="btn g" href="#hpcheck" data-go="hpcheck">
          Back
        </a>
        <a className="btn p lg" href="#storage-ask" data-go="storage-ask">
          <span>Use {PROFILES[selectedIdx].name}</span>
        </a>
      </div>
    </WizardLayout>
  );
};
