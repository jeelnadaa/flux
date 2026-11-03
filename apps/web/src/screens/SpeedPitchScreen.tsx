import React, { useState, useRef } from "react";
import {
  EffectPanelShell,
  CanvasViz,
  Slider,
  Toggle,
  Section,
  SegmentedControl,
  Chip,
  Button,
} from "@flux/ui";
import {
  TapTempoCalculator,
  calculateVarispeedPitch,
  formatDurationMinutesSeconds,
} from "@flux/app-core";

const SPEED_PRESETS = ["0.5×", "0.75×", "0.85×", "1×", "1.25×", "1.5×", "2×"];
const PITCH_PRESETS = ["−12", "−5", "−2", "0", "+2", "+5", "+12"];

const SPEED_MAP: Record<string, number> = {
  "0.5×": 0.5,
  "0.75×": 0.75,
  "0.85×": 0.85,
  "1×": 1.0,
  "1.25×": 1.25,
  "1.5×": 1.5,
  "2×": 2.0,
};

const PITCH_MAP: Record<string, number> = {
  "−12": -12,
  "−5": -5,
  "−2": -2,
  "0": 0,
  "+2": 2,
  "+5": 5,
  "+12": 12,
};

const BASE_DURATION_SEC = 222; // 3:42 base duration -> at 0.85x is ~4:21

export const SpeedPitchScreen: React.FC = () => {
  // Speed state
  const [speedVal, setSpeedVal] = useState(0.85);
  const [speedSliderPct, setSpeedSliderPct] = useState(34);
  const [activeSpeedChip, setActiveSpeedChip] = useState("0.85×");

  // Pitch state
  const [pitchVal, setPitchVal] = useState(-2.0);
  const [pitchSliderPct, setPitchSliderPct] = useState(42);
  const [activePitchChip, setActivePitchChip] = useState("−2");

  // Options
  const [keepPitch, setKeepPitch] = useState(true);
  const [modeIdx, setModeIdx] = useState(0); // 0: Music, 1: Voice, 2: Varispeed
  const [preserveFormants, setPreserveFormants] = useState(true);
  const [tempoBpm, setTempoBpm] = useState("84.0");
  const [showBeatGrid, setShowBeatGrid] = useState(true);
  const [renderQualityIdx, setRenderQualityIdx] = useState(1); // 1: High
  const [isBypassed, setIsBypassed] = useState(false);

  // Tap tempo calculator
  const tapCalculatorRef = useRef<TapTempoCalculator>(new TapTempoCalculator());

  const handleSpeedPreset = (chipLabel: string) => {
    setActiveSpeedChip(chipLabel);
    const ratio = SPEED_MAP[chipLabel] ?? 1.0;
    setSpeedVal(ratio);
    // Map ratio [0.25..4.0] to slider pct [0..100]
    const pct = Math.round(
      ((Math.log2(ratio) - Math.log2(0.25)) / (Math.log2(4.0) - Math.log2(0.25))) * 100,
    );
    setSpeedSliderPct(pct);

    if (modeIdx === 2) {
      // Varispeed mode links speed and pitch
      const linkedPitch = calculateVarispeedPitch(ratio);
      setPitchVal(linkedPitch);
      const pitchPct = Math.round(((linkedPitch + 24) / 48) * 100);
      setPitchSliderPct(pitchPct);
      const matchedChip = PITCH_PRESETS.find((p) => PITCH_MAP[p] === Math.round(linkedPitch));
      setActivePitchChip(matchedChip || "");
    }
  };

  const handlePitchPreset = (chipLabel: string) => {
    setActivePitchChip(chipLabel);
    const st = PITCH_MAP[chipLabel] ?? 0;
    setPitchVal(st);
    const pct = Math.round(((st + 24) / 48) * 100);
    setPitchSliderPct(pct);

    if (modeIdx === 2) {
      // Varispeed links pitch to speed: ratio = 2^(st / 12)
      const linkedRatio = Math.pow(2, st / 12);
      setSpeedVal(linkedRatio);
      const sPct = Math.round(
        ((Math.log2(linkedRatio) - Math.log2(0.25)) / (Math.log2(4.0) - Math.log2(0.25))) * 100,
      );
      setSpeedSliderPct(sPct);
      const matchedChip = SPEED_PRESETS.find((s) => Math.abs(SPEED_MAP[s] - linkedRatio) < 0.05);
      setActiveSpeedChip(matchedChip || "");
    }
  };

  const handleSpeedSlider = (pct: number) => {
    setSpeedSliderPct(pct);
    // Convert pct [0..100] to ratio [0.25..4.0]
    const ratio = Math.pow(2, Math.log2(0.25) + (pct / 100) * (Math.log2(4.0) - Math.log2(0.25)));
    setSpeedVal(ratio);
    const matched = SPEED_PRESETS.find((s) => Math.abs(SPEED_MAP[s] - ratio) < 0.04);
    setActiveSpeedChip(matched || "");

    if (modeIdx === 2) {
      const linkedPitch = calculateVarispeedPitch(ratio);
      setPitchVal(linkedPitch);
      setPitchSliderPct(Math.round(((linkedPitch + 24) / 48) * 100));
      const matchedPitch = PITCH_PRESETS.find((p) => Math.abs(PITCH_MAP[p] - linkedPitch) < 0.5);
      setActivePitchChip(matchedPitch || "");
    }
  };

  const handlePitchSlider = (pct: number) => {
    setPitchSliderPct(pct);
    // Convert pct [0..100] to semitones [-24..+24]
    const st = -24 + (pct / 100) * 48;
    setPitchVal(st);
    const matched = PITCH_PRESETS.find((p) => Math.abs(PITCH_MAP[p] - st) < 0.5);
    setActivePitchChip(matched || "");

    if (modeIdx === 2) {
      const linkedRatio = Math.pow(2, st / 12);
      setSpeedVal(linkedRatio);
      const sPct = Math.round(
        ((Math.log2(linkedRatio) - Math.log2(0.25)) / (Math.log2(4.0) - Math.log2(0.25))) * 100,
      );
      setSpeedSliderPct(sPct);
      const matchedSpeed = SPEED_PRESETS.find((s) => Math.abs(SPEED_MAP[s] - linkedRatio) < 0.05);
      setActiveSpeedChip(matchedSpeed || "");
    }
  };

  const handleModeChange = (idx: number) => {
    setModeIdx(idx);
    if (idx === 2) {
      // Varispeed: sync pitch to current speed
      const linkedPitch = calculateVarispeedPitch(speedVal);
      setPitchVal(linkedPitch);
      setPitchSliderPct(Math.round(((linkedPitch + 24) / 48) * 100));
    }
  };

  const handleTap = () => {
    const bpm = tapCalculatorRef.current.tap();
    if (bpm !== null) {
      setTempoBpm(bpm.toFixed(1));
    }
  };

  // Formatted duration string
  const currentDurationSec = BASE_DURATION_SEC / (speedVal || 1.0);
  const durationText = `${formatDurationMinutesSeconds(currentDurationSec)} long`;

  // Formatted cents string
  const cents = Math.round(pitchVal * 100);
  const centsText = cents < 0 ? `−${Math.abs(cents)} cents` : `+${cents} cents`;

  // Formatted stat string
  const pitchStatText =
    pitchVal < 0 ? `−${Math.abs(pitchVal).toFixed(1)} st` : `+${pitchVal.toFixed(1)} st`;

  return (
    <EffectPanelShell
      title="Speed and pitch"
      closeGo="fxbrowser"
      isBypassed={isBypassed}
      onToggleBypass={() => setIsBypassed(!isBypassed)}
      main={
        <>
          <div className="vz" style={{ height: "140px" }}>
            <CanvasViz type="wave" seed={1} stereo={true} color="--t1" step={2.5} />
          </div>
          <div className="g2">
            <div className="box col">
              <div className="between">
                <b className="h3">Speed</b>
                <b className="stat">{speedVal.toFixed(2)}×</b>
              </div>
              <Slider
                label=""
                value={speedSliderPct}
                valueText={durationText}
                onChange={handleSpeedSlider}
              />
              <div className="row">
                {SPEED_PRESETS.map((s) => (
                  <Chip
                    key={s}
                    label={s}
                    active={activeSpeedChip === s}
                    onClick={() => handleSpeedPreset(s)}
                  />
                ))}
              </div>
            </div>
            <div className="box col">
              <div className="between">
                <b className="h3">Pitch</b>
                <b className="stat">{pitchStatText}</b>
              </div>
              <Slider
                label=""
                value={pitchSliderPct}
                valueText={centsText}
                onChange={handlePitchSlider}
              />
              <div className="row">
                {PITCH_PRESETS.map((s) => (
                  <Chip
                    key={s}
                    label={s}
                    active={activePitchChip === s}
                    onClick={() => handlePitchPreset(s)}
                  />
                ))}
              </div>
            </div>
          </div>
          <div className="box row">
            <Toggle
              label="Keep pitch when changing speed"
              checked={keepPitch}
              onChange={setKeepPitch}
            />
          </div>
        </>
      }
      aside={
        <>
          <Section title="Mode">
            <SegmentedControl
              items={["Music", "Voice", "Varispeed"]}
              selectedIndex={modeIdx}
              onChange={(_, idx) => handleModeChange(idx)}
            />
            <span className="dim" style={{ fontSize: "12.5px" }}>
              Music keeps drum hits sharp. Voice keeps speech natural. Varispeed links speed and
              pitch, like a turntable.
            </span>
          </Section>
          <Toggle
            label="Preserve formants"
            sub="Stops voices sounding like chipmunks or giants"
            checked={preserveFormants}
            onChange={setPreserveFormants}
          />
          <Section title="Tempo">
            <div className="g2">
              <label className="fld">
                <span>Tempo</span>
                <input
                  className="inp"
                  value={tempoBpm}
                  onChange={(e) => setTempoBpm(e.target.value)}
                />
              </label>
              <div className="fld">
                <span>Tap</span>
                <Button label="Tap" dataToggle onClick={handleTap} />
              </div>
            </div>
            <div className="row">
              <Toggle label="Show beat grid" checked={showBeatGrid} onChange={setShowBeatGrid} />
            </div>
          </Section>
          <label className="fld">
            <span>Final render</span>
            <SegmentedControl
              items={["Fast", "High", "Mastering"]}
              selectedIndex={renderQualityIdx}
              onChange={(_, idx) => setRenderQualityIdx(idx)}
            />
          </label>
        </>
      }
    />
  );
};
