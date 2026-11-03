import React, { useState, useEffect } from "react";
import { SettingsLayout, FONTS, applyFont, getStoredFont } from "@flux/ui";

export const FontsScreen: React.FC = () => {
  const [currentFont, setCurrentFont] = useState(getStoredFont());

  useEffect(() => {
    setCurrentFont(getStoredFont());
  }, []);

  const handleSelectFont = (fontId: string, e: React.MouseEvent) => {
    setCurrentFont(fontId);
    applyFont(fontId, { clientX: e.clientX, clientY: e.clientY });
  };

  return (
    <SettingsLayout currentTab="fonts" title="Pick a voice.">
      <p className="lede" style={{ marginTop: "-8px" }}>
        Six fonts, all built in, so they work offline. Click one and the whole app changes. Numbers
        and small labels stay in Geist Mono. Your choice stays on this device.
      </p>
      <div className="g3">
        {FONTS.map((f, i) => {
          const isSelected = f.id === currentFont;
          return (
            <button
              key={f.id}
              type="button"
              className={`ftc ft-${f.id}`}
              data-set-font={f.id}
              aria-pressed={isSelected}
              onClick={(e) => handleSelectFont(f.id, e)}
            >
              <span className="meta">
                <span>{f.name}</span>
                <span>{i === 0 ? "Default" : ""}</span>
              </span>
              <span className="word">Sound, in flux.</span>
              <span className="say">{f.description} Cut it, spin it, pull the vocal out.</span>
            </button>
          );
        })}
      </div>
    </SettingsLayout>
  );
};
