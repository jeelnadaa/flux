import React, { useState } from "react";
import { WizardLayout, Icon } from "@flux/ui";

const CHIPS = [
  "Sounds right",
  "Left and right are swapped",
  "Sounds flat, inside my head",
  "I'm using speakers",
];

export const HpCheckScreen: React.FC = () => {
  const [selectedChip, setSelectedChip] = useState(0);

  return (
    <WizardLayout step={2} totalSteps={4}>
      <h1 className="h1">Are your headphones the right way round?</h1>
      <p className="lede">
        3D and 8D audio depend on it. Press each button, then tell us what you heard.
      </p>
      <div className="g2">
        <button className="tile" type="button" style={{ alignItems: "center", padding: "30px" }}>
          <span className="act">
            <Icon name="head" size={38} />
          </span>
          <b style={{ fontSize: "22px" }}>Play left</b>
          <span>A soft tone in one ear</span>
        </button>
        <button className="tile" type="button" style={{ alignItems: "center", padding: "30px" }}>
          <span className="act">
            <Icon name="head" size={38} />
          </span>
          <b style={{ fontSize: "22px" }}>Play right</b>
          <span>A soft tone in the other ear</span>
        </button>
      </div>
      <div className="box col">
        <b>Next, a sound will jump around your head.</b>
        <span className="dim">Front, left, right, then behind you.</span>
        <button className="btn" type="button">
          <Icon name="play" />
          <span>Play the moving sound</span>
        </button>
      </div>
      <div className="row">
        {CHIPS.map((text, idx) => (
          <button
            key={text}
            className={`chip ${idx === selectedChip ? "on" : ""}`}
            type="button"
            onClick={() => setSelectedChip(idx)}
          >
            {text}
          </button>
        ))}
      </div>
      <div className="callout info">
        <Icon name="info" />
        <div>
          Browsers can't detect headphones, so this only checks what you hear. If it sounds flat,
          try the next step: picking a head shape usually fixes it.
        </div>
      </div>
      <div className="between">
        <a className="btn g" href="#welcome" data-go="welcome">
          Back
        </a>
        <div className="row">
          <a className="btn g" href="#storage-ask" data-go="storage-ask">
            <span>Skip</span>
          </a>
          <a className="btn p lg" href="#hrtf" data-go="hrtf">
            <span>Continue</span>
          </a>
        </div>
      </div>
    </WizardLayout>
  );
};
