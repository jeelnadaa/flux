import React, { useEffect, useRef } from "react";

export interface CanvasVizProps {
  type:
    | "wave"
    | "spec"
    | "ruler"
    | "eq"
    | "meter"
    | "stage"
    | "rooms"
    | "slice"
    | "tape"
    | "transfer"
    | "gr"
    | "lufs"
    | string;
  seed?: number;
  stereo?: boolean | string;
  step?: number;
  kind?: string;
  color?: string;
  gain?: number;
  until?: number;
  ceil?: number;
  height?: number;
  roleLabel?: string;
  on?: boolean;
  th?: number;
  ratio?: number;
  mode?: string;
  tool?: string;
  rt?: number;
  w?: number;
  d?: number;
  h?: number;
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

const canvasRenderMap = new WeakMap<HTMLCanvasElement, () => void>();
let sharedResizeObserver: ResizeObserver | null = null;
const pendingResizeTasks = new Set<() => void>();
let resizeRafId: number | null = null;

function getSharedResizeObserver(): ResizeObserver | null {
  if (typeof ResizeObserver === "undefined") return null;
  if (!sharedResizeObserver) {
    sharedResizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const fn = canvasRenderMap.get(entry.target as HTMLCanvasElement);
        if (fn) {
          pendingResizeTasks.add(fn);
        }
      }
      if (resizeRafId === null && pendingResizeTasks.size > 0) {
        resizeRafId = requestAnimationFrame(() => {
          resizeRafId = null;
          pendingResizeTasks.forEach((fn) => fn());
          pendingResizeTasks.clear();
        });
      }
    });
  }
  return sharedResizeObserver;
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
    on?: boolean;
  },
): void {
  const docStyle =
    typeof document !== "undefined" ? getComputedStyle(document.documentElement) : null;
  const getCss = (v: string) => docStyle?.getPropertyValue(v).trim() || v;

  const n = Math.min(1600, Math.max(30, Math.floor(width / (opts.step || 2.5))));
  const e = getEnvelope(opts.seed || 1, n, opts.kind || "mix");
  const col = opts.color
    ? getCss(opts.on ? `${opts.color}-on` : opts.color) || opts.color
    : getCss("--tx");

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

const specCache: Record<string, HTMLCanvasElement> = {};
const rgb = (h: string) => [
  parseInt(h.slice(1, 3), 16),
  parseInt(h.slice(3, 5), 16),
  parseInt(h.slice(5, 7), 16),
];
const heat = (v: number, A: number[], B: number[], C: number[]) => {
  v = Math.min(1, Math.max(0, v * 1.5));
  const [p, q, k] = v < 0.55 ? [A, B, v / 0.55] : [B, C, (v - 0.55) / 0.45];
  return [p[0] + (q[0] - p[0]) * k, p[1] + (q[1] - p[1]) * k, p[2] + (q[2] - p[2]) * k];
};

export function drawSpectrogram(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  seed = 1,
): void {
  const docStyle =
    typeof document !== "undefined" ? getComputedStyle(document.documentElement) : null;
  const getCss = (v: string) => docStyle?.getPropertyValue(v).trim() || v;
  const g = getCss("--ground") || "#f3ecdc";
  const k = getCss("--ink") || "#221c15";
  const dark = rgb(k).reduce((a, b) => a + b) < rgb(g).reduce((a, b) => a + b);
  const ramp = [dark ? k : g, getCss("--sig-i") || "#8f2203", dark ? g : k];
  const key = `sp${seed}_${Math.round(width / 3)}_${Math.round(height / 3)}${ramp.join("")}`;

  let c = specCache[key];
  if (!c) {
    const w = Math.max(40, Math.round(width / 3));
    const h = Math.max(20, Math.round(height / 3));
    c = document.createElement("canvas");
    c.width = w;
    c.height = h;
    const q = c.getContext("2d");
    if (q) {
      const im = q.createImageData(w, h);
      const r = prng(seed * 91);
      const e1 = getEnvelope(seed, w, "voice");
      const e2 = getEnvelope(seed + 1, w, "drums");
      const e3 = getEnvelope(seed + 2, w, "bass");

      for (let xi = 0; xi < w; xi++) {
        for (let y = 0; y < h; y++) {
          const f = 1 - y / h;
          const t = xi / w;
          const v =
            e3[xi] * Math.exp(-f * 13) +
            e2[xi] * Math.exp(-f * 3.4) * 0.8 +
            e1[xi] *
              Math.pow(Math.max(0, Math.sin(f * 62 + Math.sin(t * 90) * 2.6)), 8) *
              Math.exp(-f * 1.7) +
            r() * 0.03 +
            0.1 * Math.exp(-f * 2.2);

          const [cr, cg, cb] = heat(v, rgb(ramp[0]), rgb(ramp[1]), rgb(ramp[2]));
          const idx = (y * w + xi) * 4;
          im.data[idx] = cr;
          im.data[idx + 1] = cg;
          im.data[idx + 2] = cb;
          im.data[idx + 3] = 255;
        }
      }
      q.putImageData(im, 0, 0);
      specCache[key] = c;
    }
  }

  ctx.imageSmoothingEnabled = true;
  ctx.drawImage(c, 0, 0, width, height);
}

export function drawSpectrum(ctx: CanvasRenderingContext2D, width: number, height: number): void {
  const docStyle =
    typeof document !== "undefined" ? getComputedStyle(document.documentElement) : null;
  const getCss = (v: string) => docStyle?.getPropertyValue(v).trim() || v;
  ctx.clearRect(0, 0, width, height);
  const n = Math.floor(width / 5);
  for (let i = 0; i < n; i++) {
    const u = i / n;
    const v = Math.min(
      1,
      Math.max(0, 0.82 - u * 0.62 + Math.sin(u * 19) * 0.1 + Math.sin(u * 53) * 0.07),
    );
    ctx.fillStyle = getCss("--ac-t") || "#c33";
    const h = v * height;
    ctx.fillRect(i * 5, height - h, 4, h);
  }
}

export function drawLufs(ctx: CanvasRenderingContext2D, width: number, height: number): void {
  const docStyle =
    typeof document !== "undefined" ? getComputedStyle(document.documentElement) : null;
  const getCss = (v: string) => docStyle?.getPropertyValue(v).trim() || v;
  ctx.clearRect(0, 0, width, height);
  const n = Math.floor(width / 3);
  const e = getEnvelope(9, n, "mix");
  ctx.beginPath();
  for (let i = 0; i < e.length; i++) {
    let sum = 0;
    const start = Math.max(0, i - 8);
    const count = i - start + 1;
    for (let k = start; k <= i; k++) {
      sum += e[k];
    }
    const s = sum / count;
    const y = height * 0.85 - s * height * 0.8;
    if (i === 0) {
      ctx.moveTo(0, y);
    } else {
      ctx.lineTo(i * 3, y);
    }
  }
  ctx.strokeStyle = getCss("--ac-t");
  ctx.lineWidth = 2;
  ctx.stroke();

  ctx.strokeStyle = getCss("--sig");
  ctx.setLineDash([4, 4]);
  ctx.beginPath();
  ctx.moveTo(0, height * 0.42);
  ctx.lineTo(width, height * 0.42);
  ctx.stroke();
  ctx.setLineDash([]);

  ctx.fillStyle = getCss("--tx3");
  ctx.font = '500 10.5px "Geist Mono",monospace';
  ctx.fillText("Target −14 LUFS", 6, height * 0.42 - 5);
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
  on = false,
  th,
  ratio,
  mode,
  tool,
  rt,
  w,
  d,
  h,
  className = "",
  style,
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const render = () => {
      const cv = canvasRef.current;
      if (!cv) return;
      const ctx = cv.getContext("2d");
      if (!ctx) return;

      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      const rect = cv.getBoundingClientRect();
      if (rect.width < 2 || rect.height < 2) return;

      const w = Math.round(rect.width * dpr);
      const h = Math.round(rect.height * dpr);

      if (cv.width !== w || cv.height !== h) {
        cv.width = w;
        cv.height = h;
      }
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

      const rw = rect.width;
      const rh = rect.height;

      if (type === "wave") {
        drawWaveform(ctx, rw, rh, {
          seed,
          stereo: Boolean(stereo),
          step,
          kind,
          color,
          gain,
          until,
          ceil,
          on,
        });
      } else if (type === "spec") {
        drawSpectrogram(ctx, rw, rh, seed);
      } else if (type === "spectrum") {
        drawSpectrum(ctx, rw, rh);
      } else if (type === "lufs") {
        drawLufs(ctx, rw, rh);
      } else if (type === "gonio") {
        ctx.clearRect(0, 0, rw, rh);
        const c = Math.min(rw, rh) / 2 - 8;
        const cx = rw / 2;
        const cy = rh / 2;
        ctx.strokeStyle = "var(--ln)";
        ctx.beginPath();
        ctx.moveTo(cx - c, cy);
        ctx.lineTo(cx + c, cy);
        ctx.moveTo(cx, cy - c);
        ctx.lineTo(cx, cy + c);
        ctx.stroke();

        ctx.fillStyle = "var(--ac-t)";
        ctx.globalAlpha = 0.55;
        const r = prng(42);
        for (let i = 0; i < 420; i++) {
          const a = r() + r() + r() - 1.5;
          const b = (r() + r() - 1) * 0.36;
          const px = cx + b * c * 1.4;
          const py = cy - a * c * 0.58;
          ctx.fillRect(px, py, 1.5, 1.5);
        }
        ctx.globalAlpha = 1;
      } else if (type === "history") {
        ctx.clearRect(0, 0, rw, rh);
        ctx.lineCap = "round";
        const ys = [rh * 0.5, rh * 0.22, rh * 0.78];
        const docStyle =
          typeof document !== "undefined" ? getComputedStyle(document.documentElement) : null;
        const getCss = (v: string) => docStyle?.getPropertyValue(v).trim() || v;

        ctx.strokeStyle = getCss("--ln2");
        ctx.lineWidth = 2;
        const stepsN = 15;
        const st = rw / (stepsN + 0.6);
        ctx.beginPath();
        ctx.moveTo(st * 0.5, ys[0]);
        ctx.lineTo(st * (stepsN - 2), ys[0]);
        ctx.stroke();

        ctx.beginPath();
        ctx.moveTo(st * 4.5, ys[0]);
        ctx.bezierCurveTo(st * 5, ys[0], st * 5, ys[1], st * 5.6, ys[1]);
        ctx.lineTo(st * 9.5, ys[1]);
        ctx.stroke();

        ctx.beginPath();
        ctx.moveTo(st * 8.5, ys[0]);
        ctx.bezierCurveTo(st * 9, ys[0], st * 9, ys[2], st * 9.6, ys[2]);
        ctx.lineTo(st * 12.5, ys[2]);
        ctx.stroke();

        for (let i = 0; i < stepsN - 2; i++) {
          ctx.beginPath();
          ctx.arc(st * (i + 0.5), ys[0], i === 11 ? 8 : 5, 0, 7);
          ctx.fillStyle = i === 11 ? getCss("--sig") : getCss("--tx2");
          ctx.fill();
        }
        for (let i = 5; i < 10; i++) {
          ctx.beginPath();
          ctx.arc(st * (i + 0.6), ys[1], 4.5, 0, 7);
          ctx.fillStyle = getCss("--t1");
          ctx.fill();
        }
        for (let i = 9; i < 13; i++) {
          ctx.beginPath();
          ctx.arc(st * (i + 0.6), ys[2], 4.5, 0, 7);
          ctx.fillStyle = getCss("--t3");
          ctx.fill();
        }
      } else if (type === "eq") {
        const docStyle =
          typeof document !== "undefined" ? getComputedStyle(document.documentElement) : null;
        const getCss = (v: string) => docStyle?.getPropertyValue(v).trim() || v;

        ctx.clearRect(0, 0, rw, rh);

        // Frequency and dB grid
        const logX = (f: number, W: number) => (Math.log10(f / 20) / Math.log10(1000)) * W;
        ctx.strokeStyle = getCss("--ln");
        ctx.lineWidth = 1;
        ctx.fillStyle = getCss("--tx3");
        ctx.font = '500 10.5px "Geist Mono",monospace';

        [20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000].forEach((f) => {
          const px = logX(f, rw);
          ctx.beginPath();
          ctx.moveTo(px, 0);
          ctx.lineTo(px, rh);
          ctx.stroke();
          if (f !== 20) {
            ctx.fillText(f >= 1000 ? `${f / 1000}k` : `${f}`, Math.min(px + 3, rw - 22), rh - 6);
          }
        });

        [-18, -12, -6, 0, 6, 12, 18].forEach((d) => {
          const y = rh / 2 - (d / 24) * rh;
          ctx.globalAlpha = d === 0 ? 1 : 0.5;
          ctx.beginPath();
          ctx.moveTo(0, y);
          ctx.lineTo(rw, y);
          ctx.stroke();
          ctx.globalAlpha = 1;
          ctx.fillText((d > 0 ? "+" : "") + d, 4, y - 3);
        });

        // Live spectrum overlay
        const n = Math.floor(rw / 4);
        ctx.fillStyle = getCss("--ac-t");
        ctx.globalAlpha = 0.22;
        const t = (cv as unknown as { _t?: number })._t || 0;
        for (let i = 0; i < n; i++) {
          const u = i / n;
          const v =
            (Math.sin(u * 16 + t * 1.4) * 0.13 +
              Math.sin(u * 47 - t * 0.9) * 0.08 +
              0.6 -
              u * 0.52) *
            (0.92 + 0.08 * Math.sin(t * 6 + i));
          const h = Math.min(1, Math.max(0, v)) * rh * 0.72;
          ctx.fillRect(i * 4, rh - h, 3, h);
        }
        ctx.globalAlpha = 1;

        // Interactive or default curve nodes
        interface EqNode {
          f: number;
          g: number;
          q: number;
          c: string;
        }
        const cvExt = cv as unknown as { _nodes?: EqNode[]; _eqd?: boolean };
        const nodes =
          cvExt._nodes ||
          (cvExt._nodes = [
            { f: 70, g: 3.5, q: 0.8, c: "--t3" },
            { f: 280, g: -3, q: 1.3, c: "--t2" },
            { f: 1200, g: 1.6, q: 1, c: "--t4" },
            { f: 3400, g: 4, q: 1.5, c: "--t5" },
            { f: 11000, g: 3, q: 0.7, c: "--t1" },
          ]);

        const resp = (u: number) => {
          let s = 0;
          const f = 20 * Math.pow(1000, u);
          nodes.forEach((node) => {
            const d = Math.log2(f / node.f) * node.q * 1.3;
            s += node.g * Math.exp(-d * d);
          });
          return s;
        };

        // Curve path
        ctx.beginPath();
        for (let px = 0; px <= rw; px += 2) {
          const y = rh / 2 - (resp(px / rw) / 24) * rh;
          if (px === 0) ctx.moveTo(px, y);
          else ctx.lineTo(px, y);
        }
        ctx.lineWidth = 2.5;
        ctx.strokeStyle = getCss("--tx");
        ctx.stroke();

        ctx.lineTo(rw, rh / 2);
        ctx.lineTo(0, rh / 2);
        ctx.closePath();
        ctx.fillStyle = getCss("--tx");
        ctx.globalAlpha = 0.08;
        ctx.fill();
        ctx.globalAlpha = 1;

        // Draw numbered band nodes
        nodes.forEach((node, i) => {
          const px = logX(node.f, rw);
          const y = rh / 2 - (node.g / 24) * rh;
          ctx.beginPath();
          ctx.arc(px, y, 11, 0, Math.PI * 2);
          ctx.fillStyle = getCss(node.c);
          ctx.fill();
          ctx.fillStyle = getCss(`${node.c}-on`);
          ctx.font = `800 11px ${getCss("--sans")}`;
          ctx.textAlign = "center";
          ctx.textBaseline = "middle";
          ctx.fillText(`${i + 1}`, px, y + 0.5);
          ctx.textAlign = "left";
          ctx.textBaseline = "alphabetic";
        });

        if (!cvExt._eqd) {
          cvExt._eqd = true;
          let dr: EqNode | null = null;
          cv.style.touchAction = "none";
          cv.addEventListener("pointerdown", (e: PointerEvent) => {
            const r = cv.getBoundingClientRect();
            const mx = e.clientX - r.left;
            const my = e.clientY - r.top;
            let best: EqNode | null = null;
            let bd = 28;
            nodes.forEach((node) => {
              const d = Math.hypot(
                logX(node.f, r.width) - mx,
                r.height / 2 - (node.g / 24) * r.height - my,
              );
              if (d < bd) {
                bd = d;
                best = node;
              }
            });
            dr = best;
            if (dr) cv.setPointerCapture(e.pointerId);
          });
          cv.addEventListener("pointermove", (e: PointerEvent) => {
            if (!dr) return;
            const r = cv.getBoundingClientRect();
            const u = Math.min(1, Math.max(0, (e.clientX - r.left) / r.width));
            dr.f = 20 * Math.pow(1000, u);
            dr.g = Math.min(
              18,
              Math.max(-18, ((r.height / 2 - (e.clientY - r.top)) / r.height) * 24),
            );
            render();
          });
          window.addEventListener("pointerup", () => {
            dr = null;
          });
        }
      } else if (type === "transfer") {
        const docStyle =
          typeof document !== "undefined" ? getComputedStyle(document.documentElement) : null;
        const getCss = (v: string) => docStyle?.getPropertyValue(v).trim() || v;

        ctx.clearRect(0, 0, rw, rh);
        ctx.strokeStyle = getCss("--ln");
        for (let i = 1; i < 6; i++) {
          ctx.beginPath();
          ctx.moveTo((i * rw) / 6, 0);
          ctx.lineTo((i * rw) / 6, rh);
          ctx.moveTo(0, (i * rh) / 6);
          ctx.lineTo(rw, (i * rh) / 6);
          ctx.stroke();
        }
        ctx.strokeStyle = getCss("--tx3");
        ctx.setLineDash([3, 4]);
        ctx.beginPath();
        ctx.moveTo(0, rh);
        ctx.lineTo(rw, 0);
        ctx.stroke();
        ctx.setLineDash([]);

        const thVal = th ?? 0.62;
        const raVal = ratio ?? 3.2;
        ctx.beginPath();
        for (let i = 0; i <= 100; i++) {
          const u = i / 100;
          const k = 0.09;
          const v =
            u < thVal - k
              ? u
              : u > thVal + k
                ? thVal + (u - thVal) / raVal
                : u + ((1 / raVal - 1) * Math.pow(u - thVal + k, 2)) / (4 * k);
          const px = u * rw;
          const py = rh - v * rh;
          if (i === 0) ctx.moveTo(px, py);
          else ctx.lineTo(px, py);
        }
        ctx.lineWidth = 2.5;
        ctx.strokeStyle = getCss("--tx");
        ctx.stroke();

        ctx.beginPath();
        ctx.arc(thVal * rw, rh - thVal * rh, 6, 0, Math.PI * 2);
        ctx.fillStyle = getCss("--sig");
        ctx.fill();
        ctx.fillStyle = getCss("--tx3");
        ctx.font = '500 10.5px "Geist Mono",monospace';
        ctx.fillText("Input", rw - 38, rh - 6);
        ctx.fillText("Output", 5, 13);
      } else if (type === "gr") {
        const docStyle =
          typeof document !== "undefined" ? getComputedStyle(document.documentElement) : null;
        const getCss = (v: string) => docStyle?.getPropertyValue(v).trim() || v;

        ctx.clearRect(0, 0, rw, rh);
        const n = Math.floor(rw / 3);
        const e = getEnvelope(4, 800, "mix");
        const t = (cv as unknown as { _t?: number })._t || 0;
        for (let i = 0; i < n; i++) {
          const idx = Math.floor(i + t * 26) % 800;
          const v = e[idx];
          const gr = Math.min(1, Math.max(0, (v - 0.4) * 1.5));
          ctx.fillStyle = getCss("--tx2");
          ctx.globalAlpha = 0.55;
          const h = v * rh * 0.8;
          ctx.fillRect(i * 3, rh - h, 2, h);
          ctx.globalAlpha = 1;
          ctx.fillStyle = getCss("--sig");
          ctx.fillRect(i * 3, 0, 2, gr * rh * 0.34);
        }
      } else if (type === "stage") {
        const docStyle =
          typeof document !== "undefined" ? getComputedStyle(document.documentElement) : null;
        const getCss = (v: string) => docStyle?.getPropertyValue(v).trim() || v;

        interface StageSource {
          az: number;
          r: number;
          c: string;
          n: string;
          fix?: number;
          _x?: number;
          _y?: number;
          tr?: [number, number][];
        }

        interface StageState {
          mode: string;
          a: number;
          src: StageSource[];
          path: [number, number][] | null;
          trail: [number, number][];
          c?: number;
          cx?: number;
          cy?: number;
        }

        const cvExt = cv as unknown as { _st?: StageState; _dr?: boolean };
        const st =
          cvExt._st ||
          (cvExt._st = {
            mode: mode || "orbit",
            a: 0,
            src: [
              { az: 0, r: 0.72, c: "--t4", n: "Vocals", fix: 1 },
              { az: 2.2, r: 0.85, c: "--t3", n: "Drums" },
              { az: 4.1, r: 0.55, c: "--t2", n: "Bass", fix: 1 },
              { az: 5.4, r: 0.9, c: "--t5", n: "Other" },
            ],
            path: null,
            trail: [],
          });

        const m = cv.dataset.mode || mode || st.mode;
        ctx.clearRect(0, 0, rw, rh);
        const c = Math.min(rw, rh) / 2 - 22;
        const cx = rw / 2;
        const cy = rh / 2;
        st.c = c;
        st.cx = cx;
        st.cy = cy;

        ctx.strokeStyle = getCss("--ln2");
        ctx.lineWidth = 1;
        [0.33, 0.66, 1].forEach((r) => {
          ctx.beginPath();
          ctx.arc(cx, cy, c * r, 0, Math.PI * 2);
          ctx.stroke();
        });

        for (let d = 0; d < 360; d += 30) {
          const a = (d * Math.PI) / 180;
          const l = d % 90 === 0 ? 12 : 6;
          ctx.beginPath();
          ctx.moveTo(cx + Math.sin(a) * c, cy - Math.cos(a) * c);
          ctx.lineTo(cx + Math.sin(a) * (c - l), cy - Math.cos(a) * (c - l));
          ctx.stroke();
        }

        ctx.fillStyle = getCss("--tx3");
        ctx.font = '500 10.5px "Geist Mono",monospace';
        ctx.textAlign = "center";
        ctx.fillText("Front", cx, cy - c - 7);
        ctx.fillText("Back", cx, cy + c + 15);
        ctx.textAlign = "left";
        ctx.fillText("L", cx - c - 14, cy + 4);
        ctx.fillText("R", cx + c + 6, cy + 4);

        const pos = (s: StageSource, i: number): [number, number] => {
          if (s.fix || m === "place") return [s.az, s.r];
          const a = st.a * (i === 1 ? 1 : -0.65) + s.az;
          if (st.path && st.path.length > 4 && i === 1) {
            const k = Math.floor(((((st.a * 0.13) % 1) + 1) % 1) * st.path.length);
            const p = st.path[k];
            return [Math.atan2(p[0], -p[1]), Math.hypot(p[0], p[1])];
          }
          if (m === "spiral") return [a, s.r * (0.55 + 0.45 * Math.sin(st.a * 0.6 + i))];
          if (m === "eight")
            return [
              Math.atan2(Math.sin(a), -Math.sin(a * 2) * 0.6),
              Math.hypot(Math.sin(a), Math.sin(a * 2) * 0.6) * 0.9,
            ];
          if (m === "swarm") {
            const b = st.a * 1.3 + s.az * 3;
            return [
              Math.atan2(Math.sin(b) * 0.7 + Math.sin(b * 2.9) * 0.2, Math.cos(b * 1.7) * 0.7),
              0.3 + 0.6 * Math.abs(Math.sin(b * 0.8)),
            ];
          }
          return [a, s.r];
        };

        if (st.path && st.path.length > 1) {
          ctx.beginPath();
          st.path.forEach((p, i) => {
            const px = cx + p[0] * c;
            const py = cy + p[1] * c;
            if (i) ctx.lineTo(px, py);
            else ctx.moveTo(px, py);
          });
          ctx.strokeStyle = getCss("--sig");
          ctx.lineWidth = 2;
          ctx.setLineDash([5, 5]);
          ctx.stroke();
          ctx.setLineDash([]);
        }

        st.src.forEach((s, i) => {
          const [az, r] = pos(s, i);
          const px = cx + Math.sin(az) * r * c;
          const py = cy - Math.cos(az) * r * c;
          const col = getCss(s.c);
          s._x = px;
          s._y = py;
          if (!s.fix && m !== "place") {
            const tr = s.tr || (s.tr = []);
            tr.push([px, py]);
            if (tr.length > 46) tr.shift();
            tr.forEach((q, k) => {
              ctx.globalAlpha = (k / tr.length) * 0.5;
              ctx.fillStyle = col;
              ctx.beginPath();
              ctx.arc(q[0], q[1], 1.5 + (k / tr.length) * 6, 0, Math.PI * 2);
              ctx.fill();
            });
            ctx.globalAlpha = 1;
          }
          ctx.beginPath();
          ctx.arc(px, py, 14, 0, Math.PI * 2);
          ctx.fillStyle = col;
          ctx.fill();
          ctx.fillStyle = getCss(`${s.c}-on`);
          ctx.font = `800 11px ${getCss("--sans")}`;
          ctx.textAlign = "center";
          ctx.textBaseline = "middle";
          ctx.fillText(s.n[0], px, py + 0.5);
          ctx.textAlign = "left";
          ctx.textBaseline = "alphabetic";
        });

        // Center head
        const hr = c * 0.12;
        ctx.beginPath();
        ctx.arc(cx, cy, hr, 0, Math.PI * 2);
        ctx.fillStyle = getCss("--s3");
        ctx.fill();
        ctx.strokeStyle = getCss("--tx");
        ctx.lineWidth = 2;
        ctx.stroke();

        // Nose
        ctx.beginPath();
        ctx.moveTo(cx, cy - hr - 9);
        ctx.lineTo(cx - 7, cy - hr + 1);
        ctx.lineTo(cx + 7, cy - hr + 1);
        ctx.closePath();
        ctx.fillStyle = getCss("--tx");
        ctx.fill();

        // Ears
        ctx.fillRect(cx - hr - 4, cy - 6, 4, 12);
        ctx.fillRect(cx + hr, cy - 6, 4, 12);

        if (!cvExt._dr) {
          cvExt._dr = true;
          cv.style.touchAction = "none";
          let drag: StageSource | null = null;
          let draw = false;
          cv.addEventListener("pointerdown", (e: PointerEvent) => {
            const r = cv.getBoundingClientRect();
            const mx = e.clientX - r.left;
            const my = e.clientY - r.top;
            if (cv.dataset.tool === "draw") {
              draw = true;
              st.path = [[(mx - (st.cx ?? cx)) / (st.c ?? c), (my - (st.cy ?? cy)) / (st.c ?? c)]];
              st.src[1].fix = 0;
              cv.setPointerCapture(e.pointerId);
              return;
            }
            drag = st.src.find((s) => Math.hypot((s._x ?? 0) - mx, (s._y ?? 0) - my) < 24) || null;
            if (drag) {
              drag.fix = 1;
              cv.setPointerCapture(e.pointerId);
            }
          });
          cv.addEventListener("pointermove", (e: PointerEvent) => {
            const r = cv.getBoundingClientRect();
            const mx = e.clientX - r.left - (st.cx ?? cx);
            const my = e.clientY - r.top - (st.cy ?? cy);
            if (draw) {
              const q: [number, number] = [mx / (st.c ?? c), my / (st.c ?? c)];
              const l = st.path ? st.path[st.path.length - 1] : null;
              if (!l || Math.hypot(q[0] - l[0], q[1] - l[1]) > 0.025) {
                if (!st.path) st.path = [];
                st.path.push(q);
              }
              render();
              return;
            }
            if (drag) {
              drag.az = Math.atan2(mx, -my);
              drag.r = Math.min(1, Math.max(0.16, Math.hypot(mx, my) / (st.c ?? c)));
              render();
            }
          });
          window.addEventListener("pointerup", () => {
            drag = null;
            draw = false;
          });
        }
      } else if (type === "elev") {
        const docStyle =
          typeof document !== "undefined" ? getComputedStyle(document.documentElement) : null;
        const getCss = (v: string) => docStyle?.getPropertyValue(v).trim() || v;

        ctx.clearRect(0, 0, rw, rh);
        ctx.strokeStyle = getCss("--ln2");
        ctx.beginPath();
        ctx.moveTo(0, rh / 2);
        ctx.lineTo(rw, rh / 2);
        ctx.stroke();

        ctx.fillStyle = getCss("--tx3");
        ctx.font = '500 10.5px "Geist Mono",monospace';
        ctx.fillText("Above", 6, 13);
        ctx.fillText("Below", 6, rh - 5);

        const cx = rw / 2;
        ctx.beginPath();
        ctx.arc(cx, rh / 2, 11, 0, Math.PI * 2);
        ctx.strokeStyle = getCss("--tx");
        ctx.lineWidth = 2;
        ctx.stroke();

        const t = (cv as unknown as { _t?: number })._t || 0;
        const y = rh / 2 - Math.sin(t * 0.5) * rh * 0.32;
        const px = cx + Math.cos(t * 0.75) * rw * 0.3;
        ctx.beginPath();
        ctx.arc(px, y, 8, 0, Math.PI * 2);
        ctx.fillStyle = getCss("--t3");
        ctx.fill();
      } else if (type === "decay") {
        const docStyle =
          typeof document !== "undefined" ? getComputedStyle(document.documentElement) : null;
        const getCss = (v: string) => docStyle?.getPropertyValue(v).trim() || v;

        ctx.clearRect(0, 0, rw, rh);
        const r = prng(11);
        const rtVal = (rt !== undefined ? rt : 2.4) / 8;
        ctx.fillStyle = getCss("--ac-t");
        for (let i = 0; i < rw; i += 2) {
          const u = i / rw;
          const v = Math.exp((-u / rtVal) * 2.4) * (0.5 + 0.5 * r()) * (u < 0.03 ? u / 0.03 : 1);
          const barH = v * rh * 0.9;
          ctx.fillRect(i, rh - barH, 1.4, barH);
        }
      } else if (type === "room") {
        const docStyle =
          typeof document !== "undefined" ? getComputedStyle(document.documentElement) : null;
        const getCss = (v: string) => docStyle?.getPropertyValue(v).trim() || v;

        ctx.clearRect(0, 0, rw, rh);
        const t = (cv as unknown as { _t?: number })._t || 0;
        const roomW = w !== undefined ? w : 1.3;
        const roomD = d !== undefined ? d : 1.0;
        const roomH = h !== undefined ? h : 0.6;
        const yaw = -0.72 + Math.sin(t * 0.22) * 0.12;
        const pitch = 0.46;
        const S = Math.min(rw, rh) * 0.34;

        const pj = (X: number, Y: number, Z: number): [number, number] => {
          const cy = Math.cos(yaw),
            sy = Math.sin(yaw);
          const x1 = X * cy - Z * sy,
            z1 = X * sy + Z * cy;
          const cp = Math.cos(pitch),
            sp = Math.sin(pitch);
          const y1 = Y * cp - z1 * sp,
            z2 = Y * sp + z1 * cp;
          const f = 3.4 / (3.4 + z2);
          return [rw / 2 + x1 * f * S, rh * 0.58 - y1 * f * S];
        };

        const C: [number, number, number][] = [
          [-roomW, 0, -roomD],
          [roomW, 0, -roomD],
          [roomW, 0, roomD],
          [-roomW, 0, roomD],
          [-roomW, 2 * roomH, -roomD],
          [roomW, 2 * roomH, -roomD],
          [roomW, 2 * roomH, roomD],
          [-roomW, 2 * roomH, roomD],
        ];
        const P = C.map((c) => pj(...c));
        const line = (a: number, b: number) => {
          ctx.beginPath();
          ctx.moveTo(P[a][0], P[a][1]);
          ctx.lineTo(P[b][0], P[b][1]);
          ctx.stroke();
        };

        ctx.fillStyle = getCss("--ac-soft");
        ctx.beginPath();
        [0, 1, 2, 3].forEach((k, i) =>
          i ? ctx.lineTo(P[k][0], P[k][1]) : ctx.moveTo(P[k][0], P[k][1]),
        );
        ctx.closePath();
        ctx.fill();

        ctx.strokeStyle = getCss("--tx2");
        ctx.lineWidth = 1.5;
        [
          [0, 1],
          [1, 2],
          [2, 3],
          [3, 0],
          [4, 5],
          [5, 6],
          [6, 7],
          [7, 4],
          [0, 4],
          [1, 5],
          [2, 6],
          [3, 7],
        ].forEach((l) => line(l[0], l[1]));

        const sp = pj(-roomW * 0.5, roomH * 0.6, -roomD * 0.4);
        const lp = pj(roomW * 0.45, roomH * 0.6, roomD * 0.45);
        const ph = (t * 0.45) % 1;

        const lerp = (a: number, b: number, k: number) => a + (b - a) * k;

        const rays: [number, number, number][] = [
          [-roomW, roomH * 0.9, 0],
          [roomW, roomH * 0.9, -roomD * 0.2],
          [0, 0, 0],
          [0, 2 * roomH, roomD * 0.1],
          [0, roomH, -roomD],
          [0, roomH, roomD],
        ];

        rays.forEach((rf, i) => {
          const q = pj(rf[0], rf[1], rf[2]);
          ctx.strokeStyle = getCss("--sig");
          ctx.globalAlpha = 0.55;
          ctx.lineWidth = 1;
          ctx.beginPath();
          ctx.moveTo(sp[0], sp[1]);
          ctx.lineTo(q[0], q[1]);
          ctx.lineTo(lp[0], lp[1]);
          ctx.stroke();
          ctx.globalAlpha = 1;

          const k = (ph + i * 0.13) % 1;
          const seg1 = k < 0.5;
          const u = seg1 ? k * 2 : (k - 0.5) * 2;
          const a = seg1 ? sp : q;
          const b = seg1 ? q : lp;
          ctx.fillStyle = getCss("--sig");
          ctx.beginPath();
          ctx.arc(lerp(a[0], b[0], u), lerp(a[1], b[1], u), 2.6, 0, Math.PI * 2);
          ctx.fill();
        });

        ctx.fillStyle = getCss("--t4");
        ctx.beginPath();
        ctx.arc(sp[0], sp[1], 8, 0, Math.PI * 2);
        ctx.fill();

        ctx.fillStyle = getCss("--tx");
        ctx.beginPath();
        ctx.arc(lp[0], lp[1], 8, 0, Math.PI * 2);
        ctx.fill();

        ctx.fillStyle = getCss("--tx3");
        ctx.font = '500 10.5px "Geist Mono",monospace';
        ctx.fillText("Source", sp[0] + 11, sp[1] + 4);
        ctx.fillText("You", lp[0] + 11, lp[1] + 4);
      }
    };

    render();

    const observer = getSharedResizeObserver();
    if (observer && canvas) {
      canvasRenderMap.set(canvas, render);
      observer.observe(canvas);
    }

    return () => {
      if (observer && canvas) {
        observer.unobserve(canvas);
      }
    };
  }, [
    type,
    seed,
    stereo,
    step,
    kind,
    color,
    gain,
    until,
    ceil,
    height,
    on,
    th,
    ratio,
    mode,
    tool,
    rt,
    w,
    d,
    h,
  ]);

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
      data-th={th}
      data-ratio={ratio}
      data-mode={mode}
      data-tool={tool}
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
