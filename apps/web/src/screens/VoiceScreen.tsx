import React, { useState } from "react";
import {
  ProjectLayout,
  EffectCard,
  Slider,
  SegmentedControl,
  Toggle,
  Section,
  Field,
  Icon,
  Pill,
  Callout,
} from "@flux/ui";

const VOICE_PRESETS = [
  { name: "Deep", subtitle: "−5 st", pal: "nightcore", fx: "" },
  { name: "Chipmunk", subtitle: "+7 st", pal: "stutter", fx: "" },
  { name: "Child", subtitle: "+4 st, formant up", pal: "radio", fx: "" },
  { name: "Elder", subtitle: "Tremor, breath", pal: "lofi", fx: "" },
  { name: "Shift up", subtitle: "Higher voice", pal: "reverse", fx: "" },
  { name: "Shift down", subtitle: "Lower voice", pal: "slowed", fx: "slow" },
  { name: "Whisper", subtitle: "No pitch", pal: "nextdoor", fx: "" },
  { name: "Announcer", subtitle: "Low, close", pal: "cassette", fx: "" },
  { name: "Radio DJ", subtitle: "Compressed", pal: "tapestop", fx: "" },
  { name: "Villain", subtitle: "Dark, slow", pal: "vinyl", fx: "" },
  { name: "Giant", subtitle: "−12 st, huge", pal: "cathedral", fx: "" },
  { name: "Tiny", subtitle: "+12 st", pal: "underwater", fx: "" },
  { name: "Robot", subtitle: "Ring mod, comb", pal: "robot", fx: "bot" },
  { name: "Cyborg", subtitle: "Half machine", pal: "robot", fx: "" },
  { name: "Alien", subtitle: "Warped formants", pal: "underwater", fx: "" },
  { name: "Monster", subtitle: "Growl, sub", pal: "tapestop", fx: "" },
];

const VOCODER_CARRIERS = ["Saw", "Noise", "Chord"];
const MUSICAL_KEYS = ["D", "C", "E", "F", "G", "A"];
const MUSICAL_SCALES = ["Minor", "Major", "Chromatic"];

export const VoiceScreen: React.FC = () => {
  const [selectedChar, setSelectedChar] = useState("Robot");
  const [isLiveActive, setIsLiveActive] = useState(false);

  // Pitch & Formant
  const [pitch, setPitch] = useState(50); // 0 st
  const [formant, setFormant] = useState(50); // 0%

  // Machine controls
  const [ringMod, setRingMod] = useState(40); // 30 Hz
  const [comb, setComb] = useState(55); // 55%
  const [carrierIdx, setCarrierIdx] = useState(0);
  const [bands, setBands] = useState(50); // 24

  // Hard tune
  const [hardTuneOn, setHardTuneOn] = useState(false);
  const [selectedKey, setSelectedKey] = useState("D");
  const [selectedScale, setSelectedScale] = useState("Minor");
  const [retuneSpeed, setRetuneSpeed] = useState(12); // 10 ms

  return (
    <ProjectLayout currentTab="fxbrowser">
      <div className="split">
        <div className="a">
          <div className="pg in" style={{ maxWidth: "none" }}>
            <h1 className="h1">Be someone else.</h1>
            <div className="g4">
              {VOICE_PRESETS.map((p) => (
                <EffectCard
                  key={p.name}
                  palette={p.pal}
                  effect={p.fx}
                  word={p.name}
                  leftMeta={p.name}
                  rightMeta={p.subtitle}
                  onClick={() => setSelectedChar(p.name)}
                />
              ))}
            </div>
            <Callout variant="info">
              These are original characters. Flux doesn't include voices that imitate real people.
            </Callout>
          </div>
        </div>

        <aside className="b in">
          <div className="between">
            <h3 className="h3">{selectedChar}</h3>
            <Pill label="Live" variant="ok" />
          </div>

          <div className="box col">
            <button
              className="btn lg magic"
              type="button"
              data-toggle
              onClick={() => setIsLiveActive(!isLiveActive)}
            >
              <Icon name="mic" size={18} />
              <span>Talk through it</span>
            </button>
            <div className="between mono dim">
              <span>Delay 92 ms</span>
              <span>Use headphones</span>
            </div>
          </div>

          <Slider
            label="Pitch"
            value={pitch}
            valueText={pitch === 50 ? "0 st" : `${Math.round(((pitch - 50) / 50) * 12)} st`}
            onChange={setPitch}
          />
          <Slider
            label="Formant"
            value={formant}
            valueText={formant === 50 ? "0%" : `${Math.round(((formant - 50) / 50) * 50)}%`}
            onChange={setFormant}
          />

          <Section title="Machine">
            <Slider label="Ring mod" value={ringMod} valueText="30 Hz" onChange={setRingMod} />
            <Slider label="Comb" value={comb} valueText="55%" onChange={setComb} />
            <Field label="Vocoder carrier">
              <SegmentedControl
                items={VOCODER_CARRIERS}
                selectedIndex={carrierIdx}
                onChange={(_, idx) => setCarrierIdx(idx)}
              />
            </Field>
            <Slider label="Bands" value={bands} valueText="24" onChange={setBands} />
          </Section>

          <Section title="Hard tune">
            <Toggle label="On" checked={hardTuneOn} onChange={setHardTuneOn} />
            <div className="g2">
              <Field label="Key">
                <select
                  className="sel"
                  value={selectedKey}
                  onChange={(e) => setSelectedKey(e.target.value)}
                >
                  {MUSICAL_KEYS.map((k) => (
                    <option key={k} value={k}>
                      {k}
                    </option>
                  ))}
                </select>
              </Field>
              <Field label="Scale">
                <select
                  className="sel"
                  value={selectedScale}
                  onChange={(e) => setSelectedScale(e.target.value)}
                >
                  {MUSICAL_SCALES.map((s) => (
                    <option key={s} value={s}>
                      {s}
                    </option>
                  ))}
                </select>
              </Field>
            </div>
            <Slider
              label="Retune speed"
              value={retuneSpeed}
              valueText="10 ms"
              onChange={setRetuneSpeed}
            />
          </Section>
        </aside>
      </div>
    </ProjectLayout>
  );
};
