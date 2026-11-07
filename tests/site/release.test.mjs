// The download page picks the right file for each computer from a GitHub release. Run: npm run test:site
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import vm from 'node:vm';

// release.js is a classic browser script that exports its helpers through `module` when one exists.
const sandbox = { module: { exports: {} } };
vm.runInNewContext(readFileSync('site/release.js', 'utf8'), sandbox);
const R = sandbox.module.exports;

const v = '2.0.42';
const names = [
  `Flux_${v}_x64-setup.exe`, `Flux_${v}_x64-setup.exe.sig`, `Flux_${v}_x64_en-US.msi`, `Flux_${v}_x64_en-US.msi.sig`,
  `Flux_${v}_aarch64.dmg`, `Flux_${v}_x64.dmg`, `Flux_aarch64.app.tar.gz`, `Flux_aarch64.app.tar.gz.sig`, `Flux_x64.app.tar.gz`,
  `Flux_${v}_amd64.AppImage`, `Flux_${v}_amd64.AppImage.sig`, `Flux_${v}_amd64.deb`, `Flux-${v}-1.x86_64.rpm`, 'latest.json',
  'flux-mcp-x86_64-pc-windows-msvc.exe',
];
const release = { tag_name: `v${v}`, published_at: '2026-11-07T10:00:00Z', assets: names.map((n, i) => ({ name: n, size: 1048576 * (i + 1), browser_download_url: `https://example.invalid/${n}` })) };

test('every installer is found, and updater files are not offered as downloads', () => {
  const f = R.classify(release);
  assert.equal(f['win-exe'].name, `Flux_${v}_x64-setup.exe`);
  assert.equal(f['win-msi'].name, `Flux_${v}_x64_en-US.msi`);
  assert.equal(f['mac-arm'].name, `Flux_${v}_aarch64.dmg`);
  assert.equal(f['mac-intel'].name, `Flux_${v}_x64.dmg`);
  assert.equal(f['linux-appimage'].name, `Flux_${v}_amd64.AppImage`);
  assert.equal(f['linux-deb'].name, `Flux_${v}_amd64.deb`);
  assert.equal(f['linux-rpm'].name, `Flux-${v}-1.x86_64.rpm`);
  for (const n of names.filter(n => /\.sig$|\.tar\.gz$|latest\.json/.test(n))) assert.ok(R.isUpdaterFile(n), n);
  for (const n of names.filter(n => /\.sig$|\.tar\.gz/.test(n))) assert.equal(R.kindOf(n), null, n);
  assert.equal(R.version(release), v);
});

test('the visitor gets the file for their computer', () => {
  const ua = {
    win: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36',
    mac: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15',
    linux: 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36',
    phone: 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 Mobile/15E148',
    android: 'Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 Chrome/130.0 Mobile Safari/537.36',
  };
  assert.equal(R.bestKind(R.detect({ userAgent: ua.win })), 'win-exe');
  assert.equal(R.bestKind(R.detect({ userAgent: ua.mac, platform: 'MacIntel' })), 'mac-arm');
  assert.equal(R.bestKind({ os: 'mac', arch: 'x64' }), 'mac-intel');
  assert.equal(R.bestKind(R.detect({ userAgent: ua.linux })), 'linux-appimage');
  assert.equal(R.bestKind(R.detect({ userAgent: ua.phone })), null);
  assert.equal(R.bestKind(R.detect({ userAgent: ua.android })), null);
  assert.equal(R.size(1048576 * 8.25), '8.3 MB');
  assert.equal(R.size(1048576 * 120), '120 MB');
  assert.equal(R.size(0), '');
});

test('the built site has no web app and only allowed outside scripts', () => {
  for (const p of ['site/landing.html', 'site/download.html']) {
    const h = readFileSync(p, 'utf8');
    assert.ok(!/href="\/app\b/.test(h), `${p} links to the removed web app`);
    assert.ok(!/Web App/i.test(h), `${p} mentions a web app`);
    for (const m of h.matchAll(/<script src="(https:[^"]+)"/g)) assert.match(m[1], /^https:\/\/(cdnjs\.cloudflare\.com|cdn\.jsdelivr\.net)\//);
    assert.ok(/release\.js/.test(h), `${p} doesn't load release.js`);
  }
  if (existsSync('dist-static/_headers')) assert.match(readFileSync('dist-static/_headers', 'utf8'), /connect-src https:\/\/api\.github\.com/);
});
