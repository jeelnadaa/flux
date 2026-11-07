// Build flux-mcp and copy it where Tauri's externalBin expects it:
// apps/desktop/src-tauri/binaries/flux-mcp-<target-triple>[.exe]
// Usage: node scripts/sidecar.mjs [--target <triple>]
import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';

const i = process.argv.indexOf('--target');
const target = i > 0 ? process.argv[i + 1] : execFileSync('rustc', ['-vV'], { encoding: 'utf8' }).match(/host: (\S+)/)[1];
const ext = target.includes('windows') ? '.exe' : '';
const args = ['build', '--release', '-p', 'flux-mcp', ...(i > 0 ? ['--target', target] : [])];
execFileSync('cargo', args, { stdio: 'inherit' });
const from = join('target', ...(i > 0 ? [target] : []), 'release', `flux-mcp${ext}`);
const dir = join('apps', 'desktop', 'src-tauri', 'binaries');
mkdirSync(dir, { recursive: true });
copyFileSync(from, join(dir, `flux-mcp-${target}${ext}`));
console.log(`sidecar: ${from} -> ${dir}/flux-mcp-${target}${ext}`);
