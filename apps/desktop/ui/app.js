/* Flux desktop behavior: routing, transitions and shared controls (ported from design/app.js), wired to the engine. */
(function () {
  const { S, M, ic, THEMES, FONTS } = Flux, F = FD, D = FD.D, stage = document.getElementById('stage');
  const calm = () => matchMedia('(prefers-reduced-motion: reduce)').matches || (D.settings && D.settings.reduce_motion);
  let cur = null, lastRev = -1, dragging = false;
  const P = () => D.project;

  /* ---------- engine bridge helpers ---------- */
  const call = (m, p) => F.call(m, p);
  async function act(m, p, o = {}) {
    try { const r = await call(m, p); if (o.refresh !== false) await refresh(); return r; }
    catch (e) { fail(e, o); return null; }
  }
  function fail(e, o = {}) {
    if (o.quiet) return;
    D.ui.error = { code: e.code || 'failed', message: e.message || String(e), title: o.title };
    D.ui.errorBase = cur && cur !== 'error' ? cur : D.ui.errorBase;
    render('error');
  }
  async function refresh(keys) {
    F.stale(...F.PROJECT_KEYS, ...(keys || []));
    await F.need(['project'].concat(keys || []), true);
    lastRev = D.project ? D.project.rev : lastRev;
    rr();
  }

  /* ---------- per-screen data ---------- */
  const NEED = {
    boot: ['caps'], unsupported: ['caps'], home: ['library'], library: ['library'], 'project-menu': ['library'], record: ['devices'], history: ['history', 'snapshots'],
    mix: ['loudness', 'problems'], meters: ['loudness', 'problems'], 'fx-eq': ['presets'], 'fx-clean': ['problems'], fxp: ['presets'], rooms: ['styles'], aitools: ['ai'], magic: ['ai'],
    analysis: ['tempokey'], master: ['loudness'], tts: ['voices'], jobs: ['jobs'], export: ['formats', 'loudness'], exporting: [], convert: ['formats'], batch: ['styles', 'presets', 'formats'],
    macros: ['actions'], presets: ['presets'], 'prefs-audio': ['devices'], 'prefs-models': ['ai'], 'prefs-storage': ['usage'], 'prefs-agents': ['mcp', 'info'], 'help-agents': ['mcp', 'info'],
    live: ['lkinds', 'lpresets', 'lstatus', 'ldevices', 'lpreset'],
    'prefs-about': ['info', 'update'], diag: ['diag'], recovery: ['recovery'], quota: ['usage'], script: ['ai'], fillers: ['ai'], dub: ['ai'], dubreview: ['ai'], clone: ['cstatus', 'cprofiles', 'ctags', 'csettings'], 'clone-get': ['cstatus'], 'stems-setup': ['ai'], 'prefs-privacy': [],
  };
  const ALWAYS = ['settings', 'catalog', 'project', 'info'];
  const ENTER = {
    magic: () => { if (!D.ui.magic && P() && P().length) call('tool.magic.scan', {}).then(r => { D.ui.magic = r; D.ui.magicOff = []; rr(); }, e => fail(e)); },
    master: () => { if (!D.ui.masterPlan && P() && P().length) planMaster(); },
    'fx-silence': () => { if (!D.ui.silScan && P() && P().length) silScan(); },
    analysis: () => { },
  };
  async function planMaster() { const st = ['gentle', 'balanced', 'loud'][D.ui.masterStyle != null ? D.ui.masterStyle : 1]; try { D.ui.masterPlan = await call('tool.master.analyze', { style: st, target: D.ui.masterTarget || (P() && P().target_lufs) || -14, amount: D.ui.masterAmt == null ? .5 : D.ui.masterAmt }); rr(); } catch (e) { fail(e); } }

  /* ---------- rendering (from design/app.js) ---------- */
  function placeInd(seg) { const on = seg.querySelector('button.on'), ind = seg.querySelector('.ind'); if (!ind) return; if (!on) { ind.style.width = '0px'; return; } ind.style.width = on.offsetWidth + 'px'; ind.style.transform = `translateX(${on.offsetLeft}px)`; }
  function hydrate(root) {
    if (Flux.hideTip) Flux.hideTip();
    root.querySelectorAll('.seg').forEach(s => { const ind = s.querySelector('.ind'); if (ind) ind.style.transition = 'none'; placeInd(s); if (ind) { ind.getBoundingClientRect(); ind.style.transition = ''; } });
    root.querySelectorAll('.in').forEach(el => [...el.children].forEach((c, i) => c.style.setProperty('--n', Math.min(i, 14))));
    Viz.mount(root);
    if (Flux.applyTips) Flux.applyTips(root);
    splitType(root);
    const ct = THEMES.find(t => t[0] === theme()) || THEMES[0];
    root.querySelectorAll('[data-set-theme]').forEach(b => b.setAttribute('aria-pressed', b.dataset.setTheme === ct[0]));
    root.querySelectorAll('[data-theme-name]').forEach(e => e.textContent = ct[1]);
    const cf = fontRow();
    root.querySelectorAll('[data-set-font]').forEach(b => b.setAttribute('aria-pressed', b.dataset.setFont === cf[0]));
    root.querySelectorAll('[data-font-name]').forEach(e => e.textContent = cf[1]);
    const rs = getComputedStyle(document.documentElement); kinLo = +rs.getPropertyValue('--kin-lo') || 640; kinHi = +rs.getPropertyValue('--kin-hi') || 900;
    root.querySelectorAll('[data-play]').forEach(b => { b.innerHTML = ic(D.st.playing ? 'pause' : 'play'); b.setAttribute('aria-label', D.st.playing ? 'Pause' : 'Play'); });
    const pal = root.querySelector('[data-palette]'); if (pal) { pal.focus(); pal.setSelectionRange(pal.value.length, pal.value.length); }
  }

  let kin = [], kinLo = 640, kinHi = 900, px = -1e4;
  function splitType(root) {
    kin = [];
    root.querySelectorAll('.h1,.bigh').forEach(h => {
      if (h.closest('[aria-hidden]') || h.querySelector('a,span.act,select,input')) return;
      const text = h.textContent; h.setAttribute('aria-label', text); h.classList.add('kin');
      h.innerHTML = text.split(' ').map(w => `<span class="wd" aria-hidden="true">${[...w].map(c => `<span class="lt">${F.esc(c)}</span>`).join('')}</span>`).join(' ');
      const fs = parseFloat(getComputedStyle(h).fontSize) || 1;
      h.querySelectorAll('.lt').forEach((l, i) => { const r = l.getBoundingClientRect(); l.style.width = (r.width / fs) + 'em'; kin.push({ l, i, x: r.left + r.width / 2 }); });
    });
  }
  addEventListener('pointermove', e => { px = e.clientX; }, { passive: true });

  /* Themes and fonts: applied at once, saved to settings so they follow the user across launches. */
  const theme = () => document.documentElement.dataset.flux || 'cassette';
  const fontRow = () => FONTS.find(x => x[0] === (document.documentElement.dataset.font || 'bricolage')) || FONTS[0];
  const canVT = () => document.startViewTransition && !calm() && document.visibilityState === 'visible';
  const quiet = vt => { vt.ready.catch(() => { }); vt.finished.catch(() => { }); return vt; };
  function applyLook(t, f) {
    const root = document.documentElement;
    if (t && THEMES.some(x => x[0] === t)) { root.dataset.flux = t; try { localStorage.setItem('flux-theme', t); } catch (e) { } }
    if (f && FONTS.some(x => x[0] === f)) { root.dataset.font = f; try { localStorage.setItem('flux-font', f); } catch (e) { } }
    const s = D.settings || {};
    root.classList.toggle('hc', !!s.high_contrast); root.classList.toggle('lg', !!s.large_text); root.classList.toggle('calm', !!s.reduce_motion);
  }
  function setTheme(id, ev) { if (id === theme()) return; reveal(() => { applyLook(id); paint(); }, ev); call('settings.set', { values: { theme: id } }).then(s => { D.settings = s; }, () => { }); }
  function setFont(id, ev) {
    const f = FONTS.find(x => x[0] === id); if (!f || f === fontRow()) return;
    const go = () => reveal(() => { applyLook(null, id); paint(); }, ev);
    document.fonts.load(`800 40px '${f[1]}'`).then(go, go);
    call('settings.set', { values: { font: id } }).then(s => { D.settings = s; }, () => { });
  }
  function reveal(apply, ev) {
    if (!canVT()) { apply(); return; }
    const x = ev && ev.clientX || innerWidth / 2, y = ev && ev.clientY || innerHeight / 2, r = Math.hypot(Math.max(x, innerWidth - x), Math.max(y, innerHeight - y)), root = document.documentElement;
    root.classList.add('vt-theme');
    const vt = quiet(document.startViewTransition(apply));
    vt.ready.then(() => root.animate({ clipPath: [`circle(0px at ${x}px ${y}px)`, `circle(${r}px at ${x}px ${y}px)`] }, { duration: 950, easing: 'cubic-bezier(.16,1,.3,1)', pseudoElement: '::view-transition-new(root)' })).catch(() => { });
    vt.finished.catch(() => { }).finally(() => root.classList.remove('vt-theme'));
  }

  function paint() { if (!cur) return; stage.innerHTML = M[cur].fn(); hydrate(stage); }
  let navSeq = 0;
  async function render(id, push) {
    if (!M[id]) id = P() ? 'edit' : 'home';
    const seq = ++navSeq;
    await F.need(ALWAYS.concat(NEED[id] || []));
    if (seq !== navSeq) return;
    const go = () => { stage.innerHTML = M[id].fn(); const prev = cur; cur = id; hydrate(stage); document.title = (P() ? P().name + ' · ' : '') + 'Flux'; const m = stage.querySelector('#m'); if (m && prev !== id) m.scrollTop = 0; };
    if (canVT() && cur && cur !== id) quiet(document.startViewTransition(go)); else go();
    if (push !== false) { try { history.replaceState(null, '', '#' + id); } catch (e) { location.hash = id; } }
    if (ENTER[id]) ENTER[id]();
    if (D.ui.updatePending && !QUIET.has(id)) setTimeout(showUpdate, 600);
  }
  /* A downloaded update asks once, but never over a recording, an export or another dialog. */
  const QUIET = new Set(['record', 'exporting', 'error', 'update', 'boot', 'recovery']);
  function showUpdate() {
    if (!D.update || !D.update.ready || D.ui.updateShown) return;
    if (QUIET.has(cur) || (D.st && D.st.recording) || stage.querySelector('.ov')) { D.ui.updatePending = true; return; }
    D.ui.updatePending = false; D.ui.updateShown = true; D.ui.updateBase = cur; render('update');
  }
  Flux.render = render;
  /* Re-render the current screen in place: keeps scroll, focus and caret. */
  let rrPending = false;
  function rr() {
    if (!cur || dragging) return; if (rrPending) return; rrPending = true;
    requestAnimationFrame(() => {
      rrPending = false; if (dragging) return;
      const m = stage.querySelector('#m'), st = m ? m.scrollTop : 0, aside = stage.querySelector('aside.b'), ast = aside ? aside.scrollTop : 0;
      const a = document.activeElement, all = [...stage.querySelectorAll('a,button,input,select,textarea,[tabindex]')], fi = all.indexOf(a), caret = a && 'selectionStart' in a ? [a.selectionStart, a.selectionEnd] : null;
      stage.innerHTML = M[cur].fn(); hydrate(stage);
      const m2 = stage.querySelector('#m'); if (m2) m2.scrollTop = st; const a2 = stage.querySelector('aside.b'); if (a2) a2.scrollTop = ast;
      if (fi >= 0) { const n = [...stage.querySelectorAll('a,button,input,select,textarea,[tabindex]')][fi]; if (n) { n.focus({ preventScroll: true }); if (caret && 'setSelectionRange' in n) try { n.setSelectionRange(caret[0], caret[1]); } catch (e) { } } }
    });
  }
  Flux.rr = rr;

  /* ---------- jobs ---------- */
  async function follow(id, screen, done) {
    D.ui.job = { id, state: 'running', progress: 0 };
    if (screen) await render(screen);
    for (; ;) {
      let j; try { j = await call('job.get', { id, wait_s: .25 }); } catch (e) { fail(e); return null; }
      D.ui.job = j; if (cur === screen || cur === 'jobs') { if (cur === 'jobs') F.stale('jobs'), await F.need(['jobs']); rr(); }
      if (j.state !== 'running') { if (j.state === 'done' && done) await done(j.result, j); return j; }
    }
  }

  /* ---------- actions ---------- */
  const tgt = () => D.sel.target || 'master';
  const ds = el => el.dataset;
  const pos = () => D.st.pos || 0;
  async function addFx(kind, target, params) { const r = await call('fx.add', Object.assign({ kind, target: target || tgt() }, params ? { params } : {})); D.sel.fx = r.fx; await refresh(); return r.fx; }
  async function setParam(el, k, v) { const d = ds(el); if (d.fx) await act('fx.set', { fx: +d.fx, params: { [k]: v } }); else if (d.kind) { try { await addFx(d.kind, null, { [k]: v }); } catch (e) { fail(e); } } }
  async function importFiles(paths, then) {
    for (const p of paths) {
      D.ui.importName = p.split(/[\\/]/).pop();
      let r; try { r = await call('media.import', { path: p }); } catch (e) { fail(e, { title: 'Flux couldn\'t open that file.' }); return; }
      D.ui.importMedia = null;
      await follow(r.job, 'import', async res => { D.ui.importMedia = res.media; await refresh(['library']); });
    }
    if (then) render(then); else if (cur === 'import') { const j = D.ui.job; if (j && j.state === 'done') render(D.settings && D.settings.mode === 'simple' ? 'task' : 'edit'); }
  }
  async function silScan() { const o = D.ui.sil || { threshold_db: -42, longer_than_s: .6, shorten_to_s: .3, padding_s: .15 }; try { D.ui.silScan = await call('tool.silence.scan', o); D.ui.silKeep = []; rr(); } catch (e) { fail(e); } }
  const togg = (arr, v) => arr.includes(v) ? arr.filter(x => x !== v) : arr.concat([v]);
  let taps = [];
  const exportOpts = o => { const x = { bits: o.bits, mono: !!o.mono, dither: o.dither !== 'none', normalize: o.normalize, target: o.target, ceiling: o.ceiling, tags: { title: o.title || undefined, artist: o.artist || undefined } }; if (o.sample_rate) x.sample_rate = o.sample_rate; if (!['wav', 'flac'].includes(o.format)) x.bitrate = o.bitrate; if (o.fade) x.fade_out_s = 2; return x; };
  const exportDir = () => (D.settings && D.settings.export_dir) || undefined;

  const ACT = {
    /* first run */
    recheck: async () => { F.stale('caps'); await F.need(['caps'], true); rr(); },
    'set-mode': async el => { await act('settings.set', { values: { mode: ds(el).mode } }, { refresh: false }).then(s => s && (D.settings = s)); },
    onboarded: async () => { const s = await act('settings.set', { values: { onboarded: true } }, { refresh: false }); if (s) D.settings = s; },
    tone: el => act('audio.test_tone', { kind: ds(el).kind === 'move' ? 'moving' : ds(el).kind }, { refresh: false }),
    'hp-answer': async el => { D.ui.hp = ds(el).v; const s = await act('settings.set', { values: { headphones_checked: true } }, { refresh: false }); if (s) D.settings = s; rr(); if (ds(el).v === 'swapped') D.ui.hpSwap = true; },
    hrtf: async el => { const s = await act('settings.set', { values: { hrtf_profile: ds(el).v, head_width_cm: +ds(el).w } }, { refresh: false }); if (s) { D.settings = s; rr(); act('audio.test_tone', { kind: 'moving' }, { refresh: false }); } },
    'pick-export-dir': async () => { const d = await F.pickFolder(); if (d) { const s = await act('settings.set', { values: { export_dir: d } }, { refresh: false }); if (s) D.settings = s; rr(); } },
    set: async el => { const v = ds(el).v; const s = await act('settings.set', { values: { [ds(el).k]: v === '' ? null : v } }, { refresh: false }); if (s) D.settings = s; rr(); },
    'set-toggle': async el => { const k = ds(el).k; const s = await act('settings.set', { values: { [k]: !(D.settings || {})[k] } }, { refresh: false }); if (s) { D.settings = s; applyLook(); } rr(); },
    /* home and library */
    import: async el => { const fs = await F.pickFiles({ multiple: true, kind: 'audio' }); if (fs && fs.length) importFiles(fs, ds(el).then); },
    'import-package': async () => { const fs = await F.pickFiles({ multiple: false, kind: 'flux' }); if (fs && fs[0]) { await act('project.import_package', { path: fs[0] }); F.stale('library'); render('edit'); } },
    'open-project': async el => { await act('project.open', { id: ds(el).id }); D.sel = { clip: null, track: null, fx: null, target: null }; resetProjectUi(); F.stale('library'); render('edit'); },
    'project-menu': el => { D.ui.menu = ds(el).id; render('project-menu'); },
    'project-dup': async el => { await act('project.duplicate', { id: ds(el).id }, { refresh: false }); F.stale('library'); render('library'); },
    'project-delete': async el => { await act('project.delete', { id: ds(el).id }, { refresh: false }); F.stale('library'); await refresh(['library']); render('library'); },
    'project-package': async el => { const p = P(); if (!p && !ds(el).id) return; if (ds(el).id && (!p || p.id !== ds(el).id)) await act('project.open', { id: ds(el).id }); const path = await F.saveAs({ name: (D.ui.pkgName || (P().name + '.flux')), kind: 'flux' }); if (path == null) return; const r = await act('project.export_package', path ? { path } : {}, { refresh: false }); if (r) { D.ui.exported = { path: r.path || path, bytes: r.bytes, encoder: '.flux package', verified: true }; D.ui.exportBase = 'library'; render('exported'); } },
    'lib-folder': el => { const q = D.ui.libq = D.ui.libq || {}; q.folder = q.folder === ds(el).v ? undefined : ds(el).v; F.stale('library'); F.need(['library']).then(rr); },
    'add-folder': async () => { const n = prompt('Folder name'); if (!n || !n.trim()) return; const s = await act('settings.set', { values: { folders: ((D.settings && D.settings.folders) || []).concat([n.trim()]) } }, { refresh: false }); if (s) D.settings = s; F.stale('library'); await F.need(['library']); rr(); },
    tpl: el => { D.ui.tpl = ds(el).v; rr(); },
    'project-new': async () => { const r = await act('project.new', { name: D.ui.newName || 'Untitled project', template: D.ui.tpl || 'blank', sample_rate: +(D.ui.rate || 48000), folder: D.ui.folder || undefined }); if (r) { resetProjectUi(); F.stale('library'); render('edit'); } },
    /* recording */
    'rec-start': async () => { if (!P()) await act('project.new', { name: 'Recording', template: 'blank' }); const p = { monitor: !!D.ui.monitor }; if (D.ui.recTo === 'sel' && D.sel.track) p.track = D.sel.track; if (D.settings && D.settings.input) p.input = D.settings.input; await act('record.start', p, { refresh: false }); poll(true); },
    'rec-pause': async () => { await act('record.pause', { paused: !(D.st.recording && D.st.recording.paused) }, { refresh: false }); poll(true); },
    'rec-stop': async () => { await act('record.stop', {}); render('edit'); },
    'rec-discard': async () => { await act('record.stop', { discard: true }); rr(); },
    /* transport */
    'tp-start': () => act('transport.seek', { pos: 0 }, { refresh: false }).then(() => poll(true)),
    'tp-stop': () => act('transport.stop', {}, { refresh: false }).then(() => poll(true)),
    'tp-loop': async () => { D.ui.loop = !D.ui.loop; const s = D.ui.specSel; await act('transport.loop', Object.assign({ on: D.ui.loop }, s ? { start: s.t0, end: s.t1 } : {}), { refresh: false }); rr(); },
    'seek-play': async el => { await act('transport.seek', { pos: +ds(el).pos }, { refresh: false }); await act('transport.play', {}, { refresh: false }); poll(true); },
    undo: () => act('history.undo', {}),
    redo: () => act('history.redo', {}),
    /* timeline */
    'edit-tool': (el, i) => { D.ui.tool = ['select', 'range', 'slip', 'fade'][i]; },
    'edit-split': () => act('clip.split', D.sel.clip != null ? { clip: D.sel.clip, at: pos() } : { at: pos() }),
    'edit-delete': () => D.sel.clip != null && act('clip.delete', { clip: D.sel.clip }).then(() => { D.sel.clip = null; rr(); }),
    'edit-copy': () => D.sel.clip != null && act('edit.copy', { clip: D.sel.clip }, { refresh: false }),
    'edit-cut': () => D.sel.clip != null && act('edit.cut', { clip: D.sel.clip }).then(() => { D.sel.clip = null; rr(); }),
    zoom: () => { D.ui.zoom = (D.ui.zoom || 1) >= 16 ? 1 : (D.ui.zoom || 1) * 2; rr(); },
    'ui-toggle': el => { const k = ds(el).k, def = ds(el).def === '1'; D.ui[k] = !(D.ui[k] == null ? def : D.ui[k]); rr(); },
    'ui-set': el => { const k = ds(el).k, v = ds(el).v; D.ui[k] = D.ui[k] === v && k === 'guide' ? null : (/^-?\d+(\.\d+)?$/.test(v) && k !== 'action' ? +v : v); rr(); },
    'track-flag': el => { const t = F.track(+ds(el).track); if (t) act('track.set', { track: t.id, [ds(el).k]: !t[ds(el).k] }); },
    'track-add': () => act('track.add', {}),
    'track-remove': el => act('track.remove', { track: +ds(el).track }),
    'marker-add': () => act('marker.add', { pos: pos() }),
    'clip-curve': (el, i) => act('clip.set', { clip: +ds(el).clip, curve: ['equal', 's', 'log', 'lin'][i] }),
    'clip-reverse': el => { const c = F.clip(+ds(el).clip); if (c) act('clip.set', { clip: c.id, reverse: !c.reverse }); },
    'clip-normalize': el => act('clip.normalize', { clip: +ds(el).clip }),
    'clip-dup': el => act('clip.duplicate', { clip: +ds(el).clip }),
    'clip-bounce': async el => { const r = await act('clip.bounce', { clip: +ds(el).clip }, { refresh: false }); if (r && r.job) await follow(r.job, null); await refresh(); },
    'set-target': el => { D.sel.target = ds(el).target; D.sel.fx = null; },
    'auto-mode': (el, i) => { D.ui.autoMode = ['read', 'write', 'touch', 'latch'][i]; },
    'lane-shape': async (el, i) => { const ln = curLane(); if (ln) await act('lane.set', { track: ln.t.id, target: ln.target, points: ln.lane ? ln.lane.points : [], shape: ['linear', 'smooth', 'hold'][i] }); },
    'lane-clear': async () => { const ln = curLane(); if (ln && ln.lane) await act('lane.remove', { track: ln.t.id, lane: ln.lane.id }); },
    'lane-remove': el => act('lane.remove', { track: +ds(el).track, lane: +ds(el).lane }),
    'history-goto': el => act('history.goto', { index: +ds(el).i }),
    'snapshot-save': async () => { const n = prompt('Snapshot name', 'Snapshot ' + new Date().toLocaleTimeString()); if (n == null) return; await act('snapshot.save', { name: n }, { refresh: false }); F.stale('snapshots'); await F.need(['snapshots']); rr(); },
    'snapshot-restore': el => act('snapshot.restore', { id: ds(el).id }),
    /* spectral */
    'spec-view': (el, i) => { D.ui.specView = ['Spectrogram', 'Waveform', 'Both'][i]; rr(); },
    'spec-fill': (el, i) => { D.ui.fill = ['both', 'before', 'after'][i]; },
    'spec-clear': () => { D.ui.specSel = null; rr(); },
    'spec-repair': () => specRepair(D.ui.fill || 'both'),
    'spec-attenuate': () => specRepair('attenuate'),
    'spec-listen': async () => { const s = D.ui.specSel; if (!s) return; await act('transport.loop', { on: true, start: s.t0, end: s.t1 }, { refresh: false }); D.ui.loop = true; await act('transport.play', { from: s.t0 }, { refresh: false }); poll(true); },
    /* effects */
    'fx-open-kind': el => { const k = ds(el).kind; if (!k) return; D.ui.fxkind = k; const e = F.fxOf(k); D.sel.fx = e ? e.id : null; if (['compressor', 'limiter', 'multiband', 'gate', 'deesser', 'transient', 'leveler'].includes(k)) D.ui.dyn = k; if (['noise', 'hum', 'declick', 'declip', 'dewind', 'stereo'].includes(k)) D.ui.clean = k; if (k === 'delay') D.ui.rvTab = 3; else if (k === 'reverb') D.ui.rvTab = 2; if (['vinyl', 'cassette', 'radio', 'phone', 'megaphone', 'walkie', 'underwater', 'distortion', 'bitcrusher', 'chorus', 'flanger', 'phaser', 'rotary'].includes(k)) D.ui.texture = k; },
    'open-fx': el => { const e = F.findFx(+ds(el).fx); if (!e) return; D.sel.fx = e.id; D.sel.target = e.target; D.ui.fxkind = e.kind; ACT['fx-open-kind']({ dataset: { kind: e.kind } }); D.sel.fx = e.id; if (!el.dataset.go) render(Flux.SCREEN[e.kind] || 'fxp'); },
    'fx-add': async el => { try { await addFx(ds(el).kind, ds(el).target); } catch (e) { fail(e); } },
    'fx-remove': async el => { await act('fx.remove', { fx: +ds(el).fx }); if (D.sel.fx === +ds(el).fx) D.sel.fx = null; },
    'fx-move': el => act('fx.move', { fx: +ds(el).fx, index: Math.max(0, +ds(el).i) }),
    'fx-enable': el => { const e = F.findFx(+ds(el).fx); if (e) act('fx.set', { fx: e.id, enabled: !e.enabled }); },
    'fx-toggle': el => { const d = ds(el), e = d.fx ? F.findFx(+d.fx) : null, v = e ? F.pval(e, d.k) : (Flux.pspec(d.kind, d.k) || {}).default; setParam(el, d.k, v >= .5 ? 0 : 1); },
    'fx-param': el => setParam(el, ds(el).k, +ds(el).v),
    'fx-enum': (el, i) => setParam(el, ds(el).k, i),
    'fx-bool': (el, i) => setParam(el, ds(el).k, i),
    'fx-slot': (el, i) => act('fx.slot', { fx: +ds(el).fx, slot: i }),
    'fx-seed': el => setParam(el, 'seed', Math.floor(Math.random() * 99999)),
    'target-kind': (el, i) => { const k = ['clip', 'track', 'master'][i]; if (k === 'master') D.sel.target = 'master'; else if (k === 'clip') { const c = D.sel.clip != null ? D.sel.clip : (F.clips()[0] || {}).id; if (c == null) return rr(); D.sel.target = 'clip:' + c; } else { const t = D.sel.track || (D.sel.clip != null ? F.clip(D.sel.clip).track : (F.tracks()[0] || {}).id); if (t == null) return rr(); D.sel.target = 'track:' + t; } D.sel.fx = null; rr(); },
    'dyn-tab': el => { D.ui.dyn = ds(el).v; const e = F.fxOf(ds(el).v); D.sel.fx = e ? e.id : null; rr(); },
    'clean-tab': el => { D.ui.clean = ds(el).v; const e = F.fxOf(ds(el).v); D.sel.fx = e ? e.id : null; rr(); },
    'rv-tab': el => { D.ui.rvTab = +ds(el).v; const k = ['convolver', 'reverb', 'reverb', 'delay'][D.ui.rvTab], e = F.fxOf(k); D.sel.fx = e ? e.id : null; rr(); },
    'texture-pick': el => { D.ui.texture = ds(el).kind; const e = F.fxOf(ds(el).kind); D.sel.fx = e ? e.id : null; rr(); },
    'eq-show': (el, i) => { if (i) { D.ui.fxkind = 'geq'; const e = F.fxOf('geq'); D.sel.fx = e ? e.id : null; render('fxp'); } },
    'preset-apply': async el => { const d = ds(el), e = F.fxOf(d.kind); const r = await act('preset.apply', e && d.kind && d.kind !== 'chain' ? { id: d.id, fx: e.id } : { id: d.id, target: tgt() }); if (d.kind === 'eq') D.ui.eqPreset = d.id; if (d.kind === 'voice') D.ui.voicePreset = d.id; if (r && cur === 'presets') render('rack'); else rr(); },
    'preset-save': async el => { const n = prompt('Preset name'); if (!n) return; await act('preset.save', { name: n, fx: +ds(el).fx }, { refresh: false }); F.stale('presets'); await F.need(['presets']); rr(); },
    'preset-save-chain': async () => { const n = prompt('Preset name'); if (!n) return; await act('preset.save', { name: n, target: tgt() }, { refresh: false }); F.stale('presets'); await F.need(['presets']); rr(); },
    'preset-kind': (el, i) => { D.ui.pkind = ['fx', 'chain', 'stage', 'export'][i]; rr(); },
    'preset-import': async () => { const fs = await F.pickFiles({ multiple: false, kind: 'preset' }); if (fs && fs[0]) fail({ code: 'unsupported', message: 'Preset files are applied through Presets in this build. Open the file in Flux by dropping it on the window.' }); },
    'learn-noise': async el => { let id = ds(el).fx ? +ds(el).fx : null; if (!id) id = await addFx('noise'); const s = P().selection || {}; await act('fx.learn_noise', s.end > s.start ? { fx: id, start: s.start, end: s.end } : { fx: id }); },
    'load-ir': async el => { const fs = await F.pickFiles({ multiple: false, kind: 'audio' }); if (!fs || !fs[0]) return; let id = ds(el).fx ? +ds(el).fx : null; if (!id) id = await addFx('convolver'); D.ui.rvTab = 0; await act('fx.load_ir', { fx: id, path: fs[0] }); },
    'match-eq': async () => { const fs = await F.pickFiles({ multiple: false, kind: 'audio' }); if (fs && fs[0]) await act('tool.match_eq', Object.assign({ path: fs[0] }, D.sel.target && D.sel.target.startsWith('track:') ? { track: +D.sel.target.split(':')[1] } : {})); },
    'gate-step': el => { const e = el.dataset.fx ? F.findFx(+el.dataset.fx) : null, cur0 = e ? F.pval(e, 'steps') : (Flux.pspec('trancegate', 'steps') || {}).default || 0; setParam(el, 'steps', (Math.round(cur0) ^ (1 << +ds(el).i))); },
    'tapestop-here': async () => { const e = F.fxOf('tapestop'), at = pos() / F.sr(); if (e) await act('fx.set', { fx: e.id, params: { at } }); else await addFx('tapestop', null, { at }); },
    'reverse-apply': async () => { const c = D.sel.clip; if (c == null) return; await act('clip.set', { clip: c, reverse: !F.clip(c).reverse }); if (D.ui.swell !== false) await act('tool.reverse_swell', { clip: c, beats: D.ui.swellBeats || 2 }); },
    'style-toggle': async el => { const id = ds(el).v, m = D.ui.styleAmt = D.ui.styleAmt || {}; const amt = m[id] != null ? 0 : .7; await act('style.apply', { style: id, amount: amt, target: tgt() === 'master' || tgt().startsWith('track:') ? tgt() : 'master' }); if (amt) m[id] = amt; else delete m[id]; rr(); },
    'stage-tool': (el, i) => { D.ui.stageTool = i ? 'draw' : 'drag'; const c = stage.querySelector('canvas[data-v="stage"]'); if (c) c.dataset.tool = D.ui.stageTool; },
    'stage-sync': async () => { const p = P(); if (!p || !p.tempo) return; D.ui.stageSync = !D.ui.stageSync; const e = F.fxOf('spatial'); if (D.ui.stageSync) { const rate = p.tempo / 60 / 8; if (e) await act('fx.set', { fx: e.id, params: { rate } }); else await addFx('spatial', null, { rate }); } rr(); },
    'stage-bake': async el => { const e = F.findFx(+ds(el).fx), p = P(); if (!e || !p) return; const tr = e.target.startsWith('track:') ? +e.target.split(':')[1] : (F.tracks()[0] || {}).id; if (tr == null) return; const n = Math.min(240, Math.max(8, Math.ceil(p.seconds * 4))), pts = []; for (let i = 0; i <= n; i++) { const t = p.seconds * i / n; const m = await call('fx.motion', { fx: e.id, t }); pts.push([Math.round(t * F.sr()), m.azimuth]); } await act('lane.set', { track: tr, target: `fx:${e.id}:azimuth`, points: pts, shape: 'smooth' }); await act('fx.set', { fx: e.id, params: { mode: 4 } }); },
    'room-use': async () => { const r = Object.assign({ w: 12, d: 18, h: 6, wall: 0, floor: 0, ceil: 0 }, D.ui.room || {}), c = Flux.roomCalc(r), e = F.fxOf('reverb'); const params = { decay: Math.max(.1, Math.min(c.rt, 12)), predelay: Math.round(c.first), size: Math.min(100, Math.cbrt(r.w * r.d * r.h) * 6) }; if (e) await act('fx.set', { fx: e.id, params }); else await addFx('reverb', null, params); render('stage'); },
    'voice-monitor': async () => { D.ui.monitor = !D.ui.monitor; rr(); fail({ code: 'unsupported', message: 'Live monitoring through effects starts with recording. Press Record and turn on "Hear yourself".' }, { title: 'Talk through it' }); },
    'tap-tempo': async () => { const now = performance.now(); taps = taps.filter(t => now - t < 2500).concat([now]); if (taps.length >= 3) { const iv = (taps[taps.length - 1] - taps[0]) / (taps.length - 1), bpm = Math.round(60000 / iv * 10) / 10; if (bpm > 30 && bpm < 300) await act('project.set', { tempo: bpm }); } },
    /* smart tools */
    'magic-mode': (el, i) => { D.ui.magicMode = i; },
    'magic-toggle': el => { D.ui.magicOff = togg(D.ui.magicOff || [], ds(el).v); rr(); },
    'magic-apply': async () => { const m = D.ui.magic; if (!m) return; const ids = m.fixes.map(f => f.id).filter(id => !(D.ui.magicOff || []).includes(id)); const r = await act('tool.magic.apply', { ids }); if (r) { D.ui.magic = null; D.sel.target = 'master'; render('rack'); } },
    'stem-mode': (el, i) => { if (i === 2) { rr(); return; } D.ui.stemMode = i === 0 ? '2' : '4'; },
    'stems-start': async () => { const r = await act('tool.stems', { mode: D.ui.stemMode || '4' }, { refresh: false }); if (!r) return; await follow(r.job, 'stems-run', async () => { await refresh(); if (D.ui.stemMute === false) { const orig = F.tracks().find(t => !/^(Vocals|Drums|Bass|Other|Music|Instrumental)\b/.test(t.name) && t.mute); if (orig) await act('track.set', { track: orig.id, mute: false }); } render('stems'); }); },
    'stem-mix': async (el, i) => { D.ui.stemMix = i; const drop = [null, ['Vocals'], ['Drums', 'Bass', 'Other', 'Music', 'Instrumental'], ['Drums'], ['Bass']][i]; if (!drop) return; for (const t of F.tracks().filter(t => /^(Vocals|Drums|Bass|Other|Music|Instrumental)\b/.test(t.name))) { const m = drop.includes(t.name.split(' ')[0]); if (!!t.mute !== m) await call('track.set', { track: t.id, mute: m }); } await refresh(); },
    'clean-adv': (el, i) => { D.ui.cleanAdv = i; },
    'cleanup-apply': async () => { const p = { strength: (D.ui.cleanStrength == null ? 60 : D.ui.cleanStrength) / 100 }; if (D.sel.target && D.sel.target.startsWith('track:')) p.track = +D.sel.target.split(':')[1]; const r = await act('tool.cleanup', p); if (r) { D.ui.cleanReport = r.report; rr(); } },
    'tempo-mul': async el => { const p = P(); if (p && p.tempo) await act('project.set', { tempo: Math.round(p.tempo * +ds(el).v * 10) / 10 }); },
    'tempo-apply': async () => { await act('analysis.tempo_key', { apply: true }, { refresh: false }); await refresh(['tempokey']); },
    'tempo-detect': async () => { F.stale('tempokey'); await F.need(['tempokey'], true); rr(); },
    'master-style': (el, i) => { D.ui.masterStyle = i; D.ui.masterPlan = null; planMaster(); },
    'master-apply': async el => { const r = await act('tool.master.apply', { style: ds(el).style, target: D.ui.masterTarget || (P() && P().target_lufs) || -14, amount: D.ui.masterAmt == null ? .5 : D.ui.masterAmt }); if (r) { D.ui.mastered = true; D.ui.masterPlan = null; await planMaster(); F.stale('loudness'); await F.need(['loudness'], true); rr(); } },
    'tts-speak': async () => { const text = (D.ui.ttsText || '').trim(); if (!text) return; const vs = (D.voices && D.voices.voices) || []; if (!P()) await act('project.new', { name: 'Speech', template: 'blank' }); const r = await act('tts.speak', { text, voice: D.ui.ttsVoice || (vs[0] && vs[0].id), rate: D.ui.ttsRate || 0, at: pos() }, { refresh: false }); if (r && r.job) await follow(r.job, null, async () => { await refresh(); render('edit'); }); },
    'job-cancel': async el => { const id = ds(el).id || (D.ui.job && D.ui.job.id); if (id) await act('job.cancel', { id }, { refresh: false }); F.stale('jobs'); if (ds(el).then) render(ds(el).then); else { await F.need(['jobs'], true); rr(); } },
    'jobs-clear': async () => { await act('job.clear', {}, { refresh: false }); F.stale('jobs'); await F.need(['jobs'], true); rr(); },
    /* export */
    'exp-dest': el => { const d = ds(el).v; D.ui.exp = Object.assign({}, Flux.xo(), Flux.DEST[d], { dest: d }); rr(); },
    'exp-set': el => { D.ui.exp = Object.assign(Flux.xo(), { [ds(el).k]: ds(el).v, dest: 'Custom' }); rr(); },
    'exp-toggle': el => { const o = Flux.xo(); o[ds(el).k] = !o[ds(el).k]; D.ui.exp = o; rr(); },
    'exp-range': (el, i) => { D.ui.exp = Object.assign(Flux.xo(), { range: ['all', 'selection', 'loop'][i] }); },
    'exp-norm': (el, i) => { D.ui.exp = Object.assign(Flux.xo(), { normalize: ['none', 'peak', 'lufs'][i] }); rr(); },
    'export-start': async () => { const o = Flux.xo(); let r; try { r = await call('export.start', { format: o.format, dir: exportDir(), options: exportOpts(o), range: o.range, tail: o.tail }); } catch (e) { fail(e, { title: 'The export didn\'t start.' }); return; } D.ui.exportBase = 'edit'; await follow(r.job, 'exporting', async res => { D.ui.exported = res; render('exported'); }); },
    'export-stems': async () => { const o = Flux.xo(); let r; try { r = await call('export.stems', { format: o.format, dir: exportDir(), options: exportOpts(o) }); } catch (e) { fail(e); return; } await follow(r.job, 'exporting', async res => { const f = (res.files || res.outputs || [])[0]; D.ui.exported = Object.assign({}, f || {}, { path: res.dir || (f && f.path) || '', encoder: (res.files || res.outputs || []).length + ' files', verified: true }); render('exported'); }); },
    reveal: el => ds(el).path && F.reveal(ds(el).path),
    'open-url': el => F.openUrl(ds(el).url),
    'update-check': async () => {
      if (!F.updater) return F.openUrl('https://github.com/jeelnadaa/flux/releases/latest');
      D.update = Object.assign({}, D.update, { busy: true, error: null }); rr();
      try { D.update = await F.updater.check(); } catch (e) { D.update = Object.assign({}, D.update, { busy: false }); return fail(e); }
      if (D.update && D.update.ready) { D.ui.updateShown = false; showUpdate(); } else rr();
    },
    'update-install': async () => { try { await F.updater.install(); } catch (e) { fail(e); } },
    /* workflow */
    'conv-add': async () => { const fs = await F.pickFiles({ multiple: true, kind: 'audio' }); if (fs && fs.length) { const c = D.ui.conv = Object.assign({ files: [], format: 'mp3', bitrate: 245, sample_rate: 0 }, D.ui.conv || {}); c.files = c.files.concat(fs); D.ui.convJob = null; rr(); } },
    'conv-set': el => { D.ui.conv = Object.assign({ files: [], format: 'mp3', bitrate: 245, sample_rate: 0 }, D.ui.conv || {}, { [ds(el).k]: ds(el).v }); rr(); },
    'files-remove': el => { const c = D.ui.conv; if (c) { c.files.splice(+ds(el).i, 1); rr(); } },
    'conv-start': async () => { const c = D.ui.conv; if (!c || !c.files.length) return; const opts = ['wav', 'flac'].includes(c.format) ? { bits: c.bits || 24 } : { bitrate: c.bitrate === 245 ? 256 : c.bitrate, vbr: c.bitrate === 245 }; let r; try { r = await call('convert.start', Object.assign({ files: c.files, format: c.format, options: opts, dir: exportDir() }, c.sample_rate ? { sample_rate: c.sample_rate } : {})); } catch (e) { fail(e); return; } D.ui.convJob = { state: 'running', progress: 0 }; rr(); await follow(r.job, null, () => { }).then(j => { D.ui.convJob = j; rr(); }); },
    'batch-add': async () => { const fs = await F.pickFiles({ multiple: true, kind: 'audio' }); if (fs && fs.length) { const b = D.ui.batch = Object.assign({ files: [], steps: [], format: 'mp3' }, D.ui.batch || {}); b.files = b.files.concat(fs); D.ui.batchJob = null; rr(); } },
    'batch-remove': el => { D.ui.batch.steps.splice(+ds(el).i, 1); rr(); },
    'batch-remove-file': el => { D.ui.batch.files.splice(+ds(el).i, 1); rr(); },
    'batch-start': async () => { const b = D.ui.batch; if (!b || !b.files.length) return; const st = b.steps.find(s => s.style), pre = b.steps.find(s => s.preset), norm = b.steps.find(s => s.norm != null); const p = { files: b.files, format: b.format, options: norm ? { normalize: 'lufs', target: norm.norm, ceiling: -1 } : {} }; if (st) { p.style = st.style; p.amount = .7; } if (pre) p.preset = pre.preset; if (b.dest === 'export' && exportDir()) p.dir = exportDir(); let r; try { r = await call('batch.start', p); } catch (e) { fail(e); return; } D.ui.batchJob = { state: 'running', progress: 0 }; rr(); await follow(r.job, null).then(j => { D.ui.batchJob = j; rr(); }); },
    'ring-phone': (el, i) => { D.ui.ring = Object.assign({ start: 0, len: 30, fi: .3, fo: 1 }, D.ui.ring || {}, { phone: i ? 'android' : 'iphone' }); rr(); },
    'ring-find': async () => { const p = P(); if (!p) return; try { const r = await call('analysis.loudness', {}); const c = r.loudness.curve || []; const len = Math.min((D.ui.ring && D.ui.ring.len) || 30, p.seconds); let best = 0, bv = -1e9; const per = p.seconds / Math.max(1, c.length); const w = Math.max(1, Math.round(len / per)); for (let i = 0; i + w <= c.length; i++) { let s = 0; for (let k = i; k < i + w; k++) s += c[k]; if (s > bv) { bv = s; best = i; } } D.ui.ring = Object.assign({ len: 30, fi: .3, fo: 1, phone: 'iphone' }, D.ui.ring || {}, { start: Math.min(best * per, Math.max(0, p.seconds - len)) }); rr(); } catch (e) { fail(e); } },
    'ring-export': async () => { const r0 = Object.assign({ start: 0, len: 30, fi: .3, fo: 1, phone: 'iphone' }, D.ui.ring || {}), p = P(); const len = Math.min(r0.len, r0.phone === 'iphone' ? 30 : 40, p.seconds); const r = await act('ringtone.make', { start_s: r0.start, len_s: len, fade_in_s: r0.fi, fade_out_s: r0.fo, format: r0.phone === 'iphone' ? 'm4r' : 'mp3', dir: exportDir() }, { refresh: false }); if (!r) return; const done = res => { D.ui.exported = res; D.ui.exportBase = 'ringtone'; render('exported'); }; if (r.job) await follow(r.job, 'exporting', async res => done(res)); else done(r); },
    'action-run': async el => { await act('action.run', { id: ds(el).id }); },
    'action-delete': async el => { await act('action.delete', { id: ds(el).id }, { refresh: false }); F.stale('actions'); await F.need(['actions'], true); rr(); },
    'action-record': async () => { await act('action.record', { on: true }, { refresh: false }); F.stale('actions'); await F.need(['actions'], true); rr(); },
    'action-stop': async () => { const n = prompt('Name this action', 'My action'); await act('action.record', { on: false, name: n || 'My action' }, { refresh: false }); F.stale('actions'); await F.need(['actions'], true); rr(); },
    /* settings */
    'key-edit': el => { D.ui.keyEdit = D.ui.keyEdit === ds(el).v ? null : ds(el).v; rr(); },
    'copy-version': () => { const i = D.info || {}; copyText(`Flux ${i.version} ${i.platform} ${i.arch}`); },
    copy: el => { const n = stage.querySelectorAll(ds(el).sel.split(':')[0]); const t = ds(el).sel.includes('last') ? n[n.length - 1] : n[0]; if (t) copyText(t.textContent); },
    'empty-trash': async () => { if (!confirm('Permanently delete everything in the trash? This can\'t be undone.')) return; await act('storage.empty_trash', {}, { refresh: false }); F.stale('usage', 'library'); await F.need(['usage'], true); rr(); },
    'diag-save': async () => { const r = D.diag || await call('system.diagnostics'); const path = await F.saveAs({ name: 'flux-diagnostics.json', kind: 'json', text: JSON.stringify(r, null, 2) }); if (path) render('trouble'); },
    'recovery-open': async el => { await act('recovery.dismiss', { id: ds(el).id }, { refresh: false }); await act('project.open', { id: ds(el).id }); render('edit'); },
    'recovery-dismiss': async () => { for (const r of D.recovery || []) await call('recovery.dismiss', { id: r.id }).catch(() => { }); D.recovery = []; render('home'); },
    relink: async el => { const fs = await F.pickFiles({ multiple: false, kind: 'audio' }); if (fs && fs[0]) await act('media.relink', { id: ds(el).id, path: fs[0] }); },
    'error-close': () => { const b = D.ui.errorBase; D.ui.error = null; render(b && b !== 'error' ? b : (P() ? 'edit' : 'home')); },
    'error-retry': () => { const e = D.ui.error; D.ui.error = null; if (e && e.retry) e.retry(); },
    'macro-map': async () => { const e = D.sel.fx && F.findFx(D.sel.fx); if (!e) return; const s = F.spec(e.kind), p = s && s.params.find(q => !/bool|enum|int/.test(q.unit) && q.key !== 'seed'); if (!p) return; const v = F.pval(e, p.key); (D.ui.macroMap = D.ui.macroMap || []).push({ fx: e.id, key: p.key, lo: v, hi: Math.min(p.max, v + (p.max - p.min) * .25) }); rr(); },
    'macro-unmap': el => { D.ui.macroMap.splice(+ds(el).i, 1); rr(); },
    'set-mode-seg': () => { },
    'sil-scan': () => silScan(),
    'sil-keep': el => { D.ui.silKeep = togg(D.ui.silKeep || [], +ds(el).i); rr(); },
    'sil-apply': async () => { const o = D.ui.sil || { threshold_db: -42, longer_than_s: .6, shorten_to_s: .3, padding_s: .15 }; const r = await act('tool.silence.apply', Object.assign({ keep: D.ui.silKeep || [] }, o)); if (r) { D.ui.silScan = null; silScan(); } },
  };
  ACT['set-mode'].seg = true;
  const SEG = {
    'set-mode': async (el, i) => { const s = await act('settings.set', { values: { mode: i ? 'pro' : 'simple' } }, { refresh: false }); if (s) D.settings = s; },
  };
  function curLane() { const p = P(); if (!p || !p.tracks.length) return null; const cur0 = D.ui.autoTarget || `${p.tracks[0].id}:volume`, [tid, ...rest] = cur0.split(':'), t = F.track(+tid) || p.tracks[0], target = rest.join(':'); return { t, target, lane: (t.lanes || []).find(l => l.target === target) }; }
  async function specRepair(fill) { const s = D.ui.specSel; if (!s) return; await act('tool.spectral.repair', { start: Math.round(s.t0), end: Math.round(s.t1), low_hz: s.f0, high_hz: s.f1, amount_db: D.ui.repairDb || 30, fill }); D.ui.specSel = null; rr(); }
  function resetProjectUi() { ['magic', 'masterPlan', 'mastered', 'silScan', 'specSel', 'stemMix', 'cleanReport', 'styleAmt', 'macroMap', 'job', 'zoom'].forEach(k => delete D.ui[k]); lastRev = -1; }
  function copyText(t) { (navigator.clipboard ? navigator.clipboard.writeText(t) : Promise.reject()).catch(() => { const a = document.createElement('textarea'); a.value = t; document.body.appendChild(a); a.select(); document.execCommand('copy'); a.remove(); }); }

  /* ---------- live parameter edits (EQ drag, stage) ---------- */
  let liveQ = null, liveT = 0, gesture = 0;
  const newGesture = () => 'g' + (++gesture) + '-' + Date.now().toString(36);
  let liveG = null;
  F.live = (fx, params, end) => {
    if (end) { flushLive(true); return; }
    if (!liveG) liveG = newGesture();
    liveQ = { fx, params: Object.assign(liveQ && liveQ.fx === fx ? liveQ.params : {}, params) };
    const e = F.findFx(fx); if (e) Object.assign(e.params, params);
    if (performance.now() - liveT > 40) flushLive();
  };
  function flushLive(end) { if (liveQ) { const q = liveQ; liveQ = null; liveT = performance.now(); call('fx.set', { fx: q.fx, params: q.params, gesture: liveG }).catch(e => fail(e)); } if (end) { liveG = null; refresh(); } }
  F.stagePlace = async (trackId, az, dist) => { const t = F.track(trackId); if (!t) return; const e = (t.fx || []).find(x => x.kind === 'spatial'); const params = { mode: 4, azimuth: Math.round(az), distance: Math.max(.2, Math.min(10, dist)) }; if (e) await act('fx.set', { fx: e.id, params }); else { try { await addFx('spatial', 'track:' + trackId, params); } catch (er) { fail(er); } } };
  F.stagePath = async pts => { let e = F.fxOf('spatial'); if (!e) { try { await addFx('spatial'); } catch (er) { fail(er); return; } e = F.fxOf('spatial'); } const xy = pts.map(([az, d]) => { const r = Math.max(0, Math.min(1, d / 10)), a = az * Math.PI / 180; return [+(Math.sin(a) * r).toFixed(4), +(Math.cos(a) * r).toFixed(4)]; }); await act('fx.set_path', { fx: e.id, points: xy }); await act('fx.set', { fx: e.id, params: { mode: 5 } }); };

  /* ---------- slider bindings ---------- */
  const UNIT = { dB: v => F.db(v) + ' dB', ms: v => Math.round(v * 1000) + ' ms', s: v => v.toFixed(1) + ' s', '%': v => Math.round(v) + '%', int: v => String(Math.round(v)), cm: v => v.toFixed(1) + ' cm', m: v => Math.round(v) + ' m', smp: v => Math.round(v / F.sr() * 1000) + ' ms', dBneg: v => '−' + Math.round(v) + ' dB', frac: v => ['Light', 'Balanced', 'Strong'][Math.min(2, Math.floor(v * 3))], LUFS: v => F.num(v, 1) + ' LUFS', dBTP: v => F.num(v, 1) + ' dBTP' };
  const lin = (parts, pct) => { const lo = +parts[0], hi = +parts[1], u = parts[2]; let v = lo + (hi - lo) * pct / 100; if (u === 'int' || u === 'smp' || u === 'm') v = Math.round(v); return [v, (UNIT[u] || (x => String(x)))(v)]; };
  const OBJ = { sil: 'sil', room: 'room', ring: 'ring', exp: 'exp' };
  const OBJDEF = { sil: () => ({ threshold_db: -42, longer_than_s: .6, shorten_to_s: .3, padding_s: .15 }), room: () => ({ w: 12, d: 18, h: 6, wall: 0, floor: 0, ceil: 0 }), ring: () => ({ start: 0, len: 30, fi: .3, fo: 1, phone: 'iphone' }), exp: () => Flux.xo() };
  let bindQ = null, bindT = 0, bindG = null, created = null;
  async function bindApply(b, pct, end) {
    const parts = b.split(':'), ns = parts[0];
    if (ns === 'lv') return Flux.liveBind(parts.slice(1), pct, end);
    if (ns === 'cl') return Flux.cloneBind(parts.slice(1), pct, end);
    if (ns === 'fx' || ns === 'fxk') {
      let id = ns === 'fx' ? +parts[1] : created; const key = parts[2], kind = ns === 'fxk' ? parts[1] : (F.findFx(id) || {}).kind, p = Flux.pspec(kind, key); if (!p) return '';
      const v = F.unpct(p, pct);
      if (!id) { if (created === 0) return F.fmtp(p, v); created = 0; try { id = created = await addFx(kind, null, { [key]: v }); } catch (e) { created = null; fail(e); } return F.fmtp(p, v); }
      queue({ m: 'fx.set', p: { fx: id, params: { [key]: v } } }, end); const e = F.findFx(id); if (e) e.params[key] = v; return F.fmtp(p, v);
    }
    if (ns === 'fxmix' || ns === 'fxkmix') { let id = ns === 'fxmix' ? +parts[1] : created; const mix = pct / 100; if (!id) { if (created !== 0) { created = 0; try { created = await addFx(parts[1]); } catch (e) { created = null; } } return Math.round(pct) + '%'; } queue({ m: 'fx.set', p: { fx: id, mix } }, end); return Math.round(pct) + '%'; }
    if (ns === 'clip') { const [v, t] = lin(parts.slice(3), pct); queue({ m: 'clip.set', p: { clip: +parts[1], [parts[2]]: v } }, end); return t; }
    if (ns === 'track' && parts.length > 3) { const [v, t] = lin(parts.slice(3), pct); queue({ m: 'track.set', p: { track: +parts[1], gain_db: v } }, end); return t; }
    if (ns === 'track') { const g = Math.round(Flux.gfp(pct) * 10) / 10; queue({ m: 'track.set', p: { track: +parts[1], gain_db: g } }, end); return F.db(g) + ' dB'; }
    if (ns === 'master') { const g = Math.round(Flux.gfp(pct) * 10) / 10; queue({ m: 'project.set', p: { master_gain_db: g } }, end); return F.db(g) + ' dB'; }
    if (ns === 'set') { const [v, t] = lin(parts.slice(2), pct); if (end) call('settings.set', { values: { [parts[1]]: v } }).then(s => { D.settings = s; rr(); }, e => fail(e)); return t; }
    if (ns === 'ui') { const [v, t] = lin(parts.slice(2), pct); D.ui[parts[1]] = v; if (end) { if (parts[1] === 'masterAmt') { D.ui.masterPlan = null; planMaster(); } rr(); } return t; }
    if (OBJ[ns]) { const [v, t] = lin(parts.slice(2), pct), o = D.ui[OBJ[ns]] = Object.assign(OBJDEF[ns](), D.ui[OBJ[ns]] || {}); o[parts[1]] = v; if (ns === 'exp') o.dest = 'Custom'; if (end) { if (ns === 'sil' && D.ui.silScan) silScan(); rr(); } return t; }
    if (ns === 'style') { const id = parts[1], amt = Math.round(pct) / 100, m = D.ui.styleAmt = D.ui.styleAmt || {}; m[id] = amt; queue({ m: 'style.apply', p: { style: id, amount: amt, target: tgt().startsWith('clip:') ? 'master' : tgt() } }, end); return Math.round(pct) + '%'; }
    if (ns === 'macro') { D.ui.macro = pct; for (const mm of D.ui.macroMap || []) queue({ m: 'fx.set', p: { fx: mm.fx, params: { [mm.key]: mm.lo + (mm.hi - mm.lo) * pct / 100 } }, multi: true }, end); return Math.round(pct) + '%'; }
    return '';
  }
  /* Sends the latest value at most every 40 ms. All calls in one drag share a gesture id, so the drag is one undo step. */
  const sendQ = [];
  function queue(c, end) { if (c.multi) sendQ.push(c); else bindQ = c; if (end || performance.now() - bindT > 40) flush(end); }
  function flush(end) {
    bindT = performance.now(); const batch = sendQ.splice(0); if (bindQ) { batch.push(bindQ); bindQ = null; }
    const g = bindG; Promise.all(batch.map(c => call(c.m, Object.assign({ gesture: g }, c.p)))).then(() => { if (end) refresh(); }, e => fail(e));
    if (end) { bindG = null; created = null; }
  }

  /* ---------- tooltips: any element with data-tip, on hover (after a short wait) or keyboard focus ---------- */
  const tipEl = document.createElement('div'); tipEl.className = 'tipbox'; tipEl.id = 'tipbox'; tipEl.setAttribute('role', 'tooltip'); document.body.appendChild(tipEl);
  let tipT = 0, tipFor = null, tipDesc = null;
  function showTip(el) {
    const t = el.dataset.tip; if (!t || !document.contains(el)) return;
    tipFor = el; tipEl.textContent = t; tipEl.classList.add('on');
    const r = el.getBoundingClientRect(), w = tipEl.offsetWidth, h = tipEl.offsetHeight;
    let y = r.top - h - 8; if (y < 8) y = r.bottom + 8;
    tipEl.style.left = Math.max(8, Math.min(innerWidth - w - 8, r.left + r.width / 2 - w / 2)) + 'px'; tipEl.style.top = Math.min(innerHeight - h - 8, y) + 'px';
    const f = el.matches('a,button,input,select,textarea,[tabindex]') ? el : el.querySelector('a,button,input,select,textarea,[tabindex]');
    if (f && !f.hasAttribute('aria-describedby')) { f.setAttribute('aria-describedby', 'tipbox'); tipDesc = f; }
  }
  function hideTip() { clearTimeout(tipT); tipEl.classList.remove('on'); if (tipDesc) { tipDesc.removeAttribute('aria-describedby'); tipDesc = null; } tipFor = null; }
  Flux.hideTip = hideTip;
  document.addEventListener('pointerover', e => { const el = e.target.closest && e.target.closest('[data-tip]'); if (el === tipFor) return; hideTip(); if (el && !drag && e.pointerType !== 'touch') tipT = setTimeout(() => showTip(el), 420); });
  document.addEventListener('focusin', e => { const el = e.target.closest && e.target.closest('[data-tip]'); hideTip(); if (el) { let fv = true; try { fv = e.target.matches(':focus-visible'); } catch (er) { } if (fv) showTip(el); } });
  document.addEventListener('focusout', () => hideTip());
  document.addEventListener('pointerdown', () => hideTip(), true);
  document.addEventListener('keydown', e => { if (e.key === 'Escape' && tipFor) hideTip(); }, true);

  /* ---------- delegated interaction ---------- */
  document.addEventListener('click', async e => {
    if (e.button) return;
    const st = e.target.closest('[data-set-theme]'); if (st) { setTheme(st.dataset.setTheme, e); return; }
    const sf = e.target.closest('[data-set-font]'); if (sf) { setFont(sf.dataset.setFont, e); return; }
    const sb = e.target.closest('.seg button');
    if (sb && !sb.disabled && !sb.dataset.act) {
      const s = sb.parentElement, i = [...s.querySelectorAll('button')].indexOf(sb);
      s.querySelectorAll('button').forEach(b => { b.classList.toggle('on', b === sb); b.setAttribute('aria-pressed', b === sb); }); placeInd(s);
      if (s.hasAttribute('data-mode')) { SEG['set-mode'](s, i); if (i === 0) render('task'); else if (cur && cur.startsWith('task')) render('edit'); return; }
      const sa = s.dataset.segact; if (sa) { const fn = SEG[sa] || ACT[sa]; if (fn) await fn(s, i); }
      return;
    }
    const pl = e.target.closest('[data-play]');
    if (pl) { togglePlay(); return; }
    const a = e.target.closest('[data-act]');
    if (a && !a.disabled && a.getAttribute('aria-disabled') !== 'true') {
      if (a.tagName === 'A') e.preventDefault();
      const fn = ACT[a.dataset.act]; const go = a.dataset.go;
      if (fn) { const r = fn(a); if (go) { if (r && r.then) await r; render(go); } else if (r && r.then) await r; }
      else if (go) render(go);
      return;
    }
    const g = e.target.closest('[data-go]');
    if (g && !g.closest('[data-hold]')) { e.preventDefault(); if (g.dataset.go) render(g.dataset.go); return; }
    const cl = e.target.closest('[data-clip]');
    if (cl && !clipDrag) { const id = +cl.dataset.clip; D.sel.clip = id; D.sel.track = (F.clip(id) || {}).track; call('selection.set', { clip: id }).catch(() => { }); rr(); return; }
    if (e.target.closest('a[href^="#"]')) e.preventDefault();
  });
  async function togglePlay() { const pl = D.st.playing; D.st.playing = !pl; document.querySelectorAll('[data-play]').forEach(b => { b.innerHTML = ic(!pl ? 'pause' : 'play'); b.setAttribute('aria-label', !pl ? 'Pause' : 'Play'); }); await act(pl ? 'transport.pause' : 'transport.play', {}, { refresh: false }); poll(true); }

  /* inputs */
  const num = (el, v) => el.dataset.num ? +v : el.dataset.bool ? v === '1' : v;
  let inT = null;
  document.addEventListener('input', e => {
    const el = e.target;
    if (el.dataset.ui) { D.ui[el.dataset.ui] = num(el, el.value); if (el.dataset.rerender) { clearTimeout(inT); inT = setTimeout(rr, 120); } return; }
    if (el.matches('[data-ui="libsearch"]')) return;
  });
  document.addEventListener('change', async e => {
    const el = e.target, d = el.dataset;
    if (d.ui === 'libsearch' || el.matches('[data-ui="libsearch"]')) { D.ui.libq = Object.assign(D.ui.libq || {}, { search: el.value || undefined }); F.stale('library'); await F.need(['library']); rr(); return; }
    if (d.ui) { D.ui[d.ui] = num(el, el.value); if (d.rerender) { if (d.ui === 'masterTarget') { D.ui.masterPlan = null; planMaster(); } rr(); } return; }
    if (d.exp) { D.ui.exp = Object.assign(Flux.xo(), { [d.exp]: num(el, el.value), dest: 'Custom' }); rr(); return; }
    if (d.conv) { D.ui.conv = Object.assign({ files: [], format: 'mp3', bitrate: 245, sample_rate: 0 }, D.ui.conv || {}, { [d.conv]: num(el, el.value) }); rr(); return; }
    if (d.batch) { D.ui.batch = Object.assign({ files: [], steps: [], format: 'mp3' }, D.ui.batch || {}, { [d.batch]: el.value }); rr(); return; }
    if (el.hasAttribute('data-batchadd')) { const v = el.value; if (!v) return; const b = D.ui.batch = Object.assign({ files: [], steps: [], format: 'mp3' }, D.ui.batch || {}); const [k, id] = [v.slice(0, v.indexOf(':')), v.slice(v.indexOf(':') + 1)]; const label = el.options[el.selectedIndex].text; b.steps = b.steps.filter(s => !(k === 'style' ? s.style : k === 'preset' ? s.preset : s.norm != null)); b.steps.push(k === 'style' ? { style: id, label } : k === 'preset' ? { preset: id, label } : { norm: +id, label }); rr(); return; }
    if (d.room) { D.ui.room = Object.assign({ w: 12, d: 18, h: 6, wall: 0, floor: 0, ceil: 0 }, D.ui.room || {}, { [d.room]: +el.value }); rr(); return; }
    if (d.setsel) { const v = el.value === '' ? null : num(el, el.value); const s = await act('settings.set', { values: { [d.setsel]: v } }, { refresh: false }); if (s) D.settings = s; if (['output', 'input', 'buffer'].includes(d.setsel)) { F.stale('devices'); await F.need(['devices'], true); } rr(); return; }
    if (d.fxsel) { setParam(el, d.fxsel, +el.value); return; }
    if (d.fxinput) { const v = parseInt(el.value, 10); if (isFinite(v)) setParam(el, d.fxinput, Math.max(0, v)); return; }
    if (d.projset) { const v = parseFloat(el.value); if (isFinite(v) && v > 0) act('project.set', { [d.projset]: v }); return; }
  });
  document.addEventListener('toggle', e => { const el = e.target; if (el.dataset && el.dataset.details) D.ui[el.dataset.details] = el.open; }, true);

  /* sliders, faders, clip drags, ruler seeks, spectral selection, automation points */
  let drag = null, clipDrag = null, specDrag = null;
  document.addEventListener('pointerdown', e => {
    if (e.button) return;
    const t = e.target.closest('.sl .track'), f = e.target.closest('.fader .ftk');
    if (t || f) {
      const el = t ? t.closest('.sl') : f.closest('.fader');
      drag = { el, box: t || f, kind: t ? 'sl' : 'f', bind: el.dataset.bind }; dragging = !!drag.bind; bindG = newGesture(); el.classList.add('dr'); (t || f).setPointerCapture(e.pointerId); move(e); return;
    }
    const h = e.target.closest('[data-hold]');
    if (h) { h.classList.add('on'); h.style.transform = 'scale(.97)'; h.dataset.txt = h.textContent; h.textContent = 'Original'; call('transport.hold', { on: true }).catch(() => { }); return; }
    const cl = e.target.closest('[data-clip]');
    if (cl && cur === 'edit') { const lane = cl.closest('.lane'), c = F.clip(+cl.dataset.clip); if (c) { clipDrag = null; const st = { el: cl, c, x0: e.clientX, w: lane.getBoundingClientRect().width, moved: false, pid: e.pointerId }; cl.setPointerCapture(e.pointerId); cl._st = st; } return; }
    const sa = e.target.closest('[data-specarea]');
    if (sa && cur === 'spectral') { const r = sa.getBoundingClientRect(); specDrag = { r, x0: e.clientX - r.left, y0: e.clientY - r.top }; sa.setPointerCapture(e.pointerId); return; }
    const rl = e.target.closest('[data-ruler]') || (e.target.closest('.lane') && !e.target.closest('[data-clip]') ? e.target.closest('.lane') : null);
    if (rl && P()) {
      const ph = stage.querySelector('[data-ph]'), r = rl.getBoundingClientRect(), u = (e.clientX - r.left) / r.width, t0 = ph ? +ph.dataset.t0 : 0, vlen = ph ? +ph.dataset.vlen : P().length, at = Math.max(0, Math.round(t0 + u * vlen));
      if (cur === 'automation' && rl.classList.contains('lane')) { addAutoPoint(+rl.dataset.lane, at, 1 - (e.clientY - r.top) / r.height); return; }
      call('transport.seek', { pos: at }).then(() => poll(true), er => fail(er));
    }
  });
  function move(e) {
    if (drag) {
      const r = drag.box.getBoundingClientRect(); let p;
      if (drag.kind === 'sl') { p = Math.min(100, Math.max(0, (e.clientX - r.left) / r.width * 100)); drag.box.setAttribute('aria-valuenow', Math.round(p)); }
      else p = Math.min(100, Math.max(0, (r.bottom - e.clientY) / r.height * 100));
      drag.el.style.setProperty('--v', p + '%'); drag.p = p;
      if (drag.bind) label(drag, bindApply(drag.bind, p, false)); else if (drag.kind === 'sl') { const lbl = drag.el.querySelector('.sv'); if (lbl && /%/.test(lbl.textContent)) lbl.textContent = Math.round(p) + '%'; }
      return;
    }
    const cl = e.target.closest && document.querySelector('[data-clip]') ? [...document.querySelectorAll('[data-clip]')].find(x => x._st && x._st.pid === e.pointerId) : null;
    if (cl) { const st = cl._st, dx = e.clientX - st.x0; if (Math.abs(dx) > 3) { st.moved = true; clipDrag = st; cl.style.transform = `translateX(${dx}px)`; } return; }
    if (specDrag) { const r = specDrag.r, x = e.clientX - r.left, y = e.clientY - r.top, len = P().length, fy = v => 20 * Math.pow(1000, 1 - Math.max(0, Math.min(1, v / r.height))); const a = Math.max(0, Math.min(r.width, Math.min(specDrag.x0, x))), b = Math.max(0, Math.min(r.width, Math.max(specDrag.x0, x))); D.ui.specSel = { t0: a / r.width * len, t1: b / r.width * len, f0: fy(Math.max(specDrag.y0, y)), f1: fy(Math.min(specDrag.y0, y)) }; specDrag.moved = true; const box = stage.querySelector('[data-specsel]'); if (box) { box.style.left = a / r.width * 100 + '%'; box.style.width = (b - a) / r.width * 100 + '%'; box.style.top = Math.min(specDrag.y0, y) / r.height * 100 + '%'; box.style.height = Math.abs(y - specDrag.y0) / r.height * 100 + '%'; } }
  }
  function label(d, v) { const lbl = d.el.querySelector('.sv,[data-gtext]') || d.el.parentElement.querySelector('[data-gtext]'); const set = t => { if (t && lbl) lbl.textContent = t; if (t) d.box.setAttribute('aria-valuetext', t); }; if (v && v.then) v.then(set); else set(v); }
  document.addEventListener('pointermove', move);
  addEventListener('pointerup', e => {
    if (drag) { const d = drag; drag = null; d.el.classList.remove('dr'); dragging = false; if (d.bind && d.p != null) bindApply(d.bind, d.p, true); }
    const cl = [...document.querySelectorAll('[data-clip]')].find(x => x._st && x._st.pid === e.pointerId);
    if (cl) { const st = cl._st; cl._st = null; if (st.moved) { const ph = stage.querySelector('[data-ph]'), vlen = ph ? +ph.dataset.vlen : P().length, dx = e.clientX - st.x0, ns = Math.max(0, Math.round(st.c.start + dx / st.w * vlen)); act('clip.move', { clip: st.c.id, start: ns }).then(() => { clipDrag = null; }); } else clipDrag = null; }
    if (specDrag) { const m = specDrag.moved; specDrag = null; if (m) rr(); }
    document.querySelectorAll('[data-hold].on').forEach(h => { h.classList.remove('on'); h.style.transform = ''; h.textContent = h.dataset.txt; call('transport.hold', { on: false }).catch(() => { }); });
  });
  async function addAutoPoint(trackId, at, y) {
    const ln = curLane(); if (!ln || ln.t.id !== trackId) { D.ui.autoTarget = `${trackId}:volume`; }
    const l2 = curLane(), tg0 = l2.target; let val;
    if (tg0 === 'volume') val = Math.round((y * 48 - 36) * 10) / 10; else if (tg0 === 'pan') val = Math.round((y * 2 - 1) * 100) / 100; else { const [, fid, key] = tg0.split(':'), e = F.findFx(+fid), p = e && Flux.pspec(e.kind, key); val = p ? F.unpct(p, y * 100) : y; }
    const pts = (l2.lane ? l2.lane.points.slice() : []).filter(q => Math.abs(q[0] - at) > F.sr() * .01).concat([[at, val]]).sort((a, b) => a[0] - b[0]);
    await act('lane.set', { track: l2.t.id, target: tg0, points: pts, shape: l2.lane ? l2.lane.shape : 'linear' });
  }

  /* keyboard */
  const keyName = e => [e.ctrlKey || e.metaKey ? 'Ctrl' : '', e.altKey ? 'Alt' : '', e.shiftKey ? 'Shift' : '', e.key === ' ' ? 'Space' : e.key === 'Delete' ? 'Del' : e.key.length === 1 ? e.key.toUpperCase() : e.key].filter(Boolean).join(' ');
  const KEYACT = { play: () => togglePlay(), split: () => P() && ACT['edit-split'](), delete: () => ACT['edit-delete'](), undo: () => ACT.undo(), redo: () => ACT.redo(), hold: null, palette: () => { D.ui.paletteBase = cur; D.ui.cq = ''; D.ui.cqi = 0; render('palette'); }, zoom: () => ACT.zoom(), marker: () => P() && ACT['marker-add'](), record: () => render('record'), import: () => ACT.import({ dataset: {} }), export: () => P() && render('export'), sheet: () => render('shortcuts') };
  document.addEventListener('keydown', e => {
    if (D.ui.keyEdit && !['Shift', 'Control', 'Alt', 'Meta'].includes(e.key)) { e.preventDefault(); const k = D.ui.keyEdit; D.ui.keyEdit = null; if (e.key !== 'Escape') { const keys = Object.assign({}, (D.settings && D.settings.keys) || {}, { [k]: keyName(e) }); act('settings.set', { values: { keys } }, { refresh: false }).then(s => { if (s) D.settings = s; rr(); }); } else rr(); return; }
    const t = e.target.closest && e.target.closest('.sl .track, .fader .ftk');
    if (t && /Arrow|Home|End|Page/.test(e.key)) {
      e.preventDefault(); const s = t.closest('.sl,.fader'), v = parseFloat(s.style.getPropertyValue('--v')) || 0, up = e.key === 'ArrowRight' || e.key === 'ArrowUp' || e.key === 'PageUp', st = /Page/.test(e.key) ? 10 : e.shiftKey ? 0.5 : 2;
      const n = e.key === 'Home' ? 0 : e.key === 'End' ? 100 : Math.min(100, Math.max(0, v + (up ? st : -st)));
      s.style.setProperty('--v', n + '%'); t.setAttribute('aria-valuenow', Math.round(n));
      if (s.dataset.bind) { if (!bindG) bindG = newGesture(); const d = { el: s, box: t }; label(d, bindApply(s.dataset.bind, n, false)); clearTimeout(t._kt); t._kt = setTimeout(() => bindApply(s.dataset.bind, n, true), 450); }
      return;
    }
    if (cur === 'palette') { if (e.key === 'ArrowDown' || e.key === 'ArrowUp') { e.preventDefault(); D.ui.cqi = Math.max(0, (D.ui.cqi || 0) + (e.key === 'ArrowDown' ? 1 : -1)); rr(); return; } if (e.key === 'Enter') { e.preventDefault(); const it = stage.querySelector('.cmd .it.on'); if (it) it.click(); return; } }
    if (e.key === 'Escape') { const c = stage.querySelector('.ov .btn.ic[aria-label="Close"]'); if (c) { e.preventDefault(); c.click(); } return; }
    if (/INPUT|TEXTAREA|SELECT/.test(document.activeElement.tagName)) return;
    const kn = keyName(e), custom = (D.settings && D.settings.keys) || {};
    const hit = Flux.KEYS.find(k => (custom[k[0]] || k[2]) === kn);
    if (hit && KEYACT[hit[0]]) { if (hit[0] === 'play' && document.activeElement.closest('button,a')) return; e.preventDefault(); KEYACT[hit[0]](); return; }
    if (hit && hit[0] === 'hold' && !e.repeat) { call('transport.hold', { on: true }).catch(() => { }); }
  });
  document.addEventListener('keyup', e => { const custom = (D.settings && D.settings.keys) || {}; if (keyName(e) === (custom.hold || 'A')) call('transport.hold', { on: false }).catch(() => { }); });

  /* command palette: screens plus actions */
  Flux.commands = q => {
    const acts = [['Play or pause', 'play', null, 'play'], ['Undo', 'undo', null, 'undo', 'undo'], ['Redo', 'redo', null, 'redo', 'redo'], ['Split at playhead', 'cut', null, 'edit-split'], ['Add a file', 'imp', null, 'import'], ['Export', 'exp', 'export'], ['Master to −14 LUFS', 'master', 'master'], ['Normalize selected clip', 'master', null, 'clip-normalize'], ['Noise reduction', 'mic', 'fx-clean'], ['Add reverb', 'room', 'fx-reverb'], ['Make it 8D', 'orbit', 'stage'], ['Split into stems', 'stem', 'stems-setup'], ['Save a snapshot', 'disk', null, 'snapshot-save'], ['Connect an agent (MCP)', 'user', 'help-agents']];
    const out = acts.map(a => ({ t: a[0], i: a[1], go: a[2], act: a[3] === 'play' ? null : a[3], x: a[3] === 'clip-normalize' && D.sel.clip != null ? ` data-clip="${D.sel.clip}"` : '' })).filter(a => a.go || a.act).concat(S.filter(s => !['palette', 'error', 'kit'].includes(s.id)).map(s => ({ t: s.name, i: 'exp', go: s.id })));
    if (!q) return out;
    const sc = c => { const t = c.t.toLowerCase(); return t.startsWith(q) ? 0 : t.includes(' ' + q) ? 1 : t.includes(q) ? 2 : 9; };
    return out.filter(c => sc(c) < 9).sort((a, b) => sc(a) - sc(b));
  };

  /* ---------- transport polling and meters ---------- */
  let polling = false, nextPoll = 0, lastScope = 0;
  async function poll(now) {
    if (now) nextPoll = 0;
    if (polling || performance.now() < nextPoll) return; polling = true;
    try {
      const s = await call('transport.state'); const was = D.st.playing, wasRec = !!D.st.recording; D.st = s;
      if (s.rev !== lastRev && !dragging) { lastRev = s.rev; if (D.project && s.project === D.project.id || s.project) { F.stale(...F.PROJECT_KEYS); await F.need(['project'].concat(NEED[cur] || []).filter(k => F.PROJECT_KEYS.includes(k)), true); rr(); } }
      if (was !== s.playing || wasRec !== !!s.recording) { document.querySelectorAll('[data-play]').forEach(b => { b.innerHTML = ic(s.playing ? 'pause' : 'play'); b.setAttribute('aria-label', s.playing ? 'Pause' : 'Play'); }); if (cur === 'record') rr(); }
      if (s.playing && Viz.needsScope() && performance.now() - lastScope > 45) { lastScope = performance.now(); call('transport.scope', { bins: 64 }).then(r => { D.scope = r; }, () => { }); }
    } catch (e) { /* the engine may be busy; try again */ }
    polling = false; nextPoll = performance.now() + (D.st.playing || D.st.recording ? 33 : 300);
  }
  function tick(ms) {
    poll();
    const s = D.st, sec = s.playing ? s.seconds : s.seconds;
    stage.querySelectorAll('[data-tc]').forEach(el => el.textContent = F.tc(sec));
    const ph = stage.querySelector('[data-ph]');
    if (ph) { const lane = stage.querySelector('.lane'); if (lane) { const x = ((s.pos || 0) - (+ph.dataset.t0 || 0)) / (+ph.dataset.vlen || 1); ph.style.transform = `translateX(${Math.max(0, Math.min(1, x)) * lane.offsetWidth}px)`; ph.style.left = lane.offsetLeft + 'px'; } }
    if (!calm()) kin.forEach(k => { const lv = s.playing ? Math.max(0, (F.lvl() + 40) / 40) : 0, v = Math.min(1, Math.max(0, (.5 + .5 * Math.sin(ms / 640 + k.i * .7) * Math.sin(ms / 1610 + k.i * .29)) * .5 + Math.exp(-Math.pow((k.x - px) / 160, 2)) * .55 + lv * .25)); k.l.style.fontWeight = Math.round(kinLo + v * (kinHi - kinLo)); k.l.style.transform = `scaleY(${(.94 + v * .2).toFixed(3)})`; });
    const pk = s.peak || [0, 0], lvp = x => Math.max(0, Math.min(100, (x > 0 ? 20 * Math.log10(x) + 60 : 0) / 60 * 100));
    stage.querySelectorAll('[data-lv]').forEach((m, i) => m.style.setProperty('--lv', lvp(pk[i % 2] || 0) + '%'));
    const tp = s.tracks || [];
    stage.querySelectorAll('[data-lvv]').forEach(m => { const k = m.dataset.lvv, c = +m.dataset.ch || 0; const v = k === 'm' ? pk[c] : ((tp.find(x => x.id === +k) || {}).peak || [0, 0])[c]; m.style.setProperty('--lv', lvp(v || 0) + '%'); });
    const ip = s.input_peak || 0; stage.querySelectorAll('[data-lvin]').forEach(m => m.style.setProperty('--lv', lvp(ip) + '%'));
    stage.querySelectorAll('[data-lufs]').forEach(el => el.textContent = (s.playing && s.lufs_s > -70 ? F.num(s.lufs_s, 1) : '−∞') + ' LUFS');
    if (s.recording) stage.querySelectorAll('[data-rectime]').forEach(el => el.textContent = 'REC ' + F.tc(s.recording.seconds || 0).slice(0, 8));
    requestAnimationFrame(tick);
  }

  /* drag and drop files onto the window */
  F.onDrop(paths => { const audio = paths.filter(p => !/\.flux$/i.test(p)), pk = paths.filter(p => /\.flux$/i.test(p)); pk.forEach(p => act('project.import_package', { path: p }).then(() => render('edit'))); if (audio.length) importFiles(audio, cur === 'convert' || cur === 'batch' ? null : null); });

  addEventListener('hashchange', () => { const id = location.hash.slice(1); if (id && id !== cur && M[id]) render(id, false); });
  addEventListener('resize', () => stage.querySelectorAll('.seg').forEach(placeInd));
  if (document.fonts && document.fonts.ready) document.fonts.ready.then(() => stage.querySelectorAll('.seg').forEach(placeInd));

  /* ---------- start ---------- */
  async function start() {
    try { await F.need(['settings', 'info', 'catalog', 'project', 'caps', 'ai', 'recovery'], true); }
    catch (e) { /* need() records errors per key */ }
    if (!D.settings) { stage.innerHTML = `<div class="wiz"><div class="inner"><h1 class="h1">Flux can't reach its engine.</h1><p class="lede">${F.esc((D.err && D.err.settings && D.err.settings.message) || 'Start Flux again. If this keeps happening, reinstall it.')}</p></div></div>`; return; }
    applyLook(D.settings.theme, D.settings.font);
    lastRev = D.project ? D.project.rev : -1;
    const id = location.hash.slice(1);
    const first = M[id] && id !== 'error' ? id : !D.settings.onboarded ? 'boot' : (D.recovery || []).length ? 'recovery' : D.project ? (D.settings.mode === 'simple' ? 'task' : 'edit') : 'home';
    await render(first, false);
    requestAnimationFrame(tick);
    if (F.updater) {
      F.updater.on((kind, v) => { if (kind === 'status') { D.update = v; if (cur === 'prefs-about') rr(); } else { D.update = Object.assign({}, D.update, { ready: v }); showUpdate(); } });
      F.updater.status().then(v => { D.update = v; if (v && v.ready) showUpdate(); }).catch(() => { });
    }
  }
  const want = () => { const n = fontRow()[1]; return Promise.all([document.fonts.load(`900 64px '${n}'`), document.fonts.load(`500 14px '${n}'`), document.fonts.load("400 12px 'Geist Mono'")]); };
  (document.fonts ? Promise.race([want(), new Promise(r => setTimeout(r, 1800))]).catch(() => { }) : Promise.resolve()).then(start);
})();
