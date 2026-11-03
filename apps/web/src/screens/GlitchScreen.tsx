import React, { useState } from "react";
import {
  ProjectLayout,
  Slider,
  SegmentedControl,
  Toggle,
  Button,
  Field,
  CanvasViz,
} from "@flux/ui";

const STUTTER_DIVISIONS = ["1/4", "1/8", "1/16", "1/32", "1/64"];
const TAPE_STOP_MODES = ["Stop", "Start", "Both"];
const REVERSE_TARGETS = ["Clip", "Region"];

export const GlitchScreen: React.FC = () => {
  // Stutter
  const [stutterDivIdx, setStutterDivIdx] = useState(2); // 1/16
  const [stutterProb, setStutterProb] = useState(72);
  const [pitchRamp, setPitchRamp] = useState(40);
  const [stutterGate, setStutterGate] = useState(80);

  // Tape stop
  const [stopTime, setStopTime] = useState(40); // 1.4 s
  const [tapeModeIdx, setTapeModeIdx] = useState(0);

  // Reverse
  const [reverseTargetIdx, setReverseTargetIdx] = useState(0);
  const [reverseSwell, setReverseSwell] = useState(true);
  const [swellDuration, setSwellDuration] = useState(50); // 2 beats

  // Granular
  const [grainSize, setGrainSize] = useState(30); // 80 ms
  const [grainSpray, setGrainSpray] = useState(50); // 50%
  const [freeze, setFreeze] = useState(false);

  // Trance gate 16 steps
  const [gateSteps, setGateSteps] = useState([
    true,
    false,
    false,
    true,
    false,
    false,
    true,
    false,
    true,
    false,
    false,
    true,
    false,
    false,
    true,
    false,
  ]);
  const [gateDepth, setGateDepth] = useState(80);

  // Glitch
  const [chaos, setChaos] = useState(34);
  const [glitchSeed, setGlitchSeed] = useState("77");

  const toggleGateStep = (index: number) => {
    setGateSteps((prev) => {
      const next = [...prev];
      next[index] = !next[index];
      return next;
    });
  };

  return (
    <ProjectLayout currentTab="fxbrowser">
      <div className="pg in">
        <h1 className="h1">Break it in time.</h1>
        <div className="vz" style={{ height: "150px" }}>
          <CanvasViz type="wave" seed={3} kind="drums" color="--t3" step={2} height={150} />
        </div>

        <div className="g3">
          {/* Stutter */}
          <div className="box col pal-stutter">
            <h2 className="h2">Stutter</h2>
            <div className="row">
              {STUTTER_DIVISIONS.map((d, i) => (
                <button
                  key={d}
                  type="button"
                  className={`chip ${i === stutterDivIdx ? "on" : ""}`}
                  onClick={() => setStutterDivIdx(i)}
                >
                  {d}
                </button>
              ))}
            </div>
            <Slider
              label="Probability"
              value={stutterProb}
              valueText={`${stutterProb}%`}
              onChange={setStutterProb}
            />
            <Slider
              label="Pitch ramp"
              value={pitchRamp}
              valueText="+3 st"
              onChange={setPitchRamp}
            />
            <Slider
              label="Gate"
              value={stutterGate}
              valueText={`${stutterGate}%`}
              onChange={setStutterGate}
            />
            <Button variant="primary" data-hold>
              Hold to stutter
            </Button>
          </div>

          {/* Tape stop */}
          <div className="box col pal-tapestop">
            <h2 className="h2">Tape stop</h2>
            <Slider label="Stop time" value={stopTime} valueText="1.4 s" onChange={setStopTime} />
            <SegmentedControl
              items={TAPE_STOP_MODES}
              selectedIndex={tapeModeIdx}
              onChange={(_, idx) => setTapeModeIdx(idx)}
            />
            <Button>Add at playhead</Button>
          </div>

          {/* Reverse */}
          <div className="box col pal-reverse">
            <h2
              className="h2"
              style={{
                transform: "scaleX(-1)",
                transformOrigin: "center",
                textAlign: "right",
              }}
            >
              Reverse
            </h2>
            <SegmentedControl
              items={REVERSE_TARGETS}
              selectedIndex={reverseTargetIdx}
              onChange={(_, idx) => setReverseTargetIdx(idx)}
            />
            <Toggle
              label="Reverse reverb swell"
              checked={reverseSwell}
              onChange={setReverseSwell}
            />
            <Slider
              label="Swell"
              value={swellDuration}
              valueText="2 beats"
              onChange={setSwellDuration}
            />
            <Button>Apply</Button>
          </div>

          {/* Granular */}
          <div className="box col pal-underwater">
            <h2 className="h2">Granular</h2>
            <Slider label="Grain" value={grainSize} valueText="80 ms" onChange={setGrainSize} />
            <Slider
              label="Spray"
              value={grainSpray}
              valueText={`${grainSpray}%`}
              onChange={setGrainSpray}
            />
            <Toggle label="Freeze" checked={freeze} onChange={setFreeze} />
            <Button>Add</Button>
          </div>

          {/* Trance gate */}
          <div className="box col pal-nightcore">
            <h2 className="h2">Trance gate</h2>
            <div className="row" style={{ gap: "3px" }}>
              {gateSteps.map((stepActive, i) => (
                <button
                  key={i}
                  type="button"
                  data-toggle
                  aria-label={`Step ${i + 1}`}
                  className={`tbtn ${stepActive ? "on" : ""}`}
                  style={{
                    width: "calc(12.5% - 3px)",
                    height: "24px",
                    borderRadius: "3px",
                    boxShadow: "inset 0 0 0 1.5px var(--tx)",
                  }}
                  onClick={() => toggleGateStep(i)}
                />
              ))}
            </div>
            <Slider
              label="Depth"
              value={gateDepth}
              valueText={`${gateDepth}%`}
              onChange={setGateDepth}
            />
            <Button>Add</Button>
          </div>

          {/* Glitch */}
          <div className="box col pal-robot">
            <h2 className="h2">Glitch</h2>
            <Slider label="Chaos" value={chaos} valueText={`${chaos}%`} onChange={setChaos} />
            <Field label="Seed">
              <input
                className="inp"
                value={glitchSeed}
                onChange={(e) => setGlitchSeed(e.target.value)}
              />
            </Field>
            <Button>Add</Button>
          </div>
        </div>

        <div className="between dim">
          <span>Tempo: 84.0 BPM from your project</span>
          <a className="act" href="#analysis" data-go="analysis">
            Detect tempo
          </a>
        </div>
      </div>
    </ProjectLayout>
  );
};
