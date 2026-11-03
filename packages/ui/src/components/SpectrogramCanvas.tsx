import React, { useEffect, useRef } from "react";
import {
  FrequencyScale,
  hzToMel,
  melToHz,
  SpectrogramColorMap,
  TimelineViewport,
} from "@flux/app-core";

export interface SpectrogramCanvasProps {
  samples?: Float32Array;
  sampleRate?: number;
  scale?: FrequencyScale;
  colormap?: SpectrogramColorMap;
  minFreq?: number;
  maxFreq?: number;
  showFrequencyGrid?: boolean;
  viewport?: TimelineViewport;
  className?: string;
  style?: React.CSSProperties;
  onSeek?: (timeSeconds: number) => void;
}

function clamp(v: number, min = 0, max = 1): number {
  return Math.min(max, Math.max(min, v));
}

function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}

// Colormap ramps
const COLORMAPS: Record<SpectrogramColorMap, [number, number, number][]> = {
  magma: [
    [0, 0, 4],
    [81, 18, 124],
    [182, 54, 121],
    [251, 136, 97],
    [252, 253, 191],
  ],
  viridis: [
    [68, 1, 84],
    [59, 82, 139],
    [33, 145, 140],
    [94, 201, 98],
    [253, 231, 37],
  ],
  inferno: [
    [0, 0, 4],
    [87, 16, 110],
    [187, 55, 84],
    [249, 142, 9],
    [252, 255, 164],
  ],
  theme: [
    [20, 20, 25],
    [120, 40, 90],
    [240, 100, 50],
    [255, 220, 180],
  ],
};

function sampleColormap(colormap: SpectrogramColorMap, normVal: number): [number, number, number] {
  const ramp = COLORMAPS[colormap] || COLORMAPS.magma;
  const scaled = clamp(normVal) * (ramp.length - 1);
  const idx = Math.floor(scaled);
  const frac = scaled - idx;

  if (idx >= ramp.length - 1) return ramp[ramp.length - 1];
  const c0 = ramp[idx];
  const c1 = ramp[idx + 1];
  return [
    Math.round(lerp(c0[0], c1[0], frac)),
    Math.round(lerp(c0[1], c1[1], frac)),
    Math.round(lerp(c0[2], c1[2], frac)),
  ];
}

export const SpectrogramCanvas: React.FC<SpectrogramCanvasProps> = ({
  samples,
  sampleRate = 48000,
  scale = "logarithmic",
  colormap = "magma",
  minFreq = 20,
  maxFreq = 20000,
  showFrequencyGrid = true,
  viewport,
  className = "",
  style,
  onSeek,
}) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const rect = canvas.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    const width = Math.max(1, Math.round(rect.width));
    const height = Math.max(1, Math.round(rect.height));

    if (canvas.width !== width * dpr || canvas.height !== height * dpr) {
      canvas.width = width * dpr;
      canvas.height = height * dpr;
    }

    ctx.save();
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, width, height);

    // Render synthetic spectrogram columns
    const numCols = Math.max(40, Math.min(width, 480));
    const numFreqBins = Math.max(20, Math.min(height, 120));
    const offscreen = document.createElement("canvas");
    offscreen.width = numCols;
    offscreen.height = numFreqBins;
    const offCtx = offscreen.getContext("2d");

    if (offCtx) {
      const imgData = offCtx.createImageData(numCols, numFreqBins);
      const minMel = hzToMel(minFreq);
      const maxMel = hzToMel(maxFreq);

      for (let x = 0; x < numCols; x++) {
        const t = x / numCols;
        for (let y = 0; y < numFreqBins; y++) {
          // Invert y: top of canvas is maxFreq, bottom is minFreq
          const normY = 1.0 - y / numFreqBins;
          let freqHz = minFreq;

          if (scale === "linear") {
            freqHz = minFreq + normY * (maxFreq - minFreq);
          } else if (scale === "logarithmic") {
            freqHz = minFreq * Math.pow(maxFreq / minFreq, normY);
          } else if (scale === "mel") {
            const m = minMel + normY * (maxMel - minMel);
            freqHz = melToHz(m);
          }

          // Synthetic harmonic and noise energy simulation matching design/viz.js
          const fNorm = freqHz / maxFreq;
          const harmonic1 = Math.pow(Math.max(0, Math.sin(fNorm * 64 + Math.sin(t * 80) * 2.5)), 8);
          const bassEnergy = Math.exp(-fNorm * 12);
          const midEnergy = Math.exp(-fNorm * 3.5) * 0.7;
          const energy =
            bassEnergy * 0.4 +
            midEnergy * 0.3 +
            harmonic1 * 0.35 +
            Math.sin(t * 19 + fNorm * 40) * 0.08;

          const [r, g, b] = sampleColormap(colormap, clamp(energy * 1.4));
          const pixelIdx = (y * numCols + x) * 4;
          imgData.data[pixelIdx] = r;
          imgData.data[pixelIdx + 1] = g;
          imgData.data[pixelIdx + 2] = b;
          imgData.data[pixelIdx + 3] = 255;
        }
      }
      offCtx.putImageData(imgData, 0, 0);
      ctx.imageSmoothingEnabled = true;
      ctx.drawImage(offscreen, 0, 0, width, height);
    }

    // Overlay frequency grid lines & labels if requested
    if (showFrequencyGrid) {
      const textCol =
        getComputedStyle(canvas).getPropertyValue("--tx3").trim() || "rgba(255,255,255,0.45)";
      const lineCol =
        getComputedStyle(canvas).getPropertyValue("--ln").trim() || "rgba(255,255,255,0.12)";

      ctx.strokeStyle = lineCol;
      ctx.lineWidth = 1;
      ctx.fillStyle = textCol;
      ctx.font = '500 10px "Geist Mono", monospace';

      const gridFreqs = [50, 200, 1000, 5000, 10000];
      const minMel = hzToMel(minFreq);
      const maxMel = hzToMel(maxFreq);

      for (const f of gridFreqs) {
        if (f < minFreq || f > maxFreq) continue;
        let normY = 0;
        if (scale === "linear") {
          normY = (f - minFreq) / (maxFreq - minFreq);
        } else if (scale === "logarithmic") {
          normY = Math.log10(f / minFreq) / Math.log10(maxFreq / minFreq);
        } else if (scale === "mel") {
          normY = (hzToMel(f) - minMel) / (maxMel - minMel);
        }

        const y = height - normY * height;
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(width, y);
        ctx.stroke();

        const label = f >= 1000 ? `${f / 1000}k` : `${f} Hz`;
        ctx.fillText(label, 6, Math.max(12, y - 3));
      }
    }

    ctx.restore();
  }, [samples, sampleRate, scale, colormap, minFreq, maxFreq, showFrequencyGrid, viewport]);

  const handlePointerDown = (e: React.PointerEvent<HTMLCanvasElement>) => {
    if (!onSeek || !canvasRef.current) return;
    const rect = canvasRef.current.getBoundingClientRect();
    const px = e.clientX - rect.left;
    if (viewport) {
      onSeek(viewport.pixelToTime(px));
    }
  };

  return (
    <canvas
      ref={canvasRef}
      className={`spectrogram-canvas ${className}`}
      style={{ width: "100%", height: "100%", display: "block", ...style }}
      onPointerDown={handlePointerDown}
    />
  );
};
