import React, { useEffect, useRef } from "react";
import {
  ChannelDisplayMode,
  encodeMidSide,
  mapAmplitudeToHeight,
  TimelineViewport,
  WaveformScale,
} from "@flux/app-core";

export interface WaveformCanvasProps {
  samples?: Float32Array;
  rightSamples?: Float32Array;
  sampleRate?: number;
  scale?: WaveformScale;
  channelMode?: ChannelDisplayMode;
  showClipping?: boolean;
  color?: string; // CSS color or variable, e.g. '--t4'
  className?: string;
  style?: React.CSSProperties;
  viewport?: TimelineViewport;
  onSeek?: (timeSeconds: number) => void;
}

export const WaveformCanvas: React.FC<WaveformCanvasProps> = ({
  samples,
  rightSamples,
  sampleRate = 48000,
  scale = "linear",
  channelMode = "mono",
  showClipping = true,
  color = "--tx",
  className = "",
  style,
  viewport,
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

    // Resolve color
    let strokeCol = color;
    if (color.startsWith("--")) {
      strokeCol = getComputedStyle(canvas).getPropertyValue(color).trim() || "#fff";
    }
    const clipCol = getComputedStyle(canvas).getPropertyValue("--sig").trim() || "#ff4040";
    const lineCol =
      getComputedStyle(canvas).getPropertyValue("--ln").trim() || "rgba(255,255,255,0.15)";

    // Use default mock samples if not provided
    let sL = samples;
    let sR = rightSamples || samples;
    if (!sL || sL.length === 0) {
      const mockLen = 4800;
      sL = new Float32Array(mockLen);
      sR = new Float32Array(mockLen);
      for (let i = 0; i < mockLen; i++) {
        const t = i / mockLen;
        sL[i] = Math.sin(t * 80) * 0.45 * Math.sin(t * 12) + (Math.random() - 0.5) * 0.1;
        sR[i] = Math.sin(t * 85) * 0.45 * Math.cos(t * 14) + (Math.random() - 0.5) * 0.1;
      }
    }

    // Mid-side transformation if requested
    if (channelMode === "midside" && sL && sR) {
      const ms = encodeMidSide(sL, sR);
      sL = ms.mid;
      sR = ms.side;
    }

    const finalL = sL ?? new Float32Array(0);
    const finalR = sR ?? finalL;

    const vp =
      viewport ||
      new TimelineViewport(
        width / (Math.max(1, finalL.length) / sampleRate),
        sampleRate,
        finalL.length / sampleRate,
      );

    const isSampleLevel = vp.isSampleLevel(4);
    const isStereoOrMS = channelMode === "stereo" || channelMode === "midside";

    const renderChannel = (channelData: Float32Array, topY: number, chHeight: number) => {
      const midY = topY + chHeight / 2;
      const ampMaxHeight = chHeight * 0.46;

      // Draw zero axis line
      ctx.strokeStyle = lineCol;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(0, midY);
      ctx.lineTo(width, midY);
      ctx.stroke();

      if (isSampleLevel) {
        // Sample-level rendering: connected line and discrete dots
        ctx.beginPath();
        let first = true;
        const visibleSampleStart = Math.max(0, vp.pixelToSample(0));
        const visibleSampleEnd = Math.min(channelData.length - 1, vp.pixelToSample(width));

        for (let s = visibleSampleStart; s <= visibleSampleEnd; s++) {
          const x = vp.sampleToPixel(s);
          const rawAmp = channelData[s];
          const mappedH = mapAmplitudeToHeight(rawAmp, scale) * ampMaxHeight;
          const y = rawAmp >= 0 ? midY - mappedH : midY + mappedH;

          if (first) {
            ctx.moveTo(x, y);
            first = false;
          } else {
            ctx.lineTo(x, y);
          }
        }
        ctx.strokeStyle = strokeCol;
        ctx.lineWidth = 1.5;
        ctx.stroke();

        // Draw sample points / dots
        for (let s = visibleSampleStart; s <= visibleSampleEnd; s++) {
          const x = vp.sampleToPixel(s);
          const rawAmp = channelData[s];
          const isClipped = showClipping && Math.abs(rawAmp) >= 0.999;
          const mappedH = mapAmplitudeToHeight(rawAmp, scale) * ampMaxHeight;
          const y = rawAmp >= 0 ? midY - mappedH : midY + mappedH;

          ctx.beginPath();
          ctx.arc(x, y, 3, 0, Math.PI * 2);
          ctx.fillStyle = isClipped ? clipCol : strokeCol;
          ctx.fill();
        }
      } else {
        // Zoomed-out peak pyramid / column rendering
        const numCols = Math.min(width, 1600);
        const colWidth = width / numCols;

        for (let c = 0; c < numCols; c++) {
          const t0 = vp.pixelToTime(c * colWidth);
          const t1 = vp.pixelToTime((c + 1) * colWidth);
          const s0 = Math.max(0, Math.floor(t0 * sampleRate));
          const s1 = Math.min(channelData.length, Math.ceil(t1 * sampleRate));

          if (s0 >= s1) continue;

          let minVal = 0;
          let maxVal = 0;
          let clipped = false;

          for (let i = s0; i < s1; i++) {
            const v = channelData[i];
            if (v < minVal) minVal = v;
            if (v > maxVal) maxVal = v;
            if (showClipping && Math.abs(v) >= 0.999) {
              clipped = true;
            }
          }

          const mappedMax = mapAmplitudeToHeight(maxVal, scale) * ampMaxHeight;
          const mappedMin = mapAmplitudeToHeight(minVal, scale) * ampMaxHeight;

          const barY = midY - mappedMax;
          const barH = Math.max(1, mappedMax + mappedMin);

          ctx.fillStyle = clipped ? clipCol : strokeCol;
          ctx.fillRect(c * colWidth, barY, Math.max(1, colWidth * 0.8), barH);
        }
      }
    };

    if (isStereoOrMS) {
      renderChannel(finalL, 0, height / 2);
      ctx.fillStyle = lineCol;
      ctx.fillRect(0, height / 2, width, 1);
      renderChannel(finalR, height / 2, height / 2);
    } else {
      renderChannel(finalL, 0, height);
    }

    ctx.restore();
  }, [samples, rightSamples, sampleRate, scale, channelMode, showClipping, color, viewport]);

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
      className={`waveform-canvas ${className}`}
      style={{ width: "100%", height: "100%", display: "block", ...style }}
      onPointerDown={handlePointerDown}
    />
  );
};
