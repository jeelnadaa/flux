import React, { useState } from "react";
import { ProjectLayout, Slider, Toggle, Icon } from "@flux/ui";
import { ProjectStore } from "@flux/app-core";

interface FillerReviewScreenProps {
  onNavigate?: (route: string) => void;
}

export const FillerReviewScreen: React.FC<FillerReviewScreenProps> = ({ onNavigate }) => {
  const [activeFilter, setActiveFilter] = useState<string>("All 23");
  const [sensitivity, setSensitivity] = useState<number>(60);
  const [keepNaturalBreaths, setKeepNaturalBreaths] = useState<boolean>(true);

  const [items, setItems] = useState([
    {
      id: "f1",
      context: 'So <b class="sig">um</b> the first thing',
      time: "00:12:04",
      label: "Um",
      checked: true,
    },
    {
      id: "f2",
      context: 'on <b class="sig">the the</b> first night',
      time: "00:12:33",
      label: "Stutter",
      checked: true,
    },
    {
      id: "f3",
      context: 'first night, <b class="sig">you know,</b> before we',
      time: "00:12:36",
      label: "You know",
      checked: true,
    },
    {
      id: "f4",
      context: 'it was <b class="sig">uh</b> around two',
      time: "00:14:02",
      label: "Uh",
      checked: true,
    },
    {
      id: "f5",
      context: 'we were <b class="sig">like</b> forty feet away',
      time: "00:15:18",
      label: "Like, not sure",
      checked: false,
    },
    {
      id: "f6",
      context: 'and <b class="sig">I I I</b> think that',
      time: "00:17:44",
      label: "Stutter",
      checked: true,
    },
  ]);

  const toggleItem = (id: string) => {
    setItems((prev) =>
      prev.map((item) => (item.id === id ? { ...item, checked: !item.checked } : item)),
    );
  };

  const removeAllSelected = () => {
    const store = ProjectStore.get();
    store.pushUndo();
    setItems((prev) => prev.filter((item) => !item.checked));
  };

  const playFillerContext = (timeStr: string) => {
    const parts = timeStr.split(":").map(Number);
    const sec = (parts[0] || 0) * 3600 + (parts[1] || 0) * 60 + (parts[2] || 0);
    const store = ProjectStore.get();
    store.seek(Math.floor(sec * store.getState().sampleRate));
    void store.play();
    setTimeout(() => {
      store.pause();
    }, 1500);
  };

  return (
    <ProjectLayout currentTab="script" onNavigate={onNavigate}>
      <div className="pg in">
        <div className="pgh">
          <div>
            <h1 className="h1">23 fillers, 1:14 saved.</h1>
            <p className="lede">
              Listen to any of them. Words like "so" and "like" are kept when they carry meaning.
            </p>
          </div>
          <div className="row">
            <button type="button" className="btn" onClick={() => onNavigate?.("script")}>
              Back to script
            </button>
            <button type="button" className="btn p" onClick={removeAllSelected}>
              Remove all selected
            </button>
          </div>
        </div>

        <div className="row">
          {["All 23", "Um, uh 14", "Like 3", "You know 2", "Stutters 4", "+ Custom word"].map(
            (chip) => (
              <button
                key={chip}
                type="button"
                className={`chip ${activeFilter === chip ? "on" : ""}`}
                onClick={() => setActiveFilter(chip)}
              >
                {chip}
              </button>
            ),
          )}
        </div>

        <Slider
          label="Sensitivity"
          value={sensitivity}
          valueText="Default"
          onChange={setSensitivity}
        />

        <div className="lst box">
          {items.map((r) => (
            <div key={r.id}>
              <button
                type="button"
                className="btn g"
                aria-label="Play in context"
                onClick={() => playFillerContext(r.time)}
              >
                <Icon name="play" size={14} />
              </button>
              <div className="ti">
                <span
                  style={{ fontSize: 15, color: "var(--tx)", whiteSpace: "normal" }}
                  dangerouslySetInnerHTML={{ __html: r.context }}
                />
                <span className="mono">
                  {r.time} · {r.label}
                </span>
              </div>
              <div style={{ width: 64, flex: "none" }}>
                <Toggle label="" checked={r.checked} onChange={() => toggleItem(r.id)} />
              </div>
            </div>
          ))}
        </div>

        <Toggle
          label="Keep natural breaths"
          checked={keepNaturalBreaths}
          onChange={setKeepNaturalBreaths}
        />
      </div>
    </ProjectLayout>
  );
};
