import React from "react";

const LETTERS_EFFECTS: Record<string, boolean> = { water: true, stut: true, stop: true };

export interface EffectCardProps {
  palette: string;
  effect?: string;
  word: string;
  leftMeta: string;
  rightMeta?: string;
  href?: string;
  dataGo?: string;
  dataTheme?: string;
  ariaPressed?: boolean | "true" | "false";
  onClick?: (e: React.MouseEvent) => void;
  className?: string;
  style?: React.CSSProperties;
}

export const EffectCard: React.FC<EffectCardProps> = ({
  palette,
  effect = "",
  word,
  leftMeta,
  rightMeta,
  href,
  dataGo,
  dataTheme,
  ariaPressed,
  onClick,
  className = "",
  style,
}) => {
  const isButton = Boolean(dataTheme || onClick || !href);
  const classes = `fxc pal-${palette} ${effect ? `f-${effect}` : ""} ${className}`;
  const ariaLabel = `${leftMeta}${rightMeta ? ", " + rightMeta : ""}`;
  const hasLetterAnim = LETTERS_EFFECTS[effect];

  const renderedWord = hasLetterAnim
    ? [...word].map((char, i) => (
        <span key={i} className="l" style={{ "--i": i } as React.CSSProperties}>
          {char === " " ? "\u00A0" : char}
        </span>
      ))
    : word;

  const inner = (
    <>
      {effect === "vinyl" && <span className="disc" aria-hidden="true" />}
      <span className="meta">
        <span>{leftMeta}</span>
        <span>{rightMeta || ""}</span>
      </span>
      <span
        className="word"
        style={{ "--n": Math.max(word.length, 4) } as React.CSSProperties}
        aria-hidden="true"
      >
        {renderedWord}
      </span>
    </>
  );

  const finalHref = href || (dataGo ? `#${dataGo}` : undefined);

  if (!isButton && finalHref) {
    return (
      <a
        className={classes}
        href={finalHref}
        data-go={dataGo}
        aria-label={ariaLabel}
        style={style}
        onClick={onClick}
      >
        {inner}
      </a>
    );
  }

  return (
    <button
      type="button"
      className={classes}
      data-set-theme={dataTheme}
      data-go={dataGo}
      aria-label={ariaLabel}
      aria-pressed={ariaPressed}
      style={style}
      onClick={onClick}
    >
      {inner}
    </button>
  );
};
