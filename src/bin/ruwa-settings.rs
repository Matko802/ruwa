#[path = "../settings.rs"]
mod settings;

use gtk::prelude::*;
use settings::{AudioBackend, DisplayBackend, Prefs};
use std::cell::RefCell;
use std::rc::Rc;

const RUWA_CSS: &str = "
.ruwa-card {
  background-color: alpha(@theme_fg_color, 0.08);
  border-radius: 14px;
  padding: 12px;
}
.ruwa-caption {
  font-weight: bold;
  margin-top: 6px;
}
";

fn caption(text: &str) -> gtk::Label {
    let label = gtk::Label::new(None);
    label.set_markup(&format!("<b>{text}</b>"));
    label.set_xalign(0.0);
    label.set_css_classes(&["ruwa-caption"]);
    label
}

fn hint(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.set_wrap(true);
    label.set_css_classes(&["dim-label"]);
    label
}

fn card_row(label_text: &str, control: &impl IsA<gtk::Widget>) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    card.set_css_classes(&["ruwa-card"]);
    let label = gtk::Label::new(Some(label_text));
    label.set_hexpand(true);
    label.set_xalign(0.0);
    label.set_valign(gtk::Align::Center);
    label.set_wrap(true);
    card.append(&label);
    card.append(control);
    card
}

fn card_wrap(widget: &impl IsA<gtk::Widget>) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    card.set_css_classes(&["ruwa-card"]);
    card.append(widget);
    card
}

fn page() -> gtk::Box {
    let body = gtk::Box::new(gtk::Orientation::Vertical, 10);
    body.set_margin_top(14);
    body.set_margin_bottom(14);
    body.set_margin_start(14);
    body.set_margin_end(14);
    body
}

fn page_scroll(body: &gtk::Box) -> gtk::ScrolledWindow {
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_hexpand(true);
    scroll.set_vexpand(true);
    scroll.set_child(Some(body));
    scroll
}

fn main() {
    let app = gtk::Application::builder()
        .application_id("com.rused.RuwaSettings")
        .build();
    app.connect_activate(build_ui);
    app.run();
}

fn switch_row(
    parent: &gtk::Box,
    state: &Rc<RefCell<Prefs>>,
    profile: &Rc<std::path::PathBuf>,
    status: &gtk::Label,
    label: &str,
    get: impl Fn(&Prefs) -> bool + 'static,
    set: impl Fn(&mut Prefs, bool) + 'static,
) {
    let st = state.clone();
    let status = status.clone();
    let profile = profile.clone();
    let sw = gtk::Switch::new();
    sw.set_active(get(&st.borrow()));
    sw.set_halign(gtk::Align::End);
    sw.set_valign(gtk::Align::Center);
    sw.connect_state_set(move |_, on| {
        set(&mut st.borrow_mut(), on);
        save(&st, &profile, &status);
        gtk::glib::Propagation::Proceed
    });
    parent.append(&card_row(label, &sw));
}

fn build_ui(app: &gtk::Application) {
    let css = gtk::CssProvider::new();
    css.load_from_data(RUWA_CSS);
    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("display"),
        &css,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let profile = std::rc::Rc::new(settings::profile_dir());
    let state = Rc::new(RefCell::new(settings::read_prefs(&profile)));

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Ruwa Settings")
        .default_width(560)
        .default_height(620)
        .build();

    let header = gtk::HeaderBar::new();
    header.set_title_widget(Some(&gtk::Label::new(Some("Ruwa Settings"))));
    window.set_titlebar(Some(&header));

    let status = gtk::Label::new(Some("Changes apply on next launch."));
    status.set_xalign(0.0);
    status.set_wrap(true);
    status.set_margin_top(8);

    let notebook = gtk::Notebook::new();
    notebook.set_hexpand(true);
    notebook.set_vexpand(true);

    notebook.append_page(
        &page_scroll(&general_page(&state, &profile, &status, &window)),
        Some(&gtk::Label::new(Some("General"))),
    );
    notebook.append_page(
        &page_scroll(&performance_page(&state, &profile, &status)),
        Some(&gtk::Label::new(Some("Performance"))),
    );
    notebook.append_page(
        &page_scroll(&appearance_page(&status)),
        Some(&gtk::Label::new(Some("Appearance"))),
    );
    notebook.append_page(
        &page_scroll(&system_page(&state, &profile, &status)),
        Some(&gtk::Label::new(Some("System"))),
    );

    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.set_margin_top(8);
    root.set_margin_bottom(10);
    root.set_margin_start(10);
    root.set_margin_end(10);
    root.append(&notebook);
    root.append(&status);

    window.set_child(Some(&root));
    window.present();
}

fn general_page(
    state: &Rc<RefCell<Prefs>>,
    profile: &Rc<std::path::PathBuf>,
    status: &gtk::Label,
    window: &gtk::ApplicationWindow,
) -> gtk::Box {
    let body = page();
    body.append(&caption("Behavior"));
    switch_row(
        &body,
        state,
        profile,
        status,
        "Keep running in tray when window closes",
        |p| p.close_to_tray,
        |p, v| p.close_to_tray = v,
    );
    switch_row(
        &body,
        state,
        profile,
        status,
        "Start minimized to tray",
        |p| p.start_minimized,
        |p, v| p.start_minimized = v,
    );
    {
        let st = state.clone();
        let status = status.clone();
        let profile = profile.clone();
        let display = gtk::DropDown::from_strings(&[
            DisplayBackend::Auto.label(),
            DisplayBackend::Wayland.label(),
            DisplayBackend::X11.label(),
        ]);
        let selected = match st.borrow().display {
            DisplayBackend::Auto => 0,
            DisplayBackend::Wayland => 1,
            DisplayBackend::X11 => 2,
        };
        display.set_selected(selected);
        display.set_valign(gtk::Align::Center);
        display.connect_selected_notify(move |d| {
            st.borrow_mut().display = match d.selected() {
                1 => DisplayBackend::Wayland,
                2 => DisplayBackend::X11,
                _ => DisplayBackend::Auto,
            };
            save(&st, &profile, &status);
        });
        body.append(&card_row("Display backend", &display));
    }

    body.append(&caption("Downloads"));
    {
        let st = state.clone();
        let status = status.clone();
        let profile = profile.clone();
        let dl = gtk::Entry::new();
        if !st.borrow().download_dir.trim().is_empty() {
            dl.set_text(&st.borrow().download_dir);
        }
        dl.set_placeholder_text(Some("~/Downloads"));
        dl.set_hexpand(true);
        dl.set_tooltip_text(Some("Supports ~/ and $HOME/ prefixes"));
        let pending: Rc<RefCell<Option<gtk::glib::SourceId>>> = Rc::new(RefCell::new(None));
        let save_debounced = {
            let st = st.clone();
            let status = status.clone();
            let profile = profile.clone();
            let pending = pending.clone();
            move |text: String| {
                if let Some(id) = pending.borrow_mut().take() {
                    id.remove();
                }
                let st = st.clone();
                let status = status.clone();
                let profile = profile.clone();
                let pending_inner = pending.clone();
                let id = gtk::glib::timeout_add_local_once(
                    std::time::Duration::from_millis(600),
                    move || {
                        *pending_inner.borrow_mut() = None;
                        st.borrow_mut().download_dir = text;
                        save(&st, &profile, &status);
                    },
                );
                *pending.borrow_mut() = Some(id);
            }
        };
        dl.connect_changed(move |e| {
            save_debounced(e.text().to_string());
        });
        let browse = gtk::Button::with_label("Browse…");
        browse.set_valign(gtk::Align::Center);
        {
            let st = st.clone();
            let status = status.clone();
            let profile = profile.clone();
            let window_weak = window.downgrade();
            let dl_weak = dl.downgrade();
            browse.connect_clicked(move |_| {
                let Some(window) = window_weak.upgrade() else {
                    return;
                };
                let dialog = gtk::FileDialog::builder()
                    .title("Choose download folder")
                    .build();
                let st = st.clone();
                let status = status.clone();
                let profile = profile.clone();
                let dl_weak = dl_weak.clone();
                dialog.select_folder(Some(&window), gtk::gio::Cancellable::NONE, move |res| {
                    match res {
                        Ok(file) => {
                            if let Some(path) = file.path() {
                                let s = path.to_string_lossy().to_string();
                                if let Some(entry) = dl_weak.upgrade() {
                                    entry.set_text(&s);
                                }
                                st.borrow_mut().download_dir = s;
                                save(&st, &profile, &status);
                            }
                        }
                        Err(_) => {}
                    }
                });
            });
        }
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.append(&dl);
        row.append(&browse);
        let eff = gtk::Label::new(Some(&format!(
            "Effective: {}",
            settings::effective_download_dir(&state.borrow()).display()
        )));
        eff.set_xalign(0.0);
        eff.set_css_classes(&["dim-label"]);
        {
            let st = state.clone();
            let eff = eff.clone();
            let tick = move || {
                eff.set_text(&format!(
                    "Effective: {}",
                    settings::effective_download_dir(&st.borrow()).display()
                ));
                gtk::glib::ControlFlow::Continue
            };
            gtk::glib::timeout_add_seconds_local(1, tick);
        }
        let wrap = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let title = gtk::Label::new(Some("Download folder"));
        title.set_xalign(0.0);
        wrap.append(&title);
        wrap.append(&row);
        wrap.append(&eff);
        body.append(&card_wrap(&wrap));
    }
    body
}

fn performance_page(
    state: &Rc<RefCell<Prefs>>,
    profile: &Rc<std::path::PathBuf>,
    status: &gtk::Label,
) -> gtk::Box {
    let body = page();
    body.append(&caption("Rendering"));
    switch_row(
        &body,
        state,
        profile,
        status,
        "GPU acceleration (WebRender)",
        |p| p.gpu_accel,
        |p, v| p.gpu_accel = v,
    );
    switch_row(
        &body,
        state,
        profile,
        status,
        "Hardware video decoding (VA-API)",
        |p| p.vaapi,
        |p, v| p.vaapi = v,
    );
    body.append(&caption("Audio"));
    {
        let st = state.clone();
        let status = status.clone();
        let profile = profile.clone();
        let audio = gtk::DropDown::from_strings(&[
            AudioBackend::Auto.label(),
            AudioBackend::Alsa.label(),
            AudioBackend::Pulse.label(),
        ]);
        let selected = match st.borrow().audio {
            AudioBackend::Auto => 0,
            AudioBackend::Alsa => 1,
            AudioBackend::Pulse => 2,
        };
        audio.set_selected(selected);
        audio.set_valign(gtk::Align::Center);
        audio.connect_selected_notify(move |d| {
            st.borrow_mut().audio = match d.selected() {
                1 => AudioBackend::Alsa,
                2 => AudioBackend::Pulse,
                _ => AudioBackend::Auto,
            };
            save(&st, &profile, &status);
        });
        body.append(&card_row("Audio backend", &audio));
    }
    body.append(&hint("Performance and audio apply on next launch."));
    body
}

fn appearance_page(status: &gtk::Label) -> gtk::Box {
    let body = page();
    body.append(&caption("Custom CSS"));
    body.append(&hint(
        "Custom CSS (~/.config/ruwa/custom.css), applied after mods. Reload the page to apply edits.",
    ));
    {
        let status = status.clone();
        let view = gtk::TextView::new();
        view.set_monospace(true);
        view.set_wrap_mode(gtk::WrapMode::None);
        view.set_hexpand(true);
        view.set_vexpand(true);
        let existing = settings::read_custom_css();
        if existing.trim().is_empty() {
            view.buffer().set_text(settings::CUSTOM_CSS_SEED);
        } else {
            view.buffer().set_text(&existing);
        }
        let scroll = gtk::ScrolledWindow::new();
        scroll.set_min_content_height(220);
        scroll.set_hexpand(true);
        scroll.set_vexpand(true);
        scroll.set_child(Some(&view));
        body.append(&card_wrap(&scroll));
        let btn_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let save_css = gtk::Button::with_label("Save custom CSS");
        save_css.set_hexpand(true);
        save_css.set_css_classes(&["suggested-action", "pill"]);
        save_css.connect_clicked(move |_| {
            let buf = view.buffer();
            let text = buf
                .text(&buf.start_iter(), &buf.end_iter(), false)
                .to_string();
            match settings::validate_css_braces(&text) {
                Err(e) => {
                    status.set_text(&format!("CSS not saved: {e}"));
                    return;
                }
                Ok(()) => {}
            }
            match settings::write_custom_css(&text) {
                Ok(()) => status.set_text("Custom CSS saved. Reload the page to apply."),
                Err(e) => status.set_text(&format!("Custom CSS save failed: {e}")),
            }
        });
        btn_row.append(&save_css);
        let open_mods = gtk::Button::with_label("Open mods folder");
        open_mods.set_css_classes(&["pill"]);
        open_mods.connect_clicked(move |_| {
            let dir = settings::mods_dir();
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::process::Command::new("xdg-open")
                .arg(&dir)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        });
        btn_row.append(&open_mods);
        body.append(&btn_row);
    }

    body.append(&caption("Mods"));
    body.append(&hint(
        "Each ~/.config/ruwa/mods/<name>/mod.css is appended on launch. Toggle to enable/disable.",
    ));
    {
        let mods = settings::list_mods();
        if mods.is_empty() {
            body.append(&card_wrap(&hint("No mods installed yet.")));
        }
        for m in mods {
            let sw = gtk::Switch::new();
            sw.set_active(m.enabled);
            sw.set_halign(gtk::Align::End);
            sw.set_valign(gtk::Align::Center);
            let name = m.name.clone();
            let status = status.clone();
            sw.connect_state_set(move |_, on| {
                match settings::set_mod_enabled(&name, on) {
                    Ok(()) => status.set_text(&format!(
                        "Mod '{name}' {}. Takes effect on next launch.",
                        if on { "enabled" } else { "disabled" }
                    )),
                    Err(e) => status.set_text(&format!("Mod toggle failed: {e}")),
                }
                gtk::glib::Propagation::Proceed
            });
            body.append(&card_row(&format!("mod: {}", m.name), &sw));
        }
    }
    body
}

fn system_page(
    state: &Rc<RefCell<Prefs>>,
    profile: &Rc<std::path::PathBuf>,
    status: &gtk::Label,
) -> gtk::Box {
    let body = page();
    body.append(&caption("Startup"));
    {
        let status = status.clone();
        let st = state.clone();
        let autostart = gtk::Switch::new();
        autostart.set_active(settings::is_autostart());
        autostart.set_halign(gtk::Align::End);
        autostart.set_valign(gtk::Align::Center);
        autostart.connect_state_set(move |_, on| {
            let exe = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("ruwa")))
                .unwrap_or_else(|| std::path::PathBuf::from("ruwa"));
            let minimized = st.borrow().start_minimized;
            match settings::set_autostart(on, &exe, minimized) {
                Ok(()) => status.set_text("Autostart updated."),
                Err(e) => status.set_text(&format!("Autostart failed: {e}")),
            }
            gtk::glib::Propagation::Proceed
        });
        body.append(&card_row("Start on login", &autostart));
    }

    body.append(&caption("Account"));
    {
        let status = status.clone();
        let profile = profile.clone();
        let logout = gtk::Button::with_label("Log out (clear site data)");
        logout.set_hexpand(true);
        logout.set_css_classes(&["destructive-action", "pill"]);
        logout.connect_clicked(move |_| match settings::clear_site_data(&profile) {
            Ok(n) => status.set_text(&format!(
                "Cleared {} entries. Log in again on next launch.",
                n.len()
            )),
            Err(e) => status.set_text(&format!("Clear failed: {e}")),
        });
        body.append(&card_wrap(&logout));
    }

    body.append(&caption("About"));
    {
        let open = gtk::Button::with_label("Open Ruwa");
        open.set_hexpand(true);
        open.set_css_classes(&["pill"]);
        open.connect_clicked(move |_| {
            if let Some(bin) = settings::sibling_binary("ruwa") {
                let _ = std::process::Command::new(bin)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
            }
        });
        body.append(&card_wrap(&open));
    }
    body
}

fn save(state: &Rc<RefCell<Prefs>>, profile: &std::path::Path, status: &gtk::Label) {
    match settings::write_prefs(profile, &state.borrow()) {
        Ok(()) => status.set_text("Saved. Takes effect on next launch."),
        Err(e) => status.set_text(&format!("Save failed: {e}")),
    }
}
