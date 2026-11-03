import React, { useState } from "react";
import { ProjectLayout, Slider, Toggle, CanvasViz } from "@flux/ui";

export const HallScreen: React.FC = () => {
  // Row 10 controls
  const [row10Enabled, setRow10Enabled] = useState(true);
  const [row, setRow] = useState(28); // Row 10
  const [crowd, setCrowd] = useState(40); // -18 dB
  const [paColor, setPaColor] = useState(30); // 30%
  const [applause, setApplause] = useState(true);

  // Next door controls
  const [nextDoorEnabled, setNextDoorEnabled] = useState(false);
  const [wallThickness, setWallThickness] = useState(55); // Low-pass 620 Hz
  const [bassThroughWall, setBassThroughWall] = useState(60); // +4 dB
  const [theirRoom, setTheirRoom] = useState(35); // 0.6 s
  const [levelDrifts, setLevelDrifts] = useState(true);

  return (
    <ProjectLayout currentTab="stage">
      <div className="pg in">
        <h1 className="h1">Two places to listen from.</h1>
        <div className="g2">
          {/* Row 10 Box */}
          <div className="box col pal-cathedral" style={{ gap: "14px" }}>
            <div className="between">
              <h2 className="h2">Row 10</h2>
              <Toggle checked={row10Enabled} onChange={setRow10Enabled} />
            </div>
            <p className="dim">
              You're at a show, ten rows back, with the crowd around you.
            </p>
            <div className="vz" style={{ height: "210px" }}>
              <CanvasViz type="room" w={1.5} d={1.2} h={0.7} height={210} />
            </div>
            <Slider
              label="Row"
              value={row}
              valueText={row === 28 ? "Row 10" : `Row ${Math.round(1 + (row / 100) * 29)}`}
              onChange={setRow}
            />
            <Slider
              label="Crowd"
              value={crowd}
              valueText={crowd === 40 ? "−18 dB" : `−${Math.round(36 - (crowd / 100) * 30)} dB`}
              onChange={setCrowd}
            />
            <Slider
              label="PA color"
              value={paColor}
              valueText={`${paColor}%`}
              onChange={setPaColor}
            />
            <Toggle
              label="Applause at the start and end"
              checked={applause}
              onChange={setApplause}
            />
          </div>

          {/* Next door Box */}
          <div className="box col pal-nextdoor" style={{ gap: "14px" }}>
            <div className="between">
              <h2 className="h2">Next door</h2>
              <Toggle checked={nextDoorEnabled} onChange={setNextDoorEnabled} />
            </div>
            <p className="dim">The same song, through your neighbor's wall.</p>
            <div className="vz" style={{ height: "210px", filter: "blur(2px)" }}>
              <CanvasViz
                type="wave"
                seed={1}
                stereo={true}
                gain={0.55}
                step={3}
                height={210}
              />
            </div>
            <Slider
              label="Wall thickness"
              value={wallThickness}
              valueText={
                wallThickness === 55
                  ? "Low-pass 620 Hz"
                  : `Low-pass ${Math.round(200 + (wallThickness / 100) * 1200)} Hz`
              }
              onChange={setWallThickness}
            />
            <Slider
              label="Bass through wall"
              value={bassThroughWall}
              valueText={
                bassThroughWall === 60
                  ? "+4 dB"
                  : `+${Math.round((bassThroughWall / 100) * 10)} dB`
              }
              onChange={setBassThroughWall}
            />
            <Slider
              label="Their room"
              value={theirRoom}
              valueText={
                theirRoom === 35
                  ? "0.6 s"
                  : `${(0.1 + (theirRoom / 100) * 1.5).toFixed(1)} s`
              }
              onChange={setTheirRoom}
            />
            <Toggle
              label="Level drifts over time"
              checked={levelDrifts}
              onChange={setLevelDrifts}
            />
          </div>
        </div>
      </div>
    </ProjectLayout>
  );
};
