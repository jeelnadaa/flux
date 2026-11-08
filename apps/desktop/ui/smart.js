/* Smart tools settings: choose what runs stems, transcripts and text to speech. Ported from the prototype's
   Settings > AI models screen (design/screens-c.js, prefs-models): rows of model, size and state with Download or
   Remove, and a callout about this computer. Online engines sit in the same rows; turning one on goes through a
   dialog that says what is sent and where, and asks for a key only when the service needs one. Keys go straight
   to the system keychain and are never shown again. */
(function () {
  const { reg, dlg, btn, pill, fld, inp, sel, sec, cal, prog, ic } = Flux, F = FD, D = FD.D, esc = F.esc;
  const call = (m, p) => F.call(m, p);
  const U = () => D.ui.sm = D.ui.sm || { jobs: {}, err: null, msg: null };
  const tip = t => ` data-tip="${esc(t)}"`;
  const eng = () => D.aeng || { roles: [], providers: [] };
  const roleOf = r => eng().roles.find(x => x.role === r) || { engines: [] };
  const byId = id => { for (const r of eng().roles) { const e = r.engines.find(x => x.id === id); if (e) return Object.assign({ role: r.role }, e); } return null; };
  const TIER = { light: 'Light', balanced: 'Balanced', best: 'Best', extra: 'Specialist', cloud: 'Online' };
  const LANGS = [['auto', 'Detect the language'], ['en', 'English'], ['es', 'Spanish'], ['fr', 'French'], ['de', 'German'], ['it', 'Italian'], ['pt', 'Portuguese'], ['nl', 'Dutch'], ['pl', 'Polish'], ['ru', 'Russian'], ['tr', 'Turkish'], ['ar', 'Arabic'], ['hi', 'Hindi'], ['ja', 'Japanese'], ['ko', 'Korean'], ['zh', 'Chinese']];
  const keyText = e => !e.key_needed ? 'Free, no key' : e.key === 'missing' ? 'Needs a key' : e.key === 'keychain' ? 'Key in the system keychain' : 'Key from ' + e.key.replace('env:', '');
  const jobOf = id => { const j = U().jobs[id]; if (j && j.state === 'running') return j; const e = byId(id); return e && e.downloading ? { id: e.downloading.job, state: 'running', progress: e.downloading.progress, stage: e.downloading.stage } : null; };

  function busyRow(id) {
    const j = jobOf(id); if (!j) return '';
    const pc = Math.round((j.progress || 0) * 100);
    return `<div class="row" style="gap:10px;align-items:center;width:100%"><div style="flex:1;min-width:120px">${prog(pc, true)}</div><span class="mono dim" style="white-space:nowrap">${pc}% · ${esc(j.stage || 'Starting')}</span>${btn('Cancel', { k: 'g', x: `data-sact="dl-cancel" data-id="${esc(id)}"` + tip('Stops the download. Download again later and it carries on from here.') })}</div>`;
  }

  function row(role, e) {
    const busy = jobOf(e.id), cloud = e.kind === 'cloud';
    const meta = e.kind === 'builtin' ? 'Built in, no download' : cloud ? `${esc(e.provider_name)} · ${keyText(e)}` : `${esc(e.licence)} · ${esc(e.size_text)}${e.installed ? ', verified' : e.partial ? ' · ' + Math.floor(e.have / e.size * 100) + '% here' : ''}`;
    const usable = cloud ? e.allowed && e.installed : e.installed;
    const use = e.chosen
      ? btn('In use', { k: 'p', x: `data-sact="use" data-role="${role}" data-id="${esc(e.id)}" aria-pressed="true"` + tip('This runs the tool now.') })
      : usable ? btn('Use', { x: `data-sact="use" data-role="${role}" data-id="${esc(e.id)}" aria-pressed="false"` + tip('Run the tool with this from now on.') }) : '';
    let act = '';
    if (busy) act = '';
    else if (cloud) act = usable ? use : btn('Turn on', { i: 'globe', x: `data-sact="cloud-ask" data-role="${role}" data-id="${esc(e.id)}"` + tip('Shows what would be sent and where first.' + (e.key_needed ? ' Asks for a key.' : '')) });
    else if (e.kind === 'builtin') act = e.installed ? use : pill('Not on this computer', 'warn');
    else if (!e.installed) act = e.fits === 'no' ? pill('Not for this computer', 'warn') : btn(e.partial ? 'Resume' : 'Download', { k: e.suggested ? 'p' : '', i: 'exp', x: `data-sact="dl-ask" data-id="${esc(e.id)}"` + tip('Shows the size, licence and folder first.') });
    else act = `${use}${btn('', { k: 'g', i: 'trash', a: 'Remove ' + esc(e.name), x: `data-sact="dl-del" data-id="${esc(e.id)}"` + tip('Deletes this model\'s files. Files another model shares are kept.') })}`;
    const pills = `${pill(TIER[e.tier] || esc(e.tier || ''), e.tier === 'cloud' ? 'sig' : '')}${e.suggested ? pill('Suggested here', 'ok') : ''}${e.free ? pill('Free') : ''}${cloud && role === 'transcript' && !e.words ? pill('Text only') : ''}`;
    return `<div style="flex-wrap:wrap;align-items:flex-start"${tip(e.fits_why || e.note || '')}><span class="act">${ic(cloud ? 'globe' : e.kind === 'builtin' ? 'chip' : 'disk', 18)}</span>
      <div class="ti" style="min-width:220px;flex:1"><b>${esc(e.name)}</b><span style="white-space:normal">${meta}</span><span style="white-space:normal">${esc(e.note || '')}</span>
        <span class="row" style="gap:6px;margin-top:4px">${pills}</span></div>${act}${busyRow(e.id)}</div>`;
  }

  function options(r) {
    const e = r.engines.find(x => x.chosen) || {};
    if (r.role === 'tts' && e.voices && e.voices.length) {
      const v = (r.options || {}).voice;
      return `<div class="row" style="gap:10px;align-items:flex-end">${fld('Voice', sel(e.voices.map(x => [x.id, esc(x.name)]), e.voices.some(x => x.id === v) ? v : e.voices[0].id, 'data-sin="voice" aria-label="Voice"' + tip('The voice ' + (e.name || '') + ' reads in.')))}</div>`;
    }
    if (r.role === 'transcript') {
      return `<div class="row" style="gap:10px;align-items:flex-end">${fld('Language', sel(LANGS, (r.options || {}).language || 'auto', 'data-sin="language" aria-label="Language"' + tip('The language people speak in the recording. Detecting it works for most recordings.')))}</div>`;
    }
    return '';
  }

  function providers() {
    const ps = eng().providers.filter(p => p.key_needed || p.key !== 'missing');
    return `<div class="lst box">${ps.map(p => `<div style="flex-wrap:wrap"${tip('Keys stay in the system keychain or your environment. Flux never shows them again or saves them in its files.')}><span class="act">${ic('lock', 18)}</span>
      <div class="ti" style="min-width:200px;flex:1"><b>${esc(p.name)}</b><span style="white-space:normal">${p.key === 'missing' ? 'No key' : p.key === 'keychain' ? 'Key in the system keychain' : 'Key from the environment variable ' + esc(p.key.replace('env:', ''))}${p.base_url ? ` · <span class="mono">${esc(p.base_url)}</span>` : ''}</span></div>
      ${p.key === 'keychain' ? btn('Forget key', { k: 'g', x: `data-sact="key-del" data-p="${esc(p.id)}"` + tip('Removes the key from the system keychain.') }) : p.key === 'missing' ? btn('Add key', { x: `data-sact="key-ask" data-p="${esc(p.id)}"` + tip('Keeps a key for ' + p.name + ' in the system keychain.') }) : ''}</div>`).join('')}</div>`;
  }

  function used() {
    let b = 0; for (const r of eng().roles) for (const e of r.engines) if (e.kind === 'local' && e.installed) b += e.size || 0;
    return b ? F.bytes(b) : '0 MB';
  }

  function body() {
    const a = eng(), h = a.hardware || {}, u = U();
    if (!D.aeng) return cal('info', 'Loading the engines…');
    const chip = h.gpu_name ? `${esc(h.gpu_name)}, ${F.num(h.vram_gb || 0, 0)} GB of graphics memory` : 'no graphics chip, so models run on the processor';
    return `<div class="between"><span class="dim">Downloaded models work offline. Online ones only run when you turn them on.</span><b class="mono">${used()} used</b></div>
      ${cal('chip', `This computer: ${chip}, ${F.num(h.ram_gb || 0, 0)} GB of memory. Suggestions follow it.`)}
      ${u.err ? cal('warn', esc(u.err), 'warn') : ''}${u.msg ? cal('check', esc(u.msg)) : ''}
      ${a.roles.map(r => sec(esc(r.name), `<p class="dim" style="margin:0 0 8px">${esc(r.why || '')}</p><div class="lst box">${r.engines.map(e => row(r.role, e)).join('')}</div>${options(r)}`)).join('')}
      ${sec('Online services', providers())}
      <span class="dim" style="font-size:12.5px">Models are kept in <span class="mono">${esc(a.folder || '')}</span>.</span>`;
  }

  reg('prefs-models', 'Settings', 'Smart tools', () => Flux.pref('models', 'Smart tools', body()));

  // Download: what, how big, which licence, where, before anything is fetched.
  reg('smart-get', 'Settings', 'Smart tools: download', () => {
    const g = U().get || {}, e = byId(g.id) || {};
    return dlg('prefs-models', 'Download ' + esc(e.name || ''), `
      <div class="lst box"><div><div class="ti"><b>${esc(e.name || '')}</b><span style="white-space:normal">${esc(e.licence || '')} licence · ${esc(e.note || '')}</span></div><span class="mono">${esc(e.size_text || '')}</span></div></div>
      ${cal('lock', `<b>Downloads once, then runs on this computer.</b> ${esc(g.text || '')} Every file is checked before it's used, and a stopped download picks up where it left off.`, 'ac')}
      ${g.err ? cal('warn', esc(g.err), 'bad') : ''}`,
      `${btn('Cancel', { k: 'g', go: 'prefs-models' })}${btn('Download', { k: 'p', i: 'exp', x: 'data-sact="dl-go"' + (g.text ? '' : ' disabled') + tip('Starts the download. You can keep working.') })}`);
  });

  // Turning on an online engine: what leaves the computer, where to, and a key if it needs one.
  reg('smart-cloud', 'Settings', 'Smart tools: online engine', () => {
    const g = U().cloud || {}, e = byId(g.id) || {}, needKey = e.key_needed && e.key === 'missing';
    return dlg('prefs-models', 'Turn on ' + esc(e.name || ''), `
      <div class="lst box"><div><span class="act">${ic('globe', 18)}</span><div class="ti"><b>${esc(e.provider_name || '')}</b><span style="white-space:normal">${esc(e.note || '')}</span></div>${e.free ? pill('Free') : ''}</div></div>
      ${cal('globe', `<b>This runs online.</b> ${esc(e.sends || '')} Only when you use this tool, never in the background. Their terms and privacy policy apply.`, 'warn')}
      ${needKey ? `${fld('Key', inp('', 'Paste the key', 'type="password" autocomplete="off" spellcheck="false" data-sin="key" aria-label="' + esc((e.provider_name || '') + ' key') + '"' + tip('Kept in the system keychain, never in Flux\'s files, and never shown again.')))}
        <span class="dim" style="font-size:12.5px">Get one at <span class="mono">${esc(e.keys_at || '')}</span>, or set <span class="mono">${esc((e.key_env || []).slice(-1)[0] || '')}</span> in your environment.</span>` : ''}
      ${g.err ? cal('warn', esc(g.err), 'bad') : ''}`,
      `${btn('Cancel', { k: 'g', go: 'prefs-models' })}${btn('Turn on and use', { k: 'p', i: 'check', x: 'data-sact="cloud-go"' + tip('Saves your yes for this engine and uses it for ' + esc((roleOf(g.role).name || '').toLowerCase()) + '.') })}`);
  });

  reg('smart-key', 'Settings', 'Smart tools: key', () => {
    const g = U().key || {}, p = eng().providers.find(x => x.id === g.p) || {};
    return dlg('prefs-models', 'Add a key for ' + esc(p.name || ''), `
      ${fld('Key', inp('', 'Paste the key', 'type="password" autocomplete="off" spellcheck="false" data-sin="key" aria-label="' + esc((p.name || '') + ' key') + '"' + tip('Kept in the system keychain, never in Flux\'s files, and never shown again.')))}
      <span class="dim" style="font-size:12.5px">Get one at <span class="mono">${esc(p.keys_at || '')}</span>, or set <span class="mono">${esc((p.key_env || []).slice(-1)[0] || '')}</span> in your environment instead.</span>
      ${g.err ? cal('warn', esc(g.err), 'bad') : ''}`,
      `${btn('Cancel', { k: 'g', go: 'prefs-models' })}${btn('Save key', { k: 'p', i: 'lock', x: 'data-sact="key-go"' + tip('Saves it in the system keychain.') })}`);
  });

  // ---------------------------------------------------------------- transcripts (prototype: design/screens-c.js script, fillers)
  const { pr, seg, tg, chip } = Flux;
  const T = () => D.trans || null;
  const trEngine = () => roleOf('transcript').engines.find(e => e.chosen) || null;
  const trReady = e => e && (e.kind === 'cloud' ? e.allowed && e.installed : e.installed);
  const at = s => F.tc(s / ((T() && T().sample_rate) || F.sr() || 48000));
  const FKIND = { filler: 'Um, uh', repeat: 'Stutter' };
  const hasAudio = () => !!(D.project && (D.project.length || (D.project.project && D.project.project.length)));
  function makeBar() {
    const e = trEngine(), u = U(), j = u.tjob && u.tjob.state === 'running' ? u.tjob : null;
    if (j) { const pc = Math.round((j.progress || 0) * 100); return `<div class="row" style="gap:10px;align-items:center"><div style="flex:1">${prog(pc, true)}</div><span class="mono dim">${pc}% · ${esc(j.stage || 'Starting')}</span>${btn('Cancel', { k: 'g', x: 'data-sact="tr-cancel"' + tip('Stops making the transcript.') })}</div>`; }
    if (!trReady(e)) return cal('info', 'Choose a transcript engine first: download Whisper, or turn on an online one. <a class="act" href="#prefs-models" data-go="prefs-models">Smart tools</a>');
    return `<div class="row">${btn(T() ? 'Update the transcript' : 'Make a transcript', { k: T() ? '' : 'p', i: 'text', x: 'data-sact="tr-make"' + (hasAudio() ? '' : ' disabled') + tip(`Transcribes the project with ${e.name}${e.kind === 'cloud' ? ', online' : ', on this computer'}.`) })}</div>`;
  }
  function wordsOf(s0) {
    const t = T(), fl = new Set(((D.tfill && D.tfill.fillers) || []).map(f => f.at)), sel = U().word;
    return (t.words || []).filter(w => w.at >= s0.at && w.at < s0.at + Math.max(1, s0.len)).map(w => `<span class="wd ${fl.has(w.at) ? 'fl' : ''} ${w.p != null && w.p < 0.5 ? 'low' : ''} ${sel && sel.at === w.at ? 'cur' : ''}" tabindex="0" role="button" data-sact="tr-word" data-at="${w.at}" data-len="${w.len}" data-w="${esc(w.word)}" data-tip="${esc(at(w.at))}">${esc(w.word)}</span>`).join(' ');
  }
  const kindOf = id => (byId(id) || {}).kind;
  reg('script', 'AI', 'Transcript editor', () => {
    const t = T(), e = trEngine(), u = U(), nf = (D.tfill && D.tfill.count) || 0, w = u.word, sr = (t && t.sample_rate) || 48000;
    const body = !t ? `<div class="pg in"><h1 class="h1">Edit audio by editing text.</h1><p class="lede">Make a transcript, then click any word to hear it there or cut it from the audio.</p>${makeBar()}${u.terr ? cal('warn', esc(u.terr), 'warn') : ''}</div>`
      : `<div class="in" style="padding:clamp(16px,3cqw,34px);display:flex;flex-direction:column;gap:24px;flex:1">${t.stale ? cal('warn', 'The project changed since this transcript, so some words may be in the wrong place. Update it to match.', 'warn') : ''}${makeBar()}${u.terr ? cal('warn', esc(u.terr), 'warn') : ''}${u.tmsg ? cal('check', esc(u.tmsg)) : ''}
        ${(t.segments || []).map(s => `<div><div class="spk"><span class="dot" style="color:var(--t4)"></span><span class="mono faint">${at(s.at)}</span></div><p class="script">${(t.words || []).length ? wordsOf(s) : esc(s.text || '')}</p></div>`).join('')}</div>`;
    const lowN = t ? (t.words || []).filter(x => x.p != null && x.p < 0.5).length : 0;
    const used = t ? t.engine : e && e.id, online = kindOf(used) === 'cloud';
    return pr('script', `<div class="split"><div class="a"><div class="tlbar">${seg(['Edit', 'Review fillers'], 0, 'data-segact="tr-tab" aria-label="Transcript view"')}<span class="sp" style="flex:1"></span>${t && (t.words || []).length ? btn('Fillers: ' + nf, { k: 'lbl', go: 'fillers' }) : ''}</div>${body}</div>
      <aside class="b in"><h3 class="h3">Selected text</h3>${w ? `<div class="box mono dim" style="padding:9px">"${esc(w.w)}" · ${at(w.at)} · ${F.num(w.len / sr, 2)} s</div><div class="row">${btn('Delete', { k: 'p', i: 'trash', x: 'data-sact="tr-del"' + (t.stale ? ' disabled' : '') + tip('Cuts this word from the audio and closes the gap. Undo brings it back.') })}${btn('Listen', { i: 'play', x: 'data-sact="tr-play"' + tip('Plays from just before this word.') })}</div>` : `<div class="box mono dim" style="padding:9px">${t ? 'Click a word' : 'No transcript yet'}</div>`}
      ${t && (t.words || []).length ? sec('Accuracy', `<div class="between"><span>Low-confidence words</span>${pill(String(lowN), lowN ? 'warn' : 'ok')}</div><span class="dim" style="font-size:12.5px">Underlined with dots.</span>`) : ''}
      ${sec('Transcribed with', `<div class="between"><span>${esc((byId(used) || {}).name || used || 'Nothing chosen')}${t && t.language ? ', ' + esc(String(t.language)) : ''}</span>${pill(online ? 'Online' : 'On device', online ? 'warn' : 'ok')}</div><a class="dim" href="#prefs-models" data-go="prefs-models">Change engine</a>`)}</aside></div>`);
  });
  reg('fillers', 'AI', 'Filler word review', () => {
    const t = T(), fs = (D.tfill && D.tfill.fillers) || [], u = U(), off = u.flOff || {}, kind = u.flKind || '';
    const shown = fs.filter(f => !kind || f.kind === kind), sr = (t && t.sample_rate) || 48000;
    const saved = fs.filter(f => !off[f.at]).reduce((a, f) => a + f.len, 0) / sr;
    const words = (t && t.words) || [];
    const ctx = f => { const i = words.findIndex(w => w.at === f.at); return words.slice(Math.max(0, i - 3), i).map(w => esc(w.word)).join(' ') + ` <b class="sig">${esc(f.word)}</b> ` + words.slice(i + 1, i + 4).map(w => esc(w.word)).join(' '); };
    const n = k => fs.filter(f => f.kind === k).length;
    const none = !t ? cal('info', 'Make a transcript first. <a class="act" href="#script" data-go="script">Transcript</a>') : !words.length ? cal('info', 'This transcript has no word timing (the free online engine gives text only), so fillers can\'t be found. Choose another transcript engine in <a class="act" href="#prefs-models" data-go="prefs-models">Smart tools</a>.') : '';
    const list = fs.length ? `<div class="row">${chip('All ' + fs.length, !kind, null, 'data-sact="fl-kind" data-k=""' + tip('Show every filler.'))}${chip('Um, uh ' + n('filler'), kind === 'filler', null, 'data-sact="fl-kind" data-k="filler"' + tip('Show only um, uh and similar.'))}${chip('Stutters ' + n('repeat'), kind === 'repeat', null, 'data-sact="fl-kind" data-k="repeat"' + tip('Show only repeated words.'))}</div>
      <div class="lst box">${shown.map(f => `<div>${btn('', { i: 'play', k: 'g', a: 'Play in context', x: `data-sact="fl-play" data-at="${f.at}"` + tip('Plays from a second before it.') })}<div class="ti"><span style="font-size:15px;color:var(--tx);white-space:normal">${ctx(f)}</span><span class="mono">${at(f.at)} · ${FKIND[f.kind] || esc(f.kind)}</span></div><div style="width:64px;flex:none">${tg('', !off[f.at], '', `data-sact="fl-toggle" data-at="${f.at}" data-label="Remove this one"` + tip('On: removed with the others. Off: kept.'))}</div></div>`).join('')}</div>` : t && words.length ? cal('check', 'No fillers or repeated words found.') : '';
    return pr('script', `<div class="pg in"><div class="pgh"><div><h1 class="h1">${fs.length} filler${fs.length === 1 ? '' : 's'}, ${F.tc(saved).replace(/^00:/, '')} saved.</h1><p class="lede">Listen to any of them. Switch off the ones that carry meaning, then remove the rest.</p></div><div class="row">${btn('Back to script', { go: 'script' })}${btn('Remove all selected', { k: 'p', x: 'data-sact="fl-remove"' + (fs.some(f => !off[f.at]) && !(t && t.stale) ? '' : ' disabled') + tip('Cuts the selected fillers from the audio and closes the gaps. Undo brings them back.') })}</div></div>
      ${none}${t && t.stale ? cal('warn', 'The project changed since the transcript. Update it before removing fillers.', 'warn') : ''}${u.terr ? cal('warn', esc(u.terr), 'warn') : ''}${u.tmsg ? cal('check', esc(u.tmsg)) : ''}${list}</div>`);
  });

  async function retrans() { F.stale('trans', 'tfill', 'project'); await F.need(['trans', 'tfill', 'project'], true); Flux.rr(); }
  async function cutRanges(list) {
    // From the end backwards, so earlier positions stay where they are.
    const sorted = list.slice().sort((a, b) => b.at - a.at);
    for (const r of sorted) await call('edit.cut_range', { start: r.at, end: r.at + r.len, ripple: true });
    return sorted.length;
  }
  const TA = {
    'tr-make': async () => {
      const u = U(); u.terr = u.tmsg = null;
      try {
        const r = await call('transcript.make', {}); u.tjob = { id: r.job, state: 'running', progress: 0 }; Flux.rr();
        for (; ;) { const j = await call('job.get', { id: r.job, wait_s: .5 }); u.tjob = j; Flux.rr(); if (j.state !== 'running') { if (j.state === 'failed') u.terr = j.error; break; } }
      } catch (e) { u.terr = e.message; }
      u.tjob = null; u.word = null; retrans();
    },
    'tr-cancel': async () => { const j = U().tjob; if (j) try { await call('job.cancel', { id: j.id }); } catch (e) { } },
    'tr-word': el => { U().word = { at: +el.dataset.at, len: +el.dataset.len, w: el.dataset.w }; Flux.rr(); },
    'tr-play': async () => { const w = U().word; if (!w) return; try { await call('transport.seek', { pos: Math.max(0, w.at - Math.round(((T() || {}).sample_rate || 48000) * 0.5)) }); await call('transport.play', {}); } catch (e) { U().terr = e.message; Flux.rr(); } },
    'tr-del': async () => { const u = U(), w = u.word; if (!w) return; u.terr = u.tmsg = null; try { await cutRanges([w]); u.tmsg = `Cut "${w.w}". Undo brings it back.`; u.word = null; } catch (e) { u.terr = e.message; } retrans(); },
    'fl-kind': el => { U().flKind = el.dataset.k; Flux.rr(); },
    'fl-toggle': el => { const u = U(); u.flOff = u.flOff || {}; u.flOff[el.dataset.at] = !u.flOff[el.dataset.at]; Flux.rr(); },
    'fl-play': async el => { try { await call('transport.seek', { pos: Math.max(0, +el.dataset.at - ((T() || {}).sample_rate || 48000)) }); await call('transport.play', {}); } catch (e) { U().terr = e.message; Flux.rr(); } },
    'fl-remove': async () => { const u = U(), off = u.flOff || {}, fs = ((D.tfill && D.tfill.fillers) || []).filter(f => !off[f.at]); u.terr = u.tmsg = null; try { const k = await cutRanges(fs); u.tmsg = `Removed ${k} filler${k === 1 ? '' : 's'}. Undo brings them back; update the transcript to keep editing.`; u.flOff = {}; } catch (e) { u.terr = e.message; } retrans(); },
  };
  document.addEventListener('click', e => { const b = e.target.closest('.seg[data-segact="tr-tab"] button'); if (!b) return; const i = [...b.parentElement.querySelectorAll('button')].indexOf(b); if (i === 1) Flux.render('fillers'); });
  document.addEventListener('keydown', e => { if ((e.key === 'Enter' || e.key === ' ') && e.target.dataset && e.target.dataset.sact === 'tr-word') { e.preventDefault(); e.target.click(); } });

  async function reload() { F.stale('aeng', 'ai'); await F.need(['aeng', 'ai'], true); Flux.rr(); }
  async function watch(id, key) {
    const u = U();
    for (; ;) {
      let j; try { j = await call('job.get', { id, wait_s: .5 }); } catch (e) { u.err = e.message; break; }
      u.jobs[key] = j; Flux.rr();
      if (j.state !== 'running') { if (j.state === 'failed') u.err = j.error; else if (j.state === 'done') u.msg = 'Downloaded and checked.'; break; }
    }
    delete u.jobs[key];
    reload();
  }
  const keyVal = () => { const el = document.querySelector('[data-sin="key"]'); return el ? el.value.trim() : ''; };

  const A = {
    use: async el => { const u = U(); u.err = u.msg = null; try { const r = await call('ai.engine.set', { role: el.dataset.role, engine: el.dataset.id }); const x = r.roles[0] || {}; u.msg = `${(byIdIn(r, el.dataset.id) || {}).name || 'It'} now runs ${(x.name || '').toLowerCase()}.`; } catch (e) { u.err = e.message; } reload(); },
    'dl-ask': async el => {
      const u = U(), id = el.dataset.id; u.get = { id };
      try { const r = await call('ai.model.download', { model: id }); if (r.installed) { reload(); return; } } catch (e) { if (e.code === 'confirm_required') u.get.text = e.message.replace(/ Ask the person first.*$/, ''); else u.get.err = e.message; }
      Flux.render('smart-get');
    },
    'dl-go': async () => {
      const u = U(), id = (u.get || {}).id; u.err = u.msg = null;
      try { const r = await call('ai.model.download', { model: id, confirm: true }); Flux.render('prefs-models'); if (r.job) watch(r.job, id); else reload(); } catch (e) { u.get.err = e.message; Flux.rr(); }
    },
    'dl-cancel': async el => { const j = jobOf(el.dataset.id); if (j) try { await call('job.cancel', { id: j.id }); } catch (e) { U().err = e.message; } },
    'dl-del': async el => { const e = byId(el.dataset.id); if (!confirm(`Remove ${e ? e.name : 'this model'}? You can download it again later.`)) return; try { const r = await call('ai.model.delete', { model: el.dataset.id }); U().msg = r.freed_text ? `Freed ${r.freed_text}.` : 'Removed.'; } catch (e2) { U().err = e2.message; } reload(); },
    'cloud-ask': el => { U().cloud = { id: el.dataset.id, role: el.dataset.role }; Flux.render('smart-cloud'); },
    'cloud-go': async () => {
      const u = U(), g = u.cloud || {}, e = byId(g.id) || {}, k = keyVal(); g.err = null; u.err = u.msg = null;
      try {
        if (e.key_needed && e.key === 'missing') { if (!k) { g.err = 'Paste the key first, or set it in your environment.'; Flux.rr(); return; } await call('ai.cloud.key.set', { provider: e.provider, key: k }); }
        await call('ai.engine.set', { role: g.role, engine: g.id, allow_cloud: true });
        u.msg = `${e.name} is on. ${e.sends}`; u.cloud = null; Flux.render('prefs-models'); reload();
      } catch (err) { g.err = err.message; Flux.rr(); }
    },
    'key-ask': el => { U().key = { p: el.dataset.p }; Flux.render('smart-key'); },
    'key-go': async () => { const u = U(), g = u.key || {}, k = keyVal(); g.err = null; if (!k) { g.err = 'Paste the key first.'; Flux.rr(); return; } try { await call('ai.cloud.key.set', { provider: g.p, key: k }); u.msg = 'Key saved in the system keychain.'; u.key = null; Flux.render('prefs-models'); reload(); } catch (e) { g.err = e.message; Flux.rr(); } },
    'key-del': async el => { if (!confirm('Forget this key? Online engines that need it stop working until you add it again.')) return; try { await call('ai.cloud.key.delete', { provider: el.dataset.p }); U().msg = 'Key removed from the system keychain.'; } catch (e) { U().err = e.message; } reload(); },
  };
  Object.assign(A, TA);
  const byIdIn = (r, id) => { for (const x of r.roles || []) { const e = x.engines.find(y => y.id === id); if (e) return e; } return null; };
  document.addEventListener('click', e => {
    if (e.button) return;
    const a = e.target.closest('[data-sact]'); if (!a || a.disabled) return; e.preventDefault(); const fn = A[a.dataset.sact]; if (fn) fn(a);
  });
  document.addEventListener('change', async e => {
    const k = e.target.dataset && e.target.dataset.sin; if (k !== 'voice' && k !== 'language') return;
    const u = U(); u.err = u.msg = null;
    try { await call('ai.engine.set', { role: k === 'voice' ? 'tts' : 'transcript', [k]: e.target.value }); } catch (err) { u.err = err.message; }
    reload();
  });
  document.addEventListener('keydown', e => { if (e.key === 'Enter' && e.target.dataset && e.target.dataset.sin === 'key') { e.preventDefault(); const b = document.querySelector('[data-sact="cloud-go"],[data-sact="key-go"]'); if (b) b.click(); } });
})();
