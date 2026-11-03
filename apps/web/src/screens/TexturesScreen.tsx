import React, { useState } from "react";
import { ProjectLayout, EffectCard, Slider, Toggle, Button, Field } from "@flux/ui";

const TEXTURE_EFFECTS = [
  { name: "Vinyl", subtitle: "Crackle", pal: "vinyl", fx: "vinyl" },
  { name: "Cassette", subtitle: "Hiss, wow", pal: "cassette", fx: "tape" },
  { name: "Old radio", subtitle: "AM band", pal: "radio", fx: "radio" },
  { name: "Phone call", subtitle: "Narrow band", pal: "nextdoor", fx: "" },
  { name: "Megaphone", subtitle: "Hard edge", pal: "tapestop", fx: "" },
  { name: "Walkie-talkie", subtitle: "Squelch", pal: "robot", fx: "bot" },
  { name: "Underwater", subtitle: "Depth", pal: "underwater", fx: "water" },
  { name: "Distortion", subtitle: "Drive", pal: "nightcore", fx: "night" },
  { name: "Bitcrusher", subtitle: "Bits", pal: "lofi", fx: "lofi" },
  { name: "Chorus", subtitle: "Depth", pal: "slowed", fx: "slow" },
  { name: "Flanger", subtitle: "Sweep", pal: "reverse", fx: "" },
  { name: "Rotary", subtitle: "Brake", pal: "stutter", fx: "stut" },
];

export const TexturesScreen: React.FC = () => {
  const [selectedTexture, setSelectedTexture] = useState("Vinyl");
  const [enabled, setEnabled] = useState(true);
  const [seed, setSeed] = useState("4821");

  // Vinyl controls
  const [crackle, setCrackle] = useState(46); // 4 /s
  const [pops, setPops] = useState(22); // 0.6 /s
  const [surface, setSurface] = useState(36); // -48 dB
  const [wow, setWow] = useState(28); // 0.5 Hz
  const [flutter, setFlutter] = useState(18); // 6 Hz
  const [warp, setWarp] = useState(10); // 10%
  const [rolloff, setRolloff] = useState(58); // 11 kHz
  const [monoLows, setMonoLows] = useState(40); // 200 Hz

  const randomizeSeed = () => {
    const s = Math.floor(1000 + Math.random() * 9000).toString();
    setSeed(s);
  };

  return (
    <ProjectLayout currentTab="fxbrowser">
      <div className="split">
        <div className="a">
          <div className="pg in" style={{ maxWidth: "none" }}>
            <h1 className="h1">Old machines, on purpose.</h1>
            <div className="g3">
              {TEXTURE_EFFECTS.map((t) => (
                <EffectCard
                  key={t.name}
                  palette={t.pal}
                  effect={t.fx}
                  word={t.name}
                  leftMeta={t.name}
                  rightMeta={t.subtitle}
                  onClick={() => setSelectedTexture(t.name)}
                />
              ))}
            </div>
          </div>
        </div>

        <aside className="b in">
          <div className="between">
            <h3 className="h3">{selectedTexture}</h3>
            <Toggle checked={enabled} onChange={setEnabled} />
          </div>

          <div
            style={{
              aspectRatio: "1",
              maxWidth: "220px",
              margin: "0 auto",
              width: "100%",
              borderRadius: "50%",
              background:
                "repeating-radial-gradient(circle, var(--tx) 0 1.5px, var(--s1) 1.5px 5px)",
              display: "grid",
              placeItems: "center",
              animation: "spinrec 3.6s linear infinite",
            }}
          >
            <span
              style={{
                width: "32%",
                aspectRatio: "1",
                borderRadius: "50%",
                background: "var(--sig)",
              }}
            />
          </div>
          <style>{`@keyframes spinrec { to { rotate: 360deg; } }`}</style>

          <div className="ctl-grid" style={{ gridTemplateColumns: "repeat(2, minmax(0, 1fr))" }}>
            <Slider label="Crackle" value={crackle} valueText="4 /s" onChange={setCrackle} />
            <Slider label="Pops" value={pops} valueText="0.6 /s" onChange={setPops} />
            <Slider label="Surface" value={surface} valueText="−48 dB" onChange={setSurface} />
            <Slider label="Wow" value={wow} valueText="0.5 Hz" onChange={setWow} />
            <Slider label="Flutter" value={flutter} valueText="6 Hz" onChange={setFlutter} />
            <Slider label="Warp" value={warp} valueText="10%" onChange={setWarp} />
            <Slider label="Roll-off" value={rolloff} valueText="11 kHz" onChange={setRolloff} />
            <Slider label="Mono lows" value={monoLows} valueText="200 Hz" onChange={setMonoLows} />
          </div>

          <Field label="Seed">
            <div className="row" style={{ flexWrap: "nowrap" }}>
              <input className="inp" value={seed} onChange={(e) => setSeed(e.target.value)} />
              <Button variant="ghost" onClick={randomizeSeed}>
                Randomize
              </Button>
            </div>
          </Field>
          <span className="dim" style={{ fontSize: "12.5px" }}>
            The same seed always gives the same crackle, so exports match the preview.
          </span>
        </aside>
      </div>
    </ProjectLayout>
  );
};
