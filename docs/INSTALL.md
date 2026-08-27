# Install, Configure & Deploy

This guide covers three scenarios:

1. **[Install](#1-install)** — get the plugin running on this machine
2. **[Configure](#2-configure)** — keybindings, bar placement, CLI, data
3. **[Deploy](#3-deploy)** — replicate on other machines, or publish publicly

For the design and architecture, see [`00-overview.md`](00-overview.md).

---

## 1. Install

### Prerequisites

| Requirement | Check | Install |
|---|---|---|
| Omarchy (Hyprland + Quickshell shell) | `omarchy version` | https://omarchy.org |
| [Rust](https://rustup.rs) 1.97.1 (only for local builds) | `cargo --version` | `rustup` or `sudo pacman -S rust` |

Marketplace installs do **not** require Rust — the plugin bundles static musl binaries (`bin/yfocus-x86_64`, `bin/yfocus-aarch64`, `bin/yfocus` shim) built via `make bundle`.

### Install from the marketplace

```bash
omarchy plugin add https://github.com/<you>/yfocus.git --enable
omarchy restart shell   # only if the bar still shows the previous version after an update
```

No installer touches your Hyprland config. Hotkeys are optional — see [Wire the hotkeys](#wire-the-hotkeys).

### Build from source (dev)

```bash
cd ~/coding/personal/plugins/yfocus
cargo test
./build.sh              # dev: host glibc binary -> bin/yfocus (~0.8 MB)
# or for a marketplace-ready bundle:
make bundle             # musl x86_64 + aarch64 -> bin/yfocus-* + shim + .sha256 + .srcid
make verify-bundle      # gate used by CI and releases
```

`build.sh` fails fast if cargo is missing. Release bundles keep symbols (`strip = "debuginfo"` so `nm` can inspect).

### Register with the Omarchy shell (manual checkout)

If you cloned without `omarchy plugin add`:

```bash
ln -sfn "$PWD" "$HOME/.config/omarchy/plugins/youn.focus-queue"
omarchy-shell shell rescanPlugins
omarchy plugin enable youn.focus-queue
```

Enabling adds the bar widget to the center section of `~/.config/omarchy/shell.json`. Restart the shell once:

```bash
omarchy restart shell
```

### Wire the hotkeys (optional)

No file is modified automatically. Add the bindings manually where you keep Hyprland binds. Two safe options:

**Option A — dedicated file (recommended, never touches your bindings.lua):**

Create `~/.config/hypr/apps/yfocus-queue.conf`:

```lua
o.bind("SUPER + E", "Pop current focus task", "yfocus pop")
o.bind("SUPER + SHIFT + T", "Focus queue jump", "omarchy-shell shell toggle youn.focus-queue '{\"mode\":\"jump\"}'")
o.bind("SUPER + ALT + T", "Focus queue add", "omarchy-shell shell toggle youn.focus-queue '{\"mode\":\"add\"}'")
o.bind("SUPER + CTRL + T", "Open focus queue", "omarchy-shell shell toggle youn.focus-queue '{\"mode\":\"manage\"}'")
```

Or run the helper (creates the same file, backs up existing):

```bash
./hooks/install.sh --apply
```

**Option B — inline in bindings.lua:**

Copy the same four `o.bind` lines into `~/.config/hypr/bindings.lua` inside your own block.

Verify (Hyprland modmasks: SUPER=64, +SHIFT=65, +CTRL=68, +ALT=72):

```bash
hyprctl binds -j | jq -r '.[] | select(.description | test("focus task|focus queue"; "i")) | "\(.modmask) + \(.key) -> \(.description)"'
# 64 + e  -> Pop current focus task
# 65 + t  -> Focus queue jump
# 72 + t  -> Focus queue add
# 68 + t  -> Open focus queue
```

`yfocus pop` works without the shell (CLI-only). If `yfocus` is not on PATH, use the bundled shim: `~/.config/omarchy/plugins/youn.focus-queue/bin/yfocus pop` or `YFOCUS_BIN=/path/to/yfocus`.

### Verify end-to-end

1. The bar shows a chip: `▸ <current task>` (or a dimmed idle label).
2. Press **SUPER+CTRL+T** → the manage overlay opens; **Esc** closes it.
3. Run `yfocus current` → prints the chip's task title.

---

## 2. Configure

### 2.1 Keybindings (system-level)

Add the four `o.bind` lines from above to the file you chose (`apps/yfocus-queue.conf` or `bindings.lua`). The plugin never rewrites your config at runtime — you own the bindings and can change the mod/key string (`"SUPER + E"` etc.) without the plugin overwriting them.

**What each dispatch does:**

| Dispatch | Effect |
|---|---|
| `yfocus pop` | CLI-only path; marks current done, promotes next. Works even if the shell is down. |
| `omarchy-shell shell toggle youn.focus-queue '{"mode":"…"}'` | Summons/hides the overlay in that mode: `jump` inserts at top (becomes current), `add` appends — and becomes current if nothing is current, `manage` opens the manager. |

**Overlay-internal keys** (handled by the plugin, not Hyprland) while the
manager is open:

| Key | Action |
|---|---|
| `p` | Pop current (same as SUPER+E) |
| `n` | Focus the add input |
| *any letter* | Type-anywhere: starts a new task seeded with that character |
| `↑` / `↓` | Move selection through the queue |
| `Ctrl+↑` / `Ctrl+↓` | Reorder selected task within the queue |
| `Space` | Promote selected queued task to current (Enter does nothing here — it belongs to the add input, so submitting a task can never also promote a row) |
| `d` | Delete selected queued task |
| `Shift+D` | Clear all completed tasks |
| `s` | Toggle showing completed tasks |
| `Esc` | Close overlay / leave the input |

In jump/add prompts: `Enter` submits, `Esc` cancels.

### 2.2 Bar placement and settings

Move the chip between sections without editing JSON:

```bash
omarchy bar move youn.focus-queue --section right
```

Per-widget settings are inline on the entry in
`~/.config/omarchy/shell.json` under `bar.layout.<section>`:

```json
{ "id": "youn.focus-queue", "idleLabel": "▸ focus" }
```

| Setting | Default | Meaning |
|---|---|---|
| `idleLabel` | `"▸ focus"` | Chip text when no task is current |

On vertical bars the chip collapses to a `▸` glyph.

### 2.3 CLI configuration

`yfocus` drives the queue without any UI — useful for scripts and for
repairing state. The plugin resolves it as `bin/yfocus-<arch>` → `bin/yfocus` shim → `yfocus` on PATH → `YFOCUS_BIN` override.

```bash
yfocus show              # full queue as JSON
yfocus current           # current task title
yfocus jump "title"      # insert at top, becomes current
yfocus add "title"       # append to end; becomes current if nothing is current
yfocus pop               # mark done + promote next
yfocus remove <id>
yfocus reorder <from> <to>
yfocus set-current <id>
yfocus clear-completed
yfocus reset             # wipe everything
yfocus path              # print data file location
```

Environment variables:

| Variable | Purpose |
|---|---|
| `XDG_STATE_HOME` | Data root; defaults to `~/.local/state`. Queue lives at `$XDG_STATE_HOME/omarchy/yfocus-queue/queue.json`. |
| `YFOCUS_BIN` | Overrides the bundled CLI path resolved by the overlay. Set it when running without the bundled musl binary. |

Writes are serialized with a lock directory and committed via
temp-file + atomic rename, so killing a writer mid-operation can never leave
a half-written `queue.json`.

### 2.4 Uninstall

```bash
omarchy plugin disable youn.focus-queue
omarchy plugin remove youn.focus-queue
# if you used --apply:
rm -f ~/.config/hypr/apps/yfocus-queue.conf
# optional: delete data
rm -rf "${XDG_STATE_HOME:-$HOME/.local/state}/omarchy/yfocus-queue"
```

The plugin never modifies `~/.config/hypr/bindings.lua` — if you added bindings there manually, remove those four `o.bind` lines yourself.

---

## 3. Deploy

### 3.1 Personal machines (dotfiles)

Commit this repo into your dotfiles, then bootstrap new machines with:

```bash
git clone <your-dotfiles> ~/dotfiles
cd ~/coding/personal/plugins/yfocus
# marketplace clones already have bin/yfocus-*; for a source checkout:
make bundle   # or ./build.sh for a host-only dev build
ln -sfn "$PWD" "$HOME/.config/omarchy/plugins/youn.focus-queue"
omarchy plugin enable youn.focus-queue || omarchy-shell shell rescanPlugins
# optional hotkeys:
./hooks/install.sh --apply   # or copy the 4 o.bind lines manually
omarchy restart shell
```

Notes for multi-machine setups:

- Bundled binaries (`bin/yfocus-*`, `bin/yfocus` shim) **are** committed via `make bundle` (reproducible musl). No per-machine rebuild needed.
- Task data is deliberately **not** in the repo (it lives in `$XDG_STATE_HOME`). Sync it across machines by pointing `XDG_STATE_HOME/omarchy/yfocus-queue` at a Syncthing/rsync folder if wanted — the lock protocol tolerates concurrent writers.
- Hotkeys are owned by you in `apps/yfocus-queue.conf` or `bindings.lua` — no plugin-side regeneration to propagate.

### 3.2 Publishing to the marketplace

Third-party plugins install via `omarchy plugin add <git-url>`, which clones
the repo into `~/.config/omarchy/plugins/<manifest-id>/`. Requirements the
repo must satisfy:

1. **Git repo with `manifest.json` at its root** — already true.
2. **Pass validation**: `omarchy plugin validate .` — checks schema fields, entry points exist, `defaultSection` value, and refuses any symlink inside the folder.
3. **Binary strategy**: `make bundle` then `make verify-bundle` on Linux (x86_64 + aarch64 musl, `strip = "debuginfo"`, `sha256` + `.srcid` attested). This matches the obsidian-daily marketplace gate; reviewers inspect the ELF with `nm`. CI runs the same gate on every push; do not add `needs:` that would skip `verify-bundle`.
4. **Assets**: `preview.png` (16:9 screenshot showing the bar chip and the manage overlay), `LICENSE` (MIT, present), `README.md` with the hotkey table (present).
5. **Push and tag**: create a public GitHub repo, push `main`, tag releases (`v0.1.0`) matching `Cargo.toml` and `manifest.json` versions. Updates flow as fast-forward pulls; users review a diff before applying.

Install command for end users once published:

```bash
omarchy plugin add https://github.com/<you>/yfocus.git --enable --yes
```

The installer never runs plugin code or hooks — the bundled `bin/yfocus-*` is cloned as plain files. Hotkeys remain opt-in via the manual block above (documented in README; marketplace links there).

### 3.3 Versioning

Bump `version` in `Cargo.toml`, `manifest.json`, and `CHANGELOG.md` for every published change; run `make bundle && make verify-bundle` in the same commit; the shell and `omarchy plugin update` both surface it. Keep `schemaVersion: 1` unless the plugin manifest contract itself changes upstream.

---

## Troubleshooting

| Symptom | Fix |
|---|---|
| Chip missing from bar | `omarchy-shell shell listPlugins | jq '.[] | select(.id=="youn.focus-queue")'` — check `enabled: true`; then `omarchy restart shell`. |
| Overlay summons but is invisible | Check `hyprctl layers | grep focus-queue` right after summoning. Absent layer = QML load error; scan `journalctl --user` for warnings mentioning the plugin. |
| Hotkey does nothing | `hyprctl binds -j | jq` grep for the description; check `~/.config/hypr/apps/yfocus-queue.conf` or `bindings.lua` contains the `o.bind` lines; try `~/.config/omarchy/plugins/youn.focus-queue/bin/yfocus pop` directly. |
| `yfocus: command not found` | Bundled binary is `bin/yfocus` shim; add `~/.config/omarchy/plugins/youn.focus-queue/bin` to PATH or `ln -sf` manually, or `cargo install --path .`, or set `YFOCUS_BIN`. |
| Mutations don't persist | `YFOCUS_BIN` unset and `bin/yfocus-*` missing → run `make bundle` (or `cargo build --release` dev) or set `YFOCUS_BIN`. |
| `queue.json` corrupt (JSON parse error on every op) | `yfocus show` reports the invariant violated. Move the file aside (`mv queue.json queue.json.bak`) and start fresh; completed history is lost but nothing else breaks. |
| Keys typed go into the add field instead of triggering shortcuts | Press `Esc` once to hand control back to the shortcut handler (by design after using the input). |
| Edited QML but behavior unchanged | The shell may serve stale code when the plugin is a symlinked checkout — `omarchy-shell shell rescanPlugins` is not always enough. Run `omarchy restart shell` after editing QML. |
