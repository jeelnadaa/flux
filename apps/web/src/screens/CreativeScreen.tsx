import React, { useState } from "react";
import { ProjectLayout, Slider, Toggle, Button } from "@flux/ui";

const LOFI_CONTROLS = [
  { name: "Low-pass", val: 62, text: "6.4 kHz" },
  { name: "High-pass", val: 18, text: "90 Hz" },
  { name: "Drive", val: 44, text: "+4 dB" },
  { name: "Bit depth", val: 55, text: "12 bit" },
  { name: "Sample rate", val: 40, text: "22 kHz" },
  { name: "Crackle", val: 46, text: "4 /s" },
  { name: "Surface", val: 36, text: "−48 dB" },
  { name: "Wow", val: 32, text: "0.6 Hz" },
  { name: "Flutter", val: 24, text: "7.5 Hz" },
  { name: "Pump", val: 35, text: "35%" },
  { name: "Room", val: 30, text: "1.1 s" },
  { name: "Noise", val: 22, text: "−54 dB" },
];

export const CreativeScreen: React.FC = () => {
  // Slowed
  const [slowedSpeed, setSlowedSpeed] = useState(40); // 0.85x
  const [slowedReverb, setSlowedReverb] = useState(62); // Hall 3.2 s
  const [slowedBrightness, setSlowedBrightness] = useState(36); // -3 dB
  const [slowedKeepPitch, setSlowedKeepPitch] = useState(false);

  // Nightcore
  const [nightcoreSpeed, setNightcoreSpeed] = useState(54); // 1.25x
  const [nightcoreTreble, setNightcoreTreble] = useState(40); // +2 dB
  const [nightcoreChorus, setNightcoreChorus] = useState(18); // 18%
  const [nightcoreKeepPitch, setNightcoreKeepPitch] = useState(false);

  // Lo-Fi
  const [lofiAmount, setLofiAmount] = useState(64); // 64%

  return (
    <ProjectLayout currentTab="fxbrowser">
      <div className="pg in">
        <h1 className="h1">Change what it is.</h1>
        <div className="g3">
          {/* Slowed + Reverb */}
          <div className="box col pal-slowed" style={{ gap: "13px" }}>
            <h2
              className="h2"
              style={{
                letterSpacing: ".01em",
                fontWeight: 300,
                fontSize: "clamp(30px, 4cqw, 52px)",
              }}
            >
              Slowed + reverb
            </h2>
            <Slider
              label="Speed"
              value={slowedSpeed}
              valueText={
                slowedSpeed === 40 ? "0.85×" : `${(0.6 + (slowedSpeed / 100) * 0.4).toFixed(2)}×`
              }
              onChange={setSlowedSpeed}
            />
            <Slider
              label="Reverb"
              value={slowedReverb}
              valueText={
                slowedReverb === 62
                  ? "Hall 3.2 s"
                  : `Hall ${(1.0 + (slowedReverb / 100) * 4.0).toFixed(1)} s`
              }
              onChange={setSlowedReverb}
            />
            <Slider
              label="Brightness"
              value={slowedBrightness}
              valueText={
                slowedBrightness === 36
                  ? "−3 dB"
                  : `${Math.round(-12 + (slowedBrightness / 100) * 15)} dB`
              }
              onChange={setSlowedBrightness}
            />
            <Toggle label="Keep pitch" checked={slowedKeepPitch} onChange={setSlowedKeepPitch} />
            <Button variant="primary" onClick={() => (window.location.hash = "#rack")}>
              Add to chain
            </Button>
          </div>

          {/* Nightcore */}
          <div className="box col pal-nightcore" style={{ gap: "13px" }}>
            <h2
              className="h2"
              style={{
                transform: "skewX(-12deg)",
                transformOrigin: "left",
                fontSize: "clamp(30px, 4cqw, 52px)",
              }}
            >
              Nightcore
            </h2>
            <Slider
              label="Speed"
              value={nightcoreSpeed}
              valueText={
                nightcoreSpeed === 54
                  ? "1.25×"
                  : `${(1.0 + (nightcoreSpeed / 100) * 0.5).toFixed(2)}×`
              }
              onChange={setNightcoreSpeed}
            />
            <Slider
              label="Treble lift"
              value={nightcoreTreble}
              valueText={
                nightcoreTreble === 40 ? "+2 dB" : `+${Math.round((nightcoreTreble / 100) * 6)} dB`
              }
              onChange={setNightcoreTreble}
            />
            <Slider
              label="Chorus"
              value={nightcoreChorus}
              valueText={`${nightcoreChorus}%`}
              onChange={setNightcoreChorus}
            />
            <Toggle
              label="Keep pitch"
              checked={nightcoreKeepPitch}
              onChange={setNightcoreKeepPitch}
            />
            <Button onClick={() => (window.location.hash = "#rack")}>Add to chain</Button>
          </div>

          {/* Lo-Fi */}
          <div className="box col pal-lofi" style={{ gap: "13px" }}>
            <h2
              className="h2"
              style={{
                filter: "url(#crush)",
                fontSize: "clamp(30px, 4cqw, 52px)",
              }}
            >
              Lo-fi
            </h2>
            <Slider
              label="Amount"
              value={lofiAmount}
              valueText={`${lofiAmount}%`}
              onChange={setLofiAmount}
              big={true}
            />
            <details>
              <summary className="b7" style={{ cursor: "pointer", padding: "6px 0" }}>
                All 12 controls
              </summary>
              <div
                className="ctl-grid"
                style={{
                  gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
                  marginTop: "8px",
                }}
              >
                {LOFI_CONTROLS.map((c) => (
                  <Slider key={c.name} label={c.name} value={c.val} valueText={c.text} />
                ))}
              </div>
            </details>
            <Button onClick={() => (window.location.hash = "#rack")}>Add to chain</Button>
          </div>
        </div>
      </div>
    </ProjectLayout>
  );
};
