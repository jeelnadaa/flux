import React, { useState } from "react";
import {
  ProjectLayout,
  SegmentedControl,
  Slider,
  Toggle,
  Pill,
  Callout,
  Icon,
  CanvasViz,
} from "@flux/ui";

interface VoiceCleanupScreenProps {
  onNavigate?: (route: string) => void;
}

export const VoiceCleanupScreen: React.FC<VoiceCleanupScreenProps> = ({ onNavigate }) => {
  const [isSimple, setIsSimple] = useState<boolean>(true);
  const [compareSlotIdx, setCompareSlotIdx] = useState<number>(0);
  const [isBypassed, setIsBypassed] = useState<boolean>(false);

  // Main controls
  const [studioSound, setStudioSound] = useState<number>(60);
  const [isHoldingOriginal, setIsHoldingOriginal] = useState<boolean>(false);

  // Steps in side panel
  const [removeNoise, setRemoveNoise] = useState<boolean>(true);
  const [noiseAmount, setNoiseAmount] = useState<number>(72);
  const [removeEcho, setRemoveEcho] = useState<boolean>(true);
  const [echoAmount, setEchoAmount] = useState<number>(50);
  const [rebuildHighs, setRebuildHighs] = useState<boolean>(false);
  const [matchStudioMic, setMatchStudioMic] = useState<boolean>(true);
  const [deEss, setDeEss] = useState<boolean>(true);
  const [evenOutLevel, setEvenOutLevel] = useState<boolean>(true);
  const [keepRoomTone, setKeepRoomTone] = useState<number>(30);

  return (
    <ProjectLayout currentTab="aitools">
      <div className="split">
        <div className="a">
          <div className="tlbar">
            <b style={{ fontSize: 15, fontWeight: 800, whiteSpace: "nowrap" }}>Voice cleanup</b>
            <SegmentedControl
              items={["Simple", "Advanced"]}
              selectedIndex={isSimple ? 0 : 1}
              onChange={(_val, idx) => setIsSimple(idx === 0)}
            />
            <span className="sp" style={{ flex: 1 }} />
            <SegmentedControl
              items={["A", "B", "C"]}
              selectedIndex={compareSlotIdx}
              onChange={(_val, idx) => setCompareSlotIdx(idx)}
            />
            <a
              className="btn"
              href="#presets"
              data-go="presets"
              onClick={(e) => {
                e.preventDefault();
                onNavigate?.("presets");
              }}
            >
              <span>Presets</span>
            </a>
            <button
              type="button"
              className={`btn ${isBypassed ? "on" : ""}`}
              data-toggle=""
              onClick={() => setIsBypassed(!isBypassed)}
            >
              <span>Bypass</span>
            </button>
            <a
              className="btn g"
              href="#rack"
              data-go="rack"
              aria-label="Close effect"
              onClick={(e) => {
                e.preventDefault();
                onNavigate?.("rack");
              }}
            >
              <Icon name="x" size={14} />
            </a>
          </div>

          <div
            className="in"
            style={{
              padding: 14,
              display: "flex",
              flexDirection: "column",
              gap: 14,
              flex: 1,
            }}
          >
            <div className="between">
              <span className="dim">Detected: speech, one speaker.</span>
              <Pill label="Runs on device" variant="ok" />
            </div>

            <div className="g2" style={{ flex: 1, minHeight: 200 }}>
              <div className="col">
                <span className="lab">Original</span>
                <div className="vz" style={{ flex: 1, minHeight: 100 }}>
                  <CanvasViz type="spec" seed={5} />
                </div>
              </div>
              <div className="col">
                <span className="lab">Cleaned</span>
                <div
                  className="vz"
                  style={{
                    flex: 1,
                    minHeight: 100,
                    filter: "contrast(1.25) brightness(.8)",
                  }}
                >
                  <CanvasViz type="spec" seed={2} />
                </div>
              </div>
            </div>

            <Slider
              label="Studio sound"
              value={studioSound}
              valueText={`${studioSound}%`}
              big={true}
              onChange={setStudioSound}
            />

            <button
              type="button"
              className={`btn lg hold ${isHoldingOriginal ? "active" : ""}`}
              data-hold=""
              onMouseDown={() => setIsHoldingOriginal(true)}
              onMouseUp={() => setIsHoldingOriginal(false)}
              onMouseLeave={() => setIsHoldingOriginal(false)}
            >
              <span>Hold for original</span>
            </button>
          </div>
        </div>

        <aside className="b in">
          <div className="sec">
            <h4 className="lab">Steps</h4>
            <Toggle label="Remove noise" checked={removeNoise} onChange={setRemoveNoise} />
            <Slider
              label="Amount"
              value={noiseAmount}
              valueText={`${noiseAmount}%`}
              onChange={setNoiseAmount}
            />
            <Toggle label="Remove echo" checked={removeEcho} onChange={setRemoveEcho} />
            <Slider
              label="Amount"
              value={echoAmount}
              valueText={`${echoAmount}%`}
              onChange={setEchoAmount}
            />
            <Toggle
              label="Rebuild missing highs"
              sub="AI invents detail here, so check it sounds like the speaker"
              checked={rebuildHighs}
              onChange={setRebuildHighs}
            />
            <Toggle
              label="Match a studio mic"
              checked={matchStudioMic}
              onChange={setMatchStudioMic}
            />
            <Toggle label="De-ess" checked={deEss} onChange={setDeEss} />
            <Toggle label="Even out level" checked={evenOutLevel} onChange={setEvenOutLevel} />
          </div>

          <Slider
            label="Keep room tone"
            value={keepRoomTone}
            valueText="−20 dB"
            onChange={setKeepRoomTone}
          />

          <Callout variant="info">
            Timing doesn't change, so your edits and transcript stay lined up.
          </Callout>

          <a
            className="btn p lg"
            href="#edit"
            data-go="edit"
            onClick={(e) => {
              e.preventDefault();
              onNavigate?.("edit");
            }}
          >
            <span>Apply</span>
          </a>
        </aside>
      </div>
    </ProjectLayout>
  );
};
