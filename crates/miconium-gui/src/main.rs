#![allow(deprecated)]

use std::cell::RefCell;
use std::rc::Rc;

use gtk::glib;
use gtk::prelude::*;

use miconium_core::color::{self, Palette};
use miconium_core::config::{self, Config};
use miconium_core::pack::{Pack, PackError};
use miconium_core::svg_engine;

const PREVIEW_SIZE: i32 = 200;
const WORKSPACE_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

struct AppData {
    config: Config,
    pack: Option<Pack>,
    palette: Palette,
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
    }));

    let window = gtk::ApplicationWindow::new(app);
    window.set_title("Miconium");
    window.set_default_size(960, 680);

    let header = gtk::HeaderBar::new();
    header.set_title(Some("Miconium"));
    header.set_subtitle(Some("SVG Icon Generator"));
    header.set_show_close_button(true);
    window.set_titlebar(Some(&header));

    let paned = gtk::Paned::new(gtk::Orientation::Horizontal);
    paned.set_position(220);

    let sidebar = build_sidebar(&data, &header);
    paned.pack1(&sidebar, false, false);

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
                match data.borrow_mut().load_pack(&path) {
                    Ok(()) => {
                        header_clone.set_subtitle(Some(&format!("Pack: {path}")));
                        rebuild_right(&data, &paned);
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
        let name = d.config.pack.path.as_deref().unwrap_or("none");
        pack_label.set_text(name);
        pack_label.set_xalign(0.0);
    }
    pack_box.pack_start(&pack_label, false, false, 0);

    let browse_btn = gtk::Button::with_label("Browse…");
    let data_clone = Rc::downgrade(data);
    let header_clone = header.downgrade();
    browse_btn.connect_clicked(move |btn| {
        if let (Some(data), Some(header)) = (data_clone.upgrade(), header_clone.upgrade()) {
            let toplevel: gtk::Window = btn.toplevel().and_downcast().unwrap();
            choose_pack(&data, &toplevel, &header);
        }
    });
    pack_box.pack_start(&browse_btn, false, false, 0);

    pack_frame.add(&pack_box);
    sidebar.pack_start(&pack_frame, false, false, 0);

    let color_frame = gtk::Frame::new(Some("Colors"));
    let color_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    color_box.set_margin(8);

    let color_info = gtk::Label::new(None);
    {
        let d = data.borrow();
        let fg = &d.palette.foreground;
        let bg = &d.palette.background;
        let ac = &d.palette.accent;
        color_info.set_markup(&format!(
            "fg: <span foreground=\"{fg}\">{fg}</span>\n\
             bg: <span foreground=\"{bg}\">{bg}</span>\n\
             accent: <span foreground=\"{ac}\">{ac}</span>",
        ));
        color_info.set_xalign(0.0);
    }
    color_box.pack_start(&color_info, false, false, 0);
    color_frame.add(&color_box);
    sidebar.pack_start(&color_frame, false, false, 0);

    let export_btn = gtk::Button::with_label("Export Pack");
    export_btn.set_margin_top(12);
    let data_clone = Rc::downgrade(data);
    export_btn.connect_clicked(move |btn| {
        if let Some(data) = data_clone.upgrade() {
            let toplevel: gtk::Window = btn.toplevel().and_downcast().unwrap();
            start_export(&data, &toplevel);
        }
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

fn build_right_area(data: &Rc<RefCell<AppData>>) -> gtk::Paned {
    let vpaned = gtk::Paned::new(gtk::Orientation::Vertical);
    vpaned.set_position(350);

    let d = data.borrow();
    if d.pack.is_none() {
        let placeholder = gtk::Label::new(Some("Load an icon pack to get started."));
        vpaned.pack1(&placeholder, true, false);
        return vpaned;
    }

    let pack = d.pack.as_ref().unwrap();
    let notebook = gtk::Notebook::new();
    for cat in &["apps", "categories", "devices", "emblems", "mime", "preferences", "status"] {
        let signs = pack.get_signs_by_category(cat);
        if signs.is_empty() {
            continue;
        }

        let scrolled =
            gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        scrolled.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);

        let flow = gtk::FlowBox::new();
        flow.set_max_children_per_line(8);
        flow.set_min_children_per_line(4);
        flow.set_selection_mode(gtk::SelectionMode::Single);
        flow.set_homogeneous(true);

        for sign in &signs {
            let btn = gtk::Button::with_label(&sign.name);
            btn.set_size_request(80, 40);
            let data_clone = Rc::downgrade(data);
            let icon_name = sign.name.clone();
            btn.connect_clicked(move |_| {
                if let Some(data) = data_clone.upgrade() {
                    preview_icon(&data, &icon_name);
                }
            });
            flow.add(&btn);
        }

        scrolled.add(&flow);

        let tab_label = gtk::Label::new(Some(cat));
        notebook.append_page(&scrolled, Some(&tab_label));
    }

    drop(d);
    vpaned.pack1(&notebook, true, false);

    let preview_placeholder = gtk::Label::new(Some("Select an icon to preview"));
    preview_placeholder.set_margin(20);
    vpaned.pack2(&preview_placeholder, true, false);

    vpaned
}

fn choose_pack(
    data: &Rc<RefCell<AppData>>,
    window: &gtk::Window,
    header: &gtk::HeaderBar,
) {
    let chooser = gtk::FileChooserNative::new(
        Some("Select Icon Pack"),
        Some(window),
        gtk::FileChooserAction::SelectFolder,
        Some("Open"),
        Some("Cancel"),
    );

    let data_clone = Rc::downgrade(data);
    let paned = window.child().and_downcast::<gtk::Paned>().expect("paned as child");
    let header_clone = header.downgrade();
    chooser.connect_response(move |chooser, response| {
        if response != gtk::ResponseType::Accept {
            return;
        }
        if let Some(gfile) = chooser.file() {
            if let Some(path) = gfile.path() {
                let path_str = path.to_string_lossy().to_string();
                if let (Some(data), Some(header)) =
                    (data_clone.upgrade(), header_clone.upgrade())
                {
                    match data.borrow_mut().load_pack(&path_str) {
                        Ok(()) => {
                            header.set_subtitle(Some(&format!("Pack: {}", &path_str)));
                            rebuild_right(&data, &paned);
                        }
                        Err(e) => {
                            let w: gtk::Window = paned.toplevel().and_downcast().unwrap();
                            show_error(&w, &format!("Failed to load pack:\n{e}"));
                        }
                    }
                }
            }
        }
    });
    chooser.show();
}

fn preview_icon(data: &Rc<RefCell<AppData>>, icon_name: &str) {
    let svg_content = {
        let d = data.borrow();
        let Some(pack) = &d.pack else { return };

        let sign = pack
            .get_signs_by_category("apps")
            .into_iter()
            .chain(pack.get_signs_by_category("categories"))
            .chain(pack.get_signs_by_category("devices"))
            .chain(pack.get_signs_by_category("emblems"))
            .chain(pack.get_signs_by_category("mime"))
            .chain(pack.get_signs_by_category("preferences"))
            .chain(pack.get_signs_by_category("status"))
            .find(|s| s.name == icon_name);

        let Some(sign) = sign else { return };

        let frame_data = d
            .pack
            .as_ref()
            .and_then(|p| p.frames.colorizable.first().cloned())
            .unwrap_or_else(|| miconium_core::pack::LayerData {
                svg_content: String::new(),
                name: "empty".into(),
            });

        let accessories: Vec<miconium_core::pack::LayerData> = d
            .pack
            .as_ref()
            .map(|p| p.accessories.clone())
            .unwrap_or_default();

        match svg_engine::assemble_icon(&frame_data, &sign, &accessories, &d.palette) {
            Ok(icon) => icon.svg,
            Err(_) => return,
        }
    };

    show_svg_preview(&svg_content, icon_name);
}

fn show_svg_preview(svg: &str, title: &str) {
    let win = gtk::Window::new(gtk::WindowType::Toplevel);
    win.set_title(title);
    win.set_default_size(PREVIEW_SIZE + 40, PREVIEW_SIZE + 40);

    let pixbuf = {
        let bytes = glib::Bytes::from(svg.as_bytes());
        let stream = gio::MemoryInputStream::from_bytes(&bytes);
        gdk_pixbuf::Pixbuf::from_stream(&stream, None::<&gio::Cancellable>)
    };

    let image = match pixbuf {
        Ok(pb) => {
            let scaled =
                pb.scale_simple(PREVIEW_SIZE, PREVIEW_SIZE, gdk_pixbuf::InterpType::Bilinear);
            match scaled {
                Some(s) => gtk::Image::from_pixbuf(Some(&s)),
                None => {
                    gtk::Image::from_icon_name(Some("image-x-generic"), gtk::IconSize::Dialog)
                }
            }
        }
        Err(_) => gtk::Image::from_icon_name(Some("image-x-generic"), gtk::IconSize::Dialog),
    };

    win.add(&image);
    win.show_all();
}

#[allow(clippy::cast_precision_loss)]
fn start_export(data: &Rc<RefCell<AppData>>, window: &gtk::Window) {
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

    let (fraction_tx, fraction_rx) =
        glib::MainContext::channel::<f64>(glib::Priority::DEFAULT);
    fraction_rx.attach(None, move |fraction| {
        progress.set_fraction(fraction);
        label.set_text(&format!("{:.0}%", fraction * 100.0));
        if fraction >= 1.0 {
            label.set_text("Done!");
            dialog.close();
            return glib::ControlFlow::Break;
        }
        glib::ControlFlow::Continue
    });

    let (pack, palette, export_cfg) = {
        let d = data.borrow();
        let pack = match &d.pack {
            Some(p) => p.clone(),
            None => return,
        };
        let palette = d.palette.clone();
        let export_cfg = d.config.export.clone();
        (pack, palette, export_cfg)
    };

    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        let tx_ref = &tx;
        let result = miconium_core::export::export_pack(
            &pack,
            &palette,
            &export_cfg,
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
                let _ = fraction_tx.send(1.0);
                eprintln!("Export error: {e}");
            }
        }
    });
}

fn show_error(window: &gtk::Window, msg: &str) {
    let dialog = gtk::MessageDialog::new(
        Some(window),
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Error,
        gtk::ButtonsType::Ok,
        msg,
    );
    dialog.connect_response(|d, _| d.close());
    dialog.show();
}
