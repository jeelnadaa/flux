import React, { useState } from "react";
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
  const [outputDevice, setOutputDevice] = useState("System default");
  const [micDevice, setMicDevice] = useState("Studio Mic USB");
  const [qualityIndex, setQualityIndex] = useState(0); // Best
  const [lowerQualityOnGlitch, setLowerQualityOnGlitch] = useState(true);
  const [softStart, setSoftStart] = useState(true);
  const [limiter, setLimiter] = useState(true);
  const [loudExportWarn, setLoudExportWarn] = useState(true);

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
          <Select
            options={["System default", "Studio Headphones", "Laptop speakers"]}
            value={outputDevice}
            onChange={setOutputDevice}
          />
        </Field>
        <Field label="Microphone">
          <Select
            options={["Studio Mic USB", "Laptop mic"]}
            value={micDevice}
            onChange={setMicDevice}
          />
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
