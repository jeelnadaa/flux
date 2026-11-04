import React, { useState, useEffect } from "react";
import {
  Button,
  Callout,
  CanvasViz,
  Field,
  ProjectLayout,
  Section,
  SegmentedControl,
  Select,
  Slider,
} from "@flux/ui";
import { ProjectStore, TransportState, type FluxProjectState } from "@flux/app-core";

interface ClipData {
  left: number;
  width: number;
  colorVar: string;
  seed: number;
  kind: string;
  label: string;
  selected?: boolean;
  fadeIn?: number;
  fadeOut?: number;
}

const ClipView: React.FC<ClipData> = ({
  left,
  width,
  colorVar,
  seed,
  kind,
  label,
  selected,
  fadeIn,
  fadeOut,
}) => (
  <div
    className={`clip ${selected ? "sel" : ""}`}
    style={
      {
        left: `${left}%`,
        width: `${width}%`,
        "--c": `var(${colorVar})`,
        "--co": `var(${colorVar}-on)`,
      } as React.CSSProperties
    }
  >
    <CanvasViz type="wave" seed={seed} kind={kind} color={colorVar} step={2.5} on />
    {fadeIn && <span className="fi" style={{ "--w": `${fadeIn}px` } as React.CSSProperties} />}
    {fadeOut && <span className="fo" style={{ "--w": `${fadeOut}px` } as React.CSSProperties} />}
    <span className="cl">{label}</span>
  </div>
);

interface TrackData {
  name: string;
  colorVar: string;
  tall?: boolean;
  muted?: boolean;
  clips: ClipData[];
  hasAutomation?: boolean;
}

const TRACKS: TrackData[] = [
  {
    name: "Vocals",
    colorVar: "--t4",
    tall: true,
    hasAutomation: true,
    clips: [
      {
        left: 4,
        width: 28,
        colorVar: "--t4",
        seed: 7,
        kind: "voice",
        label: "Verse 1",
        fadeIn: 18,
      },
      {
        left: 32.4,
        width: 22,
        colorVar: "--t4",
        seed: 8,
        kind: "voice",
        label: "Chorus",
        selected: true,
        fadeIn: 14,
        fadeOut: 14,
      },
      {
        left: 58,
        width: 34,
        colorVar: "--t4",
        seed: 9,
        kind: "voice",
        label: "Verse 2",
        fadeOut: 24,
      },
    ],
  },
  {
    name: "Drums",
    colorVar: "--t3",
    clips: [
      {
        left: 0,
        width: 96,
        colorVar: "--t3",
        seed: 3,
        kind: "drums",
        label: "Drum stem",
      },
    ],
  },
  {
    name: "Bass",
    colorVar: "--t2",
    clips: [
      {
        left: 8,
        width: 88,
        colorVar: "--t2",
        seed: 4,
        kind: "bass",
        label: "Bass stem",
      },
    ],
  },
  {
    name: "Keys",
    colorVar: "--t1",
    muted: true,
    clips: [
      {
        left: 0,
        width: 44,
        colorVar: "--t1",
        seed: 5,
        kind: "pad",
        label: "Rhodes",
      },
      {
        left: 48,
        width: 48,
        colorVar: "--t1",
        seed: 6,
        kind: "pad",
        label: "Rhodes 2",
      },
    ],
  },
  {
    name: "Guitars",
    colorVar: "--t5",
    clips: [
      {
        left: 20,
        width: 36,
        colorVar: "--t5",
        seed: 10,
        kind: "mix",
        label: "Lead take 3",
      },
      {
        left: 62,
        width: 26,
        colorVar: "--t5",
        seed: 11,
        kind: "mix",
        label: "Solo",
      },
    ],
  },
  {
    name: "Room tone",
    colorVar: "--t6",
    clips: [
      {
        left: 0,
        width: 100,
        colorVar: "--t6",
        seed: 12,
        kind: "pad",
        label: "Ambience",
      },
    ],
  },
];

export const AutomationScreen: React.FC = () => {
  const store = ProjectStore.getInstance();
  const [project, setProject] = useState<FluxProjectState>(() => store.getState());
  const [modeIdx, setModeIdx] = useState<number>(0); // Read, Write, Touch, Latch
  const [selectedParam, setSelectedParam] = useState<string>("Vocals: Volume");
  const [curveIdx, setCurveIdx] = useState<number>(0); // Linear, Smooth, Hold
  const [lanes, setLanes] = useState<string[]>(["Volume", "Pan", "Reverb mix"]);
  const [pointValue, setPointValue] = useState<number>(58);

  useEffect(() => {
    return store.subscribe((newState: FluxProjectState) => {
      setProject({ ...newState });
    });
  }, [store]);

  const removeLane = (name: string) => {
    setLanes(lanes.filter((l) => l !== name));
  };

  const addLane = () => {
    const nextName = `Lane ${lanes.length + 1}`;
    setLanes([...lanes, nextName]);
  };

  const isPlaying = project.transportState === TransportState.Playing;
  const timecode = store.formatTimecode(project.playheadFrame);

  return (
    <ProjectLayout
      currentTab="edit"
      isPlaying={isPlaying}
      timecode={timecode}
      onPlayPause={() => store.togglePlayback()}
      onStop={() => store.stop()}
      onBackToStart={() => store.seek(0)}
      onLoopToggle={() =>
        store.setLoop(!project.loopEnabled, project.loopStartFrame, project.loopEndFrame)
      }
      leftMeter={project.meterLevels.leftPeak}
      rightMeter={project.meterLevels.rightPeak}
    >
      <div className="split">
        <div className="a">
          {/* Automation Toolbar */}
          <div className="tlbar">
            <SegmentedControl
              items={["Read", "Write", "Touch", "Latch"]}
              selectedIndex={modeIdx}
              onChange={(_item, idx) => setModeIdx(idx)}
            />
            <Field label="">
              <Select
                options={[
                  "Vocals: Volume",
                  "Vocals: Pan",
                  "Reverb: Mix",
                  "Stage: Azimuth",
                  "Stage: Distance",
                ]}
                value={selectedParam}
                onChange={setSelectedParam}
              />
            </Field>
            <SegmentedControl
              items={["Linear", "Smooth", "Hold"]}
              selectedIndex={curveIdx}
              onChange={(_item, idx) => setCurveIdx(idx)}
            />
            <span className="sp" style={{ flex: 1 }} />
            <Button label="Done" dataGo="edit" href="#edit" />
          </div>

          {/* Timeline with Automation Line */}
          <div className="tl" data-tl="">
            <div className="ruler">
              <CanvasViz type="ruler" />
            </div>
            {TRACKS.map((trk) => (
              <div key={trk.name} className={`trk ${trk.tall ? "tall" : ""}`}>
                <div className="thd">
                  <div className="nm">
                    <span className="dot" style={{ color: `var(${trk.colorVar})` }} />
                    <b>{trk.name}</b>
                  </div>
                  <div className="ms">
                    <button
                      className={`m ${trk.muted ? "on" : ""}`}
                      type="button"
                      aria-label={`Mute ${trk.name}`}
                    >
                      M
                    </button>
                    <button className="s" type="button" aria-label={`Solo ${trk.name}`}>
                      S
                    </button>
                    <button type="button" aria-label={`Arm ${trk.name} for recording`}>
                      R
                    </button>
                  </div>
                </div>
                <div className="lane">
                  {trk.clips.map((c, i) => (
                    <ClipView key={i} {...c} />
                  ))}
                  {trk.hasAutomation && (
                    <svg
                      className="autoline"
                      viewBox="0 0 100 40"
                      preserveAspectRatio="none"
                      aria-hidden="true"
                    >
                      <path
                        d="M0 22 L18 22 C24 22 26 10 32 10 L52 10 C58 10 60 26 68 26 L100 26"
                        fill="none"
                        stroke="var(--sig)"
                        strokeWidth="1.8"
                        vectorEffect="non-scaling-stroke"
                      />
                    </svg>
                  )}
                </div>
              </div>
            ))}
            <span className="ph" data-ph="" style={{ transform: "translateX(120px)" }} />
          </div>
        </div>

        {/* Sidebar */}
        <aside className="b in">
          <h3 className="h3">Vocals: Volume</h3>
          <Slider
            label="Point value"
            value={pointValue}
            defaultValue={58}
            valueText="−1.5 dB"
            onChange={setPointValue}
          />
          <div className="mono dim">At 0:58.240</div>
          <Section title="Lanes on this track">
            <div className="lst">
              {lanes.map((n) => (
                <div key={n}>
                  <div className="ti">
                    <b>{n}</b>
                  </div>
                  <Button
                    variant="ghost"
                    icon="x"
                    iconOnly
                    ariaLabel="Remove lane"
                    onClick={() => removeLane(n)}
                  />
                </div>
              ))}
            </div>
            <Button icon="plus" label="Add lane" onClick={addLane} />
          </Section>
          <Callout variant="info">
            Draw on the lane, or play and move any control while Write is on.
          </Callout>
        </aside>
      </div>
    </ProjectLayout>
  );
};
