import React, { useState, useEffect, useRef } from "react";
import {
  initThemeAndFont,
  canViewTransition,
  hydrateKineticTypography,
  observeKineticTypography,
} from "@flux/ui";
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
import { ShortcutsDialogScreen } from "./screens/ShortcutsDialogScreen.js";
import { CommandPaletteDialogScreen } from "./screens/CommandPaletteDialogScreen.js";
import { KeyboardSettingsScreen } from "./screens/KeyboardSettingsScreen.js";
import { LibraryScreen } from "./screens/LibraryScreen.js";
import { EmptyProjectScreen } from "./screens/EmptyProjectScreen.js";
import { EffectRackScreen } from "./screens/EffectRackScreen.js";
import { EffectBrowserScreen } from "./screens/EffectBrowserScreen.js";
import { EqualizerScreen } from "./screens/EqualizerScreen.js";
import { DynamicsScreen } from "./screens/DynamicsScreen.js";
import { SpeedPitchScreen } from "./screens/SpeedPitchScreen.js";
import { CleanUpScreen } from "./screens/CleanUpScreen.js";
import { SilenceRemoverScreen } from "./screens/SilenceRemoverScreen.js";
import { MetersScreen } from "./screens/MetersScreen.js";
import { PresetsScreen } from "./screens/PresetsScreen.js";
import { HrtfScreen } from "./screens/HrtfScreen.js";
import { HpCheckScreen } from "./screens/HpCheckScreen.js";
import { StageScreen } from "./screens/StageScreen.js";
import { RoomsScreen } from "./screens/RoomsScreen.js";
import { RoomDesignScreen } from "./screens/RoomDesignScreen.js";
import { HallScreen } from "./screens/HallScreen.js";
import { CreativeScreen } from "./screens/CreativeScreen.js";
import { TexturesScreen } from "./screens/TexturesScreen.js";
import { GlitchScreen } from "./screens/GlitchScreen.js";
import { VoiceScreen } from "./screens/VoiceScreen.js";
import { AiToolsScreen } from "./screens/AiToolsScreen.js";
import { StemsSetupScreen } from "./screens/StemsSetupScreen.js";
import { StemsRunScreen } from "./screens/StemsRunScreen.js";
import { StemsScreen } from "./screens/StemsScreen.js";
import { VoiceCleanupScreen } from "./screens/VoiceCleanupScreen.js";
import { TranscriptScreen } from "./screens/TranscriptScreen.js";
import { FillerReviewScreen } from "./screens/FillerReviewScreen.js";
import { AiJobsScreen } from "./screens/AiJobsScreen.js";
import { CloudConsentDialogScreen } from "./screens/CloudConsentDialogScreen.js";
import { DubScreen } from "./screens/DubScreen.js";
import { AiModelsScreen } from "./screens/AiModelsScreen.js";

export function getScreenComponent(
  route: string,
  onNavigate?: (r: string) => void,
): React.ReactElement {
  const navigate =
    onNavigate ||
    ((r: string) => {
      window.location.hash = r;
    });

  switch (route) {
    case "aitools":
      return <AiToolsScreen onNavigate={navigate} />;
    case "stems-setup":
      return <StemsSetupScreen onNavigate={navigate} />;
    case "stems-run":
      return <StemsRunScreen onNavigate={navigate} />;
    case "stems":
      return <StemsScreen onNavigate={navigate} />;
    case "cleanup":
      return <VoiceCleanupScreen onNavigate={navigate} />;
    case "script":
      return <TranscriptScreen onNavigate={navigate} />;
    case "fillers":
      return <FillerReviewScreen onNavigate={navigate} />;
    case "jobs":
      return <AiJobsScreen onNavigate={navigate} />;
    case "dub":
      return <DubScreen onNavigate={navigate} />;
    case "consent":
      return <CloudConsentDialogScreen onNavigate={navigate} />;
    case "prefs-models":
      return <AiModelsScreen onNavigate={navigate} />;
    case "kit":
      return <KitScreen />;
    case "prefs-themes":
      return <ThemesScreen />;
    case "prefs-fonts":
      return <FontsScreen />;
    case "prefs-general":
      return <GeneralSettingsScreen />;
    case "prefs-keys":
      return <KeyboardSettingsScreen />;
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
    case "shortcuts":
      return <ShortcutsDialogScreen />;
    case "palette":
      return <CommandPaletteDialogScreen />;
    case "library":
      return <LibraryScreen />;
    case "empty":
      return <EmptyProjectScreen />;
    case "rack":
      return <EffectRackScreen />;
    case "fxbrowser":
      return <EffectBrowserScreen />;
    case "fx-eq":
      return <EqualizerScreen />;
    case "fx-comp":
      return <DynamicsScreen />;
    case "fx-time":
      return <SpeedPitchScreen />;
    case "fx-clean":
      return <CleanUpScreen />;
    case "fx-silence":
      return <SilenceRemoverScreen />;
    case "meters":
      return <MetersScreen />;
    case "presets":
      return <PresetsScreen />;
    case "hrtf":
      return <HrtfScreen />;
    case "hpcheck":
      return <HpCheckScreen />;
    case "stage":
      return <StageScreen />;
    case "rooms":
      return <RoomsScreen />;
    case "roomdesign":
      return <RoomDesignScreen />;
    case "hall":
      return <HallScreen />;
    case "creative":
      return <CreativeScreen />;
    case "textures":
      return <TexturesScreen />;
    case "glitch":
      return <GlitchScreen />;
    case "voice":
      return <VoiceScreen />;
    case "home":
    default:
      return <HomeScreen />;
  }
}

export const App: React.FC = () => {
  const stageRef = useRef<HTMLDivElement>(null);
  const [route, setRoute] = useState(() => {
    if (typeof window !== "undefined") {
      const hash = window.location.hash.replace(/^#/, "");
      return hash || "home";
    }
    return "home";
  });

  useEffect(() => {
    initThemeAndFont();

    if (stageRef.current) {
      const cleanup = observeKineticTypography(stageRef.current);
      return cleanup;
    }
  }, []);

  useEffect(() => {
    if (stageRef.current) {
      hydrateKineticTypography(stageRef.current);
    }
    const m = document.getElementById("m");
    if (m) m.scrollTop = 0;
  }, [route]);

  useEffect(() => {
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

  return (
    <div id="frame">
      <div id="stage" ref={stageRef}>
        {getScreenComponent(route)}
      </div>
    </div>
  );
};
