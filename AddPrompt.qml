import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qs.Commons
import qs.Ui

// Card content for the add micro-overlay: appends to the END of the queue
// without touching current. Same shape as JumpPrompt; only the copy
// differs. Pure input card: emits submitted(title).
Item {
  id: root

  // Provided by FocusOverlay.qml.
  property color background: Color.menu.background
  property color foreground: Color.menu.text
  property color border: Color.menu.border
  property string fontFamily: Style.font.menuFamily

  signal submitted(string title)
  signal cancelled()

  width: card.implicitWidth
  height: card.implicitHeight

  function focusInput() {
    Qt.callLater(function () { input.forceActiveFocus() })
  }

  function reset() {
    input.text = ""
  }

  function submit() {
    var text = String(input.text || "").trim()
    if (text.length === 0) {
      root.cancelled()
      return
    }
    root.submitted(text)
  }

  Rectangle {
    id: card
    implicitWidth: Style.space(560)
    implicitHeight: contentCol.implicitHeight + contentCol.spacing * 2 + Style.spacing.panelPadding * 2
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
      spacing: Style.spacing.sm

      Text {
        text: "Enqueue"
        color: root.foreground
        font.family: root.fontFamily
        font.pixelSize: Style.font.title
      }

      Text {
        text: "Adds to the end of the queue. If nothing is current, it starts right away."
        color: Qt.darker(root.foreground, 1.4)
        font.family: root.fontFamily
        font.pixelSize: Style.font.body
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
          color: Qt.darker(root.background, 1.08)
          radius: Style.cornerRadius / 2
          border.width: 1
          border.color: root.border
        }
        // Single deterministic submit path (BeforeItem + consume) so one
        // Enter can never leak past the field.
        Keys.priority: Keys.BeforeItem
        Keys.onReturnPressed: function (event) {
          root.submit()
          event.accepted = true
        }
        Keys.onEnterPressed: function (event) {
          root.submit()
          event.accepted = true
        }
        Keys.onEscapePressed: function (event) {
          root.cancelled()
          event.accepted = true
        }
      }

      RowLayout {
        Layout.fillWidth: true
        spacing: Style.spacing.md
        Item { Layout.fillWidth: true }
        Text {
          text: "Enter to enqueue · Esc to cancel"
          color: Qt.darker(root.foreground, 1.5)
          font.family: root.fontFamily
          font.pixelSize: Style.font.caption
        }
      }
    }
  }
}
