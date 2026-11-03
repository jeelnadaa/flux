import React, { useState } from "react";
import {
  ProjectLayout,
  SegmentedControl,
  Button,
  Chip,
  Pill,
  Slider,
  Field,
  Section,
  Icon,
  CanvasViz,
} from "@flux/ui";
import { SnappingEngine } from "@flux/app-core";

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
}

const TRACKS: TrackData[] = [
  {
    name: "Vocals",
    colorVar: "--t4",
    tall: true,
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

export const EditScreen: React.FC = () => {
  const [playheadX, setPlayheadX] = useState<number>(120);
  const [snapZeroCross, setSnapZeroCross] = useState<boolean>(true);
  const [snapGrid, setSnapGrid] = useState<boolean>(false);
  const [ripple, setRipple] = useState<boolean>(false);
  const [toolIdx, setToolIdx] = useState<number>(0);
  const [isScrubbing, setIsScrubbing] = useState<boolean>(false);

  const topBarRight = (
    <>
      <SegmentedControl items={["Simple", "Pro"]} selectedIndex={1} className="hide-s" dataMode />
      <button className="tbtn" type="button" aria-label="Undo">
        <Icon name="undo" />
      </button>
      <button className="tbtn hide-s" type="button" aria-label="Redo">
        <Icon name="redo" />
      </button>
      <Button
        variant="ghost"
        smallLabel
        className="hide-s"
        icon="hist"
        ariaLabel="History"
        dataGo="history"
      />
      <Button variant="magic" smallLabel icon="mag" label="Magic" dataGo="magic" />
      <Button variant="primary" smallLabel icon="exp" label="Export" dataGo="export" />
    </>
  );

  const handleRulerPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const rawX = Math.max(0, e.clientX - rect.left);

    if (snapZeroCross) {
      const snapped = SnappingEngine.snap(rawX / 100, 100, {
        thresholdPx: 8,
        gridIntervalSeconds: snapGrid ? 0.25 : undefined,
      });
      setPlayheadX(snapped.snappedTime * 100);
    } else {
      setPlayheadX(rawX);
    }
    setIsScrubbing(true);
    e.currentTarget.setPointerCapture(e.pointerId);
  };

  const handleRulerPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!isScrubbing) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const rawX = Math.max(0, e.clientX - rect.left);
    setPlayheadX(rawX);
  };

  const handleRulerPointerUp = (e: React.PointerEvent<HTMLDivElement>) => {
    if (isScrubbing) {
      setIsScrubbing(false);
      try {
        e.currentTarget.releasePointerCapture(e.pointerId);
      } catch {
        // Ignored
      }
    }
  };

  return (
    <ProjectLayout currentTab="edit" rightActions={topBarRight}>
      <div className="split">
        <div className="a">
          {/* Edit toolbar */}
          <div className="tlbar" role="toolbar" aria-label="Edit tools">
            <SegmentedControl
              items={["Select", "Range", "Slip", "Fade"]}
              selectedIndex={toolIdx}
              onChange={(_item, idx) => setToolIdx(idx)}
            />
            <Button variant="ghost" iconOnly icon="cut" ariaLabel="Cut" />
            <Button variant="ghost" iconOnly icon="copy" ariaLabel="Copy" />
            <Button variant="ghost" iconOnly icon="split" ariaLabel="Split at playhead" />
            <Button variant="ghost" iconOnly icon="trash" ariaLabel="Delete" />
            <span className="sp" style={{ flex: 1 }} />
            <Chip
              label="Snap: zero cross"
              active={snapZeroCross}
              onClick={() => setSnapZeroCross(!snapZeroCross)}
            />
            <Chip label="Grid: 1/4" active={snapGrid} onClick={() => setSnapGrid(!snapGrid)} />
            <Chip label="Ripple" active={ripple} onClick={() => setRipple(!ripple)} />
            <Button variant="ghost" label="Automation" dataGo="automation" />
            <Button variant="ghost" iconOnly icon="zoom" ariaLabel="Zoom" />
          </div>

          {/* Timeline */}
          <div className="tl" data-tl="">
            <div
              className="ruler"
              onPointerDown={handleRulerPointerDown}
              onPointerMove={handleRulerPointerMove}
              onPointerUp={handleRulerPointerUp}
              onPointerCancel={handleRulerPointerUp}
            >
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
                </div>
              </div>
            ))}
            <span className="ph" data-ph="" style={{ transform: `translateX(${playheadX}px)` }} />
          </div>
        </div>

        {/* Clip inspector */}
        <aside className="b in">
          <div className="between">
            <h3 className="h3">Chorus</h3>
            <Pill label="Vocals" />
          </div>
          <div className="g2 mono dim">
            <span>Start 0:42.318</span>
            <span>Len 0:18.906</span>
          </div>
          <Slider label="Clip gain" defaultValue={60} valueText="+1.5 dB" />
          <div className="g2">
            <Slider label="Fade in" defaultValue={12} valueText="40 ms" />
            <Slider label="Fade out" defaultValue={18} valueText="120 ms" />
          </div>
          <Field label="Fade curve">
            <SegmentedControl items={["Equal power", "S", "Log", "Lin"]} selectedIndex={0} />
          </Field>
          <Section title="Effects on this clip">
            <div className="lst">
              {[
                ["De-esser", "−3.2 dB"],
                ["EQ: Clear voice", "5 bands"],
                ["Compressor", "3.2:1"],
              ].map(([fxName, fxParam]) => (
                <div key={fxName}>
                  <span className="act">
                    <Icon name="fx" size={16} />
                  </span>
                  <div className="ti">
                    <b>{fxName}</b>
                    <span>{fxParam}</span>
                  </div>
                  <Button variant="ghost" label="Open" dataGo="rack" href="#rack" />
                </div>
              ))}
            </div>
            <Button icon="plus" label="Add effect" dataGo="fxbrowser" href="#fxbrowser" />
          </Section>
          <Section title="Quick">
            <div className="row">
              <Chip label="Reverse" />
              <Chip label="Normalize" />
              <Chip label="Duplicate" />
              <Chip label="Bounce" />
            </div>
          </Section>
        </aside>
      </div>
    </ProjectLayout>
  );
};
