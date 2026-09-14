![Version](https://img.shields.io/badge/version-0.1.0-E29BD3) ![License](https://img.shields.io/badge/license-GPL3-623994) ![OS](https://img.shields.io/badge/OS-Linux-000000) ![banner](img/banner.svg)
<div align="center">

# **miconium**
*произношение*: **[maɪˈkoʊ.ni.əm]**

## Генератор SVG-иконок в стиле Material Design
</div>

___
<div align="center">

## Содержание
</div>

[EN](README.md)

- [Демонстрация](#демонстрация)
- [Установка](#установка)
    - [Arch Linux](#arch-linux)
    - [Make](#через-make)
- [После установки](#настройка-после-установки)
    - [Демон](#настройка-демона)
    - [Matugen](#настройка-matugen)
___

<div align="center">

## Демонстрация

![Screenshot1](img/screenshots/1.png)
![Record1](img/screenshots/Record1.gif)
</div>

___
<div align="center">

## Установка
</div>

> [!WARNING]
> Дистрибутивы без systemd временно не поддерживаются

### Arch Linux
- *Клонировать репозиторий*
```bash
git clone https://github.com/Y-Akamirsky/miconium.git
cd miconium/install/arch-pkgbuild/
```
- *Установить*:
```bash
makepkg -fsi
```
> [!TIP]
> После установки склонированный репозиторий можно удалить: `rm -rf ~/miconium`

### Через Make
- *Клонировать репозиторий*:
```bash
git clone https://github.com/Y-Akamirsky/miconium.git
cd miconium/
```
- *Собрать проект* (Зависимости: cargo, make):
```bash
make build
```
- *Установить*
```bash
sudo make install
```
> [!TIP]
> Удаление: `sudo make uninstall`

___
<div align="center">

## Настройка после установки

### Настройка демона
</div>

> [!NOTE]
> Нужен, если вы хотите менять набор иконок «на лету».

> [!WARNING]
> Этот демон тестировался и адаптировался преимущественно для Niri+DankMaterialShell! Если вы столкнулись с проблемой в другом WM|DE и/или Shell|Dots — пожалуйста, создайте issue или внесите вклад через Pull Request!

**Тестовый запуск**
```bash
miconiumd run
```

**Служба systemd**
- *Включить*
```bash
systemctl --user enable --now miconiumd
```
- *Отключить*
```bash
systemctl --user disable --now miconiumd
```

**Очистка сгенерированных наборов иконок**
```bash
miconiumd cleanup-cache
```

<div align="center">

### Настройка Matugen
</div>

**Добавьте раздел '[templates.miconium]' в свою конфигурацию matugen**
> [!TIP]
> Шаблон устанавливается вместе с файлами программы по пути `/usr/share/miconium/matugen/template/miconium.json`

```toml
[config]

[templates.miconium]
input_path = '/usr/share/miconium/matugen/template/miconium.json'
output_path = '~/.local/share/miconium/matugen/matugen.json'
```
___

<div align="center">

## Credits
</div>

- Standard icon pack base — [Yet Another Monochrome Icon Set (YAMIS)](https://bitbucket.org/dirn-typo/yet-another-monochrome-icon-set/src/main/) ![License](https://img.shields.io/badge/license-GPL3-623994)