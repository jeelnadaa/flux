import React, { useState, useEffect } from "react";
import { TopBar, Rail, HOME_NAV_ITEMS, CanvasViz, Slider, Toggle, Chip, Icon } from "@flux/ui";
import { ProjectStore, TransportState } from "@flux/app-core";

interface TaskViewScreenProps {
  mode?: "8d" | "slowed";
  onNavigate?: (route: string) => void;
}

export const TaskViewScreen: React.FC<TaskViewScreenProps> = ({ mode = "8d", onNavigate }) => {
  const store = ProjectStore.get();
  const [projectState, setProjectState] = useState(() => store.getState());

  useEffect(() => {
    return store.subscribe((s) => {
      setProjectState({ ...s });
    });
  }, [store]);

  const is8D = mode === "8d";
  const [rotationSpeed, setRotationSpeed] = useState(24);
  const [distance, setDistance] = useState(58);
  const [room, setRoom] = useState(40);

  const [slowedAmount, setSlowedAmount] = useState(58);
  const [keepPitch, setKeepPitch] = useState(false);
  const [reverb, setReverb] = useState(66);

  const [isHolding, setIsHolding] = useState(false);

  const isPlaying = projectState.transportState === TransportState.Playing;
  const timecode = store.formatTimecode(projectState.playheadFrame);

  const navigate = (r: string) => {
    if (onNavigate) {
      onNavigate(r);
    } else {
      window.location.hash = r;
    }
  };

  const title = is8D ? "8D" : "Slowed + reverb";
  const subtitle = is8D ? (
    <>
      Sound circles your head. Use headphones.{" "}
      <a
        className="act"
        href="#hpcheck"
        data-go="hpcheck"
        onClick={(e) => {
          e.preventDefault();
          navigate("hpcheck");
        }}
      >
        Check them
      </a>
    </>
  ) : (
    "Slows the track, drops the pitch and fills the room."
  );

  const advRoute = is8D ? "stage" : "creative";

  return (
    <div className="app notp">
      <TopBar
        projectName="harbor-lights.flac"
        projectStatus="3:42 · stereo"
        rightActions={
          <>
            <div className="seg" data-mode aria-label="Mode">
              <button className="on" type="button">
                Simple
              </button>
              <button type="button" onClick={() => navigate("edit")}>
                Pro
              </button>
            </div>
            <a
              className="btn p"
              href="#export"
              data-go="export"
              onClick={(e) => {
                e.preventDefault();
                navigate("export");
              }}
            >
              <Icon name="exp" size={16} />
              Export
            </a>
          </>
        }
      />
      <Rail items={HOME_NAV_ITEMS} current="home" onNavigate={navigate} />
      <main className="main" id="m">
        <div className="pg in" style={{ maxWidth: "1000px" }}>
          <div>
            <h1 className="bigh">{title}</h1>
            <p className="lede">{subtitle}</p>
          </div>

          <div className="vz" style={{ height: "clamp(120px,24cqh,210px)" }}>
            <CanvasViz type="wave" seed={1} stereo={true} step={2.5} />
          </div>

          <div className="row">
            <button
              className={`tbtn play ${isPlaying ? "on" : ""}`}
              data-play
              aria-label={isPlaying ? "Pause" : "Play"}
              type="button"
              onClick={() => store.togglePlayback()}
            >
              <Icon name={isPlaying ? "pause" : "play"} size={18} />
            </button>
            <span className="tc mono" data-tc style={{ fontSize: "19px" }}>
              {timecode}
            </span>
            <span className="sp" style={{ flex: 1 }} />
            <button
              className={`btn lg hold ${isHolding ? "on" : ""}`}
              data-hold
              type="button"
              onMouseDown={() => setIsHolding(true)}
              onMouseUp={() => setIsHolding(false)}
              onTouchStart={() => setIsHolding(true)}
              onTouchEnd={() => setIsHolding(false)}
            >
              Hold for original
            </button>
          </div>

          {is8D ? (
            <>
              <Slider
                label="Rotation speed"
                value={rotationSpeed}
                valueText="One turn every 10 s"
                big={true}
                onChange={setRotationSpeed}
              />
              <div className="g2">
                <Slider
                  label="Distance"
                  value={distance}
                  valueText="1.8 m"
                  onChange={setDistance}
                />
                <Slider label="Room" value={room} valueText="Medium room" onChange={setRoom} />
              </div>
            </>
          ) : (
            <>
              <Slider
                label="Amount"
                value={slowedAmount}
                valueText="0.85× speed"
                big={true}
                onChange={setSlowedAmount}
              />
              <div className="g2">
                <Toggle
                  label="Keep the original pitch"
                  checked={keepPitch}
                  onChange={setKeepPitch}
                />
                <Slider label="Reverb" value={reverb} valueText="Hall 3.2 s" onChange={setReverb} />
              </div>
            </>
          )}

          <div className="row">
            <Chip label="Make it 8D" active={is8D} dataGo="task" onClick={() => navigate("task")} />
            <Chip
              label="Slowed + reverb"
              active={!is8D}
              dataGo="task-slowed"
              onClick={() => navigate("task-slowed")}
            />
            <Chip
              label="Remove vocals"
              active={false}
              dataGo="stems-setup"
              onClick={() => navigate("stems-setup")}
            />
            <Chip
              label="Clean up voice"
              active={false}
              dataGo="cleanup"
              onClick={() => navigate("cleanup")}
            />
            <Chip
              label="Remove noise"
              active={false}
              dataGo="fx-clean"
              onClick={() => navigate("fx-clean")}
            />
            <Chip
              label="Convert"
              active={false}
              dataGo="convert"
              onClick={() => navigate("convert")}
            />
          </div>

          <a
            className="dim"
            href={`#${advRoute}`}
            data-go={advRoute}
            onClick={(e) => {
              e.preventDefault();
              navigate(advRoute);
            }}
          >
            Open all the controls in Pro mode
          </a>
        </div>
      </main>
    </div>
  );
};
