import React, { useState } from "react";
import { ProjectLayout, SegmentedControl, Toggle, Pill, Icon, CanvasViz } from "@flux/ui";
import { ProjectStore } from "@flux/app-core";

interface TranscriptScreenProps {
  onNavigate?: (route: string) => void;
}

const tabItems = ["Edit", "Review fillers", "Speakers"] as const;
type TabItem = (typeof tabItems)[number];

export const TranscriptScreen: React.FC<TranscriptScreenProps> = ({ onNavigate }) => {
  const store = ProjectStore.get();
  const [activeTab, setActiveTab] = useState<TabItem>("Edit");
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [fillRoomTone, setFillRoomTone] = useState<boolean>(true);
  const [smoothPitch, setSmoothPitch] = useState<boolean>(false);

  const [selectedWord, setSelectedWord] = useState<string>('"the the" · 00:12:33.1 · 0.42 s');

  const handleWordClick = (word: string, timeSec: number) => {
    setSelectedWord(`"${word}" · 00:${Math.floor(timeSec / 60).toString().padStart(2, "0")}:${(timeSec % 60).toFixed(1)}`);
    store.seek(Math.floor(timeSec * store.getState().sampleRate));
  };

  const handleExportSubtitles = () => {
    const srtContent = `1
00:00:01,000 --> 00:00:04,500
So, um, the first thing I noticed about the harbor recordings

2
00:00:04,600 --> 00:00:08,200
was how much low rumble came through the hull.

3
00:00:08,500 --> 00:00:12,000
And that was on the first night, before we moved the mics.
`;
    const blob = new Blob([srtContent], { type: "text/plain;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "harbor-lights-transcript.srt";
    a.click();
    URL.revokeObjectURL(url);
  };

  const handleListenJoin = () => {
    store.play();
    setTimeout(() => {
      store.pause();
    }, 2000);
  };

  const mayaWords1: Array<[string, string]> = [
    ["So,", "fl"],
    ["um,", "cut fl"],
    ["the", ""],
    ["first", ""],
    ["thing", ""],
    ["I", ""],
    ["noticed", ""],
    ["about", ""],
    ["the", ""],
    ["harbor", ""],
    ["recordings", ""],
    ["was", ""],
    ["how", ""],
    ["much", ""],
    ["low", "cur"],
    ["rumble", ""],
    ["came", ""],
    ["through", ""],
    ["the", ""],
    ["hull.", ""],
    ["You", ""],
    ["could", ""],
    ["feel", ""],
    ["it", ""],
    ["before", ""],
    ["you", ""],
    ["could", ""],
    ["hear", "low"],
    ["it.", ""],
  ];

  const tomWords: Array<[string, string]> = [
    ["And", ""],
    ["that", ""],
    ["was", ""],
    ["on", ""],
    ["the", "cut"],
    ["the", "cut"],
    ["first", ""],
    ["night,", ""],
    ["you know,", "fl"],
    ["before", ""],
    ["we", ""],
    ["moved", ""],
    ["the", ""],
    ["mics", ""],
    ["up", ""],
    ["to", ""],
    ["the", ""],
    ["Kessler", "low"],
    ["deck.", ""],
  ];

  const mayaWords2: Array<[string, string]> = [
    ["Right.", ""],
    ["Which", ""],
    ["meant", ""],
    ["we", ""],
    ["had", ""],
    ["to", ""],
    ["cut", ""],
    ["around", ""],
    ["the", ""],
    ["engine", ""],
    ["for", ""],
    ["the", ""],
    ["whole", ""],
    ["second", ""],
    ["act.", ""],
  ];

  return (
    <ProjectLayout currentTab="script" onNavigate={onNavigate}>
      <div className="split">
        <div className="a">
          <div className="tlbar">
            <SegmentedControl
              items={tabItems}
              selectedIndex={tabItems.indexOf(activeTab)}
              onChange={(tab) => {
                setActiveTab(tab as TabItem);
                if (tab === "Review fillers") {
                  onNavigate?.("fillers");
                }
              }}
            />
            <span className="sp" style={{ flex: 1 }} />
            <div style={{ width: 210 }} className="hide-s">
              <input
                type="text"
                className="inp"
                placeholder="Find and replace"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
              />
            </div>
            <button type="button" className="btn lbl" onClick={() => onNavigate?.("fillers")}>
              Fillers: 23
            </button>
            <button type="button" className="btn" onClick={handleExportSubtitles}>
              <Icon name="exp" size={14} />
              Subtitles
            </button>
          </div>

          <div
            style={{
              height: 78,
              flex: "none",
              borderBottom: "1px solid var(--ln)",
              position: "relative",
            }}
          >
            <CanvasViz type="wave" seed={5} kind="voice" color="--t1" step={2} />
            <span className="ph" style={{ left: "34%" }} />
          </div>

          <div
            className="in"
            style={{
              padding: "clamp(16px, 3cqw, 34px)",
              display: "flex",
              flexDirection: "column",
              gap: 24,
              flex: 1,
            }}
          >
            <div>
              <div className="spk">
                <span className="dot" style={{ color: "var(--t4)" }} />
                Maya <span className="mono faint">00:12:04</span>
              </div>
              <p className="script">
                {mayaWords1.map(([t, c], idx) => (
                  <span
                    key={idx}
                    className={`wd ${c}`}
                    tabIndex={0}
                    style={{ cursor: "pointer" }}
                    onClick={() => handleWordClick(t, 4 + idx * 0.45)}
                  >
                    {t}{" "}
                  </span>
                ))}
              </p>
            </div>

            <div>
              <div className="spk">
                <span className="dot" style={{ color: "var(--t3)" }} />
                Tom <span className="mono faint">00:12:31</span>
              </div>
              <p className="script">
                {tomWords.map(([t, c], idx) => (
                  <span
                    key={idx}
                    className={`wd ${c}`}
                    tabIndex={0}
                    style={{ cursor: "pointer" }}
                    onClick={() => handleWordClick(t, 31 + idx * 0.45)}
                  >
                    {t}{" "}
                  </span>
                ))}
              </p>
            </div>

            <div>
              <div className="spk">
                <span className="dot" style={{ color: "var(--t4)" }} />
                Maya <span className="mono faint">00:12:48</span>
              </div>
              <p className="script">
                {mayaWords2.map(([t, c], idx) => (
                  <span
                    key={idx}
                    className={`wd ${c}`}
                    tabIndex={0}
                    style={{ cursor: "pointer" }}
                    onClick={() => handleWordClick(t, 48 + idx * 0.45)}
                  >
                    {t}{" "}
                  </span>
                ))}
              </p>
            </div>
          </div>
        </div>

        <aside className="b in">
          <h3 className="h3">Selected text</h3>
          <div className="box mono dim" style={{ padding: 9 }}>
            {selectedWord}
          </div>
          <div className="row">
            <button type="button" className="btn p">
              <Icon name="trash" size={14} />
              Delete
            </button>
            <button type="button" className="btn">
              Restore
            </button>
          </div>

          <div className="sec">
            <h4 className="lab">Join</h4>
            <Toggle label="Fill with room tone" checked={fillRoomTone} onChange={setFillRoomTone} />
            <Toggle
              label="Smooth pitch across the cut"
              checked={smoothPitch}
              onChange={setSmoothPitch}
            />
            <div className="row">
              <button type="button" className="btn" onClick={handleListenJoin}>
                <Icon name="play" size={14} />
                Listen to the join
              </button>
            </div>
          </div>

          <div className="sec">
            <h4 className="lab">Accuracy</h4>
            <div className="between">
              <span>Low-confidence words</span>
              <Pill label="14" variant="warn" />
            </div>
            <span className="dim" style={{ fontSize: 12.5 }}>
              Underlined with dots. Click one to hear it and correct the text.
            </span>
          </div>

          <div className="sec">
            <h4 className="lab">Names and terms</h4>
            <div className="row">
              <button type="button" className="chip">
                Kessler
              </button>
              <button type="button" className="chip">
                Harbor Lights
              </button>
              <button type="button" className="chip">
                + Add
              </button>
            </div>
          </div>

          <div className="sec">
            <h4 className="lab">Transcribed with</h4>
            <div className="between">
              <span>Whisper small, English</span>
              <Pill label="On device" variant="ok" />
            </div>
          </div>
        </aside>
      </div>
    </ProjectLayout>
  );
};
