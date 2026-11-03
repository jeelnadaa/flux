import React, { useState } from "react";
import { HomeScreen } from "./HomeScreen.js";
import { Dialog, Pill, Button } from "@flux/ui";

export const RecoveryDialogScreen: React.FC = () => {
  const [exportResumed, setExportResumed] = useState(false);

  return (
    <>
      {/* Base Home screen rendered behind the overlay */}
      <HomeScreen />

      <Dialog
        title="We saved your work."
        baseScreenId="home"
        footer={
          <>
            <a className="btn g" href="#history" data-go="history">
              <span>Open earlier snapshot</span>
            </a>
            <a className="btn p" href="#edit" data-go="edit">
              <span>Reopen project</span>
            </a>
          </>
        }
      >
        <p className="dim" style={{ fontSize: "15px" }}>
          Flux closed unexpectedly. Your last change was saved 2 seconds before it happened.
        </p>

        <div className="box between">
          <div>
            <b>Harbor Lights</b>
            <br />
            <span className="lab">142 edits, all recovered. Last: Move "Chorus"</span>
          </div>
          <Pill label="Complete" variant="ok" />
        </div>

        <div className="box between">
          <div>
            <b>Export: master.flac</b>
            <br />
            <span className="lab">{exportResumed ? "Resumed (98%)" : "Stopped at 62%"}</span>
          </div>
          <Button
            variant="ghost"
            label={exportResumed ? "Resumed" : "Resume"}
            onClick={() => setExportResumed(true)}
          />
        </div>
      </Dialog>
    </>
  );
};
