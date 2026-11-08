/* Live: computer sound and a microphone through live presets, played to any output.
   Not in design/: composed from the kit (pg, box, sec, sl, seg, tg, sel, fxc, meter) and tokens; see design/LIVE.md.
   Every control carries a tooltip (data-tip) from live.kinds, which the engine keeps complete (a test enforces it). */
(function () {
  const { reg, pg, btn, seg, sl, tg, pill, fld, inp, sel, sec, cal, fxc, ic, THEMES, dlg } = Flux, F = FD, D = FD.D, esc = F.esc;
  const call = (m, p) => F.call(m, p);
  const U = () => D.ui.live = D.ui.live || { tab: 'output', q: '', route: {}, err: {} };
  const TIP = (g, k) => { const t = D.lkinds && D.lkinds.tips; return (t && t[g] && t[g][k]) || ''; };
  const tip = t => t ? ` data-tip="${esc(t)}"` : '';
  const tipd = (h, t) => t ? h.replace(/^<(\w+)/, `<$1 data-tip="${esc(t)}"`) : h;
  const kinds = () => (D.lkinds && D.lkinds.kinds) || [];
  const spec = k => kinds().find(s => s.kind === k);
  const pspec = (k, key) => { const s = spec(k); return s && s.params.find(p => p.key === key); };
  const P = () => D.lpreset;
  const allMods = p => p ? p.lanes.flatMap(l => l.modules).concat(p.post) : [];
  const modOf = id => allMods(P()).find(m => m.id === id);
  const pv = (m, key) => m.params && key in m.params ? m.params[key] : (pspec(m.kind, key) || {}).default;
  const cap1 = s => s ? s.charAt(0).toUpperCase() + s.slice(1) : s;
  const ROUTE = { output: 'Computer sound', input: 'Microphone' };
  const KIND = { output: 'Computer sound', input: 'Microphone', any: 'Either' };

  /* ranges for everything that isn't an effect parameter */
  const pc = v => Math.round(v * 100) + '%', spc = v => F.minus((v > 0 ? '+' : '') + Math.round(v * 100)) + '%';
  const dB = v => F.db(v, 1) + ' dB', bal = v => Math.abs(v) < .005 ? 'Center' : (v < 0 ? 'L ' : 'R ') + Math.round(Math.abs(v) * 100);
  const ms = v => v >= 1000 ? (v / 1000).toFixed(2) + ' s' : (v < 10 ? v.toFixed(1) : Math.round(v)) + ' ms';
  const RG = {
    input_gain_db: { min: -24, max: 24, f: dB, step: .1 }, output_gain_db: { min: -24, max: 24, f: dB, step: .1 },
    width: { min: 0, max: 2, f: v => v.toFixed(2) + '×', step: .01 }, balance: { min: -1, max: 1, f: bal, step: .01 },
    tempo: { min: 30, max: 300, f: v => Math.round(v) + ' BPM', int: 1 },
    gain_db: { min: -60, max: 12, f: dB, step: .1 }, pan: { min: -1, max: 1, f: bal, step: .01 }, mix: { min: 0, max: 1, f: pc, step: .01 },
    rate_hz: { min: .01, max: 40, log: 1, f: v => (v < 1 ? v.toFixed(2) : v.toFixed(1)) + ' Hz' },
    depth: { min: 0, max: 1, f: pc, step: .01 }, offset: { min: -1, max: 1, f: spc, step: .01 }, phase: { min: 0, max: 1, f: pc, step: .01 }, smooth: { min: 0, max: 1, f: pc, step: .01 },
    attack_ms: { min: .1, max: 2000, log: 1, f: ms }, release_ms: { min: 1, max: 5000, log: 1, f: ms },
    value: { min: 0, max: 1, f: pc, step: .001 }, amount: { min: -1, max: 1, f: spc, step: .01 }, curve: { min: -1, max: 1, f: v => F.minus(v.toFixed(2)), step: .01 },
  };
  const pctOf = (v, r) => Math.max(0, Math.min(100, r.log ? Math.log(v / r.min) / Math.log(r.max / r.min) * 100 : (v - r.min) / (r.max - r.min) * 100));
  const valOf = (x, r) => { x = Math.min(100, Math.max(0, x)) / 100; let v = r.log ? r.min * Math.pow(r.max / r.min, x) : r.min + (r.max - r.min) * x; if (r.int) v = Math.round(v); else if (r.step) v = Math.round(v / r.step) * r.step; return +v.toFixed(4); };
  const rsl = (label, key, v, bind, t) => tipd(sl(label, pctOf(v, RG[key]), RG[key].f(v), { bind: 'lv:' + bind }), t);

  /* ---------- editing ---------- */
  let owning = null;
  /* Built-ins never change: the first edit makes the person's own copy and carries on there. */
  function own() {
    const p = P(); if (!p) return Promise.reject(new Error('No preset'));
    if (!p.builtin) return Promise.resolve(p.id);
    if (!owning) owning = call('live.preset.create', { from: p.id }).then(np => { if (U().edit && !U().edit.copy) U().edit.copy = np.id; U().note = `“${p.name}” is built in, so Flux made your own copy to change: “${np.name}”.`; U().sel = np.id; D.lpreset = np; useFor(np); F.stale('lpresets'); F.need(['lpresets']).then(Flux.rr); owning = null; return np.id; }, e => { owning = null; throw e; });
    return owning;
  }
  function err(e, where) { U().err[where || 'ed'] = (e && e.message) || String(e); Flux.rr(); }
  async function ed(m, params, o = {}) {
    try {
      U().err.ed = null; const id = await own(); const r = await call(m, Object.assign({ preset: id }, params));
      if (r && r.preset) D.lpreset = r.preset; if (o.list !== false) { F.stale('lpresets'); await F.need(['lpresets']); }
      Flux.rr(); return r;
    } catch (e) { err(e); return null; }
  }
  /* Slider drags: the latest value per control, at most every 40 ms, in order. */
  const pend = new Map(); let lastSend = 0, chain = Promise.resolve();
  function push(key, m, params, end) {
    pend.set(key, [m, params]);
    if (end || performance.now() - lastSend > 40) {
      lastSend = performance.now(); const batch = [...pend.values()]; pend.clear();
      chain = chain.then(async () => { const id = await own(); let r = null; for (const [mm, pp] of batch) r = await call(mm, Object.assign({ preset: id }, pp)); if (end && r && r.preset) { D.lpreset = r.preset; Flux.rr(); } }).catch(e => err(e));
    }
  }
  /* Send anything still waiting, and wait for every edit to land. */
  async function flush() {
    if (pend.size) { const b = [...pend.values()]; pend.clear(); chain = chain.then(async () => { const id = await own(); for (const [mm, pp] of b) await call(mm, Object.assign({ preset: id }, pp)); }); }
    await chain.catch(() => { }); if (owning) await owning.catch(() => { });
  }
  Flux.liveBind = (parts, pct, end) => {
    const p = P(); if (!p) return '';
    const [t, a, b, c] = parts;
    if (t === 'p') { const v = valOf(pct, RG[a]); p[a] = v; push('p:' + a, 'live.preset.update', { [a]: v }, end); return RG[a].f(v); }
    if (t === 'mp') { const mo = modOf(a), q = mo && pspec(mo.kind, b); if (!q) return ''; const v = +F.unpct(q, pct).toFixed(5); mo.params[b] = v; push('mp:' + a + ':' + b, 'live.module.set', { module: a, params: { [b]: v } }, end); return F.fmtp(q, v); }
    if (t === 'mx') { const mo = modOf(a); if (!mo) return ''; const v = valOf(pct, RG.mix); mo.mix = v; push('mx:' + a, 'live.module.set', { module: a, mix: v }, end); return pc(v); }
    if (t === 'lg' || t === 'lp') { const ln = p.lanes.find(l => l.id === a), k = t === 'lg' ? 'gain_db' : 'pan'; if (!ln) return ''; const v = valOf(pct, RG[k]); ln[k] = v; push(t + ':' + a, 'live.lane.set', { lane: a, [k]: v }, end); return RG[k].f(v); }
    if (t === 'mo') { const mo = p.mods.find(x => x.id === a); if (!mo) return ''; const v = valOf(pct, RG[b]); mo[b] = v; push('mo:' + a + ':' + b, 'live.mod.set', { mod: a, [b]: v }, end); return RG[b].f(v); }
    if (t === 'mt') { const mo = p.mods.find(x => x.id === a); if (!mo) return ''; const v = valOf(pct, RG.amount), tg0 = mo.targets.find(x => x.module === b && x.param === c); if (tg0) tg0.amount = v; push(parts.join(':'), 'live.mod.target', { mod: a, module: b, param: c, amount: v || .001 }, end); return spc(v); }
    if (t === 'mv') { const mc = p.macros.find(x => x.id === a); if (!mc) return ''; const v = valOf(pct, RG.value); mc.value = v; push('mv:' + a, 'live.macro.set', { macro: a, value: v }, end); return pc(v); }
    if (t === 'mc') { const mc = p.macros.find(x => x.id === a), tt = mc && mc.targets[+b]; if (!tt) return ''; const v = valOf(pct, RG.curve); tt.curve = v; push('mc:' + a, 'live.macro.set', { macro: a, targets: mc.targets }, end); return RG.curve.f(v); }
    return '';
  };

  /* ---------- routes ---------- */
  const presetsFor = route => (D.lpresets || []).filter(p => p.kind === route || p.kind === 'any');
  function form(route) {
    const u = U(), st = D.lstatus || {}, last = (st.last || {})[route] || {}, dv = D.ldevices || { outputs: [], inputs: [] };
    const defOut = (dv.outputs.find(o => o.default) || dv.outputs[0] || {}).id, defIn = (dv.inputs.find(o => o.default) || dv.inputs[0] || {}).id;
    const mon = st.capture && st.capture.system === 'monitor';
    const vd = dv.flux || {}, flux = route === 'output' ? !!(vd.speaker || vd.can_create) : !!(vd.mic || vd.can_create);
    const def = route === 'output' && flux ? { source: 'flux', source_device: undefined, output: 'auto', preset: (presetsFor('output')[0] || {}).id }
      : route === 'input' && flux ? { source: 'input', source_device: defIn, output: 'flux', preset: (presetsFor('input')[0] || {}).id }
      : route === 'output'
      ? { source: 'system', source_device: mon ? (dv.inputs.find(i => i.monitor) || {}).id : defOut, output: (dv.outputs.find(o => o.id !== defOut) || {}).id || 'none', preset: (presetsFor('output')[0] || {}).id }
      : { source: 'input', source_device: defIn, output: defOut || 'none', preset: (presetsFor('input')[0] || {}).id };
    const f = u.route[route] = Object.assign(def, last, u.route[route] || {});
    if (/^demo|^flux/.test(f.source)) f.source_device = undefined;
    if (route === 'output' && f.source === 'flux' && !f.output) f.output = 'auto';
    return f;
  }
  const running = route => { const s = ((D.lstatus || {}).routes || {})[route]; return s && s.running ? s : null; };
  function useFor(p) {
    const route = p.kind === 'any' ? U().tab === 'input' ? 'input' : 'output' : p.kind; form(route).preset = p.id;
    if (running(route)) call('live.set', { route, preset: p.id }).then(s => { pollNow(); }, e => err(e, route));
  }
  function sourceOpts(route) {
    const dv = D.ldevices || { outputs: [], inputs: [] }, mon = ((D.lstatus || {}).capture || {}).system === 'monitor';
    if (route === 'input') return dv.inputs.filter(i => !i.monitor).map(i => ['input|' + i.id, esc(i.name) + (i.default ? ' (default)' : '')]).concat([['demo-voice|', 'Test voice (built in)']]);
    const vd = dv.flux || {}, fx = vd.speaker || vd.can_create ? [['flux|', 'Flux speaker' + (vd.speaker && vd.speaker.kind === 'cable' ? ' (' + esc(vd.speaker.label) + ')' : '') + ': apps you send to it']] : [];
    const sys = mon ? dv.inputs.filter(i => i.monitor).map(i => ['system|' + i.id, esc(i.name)]) : dv.outputs.map(o => ['system|' + o.id, 'What plays on ' + esc(o.name) + (o.default ? ' (default)' : '')]);
    return fx.concat(sys, [['demo-music|', 'Test music (built in)']]);
  }
  const lvl = (route, io, label) => `<div class="row" style="gap:8px;flex-wrap:nowrap"><span class="lab" style="width:28px">${label}</span><div class="meter" style="flex:1;width:auto" role="img" aria-label="${ROUTE[route]} level ${label === 'In' ? 'coming in' : 'going out'}"><div class="m"><i data-lvm="${route}:${io}"></i></div></div><span class="mono dim" style="width:64px;text-align:right" data-lvd="${route}:${io}">−∞</span></div>`;
  function routeCard(route) {
    const f = form(route), s = running(route), dv = D.ldevices || { outputs: [] }, cap = (D.lstatus || {}).capture || {}, e = U().err[route];
    const vd = dv.flux || {}, real = dv.outputs.filter(o => !o.virtual).map(o => [o.id, esc(o.name) + (o.default ? ' (default)' : '')]);
    const outs = (route === 'output' && f.source === 'flux' ? [['auto', 'Your usual output']] : route === 'input' && (vd.mic || vd.can_create) ? [['flux', 'Flux microphone' + (vd.mic && vd.mic.kind === 'cable' ? ' (' + esc(vd.mic.label) + ')' : '') + ': for call apps']] : []).concat(real, [['none', 'Nowhere (meters only)']]);
    const pick = route === 'output' ? f.source === 'flux' && vd.speaker && `In your music app or the system sound settings, choose ${esc(vd.speaker.kind === 'cable' ? vd.speaker.play.name : vd.speaker.label)} as the output. You hear only the processed sound.` : f.output === 'flux' && vd.mic && `In your call app, choose ${esc(vd.mic.kind === 'cable' ? vd.mic.rec.name : vd.mic.label)} as the microphone.`;
    const ps = presetsFor(route).map(p => [p.id, esc(p.name) + (p.builtin ? '' : ' · yours')]);
    const t = TIP('route', route);
    return `<div class="box col" data-route="${route}"${s ? ' style="box-shadow:inset 0 0 0 2px var(--tx)"' : ''}>
      <div class="between"><div class="col" style="gap:2px"><b class="h3"${tip(t)}>${ROUTE[route]}</b><span class="dim" style="font-size:13px">${route === 'output' ? 'What your computer plays, through a preset, out to your headphones or speakers.' : 'Your microphone through a preset, out to any output.'}</span></div>${s ? pill(s.bypass ? 'Bypassed' : 'Live', s.bypass ? '' : 'sig') : pill('Off')}</div>
      ${tipd(fld('Sound from', sel(sourceOpts(route), f.source + '|' + (f.source_device || ''), `data-lchange="route" data-route="${route}" data-k="src" aria-label="${ROUTE[route]}: sound from"`)), route === 'output' ? cap.note : 'The microphone or input Flux listens to.')}
      ${tipd(fld('Preset', sel(ps, f.preset, `data-lchange="route" data-route="${route}" data-k="preset" aria-label="${ROUTE[route]}: preset"`)), 'The live preset the sound goes through. Changing it while live switches smoothly.')}
      ${tipd(fld('Play to', sel(outs, f.output, `data-lchange="route" data-route="${route}" data-k="output" aria-label="${ROUTE[route]}: play to"`)), 'Where the processed sound plays. Pick a different output from the one you capture, or it would feed back.')}
      <div class="row">${s ? btn('Stop', { i: 'stop', x: `data-lact="stop" data-route="${route}"${tip('Stop this route. The sound plays as normal again.')}` }) : btn('Start', { k: 'p', i: 'play', x: `data-lact="start" data-route="${route}"${tip('Start listening and processing live.')}` })}
      ${tg('Bypass', s && s.bypass, '', `data-lact="bypass" data-route="${route}"${s ? '' : ' disabled'}${tip('Hear the sound without the preset, to compare. Crossfades, so there are no clicks.')}`)}
      ${btn('Edit preset', { k: 'g', i: 'pen', x: `data-lact="editp" data-id="${esc(f.preset || '')}"${tip('Open this preset in the editor.')}` })}</div>
      <div class="col" style="gap:6px">${lvl(route, 'in', 'In')}${lvl(route, 'out', 'Out')}</div>
      <span class="mono dim" style="font-size:12px" data-lvs="${route}">${s ? stats(s) : 'Not running'}</span>
      ${e ? cal('warn', esc(e), 'warn') : ''}
      ${vd.status === 'missing' ? `<div class="row" style="gap:8px;flex-wrap:wrap" data-lv="missing-${route}"><span class="dim" style="font-size:12.5px;flex:1 1 200px">${route === 'output' ? 'No virtual cable found, so the original sound keeps playing too.' : 'No virtual cable found, so call apps can\'t use this yet.'}</span>${btn('How to install', { k: 'g', x: `data-lact="vguide"${tip('Step-by-step instructions for this computer.')}` })}</div>` : ''}
      ${pick ? `<p class="dim" style="font-size:12.5px;margin:0" data-lv="pick-${route}"${tip(cap.apps_note)}>${pick}</p>` : ''}
    </div>`;
  }
  /* ---------- installing a virtual cable (kit dlg, list rows and callouts; see design/LIVE.md) ---------- */
  const VOS = { windows: 'Windows', macos: 'macOS', linux: 'Linux' };
  const link = (t, url, why) => btn(t, { i: 'globe', x: `data-act="open-url" data-url="${esc(url)}"${tip(why)}` });
  const cmd = c => `<div class="row" style="gap:8px;flex-wrap:wrap"><code class="mono" style="font-size:12.5px;overflow-wrap:anywhere">${esc(c)}</code>${btn('Copy', { k: 'g', x: `data-lact="vcopy" data-c="${esc(c)}"${tip('Copy this command.')}` })}</div>`;
  const STEPS = {
    windows: { what: 'VB-CABLE, a free virtual cable (donations are optional). It has two sides: what plays into CABLE Input comes out of CABLE Output.', steps: [
      ['Download it', `<div>Get the newest Driver Pack zip from the maker's page.</div><div class="row">${link('Open the download page', 'https://vb-audio.com/Cable/', 'Opens the page of the company that makes VB-CABLE in your browser.')}</div>`],
      ['Install it', 'Extract all the files from the zip. Right-click the setup program for your computer (x64 on most PCs), choose Run as administrator, then Install Driver.'],
      ['Restart your computer', 'Windows only finishes adding the device after a restart.'],
      ['Check here', 'Come back and press Check again. Flux then shows the Flux speaker and microphone.'],
      ['Send your sound to it', 'In Windows Settings, System, Sound, set Output to CABLE Input, or pick it in one app. Press Start on Computer sound and Flux plays to your real speakers.'],
      ['For calls', 'In the call app, choose CABLE Output as the microphone.'],
    ], extra: 'One cable is the Flux speaker or the Flux microphone, one at a time. To run both together, add the maker\'s CABLE A+B pack (a paid donation). To remove it, run the same setup program and choose Remove Driver.' },
    macos: { what: 'BlackHole, a free virtual audio device for Macs. What plays into it can be recorded from it.', steps: [
      ['Install it', `<div>Close your audio apps. If you use Homebrew, run this in Terminal:</div>${cmd('brew install blackhole-2ch')}<div>To use the Flux speaker and microphone together, install a second one too:</div>${cmd('brew install blackhole-16ch')}<div class="row">${link('Or get the installer', 'https://existential.audio/blackhole/', 'Opens the page of the company that makes BlackHole in your browser. It asks for an email, and a donation is optional.')}</div>`],
      ['Restart your Mac', 'Restart when the installer asks you to.'],
      ['Check here', 'Come back and press Check again. Flux then shows the Flux speaker and microphone.'],
      ['Send your sound to it', 'In System Settings, Sound, set Output to BlackHole 2ch, or pick it in one app. Press Start on Computer sound and Flux plays to your real speakers. Allow Flux to use the microphone when macOS asks.'],
      ['For calls', 'In the call app, choose the BlackHole device Flux names as the microphone.'],
    ], extra: 'To remove it: brew uninstall blackhole-2ch, or the uninstaller on the maker\'s page.' },
    linux: { what: 'Nothing extra on most systems: Flux adds the Flux speaker and microphone itself, with no administrator password. It only needs the pactl command, which comes with PipeWire or PulseAudio.', steps: [
      ['Install pactl if it is missing', `<div>Debian or Ubuntu:</div>${cmd('sudo apt install pulseaudio-utils')}<div>Fedora:</div>${cmd('sudo dnf install pulseaudio-utils')}<div>Arch:</div>${cmd('sudo pacman -S libpulse')}`],
      ['Check here', 'Press Check again, then Add them on the Live page.'],
      ['Send your sound to it', 'Press Start on Computer sound. Flux makes the Flux speaker the system output until you stop, and plays to your real speakers.'],
      ['For calls', 'In the call app, choose Flux microphone.'],
    ], extra: 'Remove on the Live page takes both devices away again.' },
  };
  reg('live-cable', 'Tools', 'Virtual cable guide', () => {
    const v = (D.ldevices || {}).flux || {}, here = v.platform || 'windows', u = U(), os = u.vos || here, g = STEPS[os];
    const found = v.speaker || v.mic;
    const st = os !== here ? '' : found ? cal('check', `Found on this computer: <b>${esc((v.speaker || v.mic).label)}</b>. ${esc(v.note || '')}`) : v.status === 'can_add' ? cal('check', 'This computer is ready. Press Add them on the Live page.') : cal('warn', `<b>Not installed.</b> ${esc(v.note || '')}`, 'warn');
    return dlg('live', 'Install a virtual cable', `
      <div class="row" role="group" aria-label="Your computer">${Object.keys(VOS).map(k => Flux.chip(VOS[k] + (k === here ? ' (this computer)' : ''), k === os, null, `data-lact="vos" data-os="${k}" aria-pressed="${k === os}"${tip('Show the steps for ' + VOS[k] + '.')}`)).join('')}</div>
      ${st}${u.vmsg ? cal('info', esc(u.vmsg)) : ''}
      <p class="dim" style="margin:0">${g.what}</p>
      <div class="lst box" data-lv="guide-steps">${g.steps.map((s, i) => `<div style="align-items:flex-start"><span class="act mono">${i + 1}</span><div class="ti" style="flex:1;min-width:0"><b>${s[0]}</b><span class="col" style="white-space:normal;gap:8px">${s[1]}</span></div></div>`).join('')}</div>
      <span class="dim" style="font-size:12.5px">${g.extra}</span>
      ${os === 'linux' ? '' : cal('lock', '<b>Made by another company.</b> Flux doesn\'t install it for you. Installing adds a sound driver and asks for your administrator password.')}
      ${sec('Why Flux doesn\'t include its own', '<span class="dim">Windows and macOS only accept new sound devices from a driver that the maker has signed with a paid, verified developer identity. Flux doesn\'t have one yet, so it uses a cable you install. Linux doesn\'t need this.</span>')}`,
      `${btn('Done', { k: 'g', x: `data-lact="vdone"${tip('Back to Live.')}` })}${btn('Check again', { k: 'p', i: 'check', x: `data-lact="vcheck"${tip('Looks for a virtual cable on this computer again.')}` })}`, { wide: true, cls: 'stickfoot' });
  });
  function fluxDevices() {
    const v = (D.ldevices || {}).flux; if (!v) return '';
    const own = v.speaker && v.speaker.kind === 'flux', e = U().verr;
    const act = v.can_create ? (own ? btn('Remove', { k: 'g', i: 'trash', x: `data-lact="vset" data-v="remove"${tip('Removes the Flux speaker and microphone from this computer. Apps go back to your own devices.')}` }) : btn('Add them', { k: 'p', i: 'plus', x: `data-lact="vset" data-v="setup"${tip('Adds the Flux speaker and Flux microphone to this computer. No administrator password is needed.')}` })) : '';
    return `<div class="box col" data-lv="flux-devices"><div class="between" style="flex-wrap:wrap;gap:10px"><div class="col" style="gap:2px;flex:1 1 240px;min-width:0"><b class="h3"${tip('Devices other apps can pick, so their sound goes through Flux and your processed voice reaches call apps.')}>Flux speaker and microphone</b><span class="dim" style="font-size:13px">${esc(v.note || '')}</span></div><div class="row">${v.speaker || v.mic ? pill('Ready', 'ok') : v.status === 'missing' ? pill('Not installed', 'warn') : pill('Not set up')}${act}${v.can_create && v.status !== 'missing' ? '' : btn(v.shared ? 'Add a second cable' : v.speaker ? 'Guide' : 'How to install', { k: v.status === 'missing' ? 'p' : '', x: `data-lact="vguide"${tip('Step-by-step instructions for this computer.')}` })}</div></div>${U().vmsg ? cal('info', esc(U().vmsg)) : ''}${e ? cal('warn', esc(e), 'warn') : ''}</div>`;
  }
  const stats = s => `${F.khz((s.info || {}).sample_rate || 48000)} · ${F.num(s.buffer_ms || 0, 1)} ms buffer · ${s.underruns || 0} dropouts${s.error ? ' · ' + esc(s.error) : ''}`;

  /* ---------- gallery ---------- */
  const PAL = THEMES.map(t => [t[0], t[4]]);
  const hash = s => [...s].reduce((h, c) => (h * 31 + c.charCodeAt(0)) >>> 0, 7);
  function gallery() {
    const u = U(), q = (u.q || '').toLowerCase(), list = (D.lpresets || []).filter(p => (u.tab === 'mine' ? !p.builtin : u.tab === 'input' ? p.kind === 'input' || p.kind === 'any' : p.kind === 'output' || p.kind === 'any') && (!q || (p.name + ' ' + (p.tags || []).join(' ') + ' ' + p.description).toLowerCase().includes(q)));
    const tabs = [['output', 'Computer sound'], ['input', 'Microphone'], ['mine', 'Yours']];
    return `<div class="row">${tipd(seg(tabs.map(x => x[1]), tabs.findIndex(x => x[0] === u.tab), 'data-lseg="tab" aria-label="Presets for"'), 'Show presets made for computer sound, for a microphone, or the ones you made.')}<div style="flex:1;min-width:180px"${tip('Find a preset by name, tag or what it does.')}>${inp(u.q || '', 'Search live presets', 'data-lchange="q" aria-label="Search live presets"')}</div>
      ${btn('New preset', { i: 'plus', x: `data-lact="new"${tip('Start an empty preset and build it from any modules.')}` })}${btn('Import', { i: 'imp', x: `data-lact="import"${tip('Add a .fluxlive preset someone shared, or one you exported.')}` })}</div>
      ${list.length ? `<div class="g4">${list.map(p => { const [pal, fx] = PAL[hash(p.id) % PAL.length]; const l = KIND[p.kind] + (p.builtin ? '' : ' · yours'); return tipd(fxc(pal, fx, esc(p.name), l, p.modules + ' modules', { btn: `data-lact="pick" data-id="${esc(p.id)}" aria-pressed="${u.sel === p.id}"` }).replace(`aria-label="${l}"`, `aria-label="${esc(p.name)}, ${l}"`), p.description || p.name); }).join('')}</div>`
        : cal('info', u.tab === 'mine' ? 'You haven\'t made a live preset yet. Pick any preset and change it, or start a new one.' : 'No presets match.')}`;
  }

  /* ---------- designer ---------- */
  const paramLabel = (p, mid, key) => { const m = allMods(p).find(x => x.id === mid), s = m && spec(m.kind); if (!m) return mid; const same = allMods(p).filter(x => x.kind === m.kind), n = same.length > 1 ? ' ' + (same.indexOf(m) + 1) : ''; return (s ? s.name : m.kind) + n + ' · ' + (key === 'mix' ? 'Mix' : ((pspec(m.kind, key) || {}).label || key)); };
  const targetOpts = p => [['', 'Connect to…']].concat(allMods(p).flatMap(m => { const s = spec(m.kind); return [[m.id + '|mix', paramLabel(p, m.id, 'mix')]].concat(s ? s.params.filter(q => q.unit !== 'bool').map(q => [m.id + '|' + q.key, paramLabel(p, m.id, q.key)]) : []); }));
  const addOpts = () => [['', 'Add a module…']].concat([...new Set(kinds().map(k => k.group))].flatMap(g => kinds().filter(k => k.group === g).map(k => [k.kind, esc(g) + ' · ' + esc(k.name)])));
  function moduleCard(p, m, where, i, n) {
    const s = spec(m.kind) || { name: m.kind, params: [], tip: '' };
    const ctl = s.params.map(q => {
      const v = pv(m, q.key);
      if (q.unit === 'enum') return tipd(fld(esc(q.label), sel((q.options || []).map((o, k) => [k, esc(cap1(o))]), Math.round(v), `data-lchange="menum" data-m="${m.id}" data-k="${q.key}" aria-label="${esc(s.name + ' ' + q.label)}"`)), q.tip);
      if (q.unit === 'bool') return tipd(tg(esc(q.label), v >= .5, '', `data-lact="mbool" data-m="${m.id}" data-k="${q.key}"`), q.tip);
      return tipd(sl(esc(q.label), F.pct(q, v), F.fmtp(q, v), { bind: `lv:mp:${m.id}:${q.key}` }), q.tip);
    }).join('');
    const lanes = [['post', 'Post chain']].concat(p.lanes.map(l => [l.id, esc(l.name)]));
    const modded = p.mods.some(o => o.targets.some(t => t.module === m.id)) || p.macros.some(o => o.targets.some(t => t.module === m.id));
    return `<div class="box col" style="background:var(--s2)${m.enabled ? '' : ';opacity:.62'}" data-module="${m.id}">
      <div class="between"><b${tip(s.tip)}>${esc(s.name)}${modded ? ' <span class="pill" style="margin-left:4px">moving</span>' : ''}</b><div class="row" style="gap:4px">
        ${tg('', m.enabled, '', `data-lact="mon" data-m="${m.id}" data-label="${esc(s.name)} on"${tip(TIP('module', 'enabled'))}`)}
        ${btn('', { k: 'g', i: 'up', a: 'Move earlier', x: `data-lact="mup" data-m="${m.id}"${i ? '' : ' disabled'}${tip('Move earlier in the chain. Order changes the sound.')}` })}${btn('', { k: 'g', i: 'down', a: 'Move later', x: `data-lact="mdown" data-m="${m.id}"${i < n - 1 ? '' : ' disabled'}${tip('Move later in the chain.')}` })}
        ${btn('', { k: 'g', i: 'trash', a: 'Remove ' + esc(s.name), x: `data-lact="mrm" data-m="${m.id}"${tip('Remove this module, and anything moving it.')}` })}</div></div>
      <div class="ctl-grid">${tipd(sl('Mix', m.mix * 100, pc(m.mix), { bind: `lv:mx:${m.id}` }), TIP('module', 'mix'))}${ctl}</div>
      ${tipd(fld('In', sel(lanes, where, `data-lchange="mlane" data-m="${m.id}" aria-label="${esc(s.name)}: lane"`)), 'Move this module to another lane or the post chain.')}
    </div>`;
  }
  function chainBlock(p, title, where, list, lane) {
    const head = lane ? `<div class="row">${tipd(`<div style="flex:1;min-width:160px">${inp(lane.name, 'Lane name', `data-lchange="lname" data-lane="${lane.id}" aria-label="Lane name"`)}</div>`, 'Name this lane.')}
      <div style="flex:1;min-width:160px">${rsl('Level', 'gain_db', lane.gain_db, 'lg:' + lane.id, TIP('lane', 'gain_db'))}</div><div style="flex:1;min-width:160px">${rsl('Pan', 'pan', lane.pan, 'lp:' + lane.id, TIP('lane', 'pan'))}</div>
      ${tg('Mute', lane.mute, '', `data-lact="lmute" data-lane="${lane.id}"${tip(TIP('lane', 'mute'))}`)}${btn('', { k: 'g', i: 'trash', a: 'Remove lane ' + esc(lane.name), x: `data-lact="lrm" data-lane="${lane.id}"${tip('Remove this lane and its modules.')}` })}</div>` : '';
    return `<div class="col" style="padding-left:12px;border-left:3px solid var(--tx)">${title ? `<b${tip(lane ? TIP('lane', 'lanes') : TIP('lane', 'post'))}>${title}</b>` : ''}${head}
      ${list.length ? `<div class="lvmods">${list.map((m, i) => moduleCard(p, m, where, i, list.length)).join('')}</div>` : `<p class="dim" style="margin:0;font-size:13px">${lane ? 'Empty lane: it passes the sound through.' : 'Nothing after the lanes yet.'}</p>`}
      <div style="max-width:340px">${tipd(sel(addOpts(), '', `data-lchange="madd" data-lane="${where}" aria-label="Add a module to ${esc(lane ? lane.name : 'the post chain')}"`), 'Any of the live effects. Each has its own parameters, and modulators and macros can move every one.')}</div></div>`;
  }
  function modCard(p, o) {
    const shape = (D.lkinds.shapes || []).find(s => s.id === o.shape) || {}, syncs = [['', 'Free (Hz)']].concat((D.lkinds.syncs || []).map(s => [s, s]));
    const st = o.shape === 'steps' ? `<div class="col" style="gap:6px"><div class="between"><span class="lab"${tip(TIP('mod', 'steps'))}>Steps</span>${tipd(sel(Array.from({ length: 32 }, (_, k) => [k + 1, (k + 1) + ' steps']), o.steps.length || 8, `data-lchange="mosteps" data-mo="${o.id}" aria-label="Number of steps"`), 'How many steps before the pattern repeats.')}</div>
      <div class="lvsteps" role="group" aria-label="Step values"${tip('Click or drag up and down to set each step. Arrow keys work too.')}>${(o.steps.length ? o.steps : [0]).map((v, k) => `<button type="button" data-lstep data-mo="${o.id}" data-i="${k}" style="--h:${Math.round(v * 100)}%" aria-label="Step ${k + 1}, ${Math.round(v * 100)}%"></button>`).join('')}</div></div>` : '';
    const fol = o.shape === 'follower' ? rsl('Attack', 'attack_ms', o.attack_ms, `mo:${o.id}:attack_ms`, TIP('mod', 'attack_ms')) + rsl('Release', 'release_ms', o.release_ms, `mo:${o.id}:release_ms`, TIP('mod', 'release_ms')) : '';
    const rnd = /random|smooth|noise|steps/.test(o.shape) ? tipd(fld('Seed', `<input class="inp" type="number" min="0" max="4294967295" value="${o.seed}" data-lchange="moseed" data-mo="${o.id}" aria-label="Seed">`), TIP('mod', 'seed')) : '';
    return `<div class="box col" style="background:var(--s2)" data-mod="${o.id}">
      <div class="row">${tipd(`<div style="flex:1;min-width:140px">${inp(o.name, 'Name', `data-lchange="moname" data-mo="${o.id}" aria-label="Modulator name"`)}</div>`, 'Name this modulator.')}
        ${tipd(sel((D.lkinds.shapes || []).map(s => [s.id, cap1(s.id)]), o.shape, `data-lchange="moshape" data-mo="${o.id}" aria-label="Shape" style="width:auto;flex:0 0 128px"`), (shape.tip || TIP('mod', 'shape')))}
        ${btn('', { k: 'g', i: 'trash', a: 'Remove ' + esc(o.name), x: `data-lact="morm" data-mo="${o.id}"${tip('Remove this modulator. What it moved goes back to its setting.')}` })}</div>
      <div class="ctl-grid">${o.shape === 'follower' ? '' : tipd(fld('Sync', sel(syncs, o.sync || '', `data-lchange="mosync" data-mo="${o.id}" aria-label="Tempo sync"`)), TIP('mod', 'sync'))}
        ${o.sync && o.shape !== 'follower' ? '' : o.shape === 'follower' ? '' : rsl('Rate', 'rate_hz', o.rate_hz, `mo:${o.id}:rate_hz`, TIP('mod', 'rate_hz'))}
        ${rsl('Depth', 'depth', o.depth, `mo:${o.id}:depth`, TIP('mod', 'depth'))}${rsl('Offset', 'offset', o.offset, `mo:${o.id}:offset`, TIP('mod', 'offset'))}
        ${o.shape === 'follower' ? '' : rsl('Phase', 'phase', o.phase, `mo:${o.id}:phase`, TIP('mod', 'phase'))}${rsl('Smooth', 'smooth', o.smooth, `mo:${o.id}:smooth`, TIP('mod', 'smooth'))}${fol}
        ${tipd(tg('Both ways', o.bipolar, '', `data-lact="mobip" data-mo="${o.id}"`), TIP('mod', 'bipolar'))}${rnd}</div>${st}
      <div class="col" style="gap:6px"><span class="lab"${tip(TIP('mod', 'amount'))}>Moves</span>
        ${o.targets.map(t => `<div class="row" style="flex-wrap:nowrap"><span style="flex:1;min-width:0;font-size:13px">${esc(paramLabel(p, t.module, t.param))}</span><div style="width:46%">${tipd(sl('Amount', pctOf(t.amount, RG.amount), spc(t.amount), { bind: `lv:mt:${o.id}:${t.module}:${t.param}` }), TIP('mod', 'amount'))}</div>${btn('', { k: 'g', i: 'x', a: 'Disconnect', x: `data-lact="mtrm" data-mo="${o.id}" data-m="${t.module}" data-k="${t.param}"${tip('Stop moving this parameter.')}` })}</div>`).join('') || '<span class="dim" style="font-size:13px">Nothing yet. Connect it to any parameter.</span>'}
        ${tipd(sel(targetOpts(p), '', `data-lchange="mtadd" data-mo="${o.id}" aria-label="Connect ${esc(o.name)} to a parameter"`), 'Pick any parameter of any module, or a mix, for this modulator to move.')}</div></div>`;
  }
  function macroCard(p, mc) {
    return `<div class="box col" style="background:var(--s2)" data-macro="${mc.id}">
      <div class="row">${tipd(`<div style="flex:1;min-width:140px">${inp(mc.name, 'Name', `data-lchange="maname" data-ma="${mc.id}" aria-label="Macro name"`)}</div>`, 'Name this macro.')}${btn('', { k: 'g', i: 'trash', a: 'Remove ' + esc(mc.name), x: `data-lact="marm" data-ma="${mc.id}"${tip('Remove this macro.')}` })}</div>
      ${tipd(sl(esc(mc.name), mc.value * 100, pc(mc.value), { bind: `lv:mv:${mc.id}`, big: 1, ac: 1 }), TIP('macro', 'value'))}
      ${mc.targets.map((t, k) => { const m = modOf(t.module), q = t.param === 'mix' ? { min: 0, max: 1, unit: '' } : m && pspec(m.kind, t.param) || { min: 0, max: 1 }; return `<div class="col" style="gap:4px;padding:8px 0;border-top:1px solid var(--ln2)"><div class="between"><span style="font-size:13px">${esc(paramLabel(p, t.module, t.param))}</span>${btn('', { k: 'g', i: 'x', a: 'Disconnect', x: `data-lact="matrm" data-ma="${mc.id}" data-i="${k}"${tip('Stop this macro moving this parameter.')}` })}</div>
        <div class="row" style="flex-wrap:nowrap">${tipd(fld('At 0', `<input class="inp" type="number" step="any" min="${q.min}" max="${q.max}" value="${+(+t.from).toFixed(4)}" data-lchange="mafield" data-ma="${mc.id}" data-i="${k}" data-k="from" aria-label="Value at 0">`), TIP('macro', 'from'))}${tipd(fld('At 1', `<input class="inp" type="number" step="any" min="${q.min}" max="${q.max}" value="${+(+t.to).toFixed(4)}" data-lchange="mafield" data-ma="${mc.id}" data-i="${k}" data-k="to" aria-label="Value at 1">`), TIP('macro', 'to'))}
        <div style="flex:1">${tipd(sl('Curve', pctOf(t.curve, RG.curve), RG.curve.f(t.curve), { bind: `lv:mc:${mc.id}:${k}` }), TIP('macro', 'curve'))}</div></div></div>`; }).join('')}
      ${tipd(sel(targetOpts(p), '', `data-lchange="matadd" data-ma="${mc.id}" aria-label="Connect ${esc(mc.name)} to a parameter"`), 'Add a parameter for this macro to move, from one value to another.')}</div>`;
  }
  const strip = p => JSON.stringify(Object.assign({}, p, { builtin: false }));
  const dirty = () => { const e = U().edit; return !!e && (!!e.copy || strip(P()) !== e.snapS); };
  /* On the page: what the preset is and what you can do with it. Changing it happens in the editor. */
  function designer() {
    const p = P(), u = U();
    if (!p) return cal('info', 'Pick a preset above to see how it\'s built, then edit it.');
    return `<div class="col" style="gap:18px" data-designer>
      <div class="between" style="align-items:flex-start;flex-wrap:wrap"><div class="col" style="gap:4px;flex:1;min-width:260px">
        <h2 class="h2">${esc(p.name)}</h2><p class="dim" style="margin:0">${esc(p.description)}</p>
        <div class="row" style="gap:6px">${pill(KIND[p.kind])}${p.builtin ? pill('Built in') : pill('Yours', 'sig')}${(p.tags || []).map(t => pill(esc(t))).join('')}</div></div>
        <div class="row">${btn('Use it', { k: 'p', i: 'play', x: `data-lact="use"${tip('Use this preset for its route. If the route is live, you hear it straight away.')}` })}
          ${btn('Edit', { i: 'pen', x: `data-lact="edit"${tip(p.builtin ? 'Change anything in it. Built-ins stay as they are: your changes go into your own copy.' : 'Change anything in it. Save keeps the changes; Discard puts it back.')}` })}
          ${btn('Preview file', { i: 'wave', x: `data-lact="render"${tip('Render 8 seconds of test sound through this preset to a WAV, with before and after levels.')}` })}
          ${btn(p.builtin ? 'Make a copy' : 'Duplicate', { i: 'copy', x: `data-lact="dup"${tip('Make your own copy to change.')}` })}
          ${btn('Export', { i: 'exp', x: `data-lact="export"${tip('Save this preset as a .fluxlive file to share or keep.')}` })}
          ${p.builtin ? '' : btn('', { k: 'g', i: 'trash', a: 'Delete preset', x: `data-lact="del"${tip('Delete this preset. It can\'t be undone.')}` })}</div></div>
      ${u.note && !u.edit ? cal('info', esc(u.note)) : ''}${u.err.ed && !u.edit ? cal('warn', esc(u.err.ed), 'warn') : ''}
      ${u.render ? cal('wave', `Preview saved: ${F.num(u.render.seconds, 1)} s of ${u.render.source === 'demo-voice' ? 'test voice' : 'test music'}. In ${F.num(u.render.in_rms_db, 1)} dB RMS, out ${F.num(u.render.out_rms_db, 1)} dB RMS, peak ${F.num(u.render.out_peak_db, 1)} dB. ${btn('Show file', { k: 'g', x: `data-lact="reveal" data-path="${esc(u.render.path)}"${tip('Show the preview WAV in its folder.')}` })}`) : ''}
    </div>`;
  }
  /* The editor: every part of the preset, in a dialog. Changes play as you make them; Save keeps them, Discard or
     Esc asks first and then puts the preset back as it was when the editor opened. */
  function editor() {
    const p = P(), u = U(), L = (D.lkinds && D.lkinds.limits) || {};
    if (!p || !u.edit) return '';
    const T = k => TIP('preset', k);
    const x = `<button class="btn g ic" type="button" aria-label="Close" data-lact="edclose"${tip('Close the editor. If you changed anything, Flux asks whether to discard it.')}>${ic('x')}</button>`;
    const conf = u.edit.confirm ? `<div class="ov" role="alertdialog" aria-modal="true" aria-label="Discard changes?"><div class="dlg"><div class="between"><h2 class="h2">Discard changes?</h2><button class="btn g ic" type="button" aria-label="Close" data-lact="edkeep">${ic('x')}</button></div>
      <p class="lede" style="margin:0">${u.edit.copy ? 'The copy Flux made for your changes is deleted, and the built-in preset is used again.' : 'Everything you changed since you opened the editor goes back to how it was.'}</p>
      <div class="foot">${btn('Keep editing', { k: 'g', x: 'data-lact="edkeep"' })}${btn('Discard', { k: 'p', x: 'data-lact="eddiscard" data-edfocus' })}</div></div></div>` : '';
    return `<div class="ov" role="dialog" aria-modal="true" aria-label="Edit preset" data-lvedit><div class="dlg wide">
      <div class="between"><h2 class="h2">Edit preset</h2>${x}</div>
      <div class="col" style="gap:18px">
      <div class="between" style="align-items:flex-start;flex-wrap:wrap"><div class="col" style="gap:4px;flex:1;min-width:260px">
        ${p.builtin ? `<b class="h3">${esc(p.name)}</b><p class="dim" style="margin:0">${esc(p.description)}</p>` : `${inp(p.name, 'Preset name', 'data-lchange="pname" aria-label="Preset name"' + tip('Name your preset.'))}${inp(p.description, 'What it does', 'data-lchange="pdesc" aria-label="Description"' + tip('A line that says what it does. Agents read this too.'))}`}</div>
        <div class="row">${btn('Randomize', { i: 'dice', x: `data-lact="rand"${tip('Nudge every parameter at random, around where it is. Do it again for something new.')}` })}</div></div>
      ${u.note ? cal('info', esc(u.note)) : ''}${u.err.ed ? cal('warn', esc(u.err.ed), 'warn') : ''}
      ${sec('Whole preset', `<div class="ctl-grid">${rsl('Input gain', 'input_gain_db', p.input_gain_db, 'p:input_gain_db', T('input_gain_db'))}${rsl('Output gain', 'output_gain_db', p.output_gain_db, 'p:output_gain_db', T('output_gain_db'))}${rsl('Width', 'width', p.width, 'p:width', T('width'))}${rsl('Balance', 'balance', p.balance, 'p:balance', T('balance'))}${rsl('Tempo', 'tempo', p.tempo, 'p:tempo', T('tempo'))}
        ${tipd(tg('Safety limiter', p.safety_limiter, '', 'data-lact="lim"'), T('safety_limiter'))}${tipd(fld('For', sel([['output', 'Computer sound'], ['input', 'Microphone'], ['any', 'Either']], p.kind, 'data-lchange="pkind" aria-label="Preset is for"')), T('kind'))}
        ${tipd(fld('Tags', inp((p.tags || []).join(', '), 'Tags', 'data-lchange="ptags" aria-label="Tags, separated by commas"')), 'Words to find it by, separated by commas.')}</div>`)}
      ${sec(`Macros · ${p.macros.length} of ${L.macros || 12}`, `<p class="dim" style="margin:0;font-size:13px"${tip(TIP('macro', 'value'))}>One knob that moves many parameters at once, each between its own two values.</p>${p.macros.length ? `<div class="lvmods">${p.macros.map(mc => macroCard(p, mc)).join('')}</div>` : ''}<div>${btn('Add macro', { i: 'plus', x: `data-lact="maadd"${p.macros.length >= (L.macros || 12) ? ' disabled' : ''}${tip('Add a knob, then connect it to parameters.')}` })}</div>`)}
      ${sec(`Lanes · ${p.lanes.length} of ${L.lanes || 8}`, `<p class="dim" style="margin:0;font-size:13px">${esc(TIP('lane', 'lanes'))}</p>${p.lanes.map(l => chainBlock(p, '', l.id, l.modules, l)).join('')}<div>${btn('Add lane', { i: 'plus', x: `data-lact="ladd"${p.lanes.length >= (L.lanes || 8) ? ' disabled' : ''}${tip('Add a parallel lane on the same incoming sound.')}` })}</div>`)}
      ${sec('Post chain', `<p class="dim" style="margin:0;font-size:13px">${esc(TIP('lane', 'post'))}</p>${chainBlock(p, '', 'post', p.post, null)}`)}
      ${sec(`Modulators · ${p.mods.length} of ${L.mods || 16}`, `<p class="dim" style="margin:0;font-size:13px">LFOs, random, step sequencers and envelope followers. Each can move up to ${L.targets || 8} parameters.</p>${p.mods.length ? `<div class="lvmods">${p.mods.map(o => modCard(p, o)).join('')}</div>` : ''}<div>${btn('Add modulator', { i: 'plus', x: `data-lact="moadd"${p.mods.length >= (L.mods || 16) ? ' disabled' : ''}${tip('Add something that moves parameters by itself over time.')}` })}</div>`)}
      </div>
      <div class="foot">${btn('Discard', { k: 'g', x: `data-lact="edclose"${tip('Throw away the changes made in this editor. Flux asks first.')}` })}${btn('Save', { k: 'p', i: 'check', x: `data-lact="edsave"${tip('Keep the changes and close the editor.')}` })}</div>
    </div></div>${conf}`;
  }

  reg('live', 'Tools', 'Live', () => {
    const cap = (D.lstatus || {}).capture || {}, u = U();
    /* Open on the preset that is playing, or the one used last. */
    if (!u.sel && !u.picked) { u.picked = 1; const r = (D.lstatus || {}).routes || {}, id = (r.output && r.output.info.preset) || (r.input && r.input.info.preset) || form('output').preset; if (id) { u.sel = id; F.stale('lpreset'); F.need(['lpreset'], true).then(Flux.rr); } }
    return pg('live', 'Live', 'Put a preset on everything your computer plays, or on your microphone, as it happens. Then build your own from any effect.', `<div data-livepage class="col" style="gap:22px">
      <div class="g2">${routeCard('output')}${routeCard('input')}</div>${fluxDevices()}
      ${cap.note ? `<p class="dim" style="margin:0;font-size:13px">${esc(cap.note)}</p>` : ''}
      ${sec('Presets', gallery())}
      ${sec('Designer', designer())}</div>${editor()}`, tipd(btn('Agents can build these', { k: 'g', i: 'user', go: 'prefs-agents' }), 'Every control here is also an MCP tool (live.*), so an agent can design, audition and run presets for you.'));
  });

  /* ---------- interaction ---------- */
  async function reload(id) { const u = U(); if (id) u.sel = id; F.stale('lpresets', 'lpreset'); await F.need(['lpresets', 'lpreset'], true); Flux.rr(); }
  async function pick(id) { const u = U(); if (!id) return; u.sel = id; u.note = null; u.render = null; u.err.ed = null; F.stale('lpreset'); await F.need(['lpreset'], true); if (D.lpreset) useFor(D.lpreset); Flux.rr(); }
  const A = {
    pick: el => pick(el.dataset.id),
    editp: async el => { if (el.dataset.id && (U().sel !== el.dataset.id || !P())) await pick(el.dataset.id); openEdit(); },
    edit: () => openEdit(),
    edsave: async () => { await flush(); U().edit = null; U().note = null; await reload(); },
    edclose: async () => { await flush(); if (!U().edit) return; if (!dirty()) { U().edit = null; U().note = null; Flux.rr(); return; } U().edit.confirm = true; Flux.rr(); focusSoon('[data-edfocus]'); },
    edkeep: () => { if (U().edit) U().edit.confirm = false; Flux.rr(); focusSoon('[data-lvedit] [data-lact="edclose"]'); },
    eddiscard: async () => {
      await flush(); const e = U().edit; if (!e) return;
      try {
        if (e.copy) {
          for (const r of ['output', 'input']) { if (form(r).preset === e.copy) form(r).preset = e.snap.id; const rs = running(r); if (rs && (rs.info || {}).preset === e.copy) await call('live.set', { route: r, preset: e.snap.id }); }
          await call('live.preset.delete', { id: e.copy });
        } else await call('live.preset.save', { preset: e.snap });
        U().edit = null; U().note = null; U().err.ed = null; await reload(e.snap.id); await pollNow();
      } catch (er) { e.confirm = false; err(er); }
    },
    vguide: () => { U().vos = null; U().vmsg = null; Flux.render('live-cable'); },
    vdone: () => { U().vmsg = null; Flux.render('live'); },
    vos: el => { U().vos = el.dataset.os; U().vmsg = null; Flux.rr(); },
    vcopy: el => { const b = el.querySelector('span'); navigator.clipboard.writeText(el.dataset.c).then(() => { if (b) { b.textContent = 'Copied'; setTimeout(() => { b.textContent = 'Copy'; }, 1200); } }, () => { }); },
    vcheck: async () => { const u = U(); u.vmsg = null; F.stale('ldevices'); try { await F.need(['ldevices'], true); } catch (e) { u.vmsg = e.message; } delete u.route.output; delete u.route.input; const v = (D.ldevices || {}).flux || {}; if (!u.vmsg) u.vmsg = v.speaker || v.mic ? 'Found it. The Flux speaker and microphone are ready on the Live page.' : v.status === 'can_add' ? 'Ready to add. Press Add them on the Live page.' : 'Still not found. If you just installed one, restart your computer first.'; Flux.rr(); },
    vset: async el => { const u = U(), rm = el.dataset.v === 'remove'; u.verr = null; try { await call(rm ? 'live.virtual.remove' : 'live.virtual.setup', {}); } catch (e) { u.verr = e.message; } delete u.route.output; delete u.route.input; F.stale('ldevices'); await F.need(['ldevices'], true); Flux.rr(); },
    start: async el => { const r = el.dataset.route, f = form(r), u = U(); u.err[r] = null; try { await call('live.start', { route: r, preset: f.preset, source: f.source, source_device: f.source_device || undefined, output: f.output === 'auto' ? undefined : f.output }); } catch (e) { u.err[r] = e.message; } await pollNow(); Flux.rr(); },
    stop: async el => { try { await call('live.stop', { route: el.dataset.route }); } catch (e) { err(e, el.dataset.route); } await pollNow(); Flux.rr(); },
    bypass: async el => { const r = el.dataset.route, s = running(r); if (!s) return; try { await call('live.set', { route: r, bypass: !s.bypass }); } catch (e) { err(e, r); } await pollNow(); Flux.rr(); },
    use: () => { const p = P(); if (p) { useFor(p); Flux.rr(); } },
    new: async () => { try { const tab = U().tab, np = await call('live.preset.create', { name: 'My preset', kind: tab === 'input' ? 'input' : tab === 'output' ? 'output' : 'any' }); U().note = null; await reload(np.id); } catch (e) { err(e); } },
    dup: async () => { const p = P(); if (!p) return; try { const np = await call('live.preset.create', { from: p.id }); U().note = null; await reload(np.id); } catch (e) { err(e); } },
    del: async () => { const p = P(); if (!p || !confirm(`Delete “${p.name}”? This can't be undone.`)) return; try { await call('live.preset.delete', { id: p.id }); U().sel = null; D.lpreset = null; await reload(); } catch (e) { err(e); } },
    export: async () => { const p = P(); if (!p) return; try { const r = await call('live.preset.export', { id: p.id }); await F.saveAs({ name: p.name.replace(/[\\/:*?"<>|]+/g, ' ').trim() + '.fluxlive', kind: 'live', text: r.json }); } catch (e) { err(e); } },
    import: async () => { const fs = await F.pickFiles({ multiple: false, kind: 'live' }); if (!fs || !fs[0]) return; try { const r = await call('live.preset.import', { path: fs[0] }); const np = r.preset || r; U().tab = 'mine'; await reload(np.id); } catch (e) { err(e); } },
    rand: () => ed('live.randomize', { amount: .3, seed: Math.floor(Math.random() * 1e9) }),
    render: async () => { const p = P(); if (!p) return; try { U().render = await call('live.render', { preset: p.id, seconds: 8 }); Flux.rr(); } catch (e) { err(e); } },
    reveal: el => F.reveal(el.dataset.path),
    lim: () => ed('live.preset.update', { safety_limiter: !P().safety_limiter }, { list: false }),
    ladd: () => ed('live.lane.add', { name: 'Lane ' + (P().lanes.length + 1) }, { list: false }),
    lrm: el => ed('live.lane.remove', { lane: el.dataset.lane }),
    lmute: el => { const l = P().lanes.find(x => x.id === el.dataset.lane); if (l) ed('live.lane.set', { lane: l.id, mute: !l.mute }, { list: false }); },
    mon: el => { const m = modOf(el.dataset.m); if (m) ed('live.module.set', { module: m.id, enabled: !m.enabled }, { list: false }); },
    mbool: el => { const m = modOf(el.dataset.m); if (m) ed('live.module.set', { module: m.id, params: { [el.dataset.k]: pv(m, el.dataset.k) >= .5 ? 0 : 1 } }, { list: false }); },
    mup: el => moveBy(el.dataset.m, -1), mdown: el => moveBy(el.dataset.m, 1),
    mrm: el => ed('live.module.remove', { module: el.dataset.m }),
    moadd: () => ed('live.mod.add', { name: 'Mod ' + (P().mods.length + 1), shape: 'sine', rate_hz: .5, depth: .5 }, { list: false }),
    morm: el => ed('live.mod.remove', { mod: el.dataset.mo }, { list: false }),
    mobip: el => { const o = P().mods.find(x => x.id === el.dataset.mo); if (o) ed('live.mod.set', { mod: o.id, bipolar: !o.bipolar }, { list: false }); },
    mtrm: el => ed('live.mod.target', { mod: el.dataset.mo, module: el.dataset.m, param: el.dataset.k, amount: 0 }, { list: false }),
    maadd: () => ed('live.macro.add', { name: 'Macro ' + (P().macros.length + 1), value: .5 }),
    marm: el => ed('live.macro.remove', { macro: el.dataset.ma }),
    matrm: el => { const mc = P().macros.find(x => x.id === el.dataset.ma); if (mc) ed('live.macro.set', { macro: mc.id, targets: mc.targets.filter((_, k) => k !== +el.dataset.i) }); },
  };
  function openEdit() {
    const p = P(); if (!p) return; const u = U();
    u.edit = { snap: JSON.parse(JSON.stringify(p)), snapS: strip(p), copy: null, confirm: false }; u.note = null; u.err.ed = null;
    Flux.rr(); focusSoon('[data-lvedit] [data-lact="edclose"]');
  }
  const focusSoon = sel => requestAnimationFrame(() => requestAnimationFrame(() => { const n = document.querySelector(sel); if (n) n.focus(); }));
  function moveBy(id, d) {
    const p = P(); const where = p.post.some(m => m.id === id) ? 'post' : (p.lanes.find(l => l.modules.some(m => m.id === id)) || {}).id, list = where === 'post' ? p.post : p.lanes.find(l => l.id === where).modules, i = list.findIndex(m => m.id === id);
    return ed('live.module.move', { module: id, lane: where, index: Math.max(0, Math.min(list.length - 1, i + d)) }, { list: false });
  }
  const parseVal = (m, k, s) => { const v = parseFloat(String(s).replace(',', '.').replace('−', '-')); return isFinite(v) ? v : null; };
  const C = {
    route: el => { const r = el.dataset.route, f = form(r), k = el.dataset.k; if (k === 'src') { const [s, d] = el.value.split('|'); f.source = s; f.source_device = d || undefined; if (r === 'output') { const dv = D.ldevices || { outputs: [] }, out = dv.outputs.find(o => !o.virtual && o.id !== d); if (s === 'flux') f.output = 'auto'; else if (f.output === 'auto') f.output = (out || {}).id || 'none'; Flux.rr(); } } else f[k] = el.value; if (running(r) && k === 'preset') { call('live.set', { route: r, preset: el.value }).then(pollNow, e => err(e, r)); U().sel = el.value; F.stale('lpreset'); F.need(['lpreset'], true).then(Flux.rr); } else if (running(r)) { U().err[r] = 'Stop and start again to use the new device.'; Flux.rr(); } },
    q: el => { U().q = el.value; Flux.rr(); },
    pname: el => el.value.trim() && ed('live.preset.update', { name: el.value.trim() }),
    pdesc: el => ed('live.preset.update', { description: el.value }),
    ptags: el => ed('live.preset.update', { tags: el.value.split(',').map(s => s.trim()).filter(Boolean) }),
    pkind: el => ed('live.preset.update', { kind: el.value }),
    lname: el => ed('live.lane.set', { lane: el.dataset.lane, name: el.value }, { list: false }),
    madd: el => { const k = el.value; if (!k) return; ed('live.module.add', { kind: k, lane: el.dataset.lane }); },
    menum: el => ed('live.module.set', { module: el.dataset.m, params: { [el.dataset.k]: +el.value } }, { list: false }),
    mlane: el => ed('live.module.move', { module: el.dataset.m, lane: el.value }, { list: false }),
    moname: el => ed('live.mod.set', { mod: el.dataset.mo, name: el.value }, { list: false }),
    moshape: el => { const o = P().mods.find(x => x.id === el.dataset.mo), x = { mod: el.dataset.mo, shape: el.value }; if (el.value === 'steps' && o && !o.steps.length) x.steps = [1, 0, .5, 0, .75, 0, .25, 0]; ed('live.mod.set', x, { list: false }); },
    mosync: el => ed('live.mod.set', { mod: el.dataset.mo, sync: el.value }, { list: false }),
    moseed: el => { const v = parseInt(el.value, 10); if (isFinite(v) && v >= 0) ed('live.mod.set', { mod: el.dataset.mo, seed: v }, { list: false }); },
    mosteps: el => { const o = P().mods.find(x => x.id === el.dataset.mo); if (!o) return; const n = +el.value, s = Array.from({ length: n }, (_, k) => o.steps.length ? o.steps[k % o.steps.length] : 0); ed('live.mod.set', { mod: o.id, steps: s }, { list: false }); },
    mtadd: el => { if (!el.value) return; const [m, k] = el.value.split('|'); ed('live.mod.target', { mod: el.dataset.mo, module: m, param: k, amount: .5 }, { list: false }); },
    maname: el => ed('live.macro.set', { macro: el.dataset.ma, name: el.value }),
    matadd: el => {
      if (!el.value) return; const [m, k] = el.value.split('|'), mo = modOf(m), q = k === 'mix' ? { min: 0, max: 1 } : mo && pspec(mo.kind, k); if (!q) return;
      const cur0 = k === 'mix' ? mo.mix : pv(mo, k), to = cur0 > (q.min + q.max) / 2 ? q.min : q.max;
      ed('live.macro.target', { macro: el.dataset.ma, module: m, param: k, from: cur0, to, curve: 0 }, { list: false });
    },
    mafield: el => { const mc = P().macros.find(x => x.id === el.dataset.ma), t = mc && mc.targets[+el.dataset.i], v = parseVal(null, null, el.value); if (!t || v == null) return; t[el.dataset.k] = v; ed('live.macro.set', { macro: mc.id, targets: mc.targets }, { list: false }); },
  };
  document.addEventListener('click', e => {
    if (e.button) return;
    const sb = e.target.closest('.seg[data-lseg] button');
    if (sb) { const s = sb.parentElement, i = [...s.querySelectorAll('button')].indexOf(sb); U().tab = ['output', 'input', 'mine'][i]; setTimeout(Flux.rr, 0); return; }
    const st = e.target.closest('[data-lstep]'); if (st) { stepAt(st, e); return; }
    const a = e.target.closest('[data-lact]'); if (!a || a.disabled) return;
    e.preventDefault(); const fn = A[a.dataset.lact]; if (fn) fn(a, e);
  });
  document.addEventListener('change', e => { const el = e.target, k = el.dataset && el.dataset.lchange; if (k && C[k]) C[k](el); });
  /* Step sequencer: click or drag sets a step from where the pointer is; arrows nudge it. */
  let stepDrag = null;
  function stepVals(mo) { const o = P() && P().mods.find(x => x.id === mo); return o ? o.steps.slice() : null; }
  function stepAt(el, e, end) {
    const r = el.getBoundingClientRect(), v = Math.round(Math.max(0, Math.min(1, (r.bottom - e.clientY) / r.height)) * 20) / 20, o = P().mods.find(x => x.id === el.dataset.mo); if (!o) return;
    o.steps[+el.dataset.i] = v; el.style.setProperty('--h', v * 100 + '%'); el.setAttribute('aria-label', `Step ${+el.dataset.i + 1}, ${Math.round(v * 100)}%`);
    push('steps:' + o.id, 'live.mod.set', { mod: o.id, steps: o.steps.slice() }, end !== false);
  }
  document.addEventListener('pointerdown', e => { const st = e.target.closest('[data-lstep]'); if (st) { stepDrag = st.dataset.mo; } });
  document.addEventListener('pointermove', e => { if (!stepDrag || !e.buttons) return; const el = document.elementFromPoint(e.clientX, e.clientY); const st = el && el.closest && el.closest('[data-lstep]'); if (st && st.dataset.mo === stepDrag) stepAt(st, e, false); });
  addEventListener('pointerup', () => { if (stepDrag) { const o = P() && P().mods.find(x => x.id === stepDrag); stepDrag = null; if (o) push('steps:' + o.id, 'live.mod.set', { mod: o.id, steps: o.steps.slice() }, true); } });
  document.addEventListener('keydown', e => {
    const st = e.target.closest && e.target.closest('[data-lstep]'); if (!st || !/ArrowUp|ArrowDown|Home|End/.test(e.key)) return;
    e.preventDefault(); const o = P().mods.find(x => x.id === st.dataset.mo), i = +st.dataset.i; if (!o) return;
    const v = e.key === 'Home' ? 0 : e.key === 'End' ? 1 : Math.max(0, Math.min(1, Math.round(((o.steps[i] || 0) + (e.key === 'ArrowUp' ? .05 : -.05)) * 20) / 20));
    o.steps[i] = v; st.style.setProperty('--h', v * 100 + '%'); st.setAttribute('aria-label', `Step ${i + 1}, ${Math.round(v * 100)}%`); push('steps:' + o.id, 'live.mod.set', { mod: o.id, steps: o.steps.slice() }, true);
  });

  /* ---------- meters: poll while the page is open ---------- */
  const lvp = d => Math.max(0, Math.min(100, (d + 60) / 60 * 100));
  let sig = '', polling = false;
  async function pollNow() {
    if (polling) return; polling = true;
    try {
      const s = await call('live.status'); D.lstatus = s;
      const ns = ['output', 'input'].map(r => { const x = s.routes[r]; return x ? [x.running, x.bypass, x.info && x.info.preset, !!x.error].join() : '-'; }).join('|');
      for (const r of ['output', 'input']) {
        const x = s.routes[r];
        document.querySelectorAll(`[data-lvm^="${r}:"]`).forEach(m => { const io = m.dataset.lvm.split(':')[1]; m.style.setProperty('--lv', (x && x.running ? lvp(x[io + '_peak_db']) : 0) + '%'); });
        document.querySelectorAll(`[data-lvd^="${r}:"]`).forEach(m => { const io = m.dataset.lvd.split(':')[1]; m.textContent = x && x.running && x[io + '_rms_db'] > -90 ? F.num(x[io + '_rms_db'], 1) + ' dB' : '−∞'; });
        document.querySelectorAll(`[data-lvs="${r}"]`).forEach(m => { m.textContent = x && x.running ? stats(x) : 'Not running'; });
      }
      if (ns !== sig) { const first = !sig; sig = ns; if (!first) Flux.rr(); }
    } catch (e) { /* engine busy */ }
    polling = false;
  }
  setInterval(() => { if (!document.hidden && document.querySelector('[data-livepage]')) pollNow(); }, 120);
  Flux.livePoll = pollNow;
})();
