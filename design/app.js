/* Flux prototype behavior: routing, fluid transitions, and shared control logic. */
(function () {
  const { S, M, ic, THEMES, FONTS } = Flux, stage = document.getElementById('stage'), map = document.getElementById('map'), nameEl = document.getElementById('rvname');
  const calm = matchMedia('(prefers-reduced-motion: reduce)').matches;
  let cur = null, playing = false, t0 = 42.318, lastT = 0;

  function fmt(s) { const h = Math.floor(s / 3600), m = Math.floor(s / 60) % 60, se = s % 60; return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${se.toFixed(3).padStart(6, '0')}`; }

  function placeInd(seg) { const on = seg.querySelector('button.on'), ind = seg.querySelector('.ind'); if (!on || !ind) return; ind.style.width = on.offsetWidth + 'px'; ind.style.transform = `translateX(${on.offsetLeft}px)`; }
  function hydrate(root) {
    root.querySelectorAll('.seg').forEach(s => { const ind = s.querySelector('.ind'); if (ind) ind.style.transition = 'none'; placeInd(s); if (ind) { ind.getBoundingClientRect(); ind.style.transition = ''; } });
    root.querySelectorAll('.in').forEach(el => [...el.children].forEach((c, i) => c.style.setProperty('--n', Math.min(i, 14))));
    Viz.mount(root);
    splitType(root);
    const cur = THEMES.find(t => t[0] === theme());
    root.querySelectorAll('[data-set-theme]').forEach(b => b.setAttribute('aria-pressed', b.dataset.setTheme === cur[0]));
    root.querySelectorAll('[data-theme-name]').forEach(e => e.textContent = cur[1]);
    const cf = fontRow();
    root.querySelectorAll('[data-set-font]').forEach(b => b.setAttribute('aria-pressed', b.dataset.setFont === cf[0]));
    root.querySelectorAll('[data-font-name]').forEach(e => e.textContent = cf[1]);
    const rs = getComputedStyle(document.documentElement); kinLo = +rs.getPropertyValue('--kin-lo') || 640; kinHi = +rs.getPropertyValue('--kin-hi') || 900;
  }

  /* Display type is a live equalizer, like the landing headline: each letter's weight and height follow the level and the pointer. */
  let kin = [], kinLo = 640, kinHi = 900, px = -1e4;
  function splitType(root) {
    kin = [];
    root.querySelectorAll('.h1,.bigh').forEach(h => {
      if (h.closest('[aria-hidden]') || h.querySelector('a,span.act')) return;
      const text = h.textContent; h.setAttribute('aria-label', text); h.classList.add('kin');
      h.innerHTML = text.split(' ').map(w => `<span class="wd" aria-hidden="true">${[...w].map(c => `<span class="lt">${c}</span>`).join('')}</span>`).join(' ');
      const fs = parseFloat(getComputedStyle(h).fontSize) || 1;
      h.querySelectorAll('.lt').forEach((l, i) => { const r = l.getBoundingClientRect(); l.style.width = (r.width / fs) + 'em'; kin.push({ l, i, x: r.left + r.width / 2 }); });
    });
  }
  addEventListener('pointermove', e => { px = e.clientX; }, { passive: true });

  /* Themes */
  const theme = () => document.documentElement.dataset.flux || 'cassette';
  const fontRow = () => FONTS.find(x => x[0] === (document.documentElement.dataset.font || 'bricolage')) || FONTS[0];
  function paintThemeChrome() {
    const fr = fontRow(); document.getElementById('rvfn').textContent = fr[1];
    fpop.innerHTML = FONTS.map(x => `<button type="button" role="menuitemradio" class="ft-${x[0]}" data-set-font="${x[0]}" aria-pressed="${x[0] === fr[0]}">${x[1]}${x[0] === 'bricolage' ? ' <span class="mono" style="margin-left:auto">Default</span>' : ''}</button>`).join('');
    const t = THEMES.find(x => x[0] === theme()), sw = document.getElementById('rvswa');
    sw.style.background = t[2]; document.getElementById('rvtn').textContent = t[1];
    pop.innerHTML = THEMES.map(x => `<button type="button" role="menuitemradio" data-set-theme="${x[0]}" aria-pressed="${x[0] === t[0]}"><span class="swa" style="background:${x[2]}"></span>${x[1]}${x[0] === 'cassette' ? ' <span class="mono" style="margin-left:auto">Default</span>' : ''}</button>`).join('');
  }
  const canVT = () => document.startViewTransition && !calm && document.visibilityState === 'visible';
  const quiet = vt => { vt.ready.catch(() => {}); vt.finished.catch(() => {}); return vt; };
  function setTheme(id, ev) {
    if (id === theme()) return;
    const apply = () => { document.documentElement.dataset.flux = id; try { localStorage.setItem('flux-theme', id); } catch (e) {} paintThemeChrome(); stage.innerHTML = M[cur].fn(); hydrate(stage); };
    reveal(apply, ev);
  }
  function setFont(id, ev) {
    const f = FONTS.find(x => x[0] === id);
    if (!f || f === fontRow()) return;
    const apply = () => { document.documentElement.dataset.font = id; try { localStorage.setItem('flux-font', id); } catch (e) {} paintThemeChrome(); stage.innerHTML = M[cur].fn(); hydrate(stage); };
    const go = () => reveal(apply, ev);
    document.fonts.load(`800 40px '${f[1]}'`).then(go, go);
  }
  function reveal(apply, ev) {
    if (!canVT()) { apply(); return; }
    const x = ev && ev.clientX || innerWidth / 2, y = ev && ev.clientY || innerHeight / 2, r = Math.hypot(Math.max(x, innerWidth - x), Math.max(y, innerHeight - y)), root = document.documentElement;
    root.classList.add('vt-theme');
    const vt = quiet(document.startViewTransition(apply));
    vt.ready.then(() => root.animate({ clipPath: [`circle(0px at ${x}px ${y}px)`, `circle(${r}px at ${x}px ${y}px)`] }, { duration: 950, easing: 'cubic-bezier(.16,1,.3,1)', pseudoElement: '::view-transition-new(root)' })).catch(() => {});
    vt.finished.catch(() => {}).finally(() => root.classList.remove('vt-theme'));
  }
  function render(id, push) {
    if (!M[id]) id = 'home';
    const go = () => { stage.innerHTML = M[id].fn(); cur = id; hydrate(stage); nameEl.textContent = `${M[id].i + 1} of ${S.length}: ${M[id].group} · ${M[id].name}`; const m = stage.querySelector('#m'); if (m) m.scrollTop = 0; };
    if (canVT() && cur) quiet(document.startViewTransition(go)); else go();
    if (push !== false) { try { history.replaceState(null, '', '#' + id); } catch (e) { location.hash = id; } }
  }

  /* ---- delegated interaction ---- */
  document.addEventListener('click', e => {
    const st = e.target.closest('[data-set-theme]');
    if (st) { setTheme(st.dataset.setTheme, e); return; }
    const sf = e.target.closest('[data-set-font]');
    if (sf) { setFont(sf.dataset.setFont, e); return; }
    const pk = e.target.closest('[data-pick]');
    if (pk) { pk.parentElement.querySelectorAll('[data-pick]').forEach(b => b.setAttribute('aria-pressed', b === pk)); return; }
    const g = e.target.closest('[data-go]');
    if (g && !g.closest('[data-hold]')) { e.preventDefault(); if (g.dataset.go) render(g.dataset.go); return; }
    const tile = e.target.closest('.tile:not([data-go])');
    if (tile && tile.tagName === 'A') e.preventDefault();
    if (tile && tile.parentElement) { tile.parentElement.querySelectorAll('.tile.on').forEach(x => x.classList.remove('on')); tile.classList.add('on'); }
    const sb = e.target.closest('.seg button');
    if (sb) {
      const s = sb.parentElement; s.querySelectorAll('button').forEach(b => { b.classList.toggle('on', b === sb); b.setAttribute('aria-pressed', b === sb); }); placeInd(s);
      if (s.hasAttribute('data-mode')) { if (sb.textContent === 'Simple') render('task'); else if (cur && cur.startsWith('task')) render('edit'); }
      if (s.hasAttribute('data-stagemode')) { const c = stage.querySelector('canvas[data-v="stage"]'); c.dataset.mode = { Orbit: 'orbit', Spiral: 'spiral', 'Figure 8': 'eight', Swarm: 'swarm', Place: 'place' }[sb.textContent]; }
      if (s.hasAttribute('data-stagetool')) { const c = stage.querySelector('canvas[data-v="stage"]'); c.dataset.tool = sb.textContent === 'Draw path' ? 'draw' : 'drag'; }
      return;
    }
    const tg = e.target.closest('.tg');
    if (tg) { tg.setAttribute('aria-pressed', tg.getAttribute('aria-pressed') !== 'true'); return; }
    const tab = e.target.closest('.tabs button');
    if (tab) { tab.parentElement.querySelectorAll('button').forEach(b => b.classList.toggle('on', b === tab)); return; }
    const chip = e.target.closest('button.chip');
    if (chip && chip.parentElement.classList.contains('row') && !chip.hasAttribute('data-toggle')) { chip.parentElement.querySelectorAll('.chip').forEach(c => c.classList.toggle('on', c === chip)); return; }
    const tt = e.target.closest('[data-toggle]');
    if (tt) { tt.classList.toggle('on'); return; }
    const pl = e.target.closest('[data-play]');
    if (pl) { playing = !playing; document.querySelectorAll('[data-play]').forEach(b => { b.innerHTML = ic(playing ? 'pause' : 'play'); b.setAttribute('aria-label', playing ? 'Pause' : 'Play'); }); return; }
    const a = e.target.closest('a[href^="#"]');
    if (a) e.preventDefault();
  });

  /* sliders and faders */
  let drag = null;
  document.addEventListener('pointerdown', e => {
    const t = e.target.closest('.sl .track'); const f = e.target.closest('.fader .ftk');
    if (t) { drag = { el: t.closest('.sl'), box: t, kind: 'sl' }; drag.el.classList.add('dr'); t.setPointerCapture(e.pointerId); move(e); }
    else if (f) { drag = { el: f.closest('.fader'), box: f, kind: 'f' }; drag.el.classList.add('dr'); f.setPointerCapture(e.pointerId); move(e); }
    const h = e.target.closest('[data-hold]');
    if (h) { h.classList.add('on'); h._t = h.querySelector('span') ? null : h.textContent; h.style.transform = 'scale(.97)'; h.dataset.txt = h.textContent; h.textContent = 'Original'; }
  });
  function move(e) {
    if (!drag) return; const r = drag.box.getBoundingClientRect();
    if (drag.kind === 'sl') { const p = Math.min(100, Math.max(0, (e.clientX - r.left) / r.width * 100)); drag.el.style.setProperty('--v', p + '%'); drag.box.setAttribute('aria-valuenow', Math.round(p)); const lbl = drag.el.querySelector('.sv'); if (lbl && /%/.test(lbl.textContent)) lbl.textContent = Math.round(p) + '%'; }
    else { const p = Math.min(100, Math.max(0, (r.bottom - e.clientY) / r.height * 100)); drag.el.style.setProperty('--v', p + '%'); }
  }
  document.addEventListener('pointermove', move);
  addEventListener('pointerup', () => {
    if (drag) { drag.el.classList.remove('dr'); drag = null; }
    document.querySelectorAll('[data-hold].on').forEach(h => { h.classList.remove('on'); h.style.transform = ''; h.textContent = h.dataset.txt; });
  });
  document.addEventListener('keydown', e => {
    const t = e.target.closest && e.target.closest('.sl .track');
    if (t && (e.key === 'ArrowLeft' || e.key === 'ArrowRight')) { e.preventDefault(); const s = t.closest('.sl'), v = parseFloat(s.style.getPropertyValue('--v')) || 0, n = Math.min(100, Math.max(0, v + (e.key === 'ArrowRight' ? 2 : -2))); s.style.setProperty('--v', n + '%'); t.setAttribute('aria-valuenow', Math.round(n)); return; }
    if (/INPUT|TEXTAREA|SELECT/.test(document.activeElement.tagName)) return;
    if (e.key === ' ' && !document.activeElement.closest('button,a')) { e.preventDefault(); stage.querySelector('[data-play]') && stage.querySelector('[data-play]').click(); }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); render('palette'); }
    if (e.key === 'ArrowRight' && e.altKey) step(1);
    if (e.key === 'ArrowLeft' && e.altKey) step(-1);
    if (e.key === 'Escape') { if (!map.hidden) { map.hidden = true; return; } const c = stage.querySelector('.ov .btn.ic'); if (c) c.click(); }
  });

  /* meters and playhead */
  function tick(ms) {
    const dt = (ms - lastT) / 1000; lastT = ms;
    if (playing) { t0 += dt; }
    const tc = stage.querySelectorAll('[data-tc]'); tc.forEach(el => el.textContent = fmt(t0));
    const ph = stage.querySelector('[data-ph]');
    if (ph) { const lane = stage.querySelector('.lane'); if (lane) ph.style.transform = `translateX(${(t0 / 222) * lane.offsetWidth}px)`; }
    const lv = playing ? .5 + .4 * Math.sin(ms / 170) : 0;
    if (!calm) kin.forEach(k => { const v = Math.min(1, Math.max(0, (.5 + .5 * Math.sin(ms / 640 + k.i * .7) * Math.sin(ms / 1610 + k.i * .29)) * .5 + Math.exp(-Math.pow((k.x - px) / 160, 2)) * .55 + lv * .25)); k.l.style.fontWeight = Math.round(kinLo + v * (kinHi - kinLo)); k.l.style.transform = `scaleY(${(.94 + v * .2).toFixed(3)})`; });
    const lvl = playing ? .62 + .22 * Math.sin(ms / 170) + .1 * Math.sin(ms / 53) : .0;
    stage.querySelectorAll('[data-lv]').forEach((m, i) => m.style.setProperty('--lv', (Math.max(0, lvl + (i % 2) * .03) * 100) + '%'));
    stage.querySelectorAll('[data-lvv]').forEach((m, i) => { const l = playing ? .5 + .28 * Math.sin(ms / (130 + i * 11) + i) : 0; m.style.setProperty('--lv', (l * 100) + '%'); });
    requestAnimationFrame(tick);
  }
  requestAnimationFrame(tick);

  /* review bar */
  function step(d) { const i = M[cur].i, n = (i + d + S.length) % S.length; render(S[n].id); }
  document.getElementById('rvprev').onclick = () => step(-1);
  document.getElementById('rvnext').onclick = () => step(1);
  document.querySelectorAll('[data-w]').forEach(b => b.onclick = () => {
    document.querySelectorAll('[data-w]').forEach(x => x.classList.toggle('on', x === b));
    stage.style.width = b.dataset.w; document.getElementById('frame').classList.toggle('sized', b.dataset.w !== '100%');
    setTimeout(() => { stage.querySelectorAll('.seg').forEach(placeInd); }, 800);
  });
  const pop = document.getElementById('rvpop'), fpop = document.getElementById('rvfpop'), rvt = document.getElementById('rvtheme'), rvf = document.getElementById('rvfont');
  rvt.onclick = e => { e.stopPropagation(); fpop.hidden = true; rvf.setAttribute('aria-expanded', false); pop.hidden = !pop.hidden; rvt.setAttribute('aria-expanded', !pop.hidden); };
  rvf.onclick = e => { e.stopPropagation(); pop.hidden = true; rvt.setAttribute('aria-expanded', false); fpop.hidden = !fpop.hidden; rvf.setAttribute('aria-expanded', !fpop.hidden); };
  document.addEventListener('click', e => {
    if (!pop.hidden && !e.target.closest('#rvpop')) { pop.hidden = true; rvt.setAttribute('aria-expanded', false); }
    if (!fpop.hidden && !e.target.closest('#rvfpop')) { fpop.hidden = true; rvf.setAttribute('aria-expanded', false); }
  });
  paintThemeChrome();
  function buildMap() {
    const groups = {}; S.forEach(s => (groups[s.group] = groups[s.group] || []).push(s));
    map.innerHTML = `<h2>All ${S.length} screens</h2><p class="dim" style="font-size:15px">Plus the landing page. Click one to open it. Every button in the prototype also works.</p><input class="q inp" id="mq" placeholder="Search screens" aria-label="Search screens"><div class="gs">${Object.entries(groups).map(([g, it]) => `<section class="g"><h3>${g}</h3>${it.map(s => `<button type="button" data-open="${s.id}"><span class="mono">${s.i + 1}</span>${s.name}</button>`).join('')}</section>`).join('')}</div>`;
    map.querySelector('#mq').addEventListener('input', e => { const q = e.target.value.toLowerCase(); map.querySelectorAll('.g button').forEach(b => b.hidden = !b.textContent.toLowerCase().includes(q)); });
  }
  document.getElementById('rvmap').onclick = () => { if (map.hidden) { buildMap(); map.hidden = false; map.scrollTop = 0; } else map.hidden = true; };
  map.addEventListener('click', e => { const b = e.target.closest('[data-open]'); if (b) { map.hidden = true; render(b.dataset.open); } });
  addEventListener('hashchange', () => { const id = location.hash.slice(1); if (id && id !== cur && M[id]) render(id, false); });
  addEventListener('resize', () => stage.querySelectorAll('.seg').forEach(placeInd));
  if (document.fonts && document.fonts.ready) document.fonts.ready.then(() => stage.querySelectorAll('.seg').forEach(s => { const ind = s.querySelector('.ind'); if (ind) ind.style.transition = 'none'; placeInd(s); if (ind) { ind.getBoundingClientRect(); ind.style.transition = ''; } }));

  const start = () => { const id = location.hash.slice(1); render(M[id] ? id : 'home', false); };
  const want = () => { const n = fontRow()[1]; return Promise.all([document.fonts.load(`900 64px '${n}'`), document.fonts.load(`500 14px '${n}'`), document.fonts.load("400 12px 'Geist Mono'")]); };
  (document.fonts ? Promise.race([want(), new Promise(r => setTimeout(r, 1800))]).catch(() => {}) : Promise.resolve()).then(start);
})();
