#[path = "../settings.rs"]
mod settings;

use gtk::prelude::*;
use settings::{AudioBackend, Prefs};
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

fn build_ui(app: &gtk::Application) {
    let profile = std::rc::Rc::new(settings::profile_dir());
    let state = Rc::new(RefCell::new(settings::read_prefs(&profile)));

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Ruwa Settings")
        .default_width(460)
        .default_height(520)
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

    {
        let st = state.clone();
        let status = status.clone();
        let profile = profile.clone();
        let gpu = gtk::Switch::new();
        gpu.set_active(st.borrow().gpu_accel);
        gpu.set_halign(gtk::Align::End);
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
        dl.connect_changed(move |e| {
            st.borrow_mut().download_dir = e.text().to_string();
            save(&st, &profile, &status);
        });
        body.append(&section_label("Downloads"));
        body.append(&settings_row("Download folder", &dl));
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
        scroll.set_min_content_height(220);
        scroll.set_hexpand(true);
        scroll.set_child(Some(&view));
        let save = gtk::Button::with_label("Save custom CSS");
        save.connect_clicked(move |_| {
            let buf = view.buffer();
            let text = buf
                .text(&buf.start_iter(), &buf.end_iter(), false)
                .to_string();
            match settings::write_custom_css(&text) {
                Ok(()) => status.set_text("Custom CSS saved. Takes effect on next launch."),
                Err(e) => status.set_text(&format!("Custom CSS save failed: {e}")),
            }
        });
        body.append(&section_label("Appearance"));
        let hint = gtk::Label::new(Some("Custom CSS (~/.config/ruwa/custom.css), applied after mods."));
        hint.set_xalign(0.0);
        body.append(&hint);
        body.append(&scroll);
        body.append(&save);
    }

    {
        let status = status.clone();
        let autostart = gtk::Switch::new();
        autostart.set_active(settings::is_autostart());
        autostart.set_halign(gtk::Align::End);
        autostart.connect_state_set(move |_, on| {
            let exe = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("ruwa")))
                .unwrap_or_else(|| std::path::PathBuf::from("ruwa"));
            match settings::set_autostart(on, &exe) {
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
