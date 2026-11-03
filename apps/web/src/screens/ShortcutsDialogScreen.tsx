import React from "react";
import { EditScreen } from "./EditScreen.js";
import { Dialog } from "@flux/ui";

const SHORTCUTS = [
  ["Space", "Play or pause"],
  ["S", "Split"],
  ["A", "Hold for original"],
  ["Z", "Zoom to selection"],
  ["M", "Marker"],
  ["Ctrl Z", "Undo"],
  ["Ctrl K", "Commands"],
  ["?", "This sheet"],
] as const;

export const ShortcutsDialogScreen: React.FC = () => {
  return (
    <>
      <EditScreen />
      <Dialog
        title="Keyboard shortcuts"
        baseScreenId="edit"
        wide
        footer={
          <a className="btn" href="#prefs-keys" data-go="prefs-keys">
            <span>All shortcuts</span>
          </a>
        }
      >
        <div className="g2">
          {SHORTCUTS.map(([key, label]) => (
            <div key={key} className="between">
              <span>{label}</span>
              <span className="kbd">{key}</span>
            </div>
          ))}
        </div>
      </Dialog>
    </>
  );
};
