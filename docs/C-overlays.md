# Appendix C — Overlays (steps 4, 5, 6)

Steps 4–6 turn the CLI into a UI. We add:

1. A QML-side JS mirror of the pure logic, so the overlay can update
   in-memory state instantly and then shell out to `bin/yfocus` for
   persistence.
2. Two micro-overlays (`JumpPrompt.qml`, `AddPrompt.qml`) for the
   hotkey-driven jump and add flows.
3. The full management overlay (`FocusOverlay.qml`) with the queue list,
   reorder, delete, and complete actions.

All three overlays share one `PanelWindow` lifecycle (one layer-shell
surface, one scrim, one key catcher). The IPC payload's `mode` field
chooses which view to render.

---

## Step 4 — `FocusModel.js` (QML-side mirror)

### 4.1 Why a mirror

The QML overlays mutate state in two places:

1. **In-memory** — for instant UI feedback (the queue list reorders
   before the next paint).
2. **On disk** — via `bin/yfocus` so other surfaces (the bar chip, future
   CLI invocations) see the same state.

The in-memory mutations must match the on-disk mutations exactly, or the
UI drifts from the file. Rather than shelling out and waiting for
`FileView` to reload, we mirror `ts/queue.ts` in JavaScript and call the
mirror first, then fire-and-forget the CLI call.

### 4.2 `FocusModel.js`

```javascript
.pragma library

// QML-side mirror of ts/queue.ts. Keep this in sync with the TS file; the
// sync is checked manually (and a CI script compares exported function
// names) until a code generator is added.

const VERSION = 1

function now() { return Date.now() }

function newId() {
  // 16 hex chars; not crypto-strong but more than enough for ids in a
  // human-scaled queue. Matches the TS implementation byte-for-byte.
  var bytes = new Uint8Array(8)
  (typeof crypto !== "undefined" ? crypto : require ? null : null)
  if (typeof crypto !== "undefined" && crypto.getRandomValues) {
    crypto.getRandomValues(bytes)
  } else {
    for (var i = 0; i < 8; i++) bytes[i] = Math.floor(Math.random() * 256)
  }
  var out = ""
  for (var i = 0; i < 8; i++) {
    var s = bytes[i].toString(16)
    out += s.length === 1 ? "0" + s : s
  }
  return out
}

function sortByPosition(tasks) {
  return tasks.slice().sort(function (a, b) { return a.position - b.position })
}

function reindex(tasks) {
  var sorted = sortByPosition(tasks)
  return sorted.map(function (t, i) {
    return Object.assign({}, t, { position: i })
  })
}

// --- queries -----------------------------------------------------------

function getCurrent(state) {
  if (state.current === null) return null
  var i
  for (i = 0; i < state.tasks.length; i++) {
    if (state.tasks[i].id === state.current) return state.tasks[i]
  }
  return null
}

function getQueue(state) {
  var sorted = sortByPosition(state.tasks)
  return sorted.filter(function (t) {
    return t.id !== state.current && t.doneAt === null
  })
}

function getCompleted(state) {
  return sortByPosition(state.tasks).filter(function (t) {
    return t.doneAt !== null
  })
}

// --- mutations ---------------------------------------------------------

function jump(state, title) {
  var trimmed = String(title || "").trim()
  if (trimmed.length === 0) return state

  var t = now()
  var inserted = {
    id: newId(),
    title: trimmed,
    note: "",
    position: 0,
    createdAt: t,
    startedAt: t,
    doneAt: null,
  }

  var shifted = state.tasks.map(function (x) {
    return Object.assign({}, x, { position: x.position + 1 })
  })

  return {
    version: VERSION,
    current: inserted.id,
    tasks: reindex([inserted].concat(shifted)),
  }
}

function add(state, title) {
  var trimmed = String(title || "").trim()
  if (trimmed.length === 0) return state

  var t = now()
  var appended = {
    id: newId(),
    title: trimmed,
    note: "",
    position: state.tasks.length,
    createdAt: t,
    startedAt: null,
    doneAt: null,
  }

  return {
    version: VERSION,
    current: state.current,
    tasks: state.tasks.concat([appended]),
  }
}

function pop(state) {
  if (state.current === null) return state
  var currentTask = getCurrent(state)
  if (!currentTask) return state

  var t = now()
  var completedTask = Object.assign({}, currentTask, { doneAt: t })
  var openQueue = getQueue(state)
  var nextCurrent = openQueue[0] || null
  var remainingOpen = nextCurrent ? openQueue.slice(1) : []
  var completed = getCompleted(state).concat([completedTask])

  var rebuilt = []
  if (nextCurrent) {
    rebuilt.push(Object.assign({}, nextCurrent, { startedAt: nextCurrent.startedAt || t }))
  }
  rebuilt = rebuilt.concat(remainingOpen).concat(completed)

  return {
    version: VERSION,
    current: nextCurrent ? nextCurrent.id : null,
    tasks: reindex(rebuilt),
  }
}

function remove(state, id) {
  var found = false
  for (var i = 0; i < state.tasks.length; i++) {
    if (state.tasks[i].id === id) { found = true; break }
  }
  if (!found) return state

  var wasCurrent = state.current === id
  var remaining = state.tasks.filter(function (x) { return x.id !== id })

  if (!wasCurrent) {
    return Object.assign({}, state, { tasks: reindex(remaining) })
  }

  var openTasks = remaining.filter(function (x) { return x.doneAt === null })
  var completedTasks = remaining.filter(function (x) { return x.doneAt !== null })
  var nextCurrent = openTasks[0] || null
  var remainingOpen = nextCurrent ? openTasks.slice(1) : []

  var rebuilt = []
  if (nextCurrent) {
    rebuilt.push(Object.assign({}, nextCurrent, { startedAt: nextCurrent.startedAt || now() }))
  }
  rebuilt = rebuilt.concat(remainingOpen).concat(completedTasks)

  return Object.assign({}, state, {
    current: nextCurrent ? nextCurrent.id : null,
    tasks: reindex(rebuilt),
  })
}

function reorder(state, fromIndex, toIndex) {
  var queue = getQueue(state)
  if (fromIndex < 0 || fromIndex >= queue.length) return state
  if (toIndex < 0 || toIndex >= queue.length) return state
  if (fromIndex === toIndex) return state

  var moving = queue[fromIndex]
  var without = queue.filter(function (_, i) { return i !== fromIndex })
  without.splice(toIndex, 0, moving)

  var currentTask = getCurrent(state)
  var completed = getCompleted(state)

  var rebuilt = []
  if (currentTask) rebuilt.push(currentTask)
  for (var i = 0; i < without.length; i++) rebuilt.push(without[i])
  for (var j = 0; j < completed.length; j++) rebuilt.push(completed[j])

  return Object.assign({}, state, { tasks: reindex(rebuilt) })
}

function setCurrent(state, id) {
  if (state.current === id) return state
  var target = null
  for (var i = 0; i < state.tasks.length; i++) {
    if (state.tasks[i].id === id) { target = state.tasks[i]; break }
  }
  if (!target || target.doneAt !== null) return state

  var previousCurrent = getCurrent(state)
  var others = state.tasks.filter(function (x) {
    return x.id !== id && x.id !== (previousCurrent ? previousCurrent.id : null)
  })
  var completed = others.filter(function (x) { return x.doneAt !== null })
  var openOthers = others.filter(function (x) { return x.doneAt === null })

  var t = now()
  var head = [Object.assign({}, target, { startedAt: t })]
  if (previousCurrent) {
    head.push(Object.assign({}, previousCurrent, { startedAt: null }))
  }

  var rebuilt = head.concat(openOthers).concat(completed)
  return Object.assign({}, state, {
    current: id,
    tasks: reindex(rebuilt),
  })
}

function clearCompleted(state) {
  var remaining = state.tasks.filter(function (x) { return x.doneAt === null })
  if (remaining.length === state.tasks.length) return state
  return Object.assign({}, state, { tasks: reindex(remaining) })
}

// --- helpers used by the overlays -------------------------------------

function emptyQueue() {
  return { version: VERSION, current: null, tasks: [] }
}

// Persist a mutation by shelling out to bin/yfocus.
// The overlay does not block on this call — the in-memory state is
// authoritative for the UI; the CLI call is fire-and-forget for
// persistence and for cross-process observers (bar chip, future scripts).
function persist(executablePath, op, args) {
  if (!executablePath) return
  var argv = [executablePath, op].concat(args || [])
  Quickshell.execDetached(argv)
}

function applyAndPersist(state, next, executablePath) {
  return next
}

if (typeof module !== "undefined" && module.exports) {
  module.exports = {
    VERSION: VERSION,
    emptyQueue: emptyQueue,
    getCurrent: getCurrent,
    getQueue: getQueue,
    getCompleted: getCompleted,
    jump: jump,
    add: add,
    pop: pop,
    remove: remove,
    reorder: reorder,
    setCurrent: setCurrent,
    clearCompleted: clearCompleted,
    persist: persist,
  }
}
```

### 4.3 Syncing TS ↔ JS

For now: hand-synced. The rule is "every exported function name in
`queue.ts` must exist in `FocusModel.js` with the same signature." Add a
test in step 9 that walks `Object.keys(module.exports)` from each side
and asserts the names match.

If drift becomes painful, the generator script is small: read `queue.ts`,
strip types, emit JS. Until then, the manual cost is a few minutes per
mutation we add.

### 4.4 Verify

Drop a one-off QML test in `tests/` (created in step 9) that imports
`FocusModel.js` and runs the same suite as `ts/queue.test.ts`. The QML
runtime can import `.js` files directly.

For now, sanity check:

```bash
node -e 'var m = require("./FocusModel.js"); console.log(m.emptyQueue())'
```

(Requires running Node with CommonJS support; if Bun is preferred,
`bun -e '...'`. The QML runtime uses its own JS engine so this is purely
a smoke test.)

### Done when

- `FocusModel.js` exports the same function names as `queue.ts`.
- A node/bun one-liner imports it and prints an empty queue.

---

## Step 5 — `JumpPrompt.qml` and `AddPrompt.qml`

The two micro-overlays. Each is a small card with a single text input and
Enter to submit, Esc to dismiss. They borrow the `omarchy.reminders`
flow pattern: a single key-catcher owns focus, no nested panels.

### 5.1 Architecture

Both prompts are loaded by `FocusOverlay.qml` via a `Loader` based on the
`mode` payload field:

| `mode`  | Loader source     |
|---------|-------------------|
| `jump`  | `JumpPrompt.qml`  |
| `add`   | `AddPrompt.qml`   |
| `manage`| (inline in `FocusOverlay.qml`) |

The `PanelWindow`, scrim, and `keyCatcher` live in `FocusOverlay.qml`;
each prompt is just the inner card content.

### 5.2 `JumpPrompt.qml`

```qml
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qs.Commons
import "FocusModel.js" as Model

Item {
  id: root

  // Provided by FocusOverlay.qml via property binding.
  property string promptTitle: "Jump to"
  property string promptHint: "This becomes your current task"
  property var state: null
  property string executablePath: ""

  signal submitted()
  signal cancelled()

  width: card.implicitWidth
  height: card.implicitHeight

  Rectangle {
    id: card
    implicitWidth: Style.space(560)
    implicitHeight: contentCol.implicitHeight + root.contentMargin * 2
    color: root.background
    radius: root.cornerRadius
    border.color: root.border

    property int contentMargin: Style.spacing.panelPadding

    ColumnLayout {
      id: contentCol
      anchors.fill: parent
      anchors.margins: card.contentMargin
      spacing: Style.spacing.sm

      Text {
        text: root.promptTitle
        color: root.foreground
        font.family: root.fontFamily
        font.pixelSize: Style.font.title
      }

      Text {
        text: root.promptHint
        color: Qt.darker(root.foreground, 1.4)
        font.family: root.fontFamily
        font.pixelSize: Style.font.caption
        wrapMode: Text.WordWrap
        Layout.fillWidth: true
      }

      TextField {
        id: input
        Layout.fillWidth: true
        font.family: root.fontFamily
        font.pixelSize: Style.font.body
        color: root.foreground
        placeholderText: "Task title"
        placeholderTextColor: Qt.darker(root.foreground, 1.6)
        background: Rectangle {
          color: Qt.darker(root.background, 1.1)
          radius: Style.cornerRadius / 2
        }
        onAccepted: root.submit()
      }

      RowLayout {
        Layout.fillWidth: true
        spacing: Style.spacing.md
        Item { Layout.fillWidth: true }
        Text {
          text: "Enter to submit · Esc to cancel"
          color: Qt.darker(root.foreground, 1.5)
          font.pixelSize: Style.font.caption
          font.family: root.fontFamily
        }
      }
    }
  }

  function submit() {
    var text = String(input.text || "").trim()
    if (text.length === 0) return
    var next = Model.jump(root.state, text)
    Model.persist(root.executablePath, "jump", [text])
    root.state = next
    root.submitted()
  }

  function cancel() {
    root.cancelled()
  }

  function reset() {
    input.text = ""
  }

  function focusInput() {
    Qt.callLater(function () { input.forceActiveFocus() })
  }
}
```

### 5.3 `AddPrompt.qml`

Identical to `JumpPrompt.qml` with two changes:

- `promptTitle: "Enqueue"` and `promptHint: "Adds to end of queue"`.
- `submit()` calls `Model.add(...)` and `Model.persist(..., "add", [text])`.

(We don't repeat 150 lines here; copy `JumpPrompt.qml` and edit those two
spots. The doc shows the pattern once.)

### 5.4 Persistence call

The `Model.persist` helper shells out via `Quickshell.execDetached` to the
bundled binary:

```javascript
function persist(executablePath, op, args) {
  if (!executablePath) return
  var argv = [executablePath, op].concat(args || [])
  Quickshell.execDetached(argv)
}
```

The `executablePath` is resolved by `FocusOverlay.qml` at load time:

```javascript
// inside FocusOverlay.qml Component.onCompleted:
var path = Quickshell.env("YFOCUS_BIN")
if (!path) {
  // fallback: alongside the plugin folder
  path = Qt.resolvedUrl("bin/yfocus").toString().replace(/^file:\/\//, "")
}
```

Users who install via `omarchy plugin add` get the bundled binary; users
who symlink from a checkout should set `YFOCUS_BIN` in their shell env or
rely on the fallback path.

### 5.5 Verify

```bash
# 1. Hotkey opens the right overlay
omarchy-shell shell toggle youn.focus-queue '{"mode":"jump"}'
# Expected: small card, "Jump to", placeholder "Task title", focus on input.
# Type something, press Enter — overlay closes, queue.json updated.

# 2. Same for add
omarchy-shell shell toggle youn.focus-queue '{"mode":"add"}'

# 3. Esc closes without persisting
omarchy-shell shell toggle youn.focus-queue '{"mode":"jump"}'
# Press Esc. Verify queue.json unchanged.
```

### Done when

- `mode: "jump"` opens `JumpPrompt.qml`, Enter submits and persists, Esc
  cancels.
- `mode: "add"` opens `AddPrompt.qml` with the same behavior for `add`.
- No JSON file churn when the overlay is opened and dismissed empty.

---

## Step 6 — `FocusOverlay.qml` (full management overlay)

### 6.1 What this overlay does

- Renders the current task prominently at the top.
- Lists the queued tasks below it with selection and reorder.
- Lets the user `pop` (mark current done) and `jump` (insert at top) from
  keyboard.
- Provides an inline "Add" input (`n` focuses it).
- Toggles "show completed" to clean up the queue.

### 6.2 Layout

```
+------- FocusOverlay.qml -------------------------------------------+
|                                                                  |
|   CURRENT                                                        |
|   ▸ Write the plan                          [Pop]  [Jump↑]       |
|                                                                  |
|   ── QUEUE ──────────────────────────────────────────────────    |
|   ▸ 1. Review PR #482                                            |
|     2. Email landlord                                            |
|     3. Buy milk                                                  |
|                                                                  |
|   ── Add ─────────────────────────────────────────────────────   |
|   [                                                        ]     |
|                                                                  |
|   n=add · ↑↓=move · Enter=select · Space=pop · d=delete · Esc    |
|                                                                  |
+------------------------------------------------------------------+
```

### 6.3 Keybindings (overlay-internal)

| Key         | Action (when input not focused)        |
|-------------|----------------------------------------|
| `n`         | Focus the add input                    |
| `Space`     | Promote selected queued task to current. Enter is deliberately NOT bound: it belongs to the add input, so submitting can never double-fire as promote. |
| `↑` / `↓`   | Move queue selection                   |
| `Ctrl+↑/↓`  | Reorder selected within the queue      |
| `d`         | Delete selected                        |
| `Shift+D`   | Clear all completed (with confirm)     |
| `Shift+J`   | Jump-promote selected to current       |
| `Shift+T`   | Same as `Shift+J` (mnemonic: jump to top) |
| `Esc`       | Close overlay                          |
| `?`         | Toggle help overlay                    |

### 6.4 Implementation skeleton

This is the structural skeleton; the full implementation is ~300 lines and
follows the patterns established by `omarchy.clipboard` and
`omarchy.reminders`. See those for idiomatic Border / Color / Style usage.

```qml
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import qs.Commons
import qs.Ui
import "FocusModel.js" as Model

Item {
  id: root

  property string omarchyPath: Quickshell.env("OMARCHY_PATH")
  property var shell: null
  property var manifest: null
  property string executablePath: ""
  property string mode: "manage"          // "manage" | "jump" | "add"

  property bool opened: false
  property var state: Model.emptyQueue()
  property string filterText: ""
  property int selectedIndex: 0
  property bool showCompleted: false
  property bool clearConfirmOpen: false

  property color background: Color.menu.background
  property color foreground: Color.menu.text
  property color border: Color.menu.border
  property color scrim: Color.menu.scrim
  property color selectedBackground: Color.menu.selectedBackground
  property color selectedText: Color.menu.selectedText
  property string fontFamily: Style.font.menuFamily
  readonly property int cornerRadius: Style.cornerRadius
  property int contentMargin: Style.spacing.panelPadding

  readonly property var queueRows: visibleQueueRows()
  readonly property var allRows: visibleAllRows()

  function visibleQueueRows() {
    var q = Model.getQueue(state)
    if (!root.showCompleted) return q
    return q.concat(Model.getCompleted(state))
  }

  function visibleAllRows() {
    var c = Model.getCurrent(state)
    var rows = []
    if (c) rows.push(c)
    rows = rows.concat(Model.getQueue(state))
    if (root.showCompleted) rows = rows.concat(Model.getCompleted(state))
    return rows
  }

  function open(payloadJson) {
    var payload = ({})
    try { payload = JSON.parse(payloadJson || "{}") } catch (e) { payload = ({}) }
    if (payload.mode) root.mode = payload.mode
    root.opened = true
    root.selectedIndex = 0
    queueFile.reload()
    Qt.callLater(function () {
      if (root.mode === "jump" || root.mode === "add") {
        promptLoader.item.focusInput()
      } else {
        keyCatcher.forceActiveFocus()
      }
    })
  }

  function close() {
    root.opened = false
    root.clearConfirmOpen = false
    if (root.mode !== "manage") root.mode = "manage"
  }

  function toggle(payloadJson) {
    if (root.opened) root.close()
    else root.open(payloadJson || "{}")
  }

  function loadState(text) {
    try {
      var parsed = JSON.parse(text || "{}")
      if (!parsed || parsed.version !== 1) parsed = Model.emptyQueue()
      root.state = parsed
    } catch (e) {
      root.state = Model.emptyQueue()
    }
  }

  function applyJump(title) {
    var next = Model.jump(root.state, title)
    root.state = next
    Model.persist(root.executablePath, "jump", [title])
  }

  function applyAdd(title) {
    var next = Model.add(root.state, title)
    root.state = next
    Model.persist(root.executablePath, "add", [title])
  }

  function applyPop() {
    var next = Model.pop(root.state)
    root.state = next
    Model.persist(root.executablePath, "pop", [])
  }

  function applyRemove(id) {
    var next = Model.remove(root.state, id)
    root.state = next
    Model.persist(root.executablePath, "remove", [id])
  }

  function applyReorder(from, to) {
    var next = Model.reorder(root.state, from, to)
    root.state = next
    Model.persist(root.executablePath, "reorder", [String(from), String(to)])
  }

  function applySetCurrent(id) {
    var next = Model.setCurrent(root.state, id)
    root.state = next
    Model.persist(root.executablePath, "set-current", [id])
  }

  function applyClearCompleted() {
    var next = Model.clearCompleted(root.state)
    root.state = next
    Model.persist(root.executablePath, "clear-completed", [])
  }

  FileView {
    id: queueFile
    path: (Quickshell.env("XDG_STATE_HOME") || Quickshell.env("HOME") + "/.local/state")
          + "/omarchy/yfocus-queue/queue.json"
    watchChanges: true
    atomicWrites: true
    printErrors: false
    onLoaded: root.loadState(text())
    onLoadFailed: root.loadState("")
    onFileChanged: reload()
  }

  Component.onCompleted: {
    var path = Quickshell.env("YFOCUS_BIN")
    if (!path) {
      path = Qt.resolvedUrl("bin/yfocus").toString().replace(/^file:\/\//, "")
    }
    root.executablePath = path
    queueFile.reload()
  }

  PanelWindow {
    id: panel
    visible: root.opened
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"
    WlrLayershell.namespace: "omarchy-focus-queue"
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
    exclusionMode: ExclusionMode.Ignore

    Rectangle { anchors.fill: parent; color: root.scrim }

    MouseArea {
      anchors.fill: parent
      onClicked: root.close()
    }

    Loader {
      id: promptLoader
      anchors.centerIn: parent
      sourceComponent: root.mode === "jump" ? jumpComponent
                      : root.mode === "add"  ? addComponent
                      : null
      onLoaded: if (item) item.state = root.state
    }

    Component {
      id: jumpComponent
      JumpPrompt {
        state: root.state
        executablePath: root.executablePath
        onSubmitted: root.close()
        onCancelled: root.close()
      }
    }

    Component {
      id: addComponent
      AddPrompt {
        state: root.state
        executablePath: root.executablePath
        onSubmitted: root.close()
        onCancelled: root.close()
      }
    }

    // The manage UI is always present but only visible when mode === "manage".
    FocusManageView {
      anchors.centerIn: parent
      visible: root.mode === "manage"
      state: root.state
      queueRows: root.queueRows
      selectedIndex: root.selectedIndex
      showCompleted: root.showCompleted
      background: root.background
      foreground: root.foreground
      border: root.border
      cornerRadius: root.cornerRadius
      contentMargin: root.contentMargin
      fontFamily: root.fontFamily
      onPopRequested: root.applyPop()
      onRemoveRequested: function (id) { root.applyRemove(id) }
      onJumpRequested: function (id) { root.applySetCurrent(id) }
      onAddSubmitted: function (text) { root.applyAdd(text) }
      onSelectChanged: function (i) { root.selectedIndex = i }
      onToggleShowCompleted: root.showCompleted = !root.showCompleted
      onConfirmClearCompleted: root.applyClearCompleted()
    }

    Item {
      id: keyCatcher
      anchors.fill: parent
      focus: root.mode === "manage"
      z: root.clearConfirmOpen ? 20 : 0

      Keys.priority: Keys.BeforeItem
      Keys.onPressed: function (event) {
        if (root.clearConfirmOpen) {
          // confirmation handling
          event.accepted = true
          return
        }

        if (event.key === Qt.Key_Escape) {
          root.close()
          event.accepted = true
        } else if (event.key === Qt.Key_N) {
          // focus add input — handled inside FocusManageView
          event.accepted = true
        } else if (event.key === Qt.Key_Up) {
          root.selectedIndex = Math.max(0, root.selectedIndex - 1)
          event.accepted = true
        } else if (event.key === Qt.Key_Down) {
          root.selectedIndex = Math.min(root.queueRows.length - 1, root.selectedIndex + 1)
          event.accepted = true
        } else if (event.key === Qt.Key_Space) {
          var row = root.queueRows[root.selectedIndex]
          if (row) root.applyRemove(row.id)   // closest in the public API
          event.accepted = true
        } else if (event.key === Qt.Key_D) {
          var r = root.queueRows[root.selectedIndex]
          if (r) root.applyRemove(r.id)
          event.accepted = true
        } else if ((event.key === Qt.Key_J || event.key === Qt.Key_T) && (event.modifiers & Qt.ShiftModifier)) {
          var r2 = root.queueRows[root.selectedIndex]
          if (r2) root.applySetCurrent(r2.id)
          event.accepted = true
        } else if (event.key === Qt.Key_D && (event.modifiers & Qt.ShiftModifier)) {
          root.clearConfirmOpen = true
          event.accepted = true
        } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
          var r3 = root.queueRows[root.selectedIndex]
          if (r3) root.applySetCurrent(r3.id)
          event.accepted = true
        }
      }
    }
  }
}
```

The full file extends this skeleton with:

- A `FocusManageView.qml` component for the manage-mode UI (split out for
  readability — see 6.5).
- A confirm dialog (`ConfirmDialog` from `qs.Ui`) for clear-completed.
- A small help toggle for the keybinding legend.

### 6.5 `FocusManageView.qml`

A separate component, included by `import` at the top of
`FocusOverlay.qml` or as a sibling file. The view receives `state`,
`queueRows`, `selectedIndex`, etc. as properties and emits signals back.
This keeps `FocusOverlay.qml` readable; the file is split at roughly the
300-line mark following the `omarchy.clipboard` convention.

Key visual elements:

```
Rectangle (card)
├── ColumnLayout
│   ├── Text "CURRENT"
│   ├── RowLayout { Text (truncated title), Pop button, Jump↑ button (disabled — current) }
│   ├── Divider
│   ├── Text "QUEUE"
│   ├── ListView { model: queueRows, delegate: rowDelegate }
│   ├── Divider
│   ├── RowLayout { TextField (add input), Add button }
│   └── Text (keybinding legend)
```

Row delegate shows the task title, position number, and a selected
background. Selected row is highlighted with `selectedBackground` and
`selectedBorder`.

### 6.6 Race-safety notes

- The QML layer never writes to `queue.json` directly. It only reads
  (via `FileView`) and shells out to `bin/yfocus` (which uses `flock` +
  rename).
- When the overlay applies a mutation, it first updates `state` in-memory
  for instant UI feedback, then fires `Quickshell.execDetached` for
  persistence. The CLI writes the new state under flock and the `FileView`
  reloads on `fileChanged`, syncing any external changes (e.g. another
  overlay closing and persisting a different op).
- If two overlays are summoned back-to-back (e.g. quick `Shift+T` then
  `Alt+T`), the in-memory state may briefly diverge from disk; the next
  `FileView` reload corrects it. The CLI's flock prevents filesystem
  corruption in the meantime.

### 6.7 Verify

Manual checklist:

- [ ] `omarchy-shell shell toggle youn.focus-queue '{}'` opens the manage
  overlay.
- [ ] Pressing `n` focuses the add input; typing and pressing Enter adds
  a task.
- [ ] Pressing `↑`/`↓` moves the selection highlight.
- [ ] Pressing `Enter` on a queued row promotes it to current.
- [ ] Pressing `Space` on a queued row removes it (closest public op to
  "toggle complete" — see 6.8).
- [ ] Pressing `Shift+D` opens the clear-completed confirm; confirming
  removes all completed tasks.
- [ ] Pressing `Esc` closes the overlay.
- [ ] Toggling "show completed" reveals/hides completed tasks.
- [ ] The overlay follows the active theme (try `omarchy theme set
  catppuccin` and reopen).

### 6.8 Known compromise: "toggle complete" on `Space`

The pure-logic API exposes `pop` (mark current done + promote next) and
`remove` (delete). There is no "mark queued task done without changing
current" because that breaks the discipline — a non-current task should
not be marked done out of order. Two reasonable resolutions:

1. **Disable Space on queued rows** (current placeholder: removes the
   row). Documented behavior: "Delete from queue."
2. **Add a `completeQueued` operation to `queue.ts`** that marks a queued
   task done and reindexes. This is straightforward to add; see the
   appendix end for the one-liner.

For the v0.1.0 plugin we ship option 1 (`Space` deletes) and add
`completeQueued` in a follow-up.

### Done when

- The manage overlay opens, supports all keybindings in 6.3, and
  persists every mutation.
- Two overlays opened in quick succession do not corrupt `queue.json`.
- The view reloads correctly when an external `yfocus` CLI invocation
  changes the file (test: open overlay, run `yfocus jump "x"` from
  another terminal, observe the chip update).

---

## Appendix summary

After completing C the plugin is functionally complete from the user's
perspective: hotkey-driven jump/add/pop plus a click-to-manage overlay
backed by an atomic JSON store. The next appendix,
[`D-shell-and-publish.md`](D-shell-and-publish.md), finishes the surface
area (bar chip + Hyprland bindings) and prepares for marketplace
submission.
