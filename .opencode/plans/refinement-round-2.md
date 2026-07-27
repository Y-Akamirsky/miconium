# Refinement Round 2 — 5 Issues

## Issue 1 — Export path + overwrite dialog

### Проблема
- `resolve_output_path` получает `HOME = ""` (или `/`), из-за чего путь экспорта — `/.local/share/icons/Miconium` вместо `/home/$USER/.local/share/icons/Miconium`. IO error, fallback на `/tmp/miconium-export`.
- Нет проверки на существующий вывод — при повторном экспорте остаются файлы от предыдущего сета.

### Решение
1. **`resolve_output_path`**: заменить `std::env::var("HOME").unwrap_or_default()` на `home_dir()` из крейта `dirs` (уже в зависимостях? если нет — добавить).
   ```rust
   fn resolve_output_path(config: &ExportConfig) -> PathBuf {
       let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
       // ... use home
   }
   ```
2. **`export_pack` / `start_export`**: перед началом экспорта:
   - Проверить, существует ли `output_path`.
   - Если существует и не пуст — показать диалог: **"Папка назначения уже содержит иконки. Все они будут удалены перед экспортом. Продолжить?"**
   - Если пользователь согласен — рекурсивно удалить `output_path`.
   - Если нет — отменить экспорт.

### Файлы
- `export/mod.rs` — `resolve_output_path`, `export_pack`
- `gui/main.rs` — `start_export`: добавить диалог подтверждения

---

## Issue 2 — Битая иконки из `status/` после экспорта

### Проблема
Все или большинство иконок из `yamis/signs/status/` после экспорта битые.

### План расследования
1. **Экспортировать одну иконку** из status: вручную вызвать `assemble_icon` с одним из status-слоёв. Сравнить вывод с оригинальным SVG.
2. **Проверить `collect_namespaces`** — status SVGs могут содержать нестандартные namespace.
3. **Проверить `wrap_svg_body_with_fill`** — обёртка в `<g fill="">` может сломать SVG с `viewBox` или `preserveAspectRatio` на корневом `<svg>`.
4. **Проверить `merge_layers`** — статусные иконки могут быть многослойными или содержать `clip-path`, `mask`, `filter`.
5. **Написать тест**: экспорт одной иконки из yamis/status с проверкой через `resvg::Tree::from_str`.

### Файлы
- `svg_engine/mod.rs` — анализ `assemble_icon`, `merge_layers`, `wrap_svg_body_with_fill`, `collect_namespaces`
- `export/mod.rs` — `write_icon`
- `svg_engine/tests.rs` — новый тест

---

## Issue 3 — Collapsible категории

### Проблема
Много категорий → панель Categories слишком громоздкая.

### Решение
1. **`build_categories_panel`**: каждую категорию обернуть в `gtk::Expander`:
   - Expander label = название категории (жирный шрифт)
   - Expander child = текущая строка с контролами (variant combo + frame/acc + scales)
   - По умолчанию все Expander'ы **свёрнуты** (кроме первой категории, или все свёрнуты)
   - Expander'ы добавляются в `gtk::ListBox` (как сейчас)

2. **Визуально**: категория занимает 1 строку в свёрнутом виде, раскрывается на весь набор контролов при клике.

### Файлы
- `gui/main.rs` — `build_categories_panel`

---

## Issue 4 — Превью показывает редактируемую категорию

### Проблема
При настройке категории `preferences` превью всё ещё показывает прошлую иконку (например `apps/firefox`). Нужно что-бы при любом изменении в категории превью переключалось на иконку из этой категории.

### Решение
1. **Изменить `rebuild_preview`**: необязательный параметр `preferred_category: Option<&str>`.
   - Если `preferred_category` задан — найти первую иконку в этой категории (любой вариант) и показать её.
   - Если не задан — текущее поведение (перерисовать текущую иконку).

2. **Все сигналы в `build_categories_panel`**: при вызове `rebuild_preview` передавать `Some(&cat)`.
   - Исключение: вариант комбо — уже переключает иконку через `selected_variant`, достаточно `Some(&cat)`.

3. **Сигналы `rebuild_preview` и `show_first_preview` вне категорий** (например, из scale sliders, color picker) — оставить `None`.

### Файлы
- `gui/main.rs` — `rebuild_preview`, `build_categories_panel` сигналы, `preview_icon`

---

## Issue 5 — Per-category / per-variant scale (frame, icon, acc)

### Проблема
Сейчас `frame_scale`, `icon_scale`, `acc_scale` глобальные (один слайдер на все категории). Нужно иметь возможность настроить их отдельно для каждой (категория, вариант).

### Решение
1. **`config/mod.rs`**: в `CategoryOverride` добавить три поля:
   ```rust
   pub frame_scale: f64,  // default 1.0
   pub icon_scale: f64,   // default 1.0
   pub acc_scale: f64,    // default 1.0
   ```
   В `default_category_override`: `frame_scale: 1.0, icon_scale: 1.0, acc_scale: 1.0`.

2. **`gui/main.rs` — `build_categories_panel`**: в каждый вариант (внутри Expander) добавить 3 слайдера:
   ```
   [Fr: ===1.00===] [Ic: ===1.00===] [Ac: ===1.00===]
   ```
   Слайдеры от 0.1 до 3.0, шаг 0.05, 2 знака после запятой.
   При изменении: `ov.frame_scale = val`, и т.д.

3. **`gui/main.rs` — `preview_icon`**: вместо `d.frame_scale`, `d.icon_scale`, `d.acc_scale` использовать `ov.frame_scale`, `ov.icon_scale`, `ov.acc_scale`.

4. **`export/mod.rs` — `run_export`**: в цикле по вариантам, вместо `export_cfg.frame_scale` и т.д. использовать `ov.frame_scale`, `ov.icon_scale`, `ov.acc_scale`.

5. **Глобальные слайдеры** (в `build_scale_section`): оставить как есть, они будут baseline-значениями. Per-variant оverrides перекрывают их.

### Файлы
- `config/mod.rs` — CategoryOverride поля + default
- `gui/main.rs` — `build_categories_panel` слайдеры, `preview_icon`
- `export/mod.rs` — `run_export`
- `export/tests.rs` — обновить `overrides_with_frame`

---

## Summary таблица

| # | Задача | Файлы | Статус |
|---|--------|-------|--------|
| 1 | Export path fix + overwrite dialog | export/mod.rs, gui/main.rs | ⏳ |
| 2 | Битые status иконки (расследование) | svg_engine/mod.rs, tests | ⏳ |
| 3 | Collapsible категории (gtk::Expander) | gui/main.rs | ⏳ |
| 4 | Превью → редактируемая категория | gui/main.rs | ⏳ |
| 5 | Per-variant scale | config/mod.rs, gui/main.rs, export/mod.rs | ⏳ |
