# ADR 0001: A desktop app with Tauri, the prototype's UI, the engine in-process

Date: 2026-11-07. Status: accepted (user request).

**Context.** The user asked for Flux to be rebuilt as a desktop app for Windows, macOS (Apple silicon and Intel)
and Linux, with the UI a 1:1 copy of `design/`, and for the web version to go.

**Decision.** Tauri 2. The prototype's HTML, CSS and JS are ported as plain JS into `apps/desktop/ui` (no UI
library, no bundler). The Rust engine runs in the app process and is called through one command registry, the
same one `flux-mcp` and the tests use. `flux-mcp` ships beside the app as a sidecar.

**Why.** Tauri uses the system web view, so installers are small (NSIS 8.2 MB) and the prototype's CSS renders as
designed. Electron would add about 100 MB and a second runtime. A native toolkit would mean redrawing the design.

**Consequences.** WebKitGTK on Linux and WKWebView on macOS must render the same CSS; the E2E runs in Chromium,
so macOS and Linux rendering is in the human queue.
