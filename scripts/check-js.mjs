// Syntax-checks every JavaScript file the app and the site ship. Exits non-zero on the first error.
import { execFileSync } from 'node:child_process';
import { readdirSync, existsSync } from 'node:fs';
import { join } from 'node:path';

const dirs = ['apps/desktop/ui', 'site', 'scripts'];
let n = 0;
for (const d of dirs) {
  if (!existsSync(d)) continue;
  for (const f of readdirSync(d)) {
    if (!/\.(m?js)$/.test(f)) continue;
    execFileSync(process.execPath, ['--check', join(d, f)], { stdio: 'inherit' });
    n++;
  }
}
console.log(`${n} files ok`);
