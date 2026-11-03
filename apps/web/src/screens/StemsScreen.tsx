import React, { useState } from "react";
import { ProjectLayout, SegmentedControl, Slider, Pill, Icon, CanvasViz } from "@flux/ui";

interface StemsScreenProps {
  onNavigate?: (route: string) => void;
}

export const StemsScreen: React.FC<StemsScreenProps> = ({ onNavigate }) => {
  const [activePresetIndex, setActivePresetIndex] = useState<number>(0);
  const [vocalsLevel, setVocalsLevel] = useState<number>(78);
  const [drumsLevel, setDrumsLevel] = useState<number>(70);
  const [bassLevel, setBassLevel] = useState<number>(66);
  const [otherLevel, setOtherLevel] = useState<number>(60);

  const [muteVocals, setMuteVocals] = useState<boolean>(false);
  const [soloVocals, setSoloVocals] = useState<boolean>(false);
  const [muteDrums, setMuteDrums] = useState<boolean>(false);
  const [soloDrums, setSoloDrums] = useState<boolean>(false);
  const [muteBass, setMuteBass] = useState<boolean>(false);
  const [soloBass, setSoloBass] = useState<boolean>(false);
  const [muteOther, setMuteOther] = useState<boolean>(false);
  const [soloOther, setSoloOther] = useState<boolean>(false);

  const handlePresetChange = (preset: string, idx: number) => {
    setActivePresetIndex(idx);
    if (preset === "Karaoke") {
      setMuteVocals(true);
      setSoloVocals(false);
      setMuteDrums(false);
      setMuteBass(false);
      setMuteOther(false);
    } else if (preset === "Acapella") {
      setSoloVocals(true);
      setMuteVocals(false);
    } else if (preset === "No drums") {
      setMuteDrums(true);
      setSoloDrums(false);
      setMuteVocals(false);
    } else if (preset === "No bass") {
      setMuteBass(true);
      setSoloBass(false);
      setMuteVocals(false);
    } else {
      setMuteVocals(false);
      setSoloVocals(false);
      setMuteDrums(false);
      setSoloDrums(false);
      setMuteBass(false);
      setSoloBass(false);
      setMuteOther(false);
      setSoloOther(false);
    }
  };

  const stems = [
    {
      name: "Vocals",
      colorToken: "--t4",
      seed: 7,
      kind: "voice" as const,
      isMuted: muteVocals,
      setMuted: setMuteVocals,
      isSoloed: soloVocals,
      setSoloed: setSoloVocals,
    },
    {
      name: "Drums",
      colorToken: "--t3",
      seed: 3,
      kind: "drums" as const,
      isMuted: muteDrums,
      setMuted: setMuteDrums,
      isSoloed: soloDrums,
      setSoloed: setSoloDrums,
    },
    {
      name: "Bass",
      colorToken: "--t2",
      seed: 4,
      kind: "bass" as const,
      isMuted: muteBass,
      setMuted: setMuteBass,
      isSoloed: soloBass,
      setSoloed: setSoloBass,
    },
    {
      name: "Other",
      colorToken: "--t5",
      seed: 5,
      kind: "mix" as const,
      isMuted: muteOther,
      setMuted: setMuteOther,
      isSoloed: soloOther,
      setSoloed: setSoloOther,
    },
  ];

  return (
    <ProjectLayout currentTab="aitools">
      <div className="split">
        <div className="a">
          <div className="tlbar">
            <b style={{ fontSize: 15, fontWeight: 800 }}>Stems</b>
            <SegmentedControl
              items={["Custom", "Karaoke", "Acapella", "No drums", "No bass"]}
              selectedIndex={activePresetIndex}
              onChange={handlePresetChange}
            />
            <span className="sp" style={{ flex: 1 }} />
            <a
              className="btn"
              href="#edit"
              data-go="edit"
              onClick={(e) => {
                e.preventDefault();
                onNavigate?.("edit");
              }}
            >
              <span>Send to timeline</span>
            </a>
          </div>

          <div
            className="in"
            style={{
              padding: 12,
              display: "flex",
              flexDirection: "column",
              gap: 9,
              flex: 1,
            }}
          >
            {stems.map((s) => (
              <div
                key={s.name}
                className="box"
                style={{
                  padding: "9px 11px",
                  flex: 1,
                  minHeight: 88,
                  display: "grid",
                  gridTemplateColumns: "clamp(88px, 18cqw, 180px) minmax(0, 1fr)",
                  gap: 12,
                  alignItems: "center",
                }}
              >
                <div className="col" style={{ gap: 6 }}>
                  <b className="row" style={{ gap: 6, fontWeight: 800, flexWrap: "nowrap" }}>
                    <span className="dot" style={{ color: `var(${s.colorToken})` }} />
                    {s.name}
                  </b>
                  <div className="row" style={{ gap: 4 }}>
                    <button
                      className={`chip ${s.isMuted ? "on" : ""}`}
                      type="button"
                      data-toggle=""
                      onClick={() => s.setMuted(!s.isMuted)}
                    >
                      Mute
                    </button>
                    <button
                      className={`chip hide-s ${s.isSoloed ? "on" : ""}`}
                      type="button"
                      data-toggle=""
                      onClick={() => s.setSoloed(!s.isSoloed)}
                    >
                      Solo
                    </button>
                  </div>
                </div>
                <div style={{ height: "100%", minHeight: 62 }}>
                  <CanvasViz
                    type="wave"
                    seed={s.seed}
                    kind={s.kind}
                    color={s.colorToken}
                    step={2}
                  />
                </div>
              </div>
            ))}
          </div>
        </div>

        <aside className="b in">
          <h3 className="h3">Levels</h3>
          <Slider label="Vocals" value={vocalsLevel} valueText="0.0 dB" onChange={setVocalsLevel} />
          <Slider label="Drums" value={drumsLevel} valueText="−1.0 dB" onChange={setDrumsLevel} />
          <Slider label="Bass" value={bassLevel} valueText="−2.0 dB" onChange={setBassLevel} />
          <Slider label="Other" value={otherLevel} valueText="−3.0 dB" onChange={setOtherLevel} />

          <div className="sec">
            <h4 className="lab">Quality</h4>
            <div className="between">
              <span>Model</span>
              <Pill label="Balanced" />
            </div>
            <div className="between">
              <span>Stems add up to the original</span>
              <Pill label="Yes" variant="ok" />
            </div>
            <button type="button" className="btn">
              Redo at Best
            </button>
          </div>

          <div className="sec">
            <h4 className="lab">Export</h4>
            <a
              className="btn"
              href="#export"
              data-go="export"
              onClick={(e) => {
                e.preventDefault();
                onNavigate?.("export");
              }}
            >
              <Icon name="exp" size={14} />
              <span>Each stem as a file</span>
            </a>
            <a
              className="btn"
              href="#mix"
              data-go="mix"
              onClick={(e) => {
                e.preventDefault();
                onNavigate?.("mix");
              }}
            >
              <Icon name="mix" size={14} />
              <span>Open as a multitrack project</span>
            </a>
          </div>

          <a
            className="btn"
            href="#stage"
            data-go="stage"
            onClick={(e) => {
              e.preventDefault();
              onNavigate?.("stage");
            }}
          >
            <Icon name="orbit" size={14} />
            <span>Spatialize stems</span>
          </a>
        </aside>
      </div>
    </ProjectLayout>
  );
};
