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
  console.log("[*] Assembling static site in dist-static/ for Cloudflare Pages...");

  // Clean output directory
  if (fs.existsSync(OUT_DIR)) {
    fs.rmSync(OUT_DIR, { recursive: true, force: true });
  }
  fs.mkdirSync(OUT_DIR, { recursive: true });

  // 1. Copy landing.html as index.html so root domain opens the showcase
  const landingHtml = fs.readFileSync(path.join(WEB_DIR, "landing.html"), "utf8");
  fs.writeFileSync(path.join(OUT_DIR, "index.html"), landingHtml);
  fs.writeFileSync(path.join(OUT_DIR, "landing.html"), landingHtml);

  // 2. Copy download.html
  const downloadHtml = fs.readFileSync(path.join(WEB_DIR, "download.html"), "utf8");
  fs.writeFileSync(path.join(OUT_DIR, "download.html"), downloadHtml);

  // 3. Copy scripts and styles
  const staticFiles = ["landing.js", "app.css", "themes.css", "fonts.css", "manifest.webmanifest"];
  for (const file of staticFiles) {
    const src = path.join(WEB_DIR, file);
    if (fs.existsSync(src)) {
      fs.copyFileSync(src, path.join(OUT_DIR, file));
    }
  }

  // 4. Copy fonts and icons
  if (fs.existsSync(path.join(WEB_DIR, "fonts"))) {
    copyDirRecursive(path.join(WEB_DIR, "fonts"), path.join(OUT_DIR, "fonts"));
  }
  if (fs.existsSync(path.join(WEB_DIR, "icons"))) {
    copyDirRecursive(path.join(WEB_DIR, "icons"), path.join(OUT_DIR, "icons"));
  }

  // 5. Add Cloudflare _headers file for cache control and security headers
  const headersContent = `/*
  X-Frame-Options: DENY
  X-Content-Type-Options: nosniff
  Referrer-Policy: strict-origin-when-cross-origin
  Permissions-Policy: microphone=(), camera=()

/fonts/*
  Cache-Control: public, max-age=31536000, immutable

/icons/*
  Cache-Control: public, max-age=86400
`;
  fs.writeFileSync(path.join(OUT_DIR, "_headers"), headersContent);

  console.log("[✓] Static site built successfully in dist-static/");
  console.log(`    - index.html (Showcase Landing)`);
  console.log(`    - download.html (Windows / macOS / Linux downloads)`);
  console.log(`    - Styles, Fonts, Icons, and _headers (Zero web editor assets included)`);
}

buildStaticSite();
