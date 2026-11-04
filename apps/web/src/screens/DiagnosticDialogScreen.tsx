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

  const handleSaveReport = () => {
    try {
      const report = {
        fluxVersion: "0.1.0",
        timestamp: new Date().toISOString(),
        userAgent: typeof navigator !== "undefined" ? navigator.userAgent : "Node/CLI",
        platform: typeof navigator !== "undefined" ? navigator.platform : "Desktop",
        sampleRate: 48000,
        audioWorkletSupported: typeof window !== "undefined" && "AudioWorklet" in window,
        sharedArrayBuffer: typeof SharedArrayBuffer !== "undefined",
        webGpuAvailable: typeof navigator !== "undefined" && "gpu" in navigator,
        glitchCount: 0,
      };
      const blob = new Blob([JSON.stringify(report, null, 2)], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `flux-diagnostics-${Date.now()}.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch {
      // Diagnostic report download
    }
    navigate("trouble");
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
              onClick={handleSaveReport}
            />
          </div>
        </div>
      </div>
    </div>
  );
};
