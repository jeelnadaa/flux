# ==============================================================================
# Flux Native Audio Editor - Unified Cross-Platform Build System
# ==============================================================================

PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
DATADIR ?= $(PREFIX)/share
APPDIR ?= $(DATADIR)/applications
ICONDIR ?= $(DATADIR)/icons/hicolor/scalable/apps

CARGO ?= cargo
RUSTC ?= rustc
VERSION ?= 1.0.0

.PHONY: all build build-web build-desktop build-static check test package-linux package-windows package-macos install uninstall clean help

all: build

help:
	@echo "Flux Unified Build System (Windows, macOS, Linux, Web)"
	@echo "====================================================="
	@echo "  make              - Build native executable with full DSP engine"
	@echo "  make build-web    - Build TypeScript UI packages and web app bundle"
	@echo "  make build-static - Build production Cloudflare Pages site in dist-static/"
	@echo "  make build-desktop- Build native Rust standalone binary"
	@echo "  make check        - Verify workspace code and clippy linting"
	@echo "  make test         - Run full DSP invariant and engine test suite"
	@echo "  make package-linux- Create Linux portable tar.gz release bundle"
	@echo "  make install      - Install flux binary and desktop entries on Linux"
	@echo "  make uninstall    - Remove installed files"
	@echo "  make clean        - Remove build artifacts and temporary binaries"

build-web:
	@echo "[*] Building Flux UI and Web bundle..."
	@if command -v npm >/dev/null 2>&1; then \
		npm run build; \
	elif command -v npm.cmd >/dev/null 2>&1; then \
		npm.cmd run build; \
	fi

build-static: build-web
	@echo "[*] Assembling production static website in dist-static/..."
	@if command -v npm >/dev/null 2>&1; then \
		npm run build:static; \
	elif command -v npm.cmd >/dev/null 2>&1; then \
		npm.cmd run build:static; \
	fi

build-desktop:
	@echo "[*] Compiling native Rust DSP and Engine crates..."
	$(CARGO) build --release --workspace --bins

build: build-web build-desktop
	@mkdir -p bin
	@if [ -f target/release/flux ]; then \
		cp target/release/flux bin/flux; \
	elif [ -f target/release/flux.exe ]; then \
		cp target/release/flux.exe bin/flux.exe; \
	fi
	@echo "[✓] Native executable ready at bin/"

check:
	@echo "[*] Checking workspace integrity..."
	$(CARGO) check --workspace --all-targets

test:
	@echo "[*] Running native DSP invariant tests..."
	$(CARGO) test --workspace
	@echo "[*] Running TypeScript domain and unit tests..."
	@if command -v npm >/dev/null 2>&1; then \
		npm run test:unit; \
	elif command -v npm.cmd >/dev/null 2>&1; then \
		npm.cmd run test:unit; \
	fi

package-linux: build
	@echo "[*] Packaging Linux standalone bundle..."
	@mkdir -p dist/Flux-$(VERSION)-linux-x64/apps/web
	@cp bin/flux dist/Flux-$(VERSION)-linux-x64/
	@cp -r apps/web/dist dist/Flux-$(VERSION)-linux-x64/apps/web/
	@cp -r apps/web/fonts dist/Flux-$(VERSION)-linux-x64/apps/web/
	@cp -r apps/web/icons dist/Flux-$(VERSION)-linux-x64/apps/web/
	@cp apps/web/index.html dist/Flux-$(VERSION)-linux-x64/apps/web/
	@cp apps/web/app.css dist/Flux-$(VERSION)-linux-x64/apps/web/
	@cp apps/web/themes.css dist/Flux-$(VERSION)-linux-x64/apps/web/
	@cp apps/web/fonts.css dist/Flux-$(VERSION)-linux-x64/apps/web/
	@cp apps/web/manifest.webmanifest dist/Flux-$(VERSION)-linux-x64/apps/web/
	@cp README.md dist/Flux-$(VERSION)-linux-x64/
	@cp LICENSES.md dist/Flux-$(VERSION)-linux-x64/
	@tar -czf dist/Flux-$(VERSION)-linux-x64.tar.gz -C dist Flux-$(VERSION)-linux-x64
	@echo "[✓] Linux package ready: dist/Flux-$(VERSION)-linux-x64.tar.gz"

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
	@printf "[Desktop Entry]\nType=Application\nName=Flux\nGenericName=Audio Editor\nComment=Professional native non-destructive audio editor\nExec=flux %%F\nIcon=flux\nTerminal=false\nCategories=AudioVideo;Audio;AudioVideoEditing;\nMimeType=audio/wav;audio/x-wav;audio/flac;audio/mpeg;audio/ogg;\n" > $(DESTDIR)$(APPDIR)/flux.desktop
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
	rm -rf bin dist dist-static
