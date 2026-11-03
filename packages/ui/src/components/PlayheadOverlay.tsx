import React from "react";
import { TimelineViewport } from "@flux/app-core";

export interface PlayheadOverlayProps {
  playheadTime: number; // in seconds
  viewport?: TimelineViewport;
  headerWidthPx?: number; // default var(--hw) = 176px
  className?: string;
  style?: React.CSSProperties;
}

export const PlayheadOverlay: React.FC<PlayheadOverlayProps> = ({
  playheadTime,
  viewport,
  headerWidthPx = 176,
  className = "",
  style,
}) => {
  const vp = viewport || new TimelineViewport(100, 48000, 180, 0);
  const px = Math.round(vp.timeToPixel(playheadTime));

  return (
    <span
      className={`ph ${className}`}
      data-ph
      data-testid="timeline-playhead"
      style={{
        transform: `translateX(${px}px)`,
        left: `${headerWidthPx}px`,
        ...style,
      }}
    />
  );
};
