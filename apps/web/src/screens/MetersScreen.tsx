import React, { useState } from "react";
import { ProjectLayout, CanvasViz } from "@flux/ui";

export const MetersScreen: React.FC = () => {
  const [isReset, setIsReset] = useState(false);

  const stats = [
    [isReset ? "−inf" : "−14.2", "Integrated LUFS"],
    [isReset ? "−inf" : "−12.8", "Short-term LUFS"],
    [isReset ? "0.0" : "6.4", "Loudness range LU"],
    [isReset ? "−inf" : "−1.1", "True peak dBTP"],
  ];

  const handleReset = () => {
    setIsReset(!isReset);
  };

  return (
    <ProjectLayout currentTab="mix">
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
