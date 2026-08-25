# Appendix D — Shell Integration & Publish (steps 7, 8, 9)

Steps 7–9 bring the plugin onto the Omarchy bar, wire the system-wide
Hyprland hotkeys, and provide the validation checklist for marketplace
submission.

---

## Step 7 — Bar Widget (`BarWidget.qml`)

**Goal:** An always-visible focus chip on the Omarchy status bar that displays
the currently active task and summons the manager when clicked.

### 7.1 UX & Visual Design

```
+--- Omarchy Status Bar ------------------------------------------+
| ▌Workspaces   [ ▸ Focus: Write the plugin plan ]   Tray  Audio  |
+-----------------------------------------------------------------+
```

- **Active task present:** Displays an icon and truncated task title. Hovering
  shows the full title as a tooltip.
- **Empty queue (current is null):** Renders a subtle indicator (e.g. `Focus: idle`)
  or compact icon to keep the bar clean while remaining interactive.
- **Click action:** Summons the manage overlay by dispatching to
  `omarchy-shell shell toggle youn.focus-queue '{"mode":"manage"}'`.

### 7.2 `BarWidget.qml`

```qml
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import qs.Commons

BarWidget {
  id: root
  moduleName: "youn.focus-queue"

  property var state: ({ version: 1, current: null, tasks: [] })
  property var currentTask: getCurrentTask()

  function getCurrentTask() {
    if (!root.state || !root.state.current || !root.state.tasks) return null
    for (var i = 0; i < root.state.tasks.length; i++) {
      if (root.state.tasks[i].id === root.state.current) {
        return root.state.tasks[i]
      }
    }
    return null
  }

  function loadState(text) {
    try {
      var parsed = JSON.parse(text || "{}")
      if (parsed && parsed.version === 1) {
        root.state = parsed
        return
      }
    } catch (e) {}
    root.state = { version: 1, current: null, tasks: [] }
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

  Component.onCompleted: queueFile.reload()

  Rectangle {
    id: chip
    implicitWidth: chipRow.implicitWidth + Style.spacing.sm * 2
    implicitHeight: chipRow.implicitHeight + Style.spacing.xs * 2
    radius: Style.cornerRadius / 2
    color: mouseArea.containsMouse
           ? Color.menu.selectedBackground
           : Color.menu.background
    border.color: mouseArea.containsMouse
                  ? Color.menu.selectedBorder
                  : Color.menu.border

    RowLayout {
      id: chipRow
      anchors.centerIn: parent
      spacing: Style.spacing.xs

      Text {
        text: "▸"
        color: root.currentTask ? Color.menu.selectedText : Color.menu.text
        font.family: Style.font.menuFamily
        font.pixelSize: Style.font.caption
      }

      Text {
        text: root.currentTask ? root.currentTask.title : "idle"
        color: root.currentTask ? Color.menu.text : Qt.darker(Color.menu.text, 1.5)
        font.family: Style.font.menuFamily
        font.pixelSize: Style.font.caption
        elide: Text.ElideRight
        Layout.maximumWidth: Style.space(220)
      }
    }

    MouseArea {
      id: mouseArea
      anchors.fill: parent
      hoverEnabled: true
      cursorShape: Qt.PointingHandCursor
      onClicked: {
        Quickshell.execDetached([
          "omarchy-shell", "shell", "toggle",
          "youn.focus-queue", "{\"mode\":\"manage\"}"
        ])
      }
    }

    ToolTip.visible: mouseArea.containsMouse && root.currentTask
    ToolTip.text: root.currentTask ? root.currentTask.title : ""
    ToolTip.delay: 400
  }
}
```

---

## Step 8 — Hyprland Hotkeys & Install Hook

**Goal:** An idempotent hook script `hooks/install.sh` that configures the
four system-wide Hyprland keybindings.

### 8.1 Keybinding Map

| Shortcut | Action | Command Dispatched |
|---|---|---|
| `SUPER + T` | Pop (complete current) | `yfocus pop` |
| `SUPER + SHIFT + T` | Jump (insert at top) | `omarchy-shell shell toggle youn.focus-queue '{"mode":"jump"}'` |
| `SUPER + ALT + T` | Add (append to queue) | `omarchy-shell shell toggle youn.focus-queue '{"mode":"add"}'` |
| `SUPER + CTRL + T` | Manage (full manager) | `omarchy-shell shell toggle youn.focus-queue '{"mode":"manage"}'` |

### 8.2 `hooks/install.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

TARGET_CONF="${HOME}/.config/hypr/apps/yfocus-queue.conf"
MAIN_HYPR_CONF="${HOME}/.config/hypr/hyprland.conf"

echo "Configuring Hyprland bindings for youn.focus-queue..."
mkdir -p "$(dirname "$TARGET_CONF")"

cat > "$TARGET_CONF" << 'EOF'
# youn.focus-queue hotkeys
bind = SUPER, T, exec, yfocus pop
bind = SUPER SHIFT, T, exec, omarchy-shell shell toggle youn.focus-queue '{"mode":"jump"}'
bind = SUPER ALT, T, exec, omarchy-shell shell toggle youn.focus-queue '{"mode":"add"}'
bind = SUPER CTRL, T, exec, omarchy-shell shell toggle youn.focus-queue '{"mode":"manage"}'
EOF

# Ensure source line exists in main hyprland.conf if not already present
if [ -f "$MAIN_HYPR_CONF" ]; then
  if ! grep -q "source = ~/.config/hypr/apps/yfocus-queue.conf" "$MAIN_HYPR_CONF"; then
    echo "" >> "$MAIN_HYPR_CONF"
    echo "# Focus Queue Keybindings" >> "$MAIN_HYPR_CONF"
    echo "source = ~/.config/hypr/apps/yfocus-queue.conf" >> "$MAIN_HYPR_CONF"
  fi
fi

if command -v hyprctl &>/dev/null; then
  hyprctl reload || true
fi

echo "youn.focus-queue hotkeys installed."
```

Make it executable: `chmod +x hooks/install.sh`.

---

## Step 9 — Validation & Marketplace Polish

**Goal:** Ensure the plugin meets all standards for distribution and passes
automated checks.

### 9.1 TS ↔ JS Parity Test

Create a test in `ts/queue.test.ts` to guarantee that every operation in
`ts/queue.ts` exists in `FocusModel.js` with matching signatures:

```typescript
import { test, expect } from "bun:test";
import * as tsModel from "./queue.ts";

test("FocusModel.js exports match ts/queue.ts exports", async () => {
  // @ts-ignore
  const jsModel = await import("../FocusModel.js");
  const expectedFns = [
    "emptyQueue", "getCurrent", "getQueue", "getCompleted",
    "jump", "add", "pop", "remove", "reorder", "setCurrent", "clearCompleted"
  ];
  for (const fn of expectedFns) {
    expect(typeof (tsModel as any)[fn]).toBe("function");
    expect(typeof (jsModel as any)[fn]).toBe("function");
  }
});
```

### 9.2 Submission Checklist

- [ ] `bun test` passes (100% green on queue operations and invariant tests).
- [ ] `./build.sh` produces a valid `bin/yfocus` executable.
- [ ] `manifest.json` matches schema and passes `omarchy plugin validate ./yfocus`.
- [ ] `preview.png` is an accurate 16:9 screenshot showing the bar chip and manage overlay.
- [ ] `LICENSE` is present (MIT).
- [ ] `README.md` clearly lists keybindings and installation steps.
- [ ] Hyprland layer rules are updated for `omarchy-focus-queue` namespace.
