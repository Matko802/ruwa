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

mkdir -p "$PROFILE/chrome"

# Window chrome is managed by Ruwa: content-only window, control the app
# through the tray menu, Ctrl+R and F11.
install -m644 "$APPDIR/chrome-dist/userChrome.css" "$PROFILE/chrome/userChrome.css"

# Site style is rebuilt on every launch: Ruwa base plus the user's mods.
{
  cat "$APPDIR/chrome-dist/userContent-base.css"
  for mod in "$MODS"/*/mod.css; do
    [ -f "$mod" ] || continue
    printf '\n/* mod: %s */\n' "$mod"
    cat "$mod"
  done
} > "$PROFILE/chrome/userContent.css.tmp"
mv "$PROFILE/chrome/userContent.css.tmp" "$PROFILE/chrome/userContent.css"

if ! grep -qF "legacyUserProfileCustomizations.stylesheets" "$PROFILE/user.js" 2>/dev/null; then
  printf '%s\n' 'user_pref("toolkit.legacyUserProfileCustomizations.stylesheets", true);' >> "$PROFILE/user.js"
fi
exec "$APPDIR/ruwa-bin" --profile "$PROFILE" "$@"
