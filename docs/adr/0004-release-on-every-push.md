# ADR 0004: A release on every push to main, with build caches

Date: 2026-11-08. Status: accepted (user request).

**Context.** The user asked for every push to build and publish the apps to GitHub releases, using caches so only
changed code is rebuilt; the download page and the in-app updater read the latest release.

**Decision.** `release.yml` drafts `v2.0.<run number>`, builds four targets with tauri-action (Windows x64;
macOS arm64 and x64 on Apple silicon runners; Linux x64 on Ubuntu 22.04 for older glibc), uploads `flux-mcp` per
target, and publishes the release only when every build passed; a failed run deletes its draft. Pushes that only
change docs, the site, tests or the design skip the release. Caches: Swatinem/rust-cache per target, sccache for
compiled crates, npm.

**Signing.** Updater files are signed when the `TAURI_SIGNING_PRIVATE_KEY` secret is set, and skipped (with a
warning) when it isn't. Installers are not code-signed for SmartScreen or Gatekeeper; that needs paid
certificates and the user's decision.
