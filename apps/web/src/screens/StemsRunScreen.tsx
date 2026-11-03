import React from "react";
import { StemsScreen } from "./StemsScreen.js";
import { Dialog, Callout } from "@flux/ui";

interface StemsRunScreenProps {
  onNavigate?: (route: string) => void;
}

export const StemsRunScreen: React.FC<StemsRunScreenProps> = ({ onNavigate }) => {
  const stems = [
    { name: "Vocals", progress: 100, colorToken: "--t4" },
    { name: "Drums", progress: 100, colorToken: "--t3" },
    { name: "Bass", progress: 62, colorToken: "--t2" },
    { name: "Other", progress: 0, colorToken: "--t5" },
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
            <span>About 3 min left</span>
            <span>WebGPU · 0.8× real time</span>
          </div>

          <Callout variant="info">You can keep working. We'll let you know when it's done.</Callout>
        </div>
      </Dialog>
    </>
  );
};
