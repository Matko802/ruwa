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
MODS="${XDG_CONFIG_HOME:-$HOME/.config}/ruwa/mods"
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

{
  cat "$APPDIR/chrome-dist/userContent-base.css"
  for mod in "$MODS"/*/mod.css; do
    [ -f "$mod" ] || continue
    moddir="$(dirname "$mod")"
    if [ -f "$moddir/disabled" ]; then
      continue
    fi
    printf '\n/* mod: %s */\n' "$mod"
    cat "$mod"
  done
  if [ -f "$CUSTOM_CSS" ]; then
    printf '\n/* custom.css */\n'
    cat "$CUSTOM_CSS"
  fi
} > "$PROFILE/chrome/userContent.css.tmp"
mv "$PROFILE/chrome/userContent.css.tmp" "$PROFILE/chrome/userContent.css"

if ! grep -qF "legacyUserProfileCustomizations.stylesheets" "$PROFILE/user.js" 2>/dev/null; then
  printf '%s\n' 'user_pref("toolkit.legacyUserProfileCustomizations.stylesheets", true);' >> "$PROFILE/user.js"
fi
exec "$APPDIR/ruwa-bin" --profile "$PROFILE" "$@"
