import QtQuick 2.15
import QtQuick.Layouts 1.15
import TissCompat 1.0

ColumnLayout {
    id: root
    property string timeFormat: "hh:mm"
    property string dateFormat: "dddd, MMMM d"
    property color timeColor: Token.fg
    property color dateColor: Token.subfg
    property int timeSize: Math.round(96 * Token.fontScale)
    property int dateSize: Math.round(20 * Token.fontScale)

    spacing: 6

    Item {
        Layout.alignment: Qt.AlignHCenter
        width: Math.max(clockShadow.implicitWidth, clockText.implicitWidth)
        height: Math.max(clockShadow.implicitHeight, clockText.implicitHeight)

        Text {
            id: clockShadow
            anchors.centerIn: parent
            text: clockText.text
            color: Token.shadow
            font.pixelSize: root.timeSize
            font.family: Token.fontFamily
            font.weight: Font.Thin
            opacity: 0.6
            x: 0
            y: 2
        }

        Text {
            id: clockText
            anchors.centerIn: parent
            text: Qt.formatTime(new Date(), root.timeFormat)
            color: root.timeColor
            font.pixelSize: root.timeSize
            font.family: Token.fontFamily
            font.weight: Font.Thin
            horizontalAlignment: Text.AlignHCenter
        }

        Timer {
            interval: 1000
            running: true
            repeat: true
            onTriggered: clockText.text = Qt.formatTime(new Date(), root.timeFormat)
        }
    }

    Item {
        Layout.alignment: Qt.AlignHCenter
        width: Math.max(dateShadow.implicitWidth, dateText.implicitWidth)
        height: Math.max(dateShadow.implicitHeight, dateText.implicitHeight)

        Text {
            id: dateShadow
            anchors.centerIn: parent
            text: dateText.text
            color: Token.shadow
            font.pixelSize: root.dateSize
            font.family: Token.fontFamily
            font.weight: Font.Medium
            opacity: 0.6
            x: 0
            y: 1
        }

        Text {
            id: dateText
            anchors.centerIn: parent
            text: Qt.formatDate(new Date(), root.dateFormat)
            color: root.dateColor
            font.pixelSize: root.dateSize
            font.family: Token.fontFamily
            font.weight: Font.Medium
            horizontalAlignment: Text.AlignHCenter
        }

        Timer {
            interval: 60000
            running: true
            repeat: true
            onTriggered: dateText.text = Qt.formatDate(new Date(), root.dateFormat)
        }
    }
}
