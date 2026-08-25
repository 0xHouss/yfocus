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

Full install, configuration (keybindings, bar settings, CLI) and deployment
instructions live in [`docs/INSTALL.md`](docs/INSTALL.md). Quick version:

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

Or add them manually to `~/.config/hypr/bindings.lua`:

```lua
-- youn.focus-queue:start
o.bind("SUPER + E", "Pop current focus task", "yfocus pop")
o.bind("SUPER + SHIFT + T", "Focus queue jump", "omarchy-shell shell toggle youn.focus-queue '{\"mode\":\"jump\"}'")
o.bind("SUPER + ALT + T", "Focus queue add", "omarchy-shell shell toggle youn.focus-queue '{\"mode\":\"add\"}'")
o.bind("SUPER + CTRL + T", "Open focus queue", "omarchy-shell shell toggle youn.focus-queue '{\"mode\":\"manage\"}'")
-- youn.focus-queue:end
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
