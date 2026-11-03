import React, { useState } from "react";
import { AiToolsScreen } from "./AiToolsScreen.js";
import { Dialog, SegmentedControl, Toggle, Callout } from "@flux/ui";

interface StemsSetupScreenProps {
  onNavigate?: (route: string) => void;
}

export const StemsSetupScreen: React.FC<StemsSetupScreenProps> = ({ onNavigate }) => {
  const [separateIndex, setSeparateIndex] = useState<number>(1);
  const [selectedQuality, setSelectedQuality] = useState<number>(1);
  const [preview30s, setPreview30s] = useState<boolean>(true);

  const qualityTiers = [
    { title: "Fast", time: "3 min", desc: "340 MB model" },
    { title: "Balanced", time: "6 min", desc: "820 MB model" },
    { title: "Best", time: "22 min", desc: "Slower, fewer artifacts" },
  ];

  return (
    <>
      <AiToolsScreen onNavigate={onNavigate} />

      <Dialog
        title="Split into stems"
        baseScreenId="aitools"
        onClose={() => onNavigate?.("aitools")}
        footer={
          <>
            <a
              className="btn g"
              href="#aitools"
              data-go="aitools"
              onClick={(e) => {
                e.preventDefault();
                onNavigate?.("aitools");
              }}
            >
              <span>Cancel</span>
            </a>
            <a
              className="btn p"
              href="#stems-run"
              data-go="stems-run"
              onClick={(e) => {
                e.preventDefault();
                onNavigate?.("stems-run");
              }}
            >
              <span>Split</span>
            </a>
          </>
        }
      >
        <div className="col" style={{ gap: 14 }}>
          <div className="fld">
            <label className="lab">Separate into</label>
            <SegmentedControl
              items={["Vocals and music", "4 stems", "6 stems"]}
              selectedIndex={separateIndex}
              onChange={(_item, idx) => setSeparateIndex(idx)}
            />
          </div>

          <div className="g3">
            {qualityTiers.map((q, i) => (
              <button
                key={q.title}
                className={`tile ${selectedQuality === i ? "on" : ""}`}
                type="button"
                onClick={() => setSelectedQuality(i)}
              >
                <b>{q.title}</b>
                <span className="mono">{q.time}</span>
                <span>{q.desc}</span>
              </button>
            ))}
          </div>

          <Callout variant="ac" icon="lock">
            <b>Runs on this device.</b> Your audio is not uploaded. The Balanced model downloads
            once (820 MB) and then works offline.
          </Callout>

          <Toggle
            label="Preview the first 30 seconds first"
            checked={preview30s}
            onChange={setPreview30s}
          />

          <span className="dim" style={{ fontSize: "12.5px" }}>
            Only separate audio you have the rights to use.
          </span>
        </div>
      </Dialog>
    </>
  );
};
