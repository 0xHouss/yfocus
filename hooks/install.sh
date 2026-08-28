#!/usr/bin/env bash
# youn.yfocus — Hyprland keybinding installer.
#
# Maintains a delimited block in the user's bindings.lua (Omarchy Lua
# config convention) and rewrites it in place on every run, so binding
# changes propagate to machines that ran an older version of this script.
# Idempotent and self-healing. User-written lines are never touched.

set -euo pipefail

BINDINGS_FILE="${HOME}/.config/hypr/bindings.lua"

BLOCK_BODY=$(cat << 'EOF'
-- youn.yfocus:start
o.bind("SUPER + E", "Pop current focus task", "yfocus pop")
o.bind("SUPER + SHIFT + T", "yfocus jump", "omarchy-shell shell toggle youn.yfocus '{\"mode\":\"jump\"}'")
o.bind("SUPER + ALT + T", "yfocus add", "omarchy-shell shell toggle youn.yfocus '{\"mode\":\"add\"}'")
o.bind("SUPER + CTRL + T", "Open yfocus", "omarchy-shell shell toggle youn.yfocus '{\"mode\":\"manage\"}'")
-- youn.yfocus:end
EOF
)

echo "Configuring Hyprland bindings for youn.yfocus..."
mkdir -p "$(dirname "$BINDINGS_FILE")"
touch "$BINDINGS_FILE"

# --- conflict probe (before writing, while old binds are still live) -----

PRELUDE=""
if command -v hyprctl >/dev/null 2>&1 && hyprctl binds -j >/dev/null 2>&1; then
  OWNER_DESC=$(hyprctl binds -j 2>/dev/null | jq -r '
    .[] | select((.modmask | tostring) == "64" and ((.key | ascii_downcase) == "e")) | .description' \
    | grep -v '^Pop current focus task$' | head -1 || true)
  if [ -n "${OWNER_DESC}" ]; then
    PRELUDE="-- Displaced by youn.yfocus: SUPER+E was bound to '${OWNER_DESC}'"
    PRELUDE="${PRELUDE}
hl.unbind(\"SUPER + E\")"
    echo "Note: SUPER+E currently bound to '${OWNER_DESC}'; will hl.unbind it."
  fi
fi

# --- rewrite --------------------------------------------------------------
# Strip every line we could have written before (marked block plus legacy
# unmarked lines from older installers); keep everything else verbatim.

TEMP_BASE="$(mktemp)"
grep -vE 'youn\.yfocus(:start|:end)?|yfocus' "$BINDINGS_FILE" > "$TEMP_BASE" || true

TEMP_OUT="$(mktemp)"
{
  cat "$TEMP_BASE"
  if [ -s "$TEMP_BASE" ]; then
    echo ""
  fi
  if [ -n "$PRELUDE" ]; then
    printf '%s\n' "$PRELUDE"
  fi
  printf '%s\n' "$BLOCK_BODY"
} > "$BINDINGS_FILE"
rm -f "$TEMP_BASE" "$TEMP_OUT"

echo "Bindings written to $BINDINGS_FILE"

# --- CLI on PATH for SUPER+E ----------------------------------------------

BIN_DIR="${HOME}/.local/bin"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
if [ -x "$REPO_ROOT/bin/yfocus" ]; then
  mkdir -p "$BIN_DIR"
  ln -sf "$REPO_ROOT/bin/yfocus" "$BIN_DIR/yfocus"
  echo "Linked $REPO_ROOT/bin/yfocus -> $BIN_DIR/yfocus"
else
  echo "Warning: bin/yfocus not built yet — run ./build.sh so 'yfocus pop' works."
fi

# --- apply ------------------------------------------------------------------

if command -v hyprctl >/dev/null 2>&1; then
  hyprctl reload || true
fi

echo "youn.yfocus hotkeys installed:"
echo "  SUPER+E          pop (complete current)"
echo "  SUPER+SHIFT+T    jump (insert at top)"
echo "  SUPER+ALT+T      add (append to queue)"
echo "  SUPER+CTRL+T     manage overlay"
