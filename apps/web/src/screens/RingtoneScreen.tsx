import React, { useState } from "react";
import { PageLayout, CanvasViz, Button, SegmentedControl, Callout, Slider, Toggle } from "@flux/ui";

interface RingtoneScreenProps {
  onNavigate?: (route: string) => void;
}

export const RingtoneScreen: React.FC<RingtoneScreenProps> = ({ onNavigate }) => {
  const [phoneIndex, setPhoneIndex] = useState(0); // 0 = iPhone, 1 = Android
  const [fadeIn, setFadeIn] = useState(18);
  const [fadeOut, setFadeOut] = useState(30);
  const [levelSpeaker, setLevelSpeaker] = useState(true);
  const [loopSmoothly, setLoopSmoothly] = useState(false);

  const navigate = (r: string) => {
    if (onNavigate) {
      onNavigate(r);
    } else {
      window.location.hash = r;
    }
  };

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
          onClick={() => navigate("exported")}
        />
      }
      onNavigate={navigate}
    >
      <div className="vz" style={{ height: "170px", position: "relative" }}>
        <CanvasViz type="wave" seed={3} stereo={true} step={2} />
        <div
          style={{
            position: "absolute",
            left: "32%",
            width: "18%",
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
            left: "32%",
            top: "6px",
            paddingLeft: "9px",
          }}
        >
          30.0 s
        </span>
      </div>

      <div className="row">
        <Button label="Play selection" icon="play" variant="primary" large={true} />
        <Button label="Find the catchiest part" icon="mag" large={true} />
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
