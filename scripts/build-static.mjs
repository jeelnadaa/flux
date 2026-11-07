// Builds the website for Cloudflare Pages into dist-static/: the landing page as index.html, the download page
// (served at /download by Pages' pretty URLs), their scripts, the app's fonts and icons, and _headers/_redirects.
// The site has no build step beyond copying; it reads the latest GitHub release in the visitor's browser.
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const OUT = 'dist-static';
const SITE = 'site';
const UI = 'apps/desktop/ui';

rmSync(OUT, { recursive: true, force: true });
mkdirSync(OUT, { recursive: true });

const pages = { 'index.html': 'landing.html', 'download.html': 'download.html' };
for (const [to, from] of Object.entries(pages)) {
  const html = readFileSync(join(SITE, from), 'utf8');
  if (/href="\/app\b/.test(html)) throw new Error(`${from} still links to the removed web app`);
  writeFileSync(join(OUT, to), html);
}
for (const f of ['landing.js', 'download.js', 'release.js']) cpSync(join(SITE, f), join(OUT, f));
cpSync(join(SITE, 'icons'), join(OUT, 'icons'), { recursive: true });
cpSync(join(UI, 'fonts.css'), join(OUT, 'fonts.css'));
cpSync(join(UI, 'fonts'), join(OUT, 'fonts'), { recursive: true });

const csp = [
  "default-src 'self'",
  "script-src 'self' https://cdnjs.cloudflare.com https://cdn.jsdelivr.net",
  "style-src 'self' 'unsafe-inline'",
  "font-src 'self'",
  "img-src 'self' data:",
  'connect-src https://api.github.com',
  "media-src 'self' blob:",
  "object-src 'none'",
  "base-uri 'none'",
  "frame-ancestors 'none'",
].join('; ');
writeFileSync(join(OUT, '_headers'), `/*
  Content-Security-Policy: ${csp}
  X-Content-Type-Options: nosniff
  Referrer-Policy: strict-origin-when-cross-origin
  Permissions-Policy: camera=(), microphone=(), geolocation=()
/fonts/*
  Cache-Control: public, max-age=31536000, immutable
/icons/*
  Cache-Control: public, max-age=86400
`);
writeFileSync(join(OUT, '_redirects'), `/app / 301
/app/* / 301
/download/ /download 301
/releases https://github.com/jeelnadaa/flux/releases/latest 302
`);

// Every local file a page references must exist in the output.
let missing = [];
for (const page of Object.keys(pages)) {
  const html = readFileSync(join(OUT, page), 'utf8');
  for (const m of html.matchAll(/(?:src|href)="([^"#:]+)"/g)) {
    const ref = m[1].replace(/^\//, '');
    if (!ref || ref === 'download') continue;
    if (!existsSync(join(OUT, ref))) missing.push(`${page} -> ${m[1]}`);
  }
}
if (missing.length) throw new Error('Missing files:\n' + missing.join('\n'));

const size = d => readdirSync(d).reduce((n, f) => { const p = join(d, f), s = statSync(p); return n + (s.isDirectory() ? size(p) : s.size); }, 0);
console.log(`dist-static: ${readdirSync(OUT).length} entries, ${(size(OUT) / 1048576).toFixed(1)} MB`);
