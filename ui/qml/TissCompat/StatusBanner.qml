import QtQuick 2.15
import QtQuick.Layouts 1.15
import TissCompat 1.0

Rectangle {
    id: root
    property string message: ""
    property string kind: "info"
    property color infoColor: Token.subfg
    property color warningColor: "#f2c14e"
    property color errorColor: Token.error
    property color successColor: Token.success

    visible: message.length > 0
    color: "transparent"
    border.color: colorForKind()
    border.width: Token.outlineWidth
    radius: Token.radius

    function colorForKind() {
        if (kind === "error") {
            return errorColor
        }
        if (kind === "warning") {
            return warningColor
        }
        if (kind === "success") {
            return successColor
        }
        return infoColor
    }

    Layout.fillWidth: true
    implicitHeight: message.length > 0 ? (textItem.implicitHeight + 16) : 0

    Text {
        id: textItem
        anchors.fill: parent
        anchors.margins: 8
        text: root.message
        color: root.colorForKind()
        font.pixelSize: Math.round(12 * Token.fontScale)
        font.family: Token.fontFamily
        wrapMode: Text.WordWrap
    }
}
