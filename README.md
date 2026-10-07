![Version](https://img.shields.io/badge/version-0.1.0-E29BD3) ![License](https://img.shields.io/badge/license-GPL3-623994) ![OS](https://img.shields.io/badge/OS-Linux-000000) ![banner](img/banner.svg)
<div align="center">

# **miconium**
*pronounce*: **[maɪˈkoʊ.ni.əm]**

## Material Design SVG Icon Generator
</div>

___
<div align="center">

##  Contents
</div>

[RU](README_RU.md)

- [Showcase](#showcase)
- [Installation](#installation)
    - [Arch Linux](#arch-linux)
    - [Make](#via-make)
- [Icon packs](#icon-packs)
- [Post-install](#post-install-setup)
    - [Daemon](#daemon-setup)
    - [Matugen](#matugen-setup)
- [Credits](#credits)
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

> [!WARNING] 
> Non-systemd distros are temporarily unsupported

### Arch Linux
  - *Clone repository*
  ```bash
  git clone https://github.com/Y-Akamirsky/miconium.git
  cd miconium/install/arch-pkgbuild/
  ```
  - *Install*:
  ```bash
  makepkg -fsi
  ```
  > [!TIP]
  > You can remove cloned repo after install: `rm -rf ~/miconium`
  
### Via Make
  - *Clone repository*:
  ```bash
  git clone https://github.com/Y-Akamirsky/miconium.git
  cd miconium/
  ```
  - *Build project* (Depends on: cargo, make):
  ```bash
  make build
  ```
  - *Install*
  ```bash
  sudo make install
  ```
  > [!TIP]
  > Uninstall: `sudo make uninstall`

___
<div align="center">

## Icon packs

</div>

Miconium needs an **icon pack** — the SVG layers it assembles into a theme.
Packs live in their own repository,
**[`Y-Akamirsky/miconium-iconpack`](https://github.com/Y-Akamirsky/miconium-iconpack)**,
so artwork is versioned and updated **independently from this program**: updating
icons never touches your Miconium install, and updating Miconium never touches
your icons.

> [!IMPORTANT]
> Installing Miconium does **not** install any icons — you pick a pack
> separately. Without one, Miconium has nothing to assemble.

*Install the official `yamis` pack:*
```bash
# Arch
git clone https://github.com/Y-Akamirsky/miconium-iconpack
cd miconium-iconpack/install/arch-pkgbuild && makepkg -fsi

# any distro
git clone https://github.com/Y-Akamirsky/miconium-iconpack
cd miconium-iconpack && sudo make install
```
Then choose it in the GUI, or in your config:
```toml
[pack]
name = "yamis"
```

Packs you install yourself (or download from elsewhere) are picked up
automatically from `~/.local/share/miconium/<pack-name>/`.

**Want your own pack?** A pack is just a directory of SVGs — build your own
icons, frames and decorations in any style you like, and validate it with the
bundled script (bash only):
```bash
git clone https://github.com/Y-Akamirsky/miconium-iconpack
./validate.sh my-pack
```
See [`PACK_STRUCTURE.md`](https://github.com/Y-Akamirsky/miconium-iconpack/blob/main/PACK_STRUCTURE.md)
for the layout. Ricing is meant to be creative: mix and match freely.

___
<div align="center">

## Post-install setup

### Daemon setup
</div>

> [!NOTE]
> Needed if you want to change your iconset "on the fly".

> [!WARNING]
> This daemon was tested and adapted mostly for Niri+DankMaterialShell! If you face a problem with another WM|DE or/and Shell|Dots - please write an issue or contribute with a Pull Request!

**Dry run**
```bash
miconiumd run
```

**Systemd service**
  - *Enable*
    ```bash
    systemctl --user enable --now miconiumd
    ```
  - *Disable*
    ```bash
    systemctl --user disable --now miconiumd
    ```

**Cleanup generated iconsets**
```bash
miconiumd cleanup-cache
```

<div align="center">

### Matugen setup
</div>

**Add the '[templates.miconium]' part to your matugen config**
> [!TIP]
> The template is installed along with the binaries at `/usr/share/miconium/matugen/template/miconium.json`

```toml
[config]

[templates.miconium]
input_path = '/usr/share/miconium/matugen/template/miconium.json'
output_path = '~/.local/share/miconium/matugen/matugen.json'
```

> [!NOTE]
> If you're using Dank Material Shell (DMS), you can use its file instead of the user config: `~/.cache/DankMaterialShell/dms-colors.json`

> [!WARNING]
> This won't work if you switch your shell. The primary option is more reliable.
___

<div align="center">

## Credits
</div>

- Standard icon pack base — [Yet Another Monochrome Icon Set (YAMIS)](https://bitbucket.org/dirn-typo/yet-another-monochrome-icon-set/src/main/) ![License](https://img.shields.io/badge/license-GPL3-623994)
- Icon packs (packaged separately) — [miconium-iconpack](https://github.com/Y-Akamirsky/miconium-iconpack)