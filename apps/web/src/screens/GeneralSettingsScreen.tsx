import React from "react";
import {
  SettingsLayout,
  Button,
  Field,
  Select,
  SegmentedControl,
  Toggle,
  THEMES,
  FONTS,
  getStoredTheme,
  getStoredFont,
} from "@flux/ui";

export const GeneralSettingsScreen: React.FC = () => {
  const currentThemeId = getStoredTheme();
  const currentFontId = getStoredFont();

  const themeObj = THEMES.find((t) => t.id === currentThemeId) || THEMES[0];
  const fontObj = FONTS.find((f) => f.id === currentFontId) || FONTS[0];

  return (
    <SettingsLayout currentTab="general" title="General">
      <div className="box col">
        <div className="between">
          <div className="col" style={{ gap: "2px" }}>
            <b>Theme</b>
            <span className="lab">{themeObj.name}</span>
          </div>
          <Button label="Change theme" dataGo="prefs-themes" href="#prefs-themes" />
        </div>
        <div className="between">
          <div className="col" style={{ gap: "2px" }}>
            <b>Font</b>
            <span className="lab">{fontObj.name}</span>
          </div>
          <Button label="Change font" dataGo="prefs-fonts" href="#prefs-fonts" />
        </div>
        <Field label="Language">
          <Select
            options={["English", "Hindi", "Español", "Français", "Deutsch", "Português", "日本語"]}
            selectedIndex={0}
          />
        </Field>
        <Field label="Starting mode">
          <SegmentedControl items={["Simple", "Pro"]} selectedIndex={1} />
        </Field>
      </div>
      <div className="box col">
        <Toggle
          label="Automatic micro-fades on every cut"
          defaultChecked
          sub="Stops clicks. 3 ms by default"
        />
        <Toggle label="Snap to zero crossings" defaultChecked />
        <Field label="New project sample rate">
          <Select options={["48 kHz", "44.1 kHz", "96 kHz"]} selectedIndex={0} />
        </Field>
      </div>
    </SettingsLayout>
  );
};
