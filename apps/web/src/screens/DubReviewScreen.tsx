import React, { useState } from "react";
import { ProjectLayout, Pill, Icon, CanvasViz } from "@flux/ui";

interface DubReviewScreenProps {
  onNavigate?: (route: string) => void;
}

export const DubReviewScreen: React.FC<DubReviewScreenProps> = ({ onNavigate }) => {
  const [lines, setLines] = useState([
    {
      id: "d1",
      speaker: "Maya",
      color: "--t4",
      original:
        "The first thing I noticed about the harbor recordings was how much low rumble came through the hull.",
      translated:
        "Lo primero que noté en las grabaciones del puerto fue cuánto retumbo bajo atravesaba el casco.",
      length: "+4%",
      status: "ok" as "ok" | "warn",
    },
    {
      id: "d2",
      speaker: "Tom",
      color: "--t3",
      original: "And that was on the first night, before we moved the mics up to the Kessler deck.",
      translated:
        "Y eso fue la primera noche, antes de que subiéramos los micrófonos a la cubierta Kessler y volviéramos a empezar.",
      length: "+22%",
      status: "warn" as "ok" | "warn",
    },
    {
      id: "d3",
      speaker: "Maya",
      color: "--t4",
      original: "Which meant we had to cut around the engine for the whole second act.",
      translated: "Lo que nos obligó a editar alrededor del motor durante todo el segundo acto.",
      length: "+8%",
      status: "ok" as "ok" | "warn",
    },
  ]);

  const handleShorten = (id: string) => {
    setLines((prev) =>
      prev.map((l) =>
        l.id === id
          ? {
              ...l,
              translated:
                "Y eso fue la primera noche, antes de mover los micrófonos a la cubierta Kessler.",
              length: "+4%",
              status: "ok",
            }
          : l,
      ),
    );
  };

  const fitCount = lines.filter((l) => l.status === "ok").length;
  const warnCount = lines.length - fitCount;

  return (
    <ProjectLayout currentTab="aitools" onNavigate={onNavigate}>
      <div className="pg in">
        <div className="pgh">
          <h1 className="h1">Check each line.</h1>
          <div className="row">
            <Pill label={`${38 + (fitCount - 2)} of 42 fit`} variant="ok" />
            {warnCount > 0 && <Pill label={`${4 - (fitCount - 2)} too long`} variant="warn" />}
            <button type="button" className="btn p" onClick={() => onNavigate?.("export")}>
              Render dub
            </button>
          </div>
        </div>

        <div className="col">
          {lines.map((r) => (
            <div key={r.id} className="box g2">
              <div className="col">
                <span className="b7 row" style={{ gap: 6 }}>
                  <span className="dot" style={{ color: `var(${r.color})` }} />
                  {r.speaker}
                </span>
                <p className="dim">{r.original}</p>
                <div style={{ height: 34 }}>
                  <CanvasViz type="wave" seed={r.original.length} kind="voice" step={2} />
                </div>
              </div>

              <div className="col">
                <div className="between">
                  <span className="lab">Spanish</span>
                  <Pill label={`${r.length} length`} variant={r.status} />
                </div>
                <textarea
                  className="inp"
                  rows={3}
                  aria-label="Translated line"
                  value={r.translated}
                  onChange={(e) => {
                    const val = e.target.value;
                    setLines((prev) =>
                      prev.map((item) => (item.id === r.id ? { ...item, translated: val } : item)),
                    );
                  }}
                />
                <div className="row">
                  <button type="button" className="btn g">
                    <Icon name="play" size={14} />
                    Play
                  </button>
                  <button type="button" className="btn g">
                    Regenerate
                  </button>
                  {r.status === "warn" && (
                    <button type="button" className="btn g" onClick={() => handleShorten(r.id)}>
                      Shorten it
                    </button>
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </ProjectLayout>
  );
};
