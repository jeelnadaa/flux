import React, { useState } from "react";
import {
  ProjectLayout,
  SegmentedControl,
  Slider,
  Toggle,
  Field,
  Select,
  Section,
  Icon,
  CanvasViz,
} from "@flux/ui";

interface AutoMasterScreenProps {
  onNavigate?: (route: string) => void;
}

export const AutoMasterScreen: React.FC<AutoMasterScreenProps> = ({ onNavigate }) => {
  const [intensityIndex, setIntensityIndex] = useState<number>(1);
  const [intensitySlider, setIntensitySlider] = useState<number>(50);
  const [preserveDynamics, setPreserveDynamics] = useState<boolean>(true);
  const [holdingBefore, setHoldingBefore] = useState<boolean>(false);

  const intensityOptions = ["Gentle", "Balanced", "Loud"] as const;

  const targetOptions = ["Streaming −14 LUFS", "Podcast −16 LUFS", "Club −9 LUFS", "Custom"];

  const metrics = [
    ["−14.0", "LUFS", "Target met"],
    ["−1.0", "dBTP", "Below ceiling"],
    ["8.2", "LRA", "Dynamics kept"],
    ["+0.71", "Mono", "Safe"],
  ];

  const changes = [
    { title: "Cut 250 Hz", desc: "−2.0 dB, mud", enabled: true },
    { title: "Lift 3 kHz", desc: "+1.5 dB, presence", enabled: true },
    { title: "Air at 12 kHz", desc: "+1.0 dB", enabled: true },
    { title: "Multiband", desc: "Average −2.1 dB", enabled: true },
    { title: "Width", desc: "+6%, lows stay mono", enabled: true },
    { title: "Limiter", desc: "Average −1.4 dB", enabled: true },
  ];

  return (
    <ProjectLayout currentTab="master" onNavigate={onNavigate}>
      <div className="split">
        <div className="a">
          <div className="pg in" style={{ maxWidth: "none" }}>
            <div className="pgh">
              <h1 className="h1">Master</h1>
              <SegmentedControl
                items={intensityOptions}
                selectedIndex={intensityIndex}
                onChange={(_, idx) => setIntensityIndex(idx)}
              />
            </div>

            <div className="vz" style={{ height: 190 }}>
              <CanvasViz
                type="wave"
                seed={1}
                stereo
                gain={holdingBefore ? 1.0 : 1.25}
                ceil={0.92}
                step={2}
              />
            </div>

            <div className="g4">
              {metrics.map(([val, unit, label], idx) => (
                <div key={idx} className="box inv">
                  <div className="stat">{val}</div>
                  <span className="lab">
                    {unit}, {label.toLowerCase()}
                  </span>
                </div>
              ))}
            </div>

            <button
              type="button"
              className={`btn lg hold p ${holdingBefore ? "on" : ""}`}
              onMouseDown={() => setHoldingBefore(true)}
              onMouseUp={() => setHoldingBefore(false)}
              onMouseLeave={() => setHoldingBefore(false)}
              onTouchStart={() => setHoldingBefore(true)}
              onTouchEnd={() => setHoldingBefore(false)}
            >
              Hold for before, loudness matched
            </button>
          </div>
        </div>

        <aside className="b in">
          <Field label="Loudness target">
            <Select options={targetOptions} />
          </Field>

          <Slider
            label="Intensity"
            value={intensitySlider}
            valueText="Balanced"
            onChange={setIntensitySlider}
          />

          <Toggle
            label="Preserve dynamics"
            checked={preserveDynamics}
            onChange={setPreserveDynamics}
          />

          <button type="button" className="btn">
            <Icon name="imp" size={14} />
            Match a reference track
          </button>

          <Section title="What we changed">
            <div className="lst">
              {changes.map((r, idx) => (
                <div key={idx}>
                  <div className="ti">
                    <b>{r.title}</b>
                    <span>{r.desc}</span>
                  </div>
                  <Toggle label="" checked={r.enabled} />
                </div>
              ))}
            </div>
          </Section>

          <button type="button" className="btn p lg" onClick={() => onNavigate?.("export")}>
            Export master
          </button>
        </aside>
      </div>
    </ProjectLayout>
  );
};
