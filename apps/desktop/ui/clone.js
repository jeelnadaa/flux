/* Voice cloning: the model picked for this computer, consent-gated voice profiles, and scripts with sounds.
   Composed from the kit (wiz wide, box, sec, fld, inp, seg, tg, btn, cal, pill, lst). See design/LIVE.md for the
   pattern. Nothing here sends anything anywhere. */
(function () {
  const { reg, wiz, btn, seg, tg, pill, fld, inp, sec, cal, ic } = Flux, F = FD, D = FD.D, esc = F.esc;
  const call = (m, p) => F.call(m, p);
  const U = () => D.ui.cl = D.ui.cl || { name: '', owner: 'me', agree: false, consent: '', reference: '', script: 'So I opened the box [laugh] and, hmm, it was empty. [sigh] Of course it was.', err: null, msg: null };
  const tip = t => ` data-tip="${esc(t)}"`;
  const base = p => (p || '').split(/[\\/]/).pop();
  const SPEED = { fast: 'faster than real time', ok: 'about real time', slow: 'slower than real time' };
  const DEV = { cuda: 'NVIDIA graphics chip', mps: 'Apple graphics cores', cpu: 'processor' };

  function computer(st) {
    const h = st.hardware || {}, pk = st.pick || {}, models = st.models || [];
    return `<div class="box col">
      <div class="row" style="gap:6px">${pill(h.os + ' · ' + h.arch)}${pill(h.cores + ' cores')}${h.ram_gb ? pill(F.num(h.ram_gb, 0) + ' GB memory') : ''}${h.gpu_name ? pill(esc(h.gpu_name) + (h.vram_gb ? ' · ' + F.num(h.vram_gb, 0) + ' GB' : '')) : pill('No graphics chip found')}</div>
      <p style="margin:0"${tip('Flux picks the model from your graphics chip and memory. Nothing is downloaded without asking.')}><b>${pk.model ? esc((models.find(m => m.id === pk.model) || {}).name || pk.model) : 'No model fits'}</b>${pk.model ? `, on the ${DEV[pk.device] || pk.device}, ${SPEED[pk.speed] || ''}.` : '.'} ${esc(pk.reason || '')}</p>
      <div class="lst">${models.map(m => `<div${tip(m.note)}><div class="ti"><b>${esc(m.name)}${m.id === pk.model ? ' · picked for this computer' : m.id === pk.convert ? ' · for your own recordings' : ''}</b><span style="white-space:normal">${esc(m.note)} ${esc(m.needs)}</span></div><span class="mono dim" style="white-space:nowrap">${esc(m.licence)} · ~${F.num(m.size_gb, 1)} GB</span></div>`).join('')}</div>
      ${cal(st.available ? 'check' : 'info', esc(st.why || ''))}
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
  function script() {
    const u = U(), parts = u.parts;
    return `<div class="box col">
      <textarea class="inp" rows="5" aria-label="Script" data-cin="script" style="font-size:17px;font-weight:500"${tip('Write sounds as [laugh], (sighs), *chuckles* or {cough}. Write hesitations as words: hmm, um, uh.')}>${esc(u.script)}</textarea>
      <div class="row" style="gap:6px">${(D.ctags && D.ctags.tags || []).map(t => `<button type="button" class="chip" data-cact="tag" data-t="${esc(t.tag)}"${tip('Insert [' + t.tag + '] at the cursor. Also written as ' + t.also.slice(0, 3).join(', ') + '.')}>[${esc(t.tag)}]</button>`).join('')}</div>
      ${parts ? `<p style="margin:0;line-height:1.9">${parts.map(p => p.type === 'sound' ? pill(esc(p.value), 'sig') : esc(p.value)).join(' ')}</p>` : ''}
      <div class="row">${btn('Check script', { x: 'data-cact="check"' + tip('Shows the words and sounds in order, as the model will read them.') })}
        ${btn('Speak in this voice', { k: 'p', i: 'play', x: `data-cact="speak"${u.voice ? '' : ' disabled'}` + tip('Speaks the script in the selected voice, labelled synthetic.') })}
        ${btn('Turn my recording into this voice', { i: 'swap', x: `data-cact="convert"${u.voice ? '' : ' disabled'}` + tip('Record yourself saying it, with your own laughs and pauses, and Flux re-voices it, keeping your timing.') })}</div>
      ${u.err ? cal('warn', esc(u.err), 'warn') : ''}${u.msg ? cal('check', esc(u.msg)) : ''}
    </div>`;
  }
  reg('clone', 'AI', 'Voice cloning', () => {
    const st = D.cstatus || {};
    return wiz(0, 0, `<h1 class="h1">Clone a voice, with consent.</h1>
      <p class="lede">Only your own voice, or someone who said yes on a recording. Everything stays on this computer, and anything spoken is labelled synthetic.</p>
      ${sec('This computer', computer(st))}
      ${sec('Make a voice', consentForm())}
      ${sec('Voices', voices())}
      ${sec('Script', script())}
      <div class="between">${btn('Back', { k: 'g', go: 'tts' })}${btn('Text to speech', { go: 'tts' })}</div>`, true);
  });

  async function reload() { F.stale('cprofiles', 'cstatus'); await F.need(['cprofiles', 'cstatus'], true); Flux.rr(); }
  const A = {
    pick: async el => { const fs = await F.pickFiles({ multiple: false, kind: 'audio' }); if (fs && fs[0]) { U()[el.dataset.k] = fs[0]; Flux.rr(); } },
    agree: () => { U().agree = !U().agree; Flux.rr(); },
    create: async () => { const u = U(); u.err = null; try { const p = await call('clone.profile.create', { name: u.name.trim(), owner: u.owner, consent: u.agree, consent_audio: u.consent, reference: u.reference }); u.voice = p.id; u.msg = `Saved ${p.name}'s voice with their consent recording.`; u.name = ''; u.agree = false; u.consent = u.reference = ''; } catch (e) { u.err = e.message; } reload(); },
    use: el => { U().voice = el.dataset.id; Flux.rr(); },
    del: async el => { if (!confirm('Delete this voice and its recordings? This can\'t be undone.')) return; try { await call('clone.profile.delete', { id: el.dataset.id }); if (U().voice === el.dataset.id) U().voice = null; } catch (e) { U().err = e.message; } reload(); },
    tag: el => { const ta = document.querySelector('[data-cin="script"]'), t = '[' + el.dataset.t + ']'; const u = U(), at = ta ? ta.selectionStart : u.script.length; u.script = u.script.slice(0, at) + (at && u.script[at - 1] !== ' ' ? ' ' : '') + t + ' ' + u.script.slice(at); u.parts = null; Flux.rr(); },
    check: async () => { const u = U(); u.err = u.msg = null; try { const r = await call('clone.script', { text: u.script }); u.parts = r.parts; } catch (e) { u.parts = null; u.err = e.message; } Flux.rr(); },
    speak: async () => { const u = U(); u.err = u.msg = null; try { await call('clone.speak', { voice: u.voice, text: u.script }); u.msg = 'Done. The speech is labelled synthetic.'; } catch (e) { u.err = e.message; } Flux.rr(); },
    convert: async () => { const u = U(); u.err = u.msg = null; const fs = await F.pickFiles({ multiple: false, kind: 'audio' }); if (!fs || !fs[0]) return; try { await call('clone.convert', { voice: u.voice, path: fs[0] }); u.msg = 'Done. The result is labelled synthetic.'; } catch (e) { u.err = e.message; } Flux.rr(); },
  };
  document.addEventListener('click', e => {
    if (e.button) return;
    const sb = e.target.closest('.seg[data-cseg] button'); if (sb) { const i = [...sb.parentElement.querySelectorAll('button')].indexOf(sb); U().owner = i ? 'someone-else' : 'me'; return; }
    const a = e.target.closest('[data-cact]'); if (!a || a.disabled) return; e.preventDefault(); const fn = A[a.dataset.cact]; if (fn) fn(a);
  });
  let cinT = 0;
  document.addEventListener('input', e => { const k = e.target.dataset && e.target.dataset.cin; if (!k) return; U()[k] = e.target.value; if (k === 'script') U().parts = null; if (k === 'name') { clearTimeout(cinT); cinT = setTimeout(Flux.rr, 400); } });
})();
