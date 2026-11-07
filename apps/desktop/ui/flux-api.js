/* Flux desktop: the bridge to the engine, shared state and formatting. Everything the screens show comes from here. */
window.FD = (function () {
  const T = window.__TAURI__;
  let token = null;
  try {
    const m = location.hash.match(/token=([0-9a-f]+)/);
    if (m) { token = m[1]; sessionStorage.setItem('flux-token', token); history.replaceState(null, '', location.pathname + '#home'); }
    else token = sessionStorage.getItem('flux-token');
  } catch (e) {}
  const native = !!(T && T.core && T.core.invoke);

  async function call(method, params) {
    params = params || {};
    if (native) {
      try { return await T.core.invoke('call', { method, params }); } catch (e) { throw norm(e); }
    }
    const h = { 'Content-Type': 'application/json' };
    if (token) h.Authorization = 'Bearer ' + token;
    let r;
    try { r = await fetch('/api/call', { method: 'POST', headers: h, body: JSON.stringify({ method, params }) }); } catch (e) { throw { code: 'unavailable', message: 'Flux engine is not running.' }; }
    const j = await r.json().catch(() => ({ ok: false, error: { code: 'failed', message: 'Bad response from the engine' } }));
    if (!j.ok) throw norm(j.error);
    return j.result;
  }
  const norm = e => (e && typeof e === 'object' && e.message) ? e : { code: 'failed', message: String(e && e.message || e || 'Something went wrong') };

  /* Native dialogs (desktop). In a browser (flux-mcp serve) files are uploaded to the engine instead. */
  async function pickFiles(o) {
    o = o || {};
    if (native) return (await T.core.invoke('pick_files', { multiple: !!o.multiple, kind: o.kind || 'audio' })) || [];
    return new Promise(res => {
      const i = document.createElement('input'); i.type = 'file'; i.multiple = !!o.multiple;
      i.accept = o.kind === 'flux' ? '.flux' : o.kind === 'preset' ? '.json,.fluxpreset' : o.kind === 'live' ? '.fluxlive,.json' : 'audio/*,video/*,.flac,.m4a,.aiff,.aif,.ogg,.opus,.wav,.mp3';
      i.onchange = async () => { const out = []; for (const f of i.files) out.push(await upload(f)); res(out); };
      i.click();
    });
  }
  async function upload(f) {
    const h = token ? { Authorization: 'Bearer ' + token } : {};
    const r = await fetch('/api/upload?name=' + encodeURIComponent(f.name), { method: 'POST', headers: h, body: f });
    const j = await r.json(); if (!j.ok) throw norm(j.error); return j.result.path;
  }
  async function pickFolder() { return native ? T.core.invoke('pick_folder') : null; }
  /* Ask where to save. Returns a path, null if cancelled, or '' in a browser (the engine then uses its default folder).
     With `text`, the text is written there (a browser downloads it instead). */
  async function saveAs(o) {
    o = o || {};
    if (native) { const path = await T.core.invoke('pick_save', { name: o.name || 'Untitled', kind: o.kind || 'audio' }); if (path && o.text != null) await T.core.invoke('write_text', { path, text: o.text }); return path || null; }
    if (o.text != null) { const a = document.createElement('a'); a.href = URL.createObjectURL(new Blob([o.text], { type: 'application/json' })); a.download = o.name || 'flux.txt'; document.body.appendChild(a); a.click(); setTimeout(() => { URL.revokeObjectURL(a.href); a.remove(); }, 1000); return o.name; }
    return '';
  }
  async function reveal(path) { if (native && path) return T.core.invoke('reveal', { path }); }
  async function openUrl(url) { if (native) return T.core.invoke('open_url', { url }); window.open(url, '_blank', 'noopener'); }
  /* In-app updates (desktop only). The shell downloads new releases in the background. */
  const updater = native ? {
    status: () => T.core.invoke('update_status'),
    check: () => T.core.invoke('update_check'),
    install: () => T.core.invoke('update_install'),
    on: fn => { if (T.event) { T.event.listen('flux://update-status', e => fn('status', e.payload)); T.event.listen('flux://update-ready', e => fn('ready', e.payload)); } }
  } : null;
  function onDrop(fn) {
    if (native && T.event) { T.event.listen('tauri://drag-drop', e => { const p = e.payload && e.payload.paths; if (p && p.length) fn(p); }); return; }
    addEventListener('dragover', e => e.preventDefault());
    addEventListener('drop', async e => { e.preventDefault(); const out = []; for (const f of e.dataTransfer.files) out.push(await upload(f)); if (out.length) fn(out); });
  }

  /* ---- state ---- */
  const D = { project: null, st: { rev: 0, pos: 0, seconds: 0, playing: false, peak: [0, 0], lufs_i: -70, lufs_s: -70, lufs_m: -70 }, sel: { clip: null, track: null, fx: null, target: null }, ui: {} };
  const LOAD = {
    info: () => call('app.info'), update: () => updater ? updater.status() : Promise.resolve(null), caps: () => call('app.capabilities'), settings: () => call('settings.get'),
    project: async () => { try { return await call('project.get'); } catch (e) { return null; } },
    library: () => call('library.list', D.ui.libq || {}), jobs: () => call('job.list'), history: () => call('history.list'), snapshots: () => call('snapshot.list'),
    presets: () => call('preset.list', D.ui.presetKind ? { kind: D.ui.presetKind } : {}), actions: () => call('action.list'), styles: () => call('style.list'), catalog: () => call('fx.catalog'),
    devices: () => call('audio.devices'), voices: () => call('tts.voices'), formats: () => call('export.formats'), usage: () => call('storage.usage'), ai: () => call('ai.status'),
    lkinds: () => call('live.kinds'), lpresets: () => call('live.presets'), lstatus: () => call('live.status'), ldevices: () => call('live.devices'),
    lpreset: () => D.ui.live && D.ui.live.sel ? call('live.preset.get', { id: D.ui.live.sel }) : Promise.resolve(null),
    mcp: () => call('mcp.info'), recovery: () => call('recovery.list'), diag: () => call('system.diagnostics'),
    loudness: () => call('analysis.loudness'), tempokey: () => call('analysis.tempo_key'), problems: () => call('analysis.problems'),
  };
  const ok = {};
  async function need(keys, force) {
    const todo = keys.filter(k => force || !ok[k]);
    await Promise.all(todo.map(async k => { try { D[k] = await LOAD[k](); ok[k] = 1; } catch (e) { D[k] = null; D.err = D.err || {}; D.err[k] = e; } }));
  }
  function stale(...keys) { keys.forEach(k => delete ok[k]); }
  const PROJECT_KEYS = ['project', 'history', 'loudness', 'tempokey', 'problems'];

  /* ---- cached drawing data ---- */
  const pk = new Map(), sg = new Map();
  function peaks(media, start, end, bins, reverse) {
    const k = [media, start, end, bins, reverse ? 1 : 0].join(':');
    if (!pk.has(k)) {
      if (pk.size > 400) pk.delete(pk.keys().next().value);
      pk.set(k, call('media.peaks', { media, start: Math.max(0, Math.round(start)), end: Math.round(end), bins: Math.max(8, Math.min(4000, Math.round(bins))), reverse: !!reverse }).catch(() => null));
    }
    return pk.get(k);
  }
  function spectrogram(p) {
    const k = JSON.stringify(p) + ':' + (D.project && D.project.hash);
    if (!sg.has(k)) { if (sg.size > 40) sg.delete(sg.keys().next().value); sg.set(k, call('analysis.spectrogram', p).catch(() => null)); }
    return sg.get(k);
  }

  /* ---- formatting ---- */
  const esc = s => String(s == null ? '' : s).replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
  const minus = s => String(s).replace(/^-/, '−');
  const db = (x, d = 1) => (x == null || !isFinite(x) || x <= -69.9) ? '−∞' : minus((x > 0 ? '+' : '') + Number(x).toFixed(d));
  const num = (x, d = 1) => (x == null || !isFinite(x)) ? '—' : minus(Number(x).toFixed(d));
  function tc(sec) { sec = Math.max(0, sec || 0); const h = Math.floor(sec / 3600), m = Math.floor(sec / 60) % 60, s = sec % 60; return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${s.toFixed(3).padStart(6, '0')}`; }
  function len(sec) { sec = Math.max(0, Math.round(sec || 0)); const h = Math.floor(sec / 3600), m = Math.floor(sec / 60) % 60, s = sec % 60; return h ? `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}` : `${m}:${String(s).padStart(2, '0')}`; }
  function pos(samples, sr) { const s = (samples || 0) / (sr || 48000); const m = Math.floor(s / 60); return `${m}:${(s - m * 60).toFixed(3).padStart(6, '0')}`; }
  function bytes(b) { b = b || 0; if (b < 1024) return b + ' B'; const u = ['KB', 'MB', 'GB', 'TB']; let i = -1; do { b /= 1024; i++; } while (b >= 1024 && i < 3); return (b < 10 ? b.toFixed(1) : Math.round(b)) + ' ' + u[i]; }
  function ago(t) {
    if (!t) return '';
    const ms = t > 1e12 ? t : t * 1000, s = Math.max(0, (Date.now() - ms) / 1000);
    if (s < 5) return 'just now'; if (s < 60) return Math.round(s) + ' s ago'; if (s < 3600) return Math.round(s / 60) + ' min ago';
    if (s < 86400) return Math.round(s / 3600) + ' h ago'; if (s < 172800) return 'Yesterday';
    const d = new Date(ms); return s < 7 * 86400 ? d.toLocaleDateString(undefined, { weekday: 'short' }) : d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
  }
  const khz = r => (r % 1000 ? (r / 1000).toFixed(1) : r / 1000) + ' kHz';
  const hz = f => f >= 1000 ? (f / 1000).toFixed(f >= 10000 ? 0 : 1) + ' kHz' : f < 10 ? (f < 1 ? f.toFixed(2) : f.toFixed(1)) + ' Hz' : Math.round(f) + ' Hz';
  const sr = () => (D.project && D.project.sample_rate) || 48000;

  /* ---- project helpers ---- */
  const tracks = () => (D.project && D.project.tracks) || [];
  const clips = () => tracks().flatMap(t => t.clips.map(c => Object.assign({ track: t.id }, c)));
  const clip = id => clips().find(c => c.id === id);
  const track = id => tracks().find(t => t.id === id);
  function chain(target) {
    const p = D.project; if (!p) return [];
    if (!target || target === 'master') return p.master.fx || [];
    const [k, id] = target.split(':');
    if (k === 'track') { const t = track(+id); return t ? t.fx : []; }
    if (k === 'clip') { const c = clip(+id); return c ? c.fx : []; }
    return [];
  }
  function findFx(id) {
    if (!D.project) return null;
    const all = [['master', D.project.master.fx]].concat(tracks().map(t => ['track:' + t.id, t.fx]), clips().map(c => ['clip:' + c.id, c.fx]));
    for (const [tg, list] of all) { const e = (list || []).find(x => x.id === id); if (e) return Object.assign({ target: tg }, e); }
    return null;
  }
  const spec = kind => (D.catalog || []).find(s => s.kind === kind);
  /* The effect a screen edits: the selected one if it is that kind, else the first of that kind on the current target. */
  function fxOf(kinds, target) {
    kinds = [].concat(kinds);
    const s = D.sel.fx && findFx(D.sel.fx); if (s && kinds.includes(s.kind)) return s;
    const t = target || D.sel.target || 'master';
    const e = chain(t).find(x => kinds.includes(x.kind)) || (D.project ? [D.project.master.fx].concat(tracks().map(t => t.fx)).flat().find(x => x && kinds.includes(x.kind)) : null);
    return e ? findFx(e.id) : null;
  }
  function pval(e, key) { if (!e) { return undefined; } if (e.params && key in e.params) return e.params[key]; const s = spec(e.kind); const p = s && s.params.find(q => q.key === key); return p ? p.default : undefined; }
  function pct(p, v) { if (!p) return 0; if (p.log && p.min > 0) return Math.round(Math.log(v / p.min) / Math.log(p.max / p.min) * 100); return Math.round((v - p.min) / (p.max - p.min) * 100); }
  function unpct(p, x) { x = Math.min(100, Math.max(0, x)) / 100; let v = p.log && p.min > 0 ? p.min * Math.pow(p.max / p.min, x) : p.min + (p.max - p.min) * x; if (p.unit === 'int' || p.unit === 'bool' || p.unit === 'enum') v = Math.round(v); return v; }
  function fmtp(p, v) {
    if (!p) return '';
    const u = p.unit || '';
    if (u === 'enum') { const o = (p.options || [])[Math.round(v)] || ''; return o.charAt(0).toUpperCase() + o.slice(1); }
    if (u === 'bool') return v >= .5 ? 'On' : 'Off';
    if (u === 'Hz') return hz(v);
    if (u === 'dB') return db(v, 1) + ' dB';
    if (u === 'ms') return v >= 1000 ? (v / 1000).toFixed(2) + ' s' : (v < 10 ? v.toFixed(1) : Math.round(v)) + ' ms';
    if (u === 's') return v.toFixed(v < 10 ? 2 : 1) + ' s';
    if (u === '%') return Math.round(v) + '%';
    if (u === 'st') return minus((v > 0 ? '+' : '') + v.toFixed(1)) + ' st';
    if (u === 'x') return v.toFixed(2) + '×';
    if (u === 'deg') return minus(String(Math.round(v))) + '°';
    if (u === 'm') return v.toFixed(1) + ' m';
    if (u === 'cents') return minus((v > 0 ? '+' : '') + Math.round(v)) + ' cents';
    if (u === 'bit') return Math.round(v) + ' bit';
    if (u === '/s') return v.toFixed(1) + ' /s';
    if (u === ':1') return v.toFixed(1) + ':1';
    if (u === 'int') return String(Math.round(v));
    return (Math.abs(v) < 10 ? minus(v.toFixed(2)) : minus(String(Math.round(v)))) + (u ? ' ' + u : '');
  }
  const kindName = k => { const s = spec(k); return s ? s.name : k; };

  return { call, native, updater, pickFiles, pickFolder, saveAs, reveal, openUrl, onDrop, D, need, stale, PROJECT_KEYS, peaks, spectrogram, esc, db, num, tc, len, pos, bytes, ago, khz, hz, sr, minus, tracks, clips, clip, track, chain, findFx, spec, fxOf, pval, pct, unpct, fmtp, kindName };
})();
