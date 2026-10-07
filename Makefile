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

ICONPACK_REPO ?= https://github.com/Y-Akamirsky/miconium-iconpack

.PHONY: build install install-root print-iconpack-notice install-config \
        user-enable uninstall cleanup clean

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
	# Reference config + default preset. The icon packs themselves live in a
	# separate repository (miconium-iconpack) and are NOT installed here —
	# see the notice printed at the end of a real installation.
	$(INSTALL) -dm755 $(DESTDIR)$(SHAREDIR)
	$(INSTALL) -Dm644 install/miconium.example.toml $(DESTDIR)$(SHAREDIR)/miconium.example.toml
	$(INSTALL) -Dm644 install/std-preset.toml       $(DESTDIR)$(SHAREDIR)/presets/std-preset.toml
	# matugen template
	$(INSTALL) -Dm644 matugen/template/miconium.json $(DESTDIR)$(SHAREDIR)/matugen/template/miconium.json
	@if [ -z "$(DESTDIR)" ]; then \
		update-desktop-database $(APPDIR) || true; \
		gtk-update-icon-cache -f $(PREFIX)/share/icons/hicolor || true; \
		$(MAKE) --no-print-directory print-iconpack-notice; \
	fi

# Build as the user, then install as root via sudo. One-shot convenience:
#   make install-root
# (The `install` target itself never rebuilds, so root does not need a
# configured Rust toolchain.)
install-root: build
	sudo make install DESTDIR="$(DESTDIR)" PREFIX="$(PREFIX)" BIN_GUI="$(BIN_GUI)" BIN_DAEMON="$(BIN_DAEMON)" SHAREDIR="$(SHAREDIR)"

print-iconpack-notice:
	@echo ""
	@echo "----------------------------------------"
	@echo " Miconium icon packs are installed and updated"
	@echo " SEPARATELY from the program itself."
	@echo ""
	@echo " Official \"yamis\" pack:"
	@echo "   Arch: git clone $(ICONPACK_REPO)"
	@echo "         cd miconium-iconpack/install/arch-pkgbuild && makepkg -fsi"
	@echo "   Make: git clone $(ICONPACK_REPO)"
	@echo "         cd miconium-iconpack && sudo make install"
	@echo ""
	@echo " Any pack in ~/.local/share/miconium/<name>/ is"
	@echo " picked up automatically as well."
	@echo ""
	@echo " You can build YOUR OWN pack: your own icons, frames"
	@echo " and accessories, in any style — see PACK_STRUCTURE.md"
	@echo " in the miconium-iconpack repository."
	@echo "----------------------------------------"
	@echo ""

# Install per-user configuration. Run as the regular user (not root).
# Icon packs are not copied here: the official ones live in /usr/share/miconium
# (installed from miconium-iconpack), and your own go to
# ~/.local/share/miconium/<pack-name>/.
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
	rm -f $(DESTDIR)$(SHAREDIR)/miconium.example.toml
	rm -f $(DESTDIR)$(SHAREDIR)/presets/std-preset.toml
	rm -f $(DESTDIR)$(SHAREDIR)/matugen/template/miconium.json
	# Installed packs (e.g. /usr/share/miconium/yamis) belong to the
	# miconium-iconpack package and are deliberately left alone.
	@if [ -z "$(DESTDIR)" ]; then \
		update-desktop-database $(APPDIR) || true; \
		gtk-update-icon-cache -f $(PREFIX)/share/icons/hicolor || true; \
	fi

clean:
	cargo clean

cleanup:
	rm -rf $(DESTDIR)$(SHAREDIR)
