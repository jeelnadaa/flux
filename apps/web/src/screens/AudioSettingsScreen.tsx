import React, { useState, useEffect } from "react";
import {
  SettingsLayout,
  Field,
  Select,
  SegmentedControl,
  Toggle,
  Pill,
  Section,
  Button,
} from "@flux/ui";

interface AudioSettingsScreenProps {
  onNavigate?: (route: string) => void;
}

export const AudioSettingsScreen: React.FC<AudioSettingsScreenProps> = ({ onNavigate }) => {
  const [outputOptions, setOutputOptions] = useState<string[]>([
    "System default",
    "Studio Headphones",
    "Laptop speakers",
  ]);
  const [micOptions, setMicOptions] = useState<string[]>(["Studio Mic USB", "Laptop mic"]);
  const [outputDevice, setOutputDevice] = useState("System default");
  const [micDevice, setMicDevice] = useState("Studio Mic USB");
  const [qualityIndex, setQualityIndex] = useState(0); // Best
  const [lowerQualityOnGlitch, setLowerQualityOnGlitch] = useState(true);
  const [softStart, setSoftStart] = useState(true);
  const [limiter, setLimiter] = useState(true);
  const [loudExportWarn, setLoudExportWarn] = useState(true);

  useEffect(() => {
    if (typeof navigator !== "undefined" && navigator.mediaDevices?.enumerateDevices) {
      navigator.mediaDevices
        .enumerateDevices()
        .then((devices) => {
          const ins = devices
            .filter((d) => d.kind === "audioinput")
            .map((d) => d.label || "Microphone");
          const outs = devices
            .filter((d) => d.kind === "audiooutput")
            .map((d) => d.label || "Speakers");
          if (ins.length > 0) {
            const uniqueIns = Array.from(new Set(ins));
            setMicOptions(uniqueIns);
            setMicDevice(uniqueIns[0]);
          }
          if (outs.length > 0) {
            const uniqueOuts = Array.from(new Set(["System default", ...outs]));
            setOutputOptions(uniqueOuts);
            setOutputDevice(uniqueOuts[0]);
          }
        })
        .catch(() => {});
    }
  }, []);

  const navigate = (r: string) => {
    if (onNavigate) {
      onNavigate(r);
    } else {
      window.location.hash = r;
    }
  };

  return (
    <SettingsLayout currentTab="audio" title="Audio">
      <div className="box col">
        <Field label="Output">
          <Select options={outputOptions} value={outputDevice} onChange={setOutputDevice} />
        </Field>
        <Field label="Microphone">
          <Select options={micOptions} value={micDevice} onChange={setMicDevice} />
        </Field>
        <div className="between">
          <span className="dim">Device runs at 44.1 kHz, project at 48 kHz.</span>
          <Pill label="Resampled at output" />
        </div>
      </div>

      <div className="box col">
        <Field label="Quality">
          <SegmentedControl
            items={["Best", "Balanced", "Battery saver"]}
            selectedIndex={qualityIndex}
            onChange={(_item, idx) => setQualityIndex(idx)}
          />
        </Field>
        <Toggle
          label="Lower quality automatically if audio glitches"
          checked={lowerQualityOnGlitch}
          onChange={setLowerQualityOnGlitch}
        />
        <div className="between">
          <span className="dim">Glitches this session</span>
          <span className="mono">0</span>
        </div>
      </div>

      <Section title="Hearing safety">
        <div className="box col">
          <Toggle
            label="Soft start when playback begins"
            checked={softStart}
            onChange={setSoftStart}
          />
          <Toggle
            label="Output limiter"
            sub="Catches sudden loud peaks"
            checked={limiter}
            onChange={setLimiter}
          />
          <Toggle
            label="Warn about very loud exports"
            checked={loudExportWarn}
            onChange={setLoudExportWarn}
          />
        </div>
      </Section>

      <Button
        label="Run headphone check"
        icon="head"
        dataGo="hpcheck"
        href="#hpcheck"
        onClick={(e) => {
          e.preventDefault();
          navigate("hpcheck");
        }}
      />
    </SettingsLayout>
  );
};
