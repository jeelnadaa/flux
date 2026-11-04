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

  const playTone = (panVal: number) => {
    try {
      const AudioCtx =
        window.AudioContext ||
        (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
      const ctx = new AudioCtx();
      const osc = ctx.createOscillator();
      osc.type = "sine";
      osc.frequency.setValueAtTime(440, ctx.currentTime);

      const gain = ctx.createGain();
      gain.gain.setValueAtTime(0.18, ctx.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + 1.2);

      const panner = ctx.createStereoPanner ? ctx.createStereoPanner() : null;
      if (panner) {
        panner.pan.setValueAtTime(panVal, ctx.currentTime);
        osc.connect(panner);
        panner.connect(gain);
      } else {
        osc.connect(gain);
      }
      gain.connect(ctx.destination);
      osc.start();
      osc.stop(ctx.currentTime + 1.2);
    } catch (e) {
      console.warn("AudioContext tone check unavailable:", e);
    }
  };

  const playMovingSound = () => {
    try {
      const AudioCtx =
        window.AudioContext ||
        (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
      const ctx = new AudioCtx();
      const osc = ctx.createOscillator();
      osc.type = "triangle";
      osc.frequency.setValueAtTime(520, ctx.currentTime);

      const gain = ctx.createGain();
      gain.gain.setValueAtTime(0.15, ctx.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + 3.0);

      const panner = ctx.createStereoPanner ? ctx.createStereoPanner() : null;
      if (panner) {
        panner.pan.setValueAtTime(0, ctx.currentTime); // Front
        panner.pan.linearRampToValueAtTime(-1, ctx.currentTime + 0.8); // Left
        panner.pan.linearRampToValueAtTime(1, ctx.currentTime + 1.8); // Right
        panner.pan.linearRampToValueAtTime(0, ctx.currentTime + 2.8); // Behind / center
        osc.connect(panner);
        panner.connect(gain);
      } else {
        osc.connect(gain);
      }
      gain.connect(ctx.destination);
      osc.start();
      osc.stop(ctx.currentTime + 3.0);
    } catch (e) {
      console.warn("AudioContext moving sound unavailable:", e);
    }
  };

  return (
    <WizardLayout step={2} totalSteps={4}>
      <h1 className="h1">Are your headphones the right way round?</h1>
      <p className="lede">
        3D and 8D audio depend on it. Press each button, then tell us what you heard.
      </p>
      <div className="g2">
        <button
          className="tile"
          type="button"
          style={{ alignItems: "center", padding: "30px", cursor: "pointer" }}
          onClick={() => playTone(-1.0)}
        >
          <span className="act">
            <Icon name="head" size={38} />
          </span>
          <b style={{ fontSize: "22px" }}>Play left</b>
          <span>A soft tone in one ear</span>
        </button>
        <button
          className="tile"
          type="button"
          style={{ alignItems: "center", padding: "30px", cursor: "pointer" }}
          onClick={() => playTone(1.0)}
        >
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
        <button className="btn" type="button" onClick={playMovingSound}>
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
      <div className="callout">
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
