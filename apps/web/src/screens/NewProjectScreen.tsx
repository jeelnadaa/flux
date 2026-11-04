import React, { useState } from "react";
import { PageLayout, Tile, Field, Input, Select, Button } from "@flux/ui";
import { ProjectStore } from "@flux/app-core";

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

  const handleCreateProject = (templateTitle: string) => {
    const store = ProjectStore.get();
    store.pushUndo();
    const st = store.getState();
    st.name =
      name === "Untitled project" && templateTitle !== "Blank"
        ? `${templateTitle} Project`
        : name;
    const sr =
      sampleRate === "44.1 kHz"
        ? 44100
        : sampleRate === "96 kHz"
          ? 96000
          : sampleRate === "192 kHz"
            ? 192000
            : 48000;
    st.sampleRate = sr;

    const tColors = ["--t1", "--t2", "--t3", "--t4", "--t5"];
    let trackNames = ["Track 1"];
    if (templateTitle === "Podcast") trackNames = ["Host Voice", "Guest Voice", "Music Bed"];
    else if (templateTitle === "Music")
      trackNames = [
        "Kick",
        "Snare",
        "Bass",
        "Lead Vocal",
        "Synth 1",
        "Synth 2",
        "Guitars",
        "Master FX",
      ];
    else if (templateTitle === "Voiceover") trackNames = ["Lead Voice", "Room Tone"];
    else if (templateTitle === "Lecture") trackNames = ["Speaker Mic"];
    else if (templateTitle === "Interview") trackNames = ["Interviewer", "Subject"];
    else if (templateTitle === "8D music") trackNames = ["Stereo In", "Spatial Bed"];

    st.tracks = trackNames.map((tName, idx) => ({
      id: `track-${Date.now()}-${idx + 1}`,
      name: tName,
      colorVar: tColors[idx % tColors.length],
      volume: 1.0,
      pan: 0.0,
      muted: false,
      solo: false,
      tall: true,
      clips: [],
    }));
    st.totalFrames = sr * 16;
    st.playheadFrame = 0;
    st.selectedClipId = null;
    st.selectedTrackId = st.tracks[0]?.id || null;
    navigate("edit");
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
          onClick={() => handleCreateProject("Blank")}
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
            onClick={() => handleCreateProject(title)}
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
