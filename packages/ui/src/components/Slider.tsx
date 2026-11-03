import React, { useRef, useId, useState, useEffect } from "react";

export interface SliderProps {
  label: string;
  value?: number; // 0 to 100
  defaultValue?: number;
  valueText?: string;
  unit?: string;
  big?: boolean;
  accent?: boolean;
  onChange?: (value: number) => void;
  className?: string;
  style?: React.CSSProperties;
}

export const Slider: React.FC<SliderProps> = ({
  label,
  value,
  defaultValue = 50,
  valueText,
  unit = "%",
  big,
  accent,
  onChange,
  className = "",
  style,
}) => {
  const id = useId();
  const trackRef = useRef<HTMLDivElement>(null);
  const [internalValue, setInternalValue] = useState(value ?? defaultValue);
  const [isDragging, setIsDragging] = useState(false);

  const currentVal = value !== undefined ? value : internalValue;
  const displayVal = valueText !== undefined ? valueText : `${Math.round(currentVal)}${unit}`;

  useEffect(() => {
    if (value !== undefined) setInternalValue(value);
  }, [value]);

  const updateFromPointer = (clientX: number) => {
    const track = trackRef.current;
    if (!track) return;
    const rect = track.getBoundingClientRect();
    const pct = Math.min(100, Math.max(0, ((clientX - rect.left) / rect.width) * 100));
    setInternalValue(pct);
    onChange?.(pct);
  };

  const handlePointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    trackRef.current?.setPointerCapture(e.pointerId);
    setIsDragging(true);
    updateFromPointer(e.clientX);
  };

  const handlePointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if (isDragging) {
      updateFromPointer(e.clientX);
    }
  };

  const handlePointerUp = (e: React.PointerEvent<HTMLDivElement>) => {
    if (isDragging) {
      setIsDragging(false);
      try {
        trackRef.current?.releasePointerCapture(e.pointerId);
      } catch {
        // Pointer capture already released
      }
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
      e.preventDefault();
      const step = 2;
      const next = Math.min(100, Math.max(0, currentVal + (e.key === "ArrowRight" ? step : -step)));
      setInternalValue(next);
      onChange?.(next);
    }
  };

  const classes = ["sl", big ? "big" : "", accent ? "ac" : "", isDragging ? "dr" : "", className]
    .filter(Boolean)
    .join(" ");

  return (
    <div className={classes} style={{ "--v": `${currentVal}%`, ...style } as React.CSSProperties}>
      <div className="top">
        <label id={id}>{label}</label>
        <span className="mono dim sv">{displayVal}</span>
      </div>
      <div
        ref={trackRef}
        className="track"
        role="slider"
        tabIndex={0}
        aria-labelledby={id}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(currentVal)}
        aria-valuetext={displayVal}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onKeyDown={handleKeyDown}
      >
        <div className="rail2">
          <div className="fill" />
        </div>
        <div className="th" />
      </div>
    </div>
  );
};
