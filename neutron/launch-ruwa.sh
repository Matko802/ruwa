#!/bin/sh
# Ruwa launcher: single-instance wrapper around the rebranded
# Firefox SSB engine (Neutron build derived from Datcord).
set -eu

SCRIPT_DIR="$(dirname "$(readlink -f "$0")")"
if [ -x "$SCRIPT_DIR/ruwa-bin" ]; then
  DEFAULT_DIR="$SCRIPT_DIR"
else
  DEFAULT_DIR="$SCRIPT_DIR/../share/ruwa"
fi
APPDIR="${RUWA_DIR:-$DEFAULT_DIR}"
PROFILE="${RUWA_PROFILE:-${XDG_DATA_HOME:-$HOME/.local/share}/ruwa/profile}"
CUSTOM_CSS="${XDG_CONFIG_HOME:-$HOME/.config}/ruwa/custom.css"

case "${RUWA_BACKEND:-auto}" in
  x11|X11|xwayland)
    unset WAYLAND_DISPLAY
    ;;
  wayland|Wayland)
    ;;
  *)
    if [ -z "${WAYLAND_DISPLAY:-}" ]; then
      unset WAYLAND_DISPLAY
    fi
    ;;
esac

mkdir -p "$PROFILE/chrome"

if [ -f "$PROFILE/user.js" ] && [ ! -f "$PROFILE/user.js.ruwa-backup" ]; then
  cp -p "$PROFILE/user.js" "$PROFILE/user.js.ruwa-backup"
fi

install -m644 "$APPDIR/chrome-dist/userChrome.css" "$PROFILE/chrome/userChrome.css"

# Site style is rebuilt on every launch: Ruwa base plus the user's
# custom CSS. The trailing block publishes the enabled premade mods to
# the bundled ruwa-mods content script via --ruwa-mods (read from the
# ruwa.mods.enabled user.js pref written by Settings).
ENABLED_MODS="$(sed -n 's/^user_pref("ruwa\.mods\.enabled", "\(.*\)");$/\1/p' "$PROFILE/user.js" 2>/dev/null | tail -n 1)"
ENABLED_MODS="$(printf '%s' "$ENABLED_MODS" | tr -cd 'a-z0-9- ')"
{
  cat "$APPDIR/chrome-dist/userContent-base.css"
  if [ -f "$CUSTOM_CSS" ]; then
    printf '\n/* custom.css */\n'
    cat "$CUSTOM_CSS"
  fi
  printf '\n@-moz-document domain(web.whatsapp.com) {\n  :root { --ruwa-mods: "%s"; }\n}\n' "$ENABLED_MODS"
} > "$PROFILE/chrome/userContent.css.tmp"
mv "$PROFILE/chrome/userContent.css.tmp" "$PROFILE/chrome/userContent.css"

if ! grep -qF "legacyUserProfileCustomizations.stylesheets" "$PROFILE/user.js" 2>/dev/null; then
  printf '%s\n' 'user_pref("toolkit.legacyUserProfileCustomizations.stylesheets", true);' >> "$PROFILE/user.js"
fi
exec "$APPDIR/ruwa-bin" --profile "$PROFILE" "$@"
