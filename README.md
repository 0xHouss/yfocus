# yfocus

[![Omarchy marketplace](https://img.shields.io/badge/Omarchy-marketplace-teal)](https://omarchyplugins.com/plugin.html?id=You-ne5.yfocus)

Omarchy Quattro bar widget for a focused task queue: always shows your current task, jump (insert at top), add (append), or pop (complete + promote next) from the bar.

## Requirements

- Omarchy Quattro (Quickshell-based shell) on Linux
- A supported architecture: `x86_64` or `aarch64` (`arm64`) — other archs show `⚠ arch`
- CLI via bundled `bin/yfocus` or `YFOCUS_BIN` override (no extra env needed for the bar)

Tasks live in `$XDG_STATE_HOME/omarchy/You-ne5.yfocus/queue.json` (`~/.local/state/omarchy/You-ne5.yfocus/queue.json`).

## Architecture

- **TS backend** (`yfocus`): `ts/cli.ts` → `bin/yfocus` via `bun build --compile --target bun-linux-x64` (static ELF, `~80 MB`). `ts/store.ts` handles `XDG_STATE_HOME`, lock, atomic write; `ts/queue.ts` is the queue logic. Released as `bin/yfocus-x86_64` / `-aarch64` + `bin/yfocus` shim.
- **QML frontend** (`BarWidget.qml` / `FocusOverlay.qml`): `bar-widget` chip + `overlay` manager. Architecture detection via `uname -m` picks the bundled ELF; falls back to `yfocus` on `PATH`/`YFOCUS_BIN`.

```
yfocus watch (FileView) ──(queue.json)──▶ BarWidget ─▶ FocusOverlay
yfocus jump/add/pop ──(queue.json)──▶ BarWidget
```


## screenshots
### Current task widget
<img width="600" height="116" alt="image" src="https://github.com/user-attachments/assets/ac573c53-7e6a-4e18-9261-212f177787da" />

### Main menu
<img width="900" height="540" alt="image" src="https://github.com/user-attachments/assets/4d777d67-91de-498d-8d02-2e24c8129ccc" />

### Enqueue menu
<img width="900" height="540" alt="image" src="https://github.com/user-attachments/assets/ecb3bdd0-631f-47fc-9307-60fc1801b5c1" />

### Jump menu
<img width="900" height="540" alt="image" src="https://github.com/user-attachments/assets/ed33853d-1b88-4f00-aa08-b0654363de54" />


## Install

```bash
omarchy plugin add https://github.com/You-ne5/yfocus.git --enable
```

Update / remove:

```bash
omarchy plugin update You-ne5.yfocus
omarchy plugin remove You-ne5.yfocus
```

If the bar still shows an error right after an update, run `omarchy restart shell` once. Omarchy hot-reloads QML in place; a full restart drops a stale widget.

The plugin bundles `bin/yfocus-x86_64` / `bin/yfocus-aarch64` and a `bin/yfocus` shim that `exec`s the matching arch (so a hot-reloaded pre-bundle widget keeps working). The widget prefers the arch-specific ELF via `uname -m`. If that cannot start, it tries `yfocus` on `PATH` (`YFOCUS_BIN` override, or `make install` / `ln -sf bin/yfocus ~/.local/bin/yfocus`); if both fail, mutations stay in-memory until a binary is available. On unsupported arch the bar shows `⚠ arch`.

## Usage

- **Bar**: `▸ <current>` or `idleLabel` (`▸ focus`). Click → manager; right-click → `pop` (complete current).
- **Overlay** (`SUPER+CTRL+T` or bar click, `jump`/`add` modes via `omarchy-shell shell toggle You-ne5.yfocus '{"mode":"…"}'`):
  - `p` pop, `n` add, `↑`/`↓` select, `Ctrl+↑`/`↓` reorder, `Space` promote, `d` delete, `Shift+D` clear completed, `s` toggle completed, `Esc` close. Jump/add prompts: `Enter` submit, `Esc` cancel.
- **Vocabulary**: `jump` insert at 0 (previous current → 1), `add` append (becomes current if no current), `pop` mark done + promote next.
- **Settings** (`omarchy bar set You-ne5.yfocus …`): `idleLabel`.

```bash
omarchy bar set You-ne5.yfocus idleLabel '▸ focus'
```

Hotkeys are **opt-in** — no installer touches `bindings.lua`. Add manually to `~/.config/hypr/apps/yfocus.conf` or `bindings.lua`, or run `hooks/install.sh --apply` (creates `apps/yfocus.conf`):

```lua
o.bind("SUPER + E", "Pop current focus task", "yfocus pop")
o.bind("SUPER + SHIFT + T", "yfocus jump", "omarchy-shell shell toggle You-ne5.yfocus '{\"mode\":\"jump\"}'")
o.bind("SUPER + ALT + T", "yfocus add", "omarchy-shell shell toggle You-ne5.yfocus '{\"mode\":\"add\"}'")
o.bind("SUPER + CTRL + T", "Open yfocus", "omarchy-shell shell toggle You-ne5.yfocus '{\"mode\":\"manage\"}'")
```

## CLI

```bash
yfocus show              # queue as JSON
yfocus current           # current title
yfocus jump "title"      # insert at top, becomes current
yfocus add "title"       # append; becomes current if none
yfocus pop               # done + promote next
yfocus remove <id>
yfocus reorder <from> <to>
yfocus set-current <id>
yfocus clear-completed
yfocus reset
yfocus path
```

`YFOCUS_BIN` overrides the bundled path.

## Development

```bash
bun test                          # ts/queue.test.ts + FocusModel.js parity
omarchy plugin validate .
qmllint -I "$OMARCHY_PATH/shell" BarWidget.qml FocusOverlay.qml
./build.sh                        # dev: bun-linux-x64 → bin/yfocus
make bundle && make verify-bundle # reproducible x86_64 + aarch64 bundle gate
```

Any edit under `ts/`, `manifest.json` or `build.sh` requires a fresh `make bundle` in the same change.

### Releasing

1. Bump `manifest.json` version and `CHANGELOG.md`.
2. Run `make bundle && make verify-bundle` on Linux.
3. Open a PR; wait for CI `build` to be green.
4. Merge, then tag `vX.Y.Z` matching `manifest.json`.

## License

MIT — Copyright (c) 2026 You-ne5
