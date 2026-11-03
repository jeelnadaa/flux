import React, { useState, useEffect } from "react";
import { initThemeAndFont, canViewTransition } from "@flux/ui";
import { KitScreen } from "./screens/KitScreen.js";
import { ThemesScreen } from "./screens/ThemesScreen.js";
import { FontsScreen } from "./screens/FontsScreen.js";
import { GeneralSettingsScreen } from "./screens/GeneralSettingsScreen.js";
import { HomeScreen } from "./screens/HomeScreen.js";
import { WelcomeScreen } from "./screens/WelcomeScreen.js";
import { EditScreen } from "./screens/EditScreen.js";
import { SpectralScreen } from "./screens/SpectralScreen.js";
import { AutomationScreen } from "./screens/AutomationScreen.js";
import { MixScreen } from "./screens/MixScreen.js";
import { HistoryScreen } from "./screens/HistoryScreen.js";
import { RecoveryDialogScreen } from "./screens/RecoveryDialogScreen.js";
import { RelinkDialogScreen } from "./screens/RelinkDialogScreen.js";
import { QuotaDialogScreen } from "./screens/QuotaDialogScreen.js";

export function getScreenComponent(route: string): React.ReactElement {
  switch (route) {
    case "kit":
      return <KitScreen />;
    case "prefs-themes":
      return <ThemesScreen />;
    case "prefs-fonts":
      return <FontsScreen />;
    case "prefs-general":
      return <GeneralSettingsScreen />;
    case "welcome":
      return <WelcomeScreen />;
    case "edit":
      return <EditScreen />;
    case "spectral":
      return <SpectralScreen />;
    case "automation":
      return <AutomationScreen />;
    case "mix":
      return <MixScreen />;
    case "history":
      return <HistoryScreen />;
    case "recovery":
      return <RecoveryDialogScreen />;
    case "relink":
      return <RelinkDialogScreen />;
    case "quota":
      return <QuotaDialogScreen />;
    case "home":
    default:
      return <HomeScreen />;
  }
}

export const App: React.FC = () => {
  const [route, setRoute] = useState(() => {
    if (typeof window !== "undefined") {
      const hash = window.location.hash.replace(/^#/, "");
      return hash || "home";
    }
    return "home";
  });

  useEffect(() => {
    initThemeAndFont();

    const handleHashChange = () => {
      const nextRoute = window.location.hash.replace(/^#/, "") || "home";
      const doc = document as Document & {
        startViewTransition?: (cb: () => void) => { ready: Promise<void>; finished: Promise<void> };
      };
      if (canViewTransition() && doc.startViewTransition) {
        doc.startViewTransition(() => {
          setRoute(nextRoute);
          const m = document.getElementById("m");
          if (m) m.scrollTop = 0;
        });
      } else {
        setRoute(nextRoute);
        const m = document.getElementById("m");
        if (m) m.scrollTop = 0;
      }
    };

    window.addEventListener("hashchange", handleHashChange);
    return () => {
      window.removeEventListener("hashchange", handleHashChange);
    };
  }, []);

  useEffect(() => {
    const m = document.getElementById("m");
    if (m) m.scrollTop = 0;
  }, [route]);

  return (
    <div id="frame">
      <div id="stage">{getScreenComponent(route)}</div>
    </div>
  );
};
