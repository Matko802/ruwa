use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioBackend {
    Auto,
    Alsa,
    Pulse,
}

impl AudioBackend {
    pub fn as_pref(&self) -> Option<&'static str> {
        match self {
            AudioBackend::Auto => None,
            AudioBackend::Alsa => Some("alsa"),
            AudioBackend::Pulse => Some("pulse"),
        }
    }

    pub fn from_pref(s: &str) -> Self {
        match s {
            "alsa" => AudioBackend::Alsa,
            "pulse" => AudioBackend::Pulse,
            _ => AudioBackend::Auto,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            AudioBackend::Auto => "Automatic",
            AudioBackend::Alsa => "ALSA",
            AudioBackend::Pulse => "PulseAudio / PipeWire",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayBackend {
    Auto,
    Wayland,
    X11,
}

impl DisplayBackend {
    pub fn label(&self) -> &'static str {
        match self {
            DisplayBackend::Auto => "Automatic (prefer Wayland)",
            DisplayBackend::Wayland => "Wayland",
            DisplayBackend::X11 => "X11 / XWayland",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            DisplayBackend::Auto => "auto",
            DisplayBackend::Wayland => "wayland",
            DisplayBackend::X11 => "x11",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "wayland" => DisplayBackend::Wayland,
            "x11" | "xwayland" => DisplayBackend::X11,
            _ => DisplayBackend::Auto,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Prefs {
    pub gpu_accel: bool,
    pub vaapi: bool,
    pub audio: AudioBackend,
    pub download_dir: String,
    pub close_to_tray: bool,
    pub start_minimized: bool,
    pub display: DisplayBackend,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            gpu_accel: true,
            vaapi: true,
            audio: AudioBackend::Auto,
            download_dir: String::new(),
            close_to_tray: true,
            start_minimized: false,
            display: DisplayBackend::Auto,
        }
    }
}

pub fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

pub fn config_dir() -> PathBuf {
    if let Ok(p) = std::env::var("XDG_CONFIG_HOME") {
        if !p.trim().is_empty() {
            return PathBuf::from(p).join("ruwa");
        }
    }
    home_dir().join(".config").join("ruwa")
}

pub fn custom_css_path() -> PathBuf {
    config_dir().join("custom.css")
}

pub const CUSTOM_CSS_SEED: &str = "/* Ruwa custom CSS: applied to WhatsApp Web on every launch.\n   Reload the page to apply edits. */\n\n@-moz-document domain(web.whatsapp.com), domain(whatsapp.com), domain(whatsapp.net) {\n}\n";

pub fn read_custom_css() -> String {
    std::fs::read_to_string(custom_css_path()).unwrap_or_default()
}

pub fn write_custom_css(css: &str) -> io::Result<()> {
    let path = custom_css_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, css)
}

pub fn profile_dir() -> PathBuf {
    if let Ok(p) = std::env::var("RUWA_PROFILE") {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_dir().join(".local").join("share"));
    base.join("ruwa").join("profile")
}

pub fn user_js_path(profile: &Path) -> PathBuf {
    profile.join("user.js")
}

fn bool_pref_line(content: &str, key: &str) -> Option<bool> {
    for line in content.lines() {
        let t = line.trim();
        let rest = t
            .strip_prefix("user_pref(")?
            .trim_end_matches([';', ')'])
            .trim();
        let (k, v) = rest.split_once(',')?;
        if k.trim().trim_matches('"') == key {
            return match v.trim() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            };
        }
    }
    None
}

fn string_pref_line(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let t = line.trim();
        let rest = t
            .strip_prefix("user_pref(")?
            .trim_end_matches([';', ')'])
            .trim();
        let (k, v) = rest.split_once(',')?;
        if k.trim().trim_matches('"') == key {
            let v = v.trim();
            let unquoted = v.strip_prefix('"').and_then(|s| s.strip_suffix('"'))?;
            return Some(unquoted.to_string());
        }
    }
    None
}

pub fn default_download_dir() -> PathBuf {
    home_dir().join("Downloads")
}

pub fn expand_path(raw: &str) -> PathBuf {
    let s = raw.trim();
    if s.is_empty() {
        return default_download_dir();
    }
    if s == "~" {
        return home_dir();
    }
    if let Some(rest) = s.strip_prefix("~/") {
        return home_dir().join(rest);
    }
    if let Some(rest) = s.strip_prefix("$HOME/") {
        return home_dir().join(rest);
    }
    if s == "$HOME" {
        return home_dir();
    }
    PathBuf::from(s)
}

pub fn effective_download_dir(p: &Prefs) -> PathBuf {
    if p.download_dir.trim().is_empty() {
        default_download_dir()
    } else {
        expand_path(&p.download_dir)
    }
}

pub fn read_prefs(profile: &Path) -> Prefs {
    let mut out = Prefs::default();
    let Ok(content) = std::fs::read_to_string(user_js_path(profile)) else {
        return out;
    };
    if let Some(v) = bool_pref_line(&content, "gfx.webrender.all") {
        out.gpu_accel = v;
    }
    if let Some(v) = bool_pref_line(&content, "media.ffmpeg.vaapi.enabled") {
        out.vaapi = v;
    }
    if let Some(v) = string_pref_line(&content, "media.cubeb.backend") {
        out.audio = AudioBackend::from_pref(&v);
    }
    if let Some(v) = string_pref_line(&content, "browser.download.dir") {
        if v != default_download_dir().to_string_lossy() {
            out.download_dir = v;
        }
    }
    if let Some(v) = bool_pref_line(&content, "ruwa.close_to_tray") {
        out.close_to_tray = v;
    }
    if let Some(v) = bool_pref_line(&content, "ruwa.start_minimized") {
        out.start_minimized = v;
    }
    if let Some(v) = string_pref_line(&content, "ruwa.display.backend") {
        out.display = DisplayBackend::from_str(&v);
    }
    out
}

fn set_pref_line(lines: &mut Vec<String>, key: &str, value: &str) {
    let needle = format!("\"{key}\"");
    let replacement = format!("user_pref(\"{key}\", {value});");
    for line in lines.iter_mut() {
        let t = line.trim();
        if t.starts_with("user_pref(") && t.contains(&needle) {
            *line = replacement.clone();
            return;
        }
    }
    lines.push(replacement);
}

fn bool_str(b: bool) -> &'static str {
    if b {
        "true"
    } else {
        "false"
    }
}

fn quoted(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

pub fn write_prefs(profile: &Path, prefs: &Prefs) -> io::Result<()> {
    std::fs::create_dir_all(profile)?;
    let path = user_js_path(profile);
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let mut lines: Vec<String> = content.lines().map(str::to_string).collect();
    set_pref_line(&mut lines, "gfx.webrender.all", bool_str(prefs.gpu_accel));
    set_pref_line(
        &mut lines,
        "layers.acceleration.force-enabled",
        bool_str(prefs.gpu_accel),
    );
    set_pref_line(
        &mut lines,
        "media.ffmpeg.vaapi.enabled",
        bool_str(prefs.vaapi),
    );
    set_pref_line(
        &mut lines,
        "media.hardware-video-decoding.enabled",
        bool_str(prefs.vaapi),
    );
    match prefs.audio.as_pref() {
        Some(backend) => set_pref_line(&mut lines, "media.cubeb.backend", &quoted(backend)),
        None => lines.retain(|l| {
            !(l.trim().starts_with("user_pref(") && l.contains("\"media.cubeb.backend\""))
        }),
    }
    let dl = effective_download_dir(prefs);
    let _ = std::fs::create_dir_all(&dl);
    set_pref_line(
        &mut lines,
        "browser.download.dir",
        &quoted(&dl.to_string_lossy()),
    );
    set_pref_line(&mut lines, "browser.download.folderList", "2");
    set_pref_line(&mut lines, "browser.download.useDownloadDir", "true");
    set_pref_line(
        &mut lines,
        "browser.download.always_ask_before_handling_new_types",
        "false",
    );
    set_pref_line(
        &mut lines,
        "browser.download.start_downloads_in_tmp_dir",
        "false",
    );
    set_pref_line(
        &mut lines,
        "browser.download.allow_save_from_sandboxed_iframes",
        "true",
    );
    set_pref_line(
        &mut lines,
        "browser.download.open_pdf_attachments_inline",
        "false",
    );
    set_pref_line(
        &mut lines,
        "ruwa.close_to_tray",
        bool_str(prefs.close_to_tray),
    );
    set_pref_line(
        &mut lines,
        "ruwa.start_minimized",
        bool_str(prefs.start_minimized),
    );
    set_pref_line(
        &mut lines,
        "ruwa.display.backend",
        &quoted(prefs.display.as_str()),
    );
    let mut out = lines.join("\n");
    out.push('\n');
    std::fs::write(path, out)
}

pub fn autostart_path() -> PathBuf {
    if let Ok(p) = std::env::var("XDG_CONFIG_HOME") {
        if !p.trim().is_empty() {
            return PathBuf::from(p).join("autostart").join("ruwa.desktop");
        }
    }
    home_dir()
        .join(".config")
        .join("autostart")
        .join("ruwa.desktop")
}

pub fn desktop_entry(exec: &Path, minimized: bool) -> String {
    let exec_line = if minimized {
        format!("{} --minimized", exec.display())
    } else {
        format!("{}", exec.display())
    };
    format!(
        "[Desktop Entry]\nType=Application\nName=Ruwa\nGenericName=WhatsApp Client\nComment=Lightweight WhatsApp desktop client\nTryExec={}\nExec={} \nIcon=ruwa\nTerminal=false\nCategories=Network;InstantMessaging;\nStartupWMClass=Ruwa\nStartupNotify=true\nX-GNOME-Autostart-enabled=true\n",
        exec.display(),
        exec_line
    )
}

pub fn set_autostart(enable: bool, exe: &Path, minimized: bool) -> io::Result<()> {
    let path = autostart_path();
    if enable {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, desktop_entry(exe, minimized))
    } else {
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

pub fn is_autostart() -> bool {
    autostart_path().is_file()
}

const SITE_DATA_ENTRIES: &[&str] = &[
    "storage",
    "cookies.sqlite",
    "cookies.sqlite-wal",
    "cookies.sqlite-shm",
    "places.sqlite",
    "places.sqlite-wal",
    "places.sqlite-shm",
    "webappsstore.sqlite",
    "favicons.sqlite",
    "favicons.sqlite-wal",
    "formhistory.sqlite",
    "cache2",
    "startupCache",
    "thumbnails",
    "sessionstore.jsonlz4",
    "sessionstore-backups",
    "crashes",
    "minidumps",
    "datareporting",
];

pub fn clear_site_data(profile: &Path) -> io::Result<Vec<String>> {
    let mut removed = Vec::new();
    for entry in SITE_DATA_ENTRIES {
        let p = profile.join(entry);
        let res = if p.is_dir() {
            std::fs::remove_dir_all(&p)
        } else {
            match std::fs::remove_file(&p) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e),
            }
        };
        res?;
        removed.push(entry.to_string());
    }
    Ok(removed)
}

pub fn sibling_binary(name: &str) -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let cand = dir.join(name);
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            let cand = dir.join(name);
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModInfo {
    pub name: String,
    pub enabled: bool,
}

pub fn mods_dir() -> PathBuf {
    if let Ok(p) = std::env::var("XDG_CONFIG_HOME") {
        if !p.trim().is_empty() {
            return PathBuf::from(p).join("ruwa").join("mods");
        }
    }
    home_dir().join(".config").join("ruwa").join("mods")
}

pub fn list_mods() -> Vec<ModInfo> {
    let dir = mods_dir();
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if !path.join("mod.css").is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()).map(String::from) else {
            continue;
        };
        let enabled = !path.join("disabled").exists();
        out.push(ModInfo { name, enabled });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

pub fn set_mod_enabled(name: &str, enabled: bool) -> io::Result<()> {
    if name.contains('/') || name.contains('\0') || name.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid mod name",
        ));
    }
    let marker = mods_dir().join(name).join("disabled");
    if enabled {
        match std::fs::remove_file(&marker) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    } else {
        if let Some(parent) = marker.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(marker, "disabled by Ruwa settings\n")
    }
}

pub fn validate_css_braces(css: &str) -> Result<(), String> {
    let mut depth: i32 = 0;
    let mut in_comment = false;
    let mut in_string: Option<char> = None;
    let chars: Vec<char> = css.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if in_comment {
            if c == '*' && i + 1 < chars.len() && chars[i + 1] == '/' {
                in_comment = false;
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if let Some(q) = in_string {
            if c == '\\' {
                i += 2;
                continue;
            }
            if c == q {
                in_string = None;
            }
            i += 1;
            continue;
        }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            in_comment = true;
            i += 2;
            continue;
        }
        if c == '"' || c == '\'' {
            in_string = Some(c);
            i += 1;
            continue;
        }
        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
            if depth < 0 {
                return Err(String::from("unmatched closing brace '}'"));
            }
        }
        i += 1;
    }
    if in_comment {
        return Err(String::from("unterminated /* comment */"));
    }
    if depth != 0 {
        return Err(format!("unbalanced braces: {depth} unclosed '{{'"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_profile(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("ruwa-test-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn prefs_round_trip() {
        let dir = tmp_profile("prefs");
        let before = Prefs {
            gpu_accel: false,
            vaapi: false,
            audio: AudioBackend::Pulse,
            download_dir: String::from("/tmp/ruwa-dl"),
            close_to_tray: false,
            start_minimized: true,
            display: DisplayBackend::X11,
        };
        write_prefs(&dir, &before).unwrap();
        let mut after = read_prefs(&dir);
        after.download_dir = before.download_dir.clone();
        assert_eq!(after, before);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn prefs_preserve_unmanaged_lines() {
        let dir = tmp_profile("preserve");
        std::fs::write(
            user_js_path(&dir),
            "user_pref(\"some.other.pref\", 42);\nuser_pref(\"gfx.webrender.all\", false);\n",
        )
        .unwrap();
        let mut p = read_prefs(&dir);
        assert!(!p.gpu_accel);
        p.gpu_accel = true;
        write_prefs(&dir, &p).unwrap();
        let content = std::fs::read_to_string(user_js_path(&dir)).unwrap();
        assert!(content.contains("user_pref(\"some.other.pref\", 42);"));
        assert!(content.contains("user_pref(\"gfx.webrender.all\", true);"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn prefs_defaults_on_missing_file() {
        let dir = tmp_profile("missing");
        let p = read_prefs(&dir);
        assert_eq!(p, Prefs::default());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn download_prefs_save_automatically() {
        let dir = tmp_profile("dlfix");
        write_prefs(&dir, &Prefs::default()).unwrap();
        let content = std::fs::read_to_string(user_js_path(&dir)).unwrap();
        assert!(content.contains("browser.download.folderList\", 2);"));
        assert!(content.contains("browser.download.useDownloadDir\", true);"));
        assert!(content.contains("always_ask_before_handling_new_types\", false);"));
        assert!(content.contains("start_downloads_in_tmp_dir\", false);"));
        assert!(content.contains("allow_save_from_sandboxed_iframes\", true);"));
        assert!(content.contains("open_pdf_attachments_inline\", false);"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn custom_css_seed_and_path() {
        assert!(!CUSTOM_CSS_SEED.trim().is_empty());
        assert!(CUSTOM_CSS_SEED.contains("@-moz-document"));
        assert_eq!(
            custom_css_path().file_name().and_then(|s| s.to_str()),
            Some("custom.css")
        );
    }

    #[test]
    fn clear_site_data_removes_known_entries() {
        let dir = tmp_profile("clear");
        std::fs::create_dir_all(dir.join("storage/default")).unwrap();
        std::fs::write(dir.join("cookies.sqlite"), "x").unwrap();
        std::fs::write(dir.join("prefs.js"), "keep").unwrap();
        let removed = clear_site_data(&dir).unwrap();
        assert!(removed.contains(&String::from("storage")));
        assert!(!dir.join("storage").exists());
        assert!(!dir.join("cookies.sqlite").exists());
        assert!(dir.join("prefs.js").is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn profile_override_is_honored() {
        let old = std::env::var("RUWA_PROFILE").ok();
        std::env::set_var("RUWA_PROFILE", "/tmp/ruwa-test-profile");
        assert_eq!(profile_dir(), PathBuf::from("/tmp/ruwa-test-profile"));
        match old {
            Some(v) => std::env::set_var("RUWA_PROFILE", v),
            None => std::env::remove_var("RUWA_PROFILE"),
        }
    }

    #[test]
    fn download_tilde_expansion() {
        let home = home_dir();
        assert_eq!(expand_path("~/Downloads"), home.join("Downloads"));
        assert_eq!(expand_path("~"), home);
        assert_eq!(expand_path("$HOME/docs"), home.join("docs"));
        assert_eq!(expand_path("/tmp/absolute"), PathBuf::from("/tmp/absolute"));
        let p = Prefs {
            download_dir: String::from("~/Music"),
            ..Prefs::default()
        };
        assert_eq!(effective_download_dir(&p), home.join("Music"));
    }

    #[test]
    fn display_backend_round_trip() {
        assert_eq!(DisplayBackend::from_str("wayland"), DisplayBackend::Wayland);
        assert_eq!(DisplayBackend::from_str("X11"), DisplayBackend::X11);
        assert_eq!(DisplayBackend::from_str("bogus"), DisplayBackend::Auto);
        assert_eq!(DisplayBackend::Auto.as_str(), "auto");
        let dir = tmp_profile("display");
        let mut prefs = Prefs::default();
        prefs.display = DisplayBackend::Wayland;
        prefs.close_to_tray = false;
        prefs.start_minimized = true;
        write_prefs(&dir, &prefs).unwrap();
        let back = read_prefs(&dir);
        assert_eq!(back.display, DisplayBackend::Wayland);
        assert!(!back.close_to_tray);
        assert!(back.start_minimized);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn css_brace_validation() {
        assert!(validate_css_braces("a { color: red; }").is_ok());
        assert!(validate_css_braces("a { color: red; ").is_err());
        assert!(validate_css_braces("a { color: red; }}").is_err());
        assert!(validate_css_braces("/* { */ a { color: red; }").is_ok());
        assert!(validate_css_braces("/* unterminated").is_err());
    }

    #[test]
    fn autostart_entry_has_tryexec_and_wmclass() {
        let entry = desktop_entry(Path::new("/usr/bin/ruwa"), false);
        assert!(entry.contains("TryExec=/usr/bin/ruwa"));
        assert!(entry.contains("StartupWMClass=Ruwa"));
        let mini = desktop_entry(Path::new("/usr/bin/ruwa"), true);
        assert!(mini.contains("Exec=/usr/bin/ruwa --minimized"));
    }
}
