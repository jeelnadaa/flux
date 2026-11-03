import React, { useRef, useState, useLayoutEffect } from "react";

export interface SegmentedControlProps {
  items: readonly string[];
  selectedIndex?: number;
  value?: string;
  onChange?: (item: string, index: number) => void;
  ariaLabel?: string;
  dataMode?: boolean;
  className?: string;
  style?: React.CSSProperties;
}

export const SegmentedControl: React.FC<SegmentedControlProps> = ({
  items,
  selectedIndex,
  value,
  onChange,
  ariaLabel,
  dataMode,
  className = "",
  style,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const indRef = useRef<HTMLSpanElement>(null);

  const initialIndex =
    selectedIndex !== undefined
      ? selectedIndex
      : value !== undefined
        ? Math.max(0, items.indexOf(value))
        : 0;

  const [internalIndex, setInternalIndex] = useState(initialIndex);

  const activeIndex = selectedIndex !== undefined && onChange ? selectedIndex : internalIndex;

  useLayoutEffect(() => {
    const seg = containerRef.current;
    const ind = indRef.current;
    if (!seg || !ind) return;

    const onBtn = seg.querySelectorAll("button")[activeIndex];
    if (onBtn) {
      ind.style.width = `${onBtn.offsetWidth}px`;
      ind.style.transform = `translateX(${onBtn.offsetLeft}px)`;
    }
  }, [activeIndex, items]);

  const handleClick = (item: string, idx: number) => {
    setInternalIndex(idx);
    onChange?.(item, idx);
  };

  return (
    <div
      ref={containerRef}
      className={`seg ${className}`}
      role="group"
      aria-label={ariaLabel}
      data-mode={dataMode ? "" : undefined}
      style={style}
    >
      <span ref={indRef} className="ind" />
      {items.map((item, idx) => {
        const isSelected = idx === activeIndex;
        return (
          <button
            key={item}
            type="button"
            className={isSelected ? "on" : ""}
            aria-pressed={isSelected}
            onClick={() => handleClick(item, idx)}
          >
            {item}
          </button>
        );
      })}
    </div>
  );
};
