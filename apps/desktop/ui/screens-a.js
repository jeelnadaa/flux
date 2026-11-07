/* First run, home, editing workspaces. Ported from design/screens-a.js with real data. */
(function () {
  const { fxc, ic, v, vz, btn, seg, sl, tg, chip, pill, fld, inp, sel, sec, tile, cal, prog, pr, pg, wiz, dlg, reg } = Flux;
  const F = FD, D = FD.D, esc = FD.esc;
  const P = () => D.project;

  /* ---------- First run ---------- */
  reg('boot', 'First run', 'Checking your computer', () => wiz(0, 0, `
    <h1 class="h1">Checking what this computer can do.</h1>
    <div class="lst box">
      ${(D.caps || []).map(r => `<div><span class="${r.state === 'ok' ? 'ok' : 'act'}">${ic(r.state === 'ok' ? 'check' : 'warn', 18)}</span><div class="ti"><b>${esc(r.label)}</b><span style="white-space:normal">${esc(r.detail)}</span></div>${pill(esc(r.value), r.state === 'ok' ? 'ok' : 'warn')}</div>`).join('')}
    </div>
    <div class="between"><span class="dim">You can see this again in Settings, under Audio.</span>${btn('Continue', { k: 'p', lg: 1, go: (D.caps || []).some(r => r.id === 'engine' && r.state !== 'ok') ? 'unsupported' : 'welcome' })}</div>`));

  reg('unsupported', 'First run', 'No sound output', () => wiz(0, 0, `
    <h1 class="h1">Flux can't find a sound output.</h1>
    <p class="lede">You can still edit, analyze and export. Your files are safe, because nothing has been changed or sent anywhere.</p>
    ${cal('warn', `<b>Missing: an output device.</b> ${esc(((D.caps || []).find(r => r.id === 'engine') || {}).detail || 'Plug in headphones or speakers.')}`, 'warn')}
    <div class="g3">${tile('head', 'Plug in headphones', 'Flux picks them up when you press Play', '')}${tile('cog', 'Check sound settings', 'Make sure an output is turned on', '')}${tile('wave', 'Edit without sound', 'Everything else works, including export', '')}</div>
    <div class="row">${btn('Continue without sound', { go: 'welcome' })}${btn('Check again', { k: 'g', x: 'data-act="recheck"' })}</div>`));

  reg('welcome', 'First run', 'Welcome and mode', () => wiz(1, 4, `
    <h1 class="bigh">Hello.</h1>
    <p class="lede">How do you want to work? You can switch any time. Nothing you make is lost when you do.</p>
    <div class="g2">
      <a class="tile ${D.settings && D.settings.mode === 'simple' ? 'on' : ''}" href="#hpcheck" data-go="hpcheck" data-act="set-mode" data-mode="simple"><div class="thumb" style="height:96px;padding:14px">${sl('Amount', 62, '62%', { big: 1 })}</div><b>Simple</b><span>Pick what you want done. One slider, then export.</span></a>
      <a class="tile ${D.settings && D.settings.mode !== 'simple' ? 'on' : ''}" href="#hpcheck" data-go="hpcheck" data-act="set-mode" data-mode="pro"><div class="thumb" style="height:96px">${v('wave', { seed: 3, stereo: 1 })}</div><b>Pro</b><span>Timeline, automation, spectral editing, every parameter.</span></a>
    </div>
    <div class="between"><a class="dim" href="#home" data-go="home" data-act="onboarded">Skip setup</a>${btn('Continue', { k: 'p', lg: 1, go: 'hpcheck' })}</div>`));

  const HP = [['ok', 'Sounds right'], ['swapped', 'Left and right are swapped'], ['flat', 'Sounds flat, inside my head'], ['speakers', 'I\'m using speakers']];
  reg('hpcheck', 'First run', 'Headphone check', () => wiz(2, 4, `
    <h1 class="h1">Are your headphones the right way round?</h1>
    <p class="lede">3D and 8D audio depend on it. Press each button, then tell us what you heard.</p>
    <div class="g2">
      <button class="tile" type="button" style="align-items:center;padding:30px" data-act="tone" data-kind="left"><span class="act">${ic('head', 38)}</span><b style="font-size:22px">Play left</b><span>A soft tone in one ear</span></button>
      <button class="tile" type="button" style="align-items:center;padding:30px" data-act="tone" data-kind="right"><span class="act">${ic('head', 38)}</span><b style="font-size:22px">Play right</b><span>A soft tone in the other ear</span></button>
    </div>
    <div class="box col"><b>Next, a sound will jump around your head.</b><span class="dim">Front, left, right, then behind you.</span>${btn('Play the moving sound', { i: 'play', x: 'data-act="tone" data-kind="move"' })}</div>
    <div class="row">${HP.map(([k, t], i) => chip(t, (D.ui.hp || 'ok') === k, '', `data-act="hp-answer" data-v="${k}"`)).join('')}</div>
    ${cal('info', 'Flux can\'t tell whether headphones are plugged in, so this only checks what you hear. If it sounds flat, try the next step: picking a head shape usually fixes it.')}
    <div class="between"><a class="btn g" href="#welcome" data-go="welcome">Back</a><div class="row">${btn('Skip', { k: 'g', go: 'storage-ask' })}${btn('Continue', { k: 'p', lg: 1, go: 'hrtf' })}</div></div>`));

  const PROF = [['A', 'Wide head, bright', 19.5], ['B', 'Medium, neutral', 17.5], ['C', 'Medium, strong rear cue', 17.5], ['D', 'Narrow, bright', 15.5], ['E', 'Wide, warm', 19.5], ['F', 'Narrow, warm', 15.5]];
  reg('hrtf', 'First run', 'Pick your head shape', () => { const cur = (D.settings && D.settings.hrtf_profile) || 'C', hw = (D.settings && D.settings.head_width_cm) || 17.5; return wiz(3, 4, `
    <h1 class="h1">Which one sounds outside your head?</h1>
    <p class="lede">Everyone's ears differ. Play each profile and pick the one where the sound seems furthest from you, especially behind.</p>
    <div class="g3">${PROF.map(([n, d, w]) => `<button class="tile ${n === cur ? 'on' : ''}" type="button" data-act="hrtf" data-v="${n}" data-w="${w}"><div class="between"><b>Profile ${n}</b>${ic('play', 16)}</div><span>${d}</span></button>`).join('')}</div>
    ${sl('Head width', (hw - 12) / 10 * 100, hw.toFixed(1) + ' cm', { bind: 'set:head_width_cm:12:22:cm' })}
    <div class="between"><a class="btn g" href="#hpcheck" data-go="hpcheck">Back</a>${btn('Use Profile ' + cur, { k: 'p', lg: 1, go: 'storage-ask' })}</div>`); });

  reg('storage-ask', 'First run', 'Keep work safe', () => { const s = D.settings || {}; return wiz(4, 4, `
    <h1 class="h1">Where should your exports go?</h1>
    <p class="lede">Projects are saved as you work, in Flux's own folder on this computer. Exports are ordinary files you can see and back up.</p>
    <div class="g2">
      <a class="tile ${s.export_dir ? 'on' : ''}" href="#storage-ask" data-act="pick-export-dir">${ic('folder')}<b>Choose a folder</b><span>${s.export_dir ? esc(s.export_dir) : 'Pick where exported files are saved.'}</span></a>
      <a class="tile ${s.export_dir ? '' : 'on'}" href="#storage-ask" data-act="set" data-k="export_dir" data-v="">${ic('disk')}<b>Use the Music folder</b><span>Recommended. Exports go to Music, in a Flux folder.</span></a>
    </div>
    <div class="box col">${tg('Send anonymous crash reports', !!s.crash_reports, 'Off by default. Never includes audio, transcripts or file names.', 'data-act="set-toggle" data-k="crash_reports"')}</div>
    <div class="between"><span class="dim">Everything stays on this device.</span>${btn('Start using Flux', { k: 'p', lg: 1, go: 'home', x: 'data-act="onboarded"' })}</div>`); });

  /* ---------- Home ---------- */
  const KIND_SEED = { music: 1, podcast: 5, spatial: 8, voice: 2, lecture: 4, interview: 5 };
  const tasks = [['vinyl', 'vinyl', '8D', 'Make it 8D', 'Headphones', 'task'], ['slowed', 'slow', 'Slowed', 'Slowed + reverb', '0.85×', 'task-slowed'], ['stutter', 'stut', 'Vocals', 'Remove vocals', 'Stems', 'stems-setup'], ['underwater', 'water', 'Clean', 'Clean up a voice', 'Noise, echo', 'cleanup'], ['reverse', '', 'Text', 'Edit by text', 'Transcript', 'script'], ['tapestop', '', 'Magic', 'Make it better', 'Find and fix', 'magic'], ['radio', 'radio', 'Convert', 'Convert a file', 'MP3, FLAC', 'convert'], ['nightcore', 'night', 'Ring', 'Make a ringtone', '30 s', 'ringtone']];
  const kindName = k => ({ music: 'Music', podcast: 'Podcast', spatial: 'Spatial', voice: 'Voice', lecture: 'Lecture', interview: 'Interview' })[k] || 'Audio';
  const recentTile = r => `<a class="tile" href="#edit" data-act="open-project" data-id="${esc(r.id)}"><div class="thumb">${v('wave', { seed: KIND_SEED[r.kind] || (r.id.charCodeAt(0) % 9) + 1, step: 3 })}</div><b>${esc(r.name)}</b><span>${kindName(r.kind)} · ${F.len(r.seconds)} · ${r.tracks} track${r.tracks === 1 ? '' : 's'}</span></a>`;
  reg('home', 'Home', 'Home', () => { const rec = ((D.library && D.library.projects) || []).slice(0, 4); return pg('home', 'What are we doing to it?', '', `
    <a class="drop" href="#import" data-act="import"><span class="act">${ic('imp', 30)}</span><b style="font-size:19px;font-weight:800">Drop audio or video here</b><span class="dim">or choose a file, or <span class="act" data-go="record" role="link" tabindex="0">start recording</span>. Everything stays on this device.</span></a>
    <div class="g4">${tasks.map(t => fxc(t[0], t[1], t[2], t[3], t[4], { go: t[5] })).join('')}</div>
    <div class="between"><h2 class="h2">Recent</h2><a class="dim" href="#library" data-go="library">All projects</a></div>
    ${rec.length ? `<div class="g4">${rec.map(recentTile).join('')}</div>` : cal('info', 'Projects you make appear here. Drop a file above to start one.')}`); });

  const TPL = [['blank', 'Blank', 'One empty track', 'wave'], ['podcast', 'Podcast', 'Two voices, music bed, −16 LUFS', 'mic'], ['music', 'Music', 'Eight tracks, buses, master chain', 'mix'], ['voiceover', 'Voiceover', 'Voice, room tone, de-esser', 'voice'], ['lecture', 'Lecture', 'Long recording, silence removal', 'script'], ['interview', 'Interview', 'Two speakers, ready to edit', 'text'], ['8d', '8D music', 'Stereo in, orbit on, reverb room', 'orbit']];
  reg('new', 'Home', 'New project and templates', () => { const t = D.ui.tpl || 'blank', rate = D.ui.rate || (D.settings && D.settings.default_rate) || 48000, folders = (D.settings && D.settings.folders) || ['My Audio']; return pg('home', 'Start with a shape.', 'Templates set up tracks, effects and an export target. All of it can be changed later.', `
    <div class="g3">${TPL.map(x => tile(x[3], x[1], x[2], '', t === x[0] ? 'on' : '', `data-act="tpl" data-v="${x[0]}"`)).join('')}</div>
    <div class="box"><div class="g3">${fld('Name', inp(D.ui.newName || 'Untitled project', '', 'data-ui="newName" aria-label="Name"'))}${fld('Sample rate', sel([[48000, '48 kHz'], [44100, '44.1 kHz'], [96000, '96 kHz'], [192000, '192 kHz']], String(rate), 'data-ui="rate" aria-label="Sample rate"'))}${fld('Folder', sel(folders.map(f => [f, esc(f)]), D.ui.folder || folders[0], 'data-ui="folder" aria-label="Folder"'))}</div></div>`, btn('Create', { k: 'p', x: 'data-act="project-new"' })); });

  reg('empty', 'Home', 'Empty project', () => pr('edit', `<div class="pg in" style="margin:auto;align-items:center;text-align:center;max-width:620px"><h1 class="h1">Nothing here yet.</h1><p class="lede">Add a recording to get started. Your original files are never changed.</p>
    <div class="row" style="justify-content:center">${btn('Add a file', { k: 'p', lg: 1, i: 'imp', x: 'data-act="import"' })}${btn('Record', { lg: 1, i: 'mic', go: 'record' })}${btn('Generate speech', { lg: 1, i: 'text', go: 'tts' })}</div><div class="row dim" style="justify-content:center"><span class="kbd">Ctrl O</span> open file <span class="kbd">R</span> record <span class="kbd">Ctrl K</span> all commands</div></div>`, { name: P() ? P().name : 'Untitled project' }));

  const KINDS = [['all', 'All'], ['music', 'Music'], ['podcast', 'Podcast'], ['spatial', 'Spatial']];
  reg('library', 'Home', 'Projects', () => {
    const L = D.library || { projects: [], folders: [] }, q = D.ui.libq || {}, kon = Math.max(0, KINDS.findIndex(k => k[0] === (q.kind || 'all'))), folders = L.folders && L.folders.length ? L.folders : ['My Audio'];
    const bad = L.projects.filter(r => r.crashed || r.missing_media);
    return pg('library', 'Projects', '', `
    <div class="row"><div style="flex:1;min-width:200px">${inp(q.search || '', 'Search projects and markers', 'data-ui="libsearch"')}</div>${seg(KINDS.map(k => k[1]), kon, 'data-segact="lib-kind" aria-label="Kind"')}</div>
    <div class="row">${folders.map(f => chip(esc(f), q.folder === f, '', `data-act="lib-folder" data-v="${esc(f)}"`)).join('')}${chip('+ Folder', 0, '', 'data-act="add-folder"')}</div>
    ${L.projects.length ? `<div class="tw"><table><thead><tr><th>Name</th><th>Type</th><th>Length</th><th>Size</th><th>Changed</th><th>Stored</th><th></th></tr></thead><tbody>
    ${L.projects.map(r => `<tr><td><a class="b7" href="#edit" data-act="open-project" data-id="${esc(r.id)}" style="text-decoration:none">${esc(r.name)}</a></td><td class="dim">${kindName(r.kind)}</td><td class="mono">${F.len(r.seconds)}</td><td class="mono">${F.bytes(r.bytes)}</td><td class="dim">${F.ago(r.modified)}</td><td>${r.missing_media ? pill('Media missing', 'warn') : r.crashed ? pill('Recovered', 'warn') : pill('This computer', 'ok')}</td><td>${btn('', { k: 'g', i: 'dots', a: 'More actions for ' + esc(r.name), x: `data-act="project-menu" data-id="${esc(r.id)}"` })}</td></tr>`).join('')}</tbody></table></div>` : cal('info', q.search || q.kind || q.folder ? 'No projects match.' : 'No projects yet. <a class="act" href="#new" data-go="new">Start one</a> or drop a file on Home.')}
    ${bad.length ? cal('warn', `<b>${bad.length} project${bad.length === 1 ? ' needs' : 's need'} a look.</b> Missing media can be relinked when you open the project. <a class="act" href="#export-project" data-go="export-project">Back projects up as .flux files</a>.`, 'warn') : cal('check', 'Projects are saved on this computer as you work. <a class="act" href="#export-project" data-go="export-project">Back them up as .flux files</a> to move them.')}`, btn('Open .flux file', { i: 'folder', x: 'data-act="import-package"' }));
  });

  reg('project-menu', 'Home', 'Project actions', () => { const r = ((D.library && D.library.projects) || []).find(x => x.id === D.ui.menu) || { name: '', id: '' }; return dlg('library', esc(r.name), `
    <div class="lst">${[['folder', 'Open', 'open-project'], ['copy', 'Duplicate', 'project-dup'], ['box', 'Save as .flux', 'project-package'], ['trash', 'Move to trash', 'project-delete']].map(a => `<a href="#library" data-act="${a[2]}" data-id="${esc(r.id)}" style="text-decoration:none"><span class="act">${ic(a[0], 18)}</span><div class="ti"><b>${a[1]}</b></div></a>`).join('')}</div>
    <span class="dim">Changed ${F.ago(r.modified)} · ${F.len(r.seconds)} · ${F.bytes(r.bytes)}</span>`, btn('Close', { k: 'g', go: 'library' })); });

  reg('import', 'Home', 'Importing a file', () => {
    const j = D.ui.job || {}, m = P() && D.ui.importMedia ? (P().media || []).find(x => x.id === D.ui.importMedia) : null, pc = Math.round((j.progress || 0) * 100), done = j.state === 'done';
    return dlg('home', 'Adding ' + esc(D.ui.importName || 'audio'), `
    ${vz('wave', m ? { media: m.id, start: 0, end: m.frames, stereo: m.channels > 1 ? 1 : 0, until: done ? 1 : (j.progress || 0), color: '--t1', step: 2 } : { empty: 1, color: '--t1', step: 2 }, 130, 'Waveform being read')}
    ${prog(pc, !done && j.state === 'running')}
    <div class="g3 mono dim"><span>${m ? esc(m.format.toUpperCase()) + (m.bits ? ' · ' + m.bits + '-bit' : '') : '—'}</span><span>${m ? F.khz(m.source_rate) + ' ' + (m.channels > 1 ? 'stereo' : 'mono') : '—'}</span><span>${m ? F.pos(m.frames, F.sr()) : '—'}</span></div>
    <div class="lst">${(j.stages || []).map(([n, st]) => `<div><span class="${st === 'done' ? 'ok' : st === 'active' ? 'act' : 'faint'}">${ic(st === 'done' ? 'check' : st === 'active' ? 'exp' : 'dots', 17)}</span><div class="ti"><b>${esc(n)}</b></div><span class="mono dim">${st === 'done' ? 'Done' : st === 'active' ? pc + '%' : 'Waiting'}</span></div>`).join('')}</div>
    ${j.state === 'failed' ? cal('warn', '<b>Flux couldn\'t read this file.</b> ' + esc(j.error || ''), 'bad') : cal('info', 'You can start editing now. Flux keeps preparing the rest in the background.')}`, `${btn('Cancel', { k: 'g', x: 'data-act="job-cancel" data-then="home"' })}${btn('Open now', { k: 'p', go: 'edit' })}`);
  });

  reg('record', 'Home', 'Record', () => {
    const r = D.st.recording, dev = D.devices || { inputs: [] }, s = D.settings || {}, secs = r ? r.seconds : 0, paused = r && r.paused;
    return pr('edit', `<div class="pg in" style="max-width:900px;margin:auto"><div class="between"><h1 class="h1">${r ? (paused ? 'Paused.' : 'Recording.') : 'Ready to record.'}</h1><span class="pill ${r ? 'bad' : ''}"><span class="dot"></span><span data-rectime>${r ? 'REC ' + F.tc(secs).slice(0, 8) : 'REC 00:00:00'}</span></span></div>
    <div class="vz" style="height:170px">${r && r.media ? v('wave', { media: r.media, start: 0, end: Math.round(secs * F.sr()), kind: 'voice', color: '--sig', step: 2 }) : v('wave', { empty: 1, color: '--sig', step: 2 })}</div>
    <div class="col"><div class="between lab"><span>Input level</span><span class="mono" data-inpeak>${F.db(D.st.input_peak > 0 ? 20 * Math.log10(D.st.input_peak) : -70, 1)} dB peak</span></div><div class="meter" style="width:100%"><div class="m" style="height:12px"><i data-lvin></i></div><div class="m" style="height:12px"><i data-lvin></i></div></div></div>
    <div class="g2">${fld('Microphone', sel((dev.inputs || []).length ? dev.inputs.map(d => [d.name, esc(d.name) + (d.default ? ' (default)' : '')]) : [['', 'No microphone found']], s.input || ((dev.inputs || []).find(d => d.default) || {}).name || '', 'data-setsel="input" aria-label="Microphone"' + (r ? ' disabled' : '')))}${fld('Record to', sel([['new', 'New track'], ['sel', 'Selected track']], D.ui.recTo || 'new', 'data-ui="recTo" aria-label="Record to"' + (r ? ' disabled' : '')))}</div>
    <div class="box g2">${tg('Hear yourself while recording', !!D.ui.monitor, 'Use headphones, or you will record the playback', 'data-act="ui-toggle" data-k="monitor"')}${tg('Raw signal', true, 'No noise reduction or automatic gain on the way in')}</div>
    ${cal('check', 'Recording is written to disk as it happens. A crash loses no more than the last second.')}
    <div class="row" style="justify-content:center">${r ? btn('Stop', { k: 'p', lg: 1, i: 'stop', x: 'data-act="rec-stop"' }) + btn(paused ? 'Resume' : 'Pause', { lg: 1, i: paused ? 'rec' : 'pause', x: 'data-act="rec-pause"' }) + btn('Discard take', { k: 'g dng', lg: 1, x: 'data-act="rec-discard"' }) : btn('Start recording', { k: 'p', lg: 1, i: 'rec', x: 'data-act="rec-start"' }) + btn('Back', { lg: 1, go: P() ? 'edit' : 'home' })}</div></div>`);
  });

  /* ---------- Simple task ---------- */
  const taskView = (word, sub, ctrl, adv, style) => { const p = P(); return `<div class="app notp">${Flux.topbar(p ? esc(p.name) : 'No file yet', p ? F.len(p.seconds) + ' · ' + (p.tracks.length > 1 ? p.tracks.length + ' tracks' : 'stereo') : 'Add a file to start', `${seg(['Simple', 'Pro'], 0, 'data-mode aria-label="Mode"')}${btn('Export', { k: 'p', i: 'exp', go: 'export' })}`)}${Flux.rail(Flux.HOME, 'home')}<main class="main" id="m"><div class="pg in" style="max-width:1000px">
    <div><h1 class="bigh">${word}</h1><p class="lede">${sub}</p></div>
    ${p && p.length ? `<div class="vz" style="height:clamp(120px,24cqh,210px)">${v('wave', { src: 'mix', stereo: 1, step: 2.5 })}</div>` : `<a class="drop" href="#import" data-act="import" data-then="${style === '8d' ? 'task' : 'task-slowed'}"><span class="act">${ic('imp', 30)}</span><b style="font-size:19px;font-weight:800">Drop audio here</b><span class="dim">or choose a file. ${word} is applied as soon as it opens.</span></a>`}
    <div class="row"><button class="tbtn play" data-play aria-label="Play">${ic('play')}</button><span class="tc mono" data-tc style="font-size:19px">${F.tc(D.st.seconds)}</span><span class="sp" style="flex:1"></span><button class="btn lg hold" data-hold>Hold for original</button></div>
    ${ctrl}
    <div class="row">${chip('Make it 8D', word === '8D', 'task')}${chip('Slowed + reverb', word.startsWith('Slowed'), 'task-slowed')}${chip('Remove vocals', 0, 'stems-setup')}${chip('Clean up voice', 0, 'cleanup')}${chip('Remove noise', 0, 'fx-clean')}${chip('Convert', 0, 'convert')}</div>
    <a class="dim" href="#${adv}" data-go="${adv}">Open all the controls in Pro mode</a></div></main></div>`; };
  const pv = (e, k) => F.pval(e, k);
  const bindP = (e, k) => e ? `fx:${e.id}:${k}` : '';
  const slP = (label, e, k, o = {}) => { const s = e && F.spec(e.kind), p = s && s.params.find(q => q.key === k); if (!p) return sl(label, 0, '—', o); const val = pv(e, k); return sl(label, F.pct(p, val), o.fmt ? o.fmt(val) : F.fmtp(p, val), Object.assign({ bind: bindP(e, k) }, o)); };
  reg('task', 'Home', 'Simple mode: Make it 8D', () => { const e = F.fxOf('spatial'), rv = F.fxOf('reverb'); return taskView('8D', 'Sound circles your head. Use headphones. <a class="act" href="#hpcheck" data-go="hpcheck">Check them</a>', `${slP('Rotation speed', e, 'rate', { big: 1, fmt: r => r > 0 ? 'One turn every ' + Math.round(1 / r) + ' s' : 'Still' })}<div class="g2">${slP('Distance', e, 'distance')}${slP('Room', e, 'room', { fmt: r => r < 20 ? 'Dry' : r < 45 ? 'Small room' : r < 70 ? 'Medium room' : 'Large room' })}</div>${rv ? '' : ''}`, 'stage', '8d'); });
  reg('task-slowed', 'Home', 'Simple mode: Slowed + reverb', () => { const sp = F.fxOf('speed'), rv = F.fxOf('reverb'); return taskView('Slowed + reverb', 'Slows the track, drops the pitch and fills the room.', `${slP('Amount', sp, 'rate', { big: 1, fmt: r => r.toFixed(2) + '× speed' })}<div class="g2">${tg('Keep the original pitch', sp && pv(sp, 'keep_pitch') >= .5, '', sp ? `data-act="fx-toggle" data-fx="${sp.id}" data-k="keep_pitch"` : 'disabled')}${slP('Reverb', rv, 'decay', { fmt: d => (d > 2.5 ? 'Hall ' : d > 1.2 ? 'Room ' : 'Plate ') + d.toFixed(1) + ' s' })}</div>`, 'creative', 'slowed'); });

  /* ---------- Edit ---------- */
  const view = () => { const p = P(), sr = F.sr(), len = Math.max((p && p.length) || 0, sr * 8), z = D.ui.zoom || 1, vlen = len * 1.04 / z; let t0 = 0; if (z > 1) { t0 = Math.max(0, Math.min(len * 1.04 - vlen, (D.st.pos || 0) - vlen * .3)); } return { t0, vlen, sr }; };
  const pct = (x, w) => (x - w.t0) / w.vlen * 100;
  const clipEl = (c, t, w, col) => { const l = pct(c.start, w), wd = c.len / w.vlen * 100, s = D.sel.clip === c.id, lw = 1200; return `<div class="clip ${s ? 'sel' : ''}" style="left:${l}%;width:${wd}%;--c:var(${col});--co:var(${col}-on)" data-clip="${c.id}" tabindex="0" role="button" aria-label="${esc(c.name)}, ${F.pos(c.start, w.sr)}">${v('wave', { media: c.media, start: c.offset, end: c.offset + c.len, rev: c.reverse ? 1 : 0, color: col, on: 1, step: 2.5, gain: Math.pow(10, (c.gain_db || 0) / 20) })}${c.fade_in ? `<span class="fi" style="--w:${Math.max(4, c.fade_in / w.vlen * lw)}px"></span>` : ''}${c.fade_out ? `<span class="fo" style="--w:${Math.max(4, c.fade_out / w.vlen * lw)}px"></span>` : ''}<span class="cl">${esc(c.name)}</span></div>`; };
  const tcol = (t, i) => '--t' + (1 + ((t.color != null ? t.color : i) % 8));
  const laneSvg = (t, w, target) => { const ln = (t.lanes || []).find(l => l.target === (target || 'volume')) || (t.lanes || [])[0]; if (!ln || !ln.points || !ln.points.length) return ''; const isVol = ln.target === 'volume', ys = v => isVol ? 40 - (Math.max(-36, Math.min(12, v)) + 36) / 48 * 36 - 2 : 20 - v / (ln.target === 'pan' ? 1 : 1) * 18; const pts = ln.points.map(([x, y]) => `${Math.max(0, Math.min(100, pct(x, w))).toFixed(2)} ${ys(y).toFixed(2)}`); return `<svg class="autoline" viewBox="0 0 100 40" preserveAspectRatio="none" aria-hidden="true"><path d="M0 ${pts[0].split(' ')[1]} L${pts.join(' L')} L100 ${pts[pts.length - 1].split(' ')[1]}" fill="none" stroke="var(--sig)" stroke-width="1.8" vector-effect="non-scaling-stroke"/></svg>`; };
  const track = (t, i, w, auto) => { const c = tcol(t, i); return `<div class="trk ${i === 0 ? 'tall' : ''}" data-track="${t.id}"><div class="thd"><div class="nm"><span class="dot" style="color:var(${c})"></span><b>${esc(t.name)}</b></div><div class="ms"><button class="m ${t.mute ? 'on' : ''}" aria-pressed="${!!t.mute}" data-act="track-flag" data-track="${t.id}" data-k="mute" aria-label="Mute ${esc(t.name)}">M</button><button class="s ${t.solo ? 'on' : ''}" aria-pressed="${!!t.solo}" data-act="track-flag" data-track="${t.id}" data-k="solo" aria-label="Solo ${esc(t.name)}">S</button><button class="${t.arm ? 'on' : ''}" aria-pressed="${!!t.arm}" aria-label="Arm ${esc(t.name)} for recording" data-act="track-flag" data-track="${t.id}" data-k="arm">R</button></div></div><div class="lane" data-lane="${t.id}">${t.clips.map(cl => clipEl(cl, t, w, c)).join('')}${auto ? laneSvg(t, w, D.ui.autoTarget && D.ui.autoTarget.split(':')[0] == t.id ? D.ui.autoTarget.split(':').slice(1).join(':') : null) : ''}</div></div>`; };
  const tracks = auto => { const w = view(), p = P(); return `<div class="tl" data-tl><div class="ruler" data-ruler>${v('ruler', { len: (w.vlen / w.sr).toFixed(3), t0: (w.t0 / w.sr).toFixed(3) })}</div>
    ${p.tracks.map((t, i) => track(t, i, w, auto)).join('')}
    ${(p.markers || []).map(m => `<span class="ph" style="left:0;transform:translateX(0);opacity:.5;left:calc(var(--hd,0px) + ${pct(m.pos, w)}%)" aria-hidden="true"></span>`).join('')}
    <span class="ph" data-ph data-t0="${w.t0}" data-vlen="${w.vlen}" style="transform:translateX(0)"></span></div>`; };
  const edittools = () => `<div class="tlbar" role="toolbar" aria-label="Edit tools">${seg(['Select', 'Range', 'Slip', 'Fade'], ['select', 'range', 'slip', 'fade'].indexOf(D.ui.tool || 'select'), 'data-segact="edit-tool" aria-label="Tool"')}${btn('', { i: 'cut', k: 'g', a: 'Cut', x: 'data-act="edit-cut"' })}${btn('', { i: 'copy', k: 'g', a: 'Copy', x: 'data-act="edit-copy"' })}${btn('', { i: 'split', k: 'g', a: 'Split at playhead', x: 'data-act="edit-split"' })}${btn('', { i: 'trash', k: 'g', a: 'Delete', x: 'data-act="edit-delete"' })}<span class="sp" style="flex:1"></span>${chip('Snap: zero cross', D.ui.snap !== false, '', 'data-act="ui-toggle" data-k="snap" data-def="1" aria-pressed="' + (D.ui.snap !== false) + '"')}${chip('Grid: 1/4', !!D.ui.grid, '', 'data-act="ui-toggle" data-k="grid" aria-pressed="' + !!D.ui.grid + '"')}${chip('Ripple', !!D.ui.ripple, '', 'data-act="ui-toggle" data-k="ripple" aria-pressed="' + !!D.ui.ripple + '"')}${btn('Automation', { k: 'g', go: 'automation' })}${btn('', { i: 'zoom', k: 'g', a: 'Zoom ' + (D.ui.zoom || 1) + '×', x: 'data-act="zoom"' })}</div>`;
  const CURVES = [['equal', 'Equal power'], ['s', 'S'], ['log', 'Log'], ['lin', 'Lin']];
  const insp = () => {
    const c = D.sel.clip != null ? F.clip(D.sel.clip) : null, sr = F.sr();
    if (!c) return `<div class="between"><h3 class="h3">${esc(P().name)}</h3>${pill(P().tracks.length + ' tracks')}</div><div class="g2 mono dim"><span>Length ${F.pos(P().length, sr)}</span><span>${F.khz(sr)}</span></div>${cal('info', 'Click a clip to edit it. Drag it to move it. Press S to split at the playhead.')}
      ${sec('Tracks', `<div class="lst">${P().tracks.map((t, i) => `<div><span class="dot" style="color:var(${tcol(t, i)})"></span><div class="ti"><b>${esc(t.name)}</b><span>${t.clips.length} clip${t.clips.length === 1 ? '' : 's'} · ${F.db(t.gain_db)} dB</span></div>${btn('', { k: 'g', i: 'trash', a: 'Remove track ' + esc(t.name), x: `data-act="track-remove" data-track="${t.id}"` })}</div>`).join('')}</div>${btn('Add track', { i: 'plus', x: 'data-act="track-add"' })}`)}
      ${sec('Add', `<div class="row">${chip('Add a file', 0, '', 'data-act="import"')}${chip('Record', 0, 'record')}${chip('Marker at playhead', 0, '', 'data-act="marker-add"')}</div>`)}`;
    const t = F.track(c.track), maxf = Math.max(1, Math.min(c.len / 2, sr * 5));
    return `<div class="between"><h3 class="h3">${esc(c.name)}</h3>${pill(esc(t ? t.name : ''))}</div>
    <div class="g2 mono dim"><span>Start ${F.pos(c.start, sr)}</span><span>Len ${F.pos(c.len, sr)}</span></div>
    ${sl('Clip gain', (c.gain_db + 24) / 48 * 100, F.db(c.gain_db) + ' dB', { bind: `clip:${c.id}:gain_db:-24:24:dB` })}<div class="g2">${sl('Fade in', c.fade_in / maxf * 100, Math.round(c.fade_in / sr * 1000) + ' ms', { bind: `clip:${c.id}:fade_in:0:${Math.round(maxf)}:smp` })}${sl('Fade out', c.fade_out / maxf * 100, Math.round(c.fade_out / sr * 1000) + ' ms', { bind: `clip:${c.id}:fade_out:0:${Math.round(maxf)}:smp` })}</div>
    ${fld('Fade curve', seg(CURVES.map(x => x[1]), Math.max(0, CURVES.findIndex(x => x[0] === c.curve)), `data-segact="clip-curve" data-clip="${c.id}" aria-label="Fade curve"`))}
    ${sec('Effects on this clip', `<div class="lst">${(c.fx || []).length ? c.fx.map(e => `<div><span class="act">${ic('fx', 16)}</span><div class="ti"><b>${esc(F.kindName(e.kind))}</b><span>${e.enabled ? (e.mix < 1 ? Math.round(e.mix * 100) + '% mix' : 'On') : 'Bypassed'}</span></div><a class="btn g" href="#rack" data-act="open-fx" data-fx="${e.id}">Open</a></div>`).join('') : '<div><div class="ti"><span>No effects yet</span></div></div>'}</div>${btn('Add effect', { i: 'plus', go: 'fxbrowser', x: `data-act="set-target" data-target="clip:${c.id}"` })}`)}
    ${sec('Quick', `<div class="row">${chip('Reverse', c.reverse, '', `data-act="clip-reverse" data-clip="${c.id}"`)}${chip('Normalize', 0, '', `data-act="clip-normalize" data-clip="${c.id}"`)}${chip('Duplicate', 0, '', `data-act="clip-dup" data-clip="${c.id}"`)}${chip('Bounce', 0, '', `data-act="clip-bounce" data-clip="${c.id}"`)}</div>`)}`;
  };
  const needP = fn => () => (P() && P().tracks.length && P().tracks.some(t => t.clips.length)) ? fn() : Flux.R('empty');
  reg('edit', 'Workspaces', 'Edit: timeline', needP(() => pr('edit', `<div class="split"><div class="a">${edittools()}${tracks()}</div><aside class="b in">${insp()}</aside></div>`)));

  const SHAPES = [['linear', 'Linear'], ['smooth', 'Smooth'], ['hold', 'Hold']];
  reg('automation', 'Workspaces', 'Edit: automation lanes', needP(() => {
    const p = P(), opts = p.tracks.flatMap(t => [[`${t.id}:volume`, `${esc(t.name)}: Volume`], [`${t.id}:pan`, `${esc(t.name)}: Pan`]].concat((t.fx || []).flatMap(e => { const s = F.spec(e.kind); return s ? s.params.filter(q => !/enum|bool|int/.test(q.unit)).slice(0, 4).map(q => [`${t.id}:fx:${e.id}:${q.key}`, `${esc(t.name)}: ${esc(s.name)} ${esc(q.label)}`]) : []; })));
    const cur = D.ui.autoTarget || opts[0][0], [tid, ...rest] = cur.split(':'), t = F.track(+tid) || p.tracks[0], tgt = rest.join(':'), lane = (t.lanes || []).find(l => l.target === tgt);
    return pr('edit', `<div class="split"><div class="a"><div class="tlbar">${seg(['Read', 'Write', 'Touch', 'Latch'], ['read', 'write', 'touch', 'latch'].indexOf(D.ui.autoMode || 'read'), 'data-segact="auto-mode" aria-label="Automation mode"')}${fld('', sel(opts, cur, 'data-ui="autoTarget" data-rerender="1" aria-label="Lane"'))}${seg(SHAPES.map(x => x[1]), Math.max(0, SHAPES.findIndex(x => x[0] === (lane ? lane.shape : 'linear'))), `data-segact="lane-shape" aria-label="Shape"`)}<span class="sp" style="flex:1"></span>${btn('Done', { go: 'edit' })}</div>${tracks(true)}</div><aside class="b in"><h3 class="h3">${esc(t.name)}: ${tgt === 'volume' ? 'Volume' : tgt === 'pan' ? 'Pan' : esc(opts.find(o => o[0] === cur)[1].split(': ')[1])}</h3>
    ${lane ? `<div class="mono dim">${lane.points.length} point${lane.points.length === 1 ? '' : 's'} · from ${F.pos(lane.points[0][0], F.sr())}</div>` : cal('info', 'No points yet. Click on the track to add one.')}
    ${sec('Lanes on this track', `<div class="lst">${(t.lanes || []).length ? t.lanes.map((l, i) => `<div><div class="ti"><b>${l.target === 'volume' ? 'Volume' : l.target === 'pan' ? 'Pan' : esc(l.target)}</b><span>${l.points.length} points</span></div>${btn('', { k: 'g', i: 'x', a: 'Remove lane', x: `data-act="lane-remove" data-track="${t.id}" data-lane="${l.id}"` })}</div>`).join('') : '<div><div class="ti"><span>None yet</span></div></div>'}</div>${btn('Clear this lane', { i: 'x', x: `data-act="lane-clear"` })}`)}${cal('info', 'Click on a track to add a point at that time and height. Write mode records slider moves while playing.')}</aside></div>`);
  }));

  reg('spectral', 'Workspaces', 'Spectral: waveform and repair', needP(() => {
    const ss = D.ui.specSel, sr = F.sr(), len = P().length, view = ['Spectrogram', 'Waveform', 'Both'].indexOf(D.ui.specView || 'Both'), win = D.ui.window || 4096;
    const box = ss ? `<div data-specsel style="position:absolute;left:${ss.t0 / len * 100}%;top:${(1 - Math.log(ss.f1 / 20) / Math.log(1000)) * 100}%;width:${(ss.t1 - ss.t0) / len * 100}%;height:${(Math.log(ss.f1 / ss.f0) / Math.log(1000)) * 100}%;border:2px dashed var(--tx);border-radius:2px"><span class="mono" style="position:absolute;top:-22px;right:0;padding:1px 6px;border-radius:3px;background:var(--tx);color:var(--s0);white-space:nowrap">${F.hz(ss.f0)} to ${F.hz(ss.f1)} · ${((ss.t1 - ss.t0) / sr).toFixed(2)} s</span></div>` : '';
    return pr('spectral', `<div class="split"><div class="a"><div class="tlbar">${seg(['Spectrogram', 'Waveform', 'Both'], view, 'data-segact="spec-view" aria-label="View"')}${seg(['Rectangle', 'Lasso', 'Brush', 'Harmonic'], 0, 'aria-label="Selection tool"')}<span class="sp" style="flex:1"></span>${chip('Mel')}${chip('Log', 1)}${chip('Linear')}${fld('', sel([[4096, '4096 window'], [2048, '2048 window'], [8192, '8192 window']], String(win), 'data-ui="window" data-rerender="1" aria-label="Window"'))}</div>
    <div style="flex:1;display:flex;flex-direction:column;min-height:300px;position:relative">${view !== 0 ? `<div style="height:${view === 1 ? '100%' : '30%'};min-height:80px;border-bottom:1px solid var(--ln)">${v('wave', { src: 'mix', stereo: 1, step: 2 })}</div>` : ''}${view !== 1 ? `<div style="flex:1;position:relative;min-height:220px" data-specarea>${v('spec', { src: 'mix', window: win })}${box}</div>` : ''}</div></div>
    <aside class="b in"><h3 class="h3">Selection</h3>${ss ? `<div class="g2 mono dim"><span>${F.hz(ss.f0)} to ${F.hz(ss.f1)}</span><span>${F.pos(ss.t0, sr)} to ${F.pos(ss.t1, sr)}</span></div>` : '<span class="dim">Drag on the spectrogram to select a sound.</span>'}
    ${sec('Repair', `<div class="col">${btn('Remove and fill in', { k: 'p', i: 'pen', x: 'data-act="spec-repair"' + (ss ? '' : ' disabled') })}${sl('Amount', (D.ui.repairDb || 30) / 60 * 100, '−' + (D.ui.repairDb || 30) + ' dB', { bind: 'ui:repairDb:0:60:dBneg' })}${fld('Fill from', seg(['Both sides', 'Before', 'After'], ['both', 'before', 'after'].indexOf(D.ui.fill || 'both'), 'data-segact="spec-fill" aria-label="Fill from"'))}</div>`)}
    ${sec('Also', `<div class="row">${chip('Attenuate', 0, '', 'data-act="spec-attenuate"' + (ss ? '' : ' disabled'))}${chip('Clear selection', 0, '', 'data-act="spec-clear"')}${chip('Listen to selection', 0, '', 'data-act="spec-listen"' + (ss ? '' : ' disabled'))}</div>`)}
    ${sec('Spectrum at playhead', vz('spectrum', {}, 110))}
    ${cal('info', 'Repairs are stored as edits. Your original audio stays unchanged.')}</aside></div>`);
  }));

  /* ---------- Mix ---------- */
  const fpct = g => Math.max(0, Math.min(100, 70 + g * 6)), gfp = p => (p - 70) / 6;
  const ch = (n, c, t, mst) => { const g = mst ? P().master.gain_db : t.gain_db, fx = mst ? P().master.fx.length : t.fx.length, pan = mst ? 0 : t.pan; return `<div class="ch ${mst ? 'mst' : ''}"><span class="dot" style="color:var(${c})"></span><b>${esc(n)}</b><div class="knob" style="--a:${Math.round(pan * 135)}deg" role="img" aria-label="Pan ${Math.round(pan * 100)}"></div><a class="chip" href="#rack" data-go="rack" data-act="set-target" data-target="${mst ? 'master' : 'track:' + t.id}" style="text-decoration:none;width:100%;justify-content:center">${fx} fx</a><div class="fader" style="--v:${fpct(g)}%" data-bind="${mst ? 'master:gain' : 'track:' + t.id + ':gain_db'}"><div class="ftk" role="slider" tabindex="0" aria-label="${esc(n)} level" aria-valuenow="${Math.round(fpct(g))}" aria-valuemin="0" aria-valuemax="100" aria-valuetext="${F.db(g)} dB"><span class="fth"></span></div><div class="vu"><i data-lvv="${mst ? 'm' : t.id}" data-ch="0"></i></div><div class="vu"><i data-lvv="${mst ? 'm' : t.id}" data-ch="1"></i></div></div><span class="mono dim" data-gtext>${F.db(g)} dB</span><div class="thd" style="position:static;padding:0;width:auto;background:none;border:0"><div class="ms">${mst ? '' : `<button class="m ${t.mute ? 'on' : ''}" data-act="track-flag" data-track="${t.id}" data-k="mute" aria-pressed="${!!t.mute}" aria-label="Mute">M</button><button class="s ${t.solo ? 'on' : ''}" data-act="track-flag" data-track="${t.id}" data-k="solo" aria-pressed="${!!t.solo}" aria-label="Solo">S</button>`}</div></div></div>`; };
  Flux.gfp = gfp;
  reg('mix', 'Workspaces', 'Mix: mixer', needP(() => { const L = (D.loudness && D.loudness.loudness) || {}; return pr('mix', `<div class="split"><div class="a"><div class="mixer in">${P().tracks.map((t, i) => ch(t.name, tcol(t, i), t)).join('')}${ch('Master', '--tx', null, 1)}</div></div><aside class="b in"><h3 class="h3">Master</h3>${sec('Loudness', `<div class="g2"><div><div class="stat">${F.num(L.integrated, 1)}</div><span class="lab">Integrated LUFS</span></div><div><div class="stat">${F.num(L.true_peak, 1)}</div><span class="lab">True peak dBTP</span></div></div>`)}${sec('Stereo', vz('gonio', {}, 160, 'Goniometer'))}<div class="between"><span class="lab">Correlation</span><span class="mono" data-corr>${D.problems ? F.minus((D.problems.correlation >= 0 ? '+' : '') + D.problems.correlation.toFixed(2)) : '—'}</span></div>${btn('Open all meters', { go: 'meters' })}</aside></div>`); }));

  /* ---------- History ---------- */
  reg('history', 'Workspaces', 'Visual history', () => {
    const h = D.history || { items: [], current: 0 }, snaps = D.snapshots || [], items = h.items.slice().reverse();
    return pr('edit', `<div class="pg in"><div class="pgh"><div><h1 class="h1">History</h1><p class="lede">Every change, kept. Go back to any point, or save a snapshot and come back to it.</p></div>${btn('Save a snapshot', { i: 'disk', x: 'data-act="snapshot-save"' })}</div>
    <div class="vz" style="height:170px">${v('history')}</div>
    <div class="row"><span class="row"><span class="dot sig"></span>You are here</span><span class="row"><span class="dot" style="color:var(--tx2)"></span>Earlier changes</span>${h.items.some(i => i.state === 'future') ? '<span class="row"><span class="dot" style="color:var(--tx3)"></span>Undone, can be redone</span>' : ''}</div>
    <div class="lst box">${items.map(r => `<div><div class="ti"><b>${esc(r.state === 'current' ? 'Now' : r.label)}</b><span>${F.ago(r.time)}${r.state === 'future' ? ' · undone' : ''}</span></div>${btn(r.state === 'current' ? 'Current' : 'Go here', { k: 'g', x: r.state === 'current' ? 'disabled' : `data-act="history-goto" data-i="${r.state === 'past' ? r.index : r.index - 1}"` })}</div>`).join('')}</div>
    ${snaps.length ? sec('Snapshots', `<div class="lst box">${snaps.map(s => `<div><div class="ti"><b>${esc(s.name)}</b><span>${F.ago(s.modified)}</span></div>${btn('Restore', { k: 'g', x: `data-act="snapshot-restore" data-id="${esc(s.id)}"` })}</div>`).join('')}</div>`) : ''}</div>`);
  });
})();
