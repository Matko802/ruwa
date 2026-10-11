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

const MANAGED_DOWNLOAD_KEYS: &[&str] = &[
    "browser.download.dir",
    "browser.download.folderList",
    "browser.download.useDownloadDir",
    "browser.download.always_ask_before_handling_new_types",
    "browser.download.start_downloads_in_tmp_dir",
    "browser.download.allow_save_from_sandboxed_iframes",
    "browser.download.open_pdf_attachments_inline",
];

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
    for key in MANAGED_DOWNLOAD_KEYS {
        let needle = format!("\"{key}\"");
        lines.retain(|l| !(l.trim().starts_with("user_pref(") && l.contains(&needle)));
    }
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

pub const MODS_ENABLED_PREF: &str = "ruwa.mods.enabled";

pub fn valid_mod_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !id.starts_with('-')
        && !id.ends_with('-')
}

pub fn read_mods_enabled(profile: &Path) -> Vec<String> {
    let Ok(content) = std::fs::read_to_string(user_js_path(profile)) else {
        return Vec::new();
    };
    let Some(raw) = string_pref_line(&content, MODS_ENABLED_PREF) else {
        return Vec::new();
    };
    let mut out: Vec<String> = raw
        .split(|c: char| c == ' ' || c == ',')
        .filter(|s| valid_mod_id(s))
        .map(String::from)
        .collect();
    out.sort();
    out.dedup();
    out
}

pub fn write_mods_enabled(profile: &Path, ids: &[String]) -> io::Result<()> {
    std::fs::create_dir_all(profile)?;
    let path = user_js_path(profile);
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let mut lines: Vec<String> = content.lines().map(str::to_string).collect();
    let mut clean: Vec<String> = ids.iter().filter(|s| valid_mod_id(s)).cloned().collect();
    clean.sort();
    clean.dedup();
    set_pref_line(&mut lines, MODS_ENABLED_PREF, &quoted(&clean.join(" ")));
    let mut out = lines.join("\n");
    out.push('\n');
    std::fs::write(path, out)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PremadeMod {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
}

fn json_string_field(src: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let mut search = 0usize;
    while let Some(rel) = src[search..].find(&needle) {
        let mut i = search + rel + needle.len();
        let b = src.as_bytes();
        while i < b.len() && (b[i] == b' ' || b[i] == b'\t' || b[i] == b'\n' || b[i] == b'\r') {
            i += 1;
        }
        if i >= b.len() || b[i] != b':' {
            search = search + rel + 1;
            continue;
        }
        i += 1;
        while i < b.len() && (b[i] == b' ' || b[i] == b'\t' || b[i] == b'\n' || b[i] == b'\r') {
            i += 1;
        }
        if i >= b.len() || b[i] != b'"' {
            search = search + rel + 1;
            continue;
        }
        i += 1;
        let mut out = String::new();
        loop {
            if i >= b.len() {
                return None;
            }
            match b[i] {
                b'"' => return Some(out),
                b'\\' => {
                    i += 1;
                    if i >= b.len() {
                        return None;
                    }
                    match b[i] {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'n' => out.push('\n'),
                        b't' => out.push('\t'),
                        b'r' => out.push('\r'),
                        _ => return None,
                    }
                    i += 1;
                }
                _ => {
                    out.push(b[i] as char);
                    i += 1;
                }
            }
        }
    }
    None
}

pub fn premade_mods_dir() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("RUWA_DIR") {
        if !p.trim().is_empty() {
            return Some(PathBuf::from(p).join("mods"));
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(bin) = exe.parent() {
            if let Some(prefix) = bin.parent() {
                let packaged = prefix.join("share").join("ruwa").join("mods");
                if packaged.is_dir() {
                    return Some(packaged);
                }
            }
        }
        let mut dir = exe.parent().map(Path::to_path_buf);
        for _ in 0..8 {
            let Some(d) = dir else { break };
            if d.join("mods").is_dir() && d.join("Cargo.toml").is_file() {
                return Some(d.join("mods"));
            }
            dir = d.parent().map(Path::to_path_buf);
        }
    }
    None
}

pub fn scan_mods(dir: &Path) -> Vec<PremadeMod> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(id) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !valid_mod_id(id) {
            continue;
        }
        let meta_path = path.join("mod.json");
        let Ok(meta) = std::fs::read_to_string(&meta_path) else {
            continue;
        };
        if json_string_field(&meta, "id").as_deref() != Some(id) {
            continue;
        }
        let Some(name) = json_string_field(&meta, "name") else {
            continue;
        };
        out.push(PremadeMod {
            id: String::from(id),
            name,
            version: json_string_field(&meta, "version").unwrap_or_default(),
            description: json_string_field(&meta, "description").unwrap_or_default(),
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

pub fn available_mods() -> Vec<PremadeMod> {
    premade_mods_dir()
        .map(|d| scan_mods(&d))
        .unwrap_or_default()
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
            close_to_tray: false,
            start_minimized: true,
            display: DisplayBackend::X11,
        };
        write_prefs(&dir, &before).unwrap();
        let after = read_prefs(&dir);
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
    fn neutron_download_defaults_restored() {
        let dir = tmp_profile("dlstrip");
        std::fs::write(
            user_js_path(&dir),
            "user_pref(\"browser.download.dir\", \"/tmp/old\");\nuser_pref(\"browser.download.folderList\", 2);\nuser_pref(\"browser.download.useDownloadDir\", true);\nuser_pref(\"browser.download.always_ask_before_handling_new_types\", false);\nuser_pref(\"browser.download.start_downloads_in_tmp_dir\", false);\nuser_pref(\"browser.download.allow_save_from_sandboxed_iframes\", true);\nuser_pref(\"browser.download.open_pdf_attachments_inline\", false);\nuser_pref(\"gfx.webrender.all\", true);\n",
        )
        .unwrap();
        write_prefs(&dir, &Prefs::default()).unwrap();
        let content = std::fs::read_to_string(user_js_path(&dir)).unwrap();
        assert!(!content.contains("browser.download."));
        assert!(content.contains("user_pref(\"gfx.webrender.all\", true);"));
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
    fn mods_enabled_round_trip_and_sanitize() {
        let dir = tmp_profile("modsenabled");
        write_mods_enabled(
            &dir,
            &[
                String::from("uaaa-decryptor"),
                String::from("EVIL;id"),
                String::from("uaaa-decryptor"),
                String::from(""),
            ],
        )
        .unwrap();
        assert_eq!(
            read_mods_enabled(&dir),
            vec![String::from("uaaa-decryptor")]
        );
        let content = std::fs::read_to_string(user_js_path(&dir)).unwrap();
        assert!(content.contains("user_pref(\"ruwa.mods.enabled\", \"uaaa-decryptor\");"));
        assert!(read_mods_enabled(&PathBuf::from("/nonexistent-ruwa-test")).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn mod_id_validation() {
        assert!(valid_mod_id("uaaa-decryptor"));
        assert!(valid_mod_id("a1"));
        assert!(!valid_mod_id(""));
        assert!(!valid_mod_id("UPPER"));
        assert!(!valid_mod_id("-lead"));
        assert!(!valid_mod_id("trail-"));
        assert!(!valid_mod_id("has space"));
        assert!(!valid_mod_id("has/slash"));
        assert!(!valid_mod_id("has.dot"));
    }

    #[test]
    fn scan_mods_reads_registry() {
        let base = std::env::temp_dir().join(format!("ruwa-test-scan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let good = base.join("good-mod");
        std::fs::create_dir_all(&good).unwrap();
        std::fs::write(
            good.join("mod.json"),
            "{\"id\": \"good-mod\", \"name\": \"Good \\\"Mod\\\"\", \"version\": \"1.0\", \"description\": \"d\"}",
        )
        .unwrap();
        let bad = base.join("bad-mod");
        std::fs::create_dir_all(&bad).unwrap();
        std::fs::write(bad.join("mod.json"), "{\"id\": \"other\", \"name\": \"x\"}").unwrap();
        std::fs::create_dir_all(base.join("no-meta")).unwrap();
        let mods = scan_mods(&base);
        assert_eq!(mods.len(), 1);
        assert_eq!(mods[0].id, "good-mod");
        assert_eq!(mods[0].name, "Good \"Mod\"");
        assert_eq!(mods[0].version, "1.0");
        assert!(scan_mods(&PathBuf::from("/nonexistent-ruwa-test")).is_empty());
        let _ = std::fs::remove_dir_all(&base);
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
