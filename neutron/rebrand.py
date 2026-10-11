#!/usr/bin/env python3
"""Rebrand a Datcord (Neutron) release tree into Ruwa (WhatsApp).

Usage:
    rebrand.py --src <datcord-linux-x86_64 dir> --dst <output dir> --logo <png> --topbar <ruwa-topbar dir>
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


def magick_resize(src, dst, size):
    subprocess.run(["magick", src, "-resize", f"{size}x{size}", dst], check=True)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--src", required=True)
    ap.add_argument("--dst", required=True)
    ap.add_argument("--logo", required=True)
    ap.add_argument("--topbar", required=True)
    args = ap.parse_args()
    src, dst, logo, topbar = args.src, args.dst, args.logo, args.topbar

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
        r"http[s]?://.*\.whatsapp\.(com|net)",
    )
    patch_text("localization/en-US/branding/brand.ftl", "Datcord", "Ruwa")

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

    addon_dir = os.path.join(tmp, "chrome/browser/builtin-addons/ruwa-topbar")
    shutil.copytree(topbar, addon_dir)
    os.chmod(addon_dir, 0o755)
    for size in (16, 32, 48):
        magick_resize(logo, os.path.join(addon_dir, f"icon{size}.png"), size)
    import json as _addons_json

    registry = os.path.join(tmp, "chrome/browser/content/browser/built_in_addons.json")
    with open(registry) as f:
        builtins = _addons_json.load(f)
    entry = {
        "addon_id": "ruwa-topbar@rused",
        "addon_version": "1.0",
        "res_url": "resource://builtin-addons/ruwa-topbar/",
    }
    if entry not in builtins["builtins"]:
        builtins["builtins"].append(entry)
    with open(registry, "w") as f:
        _addons_json.dump(builtins, f)
    print("registered ruwa-topbar builtin addon")

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
    for kind in ("Camera", "Microphone", "Notifications"):
        entry = perms.setdefault(kind, {})
        allow = entry.setdefault("Allow", [])
        if WA_HOME.rstrip("/") not in [u.rstrip("/") for u in allow]:
            allow.append(WA_HOME.rstrip("/"))
    with open(pol, "w") as f:
        _json.dump(policies, f, indent=2)
        f.write("\n")
    print("patched distribution/policies.json")
    print("DONE:", dst)


if __name__ == "__main__":
    sys.exit(main())
