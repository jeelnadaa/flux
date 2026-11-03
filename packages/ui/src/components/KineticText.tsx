import React, { useEffect, useRef } from "react";

export interface KineticTextProps {
  children: string;
  as?: "h1" | "h2" | "h3" | "div" | "span";
  variant?: "h1" | "bigh" | string;
  className?: string;
  style?: React.CSSProperties;
  playing?: boolean;
}

interface LetterInfo {
  el: HTMLSpanElement;
  index: number;
  centerPx: number;
}

export const KineticText: React.FC<KineticTextProps> = ({
  children,
  as: Component = "h1",
  variant = "h1",
  className = "",
  style,
  playing = false,
}) => {
  const rootRef = useRef<HTMLElement>(null);

  useEffect(() => {
    const el = rootRef.current;
    if (!el) return;

    const calm =
      typeof window !== "undefined" &&
      window.matchMedia &&
      window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (calm) return;

    const rootStyle = getComputedStyle(document.documentElement);
    const kinLo = parseFloat(rootStyle.getPropertyValue("--kin-lo")) || 640;
    const kinHi = parseFloat(rootStyle.getPropertyValue("--kin-hi")) || 900;

    const letterEls = el.querySelectorAll<HTMLSpanElement>(".lt");
    const letters: LetterInfo[] = [];

    const fs = parseFloat(getComputedStyle(el).fontSize) || 1;
    letterEls.forEach((lt, i) => {
      const rect = lt.getBoundingClientRect();
      lt.style.width = `${rect.width / fs}em`;
      letters.push({
        el: lt,
        index: i,
        centerPx: rect.left + rect.width / 2,
      });
    });

    let pointerX = -1e4;
    const handlePointerMove = (e: PointerEvent) => {
      pointerX = e.clientX;
    };
    window.addEventListener("pointermove", handlePointerMove, { passive: true });

    let rafId = 0;
    const tick = (ms: number) => {
      const level = playing ? 0.5 + 0.4 * Math.sin(ms / 170) : 0;
      letters.forEach((item) => {
        const v = Math.min(
          1,
          Math.max(
            0,
            (0.5 +
              0.5 *
                Math.sin(ms / 640 + item.index * 0.7) *
                Math.sin(ms / 1610 + item.index * 0.29)) *
              0.5 +
              Math.exp(-Math.pow((item.centerPx - pointerX) / 160, 2)) * 0.55 +
              level * 0.25,
          ),
        );
        item.el.style.fontWeight = `${Math.round(kinLo + v * (kinHi - kinLo))}`;
        item.el.style.transform = `scaleY(${(0.94 + v * 0.2).toFixed(3)})`;
      });
      rafId = requestAnimationFrame(tick);
    };

    rafId = requestAnimationFrame(tick);

    return () => {
      window.removeEventListener("pointermove", handlePointerMove);
      cancelAnimationFrame(rafId);
    };
  }, [children, playing]);

  const words = typeof children === "string" ? children.split(" ") : [];

  return React.createElement(
    Component,
    {
      ref: rootRef,
      className: `${variant} kin ${className}`,
      "aria-label": children,
      style,
    },
    words.map((word, wIdx) => (
      <React.Fragment key={wIdx}>
        {wIdx > 0 && " "}
        <span className="wd" aria-hidden="true">
          {[...word].map((char, cIdx) => (
            <span key={cIdx} className="lt">
              {char}
            </span>
          ))}
        </span>
      </React.Fragment>
    )),
  );
};
