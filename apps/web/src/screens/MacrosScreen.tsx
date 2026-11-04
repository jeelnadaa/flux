import React, { useState } from "react";
import { PageLayout, Button } from "@flux/ui";

const ACTIONS = [
  { name: "Podcast cleanup", steps: "6 steps", shortcut: "Ctrl Shift 1" },
  { name: "Trim and normalize", steps: "3 steps", shortcut: "Ctrl Shift 2" },
  { name: "Instagram clip", steps: "4 steps", shortcut: "Ctrl Shift 3" },
];

const MACRO_STEPS: Record<string, string[]> = {
  "Podcast cleanup": [
    "Learn noise from first 2 s",
    "Noise reduction −18 dB",
    "Remove filler words",
    "Trim pauses to 300 ms",
    "Clear voice EQ",
    "Limit to −16 LUFS",
  ],
  "Trim and normalize": [
    "Trim leading and trailing silence",
    "RMS Leveler −18 dBFS",
    "True peak limiter −1.0 dBTP",
  ],
  "Instagram clip": [
    "Select loudest 30 seconds",
    "Convert to Mono speech",
    "Air EQ presence boost",
    "Fast export MP3",
  ],
};

export const MacrosScreen: React.FC = () => {
  const [activeAction, setActiveAction] = useState("Podcast cleanup");
  const [runningAction, setRunningAction] = useState<string | null>(null);
  const [isRecording, setIsRecording] = useState(false);

  const handleRun = (name: string) => {
    setRunningAction(name);
    setTimeout(() => {
      setRunningAction(null);
    }, 1200);
  };

  const steps = MACRO_STEPS[activeAction] || MACRO_STEPS["Podcast cleanup"];
  const currentActionMeta = ACTIONS.find((a) => a.name === activeAction) || ACTIONS[0];

  return (
    <PageLayout
      currentTab="macros"
      title="Actions"
      subtitle="Record what you do once, and run it again with one click."
      actions={
        <Button
          label={isRecording ? "Recording action..." : "Record action"}
          variant={isRecording ? "magic" : "primary"}
          icon="rec"
          onClick={() => setIsRecording(!isRecording)}
        />
      }
    >
      <div className="g2">
        <div className="lst box">
          {ACTIONS.map((a) => (
            <div
              key={a.name}
              style={{
                cursor: "pointer",
                background: a.name === activeAction ? "var(--bg-card-hover, rgba(255,255,255,0.04))" : undefined,
              }}
              onClick={() => setActiveAction(a.name)}
            >
              <div className="ti">
                <b>{a.name}</b>
                <span>{a.steps}</span>
              </div>
              <Button
                label={runningAction === a.name ? "Running..." : "Run"}
                variant={runningAction === a.name ? "primary" : "ghost"}
                icon="play"
                onClick={(e) => {
                  e.stopPropagation();
                  handleRun(a.name);
                }}
              />
            </div>
          ))}
        </div>

        <div className="box col">
          <div className="between">
            <h2 className="h2">{activeAction}</h2>
            <span className="kbd">{currentActionMeta.shortcut}</span>
          </div>
          <div className="lst">
            {steps.map((s, i) => (
              <div key={s}>
                <span className="mono faint">{i + 1}</span>
                <div className="ti">
                  <b>{s}</b>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </PageLayout>
  );
};
