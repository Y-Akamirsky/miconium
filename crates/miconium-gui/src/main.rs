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
    preview_image: Option<gtk::Image>,
    color_source: String,
    frame_scale: f64,
    icon_scale: f64,
    acc_scale: f64,
    sidebar: Option<gtk::Box>,
    scale_frame: Option<gtk::Frame>,
    categories_frame: Option<gtk::Frame>,
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
    let config = config::load_or_default().unwrap_or_default();
    let palette = color::resolve_palette(&config.colors).unwrap_or_default();

    let data = Rc::new(RefCell::new(AppData {
        config,
        pack: None,
        palette,
        preview_image: None,
        color_source: "auto".into(),
        frame_scale: 1.0,
        icon_scale: 1.0,
        acc_scale: 1.0,
        sidebar: None,
        scale_frame: None,
        categories_frame: None,
    }));

    let window = gtk::ApplicationWindow::new(app);
    window.set_title("Miconium");
    window.set_default_size(960, 680);

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
    paned.pack1(&sidebar, false, false);

    let right_area = build_right_area(&data);
    paned.pack2(&right_area, true, false);

    // SAFETY: data lives for the entire GTK app lifetime
    unsafe { window.set_data("app-data", data.clone()); }

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

    build_color_section(&sidebar, data);
    build_scale_section(&sidebar, data);

    let export_btn = gtk::Button::with_label("Export Pack");
    export_btn.set_margin_top(12);
    let data_clone = Rc::downgrade(data);
    export_btn.connect_clicked(move |btn| {
        eprintln!("Export button clicked");
        let Some(data) = data_clone.upgrade() else {
            eprintln!("Export: data weak ref expired");
            return;
        };
        let toplevel: gtk::Window = btn.toplevel().and_downcast().unwrap();
        start_export(&data, &toplevel);
    });
    sidebar.pack_start(&export_btn, false, false, 0);

    sidebar.pack_start(
        &gtk::Separator::new(gtk::Orientation::Horizontal),
        false,
        false,
        0,
    );

    sidebar
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
        categories_scrolled.set_min_content_width(220);
        let cat_box = build_categories_panel(data);
        categories_scrolled.add(&cat_box);
        right_paned.pack2(&categories_scrolled, false, false);
        right_paned.set_position(600);
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

    let cats = ["actions", "apps", "categories", "devices", "emblems", "mime", "places", "preferences", "status"];
    let frame_names: Vec<String> = pack.frames.colorizable.iter().map(|f| f.name.clone()).collect();
    let static_dark_names: Vec<String> = pack.frames.static_frames.as_ref().map_or(Vec::new(), |sf| {
        sf.dark.iter().map(|f| f.name.clone()).collect()
    });
    let static_light_names: Vec<String> = pack.frames.static_frames.as_ref().map_or(Vec::new(), |sf| {
        sf.light.iter().map(|f| f.name.clone()).collect()
    });
    let has_static = pack.has_static_frames();
    let acc_names: Vec<String> = pack.accessories.iter().map(|a| a.name.clone()).collect();

    for &cat in &cats {
        let signs = pack.get_signs_by_category(cat);
        if signs.is_empty() {
            continue;
        }
        let row = gtk::Box::new(gtk::Orientation::Vertical, 2);
        row.set_margin(4);

        let cat_label = gtk::Label::new(Some(cat));
        cat_label.set_xalign(0.0);
        row.pack_start(&cat_label, false, false, 0);

        let frame_cb = gtk::CheckButton::with_label("Frame");
        row.pack_start(&frame_cb, false, false, 0);

        // Frame source selector (only if pack has static frames)
        let source_combo = gtk::ComboBoxText::new();
        source_combo.append_text("colorizable");
        if has_static {
            source_combo.append_text("static dark");
            source_combo.append_text("static light");
        }
        source_combo.set_active(Some(0));
        row.pack_start(&source_combo, false, false, 0);

        // Frame name combo — populated based on source
        let frame_combo = gtk::ComboBoxText::new();
        populate_frame_combo(&frame_combo, &frame_names);
        row.pack_start(&frame_combo, false, false, 0);

        let acc_cb = gtk::CheckButton::with_label("Accessories");
        row.pack_start(&acc_cb, false, false, 0);

        let acc_combo = gtk::ComboBoxText::new();
        acc_combo.append_text("all");
        for name in &acc_names {
            acc_combo.append_text(name);
        }
        acc_combo.set_active(Some(0));
        row.pack_start(&acc_combo, false, false, 0);

        // Restore saved state
        {
            let ov = d.config.pack.category_override(cat);
            frame_cb.set_active(ov.show_frame);
            acc_cb.set_active(ov.show_accessories);

            let src_idx = match ov.frame_source.as_str() {
                "static_dark" if has_static => 1,
                "static_light" if has_static => 2,
                _ => 0,
            };
            source_combo.set_active(Some(src_idx));

            // Populate frame combo and select based on source
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

        // --- Signal: Frame checkbox ---
        let data_fc = Rc::downgrade(data);
        let cat_fc = cat.to_string();
        frame_cb.connect_toggled(move |cb| {
            let Some(data) = data_fc.upgrade() else { return };
            let def = data.borrow().config.pack.category_override(&cat_fc);
            let mut d2 = data.borrow_mut();
            let ov = d2.config.pack.category_overrides.entry(cat_fc.clone()).or_insert(category_override_defaults(&def));
            ov.show_frame = cb.is_active();
            drop(d2);
            rebuild_preview(&data);
        });

        // --- Signal: Source combo (colorizable / static dark / static light) ---
        let data_src = Rc::downgrade(data);
        let cat_src = cat.to_string();
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
            let def = data.borrow().config.pack.category_override(&cat_src);
            let mut d2 = data.borrow_mut();
            let ov = d2.config.pack.category_overrides.entry(cat_src.clone()).or_insert(category_override_defaults(&def));
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
        let cat_fcombo = cat.to_string();
        frame_combo.connect_changed(move |combo| {
            let Some(data) = data_fcombo.upgrade() else { return };
            let def = data.borrow().config.pack.category_override(&cat_fcombo);
            let mut d2 = data.borrow_mut();
            let ov = d2.config.pack.category_overrides.entry(cat_fcombo.clone()).or_insert(category_override_defaults(&def));
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
        let cat_accb = cat.to_string();
        acc_cb.connect_toggled(move |cb| {
            let Some(data) = data_accb.upgrade() else { return };
            let def = data.borrow().config.pack.category_override(&cat_accb);
            let mut d2 = data.borrow_mut();
            let ov = d2.config.pack.category_overrides.entry(cat_accb.clone()).or_insert(category_override_defaults(&def));
            ov.show_accessories = cb.is_active();
            drop(d2);
            rebuild_preview(&data);
        });

        // --- Signal: Accessories combo ---
        let data_acco = Rc::downgrade(data);
        let cat_acco = cat.to_string();
        acc_combo.connect_changed(move |combo| {
            let Some(data) = data_acco.upgrade() else { return };
            let def = data.borrow().config.pack.category_override(&cat_acco);
            let mut d2 = data.borrow_mut();
            let ov = d2.config.pack.category_overrides.entry(cat_acco.clone()).or_insert(category_override_defaults(&def));
            let active = combo.active_text();
            match active.as_deref() {
                None | Some("all") => ov.selected_accessories.clear(),
                Some(name) => {
                    ov.selected_accessories.clear();
                    ov.selected_accessories.push(name.to_string());
                }
            }
            drop(d2);
            rebuild_preview(&data);
        });

        list.add(&row);
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
        selected_accessories: Vec::new(),
        frame_source: def.frame_source.clone(),
        selected_static_frame: None,
    }
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

fn rebuild_preview(data: &Rc<RefCell<AppData>>) {
    let name = {
        let d = data.borrow();
        d.pack.as_ref().and_then(|p| {
            p.get_signs_by_category("actions")
                .into_iter()
                .chain(p.get_signs_by_category("apps"))
                .chain(p.get_signs_by_category("categories"))
                .chain(p.get_signs_by_category("devices"))
                .chain(p.get_signs_by_category("emblems"))
                .chain(p.get_signs_by_category("mime"))
                .chain(p.get_signs_by_category("places"))
                .chain(p.get_signs_by_category("preferences"))
                .chain(p.get_signs_by_category("status"))
                .next()
                .map(|s| s.name.clone())
        })
    };
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
    source_combo.connect_changed(move |combo| {
        let Some(data) = data_weak.upgrade() else { return };
        let is_manual = combo.active_text().as_deref() == Some("manual");
        for (entry, btn) in entries_cb.iter().zip(buttons_cb.iter()) {
            entry.set_sensitive(is_manual);
            btn.set_sensitive(is_manual);
        }
        data.borrow_mut().color_source = combo.active_text().unwrap_or_default().to_string();
        rebuild_preview(&data);
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

#[allow(clippy::similar_names, clippy::too_many_lines)]
#[allow(dead_code)]
fn build_categories_section(sidebar: &gtk::Box, data: &Rc<RefCell<AppData>>) {
    let categories: Vec<(String, Vec<String>, Vec<String>)> = {
        let d = data.borrow();
        let Some(pack) = &d.pack else { return };
    let cats = ["actions", "apps", "categories", "devices", "emblems", "mime", "places", "preferences", "status"];
        cats.iter()
            .filter(|c| !pack.get_signs_by_category(c).is_empty())
            .map(|&c| {
                let frames: Vec<String> = pack.frames.colorizable.iter().map(|f| f.name.clone()).collect();
                let accs: Vec<String> = pack.accessories.iter().map(|a| a.name.clone()).collect();
                (c.to_string(), frames, accs)
            })
            .collect()
    };

    if categories.is_empty() {
        return;
    }

    let cat_frame = gtk::Frame::new(Some("Categories"));
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);

    for (cat, frame_names, acc_names) in &categories {
        let row = gtk::Box::new(gtk::Orientation::Vertical, 2);
        row.set_margin(4);

        let cat_label = gtk::Label::new(Some(cat));
        cat_label.set_xalign(0.0);
        row.pack_start(&cat_label, false, false, 0);

        let frame_cb = gtk::CheckButton::with_label("Frame");
        row.pack_start(&frame_cb, false, false, 0);

        let frame_combo = gtk::ComboBoxText::new();
        frame_combo.append_text("default");
        for name in frame_names {
            frame_combo.append_text(name);
        }
        frame_combo.set_active(Some(0));
        row.pack_start(&frame_combo, false, false, 0);

        let acc_cb = gtk::CheckButton::with_label("Accessories");
        row.pack_start(&acc_cb, false, false, 0);

        let acc_combo = gtk::ComboBoxText::new();
        acc_combo.append_text("all");
        for name in acc_names {
            acc_combo.append_text(name);
        }
        acc_combo.set_active(Some(0));
        row.pack_start(&acc_combo, false, false, 0);

        {
            let d = data.borrow();
            let ov = d.config.pack.category_override(cat);
            frame_cb.set_active(ov.show_frame);
            acc_cb.set_active(ov.show_accessories);
            if let Some(ref sf) = ov.selected_frame {
                if let Some(idx) = frame_names.iter().position(|n| n == sf) {
                    #[allow(clippy::cast_possible_truncation)]
                    frame_combo.set_active(Some(idx as u32 + 1));
                }
            }
            if let Some(single) = ov.selected_accessories.first() {
                if let Some(idx) = acc_names.iter().position(|n| n == single) {
                    #[allow(clippy::cast_possible_truncation)]
                    acc_combo.set_active(Some(idx as u32 + 1));
                }
            }
        }

        let data_fc = Rc::downgrade(data);
        let cat_fc = cat.clone();
        frame_cb.connect_toggled(move |cb| {
            let Some(data) = data_fc.upgrade() else { return };
            let def = data.borrow().config.pack.category_override(&cat_fc);
            let mut d = data.borrow_mut();
            let ov = d.config.pack.category_overrides.entry(cat_fc.clone()).or_insert(category_override_defaults(&def));
            ov.show_frame = cb.is_active();
            drop(d);
            rebuild_preview(&data);
        });

        let data_fcombo = Rc::downgrade(data);
        let cat_fcombo = cat.clone();
        frame_combo.connect_changed(move |combo| {
            let Some(data) = data_fcombo.upgrade() else { return };
            let def = data.borrow().config.pack.category_override(&cat_fcombo);
            let mut d = data.borrow_mut();
            let ov = d.config.pack.category_overrides.entry(cat_fcombo.clone()).or_insert(category_override_defaults(&def));
            let active = combo.active_text();
            ov.selected_frame = active.filter(|t| t != "default").map(String::from);
            drop(d);
            rebuild_preview(&data);
        });

        let data_accb = Rc::downgrade(data);
        let cat_accb = cat.clone();
        acc_cb.connect_toggled(move |cb| {
            let Some(data) = data_accb.upgrade() else { return };
            let def = data.borrow().config.pack.category_override(&cat_accb);
            let mut d = data.borrow_mut();
            let ov = d.config.pack.category_overrides.entry(cat_accb.clone()).or_insert(category_override_defaults(&def));
            ov.show_accessories = cb.is_active();
            drop(d);
            rebuild_preview(&data);
        });

        let data_acco = Rc::downgrade(data);
        let cat_acco = cat.clone();
        acc_combo.connect_changed(move |combo| {
            let Some(data) = data_acco.upgrade() else { return };
            let def = data.borrow().config.pack.category_override(&cat_acco);
            let mut d = data.borrow_mut();
            let ov = d.config.pack.category_overrides.entry(cat_acco.clone()).or_insert(category_override_defaults(&def));
            let active = combo.active_text();
            match active.as_deref() {
                None | Some("all") => ov.selected_accessories.clear(),
                Some(name) => {
                    ov.selected_accessories.clear();
                    ov.selected_accessories.push(name.to_string());
                }
            }
            drop(d);
            rebuild_preview(&data);
        });

        list.add(&row);
    }

    let scrolled = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    scrolled.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    scrolled.set_max_content_height(400);
    scrolled.add(&list);

    cat_frame.add(&scrolled);
    sidebar.pack_start(&cat_frame, false, false, 0);
    data.borrow_mut().categories_frame = Some(cat_frame);
}

fn show_all_icons(data: &Rc<RefCell<AppData>>, parent: &gtk::Window) {
    let categories = {
        let d = data.borrow();
        let Some(pack) = &d.pack else { return };
        let mut cats: Vec<(String, Vec<miconium_core::pack::LayerData>)> = Vec::new();
        for cat in &["actions", "apps", "categories", "devices", "emblems", "mime", "places", "preferences", "status"] {
            let signs = pack.get_signs_by_category(cat);
            if !signs.is_empty() {
                cats.push((cat.to_string(), signs));
            }
        }
        cats
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
    let first_sign = {
        let d = data.borrow();
        d.pack.as_ref().and_then(|p| {
            p.get_signs_by_category("actions")
                .into_iter()
                .chain(p.get_signs_by_category("apps"))
                .chain(p.get_signs_by_category("categories"))
                .chain(p.get_signs_by_category("devices"))
                .chain(p.get_signs_by_category("emblems"))
                .chain(p.get_signs_by_category("mime"))
                .chain(p.get_signs_by_category("places"))
                .chain(p.get_signs_by_category("preferences"))
                .chain(p.get_signs_by_category("status"))
                .next()
                .map(|s| s.name.clone())
        })
    };
    if let Some(name) = first_sign {
        preview_icon(data, &name);
    }
}

fn find_sign_category<'a>(pack: &'a Pack, name: &str) -> Option<(&'a str, miconium_core::pack::LayerData)> {
    for cat in &["actions", "apps", "categories", "devices", "emblems", "mime", "places", "preferences", "status"] {
        if let Some(sign) = pack.signs_by_category_ref(cat).iter().find(|s| s.name == name) {
            return Some((cat, sign.clone()));
        }
    }
    None
}

fn preview_icon(data: &Rc<RefCell<AppData>>, icon_name: &str) {
    let result = {
        let d = data.borrow();
        let Some(pack) = &d.pack else { return };
        let Some((cat, sign)) = find_sign_category(pack, icon_name) else { return };

        let ov = d.config.pack.category_override(cat);
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

        let accessories = if use_accessories {
            if ov.selected_accessories.is_empty() {
                pack.accessories.clone()
            } else {
                pack.accessories.iter().filter(|a| ov.selected_accessories.contains(&a.name)).cloned().collect()
            }
        } else {
            Vec::new()
        };

        let fr_s = d.frame_scale;
        let ic_s = d.icon_scale;
        let ac_s = d.acc_scale;
        match svg_engine::assemble_icon(&frame_data, &sign, &accessories, &d.palette, use_frame, use_accessories, frame_is_static, fr_s, ic_s, ac_s) {
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

#[allow(clippy::cast_precision_loss)]
fn start_export(data: &Rc<RefCell<AppData>>, window: &gtk::Window) {
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
    error_rx.attach(None, move |msg| {
        show_error(&win_for_error, &msg);
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
            Ok(()) => { let _ = fraction_tx.send(1.0); }
            Err(e) => {
                eprintln!("Export error: {e}");
                let _ = error_tx.send(format!("Export failed:\n{e}"));
                let _ = fraction_tx.send(-1.0);
            }
        }
    });
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
