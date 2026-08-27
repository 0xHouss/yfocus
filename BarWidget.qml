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
  moduleName: "You-ne5.yfocus"

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
      "You-ne5.yfocus", JSON.stringify({ mode: "manage" })
    ])
  }

  function toggleManage() {
    Quickshell.execDetached([
      "omarchy-shell", "shell", "toggle",
      "You-ne5.yfocus", JSON.stringify({ mode: "manage" })
    ])
  }

  function hideManage() {
    Quickshell.execDetached(["omarchy-shell", "shell", "hide", "You-ne5.yfocus"])
  }

  FileView {
    id: queueFile
    path: (Quickshell.env("XDG_STATE_HOME") || Quickshell.env("HOME") + "/.local/state")
          + "/omarchy/yfocus/queue.json"
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

  // Bundled CLI path — prefers arch-specific musl binary, falls back to shim,
  // then to `yfocus` on PATH (e.g. `cargo install`). No auto-build: the plugin
  // bundles `bin/yfocus-<arch>` via `make bundle`; see obsidian-daily-qs.
  function decodeFileUrl(url) {
    var p = String(url).replace(/^file:\/\//, "")
    try { return decodeURIComponent(p) } catch (e) { return p }
  }
  property string _binPath: ""
  function executablePath() {
    if (root._binPath) return root._binPath
    // Prefer YFOCUS_BIN override, else bundled arch binary, else shim, else PATH
    var override = Quickshell.env("YFOCUS_BIN")
    if (override) { root._binPath = override; return override }
    // Bundled binaries are resolved relative to this QML file's directory
    var base = decodeFileUrl(Qt.resolvedUrl("bin/yfocus").toString())
    // Arch-specific first (set by hostArch below), then shim
    if (root._archBundled && root._archBundled !== "") {
      root._binPath = root._archBundled
      return root._binPath
    }
    root._binPath = base
    return base
  }
  // Migrate legacy state from yfocus-queue → yfocus (one-shot)
  Process {
    id: legacyMigrate
    command: ["bash", "-c", "old=\"${XDG_STATE_HOME:-$HOME/.local/state}/omarchy/yfocus-queue/queue.json\"; new=\"${XDG_STATE_HOME:-$HOME/.local/state}/omarchy/yfocus/queue.json\"; test -f \"$old\" && test ! -f \"$new\" && mkdir -p \"$(dirname \"$new\")\" && cp \"$old\" \"$new\" || true"]
    onExited: function(code) { queueFile.reload() }
  }
  // Arch detection (once at startup) — mirrors obsidian-daily-qs
  property string hostArch: ""
  property string _archBundled: ""
  property string _archCand: ""
  Process {
    id: archProbe
    command: ["uname", "-m"]
    onExited: function(code) {
      if (code !== 0) return
      var arch = String(stdout).trim()
      root.hostArch = arch
      if (arch === "x86_64" || arch === "aarch64") {
        root._archCand = decodeFileUrl(Qt.resolvedUrl("bin/yfocus-" + arch).toString())
        archCheck.running = true
      }
    }
  }
  Process {
    id: archCheck
    command: ["test", "-x", root._archCand]
    onExited: function(code) {
      if (code === 0) {
        root._archBundled = root._archCand
        if (!Quickshell.env("YFOCUS_BIN")) root._binPath = root._archCand
      } else {
        root._archBundled = ""
      }
    }
  }
  Component.onCompleted: {
    legacyMigrate.running = true
    archProbe.running = true
  }
}
