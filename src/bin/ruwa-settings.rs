#[path = "../settings.rs"]
mod settings;

use gtk::prelude::*;
use settings::{AudioBackend, DisplayBackend, Prefs};
use std::cell::RefCell;
use std::rc::Rc;

fn section_label(text: &str) -> gtk::Label {
    let label = gtk::Label::new(None);
    label.set_markup(&format!("<b>{text}</b>"));
    label.set_xalign(0.0);
    label
}

fn settings_row(label_text: &str, control: &impl IsA<gtk::Widget>) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let label = gtk::Label::new(Some(label_text));
    label.set_hexpand(true);
    label.set_xalign(0.0);
    row.append(&label);
    row.append(control);
    row
}

fn main() {
    let app = gtk::Application::builder()
        .application_id("com.rused.RuwaSettings")
        .build();
    app.connect_activate(build_ui);
    app.run();
}

fn switch_row(
    body: &gtk::Box,
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
    body.append(&settings_row(label, &sw));
}

fn build_ui(app: &gtk::Application) {
    let profile = std::rc::Rc::new(settings::profile_dir());
    let state = Rc::new(RefCell::new(settings::read_prefs(&profile)));

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Ruwa Settings")
        .default_width(480)
        .default_height(640)
        .build();

    let header = gtk::HeaderBar::new();
    header.set_title_widget(Some(&gtk::Label::new(Some("Ruwa Settings"))));
    window.set_titlebar(Some(&header));

    let body = gtk::Box::new(gtk::Orientation::Vertical, 10);
    body.set_margin_top(16);
    body.set_margin_bottom(16);
    body.set_margin_start(16);
    body.set_margin_end(16);

    let status = gtk::Label::new(Some("Changes apply on next launch."));
    status.set_xalign(0.0);
    status.set_wrap(true);

    body.append(&section_label("Behavior"));
    switch_row(
        &body,
        &state,
        &profile,
        &status,
        "Keep running in tray when window closes",
        |p| p.close_to_tray,
        |p, v| p.close_to_tray = v,
    );
    switch_row(
        &body,
        &state,
        &profile,
        &status,
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
        display.connect_selected_notify(move |d| {
            st.borrow_mut().display = match d.selected() {
                1 => DisplayBackend::Wayland,
                2 => DisplayBackend::X11,
                _ => DisplayBackend::Auto,
            };
            save(&st, &profile, &status);
        });
        body.append(&settings_row("Display backend", &display));
    }

    {
        let st = state.clone();
        let status = status.clone();
        let profile = profile.clone();
        let gpu = gtk::Switch::new();
        gpu.set_active(st.borrow().gpu_accel);
        gpu.set_halign(gtk::Align::End);
        gpu.set_valign(gtk::Align::Center);
        gpu.connect_state_set(move |_, on| {
            st.borrow_mut().gpu_accel = on;
            save(&st, &profile, &status);
            gtk::glib::Propagation::Proceed
        });
        body.append(&section_label("Performance"));
        body.append(&settings_row("GPU acceleration (WebRender)", &gpu));
    }

    {
        let st = state.clone();
        let status = status.clone();
        let profile = profile.clone();
        let vaapi = gtk::Switch::new();
        vaapi.set_active(st.borrow().vaapi);
        vaapi.set_halign(gtk::Align::End);
        vaapi.set_valign(gtk::Align::Center);
        vaapi.connect_state_set(move |_, on| {
            st.borrow_mut().vaapi = on;
            save(&st, &profile, &status);
            gtk::glib::Propagation::Proceed
        });
        body.append(&settings_row("Hardware video decoding (VA-API)", &vaapi));
    }

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
        audio.connect_selected_notify(move |d| {
            st.borrow_mut().audio = match d.selected() {
                1 => AudioBackend::Alsa,
                2 => AudioBackend::Pulse,
                _ => AudioBackend::Auto,
            };
            save(&st, &profile, &status);
        });
        body.append(&section_label("Audio"));
        body.append(&settings_row("Audio backend", &audio));
    }

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
        body.append(&section_label("Downloads"));
        let dl_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        dl_row.append(&dl);
        dl_row.append(&browse);
        let dl_label = gtk::Label::new(Some("Download folder"));
        dl_label.set_hexpand(true);
        dl_label.set_xalign(0.0);
        let dl_wrap = gtk::Box::new(gtk::Orientation::Vertical, 4);
        dl_wrap.append(&dl_label);
        dl_wrap.append(&dl_row);
        dl_wrap.append(&eff);
        body.append(&dl_wrap);
    }

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
        scroll.set_min_content_height(200);
        scroll.set_hexpand(true);
        scroll.set_child(Some(&view));
        let btn_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let save_css = gtk::Button::with_label("Save custom CSS");
        save_css.set_hexpand(true);
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
        body.append(&section_label("Appearance"));
        let hint = gtk::Label::new(Some(
            "Custom CSS (~/.config/ruwa/custom.css), applied after mods.",
        ));
        hint.set_xalign(0.0);
        hint.set_wrap(true);
        body.append(&hint);
        body.append(&scroll);
        body.append(&btn_row);
    }

    {
        body.append(&section_label("Mods"));
        let mods_hint = gtk::Label::new(Some(
            "Each ~/.config/ruwa/mods/<name>/mod.css is appended on launch. Toggle to enable/disable.",
        ));
        mods_hint.set_xalign(0.0);
        mods_hint.set_wrap(true);
        body.append(&mods_hint);
        let mods = settings::list_mods();
        if mods.is_empty() {
            let empty = gtk::Label::new(Some("No mods installed yet."));
            empty.set_xalign(0.0);
            body.append(&empty);
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
            body.append(&settings_row(&format!("mod: {}", m.name), &sw));
        }
    }

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
        body.append(&section_label("System"));
        body.append(&settings_row("Start on login", &autostart));
    }

    {
        let status = status.clone();
        let profile = profile.clone();
        let logout = gtk::Button::with_label("Log out (clear site data)");
        logout.connect_clicked(move |_| match settings::clear_site_data(&profile) {
            Ok(n) => status.set_text(&format!(
                "Cleared {} entries. Log in again on next launch.",
                n.len()
            )),
            Err(e) => status.set_text(&format!("Clear failed: {e}")),
        });
        body.append(&section_label("Account"));
        body.append(&logout);
    }

    {
        let open = gtk::Button::with_label("Open Ruwa");
        open.connect_clicked(move |_| {
            if let Some(bin) = settings::sibling_binary("ruwa") {
                let _ = std::process::Command::new(bin)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
            }
        });
        body.append(&open);
    }

    body.append(&status);

    let scroll = gtk::ScrolledWindow::new();
    scroll.set_child(Some(&body));
    window.set_child(Some(&scroll));
    window.present();
}

fn save(state: &Rc<RefCell<Prefs>>, profile: &std::path::Path, status: &gtk::Label) {
    match settings::write_prefs(profile, &state.borrow()) {
        Ok(()) => status.set_text("Saved. Takes effect on next launch."),
        Err(e) => status.set_text(&format!("Save failed: {e}")),
    }
}
