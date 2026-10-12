#[allow(dead_code)]
mod settings;

use settings::{AppConfig, DisplayBackend};
use std::path::PathBuf;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, TrayIconEvent};
use tauri::webview::{DownloadEvent, NewWindowResponse, PermissionKind, PermissionResponse};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

const WHATSAPP_HOME: &str = "https://web.whatsapp.com/";
const WINDOW_LABEL: &str = "main";
const CHROME_UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";

const MODS_REGISTRY_JS: &str = include_str!(concat!(env!("OUT_DIR"), "/ruwa_mods_registry.js"));
const BASE_CSS: &str = include_str!("../assets/base.css");
const TRAY_ICON_PNG: &[u8] = include_bytes!("../data/icons/ruwa.png");

fn verbose() -> bool {
    matches!(
        std::env::var("RUWA_VERBOSE")
            .or_else(|_| std::env::var("RUWA_LOG"))
            .as_deref(),
        Ok("1") | Ok("debug") | Ok("verbose") | Ok("trace")
    )
}

fn vlog(msg: &str) {
    if verbose() {
        eprintln!("ruwa: {msg}");
    }
}

fn whatsapp_host(host: &str) -> bool {
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    for base in ["whatsapp.com", "whatsapp.net", "wa.me"] {
        if host == base || host.ends_with(&format!(".{base}")) {
            return true;
        }
    }
    false
}

fn navigation_allowed(url: &tauri::Url) -> bool {
    match url.scheme() {
        "http" | "https" => url.host_str().is_some_and(whatsapp_host),
        "about" | "blob" | "data" | "tauri" => true,
        _ => false,
    }
}

fn permission_response(url: &tauri::Url, kind: PermissionKind) -> PermissionResponse {
    let media = matches!(
        kind,
        PermissionKind::Microphone
            | PermissionKind::Camera
            | PermissionKind::Notifications
            | PermissionKind::DisplayCapture
    );
    if !media {
        return PermissionResponse::Default;
    }
    if navigation_allowed(url) {
        PermissionResponse::Allow
    } else {
        PermissionResponse::Deny
    }
}

fn js_string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn build_init_script(base_css: &str, custom_css: &str, enabled_mods: &[String]) -> String {
    let css = format!("{base_css}\n{}", settings::strip_moz_document(custom_css));
    let enabled = enabled_mods
        .iter()
        .map(|s| js_string_literal(s))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"(function() {{
  try {{
    if (!/^https:\/\/([a-z0-9-]+\.)*(whatsapp\.com|whatsapp\.net|wa\.me)(\/|$)/.test(window.location.href)) return;
    var css = {css_lit};
    if (css.trim()) {{
      var el = document.getElementById("ruwa-style");
      if (!el) {{
        el = document.createElement("style");
        el.id = "ruwa-style";
        (document.head || document.documentElement).appendChild(el);
      }}
      el.textContent = css;
    }}
  }} catch (e) {{ console.warn("[ruwa] css inject failed", e); }}
  try {{
    var enabled = [{enabled}];
    var seen = {{}};
    for (var i = 0; i < enabled.length; i++) {{
      var id = enabled[i];
      if (!id || seen[id]) continue;
      seen[id] = true;
      var fn = (typeof RUWA_MODS !== "undefined") ? RUWA_MODS[id] : null;
      if (typeof fn !== "function") continue;
      try {{ fn(); }} catch (e) {{ console.warn("[ruwa-mods] mod failed: " + id, e); }}
    }}
  }} catch (e) {{ console.warn("[ruwa-mods] boot failed", e); }}
}})();"#,
        css_lit = js_string_literal(&css),
        enabled = enabled
    )
}

fn full_init_script(cfg: &AppConfig) -> String {
    let custom = settings::read_custom_css();
    format!(
        "{MODS_REGISTRY_JS}\n{}",
        build_init_script(BASE_CSS, &custom, &cfg.mods_enabled)
    )
}

fn apply_display_backend_env(backend: DisplayBackend) {
    match backend {
        DisplayBackend::X11 => {
            std::env::remove_var("WAYLAND_DISPLAY");
            std::env::set_var("GDK_BACKEND", "x11");
        }
        DisplayBackend::Wayland => {
            std::env::set_var("GDK_BACKEND", "wayland");
        }
        DisplayBackend::Auto => {}
    }
}

fn resolve_backend(cfg: &AppConfig) -> DisplayBackend {
    if let Ok(s) = std::env::var("RUWA_BACKEND") {
        let b = DisplayBackend::from_str(&s);
        if b != DisplayBackend::Auto {
            return b;
        }
        if !s.trim().is_empty() {
            return DisplayBackend::Auto;
        }
    }
    cfg.display
}

fn open_external(app: &tauri::AppHandle, url: &str) {
    vlog(&format!("opening externally: {url}"));
    if let Err(e) = app.opener().open_url(url, None::<&str>) {
        eprintln!("ruwa: cannot open URL {url}: {e:?}");
    }
}

fn show_main(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window(WINDOW_LABEL) {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
    }
}

fn reload_main(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window(WINDOW_LABEL) {
        vlog("reloading main window");
        let _ = win.eval("window.location.reload();");
    }
}

fn open_settings() {
    if let Some(bin) = settings::sibling_binary("ruwa-settings") {
        let _ = std::process::Command::new(bin)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    } else {
        eprintln!("ruwa: ruwa-settings not found next to ruwa binary");
    }
}

#[cfg(target_os = "linux")]
fn enable_linux_media_capture(window: &tauri::WebviewWindow) {
    let result = window.with_webview(|platform| {
        use webkit2gtk::{SettingsExt, WebViewExt};
        let view = platform.inner();
        if let Some(s) = WebViewExt::settings(&view) {
            s.set_enable_media_stream(true);
        }
    });
    if let Err(e) = result {
        eprintln!("ruwa: cannot enable WebKitGTK media capture: {e:?}");
    }
}

#[cfg(not(target_os = "linux"))]
fn enable_linux_media_capture(_window: &tauri::WebviewWindow) {}

fn build_tray(app: &tauri::AppHandle) -> Option<tauri::tray::TrayIcon> {
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| build_tray_inner(app)));
    match result {
        Ok(Ok(t)) => Some(t),
        Ok(Err(e)) => {
            eprintln!("ruwa: tray unavailable ({e:?}), continuing without it");
            None
        }
        Err(_) => {
            eprintln!("ruwa: tray backend unavailable, continuing without it");
            None
        }
    }
}

fn build_tray_inner(app: &tauri::AppHandle) -> tauri::Result<tauri::tray::TrayIcon> {
    let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let reload = MenuItem::with_id(app, "reload", "Reload", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show, &reload, &settings_item, &sep, &quit])?;
    let icon = tauri::image::Image::from_bytes(TRAY_ICON_PNG)?;
    tauri::tray::TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("Ruwa")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main(app),
            "reload" => reload_main(app),
            "settings" => open_settings(),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        })
        .build(app)
}

fn usage() -> &'static str {
    "ruwa — lightweight WhatsApp desktop client\n\nUsage: ruwa [options]\n\nOptions:\n  --show        Focus the running window (default for second instance)\n  --quit        Quit the running instance and exit\n  --minimized   Start minimized to tray (no window)\n  --reload      Ask the running instance to reload the page\n  -h, --help    Show this help\n\nEnv:\n  RUWA_BACKEND   auto|wayland|x11 (overrides Settings display backend)\n  RUWA_VERBOSE=1 Extra logging\n"
}

#[derive(Debug, Default)]
struct Cli {
    minimized: bool,
    quit: bool,
    reload: bool,
}

fn parse_cli(args: &[String]) -> Result<Cli, String> {
    let mut cli = Cli::default();
    for a in args {
        match a.as_str() {
            "--minimized" => cli.minimized = true,
            "--quit" => cli.quit = true,
            "--reload" => cli.reload = true,
            "--show" => {}
            "-h" | "--help" => {
                print!("{}", usage());
                std::process::exit(0);
            }
            _ if a.starts_with('-') => return Err(format!("ruwa: unknown option '{a}'")),
            _ => return Err(format!("ruwa: unexpected argument '{a}'")),
        }
    }
    Ok(cli)
}

fn main() {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    let cli = match parse_cli(&raw_args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            eprint!("{}", usage());
            std::process::exit(2);
        }
    };
    if cli.quit && cli.minimized {
        eprintln!("ruwa: --quit cannot be combined with --minimized");
        std::process::exit(2);
    }

    let cfg = settings::read_config();
    vlog(&format!(
        "config: close_to_tray={} start_minimized={} display={:?} mods={:?}",
        cfg.close_to_tray, cfg.start_minimized, cfg.display, cfg.mods_enabled
    ));
    apply_display_backend_env(resolve_backend(&cfg));

    let start_hidden = cli.minimized || cfg.start_minimized;
    let close_to_tray = cfg.close_to_tray;
    let quit_if_first = cli.quit;
    let init_script = full_init_script(&cfg);
    let data_dir = settings::webview_data_dir();
    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        eprintln!("ruwa: cannot create data dir {}: {e:?}", data_dir.display());
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            vlog(&format!("second instance: {argv:?}"));
            if argv.iter().any(|a| a == "--quit") {
                app.exit(0);
            } else if argv.iter().any(|a| a == "--reload") {
                reload_main(app);
                show_main(app);
            } else {
                show_main(app);
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            if quit_if_first {
                eprintln!("ruwa: not running");
                std::process::exit(0);
            }
            let nav_app = app.handle().clone();
            let win_app = app.handle().clone();
            let title_tray = build_tray(app.handle());
            let window = WebviewWindowBuilder::new(
                app,
                WINDOW_LABEL,
                WebviewUrl::External(
                    WHATSAPP_HOME
                        .parse()
                        .expect("WHATSAPP_HOME must be a valid URL"),
                ),
            )
            .title("Ruwa")
            .inner_size(1280.0, 800.0)
            .min_inner_size(800.0, 600.0)
            .center()
            .visible(!start_hidden)
            .user_agent(CHROME_UA)
            .data_directory(data_dir)
            .initialization_script(init_script)
            .on_navigation(move |url| {
                if navigation_allowed(url) {
                    true
                } else {
                    open_external(&nav_app, url.as_str());
                    false
                }
            })
            .on_new_window(move |url, _features| {
                if navigation_allowed(&url) {
                    if let Some(win) = win_app.get_webview_window(WINDOW_LABEL) {
                        show_main(&win_app);
                        if let Err(e) = win.navigate(url) {
                            eprintln!("ruwa: same-host popup navigate failed: {e:?}");
                        }
                    }
                    NewWindowResponse::Deny
                } else {
                    open_external(&win_app, url.as_str());
                    NewWindowResponse::Deny
                }
            })
            .on_download(move |_webview, event| {
                match event {
                    DownloadEvent::Requested { url, destination } => {
                        vlog(&format!("download requested: {url} -> {}", destination.display()));
                        if let Some(home) = std::env::var_os("HOME") {
                            let dir = PathBuf::from(home).join("Downloads");
                            let _ = std::fs::create_dir_all(&dir);
                            if let Some(name) = url
                                .path_segments()
                                .and_then(|mut s| s.next_back())
                                .filter(|s| !s.is_empty())
                            {
                                *destination = dir.join(name);
                            } else {
                                *destination = dir.join("whatsapp-download");
                            }
                        }
                        true
                    }
                    DownloadEvent::Finished { url, path, success } => {
                        vlog(&format!(
                            "download finished: {url} -> {} success={success}",
                            path
                                .as_ref()
                                .map(|p| p.display().to_string())
                                .unwrap_or_default()
                        ));
                        true
                    }
                    _ => true,
                }
            })
            .on_permission_request(move |webview, kind| {
                match webview.url() {
                    Ok(url) => permission_response(&url, kind),
                    Err(_) => PermissionResponse::Deny,
                }
            })
            .on_document_title_changed(move |_win, title| {
                let tip = if title.trim().is_empty() {
                    String::from("Ruwa")
                } else {
                    format!("Ruwa — {title}")
                };
                if let Some(tray) = title_tray.as_ref() {
                    let _ = tray.set_tooltip(Some(tip));
                }
            })
            .build()?;

            let win = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    if close_to_tray {
                        api.prevent_close();
                        let _ = win.hide();
                    }
                }
            });

            enable_linux_media_capture(&window);
            if start_hidden {
                println!("ruwa: starting minimized to tray");
            }
            Ok(())
        })
        .run(tauri::generate_context!("src-tauri/tauri.conf.json"))
        .expect("ruwa: failed to run Tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whatsapp_hosts_allowed() {
        for h in [
            "web.whatsapp.com",
            "whatsapp.com",
            "wa.me",
            "mmg.whatsapp.net",
            "sub.wa.me",
            "WEB.WHATSAPP.COM",
        ] {
            assert!(whatsapp_host(h), "{h}");
        }
        for h in [
            "evilwhatsapp.com",
            "whatsapp.com.evil.com",
            "facebook.com",
            "example.com",
            "",
        ] {
            assert!(!whatsapp_host(h), "{h}");
        }
    }

    #[test]
    fn navigation_policy() {
        let wa: tauri::Url = "https://web.whatsapp.com/".parse().unwrap();
        assert!(navigation_allowed(&wa));
        let media: tauri::Url = "https://mmg.whatsapp.net/d/f".parse().unwrap();
        assert!(navigation_allowed(&media));
        let ext: tauri::Url = "https://example.com/x".parse().unwrap();
        assert!(!navigation_allowed(&ext));
        let mailto: tauri::Url = "mailto:a@b.c".parse().unwrap();
        assert!(!navigation_allowed(&mailto));
        let blank: tauri::Url = "about:blank".parse().unwrap();
        assert!(navigation_allowed(&blank));
    }

    #[test]
    fn permission_policy() {
        let wa: tauri::Url = "https://web.whatsapp.com/".parse().unwrap();
        let ext: tauri::Url = "https://example.com/".parse().unwrap();
        assert_eq!(
            permission_response(&wa, PermissionKind::Microphone),
            PermissionResponse::Allow
        );
        assert_eq!(
            permission_response(&wa, PermissionKind::Camera),
            PermissionResponse::Allow
        );
        assert_eq!(
            permission_response(&wa, PermissionKind::Notifications),
            PermissionResponse::Allow
        );
        assert_eq!(
            permission_response(&wa, PermissionKind::DisplayCapture),
            PermissionResponse::Allow
        );
        assert_eq!(
            permission_response(&ext, PermissionKind::Microphone),
            PermissionResponse::Deny
        );
        assert_eq!(
            permission_response(&ext, PermissionKind::Geolocation),
            PermissionResponse::Default
        );
    }

    #[test]
    fn init_script_embeds_css_and_mods() {
        let script = build_init_script("#app { color: red; }", "body { margin: 0; }", &[
            String::from("uaaa-decryptor"),
        ]);
        assert!(script.contains("#app { color: red; }"));
        assert!(script.contains("body { margin: 0; }"));
        assert!(script.contains("uaaa-decryptor"));
        assert!(script.contains("RUWA_MODS"));
        assert!(script.contains("ruwa-style"));
    }

    #[test]
    fn init_script_escapes_css_quotes() {
        let script = build_init_script("a::after { content: \"x\"; }", "", &[]);
        assert!(!script.contains("content: \"x\""));
        assert!(script.contains("content: \\\"x\\\""));
    }

    #[test]
    fn cli_parsing() {
        let cli = parse_cli(&["--minimized".to_string()]).unwrap();
        assert!(cli.minimized);
        assert!(parse_cli(&["--nope".to_string()]).is_err());
        assert!(parse_cli(&["https://x".to_string()]).is_err());
    }
}
