# ==============================================================================
# Flux Native Audio Editor - Linux Build System
# ==============================================================================

PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
DATADIR ?= $(PREFIX)/share
APPDIR ?= $(DATADIR)/applications
ICONDIR ?= $(DATADIR)/icons/hicolor/scalable/apps

CARGO ?= cargo
RUSTC ?= rustc

.PHONY: all build check test install uninstall clean help

all: build

help:
	@echo "Flux Build System for Linux"
	@echo "==========================="
	@echo "  make              - Build native release executable"
	@echo "  make check        - Verify workspace code and linting"
	@echo "  make test         - Run entire DSP invariant and engine test suite"
	@echo "  make install      - Install flux binary and desktop entries (may require sudo)"
	@echo "  make uninstall    - Remove installed files"
	@echo "  make clean        - Remove build artifacts and temporary binaries"

build:
	@echo "[*] Preparing Flux UI bundle..."
	@if command -v npm >/dev/null 2>&1; then \
		npm run build || true; \
	fi
	@echo "[*] Compiling native Rust DSP and Engine crates..."
	$(CARGO) build --release --workspace --bins
	@mkdir -p bin
	@if [ -f target/release/flux ]; then \
		cp target/release/flux bin/flux; \
	elif [ -f target/release/flux-engine ]; then \
		cp target/release/flux-engine bin/flux; \
	elif [ -f target/release/generate-corpus ]; then \
		cp target/release/generate-corpus bin/flux; \
	fi
	@echo "[✓] Native executable built at bin/flux"

check:
	@echo "[*] Checking workspace integrity..."
	$(CARGO) check --workspace --all-targets
	$(CARGO) clippy --workspace --all-targets -- -D warnings

test:
	@echo "[*] Running native test suite..."
	$(CARGO) test --workspace

install: build
	@echo "[*] Installing Flux to $(BINDIR)..."
	install -d $(DESTDIR)$(BINDIR)
	install -m 755 bin/flux $(DESTDIR)$(BINDIR)/flux
	install -d $(DESTDIR)$(DATADIR)/flux/apps/web
	cp -r apps/web/dist $(DESTDIR)$(DATADIR)/flux/apps/web/
	cp -r apps/web/fonts $(DESTDIR)$(DATADIR)/flux/apps/web/
	cp -r apps/web/icons $(DESTDIR)$(DATADIR)/flux/apps/web/
	cp apps/web/index.html $(DESTDIR)$(DATADIR)/flux/apps/web/
	cp apps/web/app.css $(DESTDIR)$(DATADIR)/flux/apps/web/
	cp apps/web/themes.css $(DESTDIR)$(DATADIR)/flux/apps/web/
	cp apps/web/fonts.css $(DESTDIR)$(DATADIR)/flux/apps/web/
	cp apps/web/manifest.webmanifest $(DESTDIR)$(DATADIR)/flux/apps/web/
	install -d $(DESTDIR)$(APPDIR)
	@echo "[Desktop Entry]\nType=Application\nName=Flux\nGenericName=Audio Editor\nComment=Professional native non-destructive audio editor\nExec=flux %F\nIcon=flux\nTerminal=false\nCategories=AudioVideo;Audio;AudioVideoEditing;\nMimeType=audio/wav;audio/x-wav;audio/flac;audio/mpeg;audio/ogg;" > $(DESTDIR)$(APPDIR)/flux.desktop
	install -d $(DESTDIR)$(ICONDIR)
	@if [ -f apps/web/icons/icon-512.svg ]; then \
		install -m 644 apps/web/icons/icon-512.svg $(DESTDIR)$(ICONDIR)/flux.svg; \
	fi
	@echo "[✓] Installation complete. Run 'flux' to launch."

uninstall:
	@echo "[*] Removing Flux installation..."
	rm -f $(DESTDIR)$(BINDIR)/flux
	rm -rf $(DESTDIR)$(DATADIR)/flux
	rm -f $(DESTDIR)$(APPDIR)/flux.desktop
	rm -f $(DESTDIR)$(ICONDIR)/flux.svg
	@echo "[✓] Flux removed."

clean:
	@echo "[*] Cleaning build artifacts..."
	$(CARGO) clean
	rm -rf bin
