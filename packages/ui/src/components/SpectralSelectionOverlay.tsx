import React, { useRef, useState } from "react";

export interface SpectralSelectionData {
  left: number; // percentage (0 - 100)
  top: number; // percentage (0 - 100)
  width: number; // percentage (0 - 100)
  height: number; // percentage (0 - 100)
  label: string;
}

export interface SpectralSelectionOverlayProps {
  initialLeftPct?: number;
  initialTopPct?: number;
  initialWidthPct?: number;
  initialHeightPct?: number;
  label?: string;
  onChange?: (selection: SpectralSelectionData) => void;
  className?: string;
  style?: React.CSSProperties;
}

export const SpectralSelectionOverlay: React.FC<SpectralSelectionOverlayProps> = ({
  initialLeftPct = 54,
  initialTopPct = 26,
  initialWidthPct = 16,
  initialHeightPct = 26,
  label = "2.1 to 4.8 kHz · 1.42 s",
  onChange,
  className = "",
  style,
}) => {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [left, setLeft] = useState<number>(initialLeftPct);
  const [top, setTop] = useState<number>(initialTopPct);
  const [width, setWidth] = useState<number>(initialWidthPct);
  const [height, setHeight] = useState<number>(initialHeightPct);
  const [currentLabel, setCurrentLabel] = useState<string>(label);

  const [isDragging, setIsDragging] = useState(false);
  const [dragMode, setDragMode] = useState<"create" | "move">("move");
  const dragStartRef = useRef<{
    x: number;
    y: number;
    initLeft: number;
    initTop: number;
    initW: number;
    initH: number;
  }>({
    x: 0,
    y: 0,
    initLeft: initialLeftPct,
    initTop: initialTopPct,
    initW: initialWidthPct,
    initH: initialHeightPct,
  });

  const handlePointerDownContainer = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!containerRef.current) return;
    const rect = containerRef.current.getBoundingClientRect();
    const xPct = ((e.clientX - rect.left) / rect.width) * 100;
    const yPct = ((e.clientY - rect.top) / rect.height) * 100;

    setLeft(xPct);
    setTop(yPct);
    setWidth(2);
    setHeight(2);
    setIsDragging(true);
    setDragMode("create");
    dragStartRef.current = {
      x: e.clientX,
      y: e.clientY,
      initLeft: xPct,
      initTop: yPct,
      initW: 2,
      initH: 2,
    };
    containerRef.current.setPointerCapture(e.pointerId);
  };

  const handlePointerDownBox = (e: React.PointerEvent<HTMLDivElement>) => {
    e.stopPropagation();
    if (!containerRef.current) return;
    setIsDragging(true);
    setDragMode("move");
    dragStartRef.current = {
      x: e.clientX,
      y: e.clientY,
      initLeft: left,
      initTop: top,
      initW: width,
      initH: height,
    };
    containerRef.current.setPointerCapture(e.pointerId);
  };

  const handlePointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!isDragging || !containerRef.current) return;
    const rect = containerRef.current.getBoundingClientRect();
    const dxPct = ((e.clientX - dragStartRef.current.x) / rect.width) * 100;
    const dyPct = ((e.clientY - dragStartRef.current.y) / rect.height) * 100;

    if (dragMode === "move") {
      const newL = Math.max(
        0,
        Math.min(100 - dragStartRef.current.initW, dragStartRef.current.initLeft + dxPct),
      );
      const newT = Math.max(
        0,
        Math.min(100 - dragStartRef.current.initH, dragStartRef.current.initTop + dyPct),
      );
      setLeft(newL);
      setTop(newT);
      const updated: SpectralSelectionData = {
        left: newL,
        top: newT,
        width,
        height,
        label: currentLabel,
      };
      onChange?.(updated);
    } else if (dragMode === "create") {
      const startX = dragStartRef.current.initLeft;
      const startY = dragStartRef.current.initTop;
      const currentX = ((e.clientX - rect.left) / rect.width) * 100;
      const currentY = ((e.clientY - rect.top) / rect.height) * 100;

      const newL = Math.max(0, Math.min(startX, currentX));
      const newT = Math.max(0, Math.min(startY, currentY));
      const newW = Math.max(1, Math.abs(currentX - startX));
      const newH = Math.max(1, Math.abs(currentY - startY));

      setLeft(newL);
      setTop(newT);
      setWidth(newW);
      setHeight(newH);

      const fMin = Math.round(20 + (100 - (newT + newH)) * 199.8);
      const fMax = Math.round(20 + (100 - newT) * 199.8);
      const dur = ((newW / 100) * 10).toFixed(2);
      const newLbl = `${(fMin / 1000).toFixed(1)} to ${(fMax / 1000).toFixed(1)} kHz · ${dur} s`;
      setCurrentLabel(newLbl);

      onChange?.({
        left: newL,
        top: newT,
        width: newW,
        height: newH,
        label: newLbl,
      });
    }
  };

  const handlePointerUp = (e: React.PointerEvent<HTMLDivElement>) => {
    if (isDragging && containerRef.current) {
      setIsDragging(false);
      try {
        containerRef.current.releasePointerCapture(e.pointerId);
      } catch {
        // Ignored
      }
    }
  };

  return (
    <div
      ref={containerRef}
      className={`spectral-selection-overlay ${className}`}
      style={{
        position: "absolute",
        inset: 0,
        pointerEvents: "auto",
        cursor: "crosshair",
        ...style,
      }}
      onPointerDown={handlePointerDownContainer}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onPointerCancel={handlePointerUp}
    >
      {/* Selection Box matching design/screens-a.js:130 */}
      <div
        data-testid="spectral-box"
        style={{
          position: "absolute",
          left: `${left}%`,
          top: `${top}%`,
          width: `${width}%`,
          height: `${height}%`,
          border: "2px dashed var(--tx)",
          borderRadius: "2px",
          cursor: "move",
        }}
        onPointerDown={handlePointerDownBox}
      >
        <span
          className="mono"
          style={{
            position: "absolute",
            top: "-22px",
            right: 0,
            padding: "1px 6px",
            borderRadius: "3px",
            background: "var(--tx)",
            color: "var(--s0)",
            whiteSpace: "nowrap",
          }}
        >
          {currentLabel}
        </span>
      </div>
    </div>
  );
};
