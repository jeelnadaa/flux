import React, { useState } from "react";
import {
  FrequencyScale,
  SpectrogramColorMap,
  TimelineViewport,
  WaveformScale,
} from "@flux/app-core";
import { WaveformCanvas } from "./WaveformCanvas.js";
import { SpectrogramCanvas } from "./SpectrogramCanvas.js";
import { SpectralSelectionOverlay } from "./SpectralSelectionOverlay.js";

export type HybridDisplayMode = "Spectrogram" | "Waveform" | "Both";

export interface HybridViewProps {
  samples?: Float32Array;
  sampleRate?: number;
  initialMode?: HybridDisplayMode;
  initialScale?: FrequencyScale;
  initialColormap?: SpectrogramColorMap;
  viewport?: TimelineViewport;
  className?: string;
  style?: React.CSSProperties;
}

export const HybridView: React.FC<HybridViewProps> = ({
  samples,
  sampleRate = 48000,
  initialMode = "Both",
  initialScale = "logarithmic",
  initialColormap = "magma",
  viewport,
  className = "",
  style,
}) => {
  const [mode] = useState<HybridDisplayMode>(initialMode);
  const [freqScale] = useState<FrequencyScale>(initialScale);
  const [colormap] = useState<SpectrogramColorMap>(initialColormap);
  const [waveformScale] = useState<WaveformScale>("linear");

  return (
    <div
      className={`hybrid-view ${className}`}
      style={{
        flex: 1,
        display: "flex",
        flexDirection: "column",
        minHeight: "300px",
        position: "relative",
        ...style,
      }}
    >
      {/* Waveform Section */}
      {(mode === "Waveform" || mode === "Both") && (
        <div
          data-testid="hybrid-waveform"
          style={{
            height: mode === "Both" ? "30%" : "100%",
            minHeight: "80px",
            borderBottom: mode === "Both" ? "1px solid var(--ln)" : "none",
            position: "relative",
          }}
        >
          <WaveformCanvas
            samples={samples}
            sampleRate={sampleRate}
            scale={waveformScale}
            channelMode="stereo"
            color="--tx"
            viewport={viewport}
          />
        </div>
      )}

      {/* Spectrogram Section */}
      {(mode === "Spectrogram" || mode === "Both") && (
        <div
          data-testid="hybrid-spectrogram"
          style={{
            flex: 1,
            position: "relative",
            minHeight: "220px",
          }}
        >
          <SpectrogramCanvas
            samples={samples}
            sampleRate={sampleRate}
            scale={freqScale}
            colormap={colormap}
            viewport={viewport}
          />
          <SpectralSelectionOverlay />
        </div>
      )}
    </div>
  );
};
