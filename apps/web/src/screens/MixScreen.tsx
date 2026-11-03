import React, { useState } from "react";
import { Button, CanvasViz, ProjectLayout, Section } from "@flux/ui";

interface ChannelConfig {
  name: string;
  colorVar: string;
  defaultVal: number;
  pan: number;
  isMaster?: boolean;
}

const CHANNELS: ChannelConfig[] = [
  { name: "Vocals", colorVar: "--t4", defaultVal: 74, pan: -6 },
  { name: "Drums", colorVar: "--t3", defaultVal: 66, pan: 0 },
  { name: "Bass", colorVar: "--t2", defaultVal: 62, pan: 0 },
  { name: "Keys", colorVar: "--t1", defaultVal: 48, pan: -38 },
  { name: "Guitars", colorVar: "--t5", defaultVal: 54, pan: 34 },
  { name: "Room", colorVar: "--t6", defaultVal: 30, pan: 0 },
  { name: "Music bus", colorVar: "--t8", defaultVal: 68, pan: 0 },
  { name: "Master", colorVar: "--tx", defaultVal: 70, pan: 0, isMaster: true },
];

export const MixScreen: React.FC = () => {
  const [faders, setFaders] = useState<Record<string, number>>(() =>
    Object.fromEntries(CHANNELS.map((c) => [c.name, c.defaultVal])),
  );
  const [mutes, setMutes] = useState<Record<string, boolean>>({});
  const [solos, setSolos] = useState<Record<string, boolean>>({});

  const toggleMute = (name: string) => {
    setMutes((prev) => ({ ...prev, [name]: !prev[name] }));
  };

  const toggleSolo = (name: string) => {
    setSolos((prev) => ({ ...prev, [name]: !prev[name] }));
  };

  return (
    <ProjectLayout currentTab="mix">
      <div className="split">
        <div className="a">
          <div className="mixer in">
            {CHANNELS.map((ch) => {
              const val = faders[ch.name] ?? ch.defaultVal;
              const isMst = ch.isMaster;
              const dBText = isMst
                ? "−1.0"
                : `${val > 70 ? "+" : "−"}${Math.abs((val - 70) / 6).toFixed(1)}`;

              return (
                <div key={ch.name} className={`ch ${isMst ? "mst" : ""}`}>
                  <span className="dot" style={{ color: `var(${ch.colorVar})` }} />
                  <b>{ch.name}</b>
                  <div
                    className="knob"
                    style={{ "--a": `${ch.pan}deg` } as React.CSSProperties}
                    role="img"
                    aria-label="Pan"
                  />
                  <a
                    className="chip"
                    href="#rack"
                    data-go="rack"
                    style={{
                      textDecoration: "none",
                      width: "100%",
                      justifyContent: "center",
                    }}
                  >
                    {isMst ? 3 : 2} fx
                  </a>
                  <div className="fader" style={{ "--v": `${val}%` } as React.CSSProperties}>
                    <div
                      className="ftk"
                      role="slider"
                      tabIndex={0}
                      aria-label={`${ch.name} level`}
                      aria-valuenow={val}
                      aria-valuemin={0}
                      aria-valuemax={100}
                      onKeyDown={(e) => {
                        if (e.key === "ArrowUp") {
                          setFaders((prev) => ({
                            ...prev,
                            [ch.name]: Math.min(100, val + 2),
                          }));
                        } else if (e.key === "ArrowDown") {
                          setFaders((prev) => ({
                            ...prev,
                            [ch.name]: Math.max(0, val - 2),
                          }));
                        }
                      }}
                    >
                      <span className="fth" />
                    </div>
                    <div className="vu">
                      <i data-lvv="" />
                    </div>
                    <div className="vu">
                      <i data-lvv="" />
                    </div>
                  </div>
                  <span className="mono dim">{dBText} dB</span>
                  <div
                    className="thd"
                    style={{
                      position: "static",
                      padding: 0,
                      width: "auto",
                      background: "none",
                      border: 0,
                    }}
                  >
                    <div className="ms">
                      {!isMst && (
                        <>
                          <button
                            className={`m ${mutes[ch.name] ? "on" : ""}`}
                            data-toggle=""
                            type="button"
                            aria-label="Mute"
                            onClick={() => toggleMute(ch.name)}
                          >
                            M
                          </button>
                          <button
                            className={`s ${solos[ch.name] ? "on" : ""}`}
                            data-toggle=""
                            type="button"
                            aria-label="Solo"
                            onClick={() => toggleSolo(ch.name)}
                          >
                            S
                          </button>
                        </>
                      )}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {/* Master Sidebar */}
        <aside className="b in">
          <h3 className="h3">Master</h3>
          <Section title="Loudness">
            <div className="g2">
              <div>
                <div className="stat">−14.2</div>
                <span className="lab">Integrated LUFS</span>
              </div>
              <div>
                <div className="stat">−1.1</div>
                <span className="lab">True peak dBTP</span>
              </div>
            </div>
          </Section>
          <Section title="Stereo">
            <div className="vz" style={{ height: "160px" }} role="img" aria-label="Goniometer">
              <CanvasViz type="gonio" />
            </div>
          </Section>
          <div className="between">
            <span className="lab">Correlation</span>
            <span className="mono">+0.62</span>
          </div>
          <Button label="Open all meters" dataGo="meters" href="#meters" />
        </aside>
      </div>
    </ProjectLayout>
  );
};
