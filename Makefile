# Miconium - Makefile
# Installation targets for system-wide (root) files.
# User-level config is handled by `make install-config` (runs as the user).

DESTDIR   ?=
PREFIX    ?= /usr
BINDIR    ?= $(PREFIX)/bin
APPDIR    ?= $(PREFIX)/share/applications
ICONDIR   ?= $(PREFIX)/share/icons/hicolor/scalable/apps
SYSTEMD   ?= $(PREFIX)/lib/systemd/user

INSTALL   ?= install
CP        ?= cp -r

BIN_GUI      = miconium-gui
BIN_DAEMON   = miconiumd
SHAREDIR    ?= $(PREFIX)/share/miconium

.PHONY: build install install-root user-install uninstall clean

build:
	cargo build --release

# Install already-built binaries (run as root, e.g. `sudo make install`).
# Does NOT rebuild — building must happen as the regular user so the toolchain
# (rustup) and target cache are owned correctly.
install:
	$(INSTALL) -Dm755 target/release/$(BIN_GUI)      $(DESTDIR)$(BINDIR)/$(BIN_GUI)
	$(INSTALL) -Dm755 target/release/$(BIN_DAEMON)   $(DESTDIR)$(BINDIR)/$(BIN_DAEMON)
	$(INSTALL) -Dm644 install/miconium.desktop        $(DESTDIR)$(APPDIR)/miconium.desktop
	$(INSTALL) -Dm644 install/miconium.svg            $(DESTDIR)$(ICONDIR)/miconium.svg
	$(INSTALL) -Dm644 install/daemon-service/systemd/miconiumd.service $(DESTDIR)$(SYSTEMD)/miconiumd.service
	# bundled ("yamis") pack — system-wide default; third-party packs go to
	# the user data dir (~/.local/share/miconium).
	$(INSTALL) -dm755 $(DESTDIR)$(SHAREDIR)
	$(CP) packs/yamis $(DESTDIR)$(SHAREDIR)/
	# reference config + default preset shipped for user convenience
	$(INSTALL) -Dm644 install/miconium.example.toml $(DESTDIR)$(SHAREDIR)/miconium.example.toml
	$(INSTALL) -Dm644 install/std-preset.toml       $(DESTDIR)$(SHAREDIR)/presets/std-preset.toml
	# matugen template
	$(INSTALL) -Dm644 matugen/template/miconium.json $(DESTDIR)$(SHAREDIR)/matugen/template/miconium.json
	@if [ -z "$(DESTDIR)" ]; then \
		update-desktop-database $(APPDIR) || true; \
		gtk-update-icon-cache -f $(PREFIX)/share/icons/hicolor || true; \
	fi

# Build as the user, then install as root via sudo. One-shot convenience:
#   make install-root
# (The `install` target itself never rebuilds, so root does not need a
# configured Rust toolchain.)
install-root: build
	sudo make install DESTDIR="$(DESTDIR)" PREFIX="$(PREFIX)" BIN_GUI="$(BIN_GUI)" BIN_DAEMON="$(BIN_DAEMON)" SHAREDIR="$(SHAREDIR)"

# Install per-user configuration and presets (the bundled pack is already in the
# system share dir). Run as the regular user (not root). Third-party packs the
# user wants to add go to ~/.local/share/miconium/.
install-config:
	mkdir -p ~/.config/miconium/presets
	$(CP) install/miconium.example.toml ~/.config/miconium/miconium.toml

# Enable the optional user daemon.
user-enable:
	systemctl --user enable --now miconiumd.service

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/$(BIN_GUI)
	rm -f $(DESTDIR)$(BINDIR)/$(BIN_DAEMON)
	rm -f $(DESTDIR)$(APPDIR)/miconium.desktop
	rm -f $(DESTDIR)$(ICONDIR)/miconium.svg
	rm -f $(DESTDIR)$(SYSTEMD)/miconiumd.service
	rm -rf $(DESTDIR)$(SHAREDIR)/yamis/
	@if [ -z "$(DESTDIR)" ]; then \
		update-desktop-database $(APPDIR) || true; \
		gtk-update-icon-cache -f $(PREFIX)/share/icons/hicolor || true; \
	fi

clean:
	cargo clean

cleanup:
	rm -rf $(DESTDIR)$(SHAREDIR)
