import React, { useState } from "react";
import { ProjectLayout, CanvasViz, Icon } from "@flux/ui";

interface HistoryEntry {
  id: string;
  name: string;
  timeAgo: string;
}

const INITIAL_HISTORY: HistoryEntry[] = [
  { id: "h1", name: 'Move "Chorus" to 0:42.318', timeAgo: "12 s ago" },
  { id: "h2", name: 'Fade out 120 ms on "Chorus"', timeAgo: "40 s ago" },
  { id: "h3", name: "De-esser added to Vocals", timeAgo: "3 min ago" },
  { id: "h4", name: 'Split "Lead take 3" at 1:14.002', timeAgo: "6 min ago" },
  { id: "h5", name: "Snapshot: before mastering", timeAgo: "20 min ago" },
  { id: "h6", name: "Import harbor-lights.flac", timeAgo: "Yesterday" },
];

export const HistoryScreen: React.FC = () => {
  const [historyList, setHistoryList] = useState<HistoryEntry[]>(INITIAL_HISTORY);
  const [currentIdx, setCurrentIdx] = useState(0);

  const handleSaveSnapshot = () => {
    const snapName = `Snapshot: ${new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`;
    setHistoryList((prev) => [
      { id: `snap_${Date.now()}`, name: snapName, timeAgo: "Just now" },
      ...prev,
    ]);
    setCurrentIdx(0);
  };

  const handleGoTo = (idx: number) => {
    setCurrentIdx(idx);
  };

  return (
    <ProjectLayout currentTab="edit">
      <div className="pg in">
        <div className="pgh">
          <div>
            <h1 className="h1">History</h1>
            <p className="lede">
              Every change, kept. Go back to any point, or branch off and keep both.
            </p>
          </div>
          <button className="btn" type="button" onClick={handleSaveSnapshot}>
            <Icon name="disk" />
            <span>Save a snapshot</span>
          </button>
        </div>

        <div className="vz" style={{ height: "170px" }}>
          <CanvasViz type="history" height={170} />
        </div>

        <div className="row">
          <span className="row">
            <span className="dot sig" />
            You are here
          </span>
          <span className="row">
            <span className="dot" style={{ color: "var(--t1)" }} />
            Branch: wider chorus
          </span>
          <span className="row">
            <span className="dot" style={{ color: "var(--t3)" }} />
            Branch: no guitars
          </span>
        </div>

        <div className="lst box">
          {historyList.map((entry, idx) => (
            <div key={entry.id}>
              <div className="ti">
                <b>{entry.name}</b>
                <span>{entry.timeAgo}</span>
              </div>
              <button className="btn g" type="button" onClick={() => handleGoTo(idx)}>
                <span>{idx === currentIdx ? "Current" : "Go here"}</span>
              </button>
            </div>
          ))}
        </div>
      </div>
    </ProjectLayout>
  );
};
