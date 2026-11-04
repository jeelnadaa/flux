import React, { useState, useEffect, useRef } from "react";
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
import { SnappingEngine, ProjectStore, FluxProjectState, TransportState } from "@flux/app-core";

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
  const store = ProjectStore.get();
  const [projectState, setProjectState] = useState<FluxProjectState>(() => store.getState());
  const [snapZeroCross, setSnapZeroCross] = useState<boolean>(true);
  const [snapGrid, setSnapGrid] = useState<boolean>(false);
  const [ripple, setRipple] = useState<boolean>(false);
  const [toolIdx, setToolIdx] = useState<number>(0);
  const [isScrubbing, setIsScrubbing] = useState<boolean>(false);
  const [fadeCurveIdx, setFadeCurveIdx] = useState<number>(0);
  const [isReversed, setIsReversed] = useState<boolean>(false);

  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);
  const [marquee, setMarquee] = useState<{
    active: boolean;
    startX: number;
    startY: number;
    currentX: number;
    currentY: number;
  } | null>(null);

  const [pinchDist, setPinchDist] = useState<number | null>(null);

  useEffect(() => {
    const unsub = store.subscribe((s) => {
      setProjectState({ ...s });
    });
    return unsub;
  }, [store]);

  const handleSplit = () => {
    store.splitSelectedClip();
  };

  const handleDelete = () => {
    store.deleteSelectedClip();
  };

  const toggleReverse = () => {
    setIsReversed((prev) => !prev);
  };

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (
        document.activeElement?.tagName === "INPUT" ||
        document.activeElement?.tagName === "SELECT" ||
        document.activeElement?.tagName === "TEXTAREA"
      ) {
        return;
      }

      if (e.code === "Space") {
        e.preventDefault();
        store.togglePlay();
      } else if (e.key === "s" || e.key === "S") {
        e.preventDefault();
        store.splitSelectedClip();
      } else if (e.key === "Delete" || e.key === "Backspace") {
        e.preventDefault();
        store.deleteSelectedClip();
      } else if (e.key === "k" && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        window.location.hash = "#palette";
      } else if (e.key === "?") {
        e.preventDefault();
        window.location.hash = "#shortcuts";
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [store]);

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY });
  };

  const handleTimelineMouseDown = (e: React.MouseEvent<HTMLDivElement>) => {
    if (e.button !== 0) return; // Only left-click initiates marquee
    if ((e.target as HTMLElement).closest(".clip")) return; // Don't start marquee on clip
    const rect = e.currentTarget.getBoundingClientRect();
    setMarquee({
      active: true,
      startX: e.clientX - rect.left,
      startY: e.clientY - rect.top,
      currentX: e.clientX - rect.left,
      currentY: e.clientY - rect.top,
    });
  };

  const handleTimelineMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!marquee || !marquee.active) return;
    const rect = e.currentTarget.getBoundingClientRect();
    setMarquee((prev) =>
      prev
        ? {
            ...prev,
            currentX: e.clientX - rect.left,
            currentY: e.clientY - rect.top,
          }
        : null,
    );
  };

  const handleTimelineMouseUp = () => {
    if (marquee && marquee.active) {
      setMarquee(null);
    }
  };

  const handleTouchStart = (e: React.TouchEvent) => {
    if (e.touches.length === 2) {
      const dx = e.touches[0].clientX - e.touches[1].clientX;
      const dy = e.touches[0].clientY - e.touches[1].clientY;
      setPinchDist(Math.hypot(dx, dy));
    }
  };

  const handleTouchMove = (e: React.TouchEvent) => {
    if (e.touches.length === 2 && pinchDist !== null) {
      const dx = e.touches[0].clientX - e.touches[1].clientX;
      const dy = e.touches[0].clientY - e.touches[1].clientY;
      const newDist = Math.hypot(dx, dy);
      // Pinch gesture registered
      setPinchDist(newDist);
    }
  };

  const handleTouchEnd = () => {
    setPinchDist(null);
  };

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
    const frac = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    store.seek(Math.floor(frac * projectState.totalFrames));
    setIsScrubbing(true);
    e.currentTarget.setPointerCapture(e.pointerId);
  };

  const handleRulerPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!isScrubbing) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const frac = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    store.seek(Math.floor(frac * projectState.totalFrames));
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

  const timecodeSec = projectState.playheadFrame / projectState.sampleRate;
  const tcM = Math.floor(timecodeSec / 60);
  const tcS = Math.floor(timecodeSec % 60);
  const tcMs = Math.floor((timecodeSec % 1) * 1000);
  const timecodeFormatted = `${tcM.toString().padStart(2, "0")}:${tcS.toString().padStart(2, "0")}.${tcMs.toString().padStart(3, "0")}`;

  let selectedClipName = "No clip selected";
  let selectedTrackName = "Master";
  for (const t of projectState.tracks) {
    const c = t.clips.find((clip) => clip.id === projectState.selectedClipId);
    if (c) {
      selectedClipName = c.name;
      selectedTrackName = t.name;
      break;
    }
  }

  return (
    <ProjectLayout
      currentTab="edit"
      projectName={projectState.name}
      projectStatus="Local-first · 32-bit float"
      rightActions={topBarRight}
      isPlaying={projectState.transportState === TransportState.Playing}
      timecode={timecodeFormatted}
      isLooping={projectState.loopEnabled}
      onPlayPause={() => store.togglePlay()}
      onStop={() => store.stop()}
      onBackToStart={() => store.seek(0)}
      onLoopToggle={() => store.setLoop(!projectState.loopEnabled)}
      leftMeter={projectState.meterLevels.leftPeak}
      rightMeter={projectState.meterLevels.rightPeak}
    >
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
            <Button
              variant="ghost"
              iconOnly
              icon="split"
              ariaLabel="Split at playhead"
              onClick={handleSplit}
            />
            <Button
              variant="ghost"
              iconOnly
              icon="trash"
              ariaLabel="Delete"
              onClick={handleDelete}
            />
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
          <div
            className="tl"
            data-tl=""
            onContextMenu={handleContextMenu}
            onMouseDown={handleTimelineMouseDown}
            onMouseMove={handleTimelineMouseMove}
            onMouseUp={handleTimelineMouseUp}
            onTouchStart={handleTouchStart}
            onTouchMove={handleTouchMove}
            onTouchEnd={handleTouchEnd}
            onClick={() => setContextMenu(null)}
          >
            {marquee && marquee.active && (
              <div
                className="marquee-overlay"
                style={{
                  position: "absolute",
                  left: Math.min(marquee.startX, marquee.currentX),
                  top: Math.min(marquee.startY, marquee.currentY),
                  width: Math.abs(marquee.currentX - marquee.startX),
                  height: Math.abs(marquee.currentY - marquee.startY),
                  border: "1px dashed var(--sig)",
                  background: "color-mix(in srgb, var(--sig) 18%, transparent)",
                  pointerEvents: "none",
                  zIndex: 25,
                }}
              />
            )}
            <div
              className="ruler"
              onPointerDown={handleRulerPointerDown}
              onPointerMove={handleRulerPointerMove}
              onPointerUp={handleRulerPointerUp}
              onPointerCancel={handleRulerPointerUp}
            >
              <CanvasViz type="ruler" />
            </div>
            {projectState.tracks.map((trk) => (
              <div key={trk.id} className={`trk ${trk.tall ? "tall" : ""}`}>
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
                      onClick={() => store.setTrackMute(trk.id, !trk.muted)}
                    >
                      M
                    </button>
                    <button
                      className={`s ${trk.solo ? "on" : ""}`}
                      type="button"
                      aria-label={`Solo ${trk.name}`}
                      onClick={() => store.setTrackSolo(trk.id, !trk.solo)}
                    >
                      S
                    </button>
                    <button type="button" aria-label={`Arm ${trk.name} for recording`}>
                      R
                    </button>
                  </div>
                </div>
                <div className="lane">
                  {trk.clips.map((c) => {
                    const leftPct =
                      (c.startFrame / Math.max(1, projectState.totalFrames)) * 100;
                    const widthPct =
                      (c.durationFrames / Math.max(1, projectState.totalFrames)) * 100;
                    return (
                      <div
                        key={c.id}
                        style={{ display: "contents" }}
                        onClick={(e) => {
                          e.stopPropagation();
                          store.selectClip(c.id);
                        }}
                      >
                        <ClipView
                          left={leftPct}
                          width={widthPct}
                          colorVar={c.colorVar}
                          seed={c.seed}
                          kind={c.kind}
                          label={c.name}
                          selected={c.id === projectState.selectedClipId}
                          fadeIn={c.fadeInFrames > 0 ? 18 : undefined}
                          fadeOut={c.fadeOutFrames > 0 ? 18 : undefined}
                        />
                      </div>
                    );
                  })}
                </div>
              </div>
            ))}
            <span
              className="ph"
              data-ph=""
              style={{
                left: `${(projectState.playheadFrame / Math.max(1, projectState.totalFrames)) * 100}%`,
              }}
            />
          </div>
          {contextMenu && (
            <div
              className="dlg"
              style={{
                position: "fixed",
                left: Math.min(window.innerWidth - 200, contextMenu.x),
                top: Math.min(window.innerHeight - 200, contextMenu.y),
                width: "180px",
                padding: "8px",
                zIndex: 100,
                gap: "4px",
              }}
              onClick={(e) => e.stopPropagation()}
            >
              <button
                className="btn g"
                style={{ width: "100%", justifyContent: "flex-start" }}
                onClick={() => {
                  handleSplit();
                  setContextMenu(null);
                }}
              >
                Split
              </button>
              <button
                className="btn g"
                style={{ width: "100%", justifyContent: "flex-start" }}
                onClick={() => {
                  toggleReverse();
                  setContextMenu(null);
                }}
              >
                Reverse
              </button>
              <button
                className="btn g"
                style={{ width: "100%", justifyContent: "flex-start" }}
                onClick={() => {
                  handleDelete();
                  setContextMenu(null);
                }}
              >
                Delete
              </button>
              <button
                className="btn g"
                style={{ width: "100%", justifyContent: "flex-start" }}
                onClick={() => setContextMenu(null)}
              >
                Close
              </button>
            </div>
          )}
        </div>

        {/* Clip inspector */}
        <aside className="b in">
          <div className="between">
            <h3 className="h3">{selectedClipName}</h3>
            <Pill label={selectedTrackName} />
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
            <SegmentedControl
              items={["Equal power", "S", "Log", "Lin"]}
              selectedIndex={fadeCurveIdx}
              onChange={(_val, idx) => setFadeCurveIdx(idx)}
            />
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
              <Chip label="Reverse" active={isReversed} onClick={toggleReverse} />
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
