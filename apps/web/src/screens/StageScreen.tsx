import React, { useState } from "react";
import {
  ProjectLayout,
  SegmentedControl,
  Slider,
  Toggle,
  Section,
  Field,
  Icon,
  Pill,
  Callout,
  CanvasViz,
} from "@flux/ui";

const STAGE_MODES = ["Orbit", "Spiral", "Figure 8", "Swarm", "Place"];
const STAGE_TOOLS = ["Drag", "Draw path"];
const ORBIT_DIRS = ["Clockwise", "Counter", "Ping-pong"];
const INPUT_MODES = ["Two speakers", "Mid and side", "Stems"];
const RENDERER_MODES = ["Best", "Lite"];

const STEM_SOURCES = [
  { name: "Vocals", subtitle: "Front, fixed", colorVar: "--t4" },
  { name: "Drums", subtitle: "Orbit", colorVar: "--t3" },
  { name: "Bass", subtitle: "Center", colorVar: "--t2" },
  { name: "Other", subtitle: "Orbit, reverse", colorVar: "--t5" },
];

export const StageScreen: React.FC = () => {
  const [stageMode, setStageMode] = useState(0);
  const [stageTool, setStageTool] = useState(0);
  const [orbitDir, setOrbitDir] = useState(0);
  const [inputMode, setInputMode] = useState(0);
  const [rendererMode, setRendererMode] = useState(0);

  const [rotationSpeed, setRotationSpeed] = useState(24);
  const [distance, setDistance] = useState(58);
  const [elevation, setElevation] = useState(50);
  const [depth, setDepth] = useState(80);
  const [smoothing, setSmoothing] = useState(60);

  const [syncTempo, setSyncTempo] = useState(false);
  const [monoBass, setMonoBass] = useState(true);
  const [doppler, setDoppler] = useState(false);

  return (
    <ProjectLayout currentTab="stage">
      <div className="split" data-stage>
        <div className="a">
          <div className="tlbar">
            <SegmentedControl
              items={STAGE_MODES}
              selectedIndex={stageMode}
              onChange={(_, idx) => setStageMode(idx)}
              dataStageMode
            />
            <span className="sp" style={{ flex: 1 }} />
            <SegmentedControl
              items={STAGE_TOOLS}
              selectedIndex={stageTool}
              onChange={(_, idx) => setStageTool(idx)}
              dataStageTool
            />
            <a className="btn g hide-s" href="#hpcheck" data-go="hpcheck">
              <Icon name="head" />
              <span>Headphone check</span>
            </a>
          </div>
          <div
            style={{
              flex: 1,
              display: "grid",
              gridTemplateRows: "minmax(0, 1fr) 92px",
              minHeight: "440px",
              padding: "10px",
              gap: "10px",
            }}
          >
            <div className="vz" role="img" aria-label="Sound stage. Drag sources around your head.">
              <CanvasViz
                type="stage"
                mode={STAGE_MODES[stageMode].toLowerCase().replace(" ", "")}
                tool={stageTool === 1 ? "draw" : "drag"}
                style={{ position: "absolute", inset: 0 }}
              />
            </div>
            <div className="vz">
              <CanvasViz type="elev" />
            </div>
          </div>
        </div>
        <aside className="b in">
          <div className="between">
            <h3 className="h3">8D orbit</h3>
            <Pill label="Live" variant="ok" />
          </div>
          <Slider
            label="Rotation speed"
            value={rotationSpeed}
            valueText="0.10 Hz"
            big
            onChange={setRotationSpeed}
          />
          <div className="row">
            <SegmentedControl
              items={ORBIT_DIRS}
              selectedIndex={orbitDir}
              onChange={(_, idx) => setOrbitDir(idx)}
            />
          </div>
          <div className="ctl-grid" style={{ gridTemplateColumns: "repeat(2, minmax(0, 1fr))" }}>
            <Slider label="Distance" value={distance} valueText="1.8 m" onChange={setDistance} />
            <Slider label="Elevation" value={elevation} valueText="0°" onChange={setElevation} />
            <Slider label="Depth" value={depth} valueText="80%" onChange={setDepth} />
            <Slider label="Smoothing" value={smoothing} valueText="60%" onChange={setSmoothing} />
          </div>
          <Toggle
            label="Sync to tempo"
            sub="1 turn every 8 beats"
            checked={syncTempo}
            onChange={setSyncTempo}
          />
          <Toggle
            label="Keep bass centered"
            sub="Below 120 Hz"
            checked={monoBass}
            onChange={setMonoBass}
          />
          <Toggle label="Doppler" checked={doppler} onChange={setDoppler} />
          <Section title="Sources">
            <div className="lst">
              {STEM_SOURCES.map((s) => (
                <div key={s.name}>
                  <span className="dot" style={{ color: `var(${s.colorVar})` }} />
                  <div className="ti">
                    <b>{s.name}</b>
                    <span>{s.subtitle}</span>
                  </div>
                </div>
              ))}
            </div>
            <a className="btn" href="#stems-setup" data-go="stems-setup">
              <Icon name="stem" />
              <span>Split into stems first</span>
            </a>
          </Section>
          <Section title="Input">
            <Field label="Treat stereo as">
              <SegmentedControl
                items={INPUT_MODES}
                selectedIndex={inputMode}
                onChange={(_, idx) => setInputMode(idx)}
              />
            </Field>
          </Section>
          <Section title="Renderer">
            <SegmentedControl
              items={RENDERER_MODES}
              selectedIndex={rendererMode}
              onChange={(_, idx) => setRendererMode(idx)}
            />
            <div className="row">
              <a className="dim" href="#hrtf" data-go="hrtf">
                Head shape: Profile C
              </a>
            </div>
          </Section>
          <button className="btn" type="button">
            <Icon name="rec" />
            <span>Record movement as automation</span>
          </button>
          <Callout variant="warn" icon="warn">
            On speakers this collapses. Use headphones, or check the mono mix.
          </Callout>
        </aside>
      </div>
    </ProjectLayout>
  );
};
