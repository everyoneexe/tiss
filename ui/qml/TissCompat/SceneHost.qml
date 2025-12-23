import QtQuick 2.15

Item {
    id: root
    property string scene: "lock"
    property var backend: null
    property bool autoSync: true

    function updateScenes() {
        for (var i = 0; i < children.length; ++i) {
            var child = children[i]
            if (!child || child.sceneName === undefined || child.active === undefined) {
                continue
            }
            child.active = (child.sceneName === scene)
        }
    }

    function syncFromPhase(phase) {
        if (!autoSync) {
            return
        }
        if (phase === "auth" || phase === "waiting") {
            scene = "auth"
        } else if (phase === "success") {
            scene = "success"
        } else if (phase === "error") {
            scene = "error"
        } else {
            scene = "lock"
        }
    }

    onSceneChanged: updateScenes()
    Component.onCompleted: updateScenes()

    Connections {
        target: backend
        function onPhaseChanged() {
            if (!backend) {
                return
            }
            syncFromPhase(backend.phase)
        }
    }
}
