import React, { useState } from "react";
import { PageLayout, Button, Field, Select, ProgressBar } from "@flux/ui";

const INITIAL_FILES = [
  { name: "Episode 01.wav", status: "Done", statusClass: "ok" },
  { name: "Episode 02.wav", status: "Done", statusClass: "ok" },
  { name: "Episode 03.wav", status: "48%", statusClass: "act" },
  { name: "Episode 04.wav", status: "Waiting", statusClass: "faint" },
  { name: "corrupt-take.mp3", status: "Skipped: file is damaged", statusClass: "bad" },
];

const INITIAL_STEPS = [
  "Noise reduction",
  "Trim silence over 0.6 s",
  "Normalize to −16 LUFS",
  "Export MP3 V0",
];

interface BatchScreenProps {
  onNavigate?: (route: string) => void;
}

export const BatchScreen: React.FC<BatchScreenProps> = ({ onNavigate }) => {
  const [steps, setSteps] = useState<string[]>(INITIAL_STEPS);
  const [saveTo, setSaveTo] = useState('Same folder, new "Processed" subfolder');
  const [files, setFiles] = useState(INITIAL_FILES);
  const [isRunning, setIsRunning] = useState(false);

  const removeStep = (index: number) => {
    setSteps(steps.filter((_, i) => i !== index));
  };

  const handleStartBatch = async () => {
    setIsRunning(true);
    for (let i = 0; i < files.length; i++) {
      if (files[i].statusClass === "bad") continue; // keep skipped
      setFiles((prev) =>
        prev.map((f, idx) =>
          idx === i ? { ...f, status: "Processing...", statusClass: "act" } : f,
        ),
      );
      await new Promise((r) => setTimeout(r, 400));
      setFiles((prev) =>
        prev.map((f, idx) => (idx === i ? { ...f, status: "Done", statusClass: "ok" } : f)),
      );
    }
    setIsRunning(false);
  };

  const navigate = (r: string) => {
    if (onNavigate) {
      onNavigate(r);
    } else {
      window.location.hash = r;
    }
  };

  return (
    <PageLayout
      currentTab="batch"
      title="Do it to all of them."
      actions={
        <Button
          label={isRunning ? "Processing..." : "Start"}
          variant="primary"
          icon="play"
          onClick={handleStartBatch}
        />
      }
      onNavigate={navigate}
    >
      <div className="g2">
        <div className="box col">
          <h2 className="h2">Files</h2>
          <div className="lst">
            {files.map((f) => (
              <div key={f.name}>
                <div className="ti">
                  <b>{f.name}</b>
                </div>
                <span className={f.statusClass} style={{ fontSize: "12.5px", fontWeight: 700 }}>
                  {f.status}
                </span>
              </div>
            ))}
          </div>
          <Button label="Add files" icon="plus" />
        </div>

        <div className="box col">
          <h2 className="h2">Apply</h2>
          <div className="lst">
            {steps.map((s, i) => (
              <div key={`${s}-${i}`}>
                <span className="mono faint">{i + 1}</span>
                <div className="ti">
                  <b>{s}</b>
                </div>
                <Button
                  label=""
                  icon="x"
                  variant="ghost"
                  ariaLabel="Remove step"
                  onClick={() => removeStep(i)}
                />
              </div>
            ))}
          </div>
          <div className="row">
            <Button label="Add a step" icon="plus" />
            <Button
              label="Load preset"
              icon="tag"
              dataGo="presets"
              href="#presets"
              onClick={() => navigate("presets")}
            />
          </div>
          <Field label="Save to">
            <Select
              options={['Same folder, new "Processed" subfolder', "Choose a folder"]}
              value={saveTo}
              onChange={setSaveTo}
            />
          </Field>
        </div>
      </div>

      <div className="box col">
        <div className="between">
          <b>3 of 5 files</b>
          <span className="mono dim">12 min left · 2 at a time</span>
        </div>
        <ProgressBar percent={52} running={true} />
      </div>
    </PageLayout>
  );
};
