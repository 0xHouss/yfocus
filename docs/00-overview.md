# `youn.yfocus` — Overview

A focus-discipline widget for the Omarchy status bar. A small chip on the bar
always shows the task you are currently working on. Four hotkey-driven
overlays (plus a click on the chip) manage a tiny queue so you stop drifting
into unrelated work.

The plugin is a single git repo at `~/coding/personal/plugins/yfocus`. It is
loaded into the running Omarchy shell via a symlink at
`~/.config/omarchy/plugins/youn.yfocus`.

---

## 1. Vocabulary

Lock these in before touching anything else. They appear verbatim in the CLI,
the JSON store, the QML files, and the docs.

| Term    | Hotkey           | Meaning |
|---------|------------------|---------|
| jump    | SUPER + SHIFT + T | Insert a new task at position 0. If a task is already current, it shifts to position 1. Every other task shifts down by one. Never destructive: nothing is lost, displaced, or marked done. |
| add     | SUPER + ALT + T   | Append a new task to the end of the queue. If nothing is current (empty queue, or only completed tasks left), the added task starts right away. |
| pop     | SUPER + T         | Mark the current task done and auto-promote the task at position 1 to current. |
| manage  | SUPER + CTRL + T  | Open the full management overlay (current task, queue list, reorder, delete, all keybindings). |
| chip    | (click)           | The bar element. Clicking it is equivalent to `manage`. |

Aliases used in conversation but **not** in code:

- "enqueue" = `add`
- "complete" = `pop`
- "promote" = the auto-promotion that happens as part of `pop`
- "cycle"   = old name for `pop`; do not introduce it anywhere

### Why this vocabulary

`jump` reads as "jump to this task now" — accurate for an insert-at-top that
displaces the previous current. `add` reads as "add this to my queue" —
accurate for an append. `pop` reads as "pop this off the stack" — accurate for
mark-done-and-advance. The bar chip always reflects `current`, so the user
glances at the bar and reads their job.

---

## 2. UX overview

```
  +------------------------ Bar -------------------------------+
  |  ▌Workspaces   ▸ Write the plugin plan   Tray Audio ... |   <-- chip shows current
  +----------------------------------------------------------+
```

Pressing any of the four hotkeys (or clicking the chip) summons a layer-shell
overlay through `omarchy-shell shell toggle`. Four overlay entry points, all
declared in the manifest:

```
  +--- JumpPrompt.qml (mode: "jump") -------------------+
  |  Jump to: [                                ]  Enter |
  +------------------------------------------------------+

  +--- AddPrompt.qml (mode: "add") --------------------+
  |  Enqueue: [                                ]  Enter |
  +------------------------------------------------------+

  +--- FocusOverlay.qml (mode: "manage") -------------+
  |  CURRENT                                          |
  |  ▸ Write the plugin plan          [pop] [jump↑]   |
  |  ---                                              |
  |  QUEUE                                            |
  |    1. Review PR #482                              |
  |    2. Email landlord                              |
  |  ---                                              |
  |  Add: [                                     ]      |
  |  n=add  ↑↓=move  Enter=select  d=del  Space=pop    |
  +----------------------------------------------------+
```

Layer rules live in `~/.config/hypr/apps/omarchy-shell.lua` and already cover
the `omarchy-menu` / `omarchy-clipboard` namespaces; we extend that namespace
list to add `omarchy-yfocus`.

---

## 3. Data model

Stored at `$XDG_STATE_HOME/omarchy/yfocus/queue.json`
(default `~/.local/state/omarchy/yfocus/queue.json`).

```json
{
  "version": 1,
  "current": "uuid-1",
  "tasks": [
    {
      "id": "uuid-1",
      "title": "Write the omarchy plugin plan",
      "note": "",
      "position": 0,
      "createdAt": 1735000000000,
      "startedAt": 1735000000000,
      "doneAt": null
    },
    {
      "id": "uuid-2",
      "title": "Review PR #482",
      "position": 1,
      "createdAt": 1735000001000,
      "startedAt": null,
      "doneAt": null
    }
  ]
}
```

### Invariants (enforced by `ts/queue.ts`)

1. `version` is always `1`.
2. `current` is either `null` or the `id` of an entry in `tasks`.
3. If `current` is non-null, the matching task has `doneAt === null`.
4. Exactly one task has `position === 0`, exactly one has `position === 1`, etc.
   (positions are contiguous from `0` to `tasks.length - 1`.)
5. `current`'s task has `startedAt !== null`. All others have `startedAt === null`.
6. A task with `doneAt !== null` is treated as completed and hidden from the
   default queue view (toggled visible via the overlay's "show completed"
   switch).
7. Writes go through atomic temp-file-rename. See
   [`B-persistence.md`](B-persistence.md).

### Schema versioning

`version: 1` is the only version the running plugin understands. Future
schema changes bump `version` and ship a migration in `ts/migrate.ts`. The
plugin refuses to load an unknown version and the CLI emits a clear error.

---

## 4. Repository layout

```
~/coding/personal/plugins/yfocus/
├── manifest.json              # kinds: ["bar-widget", "overlay"]
├── README.md                  # user-facing: install, screenshots, hotkeys
├── LICENSE                    # MIT
├── preview.png                # 16:9 screenshot for marketplace
├── BarWidget.qml              # bar chip
├── FocusOverlay.qml           # manage overlay
├── JumpPrompt.qml             # jump micro-overlay
├── AddPrompt.qml              # add micro-overlay
├── FocusModel.js              # QML-side mirror of pure logic
├── ts/
│   ├── types.ts               # Task, Queue, QueueState interfaces
│   ├── queue.ts               # pure ops: jump, add, pop, remove, reorder, setCurrent, clearCompleted
│   ├── store.ts               # atomic read/write with temp-rename
│   ├── cli.ts                 # `yfocus <cmd> [args...]` entry point
│   ├── migrate.ts             # future schema migrations
│   └── queue.test.ts          # bun test
├── bin/
│   └── yfocus                 # compiled CLI (gitignored)
├── hooks/
│   └── install.sh             # idempotent Hyprland binding installer
├── build.sh                   # bun build --compile -> bin/yfocus
├── docs/                      # this directory
└── .gitignore                 # bin/yfocus, node_modules/
```

\* `cycle` is kept as an internal helper alias for `pop` to make tests read
naturally; it is **not** exposed in the CLI or QML surface.

---

## 5. Build, test, install

```bash
# one-time: install bun (Arch)
pacman -S --needed bun        # or: curl -fsSL https://bun.sh/install | bash

# from repo root
bun install                   # future; for now: nothing to install
bun test                      # ts/queue.test.ts
./build.sh                    # bun build --compile -> bin/yfocus

# symlink into the user plugins directory
ln -s "$PWD" "$HOME/.config/omarchy/plugins/youn.yfocus"
omarchy-shell shell rescanPlugins
omarchy plugin enable youn.yfocus
omarchy restart shell

# run the install hook once to wire the four hotkeys
./hooks/install.sh
```

---

## 6. Theme tokens used from `qs.Commons`

All four overlays and the bar chip pull colors and spacing from the Omarchy
shell singletons so they re-theme with everything else:

```
Color.menu.background
Color.menu.text
Color.menu.border
Color.menu.scrim
Color.menu.selectedBackground
Color.menu.selectedText
Color.menu.selectedBorder

Style.font.menuFamily
Style.font.body
Style.font.caption
Style.font.title

Style.spacing.{xs,sm,md,lg,xl,panelPadding,controlPaddingY,rowPaddingX}
Style.cornerRadius
Style.space(n)
```

Border surfaces use `Border.surfaceSpec("menu", "border", border, ...)` so
selected rows get the same accent treatment as the Omarchy menu and clipboard.

---

## 7. Document map

This overview is the spine. The four appendices cover the implementation
phases in order; read them top-to-bottom on first pass, jump back as a
reference later.

- [`A-foundation.md`](A-foundation.md) — repo bootstrap, manifest, pure logic + tests
- [`B-persistence.md`](B-persistence.md) — atomic JSON store, CLI binary
- [`C-overlays.md`](C-overlays.md) — `FocusModel.js` + the three QML overlays
- [`D-shell-and-publish.md`](D-shell-and-publish.md) — bar chip, Hyprland bindings, marketplace polish
- [`INSTALL.md`](INSTALL.md) — user-facing install, configuration (keybindings), and deployment guide
