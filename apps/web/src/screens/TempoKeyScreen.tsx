import React, { useState, useEffect } from "react";
import { ProjectLayout, Pill, CanvasViz } from "@flux/ui";
import { ProjectStore, TransportState, type FluxProjectState } from "@flux/app-core";

interface TempoKeyScreenProps {
  onNavigate?: (route: string) => void;
}

export const TempoKeyScreen: React.FC<TempoKeyScreenProps> = ({ onNavigate }) => {
  const store = ProjectStore.getInstance();
  const [project, setProject] = useState<FluxProjectState>(() => store.getState());
  const [bpm, setBpm] = useState<number>(84.0);
  const [tapTimes, setTapTimes] = useState<number[]>([]);

  useEffect(() => {
    return store.subscribe((newState: FluxProjectState) => {
      setProject({ ...newState });
    });
  }, [store]);

  const handleDouble = () => setBpm((prev) => +(prev * 2).toFixed(1));
  const handleHalf = () => setBpm((prev) => +(prev / 2).toFixed(1));

  const handleTap = () => {
    const now = Date.now();
    const newTimes = [...tapTimes.filter((t) => now - t < 3000), now];
    setTapTimes(newTimes);
    if (newTimes.length >= 2) {
      const diffs = [];
      for (let i = 1; i < newTimes.length; i++) {
        diffs.push(newTimes[i] - newTimes[i - 1]);
      }
      const avgMs = diffs.reduce((a, b) => a + b, 0) / diffs.length;
      if (avgMs > 0) {
        setBpm(+(60000 / avgMs).toFixed(1));
      }
    }
  };

  const keyTimeline = [
    { key: "D min", width: "62%", color: "--t1" },
    { key: "F maj", width: "16%", color: "--t3" },
    { key: "D min", width: "22%", color: "--t1" },
  ];

  const isPlaying = project.transportState === TransportState.Playing;
  const timecode = store.formatTimecode(project.playheadFrame);

  return (
    <ProjectLayout
      currentTab="aitools"
      onNavigate={onNavigate}
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
      <div className="pg in">
        <h1 className="h1">Tempo and key</h1>

        <div className="g4">
          <div className="box inv">
            <div className="stat">{bpm.toFixed(1)}</div>
            <span className="lab">BPM · confidence 96%</span>
            <div className="row" style={{ marginTop: 8 }}>
              <button type="button" className="chip" onClick={handleDouble}>
                ×2
              </button>
              <button type="button" className="chip" onClick={handleHalf}>
                ÷2
              </button>
              <button type="button" className="chip" onClick={handleTap}>
                Tap
              </button>
            </div>
          </div>

          <div className="box inv">
            <div className="stat">D min</div>
            <span className="lab">Key · confidence 88%</span>
            <div className="row" style={{ marginTop: 8 }}>
              <Pill label="7A Camelot" />
              <Pill label="6m Open Key" />
            </div>
          </div>

          <div className="box inv">
            <div className="stat">440</div>
            <span className="lab">Tuning Hz</span>
          </div>

          <div className="box inv">
            <div className="stat">4/4</div>
            <span className="lab">Meter</span>
          </div>
        </div>

        <div className="box col">
          <span className="lab">Beat grid</span>
          <div
            className="vz"
            style={{
              height: 120,
              background:
                "repeating-linear-gradient(90deg, var(--s2) 0 calc(6.25% - 1px), var(--ln2) calc(6.25% - 1px) 6.25%)",
            }}
          >
            <CanvasViz type="wave" seed={3} kind="drums" color="--t3" step={2.5} />
          </div>

          <span className="lab">Key over time</span>
          <div className="row" style={{ flexWrap: "nowrap", gap: 2 }}>
            {keyTimeline.map((k, idx) => (
              <span
                key={idx}
                className="pill"
                style={{
                  width: k.width,
                  background: `color-mix(in srgb, var(${k.color}) 30%, transparent)`,
                  overflow: "hidden",
                }}
              >
                {k.key}
              </span>
            ))}
          </div>
        </div>

        <div className="row">
          <button type="button" className="btn p">
            Snap edits to beats
          </button>
          <button type="button" className="btn">
            Warp clip to grid
          </button>
          <button type="button" className="btn">
            Write to file tags
          </button>
        </div>
      </div>
    </ProjectLayout>
  );
};
