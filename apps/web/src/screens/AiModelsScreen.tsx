import React, { useState } from "react";
import { SettingsLayout, Callout, Icon, Button, Section, Input } from "@flux/ui";
import { ByokAiManager, AiProvider } from "@flux/app-core";

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

  const aiManager = ByokAiManager.get();
  const [providers, setProviders] = useState(() => aiManager.getProviders());
  const [keyInputs, setKeyInputs] = useState<Record<string, string>>({});

  const handleAddKey = (provider: AiProvider) => {
    const key = keyInputs[provider];
    if (key && key.trim().length > 0) {
      aiManager.addKey(provider, key.trim());
      setProviders(aiManager.getProviders());
      setKeyInputs((prev) => ({ ...prev, [provider]: "" }));
    }
  };

  const handleRemoveKey = (provider: AiProvider, index: number) => {
    aiManager.removeKey(provider, index);
    setProviders(aiManager.getProviders());
  };

  return (
    <SettingsLayout currentTab="models" title="AI models & Cloud BYOK">
      <div className="between">
        <span className="dim">
          100% Local by default. Bring Your Own Key for cloud AI models with auto-failover.
        </span>
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

      <Section title="Bring Your Own Key (BYOK) — Cloud AI Providers">
        <p className="dim" style={{ marginBottom: "16px" }}>
          Flux never routes your audio through any cloud server. API calls are sent directly from
          your machine to the provider. Add multiple keys per provider for automatic rate-limit
          (429/401) rotation, with automatic fallback across providers.
        </p>

        <div className="lst box">
          {providers.map((p) => (
            <div key={p.provider} style={{ display: "block", padding: "16px" }}>
              <div className="between" style={{ marginBottom: "8px" }}>
                <div>
                  <b style={{ fontSize: "15px" }}>{p.name}</b>
                  <p className="dim" style={{ margin: "4px 0 0" }}>
                    {p.description}
                  </p>
                </div>
                <span className="mono dim" style={{ fontSize: "12px" }}>
                  {p.keys.length} key{p.keys.length === 1 ? "" : "s"} configured
                </span>
              </div>

              {p.keys.length > 0 && (
                <div style={{ margin: "10px 0" }}>
                  {p.keys.map((k, idx) => (
                    <div
                      key={idx}
                      className="between"
                      style={{
                        padding: "6px 10px",
                        background: "var(--surface)",
                        borderRadius: "6px",
                        marginBottom: "4px",
                      }}
                    >
                      <span className="mono" style={{ fontSize: "13px" }}>
                        {idx === p.activeKeyIndex ? "★ " : "  "}
                        {k.slice(0, 7)}...{k.slice(-4)}
                        {idx === p.activeKeyIndex ? " (Active)" : ""}
                      </span>
                      <button
                        type="button"
                        className="btn g"
                        style={{ padding: "2px 8px", fontSize: "12px" }}
                        onClick={() => handleRemoveKey(p.provider, idx)}
                      >
                        Remove
                      </button>
                    </div>
                  ))}
                </div>
              )}

              <div className="row" style={{ marginTop: "10px", gap: "8px" }}>
                <Input
                  placeholder={`Add API key for ${p.name}`}
                  value={keyInputs[p.provider] || ""}
                  onChange={(e) =>
                    setKeyInputs((prev) => ({ ...prev, [p.provider]: e.target.value }))
                  }
                />
                <Button
                  label="Add Key"
                  variant="primary"
                  onClick={() => handleAddKey(p.provider)}
                />
              </div>
            </div>
          ))}
        </div>
      </Section>

      <Callout icon="chip">
        This device: WebGPU, 8 GB of graphics memory. All models will run.
      </Callout>
    </SettingsLayout>
  );
};
