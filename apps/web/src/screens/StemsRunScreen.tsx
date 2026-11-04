import React, { useState, useEffect } from "react";
import { StemsScreen } from "./StemsScreen.js";
import { Dialog, Callout } from "@flux/ui";
import { ProjectStore } from "@flux/app-core";

interface StemsRunScreenProps {
  onNavigate?: (route: string) => void;
}

export const StemsRunScreen: React.FC<StemsRunScreenProps> = ({ onNavigate }) => {
  const [overallProgress, setOverallProgress] = useState(25);

  useEffect(() => {
    const timer = setInterval(() => {
      setOverallProgress((prev) => {
        if (prev >= 100) {
          clearInterval(timer);
          setTimeout(() => {
            onNavigate?.("stems");
          }, 400);
          return 100;
        }
        return prev + 15;
      });
    }, 350);

    return () => clearInterval(timer);
  }, [onNavigate]);

  const stems = [
    { name: "Vocals", progress: Math.min(100, Math.round(overallProgress * 1.2)), colorToken: "--t4" },
    { name: "Drums", progress: Math.min(100, Math.round(overallProgress * 1.1)), colorToken: "--t3" },
    { name: "Bass", progress: Math.min(100, Math.round(overallProgress * 0.95)), colorToken: "--t2" },
    { name: "Other", progress: Math.min(100, Math.round(overallProgress * 0.8)), colorToken: "--t5" },
  ];

  return (
    <>
      <StemsScreen onNavigate={onNavigate} />

      <Dialog
        title="Splitting Harbor Lights"
        baseScreenId="stems"
        onClose={() => onNavigate?.("stems-setup")}
        footer={
          <>
            <a
              className="btn g dng"
              href="#stems-setup"
              data-go="stems-setup"
              onClick={(e) => {
                e.preventDefault();
                onNavigate?.("stems-setup");
              }}
            >
              <span>Cancel</span>
            </a>
            <a
              className="btn"
              href="#jobs"
              data-go="jobs"
              onClick={(e) => {
                e.preventDefault();
                onNavigate?.("jobs");
              }}
            >
              <span>Run in background</span>
            </a>
          </>
        }
      >
        <div className="col" style={{ gap: 14 }}>
          <div className="col" style={{ gap: 8 }}>
            {stems.map((s) => (
              <div key={s.name} className="between" style={{ alignItems: "center" }}>
                <b style={{ width: 70 }}>{s.name}</b>
                <div style={{ flex: 1, margin: "0 10px" }}>
                  <div className={`prog ${s.progress > 0 && s.progress < 100 ? "act" : ""}`}>
                    <span
                      style={{
                        width: `${s.progress}%`,
                        background: `var(${s.colorToken})`,
                      }}
                    />
                  </div>
                </div>
                <span className="mono dim" style={{ width: 38, textAlign: "right" }}>
                  {s.progress}%
                </span>
              </div>
            ))}
          </div>

          <div className="between mono dim">
            <span>{overallProgress >= 100 ? "Complete!" : "Separating components..."}</span>
            <span>WebGPU · 0.8× real time</span>
          </div>

          <Callout variant="info">You can keep working. We'll let you know when it's done.</Callout>
        </div>
      </Dialog>
    </>
  );
};
