/* Voice cloning: models you download and choose, consent-gated voice profiles, and scripts with sounds.
   Composed from the kit (wiz wide, dlg, box, sec, fld, inp, sel, seg, sl, tg, btn, cal, pill, prog, lst). See
   design/LIVE.md for the pattern. Downloads start only from the Download dialog, which shows the size, licence and
   folder first. Nothing here sends audio or text anywhere. */
(function () {
  const { reg, wiz, dlg, btn, seg, sl, tg, pill, fld, inp, sel, sec, cal, prog, ic } = Flux, F = FD, D = FD.D, esc = F.esc;
  const call = (m, p) => F.call(m, p);
  const U = () => D.ui.cl = D.ui.cl || { name: '', owner: 'me', agree: false, consent: '', reference: '', script: 'So I opened the box [laugh] and, hmm, it was empty. [sigh] Of course it was.', err: null, msg: null, jobs: {}, loader: 'turbo' };
  const tip = t => ` data-tip="${esc(t)}"`;
  const base = p => (p || '').split(/[\\/]/).pop();
  const SPEED = { fast: 'faster than real time', ok: 'about real time', slow: 'slower than real time' };
  const DEV = { cuda: 'NVIDIA graphics chip', mps: 'Apple graphics cores', cpu: 'processor' };
  const DEVICES = [['auto', 'Auto'], ['cuda', 'NVIDIA'], ['mps', 'Apple'], ['cpu', 'Processor']];
  const LOADERS = [['turbo', 'Chatterbox Turbo'], ['english', 'Chatterbox'], ['multilingual', 'Chatterbox Multilingual'], ['vc', 'Chatterbox voice converter']];
  const ENGINE = 'voice-engine';
  const st = () => D.cstatus || {};
  const models = () => st().models || [];
  const settings = () => st().settings || {};
  const byId = id => models().find(m => m.id === id);
  const jobOf = id => { const j = U().jobs[id]; if (j && j.state === 'running') return j; const m = id === ENGINE ? st().runtime : byId(id); return m && m.downloading ? { id: m.downloading.job, state: 'running', progress: m.downloading.progress, stage: m.downloading.stage } : null; };

  function busyRow(id) {
    const j = jobOf(id); if (!j) return '';
    return Flux.dlRow(id, j, btn('Cancel', { k: 'g', x: `data-cact="dl-cancel" data-id="${esc(id)}"` + tip('Stops the download. Press Download again later and it carries on from here.') }));
  }

  function computer() {
    const s = st(), h = s.hardware || {}, pk = s.pick || {};
    return `<div class="box col">
      <div class="row" style="gap:6px">${pill(esc(h.os + ' · ' + h.arch))}${pill(h.cores + ' cores')}${h.ram_gb ? pill(F.num(h.ram_gb, 0) + ' GB memory') : ''}${h.gpu_name ? pill(esc(h.gpu_name) + (h.vram_gb ? ' · ' + F.num(h.vram_gb, 0) + ' GB' : '')) : pill('No graphics chip found')}</div>
      <p style="margin:0"${tip('Flux suggests a model from your graphics chip and memory. You can choose any other below. Nothing downloads without asking.')}><b>${pk.model ? esc((byId(pk.model) || {}).name || pk.model) : 'No model fits'}</b>${pk.model ? `, on the ${DEV[pk.device] || pk.device}, ${SPEED[pk.speed] || ''}.` : '.'} ${esc(pk.reason || '')}</p>
      ${cal(s.available ? 'check' : 'info', esc(s.why || ''))}
    </div>`;
  }

  function engine() {
    const r = st().runtime || {}, busy = jobOf(ENGINE);
    const info = r.info ? `Python ${esc(r.info.python || '')} · PyTorch ${esc(r.info.torch || '')} · ${r.info.cuda ? 'NVIDIA graphics chip' : r.info.mps ? 'Apple graphics cores' : 'processor'}` : esc(r.approx_text || '');
    return `<div style="flex-wrap:wrap"${tip(r.note || '')}><span class="act">${ic('chip', 18)}</span><div class="ti"><b>${esc(r.name || 'Voice engine')}${r.installed ? ' · installed' : ''}</b><span style="white-space:normal">${info}</span></div>
      ${!r.supported ? pill('Not for this computer') : busy ? '' : r.installed ? btn('', { k: 'g', i: 'trash', a: 'Delete the voice engine', x: `data-cact="dl-del" data-id="${ENGINE}"` + tip('Removes the voice engine. Downloaded models stay; Download any model to install it again.') }) : btn('Install', { x: `data-cact="dl-ask" data-id="${ENGINE}"` + tip('Installs only the voice engine. Downloading any model installs it too.') })}
      ${busyRow(ENGINE)}</div>`;
  }

  function modelRow(m) {
    const s = settings(), busy = jobOf(m.id), role = m.role === 'convert' ? 'convert' : 'speech';
    const used = role === 'speech' ? s.speech === m.id : s.convert === m.id;
    const fit = m.fits === 'gpu' ? pill('Graphics chip', 'ok') : m.fits === 'cpu' ? pill('Processor, slower') : m.fits === 'no' ? pill('Not for this computer', 'warn') : '';
    const size = m.local ? 'Your folder' : `${esc(m.licence)} · ${esc(m.size_text)}${!m.installed && m.partial ? ' · ' + Math.floor(m.have / m.size * 100) + '% here' : ''}`;
    const actions = busy ? '' : m.installed
      ? `${btn(used ? (role === 'speech' ? 'Speaking' : 'Converting') : (role === 'speech' ? 'Use to speak' : 'Use to convert'), { k: used ? 'p' : '', x: `data-cact="use-model" data-id="${esc(m.id)}" data-role="${role}"${used ? ' aria-pressed="true"' : ''}` + tip(role === 'speech' ? 'Speak scripts with this model.' : 'Turn your own recordings into the voice with this model.') })}
         ${m.local ? btn('', { k: 'g', i: 'x', a: 'Remove ' + esc(m.name), x: `data-cact="local-rm" data-id="${esc(m.id)}"` + tip('Stop using this folder. Its files stay where they are.') }) : btn('', { k: 'g', i: 'trash', a: 'Delete ' + esc(m.name), x: `data-cact="dl-del" data-id="${esc(m.id)}"` + tip('Deletes this model\'s files. Files another downloaded model shares are kept.') })}`
      : m.fits === 'no' ? '' : btn(m.partial ? 'Resume' : 'Download', { k: m.recommended ? 'p' : '', i: 'down', x: `data-cact="dl-ask" data-id="${esc(m.id)}"` + tip('Shows the size, licence and folder first.') });
    return `<div style="flex-wrap:wrap;align-items:flex-start"${tip(m.fits_why || m.note)}><span class="act">${ic(role === 'speech' ? 'voice' : 'swap', 18)}</span>
      <div class="ti" style="min-width:220px;flex:1"><b>${esc(m.name)}</b><span style="white-space:normal">${esc(m.note)}</span>
        <span class="row" style="gap:6px;margin-top:4px">${pill(role === 'speech' ? 'Speaks scripts' : 'Converts recordings')}${m.sounds ? pill('[laugh] and other sounds', 'sig') : ''}${m.languages && m.languages.length > 1 ? pill(m.languages.length + ' languages') : ''}${m.recommended ? pill('Suggested here', 'ok') : ''}${fit}</span></div>
      <span class="mono dim" style="white-space:nowrap">${size}</span>${actions}${busyRow(m.id)}</div>`;
  }

  function library() {
    const u = U();
    return `<div class="col">
      <div class="lst box">${engine()}${models().map(modelRow).join('')}</div>
      <div class="box col"><span class="lab">Use a model folder you already have</span>
        <div class="row" style="gap:10px;align-items:flex-end">${fld('Kind', sel(LOADERS, u.loader, 'data-cin="loader" aria-label="Kind of model"' + tip('Which Chatterbox model the folder holds, for example one you fine-tuned. Flux checks it has the files that kind needs.')))}
          ${btn('Choose folder', { i: 'folder', x: 'data-cact="local-add"' + tip('Adds a folder on this computer. Nothing is copied or downloaded.') })}</div></div>
      <span class="dim" style="font-size:12.5px">Models are kept in <span class="mono">${esc(st().folder || '')}</span>.</span>
    </div>`;
  }

  // Generation settings for the chosen speech model, with the device, language and seed.
  function voiceSettings() {
    const s = settings(), cs = D.csettings || {}, m = byId(s.speech);
    if (!m) return cal('info', 'Download a speech model above to set how it speaks.');
    const vals = (cs.params || {})[m.id] || {};
    const lang = m.languages && m.languages.length > 1 ? fld('Language', sel(m.languages.map(l => [l, esc((m.language_names || {})[l] || l)]), s.language, 'data-cin="language" aria-label="Language"' + tip('The language the script is in.'))) : '';
    const sliders = (m.params || []).map(p => { const v = vals[p.key] != null ? vals[p.key] : p.default; return `<div style="flex:1;min-width:200px">${sl(esc(p.label), (v - p.min) / (p.max - p.min) * 100, F.num(v, 2), { bind: `cl:${m.id}:${p.key}` })}</div>`; }).join('');
    return `<div class="box col">
      <p style="margin:0"><b>${esc(m.name)}</b> <span class="dim">speaks your scripts.</span></p>
      ${fld('Run on', seg(DEVICES.map(d => d[1]), Math.max(0, DEVICES.findIndex(d => d[0] === s.device)), 'data-cseg="device" aria-label="Run on"' + tip('Auto uses the graphics chip when there is one. Processor always works, more slowly.')))}
      ${sliders ? `<div class="row" style="gap:16px">${sliders}</div>` : ''}
      <div class="row" style="gap:10px;align-items:flex-end">${lang}${fld('Seed', inp(String(s.seed != null ? s.seed : 1), 'Seed', 'data-cin="seed" inputmode="numeric" aria-label="Seed"' + tip('The same seed and script give the same take. Change it for another take.')))}
        ${btn('Defaults', { k: 'g', x: `data-cact="params-reset" data-id="${esc(m.id)}"` + tip('Put this model\'s settings back to how it ships.') })}</div>
    </div>`;
  }

  function consentForm() {
    const u = U(), who = u.name.trim() || 'your name', statement = `I am ${who}. I agree that Flux may make a copy of my voice on this computer, and I know anything it says in my voice will be labelled synthetic.`;
    const ready = u.name.trim() && u.agree && u.consent && u.reference;
    return `<div class="box col">
      ${fld('Whose voice', inp(u.name, 'Their name', 'data-cin="name" aria-label="Whose voice"' + tip('The name of the person whose voice this is.')))}
      ${seg(['My own voice', 'Someone else, with their permission'], u.owner === 'me' ? 0 : 1, 'data-cseg="owner" aria-label="Whose voice is it"' + tip('Cloning someone without their permission can be illegal and harmful.'))}
      <div class="col" style="gap:6px"><span class="lab">They read this aloud, and you record it</span><blockquote style="margin:0;padding:10px 14px;border-left:3px solid var(--tx);font-size:17px;font-weight:600">${esc(statement)}</blockquote></div>
      <div class="row">${btn(u.consent ? 'Statement: ' + esc(base(u.consent)) : 'Choose the statement recording', { i: 'mic', x: 'data-cact="pick" data-k="consent"' + tip('A recording of the voice owner reading the statement above, 3 to 60 seconds. Kept with the voice as proof of consent.') })}
        ${btn(u.reference ? 'Voice: ' + esc(base(u.reference)) : 'Choose a voice clip', { i: 'wave', x: 'data-cact="pick" data-k="reference"' + tip('5 to 60 seconds of the voice speaking naturally. Laughs and hmms in it help the copy sound like them.') })}
        ${btn('', { k: 'g', i: 'rec', a: 'Record one now', go: 'record' })}</div>
      ${tg('They agreed to this', u.agree, 'Only tick this if the person whose voice it is said yes.', 'data-cact="agree"' + tip('Flux refuses to make a voice without this, the statement recording and a voice clip.'))}
      <div class="row">${btn('Make the voice', { k: 'p', i: 'check', x: `data-cact="create"${ready ? '' : ' disabled'}` + tip('Saves the voice and its consent on this computer only.') })}</div>
    </div>`;
  }
  function voices() {
    const vs = D.cprofiles || [], u = U();
    if (!vs.length) return cal('info', 'No voices yet. Make one above.');
    return `<div class="lst box">${vs.map(v => `<div><span class="act">${ic('voice', 18)}</span><div class="ti"><b>${esc(v.name)}</b><span>${v.owner === 'me' ? 'Your own voice' : 'With their permission'} · ${F.num(v.reference_seconds, 1)} s clip · consent recorded ${F.ago(v.created * 1000)}</span></div>
      ${btn(u.voice === v.id ? 'Selected' : 'Use', { k: u.voice === v.id ? 'p' : '', x: `data-cact="use" data-id="${esc(v.id)}"` + tip('Speak scripts in this voice.') })}${btn('', { k: 'g', i: 'trash', a: 'Delete ' + esc(v.name), x: `data-cact="del" data-id="${esc(v.id)}"` + tip('Delete this voice, its clip and its consent recording.') })}</div>`).join('')}</div>`;
  }
  // Placeholder chips, one row per kind, so a script can be written by clicking.
  const chips = (label, list) => list.length ? `<div class="row" style="gap:6px;align-items:center"><span class="mono" style="min-width:96px;opacity:.7">${label}</span>${list.map(([t, h]) => `<button type="button" class="chip" data-cact="tag" data-t="${esc(t)}"${tip(h)}>[${esc(t)}]</button>`).join('')}</div>` : '';
  const part = p => p.type === 'sound' ? pill(esc(p.value), 'sig') : p.type === 'hesitation' ? pill(esc(p.value), 'ok') : p.type === 'pause' ? pill(F.num(p.value / 1000, 1) + ' s pause') : esc(p.value);
  function script() {
    const u = U(), parts = u.parts, tgs = D.ctags || {}, s = settings(), sp = byId(s.speech), cv = byId(s.convert);
    const canSpeak = u.voice && sp && sp.installed && (st().runtime || {}).installed, canConvert = u.voice && cv && cv.installed && (st().runtime || {}).installed;
    const run = u.run && u.run.state === 'running' ? u.run : null, pc = run ? Math.round((run.progress || 0) * 100) : 0;
    return `<div class="box col">
      <textarea class="inp" rows="5" aria-label="Script" data-cin="script" style="font-size:17px;font-weight:500"${tip('Write sounds as [laugh], (sighs), *chuckles* or {cough}. Hesitations as [hmm], [um] or plain words, said in the voice. Pauses as [pause] or [pause 2s].')}>${esc(u.script)}</textarea>
      ${chips('Sounds', (tgs.tags || []).map(t => [t.tag, 'Insert [' + t.tag + '] at the cursor: the voice performs it. Also written as ' + t.also.slice(0, 3).join(', ') + '.']))}
      ${chips('Hesitations', (tgs.hesitations || []).map(t => [t.tag, 'Insert [' + t.tag + '] at the cursor: the voice says it, in its own way. Also written as ' + t.also.slice(0, 3).join(', ') + ', or just as a word.']))}
      ${chips('Pauses', (tgs.pauses || []).map(t => [t.tag, 'Insert [' + t.tag + '] at the cursor: ' + F.num(t.ms / 1000, 1) + ' s of silence.' + (t.note ? ' ' + t.note : '')]))}
      ${parts ? `<p style="margin:0;line-height:1.9" data-cparts>${parts.map(part).join(' ')}</p><p class="mono" style="margin:0;opacity:.7" data-csum>${u.sum}</p>` : ''}
      ${sp && !sp.sounds ? cal('info', `${esc(sp.name)} doesn't perform sounds, so [laugh] and the like are left out. Hesitations and pauses still work.`) : ''}
      ${run ? `<div class="row" style="gap:10px;align-items:center" data-dl="run"><div style="flex:1">${prog(pc, true)}</div><span class="mono dim" data-dl-text>${pc}% · ${esc(run.stage || 'Starting')}</span>${btn('Cancel', { k: 'g', x: 'data-cact="run-cancel"' })}</div>` : ''}
      <div class="row">${btn('Check script', { x: 'data-cact="check"' + tip('Shows the words, hesitations, sounds and pauses in order, as the voice will perform them.') })}
        ${btn('Speak in this voice', { k: 'p', i: 'play', x: `data-cact="speak"${canSpeak && !run ? '' : ' disabled'}` + tip(sp ? `Speaks the script in the selected voice with ${sp.name}, labelled synthetic.` : 'Download a speech model first.') })}
        ${btn('Turn my recording into this voice', { i: 'swap', x: `data-cact="convert"${canConvert && !run ? '' : ' disabled'}` + tip(cv && cv.installed ? 'Record yourself saying it, with your own laughs and pauses, and Flux re-voices it, keeping your timing.' : 'Download the voice converter first.') })}</div>
      ${u.err ? cal('warn', esc(u.err), 'warn') : ''}${u.msg ? cal('check', esc(u.msg)) : ''}
    </div>`;
  }
  reg('clone', 'AI', 'Voice cloning', () => wiz(0, 0, `<h1 class="h1">Clone a voice, with consent.</h1>
      <p class="lede">Only your own voice, or someone who said yes on a recording. Everything stays on this computer, and anything spoken is labelled synthetic.</p>
      ${sec('This computer', computer())}
      ${sec('Models', library())}
      ${sec('How it speaks', voiceSettings())}
      ${sec('Make a voice', consentForm())}
      ${sec('Voices', voices())}
      ${sec('Script', script())}
      <div class="between">${btn('Back', { k: 'g', go: 'tts' })}${btn('Text to speech', { go: 'tts' })}</div>`, true));

  // The download dialog: what, how big, which licence, where, before anything is fetched.
  reg('clone-get', 'AI', 'Voice cloning: download', () => {
    const g = U().get || {}, m = g.id === ENGINE ? null : byId(g.id), r = st().runtime || {};
    const title = m ? 'Download ' + esc(m.name) : 'Install the ' + esc((r.name || 'voice engine').toLowerCase());
    return dlg('clone', title, `
      <div class="lst box">${m ? `<div><div class="ti"><b>${esc(m.name)}</b><span style="white-space:normal">${esc(m.licence)} licence · <span class="mono">${esc(m.source)}</span></span></div><span class="mono">${esc(m.size_text)}</span></div>` : ''}
        ${!r.installed ? `<div><div class="ti"><b>${esc(r.name || 'Voice engine')}</b><span style="white-space:normal">${esc(r.note || '')}</span></div><span class="mono">${esc(r.approx_text || '')}</span></div>` : ''}</div>
      ${cal('lock', `<b>Downloads once, then runs on this computer.</b> ${esc(g.text || '')} Every file is checked before it's used, and a stopped download picks up where it left off.`, 'ac')}
      ${g.err ? cal('warn', esc(g.err), 'bad') : ''}
      <span class="dim" style="font-size:12.5px">Only clone voices you have permission to use.</span>`,
      `${btn('Cancel', { k: 'g', go: 'clone' })}${btn('Download', { k: 'p', i: 'down', x: 'data-cact="dl-go"' + (g.text ? '' : ' disabled') })}`);
  });

  async function reload() { F.stale('cprofiles', 'cstatus', 'csettings'); await F.need(['cprofiles', 'cstatus', 'csettings'], true); Flux.rr(); }
  async function watch(id, key, onDone) {
    const u = U();
    let shown = false;
    for (; ;) {
      let j; try { j = await call('job.get', { id, wait_s: .5 }); } catch (e) { u.err = e.message; break; }
      if (key) u.jobs[key] = j; else u.run = j;
      // Draw the row once; after that only its own parts change, on whatever page is open.
      if (!shown) { shown = true; Flux.rr(); } else Flux.dlPatch(key || 'run', j);
      if (j.state !== 'running') { if (j.state === 'failed') u.err = j.error; else if (j.state === 'done' && onDone) onDone(j.result); break; }
    }
    if (key) delete u.jobs[key];
    reload();
  }
  const setS = async p => { const u = U(); u.err = null; try { D.csettings = await call('clone.settings.set', p); if (D.cstatus) D.cstatus.settings = D.csettings.settings; } catch (e) { u.err = e.message; } Flux.rr(); };
  Flux.cloneBind = (parts, pct, end) => {
    const [mid, key] = parts, m = byId(mid), p = m && (m.params || []).find(q => q.key === key); if (!p) return '';
    const v = Math.round((p.min + (p.max - p.min) * pct / 100) / p.step) * p.step;
    if (end) setS({ params: { [mid]: { [key]: +v.toFixed(4) } } });
    return F.num(v, 2);
  };
  Flux.cloneTip = b => { const [, mid, key] = b.split(':'), m = byId(mid), p = m && (m.params || []).find(q => q.key === key); return p ? p.tip : ''; };

  const A = {
    'dl-ask': async el => {
      const u = U(), id = el.dataset.id; u.get = { id };
      // Asking without `confirm` changes nothing and returns what would be downloaded.
      try { const r = await call('clone.model.download', { model: id }); if (r.installed) { reload(); return; } } catch (e) { if (e.code === 'confirm_required') u.get.text = e.message.replace(/ Ask the person first.*$/, ''); else u.get.err = e.message; }
      Flux.render('clone-get');
    },
    'dl-go': async () => {
      const u = U(), id = (u.get || {}).id; u.err = null;
      try { const r = await call('clone.model.download', { model: id, confirm: true }); Flux.render('clone'); if (r.job) watch(r.job, id, () => { u.msg = 'Downloaded and checked.'; }); else reload(); } catch (e) { u.get.err = e.message; Flux.rr(); }
    },
    'dl-cancel': async el => { const j = jobOf(el.dataset.id); if (j) try { await call('job.cancel', { id: j.id }); } catch (e) { U().err = e.message; } },
    'dl-del': async el => { const m = byId(el.dataset.id), what = m ? m.name : 'the voice engine'; if (!confirm(`Delete ${what}? You can download it again later.`)) return; try { const r = await call('clone.model.delete', { model: el.dataset.id }); U().msg = r.freed_text ? `Freed ${r.freed_text}.` : 'Deleted.'; } catch (e) { U().err = e.message; } reload(); },
    'use-model': el => setS({ [el.dataset.role]: el.dataset.id }).then(reload),
    'local-add': async () => { const u = U(); u.err = null; const dir = await F.pickFolder(); if (!dir) return; try { const r = await call('clone.model.add_local', { path: dir, loader: u.loader }); u.msg = 'Added. Choose it below to use it.'; await setS({ [r.role === 'convert' ? 'convert' : 'speech']: r.id }); } catch (e) { u.err = e.message; } reload(); },
    'local-rm': async el => { try { await call('clone.model.remove_local', { id: el.dataset.id }); } catch (e) { U().err = e.message; } reload(); },
    'params-reset': el => { const m = byId(el.dataset.id); if (m) setS({ params: { [m.id]: Object.fromEntries((m.params || []).map(p => [p.key, null])) } }); },
    'run-cancel': async () => { const r = U().run; if (r) try { await call('job.cancel', { id: r.id }); } catch (e) { } },
    pick: async el => { const fs = await F.pickFiles({ multiple: false, kind: 'audio' }); if (fs && fs[0]) { U()[el.dataset.k] = fs[0]; Flux.rr(); } },
    agree: () => { U().agree = !U().agree; Flux.rr(); },
    create: async () => { const u = U(); u.err = null; try { const p = await call('clone.profile.create', { name: u.name.trim(), owner: u.owner, consent: u.agree, consent_audio: u.consent, reference: u.reference }); u.voice = p.id; u.msg = `Saved ${p.name}'s voice with their consent recording.`; u.name = ''; u.agree = false; u.consent = u.reference = ''; } catch (e) { u.err = e.message; } reload(); },
    use: el => { U().voice = el.dataset.id; Flux.rr(); },
    del: async el => { if (!confirm('Delete this voice and its recordings? This can\'t be undone.')) return; try { await call('clone.profile.delete', { id: el.dataset.id }); if (U().voice === el.dataset.id) U().voice = null; } catch (e) { U().err = e.message; } reload(); },
    tag: el => { const ta = document.querySelector('[data-cin="script"]'), t = '[' + el.dataset.t + ']'; const u = U(), at = ta ? ta.selectionStart : u.script.length; u.script = u.script.slice(0, at) + (at && u.script[at - 1] !== ' ' ? ' ' : '') + t + ' ' + u.script.slice(at); u.parts = null; Flux.rr(); },
    check: async () => { const u = U(); u.err = u.msg = null; try { const r = await call('clone.script', { text: u.script }); u.parts = r.parts; u.sum = `${r.sounds} sound${r.sounds === 1 ? '' : 's'} · ${r.hesitations} hesitation${r.hesitations === 1 ? '' : 's'} · ${F.num(r.silence_ms / 1000, 1)} s of pauses · ${r.segments.length} piece${r.segments.length === 1 ? '' : 's'}`; } catch (e) { u.parts = null; u.err = e.message; } Flux.rr(); },
    speak: async () => { const u = U(); u.err = u.msg = null; try { const r = await call('clone.speak', { voice: u.voice, text: u.script }); watch(r.job, null, res => { u.msg = `Done: ${F.num(res.seconds, 1)} s${res.clip != null ? ' on the Generated speech track' : ', saved to ' + base(res.path)}. Labelled synthetic.${res.sounds_left_out ? ` ${res.sounds_left_out} sound${res.sounds_left_out === 1 ? '' : 's'} left out.` : ''}`; }); } catch (e) { u.err = e.message; } Flux.rr(); },
    convert: async () => { const u = U(); u.err = u.msg = null; const fs = await F.pickFiles({ multiple: false, kind: 'audio' }); if (!fs || !fs[0]) return; try { const r = await call('clone.convert', { voice: u.voice, path: fs[0] }); watch(r.job, null, res => { u.msg = `Done: ${F.num(res.seconds, 1)} s${res.clip != null ? ' on the Generated speech track' : ', saved to ' + base(res.path)}. Labelled synthetic.`; }); } catch (e) { u.err = e.message; } Flux.rr(); },
  };
  document.addEventListener('click', e => {
    if (e.button) return;
    const sb = e.target.closest('.seg[data-cseg] button');
    if (sb) {
      const g = sb.parentElement, i = [...g.querySelectorAll('button')].indexOf(sb);
      if (g.dataset.cseg === 'owner') U().owner = i ? 'someone-else' : 'me';
      if (g.dataset.cseg === 'device') setS({ device: DEVICES[i][0] });
      return;
    }
    const a = e.target.closest('[data-cact]'); if (!a || a.disabled) return; e.preventDefault(); const fn = A[a.dataset.cact]; if (fn) fn(a);
  });
  let cinT = 0;
  document.addEventListener('input', e => {
    const k = e.target.dataset && e.target.dataset.cin; if (!k) return;
    if (k === 'seed') { clearTimeout(cinT); const n = parseInt(e.target.value, 10); if (n >= 0) cinT = setTimeout(() => setS({ seed: n }), 500); return; }
    U()[k] = e.target.value; if (k === 'script') U().parts = null; if (k === 'name') { clearTimeout(cinT); cinT = setTimeout(Flux.rr, 400); }
  });
  document.addEventListener('change', e => { const k = e.target.dataset && e.target.dataset.cin; if (k === 'language') setS({ language: e.target.value }); if (k === 'loader') U().loader = e.target.value; });
})();
