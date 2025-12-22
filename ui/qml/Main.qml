import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import QtQuick.Window 2.15
import IIGreetd 1.0

ApplicationWindow {
    id: root
    property bool outputReady: Screen.width > 0 && Screen.height > 0
    width: outputReady ? Screen.width : 1280
    height: outputReady ? Screen.height : 720
    visible: outputReady
    title: "ii-greetd"
    color: "#0e0f12"

    property string defaultUser: iiDefaultUser
    property bool lockUser: iiLockUser
    property bool busy: backend.busy
    property bool hasUser: (lockUser ? defaultUser.length > 0 : usernameField.text.length > 0)
    property bool showPassword: false
    property bool showPasswordToggle: iiShowPasswordToggle
    property string lastSessionId: iiLastSessionId
    property string lastProfileId: iiLastProfileId
    property string lastLocale: iiLastLocale
    property int promptId: -1
    property string promptKind: ""
    property string promptMessage: ""
    property string lastErrorCode: ""
    property bool promptEcho: true
    property bool promptActive: promptId >= 0
    property bool promptNeedsInput: promptKind === "visible" || promptKind === "secret"

    BackendProcess {
        id: backend
        sessionCommand: iiSessionCommand
        sessionEnv: iiSessionEnv
        onPhaseChanged: {
            if (phase === "auth") {
                statusText.text = "Authenticating..."
            } else if (phase === "waiting") {
                statusText.text = "Starting session..."
            } else if (phase === "idle") {
                statusText.text = ""
            } else if (phase === "success") {
                statusText.text = "Welcome"
            }
        }
        onErrorReceived: (code, message) => {
            lastErrorCode = code
            statusText.text = message
            clearPrompt()
            if (promptNeedsInput) {
                promptField.forceActiveFocus()
            } else {
                usernameField.forceActiveFocus()
            }
        }
        onBackendCrashed: message => {
            lastErrorCode = "backend_crash"
            statusText.text = message
            clearPrompt()
            if (promptNeedsInput) {
                promptField.forceActiveFocus()
            } else {
                usernameField.forceActiveFocus()
            }
        }
        onSuccess: {
            statusText.text = "Welcome"
            Qt.quit()
        }
        onPromptReceived: (id, kind, message, echo) => {
            setPrompt(id, kind, message, echo)
        }
    }

    Component.onCompleted: {
        if (defaultUser.length > 0) {
            usernameField.text = defaultUser
        }
        if (lockUser && defaultUser.length === 0) {
            statusText.text = "II_GREETD_DEFAULT_USER is required"
        }
        if (lastSessionId.length > 0) {
            backend.selectedSessionId = lastSessionId
        }
        if (lastProfileId.length > 0) {
            backend.selectedProfileId = lastProfileId
        }
        if (lastLocale.length > 0) {
            backend.selectedLocale = lastLocale
        } else if (iiLocales && iiLocales.default) {
            backend.selectedLocale = iiLocales.default
        }
        usernameField.forceActiveFocus()
    }

    function doLogin() {
        if (!hasUser) {
            statusText.text = "username is required"
            return
        }
        statusText.text = ""
        clearPrompt()
        backend.authenticate(lockUser ? defaultUser : usernameField.text)
    }

    function setPrompt(id, kind, message, echo) {
        promptId = id
        promptKind = kind
        promptMessage = message
        promptEcho = echo
        promptField.text = ""
        if (promptNeedsInput) {
            promptField.forceActiveFocus()
        }
    }

    function clearPrompt() {
        promptId = -1
        promptKind = ""
        promptMessage = ""
        promptEcho = true
        promptField.text = ""
    }

    function submitPrompt() {
        if (!promptActive) {
            return
        }
        if (promptNeedsInput && promptField.text.length === 0) {
            statusText.text = "response is required"
            return
        }
        statusText.text = ""
        if (promptNeedsInput) {
            backend.respondPrompt(promptId, promptField.text)
        } else {
            backend.ackPrompt(promptId)
        }
        clearPrompt()
    }

    Timer {
        id: clockTimer
        interval: 1000
        running: true
        repeat: true
        onTriggered: clockText.text = Qt.formatDateTime(new Date(), "hh:mm")
    }

    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            GradientStop { position: 0.0; color: "#0e0f12" }
            GradientStop { position: 1.0; color: "#1b1f2a" }
        }
    }

    ColumnLayout {
        anchors.centerIn: parent
        spacing: 16

        Text {
            id: clockText
            text: Qt.formatDateTime(new Date(), "hh:mm")
            color: "#e6e6e6"
            font.pixelSize: 64
            horizontalAlignment: Text.AlignHCenter
            Layout.alignment: Qt.AlignHCenter
        }

        Text {
            id: dateText
            text: Qt.formatDateTime(new Date(), "ddd, MMM d")
            color: "#9aa3ad"
            font.pixelSize: 16
            Layout.alignment: Qt.AlignHCenter
        }

        Rectangle {
            width: 420
            height: 1
            color: "#2a2f3a"
            Layout.alignment: Qt.AlignHCenter
        }

        TextField {
            id: usernameField
            placeholderText: "Username"
            Layout.preferredWidth: 360
            Layout.alignment: Qt.AlignHCenter
            readOnly: lockUser
            visible: !lockUser
            enabled: !busy
        }

        Text {
            id: promptLabel
            text: root.promptMessage
            visible: root.promptActive && root.promptMessage.length > 0
            color: "#c2c8d2"
            font.pixelSize: 14
            horizontalAlignment: Text.AlignHCenter
            Layout.alignment: Qt.AlignHCenter
            wrapMode: Text.WordWrap
            Layout.preferredWidth: 360
        }

        TextField {
            id: promptField
            placeholderText: root.promptKind === "secret" ? "Password" : "Response"
            echoMode: (root.promptKind === "secret" && !root.showPassword) ? TextInput.Password : TextInput.Normal
            Layout.preferredWidth: 360
            Layout.alignment: Qt.AlignHCenter
            enabled: !busy
            visible: root.promptActive && root.promptNeedsInput
            onAccepted: root.submitPrompt()
        }

        CheckBox {
            id: showPasswordCheck
            text: "Show password"
            checked: root.showPassword
            enabled: !busy
            visible: root.showPasswordToggle && root.promptKind === "secret"
            Layout.alignment: Qt.AlignHCenter
            onToggled: root.showPassword = checked
        }

        Button {
            id: loginButton
            text: busy ? "Working..." : "Continue"
            enabled: hasUser && !busy
            visible: !root.promptActive
            Layout.preferredWidth: 200
            Layout.alignment: Qt.AlignHCenter
            onClicked: {
                root.doLogin()
            }
        }

        Button {
            id: promptButton
            text: root.promptNeedsInput ? "Submit" : "Continue"
            enabled: !busy && (!root.promptNeedsInput || promptField.text.length > 0)
            visible: root.promptActive
            Layout.preferredWidth: 200
            Layout.alignment: Qt.AlignHCenter
            onClicked: root.submitPrompt()
        }

        Text {
            id: statusText
            text: ""
            color: "#d97272"
            font.pixelSize: 14
            Layout.alignment: Qt.AlignHCenter
        }
    }
}
