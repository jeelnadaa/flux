import React, { useState } from "react";
import { PageLayout, Tile, Field, Input, Select, Button } from "@flux/ui";

const TEMPLATES: readonly [string, string, string][] = [
  ["Blank", "One empty track at 48 kHz", "wave"],
  ["Podcast", "Two voices, music bed, −16 LUFS", "mic"],
  ["Music", "Eight tracks, buses, master chain", "mix"],
  ["Voiceover", "Voice, room tone, de-esser", "voice"],
  ["Lecture", "Long recording, silence removal", "script"],
  ["Interview", "Two speakers, transcript ready", "text"],
  ["8D music", "Stereo in, orbit on, reverb room", "orbit"],
];

interface NewProjectScreenProps {
  onNavigate?: (route: string) => void;
}

export const NewProjectScreen: React.FC<NewProjectScreenProps> = ({ onNavigate }) => {
  const [name, setName] = useState("Untitled project");
  const [sampleRate, setSampleRate] = useState("48 kHz");
  const [saveTo, setSaveTo] = useState("My Audio folder");

  const navigate = (r: string) => {
    if (onNavigate) {
      onNavigate(r);
    } else {
      window.location.hash = r;
    }
  };

  return (
    <PageLayout
      currentTab="home"
      title="Start with a shape."
      subtitle="Templates set up tracks, effects and an export target. All of it can be changed later."
      actions={
        <Button
          label="Create"
          variant="primary"
          dataGo="edit"
          href="#edit"
          onClick={() => navigate("edit")}
        />
      }
      onNavigate={navigate}
    >
      <div className="g3">
        {TEMPLATES.map(([title, subtitle, icon]) => (
          <Tile
            key={title}
            title={title}
            subtitle={subtitle}
            icon={icon}
            dataGo="edit"
            href="#edit"
            onClick={() => navigate("edit")}
          />
        ))}
      </div>

      <div className="box">
        <div className="g3">
          <Field label="Name">
            <Input value={name} onChange={(e) => setName(e.target.value)} />
          </Field>
          <Field label="Sample rate">
            <Select
              options={["48 kHz", "44.1 kHz", "96 kHz", "192 kHz"]}
              value={sampleRate}
              onChange={setSampleRate}
            />
          </Field>
          <Field label="Save to">
            <Select
              options={["My Audio folder", "This browser"]}
              value={saveTo}
              onChange={setSaveTo}
            />
          </Field>
        </div>
      </div>
    </PageLayout>
  );
};
