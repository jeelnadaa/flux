import React, { useState, useEffect } from "react";
import { SettingsLayout, THEMES, applyTheme, getStoredTheme } from "@flux/ui";

export const ThemesScreen: React.FC = () => {
  const [currentTheme, setCurrentTheme] = useState(getStoredTheme());

  useEffect(() => {
    setCurrentTheme(getStoredTheme());
  }, []);

  const handleSelectTheme = (themeId: string, e: React.MouseEvent) => {
    setCurrentTheme(themeId);
    applyTheme(themeId, { clientX: e.clientX, clientY: e.clientY });
  };

  return (
    <SettingsLayout currentTab="themes" title="Pick a ground.">
      <p className="lede" style={{ marginTop: "-8px" }}>
        Thirteen themes, one for each effect on the wall. Click one and it spreads across the app.
        Your choice stays on this device.
      </p>
      <div className="g3">
        {THEMES.map((t) => {
          const isSelected = t.id === currentTheme;
          return (
            <button
              key={t.id}
              type="button"
              className={`fxc pal-${t.id} ${t.effectClass ? `f-${t.effectClass}` : ""}`}
              data-set-theme={t.id}
              aria-pressed={isSelected}
              aria-label={t.name}
              onClick={(e) => handleSelectTheme(t.id, e)}
            >
              {t.effectClass === "vinyl" && <span className="disc" aria-hidden="true" />}
              <span className="meta">
                <span>{t.name}</span>
                <span>{t.meta}</span>
              </span>
              <span
                className="word"
                style={{ "--n": Math.max(t.word.length, 4) } as React.CSSProperties}
                aria-hidden="true"
              >
                {t.word}
              </span>
            </button>
          );
        })}
      </div>
    </SettingsLayout>
  );
};
