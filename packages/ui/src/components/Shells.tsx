import React from "react";
import { Icon, type IconName } from "../icons.js";
import { Button } from "./Button.js";
import { SegmentedControl } from "./SegmentedControl.js";

export const PROJ_NAV_ITEMS: readonly [string, IconName, string][] = [
  ["edit", "wave", "Edit"],
  ["spectral", "spec", "Spectral"],
  ["mix", "mix", "Mix"],
  ["stage", "orbit", "Stage"],
  ["script", "script", "Script"],
  ["master", "master", "Master"],
  ["fxbrowser", "fx", "Effects"],
  ["aitools", "ai", "AI"],
] as const;

export const HOME_NAV_ITEMS: readonly [string, IconName, string][] = [
  ["home", "home", "Home"],
  ["library", "folder", "Projects"],
  ["convert", "swap", "Convert"],
  ["batch", "batch", "Batch"],
  ["ringtone", "ring", "Ringtone"],
  ["presets", "tag", "Presets"],
  ["macros", "list", "Actions"],
] as const;

export interface WatermarkProps {
  href?: string;
  dataGo?: string;
  className?: string;
}

export const Watermark: React.FC<WatermarkProps> = ({
  href = "#home",
  dataGo = "home",
  className = "",
}) => (
  <a className={`wm ${className}`} href={href} data-go={dataGo} aria-label="Flux home">
    f<i />
    ux
  </a>
);

export interface TopBarProps {
  projectName?: string;
  projectStatus?: string;
  rightActions?: React.ReactNode;
  homeLink?: string;
  className?: string;
}

export const TopBar: React.FC<TopBarProps> = ({
  projectName = "Harbor Lights",
  projectStatus = "Saved 12 seconds ago",
  rightActions,
  homeLink = "home",
  className = "",
}) => {
  return (
    <header className={`tb ${className}`}>
      <Watermark dataGo={homeLink} href={`#${homeLink}`} />
      <div className="pname">
        <b>{projectName}</b>
        <span>{projectStatus}</span>
      </div>
      <span className="sp" />
      {rightActions}
    </header>
  );
};

export interface RailProps {
  items?: readonly (readonly [string, IconName, string])[];
  current?: string;
  onNavigate?: (id: string) => void;
  className?: string;
}

export const Rail: React.FC<RailProps> = ({
  items = HOME_NAV_ITEMS,
  current = "home",
  onNavigate,
  className = "",
}) => {
  return (
    <nav className={`rail ${className}`} aria-label="Main">
      {items.map(([id, icon, label]) => {
        const isActive = id === current;
        return (
          <a
            key={id}
            href={`#${id}`}
            data-go={id}
            className={isActive ? "on" : ""}
            aria-current={isActive ? "page" : undefined}
            onClick={() => onNavigate?.(id)}
          >
            <Icon name={icon} />
            {label}
          </a>
        );
      })}
      <span className="gap" />
      <a
        href="#help"
        data-go="help"
        className={`xm ${current === "help" ? "on" : ""}`}
        aria-current={current === "help" ? "page" : undefined}
        onClick={() => onNavigate?.("help")}
      >
        <Icon name="help" />
        Help
      </a>
      <a
        href="#prefs-general"
        data-go="prefs-general"
        className={current.startsWith("prefs") ? "on" : ""}
        aria-current={current.startsWith("prefs") ? "page" : undefined}
        onClick={() => onNavigate?.("prefs-general")}
      >
        <Icon name="cog" />
        Settings
      </a>
    </nav>
  );
};

export interface TransportProps {
  isPlaying?: boolean;
  timecode?: string;
  bpm?: string;
  keySig?: string;
  sampleRate?: string;
  lufs?: string;
  onPlayPause?: () => void;
  onRecord?: () => void;
  onLoopToggle?: () => void;
  onHoldOriginal?: () => void;
  extra?: React.ReactNode;
  className?: string;
}

export const Transport: React.FC<TransportProps> = ({
  isPlaying = false,
  timecode = "00:00:42.318",
  bpm = "84.0 BPM",
  keySig = "D min",
  sampleRate = "48 kHz",
  lufs = "−14.2 LUFS",
  onPlayPause,
  onRecord,
  onLoopToggle,
  onHoldOriginal,
  extra,
  className = "",
}) => {
  return (
    <footer className={`tp ${className}`} role="region" aria-label="Transport">
      <div className="tgrp">
        <button className="tbtn hide-s" type="button" aria-label="Back to start">
          <Icon name="back" />
        </button>
        <button
          className="tbtn play"
          type="button"
          data-play=""
          aria-label={isPlaying ? "Pause" : "Play"}
          onClick={onPlayPause}
        >
          <Icon name={isPlaying ? "pause" : "play"} />
        </button>
        <button className="tbtn hide-s" type="button" aria-label="Stop">
          <Icon name="stop" />
        </button>
        <button
          className="tbtn rec hide-s"
          type="button"
          aria-label="Record"
          data-go="record"
          onClick={onRecord}
        >
          <Icon name="rec" />
        </button>
        <button
          className="tbtn on hide-s"
          type="button"
          aria-label="Loop"
          data-toggle=""
          onClick={onLoopToggle}
        >
          <Icon name="loop" />
        </button>
      </div>
      <span className="tc" data-tc="">
        {timecode}
      </span>
      <div className="row hide-s mono dim" style={{ flexWrap: "nowrap", gap: "14px" }}>
        <span>{bpm}</span>
        <span>{keySig}</span>
        <span>{sampleRate}</span>
      </div>
      <span className="sp" />
      {extra}
      <button
        className="btn g hold hide-s"
        type="button"
        data-hold=""
        title="Hold to hear the original at matched loudness"
        onClick={onHoldOriginal}
      >
        Hold for original
      </button>
      <div className="meter" role="img" aria-label="Output level">
        <div className="m">
          <i data-lv="" />
        </div>
        <div className="m">
          <i data-lv="" />
        </div>
      </div>
      <span className="mono dim hide-m">{lufs}</span>
    </footer>
  );
};

export interface ProjectLayoutProps {
  currentTab?: string;
  projectName?: string;
  projectStatus?: string;
  children: React.ReactNode;
  rightActions?: React.ReactNode;
  transportExtra?: React.ReactNode;
  onNavigate?: (id: string) => void;
  className?: string;
}

const defaultProjectRightActions = (
  <>
    <SegmentedControl items={["Simple", "Pro"]} selectedIndex={1} className="hide-s" dataMode />
    <button className="tbtn" type="button" aria-label="Undo">
      <Icon name="undo" />
    </button>
    <button className="tbtn hide-s" type="button" aria-label="Redo">
      <Icon name="redo" />
    </button>
    <a className="btn g lbl-s hide-s" href="#history" data-go="history" aria-label="History">
      <Icon name="hist" />
    </a>
    <Button variant="magic" className="lbl-s" icon="mag" label="Magic" dataGo="magic" />
    <Button variant="primary" className="lbl-s" icon="exp" label="Export" dataGo="export" />
  </>
);

export const ProjectLayout: React.FC<ProjectLayoutProps> = ({
  currentTab = "edit",
  projectName = "Harbor Lights",
  projectStatus = "Saved 12 seconds ago",
  children,
  rightActions = defaultProjectRightActions,
  transportExtra,
  onNavigate,
  className = "",
}) => {
  return (
    <div className={`app ${className}`}>
      <TopBar projectName={projectName} projectStatus={projectStatus} rightActions={rightActions} />
      <Rail items={PROJ_NAV_ITEMS} current={currentTab} onNavigate={onNavigate} />
      <main className="main" id="m">
        {children}
      </main>
      <Transport extra={transportExtra} />
    </div>
  );
};

export interface PageLayoutProps {
  currentTab?: string;
  title: string;
  subtitle?: string;
  children: React.ReactNode;
  actions?: React.ReactNode;
  navItems?: readonly (readonly [string, IconName, string])[];
  topBarTitle?: string;
  topBarSubtitle?: string;
  onNavigate?: (id: string) => void;
  className?: string;
}

export const PageLayout: React.FC<PageLayoutProps> = ({
  currentTab = "home",
  title,
  subtitle,
  children,
  actions,
  navItems = HOME_NAV_ITEMS,
  topBarTitle = "Flux",
  topBarSubtitle = "On this device",
  onNavigate,
  className = "",
}) => {
  const defaultRight = (
    <>
      <Button
        variant="ghost"
        className="hide-s"
        icon="search"
        ariaLabel="Search commands"
        dataGo="palette"
      />
      <Button className="hide-s" label="Open project" dataGo="library" />
      <Button variant="primary" icon="plus" label="New" dataGo="new" />
    </>
  );

  return (
    <div className={`app notp ${className}`}>
      <TopBar
        projectName={topBarTitle}
        projectStatus={topBarSubtitle}
        rightActions={defaultRight}
      />
      <Rail items={navItems} current={currentTab} onNavigate={onNavigate} />
      <main className="main" id="m">
        <div className="pg in">
          <div className="pgh">
            <div>
              <h1 className="h1">{title}</h1>
              {subtitle && <p className="lede">{subtitle}</p>}
            </div>
            <div className="row">{actions}</div>
          </div>
          {children}
        </div>
      </main>
    </div>
  );
};

export const PREF_TABS: readonly (readonly [string, string])[] = [
  ["themes", "Themes"],
  ["fonts", "Fonts"],
  ["general", "General"],
  ["audio", "Audio"],
  ["keys", "Shortcuts"],
  ["privacy", "Privacy"],
  ["models", "AI models"],
  ["storage", "Storage"],
  ["access", "Accessibility"],
  ["account", "Account"],
  ["about", "About"],
] as const;

export interface SettingsLayoutProps {
  currentTab: string;
  title: string;
  children: React.ReactNode;
  maxWidth?: number;
  className?: string;
}

export const SettingsLayout: React.FC<SettingsLayoutProps> = ({
  currentTab,
  title,
  children,
  maxWidth,
  className = "",
}) => {
  const actualMaxWidth =
    maxWidth ?? (currentTab === "themes" || currentTab === "fonts" ? 1320 : 860);

  return (
    <div className={`app notp ${className}`}>
      <TopBar
        projectName="Settings"
        projectStatus="Saved automatically"
        rightActions={<Button variant="ghost" label="Close" dataGo="home" href="#home" />}
      />
      <Rail items={HOME_NAV_ITEMS} current="prefs" />
      <main className="main" id="m">
        <div className="tabs">
          {PREF_TABS.map(([tabId, tabLabel]) => (
            <a
              key={tabId}
              href={`#prefs-${tabId}`}
              data-go={`prefs-${tabId}`}
              className={tabId === currentTab ? "on" : ""}
            >
              {tabLabel}
            </a>
          ))}
        </div>
        <div className="pg in" style={{ maxWidth: `${actualMaxWidth}px` }}>
          <h1 className="h1">{title}</h1>
          {children}
        </div>
      </main>
    </div>
  );
};

export interface WizardLayoutProps {
  step?: number;
  totalSteps?: number;
  children: React.ReactNode;
  wide?: boolean;
  watermarkGo?: string;
  className?: string;
}

export const WizardLayout: React.FC<WizardLayoutProps> = ({
  step = 0,
  totalSteps = 0,
  children,
  wide = false,
  watermarkGo = "welcome",
  className = "",
}) => {
  return (
    <div className={`wiz ${wide ? "wide" : ""} ${className}`}>
      <div className="inner in">
        <div className="between">
          <Watermark dataGo={watermarkGo} href={`#${watermarkGo}`} />
          {totalSteps > 0 && (
            <div className="dots" role="img" aria-label={`Step ${step} of ${totalSteps}`}>
              {Array.from({ length: totalSteps }, (_, i) => (
                <i key={i} className={i < step ? "on" : ""} />
              ))}
            </div>
          )}
        </div>
        {children}
      </div>
    </div>
  );
};

export interface DialogProps {
  title: string;
  children: React.ReactNode;
  footer?: React.ReactNode;
  baseScreenId?: string;
  onClose?: () => void;
  wide?: boolean;
  className?: string;
}

export const Dialog: React.FC<DialogProps> = ({
  title,
  children,
  footer,
  baseScreenId = "edit",
  onClose,
  wide = false,
  className = "",
}) => {
  return (
    <div className="ov" role="dialog" aria-modal="true" aria-label={title}>
      <div className={`dlg ${wide ? "wide" : ""} ${className}`}>
        <div className="between">
          <h2 className="h2">{title}</h2>
          <Button
            variant="ghost"
            iconOnly
            icon="x"
            ariaLabel="Close"
            dataGo={baseScreenId}
            href={`#${baseScreenId}`}
            onClick={onClose}
          />
        </div>
        {children}
        {footer && <div className="foot">{footer}</div>}
      </div>
    </div>
  );
};
