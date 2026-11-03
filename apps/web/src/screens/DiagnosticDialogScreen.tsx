import React from "react";
import { Button, Icon } from "@flux/ui";
import { TroubleshootingScreen } from "./TroubleshootingScreen.js";

const REPORT_DISCLOSURE: readonly [string, string, string][] = [
  ["Included", "Browser, device, Flux version, error log, glitch counts", "ok"],
  ["Never included", "Audio, transcripts, project names, file names", "bad"],
];

interface DiagnosticDialogScreenProps {
  onNavigate?: (route: string) => void;
}

export const DiagnosticDialogScreen: React.FC<DiagnosticDialogScreenProps> = ({ onNavigate }) => {
  const navigate = (r: string) => {
    if (onNavigate) {
      onNavigate(r);
    } else {
      window.location.hash = r;
    }
  };

  return (
    <div style={{ position: "relative", width: "100%", height: "100%" }}>
      {/* Background base screen */}
      <TroubleshootingScreen onNavigate={onNavigate} />

      {/* Modal Dialog Overlay matching design/app-core.js dlg() 1:1 */}
      <div className="ov" role="dialog" aria-modal="true" aria-label="Diagnostic report">
        <div className="dlg">
          <div className="between">
            <h2 className="h2">Diagnostic report</h2>
            <a
              className="btn g ic"
              href="#trouble"
              data-go="trouble"
              aria-label="Close"
              onClick={(e) => {
                e.preventDefault();
                navigate("trouble");
              }}
            >
              <Icon name="x" />
            </a>
          </div>

          <p className="dim">
            Creates a small file you can send with a support request. You decide whether to send it.
          </p>

          <div className="lst">
            {REPORT_DISCLOSURE.map(([title, desc, colorClass]) => (
              <div key={title} style={{ alignItems: "flex-start" }}>
                <div className="ti">
                  <b className={colorClass}>{title}</b>
                  <span style={{ whiteSpace: "normal" }}>{desc}</span>
                </div>
              </div>
            ))}
          </div>

          <div className="foot">
            <Button
              label="Cancel"
              variant="ghost"
              dataGo="trouble"
              href="#trouble"
              onClick={() => navigate("trouble")}
            />
            <Button
              label="Save report"
              variant="primary"
              icon="exp"
              dataGo="trouble"
              href="#trouble"
              onClick={() => navigate("trouble")}
            />
          </div>
        </div>
      </div>
    </div>
  );
};
