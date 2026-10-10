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
LOCK="$PROFILE/.ruwa-lock"

mkdir -p "$PROFILE"
exec 9>"$LOCK"
if ! flock -n 9; then
  echo "ruwa is already running" >&2
  exit 0
fi

exec "$APPDIR/ruwa-bin" --profile "$PROFILE" "$@"
