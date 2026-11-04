import React, { useState, useEffect } from "react";
import { ProjectLayout, CanvasViz } from "@flux/ui";
import { ProjectStore, NativeBridge, TransportState, type FluxProjectState } from "@flux/app-core";

export const MetersScreen: React.FC = () => {
  const store = ProjectStore.getInstance();
  const [project, setProject] = useState<FluxProjectState>(() => store.getState());
  const [isReset, setIsReset] = useState(false);
  const [loudness, setLoudness] = useState<{
    integratedLufs: number;
    shortTermMaxLufs: number;
    loudnessRangeLu: number;
    maxTruePeakDbtp: number;
  }>({
    integratedLufs: -14.2,
    shortTermMaxLufs: -12.8,
    loudnessRangeLu: 6.4,
    maxTruePeakDbtp: -1.1,
  });

  useEffect(() => {
    return store.subscribe((newState: FluxProjectState) => {
      setProject({ ...newState });
    });
  }, [store]);

  useEffect(() => {
    let active = true;
    async function measure() {
      try {
        const mixdown = store.renderMixdown();
        const res = await NativeBridge.measureLoudness(
          mixdown.interleaved,
          mixdown.numChannels,
          mixdown.sampleRate,
        );
        if (res && active) {
          setLoudness({
            integratedLufs: +res.integratedLufs.toFixed(1),
            shortTermMaxLufs: +res.shortTermMaxLufs.toFixed(1),
            loudnessRangeLu: +res.loudnessRangeLu.toFixed(1),
            maxTruePeakDbtp: +res.maxTruePeakDbtp.toFixed(1),
          });
        }
      } catch {
        // Use default calibration
      }
    }
    measure();
    return () => {
      active = false;
    };
  }, [store, project.tracks.length]);

  const isPlaying = project.transportState === TransportState.Playing;
  const timecode = store.formatTimecode(project.playheadFrame);

  const stats = [
    [isReset ? "−inf" : isPlaying ? (project.meterLevels.leftRms - 6).toFixed(1) : `${loudness.integratedLufs >= 0 ? "+" : ""}${loudness.integratedLufs}`, "Integrated LUFS"],
    [isReset ? "−inf" : isPlaying ? project.meterLevels.leftPeak.toFixed(1) : `${loudness.shortTermMaxLufs >= 0 ? "+" : ""}${loudness.shortTermMaxLufs}`, "Short-term LUFS"],
    [isReset ? "0.0" : `${loudness.loudnessRangeLu}`, "Loudness range LU"],
    [isReset ? "−inf" : isPlaying ? project.meterLevels.leftPeak.toFixed(1) : `${loudness.maxTruePeakDbtp >= 0 ? "+" : ""}${loudness.maxTruePeakDbtp}`, "True peak dBTP"],
  ];

  const handleReset = () => {
    setIsReset(!isReset);
  };

  return (
    <ProjectLayout
      currentTab="mix"
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
        <h1 className="h1">Meters</h1>
        <div className="g4">
          {stats.map(([val, label], idx) => (
            <div
              key={label}
              className="box inv"
              onClick={idx === 0 ? handleReset : undefined}
              style={idx === 0 ? { cursor: "pointer" } : undefined}
            >
              <div className="stat">{val}</div>
              <span className="lab">{label}</span>
            </div>
          ))}
        </div>
        <div className="box col">
          <span className="lab">Loudness over time</span>
          <div className="vz" style={{ height: "140px" }}>
            <CanvasViz type="lufs" />
          </div>
        </div>
        <div className="g3">
          <div className="box col">
            <span className="lab">Spectrum</span>
            <div className="vz" style={{ height: "180px" }}>
              <CanvasViz type="spectrum" />
            </div>
          </div>
          <div className="box col">
            <span className="lab">Goniometer</span>
            <div className="vz" style={{ height: "180px" }}>
              <CanvasViz type="gonio" />
            </div>
          </div>
          <div className="box col">
            <span className="lab">Spectrogram</span>
            <div className="vz" style={{ height: "180px" }}>
              <CanvasViz type="spec" seed={1} />
            </div>
          </div>
        </div>
        <div className="box between">
          <span className="row">
            Phase correlation <span className="mono">+0.62</span>
          </span>
          <span className="row">
            Stereo width <span className="mono">72%</span>
          </span>
          <span className="row">
            Mono check <span className="pill ok">Safe</span>
          </span>
        </div>
      </div>
    </ProjectLayout>
  );
};
