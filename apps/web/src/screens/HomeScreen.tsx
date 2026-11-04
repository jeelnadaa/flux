import React, { useRef, useEffect } from "react";
import { PageLayout, Icon, EffectCard, CanvasViz } from "@flux/ui";
import { ProjectStore } from "@flux/app-core";

const RECENT_PROJECTS = [
  { name: "Harbor Lights", meta: "Music · 3:42 · 6 tracks", seed: 1, route: "edit" },
  { name: "Interview ep. 12", meta: "Podcast · 48:10 · 2 speakers", seed: 5, route: "edit" },
  { name: "Sunset 8D", meta: "Spatial · 4:18", seed: 8, route: "edit" },
  { name: "Rain voiceover", meta: "Voice · 1:06", seed: 2, route: "edit" },
] as const;

const TASKS = [
  {
    palette: "vinyl",
    effect: "vinyl",
    word: "8D",
    left: "Make it 8D",
    right: "Headphones",
    go: "stage",
  },
  {
    palette: "slowed",
    effect: "slow",
    word: "Slowed",
    left: "Slowed + reverb",
    right: "0.85×",
    go: "task-slowed",
  },
  {
    palette: "stutter",
    effect: "stut",
    word: "Vocals",
    left: "Remove vocals",
    right: "Stems",
    go: "stems-setup",
  },
  {
    palette: "underwater",
    effect: "water",
    word: "Clean",
    left: "Clean up a voice",
    right: "Noise, echo",
    go: "cleanup",
  },
  {
    palette: "reverse",
    effect: "",
    word: "Text",
    left: "Edit by text",
    right: "Transcript",
    go: "script",
  },
  {
    palette: "tapestop",
    effect: "",
    word: "Magic",
    left: "Make it better",
    right: "Find and fix",
    go: "magic",
  },
  {
    palette: "radio",
    effect: "radio",
    word: "Convert",
    left: "Convert a file",
    right: "MP3, FLAC",
    go: "convert",
  },
  {
    palette: "nightcore",
    effect: "night",
    word: "Ring",
    left: "Make a ringtone",
    right: "30 s",
    go: "ringtone",
  },
] as const;

export const HomeScreen: React.FC = () => {
  const fileInputRef = useRef<HTMLInputElement>(null);

  const processFile = async (file: File) => {
    try {
      await ProjectStore.get().importAudioFile(file);
      window.location.hash = "#edit";
    } catch (err) {
      console.error("Failed to decode audio file:", err);
      window.location.hash = "#edit";
    }
  };

  useEffect(() => {
    const handlePaste = (e: ClipboardEvent) => {
      if (e.clipboardData && e.clipboardData.files.length > 0) {
        const file = e.clipboardData.files[0];
        void processFile(file);
      }
    };
    window.addEventListener("paste", handlePaste);
    return () => window.removeEventListener("paste", handlePaste);
  }, []);

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    if (e.dataTransfer.files.length > 0) {
      const file = e.dataTransfer.files[0];
      void processFile(file);
    }
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
  };

  const handleChooseFile = (e: React.MouseEvent) => {
    e.preventDefault();
    fileInputRef.current?.click();
  };

  return (
    <PageLayout currentTab="home" title="What are we doing to it?">
      <input
        ref={fileInputRef}
        type="file"
        accept="audio/*,video/*"
        style={{ display: "none" }}
        onChange={(e) => {
          if (e.target.files && e.target.files.length > 0) {
            void processFile(e.target.files[0]);
          }
        }}
      />
      <a
        className="drop"
        href="#import"
        data-go="import"
        onDrop={handleDrop}
        onDragOver={handleDragOver}
        onClick={handleChooseFile}
      >
        <span className="act">
          <Icon name="imp" size={30} />
        </span>
        <b style={{ fontSize: "19px", fontWeight: 800 }}>Drop audio or video here</b>
        <span className="dim">
          or choose a file, paste audio, or{" "}
          <span
            className="act"
            style={{ cursor: "pointer" }}
            onClick={(e) => {
              e.stopPropagation();
              e.preventDefault();
              window.location.hash = "#record";
            }}
          >
            start recording
          </span>
          . Everything stays on this device.
        </span>
      </a>

      <div className="g4">
        {TASKS.map((t) => (
          <EffectCard
            key={t.left}
            palette={t.palette}
            effect={t.effect}
            word={t.word}
            leftMeta={t.left}
            rightMeta={t.right}
            dataGo={t.go}
            href={`#${t.go}`}
          />
        ))}
      </div>

      <div className="between">
        <h2 className="h2">Recent</h2>
        <a className="dim" href="#library" data-go="library">
          All projects
        </a>
      </div>

      <div className="g4">
        {RECENT_PROJECTS.map((r) => (
          <a key={r.name} className="tile" href={`#${r.route}`} data-go={r.route}>
            <div className="thumb">
              <CanvasViz type="wave" seed={r.seed} step={3} />
            </div>
            <b>{r.name}</b>
            <span>{r.meta}</span>
          </a>
        ))}
      </div>
    </PageLayout>
  );
};
