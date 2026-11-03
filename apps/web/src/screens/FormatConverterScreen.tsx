import React, { useState } from "react";
import { PageLayout, Icon, Field, SegmentedControl, Select, ProgressBar, Button } from "@flux/ui";

const CONVERT_ITEMS = [
  { name: "interview-raw.wav", size: "1.2 GB", progress: 100 },
  { name: "harbor-lights.flac", size: "38 MB", progress: 64 },
  { name: "voice-memo.m4a", size: "12 MB", progress: 0 },
];

export const FormatConverterScreen: React.FC = () => {
  const [selectedFormatIndex, setSelectedFormatIndex] = useState(0);
  const [quality, setQuality] = useState("MP3 V0, about 245 kbps");
  const [sampleRate, setSampleRate] = useState("Keep original");

  return (
    <PageLayout
      currentTab="convert"
      title="Convert"
      subtitle="Files are converted on this device, and tags and cover art are kept."
      actions={<Button label="Convert 3 files" variant="primary" />}
    >
      <a className="drop" href="#convert" data-go="convert">
        <span className="act">
          <Icon name="imp" size={26} />
        </span>
        <b style={{ fontSize: "17px", fontWeight: 800 }}>Drop files here</b>
      </a>

      <div className="box row">
        <Field label="To">
          <SegmentedControl
            items={["MP3", "WAV", "FLAC", "M4A", "OGG", "Opus"]}
            selectedIndex={selectedFormatIndex}
            onChange={(_item, idx) => setSelectedFormatIndex(idx)}
          />
        </Field>
        <div style={{ minWidth: "170px", flex: 1 }}>
          <Field label="Quality">
            <Select
              options={["MP3 V0, about 245 kbps", "MP3 320 kbps", "MP3 192 kbps"]}
              value={quality}
              onChange={setQuality}
            />
          </Field>
        </div>
        <Field label="Sample rate">
          <Select
            options={["Keep original", "48 kHz", "44.1 kHz"]}
            value={sampleRate}
            onChange={setSampleRate}
          />
        </Field>
      </div>

      <div className="lst box">
        {CONVERT_ITEMS.map((item) => (
          <div key={item.name} style={{ flexWrap: "wrap" }}>
            <div className="ti" style={{ minWidth: "150px" }}>
              <b>{item.name}</b>
              <span>{item.size}</span>
            </div>
            <div style={{ flex: 1, minWidth: "90px" }}>
              <ProgressBar
                percent={item.progress}
                running={item.progress > 0 && item.progress < 100}
              />
            </div>
            <span className="mono dim">{item.progress}%</span>
          </div>
        ))}
      </div>
    </PageLayout>
  );
};
