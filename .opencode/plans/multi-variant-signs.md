# Issue 28 — Multi-variant знаки (scalable / 16 / scalable-outlined)

## Проблема
Сейчас `signs.categories = HashMap<String, Vec<LayerData>>` — одна плоская
пачка иконок на категорию. Пак yamis хранит иконки в поддиректориях
(`signs/<cat>/scalable/`, `signs/<cat>/16/` и т.д.). Программа:
1. Не видит варианты (читает только `scalable/` через fallback)
2. Не разделяет их в GUI
3. Экспортирует всё только в `scalable/`
4. Использует симлинки вместо реальной структуры

## Решение
Перевести `signs` на `HashMap<String, HashMap<String, Vec<LayerData>>>`
(категория → вариант → список иконок). Вариант = имя поддиректории
(`"scalable"`, `"16"`, `"scalable-outlined"`). Для плоских паков вариант = `"scalable"`.

Перевести `category_overrides` на `HashMap<String, HashMap<String, CategoryOverride>>`
(категория → вариант → настройки). У каждого варианта свои подложка и аксессуары.

Удалить симлинки. Экспорт пишет в `<cat>/<variant>/`.

## План по фазам

### Phase 0 — Подготовка
1. Удалить `generate_16_symlinks` из `ExportConfig` и всех usage.

### Phase 1 — `pack/mod.rs` (Signs → multi-variant)

1. **Тип `Signs`** (строки 37-38):
   ```rust
   pub struct Signs {
       pub categories: HashMap<String, HashMap<String, Vec<LayerData>>>,
   }
   ```

2. **`Pack::load`** (строки 86-93):
   - Для каждой директории в `signs/`:
     - Прочитать поддиректории (через `std::fs::read_dir`)
     - Для каждой поддиректории: `read_svg_dir(subdir)` → variant = имя поддира
     - Если поддиректорий нет: `read_svg_dir(dir)` → variant = `"scalable"`
     - Пропускать пустые варианты
     - Вставлять категорию, только если есть ≥1 непустой вариант

3. **Методы `Signs`**:
   - `categories()` — без изменений (ключи внешнего HashMap)
   - `get_signs_by_category(cat) → Vec<LayerData>` — все иконки из всех вариантов
   - `get_category_variants(cat) → Vec<String>` — отсортированные имена вариантов
   - `get_sign_variant(cat, variant) → Vec<LayerData>` — иконки конкретного варианта
   - `signs_by_category_ref` — **удалить**
   - `all_signs()` — склеивает все варианты в плоский `HashMap<String, Vec<LayerData>>`
   - `sign_count()` — сумма всех Vec всех вариантов

4. **`find_sign_category`** (main.rs) — искать по всем вариантам.

### Phase 2 — `config/mod.rs` (overrides → per-variant)

1. **`PackConfig.category_overrides`**:
   ```rust
   pub category_overrides: HashMap<String, HashMap<String, CategoryOverride>>,
   ```

2. **`selected_variants`**:
   ```rust
   pub selected_variants: HashMap<String, String>,
   ```

3. **Методы**:
   - `variant_override(cat, variant) → CategoryOverride`
   - `selected_variant(cat) → String` (default: "scalable")
   - `set_selected_variant(cat, variant)`

4. **`ExportConfig`**: удалить `generate_16_symlinks`.

### Phase 3 — `export/mod.rs` (экспорт всех вариантов)

1. **Удалить** `symlink_scalable` (unix и non-unix).

2. **`create_output_tree(output, pack)`**:
   - Для каждой категории: для каждого её варианта → `create_dir_all(<cat>/<variant>/)`

3. **`write_icon(output, category, variant, icon, filename)`**:
   - Путь: `output/<category>/<variant>/<filename>.svg`

4. **`run_export`**:
   - Цикл по `pack.categories()`
   - Для каждой категории: цикл по `pack.get_category_variants(cat)`
   - Для каждого варианта: `ov = config.variant_override(cat, variant)`
   - `resolve_frame(ov, pack)` + `accessories` фильтр
   - Для каждой иконки варианта: `assemble_icon` + `write_icon`

### Phase 4 — `main.rs` (GUI)

1. **`build_categories_panel`**: В каждую строку категории добавить:
   - **Variant ComboBox** (после category label, перед frame controls)
   - При переключении: `set_selected_variant(cat, variant)`
   - Frame/accessory контролы читают/пишут `variant_override(cat, variant)`
   - restore: `variant_combo.set_active(selected_variant(cat))`

2. **`category_override_defaults`**: параметр `(cat, variant)`.

3. **Сигналы**: все `entry(cat).or_default().entry(variant)`.

4. **`preview_icon`**: `selected_variant(cat)` + `variant_override(cat, variant)`.

5. **`find_sign_category`**: ищет по всем вариантам.

6. **`rebuild_preview`** (строки 535-544): заменить хардкод на `pack.categories()`.

7. **`show_first_preview`** (строки 819-828): заменить хардкод на `pack.categories()`.

### Phase 5 — Тесты

1. `pack/tests.rs`: обновить под новый тип Signs.
2. `export/tests.rs`: убрать symlink тест, обновить ExportConfig.
3. `config/tests.rs`: убрать `generate_16_symlinks`.
4. `pack_integration.rs`: yamis variant test.

## Файлы для изменения

| Файл | Что меняется |
|------|-------------|
| `pack/mod.rs` | Signs тип, Pack::load, методы |
| `pack/tests.rs` | Тесты под новый тип |
| `config/mod.rs` | category_overrides → nested, +selected_variants, -generate_16_symlinks |
| `config/tests.rs` | Убрать generate_16_symlinks тесты |
| `export/mod.rs` | create_output_tree, write_icon, run_export, -symlink_scalable |
| `export/tests.rs` | Тесты без симлинков, с variant overrides |
| `gui/main.rs` | +variant combo, +nested overrides, +selected_variant, хардкод → categories() |
| `pack_integration.rs` | yamis variant test |
