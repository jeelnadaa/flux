/* Flux visuals: every waveform, spectrogram, curve and stage. Ported from design/viz.js. The drawing is the same;
   the data is real: peaks, spectrograms, meters and effect parameters come from the engine through FD.
   Canvases without a data source (illustrations in first run) keep the design's seeded shapes. */
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

  /* ---- real data: peaks of one clip/media range, or of the whole project (clips composited on the timeline) ---- */
  function peaksFor(o, n, cv) {
    const key = [o.media, o.start, o.end, o.rev, o.src, n, FD.D.project && FD.D.project.hash].join(':');
    if (cv._pkk === key) return cv._pk;
    if (cv._pkw !== key) {
      cv._pkw = key;
      const done = d => { if (cv._pkw !== key) return; cv._pkk = key; cv._pk = d; draw(cv, performance.now() / 1000); };
      if (o.media) FD.peaks(o.media, +o.start || 0, +o.end || 0, n, o.rev === '1').then(r => done(r ? toEnv(r, n) : null));
      else if (o.src === 'mix' || o.src === 'track') mixPeaks(n, o.src === 'track' ? +o.track : null, +o.start || 0, o.end ? +o.end : null).then(done);
    }
    return cv._pkk === key ? cv._pk : null;
  }
  function toEnv(r, n) {
    return r.channels.map(c => { const a = new Float32Array(n); for (let i = 0; i < n && 2 * i + 1 < c.length; i++) a[i] = Math.max(Math.abs(c[2 * i]), Math.abs(c[2 * i + 1])); return a; });
  }
  async function mixPeaks(n, trackId, t0, t1) {
    const p = FD.D.project; if (!p) return null;
    const total = t1 || p.length || 1, span = Math.max(1, total - t0), out = [new Float32Array(n), new Float32Array(n)];
    const list = p.tracks.filter(t => trackId == null || t.id === trackId).flatMap(t => t.clips);
    await Promise.all(list.map(async c => {
      const a = Math.max(c.start, t0), b = Math.min(c.start + c.len, total); if (b <= a) return;
      const i0 = Math.floor((a - t0) / span * n), i1 = Math.max(i0 + 1, Math.ceil((b - t0) / span * n)), bins = Math.min(4000, i1 - i0);
      const off = c.offset + (a - c.start), r = await FD.peaks(c.media, off, off + (b - a), bins, c.reverse); if (!r) return;
      const g = Math.pow(10, (c.gain_db || 0) / 20), e = toEnv(r, bins);
      for (let ch = 0; ch < 2; ch++) { const src = e[Math.min(ch, e.length - 1)]; for (let k = 0; k < bins && i0 + k < n; k++) out[ch][i0 + k] = Math.min(1, out[ch][i0 + k] + src[k] * g); }
    }));
    return out;
  }

  const V = {};
  V.wave = (x, W, H, o, t, cv) => {
    const n = Math.min(1600, Math.max(30, Math.floor(W / (+o.step || 2.5)))), real = o.media || o.src, col = o.color ? css(o.on ? o.color + '-on' : o.color) || o.color : css('--tx');
    x.clearRect(0, 0, W, H); const stereo = o.stereo === '1', bw = W / n, gain = +o.gain || 1, mid = H / 2;
    let e, e2;
    if (real) { const pk = peaksFor(o, n, cv); if (pk) { e = pk[0]; e2 = pk[1] || pk[0]; } }
    else if (!o.empty) { e = env(+o.seed || 1, n, o.kind || 'mix'); }
    x.fillStyle = col;
    const until = o.until ? +o.until : 1;
    if (e) for (let i = 0; i < n; i++) {
      if (i / n > until) { x.globalAlpha = .25; }
      const v = e[i] * gain;
      if (stereo) { const h = H * .23; const v2 = (e2 ? e2[i] : e[i] * (.9 + .1 * Math.sin(i * .3))) * gain; const h1 = Math.max(.5, v * h * (real ? 2 : 1)), h2 = Math.max(.5, v2 * h * (real ? 2 : 1)); x.fillRect(i * bw, H * .26 - Math.min(h1, H * .23), Math.max(.8, bw * .66), Math.min(h1, H * .23) * 2); x.fillRect(i * bw, H * .74 - Math.min(h2, H * .23), Math.max(.8, bw * .66), Math.min(h2, H * .23) * 2); }
      else { const h = Math.min(H * .46, Math.max(.5, v * H * (real ? .5 : .46))); x.fillRect(i * bw, mid - h, Math.max(.8, bw * .66), h * 2); }
    }
    else { x.globalAlpha = .4; x.fillRect(0, mid - .5, W, 1); }
    x.globalAlpha = 1;
    if (stereo) { x.fillStyle = css('--ln'); x.fillRect(0, H / 2, W, 1); }
    if (o.ceil) { x.fillStyle = css('--sig'); const y = H * (.5 - .46 * +o.ceil); x.fillRect(0, y, W, 1); x.fillRect(0, H - y, W, 1); }
    if (o.marks) { x.fillStyle = css('--sig'); o.marks.split(',').forEach(m => { const u = +m; if (u >= 0 && u <= 1) x.fillRect(u * W, 0, 1.5, H); }); }
  };
  const rgb = h => { h = h.trim(); if (h[0] !== '#') { const m = h.match(/\d+(\.\d+)?/g) || [0, 0, 0]; return [+m[0], +m[1], +m[2]]; } if (h.length === 4) h = '#' + h[1] + h[1] + h[2] + h[2] + h[3] + h[3]; return [parseInt(h.slice(1, 3), 16), parseInt(h.slice(3, 5), 16), parseInt(h.slice(5, 7), 16)]; };
  /* Spectrogram ramp from the theme: ground, then signal, then ink. */
  const heat = (v, A, B, C) => { v = clamp(v * 1.5); const [p, q, k] = v < .55 ? [A, B, v / .55] : [B, C, (v - .55) / .45]; return [lerp(p[0], q[0], k), lerp(p[1], q[1], k), lerp(p[2], q[2], k)]; };
  const b64 = s => { const b = atob(s), u = new Uint8Array(b.length); for (let i = 0; i < b.length; i++) u[i] = b.charCodeAt(i); return u; };
  V.spec = (x, W, H, o, t, cv) => {
    const g = css('--ground'), k = css('--ink'), dark = rgb(k).reduce((a, b) => a + b) < rgb(g).reduce((a, b) => a + b), ramp = [dark ? k : g, css('--sig-i'), dark ? g : k];
    if (o.src || o.media) {
      const cols = Math.max(40, Math.min(1200, Math.round(W / 3))), rows = Math.max(24, Math.min(256, Math.round(H / 3)));
      const q = { cols, rows }; if (o.media) q.media = o.media; if (o.start) { q.start = +o.start; q.end = +o.end; }
      if (o.window) q.window = +o.window;
      const key = JSON.stringify(q) + ramp.join('') + (FD.D.project && FD.D.project.hash);
      if (cv._sk !== key) {
        if (cv._sw !== key) { cv._sw = key; FD.spectrogram(q).then(r => { if (cv._sw !== key || !r) return; const d = b64(r.data), c = document.createElement('canvas'); c.width = r.cols; c.height = r.rows; const qq = c.getContext('2d'), im = qq.createImageData(r.cols, r.rows), R = ramp.map(rgb); for (let i = 0; i < d.length; i++) { const [a, b, e] = heat(Math.pow(d[i] / 255, 1.6) * .75, ...R); im.data[i * 4] = a; im.data[i * 4 + 1] = b; im.data[i * 4 + 2] = e; im.data[i * 4 + 3] = 255; } qq.putImageData(im, 0, 0); cv._sc = c; cv._sk = key; draw(cv, 0); }); }
        x.fillStyle = ramp[0]; x.fillRect(0, 0, W, H); if (!cv._sc) return;
      }
      x.imageSmoothingEnabled = true; x.drawImage(cv._sc, 0, 0, W, H); return;
    }
    const key = 'sp' + (o.seed || 1) + '_' + Math.round(W / 3) + '_' + Math.round(H / 3) + ramp.join(''); let c = cache[key];
    if (!c) {
      const w = Math.max(40, Math.round(W / 3)), h = Math.max(20, Math.round(H / 3)); c = document.createElement('canvas'); c.width = w; c.height = h; const q = c.getContext('2d'), im = q.createImageData(w, h), r = prng((+o.seed || 1) * 91), e1 = env(+o.seed || 1, w, 'voice'), e2 = env((+o.seed || 1) + 1, w, 'drums'), e3 = env((+o.seed || 1) + 2, w, 'bass');
      for (let xi = 0; xi < w; xi++) for (let y = 0; y < h; y++) {
        const f = 1 - y / h, tt = xi / w;
        const v = e3[xi] * Math.exp(-f * 13) + e2[xi] * Math.exp(-f * 3.4) * .8 + e1[xi] * Math.pow(Math.max(0, Math.sin(f * 62 + Math.sin(tt * 90) * 2.6)), 8) * Math.exp(-f * 1.7) + r() * .03 + .1 * Math.exp(-f * 2.2);
        const [a, b, d] = heat(v, ...ramp.map(rgb)); const i = (y * w + xi) * 4; im.data[i] = a; im.data[i + 1] = b; im.data[i + 2] = d; im.data[i + 3] = 255;
      }
      q.putImageData(im, 0, 0); cache[key] = c;
    }
    x.imageSmoothingEnabled = true; x.drawImage(c, 0, 0, W, H);
  };
  /* Ruler in real time: data-len is the visible length in seconds. */
  V.ruler = (x, W, H, o) => {
    x.clearRect(0, 0, W, H); x.fillStyle = css('--tx3'); x.font = '500 10.5px "Geist Mono",monospace';
    const total = +o.len || 32, t0 = +o.t0 || 0, steps = [0.1, .25, .5, 1, 2, 5, 10, 15, 30, 60, 120, 300, 600, 1800, 3600], want = total / Math.max(4, Math.floor(W / 56)), st = steps.find(s => s >= want) || 3600;
    const first = Math.ceil(t0 / (st / 4)) * (st / 4);
    for (let s = first, i = Math.round(first / (st / 4)); s <= t0 + total + 1e-9; s += st / 4, i++) {
      const px = (s - t0) / total * W, ma = i % 4 === 0; x.globalAlpha = ma ? .9 : .4; x.fillRect(px, H - (ma ? 10 : 5), 1, ma ? 10 : 5);
      if (ma && px < W - 30) { const m = Math.floor(s / 60), se = s - m * 60; x.fillText(`${m}:${(st < 1 ? se.toFixed(1) : String(Math.round(se))).padStart(st < 1 ? 4 : 2, '0')}`, px + 4, 14); }
    }
    x.globalAlpha = 1;
  };
  function logX(f, W) { return Math.log10(f / 20) / Math.log10(1000) * W; }
  function freqGrid(x, W, H) {
    x.strokeStyle = css('--ln'); x.lineWidth = 1; x.fillStyle = css('--tx3'); x.font = '500 10.5px "Geist Mono",monospace';
    [20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000].forEach(f => { const px = logX(f, W); x.beginPath(); x.moveTo(px, 0); x.lineTo(px, H); x.stroke(); if (f !== 20) x.fillText(f >= 1000 ? f / 1000 + 'k' : f, Math.min(px + 3, W - 22), H - 6); });
    [-18, -12, -6, 0, 6, 12, 18].forEach(d => { const y = H / 2 - d / 24 * H; x.globalAlpha = d === 0 ? 1 : .5; x.beginPath(); x.moveTo(0, y); x.lineTo(W, y); x.stroke(); x.globalAlpha = 1; x.fillText((d > 0 ? '+' : '') + d, 4, y - 3); });
  }
  /* Live spectrum bars from the output scope (dB per log band). */
  function scopeBand(u) { const s = FD.D.scope && FD.D.scope.spectrum; if (!s || !s.length) return 0; const v = s[Math.min(s.length - 1, Math.floor(u * s.length))]; return clamp((v + 90) / 90); }
  const BC = ['--t3', '--t2', '--t4', '--t5', '--t1'];
  const RBJ = { bell: 0, lowshelf: 1, highshelf: 2, lowpass: 3, highpass: 4, notch: 5 };
  /* Biquad magnitude for UI curves (same RBJ formulas as the engine). */
  function biq(type, f0, g, q, f, sr) {
    const A = Math.pow(10, g / 40), w0 = 2 * Math.PI * f0 / sr, cs = Math.cos(w0), sn = Math.sin(w0), al = sn / (2 * Math.max(.05, q)); let b0, b1, b2, a0, a1, a2;
    if (type === 0) { b0 = 1 + al * A; b1 = -2 * cs; b2 = 1 - al * A; a0 = 1 + al / A; a1 = -2 * cs; a2 = 1 - al / A; }
    else if (type === 1 || type === 2) { const s = type === 1 ? -1 : 1, sq = 2 * Math.sqrt(A) * al; b0 = A * ((A + 1) + s * (A - 1) * cs + sq); b1 = -2 * s * A * ((A - 1) + s * (A + 1) * cs); b2 = A * ((A + 1) + s * (A - 1) * cs - sq); a0 = (A + 1) - s * (A - 1) * cs + sq; a1 = 2 * s * ((A - 1) - s * (A + 1) * cs); a2 = (A + 1) - s * (A - 1) * cs - sq; }
    else if (type === 3) { b0 = (1 - cs) / 2; b1 = 1 - cs; b2 = b0; a0 = 1 + al; a1 = -2 * cs; a2 = 1 - al; }
    else if (type === 4) { b0 = (1 + cs) / 2; b1 = -(1 + cs); b2 = b0; a0 = 1 + al; a1 = -2 * cs; a2 = 1 - al; }
    else { b0 = 1; b1 = -2 * cs; b2 = 1; a0 = 1 + al; a1 = -2 * cs; a2 = 1 - al; }
    const w = 2 * Math.PI * f / sr, c1 = Math.cos(w), c2 = Math.cos(2 * w), s1 = Math.sin(w), s2 = Math.sin(2 * w);
    const nr = b0 + b1 * c1 + b2 * c2, ni = -(b1 * s1 + b2 * s2), dr = a0 + a1 * c1 + a2 * c2, di = -(a1 * s1 + a2 * s2);
    return 10 * Math.log10((nr * nr + ni * ni) / Math.max(1e-30, dr * dr + di * di));
  }
  V.eq = (x, W, H, o, t, cv) => {
    x.clearRect(0, 0, W, H); freqGrid(x, W, H);
    const n = Math.floor(W / 4); x.fillStyle = css('--ac-t'); x.globalAlpha = .22;
    for (let i = 0; i < n; i++) { const h = scopeBand(i / n) * H * .72; x.fillRect(i * 4, H - h, 3, h); }
    x.globalAlpha = 1;
    const e = o.fx ? FD.findFx(+o.fx) : null, sr = FD.sr();
    const nodes = cv._nodes = [1, 2, 3, 4, 5].map((b, i) => ({ b, f: FD.pval(e, `b${b}_freq`) || [70, 280, 1200, 3400, 11000][i], g: FD.pval(e, `b${b}_gain`) || 0, q: FD.pval(e, `b${b}_q`) || 1, ty: FD.pval(e, `b${b}_type`) || 0, c: BC[i] }));
    if (cv._drag) Object.assign(nodes[cv._drag.i], cv._drag.v);
    const out = FD.pval(e, 'output') || 0;
    const resp = u => { const f = 20 * Math.pow(1000, u); let s = out; nodes.forEach(nd => { s += biq(nd.ty, nd.f, nd.g, nd.q, f, sr); }); return clamp(s, -24, 24); };
    x.beginPath(); for (let px = 0; px <= W; px += 2) { const y = H / 2 - resp(px / W) / 24 * H; px === 0 ? x.moveTo(px, y) : x.lineTo(px, y); } x.lineWidth = 2.5; x.strokeStyle = css('--tx'); x.stroke();
    x.lineTo(W, H / 2); x.lineTo(0, H / 2); x.closePath(); x.fillStyle = css('--tx'); x.globalAlpha = .08; x.fill(); x.globalAlpha = 1;
    nodes.forEach((nd, i) => { const px = logX(nd.f, W), y = H / 2 - (nd.ty <= 2 ? nd.g : 0) / 24 * H; x.beginPath(); x.arc(px, y, 11, 0, 7); x.fillStyle = css(nd.c); x.fill(); x.fillStyle = css(nd.c + '-on'); x.font = '800 11px ' + css('--sans'); x.textAlign = 'center'; x.textBaseline = 'middle'; x.fillText(i + 1, px, y + .5); x.textAlign = 'left'; x.textBaseline = 'alphabetic'; });
    if (!cv._eqd) {
      cv._eqd = 1; cv.style.touchAction = 'none';
      cv.addEventListener('pointerdown', ev => { const r = cv.getBoundingClientRect(), mx = ev.clientX - r.left, my = ev.clientY - r.top; let best = -1, bd = 28; cv._nodes.forEach((nd, i) => { const d = Math.hypot(logX(nd.f, r.width) - mx, r.height / 2 - nd.g / 24 * r.height - my); if (d < bd) { bd = d; best = i; } }); if (best >= 0) { cv._drag = { i: best, v: {} }; cv.setPointerCapture(ev.pointerId); } });
      cv.addEventListener('pointermove', ev => { if (!cv._drag) return; const r = cv.getBoundingClientRect(), b = cv._drag.i + 1; cv._drag.v = { f: 20 * Math.pow(1000, clamp((ev.clientX - r.left) / r.width)), g: clamp((r.height / 2 - (ev.clientY - r.top)) / r.height * 24, -18, 18) }; if (cv.dataset.fx) FD.live(+cv.dataset.fx, { [`b${b}_freq`]: Math.round(cv._drag.v.f), [`b${b}_gain`]: Math.round(cv._drag.v.g * 10) / 10 }); });
      cv.addEventListener('pointerup', () => { if (cv._drag && cv.dataset.fx) FD.live(+cv.dataset.fx, null, true); cv._drag = null; });
    }
  };
  V.transfer = (x, W, H, o) => {
    x.clearRect(0, 0, W, H); x.strokeStyle = css('--ln'); for (let i = 1; i < 6; i++) { x.beginPath(); x.moveTo(i * W / 6, 0); x.lineTo(i * W / 6, H); x.moveTo(0, i * H / 6); x.lineTo(W, i * H / 6); x.stroke(); }
    x.strokeStyle = css('--tx3'); x.setLineDash([3, 4]); x.beginPath(); x.moveTo(0, H); x.lineTo(W, 0); x.stroke(); x.setLineDash([]);
    const th = +o.th || .62, ra = +o.ratio || 3.2; x.beginPath(); for (let i = 0; i <= 100; i++) { const u = i / 100, k = .09, v = u < th - k ? u : u > th + k ? th + (u - th) / ra : u + (1 / ra - 1) * Math.pow(u - th + k, 2) / (4 * k); const px = u * W, py = H - v * H; i ? x.lineTo(px, py) : x.moveTo(px, py); } x.lineWidth = 2.5; x.strokeStyle = css('--tx'); x.stroke();
    const lv = clamp((FD.lvl() + 60) / 60); x.beginPath(); x.arc(lv * W, H - (lv < th ? lv : th + (lv - th) / ra) * H, 6, 0, 7); x.fillStyle = css('--sig'); x.fill(); x.fillStyle = css('--tx3'); x.font = '500 10.5px "Geist Mono",monospace'; x.fillText('Input', W - 38, H - 6); x.fillText('Output', 5, 13);
  };
  /* Level history (bars) and gain reduction (signal), from the output meter and the compressor curve. */
  const hist = [];
  V.gr = (x, W, H, o) => {
    x.clearRect(0, 0, W, H); const n = Math.floor(W / 3), th = +o.thdb || -18, ra = +o.ratio || 3;
    for (let i = 0; i < n; i++) { const lv = hist[hist.length - n + i]; if (lv == null) continue; const v = clamp((lv + 60) / 60), over = Math.max(0, lv - th), gr = over * (1 - 1 / ra); x.fillStyle = css('--tx2'); x.globalAlpha = .55; const h = v * H * .8; x.fillRect(i * 3, H - h, 2, h); x.globalAlpha = 1; x.fillStyle = css('--sig'); x.fillRect(i * 3, 0, 2, clamp(gr / 24) * H * .34); }
  };
  V.gonio = (x, W, H, o, t) => {
    x.clearRect(0, 0, W, H); const c = Math.min(W, H) / 2 - 8, cx = W / 2, cy = H / 2; x.strokeStyle = css('--ln'); x.beginPath(); x.moveTo(cx - c, cy); x.lineTo(cx + c, cy); x.moveTo(cx, cy - c); x.lineTo(cx, cy + c); x.stroke(); x.fillStyle = css('--ac-t'); x.globalAlpha = .55;
    const s = FD.D.scope; if (s && s.l) { for (let i = 0; i < s.l.length; i++) { const l = s.l[i], r = s.r[i], side = (l - r) * .7071, m = (l + r) * .7071; x.fillRect(cx + clamp(side, -1, 1) * c, cy - clamp(m, -1, 1) * c, 1.5, 1.5); } }
    x.globalAlpha = 1;
  };
  V.spectrum = (x, W, H, o) => {
    x.clearRect(0, 0, W, H); const n = Math.floor(W / 5), still = o.still && FD.D.spec;
    for (let i = 0; i < n; i++) { const u = i / n; let v; if (still) { const s = FD.D.spec; v = clamp((s[Math.min(s.length - 1, Math.floor(u * s.length))] + 90) / 90); } else v = scopeBand(u); x.fillStyle = css('--ac-t'); const h = v * H; x.fillRect(i * 5, H - h, 4, h); }
  };
  /* History: one line of changes, oldest left. The current point is the signal color, future (redo) steps are faint. */
  V.history = (x, W, H) => {
    x.clearRect(0, 0, W, H); x.lineCap = 'round'; const h = FD.D.history, items = h ? h.items : []; const n = Math.max(2, items.length), y = H * .5;
    const show = Math.min(n, Math.max(4, Math.floor(W / 26))), from = Math.max(0, Math.min(n - show, (h ? h.current : 0) - Math.floor(show * .7))), st = W / (show + .6);
    x.strokeStyle = css('--ln2'); x.lineWidth = 2; x.beginPath(); x.moveTo(st * .5, y); x.lineTo(st * (show - .5), y); x.stroke();
    for (let k = 0; k < show; k++) { const it = items[from + k]; if (!it) continue; const cur = it.state === 'current'; x.beginPath(); x.arc(st * (k + .5), y, cur ? 8 : 5, 0, 7); x.fillStyle = cur ? css('--sig') : css('--tx2'); x.globalAlpha = it.state === 'future' ? .35 : 1; x.fill(); }
    x.globalAlpha = 1;
  };

  /* Sound stage: top-down. Sources are tracks; each moves the way its 3D/8D effect says. Drag to place, draw a path. */
  const MODES = ['orbit', 'spiral', 'eight', 'swarm', 'place', 'path'];
  V.stage = (x, W, H, o, t, cv) => {
    const p = FD.D.project, tr = p ? p.tracks : [];
    const st = cv._st || (cv._st = { a: 0, path: null });
    const srcs = tr.map((tk, i) => { const e = (tk.fx || []).find(f => f.kind === 'spatial') || ((p.master.fx || []).find(f => f.kind === 'spatial')); return { tk, e, c: '--t' + (1 + (tk.color != null ? tk.color : i) % 8), n: tk.name || 'Track' }; });
    const lead = srcs.find(s => s.e) || null;
    const mode = cv.dataset.mode || (lead ? MODES[Math.round(FD.pval(lead.e, 'mode') || 0)] : 'orbit');
    const rate = lead ? FD.pval(lead.e, 'rate') : .1, dir = lead ? Math.round(FD.pval(lead.e, 'direction') || 0) : 0;
    const playing = FD.D.st && FD.D.st.playing;
    st.a = (FD.D.st ? FD.D.st.seconds : 0) * rate * 2 * Math.PI * (dir === 1 ? -1 : 1);
    x.clearRect(0, 0, W, H); const c = Math.min(W, H) / 2 - 22, cx = W / 2, cy = H / 2; st.c = c; st.cx = cx; st.cy = cy;
    x.strokeStyle = css('--ln2'); x.lineWidth = 1;
    [.33, .66, 1].forEach(r => { x.beginPath(); x.arc(cx, cy, c * r, 0, 7); x.stroke(); });
    for (let d = 0; d < 360; d += 30) { const a = d * Math.PI / 180, l = d % 90 === 0 ? 12 : 6; x.beginPath(); x.moveTo(cx + Math.sin(a) * c, cy - Math.cos(a) * c); x.lineTo(cx + Math.sin(a) * (c - l), cy - Math.cos(a) * (c - l)); x.stroke(); }
    x.fillStyle = css('--tx3'); x.font = '500 10.5px "Geist Mono",monospace'; x.textAlign = 'center'; x.fillText('Front', cx, cy - c - 7); x.fillText('Back', cx, cy + c + 15); x.textAlign = 'left'; x.fillText('L', cx - c - 14, cy + 4); x.fillText('R', cx + c + 6, cy + 4);
    const path = st.path || (lead && lead.e.path && lead.e.path.length ? lead.e.path.map(q => [Math.sin(q[0] * Math.PI / 180) * q[1] / 10, -Math.cos(q[0] * Math.PI / 180) * q[1] / 10]) : null);
    const pos = (s, i) => {
      const az0 = (FD.pval(s.e, 'azimuth') || 0) * Math.PI / 180, r0 = clamp((FD.pval(s.e, 'distance') || 1.8) / 10 * 2.2, .16, 1);
      if (s.drag) return [s.drag.az, s.drag.r];
      if (!s.e || mode === 'place') return [s.e ? az0 : (i - (srcs.length - 1) / 2) * .5, s.e ? r0 : .7];
      const a = st.a + az0;
      if (mode === 'path' && path && path.length > 4) { const k = Math.floor(((st.a / (2 * Math.PI)) % 1 + 1) % 1 * path.length), q = path[k]; return [Math.atan2(q[0], -q[1]), Math.hypot(q[0], q[1])]; }
      if (mode === 'spiral') return [a, r0 * (.55 + .45 * Math.sin(st.a * .6 + i))];
      if (mode === 'eight') return [Math.atan2(Math.sin(a), -Math.sin(a * 2) * .6), Math.hypot(Math.sin(a), Math.sin(a * 2) * .6) * .9];
      if (mode === 'swarm') { const b = st.a * 1.3 + i * 3; return [Math.atan2(Math.sin(b) * .7 + Math.sin(b * 2.9) * .2, Math.cos(b * 1.7) * .7), .3 + .6 * Math.abs(Math.sin(b * .8))]; }
      return [a, r0];
    };
    if (path && path.length > 1) { x.beginPath(); path.forEach((q, i) => { const px = cx + q[0] * c, py = cy + q[1] * c; i ? x.lineTo(px, py) : x.moveTo(px, py); }); x.strokeStyle = css('--sig'); x.lineWidth = 2; x.setLineDash([5, 5]); x.stroke(); x.setLineDash([]); }
    cv._srcs = srcs;
    srcs.forEach((s, i) => {
      const prev = (cv._prev || [])[i]; if (prev && prev.drag) s.drag = prev.drag;
      const [az, r] = pos(s, i), px = cx + Math.sin(az) * r * c, py = cy - Math.cos(az) * r * c, col = css(s.c);
      s._x = px; s._y = py;
      const moving = s.e && mode !== 'place' && playing; s.tr = (prev && prev.tr) || [];
      if (moving) { s.tr.push([px, py]); if (s.tr.length > 46) s.tr.shift(); s.tr.forEach((q, k) => { x.globalAlpha = k / s.tr.length * .5; x.fillStyle = col; x.beginPath(); x.arc(q[0], q[1], 1.5 + k / s.tr.length * 6, 0, 7); x.fill(); }); x.globalAlpha = 1; }
      x.beginPath(); x.arc(px, py, 14, 0, 7); x.fillStyle = col; x.fill(); x.fillStyle = css(s.c + '-on'); x.font = '800 11px ' + css('--sans'); x.textAlign = 'center'; x.textBaseline = 'middle'; x.fillText(s.n[0], px, py + .5); x.textAlign = 'left'; x.textBaseline = 'alphabetic';
    });
    cv._prev = srcs;
    const hr = c * .12; x.beginPath(); x.arc(cx, cy, hr, 0, 7); x.fillStyle = css('--s3'); x.fill(); x.strokeStyle = css('--tx'); x.lineWidth = 2; x.stroke();
    x.beginPath(); x.moveTo(cx, cy - hr - 9); x.lineTo(cx - 7, cy - hr + 1); x.lineTo(cx + 7, cy - hr + 1); x.closePath(); x.fillStyle = css('--tx'); x.fill();
    x.fillRect(cx - hr - 4, cy - 6, 4, 12); x.fillRect(cx + hr, cy - 6, 4, 12);
    if (!cv._dr) {
      cv._dr = 1; cv.style.touchAction = 'none'; let drag = null, draw = false;
      cv.addEventListener('pointerdown', e => {
        const r = cv.getBoundingClientRect(), mx = e.clientX - r.left, my = e.clientY - r.top;
        if (cv.dataset.tool === 'draw') { draw = true; st.path = [[(mx - st.cx) / st.c, (my - st.cy) / st.c]]; cv.setPointerCapture(e.pointerId); return; }
        drag = (cv._prev || []).find(s => Math.hypot(s._x - mx, s._y - my) < 24); if (drag) { drag.drag = { az: 0, r: .5 }; cv.setPointerCapture(e.pointerId); }
      });
      cv.addEventListener('pointermove', e => {
        const r = cv.getBoundingClientRect(), mx = e.clientX - r.left - st.cx, my = e.clientY - r.top - st.cy;
        if (draw) { const q = [mx / st.c, my / st.c], l = st.path[st.path.length - 1]; if (Math.hypot(q[0] - l[0], q[1] - l[1]) > .025) st.path.push(q); return; }
        if (drag) { drag.drag = { az: Math.atan2(mx, -my), r: clamp(Math.hypot(mx, my) / st.c, .16, 1) }; }
      });
      cv.addEventListener('pointerup', () => {
        if (draw && st.path && st.path.length > 4) FD.stagePath(st.path.map(q => [Math.atan2(q[0], -q[1]) * 180 / Math.PI, clamp(Math.hypot(q[0], q[1]), .05, 1) * 10 / 2.2]));
        if (drag && drag.drag) { const d = drag.drag; drag.drag = null; FD.stagePlace(drag.tk.id, d.az * 180 / Math.PI, d.r * 10 / 2.2); }
        drag = null; draw = false;
      });
    }
  };
  /* Stage elevation strip: the lead source's elevation. */
  V.elev = (x, W, H) => {
    x.clearRect(0, 0, W, H); x.strokeStyle = css('--ln2'); x.beginPath(); x.moveTo(0, H / 2); x.lineTo(W, H / 2); x.stroke(); x.fillStyle = css('--tx3'); x.font = '500 10.5px "Geist Mono",monospace'; x.fillText('Above', 6, 13); x.fillText('Below', 6, H - 5);
    const e = FD.fxOf('spatial'), el = e ? FD.pval(e, 'elevation') : 0, cx = W / 2; x.beginPath(); x.arc(cx, H / 2, 11, 0, 7); x.strokeStyle = css('--tx'); x.lineWidth = 2; x.stroke();
    const sec = FD.D.st ? FD.D.st.seconds : 0, rate = e ? FD.pval(e, 'rate') : 0, y = H / 2 - el / 90 * H * .4, px = cx + Math.cos(sec * rate * 2 * Math.PI) * W * .3; x.beginPath(); x.arc(px, y, 8, 0, 7); x.fillStyle = css('--t3'); x.fill();
  };
  /* Room box in perspective */
  V.room = (x, W, H, o, t) => {
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
  /* Reverb decay: the envelope of a room with this RT60 (exponential, -60 dB at rt). Fixed seed, so it doesn't flicker. */
  V.decay = (x, W, H, o) => { x.clearRect(0, 0, W, H); const r = prng(11), rt = +(o.rt || 2.4) / 8; for (let i = 0; i < W; i += 2) { const u = i / W, v = Math.exp(-u / rt * 2.4) * (.5 + .5 * r()) * (u < .03 ? u / .03 : 1); x.fillStyle = css('--ac-t'); const h = v * H * .9; x.fillRect(i, H - h, 1.4, h); } };
  /* Short-term loudness of the project over time against the target. */
  V.lufs = (x, W, H) => {
    x.clearRect(0, 0, W, H); const L = FD.D.loudness && FD.D.loudness.loudness, cur = L ? L.curve : [], tg = (FD.D.project && FD.D.project.target_lufs) || -14, ym = l => H * .9 - clamp((l + 40) / 40) * H * .8;
    if (cur.length > 1) { x.beginPath(); cur.forEach((l, i) => { const px = i / (cur.length - 1) * W, y = ym(Math.max(-40, l)); i ? x.lineTo(px, y) : x.moveTo(px, y); }); x.strokeStyle = css('--ac-t'); x.lineWidth = 2; x.stroke(); }
    x.strokeStyle = css('--sig'); x.setLineDash([4, 4]); x.beginPath(); x.moveTo(0, ym(tg)); x.lineTo(W, ym(tg)); x.stroke(); x.setLineDash([]); x.fillStyle = css('--tx3'); x.font = '500 10.5px "Geist Mono",monospace'; x.fillText('Target ' + FD.minus(tg.toFixed(0)) + ' LUFS', 6, ym(tg) - 5);
  };

  const live = new Set();
  const LIVE = { eq: 1, gr: 1, gonio: 1, spectrum: 1, stage: 1, elev: 1, room: 1, transfer: 1 };
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
  let lastHist = 0;
  function loop(ms) {
    const t = ms / 1000;
    if (ms - lastHist > 33) { lastHist = ms; hist.push(FD.lvl()); if (hist.length > 1200) hist.splice(0, 200); }
    live.forEach(c => { if (c.isConnected) draw(c, t); }); requestAnimationFrame(loop);
  }
  requestAnimationFrame(loop);
  return { mount, needsScope: () => [...live].some(c => c.isConnected && /eq|gonio|spectrum|gr|transfer/.test(c.dataset.v)), redraw: root => (root || document).querySelectorAll('canvas[data-v]').forEach(c => draw(c, performance.now() / 1000)) };
})();
