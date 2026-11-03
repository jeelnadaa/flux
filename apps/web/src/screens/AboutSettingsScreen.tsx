import React from "react";
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
  const navigate = (r: string) => {
    if (onNavigate) {
      onNavigate(r);
    } else {
      window.location.hash = r;
    }
  };

  return (
    <SettingsLayout currentTab="about" title="About">
      <div className="box col">
        <div className="between">
          <b>Flux</b>
          <span className="mono">0.1.0 build 1</span>
        </div>
        <div className="row">
          <Button label="Check for updates" />
          <Button label="Copy version info" variant="ghost" />
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
