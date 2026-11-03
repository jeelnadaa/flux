import React, { useState } from "react";
import {
  EffectPanelShell,
  CanvasViz,
  SegmentedControl,
  Button,
  Chip,
  Section,
  Toggle,
  Slider,
} from "@flux/ui";
import { EQ_PRESETS } from "@flux/app-core";

const INITIAL_BANDS = [
  { id: "1", type: "Low shelf", freq: "70 Hz", gain: "+3.5", color: "--t3" },
  { id: "2", type: "Bell", freq: "280 Hz", gain: "−3.0", color: "--t2" },
  { id: "3", type: "Bell", freq: "1.2 kHz", gain: "+1.6", color: "--t4" },
  { id: "4", type: "Bell", freq: "3.4 kHz", gain: "+4.0", color: "--t5" },
  { id: "5", type: "High shelf", freq: "11 kHz", gain: "+3.0", color: "--t1" },
];

const PRESETS_LIST = [
  "Clear voice",
  "Bass boost",
  "Treble boost",
  "Warm",
  "Bright",
  "Podcast",
  "De-mud",
  "Presence",
  "Air",
];

export const EqualizerScreen: React.FC = () => {
  const [activePreset, setActivePreset] = useState("Clear voice");
  const [modeIndex, setModeIndex] = useState(0); // 0: Natural, 1: Analog match, 2: Linear phase
  const [viewIndex, setViewIndex] = useState(0); // 0: Parametric, 1: 10 band, 2: 31 band
  const [outputVal, setOutputVal] = useState(50);
  const [bands, setBands] = useState(INITIAL_BANDS);
  const [isBypassed, setIsBypassed] = useState(false);

  const handlePresetSelect = (presetName: string) => {
    setActivePreset(presetName);
    const p = EQ_PRESETS[presetName];
    if (p) {
      const updated = p.bands.slice(0, 5).map((b, i) => {
        const typeLabel =
          b.type === "peaking"
            ? "Bell"
            : b.type === "lowshelf"
              ? "Low shelf"
              : b.type === "highshelf"
                ? "High shelf"
                : b.type === "highpass"
                  ? "High pass"
                  : b.type === "lowpass"
                    ? "Low pass"
                    : b.type;
        const freqLabel =
          b.frequency >= 1000 ? `${(b.frequency / 1000).toFixed(1)} kHz` : `${b.frequency} Hz`;
        const gainLabel = `${b.gainDb >= 0 ? "+" : "−"}${Math.abs(b.gainDb).toFixed(1)}`;
        return {
          id: `${i + 1}`,
          type: typeLabel,
          freq: freqLabel,
          gain: gainLabel,
          color: b.colorVar || ["--t3", "--t2", "--t4", "--t5", "--t1"][i] || "--t1",
        };
      });
      setBands(updated);
    }
  };

  const mainContent = (
    <>
      <div
        className="vz"
        style={{ flex: 1, minHeight: "300px" }}
        role="img"
        aria-label="EQ curve. Drag the numbered points."
      >
        <CanvasViz type="eq" />
      </div>

      <div className="g5">
        {bands.map((b) => (
          <div key={b.id} className="box" style={{ padding: "10px" }}>
            <div className="between">
              <span
                className="knob"
                style={{
                  width: "22px",
                  height: "22px",
                  background: `var(${b.color})`,
                  color: `var(${b.color}-on)`,
                  display: "grid",
                  placeItems: "center",
                  fontWeight: 800,
                  fontSize: "12px",
                }}
              >
                {b.id}
              </span>
              <span className="lab">{b.type}</span>
            </div>
            <div className="mono">{b.freq}</div>
            <div className="mono dim">{b.gain} dB</div>
          </div>
        ))}
      </div>
    </>
  );

  const asideContent = (
    <>
      <h3 className="h3">Presets</h3>
      <div className="row">
        {PRESETS_LIST.map((preset) => (
          <Chip
            key={preset}
            label={preset}
            active={activePreset === preset}
            onClick={() => handlePresetSelect(preset)}
          />
        ))}
      </div>

      <Section title="Mode">
        <SegmentedControl
          items={["Natural", "Analog match", "Linear phase"]}
          selectedIndex={modeIndex}
          onChange={(_item, idx) => setModeIndex(idx)}
        />
        <span className="dim" style={{ fontSize: "12.5px" }}>
          Linear phase adds 42 ms of latency, compensated automatically.
        </span>
      </Section>

      <Section title="Show">
        <SegmentedControl
          items={["Parametric", "10 band", "31 band"]}
          selectedIndex={viewIndex}
          onChange={(_item, idx) => setViewIndex(idx)}
        />
        <div className="col">
          <Toggle label="Spectrum before" defaultChecked={false} />
          <Toggle label="Spectrum after" defaultChecked={true} />
        </div>
      </Section>

      <Section title="Match">
        <span className="dim">Copy the tone of another recording.</span>
        <Button label="Choose a reference" icon="imp" />
      </Section>

      <Slider
        label="Output"
        value={outputVal}
        valueText={`${((outputVal - 50) * 0.24).toFixed(1)} dB`}
        onChange={setOutputVal}
      />
    </>
  );

  return (
    <EffectPanelShell
      title="Parametric EQ"
      isBypassed={isBypassed}
      onToggleBypass={() => setIsBypassed(!isBypassed)}
      closeGo="rack"
      main={mainContent}
      aside={asideContent}
    />
  );
};
