![Version](https://img.shields.io/badge/version-0.1.0-E29BD3) ![License](https://img.shields.io/badge/license-GPL3-623994) ![OS](https://img.shields.io/badge/OS-Linux-000000) ![banner](img/banner.svg)
<div align="center">

# **miconium**
*pronounce*: **[maɪˈkoʊ.ni.əm]**

## Material Design SVG icon Generator
</div>

___
<div align="center">

##  Contents
</div>

- [Showcase](#showcase)
- [Installation](#installation)
    - [Via script](#1-via-script)
    - [AppImage](#2-appimage)
    - [FromScratch](#3-from-scratch)
- [Documentation](#documentation)
___

<div align="center">

## Showcase

![Screenshot1](img/screenshots/1.png)
![Record1](img/screenshots/Record1.gif)
</div>

___
<div align="center">

## Installation
</div>

1. **Via script**
```bash
curl -sSL https://raw.githubusercontent.com/Y-Akamirsky/miconium/main/install/install.sh | bash
```
2. **AppImage** (Only GUI, without daemon)
	- *download latest **.AppImage** from [Releases](https://github.com/Y-Akamirsky/miconium/releases/latest)*
	- *Make executable*:  
	```bash
	chmod +x miconium.AppImage
	```
	
	- *Run as is, or install via [AppManager](https://github.com/kem-a/AppManager)*
3. **From scratch**
   - *Clone repository*:
   ```bash
   git clone https://github.com/Y-Akamirsky/miconium.git
   cd miconium/
   ```
   - *Build project* (Depends Rust and Cargo):
   ```bash
   cargo build --release
   ```
   - *Copy binaries and other stuff to system*:
   ```bash
   # binaries
   sudo cp target/release/miconium-gui /usr/bin/
   sudo cp target/release/miconiumd /usr/bin/
   # appearance stuff
   sudo cp install/miconium.desktop /usr/share/applications/
   sudo cp install/miconium.svg /usr/share/icons/hicolor/scalable/
   # configuration stuff
   mkdir -p ~/.config/miconium/presets
   mkdir -p ~/.local/share/miconium/
   cp install/miconium.example.toml ~/.config/miconium/config.toml
   cp install/std-preset.toml ~/.config/miconium/presets
   cp -r packs/yamis ~/.local/share/miconium/
   # daemon (systemd);(optional)
   sudo cp install/daemon-service/systemd/miconiumd.service /usr/lib/systemd/user/
   ```
   - *Update desktop database and icon cache*
   ```bash
   sudo update-desktop-database /usr/share/applications
   sudo gtk-update-icon-cache -f /usr/share/icons/hicolor
   ```
   - *Enable a daemon* (Optional):
   ```bash
   systemctl --user enable --now miconiumd.service
   ```