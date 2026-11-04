import React, { useState } from "react";
import { PageLayout, CanvasViz, Button, SegmentedControl, Callout, Slider, Toggle } from "@flux/ui";
import { ProjectStore, RingtoneMaker, ExportManager } from "@flux/app-core";

interface RingtoneScreenProps {
  onNavigate?: (route: string) => void;
}

export const RingtoneScreen: React.FC<RingtoneScreenProps> = ({ onNavigate }) => {
  const store = ProjectStore.getInstance();
  const [phoneIndex, setPhoneIndex] = useState(0); // 0 = iPhone, 1 = Android
  const [fadeIn, setFadeIn] = useState(18);
  const [fadeOut, setFadeOut] = useState(30);
  const [levelSpeaker, setLevelSpeaker] = useState(true);
  const [loopSmoothly, setLoopSmoothly] = useState(false);
  const [selectionStartSec, setSelectionStartSec] = useState(0);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);

  const navigate = (r: string) => {
    if (onNavigate) {
      onNavigate(r);
    } else {
      window.location.hash = r;
    }
  };

  const handleFindCatchiest = () => {
    const mixdown = store.renderMixdown();
    const section = RingtoneMaker.findCatchiestPart(
      mixdown.interleaved,
      mixdown.sampleRate,
      30.0,
    );
    setSelectionStartSec(section.startTimeSec);
    setStatusMessage(`Found chorus peak energy at ${section.startTimeSec.toFixed(1)}s (score ${section.score.toFixed(0)})`);
  };

  const handlePlaySelection = () => {
    const sr = store.getState().sampleRate;
    const startFrame = Math.floor(selectionStartSec * sr);
    const endFrame = startFrame + Math.floor(30.0 * sr);
    store.setLoop(true, startFrame, endFrame);
    store.seek(startFrame);
    store.play();
  };

  const handleExportRingtone = async () => {
    const sr = store.getState().sampleRate;
    const startFrame = Math.floor(selectionStartSec * sr);
    const endFrame = startFrame + Math.floor(30.0 * sr);
    const mix = store.renderMixdown(startFrame, endFrame);

    let finalSamples = mix.interleaved;
    if (levelSpeaker) {
      finalSamples = RingtoneMaker.applySpeakerLeveling(finalSamples);
    }

    const ext = phoneIndex === 0 ? "m4a" : "wav";
    const exportMgr = new ExportManager();
    const outBytes = await exportMgr.exportAudio(finalSamples, mix.numChannels, sr, {
      format: "wav_16",
    });

    const blob = new Blob([new Uint8Array(outBytes)], { type: "audio/wav" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `flux-ringtone-${phoneIndex === 0 ? "iphone.m4r" : "android.wav"}`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    setTimeout(() => URL.revokeObjectURL(url), 5000);

    navigate("exported");
  };

  const leftPercent = Math.min(80, (selectionStartSec / 60) * 100);

  return (
    <PageLayout
      currentTab="ringtone"
      title="Pick your 30 seconds."
      actions={
        <Button
          label="Export ringtone"
          variant="primary"
          icon="exp"
          dataGo="exported"
          href="#exported"
          onClick={handleExportRingtone}
        />
      }
      onNavigate={navigate}
    >
      <div className="vz" style={{ height: "170px", position: "relative" }}>
        <CanvasViz type="wave" seed={3} stereo={true} step={2} />
        <div
          style={{
            position: "absolute",
            left: `${leftPercent}%`,
            width: "30%",
            top: 0,
            bottom: 0,
            background: "color-mix(in srgb,var(--sig) 26%,transparent)",
            borderInline: "3px solid var(--sig)",
          }}
        />
        <span
          className="mono"
          style={{
            position: "absolute",
            left: `${leftPercent}%`,
            top: "6px",
            paddingLeft: "9px",
          }}
        >
          30.0 s
        </span>
      </div>

      {statusMessage && (
        <Callout icon="sparkle" variant="info">
          {statusMessage}
        </Callout>
      )}

      <div className="row">
        <Button
          label="Play selection"
          icon="play"
          variant="primary"
          large={true}
          onClick={handlePlaySelection}
        />
        <Button
          label="Find the catchiest part"
          icon="mag"
          large={true}
          onClick={handleFindCatchiest}
        />
      </div>

      <div className="g2">
        <div className="box col">
          <h2 className="h2">For your phone</h2>
          <SegmentedControl
            items={["iPhone", "Android"]}
            selectedIndex={phoneIndex}
            onChange={(_item, idx) => setPhoneIndex(idx)}
          />
          <Callout icon="info">
            iPhone ringtones must be 30 seconds or less, in .m4r. On a computer, drag the file to
            your iPhone in Finder or iTunes.
          </Callout>
        </div>

        <div className="box col">
          <h2 className="h2">Sound</h2>
          <Slider label="Fade in" value={fadeIn} valueText="0.3 s" onChange={setFadeIn} />
          <Slider label="Fade out" value={fadeOut} valueText="1.0 s" onChange={setFadeOut} />
          <Toggle
            label="Level it for a phone speaker"
            checked={levelSpeaker}
            onChange={setLevelSpeaker}
          />
          <Toggle label="Loop smoothly" checked={loopSmoothly} onChange={setLoopSmoothly} />
        </div>
      </div>
    </PageLayout>
  );
};
