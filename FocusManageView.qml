import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import qs.Commons
import qs.Ui
import "FocusModel.js" as Model

// Manage-mode card: current task up top, the queued tasks below it, an
// add input at the bottom. All mutations flow through FocusOverlay.qml's
// apply* functions; this view only renders state and emits intents.
Item {
  id: root

  // Provided by FocusOverlay.qml.
  property var state: null
  property var queueRows: []
  property int selectedIndex: 0
  property bool showCompleted: false
  property color background: Color.menu.background
  property color foreground: Color.menu.text
  property color border: Color.menu.border
  property color selectedBackground: Color.menu.selectedBackground
  property color selectedText: Color.menu.selectedText
  property string fontFamily: Style.font.menuFamily
  property bool confirmOpen: false

  signal popRequested()
  signal removeRequested(string id)
  signal jumpRequested(string id)
  signal addSubmitted(string text)
  signal reorderRequested(int fromIndex, int toIndex)
  signal selectionChanged(int index)
  signal toggleShowCompleted()
  signal clearCompletedRequested()
  signal escapeRequested()

  readonly property var currentTask: Model.getCurrent(state)
  readonly property int rowCount: queueRows.length

  width: card.width
  height: card.height

  function focusAddInput() {
    Qt.callLater(function () { addInput.forceActiveFocus() })
  }

  // Start capturing a task title, seeding the field with the first
  // character so "type anywhere" flows don't drop it.
  function beginCapture(seed) {
    addInput.text = String(seed || "")
    Qt.callLater(function () {
      addInput.forceActiveFocus()
      addInput.cursorPosition = addInput.text.length
    })
  }

  Rectangle {
    id: card
    width: Style.space(620)
    height: Math.min(contentCol.implicitHeight + Style.spacing.panelPadding * 2, Screen.desktopAvailableHeight - Style.space(80))
    color: root.background
    radius: Style.cornerRadius
    border.width: Math.max(1, Style.space(2))
    border.color: root.border

    ColumnLayout {
      id: contentCol
      anchors.left: parent.left
      anchors.right: parent.right
      anchors.top: parent.top
      anchors.margins: Style.spacing.panelPadding
      spacing: Style.spacing.md

      // ---- current task -------------------------------------------

      ColumnLayout {
        Layout.fillWidth: true
        spacing: Style.spacing.xs

        Text {
          text: "CURRENT"
          color: Qt.darker(root.foreground, 1.5)
          font.family: root.fontFamily
          font.pixelSize: Style.font.caption
          font.letterSpacing: 1
        }

        RowLayout {
          Layout.fillWidth: true
          spacing: Style.spacing.sm

          Text {
            text: "▸"
            color: root.currentTask ? Color.accent : Qt.darker(root.foreground, 1.8)
            font.family: root.fontFamily
            font.pixelSize: Style.font.title
          }

          Text {
            id: currentTitle
            Layout.fillWidth: true
            text: root.currentTask ? root.currentTask.title : "Nothing in focus — press n to add a task"
            color: root.currentTask ? root.foreground : Qt.darker(root.foreground, 1.7)
            font.family: root.fontFamily
            font.pixelSize: Style.font.title
            elide: Text.ElideRight
            wrapMode: Text.NoWrap
          }
        }
      }

      Rectangle {
        Layout.fillWidth: true
        height: 1
        color: Qt.darker(root.border, 1.4)
        opacity: 0.35
      }

      // ---- queue header -------------------------------------------

      RowLayout {
        Layout.fillWidth: true
        spacing: Style.spacing.sm

        Text {
          text: "QUEUE"
          color: Qt.darker(root.foreground, 1.5)
          font.family: root.fontFamily
          font.pixelSize: Style.font.caption
          font.letterSpacing: 1
        }

        Item { Layout.fillWidth: true }

        Text {
          text: root.rowCount === 0 ? "empty" : (root.rowCount + " queued")
          color: Qt.darker(root.foreground, 1.7)
          font.family: root.fontFamily
          font.pixelSize: Style.font.caption
        }
      }

      // ---- queue list ---------------------------------------------

      ListView {
        id: resultList
        Layout.fillWidth: true
        implicitHeight: Math.min(contentHeight, Style.space(360))
        spacing: Style.spacing.xxs
        clip: true
        model: root.queueRows

        delegate: Rectangle {
          required property var modelData
          required property int index

          width: resultList.width
          height: rowLayout.implicitHeight + Style.spacing.sm * 2
          radius: Style.cornerRadius / 2
          color: index === root.selectedIndex ? root.selectedBackground : "transparent"
          border.width: index === root.selectedIndex ? 1 : 0
          border.color: root.selectedBackground

          RowLayout {
            id: rowLayout
            anchors.fill: parent
            anchors.margins: Style.spacing.sm / 2 * 2
            spacing: Style.spacing.sm

            Text {
              text: String(index + 1)
              color: Qt.darker(root.foreground, 1.6)
              font.family: root.fontFamily
              font.pixelSize: Style.font.caption
              Layout.preferredWidth: Style.space(18)
              horizontalAlignment: Text.AlignRight
            }

            Text {
              Layout.fillWidth: true
              text: modelData.title
              color: index === root.selectedIndex ? root.selectedText : root.foreground
              font.family: root.fontFamily
              font.pixelSize: Style.font.body
              elide: Text.ElideRight
              wrapMode: Text.NoWrap
            }
          }

          MouseArea {
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: function (mouse) {
              if (root.selectedIndex !== index) {
                root.selectionChanged(index)
              } else {
                root.jumpRequested(modelData.id)
              }
            }
          }
        }

        Text {
          anchors.centerIn: parent
          visible: root.rowCount === 0
          text: "Queue is empty"
          color: Qt.darker(root.foreground, 1.8)
          font.family: root.fontFamily
          font.pixelSize: Style.font.caption
        }

        onVisibleChanged: if (visible && root.rowCount > 0) positionViewAtIndex(root.selectedIndex, ListView.Contain)
        onCountChanged: if (count > 0) positionViewAtIndex(Math.min(root.selectedIndex, count - 1), ListView.Contain)
      }

      // ---- completed section (toggleable) --------------------------

      ColumnLayout {
        visible: root.showCompleted && Model.getCompleted(root.state).length > 0
        Layout.fillWidth: true
        spacing: Style.spacing.xxs

        Repeater {
          model: root.showCompleted ? Model.getCompleted(root.state) : []

          delegate: RowLayout {
            required property var modelData
            required property int index
            Layout.fillWidth: true
            spacing: Style.spacing.sm

            Text {
              text: "✓"
              color: Qt.darker(root.foreground, 1.9)
              font.family: root.fontFamily
              font.pixelSize: Style.font.caption
            }

            Text {
              Layout.fillWidth: true
              text: modelData.title
              color: Qt.darker(root.foreground, 1.9)
              font.family: root.fontFamily
              font.pixelSize: Style.font.body
              elide: Text.ElideRight
            }

            Text {
              text: "(done — Shift+D clears)"
              color: Qt.darker(root.foreground, 2.2)
              font.family: root.fontFamily
              font.pixelSize: Style.font.caption
            }
          }
        }
      }

      Rectangle {
        Layout.fillWidth: true
        height: 1
        color: Qt.darker(root.border, 1.4)
        opacity: 0.35
      }

      // ---- add input -----------------------------------------------

      TextField {
        id: addInput
        Layout.fillWidth: true
        font.family: root.fontFamily
        font.pixelSize: Style.font.body
        color: root.foreground
        placeholderText: "n then type to add a task…"
        placeholderTextColor: Qt.darker(root.foreground, 1.6)
        background: Rectangle {
          color: Qt.darker(root.background, 1.08)
          radius: Style.cornerRadius / 2
          border.width: 1
          border.color: addInput.activeFocus ? Color.accent : root.border
        }

        function submitCurrent() {
          var text = String(addInput.text || "").trim()
          if (text.length > 0) root.addSubmitted(text)
          addInput.text = ""
          // Keep focus in the input for rapid entry; Esc leaves.
        }

        // Single deterministic submit path: intercept Return/Enter BEFORE
        // TextField's builtin handling (BeforeItem) and consume them here.
        // No onAccepted fallback — mixing the two let one Enter press
        // bubble to keyCatcher's promote binding as well.
        Keys.priority: Keys.BeforeItem
        Keys.onReturnPressed: function (event) {
          addInput.submitCurrent()
          event.accepted = true
        }
        Keys.onEnterPressed: function (event) {
          addInput.submitCurrent()
          event.accepted = true
        }
        Keys.onEscapePressed: function (event) {
          addInput.text = ""
          root.escapeRequested()
          event.accepted = true
        }
        Keys.onUpPressed: function (event) { event.accepted = false }
        Keys.onDownPressed: function (event) { event.accepted = false }
      }

      // ---- legend ---------------------------------------------------

      Text {
        Layout.fillWidth: true
        text: "p pop · Space promote · d delete · ↑↓ select · Ctrl+↑↓ reorder · s completed · Shift+D clear done · n add · Esc close"
        color: Qt.darker(root.foreground, 1.7)
        font.family: root.fontFamily
        font.pixelSize: Style.font.caption
        elide: Text.ElideMiddle
        horizontalAlignment: Text.AlignHCenter
      }
    }
  }
}
