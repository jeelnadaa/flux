set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

npm_cmd := if os() == "windows" { "npm.cmd" } else { "npm" }

# Default recipe: list available recipes
default:
    @just --list

# The local CI gate: format, lint, typos, licenses, unit, property, golden, null, integration, E2E, WASM build, Python eval, browser smoke test
ci: fmt-check lint typos license-scan check-rust test-rust test-python test-browser build-wasm check-ts

# Format check across Rust and TypeScript
fmt-check:
    cargo fmt --all -- --check
    {{npm_cmd}} run format:check

# Format code across Rust and TypeScript
fmt:
    cargo fmt --all
    {{npm_cmd}} run format:write

# Lint across Rust and TypeScript
lint:
    cargo clippy --workspace --all-targets -- -D warnings
    {{npm_cmd}} run lint

# Check spelling and typos across the repo
typos:
    typos

# Run automated license scan across Rust and JavaScript dependencies
license-scan:
    cargo deny check licenses
    {{npm_cmd}} run license:check

# Check all Rust crates and targets
check-rust:
    cargo check --workspace --all-targets

# Run all Rust tests
test-rust:
    cargo test --workspace

# Run Python evaluation harness unit tests
test-python:
    python -m pytest tools/eval/tests

# Run headless browser smoke tests (Spikes, Recording, Engine, Export, Analysis, Pixel-Diff)
test-browser:
    python -m pytest tests/spikes/spike_a/test_spike_a.py tests/spikes/spike_b/test_spike_b.py tests/spikes/spike_c/test_spike_c.py tests/spikes/spike_d/test_spike_d.py tests/spikes/spike_f/test_spike_f.py tests/spikes/spike_g/test_spike_g.py tests/recording/test_recording.py tests/engine/test_engine.py tests/export/test_export.py tests/analysis/test_analysis.py tests/pixel_diff/test_pixel_diff.py

# Run full Python evaluation benchmark suite and update dashboards
eval:
    python tools/eval/run_eval.py

# Build WebAssembly crate
build-wasm:
    cargo build -p flux-wasm --target wasm32-unknown-unknown

# Typecheck TypeScript packages and apps
check-ts:
    {{npm_cmd}} run typecheck

# Clean workspace build artifacts
clean:
    cargo clean
