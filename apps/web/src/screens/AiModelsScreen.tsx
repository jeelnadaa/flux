import React, { useState } from "react";
import { SettingsLayout, Callout, Icon } from "@flux/ui";

interface AiModelsScreenProps {
  onNavigate?: (route: string) => void;
}

export const AiModelsScreen: React.FC<AiModelsScreenProps> = () => {
  const [models, setModels] = useState([
    {
      id: "m1",
      title: "Stems: Balanced",
      size: "820 MB",
      downloaded: true,
      verified: true,
    },
    {
      id: "m2",
      title: "Voice cleanup",
      size: "64 MB",
      downloaded: true,
      verified: true,
    },
    {
      id: "m3",
      title: "Transcript: Whisper small",
      size: "240 MB",
      downloaded: true,
      verified: true,
    },
    {
      id: "m4",
      title: "Tempo and key",
      size: "18 MB",
      downloaded: true,
      verified: true,
    },
    {
      id: "m5",
      title: "Stems: Best",
      size: "1.9 GB",
      downloaded: false,
      verified: false,
    },
    {
      id: "m6",
      title: "Transcript: Whisper medium",
      size: "780 MB",
      downloaded: false,
      verified: false,
    },
    {
      id: "m7",
      title: "Speech voices (English)",
      size: "310 MB",
      downloaded: false,
      verified: false,
    },
  ]);

  const toggleDownload = (id: string) => {
    setModels((prev) =>
      prev.map((m) =>
        m.id === id ? { ...m, downloaded: !m.downloaded, verified: !m.downloaded } : m,
      ),
    );
  };

  return (
    <SettingsLayout currentTab="models" title="AI models">
      <div className="between">
        <span className="dim">Downloaded once and then work offline.</span>
        <b className="mono">1.2 GB used</b>
      </div>

      <div className="lst box">
        {models.map((r) => (
          <div key={r.id}>
            <div className="ti">
              <b>{r.title}</b>
              <span>
                {r.size}
                {r.verified ? ", verified" : ""}
              </span>
            </div>
            {r.downloaded ? (
              <button type="button" className="btn g" onClick={() => toggleDownload(r.id)}>
                Remove
              </button>
            ) : (
              <button type="button" className="btn" onClick={() => toggleDownload(r.id)}>
                <Icon name="exp" size={14} />
                Download
              </button>
            )}
          </div>
        ))}
      </div>

      <Callout icon="chip">
        This device: WebGPU, 8 GB of graphics memory. All models will run.
      </Callout>
    </SettingsLayout>
  );
};
