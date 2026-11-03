import React from "react";

export const ICON_PATHS = {
  home: "M3 11l9-7 9 7v9a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1z",
  folder: "M3 6a2 2 0 0 1 2-2h4l2 3h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
  wave: "M3 12h2M7 7v10M11 3v18M15 8v8M19 5v14M22 12h-1",
  spec: "M3 5h18M3 10h12M3 15h16M3 20h8",
  mix: "M6 4v8M6 16v4M12 4v3M12 11v9M18 4v10M18 18v2M4 12h4v4H4zM10 7h4v4h-4zM16 14h4v4h-4z",
  orbit: "M12 12m-3 0a3 3 0 1 0 6 0a3 3 0 1 0-6 0M12 3a9 9 0 1 0 9 9M19 4.5a1.5 1.5 0 1 0 .01 0",
  script: "M5 4h14v16H5zM8 8h8M8 12h8M8 16h5",
  master: "M12 21a9 9 0 1 1 9-9M12 12l5-5",
  fx: "M4 6h10M18 6h2M4 12h2M10 12h10M4 18h7M15 18h5M14 4v4M6 10v4M11 16v4",
  ai: "M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8zM19 16l.8 2.2L22 19l-2.2.8L19 22l-.8-2.2L16 19l2.2-.8z",
  cog: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM19.4 15a1.6 1.6 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.6 1.6 0 0 0-2.8 1.1V21a2 2 0 1 1-4 0v-.1a1.6 1.6 0 0 0-2.8-1l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.6 1.6 0 0 0-1.1-2.8H3a2 2 0 1 1 0-4h.1a1.6 1.6 0 0 0 1-2.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.6 1.6 0 0 0 1.8.3H9a1.6 1.6 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.6 1.6 0 0 0 2.8 1.1l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.6 1.6 0 0 0-.3 1.8V9a1.6 1.6 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.6 1.6 0 0 0-1.5 1z",
  help: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM9.5 9a2.5 2.5 0 1 1 3.6 2.3c-.7.4-1.1 1-1.1 1.7M12 17h.01",
  play: "M7 4l13 8-13 8z",
  pause: "M7 4h4v16H7zM14 4h4v16h-4z",
  stop: "M5 5h14v14H5z",
  rec: "M12 12m-7 0a7 7 0 1 0 14 0a7 7 0 1 0-14 0",
  loop: "M17 2l4 4-4 4M3 11V9a3 3 0 0 1 3-3h15M7 22l-4-4 4-4M21 13v2a3 3 0 0 1-3 3H3",
  back: "M19 20L9 12l10-8zM5 5v14",
  undo: "M9 14L4 9l5-5M4 9h11a5 5 0 0 1 0 10h-3",
  redo: "M15 14l5-5-5-5M20 9H9a5 5 0 0 0 0 10h3",
  exp: "M12 3v12M7 10l5 5 5-5M4 19h16",
  imp: "M12 15V3M7 8l5-5 5 5M4 19h16",
  plus: "M12 5v14M5 12h14",
  search: "M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16zM21 21l-4.3-4.3",
  x: "M6 6l12 12M18 6L6 18",
  check: "M4 12l5 5L20 6",
  cut: "M6 9a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM6 21a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM8.1 8.1L20 20M8.1 15.9L20 4",
  copy: "M9 9h11v11H9zM5 15H4V4h11v1",
  trash: "M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13",
  split: "M12 3v18M5 8l4 4-4 4M19 8l-4 4 4 4",
  fade: "M3 20L21 4v16z",
  mag: "M15 4l1.2 3.3L19.5 8.5l-3.3 1.2L15 13l-1.2-3.3-3.3-1.2 3.3-1.2zM7 13l.8 2.2L10 16l-2.2.8L7 19l-.8-2.2L4 16l2.2-.8z",
  mic: "M12 3a3 3 0 0 0-3 3v6a3 3 0 0 0 6 0V6a3 3 0 0 0-3-3zM5 11a7 7 0 0 0 14 0M12 18v3",
  head: "M4 15v-3a8 8 0 0 1 16 0v3M4 15a2 2 0 0 1 2-2h1v6H6a2 2 0 0 1-2-2zM20 15a2 2 0 0 0-2-2h-1v6h1a2 2 0 0 0 2-2z",
  lock: "M6 10h12v11H6zM8 10V7a4 4 0 0 1 8 0v3",
  cloud: "M7 18a5 5 0 1 1 1-9.9A6 6 0 0 1 19.5 10 4 4 0 0 1 18 18z",
  chip: "M7 7h10v10H7zM10 3v4M14 3v4M10 17v4M14 17v4M3 10h4M3 14h4M17 10h4M17 14h4",
  warn: "M12 3l10 18H2zM12 10v5M12 18h.01",
  info: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 11v5M12 8h.01",
  drop: "M12 3s7 7 7 12a7 7 0 0 1-14 0c0-5 7-12 7-12z",
  room: "M3 21V8l9-5 9 5v13M9 21v-7h6v7",
  voice: "M4 10v4M8 7v10M12 4v16M16 7v10M20 10v4",
  stem: "M4 6h16M4 10h16M4 14h16M4 18h16",
  text: "M5 5h14M12 5v14M9 19h6",
  globe:
    "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM3 12h18M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18",
  bell: "M6 9a6 6 0 0 1 12 0c0 7 3 8 3 8H3s3-1 3-8M10 21h4",
  keys: "M3 6h18v13H3zM7 10h.01M11 10h.01M15 10h.01M7 15h10",
  box: "M3 8l9-5 9 5v8l-9 5-9-5zM3 8l9 5 9-5M12 13v8",
  batch: "M4 6h16M4 12h16M4 18h10M18 16l3 2-3 2z",
  ring: "M6 9a6 6 0 0 1 12 0c0 7 3 8 3 8H3s3-1 3-8",
  hist: "M3 12a9 9 0 1 0 3-6.7L3 8M3 3v5h5M12 7v5l3 2",
  swap: "M4 8h13l-3-3M20 16H7l3 3",
  dots: "M5 12h.01M12 12h.01M19 12h.01",
  list: "M8 6h13M8 12h13M8 18h13M4 6h.01M4 12h.01M4 18h.01",
  pen: "M4 20l1-5L16 4l4 4L9 19zM14 6l4 4",
  zoom: "M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16zM21 21l-4.3-4.3M8 11h6M11 8v6",
  phone: "M8 3h8v18H8zM11 18h2",
  disk: "M4 4h13l3 3v13H4zM8 4v6h8V4M8 20v-6h8v6",
  user: "M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM4 21a8 8 0 0 1 16 0",
  a11y: "M12 6a2 2 0 1 0 0-4 2 2 0 0 0 0 4zM4 8l8 2 8-2M12 10v5l-3 6M12 15l3 6",
  tag: "M3 12V4h8l10 10-8 8z",
  bug: "M8 8a4 4 0 0 1 8 0v8a4 4 0 0 1-8 0zM4 13h4M16 13h4M5 7l3 2M19 7l-3 2M5 19l3-2M19 19l-3-2",
} as const;

export type IconName = keyof typeof ICON_PATHS;

export interface IconProps {
  name: IconName | string;
  size?: number;
  className?: string;
  style?: React.CSSProperties;
  ariaHidden?: boolean;
}

export const Icon: React.FC<IconProps> = ({ name, size, className, style, ariaHidden = true }) => {
  const path = ICON_PATHS[name as IconName] || ICON_PATHS.dots;
  const sizeStyle = size ? { width: `${size}px`, height: `${size}px` } : undefined;

  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.9"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden={ariaHidden}
      className={className}
      style={{ ...sizeStyle, ...style }}
    >
      <path d={path} />
    </svg>
  );
};

export function getIconSvg(name: IconName | string, size?: number): string {
  const path = ICON_PATHS[name as IconName] || ICON_PATHS.dots;
  const sz = size ? ` style="width:${size}px;height:${size}px"` : "";
  return `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"${sz}><path d="${path}"/></svg>`;
}
