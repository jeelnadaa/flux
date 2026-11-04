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
import { ByokAiManager, ProjectStore } from "@flux/app-core";

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

  const [isSynthesizing, setIsSynthesizing] = useState<boolean>(false);
  const [providerStatus, setProviderStatus] = useState<string>("");

  const handlePreview = async () => {
    if (!script.trim() || isSynthesizing) return;
    setIsSynthesizing(true);
    setProviderStatus("Synthesizing speech...");

    try {
      const ai = ByokAiManager.get();
      const res = await ai.synthesizeSpeech(script, { voice: selectedVoice });
      setProviderStatus(`Generated using ${res.providerUsed}`);

      const AudioContextClass =
        window.AudioContext ||
        (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
      const ctx = new AudioContextClass();
      const buf = await ctx.decodeAudioData(res.audioBuffer.slice(0));
      const src = ctx.createBufferSource();
      src.buffer = buf;
      src.connect(ctx.destination);
      src.start();
    } catch (err) {
      console.error("Preview TTS failed:", err);
      setProviderStatus("Synthesis failed");
    } finally {
      setIsSynthesizing(false);
    }
  };

  const handleAddToTimeline = async () => {
    if (!script.trim() || isSynthesizing) return;
    setIsSynthesizing(true);
    setProviderStatus("Rendering speech to timeline...");

    try {
      const ai = ByokAiManager.get();
      const res = await ai.synthesizeSpeech(script, { voice: selectedVoice });
      const blob = new Blob([res.audioBuffer], {
        type: res.format === "mp3" ? "audio/mpeg" : "audio/wav",
      });
      await ProjectStore.get().importAudioFile(blob, `Speech: ${selectedVoice}`);
      if (onNavigate) {
        onNavigate("edit");
      } else {
        window.location.hash = "#edit";
      }
    } catch (err) {
      console.error("Add TTS to timeline failed:", err);
      setProviderStatus("Failed to add speech to timeline");
    } finally {
      setIsSynthesizing(false);
    }
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
              <button
                type="button"
                className="btn lg"
                onClick={handlePreview}
                disabled={isSynthesizing}
              >
                <Icon name="play" size={14} />
                {isSynthesizing ? "Synthesizing..." : "Preview"}
              </button>
              <button
                type="button"
                className="btn p lg"
                onClick={handleAddToTimeline}
                disabled={isSynthesizing}
              >
                Add to timeline
              </button>
              {providerStatus && (
                <span className="mono dim" style={{ fontSize: 13, alignSelf: "center" }}>
                  {providerStatus}
                </span>
              )}
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
