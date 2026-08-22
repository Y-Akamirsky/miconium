#![allow(deprecated)]

use std::cell::RefCell;
use std::rc::Rc;

use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;

use miconium_core::color::{self, Palette};
use miconium_core::config::{self, Config};
use miconium_core::pack::{Pack, PackError};
use miconium_core::svg_engine;

const WORKSPACE_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

struct AppData {
    config: Config,
    pack: Option<Pack>,
    palette: Palette,
    base_palette: Palette,
    preview_image: Option<gtk::Image>,
    color_source: String,
    pack_label: Option<gtk::Label>,
    frame_scale: f64,
    icon_scale: f64,
    acc_scale: f64,
    sidebar: Option<gtk::Box>,
    scale_frame: Option<gtk::Frame>,
    categories_frame: Option<gtk::Frame>,
    active_category: Option<String>,
    preview_sign_name: Option<String>,
    preset_combo: Option<gtk::ComboBoxText>,
    current_preset: Option<String>,
}

impl AppData {
    fn load_pack(&mut self, path: &str) -> Result<(), PackError> {
        self.pack = Some(Pack::load(path)?);
        Ok(())
    }

}

fn main() -> glib::ExitCode {
    let app = gtk::Application::builder()
        .application_id("com.miconium.app")
        .build();

    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &gtk::Application) {
    let loaded = config::load_or_default().unwrap_or_default();
    let mut config = loaded.clone();
    let default_preset_name = loaded.default_preset.clone();
    // If a default preset is configured, start from it instead of the bare
    // main config (item 12: "Можно выбрать пресет по умолчанию").
    if let Some(def) = &default_preset_name {
        if let Ok(preset) = config::load_preset(def) {
            config = preset;
        }
    }

    let base_palette = color::resolve_palette(&config.colors).unwrap_or_default();
    let palette = base_palette.clone().with_layer_map(&config.colors.map);

    let data = Rc::new(RefCell::new(AppData {
        config,
        pack: None,
        palette,
        base_palette,
        preview_image: None,
        color_source: "auto".into(),
        pack_label: None,
        frame_scale: 1.0,
        icon_scale: 1.0,
        acc_scale: 1.0,
        sidebar: None,
        scale_frame: None,
        categories_frame: None,
        active_category: None,
        preview_sign_name: None,
        preset_combo: None,
        current_preset: default_preset_name,
    }));

    let window = gtk::ApplicationWindow::new(app);
    window.set_title("Miconium");
    window.set_default_size(1200, 720);

    let header = gtk::HeaderBar::new();
    header.set_title(Some("Miconium"));
    header.set_subtitle(Some("SVG Icon Generator"));
    header.set_show_close_button(true);

    let all_icons_btn = gtk::Button::with_label("All Icons");
    let data_btn = data.clone();
    all_icons_btn.connect_clicked(move |btn| {
        let toplevel: gtk::Window = btn.toplevel().and_downcast().unwrap();
        show_all_icons(&data_btn, &toplevel);
    });
    header.pack_end(&all_icons_btn);
    window.set_titlebar(Some(&header));

    let paned = gtk::Paned::new(gtk::Orientation::Horizontal);
    paned.set_position(220);

    let sidebar = build_sidebar(&data, &header);
    data.borrow_mut().sidebar = Some(sidebar.clone());
    let sidebar_scrolled = wrap_sidebar_scrolled(&sidebar);
    paned.pack1(&sidebar_scrolled, false, false);

    let right_area = build_right_area(&data);
    paned.pack2(&right_area, true, false);

    window.add(&paned);
    window.show_all();

    try_load_initial_pack(&data, &window, &header);
}

fn try_load_initial_pack(
    data: &Rc<RefCell<AppData>>,
    window: &gtk::ApplicationWindow,
    header: &gtk::HeaderBar,
) {
    let pack_path = {
        let d = data.borrow();
        d.config.pack.path.clone()
    };

    if let Some(path) = pack_path {
        let data_clone = Rc::downgrade(data);
        let paned = window.child().and_downcast::<gtk::Paned>().expect("paned as child");
        let header_clone = header.clone();
        glib::idle_add_local(move || {
            if let Some(data) = data_clone.upgrade() {
                let loaded = data.borrow_mut().load_pack(&path);
                match loaded {
                    Ok(()) => {
                        header_clone.set_subtitle(Some(&format!("Pack: {path}")));
                        rebuild_dynamic_sections(&data);
                        rebuild_right(&data, &paned);
                        update_pack_label(&data);
                        show_first_preview(&data);
                    }
                    Err(e) => {
                        let w: gtk::Window = paned.toplevel().and_downcast().unwrap();
                        show_error(&w, &format!("Failed to load pack: {e}"));
                    }
                }
            }
            glib::ControlFlow::Break
        });
    }
}

fn rebuild_right(data: &Rc<RefCell<AppData>>, paned: &gtk::Paned) {
    if let Some(old_right) = paned.child2() {
        paned.remove(&old_right);
    }
    let new_right = build_right_area(data);
    paned.pack2(&new_right, true, false);
    paned.show_all();
}

fn build_sidebar(data: &Rc<RefCell<AppData>>, header: &gtk::HeaderBar) -> gtk::Box {
    let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 6);
    sidebar.set_margin(8);
    sidebar.set_width_request(200);

    let pack_frame = gtk::Frame::new(Some("Pack"));
    let pack_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    pack_box.set_margin(8);

    let pack_label = gtk::Label::new(None);
    {
        let d = data.borrow();
        let display_name = d.config.pack.path.as_deref()
            .and_then(|p| std::path::Path::new(p).file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("none");
        pack_label.set_text(display_name);
        pack_label.set_xalign(0.0);
    }
    pack_box.pack_start(&pack_label, false, false, 0);
    data.borrow_mut().pack_label = Some(pack_label);

    let browse_btn = gtk::Button::with_label("Browse…");
    let data_clone = Rc::downgrade(data);
    let header_clone = header.downgrade();
    browse_btn.connect_clicked(move |btn| {
        eprintln!("Browse button clicked");
        let (Some(data), Some(header)) = (data_clone.upgrade(), header_clone.upgrade()) else {
            eprintln!("Browse: data or header weak ref expired");
            return;
        };
        let toplevel: gtk::Window = btn.toplevel().and_downcast().unwrap();
        choose_pack(&data, &toplevel, &header);
    });
    pack_box.pack_start(&browse_btn, false, false, 0);

    pack_frame.add(&pack_box);
    sidebar.pack_start(&pack_frame, false, false, 0);

    build_preset_section(&sidebar, data);

    build_color_section(&sidebar, data);
    build_scale_section(&sidebar, data);

    let apply_btn = gtk::Button::with_label("Apply");
    apply_btn.set_margin_top(12);
    let data_clone = Rc::downgrade(data);
    apply_btn.connect_clicked(move |btn| {
        eprintln!("Apply button clicked");
        let Some(data) = data_clone.upgrade() else {
            eprintln!("Apply: data weak ref expired");
            return;
        };
        let toplevel: gtk::Window = btn.toplevel().and_downcast().unwrap();
        run_export_dialog(&data, &toplevel, None, true);
    });
    sidebar.pack_start(&apply_btn, false, false, 0);

    let export_to_btn = gtk::Button::with_label("Export to…");
    let data_clone = Rc::downgrade(data);
    export_to_btn.connect_clicked(move |btn| {
        eprintln!("Export to… button clicked");
        let Some(data) = data_clone.upgrade() else {
            eprintln!("Export to…: data weak ref expired");
            return;
        };
        let toplevel: gtk::Window = btn.toplevel().and_downcast().unwrap();
        choose_export_dir(&data, &toplevel);
    });
    sidebar.pack_start(&export_to_btn, false, false, 0);

    sidebar.pack_start(
        &gtk::Separator::new(gtk::Orientation::Horizontal),
        false,
        false,
        0,
    );

    sidebar
}

/// Wrap the sidebar box in a vertical-only `ScrolledWindow` so the window can
/// shrink below the full content height instead of overflowing small screens
/// (e.g. 1080p). Horizontal scrolling is disabled; the menu scrolls
/// vertically instead.
fn wrap_sidebar_scrolled(sidebar: &gtk::Box) -> gtk::ScrolledWindow {
    let scrolled = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    scrolled.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scrolled.set_propagate_natural_height(false);
    scrolled.set_min_content_height(420);
    scrolled.add(sidebar);
    scrolled
}

fn rebuild_dynamic_sections(data: &Rc<RefCell<AppData>>) {
    let (sidebar, old_scale) = {
        let mut d = data.borrow_mut();
        let sidebar = d.sidebar.clone();
        let old_scale = d.scale_frame.take();
        d.categories_frame = None;
        (sidebar, old_scale)
    };
    let Some(ref sidebar) = sidebar else { return };
    if let Some(old) = old_scale { sidebar.remove(&old); }
    build_scale_section(sidebar, data);
    sidebar.show_all();
}

/// Build the "Presets" frame (item 12): a dropdown to switch presets plus
/// controls to save, delete, and mark the default preset.
fn build_preset_section(sidebar: &gtk::Box, data: &Rc<RefCell<AppData>>) {
    let frame = gtk::Frame::new(Some("Presets"));
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 4);
    box_.set_margin(8);

    let combo = gtk::ComboBoxText::new();
    for name in config::list_presets() {
        combo.append_text(&name);
    }
    {
        let d = data.borrow();
        let active = d
            .current_preset
            .clone()
            .or_else(config::default_preset);
        if let Some(name) = active {
            combo.set_active_id(Some(&name));
        }
    }
    let data_weak = Rc::downgrade(data);
    combo.connect_changed(move |cb| {
        let Some(name) = cb.active_text() else { return };
        let Some(data) = data_weak.upgrade() else { return };
        // Avoid re-loading the preset we already have active (e.g. when the
        // selection is set programmatically while refreshing the list).
        if data.borrow().current_preset.as_deref() == Some(&name) {
            return;
        }
        let toplevel: gtk::ApplicationWindow = match cb.toplevel().and_downcast() {
            Some(w) => w,
            None => return,
        };
        let idle_weak = data_weak.clone();
        glib::idle_add_local(move || {
            if let Some(data) = idle_weak.upgrade() {
                let paned = toplevel.child().and_downcast::<gtk::Paned>();
                let header = toplevel.titlebar().and_downcast::<gtk::HeaderBar>();
                if let (Some(paned), Some(header)) = (paned, header) {
                    apply_preset(&data, &name, &paned, &header, &toplevel);
                }
            }
            glib::ControlFlow::Break
        });
    });
    box_.pack_start(&combo, false, false, 0);
    data.borrow_mut().preset_combo = Some(combo.clone());

    let name_entry = gtk::Entry::new();
    name_entry.set_placeholder_text(Some("preset name"));
    box_.pack_start(&name_entry, false, false, 0);

    let btn_row = build_preset_buttons(data, &name_entry);
    box_.pack_start(&btn_row, false, false, 0);

    let cleanup_btn = gtk::Button::with_label("Cleanup cache");
    cleanup_btn.connect_clicked(move |btn| {
        let icons_root = miconium_core::daemon::default_icons_root();
        match miconium_core::daemon::cleanup_all_except_active(&icons_root) {
            Ok(n) => {
                let msg = if n == 0 {
                    "No inactive icon themes to remove.".to_string()
                } else {
                    format!("Removed {n} inactive icon theme director(y/ies).")
                };
                if let Some(win) = btn.toplevel().and_downcast::<gtk::Window>() {
                    notify_or_dialog(&win, &msg);
                }
            }
            Err(e) => {
                if let Some(win) = btn.toplevel().and_downcast::<gtk::Window>() {
                    show_error(&win, &format!("Cache cleanup failed: {e}"));
                }
            }
        }
    });
    box_.pack_start(&cleanup_btn, false, false, 0);

    frame.add(&box_);
    sidebar.pack_start(&frame, false, false, 0);
}

/// Build the "Save as" / "Delete" / "Set default" button row for the Presets
/// frame. `name_entry` is the preset-name field owned by the parent section.
fn build_preset_buttons(
    data: &Rc<RefCell<AppData>>,
    name_entry: &gtk::Entry,
) -> gtk::Box {
    let btn_row = gtk::Box::new(gtk::Orientation::Horizontal, 4);

    let save_btn = gtk::Button::with_label("Save as");
    let data_weak = Rc::downgrade(data);
    let entry_weak = name_entry.downgrade();
    save_btn.connect_clicked(move |_| {
        let (Some(data), Some(entry)) = (data_weak.upgrade(), entry_weak.upgrade()) else {
            return;
        };
        let name = entry.text().to_string();
        if name.trim().is_empty() {
            return;
        }
        let cfg = data.borrow().config.clone();
        match config::save_preset(&name, &cfg) {
            Ok(()) => {
                data.borrow_mut().current_preset = Some(name.clone());
                refresh_preset_combo(&data);
            }
            Err(e) => {
                if let Some(win) = entry.toplevel().and_downcast::<gtk::Window>() {
                    show_error(&win, &format!("Failed to save preset: {e}"));
                }
            }
        }
    });
    btn_row.pack_start(&save_btn, true, true, 0);

    let delete_btn = gtk::Button::with_label("Delete");
    let data_weak = Rc::downgrade(data);
    delete_btn.connect_clicked(move |btn| {
        let Some(data) = data_weak.upgrade() else { return };
        let name = data.borrow().current_preset.clone();
        let Some(name) = name else { return };
        if let Err(e) = config::delete_preset(&name) {
            if let Some(win) = btn.toplevel().and_downcast::<gtk::Window>() {
                show_error(&win, &format!("Failed to delete preset: {e}"));
            }
            return;
        }
        data.borrow_mut().current_preset = None;
        refresh_preset_combo(&data);
    });
    btn_row.pack_start(&delete_btn, true, true, 0);

    let default_btn = gtk::Button::with_label("Set default");
    let data_weak = Rc::downgrade(data);
    default_btn.connect_clicked(move |btn| {
        let Some(data) = data_weak.upgrade() else { return };
        let name = data.borrow().current_preset.clone();
        let Some(name) = name else { return };
        if let Err(e) = config::set_default_preset(&name) {
            if let Some(win) = btn.toplevel().and_downcast::<gtk::Window>() {
                show_error(&win, &format!("Failed to set default preset: {e}"));
            }
            return;
        }
        if let Some(win) = btn.toplevel().and_downcast::<gtk::Window>() {
            notify_or_dialog(&win, &format!("Default preset set to '{name}'."));
        }
    });
    btn_row.pack_start(&default_btn, true, true, 0);

    btn_row
}

/// Re-populate the preset dropdown from disk and re-select the active preset.
fn refresh_preset_combo(data: &Rc<RefCell<AppData>>) {
    let Some(combo) = data.borrow().preset_combo.clone() else {
        return;
    };
    combo.remove_all();
    for name in config::list_presets() {
        combo.append_text(&name);
    }
    let active = data.borrow().current_preset.clone();
    if let Some(name) = active {
        combo.set_active_id(Some(&name));
    }
}

/// Load a preset by `name`, swap it into `AppData`, and rebuild the UI.
fn apply_preset(
    data: &Rc<RefCell<AppData>>,
    name: &str,
    paned: &gtk::Paned,
    header: &gtk::HeaderBar,
    window: &gtk::ApplicationWindow,
) {
    let new_config = match config::load_preset(name) {
        Ok(c) => c,
        Err(e) => {
            show_error(window.upcast_ref::<gtk::Window>(), &format!("Failed to load preset '{name}': {e}"));
            return;
        }
    };

    {
        let mut d = data.borrow_mut();
        d.config = new_config.clone();
        d.current_preset = Some(name.to_string());
        d.base_palette = color::resolve_palette(&new_config.colors).unwrap_or_default();
        d.palette = d.base_palette.clone().with_layer_map(&new_config.colors.map);
        match &new_config.pack.path {
            Some(path) => {
                if let Err(e) = d.load_pack(path) {
                    show_error(window.upcast_ref::<gtk::Window>(), &format!("Failed to load pack: {e}"));
                }
            }
            None => d.pack = None,
        }
    }

    // Rebuild the whole sidebar so the color/scale sections reflect the new
    // config, then rebuild the right area and refresh the pack label.
    if let Some(old) = paned.child1() {
        paned.remove(&old);
    }
    let new_sidebar = build_sidebar(data, header);
    data.borrow_mut().sidebar = Some(new_sidebar.clone());
    let new_sidebar_scrolled = wrap_sidebar_scrolled(&new_sidebar);
    paned.pack1(&new_sidebar_scrolled, false, false);

    rebuild_right(data, paned);
    update_pack_label(data);
    show_first_preview(data);
    paned.show_all();
}

fn build_right_area(data: &Rc<RefCell<AppData>>) -> gtk::Box {
    let right_box = gtk::Box::new(gtk::Orientation::Vertical, 4);

    let d = data.borrow();
    if d.pack.is_none() {
        let placeholder = gtk::Label::new(Some("Load an icon pack to get started."));
        right_box.pack_start(&placeholder, true, false, 0);
        return right_box;
    }

    drop(d);

    let right_paned = gtk::Paned::new(gtk::Orientation::Horizontal);

    let preview_scrolled =
        gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    preview_scrolled.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);

    let preview_image = gtk::Image::new();
    preview_image.set_margin(20);
    preview_scrolled.add(&preview_image);

    data.borrow_mut().preview_image = Some(preview_image);
    right_paned.pack1(&preview_scrolled, true, false);

    // Categories panel on the right side
    if data.borrow().pack.is_some() {
        let categories_scrolled = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        categories_scrolled.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
        categories_scrolled.set_min_content_width(440);
        let cat_box = build_categories_panel(data);
        categories_scrolled.add(&cat_box);
        right_paned.pack2(&categories_scrolled, false, false);
        right_paned.set_position(520);
    }

    right_box.pack_start(&right_paned, true, true, 0);
    right_box
}

#[allow(clippy::similar_names, clippy::too_many_lines)]
fn build_categories_panel(data: &Rc<RefCell<AppData>>) -> gtk::Box {
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let d = data.borrow();
    let Some(pack) = &d.pack else { return outer };

    let frame = gtk::Frame::new(Some("Categories"));
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);

    let cats = pack.categories();
    let frame_names: Vec<String> = pack.frames.colorizable.iter().map(|f| f.name.clone()).collect();
    let static_dark_names: Vec<String> = pack.frames.static_frames.as_ref().map_or(Vec::new(), |sf| {
        sf.dark.iter().map(|f| f.name.clone()).collect()
    });
    let static_light_names: Vec<String> = pack.frames.static_frames.as_ref().map_or(Vec::new(), |sf| {
        sf.light.iter().map(|f| f.name.clone()).collect()
    });
    let has_static = pack.has_static_frames();
    let acc_names: Vec<String> = pack.accessories.iter().map(|a| a.name.clone()).collect();

    for cat in &cats {
        let signs = pack.get_signs_by_category(cat);
        if signs.is_empty() {
            continue;
        }
        let expander = gtk::Expander::new(Some(cat.as_str()));

        let data_exp = Rc::downgrade(data);
        let cat_exp = cat.to_string();
        expander.connect_expanded_notify(move |ex: &gtk::Expander| {
            if !ex.is_expanded() {
                return;
            }
            let Some(data) = data_exp.upgrade() else { return };
            let variant = data.borrow().config.pack.selected_variant(&cat_exp);
            let pack = data.borrow().pack.clone();
            let Some(pack) = pack else { return };
            data.borrow_mut().active_category = Some(cat_exp.clone());
            data.borrow_mut().preview_sign_name = pick_random_sign(&pack, &cat_exp, &variant);
            rebuild_preview(&data);
        });

        let row = gtk::Box::new(gtk::Orientation::Vertical, 2);
        row.set_margin(4);

        // Variant selector combo
        let variants = pack.get_category_variants(cat);
        let variant_combo = gtk::ComboBoxText::new();
        for v in &variants {
            variant_combo.append_text(v);
        }
        let initial_variant = d.config.pack.selected_variant(cat);
        let initial_idx = variants.iter().position(|v| v == &initial_variant).unwrap_or(0);
        #[allow(clippy::cast_possible_truncation)]
        variant_combo.set_active(Some(initial_idx as u32));
        row.pack_start(&variant_combo, false, false, 0);

        // Scale row: 3 compact sliders
        let scale_row = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        let fr_adj = gtk::Adjustment::new(1.0, 0.1, 3.0, 0.05, 0.1, 0.0);
        let ic_adj = gtk::Adjustment::new(1.0, 0.1, 3.0, 0.05, 0.1, 0.0);
        let ac_adj = gtk::Adjustment::new(1.0, 0.1, 3.0, 0.05, 0.1, 0.0);
        let fr_slider = gtk::Scale::new(gtk::Orientation::Horizontal, Some(&fr_adj));
        fr_slider.set_digits(2);
        fr_slider.set_size_request(80, -1);
        let ic_slider = gtk::Scale::new(gtk::Orientation::Horizontal, Some(&ic_adj));
        ic_slider.set_digits(2);
        ic_slider.set_size_request(80, -1);
        let ac_slider = gtk::Scale::new(gtk::Orientation::Horizontal, Some(&ac_adj));
        ac_slider.set_digits(2);
        ac_slider.set_size_request(80, -1);
        let fr_lbl = gtk::Label::new(Some("Fr"));
        let ic_lbl = gtk::Label::new(Some("Ic"));
        let ac_lbl = gtk::Label::new(Some("Ac"));
        scale_row.pack_start(&fr_lbl, false, false, 0);
        scale_row.pack_start(&fr_slider, true, true, 0);
        scale_row.pack_start(&ic_lbl, false, false, 0);
        scale_row.pack_start(&ic_slider, true, true, 0);
        scale_row.pack_start(&ac_lbl, false, false, 0);
        scale_row.pack_start(&ac_slider, true, true, 0);
        row.pack_start(&scale_row, false, false, 0);

        let frame_cb = gtk::CheckButton::with_label("Frame");
        row.pack_start(&frame_cb, false, false, 0);

        let source_combo = gtk::ComboBoxText::new();
        source_combo.append_text("colorizable");
        if has_static {
            source_combo.append_text("static dark");
            source_combo.append_text("static light");
        }
        source_combo.set_active(Some(0));
        row.pack_start(&source_combo, false, false, 0);

        let frame_combo = gtk::ComboBoxText::new();
        populate_frame_combo(&frame_combo, &frame_names);
        row.pack_start(&frame_combo, false, false, 0);

        let acc_cb = gtk::CheckButton::with_label("Accessories");
        row.pack_start(&acc_cb, false, false, 0);

        let acc_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
        row.pack_start(&acc_box, false, false, 0);

        let add_acc_btn = gtk::Button::with_label("+ Add Accessory");
        row.pack_start(&add_acc_btn, false, false, 0);

        let current_variant = || {
            variant_combo.active_text().unwrap_or_else(|| "scalable".into())
        };

        // Restore saved state
        {
            let variant = current_variant();
            let ov = d.config.pack.variant_override(cat, &variant);
            frame_cb.set_active(ov.show_frame);
            acc_cb.set_active(ov.show_accessories);
            fr_slider.set_value(ov.frame_scale);
            ic_slider.set_value(ov.icon_scale);
            ac_slider.set_value(ov.acc_scale);

            let src_idx = match ov.frame_source.as_str() {
                "static_dark" if has_static => 1,
                "static_light" if has_static => 2,
                _ => 0,
            };
            source_combo.set_active(Some(src_idx));

            let (pool_names, sel_name) = match ov.frame_source.as_str() {
                "static_dark" => (&static_dark_names, ov.selected_static_frame.as_ref()),
                "static_light" => (&static_light_names, ov.selected_static_frame.as_ref()),
                _ => (&frame_names, ov.selected_frame.as_ref()),
            };
            populate_frame_combo(&frame_combo, pool_names);
            if let Some(name) = sel_name {
                if let Some(idx) = pool_names.iter().position(|n| n == name) {
                    #[allow(clippy::cast_possible_truncation)]
                    frame_combo.set_active(Some(idx as u32 + 1));
                }
            }
        }

        rebuild_acc_modules(&acc_box, data, cat, &acc_names);

        // --- Signal: Add accessory button ---
        let add_data = Rc::downgrade(data);
        let add_cat = cat.to_string();
        let add_names = acc_names.clone();
        let add_box = acc_box.clone();
        add_acc_btn.connect_clicked(move |_| {
            let Some(data) = add_data.upgrade() else { return };
            {
                let mut d = data.borrow_mut();
                let variant = d.config.pack.selected_variant(&add_cat);
                let def = d.config.pack.variant_override(&add_cat, &variant);
                let ov = d.config.pack.category_overrides
                    .entry(add_cat.clone()).or_default()
                    .entry(variant).or_insert_with(|| category_override_defaults(&def));
                ov.accessories.push(miconium_core::config::AccessoryConfig::default());
            }
            rebuild_acc_modules(&add_box, &data, &add_cat, &add_names);
            rebuild_preview(&data);
        });

        // Helper: get/save overrides for current variant
        let cat_owned = cat.to_string();

        // --- Signal: Variant combo ---
        let data_v = Rc::downgrade(data);
        let cat_v = cat_owned.clone();
        let fn_fc_v = frame_names.clone();
        let fn_dark_v = static_dark_names.clone();
        let fn_light_v = static_light_names.clone();
        let frame_combo_v = frame_combo.clone();
        let source_combo_v = source_combo.clone();
        let fr_slider_v = fr_slider.clone();
        let ic_slider_v = ic_slider.clone();
        let ac_slider_v = ac_slider.clone();
        let frame_cb_v = frame_cb.clone();
        let acc_cb_v = acc_cb.clone();
        let acc_box_v = acc_box.clone();
        let acc_names_v = acc_names.clone();
        variant_combo.connect_changed(move |combo| {
            let Some(data) = data_v.upgrade() else { return };
            let variant = combo.active_text().unwrap_or_else(|| "scalable".into());
            data.borrow_mut().config.pack.selected_variants.insert(cat_v.clone(), variant.to_string());
            let ov = data.borrow().config.pack.variant_override(&cat_v, &variant);
            frame_cb_v.set_active(ov.show_frame);
            acc_cb_v.set_active(ov.show_accessories);
            fr_slider_v.set_value(ov.frame_scale);
            ic_slider_v.set_value(ov.icon_scale);
            ac_slider_v.set_value(ov.acc_scale);
            let src_idx = match ov.frame_source.as_str() {
                "static_dark" => 1,
                "static_light" => 2,
                _ => 0,
            };
            source_combo_v.set_active(Some(src_idx));
            let (pool, sel_name) = match ov.frame_source.as_str() {
                "static_dark" => (&fn_dark_v, ov.selected_static_frame.as_ref()),
                "static_light" => (&fn_light_v, ov.selected_static_frame.as_ref()),
                _ => (&fn_fc_v, ov.selected_frame.as_ref()),
            };
            populate_frame_combo(&frame_combo_v, pool);
            if let Some(name) = sel_name {
                if let Some(idx) = pool.iter().position(|n| n == name) {
                    #[allow(clippy::cast_possible_truncation)]
                    frame_combo_v.set_active(Some(idx as u32 + 1));
                }
            }
            rebuild_acc_modules(&acc_box_v, &data, &cat_v, &acc_names_v);
            let packv = data.borrow().pack.clone();
            if let Some(pack) = packv {
                data.borrow_mut().preview_sign_name = pick_random_sign(&pack, &cat_v, &variant);
            }
            rebuild_preview(&data);
        });

        // --- Signal: Frame scale slider ---
        let data_frs = Rc::downgrade(data);
        let cat_frs = cat_owned.clone();
        fr_slider.connect_value_changed(move |s| {
            let Some(data) = data_frs.upgrade() else { return };
            let variant = data.borrow().config.pack.selected_variant(&cat_frs);
            let def = data.borrow().config.pack.variant_override(&cat_frs, &variant);
            let mut d2 = data.borrow_mut();
            let ov = d2.config.pack.category_overrides
                .entry(cat_frs.clone()).or_default()
                .entry(variant).or_insert_with(|| category_override_defaults(&def));
            ov.frame_scale = s.value();
            drop(d2);
            rebuild_preview(&data);
        });

        // --- Signal: Icon scale slider ---
        let data_ics = Rc::downgrade(data);
        let cat_ics = cat_owned.clone();
        ic_slider.connect_value_changed(move |s| {
            let Some(data) = data_ics.upgrade() else { return };
            let variant = data.borrow().config.pack.selected_variant(&cat_ics);
            let def = data.borrow().config.pack.variant_override(&cat_ics, &variant);
            let mut d2 = data.borrow_mut();
            let ov = d2.config.pack.category_overrides
                .entry(cat_ics.clone()).or_default()
                .entry(variant).or_insert_with(|| category_override_defaults(&def));
            ov.icon_scale = s.value();
            drop(d2);
            rebuild_preview(&data);
        });

        // --- Signal: Acc scale slider ---
        let data_acs = Rc::downgrade(data);
        let cat_acs = cat_owned.clone();
        ac_slider.connect_value_changed(move |s| {
            let Some(data) = data_acs.upgrade() else { return };
            let variant = data.borrow().config.pack.selected_variant(&cat_acs);
            let def = data.borrow().config.pack.variant_override(&cat_acs, &variant);
            let mut d2 = data.borrow_mut();
            let ov = d2.config.pack.category_overrides
                .entry(cat_acs.clone()).or_default()
                .entry(variant).or_insert_with(|| category_override_defaults(&def));
            ov.acc_scale = s.value();
            drop(d2);
            rebuild_preview(&data);
        });

        // --- Signal: Frame checkbox ---
        let data_fc = Rc::downgrade(data);
        let cat_fc = cat_owned.clone();
        frame_cb.connect_toggled(move |cb| {
            let Some(data) = data_fc.upgrade() else { return };
            let variant = data.borrow().config.pack.selected_variant(&cat_fc);
            let def = data.borrow().config.pack.variant_override(&cat_fc, &variant);
            let mut d2 = data.borrow_mut();
            let ov = d2.config.pack.category_overrides
                .entry(cat_fc.clone()).or_default()
                .entry(variant).or_insert_with(|| category_override_defaults(&def));
            ov.show_frame = cb.is_active();
            drop(d2);
            rebuild_preview(&data);
        });

        // --- Signal: Source combo ---
        let data_src = Rc::downgrade(data);
        let cat_src = cat_owned.clone();
        let fn_fc = frame_names.clone();
        let fn_dark = static_dark_names.clone();
        let fn_light = static_light_names.clone();
        let frame_combo_src = frame_combo.clone();
source_combo.connect_changed(move |combo| {
            let active = combo.active_text().unwrap_or_default();
            let pool: &[String] = match active.as_str() {
                "static dark" => &fn_dark,
                "static light" => &fn_light,
                _ => &fn_fc,
            };
            populate_frame_combo(&frame_combo_src, pool);

            let Some(data) = data_src.upgrade() else { return };
            let variant = data.borrow().config.pack.selected_variant(&cat_src);
            let def = data.borrow().config.pack.variant_override(&cat_src, &variant);
            let mut d2 = data.borrow_mut();
            let ov = d2.config.pack.category_overrides
                .entry(cat_src.clone()).or_default()
                .entry(variant).or_insert_with(|| category_override_defaults(&def));
            ov.frame_source = match active.as_str() {
                "static dark" => "static_dark".into(),
                "static light" => "static_light".into(),
                _ => "colorizable".into(),
            };
            ov.selected_frame = None;
            ov.selected_static_frame = None;
            drop(d2);
            rebuild_preview(&data);
        });

        // --- Signal: Frame combo ---
        let data_fcombo = Rc::downgrade(data);
        let cat_fcombo = cat_owned.clone();
        frame_combo.connect_changed(move |combo| {
            let Some(data) = data_fcombo.upgrade() else { return };
            let variant = data.borrow().config.pack.selected_variant(&cat_fcombo);
            let def = data.borrow().config.pack.variant_override(&cat_fcombo, &variant);
            let mut d2 = data.borrow_mut();
            let ov = d2.config.pack.category_overrides
                .entry(cat_fcombo.clone()).or_default()
                .entry(variant).or_insert_with(|| category_override_defaults(&def));
            let active = combo.active_text();
            match ov.frame_source.as_str() {
                "static_dark" => {
                    ov.selected_static_frame = active.filter(|t| t != "default").map(String::from);
                }
                "static_light" => {
                    ov.selected_static_frame = active.filter(|t| t != "default").map(String::from);
                }
                _ => {
                    ov.selected_frame = active.filter(|t| t != "default").map(String::from);
                }
            }
            drop(d2);
            rebuild_preview(&data);
        });

        // --- Signal: Accessories checkbox ---
        let data_accb = Rc::downgrade(data);
        let cat_accb = cat_owned.clone();
        acc_cb.connect_toggled(move |cb| {
            let Some(data) = data_accb.upgrade() else { return };
            let variant = data.borrow().config.pack.selected_variant(&cat_accb);
            let def = data.borrow().config.pack.variant_override(&cat_accb, &variant);
            let mut d2 = data.borrow_mut();
            let ov = d2.config.pack.category_overrides
                .entry(cat_accb.clone()).or_default()
                .entry(variant).or_insert_with(|| category_override_defaults(&def));
            ov.show_accessories = cb.is_active();
            drop(d2);
            rebuild_preview(&data);
        });

        expander.add(&row);
        list.add(&expander);
    }

    frame.add(&list);
    outer.pack_start(&frame, true, true, 0);
    outer
}

fn populate_frame_combo(combo: &gtk::ComboBoxText, names: &[String]) {
    combo.remove_all();
    combo.append_text("default");
    for name in names {
        combo.append_text(name.as_str());
    }
    combo.set_active(Some(0));
}

fn category_override_defaults(def: &miconium_core::config::CategoryOverride) -> miconium_core::config::CategoryOverride {
    miconium_core::config::CategoryOverride {
        show_frame: def.show_frame,
        show_accessories: def.show_accessories,
        selected_frame: None,
        accessories: Vec::new(),
        frame_source: def.frame_source.clone(),
        selected_static_frame: None,
        frame_scale: def.frame_scale,
        icon_scale: def.icon_scale,
        acc_scale: def.acc_scale,
    }
}

#[allow(clippy::too_many_lines)]
fn build_acc_module_row(
    container: &gtk::Box,
    data: &Rc<RefCell<AppData>>,
    cat: &str,
    acc_names: &[String],
    idx: usize,
    ac: &miconium_core::config::AccessoryConfig,
) -> gtk::Box {
    use miconium_core::config::{AccessoryConfig, RotationCenter};

    let row = gtk::Box::new(gtk::Orientation::Vertical, 4);
    row.set_margin_start(12);

    let top = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    let combo = gtk::ComboBoxText::new();
    for name in acc_names {
        combo.append_text(name);
    }
    if let Some(i) = acc_names.iter().position(|n| n == &ac.name) {
        #[allow(clippy::cast_possible_truncation)]
        combo.set_active(Some(i as u32));
    }
    let rm = gtk::Button::with_label("✕");
    top.pack_start(&combo, true, true, 0);
    top.pack_start(&rm, false, false, 0);
    row.pack_start(&top, false, false, 0);

    let sliders = [
        ("X", ac.x, "x", (-100.0, 100.0)),
        ("Y", ac.y, "y", (-100.0, 100.0)),
        ("Rot", ac.rotation, "rotation", (-180.0, 180.0)),
        ("Scl", ac.scale, "scale", (0.1, 3.0)),
    ];

    for (label, value, key, (lo, hi)) in sliders {
        let srow = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        let lbl = gtk::Label::new(Some(label));
        lbl.set_width_chars(3);
        srow.pack_start(&lbl, false, false, 0);

        let adj = gtk::Adjustment::new(value, lo, hi, 0.1, 1.0, 0.0);
        let slider = gtk::Scale::new(gtk::Orientation::Horizontal, Some(&adj));
        slider.set_digits(1);
        slider.set_size_request(120, -1);
        slider.set_value_pos(gtk::PositionType::Right);
        srow.pack_start(&slider, true, true, 0);
        row.pack_start(&srow, false, false, 0);

        let data_c = Rc::downgrade(data);
        let cat_c = cat.to_string();
        let key_c = key.to_string();
        slider.connect_value_changed(move |s| {
            let Some(data) = data_c.upgrade() else { return };
            let val = s.value();
            {
                let mut d = data.borrow_mut();
                let variant = d.config.pack.selected_variant(&cat_c);
                let def = d.config.pack.variant_override(&cat_c, &variant);
                let ov = d.config.pack.category_overrides
                    .entry(cat_c.clone()).or_default()
                    .entry(variant).or_insert_with(|| category_override_defaults(&def));
                while ov.accessories.len() <= idx {
                    ov.accessories.push(AccessoryConfig::default());
                }
                let ac = &mut ov.accessories[idx];
                match key_c.as_str() {
                    "x" => ac.x = val,
                    "y" => ac.y = val,
                    "rotation" => ac.rotation = val,
                    "scale" => ac.scale = val,
                    _ => {}
                }
            }
            rebuild_preview(&data);
        });
    }

    // Rotation pivot (center / corners).
    let pivot_row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    let pivot_lbl = gtk::Label::new(Some("Pivot"));
    pivot_lbl.set_width_chars(7);
    pivot_row.pack_start(&pivot_lbl, false, false, 0);
    let pivot_combo = gtk::ComboBoxText::new();
    pivot_combo.set_hexpand(true);
    for c in &["Center", "UL", "UR", "DL", "DR"] {
        pivot_combo.append_text(c);
    }
    let pivot_list = [
        ("Center", RotationCenter::Center),
        ("UL", RotationCenter::Ul),
        ("UR", RotationCenter::Ur),
        ("DL", RotationCenter::Dl),
        ("DR", RotationCenter::Dr),
    ];
    if let Some(i) = pivot_list.iter().position(|(_l, c)| *c == ac.rotation_center) {
        #[allow(clippy::cast_possible_truncation)]
        pivot_combo.set_active(Some(i as u32));
    }
    pivot_row.pack_start(&pivot_combo, true, true, 0);
    row.pack_start(&pivot_row, false, false, 0);

    let data_p = Rc::downgrade(data);
    let cat_p = cat.to_string();
    pivot_combo.connect_changed(move |combo| {
        let Some(data) = data_p.upgrade() else { return };
        let Some(label) = combo.active_text() else { return };
        let center = pivot_list
            .iter()
            .find(|(l, _)| *l == label.as_str())
            .map_or(RotationCenter::Center, |(_, c)| *c);
        {
            let mut d = data.borrow_mut();
            let variant = d.config.pack.selected_variant(&cat_p);
            let def = d.config.pack.variant_override(&cat_p, &variant);
            let ov = d.config.pack.category_overrides
                .entry(cat_p.clone()).or_default()
                .entry(variant).or_insert_with(|| category_override_defaults(&def));
            while ov.accessories.len() <= idx {
                ov.accessories.push(AccessoryConfig::default());
            }
            ov.accessories[idx].rotation_center = center;
        }
        rebuild_preview(&data);
    });

    // Scale pivot (center / corners). Independent from the rotation pivot.
    let scale_pivot_row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    let scale_pivot_lbl = gtk::Label::new(Some("Scl."));
    scale_pivot_lbl.set_width_chars(7);
    scale_pivot_row.pack_start(&scale_pivot_lbl, false, false, 0);
    let scale_pivot_combo = gtk::ComboBoxText::new();
    scale_pivot_combo.set_hexpand(true);
    for c in &["Center", "UL", "UR", "DL", "DR"] {
        scale_pivot_combo.append_text(c);
    }
    let scale_pivot_list = [
        ("Center", RotationCenter::Center),
        ("UL", RotationCenter::Ul),
        ("UR", RotationCenter::Ur),
        ("DL", RotationCenter::Dl),
        ("DR", RotationCenter::Dr),
    ];
    if let Some(i) = scale_pivot_list.iter().position(|(_l, c)| *c == ac.scale_center) {
        #[allow(clippy::cast_possible_truncation)]
        scale_pivot_combo.set_active(Some(i as u32));
    }
    scale_pivot_row.pack_start(&scale_pivot_combo, true, true, 0);
    row.pack_start(&scale_pivot_row, false, false, 0);

    let data_scale = Rc::downgrade(data);
    let cat_scale = cat.to_string();
    scale_pivot_combo.connect_changed(move |combo| {
        let Some(data) = data_scale.upgrade() else { return };
        let Some(label) = combo.active_text() else { return };
        let center = scale_pivot_list
            .iter()
            .find(|(l, _)| *l == label.as_str())
            .map_or(RotationCenter::Center, |(_, c)| *c);
        {
            let mut d = data.borrow_mut();
            let variant = d.config.pack.selected_variant(&cat_scale);
            let def = d.config.pack.variant_override(&cat_scale, &variant);
            let ov = d.config.pack.category_overrides
                .entry(cat_scale.clone()).or_default()
                .entry(variant).or_insert_with(|| category_override_defaults(&def));
            while ov.accessories.len() <= idx {
                ov.accessories.push(AccessoryConfig::default());
            }
            ov.accessories[idx].scale_center = center;
        }
        rebuild_preview(&data);
    });

    let data_c = Rc::downgrade(data);
    let cat_c = cat.to_string();
    combo.connect_changed(move |combo| {
        let Some(data) = data_c.upgrade() else { return };
        if let Some(name) = combo.active_text() {
            let mut d = data.borrow_mut();
            let variant = d.config.pack.selected_variant(&cat_c);
            let def = d.config.pack.variant_override(&cat_c, &variant);
            let ov = d.config.pack.category_overrides
                .entry(cat_c.clone()).or_default()
                .entry(variant).or_insert_with(|| category_override_defaults(&def));
            while ov.accessories.len() <= idx {
                ov.accessories.push(AccessoryConfig::default());
            }
            ov.accessories[idx].name = name.to_string();
        }
        rebuild_preview(&data);
    });

    let data_r = Rc::downgrade(data);
    let cat_r = cat.to_string();
    let names_r = acc_names.to_vec();
    let container_r = container.clone();
    rm.connect_clicked(move |_| {
        let Some(data) = data_r.upgrade() else { return };
        {
            let mut d = data.borrow_mut();
            let variant = d.config.pack.selected_variant(&cat_r);
            let def = d.config.pack.variant_override(&cat_r, &variant);
            let ov = d.config.pack.category_overrides
                .entry(cat_r.clone()).or_default()
                .entry(variant).or_insert_with(|| category_override_defaults(&def));
            if idx < ov.accessories.len() {
                ov.accessories.remove(idx);
            }
        }
        rebuild_acc_modules(&container_r, &data, &cat_r, &names_r);
        rebuild_preview(&data);
    });

    row
}

fn rebuild_acc_modules(
    container: &gtk::Box,
    data: &Rc<RefCell<AppData>>,
    cat: &str,
    acc_names: &[String],
) {
    for child in container.children() {
        container.remove(&child);
    }
    let acc_list: Vec<miconium_core::config::AccessoryConfig> = {
        let d = data.borrow();
        let variant = d.config.pack.selected_variant(cat);
        d.config.pack.variant_override(cat, &variant).accessories.clone()
    };
    for (idx, ac) in acc_list.iter().enumerate() {
        let r = build_acc_module_row(container, data, cat, acc_names, idx, ac);
        container.pack_start(&r, false, false, 0);
    }
    container.show_all();
}

fn update_pack_label(data: &Rc<RefCell<AppData>>) {
    let Some(label) = data.borrow().pack_label.clone() else { return };
    let name = data.borrow().config.pack.path.as_deref()
        .and_then(|p| std::path::Path::new(p).file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("none")
        .to_string();
    label.set_text(&name);
}

fn choose_pack(
    data: &Rc<RefCell<AppData>>,
    window: &gtk::Window,
    header: &gtk::HeaderBar,
) {
    let chooser = gtk::FileChooserDialog::new(
        Some("Select Icon Pack"),
        Some(window),
        gtk::FileChooserAction::SelectFolder,
    );
    chooser.add_button("_Open", gtk::ResponseType::Accept);
    chooser.add_button("_Cancel", gtk::ResponseType::Cancel);

    let data_clone = Rc::downgrade(data);
    let paned = window.child().and_downcast::<gtk::Paned>().expect("paned as child");
    let header_clone = header.downgrade();
    chooser.connect_response(move |chooser, response| {
        if response != gtk::ResponseType::Accept {
            chooser.close();
            return;
        }
        if let Some(path) = chooser.filename() {
            let path_str = path.to_string_lossy().to_string();
            if let (Some(data), Some(header)) =
                (data_clone.upgrade(), header_clone.upgrade())
            {
                let loaded = data.borrow_mut().load_pack(&path_str);
                chooser.close();
                match loaded {
                    Ok(()) => {
                        header.set_subtitle(Some(&format!("Pack: {}", &path_str)));
                        rebuild_dynamic_sections(&data);
                        rebuild_right(&data, &paned);
                        update_pack_label(&data);
                        show_first_preview(&data);
                    }
                    Err(e) => {
                        let w: gtk::Window = paned.toplevel().and_downcast().unwrap();
                        show_error(&w, &format!("Failed to load pack:\n{e}"));
                    }
                }
            }
        }
    });
    chooser.show_all();
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn random_usize(max: usize) -> usize {
    if max == 0 {
        return 0;
    }
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    (seed as usize) % max
}

fn pick_random_sign(pack: &Pack, category: &str, variant: &str) -> Option<String> {
    let signs = pack.get_sign_variant(category, variant);
    if signs.is_empty() {
        return None;
    }
    let idx = random_usize(signs.len());
    Some(signs[idx].name.clone())
}

fn pick_random_category(pack: &Pack) -> Option<String> {
    let cats = pack.categories();
    if cats.is_empty() {
        return None;
    }
    let idx = random_usize(cats.len());
    Some(cats[idx].clone())
}

fn rebuild_preview(data: &Rc<RefCell<AppData>>) {
    let name = data.borrow().preview_sign_name.clone();
    if let Some(name) = name {
        preview_icon(data, &name);
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn build_scale_section(sidebar: &gtk::Box, data: &Rc<RefCell<AppData>>) {
    let scale_frame = gtk::Frame::new(Some("Scales"));
    let scale_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    scale_box.set_margin(8);

    let sliders = [
        ("Frame", "frame_scale"),
        ("Icon", "icon_scale"),
        ("Acc", "acc_scale"),
    ];

    for (label, key) in &sliders {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        let lbl = gtk::Label::new(Some(label));
        lbl.set_width_chars(4);
        row.pack_start(&lbl, false, false, 0);

        let adj = gtk::Adjustment::new(1.0, 0.1, 3.0, 0.05, 0.1, 0.0);
        let scale = gtk::Scale::new(gtk::Orientation::Horizontal, Some(&adj));
        scale.set_width_request(100);
        scale.set_digits(2);
        scale.set_value_pos(gtk::PositionType::Right);
        row.pack_start(&scale, true, true, 0);

        let data_cl = Rc::downgrade(data);
        let key_str = key.to_string();
        scale.connect_value_changed(move |s| {
            let Some(data) = data_cl.upgrade() else { return };
            let val = s.value();
            {
                let mut d = data.borrow_mut();
                match key_str.as_str() {
                    "frame_scale" => d.frame_scale = val,
                    "icon_scale" => d.icon_scale = val,
                    "acc_scale" => d.acc_scale = val,
                    _ => {}
                }
            }
            rebuild_preview(&data);
        });

        scale_box.pack_start(&row, false, false, 0);
    }

    scale_frame.add(&scale_box);
    sidebar.pack_start(&scale_frame, false, false, 0);
    data.borrow_mut().scale_frame = Some(scale_frame);
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn rgba_to_hex(color: &gdk::RGBA) -> String {
    let r = (color.red() * 255.0).round() as u8;
    let g = (color.green() * 255.0).round() as u8;
    let b = (color.blue() * 255.0).round() as u8;
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn hex_to_rgba(hex: &str) -> Option<gdk::RGBA> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(gdk::RGBA::new(
        f64::from(r) / 255.0,
        f64::from(g) / 255.0,
        f64::from(b) / 255.0,
        1.0,
    ))
}

fn apply_color_source(
    data: &Rc<RefCell<AppData>>,
    window: &gtk::Window,
    key: &str,
    path: &str,
) {
    let path = path.trim().to_string();
    if path.is_empty() {
        return;
    }
    let resolved = {
        let mut d = data.borrow_mut();
        let c = &mut d.config.colors;
        c.scheme = None;
        c.matugen = None;
        c.manual = None;
        match key {
            "scheme" => c.scheme = Some(path),
            "matugen" => c.matugen = Some(path),
            "manual" => c.manual = Some(path),
            _ => {}
        }
        color::resolve_palette(c)
    };
    match resolved {
        Ok(p) => {
            let mut d = data.borrow_mut();
            d.base_palette = p.clone();
            d.palette = p.with_layer_map(&d.config.colors.map);
        }
        Err(e) => {
            notify_or_dialog(window, &format!("Не удалось загрузить источник цветов: {e}"));
        }
    }
    let cfg = data.borrow().config.clone();
    if let Err(e) = config::save(&cfg) {
        notify_or_dialog(window, &format!("Не удалось сохранить конфиг: {e}"));
    }
    rebuild_preview(data);
}

/// Rebuild the fg/bg/ac `Entry` texts from the current palette.
/// The `data` borrow is dropped before any `set_text`, so the entry's `changed`
/// signal (which re-borrows `data`) cannot run while the borrow is held.
fn sync_slot_entries(entries: &[gtk::Entry], data: &Rc<RefCell<AppData>>) {
    let texts = {
        let d = data.borrow();
        vec![
            d.palette.foreground.clone(),
            d.palette.background.clone(),
            d.palette.accent.clone(),
        ]
    };
    for (entry, text) in entries.iter().zip(texts.iter()) {
        entry.set_text(text);
    }
}

/// Rebuild the Frame/Sign/Acc role `ComboBox` children from the palette's roles
/// and restore the selection from `config.colors.map`.
fn populate_role_combos(combos: &[gtk::ComboBoxText], data: &Rc<RefCell<AppData>>) {
    let (names, map) = {
        let d = data.borrow();
        (d.base_palette.role_names(), d.config.colors.map.clone())
    };
    let targets = [map.frame, map.sign, map.accessory];
    for (combo, target) in combos.iter().zip(targets.iter()) {
        combo.remove_all();
        for n in &names {
            combo.append_text(n);
        }
        if let Some(idx) = names.iter().position(|n| n == target) {
            if let Ok(idx) = u32::try_from(idx) {
                combo.set_active(Some(idx));
            }
        } else if !names.is_empty() {
            combo.set_active(Some(0));
        }
    }
}

#[allow(clippy::too_many_lines)]
fn build_color_section(sidebar: &gtk::Box, data: &Rc<RefCell<AppData>>) {
    let color_frame = gtk::Frame::new(Some("Colors"));
    let color_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    color_box.set_margin(8);

    let source_combo = gtk::ComboBoxText::new();
    for s in &["auto", "xdg", "matugen", "manual"] {
        source_combo.append_text(s);
    }
    source_combo.set_active(Some(0));
    color_box.pack_start(&source_combo, false, false, 0);

    // --- per-source path rows (xdg/matugen/manual) ---
    let sources = [
        ("scheme", "Xdg/Scheme"),
        ("matugen", "Matugen"),
        ("manual", "Manual"),
    ];
    let mut source_entries: Vec<gtk::Entry> = Vec::new();
    let mut source_buttons: Vec<gtk::Button> = Vec::new();

    for (_key, label) in &sources {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        let lbl = gtk::Label::new(Some(label));
        lbl.set_width_chars(10);
        row.pack_start(&lbl, false, false, 0);
        let entry = gtk::Entry::new();
        entry.set_width_chars(14);
        entry.set_hexpand(true);
        entry.set_sensitive(false);
        row.pack_start(&entry, true, true, 0);
        let btn = gtk::Button::with_label("Browse…");
        btn.set_sensitive(false);
        row.pack_start(&btn, false, false, 0);
        color_box.pack_start(&row, false, false, 0);
        source_entries.push(entry);
        source_buttons.push(btn);
    }

    let color_slots = [
        ("fg", "foreground"),
        ("bg", "background"),
        ("ac", "accent"),
    ];
    let mut entries: Vec<gtk::Entry> = Vec::new();
    let mut buttons: Vec<gtk::ColorButton> = Vec::new();

    for (label_text, _key) in &color_slots {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 4);

        let lbl = gtk::Label::new(Some(label_text));
        lbl.set_width_chars(3);
        row.pack_start(&lbl, false, false, 0);

        let entry = gtk::Entry::new();
        entry.set_width_chars(8);
        entry.set_text("#000000");
        entry.set_sensitive(false);
        row.pack_start(&entry, true, true, 0);

        let color_btn = gtk::ColorButton::new();
        color_btn.set_sensitive(false);
        row.pack_start(&color_btn, false, false, 0);

        color_box.pack_start(&row, false, false, 0);
        entries.push(entry);
        buttons.push(color_btn);
    }

    {
        let d = data.borrow();
        let palette_vals = [
            &d.palette.foreground,
            &d.palette.background,
            &d.palette.accent,
        ];
        for (entry, hex) in entries.iter().zip(palette_vals.iter()) {
            entry.set_text(hex);
        }
    }

    let data_weak = Rc::downgrade(data);
    let entries_cb = entries.clone();
    let buttons_cb = buttons.clone();
    let src_entries = source_entries.clone();
    let src_buttons = source_buttons.clone();
    source_combo.connect_changed(move |combo| {
        let Some(data) = data_weak.upgrade() else { return };
        let active = combo.active_text().map(String::from).unwrap_or_default();
        let is_manual = active == "manual";
        for (entry, btn) in entries_cb.iter().zip(buttons_cb.iter()) {
            entry.set_sensitive(is_manual);
            btn.set_sensitive(is_manual);
        }
        let active_idx = match active.as_str() {
            "xdg" => Some(0),
            "matugen" => Some(1),
            "manual" => Some(2),
            _ => None,
        };
        for (i, (entry, btn)) in src_entries.iter().zip(src_buttons.iter()).enumerate() {
            entry.set_sensitive(active_idx == Some(i));
            btn.set_sensitive(active_idx == Some(i));
        }
data.borrow_mut().color_source = active;
    });

    for (i, entry) in entries.iter().enumerate() {
        let data_ec = Rc::downgrade(data);
        let btn = buttons[i].clone();
        let slot_key = color_slots[i].1.to_string();
        entry.connect_changed(move |entry| {
            let Some(data) = data_ec.upgrade() else { return };
            let hex = entry.text().to_string();
            {
                let mut d = data.borrow_mut();
                match slot_key.as_str() {
                    "foreground" => d.palette.foreground.clone_from(&hex),
                    "background" => d.palette.background.clone_from(&hex),
                    "accent" => d.palette.accent.clone_from(&hex),
                    _ => {}
                }
            }
            if let Some(rgba) = hex_to_rgba(&hex) {
                btn.set_rgba(&rgba);
            }
            rebuild_preview(&data);
        });
    }

    for (i, btn) in buttons.iter().enumerate() {
        let data_bc = Rc::downgrade(data);
        let entry = entries[i].clone();
        let slot_key = color_slots[i].1.to_string();
        btn.connect_color_set(move |btn| {
            let Some(data) = data_bc.upgrade() else { return };
            let hex = rgba_to_hex(&btn.rgba());
            entry.set_text(&hex);
            {
                let mut d = data.borrow_mut();
                match slot_key.as_str() {
                    "foreground" => d.palette.foreground.clone_from(&hex),
                    "background" => d.palette.background.clone_from(&hex),
                    "accent" => d.palette.accent.clone_from(&hex),
                    _ => {}
                }
            }
            rebuild_preview(&data);
        });
    }

    // Initial sync: set ColorButton from entry hex
    for (entry, btn) in entries.iter().zip(buttons.iter()) {
        if let Some(rgba) = hex_to_rgba(&entry.text()) {
            btn.set_rgba(&rgba);
        }
    }

    // Layer color mapping: which palette role each icon layer receives.
    let role_labels = [
        ("Frame", "frame"),
        ("Sign", "sign"),
        ("Acc", "accessory"),
    ];
    let map_sep = gtk::Separator::new(gtk::Orientation::Horizontal);
    color_box.pack_start(&map_sep, false, false, 4);

    let role_combos: Vec<gtk::ComboBoxText> = role_labels
        .iter()
        .map(|(label_text, _key)| {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
            let lbl = gtk::Label::new(Some(&format!("{label_text} ->")));
            lbl.set_width_chars(7);
            row.pack_start(&lbl, false, false, 0);

            let combo = gtk::ComboBoxText::new();
            combo.set_hexpand(true);
            row.pack_start(&combo, true, true, 0);
            color_box.pack_start(&row, false, false, 0);
            combo
        })
        .collect();

    populate_role_combos(&role_combos, data);

    for (i, combo) in role_combos.iter().enumerate() {
        let data_rc = Rc::downgrade(data);
        let key = role_labels[i].1.to_string();
        combo.connect_changed(move |combo| {
            let Some(data) = data_rc.upgrade() else { return };
            let Some(role) = combo.active_text() else { return };
            let role = role.to_string();
            {
                let mut d = data.borrow_mut();
                match key.as_str() {
                    "frame" => d.config.colors.map.frame = role,
                    "sign" => d.config.colors.map.sign = role,
                    "accessory" => d.config.colors.map.accessory = role,
                    _ => {}
                }
                d.palette = d
                    .base_palette
                    .clone()
                    .with_layer_map(&d.config.colors.map);
            }
            rebuild_preview(&data);
        });
    }

    // Wire per-source: browse button (open file chooser) and Enter on the entry.
    for (i, (key, _label)) in sources.iter().enumerate() {
        let entry = source_entries[i].clone();
        let btn = source_buttons[i].clone();
        let key_c = key.to_string();
        let key_c2 = key_c.clone();
        let data_b = Rc::downgrade(data);
        let data_bf = Rc::downgrade(data);
        let entry_b = entry.clone();
        let entries_c = entries.clone();
        let entries_click = entries_c.clone();
        let role_combos_c = role_combos.clone();
        let role_combos_click = role_combos_c.clone();

        entry.connect_activate(move |entry| {
            let Some(data) = data_b.upgrade() else { return };
            let path = entry.text();
            if let Some(toplevel) = entry.toplevel().and_then(|w| w.downcast::<gtk::Window>().ok()) {
                apply_color_source(&data, &toplevel, &key_c, &path);
                sync_slot_entries(&entries_c, &data);
                populate_role_combos(&role_combos_c, &data);
            }
        });

        btn.connect_clicked(move |btn| {
            let Some(data) = data_bf.upgrade() else { return };
            if let Some(toplevel) = btn.toplevel().and_then(|w| w.downcast::<gtk::Window>().ok()) {
                let filter = gtk::FileFilter::new();
                filter.set_name(Some(match key_c2.as_str() {
                    "scheme" => "Color schemes (*.colors)",
                    "matugen" => "Matugen JSON (*.json)",
                    _ => "TOML (*.toml)",
                }));
                filter.add_pattern(match key_c2.as_str() {
                    "scheme" => "*.colors",
                    "matugen" => "*.json",
                    _ => "*.toml",
                });
                let all_filter = gtk::FileFilter::new();
                all_filter.set_name(Some("All files (*)"));
                all_filter.add_pattern("*");
                let dialog = gtk::FileChooserDialog::new(
                    Some("Выберите файл цвета"),
                    Some(&toplevel),
                    gtk::FileChooserAction::Open,
                );
                dialog.add_buttons(&[
                    ("Open", gtk::ResponseType::Accept),
                    ("Cancel", gtk::ResponseType::Cancel),
                ]);
                dialog.set_modal(true);
                dialog.add_filter(all_filter);
                dialog.add_filter(filter);
                if dialog.run() == gtk::ResponseType::Accept {
                    if let Some(path) = dialog.filename() {
                        let path = path.to_string_lossy().to_string();
                        entry_b.set_text(&path);
                        apply_color_source(&data, &toplevel, &key_c2, &path);
                        sync_slot_entries(&entries_click, &data);
                        populate_role_combos(&role_combos_click, &data);
                    }
                }
                dialog.close();
            }
        });
    }

    // Initial source rows: fill configured paths, or probe common locations.
    // Values are collected under the borrow, then the borrow is dropped before
    // any widget mutation that can emit signals synchronously.
    let (active_idx, paths) = {
        let d = data.borrow();
        let configured = [
            ("scheme", d.config.colors.scheme.clone()),
            ("matugen", d.config.colors.matugen.clone()),
            ("manual", d.config.colors.manual.clone()),
        ];
        let active_idx = configured
            .iter()
            .position(|(_, p)| p.is_some())
            .map_or(0, |i| i + 1);
        let paths: Vec<String> = configured
            .iter()
            .map(|(key, cfg)| {
                let kind = match *key {
                    "scheme" => miconium_core::color::ColorSourceKind::Xdg,
                    "matugen" => miconium_core::color::ColorSourceKind::Matugen,
                    _ => miconium_core::color::ColorSourceKind::Manual,
                };
                cfg.clone().unwrap_or_else(|| {
                    color::probe_source_path(kind)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default()
                })
            })
            .collect();
        (active_idx, paths)
    };
    #[allow(clippy::cast_possible_truncation)]
    source_combo.set_active(Some(active_idx as u32));
    for (entry, path) in source_entries.iter().zip(paths.iter()) {
        entry.set_text(path);
    }

    color_frame.add(&color_box);
    sidebar.pack_start(&color_frame, false, false, 0);
}

#[allow(dead_code)]
fn find_frame_by_name(pack: &Pack, name: Option<&str>) -> Option<miconium_core::pack::LayerData> {
    let name = name?;
    pack.frames.colorizable.iter().find(|f| f.name == name).cloned()
        .or_else(|| {
            pack.frames.static_frames.as_ref().and_then(|sf| {
                sf.light.iter().chain(sf.dark.iter()).find(|f| f.name == name).cloned()
            })
        })
}


fn show_all_icons(data: &Rc<RefCell<AppData>>, parent: &gtk::Window) {
    let categories = {
        let d = data.borrow();
        let Some(pack) = &d.pack else { return };
        pack.categories().into_iter().filter_map(|cat| {
            let signs = pack.get_signs_by_category(&cat);
            if signs.is_empty() { None } else { Some((cat, signs)) }
        }).collect::<Vec<_>>()
    };

    let win = gtk::Window::new(gtk::WindowType::Toplevel);
    win.set_title("All Icons — Miconium");
    win.set_default_size(700, 550);
    win.set_transient_for(Some(parent));

    let notebook = gtk::Notebook::new();
    let data_weak = Rc::downgrade(data);

    for (cat_name, signs) in &categories {
        let scrolled =
            gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        scrolled.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);

        let flow = gtk::FlowBox::new();
        flow.set_max_children_per_line(8);
        flow.set_min_children_per_line(4);
        flow.set_selection_mode(gtk::SelectionMode::Single);
        flow.set_homogeneous(true);

        for sign in signs {
            let btn = gtk::Button::with_label(&sign.name);
            btn.set_size_request(100, 50);
            let name_clone = sign.name.clone();
            let data_clone = data_weak.clone();
            btn.connect_clicked(move |_| {
                if let Some(data) = data_clone.upgrade() {
                    preview_icon(&data, &name_clone);
                }
            });
            flow.add(&btn);
        }

        scrolled.add(&flow);
        let tab_label = gtk::Label::new(Some(cat_name));
        notebook.append_page(&scrolled, Some(&tab_label));
    }

    win.add(&notebook);
    win.show_all();
}

fn show_first_preview(data: &Rc<RefCell<AppData>>) {
    let sign_name = {
        let d = data.borrow();
        let Some(pack) = &d.pack else { return };
        let cat = pick_random_category(pack);
        let Some(cat) = cat else { return };
        let variant = d.config.pack.selected_variant(&cat);
        pick_random_sign(pack, &cat, &variant)
    };
    let Some(sign_name) = sign_name else { return };
    data.borrow_mut().preview_sign_name = Some(sign_name);
    rebuild_preview(data);
}

fn find_sign_category(pack: &Pack, name: &str) -> Option<(String, String, miconium_core::pack::LayerData)> {
    for cat in pack.categories() {
        for variant in pack.get_category_variants(&cat) {
            if let Some(sign) = pack.get_sign_variant(&cat, &variant).iter().find(|s| s.name == name) {
                return Some((cat, variant, sign.clone()));
            }
        }
    }
    None
}

fn preview_icon(data: &Rc<RefCell<AppData>>, icon_name: &str) {
    let result = {
        let d = data.borrow();
        let Some(pack) = &d.pack else { return };
        let Some((cat, _found_variant, _)) = find_sign_category(pack, icon_name) else { return };
        let variant = d.config.pack.selected_variant(&cat);
        let Some(sign) = pack.get_sign_variant(&cat, &variant).iter().find(|s| s.name == icon_name).cloned() else { return };

        let ov = d.config.pack.variant_override(&cat, &variant);
        let use_frame = ov.show_frame;
        let use_accessories = ov.show_accessories;

        let (frame_data, frame_is_static) = if use_frame {
            let found = match ov.frame_source.as_str() {
                "static_dark" => {
                    let name = ov.selected_static_frame.as_deref();
                    name.and_then(|n| {
                        pack.frames.static_frames.as_ref()?
                            .dark.iter().find(|f| f.name == n).cloned()
                    })
                    .or_else(|| {
                        pack.frames.static_frames.as_ref()?
                            .dark.first().cloned()
                    })
                }
                "static_light" => {
                    let name = ov.selected_static_frame.as_deref();
                    name.and_then(|n| {
                        pack.frames.static_frames.as_ref()?
                            .light.iter().find(|f| f.name == n).cloned()
                    })
                    .or_else(|| {
                        pack.frames.static_frames.as_ref()?
                            .light.first().cloned()
                    })
                }
                _ => {
                    ov.selected_frame.as_deref()
                        .and_then(|n| pack.frames.colorizable.iter().find(|f| f.name == n).cloned())
                        .or_else(|| pack.frames.colorizable.first().cloned())
                }
            };
            let Some(f) = found else {
                eprintln!("Preview: no frames available for '{icon_name}'");
                return;
            };
            let is_static = ov.frame_source.as_str() != "colorizable";
            (f, is_static)
        } else {
            (miconium_core::pack::LayerData {
                svg_content: String::new(),
                name: "empty".into(),
            }, false)
        };

        let accessories: Vec<miconium_core::pack::LayerData> = if use_accessories {
            ov.accessories.iter()
                .filter_map(|ac| pack.accessories.iter().find(|a| a.name == ac.name).cloned())
                .collect()
        } else {
            Vec::new()
        };

        let accessory_transforms: Vec<Option<miconium_core::svg_engine::LayerTransform>> = if use_accessories {
            ov.accessories.iter()
                .map(|ac| if ac.has_offset() { Some(ac.layer_transform()) } else { None })
                .collect()
        } else {
            Vec::new()
        };

        match svg_engine::assemble_icon(&frame_data, &sign, &accessories, &accessory_transforms, &d.palette, use_frame, use_accessories, frame_is_static, ov.frame_scale, ov.icon_scale, ov.acc_scale) {
            Ok(icon) => icon.svg,
            Err(e) => {
                eprintln!("Preview: assemble_icon failed for '{icon_name}': {e}");
                return;
            }
        }
    };

    if let Some(img) = data.borrow().preview_image.as_ref() {
        let tw = img.allocated_width().max(100);
        let th = img.allocated_height().max(100);
        if let Some(pb) = render_svg_to_pixbuf(&result, tw, th) {
            img.set_from_pixbuf(Some(&pb));
        } else {
            img.set_from_icon_name(Some("image-x-generic"), gtk::IconSize::Dialog);
        }
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
fn render_svg_to_pixbuf(svg: &str, target_w: i32, target_h: i32) -> Option<gdk_pixbuf::Pixbuf> {
    use resvg::usvg::TreeParsing;

    let opt = resvg::usvg::Options::default();
    let usvg_tree = match resvg::usvg::Tree::from_str(svg, &opt) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("resvg parse error: {e}");
            return None;
        }
    };
    let rtree = resvg::Tree::from_usvg(&usvg_tree);

    let sw = f64::from(rtree.size.width());
    let sh = f64::from(rtree.size.height());
    let target = f64::from(target_w.min(target_h));
    let scale = (target / sw).min(target / sh);

    let w = (sw * scale).ceil() as u32;
    let h = (sh * scale).ceil() as u32;
    let mut pixmap = tiny_skia::Pixmap::new(w, h)?;

    let ts = tiny_skia::Transform::from_scale(scale as f32, scale as f32);
    rtree.render(ts, &mut pixmap.as_mut());

    let bytes = glib::Bytes::from(pixmap.data());
    Some(gdk_pixbuf::Pixbuf::from_bytes(
        &bytes,
        gdk_pixbuf::Colorspace::Rgb,
        true,
        8,
        w as i32,
        h as i32,
        w as i32 * 4,
    ))
}

fn choose_export_dir(data: &Rc<RefCell<AppData>>, window: &gtk::Window) {
    let chooser = gtk::FileChooserDialog::new(
        Some("Export to folder"),
        Some(window),
        gtk::FileChooserAction::SelectFolder,
    );
    chooser.add_button("_Export", gtk::ResponseType::Accept);
    chooser.add_button("_Cancel", gtk::ResponseType::Cancel);

    let data_clone = Rc::downgrade(data);
    let window_clone = window.clone();
    chooser.connect_response(move |chooser, response| {
        if response != gtk::ResponseType::Accept {
            chooser.close();
            return;
        }
        let Some(path) = chooser.filename() else {
            chooser.close();
            return;
        };
        chooser.close();
        if let Some(data) = data_clone.upgrade() {
            run_export_dialog(&data, &window_clone, Some(path.to_string_lossy().to_string()), false);
        }
    });
    chooser.show_all();
}

#[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
fn run_export_dialog(
    data: &Rc<RefCell<AppData>>,
    window: &gtk::Window,
    output_override: Option<String>,
    refresh_cache: bool,
) {
    let (pack, palette, mut export_cfg, category_overrides) = {
        let d = data.borrow();
        let Some(pack) = &d.pack else {
            eprintln!("Export cancelled: no pack loaded");
            show_error(window, "No pack loaded.\nUse Browse to select an icon pack first.");
            return;
        };
        let pack = pack.clone();
        (pack, d.palette.clone(), d.config.export.clone(), d.config.pack.category_overrides.clone())
    };
    if let Some(path) = output_override {
        export_cfg.output = Some(path);
    }
    // Copy live scale sliders into export config
    {
        let d = data.borrow();
        export_cfg.frame_scale = d.frame_scale;
        export_cfg.icon_scale = d.icon_scale;
        export_cfg.acc_scale = d.acc_scale;
    }

    let dialog = gtk::Dialog::with_buttons(
        Some("Exporting…"),
        Some(window),
        gtk::DialogFlags::MODAL,
        &[("Cancel", gtk::ResponseType::Cancel)],
    );
    dialog.set_default_size(400, 120);

    let content = dialog.content_area();
    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);
    vbox.set_margin(12);
    let label = gtk::Label::new(Some("Generating icon pack…"));
    vbox.pack_start(&label, false, false, 0);
    let progress = gtk::ProgressBar::new();
    progress.set_show_text(true);
    vbox.pack_start(&progress, false, false, 0);
    content.pack_start(&vbox, true, true, 0);
    dialog.show_all();

    let win_for_error = window.clone();
    let (fraction_tx, fraction_rx) =
        glib::MainContext::channel::<f64>(glib::Priority::DEFAULT);
    let (error_tx, error_rx) =
        glib::MainContext::channel::<String>(glib::Priority::DEFAULT);
    let (done_tx, done_rx) =
        glib::MainContext::channel::<(std::path::PathBuf, bool)>(glib::Priority::DEFAULT);
    error_rx.attach(None, move |msg| {
        show_error(&win_for_error, &msg);
        glib::ControlFlow::Break
    });
    let win_for_done = window.clone();
    done_rx.attach(None, move |(path, refresh)| {
        if refresh {
            match miconium_core::export::refresh_icon_cache(&path) {
                Ok(()) => {
                    notify_send("Theme applied and icon cache updated.");
                }
                Err(e) => {
                    eprintln!("Icon cache update failed: {e}");
                    notify_or_dialog(
                        &win_for_done,
                        &format!(
                            "Theme exported to '{}', but the icon cache could not be updated.\n{e}\n\n\
                             Icons may not refresh until the cache is rebuilt \
                             (gtk-update-icon-cache is required).",
                            path.display()
                        ),
                    );
                }
            }
        } else {
            notify_send("Export finished.");
        }
        glib::ControlFlow::Break
    });
    fraction_rx.attach(None, move |fraction| {
        if fraction < 0.0 {
            dialog.close();
            return glib::ControlFlow::Break;
        }
        progress.set_fraction(fraction);
        label.set_text(&format!("{:.0}%", fraction * 100.0));
        if fraction >= 1.0 {
            label.set_text("Done!");
            dialog.close();
            return glib::ControlFlow::Break;
        }
        glib::ControlFlow::Continue
    });

    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        let tx_ref = &tx;
        let result = miconium_core::export::export_pack(
            &pack,
            &palette,
            &export_cfg,
            &category_overrides,
            std::path::Path::new(WORKSPACE_ROOT),
            tx_ref,
        );
        drop(tx);

        for p in &rx {
            let fraction = if p.total > 0 {
                p.current as f64 / p.total as f64
            } else {
                0.0
            };
            if fraction_tx.send(fraction).is_err() {
                break;
            }
        }

        match result {
            Ok(path) => {
                let _ = fraction_tx.send(1.0);
                let _ = done_tx.send((path, refresh_cache));
            }
            Err(e) => {
                eprintln!("Export error: {e}");
                let _ = error_tx.send(format!("Export failed:\n{e}"));
                let _ = fraction_tx.send(-1.0);
            }
        }
    });
}

fn notify_send(summary: &str) {
    let _ = std::process::Command::new("notify-send")
        .args(["-a", "Miconium", "-u", "low"])
        .arg(summary)
        .status();
}

fn notify_or_dialog(window: &gtk::Window, msg: &str) {
    let ok = std::process::Command::new("notify-send")
        .args(["-a", "Miconium", "-u", "normal"])
        .arg(msg)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        show_error(window, msg);
    }
}

fn show_error(window: &gtk::Window, msg: &str) {
    let dialog = gtk::MessageDialog::new(
        Some(window),
        gtk::DialogFlags::empty(),
        gtk::MessageType::Error,
        gtk::ButtonsType::Ok,
        msg,
    );
    dialog.connect_response(|d, _| d.close());
    dialog.show();
}
