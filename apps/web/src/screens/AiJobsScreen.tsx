import React, { useState } from "react";
import { ProjectLayout } from "@flux/ui";

interface AiJobsScreenProps {
  onNavigate?: (route: string) => void;
}

export const AiJobsScreen: React.FC<AiJobsScreenProps> = ({ onNavigate }) => {
  const [jobs, setJobs] = useState([
    {
      id: "j1",
      title: "Stems: Harbor Lights",
      details: "On device · 3 min left",
      progress: 62,
      active: true,
    },
    {
      id: "j2",
      title: "Transcript: Interview ep. 12",
      details: "On device · Waiting",
      progress: 0,
      active: false,
    },
    {
      id: "j3",
      title: "Dub to Spanish",
      details: "Cloud · 1 min left · Deletes in 24 h",
      progress: 84,
      active: true,
    },
    {
      id: "j4",
      title: "Export: Harbor Lights master.flac",
      details: "Done 4 min ago",
      progress: 100,
      active: false,
    },
  ]);

  const handleCancel = (id: string) => {
    setJobs((prev) => prev.filter((j) => j.id !== id));
  };

  return (
    <ProjectLayout currentTab="aitools" onNavigate={onNavigate}>
      <div className="pg in">
        <h1 className="h1">Jobs</h1>
        <div className="lst box">
          {jobs.map((r) => (
            <div key={r.id} style={{ flexWrap: "wrap" }}>
              <div className="ti" style={{ minWidth: 180 }}>
                <b>{r.title}</b>
                <span>{r.details}</span>
              </div>
              <div style={{ flex: 1, minWidth: 100 }}>
                <div className={`prog ${r.active ? "act" : ""}`}>
                  <span style={{ width: `${r.progress}%` }} />
                </div>
              </div>
              {r.progress === 100 ? (
                <button type="button" className="btn g" onClick={() => onNavigate?.("exported")}>
                  Show
                </button>
              ) : (
                <button type="button" className="btn g" onClick={() => handleCancel(r.id)}>
                  Cancel
                </button>
              )}
            </div>
          ))}
        </div>
      </div>
    </ProjectLayout>
  );
};
