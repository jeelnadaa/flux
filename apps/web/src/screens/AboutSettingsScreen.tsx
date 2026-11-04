import React, { useState } from "react";
import { SettingsLayout, Button, Icon } from "@flux/ui";

const ABOUT_LINKS = [
  { label: "Attributions and licenses", route: "legal-credits" },
  { label: "Privacy policy", route: "legal" },
  { label: "Terms", route: "legal-terms" },
  { label: "Create a diagnostic report", route: "diag" },
];

interface AboutSettingsScreenProps {
  onNavigate?: (route: string) => void;
}

export const AboutSettingsScreen: React.FC<AboutSettingsScreenProps> = ({ onNavigate }) => {
  const [updateStatus, setUpdateStatus] = useState<string | null>(null);
  const [copyStatus, setCopyStatus] = useState<string | null>(null);

  const navigate = (r: string) => {
    if (onNavigate) {
      onNavigate(r);
    } else {
      window.location.hash = r;
    }
  };

  const handleCheckUpdates = () => {
    setUpdateStatus("Checking...");
    setTimeout(() => {
      setUpdateStatus("You are on the latest release (0.1.0)");
      setTimeout(() => setUpdateStatus(null), 3000);
    }, 800);
  };

  const handleCopyVersion = () => {
    const info = `Flux 0.1.0 build 1 (${typeof navigator !== "undefined" ? navigator.platform : "Desktop"})`;
    navigator.clipboard?.writeText(info);
    setCopyStatus("Copied version info!");
    setTimeout(() => setCopyStatus(null), 2500);
  };

  return (
    <SettingsLayout currentTab="about" title="About">
      <div className="box col">
        <div className="between">
          <b>Flux</b>
          <span className="mono">0.1.0 build 1</span>
        </div>
        <div className="row">
          <Button label={updateStatus || "Check for updates"} onClick={handleCheckUpdates} />
          <Button
            label={copyStatus || "Copy version info"}
            variant="ghost"
            onClick={handleCopyVersion}
          />
        </div>
      </div>

      <div className="lst box">
        {ABOUT_LINKS.map((link) => (
          <a
            key={link.route}
            href={`#${link.route}`}
            data-go={link.route}
            style={{ textDecoration: "none" }}
            onClick={(e) => {
              e.preventDefault();
              navigate(link.route);
            }}
          >
            <div className="ti">
              <b>{link.label}</b>
            </div>
            <Icon name="exp" size={15} />
          </a>
        ))}
      </div>
    </SettingsLayout>
  );
};
