import React, { useEffect, useRef } from "react";

export interface CanvasVizProps {
  type: "wave" | "spec" | "ruler" | "eq" | "meter" | "stage" | "rooms" | "slice" | "tape" | string;
  seed?: number;
  stereo?: boolean;
  step?: number;
  kind?: string;
  color?: string;
  gain?: number;
  until?: number;
  ceil?: number;
  height?: number;
  roleLabel?: string;
  className?: string;
  style?: React.CSSProperties;
}

function prng(seed: number) {
  let s = seed | 0;
  return () => {
    s = (s + 0x6d2b79f5) | 0;
    let t = Math.imul(s ^ (s >>> 15), 1 | s);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const envCache: Record<string, Float32Array> = {};
function getEnvelope(seed: number, n: number, kind: string): Float32Array {
  const key = `${seed}:${n}:${kind}`;
  if (envCache[key]) return envCache[key];

  const r = prng(seed * 977 + 13);
  const arr = new Float32Array(n);
  const p1 = r() * 9;
  const p2 = r() * 9;
  const f1 = 8 + r() * 26;
  const f2 = 60 + r() * 200;

  for (let i = 0; i < n; i++) {
    const x = i / n;
    const beat = (x * (kind === "drums" ? 64 : 32)) % 1;
    let v: number;
    if (kind === "voice") {
      const g = Math.pow(Math.max(0, Math.sin(x * f1 + p1)), 0.5);
      v = g * (0.45 + 0.4 * Math.abs(Math.sin(x * f2 + p2))) * (0.75 + 0.25 * r());
    } else if (kind === "drums") {
      v = Math.exp(-beat * 7) * 0.95 + r() * 0.06;
    } else if (kind === "bass") {
      v = (0.4 + 0.16 * Math.sin(x * f1)) * (1 - Math.exp(-beat * 3) * 0.5);
    } else if (kind === "pad") {
      v = 0.22 + 0.14 * Math.sin(x * f1 * 0.5 + p1) + 0.05 * r();
    } else {
      v =
        0.22 +
        0.36 * Math.pow(Math.abs(Math.sin(x * f1 + p1)), 1.4) +
        0.3 * Math.exp(-beat * 5) +
        0.1 * r();
    }
    arr[i] = Math.min(1, Math.max(0, v));
  }
  envCache[key] = arr;
  return arr;
}

export function drawWaveform(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  opts: {
    seed?: number;
    kind?: string;
    step?: number;
    stereo?: boolean;
    gain?: number;
    until?: number;
    color?: string;
    ceil?: number;
  },
): void {
  const docStyle =
    typeof document !== "undefined" ? getComputedStyle(document.documentElement) : null;
  const getCss = (v: string) => docStyle?.getPropertyValue(v).trim() || v;

  const n = Math.min(1600, Math.max(30, Math.floor(width / (opts.step || 2.5))));
  const e = getEnvelope(opts.seed || 1, n, opts.kind || "mix");
  const col = opts.color ? getCss(opts.color) || opts.color : getCss("--tx");

  ctx.clearRect(0, 0, width, height);
  const stereo = Boolean(opts.stereo);
  const bw = width / n;
  const gain = opts.gain || 1;
  const mid = height / 2;

  ctx.fillStyle = col;
  const until = opts.until ?? 1;

  for (let i = 0; i < n; i++) {
    if (i / n > until) ctx.globalAlpha = 0.25;
    const v = e[i] * gain;
    if (stereo) {
      const h = height * 0.23;
      const h1 = Math.max(0.5, v * h);
      const h2 = Math.max(0.5, v * h * (0.9 + 0.1 * Math.sin(i * 0.3)));
      ctx.fillRect(i * bw, height * 0.26 - h1, Math.max(0.8, bw * 0.66), h1 * 2);
      ctx.fillRect(i * bw, height * 0.74 - h2, Math.max(0.8, bw * 0.66), h2 * 2);
    } else {
      const h = Math.max(0.5, v * height * 0.46);
      ctx.fillRect(i * bw, mid - h, Math.max(0.8, bw * 0.66), h * 2);
    }
  }
  ctx.globalAlpha = 1;

  if (stereo) {
    ctx.fillStyle = getCss("--ln");
    ctx.fillRect(0, height / 2, width, 1);
  }
  if (opts.ceil) {
    ctx.fillStyle = getCss("--sig");
    const y = height * (0.5 - 0.46 * opts.ceil);
    ctx.fillRect(0, y, width, 1);
    ctx.fillRect(0, height - y, width, 1);
  }
}

export const CanvasViz: React.FC<CanvasVizProps> = ({
  type,
  seed = 1,
  stereo = false,
  step = 2.5,
  kind = "mix",
  color,
  gain = 1,
  until = 1,
  ceil,
  height,
  roleLabel,
  className = "",
  style,
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const rect = canvas.getBoundingClientRect();
    const w = Math.max(30, Math.round(rect.width));
    const h = Math.max(20, Math.round(rect.height || height || 80));

    canvas.width = w * dpr;
    canvas.height = h * dpr;
    ctx.scale(dpr, dpr);

    if (type === "wave") {
      drawWaveform(ctx, w, h, { seed, stereo, step, kind, color, gain, until, ceil });
    } else if (type === "eq") {
      // Draw simplified interactive EQ curve
      ctx.clearRect(0, 0, w, h);
      ctx.strokeStyle = "#B8350A";
      ctx.lineWidth = 2;
      ctx.beginPath();
      for (let x = 0; x < w; x++) {
        const normX = x / w;
        const y = h / 2 - Math.sin(normX * Math.PI * 2) * (h * 0.25);
        if (x === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.stroke();
    }
  }, [type, seed, stereo, step, kind, color, gain, until, ceil, height]);

  const canvasEl = (
    <canvas
      ref={canvasRef}
      data-v={type}
      data-seed={seed}
      data-stereo={stereo ? "1" : undefined}
      data-step={step}
      data-kind={kind}
      data-color={color}
      data-gain={gain}
      data-until={until}
      data-ceil={ceil}
      aria-hidden={!roleLabel}
      style={{ width: "100%", height: height ? `${height}px` : "100%", ...style }}
    />
  );

  if (height) {
    return (
      <div
        className={`vz ${className}`}
        style={{ height: `${height}px` }}
        role={roleLabel ? "img" : undefined}
        aria-label={roleLabel}
      >
        {canvasEl}
      </div>
    );
  }

  return canvasEl;
};
