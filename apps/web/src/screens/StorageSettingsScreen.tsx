import React, { useState } from "react";
import { SettingsLayout, ProgressBar, Pill, Button, Field, Input } from "@flux/ui";

const STORAGE_BREAKDOWN = [
  { name: "Projects in browser", sub: "2 projects", size: "422 MB", action: "Manage" },
  { name: "AI models", sub: "4 models", size: "1.2 GB", action: "Manage" },
  { name: "Waveform caches", sub: "Can be rebuilt", size: "3.1 GB", action: "Clear" },
  { name: "AI results", sub: "Stems, cleaned audio", size: "1.6 GB", action: "Clear" },
];

export const StorageSettingsScreen: React.FC = () => {
  const [defaultFolder, setDefaultFolder] = useState("My Audio");

  return (
    <SettingsLayout currentTab="storage" title="Storage">
      <div className="box col">
        <div className="between">
          <b>Browser storage</b>
          <span className="mono">6.4 of 24 GB</span>
        </div>
        <ProgressBar percent={27} />
        <div className="between">
          <Pill label="Persistent: granted" variant="ok" />
          <span className="dim">The browser won't clear this automatically</span>
        </div>
      </div>

      <div className="lst box">
        {STORAGE_BREAKDOWN.map((item) => (
          <div key={item.name}>
            <div className="ti">
              <b>{item.name}</b>
              <span>{item.sub}</span>
            </div>
            <span className="mono">{item.size}</span>
            <Button label={item.action} variant="ghost" />
          </div>
        ))}
      </div>

      <Field label="Default project folder">
        <div className="row" style={{ flexWrap: "nowrap" }}>
          <Input value={defaultFolder} onChange={(e) => setDefaultFolder(e.target.value)} />
          <Button label="Change" />
        </div>
      </Field>
    </SettingsLayout>
  );
};
