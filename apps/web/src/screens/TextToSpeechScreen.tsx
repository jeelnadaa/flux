import React, { useState } from "react";
import {
  ProjectLayout,
  SegmentedControl,
  Slider,
  Field,
  Select,
  Section,
  Pill,
  Callout,
  Icon,
  CanvasViz,
} from "@flux/ui";

interface TextToSpeechScreenProps {
  onNavigate?: (route: string) => void;
}

export const TextToSpeechScreen: React.FC<TextToSpeechScreenProps> = ({ onNavigate }) => {
  const [script, setScript] = useState<string>(
    "Tonight on Harbor Lights: the ships that never came home, [pause 500ms] and the people who still listen for them.",
  );
  const [selectedVoice, setSelectedVoice] = useState<string>("Ash");
  const [rate, setRate] = useState<number>(50);
  const [pitch, setPitch] = useState<number>(50);
  const [runOnIndex, setRunOnIndex] = useState<number>(0);

  const voices = [
    { name: "Ash", desc: "Warm, narration" },
    { name: "Reed", desc: "Clear, neutral" },
    { name: "Marin", desc: "Bright, friendly" },
    { name: "Oren", desc: "Deep, slow" },
  ];

  const languages = [
    "English (US)",
    "English (UK)",
    "Hindi",
    "Spanish",
    "French",
    "German",
    "Japanese",
  ];

  const runOnOptions = ["This device", "Cloud, higher quality"] as const;

  const insertTag = (tag: string) => {
    setScript((prev) => `${prev} [${tag}]`);
  };

  return (
    <ProjectLayout currentTab="aitools" onNavigate={onNavigate}>
      <div className="split">
        <div className="a">
          <div className="pg in" style={{ maxWidth: "none" }}>
            <h1 className="h1">Write it. Hear it.</h1>
            <textarea
              className="inp"
              rows={9}
              aria-label="Script"
              style={{ fontSize: 18, fontWeight: 500 }}
              value={script}
              onChange={(e) => setScript(e.target.value)}
            />

            <div className="row">
              <button type="button" className="chip" onClick={() => insertTag("pause 500ms")}>
                Pause
              </button>
              <button type="button" className="chip" onClick={() => insertTag("emphasis")}>
                Emphasis
              </button>
              <button type="button" className="chip" onClick={() => insertTag("slower")}>
                Slower
              </button>
              <button type="button" className="chip" onClick={() => insertTag("pronounce")}>
                Pronounce…
              </button>
              <span className="sp" style={{ flex: 1 }} />
              <span className="mono dim">{script.length} characters · about 9 s</span>
            </div>

            <div className="vz" style={{ height: 96 }}>
              <CanvasViz type="wave" seed={6} kind="voice" step={2} />
            </div>

            <div className="row">
              <button type="button" className="btn lg">
                <Icon name="play" size={14} />
                Preview
              </button>
              <button type="button" className="btn p lg" onClick={() => onNavigate?.("edit")}>
                Add to timeline
              </button>
            </div>

            <Callout icon="tag">
              Generated speech is labeled as synthetic in the file's metadata.
            </Callout>
          </div>
        </div>

        <aside className="b in">
          <Field label="Language">
            <Select options={languages} />
          </Field>

          <Section title="Voice">
            <div className="lst">
              {voices.map((v) => (
                <div key={v.name}>
                  <button type="button" className="btn g" aria-label="Preview voice">
                    <Icon name="play" size={14} />
                  </button>
                  <div
                    className="ti"
                    style={{ cursor: "pointer" }}
                    onClick={() => setSelectedVoice(v.name)}
                  >
                    <b>{v.name}</b>
                    <span>{v.desc}</span>
                  </div>
                  {selectedVoice === v.name && <Pill label="Selected" variant="ai" />}
                </div>
              ))}
            </div>
          </Section>

          <Slider
            label="Rate"
            value={rate}
            valueText={`${(rate / 50).toFixed(1)}×`}
            onChange={setRate}
          />

          <Slider label="Pitch" value={pitch} valueText={`${pitch - 50} st`} onChange={setPitch} />

          <Section title="Run on">
            <SegmentedControl
              items={runOnOptions}
              selectedIndex={runOnIndex}
              onChange={(_, idx) => setRunOnIndex(idx)}
            />
          </Section>

          <a
            className="dim"
            href="#clone"
            data-go="clone"
            onClick={(e) => {
              e.preventDefault();
              onNavigate?.("clone");
            }}
          >
            Use your own voice
          </a>
        </aside>
      </div>
    </ProjectLayout>
  );
};
