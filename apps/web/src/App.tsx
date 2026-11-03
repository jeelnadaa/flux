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
import { TempoKeyScreen } from "./screens/TempoKeyScreen.js";
import { AutoMasterScreen } from "./screens/AutoMasterScreen.js";
import { TextToSpeechScreen } from "./screens/TextToSpeechScreen.js";
import { DubReviewScreen } from "./screens/DubReviewScreen.js";
import { VoiceCloneConsentScreen } from "./screens/VoiceCloneConsentScreen.js";
import { MagicAssistantScreen } from "./screens/MagicAssistantScreen.js";
import { TaskViewScreen } from "./screens/TaskViewScreen.js";
import { NewProjectScreen } from "./screens/NewProjectScreen.js";
import { FormatConverterScreen } from "./screens/FormatConverterScreen.js";
import { BatchScreen } from "./screens/BatchScreen.js";
import { RingtoneScreen } from "./screens/RingtoneScreen.js";
import { MacrosScreen } from "./screens/MacrosScreen.js";
import { AudioSettingsScreen } from "./screens/AudioSettingsScreen.js";
import { PrivacySettingsScreen } from "./screens/PrivacySettingsScreen.js";
import { StorageSettingsScreen } from "./screens/StorageSettingsScreen.js";
import { AccessibilitySettingsScreen } from "./screens/AccessibilitySettingsScreen.js";
import { AccountSettingsScreen } from "./screens/AccountSettingsScreen.js";
import { AboutSettingsScreen } from "./screens/AboutSettingsScreen.js";
import { HelpScreen } from "./screens/HelpScreen.js";
import { TroubleshootingScreen } from "./screens/TroubleshootingScreen.js";
import { DiagnosticDialogScreen } from "./screens/DiagnosticDialogScreen.js";
import { LegalPrivacyScreen } from "./screens/LegalPrivacyScreen.js";
import { LegalTermsScreen } from "./screens/LegalTermsScreen.js";
import { LegalCreditsScreen } from "./screens/LegalCreditsScreen.js";
import { ExportDialogScreen } from "./screens/ExportDialogScreen.js";
import { RecordScreen } from "./screens/RecordScreen.js";
import { BootScreen } from "./screens/BootScreen.js";
import { StorageAskScreen } from "./screens/StorageAskScreen.js";

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
    case "dubreview":
      return <DubReviewScreen onNavigate={navigate} />;
    case "clone":
      return <VoiceCloneConsentScreen onNavigate={navigate} />;
    case "magic":
      return <MagicAssistantScreen onNavigate={navigate} />;
    case "analysis":
      return <TempoKeyScreen onNavigate={navigate} />;
    case "master":
      return <AutoMasterScreen onNavigate={navigate} />;
    case "tts":
      return <TextToSpeechScreen onNavigate={navigate} />;
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
    case "prefs-audio":
      return <AudioSettingsScreen onNavigate={navigate} />;
    case "prefs-privacy":
      return <PrivacySettingsScreen />;
    case "prefs-storage":
      return <StorageSettingsScreen />;
    case "prefs-access":
      return <AccessibilitySettingsScreen onNavigate={navigate} />;
    case "prefs-account":
      return <AccountSettingsScreen />;
    case "prefs-about":
      return <AboutSettingsScreen onNavigate={navigate} />;
    case "task":
      return <TaskViewScreen mode="8d" onNavigate={navigate} />;
    case "task-slowed":
      return <TaskViewScreen mode="slowed" onNavigate={navigate} />;
    case "new":
      return <NewProjectScreen onNavigate={navigate} />;
    case "convert":
      return <FormatConverterScreen />;
    case "batch":
      return <BatchScreen onNavigate={navigate} />;
    case "ringtone":
      return <RingtoneScreen onNavigate={navigate} />;
    case "macros":
      return <MacrosScreen />;
    case "help":
      return <HelpScreen onNavigate={navigate} />;
    case "trouble":
      return <TroubleshootingScreen onNavigate={navigate} />;
    case "diag":
      return <DiagnosticDialogScreen onNavigate={navigate} />;
    case "legal":
      return <LegalPrivacyScreen onNavigate={navigate} />;
    case "legal-terms":
      return <LegalTermsScreen onNavigate={navigate} />;
    case "legal-credits":
      return <LegalCreditsScreen onNavigate={navigate} />;
    case "export":
      return <ExportDialogScreen mode="export" onNavigate={navigate} />;
    case "exporting":
      return <ExportDialogScreen mode="exporting" onNavigate={navigate} />;
    case "exported":
      return <ExportDialogScreen mode="exported" onNavigate={navigate} />;
    case "export-sub":
      return <ExportDialogScreen mode="export-sub" onNavigate={navigate} />;
    case "record":
      return <RecordScreen onNavigate={navigate} />;
    case "boot":
      return <BootScreen onNavigate={navigate} />;
    case "storage-ask":
      return <StorageAskScreen onNavigate={navigate} />;
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
