import React, { useState } from "react";
import { EditScreen } from "./EditScreen.js";
import { Dialog, Pill, Toggle, Button } from "@flux/ui";

export const RelinkDialogScreen: React.FC = () => {
  const [lookInFolder, setLookInFolder] = useState(true);
  const [relinkedDrums, setRelinkedDrums] = useState(false);
  const [relinkedBass, setRelinkedBass] = useState(false);

  return (
    <>
      {/* Base Edit screen rendered behind the overlay */}
      <EditScreen />

      <Dialog
        title="2 files have moved"
        baseScreenId="edit"
        footer={
          <>
            <a className="btn g" href="#edit" data-go="edit">
              <span>Keep offline for now</span>
            </a>
            <a className="btn p" href="#edit" data-go="edit">
              <span>Relink</span>
            </a>
          </>
        }
      >
        <p className="dim">Your edits are safe. Flux just needs to find the originals.</p>

        <div className="lst">
          <div>
            <div className="ti">
              <b>drums-final.wav</b>
              <span>{relinkedDrums ? "Found in Audio/Stems" : "Was in Sessions/June"}</span>
            </div>
            <Pill
              label={relinkedDrums ? "Relinked" : "Not found"}
              variant={relinkedDrums ? "ok" : "warn"}
            />
            <Button
              variant="ghost"
              label={relinkedDrums ? "Change" : "Locate"}
              onClick={() => setRelinkedDrums(true)}
            />
          </div>
          <div>
            <div className="ti">
              <b>bass-di.wav</b>
              <span>{relinkedBass ? "Relinked" : "Was in Sessions/June"}</span>
            </div>
            <Pill label={relinkedBass ? "Relinked" : "Found nearby"} variant="ok" />
            <Button
              variant="ghost"
              label={relinkedBass ? "Change" : "Locate"}
              onClick={() => setRelinkedBass(true)}
            />
          </div>
        </div>

        <Toggle
          label="Look in the same folder for other files"
          checked={lookInFolder}
          onChange={setLookInFolder}
        />
      </Dialog>
    </>
  );
};
