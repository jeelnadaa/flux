import React, { useState } from "react";
import { EditScreen } from "./EditScreen.js";
import { Dialog, Button } from "@flux/ui";

export const QuotaDialogScreen: React.FC = () => {
  const [clearedWaveforms, setClearedWaveforms] = useState(false);
  const [clearedAi, setClearedAi] = useState(false);
  const [movedProject, setMovedProject] = useState(false);

  return (
    <>
      {/* Base Edit screen rendered behind the overlay */}
      <EditScreen />

      <Dialog
        title="Storage is almost full"
        baseScreenId="edit"
        footer={
          <a className="btn" href="#prefs-storage" data-go="prefs-storage">
            <span>Open storage settings</span>
          </a>
        }
      >
        <p className="dim" style={{ fontSize: "15px" }}>
          You have 180 MB left in this browser. Your work is saved, but Flux can't add more audio
          until there is room.
        </p>

        <div
          className="prog"
          style={{ "--v": "98%" } as React.CSSProperties}
          role="progressbar"
          aria-valuenow={98}
          aria-valuemin={0}
          aria-valuemax={100}
        >
          <i />
        </div>

        <div className="lst">
          <div>
            <div className="ti">
              <b>Clear waveform caches</b>
              <span style={{ whiteSpace: "normal" }}>
                {clearedWaveforms
                  ? "Caches cleared (freed 3.1 GB)"
                  : "Frees 3.1 GB. They rebuild when needed"}
              </span>
            </div>
            <Button
              variant="ghost"
              label={clearedWaveforms ? "Done" : "Clear"}
              onClick={() => setClearedWaveforms(true)}
            />
          </div>
          <div>
            <div className="ti">
              <b>Remove AI results</b>
              <span style={{ whiteSpace: "normal" }}>
                {clearedAi ? "AI cache removed (freed 1.6 GB)" : "Frees 1.6 GB. You can redo them"}
              </span>
            </div>
            <Button
              variant="ghost"
              label={clearedAi ? "Done" : "Remove"}
              onClick={() => setClearedAi(true)}
            />
          </div>
          <div>
            <div className="ti">
              <b>Move this project to a folder</b>
              <span style={{ whiteSpace: "normal" }}>
                {movedProject ? "Project moved (freed 412 MB here)" : "Frees 412 MB here"}
              </span>
            </div>
            <Button
              variant="ghost"
              label={movedProject ? "Done" : "Move"}
              onClick={() => setMovedProject(true)}
            />
          </div>
        </div>
      </Dialog>
    </>
  );
};
