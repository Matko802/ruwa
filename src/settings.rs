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

#[derive(Debug, Clone, PartialEq)]
pub struct Prefs {
    pub gpu_accel: bool,
    pub vaapi: bool,
    pub audio: AudioBackend,
    pub download_dir: String,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            gpu_accel: true,
            vaapi: true,
            audio: AudioBackend::Auto,
            download_dir: String::new(),
        }
    }
}

pub fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
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

pub fn effective_download_dir(p: &Prefs) -> PathBuf {
    if p.download_dir.trim().is_empty() {
        default_download_dir()
    } else {
        PathBuf::from(p.download_dir.trim())
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
    let mut out = lines.join("\n");
    out.push('\n');
    std::fs::write(path, out)
}

pub fn autostart_path() -> PathBuf {
    home_dir()
        .join(".config")
        .join("autostart")
        .join("ruwa.desktop")
}

pub fn desktop_entry(exec: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Ruwa\nExec={} \nIcon=ruwa\nTerminal=false\nCategories=Network;InstantMessaging;\nX-GNOME-Autostart-enabled=true\n",
        exec.display()
    )
}

pub fn set_autostart(enable: bool, exe: &Path) -> io::Result<()> {
    let path = autostart_path();
    if enable {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, desktop_entry(exe))
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
    None
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
}
