use std::path::{Path, PathBuf};

fn manifest_dir() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"))
}

fn mod_id_ok(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !id.starts_with('-')
        && !id.ends_with('-')
}

fn json_string_field(src: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let mut search = 0usize;
    let b = src.as_bytes();
    while let Some(rel) = src[search..].find(&needle) {
        let mut i = search + rel + needle.len();
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

fn escape_js_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

fn list_mods(mods_dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(mods_dir) else {
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
        if !mod_id_ok(id) {
            continue;
        }
        if path.join("mod.json").is_file() {
            out.push(id.to_string());
        }
    }
    out.sort();
    out
}

fn build_mods_registry(mods_dir: &Path) -> String {
    let mut chunks = Vec::new();
    let mut ids = Vec::new();
    for id in list_mods(mods_dir) {
        let meta_path = mods_dir.join(&id).join("mod.json");
        let meta = std::fs::read_to_string(&meta_path)
            .unwrap_or_else(|e| panic!("ruwa build: cannot read {}: {e}", meta_path.display()));
        assert_eq!(
            json_string_field(&meta, "id").as_deref(),
            Some(id.as_str()),
            "ruwa build: {}: id mismatch",
            meta_path.display()
        );
        let files_raw = meta
            .split_once("\"files\"")
            .unwrap_or_else(|| panic!("ruwa build: {}: missing 'files'", meta_path.display()))
            .1;
        let mut files = Vec::new();
        let mut in_str = false;
        let mut esc = false;
        let mut cur = String::new();
        let mut depth = 0i32;
        let mut started = false;
        for c in files_raw.chars() {
            if !started {
                if c == '[' {
                    started = true;
                    depth = 1;
                }
                continue;
            }
            if in_str {
                if esc {
                    cur.push(c);
                    esc = false;
                } else if c == '\\' {
                    cur.push(c);
                    esc = true;
                } else if c == '"' {
                    in_str = false;
                    files.push(std::mem::take(&mut cur));
                } else {
                    cur.push(c);
                }
                continue;
            }
            match c {
                '"' => in_str = true,
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
        assert!(
            !files.is_empty(),
            "ruwa build: {}: 'files' must be a non-empty string list",
            meta_path.display()
        );
        let mut body = String::new();
        for rel in &files {
            assert!(
                !rel.starts_with('/') && !rel.split('/').any(|s| s == ".."),
                "ruwa build: {}: unsafe file entry {rel:?}",
                meta_path.display()
            );
            let full = mods_dir.join(&id).join(rel);
            assert!(
                full.is_file(),
                "ruwa build: {}: missing file {rel:?}",
                meta_path.display()
            );
            let content = std::fs::read_to_string(&full)
                .unwrap_or_else(|e| panic!("ruwa build: cannot read {}: {e}", full.display()));
            assert!(
                !content.to_lowercase().contains("</script"),
                "ruwa build: mod {id}: inline </script> not allowed"
            );
            if !body.is_empty() {
                body.push('\n');
            }
            body.push_str(&content);
        }
        ids.push(id.clone());
        chunks.push(format!("  \"{}\": function() {{\n{body}\n  }}", escape_js_string(&id)));
    }
    let id_list = ids
        .iter()
        .map(|i| format!("\"{}\"", escape_js_string(i)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("var RUWA_MOD_IDS = [{id_list}];\nvar RUWA_MODS = {{\n{}\n}};", chunks.join(",\n"))
}

fn main() {
    let root = manifest_dir();
    let mods_dir = root.join("mods");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    println!("cargo:rerun-if-changed=mods");
    println!("cargo:rerun-if-changed=assets/base.css");
    println!("cargo:rerun-if-changed=src-tauri/tauri.conf.json");
    let registry = build_mods_registry(&mods_dir);
    std::fs::write(out_dir.join("ruwa_mods_registry.js"), registry)
        .expect("ruwa build: cannot write mods registry");
    tauri_build::try_build(
        tauri_build::Attributes::new().config_path("src-tauri/tauri.conf.json"),
    )
    .expect("ruwa build: tauri_build failed");
}
