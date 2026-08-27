# Appendix A — Foundation (steps 0, 1, 2)

Steps 0–2 cover everything before the first user-visible byte lands: the
repo skeleton, the plugin manifest, and the pure logic that drives every
mutation. Nothing here requires the Omarchy shell or any UI; you can land
this appendix on a fresh machine in under an hour.

---

## Step 0 — Repo & toolchain bootstrap

**Goal:** an empty git repo with a `.gitignore`, `LICENSE`, README skeleton,
and a verified bun install.

### 0.1 Create the directory and initialize git

```bash
mkdir -p ~/coding/personal/plugins/yfocus
cd ~/coding/personal/plugins/yfocus
git init
git switch -c main    # or 'master' if that is your default
```

### 0.2 Verify bun

```bash
bun --version    # expected: >= 1.1.0
```

If bun is not installed:

```bash
# Arch / Omarchy:
sudo pacman -S --needed bun

# or the universal installer:
curl -fsSL https://bun.sh/install | bash
```

### 0.3 `.gitignore`

```gitignore
# build artifacts
bin/yfocus
bin/*.bun

# node
node_modules/

# editor
.vscode/
.idea/
*.swp

# OS
.DS_Store
Thumbs.db
```

### 0.4 `LICENSE` (MIT)

```
MIT License

Copyright (c) <year> <author>

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

### 0.5 `README.md` skeleton

```markdown
# youn.yfocus

Always-visible focus chip + tiny task queue for the Omarchy bar.

See [`docs/00-overview.md`](docs/00-overview.md) for the design, and the
appendices for implementation notes.

## Hotkeys

| Key             | Action                       |
|-----------------|------------------------------|
| SUPER + T       | Pop current                  |
| SUPER + SHIFT + T | Jump (insert at top)       |
| SUPER + ALT + T | Add (append to queue)        |
| SUPER + CTRL + T | Open full management overlay |

## Install (development)

    ln -s "$PWD" "$HOME/.config/omarchy/plugins/youn.yfocus"
    omarchy-shell shell rescanPlugins
    omarchy plugin enable youn.yfocus
    omarchy restart shell
    ./hooks/install.sh
```

### 0.6 Verify

```bash
git status     # clean
ls -la         # .git/ .gitignore LICENSE README.md
```

### Done when

- `git status` is clean.
- `bun --version` exits 0.

---

## Step 1 — Manifest + plugin shell

**Goal:** Omarchy's plugin registry discovers and loads the plugin. Four
entry points are registered (`bar-widget`, `overlay` x 3) but each renders
only a placeholder label. The plugin passes `omarchy plugin validate`.

### 1.1 `manifest.json`

```json
{
  "schemaVersion": 1,
  "id": "youn.yfocus",
  "name": "yfocus",
  "version": "0.1.0",
  "author": "youn",
  "license": "MIT",
  "description": "Task Queue, allowing you to enqueue, jump, or pop tasks, constantly shows current task to improve focus.",
  "kinds": ["bar-widget", "overlay"],
  "activation": "on-demand",
  "entryPoints": {
    "barWidget": "BarWidget.qml",
    "overlay": "FocusOverlay.qml"
  },
  "barWidget": {
    "displayName": "yfocus",
    "description": "Shows the current task. Click to open the queue manager.",
    "category": "Productivity",
    "allowMultiple": false,
    "defaultSection": "center"
  }
}
```

### 1.2 Placeholder QML files

The plugin will eventually declare four overlay entry points in
`entryPoints`, but `omarchy plugin validate` only checks one entry point per
declared kind. For `kind: overlay`, we register `FocusOverlay.qml` (the manage
overlay); the other two overlays (`JumpPrompt.qml`, `AddPrompt.qml`) are
loaded by `FocusOverlay.qml` on demand via a `Loader` based on the IPC
payload's `mode` field. That keeps the manifest honest: one entry point per
declared kind, no extras.

This is also the simplest model — three related UIs share one layer-shell
window lifecycle, one key catcher, one scrim. See
[`C-overlays.md`](C-overlays.md) for the full design.

So the four placeholder files are:

- `BarWidget.qml`
- `FocusOverlay.qml`
- `JumpPrompt.qml` (loaded inside `FocusOverlay.qml` based on `mode`)
- `AddPrompt.qml`  (loaded inside `FocusOverlay.qml` based on `mode`)

Each renders just a label so we can confirm the entry points are wired:

#### `BarWidget.qml` (placeholder)

```qml
import QtQuick
import Quickshell
import qs.Commons

BarWidget {
  id: root
  moduleName: "youn.yfocus"

  Text {
    anchors.centerIn: parent
    text: "focus.queue (bar placeholder)"
    color: Color.foreground
  }
}
```

#### `FocusOverlay.qml` (placeholder)

```qml
import QtQuick
import QtQuick.Controls
import Quickshell
import Quickshell.Wayland
import qs.Commons

Item {
  id: root

  property bool opened: false
  property string omarchyPath: Quickshell.env("OMARCHY_PATH")
  property var shell: null
  property var manifest: null

  function open(payloadJson) {
    root.opened = true
  }

  function close() {
    root.opened = false
  }

  function toggle() {
    if (root.opened) root.close()
    else root.open("{}")
  }

  PanelWindow {
    id: panel
    visible: root.opened
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"
    WlrLayershell.namespace: "omarchy-yfocus"
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
    exclusionMode: ExclusionMode.Ignore

    Rectangle {
      anchors.fill: parent
      color: "black"
      opacity: 0.6
    }

    Text {
      anchors.centerIn: parent
      text: "focus.queue overlay placeholder (mode: " + (root.mode || "manage") + ")"
      color: "white"
    }
  }

  Component.onCompleted: console.log("FocusOverlay loaded")
}
```

#### `JumpPrompt.qml` (placeholder)

```qml
import QtQuick

Item {
  id: root
  Text {
    anchors.centerIn: parent
    text: "JumpPrompt placeholder"
  }
}
```

#### `AddPrompt.qml` (placeholder)

```qml
import QtQuick

Item {
  id: root
  Text {
    anchors.centerIn: parent
    text: "AddPrompt placeholder"
  }
}
```

### 1.3 Symlink into the user plugins directory and validate

```bash
ln -s "$PWD" "$HOME/.config/omarchy/plugins/youn.yfocus"
omarchy plugin validate ./yfocus
omarchy-shell shell rescanPlugins
omarchy plugin enable youn.yfocus
omarchy restart shell
```

Expected from `omarchy plugin validate`:

```
(exits 0, no output)
```

Expected from `omarchy-shell shell listPlugins`:

```
... contains an entry with id "youn.yfocus", kinds ["bar-widget","overlay"] ...
```

### 1.4 Layer rule (Hyprland)

Add `omarchy-yfocus` to the namespace exclusion list so the overlay
plays nicely with Hyprland animations. In
`~/.config/hypr/apps/omarchy-shell.lua`:

```lua
hl.layer_rule({ match = { namespace = "^(omarchy-menu|omarchy-image-selector|omarchy-emojis|omarchy-clipboard|omarchy-keyboard-panel|omarchy-yfocus)$" }, no_anim = true, animation = "none" })
```

Reload Hyprland:

```bash
hyprctl reload
```

### 1.5 Smoke test

1. The bar should show a `focus.queue (bar placeholder)` chip somewhere in
   the center.
2. `omarchy-shell shell toggle youn.yfocus '{}'` should summon the dark
   scrim with the placeholder text.

### Done when

- `omarchy plugin validate ./yfocus` exits 0.
- `omarchy-shell shell listPlugins` lists `youn.yfocus`.
- The chip and overlay placeholders are visible.

---

## Step 2 — Pure logic + tests

**Goal:** all queue mutations exist as pure TypeScript functions, covered
by `bun test`. No I/O, no Omarchy, no QML.

### 2.1 `ts/types.ts`

```typescript
export interface Task {
  id: string;
  title: string;
  note: string;
  position: number;
  createdAt: number;       // unix ms
  startedAt: number | null;
  doneAt: number | null;
}

export interface QueueState {
  version: 1;
  current: string | null;
  tasks: Task[];
}

export const EMPTY_QUEUE: QueueState = {
  version: 1,
  current: null,
  tasks: [],
};
```

### 2.2 `ts/queue.ts`

Every function takes a `QueueState` and returns a new `QueueState`. They
never mutate in place; the store layer is responsible for writing the
result back to disk.

```typescript
import { QueueState, Task, EMPTY_QUEUE } from "./types.ts";

// --- helpers -------------------------------------------------------------

function sortByPosition(tasks: Task[]): Task[] {
  return [...tasks].sort((a, b) => a.position - b.position);
}

function reindex(tasks: Task[]): Task[] {
  const sorted = sortByPosition(tasks);
  return sorted.map((t, i) => ({ ...t, position: i }));
}

function newId(): string {
  // 16 random hex chars; collision-free at human scales without crypto deps.
  return [...crypto.getRandomValues(new Uint8Array(8))]
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

// --- constructors --------------------------------------------------------

export function emptyQueue(): QueueState {
  return { ...EMPTY_QUEUE, tasks: [] };
}

export function fromTasks(tasks: Task[], current: string | null = null): QueueState {
  return { version: 1, current, tasks: reindex(tasks) };
}

// --- queries -------------------------------------------------------------

export function getCurrent(state: QueueState): Task | null {
  if (state.current === null) return null;
  return state.tasks.find((t) => t.id === state.current) ?? null;
}

export function getQueue(state: QueueState): Task[] {
  // open tasks, ordered, excluding current and completed.
  return sortByPosition(state.tasks).filter(
    (t) => t.id !== state.current && t.doneAt === null,
  );
}

export function getCompleted(state: QueueState): Task[] {
  return sortByPosition(state.tasks).filter((t) => t.doneAt !== null);
}

// --- mutations -----------------------------------------------------------

/**
 * Jump: insert a new task at position 0.
 * - If a task is current, it shifts to position 1.
 * - All other tasks shift down by one.
 * - The new task becomes current with startedAt = now.
 * - Never destructive.
 */
export function jump(state: QueueState, title: string, now: number = Date.now()): QueueState {
  const trimmed = title.trim();
  if (trimmed.length === 0) return state;

  const inserted: Task = {
    id: newId(),
    title: trimmed,
    note: "",
    position: 0,
    createdAt: now,
    startedAt: now,
    doneAt: null,
  };

  // All existing tasks move down by one position. The previous current
  // (if any) keeps its slot semantics by virtue of position shifting.
  const shifted = state.tasks.map((t) => ({ ...t, position: t.position + 1 }));

  return {
    version: 1,
    current: inserted.id,
    tasks: reindex([inserted, ...shifted]),
  };
}

/**
 * Add: append a new task to the end of the queue.
 * - Does not affect current.
 */
export function add(state: QueueState, title: string, now: number = Date.now()): QueueState {
  const trimmed = title.trim();
  if (trimmed.length === 0) return state;

  const appended: Task = {
    id: newId(),
    title: trimmed,
    note: "",
    position: state.tasks.length,
    createdAt: now,
    startedAt: null,
    doneAt: null,
  };

  return {
    ...state,
    tasks: [...state.tasks, appended],
  };
}

/**
 * Pop: mark the current task done and promote the task at position 1 to
 * current.
 * - If there is no current, returns the state unchanged.
 * - If there is no task at position 1, current becomes null.
 */
export function pop(state: QueueState, now: number = Date.now()): QueueState {
  if (state.current === null) return state;

  const updated = state.tasks.map((t) => {
    if (t.id === state.current) return { ...t, doneAt: now };
    return t;
  });

  const sorted = sortByPosition(updated);
  const currentTask = sorted.find((t) => t.id === state.current);
  if (currentTask && currentTask.position !== 0) {
    // Invariant violated; refuse to silently corrupt.
    throw new Error(`pop: current task is at position ${currentTask.position}, expected 0`);
  }

  // Next-up is the open task at position 1.
  const next = sorted.find((t) => t.position === 1 && t.doneAt === null);
  if (!next) {
    return { ...state, tasks: sorted, current: null };
  }

  const withStarted = sorted.map((t) =>
    t.id === next.id ? { ...t, startedAt: t.startedAt ?? now } : t,
  );

  return { ...state, tasks: withStarted, current: next.id };
}

/**
 * Remove: delete a task by id.
 * - If the removed task was current, promote the task at position 1 to
 *   current. If none, current becomes null.
 * - Cannot remove the current task if it is the only one and current
 *   becomes null as a result; user must add something new or leave the
 *   queue empty.
 */
export function remove(state: QueueState, id: string): QueueState {
  if (!state.tasks.some((t) => t.id === id)) return state;

  const wasCurrent = state.current === id;
  const remaining = state.tasks.filter((t) => t.id !== id);

  if (!wasCurrent) {
    return { ...state, tasks: reindex(remaining) };
  }

  const reindexed = reindex(remaining);
  // Promote the new position 0 (which was position 1 before removal).
  const promoted = reindexed.find((t) => t.position === 0);
  if (!promoted) {
    return { ...state, tasks: reindexed, current: null };
  }
  return {
    ...state,
    tasks: reindexed.map((t) =>
      t.position === 0 && t.startedAt === null
        ? { ...t, startedAt: t.startedAt ?? Date.now() }
        : t,
    ),
    current: promoted.id,
  };
}

/**
 * Reorder: move a task to a new position among open, non-current tasks.
 * Positions are 0-based over the open-and-non-current queue view
 * (current is implicit at position 0 in the bar chip but does not
 * participate in reorder). Indices passed to this function are positions
 * in that view.
 */
export function reorder(
  state: QueueState,
  fromIndex: number,
  toIndex: number,
): QueueState {
  const queue = getQueue(state);
  if (fromIndex < 0 || fromIndex >= queue.length) return state;
  if (toIndex < 0 || toIndex >= queue.length) return state;
  if (fromIndex === toIndex) return state;

  const moving = queue[fromIndex];
  const without = queue.filter((_, i) => i !== fromIndex);
  without.splice(toIndex, 0, moving);

  // Rebuild tasks: current stays at position 0, reindexed queue follows.
  const currentTask = getCurrent(state);
  const completed = getCompleted(state);

  const rebuilt: Task[] = [];
  if (currentTask) rebuilt.push(currentTask);
  rebuilt.push(...without, ...completed);
  return { ...state, tasks: reindex(rebuilt) };
}

/**
 * setCurrent: pick any open task by id and make it current.
 * - The previous current becomes the head of the queue (position 1).
 * - Already-current id is a no-op.
 */
export function setCurrent(state: QueueState, id: string, now: number = Date.now()): QueueState {
  if (state.current === id) return state;
  const target = state.tasks.find((t) => t.id === id);
  if (!target || target.doneAt !== null) return state;

  const previousCurrent = getCurrent(state);
  const others = state.tasks.filter(
    (t) => t.id !== id && t.id !== (previousCurrent?.id ?? null),
  );
  const completed = others.filter((t) => t.doneAt !== null);
  const openOthers = others.filter((t) => t.doneAt === null);

  const head: Task[] = [];
  if (previousCurrent) {
    head.push({ ...previousCurrent, startedAt: previousCurrent.startedAt ?? now });
  }
  head.push({ ...target, position: previousCurrent ? 1 : 0, startedAt: now });

  return {
    ...state,
    current: id,
    tasks: reindex([...head, ...openOthers, ...completed]),
  };
}

// --- validation ----------------------------------------------------------

/**
 * Invariants are checked by `assertInvariants` on every test, and once on
 * startup by the CLI before applying mutations.
 */
export function assertInvariants(state: QueueState): void {
  if (state.version !== 1) {
    throw new Error(`version must be 1, got ${state.version}`);
  }
  if (state.current !== null && !state.tasks.some((t) => t.id === state.current)) {
    throw new Error(`current id ${state.current} not in tasks`);
  }
  const currentTask = state.tasks.find((t) => t.id === state.current);
  if (currentTask && currentTask.doneAt !== null) {
    throw new Error(`current task is marked done`);
  }
  const positions = new Set<number>();
  for (const t of state.tasks) {
    if (positions.has(t.position)) {
      throw new Error(`duplicate position ${t.position}`);
    }
    positions.add(t.position);
  }
  if (positions.size !== state.tasks.length) {
    throw new Error(`positions are not contiguous`);
  }
  if (state.tasks.length > 0) {
    for (let i = 0; i < state.tasks.length; i++) {
      if (!positions.has(i)) {
        throw new Error(`missing position ${i}`);
      }
    }
  }
  if (currentTask && currentTask.startedAt === null) {
    throw new Error(`current task has no startedAt`);
  }
  for (const t of state.tasks) {
    if (t.id !== state.current && t.doneAt === null && t.startedAt !== null) {
      throw new Error(`non-current open task ${t.id} has startedAt set`);
    }
  }
}
```

### 2.3 `ts/queue.test.ts`

```typescript
import { describe, expect, test } from "bun:test";
import {
  add,
  assertInvariants,
  emptyQueue,
  fromTasks,
  getCompleted,
  getCurrent,
  getQueue,
  jump,
  pop,
  remove,
  reorder,
  setCurrent,
} from "./queue.ts";

const NOW = 1_700_000_000_000;

function task(id: string, title: string, position: number, overrides: Partial<{
  startedAt: number | null;
  doneAt: number | null;
}> = {}): any {
  return {
    id,
    title,
    note: "",
    position,
    createdAt: NOW,
    startedAt: overrides.startedAt ?? null,
    doneAt: overrides.doneAt ?? null,
  };
}

describe("emptyQueue", () => {
  test("starts empty", () => {
    const s = emptyQueue();
    expect(s.version).toBe(1);
    expect(s.current).toBeNull();
    expect(s.tasks).toHaveLength(0);
    assertInvariants(s);
  });
});

describe("add", () => {
  test("appends to end and does not affect current", () => {
    const s0 = emptyQueue();
    const s1 = add(s0, "first", NOW);
    const s2 = add(s1, "second", NOW);
    expect(s2.tasks).toHaveLength(2);
    expect(s2.tasks[0].title).toBe("first");
    expect(s2.tasks[1].title).toBe("second");
    expect(s2.current).toBeNull();
    assertInvariants(s2);
  });

  test("empty/whitespace title is a no-op", () => {
    const s0 = add(emptyQueue(), "real", NOW);
    const s1 = add(s0, "   ", NOW);
    expect(s1.tasks).toHaveLength(1);
    expect(s1).toBe(s0); // same reference returned
  });
});

describe("jump", () => {
  test("into empty queue becomes current", () => {
    const s = jump(emptyQueue(), "do the thing", NOW);
    expect(s.current).not.toBeNull();
    expect(getCurrent(s)?.title).toBe("do the thing");
    expect(s.tasks).toHaveLength(1);
    assertInvariants(s);
  });

  test("with current: previous current shifts to position 1", () => {
    let s = jump(emptyQueue(), "first", NOW);
    const firstId = s.current!;
    s = jump(s, "second", NOW + 1);
    expect(getCurrent(s)?.title).toBe("second");
    expect(getQueue(s)).toHaveLength(1);
    expect(getQueue(s)[0].id).toBe(firstId);
    assertInvariants(s);
  });

  test("three jumps: each previous shifts down by one, never lost", () => {
    let s = emptyQueue();
    const ids: string[] = [];
    for (const title of ["a", "b", "c"]) {
      s = jump(s, title, NOW);
      ids.push(s.current!);
    }
    expect(getCurrent(s)?.title).toBe("c");
    expect(getQueue(s).map((t) => t.title)).toEqual(["b", "a"]);
    assertInvariants(s);
  });

  test("empty title is a no-op", () => {
    const s0 = jump(emptyQueue(), "x", NOW);
    const s1 = jump(s0, "", NOW + 1);
    expect(s1).toBe(s0);
  });
});

describe("pop", () => {
  test("marks current done and promotes next", () => {
    let s = emptyQueue();
    s = jump(s, "first", NOW);
    s = jump(s, "second", NOW + 1);
    const secondId = s.current!;
    const firstId = getQueue(s)[0].id;
    s = pop(s, NOW + 2);
    expect(getCurrent(s)?.id).toBe(firstId);
    expect(getCompleted(s)[0].id).toBe(secondId);
    expect(getCompleted(s)[0].doneAt).toBe(NOW + 2);
    assertInvariants(s);
  });

  test("empty current is a no-op", () => {
    const s0 = add(emptyQueue(), "only", NOW);
    const s1 = pop(s0, NOW + 1);
    expect(s1).toBe(s0);
  });

  test("pop when nothing in queue: current becomes null", () => {
    let s = jump(emptyQueue(), "solo", NOW);
    s = pop(s, NOW + 1);
    expect(s.current).toBeNull();
    expect(getCompleted(s)).toHaveLength(1);
    assertInvariants(s);
  });
});

describe("remove", () => {
  test("removing a queued task leaves current alone", () => {
    let s = emptyQueue();
    s = jump(s, "a", NOW);
    s = jump(s, "b", NOW + 1);
    const aId = getQueue(s)[0].id;
    s = remove(s, aId);
    expect(getCurrent(s)?.title).toBe("b");
    expect(s.tasks).toHaveLength(1);
    assertInvariants(s);
  });

  test("removing current promotes position 1", () => {
    let s = emptyQueue();
    s = jump(s, "a", NOW);
    s = jump(s, "b", NOW + 1);
    const currentId = s.current!;
    const promotedId = getQueue(s)[0].id; // before remove, b is at position 1
    s = remove(s, currentId);
    expect(s.current).toBe(promotedId);
    assertInvariants(s);
  });

  test("removing current when alone sets current to null", () => {
    let s = jump(emptyQueue(), "solo", NOW);
    const id = s.current!;
    s = remove(s, id);
    expect(s.current).toBeNull();
    expect(s.tasks).toHaveLength(0);
    assertInvariants(s);
  });

  test("removing unknown id is a no-op", () => {
    const s0 = jump(emptyQueue(), "x", NOW);
    const s1 = remove(s0, "nonexistent");
    expect(s1).toBe(s0);
  });
});

describe("reorder", () => {
  test("moves a queued task up", () => {
    let s = emptyQueue();
    s = jump(s, "a", NOW);
    s = jump(s, "b", NOW + 1);
    s = jump(s, "c", NOW + 2);
    // queue is [b, a] (positions 1, 2)
    s = reorder(s, 1, 0);
    expect(getQueue(s).map((t) => t.title)).toEqual(["a", "b"]);
    assertInvariants(s);
  });

  test("out-of-bounds is a no-op", () => {
    let s = emptyQueue();
    s = jump(s, "a", NOW);
    s = jump(s, "b", NOW + 1);
    const before = s;
    expect(reorder(s, -1, 0)).toBe(s);
    expect(reorder(s, 0, 5)).toBe(s);
    expect(reorder(s, 5, 0)).toBe(s);
  });
});

describe("setCurrent", () => {
  test("promotes a queued task and demotes previous current to position 1", () => {
    let s = emptyQueue();
    s = jump(s, "a", NOW);
    s = jump(s, "b", NOW + 1);
    s = jump(s, "c", NOW + 2);
    // current is c, queue is [b, a]
    const aId = getQueue(s)[1].id;
    const cId = s.current!;
    s = setCurrent(s, aId, NOW + 3);
    expect(s.current).toBe(aId);
    expect(getQueue(s).map((t) => t.id)).toEqual([cId]);
    assertInvariants(s);
  });

  test("setting current to itself is a no-op", () => {
    let s = jump(emptyQueue(), "x", NOW);
    const id = s.current!;
    expect(setCurrent(s, id)).toBe(s);
  });

  test("cannot set current to a completed task", () => {
    let s = emptyQueue();
    s = jump(s, "a", NOW);
    s = jump(s, "b", NOW + 1);
    const bId = s.current!;
    s = pop(s, NOW + 2);
    expect(setCurrent(s, bId)).toBe(s); // unchanged
  });
});

describe("invariants", () => {
  test("assertInvariants catches bad state", () => {
    const bad = {
      version: 1,
      current: "x",
      tasks: [{ id: "x", title: "x", note: "", position: 0, createdAt: 0, startedAt: null, doneAt: null }],
    } as any;
    expect(() => assertInvariants(bad)).toThrow(/startedAt/);
  });
});
```

### 2.4 Verify

```bash
bun test
```

Expected:

```
bun test v1.x.x

 22 pass
 0 fail
```

### Done when

- `bun test` is green.
- Every operation is covered: empty case, single-item case, multi-item case.
- All test states pass `assertInvariants`.

---

## Appendix summary

After completing A you have:

- A symlinked, validated plugin that the Omarchy shell discovers.
- A tested pure-logic core that the rest of the plugin will compose on.

The next appendix, [`B-persistence.md`](B-persistence.md), puts a CLI in
front of this core and adds crash-safe writes to disk.
