/* First run, home, editing workspaces. */
(function () {
  const { fxc, ic, v, vz, btn, seg, sl, tg, chip, pill, fld, inp, sel, sec, tile, cal, prog, pr, pg, wiz, dlg, reg } = Flux;

  /* ---------- First run ---------- */
  reg('boot', 'First run', 'Checking your browser', () => wiz(0, 0, `
    <h1 class="h1">Checking what this browser can do.</h1>
    <div class="lst box">
      ${[['Audio engine', 'AudioWorklet and WebAssembly', 'Ready', 'ok'], ['Low-latency mode', 'Shared memory is on', 'Ready', 'ok'], ['On-device AI', 'WebGPU found: stems run at about 0.8× real time', 'Fast', 'ok'], ['Saving to folders', 'File System Access available', 'Ready', 'ok'], ['AAC export', 'Encoder available in this browser', 'Ready', 'ok'], ['Offline use', 'Installing for offline', '64%', 'ai']].map(r => `<div><span class="${r[3] === 'ok' ? 'ok' : 'act'}">${ic(r[3] === 'ok' ? 'check' : 'exp', 18)}</span><div class="ti"><b>${r[0]}</b><span style="white-space:normal">${r[1]}</span></div>${pill(r[2], r[3])}</div>`).join('')}
    </div>
    <div class="between"><span class="dim">You can see this again in Settings, under Audio.</span>${btn('Continue', { k: 'p', lg: 1, go: 'welcome' })}</div>`));

  reg('unsupported', 'First run', 'Limited browser', () => wiz(0, 0, `
    <h1 class="h1">Flux needs a newer browser for this.</h1>
    <p class="lede">This browser can't run the audio engine. Your files are safe, because nothing has been changed or sent anywhere.</p>
    ${cal('warn', '<b>Missing: AudioWorklet.</b> Without it, Flux can\'t play audio without glitches.', 'warn')}
    <div class="g3">${tile('globe', 'Chrome or Edge', 'Every feature, including on-device AI', '')}${tile('globe', 'Firefox', 'Editing and effects. AI depends on your device', '')}${tile('globe', 'Safari', 'Editing and effects. Back up projects often', '')}</div>
    <div class="row">${btn('Try anyway with basic playback', { go: 'welcome' })}${btn('Read what each browser supports', { k: 'g', go: 'help' })}</div>`));

  reg('welcome', 'First run', 'Welcome and mode', () => wiz(1, 4, `
    <h1 class="bigh">Hello.</h1>
    <p class="lede">How do you want to work? You can switch any time. Nothing you make is lost when you do.</p>
    <div class="g2">
      <a class="tile" href="#hpcheck" data-go="hpcheck"><div class="thumb" style="height:96px;padding:14px">${sl('Amount', 62, '62%', { big: 1 })}</div><b>Simple</b><span>Pick what you want done. One slider, then export.</span></a>
      <a class="tile" href="#hpcheck" data-go="hpcheck"><div class="thumb" style="height:96px">${v('wave', { seed: 3, stereo: 1 })}</div><b>Pro</b><span>Timeline, automation, spectral editing, every parameter.</span></a>
    </div>
    <div class="between"><a class="dim" href="#home" data-go="home">Skip setup</a>${btn('Continue', { k: 'p', lg: 1, go: 'hpcheck' })}</div>`));

  reg('hpcheck', 'First run', 'Headphone check', () => wiz(2, 4, `
    <h1 class="h1">Are your headphones the right way round?</h1>
    <p class="lede">3D and 8D audio depend on it. Press each button, then tell us what you heard.</p>
    <div class="g2">
      <button class="tile" type="button" style="align-items:center;padding:30px"><span class="act">${ic('head', 38)}</span><b style="font-size:22px">Play left</b><span>A soft tone in one ear</span></button>
      <button class="tile" type="button" style="align-items:center;padding:30px"><span class="act">${ic('head', 38)}</span><b style="font-size:22px">Play right</b><span>A soft tone in the other ear</span></button>
    </div>
    <div class="box col"><b>Next, a sound will jump around your head.</b><span class="dim">Front, left, right, then behind you.</span>${btn('Play the moving sound', { i: 'play' })}</div>
    <div class="row">${chip('Sounds right', 1)}${chip('Left and right are swapped')}${chip('Sounds flat, inside my head')}${chip('I\'m using speakers')}</div>
    ${cal('info', 'Browsers can\'t detect headphones, so this only checks what you hear. If it sounds flat, try the next step: picking a head shape usually fixes it.')}
    <div class="between"><a class="btn g" href="#welcome" data-go="welcome">Back</a><div class="row">${btn('Skip', { k: 'g', go: 'storage-ask' })}${btn('Continue', { k: 'p', lg: 1, go: 'hrtf' })}</div></div>`));

  reg('hrtf', 'First run', 'Pick your head shape', () => wiz(3, 4, `
    <h1 class="h1">Which one sounds outside your head?</h1>
    <p class="lede">Everyone's ears differ. Play each profile and pick the one where the sound seems furthest from you, especially behind.</p>
    <div class="g3">${['Profile A', 'Profile B', 'Profile C', 'Profile D', 'Profile E', 'Profile F'].map((n, i) => `<button class="tile ${i === 2 ? 'on' : ''}" type="button"><div class="between"><b>${n}</b>${ic('play', 16)}</div><span>${['Wide head, bright', 'Medium, neutral', 'Medium, strong rear cue', 'Narrow, bright', 'Wide, warm', 'Narrow, warm'][i]}</span></button>`).join('')}</div>
    ${sl('Head width', 52, '17.5 cm')}
    <div class="between"><a class="btn g" href="#hpcheck" data-go="hpcheck">Back</a>${btn('Use Profile C', { k: 'p', lg: 1, go: 'storage-ask' })}</div>`));

  reg('storage-ask', 'First run', 'Keep work safe', () => wiz(4, 4, `
    <h1 class="h1">Where should your projects live?</h1>
    <p class="lede">Browsers can clear their own storage when space runs low. A real folder means your work can't disappear.</p>
    <div class="g2">
      <a class="tile on" href="#home" data-go="home">${ic('folder')}<b>Choose a folder</b><span>Recommended. Projects are ordinary files you can see and back up.</span></a>
      <a class="tile" href="#home" data-go="home">${ic('disk')}<b>Keep them in this browser</b><span>Works everywhere. We'll ask for persistent storage and remind you to back up.</span></a>
    </div>
    <div class="box col">${tg('Send anonymous crash reports', false, 'Off by default. Never includes audio, transcripts or file names.')}</div>
    <div class="between"><span class="dim">Everything stays on this device.</span>${btn('Start using Flux', { k: 'p', lg: 1, go: 'home' })}</div>`));

  /* ---------- Home ---------- */
  const recent = [['Harbor Lights', 'Music · 3:42 · 6 tracks', 1, 'edit'], ['Interview ep. 12', 'Podcast · 48:10 · 2 speakers', 5, 'script'], ['Sunset 8D', 'Spatial · 4:18', 8, 'stage'], ['Rain voiceover', 'Voice · 1:06', 2, 'edit']];
  const tasks = [['vinyl', 'vinyl', '8D', 'Make it 8D', 'Headphones', 'task'], ['slowed', 'slow', 'Slowed', 'Slowed + reverb', '0.85×', 'task-slowed'], ['stutter', 'stut', 'Vocals', 'Remove vocals', 'Stems', 'stems-setup'], ['underwater', 'water', 'Clean', 'Clean up a voice', 'Noise, echo', 'cleanup'], ['reverse', '', 'Text', 'Edit by text', 'Transcript', 'script'], ['tapestop', '', 'Magic', 'Make it better', 'Find and fix', 'magic'], ['radio', 'radio', 'Convert', 'Convert a file', 'MP3, FLAC', 'convert'], ['nightcore', 'night', 'Ring', 'Make a ringtone', '30 s', 'ringtone']];
  reg('home', 'Home', 'Home', () => pg('home', 'What are we doing to it?', '', `
    <a class="drop" href="#import" data-go="import"><span class="act">${ic('imp', 30)}</span><b style="font-size:19px;font-weight:800">Drop audio or video here</b><span class="dim">or choose a file, paste audio, or <span class="act">start recording</span>. Everything stays on this device.</span></a>
    <div class="g4">${tasks.map(t => fxc(t[0], t[1], t[2], t[3], t[4], { go: t[5] })).join('')}</div>
    <div class="between"><h2 class="h2">Recent</h2><a class="dim" href="#library" data-go="library">All projects</a></div>
    <div class="g4">${recent.map(r => `<a class="tile" href="#${r[3]}" data-go="${r[3]}"><div class="thumb">${v('wave', { seed: r[2], step: 3 })}</div><b>${r[0]}</b><span>${r[1]}</span></a>`).join('')}</div>`));

  reg('new', 'Home', 'New project and templates', () => pg('home', 'Start with a shape.', 'Templates set up tracks, effects and an export target. All of it can be changed later.', `
    <div class="g3">${[['Blank', 'One empty track at 48 kHz', 'wave'], ['Podcast', 'Two voices, music bed, −16 LUFS', 'mic'], ['Music', 'Eight tracks, buses, master chain', 'mix'], ['Voiceover', 'Voice, room tone, de-esser', 'voice'], ['Lecture', 'Long recording, silence removal', 'script'], ['Interview', 'Two speakers, transcript ready', 'text'], ['8D music', 'Stereo in, orbit on, reverb room', 'orbit']].map(t => tile(t[2], t[0], t[1], 'edit')).join('')}</div>
    <div class="box"><div class="g3">${fld('Name', inp('Untitled project'))}${fld('Sample rate', sel(['48 kHz', '44.1 kHz', '96 kHz', '192 kHz']))}${fld('Save to', sel(['My Audio folder', 'This browser']))}</div></div>`, btn('Create', { k: 'p', go: 'edit' })));

  reg('empty', 'Home', 'Empty project', () => pr('edit', `<div class="pg in" style="margin:auto;align-items:center;text-align:center;max-width:620px"><h1 class="h1">Nothing here yet.</h1><p class="lede">Add a recording to get started. Your original files are never changed.</p>
    <div class="row" style="justify-content:center">${btn('Add a file', { k: 'p', lg: 1, i: 'imp', go: 'import' })}${btn('Record', { lg: 1, i: 'mic', go: 'record' })}${btn('Generate speech', { lg: 1, i: 'text', go: 'tts' })}</div><div class="row dim" style="justify-content:center"><span class="kbd">Ctrl O</span> open file <span class="kbd">R</span> record <span class="kbd">Ctrl K</span> all commands</div></div>`, { name: 'Untitled project' }));

  reg('library', 'Home', 'Projects', () => pg('library', 'Projects', '', `
    <div class="row"><div style="flex:1;min-width:200px">${inp('', 'Search projects, transcripts and markers')}</div>${seg(['All', 'Music', 'Podcast', 'Spatial'])}</div>
    <div class="row">${chip('My Audio', 1)}${chip('Client work')}${chip('Archive')}${chip('+ Folder')}</div>
    <div class="tw"><table><thead><tr><th>Name</th><th>Type</th><th>Length</th><th>Size</th><th>Changed</th><th>Stored</th><th></th></tr></thead><tbody>
    ${[['Harbor Lights', 'Music', '3:42', '412 MB', '12 s ago', 'Folder'], ['Interview ep. 12', 'Podcast', '48:10', '1.2 GB', 'Yesterday', 'Folder'], ['Sunset 8D', 'Spatial', '4:18', '380 MB', 'Mon', 'Browser'], ['Rain voiceover', 'Voice', '1:06', '42 MB', '2 weeks ago', 'Browser'], ['Lecture 4 raw', 'Lecture', '1:52:14', '2.1 GB', 'Sep 18', 'Folder']].map(r => `<tr><td><a class="b7" href="#edit" data-go="edit" style="text-decoration:none">${r[0]}</a></td><td class="dim">${r[1]}</td><td class="mono">${r[2]}</td><td class="mono">${r[3]}</td><td class="dim">${r[4]}</td><td>${r[5] === 'Folder' ? pill('Folder', 'ok') : pill('Browser', 'warn')}</td><td>${btn('', { k: 'g', i: 'dots', a: 'More actions' })}</td></tr>`).join('')}</tbody></table></div>
    ${cal('warn', '<b>2 projects are only in this browser.</b> Browsers may clear storage when space is low. <a class="act" href="#export-project" data-go="export-project">Back them up as .flux files</a> or move them to a folder.', 'warn')}`, btn('Open .flux file', { i: 'folder' })));

  reg('import', 'Home', 'Importing a file', () => dlg('home', 'Adding harbor-lights.flac', `
    ${vz('wave', { seed: 1, stereo: 1, until: .62, color: '--t1', step: 2 }, 130, 'Waveform being read')}
    ${prog(62, 1)}
    <div class="g3 mono dim"><span>FLAC · 24-bit</span><span>44.1 kHz stereo</span><span>3:42.118</span></div>
    <div class="lst">${[['Reading audio', 'Done', 'check', 'ok'], ['Building waveform', '62%', 'exp', 'act'], ['Saving a safe copy in your project', 'Waiting', 'dots', 'faint'], ['Finding tempo and key', 'Waiting', 'dots', 'faint']].map(r => `<div><span class="${r[3]}">${ic(r[2], 17)}</span><div class="ti"><b>${r[0]}</b></div><span class="mono dim">${r[1]}</span></div>`).join('')}</div>
    ${cal('info', 'You can start editing now. Flux keeps preparing the rest in the background.')}`, `${btn('Cancel', { k: 'g', go: 'home' })}${btn('Open now', { k: 'p', go: 'edit' })}`));

  reg('record', 'Home', 'Record', () => pr('edit', `<div class="pg in" style="max-width:900px;margin:auto"><div class="between"><h1 class="h1">Recording.</h1><span class="pill bad"><span class="dot"></span>REC 00:01:24</span></div>
    <div class="vz" style="height:170px">${v('wave', { seed: 2, kind: 'voice', color: '--sig', until: .7, step: 2 })}</div>
    <div class="col"><div class="between lab"><span>Input level</span><span class="mono">−12.4 dB peak</span></div><div class="meter" style="width:100%"><div class="m" style="height:12px"><i data-lv></i></div><div class="m" style="height:12px"><i data-lv></i></div></div></div>
    <div class="g2">${fld('Microphone', sel(['Studio Mic USB', 'Laptop mic']))}${fld('Record to', sel(['New track', 'Selected track']))}</div>
    <div class="box g2">${tg('Hear yourself while recording', false, 'About 90 ms delay in the browser')}${tg('Raw signal', true, 'Browser noise reduction and auto gain off')}</div>
    ${cal('check', 'Recording is written to disk as it happens. A crash loses no more than the last second.')}
    <div class="row" style="justify-content:center">${btn('Stop', { k: 'p', lg: 1, i: 'stop', go: 'edit' })}${btn('Pause', { lg: 1, i: 'pause' })}${btn('Discard take', { k: 'g dng', lg: 1 })}</div></div>`));

  /* ---------- Simple task ---------- */
  const taskView = (word, sub, ctrl, adv) => `<div class="app notp">${Flux.topbar('harbor-lights.flac', '3:42 · stereo', `${seg(['Simple', 'Pro'], 0, 'data-mode aria-label="Mode"')}${btn('Export', { k: 'p', i: 'exp', go: 'export' })}`)}${Flux.rail(Flux.HOME, 'home')}<main class="main" id="m"><div class="pg in" style="max-width:1000px">
    <div><h1 class="bigh">${word}</h1><p class="lede">${sub}</p></div>
    <div class="vz" style="height:clamp(120px,24cqh,210px)">${v('wave', { seed: 1, stereo: 1, step: 2.5 })}</div>
    <div class="row"><button class="tbtn play" data-play aria-label="Play">${ic('play')}</button><span class="tc mono" data-tc style="font-size:19px">00:00:42.318</span><span class="sp" style="flex:1"></span><button class="btn lg hold" data-hold>Hold for original</button></div>
    ${ctrl}
    <div class="row">${chip('Make it 8D', word === '8D', 'task')}${chip('Slowed + reverb', word.startsWith('Slowed'), 'task-slowed')}${chip('Remove vocals', 0, 'stems-setup')}${chip('Clean up voice', 0, 'cleanup')}${chip('Remove noise', 0, 'fx-clean')}${chip('Convert', 0, 'convert')}</div>
    <a class="dim" href="#${adv}" data-go="${adv}">Open all the controls in Pro mode</a></div></main></div>`;
  reg('task', 'Home', 'Simple mode: Make it 8D', () => taskView('8D', 'Sound circles your head. Use headphones. <a class="act" href="#hpcheck" data-go="hpcheck">Check them</a>', `${sl('Rotation speed', 24, 'One turn every 10 s', { big: 1 })}<div class="g2">${sl('Distance', 58, '1.8 m')}${sl('Room', 40, 'Medium room')}</div>`, 'stage'));
  reg('task-slowed', 'Home', 'Simple mode: Slowed + reverb', () => taskView('Slowed + reverb', 'Slows the track, drops the pitch and fills the room.', `${sl('Amount', 58, '0.85× speed', { big: 1 })}<div class="g2">${tg('Keep the original pitch', false)}${sl('Reverb', 66, 'Hall 3.2 s')}</div>`, 'creative'));

  /* ---------- Edit ---------- */
  const clip = (l, w, c, seed, kind, label, o = {}) => `<div class="clip ${o.sel ? 'sel' : ''}" style="left:${l}%;width:${w}%;--c:var(${c});--co:var(${c}-on)">${v('wave', { seed, kind, color: c, on: 1, step: 2.5 })}${o.fi ? `<span class="fi" style="--w:${o.fi}px"></span>` : ''}${o.fo ? `<span class="fo" style="--w:${o.fo}px"></span>` : ''}<span class="cl">${label}</span></div>`;
  const track = (n, c, clips, o = {}) => `<div class="trk ${o.tall ? 'tall' : ''}"><div class="thd"><div class="nm"><span class="dot" style="color:var(${c})"></span><b>${n}</b></div><div class="ms"><button class="m ${o.m ? 'on' : ''}" data-toggle aria-label="Mute ${n}">M</button><button class="s" data-toggle aria-label="Solo ${n}">S</button><button aria-label="Arm ${n} for recording" data-toggle>R</button></div></div><div class="lane">${clips}${o.ex || ''}</div></div>`;
  const tracks = (auto) => `<div class="tl" data-tl><div class="ruler">${v('ruler')}</div>
    ${track('Vocals', '--t4', clip(4, 28, '--t4', 7, 'voice', 'Verse 1', { fi: 18 }) + clip(32.4, 22, '--t4', 8, 'voice', 'Chorus', { sel: 1, fi: 14, fo: 14 }) + clip(58, 34, '--t4', 9, 'voice', 'Verse 2', { fo: 24 }), { tall: 1, ex: auto ? `<svg class="autoline" viewBox="0 0 100 40" preserveAspectRatio="none" aria-hidden="true"><path d="M0 22 L18 22 C24 22 26 10 32 10 L52 10 C58 10 60 26 68 26 L100 26" fill="none" stroke="var(--sig)" stroke-width="1.8" vector-effect="non-scaling-stroke"/></svg>` : '' })}
    ${track('Drums', '--t3', clip(0, 96, '--t3', 3, 'drums', 'Drum stem'))}
    ${track('Bass', '--t2', clip(8, 88, '--t2', 4, 'bass', 'Bass stem'))}
    ${track('Keys', '--t1', clip(0, 44, '--t1', 5, 'pad', 'Rhodes') + clip(48, 48, '--t1', 6, 'pad', 'Rhodes 2'), { m: 1 })}
    ${track('Guitars', '--t5', clip(20, 36, '--t5', 10, 'mix', 'Lead take 3') + clip(62, 26, '--t5', 11, 'mix', 'Solo'))}
    ${track('Room tone', '--t6', clip(0, 100, '--t6', 12, 'pad', 'Ambience'))}
    <span class="ph" data-ph style="transform:translateX(120px)"></span></div>`;
  const edittools = `<div class="tlbar" role="toolbar" aria-label="Edit tools">${seg(['Select', 'Range', 'Slip', 'Fade'])}${btn('', { i: 'cut', k: 'g', a: 'Cut' })}${btn('', { i: 'copy', k: 'g', a: 'Copy' })}${btn('', { i: 'split', k: 'g', a: 'Split at playhead' })}${btn('', { i: 'trash', k: 'g', a: 'Delete' })}<span class="sp" style="flex:1"></span>${chip('Snap: zero cross', 1)}${chip('Grid: 1/4')}${chip('Ripple')}${btn('Automation', { k: 'g', go: 'automation' })}${btn('', { i: 'zoom', k: 'g', a: 'Zoom' })}</div>`;
  const insp = `<div class="between"><h3 class="h3">Chorus</h3>${pill('Vocals')}</div>
    <div class="g2 mono dim"><span>Start 0:42.318</span><span>Len 0:18.906</span></div>
    ${sl('Clip gain', 60, '+1.5 dB')}<div class="g2">${sl('Fade in', 12, '40 ms')}${sl('Fade out', 18, '120 ms')}</div>
    ${fld('Fade curve', seg(['Equal power', 'S', 'Log', 'Lin']))}
    ${sec('Effects on this clip', `<div class="lst">${[['De-esser', '−3.2 dB'], ['EQ: Clear voice', '5 bands'], ['Compressor', '3.2:1']].map(r => `<div><span class="act">${ic('fx', 16)}</span><div class="ti"><b>${r[0]}</b><span>${r[1]}</span></div><a class="btn g" href="#rack" data-go="rack">Open</a></div>`).join('')}</div>${btn('Add effect', { i: 'plus', go: 'fxbrowser' })}`)}
    ${sec('Quick', `<div class="row">${chip('Reverse')}${chip('Normalize')}${chip('Duplicate')}${chip('Bounce')}</div>`)}`;
  reg('edit', 'Workspaces', 'Edit: timeline', () => pr('edit', `<div class="split"><div class="a">${edittools}${tracks()}</div><aside class="b in">${insp}</aside></div>`));
  reg('automation', 'Workspaces', 'Edit: automation lanes', () => pr('edit', `<div class="split"><div class="a"><div class="tlbar">${seg(['Read', 'Write', 'Touch', 'Latch'])}${fld('', sel(['Vocals: Volume', 'Vocals: Pan', 'Reverb: Mix', 'Stage: Azimuth', 'Stage: Distance']))}${seg(['Linear', 'Smooth', 'Hold'])}<span class="sp" style="flex:1"></span>${btn('Done', { go: 'edit' })}</div>${tracks(true)}</div><aside class="b in"><h3 class="h3">Vocals: Volume</h3>${sl('Point value', 58, '−1.5 dB')}<div class="mono dim">At 0:58.240</div>${sec('Lanes on this track', `<div class="lst">${['Volume', 'Pan', 'Reverb mix'].map(n => `<div><div class="ti"><b>${n}</b></div>${btn('', { k: 'g', i: 'x', a: 'Remove lane' })}</div>`).join('')}</div>${btn('Add lane', { i: 'plus' })}`)}${cal('info', 'Draw on the lane, or play and move any control while Write is on.')}</aside></div>`));

  reg('spectral', 'Workspaces', 'Spectral: waveform and repair', () => pr('spectral', `<div class="split"><div class="a"><div class="tlbar">${seg(['Spectrogram', 'Waveform', 'Both'], 2)}${seg(['Rectangle', 'Lasso', 'Brush', 'Harmonic'])}<span class="sp" style="flex:1"></span>${chip('Mel')}${chip('Log', 1)}${chip('Linear')}${fld('', sel(['4096 window', '2048 window', '8192 window']))}</div>
    <div style="flex:1;display:flex;flex-direction:column;min-height:300px;position:relative"><div style="height:30%;min-height:80px;border-bottom:1px solid var(--ln)">${v('wave', { seed: 1, stereo: 1, step: 2 })}</div><div style="flex:1;position:relative;min-height:220px">${v('spec', { seed: 3 })}
    <div style="position:absolute;left:54%;top:26%;width:16%;height:26%;border:2px dashed var(--tx);border-radius:2px"><span class="mono" style="position:absolute;top:-22px;right:0;padding:1px 6px;border-radius:3px;background:var(--tx);color:var(--s0);white-space:nowrap">2.1 to 4.8 kHz · 1.42 s</span></div></div></div></div>
    <aside class="b in"><h3 class="h3">Selection</h3><div class="g2 mono dim"><span>2.1 to 4.8 kHz</span><span>1:02.1 to 1:03.5</span></div>
    ${sec('Repair', `<div class="col">${btn('Remove and fill in', { k: 'p', i: 'pen' })}${sl('Amount', 90, '−30 dB')}${fld('Fill from', seg(['Both sides', 'Before', 'After']))}</div>`)}
    ${sec('Also', `<div class="row">${chip('Attenuate')}${chip('Copy to track')}${chip('Listen to selection')}</div>`)}
    ${sec('Spectrum at playhead', vz('spectrum', {}, 110))}
    ${cal('info', 'Repairs are stored as edits. Your original audio stays unchanged.')}</aside></div>`));

  /* ---------- Mix ---------- */
  const ch = (n, c, val, pan, mst) => `<div class="ch ${mst ? 'mst' : ''}"><span class="dot" style="color:var(${c})"></span><b>${n}</b><div class="knob" style="--a:${pan}deg" role="img" aria-label="Pan"></div><a class="chip" href="#rack" data-go="rack" style="text-decoration:none;width:100%;justify-content:center">${mst ? 3 : 2} fx</a><div class="fader" style="--v:${val}%"><div class="ftk" role="slider" tabindex="0" aria-label="${n} level" aria-valuenow="${val}" aria-valuemin="0" aria-valuemax="100"><span class="fth"></span></div><div class="vu"><i data-lvv></i></div><div class="vu"><i data-lvv></i></div></div><span class="mono dim">${mst ? '−1.0' : (val > 70 ? '+' : '−') + Math.abs((val - 70) / 6).toFixed(1)} dB</span><div class="thd" style="position:static;padding:0;width:auto;background:none;border:0"><div class="ms">${mst ? '' : '<button class="m" data-toggle aria-label="Mute">M</button><button class="s" data-toggle aria-label="Solo">S</button>'}</div></div></div>`;
  reg('mix', 'Workspaces', 'Mix: mixer', () => pr('mix', `<div class="split"><div class="a"><div class="mixer in">${ch('Vocals', '--t4', 74, -6)}${ch('Drums', '--t3', 66, 0)}${ch('Bass', '--t2', 62, 0)}${ch('Keys', '--t1', 48, -38)}${ch('Guitars', '--t5', 54, 34)}${ch('Room', '--t6', 30, 0)}${ch('Music bus', '--t8', 68, 0)}${ch('Master', '--tx', 70, 0, 1)}</div></div><aside class="b in"><h3 class="h3">Master</h3>${sec('Loudness', `<div class="g2"><div><div class="stat">−14.2</div><span class="lab">Integrated LUFS</span></div><div><div class="stat">−1.1</div><span class="lab">True peak dBTP</span></div></div>`)}${sec('Stereo', vz('gonio', {}, 160, 'Goniometer'))}<div class="between"><span class="lab">Correlation</span><span class="mono">+0.62</span></div>${btn('Open all meters', { go: 'meters' })}</aside></div>`));

  /* ---------- History ---------- */
  reg('history', 'Workspaces', 'Visual history', () => pr('edit', `<div class="pg in"><div class="pgh"><div><h1 class="h1">History</h1><p class="lede">Every change, kept. Go back to any point, or branch off and keep both.</p></div>${btn('Save a snapshot', { i: 'disk' })}</div>
    <div class="vz" style="height:170px">${v('history')}</div>
    <div class="row"><span class="row"><span class="dot sig"></span>You are here</span><span class="row"><span class="dot" style="color:var(--t1)"></span>Branch: wider chorus</span><span class="row"><span class="dot" style="color:var(--t3)"></span>Branch: no guitars</span></div>
    <div class="lst box">${[['Move "Chorus" to 0:42.318', '12 s ago'], ['Fade out 120 ms on "Chorus"', '40 s ago'], ['De-esser added to Vocals', '3 min ago'], ['Split "Lead take 3" at 1:14.002', '6 min ago'], ['Snapshot: before mastering', '20 min ago'], ['Import harbor-lights.flac', 'Yesterday']].map((r, i) => `<div><div class="ti"><b>${r[0]}</b><span>${r[1]}</span></div>${btn(i === 0 ? 'Current' : 'Go here', { k: 'g' })}</div>`).join('')}</div></div>`));
})();
