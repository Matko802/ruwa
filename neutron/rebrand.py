#!/usr/bin/env python3
"""Rebrand a Datcord (Neutron) release tree into Ruwa (WhatsApp).

Usage:
    rebrand.py --src <datcord-linux-x86_64 dir> --dst <output dir> --logo <png>
"""
import argparse
import os
import shutil
import subprocess
import sys
import zipfile

WA_HOME = "https://web.whatsapp.com/"

BRAND_PNGS = {
    "chrome/browser/content/branding/icon16.png": 16,
    "chrome/browser/content/branding/icon32.png": 32,
    "chrome/browser/content/branding/icon48.png": 48,
    "chrome/browser/content/branding/icon64.png": 64,
    "chrome/browser/content/branding/icon128.png": 128,
    "chrome/browser/content/branding/about-logo.png": 256,
    "chrome/browser/content/branding/about.png": 256,
}
DISK_ICONS = {16, 32, 48, 64, 128}

MOD_ID_RE = None


def _mod_id_ok(mod_id):
    import re

    return re.fullmatch(r"[a-z0-9][a-z0-9-]*", mod_id) is not None


def load_mod_meta(mods_dir, mod_id):
    import json as _json

    if not _mod_id_ok(mod_id):
        raise ValueError(f"invalid mod id: {mod_id!r}")
    meta_path = os.path.join(mods_dir, mod_id, "mod.json")
    with open(meta_path, encoding="utf-8") as f:
        meta = _json.load(f)
    if meta.get("id") != mod_id:
        raise ValueError(f"{meta_path}: id mismatch ({meta.get('id')!r})")
    files = meta.get("files")
    if not isinstance(files, list) or not files or not all(isinstance(x, str) for x in files):
        raise ValueError(f"{meta_path}: 'files' must be a non-empty string list")
    for rel in files:
        if rel.startswith("/") or ".." in rel.split("/"):
            raise ValueError(f"{meta_path}: unsafe file entry {rel!r}")
        full = os.path.join(mods_dir, mod_id, rel)
        if not os.path.isfile(full):
            raise ValueError(f"{meta_path}: missing file {rel!r}")
    return meta


def list_mods(mods_dir):
    if not os.path.isdir(mods_dir):
        return []
    out = sorted(
        d
        for d in os.listdir(mods_dir)
        if os.path.isdir(os.path.join(mods_dir, d)) and _mod_id_ok(d)
    )
    return [d for d in out if os.path.isfile(os.path.join(mods_dir, d, "mod.json"))]


def assemble_mods_loader(loader_template, manifest_template, mods_dir):
    with open(loader_template, encoding="utf-8") as f:
        template = f.read()
    with open(manifest_template, encoding="utf-8") as f:
        manifest = f.read()
    marker = "/*__RUWA_MOD_REGISTRY__*/"
    assert marker in template, "loader template marker missing"
    ids = list_mods(mods_dir)
    chunks = []
    for mod_id in ids:
        meta = load_mod_meta(mods_dir, mod_id)
        parts = []
        for rel in meta["files"]:
            with open(os.path.join(mods_dir, mod_id, rel), encoding="utf-8") as f:
                parts.append(f.read())
        body = "\n".join(parts)
        assert "</script" not in body.lower(), f"mod {mod_id}: inline </script> not allowed"
        chunks.append(f'  "{mod_id}": function() {{\n{body}\n  }}')
    registry = "var RUWA_MOD_IDS = [{}];\nvar RUWA_MODS = {{\n{}\n}};".format(
        ", ".join(f'"{i}"' for i in ids), ",\n".join(chunks)
    )
    return template.replace(marker, registry), manifest, ids


def install_mods_builtin_addon(omni_tmp, loader_template, manifest_template, mods_dir):
    addon_dir = os.path.join(omni_tmp, "chrome/browser/builtin-addons/ruwa-mods")
    os.makedirs(addon_dir, exist_ok=True)
    loader, manifest, ids = assemble_mods_loader(
        loader_template, manifest_template, mods_dir
    )
    with open(os.path.join(addon_dir, "manifest.json"), "w", encoding="utf-8") as f:
        f.write(manifest)
    with open(os.path.join(addon_dir, "loader.js"), "w", encoding="utf-8") as f:
        f.write(loader)
    print(f"installed ruwa-mods builtin addon ({len(ids)} mods: {', '.join(ids) or 'none'})")


def magick_resize(src, dst, size):
    subprocess.run(["magick", src, "-resize", f"{size}x{size}", dst], check=True)


def default_repo_path(name):
    here = os.path.dirname(os.path.abspath(__file__))
    return os.path.normpath(os.path.join(here, "..", name))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--src", required=True)
    ap.add_argument("--dst", required=True)
    ap.add_argument("--logo", required=True)
    ap.add_argument("--mods-dir", default=default_repo_path("mods"))
    ap.add_argument(
        "--loader-template",
        default=os.path.join(
            os.path.dirname(os.path.abspath(__file__)), "ruwa-mods-loader.js"
        ),
    )
    ap.add_argument(
        "--manifest-template",
        default=os.path.join(
            os.path.dirname(os.path.abspath(__file__)), "ruwa-mods-manifest.json"
        ),
    )
    args = ap.parse_args()
    src, dst, logo = args.src, args.dst, args.logo

    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(src, dst, symlinks=True)

    os.rename(os.path.join(dst, "datcord"), os.path.join(dst, "ruwa"))
    os.rename(os.path.join(dst, "datcord-bin"), os.path.join(dst, "ruwa-bin"))

    lp = os.path.join(dst, "launch-app")
    with open(lp, "rb") as f:
        data = f.read()
    n = data.count(b"datcord")
    data = data.replace(b"datcord", b"ruwa\x00\x00\x00")
    with open(lp, "wb") as f:
        f.write(data)
    print(f"launch-app: patched {n} occurrences")

    for size in DISK_ICONS:
        magick_resize(
            logo, os.path.join(dst, f"browser/chrome/icons/default/default{size}.png"), size
        )

    omnip = os.path.join(dst, "browser/omni.ja")
    tmp = os.path.join(os.path.dirname(dst), "omni-work")
    if os.path.exists(tmp):
        shutil.rmtree(tmp)
    os.makedirs(tmp)
    with zipfile.ZipFile(omnip, "r") as z:
        z.extractall(tmp)

    def patch_text(rel, old, new):
        p = os.path.join(tmp, rel)
        with open(p, "r", encoding="utf-8") as f:
            content = f.read()
        assert old in content, f"{old!r} not found in {rel}"
        with open(p, "w", encoding="utf-8") as f:
            f.write(content.replace(old, new))
        print(f"patched {rel}")

    patch_text(
        "defaults/preferences/firefox-branding.js",
        '"https://discord.com/app"',
        f'"{WA_HOME}"',
    )
    patch_text(
        "chrome/browser/builtin-addons/open-in-default-browser/replaceLinks.js",
        "http[s]?://discord.com",
        r"http[s]?://([a-z0-9-]+\.)*(whatsapp\.com|whatsapp\.net|wa\.me)",
    )
    patch_text("localization/en-US/branding/brand.ftl", "Datcord", "Ruwa")

    install_mods_builtin_addon(tmp, args.loader_template, args.manifest_template, args.mods_dir)

    with open(os.path.join(tmp, "defaults/preferences/firefox-branding.js"), "a") as f:
        f.write(
            """
/* Ruwa: hardware-accelerated rendering + media */
pref("gfx.webrender.all", true);
pref("layers.acceleration.force-enabled", true);
pref("media.ffmpeg.vaapi.enabled", true);
pref("media.hardware-video-decoding.enabled", true);
"""
        )
    print("appended Ruwa performance prefs")

    for rel, size in BRAND_PNGS.items():
        magick_resize(logo, os.path.join(tmp, rel), size)

    os.remove(omnip)
    with zipfile.ZipFile(omnip, "w", zipfile.ZIP_STORED) as z:
        for root, _, files in os.walk(tmp):
            for fn in sorted(files):
                full = os.path.join(root, fn)
                z.write(full, os.path.relpath(full, tmp))
    shutil.rmtree(tmp)
    print("repacked browser/omni.ja")

    ini = os.path.join(dst, "application.ini")
    with open(ini) as f:
        content = f.read()
    for old, new in [
        ("Vendor=Datcord", "Vendor=Ruwa"),
        ("Name=Datcord", "Name=Ruwa"),
        ("RemotingName=datcord", "RemotingName=ruwa"),
        ("CodeName=datcord", "CodeName=ruwa"),
        ("Profile=datcord", "Profile=ruwa"),
    ]:
        assert old in content, old
        content = content.replace(old, new)
    with open(ini, "w") as f:
        f.write(content)
    print("patched application.ini")

    import json as _json

    pol = os.path.join(dst, "distribution/policies.json")
    with open(pol) as f:
        policies = _json.load(f)
    perms = policies.setdefault("policies", {}).setdefault("Permissions", {})
    allowed_origins = [
        WA_HOME.rstrip("/"),
        "https://web.whatsapp.com",
        "https://whatsapp.com",
        "https://wa.me",
    ]
    for kind in ("Camera", "Microphone", "Notifications"):
        entry = perms.setdefault(kind, {})
        allow = entry.setdefault("Allow", [])
        existing = [u.rstrip("/") for u in allow]
        for origin in allowed_origins:
            if origin.rstrip("/") not in existing:
                allow.append(origin)
                existing.append(origin.rstrip("/"))
    with open(pol, "w") as f:
        _json.dump(policies, f, indent=2)
        f.write("\n")
    print("patched distribution/policies.json")
    print("DONE:", dst)


if __name__ == "__main__":
    sys.exit(main())
