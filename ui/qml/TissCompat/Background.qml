import QtQuick 2.15
import TissCompat 1.0

Item {
    id: root
    property url wallpaperSource: ""
    property color overlayColor: Token.bg
    property real overlayOpacity: Token.blurOpacity
    property real zoom: 1.06
    property bool dim: false
    property real dimOpacity: 0.0

    Image {
        anchors.fill: parent
        source: wallpaperSource
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        cache: true
        transform: Scale {
            origin.x: width / 2
            origin.y: height / 2
            xScale: root.zoom
            yScale: root.zoom
        }
    }

    Rectangle {
        anchors.fill: parent
        color: overlayColor
        opacity: overlayOpacity
    }

    Behavior on zoom {
        NumberAnimation {
            duration: Token.motionSlow
            easing.type: Token.easingOut
        }
    }

    Rectangle {
        anchors.fill: parent
        color: overlayColor
        opacity: dim ? dimOpacity : 0.0
        Behavior on opacity {
            NumberAnimation {
                duration: Token.motionMed
                easing.type: Token.easingOut
            }
        }
    }
}
