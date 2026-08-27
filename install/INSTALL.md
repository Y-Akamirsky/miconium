<div align="center">

# Install guide
</div>

## Arch Linux

1. **Clone repo**
```sh
cd ~ && git clone https://github.com/Y-Akamirsky/miconium
cd miconium/install/arch-pkgbuild/
```

2. **Build and install**
```sh
makepkg -fsi
```

3. **Set-up configs then up the daemon** *(OPTIONAL)*
```sh
systemctl --user enable --now miconiumd
```

<div align="center">

### **Done**
</div>

## Nix
<div align="center">

<>==============<>
## *Coming Soon*
<>==============<>
</div>

## Other

1. **Clone repo**
```sh
cd ~ && git clone https://github.com/Y-Akamirsky/miconium
cd miconium
```

2. **Build and install**
```sh
make build
sudo make install
```

3. **Set-up configs then up the daemon** *(OPTIONAL)*
```sh
make user-enable
```
- Or
```sh
systemctl --user enable --now miconiumd
```