import React, { useState } from "react";
import { ProjectLayout, Slider, Icon, CanvasViz } from "@flux/ui";

const ROOMS = [
  { name: "Cathedral", rt: 6.8, rtStr: "6.8 s" },
  { name: "Bathroom", rt: 0.9, rtStr: "0.9 s" },
  { name: "Concert hall", rt: 3.2, rtStr: "3.2 s" },
  { name: "Cave", rt: 4.4, rtStr: "4.4 s" },
  { name: "Parking garage", rt: 2.6, rtStr: "2.6 s" },
  { name: "Studio booth", rt: 0.2, rtStr: "0.2 s" },
  { name: "Stairwell", rt: 1.8, rtStr: "1.8 s" },
  { name: "Living room", rt: 0.5, rtStr: "0.5 s" },
  { name: "Car interior", rt: 0.1, rtStr: "0.1 s" },
  { name: "Stadium", rt: 5.1, rtStr: "5.1 s" },
  { name: "Forest clearing", rt: 0.7, rtStr: "0.7 s" },
  { name: "Subway tunnel", rt: 3.8, rtStr: "3.8 s" },
];

export const RoomsScreen: React.FC = () => {
  const [selectedIdx, setSelectedIdx] = useState(2); // Concert hall on by default
  const [mix, setMix] = useState(34);
  const [distance, setDistance] = useState(50);
  const [earlyRefl, setEarlyRefl] = useState(70);

  return (
    <ProjectLayout currentTab="stage">
      <div className="pg in">
        <div className="pgh">
          <div>
            <h1 className="h1">Put it in a room.</h1>
            <p className="lede">Each room is real reflections, rendered around your head.</p>
          </div>
          <a className="btn p" href="#roomdesign" data-go="roomdesign">
            <Icon name="room" size={16} />
            <span>Design your own</span>
          </a>
        </div>

        <div className="g4">
          {ROOMS.map((r, i) => (
            <button
              key={r.name}
              className={`tile ${i === selectedIdx ? "on" : ""}`}
              type="button"
              onClick={() => setSelectedIdx(i)}
            >
              <div className="thumb">
                <CanvasViz type="decay" rt={r.rt} />
              </div>
              <b>{r.name}</b>
              <span className="mono">RT60 {r.rtStr}</span>
            </button>
          ))}
        </div>

        <div className="box g3">
          <Slider label="Mix" value={mix} valueText={`${mix}%`} onChange={setMix} />
          <Slider
            label="Your distance"
            value={distance}
            valueText={`${Math.round(1 + (distance / 100) * 6)} m`}
            onChange={setDistance}
          />
          <Slider
            label="Early reflections"
            value={earlyRefl}
            valueText={`${earlyRefl}%`}
            onChange={setEarlyRefl}
          />
        </div>
      </div>
    </ProjectLayout>
  );
};
