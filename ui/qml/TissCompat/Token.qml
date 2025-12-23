pragma Singleton
import QtQuick 2.15

QtObject {
    id: root

    property var data: (typeof tissAppearance !== "undefined" && tissAppearance) ? tissAppearance : ({})

    function read(key, fallback) {
        if (data && data[key] !== undefined && data[key] !== null) {
            var value = String(data[key])
            if (value.length > 0) {
                return value
            }
        }
        return fallback
    }

    function num(key, fallback) {
        var raw = read(key, "")
        if (raw === "") {
            return fallback
        }
        var value = Number(raw)
        return isNaN(value) ? fallback : value
    }

    function easingType(key, fallback) {
        var name = String(read(key, fallback))
        switch (name) {
        case "OutCubic":
            return Easing.OutCubic
        case "OutQuad":
            return Easing.OutQuad
        case "OutExpo":
            return Easing.OutExpo
        case "InCubic":
            return Easing.InCubic
        case "InOutCubic":
            return Easing.InOutCubic
        default:
            return Easing.OutCubic
        }
    }

    property color bg: read("bg", "#0e0f12")
    property color fg: read("fg", "#f2f4f8")
    property color subfg: read("subfg", "#c2c8d2")
    property color accent: read("accent", "#7dd3fc")
    property color accentFg: read("accent_fg", "#0b0c10")
    property color error: read("error", "#ff3b30")
    property color success: read("success", "#34c759")
    property color cardBg: read("card_bg", "#121620")
    property color cardBorder: read("card_border", "#232a3a")
    property color shadow: read("shadow", "#12151a")
    property color inputBg: read("input_bg", "#11151f")
    property color inputBorder: read("input_border", "#2a3040")
    property color inputBorderFocus: read("input_border_focus", accent)
    property color pillBg: read("pill_bg", "#1a1f2a")
    property color pillBorder: read("pill_border", "#2a3040")
    property color pillText: read("pill_text", subfg)

    property real radius: num("radius", 18)
    property real cardRadius: num("card_radius", radius)
    property real outlineWidth: num("outline_width", 1)
    property real fontScale: num("font_scale", 1.0)
    property string fontFamily: read("font_family", "Noto Sans")

    property int motionFast: num("motion_fast", 120)
    property int motionMed: num("motion_med", 180)
    property int motionSlow: num("motion_slow", 280)
    property int easingOut: easingType("easing_out", "OutCubic")
    property int easingIn: easingType("easing_in", "InCubic")

    property real blurAmount: num("blur_amount", 0)
    property real blurOpacity: num("blur_opacity", 0.35)
}
