import React, { useEffect, useRef } from "react";

export interface CanvasVizProps {
  type: "wave" | "spec" | "ruler" | "eq" | "meter" | "stage" | "rooms" | "slice" | "tape" | string;
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
      }
    };

    render();

    let ro: ResizeObserver | null = null;
    if (typeof ResizeObserver !== "undefined") {
      ro = new ResizeObserver(() => {
        render();
      });
      ro.observe(canvas);
    }

    return () => {
      if (ro) {
        ro.disconnect();
      }
    };
  }, [type, seed, stereo, step, kind, color, gain, until, ceil, height, on]);

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
