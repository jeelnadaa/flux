import React from "react";
import { TopBar, Rail, HOME_NAV_ITEMS, Button } from "@flux/ui";

export const HELP_TABS: readonly [string, string][] = [
  ["help", "Guides"],
  ["trouble", "Troubleshooting"],
  ["legal", "Privacy"],
  ["legal-terms", "Terms"],
  ["legal-credits", "Attributions"],
];

interface HelpLayoutProps {
  currentTab: "help" | "trouble" | "legal" | "legal-terms" | "legal-credits";
  title: string;
  children: React.ReactNode;
  onNavigate?: (route: string) => void;
}

export const HelpLayout: React.FC<HelpLayoutProps> = ({
  currentTab,
  title,
  children,
  onNavigate,
}) => {
  const navigate = (r: string) => {
    if (onNavigate) {
      onNavigate(r);
    } else {
      window.location.hash = r;
    }
  };

  return (
    <div className="app notp">
      <TopBar
        projectName="Help"
        projectStatus="Works offline"
        rightActions={
          <Button
            variant="ghost"
            label="Close"
            dataGo="home"
            href="#home"
            onClick={(e) => {
              e.preventDefault();
              navigate("home");
            }}
          />
        }
      />
      <Rail items={HOME_NAV_ITEMS} current="help" />
      <main className="main" id="m">
        <div className="tabs">
          {HELP_TABS.map(([tabId, tabLabel]) => (
            <a
              key={tabId}
              href={`#${tabId}`}
              data-go={tabId}
              className={tabId === currentTab ? "on" : ""}
              onClick={(e) => {
                e.preventDefault();
                navigate(tabId);
              }}
            >
              {tabLabel}
            </a>
          ))}
        </div>
        <div className="pg in">
          <h1 className="h1">{title}</h1>
          {children}
        </div>
      </main>
    </div>
  );
};
