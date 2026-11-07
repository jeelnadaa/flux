/* Flux release links.
   Reads the latest GitHub release and points every [data-asset] link at the right file, fills
   [data-rel] text (version, date), lists every file in #filelist, and labels [data-os-label] /
   [data-os-href] for the visitor's computer. Works without the API: links fall back to the
   releases page. Pure helpers are exported for tests (tests/site/release.test.mjs). */
(function (root) {
  'use strict';
  const REPO = 'jeelnadaa/flux';
  const API = `https://api.github.com/repos/${REPO}/releases/latest`;
  const PAGE = `https://github.com/${REPO}/releases/latest`;
  const CACHE_KEY = 'flux-release-v1', CACHE_MS = 10 * 60 * 1000;

  /* Which file is which. Names follow tauri-action: Flux_2.0.7_x64-setup.exe, Flux_2.0.7_x64_en-US.msi,
     Flux_2.0.7_aarch64.dmg, Flux_2.0.7_x64.dmg, Flux_2.0.7_amd64.AppImage, Flux_2.0.7_amd64.deb,
     Flux-2.0.7-1.x86_64.rpm. Updater archives (.tar.gz, .sig, latest.json) are not downloads. */
  const KINDS = [
    ['win-exe', /x64-setup\.exe$/i, 'Windows installer'],
    ['win-msi', /x64.*\.msi$/i, 'Windows MSI'],
    ['mac-arm', /(aarch64|arm64).*\.dmg$/i, 'macOS, Apple silicon'],
    ['mac-intel', /(x64|x86_64|intel).*\.dmg$/i, 'macOS, Intel'],
    ['linux-appimage', /\.AppImage$/i, 'Linux AppImage'],
    ['linux-deb', /\.deb$/i, 'Debian and Ubuntu'],
    ['linux-rpm', /\.rpm$/i, 'Fedora and openSUSE'],
  ];
  const kindOf = name => { for (const [id, re] of KINDS) if (re.test(name)) return id; return null; };
  const isUpdaterFile = name => /\.(sig|tar\.gz|zip)$/i.test(name) || /^latest\.json$/i.test(name);

  function classify(release) {
    const out = {};
    for (const a of (release && release.assets) || []) {
      const k = kindOf(a.name);
      if (k && !out[k]) out[k] = { name: a.name, url: a.browser_download_url, size: a.size };
    }
    return out;
  }

  function size(bytes) {
    if (!(bytes > 0)) return '';
    const mb = bytes / 1048576;
    return mb >= 10 ? `${Math.round(mb)} MB` : `${mb.toFixed(1)} MB`;
  }

  function version(release) {
    const t = (release && (release.tag_name || release.name)) || '';
    return t.replace(/^v/i, '');
  }

  /* The visitor's computer: { os: 'win' | 'mac' | 'linux' | null, arch: 'arm' | 'x64' | null } */
  function detect(nav) {
    nav = nav || {};
    const ua = String(nav.userAgent || ''), p = String((nav.userAgentData && nav.userAgentData.platform) || nav.platform || '');
    const s = (p + ' ' + ua).toLowerCase();
    if (/iphone|ipad|android/.test(s)) return { os: null, arch: null };
    if (/win/.test(s)) return { os: 'win', arch: 'x64' };
    // Browsers report "Intel Mac" on Apple silicon too, so the user agent can't settle this.
    if (/mac/.test(s)) return { os: 'mac', arch: 'arm' };
    if (/linux|x11|cros/.test(s)) return { os: 'linux', arch: 'x64' };
    return { os: null, arch: null };
  }
  const BEST = { win: 'win-exe', 'mac-arm': 'mac-arm', 'mac-x64': 'mac-intel', linux: 'linux-appimage' };
  const bestKind = d => (!d || !d.os ? null : BEST[d.os === 'mac' ? `mac-${d.arch || 'arm'}` : d.os]);
  const OS_NAME = { win: 'Windows', mac: 'macOS', linux: 'Linux' };
  const CHAPTER = { win: '#dl-win', mac: '#dl-mac', linux: '#dl-linux' };

  const api = { REPO, API, PAGE, KINDS, kindOf, isUpdaterFile, classify, size, version, detect, bestKind, OS_NAME, CHAPTER };
  if (typeof module === 'object' && module.exports) { module.exports = api; return; }
  root.FluxRelease = api;

  /* ---------- page wiring ---------- */
  const doc = root.document;
  const $$ = s => [...doc.querySelectorAll(s)];
  let who = detect(root.navigator);

  function label() {
    const name = OS_NAME[who.os];
    $$('[data-os-label]').forEach(el => { el.textContent = name ? `Download for ${name}` : el.dataset.osLabel || 'Download Flux'; });
    $$('[data-os-href]').forEach(el => { el.setAttribute('href', `${el.dataset.osHref || ''}${CHAPTER[who.os] || ''}`); });
    $$('.chapter[data-os]').forEach(el => el.classList.toggle('yours', el.dataset.os === who.os));
  }

  function fill(rel) {
    const files = classify(rel), v = version(rel);
    $$('[data-asset]').forEach(el => {
      let k = el.dataset.asset;
      if (k === 'auto') k = bestKind(who);
      const f = k && files[k];
      el.setAttribute('href', f ? f.url : PAGE);
      if (f) el.removeAttribute('target'); else el.setAttribute('target', '_blank');
      const s = el.querySelector('[data-size]');
      if (s) s.textContent = f ? size(f.size) : 'GitHub ↗';
      el.toggleAttribute('data-missing', !f);
    });
    $$('[data-rel="version"]').forEach(el => { el.textContent = v ? `Flux ${v}` : el.textContent; });
    $$('[data-rel="date"]').forEach(el => {
      const d = rel && rel.published_at ? new Date(rel.published_at) : null;
      if (d && !isNaN(d)) el.textContent = `Released ${d.toLocaleDateString(undefined, { year: 'numeric', month: 'long', day: 'numeric' })}`;
    });
    const list = doc.getElementById('filelist');
    if (list) {
      const rows = ((rel && rel.assets) || []).filter(a => !isUpdaterFile(a.name)).sort((a, b) => a.name.localeCompare(b.name));
      list.textContent = '';
      if (!rows.length) { const p = doc.createElement('p'); p.textContent = rel ? 'This release has no files yet. They appear a few minutes after a build finishes.' : 'Couldn\'t reach GitHub. Every file is on the releases page.'; list.appendChild(p); }
      for (const a of rows) {
        const row = doc.createElement('a'); row.href = a.browser_download_url; row.className = 'file';
        const n = doc.createElement('span'); n.textContent = a.name;
        const k = doc.createElement('span'); k.textContent = (KINDS.find(x => x[0] === kindOf(a.name)) || [, , 'Command-line tool'])[2];
        const s = doc.createElement('span'); s.textContent = size(a.size);
        row.append(n, k, s); list.appendChild(row);
      }
    }
    doc.documentElement.dataset.release = rel ? 'ok' : 'offline';
  }

  async function latest() {
    try { const c = JSON.parse(root.sessionStorage.getItem(CACHE_KEY) || 'null'); if (c && Date.now() - c.t < CACHE_MS) return c.r; } catch (e) { /* storage blocked */ }
    const r = await fetch(API, { headers: { Accept: 'application/vnd.github+json' } });
    if (!r.ok) throw new Error(`GitHub answered ${r.status}`);
    const j = await r.json();
    const slim = { tag_name: j.tag_name, name: j.name, published_at: j.published_at, html_url: j.html_url, assets: (j.assets || []).map(a => ({ name: a.name, size: a.size, browser_download_url: a.browser_download_url })) };
    try { root.sessionStorage.setItem(CACHE_KEY, JSON.stringify({ t: Date.now(), r: slim })); } catch (e) { /* storage blocked */ }
    return slim;
  }

  async function start() {
    // Chromium can tell Apple silicon from Intel; other browsers keep the Apple silicon default.
    try {
      const ua = root.navigator.userAgentData;
      if (who.os === 'mac' && ua && ua.getHighEntropyValues) { const h = await ua.getHighEntropyValues(['architecture']); if (h.architecture === 'x86') who = { os: 'mac', arch: 'x64' }; }
    } catch (e) { /* not available */ }
    label();
    if (!$$('[data-asset],[data-rel],#filelist').length) return;
    let rel = null;
    try { rel = await latest(); } catch (e) { rel = null; }
    fill(rel);
  }
  if (doc.readyState === 'loading') doc.addEventListener('DOMContentLoaded', start); else start();
})(typeof window !== 'undefined' ? window : globalThis);
