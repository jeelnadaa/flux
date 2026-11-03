import React, { useState } from "react";
import { EffectPanelShell, CanvasViz, Slider, Toggle, Section } from "@flux/ui";

const TABS = ["Compressor", "Limiter", "Multiband", "Gate", "De-esser", "Transient", "Leveler"];

export const DynamicsScreen: React.FC = () => {
  const [activeTab, setActiveTab] = useState("Compressor");
  const [isBypassed, setIsBypassed] = useState(false);

  // Compressor parameters
  const [threshold, setThreshold] = useState(62);
  const [ratio, setRatio] = useState(40);
  const [attack, setAttack] = useState(22);
  const [release, setRelease] = useState(38);
  const [knee, setKnee] = useState(50);
  const [makeup, setMakeup] = useState(45);
  const [lookahead, setLookahead] = useState(15);
  const [mix, setMix] = useState(100);
  const [autoMakeup, setAutoMakeup] = useState(true);
  const [progRelease, setProgRelease] = useState(true);
  const [sidechainSource, setSidechainSource] = useState("This track");
  const [lowCut, setLowCut] = useState(18);

  // Limiter parameters
  const [ceiling, setCeiling] = useState(90);
  const [limiterRelease, setLimiterRelease] = useState(50);
  const [limiterLookahead, setLimiterLookahead] = useState(30);

  // Gate parameters
  const [gateThreshold, setGateThreshold] = useState(40);
  const [gateRatio, setGateRatio] = useState(50);
  const [gateAttack, setGateAttack] = useState(10);
  const [gateHold, setGateHold] = useState(20);
  const [gateRelease, setGateRelease] = useState(30);

  // De-esser parameters
  const [deessThreshold, setDeessThreshold] = useState(45);
  const [deessFreq, setDeessFreq] = useState(68);
  const [splitBand, setSplitBand] = useState(true);

  // Transient shaper parameters
  const [transientAttack, setTransientAttack] = useState(50);
  const [transientSustain, setTransientSustain] = useState(50);

  // Leveler parameters
  const [targetLufs, setTargetLufs] = useState(50);
  const [maxBoost, setMaxBoost] = useState(40);

  // Compute transfer th & ratio for visualizer based on active mode
  let transferTh = 0.62;
  let transferRatio = 3.2;

  if (activeTab === "Compressor") {
    transferTh = threshold / 100;
    transferRatio = 1.0 + (ratio / 100) * 8.0;
  } else if (activeTab === "Limiter") {
    transferTh = ceiling / 100;
    transferRatio = 20.0;
  } else if (activeTab === "Gate") {
    transferTh = gateThreshold / 100;
    transferRatio = 0.2; // expander slope
  } else if (activeTab === "De-esser") {
    transferTh = deessThreshold / 100;
    transferRatio = 4.0;
  }

  const mainContent = (
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
      <div className="g2" style={{ flex: 1 }}>
        <div
          className="vz"
          style={{ minHeight: "240px", aspectRatio: "1/1", maxHeight: "360px" }}
          role="img"
          aria-label={`${activeTab} static transfer curve`}
        >
          <CanvasViz type="transfer" th={transferTh} ratio={transferRatio} />
        </div>
        <div className="col">
          <div
            className="vz"
            style={{ flex: 1, minHeight: "150px" }}
            role="img"
            aria-label="Real-time gain reduction history"
          >
            <CanvasViz type="gr" />
          </div>
          <div className="between">
            <span className="lab">Gain reduction</span>
            <b className="stat">−4.2 dB</b>
          </div>
        </div>
      </div>
    </>
  );

  const asideContent = (
    <>
      {activeTab === "Compressor" && (
        <>
          <div className="ctl-grid" style={{ gridTemplateColumns: "repeat(2,minmax(0,1fr))" }}>
            <Slider
              label="Threshold"
              value={threshold}
              valueText="−18 dB"
              onChange={setThreshold}
            />
            <Slider label="Ratio" value={ratio} valueText="3.2:1" onChange={setRatio} />
            <Slider label="Attack" value={attack} valueText="12 ms" onChange={setAttack} />
            <Slider label="Release" value={release} valueText="180 ms" onChange={setRelease} />
            <Slider label="Knee" value={knee} valueText="6 dB" onChange={setKnee} />
            <Slider label="Makeup" value={makeup} valueText="+3.5 dB" onChange={setMakeup} />
            <Slider label="Lookahead" value={lookahead} valueText="3 ms" onChange={setLookahead} />
            <Slider label="Mix" value={mix} valueText="100%" onChange={setMix} />
          </div>
          <Toggle
            label="Auto makeup"
            defaultChecked={autoMakeup}
            onChange={(checked) => setAutoMakeup(checked)}
          />
          <Toggle
            label="Program-dependent release"
            defaultChecked={progRelease}
            onChange={(checked) => setProgRelease(checked)}
          />
          <Section title="Sidechain">
            <label className="fld">
              <span>Listen to</span>
              <select
                className="inp"
                value={sidechainSource}
                onChange={(e) => setSidechainSource(e.target.value)}
              >
                <option>This track</option>
                <option>Drums</option>
                <option>Music bus</option>
              </select>
            </label>
            <Slider label="Low cut" value={lowCut} valueText="80 Hz" onChange={setLowCut} />
          </Section>
        </>
      )}

      {activeTab === "Limiter" && (
        <>
          <div className="ctl-grid" style={{ gridTemplateColumns: "repeat(2,minmax(0,1fr))" }}>
            <Slider label="Ceiling" value={ceiling} valueText="−1.0 dBTP" onChange={setCeiling} />
            <Slider
              label="Release"
              value={limiterRelease}
              valueText="50 ms"
              onChange={setLimiterRelease}
            />
            <Slider
              label="Lookahead"
              value={limiterLookahead}
              valueText="3 ms"
              onChange={setLimiterLookahead}
            />
            <Slider label="True peak" value={100} valueText="4× polyphase" onChange={() => {}} />
          </div>
          <Toggle label="ISP-safe lookahead" defaultChecked={true} />
        </>
      )}

      {activeTab === "Multiband" && (
        <>
          <div className="ctl-grid" style={{ gridTemplateColumns: "repeat(2,minmax(0,1fr))" }}>
            <Slider label="Low (20-200 Hz)" value={60} valueText="−18 dB" onChange={() => {}} />
            <Slider label="Low-Mid (200-1.5k)" value={55} valueText="−16 dB" onChange={() => {}} />
            <Slider label="High-Mid (1.5k-6k)" value={50} valueText="−14 dB" onChange={() => {}} />
            <Slider label="High (>6 kHz)" value={45} valueText="−12 dB" onChange={() => {}} />
          </div>
          <Toggle label="Linkwitz-Riley 4th-order (LR4)" defaultChecked={true} />
        </>
      )}

      {activeTab === "Gate" && (
        <>
          <div className="ctl-grid" style={{ gridTemplateColumns: "repeat(2,minmax(0,1fr))" }}>
            <Slider
              label="Threshold"
              value={gateThreshold}
              valueText="−40 dB"
              onChange={setGateThreshold}
            />
            <Slider label="Ratio" value={gateRatio} valueText="20:1" onChange={setGateRatio} />
            <Slider label="Attack" value={gateAttack} valueText="2 ms" onChange={setGateAttack} />
            <Slider label="Hold" value={gateHold} valueText="20 ms" onChange={setGateHold} />
            <Slider
              label="Release"
              value={gateRelease}
              valueText="100 ms"
              onChange={setGateRelease}
            />
            <Slider label="Hysteresis" value={30} valueText="3 dB" onChange={() => {}} />
          </div>
          <Toggle label="Hysteresis chatter prevention" defaultChecked={true} />
        </>
      )}

      {activeTab === "De-esser" && (
        <>
          <div className="ctl-grid" style={{ gridTemplateColumns: "repeat(2,minmax(0,1fr))" }}>
            <Slider
              label="Threshold"
              value={deessThreshold}
              valueText="−24 dB"
              onChange={setDeessThreshold}
            />
            <Slider
              label="Frequency"
              value={deessFreq}
              valueText="6.8 kHz"
              onChange={setDeessFreq}
            />
            <Slider label="Ratio" value={40} valueText="4.0:1" onChange={() => {}} />
            <Slider label="Bandwidth" value={50} valueText="1.0 oct" onChange={() => {}} />
          </div>
          <Toggle
            label="Split-band ducking"
            defaultChecked={splitBand}
            onChange={(checked) => setSplitBand(checked)}
          />
        </>
      )}

      {activeTab === "Transient" && (
        <>
          <div className="ctl-grid" style={{ gridTemplateColumns: "repeat(2,minmax(0,1fr))" }}>
            <Slider
              label="Attack"
              value={transientAttack}
              valueText="0.0 dB"
              onChange={setTransientAttack}
            />
            <Slider
              label="Sustain"
              value={transientSustain}
              valueText="0.0 dB"
              onChange={setTransientSustain}
            />
          </div>
          <Toggle label="Dual envelope follower" defaultChecked={true} />
        </>
      )}

      {activeTab === "Leveler" && (
        <>
          <div className="ctl-grid" style={{ gridTemplateColumns: "repeat(2,minmax(0,1fr))" }}>
            <Slider
              label="Target LUFS"
              value={targetLufs}
              valueText="−16.0 LUFS"
              onChange={setTargetLufs}
            />
            <Slider label="Max boost" value={maxBoost} valueText="+8.0 dB" onChange={setMaxBoost} />
            <Slider label="Max cut" value={50} valueText="−12.0 dB" onChange={() => {}} />
            <Slider label="Gate threshold" value={30} valueText="−45.0 dBFS" onChange={() => {}} />
          </div>
          <Toggle label="Speech-aware noise gating" defaultChecked={true} />
        </>
      )}
    </>
  );

  return (
    <EffectPanelShell
      title="Compressor"
      isBypassed={isBypassed}
      onToggleBypass={() => setIsBypassed(!isBypassed)}
      closeGo="fxbrowser"
      main={mainContent}
      aside={asideContent}
    />
  );
};
