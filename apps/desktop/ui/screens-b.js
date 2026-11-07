/* Effects, spatial, creative, voice. Ported from design/screens-b.js; every control is bound to a real effect. */
(function () {
  const { fxc, ic, v, vz, btn, seg, sl, tg, chip, pill, fld, inp, sel, sec, tile, cal, prog, pr, reg } = Flux;
  const F = FD, D = FD.D, esc = FD.esc;
  const P = () => D.project;

  /* ---- bound controls ----
     A control edits the effect of `kind` on the current target (F.fxOf). If there is none yet, it is bound by kind,
     and app.js adds the effect on the first change, so every screen works on an empty chain. */
  const E = kind => F.fxOf(kind);
  const ref = (kind, e) => e ? `data-fx="${e.id}"` : `data-kind="${kind}"`;
  const pspec = (kind, key) => { const s = F.spec(kind); return s && s.params.find(q => q.key === key); };
  const val = (kind, key, e = E(kind)) => { const p = pspec(kind, key); return e ? F.pval(e, key) : p ? p.default : 0; };
  const ps = (label, kind, key, o = {}) => { const p = pspec(kind, key); if (!p) return sl(label, 0, '—'); const e = o.e !== undefined ? o.e : E(kind), x = val(kind, key, e); return sl(label, F.pct(p, x), o.fmt ? o.fmt(x) : F.fmtp(p, x), Object.assign({ bind: e ? `fx:${e.id}:${key}` : `fxk:${kind}:${key}` }, o)); };
  const pt = (label, kind, key, sub, e = E(kind)) => tg(label, val(kind, key, e) >= .5, sub, `data-act="fx-toggle" ${ref(kind, e)} data-k="${key}"${label ? '' : ` data-label="${esc(F.kindName(kind))}"`}`);
  const pseg = (kind, key, labels, e = E(kind), attr = '') => seg(labels, Math.round(val(kind, key, e)), `data-segact="fx-enum" ${ref(kind, e)} data-k="${key}" ${attr}`);
  const pchips = (kind, key, items, e = E(kind), near = .001) => { const x = val(kind, key, e); return items.map(([value, label]) => chip(label, Math.abs(x - value) < near * Math.max(1, Math.abs(value)), '', `data-act="fx-param" ${ref(kind, e)} data-k="${key}" data-v="${value}"`)).join(''); };
  const mixsl = (kind, e = E(kind)) => sl('Mix', e ? e.mix * 100 : 100, Math.round(e ? e.mix * 100 : 100) + '%', { bind: e ? `fxmix:${e.id}` : `fxkmix:${kind}` });
  const onoff = (kind, e = E(kind)) => tg('', e && e.enabled, '', e ? `data-act="fx-enable" data-fx="${e.id}" data-label="${esc(F.kindName(kind))}"` : `data-act="fx-add" data-kind="${kind}" data-label="${esc(F.kindName(kind))}"`);
  const addBtn = (kind, label = 'Add to chain', k = '') => { const e = E(kind); return e ? btn('Open in chain', { k, go: 'rack', x: '' }) : btn(label, { k, x: `data-act="fx-add" data-kind="${kind}"` }); };
  Object.assign(Flux, { E, ps, pt, pseg, pchips, mixsl, onoff, val, pspec });

  /* The effect-page shell (design `fxp`) with live A/B/C, bypass and a target switch. */
  const fxpE = (ws, title, kinds, main, side) => { const e = E(kinds); return pr(ws, `<div class="split"><div class="a"><div class="tlbar"><b style="font-size:15px;font-weight:800;white-space:nowrap">${title}</b>${targetSeg()}<span class="sp"></span>${e ? seg(['A', 'B', 'C'], e.slot || 0, `data-segact="fx-slot" data-fx="${e.id}" aria-label="Settings slot"`) : seg(['A', 'B', 'C'], 0, 'aria-label="Settings slot"')}${btn('Presets', { go: 'presets' })}${e ? btn(e.enabled ? 'Bypass' : 'Bypassed', { k: e.enabled ? '' : 'on', x: `data-act="fx-enable" data-fx="${e.id}" aria-pressed="${!e.enabled}"` }) : btn('Add', { k: 'p', i: 'plus', x: `data-act="fx-add" data-kind="${[].concat(kinds)[0]}"` })}${btn('', { i: 'x', k: 'g', a: 'Close effect', go: 'rack' })}</div><div class="in" style="padding:14px;display:flex;flex-direction:column;gap:14px;flex:1">${main}</div></div><aside class="b in">${side}</aside></div>`); };
  const tgtName = t => { if (!t || t === 'master') return 'Master'; const [k, id] = t.split(':'); if (k === 'track') { const x = F.track(+id); return x ? x.name : 'Track'; } const c = F.clip(+id); return c ? c.name : 'Clip'; };
  const targetSeg = () => { const t = D.sel.target || 'master', k = t.split(':')[0], i = k === 'clip' ? 0 : k === 'track' ? 1 : 2; return seg(['Clip', 'Track', 'Master'], i, 'data-segact="target-kind" aria-label="Apply to"'); };
  Object.assign(Flux, { fxpE, tgtName, targetSeg });

  const FX = [['Fix', [['Noise reduction', 'fx-clean', 'noise'], ['Hum removal', 'fx-clean', 'hum'], ['De-click', 'fx-clean', 'declick'], ['De-clip', 'fx-clean', 'declip'], ['De-wind', 'fx-clean', 'dewind'], ['Silence remover', 'fx-silence'], ['Spectral repair', 'spectral']]], ['Tone', [['Parametric EQ', 'fx-eq', 'eq'], ['Graphic EQ', 'fxp', 'geq'], ['Match EQ', 'fx-eq'], ['De-esser', 'fx-comp', 'deesser']]], ['Dynamics', [['Compressor', 'fx-comp', 'compressor'], ['Limiter', 'fx-comp', 'limiter'], ['Multiband', 'fx-comp', 'multiband'], ['Gate', 'fx-comp', 'gate'], ['Transient shaper', 'fx-comp', 'transient'], ['Leveler', 'fx-comp', 'leveler']]], ['Time and pitch', [['Speed', 'fx-time', 'speed'], ['Pitch', 'fx-time', 'pitch'], ['Varispeed', 'fx-time', 'speed']]], ['Space', [['3D and 8D', 'stage', 'spatial'], ['Reverb', 'fx-reverb', 'reverb'], ['Rooms', 'rooms'], ['Row 10', 'hall', 'row10'], ['Next door', 'hall', 'nextdoor'], ['Delay', 'fx-reverb', 'delay']]], ['Character', [['Slowed + reverb', 'creative'], ['Nightcore', 'creative'], ['Lo-fi', 'creative', 'lofi'], ['Vinyl', 'textures', 'vinyl'], ['Cassette', 'textures', 'cassette'], ['Old radio', 'textures', 'radio'], ['Phone', 'textures', 'phone'], ['Megaphone', 'textures', 'megaphone'], ['Underwater', 'textures', 'underwater'], ['Distortion', 'textures', 'distortion'], ['Bitcrusher', 'textures', 'bitcrusher'], ['Chorus', 'textures', 'chorus'], ['Flanger', 'textures', 'flanger'], ['Phaser', 'textures', 'phaser']]], ['Glitch', [['Stutter', 'glitch', 'stutter'], ['Tape stop', 'glitch', 'tapestop'], ['Reverse', 'glitch'], ['Granular freeze', 'glitch', 'granular'], ['Trance gate', 'glitch', 'trancegate']]], ['Voice', [['Voice changer', 'voice', 'voice'], ['Robot', 'voice', 'voice'], ['Vocoder', 'voice', 'voice'], ['Hard tune', 'voice', 'voice']]]];
  const FXGROUP = { Fix: ['stutter'], Tone: ['reverse'], Dynamics: ['radio'], 'Time and pitch': ['slowed'], Space: ['cathedral'], Character: ['lofi'], Glitch: ['tapestop'], Voice: ['robot'] };
  const FXSTYLE = { 'Slowed + reverb': ['slowed', 'slow', '0.85×'], Nightcore: ['nightcore', 'night', '1.25×'], 'Lo-fi': ['lofi', 'lofi', '64%'], Vinyl: ['vinyl', 'vinyl', '33⅓'], Cassette: ['cassette', 'tape', 'Wow 0.6 Hz'], 'Old radio': ['radio', 'radio', '3 kHz'], Underwater: ['underwater', 'water', '12 m'], Stutter: ['stutter', 'stut', '1/16'], 'Tape stop': ['tapestop', 'stop', '1.4 s'], Reverse: ['reverse', 'rev', ''], Reverb: ['cathedral', 'cath', '6.8 s'], 'Next door': ['nextdoor', 'next', '620 Hz'], Robot: ['robot', 'bot', '30 Hz'] };
  reg('fxbrowser', 'Effects', 'Effect browser', () => {
    const q = (D.ui.fxq || '').toLowerCase(), f = D.ui.fxfilter || 'all', s = D.settings || {}, fav = s.favorites || [], rec = s.recent_fx || [];
    const keep = ([n, , kind]) => (!q || n.toLowerCase().includes(q)) && (f === 'all' || (f === 'fav' && kind && fav.includes(kind)) || (f === 'recent' && kind && rec.includes(kind)) || (f === 'live' && kind !== 'speed') || (f === 'render' && (kind === 'speed' || !kind)));
    const groups = FX.map(([g, it]) => [g, it.filter(keep)]).filter(([, it]) => it.length);
    return pr('fxbrowser', `<div class="pg in"><div class="pgh"><h1 class="h1">Effects</h1><div class="row"><div style="width:min(300px,100%)">${inp(D.ui.fxq || '', 'Search effects', 'data-ui="fxq" data-rerender="1" aria-label="Search effects"')}</div>${btn('Your chain', { go: 'rack', i: 'fx' })}</div></div>
    <div class="row">${[['all', 'All'], ['fav', 'Favorites'], ['recent', 'Recent'], ['live', 'Runs live'], ['render', 'Needs rendering']].map(([k, t]) => chip(t, f === k, '', `data-act="ui-set" data-k="fxfilter" data-v="${k}"`)).join('')}</div>
    ${groups.length ? groups.map(([g, it]) => `<section class="sec"><h2 class="h2">${g}</h2><div class="g5">${it.map(([n, go, kind]) => { const k = FXSTYLE[n] || FXGROUP[g]; return fxc(k[0], k[1] || '', n, g, k[2] || '', { go }).replace('<a ', `<a data-act="fx-open-kind"${kind ? ` data-kind="${kind}"` : ''} `); }).join('')}</div></section>`).join('') : cal('info', 'No effects match.')}
    ${cal('info', 'Press and hold any tile to hear it on your selection before you add it. Previews never change your project.')}</div>`);
  });

  const TCOL = { Fix: '--t5', Tone: '--t1', Dynamics: '--t2', Space: '--t3', Utility: '--t4', Character: '--t6', Glitch: '--t7', Voice: '--t8', 'Time and pitch': '--t4' };
  const SCREEN = { eq: 'fx-eq', compressor: 'fx-comp', limiter: 'fx-comp', multiband: 'fx-comp', gate: 'fx-comp', deesser: 'fx-comp', transient: 'fx-comp', leveler: 'fx-comp', speed: 'fx-time', pitch: 'fx-time', noise: 'fx-clean', hum: 'fx-clean', declick: 'fx-clean', declip: 'fx-clean', dewind: 'fx-clean', stereo: 'fx-clean', reverb: 'fx-reverb', delay: 'fx-reverb', convolver: 'fx-reverb', spatial: 'stage', row10: 'hall', nextdoor: 'hall', voice: 'voice', lofi: 'creative' };
  Flux.SCREEN = SCREEN;
  /* One line saying what an effect is set to, from its real parameters. */
  const summary = e => { const s = F.spec(e.kind); if (!s) return ''; const ks = s.params.filter(p => !/seed/.test(p.key)).slice(0, 3); return ks.map(p => F.fmtp(p, F.pval(e, p.key))).join(', '); };
  reg('rack', 'Effects', 'Effect chain', () => {
    const t = D.sel.target || 'master', list = F.chain(t), sel = D.sel.fx && list.find(e => e.id === D.sel.fx);
    return pr('fxbrowser', `<div class="split"><div class="a"><div class="tlbar"><b style="font-size:15px;font-weight:800">${esc(tgtName(t))} chain</b>${targetSeg()}<span class="sp" style="flex:1"></span>${sel ? seg(['A', 'B', 'C'], sel.slot || 0, `data-segact="fx-slot" data-fx="${sel.id}" aria-label="Settings slot"`) : seg(['A', 'B', 'C'], 0, 'aria-label="Settings slot"')}${btn('Save preset', { i: 'tag', x: 'data-act="preset-save-chain"' + (list.length ? '' : ' disabled') })}${btn('Add effect', { k: 'p', i: 'plus', go: 'fxbrowser' })}</div>
    <div class="in" style="padding:14px;display:flex;flex-direction:column;gap:9px" data-rack>${list.length ? list.map((e, i) => { const s = F.spec(e.kind) || { name: e.kind, group: '' }, go = SCREEN[e.kind] || 'fxp'; return `<div class="box row ${D.sel.fx === e.id ? 'sel' : ''}" style="flex-wrap:nowrap;padding:10px 12px" data-rackrow="${e.id}" draggable="true"><span class="faint" style="cursor:grab" aria-hidden="true">${ic('list', 17)}</span><span class="dot" style="color:var(${TCOL[s.group] || '--t1'})"></span><a href="#${go}" data-go="${go}" data-act="open-fx" data-fx="${e.id}" style="text-decoration:none;min-width:0;flex:1"><b class="b7" style="display:block;white-space:nowrap;overflow:hidden;text-overflow:ellipsis">${esc(s.name)}</b><span class="lab hide-s">${esc(summary(e))}</span></a><div style="width:110px" class="hide-s">${sl('Mix', e.mix * 100, Math.round(e.mix * 100) + '%', { bind: `fxmix:${e.id}` })}</div>${btn('', { k: 'g hide-s', i: 'up', a: 'Move ' + esc(s.name) + ' up', x: `data-act="fx-move" data-fx="${e.id}" data-i="${i - 1}"` + (i ? '' : ' disabled') })}${btn('', { k: 'g', i: 'trash', a: 'Remove ' + esc(s.name), x: `data-act="fx-remove" data-fx="${e.id}"` })}<button class="tg" style="width:auto" type="button" aria-pressed="${e.enabled}" aria-label="Enable ${esc(s.name)}" data-act="fx-enable" data-fx="${e.id}"><i></i></button></div>`; }).join('') : cal('info', `Nothing on the ${esc(tgtName(t).toLowerCase())} yet. <a class="act" href="#fxbrowser" data-go="fxbrowser">Add an effect</a> or <a class="act" href="#presets" data-go="presets">load a preset</a>.`)}</div>
    <div class="box" style="margin:0 14px 14px"><div class="between"><b>Compare</b><span class="dim">Levels are matched so louder doesn't win</span></div><div class="row" style="margin-top:9px"><button class="btn lg hold p" data-hold style="flex:1">Hold for before</button></div></div></div>
    <aside class="b in"><h3 class="h3">Macro</h3><p class="dim">One slider that moves several controls at once.</p>${sl('Presence', D.ui.macro == null ? 50 : D.ui.macro, Math.round(D.ui.macro == null ? 50 : D.ui.macro) + '%', { big: 1, bind: 'macro' })}${sec('This macro moves', `<div class="lst">${(D.ui.macroMap || []).length ? D.ui.macroMap.map((m, i) => { const e = F.findFx(m.fx), p = e && pspec(e.kind, m.key); return `<div><div class="ti"><b>${e && p ? esc(F.kindName(e.kind) + ' ' + p.label.toLowerCase()) : 'Missing control'}</b><span>${p ? F.fmtp(p, m.lo) + ' to ' + F.fmtp(p, m.hi) : ''}</span></div>${btn('', { k: 'g', i: 'x', a: 'Unmap', x: `data-act="macro-unmap" data-i="${i}"` })}</div>`; }).join('') : '<div><div class="ti"><span>Nothing mapped yet</span></div></div>'}</div>${btn('Map a control', { i: 'plus', x: 'data-act="macro-map"' + (sel ? '' : ' disabled title="Select an effect first"') })}`)}${sec('CPU', `<div class="between"><span class="mono" data-load>${Math.round((D.st.load || 0) * 100)}%</span>${(D.st.load || 0) < .8 ? pill('Real time', 'ok') : pill('Heavy', 'warn')}</div>${prog(Math.round((D.st.load || 0) * 100))}`)}</aside></div>`);
  });

  /* ---------- Equalizer ---------- */
  const BANDT = ['Bell', 'Low shelf', 'High shelf', 'Low cut', 'High cut', 'Notch'], BCOL = ['--t3', '--t2', '--t4', '--t5', '--t1'];
  reg('fx-eq', 'Effects', 'Equalizer', () => {
    const e = E('eq'), presets = (D.presets || []).filter(p => p.kind === 'eq');
    return fxpE('fxbrowser', 'Parametric EQ', 'eq', `
    <div class="vz" style="flex:1;min-height:300px" role="img" aria-label="EQ curve. Drag the numbered points.">${v('eq', e ? { fx: e.id } : { kind: 'eq' })}</div>
    <div class="g5">${[1, 2, 3, 4, 5].map((b, i) => `<div class="box" style="padding:10px"><div class="between"><span class="knob" style="width:22px;height:22px;background:var(${BCOL[i]});color:var(${BCOL[i]}-on);display:grid;place-items:center;font-weight:800;font-size:12px">${b}</span>${fld('', sel(BANDT.map((t, j) => [j, t]), Math.round(val('eq', 'b' + b + '_type', e)), `data-fxsel="b${b}_type" ${ref('eq', e)} aria-label="Band ${b} type"`))}</div><div class="mono">${F.hz(val('eq', 'b' + b + '_freq', e))}</div><div class="mono dim">${F.db(val('eq', 'b' + b + '_gain', e))} dB</div></div>`).join('')}</div>`, `
    <h3 class="h3">Presets</h3><div class="row">${presets.map(p => chip(esc(p.name), D.ui.eqPreset === p.id, '', `data-act="preset-apply" data-id="${esc(p.id)}" data-kind="eq"`)).join('')}</div>
    ${sec('Mode', `${seg(['Natural', 'Analog match', 'Linear phase'], 0, 'aria-label="EQ mode"')}<span class="dim" style="font-size:12.5px">Flux uses natural (minimum-phase) filters, so there is no added latency.</span>`)}
    ${sec('Show', `${seg(['Parametric', '10 band', '31 band'], 0, 'data-segact="eq-show" aria-label="Show"')}<div class="col">${tg('Spectrum before', !!D.ui.specBefore, '', 'data-act="ui-toggle" data-k="specBefore"')}${tg('Spectrum after', D.ui.specAfter !== false, '', 'data-act="ui-toggle" data-k="specAfter" data-def="1"')}</div>`)}
    ${sec('Match', `<span class="dim">Copy the tone of another recording.</span>${btn('Choose a reference', { i: 'imp', x: 'data-act="match-eq"' })}`)}
    ${ps('Output', 'eq', 'output', { e })}`);
  });

  /* ---------- Dynamics ---------- */
  const DYN = [['compressor', 'Compressor'], ['limiter', 'Limiter'], ['multiband', 'Multiband'], ['gate', 'Gate'], ['deesser', 'De-esser'], ['transient', 'Transient'], ['leveler', 'Leveler']];
  const dynKind = () => { const s = D.sel.fx && F.findFx(D.sel.fx); if (s && DYN.some(d => d[0] === s.kind)) return s.kind; return D.ui.dyn || 'compressor'; };
  const grid = (...x) => `<div class="ctl-grid" style="grid-template-columns:repeat(2,minmax(0,1fr))">${x.join('')}</div>`;
  const DYNSIDE = {
    compressor: e => `${grid(ps('Threshold', 'compressor', 'threshold', { e }), ps('Ratio', 'compressor', 'ratio', { e }), ps('Attack', 'compressor', 'attack', { e }), ps('Release', 'compressor', 'release', { e }), ps('Knee', 'compressor', 'knee', { e }), ps('Makeup', 'compressor', 'makeup', { e }), ps('Lookahead', 'compressor', 'lookahead', { e }), mixsl('compressor', e))}
      ${pt('Auto makeup', 'compressor', 'auto_makeup', '', e)}
      ${sec('Sidechain', `${fld('Listen to', sel(['This track'], 0, 'aria-label="Listen to" disabled'))}${ps('Low cut', 'compressor', 'sc_lowcut', { e })}`)}`,
    limiter: e => grid(ps('Ceiling', 'limiter', 'ceiling', { e }), ps('Input', 'limiter', 'input', { e }), ps('Release', 'limiter', 'release', { e }), ps('Lookahead', 'limiter', 'lookahead', { e })),
    multiband: e => `${grid(ps('Low split', 'multiband', 'low_x', { e }), ps('High split', 'multiband', 'high_x', { e }), ps('Threshold', 'multiband', 'threshold', { e }), ps('Ratio', 'multiband', 'ratio', { e }), ps('Attack', 'multiband', 'attack', { e }), ps('Release', 'multiband', 'release', { e }))}${sec('Bands', grid(ps('Low', 'multiband', 'low', { e }), ps('Mid', 'multiband', 'mid', { e }), ps('High', 'multiband', 'high', { e })))}`,
    gate: e => grid(ps('Threshold', 'gate', 'threshold', { e }), ps('Range', 'gate', 'range', { e }), ps('Attack', 'gate', 'attack', { e }), ps('Hold', 'gate', 'hold', { e }), ps('Release', 'gate', 'release', { e }), mixsl('gate', e)),
    deesser: e => grid(ps('Frequency', 'deesser', 'freq', { e }), ps('Threshold', 'deesser', 'threshold', { e }), ps('Amount', 'deesser', 'amount', { e }), mixsl('deesser', e)),
    transient: e => grid(ps('Attack', 'transient', 'attack', { e }), ps('Sustain', 'transient', 'sustain', { e }), mixsl('transient', e)),
    leveler: e => grid(ps('Target', 'leveler', 'target', { e }), ps('Max gain', 'leveler', 'max_gain', { e }), ps('Speed', 'leveler', 'speed', { e }), mixsl('leveler', e)),
  };
  reg('fx-comp', 'Effects', 'Dynamics', () => {
    const k = dynKind(), e = E(k), th = val(k, k === 'limiter' ? 'ceiling' : 'threshold', e), ratio = k === 'limiter' ? 100 : k === 'gate' ? .2 : val(k, 'ratio', e) || 2;
    const gr = D.ui.gr != null ? D.ui.gr : null;
    return fxpE('fxbrowser', DYN.find(d => d[0] === k)[1], k, `
    <div class="tabs" style="padding:0;margin-top:-6px" role="tablist">${DYN.map(([kk, t]) => `<button class="${kk === k ? 'on' : ''}" type="button" role="tab" aria-selected="${kk === k}" data-act="dyn-tab" data-v="${kk}">${t}</button>`).join('')}</div>
    <div class="g2" style="flex:1"><div class="vz" style="min-height:240px;aspect-ratio:1/1;max-height:360px">${v('transfer', { thdb: th == null ? -18 : th, ratio, knee: k === 'compressor' ? val(k, 'knee', e) : 0 })}</div><div class="col"><div class="vz" style="flex:1;min-height:150px">${v('gr', { thdb: th == null ? -18 : th, ratio })}</div><div class="between"><span class="lab">Gain reduction</span><b class="stat" data-grtext>${gr == null ? '—' : F.db(-gr, 1) + ' dB'}</b></div></div></div>`, (DYNSIDE[k] || DYNSIDE.compressor)(e));
  });

  /* ---------- Speed and pitch ---------- */
  reg('fx-time', 'Effects', 'Speed and pitch', () => {
    const sp = E('speed'), pi = E('pitch'), rate = val('speed', 'rate', sp), st = val('pitch', 'semitones', pi), p = P(), mode = Math.round(val('speed', 'mode', sp));
    return fxpE('fxbrowser', 'Speed and pitch', ['speed', 'pitch'], `
    <div class="vz" style="height:140px">${p && p.length ? v('wave', { src: 'mix', stereo: 1, color: '--t1', step: 2.5 }) : v('wave', { empty: 1, color: '--t1', step: 2.5 })}</div>
    <div class="g2"><div class="box col"><div class="between"><b class="h3">Speed</b><b class="stat">${rate.toFixed(2)}×</b></div>${ps('', 'speed', 'rate', { e: sp, fmt: r => p ? F.len(p.seconds / r) + ' long' : r.toFixed(2) + '×' })}<div class="row">${pchips('speed', 'rate', [[.5, '0.5×'], [.75, '0.75×'], [.85, '0.85×'], [1, '1×'], [1.25, '1.25×'], [1.5, '1.5×'], [2, '2×']], sp)}</div></div>
    <div class="box col"><div class="between"><b class="h3">Pitch</b><b class="stat">${F.minus((st > 0 ? '+' : '') + st.toFixed(1))} st</b></div>${ps('', 'pitch', 'semitones', { e: pi, fmt: s => F.minus((s > 0 ? '+' : '') + Math.round(s * 100)) + ' cents' })}<div class="row">${pchips('pitch', 'semitones', [[-12, '−12'], [-5, '−5'], [-2, '−2'], [0, '0'], [2, '+2'], [5, '+5'], [12, '+12']], pi)}</div></div></div>
    <div class="box row">${pt('Keep pitch when changing speed', 'speed', 'keep_pitch', '', sp)}</div>`, `
    ${sec('Mode', `${pseg('speed', 'mode', ['Music', 'Voice', 'Varispeed'], sp, 'aria-label="Mode"')}<span class="dim" style="font-size:12.5px">Music keeps drum hits sharp. Voice keeps speech natural. Varispeed links speed and pitch, like a turntable.</span>`)}
    ${pt('Preserve formants', 'speed', 'formant', 'Stops voices sounding like chipmunks or giants', sp)}
    ${sec('Tempo', `<div class="g2">${fld('Tempo', inp(p && p.tempo ? p.tempo.toFixed(1) : '', 'BPM', 'data-projset="tempo" inputmode="decimal" aria-label="Tempo"'))}<div class="fld"><span>Tap</span>${btn('Tap', { x: 'data-act="tap-tempo"' })}</div></div><div class="row">${tg('Show beat grid', !!D.ui.grid, '', 'data-act="ui-toggle" data-k="grid"')}</div>`)}
    ${fld('Final render', seg(['Fast', 'High', 'Mastering'], 1, 'aria-label="Final render"'))}${mode === 2 ? cal('info', 'Varispeed changes speed and pitch together.') : ''}`);
  });

  /* ---------- Clean up ---------- */
  const CLEAN = [['noise', 'Noise'], ['hum', 'Hum'], ['declick', 'Clicks'], ['declip', 'Clipping'], ['dewind', 'Wind'], ['stereo', 'Stereo tools']];
  const cleanKind = () => { const s = D.sel.fx && F.findFx(D.sel.fx); if (s && CLEAN.some(d => d[0] === s.kind)) return s.kind; return D.ui.clean || 'noise'; };
  const CLEANSIDE = {
    noise: e => `${sec('Noise profile', `<div class="box" style="padding:10px"><div class="between"><b>Room noise</b>${e && e.profile ? pill('Learned', 'ok') : pill('Automatic', '')}</div><span class="lab">${e && e.profile ? 'Learned from your recording' : 'Flux tracks the noise floor as it plays'}</span></div>${btn('Learn from selection', { i: 'mic', x: e ? `data-act="learn-noise" data-fx="${e.id}"` : 'data-act="learn-noise" data-kind="noise"' })}`)}
      ${ps('Sensitivity', 'noise', 'sensitivity', { e })}${ps('Smoothing', 'noise', 'smoothing', { e })}${ps('Keep room tone', 'noise', 'keep', { e })}
      ${pt('Listen to what is removed', 'noise', 'listen', 'Should only sound like noise', e)}
      ${cal('info', 'For speech, <a class="act" href="#cleanup" data-go="cleanup">voice cleanup</a> usually sounds better.')}`,
    hum: e => `${fld('Mains', pseg('hum', 'freq', ['Auto', '50 Hz', '60 Hz'], e, 'aria-label="Mains frequency"'))}${ps('Harmonics', 'hum', 'harmonics', { e })}${ps('Width', 'hum', 'q', { e })}${D.problems && D.problems.hum_hz ? cal('check', 'Found hum at ' + F.hz(D.problems.hum_hz) + '.') : ''}`,
    declick: e => `${ps('Sensitivity', 'declick', 'sensitivity', { e })}${D.problems ? cal('info', (D.problems.clicks || 0) + ' clicks found in the mix.') : ''}`,
    declip: e => `${ps('Threshold', 'declip', 'threshold', { e })}${D.problems ? cal('info', (D.problems.clipped_samples || 0) + ' clipped samples found.') : ''}`,
    dewind: e => ps('Amount', 'dewind', 'amount', { e }),
    stereo: e => `${ps('Width', 'stereo', 'width', { e })}${ps('Balance', 'stereo', 'balance', { e })}${pt('Swap left and right', 'stereo', 'swap', '', e)}${pt('Mono', 'stereo', 'mono', '', e)}${pt('Invert left', 'stereo', 'invert_l', '', e)}${pt('Invert right', 'stereo', 'invert_r', '', e)}`,
  };
  const BIGKEY = { noise: 'amount', hum: 'harmonics', declick: 'sensitivity', declip: 'threshold', dewind: 'amount', stereo: 'width' };
  reg('fx-clean', 'Effects', 'Noise, hum and clicks', () => {
    const k = cleanKind(), e = E(k), p = P();
    return fxpE('fxbrowser', 'Clean up', k, `
    <div class="tabs" style="padding:0;margin-top:-6px" role="tablist">${CLEAN.map(([kk, t]) => `<button class="${kk === k ? 'on' : ''}" type="button" role="tab" aria-selected="${kk === k}" data-act="clean-tab" data-v="${kk}">${t}</button>`).join('')}</div>
    <div class="g2" style="flex:1;min-height:240px"><div class="col"><span class="lab">Before</span><div class="vz" style="flex:1;min-height:110px">${(F.clips()[0] ? v('spec', { media: F.clips()[0].media }) : v('spec', { empty: 1 }))}</div></div><div class="col"><span class="lab">After</span><div class="vz" style="flex:1;min-height:110px">${p && p.length ? v('spec', { src: 'mix' }) : v('spec', { empty: 1 })}</div></div></div>
    ${ps(k === 'noise' ? 'Amount' : F.spec(k) ? pspec(k, BIGKEY[k]).label : 'Amount', k, BIGKEY[k], { big: 1, e })}`, CLEANSIDE[k](e));
  });

  /* ---------- Silence ---------- */
  reg('fx-silence', 'Effects', 'Silence remover', () => {
    const o = D.ui.sil || { threshold_db: -42, longer_than_s: .6, shorten_to_s: .3, padding_s: .15 }, r = D.ui.silScan, p = P(), keep = D.ui.silKeep || [];
    const len = r ? r.length_s : (p ? p.seconds : 0);
    return Flux.pr('fxbrowser', `<div class="split"><div class="a"><div class="tlbar"><b style="font-size:15px;font-weight:800;white-space:nowrap">Silence remover</b><span class="sp"></span>${btn('Scan again', { k: 'g', i: 'search', x: 'data-act="sil-scan"' })}${btn('', { i: 'x', k: 'g', a: 'Close effect', go: 'fxbrowser' })}</div><div class="in" style="padding:14px;display:flex;flex-direction:column;gap:14px;flex:1">
    <div class="box row" style="justify-content:space-between"><div><b class="stat">${r ? F.minus('-' + F.len(r.removed_s)) : '—'}</b> <span class="dim">from ${F.len(len)}</span></div><span>${r ? pill(r.pauses.length + ' pause' + (r.pauses.length === 1 ? '' : 's') + ' found', r.pauses.length ? 'ok' : '') : pill('Not scanned')}</span></div>
    <div class="vz" style="height:180px;position:relative">${p && p.length ? v('wave', { src: 'mix', kind: 'voice', color: '--t1', step: 2 }) : v('wave', { empty: 1 })}${r ? r.pauses.map(x => `<span style="position:absolute;left:${x.start / (len * F.sr()) * 100}%;top:0;bottom:0;width:${Math.max(.4, (x.end - x.start) / (len * F.sr()) * 100)}%;background:color-mix(in srgb,var(--sig) 26%,transparent);border-inline:1.5px solid var(--sig)"></span>`).join('') : ''}</div>
    <div class="lst box" style="max-height:220px;overflow:auto">${r && r.pauses.length ? r.pauses.map(x => `<div><span class="mono">${F.pos(x.start, F.sr())}</span><div class="ti"><b>${x.seconds.toFixed(1)} s pause</b><span>${keep.includes(x.index) ? 'Keep' : 'Shorten to ' + Math.round(o.shorten_to_s * 1000) + ' ms'}</span></div>${btn('', { i: 'play', k: 'g', a: 'Preview', x: `data-act="seek-play" data-pos="${Math.max(0, x.start - F.sr())}"` })}${tg('', !keep.includes(x.index), '', `data-act="sil-keep" data-i="${x.index}" data-label="Shorten pause at ${F.pos(x.start, F.sr())}"`)}</div>`).join('') : `<div><div class="ti"><span>${r ? 'No pauses longer than ' + o.longer_than_s.toFixed(1) + ' s.' : 'Press Scan to find pauses.'}</span></div></div>`}</div></div></div>
    <aside class="b in">${sl('Longer than', (o.longer_than_s - .1) / 2.9 * 100, o.longer_than_s.toFixed(1) + ' s', { bind: 'sil:longer_than_s:0.1:3:s' })}${sl('Shorten to', o.shorten_to_s / 1.5 * 100, Math.round(o.shorten_to_s * 1000) + ' ms', { bind: 'sil:shorten_to_s:0:1.5:ms' })}${sl('Padding', o.padding_s / .7 * 100, Math.round(o.padding_s * 1000) + ' ms', { bind: 'sil:padding_s:0:0.7:ms' })}${sl('Threshold', (o.threshold_db + 70) / 60 * 100, F.db(o.threshold_db, 0) + ' dB', { bind: 'sil:threshold_db:-70:-10:dB' })}
    ${fld('Detect', seg(['Speech only', 'Any audio'], 1, 'aria-label="Detect"'))}
    ${tg('Protect breaths', true, 'Padding keeps breaths next to speech')}${tg('Crossfade joins', true, 'Every join gets a 10 ms crossfade')}
    ${btn(r ? 'Apply to ' + (r.pauses.length - keep.length) : 'Scan', { k: 'p', lg: 1, x: r ? 'data-act="sil-apply"' + (r.pauses.length - keep.length ? '' : ' disabled') : 'data-act="sil-scan"' })}</aside></div>`);
  });

  /* ---------- Reverb and delay ---------- */
  const RV = [['convolver', 'Convolution'], ['reverb', 'Plate'], ['reverb', 'Hall'], ['delay', 'Delay']];
  const ROOMP = [['Cathedral', 6.8], ['Concert hall', 3.2], ['Plate', 1.4], ['Bathroom', .9], ['Cave', 4.4], ['Parking garage', 2.6]];
  reg('fx-reverb', 'Effects', 'Reverb and delay', () => {
    const s = D.sel.fx && F.findFx(D.sel.fx); let tab = D.ui.rvTab != null ? D.ui.rvTab : 2; if (s && s.kind === 'delay') tab = 3; else if (s && s.kind === 'convolver') tab = 0;
    const k = RV[tab][0], e = E(k), dec = k === 'reverb' ? val('reverb', 'decay', e) : 1.2;
    const main = k === 'delay' ? `<div class="vz" style="height:190px">${v('decay', { rt: Math.max(.2, val('delay', 'time', e) / 1000 * 8 * val('delay', 'feedback', e) / 100) })}</div>` : `<div class="vz" style="height:190px">${v('decay', { rt: dec })}</div>
    <div class="g3">${ROOMP.map(([n, rt]) => `<button class="tile ${Math.abs(dec - rt) < .05 ? 'on' : ''}" type="button" data-act="fx-param" ${ref('reverb', E('reverb'))} data-k="decay" data-v="${rt}"><b>${n}</b><span class="mono">${rt.toFixed(1)} s</span></button>`).join('')}</div>`;
    const side = k === 'delay' ? `${grid(ps('Time', 'delay', 'time', { e }), ps('Feedback', 'delay', 'feedback', { e }), ps('Tone', 'delay', 'tone', { e }), mixsl('delay', e))}${pt('Ping-pong', 'delay', 'pingpong', '', e)}`
      : k === 'convolver' ? `${ps('Level', 'convolver', 'gain', { e })}${mixsl('convolver', e)}${btn('Load your own impulse response', { i: 'imp', x: e ? `data-act="load-ir" data-fx="${e.id}"` : 'data-act="load-ir"' })}${e && e.ir ? cal('check', 'Impulse response loaded.') : cal('info', 'Load a WAV, FLAC or other audio file recorded in a real space.')}`
        : `${grid(mixsl('reverb', e), ps('Decay', 'reverb', 'decay', { e }), ps('Predelay', 'reverb', 'predelay', { e }), ps('Damping', 'reverb', 'damping', { e }), ps('Low cut', 'reverb', 'lowcut', { e }), ps('Width', 'reverb', 'width', { e }))}
    ${pt('Reverse', 'reverb', 'reverse', '', e)}${tg('Include tail on export', true, 'Exports run until the tail is silent')}
    ${btn('Load your own impulse response', { i: 'imp', x: 'data-act="ui-set" data-k="rvTab" data-v="0"' })}${btn('Design a room', { i: 'room', go: 'roomdesign' })}`;
    return fxpE('fxbrowser', RV[tab][1] === 'Delay' ? 'Delay' : 'Reverb', k, `
    <div class="tabs" style="padding:0;margin-top:-6px" role="tablist">${RV.map((t, i) => `<button class="${i === tab ? 'on' : ''}" type="button" role="tab" aria-selected="${i === tab}" data-act="rv-tab" data-v="${i}">${t[1]}</button>`).join('')}</div>${main}`, side);
  });

  /* ---------- Generic effect editor, for kinds without their own screen ---------- */
  reg('fxp', 'Effects', 'Effect settings', () => {
    const e = D.sel.fx && F.findFx(D.sel.fx), kind = e ? e.kind : (D.ui.fxkind || 'geq'), s = F.spec(kind);
    if (!s) return Flux.R('rack');
    const ctl = p => p.unit === 'bool' ? pt(esc(p.label), kind, p.key, '', e) : p.unit === 'enum' ? fld(esc(p.label), pseg(kind, p.key, p.options.map(o => esc(o.charAt(0).toUpperCase() + o.slice(1))), e, `aria-label="${esc(p.label)}"`)) : p.key === 'seed' ? fld('Seed', `<div class="row" style="flex-wrap:nowrap">${inp(String(Math.round(val(kind, 'seed', e))), '', `data-fxinput="seed" ${ref(kind, e)} inputmode="numeric" aria-label="Seed"`)}${btn('Randomize', { k: 'g', x: `data-act="fx-seed" ${ref(kind, e)}` })}</div>`) : ps(esc(p.label), kind, p.key, { e });
    const cont = s.params.filter(p => !/bool|enum/.test(p.unit) && p.key !== 'seed'), rest = s.params.filter(p => /bool|enum/.test(p.unit) || p.key === 'seed');
    return fxpE('fxbrowser', esc(s.name), kind, `<div class="ctl-grid" style="grid-template-columns:repeat(auto-fill,minmax(220px,1fr))">${cont.map(ctl).join('')}</div>${rest.map(ctl).join('')}`, `<h3 class="h3">${esc(s.name)}</h3><span class="lab">${esc(s.group)}</span>${mixsl(kind, e)}${sec('Presets', `<div class="row">${(D.presets || []).filter(p => p.kind === kind).map(p => chip(esc(p.name), 0, '', `data-act="preset-apply" data-id="${esc(p.id)}" data-kind="${kind}"`)).join('') || '<span class="dim">None saved yet.</span>'}</div>${e ? btn('Save as preset', { i: 'tag', x: `data-act="preset-save" data-fx="${e.id}"` }) : ''}`)}`);
  });

  /* ---------- Meters ---------- */
  reg('meters', 'Effects', 'Meters', () => {
    const L = (D.loudness && D.loudness.loudness) || {}, pr0 = D.problems || {}, corr = D.loudness && D.loudness.correlation != null ? D.loudness.correlation : pr0.correlation;
    return pr('mix', `<div class="pg in"><h1 class="h1">Meters</h1>
    <div class="g4">${[[F.num(L.integrated, 1), 'Integrated LUFS', 'i'], [F.num(D.st.lufs_s > -70 ? D.st.lufs_s : L.short_term_max, 1), 'Short-term LUFS', 's'], [F.num(L.lra, 1), 'Loudness range LU', ''], [F.num(L.true_peak, 1), 'True peak dBTP', '']].map(r => `<div class="box inv"><div class="stat"${r[2] ? ` data-lufs-${r[2]}` : ''}>${r[0]}</div><span class="lab">${r[1]}</span></div>`).join('')}</div>
    <div class="box col"><span class="lab">Loudness over time</span>${vz('lufs', {}, 140)}</div>
    <div class="g3"><div class="box col"><span class="lab">Spectrum</span>${vz('spectrum', {}, 180)}</div><div class="box col"><span class="lab">Goniometer</span>${vz('gonio', {}, 180)}</div><div class="box col"><span class="lab">Spectrogram</span>${vz('spec', P() && P().length ? { src: 'mix' } : { empty: 1 }, 180)}</div></div>
    <div class="box between"><span class="row">Phase correlation <span class="mono" data-corr>${corr == null ? '—' : F.minus((corr >= 0 ? '+' : '') + corr.toFixed(2))}</span></span><span class="row">Stereo width <span class="mono">${D.loudness && D.loudness.width != null ? Math.round(D.loudness.width * 100) + '%' : '—'}</span></span><span class="row">Mono check ${corr == null ? pill('—') : corr > .2 ? pill('Safe', 'ok') : corr > -.2 ? pill('Check', 'warn') : pill('Cancels', 'bad')}</span></div></div>`);
  });

  /* ---------- Spatial ---------- */
  const MODES = ['orbit', 'spiral', 'eight', 'swarm', 'place', 'path'];
  const srcDesc = e => !e ? 'Not placed' : (['Orbit', 'Spiral', 'Figure 8', 'Swarm', 'Fixed', 'Path'][Math.round(F.pval(e, 'mode'))] || '') + (Math.round(F.pval(e, 'direction')) === 1 ? ', reverse' : '');
  reg('stage', 'Spatial', 'Sound stage: 3D and 8D', () => {
    const e = E('spatial'), mode = Math.round(val('spatial', 'mode', e)), rate = val('spatial', 'rate', e), hp = (D.settings && D.settings.hrtf_profile) || 'C', ts = F.tracks();
    return pr('stage', `<div class="split" data-stage><div class="a"><div class="tlbar">${seg(['Orbit', 'Spiral', 'Figure 8', 'Swarm', 'Place'], Math.min(mode, 4), `data-segact="fx-enum" ${ref('spatial', e)} data-k="mode" aria-label="Movement"`)}<span class="sp" style="flex:1"></span>${seg(['Drag', 'Draw path'], D.ui.stageTool === 'draw' || mode === 5 ? 1 : 0, 'data-segact="stage-tool" aria-label="Tool"')}${btn('Headphone check', { k: 'g hide-s', i: 'head', go: 'hpcheck' })}</div>
    <div style="flex:1;display:grid;grid-template-rows:minmax(0,1fr) 92px;min-height:440px;padding:10px;gap:10px"><div class="vz" role="img" aria-label="Sound stage. Drag sources around your head.">${v('stage', { mode: MODES[mode] || 'orbit', tool: D.ui.stageTool || 'drag' }, 'position:absolute;inset:0')}</div><div class="vz">${v('elev')}</div></div></div>
    <aside class="b in"><div class="between"><h3 class="h3">${e ? '8D ' + (['orbit', 'spiral', 'figure 8', 'swarm', 'placement', 'path'][mode] || '') : '8D orbit'}</h3>${e ? pill(e.enabled ? 'Live' : 'Bypassed', e.enabled ? 'ok' : '') : pill('Off')}</div>
    ${ps('Rotation speed', 'spatial', 'rate', { big: 1, e })}
    <div class="row">${pseg('spatial', 'direction', ['Clockwise', 'Counter', 'Ping-pong'], e, 'aria-label="Direction"')}</div>
    ${grid(ps('Distance', 'spatial', 'distance', { e }), ps('Elevation', 'spatial', 'elevation', { e }), ps('Depth', 'spatial', 'depth', { e }), ps('Smoothing', 'spatial', 'smoothing', { e }))}
    ${tg('Sync to tempo', !!D.ui.stageSync, P() && P().tempo ? '1 turn every 8 beats' : 'Set a tempo first', 'data-act="stage-sync"' + (P() && P().tempo ? '' : ' disabled'))}${pt('Keep bass centered', 'spatial', 'bass_mono', 'Below ' + F.hz(val('spatial', 'bass_freq', e)), e)}${pt('Doppler', 'spatial', 'doppler', '', e)}
    ${sec('Sources', `<div class="lst">${ts.length ? ts.map((t, i) => { const te = (t.fx || []).find(x => x.kind === 'spatial'); return `<div><span class="dot" style="color:var(--t${1 + ((t.color != null ? t.color : i) % 8)})"></span><div class="ti"><b>${esc(t.name)}</b><span>${te ? srcDesc(te) : (e && e.target === 'master' ? 'With the mix' : 'Center')}</span></div>${te ? '' : btn('', { k: 'g', i: 'plus', a: 'Place ' + esc(t.name) + ' on the stage', x: `data-act="fx-add" data-kind="spatial" data-target="track:${t.id}"` })}</div>`; }).join('') : '<div><div class="ti"><span>No tracks yet</span></div></div>'}</div>${btn('Split into stems first', { i: 'stem', go: 'stems-setup' })}`)}
    ${sec('Input', fld('Treat stereo as', pseg('spatial', 'input', ['Two speakers', 'Mid and side'], e, 'aria-label="Treat stereo as"')))}
    ${sec('Renderer', `${seg(['Best', 'Lite'], Math.round(val('spatial', 'lite', e)) ? 1 : 0, `data-segact="fx-bool" ${ref('spatial', e)} data-k="lite" aria-label="Renderer"`)}<div class="row"><a class="dim" href="#hrtf" data-go="hrtf">Head shape: Profile ${esc(hp)}</a></div>`)}
    ${btn('Record movement as automation', { i: 'rec', x: e ? `data-act="stage-bake" data-fx="${e.id}"` : 'disabled' })}
    ${cal('warn', 'On speakers this collapses. Use headphones, or check the mono mix.', 'warn')}</aside></div>`);
  });

  reg('rooms', 'Spatial', 'Rooms', () => {
    const e = E('reverb'), rooms = (D.styles && D.styles.rooms) || [], dec = e ? val('reverb', 'decay', e) : null;
    return pr('stage', `<div class="pg in"><div class="pgh"><div><h1 class="h1">Put it in a room.</h1><p class="lede">Each room is a measured decay, rendered around your head.</p></div>${btn('Design your own', { k: 'p', i: 'room', go: 'roomdesign' })}</div>
    <div class="g4">${rooms.map(r => `<button class="tile ${dec != null && Math.abs(dec - r.rt60) < .05 ? 'on' : ''}" type="button" data-act="preset-apply" data-id="room-${r.name.toLowerCase().replace(/ /g, '-')}" data-kind="reverb"><div class="thumb">${v('decay', { rt: r.rt60 })}</div><b>${esc(r.name)}</b><span class="mono">RT60 ${r.rt60.toFixed(1)} s</span></button>`).join('')}</div>
    <div class="box g3">${mixsl('reverb', e)}${ps('Your distance', 'reverb', 'predelay', { e, fmt: x => Math.max(1, Math.round(x / 3)) + ' m' })}${ps('Early reflections', 'reverb', 'size', { e })}</div></div>`);
  });

  /* Sabine RT60 = 0.161 V / A. Absorption at 1 kHz for each surface. */
  const WALL = [['Plaster', .04], ['Concrete', .02], ['Brick', .04], ['Wood', .09], ['Glass', .04], ['Curtain', .5]], FLOOR = [['Wood', .07], ['Carpet', .37], ['Stone', .02], ['Tile', .01]], CEIL = [['Plaster', .04], ['Acoustic tile', .7], ['Wood', .09]];
  const roomCalc = r => { const V = r.w * r.d * r.h, A = 2 * (r.w + r.d) * r.h * WALL[r.wall][1] + r.w * r.d * (FLOOR[r.floor][1] + CEIL[r.ceil][1]); return { rt: Math.min(12, .161 * V / Math.max(A, .1)), first: Math.min(r.w, r.d, r.h) / 343 * 1000 }; };
  Flux.roomCalc = roomCalc;
  reg('roomdesign', 'Spatial', 'Room designer', () => {
    const r = Object.assign({ w: 12, d: 18, h: 6, wall: 0, floor: 0, ceil: 0 }, D.ui.room || {}), c = roomCalc(r);
    return pr('stage', `<div class="split"><div class="a"><div class="tlbar"><b style="font-size:15px;font-weight:800">Room designer</b><span class="sp" style="flex:1"></span>${btn('', { i: 'x', k: 'g', a: 'Close', go: 'rooms' })}</div><div style="flex:1;padding:10px;display:grid;grid-template-rows:minmax(0,1fr) 110px;gap:10px;min-height:440px"><div class="vz" role="img" aria-label="Room in perspective with reflection paths">${v('room', { w: r.w / 12, d: r.d / 15, h: r.h / 8.5 }, 'position:absolute;inset:0')}</div><div class="vz">${v('decay', { rt: c.rt })}</div></div></div>
    <aside class="b in"><h3 class="h3">Size</h3><div class="col">${sl('Width', (r.w - 2) / 28 * 100, Math.round(r.w) + ' m', { bind: 'room:w:2:30:m' })}${sl('Depth', (r.d - 2) / 38 * 100, Math.round(r.d) + ' m', { bind: 'room:d:2:40:m' })}${sl('Height', (r.h - 2) / 18 * 100, Math.round(r.h) + ' m', { bind: 'room:h:2:20:m' })}</div>
    ${sec('Walls', `${fld('Walls', sel(WALL.map((x, i) => [i, x[0]]), r.wall, 'data-room="wall" aria-label="Walls"'))}${fld('Floor', sel(FLOOR.map((x, i) => [i, x[0]]), r.floor, 'data-room="floor" aria-label="Floor"'))}${fld('Ceiling', sel(CEIL.map((x, i) => [i, x[0]]), r.ceil, 'data-room="ceil" aria-label="Ceiling"'))}`)}
    ${sec('Result', `<div class="g2"><div><b class="b9" style="font-size:26px">${c.rt.toFixed(1)} s</b><br><span class="lab">RT60</span></div><div><b class="b9" style="font-size:26px">${Math.round(c.first)} ms</b><br><span class="lab">First reflection</span></div></div>`)}
    ${tg('Link source and listener to the stage', D.ui.roomLink !== false, '', 'data-act="ui-toggle" data-k="roomLink" data-def="1"')}${btn('Use this room', { k: 'p', lg: 1, x: 'data-act="room-use"' })}</aside></div>`);
  });

  reg('hall', 'Spatial', 'Row 10 and Next door', () => {
    const a = E('row10'), b = E('nextdoor'), p = P();
    return pr('stage', `<div class="pg in"><h1 class="h1">Two places to listen from.</h1><div class="g2">
    <div class="box col pal-cathedral" style="gap:14px"><div class="between"><h2 class="h2">Row 10</h2>${onoff('row10', a)}</div><p class="dim">You're at a show, ten rows back, with the crowd around you.</p>${vz('room', { w: 1.5, d: 1.2, h: .7 }, 210)}${ps('Row', 'row10', 'row', { e: a, fmt: x => 'Row ' + Math.round(x) })}${ps('Crowd', 'row10', 'crowd', { e: a })}${ps('PA color', 'row10', 'pa_color', { e: a })}${pt('Applause at the start and end', 'row10', 'applause', '', a)}</div>
    <div class="box col pal-nextdoor" style="gap:14px"><div class="between"><h2 class="h2">Next door</h2>${onoff('nextdoor', b)}</div><p class="dim">The same song, through your neighbor's wall.</p><div class="vz" style="height:210px;filter:blur(2px)">${p && p.length ? v('wave', { src: 'mix', stereo: 1, gain: .55, step: 3 }) : v('wave', { empty: 1 })}</div>${ps('Wall thickness', 'nextdoor', 'wall', { e: b, fmt: x => 'Low-pass ' + F.hz(x) })}${ps('Bass through wall', 'nextdoor', 'bass', { e: b })}${ps('Their room', 'nextdoor', 'room', { e: b })}${pt('Level drifts over time', 'nextdoor', 'drift', '', b)}</div>
    </div></div>`);
  });

  /* ---------- Creative ---------- */
  const styleAmt = id => (D.ui.styleAmt || {})[id];
  const styleSl = (id, label, big) => { const a = styleAmt(id); return sl(label, a == null ? 0 : a * 100, a == null ? 'Off' : Math.round(a * 100) + '%', { big, bind: 'style:' + id }); };
  reg('creative', 'Creative', 'Slowed, nightcore, lo-fi', () => {
    const lf = E('lofi');
    const lofiCtl = [['Low-pass', 'lowpass'], ['High-pass', 'highpass'], ['Drive', 'drive'], ['Bit depth', 'bits'], ['Sample rate', 'rate'], ['Crackle', 'crackle'], ['Surface', 'surface'], ['Wow', 'wow'], ['Flutter', 'flutter'], ['Pump', 'pump'], ['Room', 'room'], ['Noise', 'noise']];
    return pr('fxbrowser', `<div class="pg in"><h1 class="h1">Change what it is.</h1><div class="g3">
    <div class="box col pal-slowed" style="gap:13px"><h2 class="h2" style="letter-spacing:.01em;font-weight:300;font-size:clamp(30px,4cqw,52px)">Slowed + reverb</h2>${styleSl('slowed', 'Amount')}${ps('Speed', 'speed', 'rate')}${ps('Reverb', 'reverb', 'decay', { fmt: d => (d > 2.5 ? 'Hall ' : d > 1.2 ? 'Room ' : 'Plate ') + d.toFixed(1) + ' s' })}${pt('Keep pitch', 'speed', 'keep_pitch')}${btn(styleAmt('slowed') != null ? 'Remove' : 'Add to chain', { k: 'p', x: `data-act="style-toggle" data-v="slowed"` })}</div>
    <div class="box col pal-nightcore" style="gap:13px"><h2 class="h2" style="transform:skewX(-12deg);transform-origin:left;font-size:clamp(30px,4cqw,52px)">Nightcore</h2>${styleSl('nightcore', 'Amount')}${ps('Speed', 'speed', 'rate')}${ps('Chorus', 'chorus', 'depth')}${pt('Keep pitch', 'speed', 'keep_pitch')}${btn(styleAmt('nightcore') != null ? 'Remove' : 'Add to chain', { x: `data-act="style-toggle" data-v="nightcore"` })}</div>
    <div class="box col pal-lofi" style="gap:13px"><h2 class="h2" style="filter:url(#crush);font-size:clamp(30px,4cqw,52px)">Lo-fi</h2>${ps('Amount', 'lofi', 'amount', { big: 1, e: lf })}<details${D.ui.lofiOpen ? ' open' : ''} data-details="lofiOpen"><summary class="b7" style="cursor:pointer;padding:6px 0">All 12 controls</summary><div class="ctl-grid" style="grid-template-columns:repeat(2,minmax(0,1fr));margin-top:8px">${lofiCtl.map(c => ps(c[0], 'lofi', c[1], { e: lf })).join('')}</div></details>${addBtn('lofi')}</div>
    </div>${cal('info', 'Speed changes apply to the whole mix. They are rendered with the export, and play live as you listen.')}</div>`);
  });

  const TEX = [['Vinyl', 'Crackle', 'vinyl', 'vinyl', 'vinyl'], ['Cassette', 'Hiss, wow', 'cassette', 'tape', 'cassette'], ['Old radio', 'AM band', 'radio', 'radio', 'radio'], ['Phone call', 'Narrow band', 'nextdoor', '', 'phone'], ['Megaphone', 'Hard edge', 'tapestop', '', 'megaphone'], ['Walkie-talkie', 'Squelch', 'robot', 'bot', 'walkie'], ['Underwater', 'Depth', 'underwater', 'water', 'underwater'], ['Distortion', 'Drive', 'nightcore', 'night', 'distortion'], ['Bitcrusher', 'Bits', 'lofi', 'lofi', 'bitcrusher'], ['Chorus', 'Depth', 'slowed', 'slow', 'chorus'], ['Flanger', 'Sweep', 'reverse', '', 'flanger'], ['Rotary', 'Brake', 'stutter', 'stut', 'rotary']];
  reg('textures', 'Creative', 'Textures and media', () => {
    const s = D.sel.fx && F.findFx(D.sel.fx), cur = (s && TEX.some(t => t[4] === s.kind)) ? s.kind : (D.ui.texture || 'vinyl'), t = TEX.find(x => x[4] === cur), e = E(cur), sp = F.spec(cur);
    const ctl = sp ? sp.params.filter(p => p.key !== 'seed').map(p => p.unit === 'enum' ? fld(esc(p.label), pseg(cur, p.key, p.options.map(o => esc(o.charAt(0).toUpperCase() + o.slice(1))), e, `aria-label="${esc(p.label)}"`)) : p.unit === 'bool' ? pt(esc(p.label), cur, p.key, '', e) : ps(esc(p.label), cur, p.key, { e })).join('') : '';
    return pr('fxbrowser', `<div class="split"><div class="a"><div class="pg in" style="max-width:none"><h1 class="h1">Old machines, on purpose.</h1><div class="g3">${TEX.map(x => fxc(x[2], x[3], x[0], x[0], x[1], { btn: `data-act="texture-pick" data-kind="${x[4]}" aria-pressed="${x[4] === cur}"` })).join('')}</div></div></div>
    <aside class="b in"><div class="between"><h3 class="h3">${t[0]}</h3>${onoff(cur, e)}</div>${cur === 'vinyl' ? `<div style="aspect-ratio:1;max-width:220px;margin:0 auto;width:100%;border-radius:50%;background:repeating-radial-gradient(circle,var(--tx) 0 1.5px,var(--s1) 1.5px 5px);display:grid;place-items:center;animation:spinrec 3.6s linear infinite;animation-play-state:${D.st.playing ? 'running' : 'paused'}"><span style="width:32%;aspect-ratio:1;border-radius:50%;background:var(--sig)"></span></div><style>@keyframes spinrec{to{rotate:360deg}}</style>` : ''}
    <div class="ctl-grid" style="grid-template-columns:repeat(2,minmax(0,1fr))">${ctl}${mixsl(cur, e)}</div>
    ${sp && sp.params.some(p => p.key === 'seed') ? fld('Seed', `<div class="row" style="flex-wrap:nowrap">${inp(String(Math.round(val(cur, 'seed', e))), '', `data-fxinput="seed" ${ref(cur, e)} inputmode="numeric" aria-label="Seed"`)}${btn('Randomize', { k: 'g', x: `data-act="fx-seed" ${ref(cur, e)}` })}</div>`) + `<span class="dim" style="font-size:12.5px">The same seed always gives the same ${cur === 'vinyl' ? 'crackle' : 'texture'}, so exports match the preview.</span>` : ''}</aside></div>`);
  });

  reg('glitch', 'Creative', 'Glitch and rhythm', () => {
    const p = P(), tg0 = E('trancegate'), steps = Math.round(val('trancegate', 'steps', tg0));
    return pr('fxbrowser', `<div class="pg in"><h1 class="h1">Break it in time.</h1><div class="vz" style="height:150px">${p && p.length ? v('wave', { src: 'mix', kind: 'drums', color: '--t3', step: 2 }) : v('wave', { empty: 1, color: '--t3' })}</div><div class="g3">
    <div class="box col pal-stutter"><h2 class="h2">Stutter</h2><div class="row">${pchips('stutter', 'division', [[0, '1/4'], [1, '1/8'], [2, '1/16'], [3, '1/32'], [4, '1/64']])}</div>${ps('Probability', 'stutter', 'probability')}${ps('Pitch ramp', 'stutter', 'pitch_ramp')}${ps('Gate', 'stutter', 'gate')}${addBtn('stutter', 'Add', 'p')}</div>
    <div class="box col pal-tapestop"><h2 class="h2">Tape stop</h2>${ps('Stop time', 'tapestop', 'time')}${pseg('tapestop', 'mode', ['Stop', 'Start', 'Both'], E('tapestop'), 'aria-label="Tape stop mode"')}${btn('Add at playhead', { x: 'data-act="tapestop-here"' })}</div>
    <div class="box col pal-reverse"><h2 class="h2" style="transform:scaleX(-1);transform-origin:center;text-align:right">Reverse</h2>${seg(['Clip', 'Region'], 0, 'aria-label="Reverse what"')}${tg('Reverse reverb swell', D.ui.swell !== false, '', 'data-act="ui-toggle" data-k="swell" data-def="1"')}${sl('Swell', (D.ui.swellBeats || 2) / 8 * 100, (D.ui.swellBeats || 2) + ' beat' + ((D.ui.swellBeats || 2) === 1 ? '' : 's'), { bind: 'ui:swellBeats:1:8:int' })}${btn('Apply', { x: 'data-act="reverse-apply"' + (D.sel.clip != null ? '' : ' disabled title="Select a clip first"') })}</div>
    <div class="box col pal-underwater"><h2 class="h2">Granular</h2>${ps('Grain', 'granular', 'grain')}${ps('Spray', 'granular', 'spray')}${pt('Freeze', 'granular', 'freeze')}${addBtn('granular', 'Add')}</div>
    <div class="box col pal-nightcore"><h2 class="h2">Trance gate</h2><div class="row" style="gap:3px">${Array.from({ length: 16 }, (_, i) => `<button type="button" data-act="gate-step" data-i="${i}" ${ref('trancegate', tg0)} aria-pressed="${!!(steps >> i & 1)}" aria-label="Step ${i + 1}" class="tbtn ${steps >> i & 1 ? 'on' : ''}" style="width:calc(12.5% - 3px);height:24px;border-radius:3px;box-shadow:inset 0 0 0 1.5px var(--tx)"></button>`).join('')}</div>${ps('Depth', 'trancegate', 'depth', { e: tg0 })}${addBtn('trancegate', 'Add')}</div>
    <div class="box col pal-robot"><h2 class="h2">Glitch</h2>${ps('Chaos', 'glitch', 'chaos')}${fld('Seed', inp(String(Math.round(val('glitch', 'seed'))), '', `data-fxinput="seed" ${ref('glitch', E('glitch'))} inputmode="numeric" aria-label="Glitch seed"`))}${addBtn('glitch', 'Add')}</div>
    </div><div class="between dim"><span>${p && p.tempo ? 'Tempo: ' + p.tempo.toFixed(1) + ' BPM from your project' : 'No tempo set. Divisions use 120 BPM.'}</span><a class="act" href="#analysis" data-go="analysis">Detect tempo</a></div></div>`);
  });

  const VP = ['Deep', 'Chipmunk', 'Child', 'Elder', 'Shift up', 'Shift down', 'Whisper', 'Announcer', 'Radio DJ', 'Villain', 'Giant', 'Tiny', 'Robot', 'Cyborg', 'Alien', 'Monster'], VS = ['−5 st', '+7 st', '+4 st, formant up', 'Tremor, breath', 'Higher voice', 'Lower voice', 'No pitch', 'Low, close', 'Compressed', 'Dark, slow', '−12 st, huge', '+12 st', 'Ring mod, comb', 'Half machine', 'Warped formants', 'Growl, sub'];
  const VPAL = ['nightcore', 'stutter', 'radio', 'lofi', 'reverse', 'slowed', 'nextdoor', 'cassette', 'tapestop', 'vinyl', 'cathedral', 'underwater', 'robot', 'robot', 'underwater', 'tapestop'];
  reg('voice', 'Creative', 'Voice changer', () => {
    const e = E('voice'), cur = D.ui.voicePreset;
    return pr('fxbrowser', `<div class="split"><div class="a"><div class="pg in" style="max-width:none"><h1 class="h1">Be someone else.</h1>
    <div class="g4">${VP.map((n, i) => fxc(VPAL[i], i === 12 ? 'bot' : i === 5 ? 'slow' : '', n, n, VS[i], { btn: `data-act="preset-apply" data-id="voice-${n.toLowerCase().replace(/ /g, '-')}" data-kind="voice" aria-pressed="${cur === 'voice-' + n.toLowerCase().replace(/ /g, '-')}"` })).join('')}</div>
    ${cal('info', 'These are original characters. Flux doesn\'t include voices that imitate real people.')}</div></div>
    <aside class="b in"><div class="between"><h3 class="h3">${cur ? esc(VP.find(n => 'voice-' + n.toLowerCase().replace(/ /g, '-') === cur) || 'Voice') : 'Voice'}</h3>${e ? pill(e.enabled ? 'Live' : 'Bypassed', e.enabled ? 'ok' : '') : pill('Off')}</div>
    <div class="box col"><button class="btn lg magic ${D.ui.monitor ? 'on' : ''}" type="button" data-act="voice-monitor" aria-pressed="${!!D.ui.monitor}">${ic('mic')}<span>Talk through it</span></button><div class="between mono dim"><span>Delay ${D.st.engine && D.st.engine.buffer ? Math.round(D.st.engine.buffer * 2 / (D.st.engine.sample_rate || 48000) * 1000 + 3) : '—'} ms</span><span>Use headphones</span></div></div>
    ${ps('Pitch', 'voice', 'pitch', { e })}${ps('Formant', 'voice', 'formant', { e })}
    ${sec('Machine', `${ps('Ring mod', 'voice', 'ring_freq', { e })}${ps('Ring mix', 'voice', 'ring', { e })}${ps('Comb', 'voice', 'comb', { e })}${ps('Vocoder', 'voice', 'vocoder', { e })}${fld('Vocoder carrier', pseg('voice', 'carrier', ['Saw', 'Noise', 'Chord'], e, 'aria-label="Vocoder carrier"'))}${ps('Bands', 'voice', 'bands', { e })}`)}
    ${sec('Hard tune', `${pt('On', 'voice', 'tune', '', e)}<div class="g2">${fld('Key', sel(['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'].map((k, i) => [i, k]), Math.round(val('voice', 'key', e)), `data-fxsel="key" ${ref('voice', e)} aria-label="Key"`))}${fld('Scale', sel([[0, 'Major'], [1, 'Minor'], [2, 'Chromatic']], Math.round(val('voice', 'scale', e)), `data-fxsel="scale" ${ref('voice', e)} aria-label="Scale"`))}</div>${ps('Retune speed', 'voice', 'retune', { e })}`)}${ps('Tremor', 'voice', 'tremor', { e })}</aside></div>`);
  });
})();
