import React, { useState } from "react";
import { DubScreen } from "./DubScreen.js";
import { Dialog, Toggle, Icon, type IconName } from "@flux/ui";

interface CloudConsentDialogScreenProps {
  onNavigate?: (route: string) => void;
}

export const CloudConsentDialogScreen: React.FC<CloudConsentDialogScreenProps> = ({
  onNavigate,
}) => {
  const [rememberConsent, setRememberConsent] = useState<boolean>(false);

  const consentPoints: Array<{ icon: IconName; title: string; desc: string }> = [
    {
      icon: "exp",
      title: "What is sent",
      desc: "Harbor Lights mix, 3:42, 38 MB",
    },
    {
      icon: "lock",
      title: "How",
      desc: "Encrypted in transit, only for this job",
    },
    {
      icon: "trash",
      title: "Deleted",
      desc: "Automatically within 24 hours, or now from Jobs",
    },
    {
      icon: "bug",
      title: "Never",
      desc: "Not used for training. Not shared. Not logged",
    },
  ];

  return (
    <>
      <DubScreen onNavigate={onNavigate} />

      <Dialog
        title="Use the cloud for this job?"
        baseScreenId="dub"
        onClose={() => onNavigate?.("dub")}
        footer={
          <>
            <button type="button" className="btn g" onClick={() => onNavigate?.("dub")}>
              Keep it on this device
            </button>
            <button type="button" className="btn p" onClick={() => onNavigate?.("jobs")}>
              Upload and continue
            </button>
          </>
        }
      >
        <p className="dim" style={{ fontSize: 15 }}>
          Dubbing voices need more power than this device has. Here's exactly what happens.
        </p>

        <div className="lst">
          {consentPoints.map((r) => (
            <div key={r.title}>
              <span className="act">
                <Icon name={r.icon} size={18} />
              </span>
              <div className="ti">
                <b>{r.title}</b>
                <span style={{ whiteSpace: "normal" }}>{r.desc}</span>
              </div>
            </div>
          ))}
        </div>

        <Toggle
          label="Remember for dubbing"
          sub="You can change this in Settings, under Privacy"
          checked={rememberConsent}
          onChange={setRememberConsent}
        />
      </Dialog>
    </>
  );
};
