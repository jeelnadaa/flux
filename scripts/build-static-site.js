import fs from "node:fs";
import path from "node:path";

const ROOT_DIR = process.cwd();
const WEB_DIR = path.join(ROOT_DIR, "apps", "web");
const OUT_DIR = path.join(ROOT_DIR, "dist-static");

function copyDirRecursive(src, dest) {
  if (!fs.existsSync(dest)) {
    fs.mkdirSync(dest, { recursive: true });
  }
  const entries = fs.readdirSync(src, { withFileTypes: true });
  for (const entry of entries) {
    const srcPath = path.join(src, entry.name);
    const destPath = path.join(dest, entry.name);
    if (entry.isDirectory()) {
      copyDirRecursive(srcPath, destPath);
    } else {
      fs.copyFileSync(srcPath, destPath);
    }
  }
}

function buildStaticSite() {
  console.log("[*] Assembling production static site in dist-static/ for Cloudflare Pages...");

  // Ensure output directory exists
  if (!fs.existsSync(OUT_DIR)) {
    fs.mkdirSync(OUT_DIR, { recursive: true });
  }

  // 1. Copy landing.html as index.html so root domain opens the showcase landing
  const landingHtml = fs.readFileSync(path.join(WEB_DIR, "landing.html"), "utf8");
  fs.writeFileSync(path.join(OUT_DIR, "index.html"), landingHtml);
  fs.writeFileSync(path.join(OUT_DIR, "landing.html"), landingHtml);

  // 2. Copy download.html and write download/index.html for clean Cloudflare Pages URLs
  const downloadHtml = fs.readFileSync(path.join(WEB_DIR, "download.html"), "utf8");
  fs.writeFileSync(path.join(OUT_DIR, "download.html"), downloadHtml);
  const downloadDir = path.join(OUT_DIR, "download");
  if (!fs.existsSync(downloadDir)) {
    fs.mkdirSync(downloadDir, { recursive: true });
  }
  fs.writeFileSync(path.join(downloadDir, "index.html"), downloadHtml);

  // 3. Assemble Web App Editor in /app and /editor
  const appIndexHtml = fs.readFileSync(path.join(WEB_DIR, "index.html"), "utf8");
  // Adjust paths for /app/index.html sub-directory
  const appHtmlAdjusted = appIndexHtml
    .replace('href="manifest.webmanifest"', 'href="/manifest.webmanifest"')
    .replace('href="icons/icon-192.svg"', 'href="/icons/icon-192.svg"')
    .replace('href="fonts.css"', 'href="/fonts.css"')
    .replace('href="themes.css"', 'href="/themes.css"')
    .replace('href="app.css"', 'href="/app.css"')
    .replace('src="dist/bundle.js"', 'src="/dist/bundle.js"');

  const appDir = path.join(OUT_DIR, "app");
  if (!fs.existsSync(appDir)) {
    fs.mkdirSync(appDir, { recursive: true });
  }
  fs.writeFileSync(path.join(appDir, "index.html"), appHtmlAdjusted);

  const editorDir = path.join(OUT_DIR, "editor");
  if (!fs.existsSync(editorDir)) {
    fs.mkdirSync(editorDir, { recursive: true });
  }
  fs.writeFileSync(path.join(editorDir, "index.html"), appHtmlAdjusted);

  // 4. Copy web bundle (dist/bundle.js)
  const bundleSrc = path.join(WEB_DIR, "dist", "bundle.js");
  const bundleDestDir = path.join(OUT_DIR, "dist");
  if (!fs.existsSync(bundleDestDir)) {
    fs.mkdirSync(bundleDestDir, { recursive: true });
  }
  if (fs.existsSync(bundleSrc)) {
    fs.copyFileSync(bundleSrc, path.join(bundleDestDir, "bundle.js"));
  }

  // 5. Copy scripts and styles
  const staticFiles = [
    "landing.js",
    "download.js",
    "app.css",
    "themes.css",
    "fonts.css",
    "manifest.webmanifest",
    "sw.js",
  ];
  for (const file of staticFiles) {
    const src = path.join(WEB_DIR, file);
    if (fs.existsSync(src)) {
      fs.copyFileSync(src, path.join(OUT_DIR, file));
    }
  }

  // 6. Copy fonts and icons
  if (fs.existsSync(path.join(WEB_DIR, "fonts"))) {
    copyDirRecursive(path.join(WEB_DIR, "fonts"), path.join(OUT_DIR, "fonts"));
  }
  if (fs.existsSync(path.join(WEB_DIR, "icons"))) {
    copyDirRecursive(path.join(WEB_DIR, "icons"), path.join(OUT_DIR, "icons"));
  }

  // 6b. Copy flux_wasm.wasm if built
  const wasmCandidate = path.join(
    ROOT_DIR,
    "target",
    "wasm32-unknown-unknown",
    "release",
    "flux_wasm.wasm",
  );
  if (fs.existsSync(wasmCandidate)) {
    fs.copyFileSync(wasmCandidate, path.join(OUT_DIR, "flux_wasm.wasm"));
    fs.copyFileSync(wasmCandidate, path.join(WEB_DIR, "flux_wasm.wasm"));
    console.log("    - flux_wasm.wasm (Compiled WebAssembly Engine)");
  }

  // 7. Add Cloudflare _headers with COOP/COEP and Microphone permissions
  const headersContent = `/*
  X-Frame-Options: SAMEORIGIN
  X-Content-Type-Options: nosniff
  Referrer-Policy: strict-origin-when-cross-origin
  Cross-Origin-Opener-Policy: same-origin
  Cross-Origin-Embedder-Policy: require-corp
  Permissions-Policy: microphone=(self), camera=()

/fonts/*
  Cache-Control: public, max-age=31536000, immutable

/icons/*
  Cache-Control: public, max-age=86400

/dist/*
  Cache-Control: public, max-age=3600
`;
  fs.writeFileSync(path.join(OUT_DIR, "_headers"), headersContent);

  // 8. Add Cloudflare _redirects file for clean routing
  const redirectsContent = `/downloads    /download    301
/get          /download    302
/edit         /app         302
/editor       /app         302
/web          /app         302
/tour         /#scenes     302
`;
  fs.writeFileSync(path.join(OUT_DIR, "_redirects"), redirectsContent);

  console.log("[✓] Static site built successfully in dist-static/");
  console.log(`    - index.html & landing.html (Showcase Landing)`);
  console.log(`    - download/index.html (Native Releases Download Page)`);
  console.log(`    - app/index.html & editor/index.html (Full Web PWA Editor)`);
  console.log(`    - dist/bundle.js (Compiled Application Code)`);
  console.log(`    - _headers (COOP, COEP, Permissions-Policy: microphone=(self))`);
  console.log(`    - _redirects (Clean Cloudflare Pages Routing)`);
}

buildStaticSite();
