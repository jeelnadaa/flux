import React, { useState, useEffect } from "react";
import { ProjectLayout, SegmentedControl, Toggle, Icon, CanvasViz } from "@flux/ui";
import { ProjectStore, TransportState } from "@flux/app-core";

interface MagicAssistantScreenProps {
  onNavigate?: (route: string) => void;
}

export const MagicAssistantScreen: React.FC<MagicAssistantScreenProps> = ({ onNavigate }) => {
  const store = ProjectStore.get();
  const [projectState, setProjectState] = useState(() => store.getState());

  useEffect(() => {
    return store.subscribe((s) => {
      setProjectState({ ...s });
    });
  }, [store]);

  const [goalIndex, setGoalIndex] = useState<number>(0);
  const [holdingBefore, setHoldingBefore] = useState<boolean>(false);

  const isPlaying = projectState.transportState === TransportState.Playing;

  const goalOptions = [
    "Make it clear",
    "Make it loud",
    "Dreamy",
    "8D",
    "Podcast",
    "Music",
  ] as const;

  const diagnosis = [
    { label: "Noise", value: "Moderate", badge: "warn" },
    { label: "Echo", value: "Slight", badge: "warn" },
    { label: "Clipping", value: "None", badge: "ok" },
    { label: "Hum", value: "50 Hz", badge: "warn" },
    { label: "Loudness", value: "-21.3 LUFS", badge: "warn" },
  ];

  const [chain, setChain] = useState([
    { title: "Hum removal", desc: "Found 50 Hz and 6 harmonics", enabled: true },
    { title: "Noise reduction", desc: "Room noise at -52 dB", enabled: true },
    { title: "De-reverb", desc: "Short room echo", enabled: true },
    {
      title: "EQ: Clear voice",
      desc: "Cut 250 Hz -2 dB, lift 3 kHz +1.5 dB",
      enabled: true,
    },
    { title: "De-esser", desc: "Sibilance at 6.8 kHz", enabled: true },
    { title: "Leveler", desc: "Evens out quiet phrases", enabled: true },
    { title: "Limiter", desc: "Target -16 LUFS, -1 dBTP", enabled: true },
  ]);

  const toggleChainItem = (index: number) => {
    setChain((prev) =>
      prev.map((item, idx) => (idx === index ? { ...item, enabled: !item.enabled } : item)),
    );
  };

  return (
    <ProjectLayout currentTab="aitools" onNavigate={onNavigate}>
      <div className="split">
        <div className="a">
          <div className="pg in" style={{ maxWidth: "none" }}>
            <div className="pgh">
              <h1 className="h1">Here's what we found.</h1>
              <div className="row">
                <SegmentedControl
                  items={goalOptions}
                  selectedIndex={goalIndex}
                  onChange={(_, idx) => setGoalIndex(idx)}
                />
              </div>
            </div>

            <div className="g5">
              {diagnosis.map((r, idx) => (
                <div key={idx} className="box">
                  <span className="lab">{r.label}</span>
                  <div
                    className={`${r.badge} b9`}
                    style={{ fontSize: 18, letterSpacing: "-.02em" }}
                  >
                    {r.value}
                  </div>
                </div>
              ))}
            </div>

            <div className="box between" style={{ flexWrap: "wrap" }}>
              <div className="row" style={{ flex: 1, minWidth: 240 }}>
                <button
                  type="button"
                  className={`tbtn play ${isPlaying ? "on" : ""}`}
                  aria-label="Play"
                  onClick={() => store.togglePlayback()}
                >
                  <Icon name={isPlaying ? "pause" : "play"} size={16} />
                </button>
                <div
                  style={{
                    flex: 1,
                    height: 64,
                    minWidth: 120,
                  }}
                >
                  <CanvasViz type="wave" seed={2} kind="voice" step={2} />
                </div>
              </div>
              <button
                type="button"
                className={`btn lg hold ${holdingBefore ? "on" : ""}`}
                onMouseDown={() => setHoldingBefore(true)}
                onMouseUp={() => setHoldingBefore(false)}
                onMouseLeave={() => setHoldingBefore(false)}
                onTouchStart={() => setHoldingBefore(true)}
                onTouchEnd={() => setHoldingBefore(false)}
              >
                Hold for before
              </button>
            </div>

            <span className="dim">
              Before and after are matched in loudness, so you judge the sound and not the volume.
            </span>
          </div>
        </div>

        <aside className="b in">
          <h3 className="h3">Proposed chain</h3>
          <span className="dim">
            Turn off anything you don't want. This is an ordinary chain you can edit.
          </span>

          <div className="lst">
            {chain.map((r, idx) => (
              <div key={idx} style={{ alignItems: "flex-start" }}>
                <div className="ti">
                  <b>{r.title}</b>
                  <span style={{ whiteSpace: "normal" }}>{r.desc}</span>
                </div>
                <Toggle label="" checked={r.enabled} onChange={() => toggleChainItem(idx)} />
              </div>
            ))}
          </div>

          <button type="button" className="btn p lg" onClick={() => onNavigate?.("rack")}>
            Apply chain
          </button>
          <button type="button" className="btn g" onClick={() => onNavigate?.("edit")}>
            Not now
          </button>
        </aside>
      </div>
    </ProjectLayout>
  );
};
