import React, { useState } from "react";
import {
  EffectPanelShell,
  CanvasViz,
  Slider,
  Toggle,
  Pill,
  Button,
  SegmentedControl,
} from "@flux/ui";
import { ProjectStore } from "@flux/app-core";

interface PauseData {
  time: string;
  duration: string;
  action: string;
  enabled: boolean;
  startSec: number;
}

const INITIAL_PAUSES: PauseData[] = [
  { time: "0:42.1", duration: "1.9 s", action: "Shorten to 300 ms", enabled: true, startSec: 42.1 },
  { time: "1:18.4", duration: "0.8 s", action: "Shorten to 300 ms", enabled: true, startSec: 78.4 },
  { time: "2:06.7", duration: "3.4 s", action: "Shorten to 300 ms", enabled: true, startSec: 126.7 },
  { time: "2:44.0", duration: "1.1 s", action: "Keep", enabled: false, startSec: 164.0 },
];

export const SilenceRemoverScreen: React.FC = () => {
  const [isBypassed, setIsBypassed] = useState(false);
  const [pauses, setPauses] = useState<PauseData[]>(INITIAL_PAUSES);
  const [longerThan, setLongerThan] = useState(24); // 0.6 s
  const [shortenTo, setShortenTo] = useState(20); // 300 ms
  const [padding, setPadding] = useState(22); // 150 ms
  const [threshold, setThreshold] = useState(46); // −42 dB
  const [detectModeIdx, setDetectModeIdx] = useState(0); // 0: Speech only, 1: Any audio
  const [protectBreaths, setProtectBreaths] = useState(true);
  const [crossfadeJoins, setCrossfadeJoins] = useState(true);
  const [applied, setApplied] = useState(false);

  const store = ProjectStore.getInstance();

  const handleTogglePause = (index: number) => {
    setPauses((prev) => prev.map((p, i) => (i === index ? { ...p, enabled: !p.enabled } : p)));
  };

  const handlePreviewPause = (startSec: number) => {
    const sr = store.getState().sampleRate;
    const startFrame = Math.max(0, Math.floor((startSec - 0.5) * sr));
    store.seek(startFrame);
    store.play();
    setTimeout(() => store.stop(), 2000);
  };

  const handleApply = () => {
    setApplied(true);
  };

  return (
    <EffectPanelShell
      title="Silence remover"
      closeGo="fxbrowser"
      isBypassed={isBypassed}
      onToggleBypass={() => setIsBypassed(!isBypassed)}
      main={
        <>
          <div className="box row" style={{ justifyContent: "space-between" }}>
            <div>
              <b className="stat">−4:12</b> <span className="dim">from 48:10</span>
            </div>
            <span>
              <Pill label={applied ? "Applied 38 pauses" : "38 pauses found"} variant="ok" />
            </span>
          </div>

          <div className="vz" style={{ height: "180px", position: "relative" }}>
            <CanvasViz type="wave" seed={2} kind="voice" color="--t1" step={2} />
            {[12, 22, 37, 49, 63, 78].map((l) => (
              <span
                key={l}
                style={{
                  position: "absolute",
                  left: `${l}%`,
                  top: 0,
                  bottom: 0,
                  width: "3.5%",
                  background: "color-mix(in srgb,var(--sig) 26%,transparent)",
                  borderInline: "1.5px solid var(--sig)",
                }}
              />
            ))}
          </div>

          <div className="lst box" style={{ maxHeight: "220px", overflow: "auto" }}>
            {pauses.map((r, i) => (
              <div key={r.time}>
                <span className="mono">{r.time}</span>
                <div className="ti">
                  <b>{r.duration} pause</b>
                  <span>{r.action}</span>
                </div>
                <Button
                  variant="ghost"
                  icon="play"
                  iconOnly
                  ariaLabel="Preview"
                  onClick={() => handlePreviewPause(r.startSec)}
                />
                <Toggle label="" checked={r.enabled} onChange={() => handleTogglePause(i)} />
              </div>
            ))}
          </div>
        </>
      }
      aside={
        <>
          <Slider
            label="Longer than"
            value={longerThan}
            valueText={`${(0.2 + longerThan * 0.0167).toFixed(1)} s`}
            onChange={setLongerThan}
          />
          <Slider
            label="Shorten to"
            value={shortenTo}
            valueText={`${Math.round(100 + shortenTo * 10)} ms`}
            onChange={setShortenTo}
          />
          <Slider
            label="Padding"
            value={padding}
            valueText={`${Math.round(50 + padding * 4.5)} ms`}
            onChange={setPadding}
          />
          <Slider
            label="Threshold"
            value={threshold}
            valueText={`−${Math.round(20 + threshold * 0.48)} dB`}
            onChange={setThreshold}
          />
          <label className="fld">
            <span>Detect</span>
            <SegmentedControl
              items={["Speech only", "Any audio"]}
              selectedIndex={detectModeIdx}
              onChange={(_, idx) => setDetectModeIdx(idx)}
            />
          </label>
          <Toggle label="Protect breaths" checked={protectBreaths} onChange={setProtectBreaths} />
          <Toggle label="Crossfade joins" checked={crossfadeJoins} onChange={setCrossfadeJoins} />
          <Button
            label={`Apply to all ${applied ? "applied" : "38"}`}
            variant="primary"
            large
            onClick={handleApply}
          />
        </>
      }
    />
  );
};
