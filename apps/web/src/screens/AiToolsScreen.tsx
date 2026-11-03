import React from "react";
import { ProjectLayout, EffectCard, Icon } from "@flux/ui";

interface AiToolsScreenProps {
  onNavigate?: (route: string) => void;
}

export const AiToolsScreen: React.FC<AiToolsScreenProps> = ({ onNavigate }) => {
  const tools = [
    {
      palette: "tapestop",
      title: "Magic",
      desc: "Find and fix problems",
      route: "magic",
      tag: "Device",
    },
    {
      palette: "stutter",
      title: "Stems",
      desc: "Vocals, drums, bass, other",
      route: "stems-setup",
      tag: "Device",
    },
    {
      palette: "underwater",
      title: "Cleanup",
      desc: "Noise, echo, studio sound",
      route: "cleanup",
      tag: "Device",
    },
    {
      palette: "reverse",
      title: "Script",
      desc: "Edit audio by editing text",
      route: "script",
      tag: "Device",
    },
    {
      palette: "radio",
      title: "Fillers",
      desc: "Um, uh, stutters",
      route: "fillers",
      tag: "Device",
    },
    {
      palette: "slowed",
      title: "Tempo",
      desc: "BPM, beats, key",
      route: "analysis",
      tag: "Device",
    },
    {
      palette: "vinyl",
      title: "Master",
      desc: "One tap to finished",
      route: "master",
      tag: "Device",
    },
    {
      palette: "lofi",
      title: "Speech",
      desc: "Write it, hear it",
      route: "tts",
      tag: "Device or cloud",
    },
    {
      palette: "nightcore",
      title: "Dub",
      desc: "New language, same mix",
      route: "dub",
      tag: "Cloud",
    },
    {
      palette: "robot",
      title: "Clone",
      desc: "Yours, with consent only",
      route: "clone",
      tag: "Off",
    },
  ];

  return (
    <ProjectLayout currentTab="aitools">
      <div className="pg in">
        <div className="pgh">
          <div>
            <h1 className="h1">AI that stays on your device.</h1>
            <p className="lede">
              Each tool shows where it runs before it starts. Results are saved with your project
              and can always be turned off.
            </p>
          </div>
          <a
            className="btn"
            href="#jobs"
            data-go="jobs"
            onClick={(e) => {
              e.preventDefault();
              onNavigate?.("jobs");
            }}
          >
            <Icon name="list" size={14} />
            <span>Jobs</span>
          </a>
        </div>

        <div className="g4">
          {tools.map((t) => (
            <EffectCard
              key={t.palette}
              palette={t.palette}
              effect=""
              word={t.title}
              leftMeta={t.desc}
              rightMeta={t.tag}
              dataGo={t.route}
              href={`#${t.route}`}
              onClick={(e) => {
                e.preventDefault();
                onNavigate?.(t.route);
              }}
            />
          ))}
        </div>
      </div>
    </ProjectLayout>
  );
};
