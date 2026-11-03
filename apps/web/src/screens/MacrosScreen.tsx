import React, { useState } from "react";
import { PageLayout, Button } from "@flux/ui";

const ACTIONS = [
  { name: "Podcast cleanup", steps: "6 steps" },
  { name: "Trim and normalize", steps: "3 steps" },
  { name: "Instagram clip", steps: "4 steps" },
];

const PODCAST_STEPS = [
  "Learn noise from first 2 s",
  "Noise reduction −18 dB",
  "Remove filler words",
  "Trim pauses to 300 ms",
  "Clear voice EQ",
  "Limit to −16 LUFS",
];

export const MacrosScreen: React.FC = () => {
  const [activeAction] = useState("Podcast cleanup");

  return (
    <PageLayout
      currentTab="macros"
      title="Actions"
      subtitle="Record what you do once, and run it again with one click."
      actions={<Button label="Record action" variant="primary" icon="rec" />}
    >
      <div className="g2">
        <div className="lst box">
          {ACTIONS.map((a) => (
            <div key={a.name}>
              <div className="ti">
                <b>{a.name}</b>
                <span>{a.steps}</span>
              </div>
              <Button label="Run" variant="ghost" icon="play" />
            </div>
          ))}
        </div>

        <div className="box col">
          <div className="between">
            <h2 className="h2">{activeAction}</h2>
            <span className="kbd">Ctrl Shift 1</span>
          </div>
          <div className="lst">
            {PODCAST_STEPS.map((s, i) => (
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
