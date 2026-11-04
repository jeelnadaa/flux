/**
 * Kinetic typography and entrance animations.
 * 1:1 replica of design/app.js kinetic equalizer and staggered entrance animations.
 */

export interface LetterInfo {
  l: HTMLElement;
  i: number;
  x: number;
  lastWeight?: number;
  lastScale?: string;
}

let kin: LetterInfo[] = [];
let kinLo = 640;
let kinHi = 900;
let px = -1e4;
let isPlaying = false;
let rafId: number | null = null;
let listenersAttached = false;
let idleFrames = 0;

function updateKinTokens() {
  if (typeof window === "undefined" || typeof document === "undefined") return;
  const rs = window.getComputedStyle(document.documentElement);
  kinLo = parseFloat(rs.getPropertyValue("--kin-lo")) || 640;
  kinHi = parseFloat(rs.getPropertyValue("--kin-hi")) || 900;
}

export function lockWidths(): void {
  if (typeof window === "undefined" || typeof document === "undefined") return;
  kin.forEach((k) => {
    if (!k.l.isConnected) return;
    const parentHeading = k.l.closest<HTMLElement>(".h1, .bigh");
    if (!parentHeading) return;
    const fs = parseFloat(window.getComputedStyle(parentHeading).fontSize) || 1;
    k.l.style.width = "";
    const r = k.l.getBoundingClientRect();
    if (r.width > 0) {
      k.l.style.width = `${r.width / fs}em`;
      k.x = r.left + r.width / 2;
    }
  });
}

export function splitType(root: HTMLElement): LetterInfo[] {
  if (typeof window === "undefined" || typeof document === "undefined") return [];
  updateKinTokens();

  const headings = root.querySelectorAll<HTMLElement>(".h1, .bigh");
  const newLetters: LetterInfo[] = [];
  let indexCounter = 0;

  headings.forEach((h) => {
    if (h.closest("[aria-hidden]") || h.querySelector("a, span.act")) return;

    const isAlreadySplit = h.classList.contains("kin") && h.querySelector(".lt") !== null;
    if (!isAlreadySplit) {
      const text = h.textContent || "";
      if (!text.trim()) return;
      h.setAttribute("aria-label", text);
      h.classList.add("kin");
      h.innerHTML = text
        .split(" ")
        .map(
          (w) =>
            `<span class="wd" aria-hidden="true">${[...w]
              .map((c) => `<span class="lt">${c}</span>`)
              .join("")}</span>`,
        )
        .join(" ");
    }

    const fs = parseFloat(window.getComputedStyle(h).fontSize) || 1;
    const letters = h.querySelectorAll<HTMLElement>(".lt");
    letters.forEach((l) => {
      const r = l.getBoundingClientRect();
      if (r.width > 0) {
        l.style.width = `${r.width / fs}em`;
      }
      newLetters.push({
        l,
        i: indexCounter++,
        x: r.left + r.width / 2,
      });
    });
  });

  kin = newLetters;

  if (document.fonts && typeof document.fonts.ready?.then === "function") {
    document.fonts.ready.then(() => {
      lockWidths();
    });
  }

  return newLetters;
}

export function staggerEntrance(root: HTMLElement): void {
  if (typeof window === "undefined" || typeof document === "undefined") return;
  root.querySelectorAll<HTMLElement>(".in").forEach((el) => {
    Array.from(el.children).forEach((child, i) => {
      (child as HTMLElement).style.setProperty("--n", `${Math.min(i, 14)}`);
    });
  });
}

function safeRaf(cb: (ms: number) => void): number | null {
  if (typeof requestAnimationFrame !== "undefined") {
    return requestAnimationFrame(cb);
  }
  return null;
}

function safeCancelRaf(id: number | null) {
  if (id !== null && typeof cancelAnimationFrame !== "undefined") {
    cancelAnimationFrame(id);
  }
}

function tick(ms: number) {
  if (typeof window === "undefined") return;

  const calm = window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  kin = kin.filter((k) => k.l.isConnected);

  if (!calm && kin.length > 0) {
    const lv = isPlaying ? 0.5 + 0.4 * Math.sin(ms / 170) : 0;
    let hasActiveMotion = false;

    kin.forEach((k) => {
      const v = Math.min(
        1,
        Math.max(
          0,
          (0.5 + 0.5 * Math.sin(ms / 640 + k.i * 0.7) * Math.sin(ms / 1610 + k.i * 0.29)) * 0.5 +
            Math.exp(-Math.pow((k.x - px) / 160, 2)) * 0.55 +
            lv * 0.25,
        ),
      );
      const targetWeight = Math.round(kinLo + v * (kinHi - kinLo));
      const targetScale = (0.94 + v * 0.2).toFixed(3);

      if (k.lastWeight !== targetWeight) {
        k.l.style.fontWeight = `${targetWeight}`;
        k.lastWeight = targetWeight;
        hasActiveMotion = true;
      }
      if (k.lastScale !== targetScale) {
        k.l.style.transform = `scaleY(${targetScale})`;
        k.lastScale = targetScale;
        hasActiveMotion = true;
      }
    });

    if (hasActiveMotion || isPlaying || px !== -1e4) {
      idleFrames = 0;
    } else {
      idleFrames++;
    }

    // Sleep RAF if nothing is playing, mouse is outside, and styles are stationary
    if (!isPlaying && px === -1e4 && idleFrames > 10) {
      rafId = null;
      return;
    }
  }

  rafId = safeRaf(tick);
}

function attachGlobalListeners() {
  if (listenersAttached || typeof window === "undefined") return;
  listenersAttached = true;

  window.addEventListener(
    "pointermove",
    (e: PointerEvent) => {
      px = e.clientX;
      idleFrames = 0;
      if (rafId === null) {
        rafId = safeRaf(tick);
      }
    },
    { passive: true },
  );

  window.addEventListener(
    "pointerleave",
    () => {
      px = -1e4;
    },
    { passive: true },
  );

  window.addEventListener("resize", () => {
    lockWidths();
  });
}

export function startKineticAnimation(): void {
  if (typeof window === "undefined") return;
  attachGlobalListeners();
  idleFrames = 0;
  if (rafId === null) {
    rafId = safeRaf(tick);
  }
}

export function stopKineticAnimation(): void {
  safeCancelRaf(rafId);
  rafId = null;
}

export function setKineticPlaying(playing: boolean): void {
  isPlaying = playing;
  if (playing && rafId === null) {
    idleFrames = 0;
    rafId = safeRaf(tick);
  }
}

export function getKineticLetters(): LetterInfo[] {
  return kin;
}

export function hydrateKineticTypography(root: HTMLElement): void {
  if (!root || typeof window === "undefined") return;
  staggerEntrance(root);
  splitType(root);
  startKineticAnimation();
}

export function observeKineticTypography(root: HTMLElement): () => void {
  if (!root || typeof window === "undefined") return () => {};

  hydrateKineticTypography(root);

  let scheduled = false;
  const observer = new MutationObserver(() => {
    if (scheduled) return;
    scheduled = true;
    requestAnimationFrame(() => {
      scheduled = false;
      if (root.isConnected) {
        // Only run if unhydrated headings or .in children exist
        const hasUnhydratedHeadings = root.querySelector(".h1:not(.kin), .bigh:not(.kin)");
        const hasUnhydratedIn = Array.from(root.querySelectorAll<HTMLElement>(".in")).some(
          (inEl) =>
            inEl.children.length > 0 &&
            !(inEl.children[0] as HTMLElement).style.getPropertyValue("--n"),
        );
        if (hasUnhydratedHeadings || hasUnhydratedIn) {
          hydrateKineticTypography(root);
        }
      }
    });
  });

  observer.observe(root, {
    childList: true,
    subtree: true,
  });

  return () => {
    observer.disconnect();
  };
}
