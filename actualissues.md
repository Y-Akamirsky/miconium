# Актуальные проблемы программы

**Решённые проблемы указаны в [Чеклисте](#Чеклист) галочкой в чекбоксах**

## 1. Режимы подбора цветов
Режимы подбора цветов из выбранной в DE|WM цветовой схемы .colors (xdg), из matugen палитры. Работает только ручной ввод цвета. На счет matugen не уверен, возможно у меня в dms (Dank Material Shell) палитры матугена лежат не там где нужно Miconium.
### Как решить:  
 - Добавить как минимум рабочие режимы с .colors цвет. схемой и matugen палитрой.
 - В режиме matugen дать возможность выбирать какой цвет к какому слою иконки применять. (Например: некоторым захочется применить surface цвета на знак, а на подложку primary. Но не обязательно именно так, нужна полная свобода)
## 2. Система аксессуаров
Нет возможности выбрать конкретные комбинации аксессуаров, что очень ограничивает возможности программы.
### Как решить:
 - Вместо одного контекстного меню с выбором из (all, none, example1, example2, example3) сделать плюсик, который будет добавлять новый модуль аксессуара.
 - Внутри модуля аксессуара контекстное меню с выбором конкретного украшения (без all, none) и ползунки расположения, пповорота в градусах, и масштаба (ползунок масштаба не должен переопределять глобальные и внутри-категорийные параметры, а "прибавляться" к ним)
## 3. [BUG|CRITICAL] При переключении варианта категории сбрасывается выбор подложки категории предыдущеговарианта. Например:
 - Я выбрал у варианта scalable подложку squircle
 - Мне понадобилось сделать то-же самое для варианта 64, и я на него переключился
 - По возвращении на scalable я вижу что вместо squircle установлен default
## 4. [BUG|MINOR] При выборе другого пака название пака над кнопкой Browse... не меняется
## 5. [COSMETIC] При запуске правое меню слишком узкое.
### Как решить:
 - Необходимо запускать программу на одну ширину этого меню шире и расширить само меню вдвое.
## 6. [BUG|MINOR] [POTENTIALLY FEATURE] rotation функция для аксессуаров крутит относительно угла, а не центра. 
### Как решить:
 - Дать возможность выбрать центр оси кручения (Центр, углы [UR,UL,DR,DL])
## 7. [FEATURE|DEBUG] Необходим дебаг вывод в вольную директорию для тестовых генераций не изменяя актуальные иконки в .local/share/icons/
### Как решить:
 - Новая кнопка "Export to..."
 - Старую кнопку стандартного экспорта в .local/share/icons/ переименовать в "Apply"
## 8. [BUG|QoL] Прокручивание вокруг центра есть, но масштабирование идет от угла, а Pivot на это не влияет.
### Как решить (варианты):
 1. Общий Pivot (один для всех)
 2. Раздельные Pivot (один для поворота, другой для масштабирования) [Recommeded]: Более гибкое решение
**Комментарий:** При исправлении поворота неясен был вопрос про масштаб, странные формулировки. Поэтому прошу исправить масштаб отдельно.
## 9. [BUG|CRITICAL] Матуген/xdg все еще не влияют на цвета. Тестовая генерация показала, что матуген/xdg не влияют ни на превью, ни на экспорт.
## 10. [BUG|FATAL] Паника при выборе цветовой темы XDG из .local/share/color-schemes/ "DankMatugen.colors" и Matugen схемы из .cache/DankMaterialShell/ "dms-colors.json"
 - Запись в конфиг сыграла против - перезапуск не помогает т.к. выбор сохранен в конфиг.
**Лог:**
```log
╭─ [akamirsky]>>>[~/Som/Rus/miconium][main]
╰─── ./target/release/miconium-gui

thread 'main' (383864) panicked at crates/miconium-gui/src/main.rs:1275:34:
RefCell already borrowed
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

thread 'main' (383864) panicked at library/core/src/panicking.rs:225:5:
panic in a function that cannot unwind
stack backtrace:
   0:     0x555a11abba32 - <std::sys::backtrace::BacktraceLock::print::DisplayBacktrace as core::fmt::Display>::fmt::hac1d885928ba8582
   1:     0x555a11acd327 - core::fmt::write::h83ebb4d32483be9e
   2:     0x555a11a8ed06 - std::io::Write::write_fmt::ha6a1d6c1ea64b2d0
   3:     0x555a11a9d0b9 - std::panicking::default_hook::{{closure}}::h8e9c4d1276f0925f
   4:     0x555a11a9cf19 - std::panicking::default_hook::h2b2078d38b534dfb
   5:     0x555a11a9d2fb - std::panicking::panic_with_hook::h39b739724e701bfd
   6:     0x555a11a9d1aa - std::panicking::panic_handler::{{closure}}::he540c4833054e458
   7:     0x555a11a99ce9 - std::sys::backtrace::__rust_end_short_backtrace::hfa179d89deec8aed
   8:     0x555a11a828cd - __rustc[d131491b17107b07]::rust_begin_unwind
   9:     0x555a11ad67fd - core::panicking::panic_nounwind_fmt::h2987a0c8ccb716e5
  10:     0x555a11ad677b - core::panicking::panic_nounwind::h6771efadd131a23b
  11:     0x555a11ad6907 - core::panicking::panic_cannot_unwind::h85c6e55646eb04d5
  12:     0x555a118c205f - gtk::signal::editable::trampoline::h098b0c32487f2b0b
  13:     0x7f185d257e33 - g_closure_invoke
  14:     0x7f185d283fa2 - <unknown>
  15:     0x7f185d287ae6 - <unknown>
  16:     0x7f185d288756 - g_signal_emit_by_name
  17:     0x7f185cb732e2 - gtk_entry_set_text
  18:     0x555a1189d909 - miconium_gui::sync_slot_entries::hcdc0f2651491e194
  19:     0x555a118c1cd5 - gtk::auto::entry::EntryExt::connect_activate::activate_trampoline::h9871bf4634c82c64
  20:     0x7f185d257e33 - g_closure_invoke
  21:     0x7f185d283fa2 - <unknown>
  22:     0x7f185d28646b - g_signal_emitv
  23:     0x7f185caa61fd - <unknown>
  24:     0x7f185caa7a79 - <unknown>
  25:     0x7f185caa7e70 - gtk_bindings_activate_event
  26:     0x7f185cb6ae6d - <unknown>
  27:     0x7f185ca74590 - <unknown>
  28:     0x7f185d286c25 - <unknown>
  29:     0x7f185d288349 - g_signal_emit_valist
  30:     0x7f185d28840c - g_signal_emit
  31:     0x7f185cdc17f5 - <unknown>
  32:     0x7f185cdd92cc - gtk_window_propagate_key_event
  33:     0x7f185cdd9384 - <unknown>
  34:     0x7f185ca74590 - <unknown>
  35:     0x7f185d287d0b - <unknown>
  36:     0x7f185d288349 - g_signal_emit_valist
  37:     0x7f185d28840c - g_signal_emit
  38:     0x7f185cdc17f5 - <unknown>
  39:     0x7f185cc21522 - <unknown>
  40:     0x7f185cc221e7 - gtk_main_do_event
  41:     0x7f185c49d667 - <unknown>
  42:     0x7f185c4e1338 - <unknown>
  43:     0x7f185c6edf4d - <unknown>
  44:     0x7f185c6ef478 - <unknown>
  45:     0x7f185c6ef5d2 - g_main_context_iteration
  46:     0x7f185c8fae0e - g_application_run
  47:     0x555a118bb80a - gio::application::ApplicationExtManual::run::ha2a43e5c98e2236b
  48:     0x555a118a5be4 - miconium_gui::main::h993ebe9e5c9d2bc0
  49:     0x555a118b8dc3 - std::sys::backtrace::__rust_begin_short_backtrace::h6c1e3e8f28522e63
  50:     0x555a118bf879 - std::rt::lang_start::{{closure}}::haffa70f67b9d8e8a
  51:     0x555a11a90316 - std::rt::lang_start_internal::h74b643a2cc7fe3b4
  52:     0x555a118a90c5 - main
  53:     0x7f185c227d0e - <unknown>
  54:     0x7f185c227e4b - __libc_start_main
  55:     0x555a118968d5 - _start
  56:                0x0 - <unknown>
thread caused non-unwinding panic. aborting.
fish: Job 1, './target/release/miconium-gui' terminated by signal SIGABRT (Abort)
```
## 11. [BUG|CRITICAL] Попытка выбрать с помощью filechooser не удалась. Скрывает все виды файлов, хотя в filechooser'е выбран тот тип файла, который требуется варианту цветовой схемы (*.json для matugen и *.colors для xdg)
## 12. [FEATURE] Сохранять выбор цветов в конфиг. Например в файле матугена есть большое количество разных цветов, хорошо бы если было поле [matugen] и [xdg-colors] в которых сохранялись выбранные значения цветов на элементах иконки.
 - Естественно при этом если таких цветов нет впоследсвии при запуске программа должна возвращать значения по умолчанию для выбранного источника темы.

___

## Чеклист
- [x] 1
- [x] 2
- [x] 3
- [x] 4
- [x] 5
- [x] 6
- [x] 7
- [x] 8
- [x] 9
- [x] 10
- [x] 11
- [ ] 12
- [ ] 13
