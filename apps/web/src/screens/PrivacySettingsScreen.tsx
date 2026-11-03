import React, { useState } from "react";
import { SettingsLayout, Callout, Toggle, Button } from "@flux/ui";

export const PrivacySettingsScreen: React.FC = () => {
  const [cloudStems, setCloudStems] = useState(false);
  const [cloudTts, setCloudTts] = useState(false);
  const [cloudDubbing, setCloudDubbing] = useState(false);
  const [crashReports, setCrashReports] = useState(false);
  const [usageStats, setUsageStats] = useState(false);

  return (
    <SettingsLayout currentTab="privacy" title="Privacy">
      <Callout icon="lock" variant="ac">
        <b>Nothing leaves this device right now.</b> 0 bytes uploaded this session.
      </Callout>

      <div className="box col">
        <b>Cloud, per feature</b>
        <Toggle label="Stems: Best quality" checked={cloudStems} onChange={setCloudStems} />
        <Toggle
          label="Text to speech: higher quality voices"
          checked={cloudTts}
          onChange={setCloudTts}
        />
        <Toggle
          label="Dubbing"
          sub="Required for this feature"
          checked={cloudDubbing}
          onChange={setCloudDubbing}
        />
      </div>

      <div className="box col">
        <Toggle
          label="Crash reports"
          sub="Never includes audio, transcripts or file names"
          checked={crashReports}
          onChange={setCrashReports}
        />
        <Toggle label="Usage statistics" checked={usageStats} onChange={setUsageStats} />
      </div>

      <div className="row">
        <Button label="Delete all my cloud data" variant="danger" />
        <Button label="See what is stored" variant="ghost" />
      </div>
    </SettingsLayout>
  );
};
