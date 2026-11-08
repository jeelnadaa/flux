/* Assets: generated audio you listen to first, and each project's audio, on the timeline or not.
   Composed from the kit (pr, pg, sec, lst, box, btn, sel, pill, cal). See design/LIVE.md. Listening uses a plain audio
   element fed from the engine, so it plays on the system's default output and needs no open project. */
(function () {
  const { reg, pr, pg, btn, pill, sel, sec, cal, ic } = Flux, F = FD, D = FD.D, esc = F.esc;
  const call = (m, p) => F.call(m, p);
  const U = () => D.ui.as = D.ui.as || { to: {}, names: {}, err: null, msg: null, adding: false };
  const P = () => D.project;
  const NEW = '__new';

  /* ---------- listening ---------- */
  const audio = new Audio(), pl = { key: null, loading: false }, cache = new Map();
  const mmss = s => { s = Math.max(0, Math.floor(s || 0)); return Math.floor(s / 60) + ':' + String(s % 60).padStart(2, '0'); };
  function paint() {
    document.querySelectorAll('[data-pl]').forEach(r => {
      const on = r.dataset.pl === pl.key, playing = on && !audio.paused && !pl.loading, len = (on && isFinite(audio.duration) && audio.duration) || +r.dataset.len || 0, t = on && !pl.loading ? audio.currentTime : 0;
      const b = r.querySelector('[data-pl-b]'), bar = r.querySelector('[data-aseek]'), txt = r.querySelector('[data-pl-t]'), word = on && pl.loading ? 'Loading…' : playing ? 'Pause' : 'Listen';
      if (b) { b.innerHTML = ic(playing ? 'pause' : 'play') + (b.dataset.lbl ? `<span>${word}</span>` : ''); if (!b.dataset.lbl) b.setAttribute('aria-label', word + ' ' + (b.dataset.name || '')); }
      if (bar) { const pc = len ? Math.min(100, t / len * 100) : 0; bar.style.setProperty('--v', pc + '%'); bar.setAttribute('aria-valuenow', Math.round(pc)); }
      if (txt) txt.textContent = `${mmss(t)} / ${mmss(len)}`;
      const box = r.querySelector('[data-pl-more]'); if (box) box.hidden = !on;
    });
  }
  ['play', 'pause', 'ended', 'timeupdate', 'loadedmetadata'].forEach(n => audio.addEventListener(n, paint));
  async function urlFor(key, src) {
    if (cache.has(key)) { const u = cache.get(key); cache.delete(key); cache.set(key, u); return u; }
    const r = await call('audio.preview', src), parts = [];
    for (let at = 0; ;) {
      const c = await call('audio.preview_read', { token: r.token, at }), bin = atob(c.data), b = new Uint8Array(bin.length);
      for (let i = 0; i < bin.length; i++) b[i] = bin.charCodeAt(i);
      parts.push(b); at += b.length;
      if (!c.more || !b.length) break;
    }
    const url = URL.createObjectURL(new Blob(parts, { type: 'audio/wav' }));
    cache.set(key, url);
    if (cache.size > 6) { const [k, u] = cache.entries().next().value; URL.revokeObjectURL(u); cache.delete(k); }
    return url;
  }
  async function listen(key, src) {
    const quiet = () => { if (D.st && D.st.playing && Flux.pauseProject) Flux.pauseProject(); };
    if (pl.key === key && !pl.loading) { if (audio.paused) { quiet(); audio.play(); } else audio.pause(); return; }
    audio.pause(); pl.key = key; pl.loading = true; paint(); quiet();
    try {
      const url = await urlFor(key, src);
      if (pl.key !== key) return;
      audio.src = url; pl.loading = false; await audio.play();
    } catch (e) { if (pl.key === key) { pl.key = null; pl.loading = false; U().err = e.message || 'That can\'t be played.'; Flux.rr(); } }
    paint();
  }
  /* The project and this preview never play over each other. */
  Flux.stopPreview = () => { if (!audio.paused) audio.pause(); };

  const seekBar = (key, name) => `<div class="plbar" data-tip="Click or use the arrow keys to move through it." role="slider" tabindex="0" aria-label="Position in ${esc(name)}" aria-valuemin="0" aria-valuemax="100" aria-valuenow="0" data-aseek style="--v:0%"><i></i></div>`;
  const srcOf = el => el.dataset.take ? { take: el.dataset.take } : { media: el.dataset.media, project: el.dataset.project };

  /* ---------- generated takes ---------- */
  const projects = () => ((D.library && D.library.projects) || []).filter(p => !p.open_elsewhere || (P() && P().id === p.id));
  function pickTo(t) {
    const u = U(), list = projects();
    if (u.to[t.id] === NEW || list.some(p => p.id === u.to[t.id])) return u.to[t.id];
    return P() ? P().id : (list[0] ? list[0].id : NEW);
  }
  function takeCard(t) {
    const u = U(), to = pickTo(t), open = P() && P().id, list = projects(), saved = t.saved.some(s => s.project === to);
    const opts = [...(open ? [[open, 'This project: ' + esc(P().name)]] : []), ...list.filter(p => p.id !== open).map(p => [p.id, esc(p.name)]), [NEW, 'New project']];
    const key = 't' + t.id, sub = [F.num(t.seconds, 1) + ' s', t.kind, t.voice, F.ago(t.created * 1000)].filter(Boolean).map(esc).join(' · ');
    return `<div class="box col" data-take-card="${esc(t.id)}">
      <div class="lst"><div><div class="ti"><b>${esc(t.name)}</b><span>${sub}</span></div>${pill('Synthetic', 'ai')}</div></div>
      <div class="row plr" data-pl="${key}" data-take="${esc(t.id)}" data-len="${t.seconds}">${btn('Listen', { i: 'play', x: 'data-aact="play" data-pl-b data-lbl="1" data-tip="Hear it before it goes anywhere."' })}${seekBar(key, t.name)}<span class="mono dim" data-pl-t>0:00 / ${mmss(t.seconds)}</span></div>
      ${t.sounds_left_out ? cal('info', `${t.sounds_left_out} sound${t.sounds_left_out === 1 ? '' : 's'} left out. This voice can't perform them.`) : ''}
      <div class="row">${sel(opts, to, `data-aact-change="to" data-id="${esc(t.id)}" aria-label="Save to" data-tip="Which project to add this to. It doesn't have to be the open one." style="flex:1 1 180px;min-width:0"`)}
      ${to === NEW ? `<input class="inp" style="flex:1 1 160px;min-width:0" aria-label="New project name" placeholder="Project name" data-tip="Name for the new project." data-aname="${esc(t.id)}" value="${esc(u.names[t.id] || '')}">` : ''}
      ${saved && to === open ? btn('Add to timeline', { k: 'p', i: 'plus', x: `data-aact="place" data-media="${esc(t.id)}" data-tip="Put it on the timeline at the playhead."` }) : btn(saved ? 'In that project' : 'Add to assets', { k: 'p', i: saved ? 'check' : 'plus', x: saved ? 'disabled data-tip="It\'s already there."' : `data-aact="save" data-id="${esc(t.id)}" data-tip="Keep it in that project. The timeline isn't touched."` })}
      ${btn('Discard', { k: 'g dng', x: `data-aact="discard" data-id="${esc(t.id)}" data-tip="Throw this away. Copies already in projects stay."` })}</div></div>`;
  }
  const notes = () => { const u = U(); return `${u.err ? cal('warn', esc(u.err), 'bad') : ''}${u.msg ? cal('check', esc(u.msg)) : ''}`; };
  /* The newest result on the page that made it. */
  Flux.takeCardFor = id => { const t = id && D.takes && D.takes.takes.find(x => x.id === id); return t ? `${takeCard(t)}${notes()}` : ''; };

  /* ---------- a project's assets ---------- */
  function assetRow(a, A) {
    const key = 'a' + A.project.id + a.id, where = a.uses ? `On the timeline${a.uses > 1 ? ' ' + a.uses + ' times' : ''}` : 'Not on the timeline';
    return `<div data-pl="${key}" data-media="${esc(a.id)}" data-project="${esc(A.project.id)}" data-len="${a.seconds}" data-asset="${esc(a.id)}">
      ${btn('', { k: 'g', i: 'play', a: 'Listen ' + esc(a.name), x: `data-aact="play" data-pl-b data-name="${esc(a.name)}"` })}
      <div class="ti"><b>${esc(a.name)}</b><span>${mmss(a.seconds)} · ${where}${a.synthetic ? ' · Synthetic' : ''}</span></div>
      ${A.open ? btn('Add to timeline', { x: `data-aact="place" data-media="${esc(a.id)}"` }) : ''}
      ${A.open && !a.uses ? btn('', { k: 'g', i: 'trash', a: 'Remove ' + esc(a.name) + ' from this project', x: `data-aact="remove" data-media="${esc(a.id)}"` }) : ''}
      <div class="row plr plm" data-pl-more hidden>${seekBar(key, a.name)}<span class="mono dim" data-pl-t>0:00 / ${mmss(a.seconds)}</span></div></div>`;
  }
  const none = t => `<div class="lst"><div><div class="ti"><span>${t}</span></div></div></div>`;
  /* For the Edit panel and the empty project page: the open project's assets. */
  Flux.assetBlock = (limit, title = 'Assets') => {
    const A = D.assets; if (!A || !A.project) return '';
    const list = A.assets.slice().reverse(), rows = limit ? list.slice(0, limit) : list;
    return sec(title, `${rows.length ? `<div class="lst">${rows.map(a => assetRow(a, A)).join('')}</div>` : none('Nothing saved in this project yet.')}<div class="row">${btn('All assets', { go: 'assets' })}${btn('Generate speech', { k: 'g', go: 'tts' })}</div>`);
  };

  reg('assets', 'Workspaces', 'Assets', () => {
    const u = U(), other = D.ui.assetsProject || !P(), A = (other ? D.assetsOther : D.assets) || { assets: [] }, takes = (D.takes && D.takes.takes) || [], list = (D.library && D.library.projects) || [];
    const body = `
      ${sec('Generated', takes.length ? `<p class="dim" style="margin:0;font-size:13px">Listen first. Nothing goes in a project until you add it.</p>${takes.slice(0, 30).map(takeCard).join('')}` : none('Nothing generated yet.'))}
      ${sec('In a project', `<div class="row">${list.length ? sel(list.map(p => [p.id, esc(p.name)]), A.project ? A.project.id : '', 'data-aact-change="view" aria-label="Project" style="flex:1 1 200px;min-width:0"') : ''}${A.project ? btn(u.adding ? 'Adding…' : 'Add a file', { i: 'imp', x: `data-aact="addfile" data-project="${esc(A.project.id)}"${u.adding ? ' disabled' : ''}` }) : ''}${A.project && !A.open ? btn('Open project', { k: 'g', x: `data-act="open-project" data-id="${esc(A.project.id)}"` }) : ''}</div>
        ${A.project ? (A.assets.length ? `<div class="lst box">${A.assets.slice().reverse().map(a => assetRow(a, A)).join('')}</div>` : none('No audio in this project yet.')) : none('No projects yet. Add generated audio to a new one above.')}`)}
      ${notes()}`;
    const go = btn('Generate speech', { k: 'p', i: 'text', go: 'tts' }), sub = 'Audio you\'ve made or added. Listen, then put it where you want it.';
    return P() ? pr('assets', `<div class="pg in"><div class="pgh"><div><h1 class="h1">Assets</h1><p class="lede">${sub}</p></div><div class="row">${go}</div></div>${body}</div>`) : pg('assets', 'Assets', sub, body, go);
  });

  /* ---------- actions ---------- */
  async function reloadAll(extra = []) {
    F.stale('takes', 'library', 'assets', 'assetsOther', ...F.PROJECT_KEYS, ...extra);
    await F.need(['takes', 'library', 'assets', 'assetsOther', 'project'], true);
    Flux.rr();
  }
  const say = (msg, err) => { const u = U(); u.msg = msg || null; u.err = err || null; };
  const ACT = {
    play: el => { const r = el.closest('[data-pl]'); listen(r.dataset.pl, srcOf(r)); },
    save: async el => {
      const u = U(), id = el.dataset.id, to = pickTo({ id }), p = to === NEW ? { id, new_project: u.names[id] || 'Generated speech' } : { id, project: to };
      try { const r = await call('take.save', p); u.to[id] = r.project; await reloadAll(); const n = (projects().find(x => x.id === r.project) || {}).name || 'the project'; say(`In the assets of ${n}.`); } catch (e) { say(null, e.message); }
      Flux.rr();
    },
    discard: async el => { try { await call('take.delete', { id: el.dataset.id }); say(); if (pl.key === 't' + el.dataset.id) { audio.pause(); pl.key = null; } if (D.ui.ttsTake === el.dataset.id) D.ui.ttsTake = null; } catch (e) { say(null, e.message); } await reloadAll(); },
    place: async el => { try { await call('asset.place', { media: el.dataset.media }); say('On the timeline, at the playhead.'); } catch (e) { say(null, e.message); } await reloadAll(); },
    remove: async el => { try { await call('asset.remove', { media: el.dataset.media }); say(); } catch (e) { say(null, e.message); } await reloadAll(); },
    addfile: async el => {
      const fs = await F.pickFiles({ multiple: true, kind: 'audio' }); if (!fs || !fs.length) return;
      const u = U(); u.adding = true; say(); Flux.rr();
      try { for (const f of fs) { await call('asset.add', { path: f, project: el.dataset.project, wait: true }); } } catch (e) { say(null, e.message); }
      u.adding = false; await reloadAll();
    },
    tag: el => { const ta = document.querySelector('[data-ui="ttsText"]'); if (!ta) return; const t = '[' + el.dataset.t + ']', at = ta.selectionStart, v = ta.value, ins = (at && v[at - 1] !== ' ' ? ' ' : '') + t + ' '; ta.value = v.slice(0, at) + ins + v.slice(at); ta.focus(); ta.selectionStart = ta.selectionEnd = at + ins.length; ta.dispatchEvent(new Event('input', { bubbles: true })); },
  };
  Flux.reloadTakes = reloadAll;
  document.addEventListener('click', e => { const el = e.target.closest('[data-aact]'); if (!el || el.disabled || e.button) return; e.preventDefault(); ACT[el.dataset.aact](el); });
  document.addEventListener('change', async e => {
    const el = e.target.closest('[data-aact-change]'); if (!el) return;
    if (el.dataset.aactChange === 'to') { U().to[el.dataset.id] = el.value; say(); Flux.rr(); return; }
    D.ui.assetsProject = P() && P().id === el.value ? null : el.value; say(); F.stale('assetsOther'); await F.need(['assetsOther'], true); Flux.rr();
  });
  document.addEventListener('input', e => { const el = e.target.closest('[data-aname]'); if (el) U().names[el.dataset.aname] = el.value; });
  document.addEventListener('pointerdown', e => {
    const bar = e.target.closest('[data-aseek]'); if (!bar || bar.closest('[data-pl]').dataset.pl !== pl.key || !isFinite(audio.duration)) return;
    const r = bar.getBoundingClientRect(); audio.currentTime = Math.max(0, Math.min(1, (e.clientX - r.left) / r.width)) * audio.duration;
  });
  document.addEventListener('keydown', e => {
    const bar = e.target.closest && e.target.closest('[data-aseek]'); if (!bar || !/^Arrow(Left|Right)$/.test(e.key)) return;
    e.stopImmediatePropagation(); e.preventDefault();
    if (bar.closest('[data-pl]').dataset.pl === pl.key) audio.currentTime = Math.max(0, audio.currentTime + (e.key === 'ArrowLeft' ? -5 : 5));
  }, true);
})();
