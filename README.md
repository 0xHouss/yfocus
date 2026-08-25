# youn.focus-queue

Always-visible focus chip + tiny task queue for the Omarchy bar.

The bar chip shows the task you are currently working on. Four hotkeys drive
a tiny queue so you stop drifting into unrelated work:

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
- **add** — append to the end of the queue.
- **pop** — mark the current task done and promote the next queued task.

## Install

Requires [bun](https://bun.sh) to build:

```bash
git clone <this-repo>
cd yfocus
./build.sh

ln -s "$PWD" "$HOME/.config/omarchy/plugins/youn.focus-queue"
omarchy-shell shell rescanPlugins
omarchy plugin enable youn.focus-queue
```

Then wire the hotkeys:

```bash
./hooks/install.sh
```

Or add them manually to your Hyprland bindings:

```
bind = SUPER, E, exec, yfocus pop
bind = SUPER SHIFT, T, exec, omarchy-shell shell toggle youn.focus-queue '{"mode":"jump"}'
bind = SUPER ALT, T, exec, omarchy-shell shell toggle youn.focus-queue '{"mode":"add"}'
bind = SUPER CTRL, T, exec, omarchy-shell shell toggle youn.focus-queue '{"mode":"manage"}'
```

(`yfocus` must be on PATH — see `build.sh`.)

## Data

Tasks live in `$XDG_STATE_HOME/omarchy/yfocus-queue/queue.json`
(default `~/.local/state/omarchy/yfocus-queue/queue.json`). The CLI can drive
the queue without any UI:

```bash
yfocus show            # print queue as JSON
yfocus jump "title"    # insert at top, becomes current
yfocus add "title"     # append to end of queue
yfocus pop             # mark current done, promote next
yfocus --help          # full command list
```

## Development

```bash
bun test      # pure logic tests
./build.sh    # compile bin/yfocus
```

See `docs/00-overview.md` for the design and the appendices in `docs/` for
implementation notes.
