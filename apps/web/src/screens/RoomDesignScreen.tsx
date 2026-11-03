import React, { useState } from "react";
import { ProjectLayout, Slider, Toggle, Section, Field, Icon, CanvasViz } from "@flux/ui";

const WALL_MATERIALS = ["Plaster", "Concrete", "Brick", "Wood", "Glass", "Curtain"];
const FLOOR_MATERIALS = ["Wood", "Carpet", "Stone", "Tile"];
const CEILING_MATERIALS = ["Plaster", "Acoustic tile", "Wood"];

export const RoomDesignScreen: React.FC = () => {
  const [width, setWidth] = useState(46); // 12 m
  const [depth, setDepth] = useState(60); // 18 m
  const [height, setHeight] = useState(32); // 6 m

  const [wallMat, setWallMat] = useState(0);
  const [floorMat, setFloorMat] = useState(0);
  const [ceilingMat, setCeilingMat] = useState(0);

  const [linkStage, setLinkStage] = useState(true);

  // Compute interactive dimensions and Sabine acoustics
  const widthM = width === 46 ? 12 : Math.round(2 + (width / 100) * 22);
  const depthM = depth === 60 ? 18 : Math.round(2 + (depth / 100) * 26);
  const heightM = height === 32 ? 6 : Math.round(2 + (height / 100) * 12);

  return (
    <ProjectLayout currentTab="stage">
      <div className="split">
        <div className="a">
          <div className="tlbar">
            <b style={{ fontSize: "15px", fontWeight: 800 }}>Room designer</b>
            <span className="sp" style={{ flex: 1 }} />
            <a className="btn g" href="#rooms" data-go="rooms" aria-label="Close" title="Close">
              <Icon name="x" size={16} />
            </a>
          </div>
          <div
            style={{
              flex: 1,
              padding: "10px",
              display: "grid",
              gridTemplateRows: "minmax(0, 1fr) 110px",
              gap: "10px",
              minHeight: "440px",
            }}
          >
            <div
              className="vz"
              role="img"
              aria-label="Room in perspective with reflection paths"
              style={{ position: "relative" }}
            >
              <CanvasViz
                type="room"
                w={widthM / 10}
                d={depthM / 18}
                h={heightM / 10}
                style={{ position: "absolute", inset: 0 }}
              />
            </div>
            <div className="vz">
              <CanvasViz type="decay" rt={2.1} />
            </div>
          </div>
        </div>

        <aside className="b in">
          <h3 className="h3">Size</h3>
          <div className="col">
            <Slider label="Width" value={width} valueText={`${widthM} m`} onChange={setWidth} />
            <Slider label="Depth" value={depth} valueText={`${depthM} m`} onChange={setDepth} />
            <Slider label="Height" value={height} valueText={`${heightM} m`} onChange={setHeight} />
          </div>

          <Section title="Walls">
            <Field label="Walls">
              <select
                className="sel"
                value={wallMat}
                onChange={(e) => setWallMat(Number(e.target.value))}
              >
                {WALL_MATERIALS.map((m, i) => (
                  <option key={m} value={i}>
                    {m}
                  </option>
                ))}
              </select>
            </Field>
            <Field label="Floor">
              <select
                className="sel"
                value={floorMat}
                onChange={(e) => setFloorMat(Number(e.target.value))}
              >
                {FLOOR_MATERIALS.map((m, i) => (
                  <option key={m} value={i}>
                    {m}
                  </option>
                ))}
              </select>
            </Field>
            <Field label="Ceiling">
              <select
                className="sel"
                value={ceilingMat}
                onChange={(e) => setCeilingMat(Number(e.target.value))}
              >
                {CEILING_MATERIALS.map((m, i) => (
                  <option key={m} value={i}>
                    {m}
                  </option>
                ))}
              </select>
            </Field>
          </Section>

          <Section title="Result">
            <div className="g2">
              <div>
                <b className="b9" style={{ fontSize: "26px" }}>
                  2.1 s
                </b>
                <br />
                <span className="lab">RT60</span>
              </div>
              <div>
                <b className="b9" style={{ fontSize: "26px" }}>
                  18 ms
                </b>
                <br />
                <span className="lab">First reflection</span>
              </div>
            </div>
          </Section>

          <Toggle
            label="Link source and listener to the stage"
            checked={linkStage}
            onChange={setLinkStage}
          />
          <a className="btn p lg" href="#stage" data-go="stage">
            Use this room
          </a>
        </aside>
      </div>
    </ProjectLayout>
  );
};
