import React, { useState, useEffect } from "react";
import {
  Button,
  Callout,
  CanvasViz,
  Chip,
  Field,
  ProjectLayout,
  Section,
  SegmentedControl,
  Select,
  Slider,
} from "@flux/ui";
import { ProjectStore, TransportState, type FluxProjectState } from "@flux/app-core";

export const SpectralScreen: React.FC = () => {
  const store = ProjectStore.getInstance();
  const [project, setProject] = useState<FluxProjectState>(() => store.getState());

  useEffect(() => {
    return store.subscribe((newState: FluxProjectState) => {
      setProject({ ...newState });
    });
  }, [store]);

  const [viewMode, setViewMode] = useState<number>(2); // 0: Spectrogram, 1: Waveform, 2: Both
  const [toolIdx, setToolIdx] = useState<number>(0);
  const [scale, setScale] = useState<string>("Log");
  const [windowSize, setWindowSize] = useState<string>("4096 window");
  const [fillFrom, setFillFrom] = useState<number>(0);

  const [boxPos, setBoxPos] = useState({
    left: 54,
    top: 26,
    width: 16,
    height: 26,
    label: "2.1 to 4.8 kHz · 1.42 s",
  });
  const [isDragging, setIsDragging] = useState(false);
  const dragStartRef = React.useRef({ x: 0, y: 0, initLeft: 54, initTop: 26 });

  const handleBoxPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    setIsDragging(true);
    dragStartRef.current = {
      x: e.clientX,
      y: e.clientY,
      initLeft: boxPos.left,
      initTop: boxPos.top,
    };
    e.currentTarget.setPointerCapture(e.pointerId);
  };

  const handleBoxPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!isDragging) return;
    const parent = e.currentTarget.parentElement;
    if (!parent) return;
    const rect = parent.getBoundingClientRect();
    const dx = ((e.clientX - dragStartRef.current.x) / rect.width) * 100;
    const dy = ((e.clientY - dragStartRef.current.y) / rect.height) * 100;
    const newL = Math.max(0, Math.min(100 - boxPos.width, dragStartRef.current.initLeft + dx));
    const newT = Math.max(0, Math.min(100 - boxPos.height, dragStartRef.current.initTop + dy));
    setBoxPos((prev) => ({ ...prev, left: newL, top: newT }));
  };

  const handleBoxPointerUp = (e: React.PointerEvent<HTMLDivElement>) => {
    if (isDragging) {
      setIsDragging(false);
      try {
        e.currentTarget.releasePointerCapture(e.pointerId);
      } catch {
        // Ignored
      }
    }
  };

  const isPlaying = project.transportState === TransportState.Playing;
  const timecode = store.formatTimecode(project.playheadFrame);

  return (
    <ProjectLayout
      currentTab="spectral"
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
          {/* Top toolbar */}
          <div className="tlbar">
            <SegmentedControl
              items={["Spectrogram", "Waveform", "Both"]}
              selectedIndex={viewMode}
              onChange={(_val, idx) => setViewMode(idx)}
            />
            <SegmentedControl
              items={["Rectangle", "Lasso", "Brush", "Harmonic"]}
              selectedIndex={toolIdx}
              onChange={(_val, idx) => setToolIdx(idx)}
            />
            <span className="sp" style={{ flex: 1 }} />
            <Chip label="Mel" active={scale === "Mel"} onClick={() => setScale("Mel")} />
            <Chip label="Log" active={scale === "Log"} onClick={() => setScale("Log")} />
            <Chip label="Linear" active={scale === "Linear"} onClick={() => setScale("Linear")} />
            <Field label="">
              <Select
                options={["4096 window", "2048 window", "8192 window"]}
                value={windowSize}
                onChange={setWindowSize}
              />
            </Field>
          </div>

          {/* Main Hybrid Viewport */}
          <div
            style={{
              flex: 1,
              display: "flex",
              flexDirection: "column",
              minHeight: "300px",
              position: "relative",
            }}
          >
            {(viewMode === 1 || viewMode === 2) && (
              <div
                style={{
                  height: viewMode === 2 ? "30%" : "100%",
                  minHeight: "80px",
                  borderBottom: viewMode === 2 ? "1px solid var(--ln)" : "none",
                  position: "relative",
                }}
              >
                <CanvasViz type="wave" seed={1} stereo="1" step={2} />
              </div>
            )}
            {(viewMode === 0 || viewMode === 2) && (
              <div
                style={{
                  flex: 1,
                  position: "relative",
                  minHeight: "220px",
                }}
              >
                <CanvasViz type="spec" seed={3} />
                <div
                  data-testid="spectral-box"
                  style={{
                    position: "absolute",
                    left: `${boxPos.left}%`,
                    top: `${boxPos.top}%`,
                    width: `${boxPos.width}%`,
                    height: `${boxPos.height}%`,
                    border: "2px dashed var(--tx)",
                    borderRadius: "2px",
                    cursor: "move",
                  }}
                  onPointerDown={handleBoxPointerDown}
                  onPointerMove={handleBoxPointerMove}
                  onPointerUp={handleBoxPointerUp}
                >
                  <span
                    className="mono"
                    style={{
                      position: "absolute",
                      top: "-22px",
                      right: 0,
                      padding: "1px 6px",
                      borderRadius: "3px",
                      background: "var(--tx)",
                      color: "var(--s0)",
                      whiteSpace: "nowrap",
                    }}
                  >
                    {boxPos.label}
                  </span>
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Sidebar */}
        <aside className="b in">
          <h3 className="h3">Selection</h3>
          <div className="g2 mono dim">
            <span>2.1 to 4.8 kHz</span>
            <span>1:02.1 to 1:03.5</span>
          </div>

          <Section title="Repair">
            <div className="col">
              <Button variant="primary" icon="pen" label="Remove and fill in" />
              <Slider label="Amount" defaultValue={90} valueText="−30 dB" />
              <Field label="Fill from">
                <SegmentedControl
                  items={["Both sides", "Before", "After"]}
                  selectedIndex={fillFrom}
                  onChange={(_val, idx) => setFillFrom(idx)}
                />
              </Field>
            </div>
          </Section>

          <Section title="Also">
            <div className="row">
              <Chip label="Attenuate" />
              <Chip label="Copy to track" />
              <Chip label="Listen to selection" />
            </div>
          </Section>

          <Section title="Spectrum at playhead">
            <div className="vz" style={{ height: "110px" }}>
              <CanvasViz type="spectrum" />
            </div>
          </Section>

          <Callout variant="info">
            Repairs are stored as edits. Your original audio stays unchanged.
          </Callout>
        </aside>
      </div>
    </ProjectLayout>
  );
};
