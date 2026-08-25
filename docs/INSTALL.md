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
| [bun](https://bun.sh) ≥ 1.1 (builds the CLI binary and runs tests) | `bun --version` | `mise use -g bun@latest` or `sudo pacman -S bun` |
| `jq` (used by the installer's conflict probe) | `jq --version` | preinstalled on Omarchy |

### Build

```bash
cd ~/coding/personal/plugins/yfocus
bun test        # 41 tests should pass
./build.sh      # compiles bin/yfocus (~79 MB self-contained ELF)
```

`build.sh` fails fast with a hint if bun is missing. The output binary keeps
its symbol table (marketplace review inspects it with `nm`).

### Register with the Omarchy shell

The shell discovers user plugins in `~/.config/omarchy/plugins/`. A symlink
keeps your checkout as the source of truth:

```bash
ln -sfn "$PWD" "$HOME/.config/omarchy/plugins/youn.focus-queue"
omarchy-shell shell rescanPlugins
omarchy plugin enable youn.focus-queue
```

Enabling adds the bar widget to the center section of
`~/.config/omarchy/shell.json`. Restart the shell once:

```bash
omarchy restart shell
```

### Wire the hotkeys

```bash
./hooks/install.sh
```

The installer (idempotent, safe to re-run):

- writes a delimited block of bindings into `~/.config/hypr/bindings.lua`
- symlinks `bin/yfocus` → `~/.local/bin/yfocus` so `yfocus pop` works from
  Hyprland exec
- probes `hyprctl binds -j`; if SUPER+E is already owned by another binding,
  prepends an `hl.unbind("SUPER + E")` with a comment naming what was displaced
- runs `hyprctl reload`

Verify (Hyprland modmasks: SUPER=64, +SHIFT=65, +CTRL=68, +ALT=72):

```bash
hyprctl binds -j | jq -r '.[] | select(.description | test("focus task|focus queue"; "i"))
  | "\(.modmask) + \(.key) -> \(.description)"'
# 64 + e  -> Pop current focus task     (SUPER + E)
# 65 + t  -> Focus queue jump           (SUPER + SHIFT + T)
# 72 + t  -> Focus queue add            (SUPER + ALT + T)
# 68 + t  -> Open focus queue           (SUPER + CTRL + T)
```

### Verify end-to-end

1. The bar shows a chip: `▸ <current task>` (or a dimmed idle label).
2. Press **SUPER+CTRL+T** → the manage overlay opens; **Esc** closes it.
3. Run `yfocus current` → prints the chip's task title.

---

## 2. Configure

### 2.1 Keybindings (system-level)

The four system hotkeys live in one delimited block inside
`~/.config/hypr/bindings.lua`:

```lua
-- youn.focus-queue:start
o.bind("SUPER + E", "Pop current focus task", "yfocus pop")
o.bind("SUPER + SHIFT + T", "Focus queue jump", "omarchy-shell shell toggle youn.focus-queue '{\"mode\":\"jump\"}'")
o.bind("SUPER + ALT + T", "Focus queue add", "omarchy-shell shell toggle youn.focus-queue '{\"mode\":\"add\"}'")
o.bind("SUPER + CTRL + T", "Open focus queue", "omarchy-shell shell toggle youn.focus-queue '{\"mode\":\"manage\"}'")
-- youn.focus-queue:end
```

**How the block behaves:**

- Every `hooks/install.sh` run **rewrites the region between the markers**
  and strips any other line containing `youn.focus-queue` or `yfocus`.
- Consequence: to change keys, edit the `BLOCK_BODY` heredoc inside
  `hooks/install.sh`, then re-run it. Hand-editing lines that contain
  `yfocus` directly in `bindings.lua` will be overwritten next run.
- Custom bindings for *other* tools are never touched.
- If you prefer different keys entirely, keep the command strings and change
  only the first argument (`"MODS + KEY"`); valid modifier names match
  Hyprland's (`SUPER`, `SHIFT`, `ALT`, `CTRL`, e.g. `"SUPER + SHIFT"`).

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
repairing state:

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
| `YFOCUS_BIN` | Overrides the bundled CLI path resolved by the overlay. Set it when running the plugin straight from a checkout without `bin/yfocus` built. |

Writes are serialized with a lock directory and committed via
temp-file + atomic rename, so killing a writer mid-operation can never leave
a half-written `queue.json`.

### 2.4 Uninstall

```bash
omarchy plugin disable youn.focus-queue
rm "$HOME/.config/omarchy/plugins/youn.focus-queue"     # removes symlink only
rm -f "$HOME/.local/bin/yfocus"
sed -i '/youn\.focus-queue\|yfocus/d' ~/.config/hypr/bindings.lua && hyprctl reload
rm -rf "${XDG_STATE_HOME:-$HOME/.local/state}/omarchy/yfocus-queue"   # optional: delete data
```

---

## 3. Deploy

### 3.1 Personal machines (dotfiles)

Commit this repo into your dotfiles, then bootstrap new machines with:

```bash
git clone <your-dotfiles> ~/dotfiles
cd ~/coding/personal/plugins/yfocus     # wherever dotfiles place it
./build.sh
ln -sfn "$PWD" "$HOME/.config/omarchy/plugins/youn.focus-queue"
omarchy plugin enable youn.focus-queue || omarchy-shell shell rescanPlugins
./hooks/install.sh
omarchy restart shell
```

Notes for multi-machine setups:

- `bin/yfocus` is gitignored (79 MB build artifact); rebuild per machine via
  `./build.sh`.
- Task data is deliberately **not** in the repo (it lives in
  `$XDG_STATE_HOME`). Sync it across machines by pointing
  `XDG_STATE_HOME/omarchy/yfocus-queue` at a Syncthing/rsync folder if
  wanted — the lock protocol tolerates concurrent writers.
- The bindings block is regenerated by `hooks/install.sh` on each machine,
  so rebinding in the script propagates everywhere on next bootstrap.

### 3.2 Publishing to the marketplace

Third-party plugins install via `omarchy plugin add <git-url>`, which clones
the repo into `~/.config/omarchy/plugins/<manifest-id>/`. Requirements the
repo must satisfy:

1. **Git repo with `manifest.json` at its root** — already true.
   - `id` must not use the reserved `omarchy.*` namespace (`youn.focus-queue`
     is fine).
2. **Pass validation**: `omarchy plugin validate .` — checks schema fields,
   entry points exist, `defaultSection` value, and refuses any symlink
   inside the folder (the `.git` dir is exempt).
3. **Binary strategy** — decide how consumers get `yfocus`:
   - **Commit the compiled binary** on release branches
     (`git add -f bin/yfocus`; it is gitignored for daily dev). This is what
     the obsidian-daily plugin does; reviewers inspect the ELF with `nm`, so
     keep symbols (`strip = "debuginfo"` equivalent — bun's `--compile`
     doesn't strip by default).
   - **Or document post-install build**: README instructs `./build.sh`;
     requires bun on every consuming machine. Without the binary, the UI
     still renders but mutations don't persist.
4. **Assets**: `preview.png` (16:9 screenshot showing the bar chip and the
   manage overlay), `LICENSE` (MIT, present), `README.md` with the hotkey
   table (present).
5. **Push and tag**: create a public GitHub repo, push `main`, tag releases
   (`v0.1.0`). Updates flow as fast-forward pulls; users review a diff before
   applying.

Install command for end users once published:

```bash
omarchy plugin add https://github.com/<you>/yfocus.git --enable --yes
```

The installer never runs plugin code or hooks — users must run
`hooks/install.sh` themselves for the hotkeys (document this in the repo
README; the marketplace listing links there).

### 3.3 Versioning

Bump `version` in `manifest.json` for every published change; the shell and
`omarchy plugin update` both surface it. Keep `schemaVersion: 1` unless the
plugin manifest contract itself changes upstream.

---

## Troubleshooting

| Symptom | Fix |
|---|---|
| Chip missing from bar | `omarchy-shell shell listPlugins | jq '.[] | select(.id=="youn.focus-queue")'` — check `enabled: true`; then `omarchy restart shell`. |
| Overlay summons but is invisible | Check `hyprctl layers | grep focus-queue` right after summoning. Absent layer = QML load error; scan `journalctl --user` for warnings mentioning the plugin. |
| Hotkey does nothing | `hyprctl binds -j | jq` grep for the description; confirm `~/.local/bin/yfocus` exists; run `yfocus pop` manually. |
| Mutations don't persist | `YFOCUS_BIN` unset and `bin/yfocus` not built → build it (`./build.sh`) or export `YFOCUS_BIN=/path/to/yfocus`. |
| `queue.json` corrupt (JSON parse error on every op) | `yfocus show` reports the invariant violated. Move the file aside (`mv queue.json queue.json.bak`) and start fresh; completed history is lost but nothing else breaks. |
| Keys typed go into the add field instead of triggering shortcuts | Press `Esc` once to hand control back to the shortcut handler (by design after using the input). |
| Edited QML but behavior unchanged | The shell may serve stale code when the plugin is a symlinked checkout — `omarchy-shell shell rescanPlugins` is not always enough. Run `omarchy restart shell` after editing QML. |
