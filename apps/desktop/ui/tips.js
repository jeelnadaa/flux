/* Tooltips for every control. Effect and parameter tips come from the engine (fx.catalog, enforced complete by a
   Rust test); the rest are here, keyed by what the control does. applyTips runs after every render: any control
   still without a tip gets one, so nothing is left bare. tests/e2e/test_live.py checks every screen. */
(function () {
  const F = FD, D = FD.D;
  const GO = {
    home: 'Your recent projects and quick starts.', live: 'Put a preset on your computer\'s sound or your microphone, live.', library: 'Every project on this computer.',
    new: 'Start a new project, empty or from a template.', record: 'Record from a microphone or other input.', edit: 'The timeline: arrange, cut and fade clips.',
    convert: 'Change files from one format to another, many at once.', batch: 'Run the same steps on many files.', ringtone: 'Make a ringtone from part of a song.',
    presets: 'Effect and chain presets, built in and your own.', macros: 'Record a series of steps once and replay it with a click.',
    help: 'Guides to every part of Flux.', 'help-agents': 'How to connect an AI agent to Flux over MCP.', palette: 'Search every screen and command. Ctrl K.',
    export: 'Save the finished sound as a file.', magic: 'Find and fix common problems in one go.', master: 'Make it loud and balanced for where it\'s going.',
    spectral: 'See frequencies over time and paint out noises.', mix: 'Levels, pan, mute and solo for each track.', stage: 'Place sounds around your head in 3D and 8D.',
    script: 'Edit speech by editing its words.', fxbrowser: 'Every effect, by group.', aitools: 'Stems, clean-up, mastering and other smart tools.', rack: 'The effects on this clip, track or the master, in order.',
    history: 'Every change, with snapshots you can go back to.', jobs: 'Long tasks running in the background.', stems: 'Split a song into vocals, drums, bass and the rest.',
    'stems-setup': 'Choose how to split the song.', cleanup: 'Make a voice recording clearer.', tts: 'Turn text into speech with your system voices. Always labelled synthetic.',
    analysis: 'Find the tempo and key.', 'fx-clean': 'Remove noise, hum, clicks and wind.', 'fx-silence': 'Find and shorten long silences.', creative: 'Slowed, nightcore, lo-fi and other looks.',
    rooms: 'Put the sound in a real-sounding room.', roomdesign: 'Design a room by its size and surfaces.', clone: 'Voice cloning and what it needs.', dub: 'Translate speech and dub it.',
    fillers: 'Find ums and uhs to remove.', diag: 'A report to help fix a problem. Contains no audio or file names.', trouble: 'Fixes for common problems.',
    welcome: 'Back to the welcome screen.', hpcheck: 'Check your headphones are the right way round.', hrtf: 'Pick the head shape 3D sound is tuned for.',
    'export-project': 'Save the whole project as one .flux file to move or share.', 'storage-ask': 'Where Flux keeps your work.', task: 'Simple mode: one task, a few big controls.',
    'task-slowed': 'Simple mode: slowed + reverb.', legal: 'What Flux does with your data: nothing leaves this computer unless you choose.', 'legal-terms': 'The terms for using Flux.', 'legal-credits': 'Open-source software Flux uses.',
  };
  const ACT = {
    'action-record': 'Start recording your steps as a reusable action.', 'add-folder': 'Make a folder to group projects.', 'batch-add': 'Add files to process.', 'batch-start': 'Run the steps on every file.',
    'clean-tab': 'Show this kind of clean-up.', 'conv-add': 'Add files to convert.', 'conv-set': 'Use this setting for every file.', 'conv-start': 'Convert every file in the list.',
    copy: 'Copy to the clipboard.', 'copy-version': 'Copy the version, for a bug report.', 'diag-save': 'Save the diagnostic report as a file.', 'dyn-tab': 'Show this dynamics effect.',
    'empty-trash': 'Permanently delete everything in the trash.', 'export-stems': 'Export each track as its own file.', 'export-start': 'Write the file with these settings.',
    'fx-seed': 'A new random pattern for this effect.', 'gate-step': 'Turn this step of the gate on or off.', 'hp-answer': 'Tell Flux what you heard.', hrtf: 'Use this head shape for 3D sound.',
    import: 'Open audio files from this computer.', 'import-package': 'Open a .flux project file.', 'job-cancel': 'Stop this task.', 'key-edit': 'Press the keys you want for this command.',
    'learn-noise': 'Play a part with only the noise, and Flux learns what to remove.', 'lib-folder': 'Show only projects in this folder.', 'match-eq': 'Make the tone match another file.',
    onboarded: 'Finish setting up and start using Flux.', 'open-project': 'Open this project.', 'pick-export-dir': 'Choose where exported files go.', 'preset-apply': 'Apply this preset.',
    'preset-import': 'Add a preset file.', 'preset-save-chain': 'Save every effect here as one preset.', 'preset-save': 'Save these settings as a preset.', 'project-delete': 'Move this project to the trash.',
    'project-dup': 'Make a copy of this project.', 'project-menu': 'More things to do with this project.', 'project-new': 'Create the project.', 'project-package': 'Save the project as one .flux file.',
    'rec-start': 'Start recording.', 'rec-stop': 'Stop and keep the recording.', 'rec-pause': 'Pause or carry on recording.', 'rec-discard': 'Stop and throw the recording away.', recheck: 'Check this computer again.',
    'recovery-dismiss': 'Close without recovering.', 'recovery-open': 'Open the recovered project.', redo: 'Redo the last change you undid. Ctrl Shift Z.', undo: 'Undo the last change. Ctrl Z.', reveal: 'Show the file in its folder.',
    'room-use': 'Use this room on the sound.', 'rv-tab': 'Show this kind of space.', set: 'Use this setting.', 'set-mode': 'Simple shows one task at a time; Pro shows everything.',
    'sil-scan': 'Find the silences again with these settings.', 'sil-apply': 'Shorten the silences you kept ticked.', 'sil-keep': 'Keep this silence as it is.', 'snapshot-save': 'Save how the project is now, to come back to.',
    'snapshot-restore': 'Go back to this snapshot. You can undo it.', 'history-goto': 'Go back to this point. You can redo.', 'stage-sync': 'Lock the movement to the tempo.', 'stems-start': 'Start splitting.',
    'style-toggle': 'Add or remove this look.', 'tap-tempo': 'Tap in time with the music to set the tempo.', 'tapestop-here': 'Put the tape stop at the playhead.', 'texture-pick': 'Show this texture.',
    tone: 'Play a test sound.', 'tp-loop': 'Repeat the selection while playing.', 'tp-start': 'Back to the start.', 'tp-stop': 'Stop and go back to where you started.', tpl: 'Start from this template.',
    'tts-speak': 'Make the speech. It is labelled synthetic.', 'ui-set': 'Show this.', 'ui-toggle': 'Turn this on or off.', 'update-check': 'Look for a newer version now.', 'update-install': 'Restart Flux to finish updating.',
    'voice-monitor': 'Hear yourself through the voice effect.', 'edit-split': 'Split the clip at the playhead. S.', 'edit-delete': 'Delete the selected clip.', 'edit-copy': 'Copy the selected clip.', 'edit-cut': 'Cut the selected clip.',
    zoom: 'Zoom in on the timeline.', 'track-add': 'Add a track.', 'track-remove': 'Remove this track.', 'track-flag': 'Mute, solo or arm this track.', 'marker-add': 'Add a marker at the playhead.',
    'clip-reverse': 'Play this clip backwards.', 'clip-normalize': 'Make this clip\'s loudest peak just below full scale.', 'clip-dup': 'Duplicate this clip.', 'clip-bounce': 'Print the clip\'s effects into new audio.',
    'set-target': 'Edit effects on this clip, track or the master.', 'fx-remove': 'Remove this effect.', 'fx-move': 'Move this effect in the chain. Order changes the sound.', 'fx-enable': 'Turn this effect on or off.',
    'open-fx': 'Open this effect\'s settings.', 'spec-repair': 'Fill the selection from the sound around it.', 'spec-attenuate': 'Turn the selection down.', 'spec-listen': 'Hear only the selection.', 'spec-clear': 'Clear the selection.',
    'magic-apply': 'Apply the fixes that are ticked.', 'magic-toggle': 'Include or skip this fix.', 'cleanup-apply': 'Clean up the voice.', 'master-apply': 'Master with these settings.', 'tempo-apply': 'Set the project tempo and key to these.',
    'tempo-detect': 'Detect again.', 'tempo-mul': 'Halve or double the tempo.', 'action-run': 'Run this action.', 'action-delete': 'Delete this action.', 'action-stop': 'Stop recording and name the action.',
    'ring-find': 'Find the catchiest part.', 'ring-export': 'Save the ringtone.', 'ring-phone': 'Fit the length and format to this phone.', 'exp-dest': 'Use settings for where it\'s going.', 'exp-toggle': 'Turn this on or off for the export.',
    'files-remove': 'Remove this file from the list.', 'batch-remove': 'Remove this step.', 'batch-remove-file': 'Remove this file.', 'error-close': 'Close this message.', 'error-retry': 'Try again.', relink: 'Find the missing file.',
    'macro-map': 'Let the macro knob move the selected parameter.', 'macro-unmap': 'Stop the macro moving this.', 'reverse-apply': 'Reverse the clip and add a swell into it.', 'stage-bake': 'Write the movement as automation.',
    'open-url': 'Open in your browser.', 'fx-open-kind': 'Open this effect.', 'fx-add': 'Add this effect.', 'eq-show': 'Switch between the parametric and graphic EQ.',
  };
  const SET = {
    crash_reports: 'Send a crash report if Flux crashes. Reports never contain audio, transcripts or file names. Off by default.', screen_reader_hints: 'Screen readers describe waveforms and meters in words.',
    high_contrast: 'Stronger lines and text.', large_text: 'Makes all text bigger.', mcp_enabled: 'Lets AI agents on this computer drive Flux through MCP. Off means agents can\'t reach the app.',
    safety_limiter: 'A limiter on the output so a mistake can\'t blast your ears or speakers.', reduce_motion: 'Turns off animation.', autosave: 'Saves every change as you make it.', auto_update: 'Downloads new versions in the background and installs them when you restart.',
    export_dir: 'Where exported files go.', input: 'The microphone or input Flux records from.', output: 'Where Flux plays sound.', buffer: 'Smaller is quicker to respond; larger is safer from dropouts.',
    default_rate: 'The sample rate new projects start with.', language: 'The language Flux uses.', head_width_cm: 'Your head width, for 3D sound tuned to you.', theme: 'Flux\'s colors.', font: 'Flux\'s typeface.',
  };
  const UI = {
    grid: 'Snap and show a beat grid.', monitor: 'Hear the input while recording. Use headphones.', ripple: 'Moving or deleting a clip moves everything after it.', roomLink: 'Change all surfaces together.',
    snap: 'Clips snap to the grid and to each other.', specAfter: 'Show the result.', specBefore: 'Show the original.', stemMute: 'Mute the original track after splitting.', swell: 'Add a reversed swell leading in.',
    cleanStrength: 'How hard the clean-up works. Higher removes more noise but can sound processed.', masterAmt: 'How much the master changes the sound.', repairDb: 'How far the selection is turned down.',
    swellBeats: 'How long the swell is, in beats.', ttsRate: 'How fast the voice speaks.', threshold_db: 'Quieter than this counts as silence.', longer_than_s: 'Only silences longer than this are shortened.',
    shorten_to_s: 'What each long silence becomes.', padding_s: 'Sound kept either side of a silence, so words aren\'t clipped.', w: 'Room width.', d: 'Room depth.', h: 'Room height.', action: 'Choose an action.',
    fxfilter: 'Show only this group.', guide: 'Show this guide.', rvTab: 'Show this kind of space.', ttsVoice: 'The voice that speaks.',
  };
  const LABEL = {
    'search commands': 'Search every screen and command.', 'search effects': 'Find an effect by name.', 'search help': 'Search the guides.', 'search presets': 'Find a preset by name.',
    'export folder': 'Where exported files go.', 'file name': 'The exported file\'s name.', name: 'A name for this.', seed: 'Starting point for anything random. Same seed, same result.', 'glitch seed': 'Starting point for the glitch pattern. Same seed, same pattern.',
    tempo: 'Beats per minute.', 'add a step': 'Add a step to run on every file.', format: 'The file type.', quality: 'Higher sounds better and makes a bigger file.', 'sample rate': 'Samples per second. 48 kHz suits video; 44.1 kHz suits music.',
    folder: 'Which folder it goes in.', key: 'The musical key.', scale: 'Which notes the voice snaps to.', 'record to': 'Record onto a new track or the selected one.', 'save to': 'Where the file goes.',
    'listen to': 'What you hear while it works.', ceiling: 'The highest the peaks can go.', floor: 'How far noise is turned down.', walls: 'What the walls are made of. Hard walls ring longer.',
    'search projects and files': 'Find a project by name.', 'search projects and markers': 'Find a project by name, or by a marker inside it.', 'search projects': 'Find a project by name.', 'keep the original pitch': 'Change the speed without changing the pitch.',
    'record movement as automation': 'Write the movement into an automation lane as you move it.', continue: 'Go on to the next step.', 'type a command': 'Search every screen and command.',
    current: 'The effect you are editing now.', script: 'Export the words as a text file.',
    microphone: 'The microphone Flux records from.', output: 'Where Flux plays sound.', buffer: 'Smaller is quicker to respond; larger is safer from dropouts.', language: 'The language Flux uses.', 'new project sample rate': 'The sample rate new projects start with.',
  };
  const SEG = {
    'set-mode': 'Simple shows one task at a time; Pro shows everything.', mode: 'Simple shows one task at a time; Pro shows everything.', 'preset-kind': 'Show this kind of preset.', 'lib-kind': 'Show projects this way.',
    'stage-tool': 'Drag sounds, or draw a path for them to follow.', 'stem-mode': 'How many parts to split into.', 'target-kind': 'Edit effects on a clip, a track or the master.', 'eq-show': 'Switch between the parametric and graphic EQ.',
    audio: 'Which audio to use.', detect: 'What to look for.', 'eq mode': 'Switch between the parametric and graphic EQ.', 'final render': 'How carefully to render the final file.', 'reverse what': 'Reverse the clip or only part of it.',
    'run on': 'Run on this computer\'s processor or graphics chip.', 'settings slot': 'Three slots to compare settings. Click to switch.',
  };
  const low = s => (s || '').trim().toLowerCase();
  const effectTip = k => { const s = k && F.spec(k); return s && s.tip; };
  const paramTip = (k, key) => { const s = k && F.spec(k), p = s && s.params.find(q => q.key === key); return p && p.tip; };
  const kindOf = d => d.kind || (d.fx && (F.findFx(+d.fx) || {}).kind);
  function bindTip(b) {
    const [ns, a, c] = b.split(':');
    if (ns === 'fx' || ns === 'fxk') return paramTip(ns === 'fx' ? (F.findFx(+a) || {}).kind : a, c);
    if (ns === 'cl') return (Flux.cloneTip && Flux.cloneTip(b)) || '';
    if (ns === 'fxmix' || ns === 'fxkmix') return 'Balance between the original and the effect.';
    if (ns === 'clip') return a === 'gain_db' ? 'This clip\'s level.' : 'This clip\'s ' + String(c || a).replace(/_/g, ' ') + '.';
    if (ns === 'track') return 'This track\'s level.';
    if (ns === 'master') return 'The level of everything together.';
    if (ns === 'set') return SET[a];
    if (ns === 'style') return 'How much of this look.';
    if (ns === 'macro') return 'Moves every parameter you mapped to it.';
    if (ns === 'exp') return 'Export setting: ' + a.replace(/_/g, ' ') + '.';
    return UI[a] || (ns === 'ring' ? { start: 'Where the ringtone starts.', len: 'How long it is.', fi: 'Fade in length.', fo: 'Fade out length.' }[a] : '');
  }
  function textOf(c) { const sp = c.querySelector && c.querySelector('span:not(.ind) + span, small, b + span'); return sp && sp.textContent.trim().length > 12 ? sp.textContent.trim() : ''; }
  function tipFor(c) {
    const d = c.dataset, bind = c.closest('[data-bind]');
    if (bind) return bindTip(bind.dataset.bind);
    if (d.setTheme) return `Switch Flux to the ${c.getAttribute('aria-label') || d.setTheme} theme.`;
    if (d.setFont) return 'Use this typeface everywhere in Flux.';
    if (d.k && (d.fx || d.kind) && /^fx-/.test(d.act || '')) return paramTip(kindOf(d), d.k);
    if (d.fxsel || d.fxinput) return paramTip(kindOf(d), d.fxsel || d.fxinput);
    if (d.act === 'set-toggle' || d.act === 'set') return SET[d.k];
    if (d.setsel) return SET[d.setsel];
    if ((d.act === 'ui-toggle' || d.act === 'ui-set') && UI[d.k]) return UI[d.k];
    if (d.ui && UI[d.ui]) return UI[d.ui];
    if (d.room) return UI[d.room] || 'Room setting.';
    const seg = c.closest('.seg');
    if (seg) { const sk = seg.dataset.segact, t = SEG[sk] || SEG[low(seg.getAttribute('aria-label'))]; const pk = seg.dataset.fx || seg.dataset.kind ? paramTip(kindOf(seg.dataset), seg.dataset.k) : ''; return pk || t || 'Choose ' + c.textContent.trim() + '.'; }
    if (d.kind && /fx-open-kind|fx-add|texture-pick/.test(d.act || '')) return effectTip(d.kind);
    if (d.act && ACT[d.act]) return ACT[d.act];
    if (d.go) return GO[d.go] || textOf(c) || (Flux.M[d.go] ? 'Open ' + Flux.M[d.go].name + '.' : '');
    if (c.hasAttribute('data-play')) return 'Play or pause. Space.';
    if (c.hasAttribute('data-hold')) return 'Hold to hear the original at the same loudness.';
    const lb = c.getAttribute('aria-labelledby'), lbt = lb && document.getElementById(lb) ? document.getElementById(lb).textContent : '';
    const al = low(c.getAttribute('aria-label') || lbt || (c.labels && c.labels[0] && c.labels[0].textContent) || c.getAttribute('placeholder') || '');
    if (LABEL[al]) return LABEL[al];
    const tx = low(c.textContent);
    if (LABEL[tx]) return LABEL[tx];
    if (UI[al]) return UI[al];
    if (c.getAttribute('role') === 'slider' && al) return 'Drag, or use the arrow keys, to set ' + al + '.';
    if (/^band \d type$/.test(al)) return 'The shape of this EQ band: bell, shelf, cut or notch.';
    return textOf(c) || c.getAttribute('aria-label') || '';
  }
  Flux.applyTips = root => {
    root.querySelectorAll('[title]').forEach(e => { if (!e.dataset.tip) e.dataset.tip = e.title; e.removeAttribute('title'); });
    root.querySelectorAll('button, a[href], select, input, textarea, [role=slider]').forEach(c => {
      if (c.closest('[data-tip]')) return;
      const host = c.closest('.sl, .fader, .fld, .tg') || c, t = tipFor(c) || tipFor(host) || (c.textContent || '').trim();
      if (t) host.dataset.tip = t;
    });
  };
})();
