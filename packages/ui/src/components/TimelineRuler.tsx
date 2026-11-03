import React, { useEffect, useRef, useState } from "react";
import { Marker, Region, TimelineViewport } from "@flux/app-core";

export interface TimelineRulerProps {
  viewport?: TimelineViewport;
  durationSeconds?: number;
  playheadTime?: number;
  loopEnabled?: boolean;
  loopStartTime?: number;
  loopEndTime?: number;
  markers?: Marker[];
  regions?: Region[];
  onSeek?: (timeSeconds: number) => void;
  onScrubStart?: () => void;
  onScrubEnd?: () => void;
  onLoopChange?: (start: number, end: number) => void;
  className?: string;
  style?: React.CSSProperties;
}

export const TimelineRuler: React.FC<TimelineRulerProps> = ({
  viewport,
  durationSeconds = 180,
  playheadTime = 0,
  loopEnabled = false,
  loopStartTime = 10,
  loopEndTime = 30,
  markers = [],
  regions = [],
  onSeek,
  onScrubStart,
  onScrubEnd,
  className = "",
  style,
}) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [isScrubbing, setIsScrubbing] = useState(false);

  const vp = viewport || new TimelineViewport(100, 48000, durationSeconds, 0);

  // Render ruler ticks and labels
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

    ctx.fillStyle = getComputedStyle(canvas).getPropertyValue("--tx3").trim() || "#888";
    ctx.font = '500 10.5px "Geist Mono", monospace';

    // Time division spacing based on pixels per second
    const pxPerSec = vp.pixelsPerSecond;
    let stepSec = 1;
    if (pxPerSec < 10) stepSec = 10;
    else if (pxPerSec < 40) stepSec = 5;
    else if (pxPerSec < 100) stepSec = 2;
    else if (pxPerSec < 300) stepSec = 1;
    else if (pxPerSec < 1000) stepSec = 0.5;
    else stepSec = 0.1;

    const startTime = vp.pixelToTime(0);
    const endTime = vp.pixelToTime(width);

    const firstTick = Math.floor(startTime / stepSec) * stepSec;
    const lineCol =
      getComputedStyle(canvas).getPropertyValue("--ln").trim() || "rgba(255,255,255,0.2)";

    for (let t = firstTick; t <= endTime; t += stepSec) {
      const px = vp.timeToPixel(t);
      if (px < 0 || px > width) continue;

      ctx.fillStyle = lineCol;
      ctx.fillRect(px, height - 10, 1, 10);

      // Minor tick at half interval
      const halfPx = vp.timeToPixel(t + stepSec / 2);
      if (halfPx <= width) {
        ctx.fillRect(halfPx, height - 5, 1, 5);
      }

      // Time label
      ctx.fillStyle = getComputedStyle(canvas).getPropertyValue("--tx3").trim() || "#888";
      const mins = Math.floor(t / 60);
      const secs = (t % 60).toFixed(stepSec < 1 ? 1 : 0);
      const label = `${mins}:${secs.padStart(stepSec < 1 ? 4 : 2, "0")}`;
      ctx.fillText(label, px + 4, 14);
    }

    ctx.restore();
  }, [vp, durationSeconds]);

  const handlePointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!containerRef.current) return;
    const rect = containerRef.current.getBoundingClientRect();
    const px = e.clientX - rect.left;
    const seekTime = Math.max(0, Math.min(durationSeconds, vp.pixelToTime(px)));

    setIsScrubbing(true);
    onScrubStart?.();
    onSeek?.(seekTime);
    containerRef.current.setPointerCapture(e.pointerId);
  };

  const handlePointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!isScrubbing || !containerRef.current) return;
    const rect = containerRef.current.getBoundingClientRect();
    const px = e.clientX - rect.left;
    const seekTime = Math.max(0, Math.min(durationSeconds, vp.pixelToTime(px)));
    onSeek?.(seekTime);
  };

  const handlePointerUp = (e: React.PointerEvent<HTMLDivElement>) => {
    if (isScrubbing && containerRef.current) {
      setIsScrubbing(false);
      onScrubEnd?.();
      try {
        containerRef.current.releasePointerCapture(e.pointerId);
      } catch {
        // Ignored
      }
    }
  };

  // Playhead pixel position
  const playheadPx = vp.timeToPixel(playheadTime);

  // Loop band positions
  const loopStartPx = vp.timeToPixel(loopStartTime);
  const loopEndPx = vp.timeToPixel(loopEndTime);

  return (
    <div
      ref={containerRef}
      className={`ruler ${className}`}
      style={{
        position: "relative",
        userSelect: "none",
        cursor: "ew-resize",
        ...style,
      }}
      role="slider"
      aria-label="Timeline position"
      aria-valuenow={Math.round(playheadTime)}
      aria-valuemin={0}
      aria-valuemax={Math.round(durationSeconds)}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onPointerCancel={handlePointerUp}
    >
      <canvas ref={canvasRef} style={{ width: "100%", height: "100%", display: "block" }} />

      {/* Loop Region Indicator */}
      {loopEnabled && (
        <div
          data-testid="loop-region"
          style={{
            position: "absolute",
            top: 0,
            bottom: 0,
            left: `${loopStartPx}px`,
            width: `${Math.max(2, loopEndPx - loopStartPx)}px`,
            background: "color-mix(in srgb, var(--sig) 20%, transparent)",
            borderLeft: "2px solid var(--sig)",
            borderRight: "2px solid var(--sig)",
            pointerEvents: "none",
            zIndex: 3,
          }}
        />
      )}

      {/* Shaded Regions */}
      {regions.map((r) => {
        const x1 = vp.timeToPixel(r.startTime);
        const x2 = vp.timeToPixel(r.endTime);
        return (
          <div
            key={r.id}
            data-testid={`region-${r.id}`}
            style={{
              position: "absolute",
              top: 0,
              bottom: 0,
              left: `${x1}px`,
              width: `${Math.max(2, x2 - x1)}px`,
              background: r.color
                ? `color-mix(in srgb, var(${r.color}) 15%, transparent)`
                : "var(--s2)",
              pointerEvents: "none",
              zIndex: 2,
            }}
          />
        );
      })}

      {/* Markers */}
      {markers.map((m) => {
        const x = vp.timeToPixel(m.time);
        return (
          <div
            key={m.id}
            data-testid={`marker-${m.id}`}
            style={{
              position: "absolute",
              top: 0,
              bottom: 0,
              left: `${x}px`,
              pointerEvents: "none",
              zIndex: 4,
            }}
          >
            <div
              style={{
                position: "absolute",
                top: 0,
                left: "-4px",
                width: "8px",
                height: "8px",
                borderRadius: "50%",
                background: m.color ? `var(${m.color})` : "var(--tx)",
              }}
            />
            <span
              className="mono"
              style={{
                position: "absolute",
                top: "10px",
                left: "2px",
                fontSize: "9px",
                color: "var(--tx)",
                whiteSpace: "nowrap",
              }}
            >
              {m.label}
            </span>
          </div>
        );
      })}

      {/* Playhead indicator on ruler */}
      <div
        data-testid="ruler-playhead"
        style={{
          position: "absolute",
          top: 0,
          bottom: 0,
          left: `${playheadPx}px`,
          width: "2px",
          background: "var(--sig)",
          zIndex: 6,
          pointerEvents: "none",
        }}
      >
        <span
          style={{
            position: "absolute",
            top: 0,
            left: "-4px",
            border: "5px solid transparent",
            borderTopColor: "var(--sig)",
          }}
        />
      </div>
    </div>
  );
};
