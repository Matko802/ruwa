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

mkdir -p "$PROFILE"
mkdir -p "$PROFILE/chrome"
if [ ! -f "$PROFILE/chrome/userChrome.css" ] && [ -f "$APPDIR/chrome-dist/userChrome.css" ]; then
  cp "$APPDIR/chrome-dist/userChrome.css" "$PROFILE/chrome/userChrome.css"
fi
if ! grep -qF "legacyUserProfileCustomizations.stylesheets" "$PROFILE/user.js" 2>/dev/null; then
  printf '%s\n' 'user_pref("toolkit.legacyUserProfileCustomizations.stylesheets", true);' >> "$PROFILE/user.js"
fi
exec "$APPDIR/ruwa-bin" --profile "$PROFILE" "$@"
