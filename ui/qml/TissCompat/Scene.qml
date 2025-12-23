import QtQuick 2.15
import TissCompat 1.0

Item {
    id: root
    property string sceneName: "lock"
    property bool active: false
    property real activeOpacity: 1.0
    property real inactiveOpacity: 0.0
    property real activeScale: 1.0
    property real inactiveScale: 1.02

    opacity: active ? activeOpacity : inactiveOpacity
    scale: active ? activeScale : inactiveScale
    visible: opacity > 0.001

    Behavior on opacity {
        NumberAnimation {
            duration: Token.motionMed
            easing.type: Token.easingOut
        }
    }

    Behavior on scale {
        NumberAnimation {
            duration: Token.motionMed
            easing.type: Token.easingOut
        }
    }
}
