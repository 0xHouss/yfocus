import QtQuick
import Quickshell
import Quickshell.Io
import qs.Commons
import qs.Ui
import "FocusModel.js" as Model

// Bar chip: shows the task you are currently working on. Click opens the
// manage overlay. Reads queue.json via FileView watch; never writes.
BarWidget {
  id: root
  moduleName: "youn.focus-queue"

  property var state: Model.emptyQueue()
  readonly property var currentTask: Model.getCurrent(state)
  readonly property bool hasTask: !!currentTask

  function loadState(raw) {
    try {
      var parsed = JSON.parse(String(raw || ""))
      if (parsed && parsed.version === 1 && Array.isArray(parsed.tasks)) return parsed
    } catch (e) {}
    return Model.emptyQueue()
  }

  // Shape contract for shell.summon/hide/toggle routing: Bar.findPanelWidget
  // requires open/close/opened on the bar-widget root. The panel lives in
  // another process surface (the overlay plugin), so these forward over IPC.
  readonly property bool opened: false

  function open() { summonManage() }
  function close() { hideManage() }
  function toggle() { toggleManage() }

  function summonManage() {
    Quickshell.execDetached([
      "omarchy-shell", "shell", "summon",
      "youn.focus-queue", JSON.stringify({ mode: "manage" })
    ])
  }

  function toggleManage() {
    Quickshell.execDetached([
      "omarchy-shell", "shell", "toggle",
      "youn.focus-queue", JSON.stringify({ mode: "manage" })
    ])
  }

  function hideManage() {
    Quickshell.execDetached(["omarchy-shell", "shell", "hide", "youn.focus-queue"])
  }

  FileView {
    id: queueFile
    path: (Quickshell.env("XDG_STATE_HOME") || Quickshell.env("HOME") + "/.local/state")
          + "/omarchy/yfocus-queue/queue.json"
    watchChanges: true
    atomicWrites: true
    printErrors: false
    onLoaded: root.state = root.loadState(text())
    onLoadFailed: root.state = Model.emptyQueue()
    onFileChanged: reload()
  }

  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight

  WidgetButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    text: {
      if (root.vertical) return root.hasTask ? "▸" : ""
      if (root.hasTask) return "▸ " + root.currentTask.title
      return setting("idleLabel", "▸ focus")
    }
    tooltipText: root.hasTask ? root.currentTask.title : ""
    dimmed: !root.hasTask

    onPressed: function (b) {
      if (b === Qt.RightButton) {
        // Right click pops from anywhere — no need to open the manager.
        Quickshell.execDetached([root.executablePath(), "pop"])
      } else {
        root.toggleManage()
      }
    }
  }

  // Bundled CLI path, resolved once; falls back to PATH when missing.
  function executablePath() {
    if (root._binPath) return root._binPath
    var url = Qt.resolvedUrl("bin/yfocus").toString()
    var p = url.replace(/^file:\/\//, "")
    try { p = decodeURIComponent(p) } catch (e) {}
    root._binPath = p
    return p
  }
  property string _binPath: ""
}
