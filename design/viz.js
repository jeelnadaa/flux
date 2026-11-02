/* Flux visuals: every waveform, spectrogram, curve and stage in the prototype. Procedural and seeded. */
window.Viz = (function () {
  const css = n => getComputedStyle(document.documentElement).getPropertyValue(n).trim();
  const clamp = (v, a = 0, b = 1) => Math.min(b, Math.max(a, v));
  const lerp = (a, b, t) => a + (b - a) * t;
  function prng(s) { return () => { s |= 0; s = s + 0x6D2B79F5 | 0; let t = Math.imul(s ^ s >>> 15, 1 | s); t = t + Math.imul(t ^ t >>> 7, 61 | t) ^ t; return ((t ^ t >>> 14) >>> 0) / 4294967296; }; }
  const cache = {};
  function env(seed, n, kind) {
    const k = seed + ':' + n + ':' + kind; if (cache[k]) return cache[k];
    const r = prng(seed * 977 + 13), a = new Float32Array(n), p1 = r() * 9, p2 = r() * 9, f1 = 8 + r() * 26, f2 = 60 + r() * 200;
    for (let i = 0; i < n; i++) {
      const x = i / n, beat = (x * (kind === 'drums' ? 64 : 32)) % 1;
      let v;
      if (kind === 'voice') { const g = Math.pow(Math.max(0, Math.sin(x * f1 + p1)), .5); v = g * (.45 + .4 * Math.abs(Math.sin(x * f2 + p2))) * (.75 + .25 * r()); }
      else if (kind === 'drums') { v = Math.exp(-beat * 7) * .95 + r() * .06; }
      else if (kind === 'bass') { v = (.4 + .16 * Math.sin(x * f1)) * (1 - Math.exp(-beat * 3) * .5); }
      else if (kind === 'pad') { v = .22 + .14 * Math.sin(x * f1 * .5 + p1) + .05 * r(); }
      else { v = .22 + .36 * Math.pow(Math.abs(Math.sin(x * f1 + p1)), 1.4) + .3 * Math.exp(-beat * 5) + .1 * r(); }
      a[i] = clamp(v);
    }
    return cache[k] = a;
  }

  const V = {};
  V.wave = (x, W, H, o) => {
    const n = Math.min(1600, Math.max(30, Math.floor(W / (+o.step || 2.5)))), e = env(+o.seed || 1, n, o.kind || 'mix'), col = o.color ? css(o.on ? o.color + '-on' : o.color) || o.color : css('--tx');
    x.clearRect(0, 0, W, H); const stereo = o.stereo === '1', bw = W / n, gain = +o.gain || 1, mid = H / 2;
    x.fillStyle = col;
    const until = o.until ? +o.until : 1;
    for (let i = 0; i < n; i++) {
      if (i / n > until) { x.globalAlpha = .25; }
      const v = e[i] * gain;
      if (stereo) { const h = H * .23; const h1 = Math.max(.5, v * h), h2 = Math.max(.5, v * h * (.9 + .1 * Math.sin(i * .3))); x.fillRect(i * bw, H * .26 - h1, Math.max(.8, bw * .66), h1 * 2); x.fillRect(i * bw, H * .74 - h2, Math.max(.8, bw * .66), h2 * 2); }
      else { const h = Math.max(.5, v * H * .46); x.fillRect(i * bw, mid - h, Math.max(.8, bw * .66), h * 2); }
    }
    x.globalAlpha = 1;
    if (stereo) { x.fillStyle = css('--ln'); x.fillRect(0, H / 2, W, 1); }
    if (o.ceil) { x.fillStyle = css('--sig'); const y = H * (.5 - .46 * +o.ceil); x.fillRect(0, y, W, 1); x.fillRect(0, H - y, W, 1); }
  };
  const rgb = h => [parseInt(h.slice(1, 3), 16), parseInt(h.slice(3, 5), 16), parseInt(h.slice(5, 7), 16)];
  /* Spectrogram ramp from the theme: ground, then signal, then ink. */
  const heat = (v, A, B, C) => { v = clamp(v * 1.5); const [p, q, k] = v < .55 ? [A, B, v / .55] : [B, C, (v - .55) / .45]; return [lerp(p[0], q[0], k), lerp(p[1], q[1], k), lerp(p[2], q[2], k)]; };
  V.spec = (x, W, H, o) => {
    const g = css('--ground'), k = css('--ink'), dark = rgb(k).reduce((a, b) => a + b) < rgb(g).reduce((a, b) => a + b), ramp = [dark ? k : g, css('--sig-i'), dark ? g : k], key = 'sp' + (o.seed || 1) + '_' + Math.round(W / 3) + '_' + Math.round(H / 3) + ramp.join(''); let c = cache[key];
    if (!c) {
      const w = Math.max(40, Math.round(W / 3)), h = Math.max(20, Math.round(H / 3)); c = document.createElement('canvas'); c.width = w; c.height = h; const q = c.getContext('2d'), im = q.createImageData(w, h), r = prng((+o.seed || 1) * 91), e1 = env(+o.seed || 1, w, 'voice'), e2 = env((+o.seed || 1) + 1, w, 'drums'), e3 = env((+o.seed || 1) + 2, w, 'bass');
      for (let xi = 0; xi < w; xi++) for (let y = 0; y < h; y++) {
        const f = 1 - y / h, t = xi / w;
        const v = e3[xi] * Math.exp(-f * 13) + e2[xi] * Math.exp(-f * 3.4) * .8 + e1[xi] * Math.pow(Math.max(0, Math.sin(f * 62 + Math.sin(t * 90) * 2.6)), 8) * Math.exp(-f * 1.7) + r() * .03 + .1 * Math.exp(-f * 2.2);
        const [a, b, d] = heat(v, ...ramp.map(rgb)); const i = (y * w + xi) * 4; im.data[i] = a; im.data[i + 1] = b; im.data[i + 2] = d; im.data[i + 3] = 255;
      }
      q.putImageData(im, 0, 0); cache[key] = c;
    }
    x.imageSmoothingEnabled = true; x.drawImage(c, 0, 0, W, H);
  };
  V.ruler = (x, W, H) => {
    x.clearRect(0, 0, W, H); x.fillStyle = css('--tx3'); x.font = '500 10.5px "Geist Mono",monospace'; const n = Math.max(4, Math.floor(W / 56)), st = W / n;
    for (let i = 0; i <= n * 4; i++) { const px = i * st / 4, ma = i % 4 === 0; x.globalAlpha = ma ? .9 : .4; x.fillRect(px, H - (ma ? 10 : 5), 1, ma ? 10 : 5); if (ma && i < n * 4) { const s = i / 4 * 8; x.fillText(`${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`, px + 4, 14); } }
    x.globalAlpha = 1;
  };
  function logX(f, W) { return Math.log10(f / 20) / Math.log10(1000) * W; }
  function freqGrid(x, W, H, zero) {
    x.strokeStyle = css('--ln'); x.lineWidth = 1; x.fillStyle = css('--tx3'); x.font = '500 10.5px "Geist Mono",monospace';
    [20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000].forEach(f => { const px = logX(f, W); x.beginPath(); x.moveTo(px, 0); x.lineTo(px, H); x.stroke(); if (f !== 20) x.fillText(f >= 1000 ? f / 1000 + 'k' : f, Math.min(px + 3, W - 22), H - 6); });
    [-18, -12, -6, 0, 6, 12, 18].forEach(d => { const y = H / 2 - d / 24 * H; x.globalAlpha = d === 0 ? 1 : .5; x.beginPath(); x.moveTo(0, y); x.lineTo(W, y); x.stroke(); x.globalAlpha = 1; x.fillText((d > 0 ? '+' : '') + d, 4, y - 3); });
  }
  V.eq = (x, W, H, o, t, cv) => {
    x.clearRect(0, 0, W, H); freqGrid(x, W, H);
    /* live spectrum */
    const n = Math.floor(W / 4); x.fillStyle = css('--ac-t'); x.globalAlpha = .22;
    for (let i = 0; i < n; i++) { const u = i / n, v = (Math.sin(u * 16 + t * 1.4) * .13 + Math.sin(u * 47 - t * .9) * .08 + .6 - u * .52) * (.92 + .08 * Math.sin(t * 6 + i)); const h = clamp(v) * H * .72; x.fillRect(i * 4, H - h, 3, h); }
    x.globalAlpha = 1;
    const nodes = cv._nodes || (cv._nodes = [{ f: 70, g: 3.5, q: .8, c: '--t3' }, { f: 280, g: -3, q: 1.3, c: '--t2' }, { f: 1200, g: 1.6, q: 1, c: '--t4' }, { f: 3400, g: 4, q: 1.5, c: '--t5' }, { f: 11000, g: 3, q: .7, c: '--t1' }]);
    const resp = u => { let s = 0; const f = 20 * Math.pow(1000, u); nodes.forEach(nd => { const d = Math.log2(f / nd.f) * nd.q * 1.3; s += nd.g * Math.exp(-d * d); }); return s; };
    x.beginPath(); for (let px = 0; px <= W; px += 2) { const y = H / 2 - resp(px / W) / 24 * H; px === 0 ? x.moveTo(px, y) : x.lineTo(px, y); } x.lineWidth = 2.5; x.strokeStyle = css('--tx'); x.stroke();
    x.lineTo(W, H / 2); x.lineTo(0, H / 2); x.closePath(); x.fillStyle = css('--tx'); x.globalAlpha = .08; x.fill(); x.globalAlpha = 1;
    nodes.forEach((nd, i) => { const px = logX(nd.f, W), y = H / 2 - nd.g / 24 * H; x.beginPath(); x.arc(px, y, 11, 0, 7); x.fillStyle = css(nd.c); x.fill(); x.fillStyle = css(nd.c + '-on'); x.font = '800 11px ' + css('--sans'); x.textAlign = 'center'; x.textBaseline = 'middle'; x.fillText(i + 1, px, y + .5); x.textAlign = 'left'; x.textBaseline = 'alphabetic'; });
    if (!cv._eqd) {
      cv._eqd = 1; let dr = null; cv.style.touchAction = 'none';
      cv.addEventListener('pointerdown', e => { const r = cv.getBoundingClientRect(), mx = e.clientX - r.left, my = e.clientY - r.top; let best = null, bd = 28; cv._nodes.forEach(nd => { const d = Math.hypot(logX(nd.f, r.width) - mx, r.height / 2 - nd.g / 24 * r.height - my); if (d < bd) { bd = d; best = nd; } }); dr = best; if (dr) cv.setPointerCapture(e.pointerId); });
      cv.addEventListener('pointermove', e => { if (!dr) return; const r = cv.getBoundingClientRect(); dr.f = 20 * Math.pow(1000, clamp((e.clientX - r.left) / r.width)); dr.g = clamp((r.height / 2 - (e.clientY - r.top)) / r.height * 24, -18, 18); });
      addEventListener('pointerup', () => { dr = null; });
    }
  };
  V.transfer = (x, W, H, o) => {
    x.clearRect(0, 0, W, H); x.strokeStyle = css('--ln'); for (let i = 1; i < 6; i++) { x.beginPath(); x.moveTo(i * W / 6, 0); x.lineTo(i * W / 6, H); x.moveTo(0, i * H / 6); x.lineTo(W, i * H / 6); x.stroke(); }
    x.strokeStyle = css('--tx3'); x.setLineDash([3, 4]); x.beginPath(); x.moveTo(0, H); x.lineTo(W, 0); x.stroke(); x.setLineDash([]);
    const th = +o.th || .62, ra = +o.ratio || 3.2; x.beginPath(); for (let i = 0; i <= 100; i++) { const u = i / 100, k = .09, v = u < th - k ? u : u > th + k ? th + (u - th) / ra : u + (1 / ra - 1) * Math.pow(u - th + k, 2) / (4 * k); const px = u * W, py = H - v * H; i ? x.lineTo(px, py) : x.moveTo(px, py); } x.lineWidth = 2.5; x.strokeStyle = css('--tx'); x.stroke();
    x.beginPath(); x.arc(th * W, H - th * H, 6, 0, 7); x.fillStyle = css('--sig'); x.fill(); x.fillStyle = css('--tx3'); x.font = '500 10.5px "Geist Mono",monospace'; x.fillText('Input', W - 38, H - 6); x.fillText('Output', 5, 13);
  };
  V.gr = (x, W, H, o, t) => {
    x.clearRect(0, 0, W, H); const n = Math.floor(W / 3), e = env(4, 800, 'mix');
    for (let i = 0; i < n; i++) { const idx = Math.floor(i + t * 26) % 800, v = e[idx], gr = clamp((v - .4) * 1.5); x.fillStyle = css('--tx2'); x.globalAlpha = .55; const h = v * H * .8; x.fillRect(i * 3, H - h, 2, h); x.globalAlpha = 1; x.fillStyle = css('--sig'); x.fillRect(i * 3, 0, 2, gr * H * .34); }
  };
  V.gonio = (x, W, H, o, t) => {
    x.clearRect(0, 0, W, H); const c = Math.min(W, H) / 2 - 8, cx = W / 2, cy = H / 2; x.strokeStyle = css('--ln'); x.beginPath(); x.moveTo(cx - c, cy); x.lineTo(cx + c, cy); x.moveTo(cx, cy - c); x.lineTo(cx, cy + c); x.stroke(); x.fillStyle = css('--ac-t'); x.globalAlpha = .55;
    const r = prng(Math.floor(t * 20)); for (let i = 0; i < 420; i++) { const a = r() + r() + r() - 1.5, b = (r() + r() - 1) * .36, px = cx + b * c * 1.4, py = cy - a * c * .58 * (1 + .3 * Math.sin(t * 2)); x.fillRect(px, py, 1.5, 1.5); } x.globalAlpha = 1;
  };
  V.spectrum = (x, W, H, o, t) => { x.clearRect(0, 0, W, H); const n = Math.floor(W / 5); for (let i = 0; i < n; i++) { const u = i / n, v = clamp((.82 - u * .62) + Math.sin(u * 19 + t * 2.2) * .1 + Math.sin(u * 53 - t * 3.1) * .07); x.fillStyle = css('--ac-t'); const h = v * H; x.fillRect(i * 5, H - h, 4, h); } };
  V.history = (x, W, H, o, t, cv) => {
    x.clearRect(0, 0, W, H); x.lineCap = 'round'; const ys = [H * .5, H * .22, H * .78]; x.strokeStyle = css('--ln2'); x.lineWidth = 2;
    const stepsN = 15, st = W / (stepsN + .6); x.beginPath(); x.moveTo(st * .5, ys[0]); x.lineTo(st * (stepsN - 2), ys[0]); x.stroke();
    x.beginPath(); x.moveTo(st * 4.5, ys[0]); x.bezierCurveTo(st * 5, ys[0], st * 5, ys[1], st * 5.6, ys[1]); x.lineTo(st * 9.5, ys[1]); x.stroke();
    x.beginPath(); x.moveTo(st * 8.5, ys[0]); x.bezierCurveTo(st * 9, ys[0], st * 9, ys[2], st * 9.6, ys[2]); x.lineTo(st * 12.5, ys[2]); x.stroke();
    for (let i = 0; i < stepsN - 2; i++) { x.beginPath(); x.arc(st * (i + .5), ys[0], i === 11 ? 8 : 5, 0, 7); x.fillStyle = i === 11 ? css('--sig') : css('--tx2'); x.fill(); }
    for (let i = 5; i < 10; i++) { x.beginPath(); x.arc(st * (i + .6), ys[1], 4.5, 0, 7); x.fillStyle = css('--t1'); x.fill(); }
    for (let i = 9; i < 13; i++) { x.beginPath(); x.arc(st * (i + .6), ys[2], 4.5, 0, 7); x.fillStyle = css('--t3'); x.fill(); }
  };

  /* Sound stage: top-down, drag to place, draw a path */
  V.stage = (x, W, H, o, t, cv) => {
    const st = cv._st || (cv._st = { mode: o.mode || 'orbit', a: 0, src: [{ az: 0, r: .72, c: '--t4', n: 'Vocals', fix: 1 }, { az: 2.2, r: .85, c: '--t3', n: 'Drums' }, { az: 4.1, r: .55, c: '--t2', n: 'Bass', fix: 1 }, { az: 5.4, r: .9, c: '--t5', n: 'Other' }], path: null, trail: [] });
    const m = cv.dataset.mode || st.mode; st.a += (+(cv.dataset.rate || 1)) * .016 * .75;
    x.clearRect(0, 0, W, H); const c = Math.min(W, H) / 2 - 22, cx = W / 2, cy = H / 2; st.c = c; st.cx = cx; st.cy = cy;
    x.strokeStyle = css('--ln2'); x.lineWidth = 1;
    [.33, .66, 1].forEach(r => { x.beginPath(); x.arc(cx, cy, c * r, 0, 7); x.stroke(); });
    for (let d = 0; d < 360; d += 30) { const a = d * Math.PI / 180, l = d % 90 === 0 ? 12 : 6; x.beginPath(); x.moveTo(cx + Math.sin(a) * c, cy - Math.cos(a) * c); x.lineTo(cx + Math.sin(a) * (c - l), cy - Math.cos(a) * (c - l)); x.stroke(); }
    x.fillStyle = css('--tx3'); x.font = '500 10.5px "Geist Mono",monospace'; x.textAlign = 'center'; x.fillText('Front', cx, cy - c - 7); x.fillText('Back', cx, cy + c + 15); x.textAlign = 'left'; x.fillText('L', cx - c - 14, cy + 4); x.fillText('R', cx + c + 6, cy + 4);
    const pos = (s, i) => {
      if (s.fix || m === 'place') return [s.az, s.r, 0];
      const a = st.a * (i === 1 ? 1 : -.65) + s.az;
      if (st.path && st.path.length > 4 && i === 1) { const k = Math.floor(((st.a * .13) % 1 + 1) % 1 * st.path.length), p = st.path[k]; return [Math.atan2(p[0], -p[1]), Math.hypot(p[0], p[1]), 0]; }
      if (m === 'spiral') return [a, s.r * (.55 + .45 * Math.sin(st.a * .6 + i))];
      if (m === 'eight') return [Math.atan2(Math.sin(a), -Math.sin(a * 2) * .6), Math.hypot(Math.sin(a), Math.sin(a * 2) * .6) * .9];
      if (m === 'swarm') { const b = st.a * 1.3 + s.az * 3; return [Math.atan2(Math.sin(b) * .7 + Math.sin(b * 2.9) * .2, Math.cos(b * 1.7) * .7), .3 + .6 * Math.abs(Math.sin(b * .8))]; }
      return [a, s.r];
    };
    if (st.path && st.path.length > 1) { x.beginPath(); st.path.forEach((p, i) => { const px = cx + p[0] * c, py = cy + p[1] * c; i ? x.lineTo(px, py) : x.moveTo(px, py); }); x.strokeStyle = css('--sig'); x.lineWidth = 2; x.setLineDash([5, 5]); x.stroke(); x.setLineDash([]); }
    st.src.forEach((s, i) => {
      const [az, r] = pos(s, i), px = cx + Math.sin(az) * r * c, py = cy - Math.cos(az) * r * c, col = css(s.c);
      s._x = px; s._y = py;
      if (!s.fix && m !== 'place') { s.tr = s.tr || []; s.tr.push([px, py]); if (s.tr.length > 46) s.tr.shift(); s.tr.forEach((q, k) => { x.globalAlpha = k / s.tr.length * .5; x.fillStyle = col; x.beginPath(); x.arc(q[0], q[1], 1.5 + k / s.tr.length * 6, 0, 7); x.fill(); }); x.globalAlpha = 1; }
      x.beginPath(); x.arc(px, py, 14, 0, 7); x.fillStyle = col; x.fill(); x.fillStyle = css(s.c + '-on'); x.font = '800 11px ' + css('--sans'); x.textAlign = 'center'; x.textBaseline = 'middle'; x.fillText(s.n[0], px, py + .5); x.textAlign = 'left'; x.textBaseline = 'alphabetic';
    });
    const hr = c * .12; x.beginPath(); x.arc(cx, cy, hr, 0, 7); x.fillStyle = css('--s3'); x.fill(); x.strokeStyle = css('--tx'); x.lineWidth = 2; x.stroke();
    x.beginPath(); x.moveTo(cx, cy - hr - 9); x.lineTo(cx - 7, cy - hr + 1); x.lineTo(cx + 7, cy - hr + 1); x.closePath(); x.fillStyle = css('--tx'); x.fill();
    x.fillRect(cx - hr - 4, cy - 6, 4, 12); x.fillRect(cx + hr, cy - 6, 4, 12);
    if (!cv._dr) {
      cv._dr = 1; cv.style.touchAction = 'none'; let drag = null, draw = false;
      cv.addEventListener('pointerdown', e => {
        const r = cv.getBoundingClientRect(), mx = e.clientX - r.left, my = e.clientY - r.top;
        if (cv.dataset.tool === 'draw') { draw = true; st.path = [[(mx - st.cx) / st.c, (my - st.cy) / st.c]]; st.src[1].fix = 0; cv.setPointerCapture(e.pointerId); return; }
        drag = st.src.find(s => Math.hypot(s._x - mx, s._y - my) < 24); if (drag) { drag.fix = 1; cv.setPointerCapture(e.pointerId); }
      });
      cv.addEventListener('pointermove', e => {
        const r = cv.getBoundingClientRect(), mx = e.clientX - r.left - st.cx, my = e.clientY - r.top - st.cy;
        if (draw) { const q = [mx / st.c, my / st.c], l = st.path[st.path.length - 1]; if (Math.hypot(q[0] - l[0], q[1] - l[1]) > .025) st.path.push(q); return; }
        if (drag) { drag.az = Math.atan2(mx, -my); drag.r = clamp(Math.hypot(mx, my) / st.c, .16, 1); }
      });
      addEventListener('pointerup', () => { drag = null; draw = false; });
    }
  };
  /* Stage elevation strip */
  V.elev = (x, W, H, o, t) => { x.clearRect(0, 0, W, H); x.strokeStyle = css('--ln2'); x.beginPath(); x.moveTo(0, H / 2); x.lineTo(W, H / 2); x.stroke(); x.fillStyle = css('--tx3'); x.font = '500 10.5px "Geist Mono",monospace'; x.fillText('Above', 6, 13); x.fillText('Below', 6, H - 5); const cx = W / 2; x.beginPath(); x.arc(cx, H / 2, 11, 0, 7); x.strokeStyle = css('--tx'); x.lineWidth = 2; x.stroke(); const y = H / 2 - Math.sin(t * .5) * H * .32, px = cx + Math.cos(t * .75) * W * .3; x.beginPath(); x.arc(px, y, 8, 0, 7); x.fillStyle = css('--t3'); x.fill(); };
  /* Room box in perspective */
  V.room = (x, W, H, o, t, cv) => {
    x.clearRect(0, 0, W, H); const w = +(o.w || 1.3), d = +(o.d || 1), h = +(o.h || .6), yaw = -.72 + Math.sin(t * .22) * .12, pitch = .46, S = Math.min(W, H) * .34;
    const pj = (X, Y, Z) => { const cy = Math.cos(yaw), sy = Math.sin(yaw), x1 = X * cy - Z * sy, z1 = X * sy + Z * cy, cp = Math.cos(pitch), sp = Math.sin(pitch), y1 = Y * cp - z1 * sp, z2 = Y * sp + z1 * cp, f = 3.4 / (3.4 + z2); return [W / 2 + x1 * f * S, H * .58 - y1 * f * S]; };
    const C = [[-w, 0, -d], [w, 0, -d], [w, 0, d], [-w, 0, d], [-w, 2 * h, -d], [w, 2 * h, -d], [w, 2 * h, d], [-w, 2 * h, d]];
    const P = C.map(c => pj(...c)); const line = (a, b) => { x.beginPath(); x.moveTo(P[a][0], P[a][1]); x.lineTo(P[b][0], P[b][1]); x.stroke(); };
    x.fillStyle = css('--ac-soft'); x.beginPath(); [0, 1, 2, 3].forEach((k, i) => i ? x.lineTo(P[k][0], P[k][1]) : x.moveTo(P[k][0], P[k][1])); x.closePath(); x.fill();
    x.strokeStyle = css('--tx2'); x.lineWidth = 1.5; [[0, 1], [1, 2], [2, 3], [3, 0], [4, 5], [5, 6], [6, 7], [7, 4], [0, 4], [1, 5], [2, 6], [3, 7]].forEach(l => line(l[0], l[1]));
    const sp = pj(-w * .5, h * .6, -d * .4), lp = pj(w * .45, h * .6, d * .45);
    const ph = (t * .45) % 1; [[-w, h * .9, 0], [w, h * .9, -d * .2], [0, 0, 0], [0, 2 * h, d * .1], [0, h, -d], [0, h, d]].forEach((rf, i) => { const q = pj(rf[0], rf[1], rf[2]); x.strokeStyle = css('--sig'); x.globalAlpha = .55; x.lineWidth = 1; x.beginPath(); x.moveTo(sp[0], sp[1]); x.lineTo(q[0], q[1]); x.lineTo(lp[0], lp[1]); x.stroke(); x.globalAlpha = 1; const k = ((ph + i * .13) % 1); const seg1 = k < .5, u = seg1 ? k * 2 : (k - .5) * 2, a = seg1 ? sp : q, b = seg1 ? q : lp; x.fillStyle = css('--sig'); x.beginPath(); x.arc(lerp(a[0], b[0], u), lerp(a[1], b[1], u), 2.6, 0, 7); x.fill(); });
    x.fillStyle = css('--t4'); x.beginPath(); x.arc(sp[0], sp[1], 8, 0, 7); x.fill(); x.fillStyle = css('--tx'); x.beginPath(); x.arc(lp[0], lp[1], 8, 0, 7); x.fill();
    x.fillStyle = css('--tx3'); x.font = '500 10.5px "Geist Mono",monospace'; x.fillText('Source', sp[0] + 11, sp[1] + 4); x.fillText('You', lp[0] + 11, lp[1] + 4);
  };
  V.decay = (x, W, H, o) => { x.clearRect(0, 0, W, H); const r = prng(11), rt = +(o.rt || 2.4) / 8; for (let i = 0; i < W; i += 2) { const u = i / W, v = Math.exp(-u / rt * 2.4) * (.5 + .5 * r()) * (u < .03 ? u / .03 : 1); x.fillStyle = css('--ac-t'); const h = v * H * .9; x.fillRect(i, H - h, 1.4, h); } };
  V.lufs = (x, W, H) => { x.clearRect(0, 0, W, H); const e = env(9, Math.floor(W / 3), 'mix'); x.beginPath(); for (let i = 0; i < e.length; i++) { const s = (e.slice(Math.max(0, i - 8), i + 1).reduce((a, b) => a + b, 0) / Math.min(i + 1, 9)); const y = H * .85 - s * H * .8; i ? x.lineTo(i * 3, y) : x.moveTo(i * 3, y); } x.strokeStyle = css('--ac-t'); x.lineWidth = 2; x.stroke(); x.strokeStyle = css('--sig'); x.setLineDash([4, 4]); x.beginPath(); x.moveTo(0, H * .42); x.lineTo(W, H * .42); x.stroke(); x.setLineDash([]); x.fillStyle = css('--tx3'); x.font = '500 10.5px "Geist Mono",monospace'; x.fillText('Target −14 LUFS', 6, H * .42 - 5); };
  V.chain = (x, W, H, o, t) => { x.clearRect(0, 0, W, H); const e = env(+o.seed || 3, Math.floor(W / 3), 'mix'); x.fillStyle = css(o.color || '--t1'); const ph = (t * .0) | 0; for (let i = 0; i < e.length; i++) { const h = e[i] * H * .42; x.fillRect(i * 3, H / 2 - h, 2, h * 2); } };

  const live = new Set(), objs = new WeakMap();
  const LIVE = { eq: 1, gr: 1, gonio: 1, spectrum: 1, stage: 1, elev: 1, room: 1 };
  function draw(c, t) {
    const d = Math.min(devicePixelRatio || 1, 2), r = c.getBoundingClientRect(); if (r.width < 2 || r.height < 2) return;
    const w = Math.round(r.width * d), h = Math.round(r.height * d); if (c.width !== w || c.height !== h) { c.width = w; c.height = h; }
    const x = c.getContext('2d'); x.setTransform(d, 0, 0, d, 0, 0);
    V[c.dataset.v](x, r.width, r.height, c.dataset, t || 0, c);
  }
  const ro = 'ResizeObserver' in window ? new ResizeObserver(es => es.forEach(e => { if (e.target.dataset.v && document.contains(e.target)) draw(e.target, performance.now() / 1000); })) : null;
  function mount(root) {
    live.clear(); root.querySelectorAll('canvas[data-v]').forEach(c => { if (!V[c.dataset.v]) return; draw(c, 0); if (ro) ro.observe(c); if (LIVE[c.dataset.v]) live.add(c); });
  }
  function loop(ms) { const t = ms / 1000; live.forEach(c => { if (c.isConnected) draw(c, t); }); requestAnimationFrame(loop); }
  requestAnimationFrame(loop);
  return { mount, redraw: root => root.querySelectorAll('canvas[data-v]').forEach(c => draw(c, performance.now() / 1000)) };
})();
