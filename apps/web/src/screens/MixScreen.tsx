import React, { useState, useEffect } from "react";
import { Button, CanvasViz, ProjectLayout, Section } from "@flux/ui";
import {
  ProjectStore,
  TransportState,
  type FluxProjectState,
  type AudioTrack,
} from "@flux/app-core";

export const MixScreen: React.FC = () => {
  const store = ProjectStore.getInstance();
  const [project, setProject] = useState<FluxProjectState>(() => store.getState());
  const [masterVolume, setMasterVolume] = useState<number>(70);

  useEffect(() => {
    return store.subscribe((newState: FluxProjectState) => {
      setProject({ ...newState });
    });
  }, [store]);

  const toggleMute = (trackId: string) => {
    store.toggleTrackMute(trackId);
  };

  const toggleSolo = (trackId: string) => {
    store.toggleTrackSolo(trackId);
  };

  const handleTrackVolume = (trackId: string, sliderPct: number) => {
    // Slider 0-100 maps to 0.0 - 1.5 volume
    const vol = (sliderPct / 70) * 1.0;
    store.setTrackVolume(trackId, vol);
  };

  const handleTrackPan = (trackId: string, currentPan: number) => {
    // Cycles pan between -0.4, 0.0, +0.4
    const nextPan = currentPan === 0 ? 0.35 : currentPan > 0 ? -0.35 : 0;
    store.setTrackPan(trackId, nextPan);
  };

  const isPlaying = project.transportState === TransportState.Playing;
  const timecode = store.formatTimecode(project.playheadFrame);

  return (
    <ProjectLayout
      currentTab="mix"
      isPlaying={isPlaying}
      timecode={timecode}
      onPlayPause={() => store.togglePlayback()}
      onStop={() => store.stop()}
      onBackToStart={() => store.seek(0)}
      onLoopToggle={() =>
        store.setLoop(!project.loopEnabled, project.loopStartFrame, project.loopEndFrame)
      }
      leftMeter={project.meterLevels.leftPeak}
      rightMeter={project.meterLevels.rightPeak}
    >
      <div className="split">
        <div className="a">
          <div className="mixer in">
            {project.tracks.map((track: AudioTrack) => {
              const sliderVal = Math.round((track.volume / 1.0) * 70);
              const val = Math.max(0, Math.min(100, sliderVal));
              const dB =
                track.volume <= 0.001
                  ? "−inf"
                  : (20 * Math.log10(Math.max(0.01, track.volume))).toFixed(1);
              const dBText = track.volume >= 1.0 ? `+${dB}` : dB;
              const panDeg = Math.round(track.pan * 45);

              return (
                <div key={track.id} className="ch">
                  <span className="dot" style={{ color: `var(${track.colorVar})` }} />
                  <b>{track.name}</b>
                  <div
                    className="knob"
                    style={{ "--a": `${panDeg}deg`, cursor: "pointer" } as React.CSSProperties}
                    role="img"
                    aria-label={`Pan: ${track.pan > 0 ? "Right" : track.pan < 0 ? "Left" : "Center"}`}
                    onClick={() => handleTrackPan(track.id, track.pan)}
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
                    2 fx
                  </a>
                  <div className="fader" style={{ "--v": `${val}%` } as React.CSSProperties}>
                    <div
                      className="ftk"
                      role="slider"
                      tabIndex={0}
                      aria-label={`${track.name} level`}
                      aria-valuenow={val}
                      aria-valuemin={0}
                      aria-valuemax={100}
                      onKeyDown={(e) => {
                        if (e.key === "ArrowUp") {
                          handleTrackVolume(track.id, Math.min(100, val + 2));
                        } else if (e.key === "ArrowDown") {
                          handleTrackVolume(track.id, Math.max(0, val - 2));
                        }
                      }}
                    >
                      <span className="fth" />
                    </div>
                    <div className="vu">
                      <i
                        data-lvv=""
                        style={{
                          height:
                            isPlaying && !track.muted
                              ? `${Math.max(10, Math.min(95, 60 + project.meterLevels.leftPeak * 2))}%`
                              : "0%",
                        }}
                      />
                    </div>
                    <div className="vu">
                      <i
                        data-lvv=""
                        style={{
                          height:
                            isPlaying && !track.muted
                              ? `${Math.max(10, Math.min(95, 60 + project.meterLevels.rightPeak * 2))}%`
                              : "0%",
                        }}
                      />
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
                      <button
                        className={`m ${track.muted ? "on" : ""}`}
                        data-toggle=""
                        type="button"
                        aria-label="Mute"
                        onClick={() => toggleMute(track.id)}
                      >
                        M
                      </button>
                      <button
                        className={`s ${track.solo ? "on" : ""}`}
                        data-toggle=""
                        type="button"
                        aria-label="Solo"
                        onClick={() => toggleSolo(track.id)}
                      >
                        S
                      </button>
                    </div>
                  </div>
                </div>
              );
            })}

            {/* Master Channel Strip */}
            <div className="ch mst">
              <span className="dot" style={{ color: "var(--tx)" }} />
              <b>Master</b>
              <div
                className="knob"
                style={{ "--a": "0deg" } as React.CSSProperties}
                role="img"
                aria-label="Master Pan"
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
                3 fx
              </a>
              <div className="fader" style={{ "--v": `${masterVolume}%` } as React.CSSProperties}>
                <div
                  className="ftk"
                  role="slider"
                  tabIndex={0}
                  aria-label="Master level"
                  aria-valuenow={masterVolume}
                  aria-valuemin={0}
                  aria-valuemax={100}
                  onKeyDown={(e) => {
                    if (e.key === "ArrowUp") {
                      setMasterVolume((prev) => Math.min(100, prev + 2));
                    } else if (e.key === "ArrowDown") {
                      setMasterVolume((prev) => Math.max(0, prev - 2));
                    }
                  }}
                >
                  <span className="fth" />
                </div>
                <div className="vu">
                  <i
                    data-lvv=""
                    style={{
                      height: isPlaying
                        ? `${Math.max(10, Math.min(95, 60 + project.meterLevels.leftPeak * 2))}%`
                        : "0%",
                    }}
                  />
                </div>
                <div className="vu">
                  <i
                    data-lvv=""
                    style={{
                      height: isPlaying
                        ? `${Math.max(10, Math.min(95, 60 + project.meterLevels.rightPeak * 2))}%`
                        : "0%",
                    }}
                  />
                </div>
              </div>
              <span className="mono dim">−1.0 dB</span>
              <div
                className="thd"
                style={{
                  position: "static",
                  padding: 0,
                  width: "auto",
                  background: "none",
                  border: 0,
                }}
              />
            </div>
          </div>
        </div>

        {/* Master Sidebar */}
        <aside className="b in">
          <h3 className="h3">Master</h3>
          <Section title="Loudness">
            <div className="g2">
              <div>
                <div className="stat">
                  {isPlaying ? (project.meterLevels.leftRms - 6).toFixed(1) : "−14.2"}
                </div>
                <span className="lab">Integrated LUFS</span>
              </div>
              <div>
                <div className="stat">
                  {isPlaying ? project.meterLevels.leftPeak.toFixed(1) : "−1.1"}
                </div>
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
