# youn.yfocus

Omarchy plugin that helps with task management, it showcases your current task on the omarchy bar, and allows you to quickly enqueue, jump or pop a task
So it basically allows you to manage your tasks, and constantly keep track of what you're currently working on.

| Key              | Action                          |
|------------------|---------------------------------|
| SUPER + E        | Pop current (mark done, promote next) |
| SUPER + SHIFT + T | Jump (insert at top, becomes current) |
| SUPER + ALT + T  | Add (append to end of queue)    |
| SUPER + CTRL + T | Open the full management overlay |

Clicking the bar chip also opens the manager.

## Vocabulary

- **jump** — insert at position 0; the previous current task shifts to
  position 1; everything else shifts down by one. Never destructive.
- **add** — append to the end of the queue. If nothing is current (empty
  queue, or only completed tasks left), it becomes current right away.
- **pop** — mark the current task done and promote the next queued task.

## Install

```bash
omarchy plugin add https://github.com/<you>/yfocus.git --enable
```

No files outside the plugin are touched. Bundled musl binaries (`bin/yfocus-x86_64`, `bin/yfocus-aarch64`, `bin/yfocus` shim) are included via `make bundle` — no Rust needed on the target machine. See [`docs/INSTALL.md`](docs/INSTALL.md) for manual build and dotfiles checkout:

```bash
git clone <this-repo> && cd yfocus
# source checkout without bundle:
./build.sh   # or: make bundle  (musl + sha256 + srcid, marketplace gate)
ln -sfn "$PWD" "$HOME/.config/omarchy/plugins/youn.focus-queue"
omarchy plugin enable youn.focus-queue
```

Hotkeys are **opt-in** and never installed automatically. Copy to `~/.config/hypr/bindings.lua` or run the helper that creates a dedicated file:

```bash
./hooks/install.sh --apply   # creates ~/.config/hypr/apps/yfocus-queue.conf
# or manually:
```

```lua
o.bind("SUPER + E", "Pop current focus task", "yfocus pop")
o.bind("SUPER + SHIFT + T", "Focus queue jump", "omarchy-shell shell toggle youn.focus-queue '{\"mode\":\"jump\"}'")
o.bind("SUPER + ALT + T", "Focus queue add", "omarchy-shell shell toggle youn.focus-queue '{\"mode\":\"add\"}'")
o.bind("SUPER + CTRL + T", "Open focus queue", "omarchy-shell shell toggle youn.focus-queue '{\"mode\":\"manage\"}'")
```

Full hotkey, bar, CLI, and deployment details in [`docs/INSTALL.md`](docs/INSTALL.md).

## Data

Tasks live in `$XDG_STATE_HOME/omarchy/yfocus-queue/queue.json`
(default `~/.local/state/omarchy/yfocus-queue/queue.json`). The CLI can drive
the queue without any UI (prefers `bin/yfocus-<arch>` then `bin/yfocus` shim, then `yfocus` on PATH, then `YFOCUS_BIN`):

```bash
yfocus show            # print queue as JSON
yfocus jump "title"    # insert at top, becomes current
yfocus add "title"     # append to end of queue
yfocus pop             # mark current done, promote next
yfocus --help          # full command list
```

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
make bundle && make verify-bundle  # reproducible musl bundle gate
omarchy plugin validate .
qmllint -I "$OMARCHY_PATH/shell" BarWidget.qml FocusOverlay.qml
```

See `docs/00-overview.md` for the design and the appendices in `docs/` for
implementation notes.
