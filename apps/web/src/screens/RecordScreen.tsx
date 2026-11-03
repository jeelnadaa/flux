import React, { useState } from "react";
import { ProjectLayout, CanvasViz, Field, Select, Toggle, Callout, Button } from "@flux/ui";

interface RecordScreenProps {
  onNavigate?: (route: string) => void;
}

export const RecordScreen: React.FC<RecordScreenProps> = ({ onNavigate }) => {
  const [microphone, setMicrophone] = useState("Studio Mic USB");
  const [recordTo, setRecordTo] = useState("New track");
  const [monitor, setMonitor] = useState(false);
  const [rawSignal, setRawSignal] = useState(true);

  const navigate = (r: string) => {
    if (onNavigate) {
      onNavigate(r);
    } else {
      window.location.hash = r;
    }
  };

  return (
    <ProjectLayout currentTab="edit">
      <div className="pg in" style={{ maxWidth: "900px", margin: "auto" }}>
        <div className="between">
          <h1 className="h1">Recording.</h1>
          <span className="pill bad">
            <span className="dot" />
            REC 00:01:24
          </span>
        </div>

        <div className="vz" style={{ height: "170px" }}>
          <CanvasViz type="wave" seed={2} kind="voice" color="--sig" until={0.7} step={2} />
        </div>

        <div className="col">
          <div className="between lab">
            <span>Input level</span>
            <span className="mono">−12.4 dB peak</span>
          </div>
          <div className="meter" style={{ width: "100%" }}>
            <div className="m" style={{ height: "12px" }}>
              <i data-lv />
            </div>
            <div className="m" style={{ height: "12px" }}>
              <i data-lv />
            </div>
          </div>
        </div>

        <div className="g2">
          <Field label="Microphone">
            <Select
              options={["Studio Mic USB", "Laptop mic"]}
              value={microphone}
              onChange={setMicrophone}
            />
          </Field>
          <Field label="Record to">
            <Select
              options={["New track", "Selected track"]}
              value={recordTo}
              onChange={setRecordTo}
            />
          </Field>
        </div>

        <div className="box g2">
          <Toggle
            label="Hear yourself while recording"
            sub="About 90 ms delay in the browser"
            checked={monitor}
            onChange={setMonitor}
          />
          <Toggle
            label="Raw signal"
            sub="Browser noise reduction and auto gain off"
            checked={rawSignal}
            onChange={setRawSignal}
          />
        </div>

        <Callout icon="check">
          Recording is written to disk as it happens. A crash loses no more than the last second.
        </Callout>

        <div className="row" style={{ justifyContent: "center" }}>
          <Button
            label="Stop"
            variant="primary"
            large={true}
            icon="stop"
            dataGo="edit"
            href="#edit"
            onClick={() => navigate("edit")}
          />
          <Button label="Pause" large={true} icon="pause" />
          <Button label="Discard take" variant="danger" large={true} />
        </div>
      </div>
    </ProjectLayout>
  );
};
