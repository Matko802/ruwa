use std::io;
use std::path::{Path, PathBuf};

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
pub struct AppConfig {
    pub close_to_tray: bool,
    pub start_minimized: bool,
    pub display: DisplayBackend,
    pub mods_enabled: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            close_to_tray: true,
            start_minimized: false,
            display: DisplayBackend::Auto,
            mods_enabled: Vec::new(),
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

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn data_root() -> PathBuf {
    if let Ok(p) = std::env::var("RUWA_DATA_DIR") {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_dir().join(".local").join("share"));
    base.join("ruwa")
}

pub fn webview_data_dir() -> PathBuf {
    data_root().join("webview")
}

pub fn legacy_firefox_profile_dir() -> PathBuf {
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

pub fn custom_css_path() -> PathBuf {
    config_dir().join("custom.css")
}

pub const CUSTOM_CSS_SEED: &str = "/* Ruwa custom CSS: applied to WhatsApp Web on every launch.\n   Plain CSS, no wrapper needed. Reload the page to apply edits. */\n\n#app {\n}\n";

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

#[allow(dead_code)]
pub fn strip_moz_document(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let bytes = css.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if css[i..].starts_with("@-moz-document") {
            let Some(open) = css[i..].find('{') else {
                out.push_str(&css[i..]);
                break;
            };
            let mut depth = 0i32;
            let mut j = i + open;
            let mut in_comment = false;
            let mut in_string: Option<u8> = None;
            let chars = bytes;
            while j < chars.len() {
                let c = chars[j];
                if in_comment {
                    if c == b'*' && j + 1 < chars.len() && chars[j + 1] == b'/' {
                        in_comment = false;
                        j += 2;
                        continue;
                    }
                    j += 1;
                    continue;
                }
                if let Some(q) = in_string {
                    if c == b'\\' {
                        j += 2;
                        continue;
                    }
                    if c == q {
                        in_string = None;
                    }
                    j += 1;
                    continue;
                }
                if c == b'/' && j + 1 < chars.len() && chars[j + 1] == b'*' {
                    in_comment = true;
                    j += 2;
                    continue;
                }
                if c == b'"' || c == b'\'' {
                    in_string = Some(c);
                    j += 1;
                    continue;
                }
                if c == b'{' {
                    depth += 1;
                } else if c == b'}' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                j += 1;
            }
            let inner_start = i + open + 1;
            let inner_end = j.min(css.len());
            if inner_start < inner_end {
                out.push_str(&css[inner_start..inner_end]);
                out.push('\n');
            }
            i = (j + 1).min(css.len());
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
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

fn json_bool_field(src: &str, key: &str) -> Option<bool> {
    let needle = format!("\"{key}\"");
    let mut search = 0usize;
    while let Some(rel) = src[search..].find(&needle) {
        let mut i = search + rel + needle.len();
        let b = src.as_bytes();
        while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\n' | b'\r') {
            i += 1;
        }
        if i >= b.len() || b[i] != b':' {
            search = search + rel + 1;
            continue;
        }
        i += 1;
        while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\n' | b'\r') {
            i += 1;
        }
        let rest = &src[i..];
        if rest.starts_with("true") {
            return Some(true);
        }
        if rest.starts_with("false") {
            return Some(false);
        }
        return None;
    }
    None
}

fn json_string_field(src: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let mut search = 0usize;
    while let Some(rel) = src[search..].find(&needle) {
        let mut i = search + rel + needle.len();
        let b = src.as_bytes();
        while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\n' | b'\r') {
            i += 1;
        }
        if i >= b.len() || b[i] != b':' {
            search = search + rel + 1;
            continue;
        }
        i += 1;
        while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\n' | b'\r') {
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
                c => {
                    out.push(c as char);
                    i += 1;
                }
            }
        }
    }
    None
}

fn json_string_list_field(src: &str, key: &str) -> Option<Vec<String>> {
    let needle = format!("\"{key}\"");
    let start = src.find(&needle)?;
    let after = &src[start + needle.len()..];
    let colon = after.find(':')?;
    let mut rest = after[colon + 1..].trim_start();
    if !rest.starts_with('[') {
        return None;
    }
    rest = &rest[1..];
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_str = false;
    let mut esc = false;
    for c in rest.chars() {
        if in_str {
            if esc {
                cur.push(c);
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                in_str = false;
                out.push(std::mem::take(&mut cur));
            } else {
                cur.push(c);
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            ']' => break,
            _ => {}
        }
    }
    Some(out)
}

fn escape_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
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
    out
}

pub fn valid_mod_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !id.starts_with('-')
        && !id.ends_with('-')
}

pub fn sanitize_mod_ids(ids: &[String]) -> Vec<String> {
    let mut clean: Vec<String> = ids
        .iter()
        .filter(|s| valid_mod_id(s))
        .cloned()
        .collect();
    clean.sort();
    clean.dedup();
    clean
}

fn legacy_config_from_user_js(content: &str) -> Option<AppConfig> {
    let mut cfg = AppConfig::default();
    let mut found = false;
    if let Some(v) = bool_pref_line(content, "ruwa.close_to_tray") {
        cfg.close_to_tray = v;
        found = true;
    }
    if let Some(v) = bool_pref_line(content, "ruwa.start_minimized") {
        cfg.start_minimized = v;
        found = true;
    }
    if let Some(v) = string_pref_line(content, "ruwa.display.backend") {
        cfg.display = DisplayBackend::from_str(&v);
        found = true;
    }
    if let Some(raw) = string_pref_line(content, "ruwa.mods.enabled") {
        cfg.mods_enabled = sanitize_mod_ids(
            &raw
                .split(|c: char| c == ' ' || c == ',')
                .map(String::from)
                .collect::<Vec<_>>(),
        );
        found = true;
    }
    if found { Some(cfg) } else { None }
}

fn legacy_migrated_config() -> Option<AppConfig> {
    let content = std::fs::read_to_string(legacy_firefox_profile_dir().join("user.js")).ok()?;
    legacy_config_from_user_js(&content)
}

pub fn read_config_from(path: &Path) -> AppConfig {
    let Ok(content) = std::fs::read_to_string(path) else {
        return AppConfig::default();
    };
    let mut cfg = AppConfig::default();
    if let Some(v) = json_bool_field(&content, "close_to_tray") {
        cfg.close_to_tray = v;
    }
    if let Some(v) = json_bool_field(&content, "start_minimized") {
        cfg.start_minimized = v;
    }
    if let Some(v) = json_string_field(&content, "display") {
        cfg.display = DisplayBackend::from_str(&v);
    }
    if let Some(v) = json_string_list_field(&content, "mods_enabled") {
        cfg.mods_enabled = sanitize_mod_ids(&v);
    }
    cfg
}

pub fn write_config_to(path: &Path, cfg: &AppConfig) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mods = sanitize_mod_ids(&cfg.mods_enabled);
    let ids = mods
        .iter()
        .map(|s| format!("\"{}\"", escape_json_string(s)))
        .collect::<Vec<_>>()
        .join(", ");
    let content = format!(
        "{{\n  \"close_to_tray\": {},\n  \"start_minimized\": {},\n  \"display\": \"{}\",\n  \"mods_enabled\": [{}]\n}}\n",
        if cfg.close_to_tray { "true" } else { "false" },
        if cfg.start_minimized { "true" } else { "false" },
        cfg.display.as_str(),
        ids
    );
    std::fs::write(path, content)
}

pub fn read_config() -> AppConfig {
    let path = config_path();
    if path.is_file() {
        return read_config_from(&path);
    }
    if let Some(migrated) = legacy_migrated_config() {
        let _ = write_config(&migrated);
        return migrated;
    }
    AppConfig::default()
}

pub fn write_config(cfg: &AppConfig) -> io::Result<()> {
    write_config_to(&config_path(), cfg)
}

pub fn read_mods_enabled() -> Vec<String> {
    read_config().mods_enabled
}

pub fn write_mods_enabled(ids: &[String]) -> io::Result<()> {
    let mut cfg = read_config();
    cfg.mods_enabled = sanitize_mod_ids(ids);
    write_config(&cfg)
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

pub fn clear_data_dir(dir: &Path) -> io::Result<Vec<String>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    std::fs::remove_dir_all(dir)?;
    std::fs::create_dir_all(dir)?;
    Ok(vec![String::from("webview")])
}

pub fn clear_site_data() -> io::Result<Vec<String>> {
    clear_data_dir(&webview_data_dir())
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
pub struct PremadeMod {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
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

    fn tmp_base(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("ruwa-test-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn config_round_trip() {
        let base = tmp_base("cfg");
        let path = base.join("config.json");
        let before = AppConfig {
            close_to_tray: false,
            start_minimized: true,
            display: DisplayBackend::X11,
            mods_enabled: vec![String::from("uaaa-decryptor")],
        };
        write_config_to(&path, &before).unwrap();
        let after = read_config_from(&path);
        assert_eq!(after, before);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn config_defaults_on_missing_file() {
        let base = tmp_base("cfgmissing");
        assert_eq!(
            read_config_from(&base.join("nonexistent.json")),
            AppConfig::default()
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn config_tolerates_unknown_fields() {
        let base = tmp_base("cfgunknown");
        let path = base.join("config.json");
        std::fs::write(
            &path,
            "{\"close_to_tray\": false, \"future\": 42, \"mods_enabled\": [\"uaaa-decryptor\"]}",
        )
        .unwrap();
        let cfg = read_config_from(&path);
        assert!(!cfg.close_to_tray);
        assert_eq!(cfg.mods_enabled, vec![String::from("uaaa-decryptor")]);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn legacy_user_js_is_migrated() {
        let cfg = legacy_config_from_user_js(
            "user_pref(\"ruwa.close_to_tray\", false);\nuser_pref(\"ruwa.start_minimized\", true);\nuser_pref(\"ruwa.display.backend\", \"x11\");\nuser_pref(\"ruwa.mods.enabled\", \"uaaa-decryptor\");\n",
        )
        .expect("legacy prefs must parse");
        assert!(!cfg.close_to_tray);
        assert!(cfg.start_minimized);
        assert_eq!(cfg.display, DisplayBackend::X11);
        assert_eq!(cfg.mods_enabled, vec![String::from("uaaa-decryptor")]);
        assert!(legacy_config_from_user_js("user_pref(\"other\", 1);\n").is_none());
    }

    #[test]
    fn custom_css_seed_is_plain_css() {
        assert!(!CUSTOM_CSS_SEED.trim().is_empty());
        assert!(!CUSTOM_CSS_SEED.contains("@-moz-document"));
        assert_eq!(
            custom_css_path().file_name().and_then(|s| s.to_str()),
            Some("custom.css")
        );
    }

    #[test]
    fn moz_document_wrapper_is_unwrapped() {
        let css = "@-moz-document domain(web.whatsapp.com) {\n  #app { color: red; }\n}\nbody { margin: 0; }";
        let out = strip_moz_document(css);
        assert!(out.contains("#app { color: red; }"));
        assert!(out.contains("body { margin: 0; }"));
        assert!(!out.contains("@-moz-document"));
    }

    #[test]
    fn clear_site_data_removes_webview_dir() {
        let base = tmp_base("clear");
        let dir = base.join("webview");
        std::fs::create_dir_all(dir.join("Cache")).unwrap();
        std::fs::write(dir.join("cookies.txt"), "x").unwrap();
        let removed = clear_data_dir(&dir).unwrap();
        assert_eq!(removed, vec![String::from("webview")]);
        assert!(dir.is_dir());
        assert!(!dir.join("cookies.txt").exists());
        let second = clear_data_dir(&dir).unwrap();
        assert_eq!(second, vec![String::from("webview")]);
        assert!(clear_data_dir(&base.join("missing")).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn mods_enabled_round_trip_and_sanitize() {
        let base = tmp_base("modsenabled");
        let path = base.join("config.json");
        let mut cfg = AppConfig::default();
        cfg.mods_enabled = sanitize_mod_ids(&[
            String::from("uaaa-decryptor"),
            String::from("EVIL;id"),
            String::from("uaaa-decryptor"),
            String::from(""),
        ]);
        write_config_to(&path, &cfg).unwrap();
        assert_eq!(
            read_config_from(&path).mods_enabled,
            vec![String::from("uaaa-decryptor")]
        );
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"mods_enabled\": [\"uaaa-decryptor\"]"));
        let _ = std::fs::remove_dir_all(&base);
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
        let base = tmp_base("display");
        let path = base.join("config.json");
        let cfg = AppConfig {
            close_to_tray: false,
            start_minimized: true,
            display: DisplayBackend::Wayland,
            mods_enabled: Vec::new(),
        };
        write_config_to(&path, &cfg).unwrap();
        let back = read_config_from(&path);
        assert_eq!(back.display, DisplayBackend::Wayland);
        assert!(!back.close_to_tray);
        assert!(back.start_minimized);
        let _ = std::fs::remove_dir_all(&base);
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
