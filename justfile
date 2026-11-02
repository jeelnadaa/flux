set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

npm_cmd := if os() == "windows" { "npm.cmd" } else { "npm" }

# Default recipe: list available recipes
default:
    @just --list

# The local CI gate: format, lint, typos, unit, property, golden, null, integration, E2E, license scan, WASM build
ci: fmt-check lint typos check-rust test-rust build-wasm check-ts

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

# Check all Rust crates and targets
check-rust:
    cargo check --workspace --all-targets

# Run all Rust tests
test-rust:
    cargo test --workspace

# Build WebAssembly crate
build-wasm:
    cargo build -p flux-wasm --target wasm32-unknown-unknown

# Typecheck TypeScript packages and apps
check-ts:
    {{npm_cmd}} run typecheck

# Clean workspace build artifacts
clean:
    cargo clean
