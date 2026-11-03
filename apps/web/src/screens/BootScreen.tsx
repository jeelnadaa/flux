import React from "react";
import { WizardLayout, Icon, Pill, Button } from "@flux/ui";

const CAPABILITIES = [
  {
    name: "Audio engine",
    desc: "AudioWorklet and WebAssembly",
    status: "Ready",
    variant: "ok" as const,
  },
  {
    name: "Low-latency mode",
    desc: "Shared memory is on",
    status: "Ready",
    variant: "ok" as const,
  },
  {
    name: "On-device AI",
    desc: "WebGPU found: stems run at about 0.8× real time",
    status: "Fast",
    variant: "ok" as const,
  },
  {
    name: "Saving to folders",
    desc: "File System Access available",
    status: "Ready",
    variant: "ok" as const,
  },
  {
    name: "AAC export",
    desc: "Encoder available in this browser",
    status: "Ready",
    variant: "ok" as const,
  },
  { name: "Offline use", desc: "Installing for offline", status: "64%", variant: "act" as const },
];

interface BootScreenProps {
  onNavigate?: (route: string) => void;
}

export const BootScreen: React.FC<BootScreenProps> = ({ onNavigate }) => {
  const navigate = (r: string) => {
    if (onNavigate) {
      onNavigate(r);
    } else {
      window.location.hash = r;
    }
  };

  return (
    <WizardLayout step={0} totalSteps={0}>
      <h1 className="h1">Checking what this browser can do.</h1>
      <div className="lst box">
        {CAPABILITIES.map((c) => {
          const isOk = c.variant === "ok";
          return (
            <div key={c.name}>
              <span className={isOk ? "ok" : "act"}>
                <Icon name={isOk ? "check" : "exp"} size={18} />
              </span>
              <div className="ti">
                <b>{c.name}</b>
                <span style={{ whiteSpace: "normal" }}>{c.desc}</span>
              </div>
              <Pill label={c.status} variant={isOk ? "ok" : undefined} />
            </div>
          );
        })}
      </div>
      <div className="between">
        <span className="dim">You can see this again in Settings, under Audio.</span>
        <Button
          label="Continue"
          variant="primary"
          large={true}
          dataGo="welcome"
          href="#welcome"
          onClick={() => navigate("welcome")}
        />
      </div>
    </WizardLayout>
  );
};
