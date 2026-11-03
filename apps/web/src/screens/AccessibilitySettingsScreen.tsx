import React, { useState } from "react";
import { SettingsLayout, Toggle, Slider, Field, SegmentedControl, Button } from "@flux/ui";

interface AccessibilitySettingsScreenProps {
  onNavigate?: (route: string) => void;
}

export const AccessibilitySettingsScreen: React.FC<AccessibilitySettingsScreenProps> = ({
  onNavigate,
}) => {
  const [reduceMotion, setReduceMotion] = useState(false);
  const [highContrast, setHighContrast] = useState(false);
  const [colorBlindSafe, setColorBlindSafe] = useState(false);
  const [textSize, setTextSize] = useState(40);
  const [describeWaveforms, setDescribeWaveforms] = useState(true);
  const [announcePosition, setAnnouncePosition] = useState(false);
  const [announcementRateIndex, setAnnouncementRateIndex] = useState(1); // Medium

  const navigate = (r: string) => {
    if (onNavigate) {
      onNavigate(r);
    } else {
      window.location.hash = r;
    }
  };

  return (
    <SettingsLayout currentTab="access" title="Accessibility">
      <div className="box col">
        <Toggle
          label="Reduce motion"
          sub="Follows your system setting"
          checked={reduceMotion}
          onChange={setReduceMotion}
        />
        <Toggle label="High contrast" checked={highContrast} onChange={setHighContrast} />
        <Toggle
          label="Color-blind safe track colors"
          checked={colorBlindSafe}
          onChange={setColorBlindSafe}
        />
        <Slider label="Text size" value={textSize} valueText="100%" onChange={setTextSize} />
      </div>

      <div className="box col">
        <Toggle
          label="Describe waveforms for screen readers"
          sub="For example: loud for 10 seconds, then quiet"
          checked={describeWaveforms}
          onChange={setDescribeWaveforms}
        />
        <Toggle
          label="Announce playback position"
          checked={announcePosition}
          onChange={setAnnouncePosition}
        />
        <Field label="Announcement rate">
          <SegmentedControl
            items={["Low", "Medium", "High"]}
            selectedIndex={announcementRateIndex}
            onChange={(_item, idx) => setAnnouncementRateIndex(idx)}
          />
        </Field>
      </div>

      <Button
        label="See keyboard commands"
        icon="keys"
        dataGo="prefs-keys"
        href="#prefs-keys"
        onClick={(e) => {
          e.preventDefault();
          navigate("prefs-keys");
        }}
      />
    </SettingsLayout>
  );
};
