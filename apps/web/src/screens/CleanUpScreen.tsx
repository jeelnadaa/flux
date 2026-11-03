import React, { useState } from "react";
import {
  EffectPanelShell,
  CanvasViz,
  Slider,
  Toggle,
  Section,
  Pill,
  Button,
  Callout,
  SegmentedControl,
} from "@flux/ui";

const TABS = ["Noise", "Hum", "Clicks", "Clipping", "Wind", "Stereo tools"];

export const CleanUpScreen: React.FC = () => {
  const [activeTab, setActiveTab] = useState("Noise");
  const [isBypassed, setIsBypassed] = useState(false);

  // Common parameters
  const [amount, setAmount] = useState(66);
  const [listenDifference, setListenDifference] = useState(false);

  // Noise tab parameters
  const [sensitivity, setSensitivity] = useState(45);
  const [smoothing, setSmoothing] = useState(58);
  const [keepRoomTone, setKeepRoomTone] = useState(30);
  const [profileLearned, setProfileLearned] = useState(true);

  // Hum tab parameters
  const [humFreqIdx, setHumFreqIdx] = useState(0); // 0: 50 Hz, 1: 60 Hz, 2: Auto
  const [humHarmonics, setHumHarmonics] = useState(8);
  const [humQ, setHumQ] = useState(60);

  // Clicks tab parameters
  const [clicksSens, setClicksSens] = useState(50);
  const [deCrackle, setDeCrackle] = useState(true);

  // Clipping tab parameters
  const [clipCeiling, setClipCeiling] = useState(90);
  const [clipMethodIdx, setClipMethodIdx] = useState(0); // Cubic, Spline, AR

  // Wind tab parameters
  const [windCutoff, setWindCutoff] = useState(80);
  const [windDepth, setWindDepth] = useState(65);

  // Stereo tools tab parameters
  const [stereoWidth, setStereoWidth] = useState(50); // 100%
  const [stereoPan, setStereoPan] = useState(50); // Center
  const [swapChannels, setSwapChannels] = useState(false);
  const [invertPhase, setInvertPhase] = useState(false);

  // Sensitivity display string
  const sensitivityText = sensitivity < 35 ? "Low" : sensitivity > 65 ? "High" : "Medium";

  // Amount display string
  const amountDb = Math.round(-36 * (amount / 100));
  const amountText =
    amount === 66 ? "−18 dB" : amountDb < 0 ? `−${Math.abs(amountDb)} dB` : `${amountDb} dB`;

  // Smoothing display string
  const smoothingText = smoothing === 58 ? "120 ms" : `${Math.round(20 + smoothing * 2)} ms`;

  // Room tone display string
  const roomToneText =
    keepRoomTone === 30 ? "−24 dB" : `−${Math.round(12 + (100 - keepRoomTone) * 0.24)} dB`;

  return (
    <EffectPanelShell
      title="Clean up"
      closeGo="fxbrowser"
      isBypassed={isBypassed}
      onToggleBypass={() => setIsBypassed(!isBypassed)}
      main={
        <>
          <div className="tabs" style={{ padding: 0, marginTop: "-6px" }}>
            {TABS.map((tab) => (
              <button
                key={tab}
                className={activeTab === tab ? "on" : ""}
                type="button"
                onClick={() => setActiveTab(tab)}
              >
                {tab}
              </button>
            ))}
          </div>
          <div className="g2" style={{ flex: 1, minHeight: "240px" }}>
            <div className="col">
              <span className="lab">Before</span>
              <div className="vz" style={{ flex: 1, minHeight: "110px" }}>
                <CanvasViz type="spec" seed={5} />
              </div>
            </div>
            <div className="col">
              <span className="lab">After</span>
              <div
                className="vz"
                style={{
                  flex: 1,
                  minHeight: "110px",
                  filter: "saturate(.85) contrast(1.18) brightness(.82)",
                }}
              >
                <CanvasViz type="spec" seed={3} />
              </div>
            </div>
          </div>
          <Slider label="Amount" value={amount} valueText={amountText} big onChange={setAmount} />
        </>
      }
      aside={
        activeTab === "Noise" ? (
          <>
            <Section title="Noise profile">
              <div className="box" style={{ padding: "10px" }}>
                <div className="between">
                  <b>Room noise</b>
                  <Pill label={profileLearned ? "Learned" : "Not learned"} variant="ok" />
                </div>
                <span className="lab">From 0:00.4 to 0:02.1</span>
              </div>
              <Button
                label="Learn from selection"
                icon="mic"
                onClick={() => setProfileLearned(true)}
              />
            </Section>
            <Slider
              label="Sensitivity"
              value={sensitivity}
              valueText={sensitivityText}
              onChange={setSensitivity}
            />
            <Slider
              label="Smoothing"
              value={smoothing}
              valueText={smoothingText}
              onChange={setSmoothing}
            />
            <Slider
              label="Keep room tone"
              value={keepRoomTone}
              valueText={roomToneText}
              onChange={setKeepRoomTone}
            />
            <Toggle
              label="Listen to what is removed"
              sub="Should only sound like noise"
              checked={listenDifference}
              onChange={setListenDifference}
            />
            <Callout icon="info">
              For speech,{" "}
              <a className="act" href="#cleanup" data-go="cleanup">
                AI voice cleanup
              </a>{" "}
              usually sounds better.
            </Callout>
          </>
        ) : activeTab === "Hum" ? (
          <>
            <Section title="Fundamental">
              <SegmentedControl
                items={["50 Hz", "60 Hz", "Auto"]}
                selectedIndex={humFreqIdx}
                onChange={(_, idx) => setHumFreqIdx(idx)}
              />
            </Section>
            <Slider
              label="Harmonics"
              value={(humHarmonics / 8) * 100}
              valueText={`${humHarmonics} harmonics`}
              onChange={(val) => setHumHarmonics(Math.max(1, Math.round((val / 100) * 8)))}
            />
            <Slider
              label="Notch Q"
              value={humQ}
              valueText={`Q = ${Math.round(10 + humQ * 0.5)}`}
              onChange={setHumQ}
            />
            <Toggle
              label="Listen to what is removed"
              sub="Should only sound like mains hum"
              checked={listenDifference}
              onChange={setListenDifference}
            />
          </>
        ) : activeTab === "Clicks" ? (
          <>
            <Slider
              label="Sensitivity"
              value={clicksSens}
              valueText={clicksSens < 40 ? "Low" : clicksSens > 60 ? "High" : "Medium"}
              onChange={setClicksSens}
            />
            <Toggle
              label="De-crackle"
              sub="Suppresses fine continuous vinyl surface crackle"
              checked={deCrackle}
              onChange={setDeCrackle}
            />
            <Toggle
              label="Listen to what is removed"
              sub="Should only sound like clicks"
              checked={listenDifference}
              onChange={setListenDifference}
            />
          </>
        ) : activeTab === "Clipping" ? (
          <>
            <Slider
              label="Ceiling threshold"
              value={clipCeiling}
              valueText={`−${((100 - clipCeiling) * 0.1).toFixed(1)} dBFS`}
              onChange={setClipCeiling}
            />
            <label className="fld">
              <span>Method</span>
              <SegmentedControl
                items={["Cubic", "Spline", "AR"]}
                selectedIndex={clipMethodIdx}
                onChange={(_, idx) => setClipMethodIdx(idx)}
              />
            </label>
            <Toggle
              label="Auto makeup headroom"
              sub="Prevents digital overs after peak restoration"
              defaultChecked={true}
            />
          </>
        ) : activeTab === "Wind" ? (
          <>
            <Slider
              label="Cutoff"
              value={windCutoff}
              valueText={`${Math.round(20 + windCutoff * 1.6)} Hz`}
              onChange={setWindCutoff}
            />
            <Slider
              label="Depth"
              value={windDepth}
              valueText={`−${Math.round(12 + windDepth * 0.36)} dB`}
              onChange={setWindDepth}
            />
            <Toggle
              label="Adaptive spectral gating"
              sub="Dynamically removes low frequency gusts"
              defaultChecked={true}
            />
          </>
        ) : (
          <>
            <Slider
              label="Stereo width"
              value={stereoWidth}
              valueText={`${Math.round(stereoWidth * 2)}%`}
              onChange={setStereoWidth}
            />
            <Slider
              label="Pan"
              value={stereoPan}
              valueText={
                stereoPan === 50
                  ? "Center"
                  : stereoPan < 50
                    ? `L${50 - stereoPan}`
                    : `R${stereoPan - 50}`
              }
              onChange={setStereoPan}
            />
            <Toggle
              label="Swap channels"
              sub="Swaps left and right channels"
              checked={swapChannels}
              onChange={setSwapChannels}
            />
            <Toggle
              label="Invert phase"
              sub="Inverts right channel polarity"
              checked={invertPhase}
              onChange={setInvertPhase}
            />
            <div className="box row" style={{ justifyContent: "space-between", padding: "10px" }}>
              <span>Phase correlation</span>
              <span className="mono bold">+0.94</span>
            </div>
          </>
        )
      }
    />
  );
};
