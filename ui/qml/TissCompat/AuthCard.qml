import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import TissCompat 1.0

Item {
    id: root
    property var backend: null
    property string defaultUser: ""
    property bool lockUser: false
    property bool showPasswordToggle: true
    property bool busy: backend ? backend.busy : false
    property bool showPassword: false

    property int promptId: -1
    property string promptKind: ""
    property string promptMessage: ""
    property bool promptActive: promptId >= 0
    property bool promptNeedsInput: promptKind === "visible" || promptKind === "secret"
    property string stagedPromptResponse: ""

    property string statusMessage: ""
    property string statusKind: "info"
    property bool hasAttemptedUnlock: false
    property real shakeOffset: 0
    property string errorPlaceholder: "Incorrect password"

    signal authenticated()

    implicitWidth: Math.min(parent ? parent.width * 0.56 : 540, 540 * Token.fontScale)
    implicitHeight: cardLayout.implicitHeight + 56

    function setStatus(message, kind) {
        statusMessage = message
        statusKind = kind || "info"
    }

    function doLogin() {
        if (!lockUser && usernameField.text.length === 0) {
            setStatus("username is required", "warning")
            return
        }
        setStatus("", "info")
        hasAttemptedUnlock = true
        stagedPromptResponse = passwordInput.text
        if (backend) {
            backend.authenticate(lockUser ? defaultUser : usernameField.text)
        }
    }

    function focusPassword() {
        if (promptActive && promptNeedsInput) {
            promptField.forceActiveFocus()
        } else {
            passwordInput.forceActiveFocus()
        }
    }

    function appendPassword(text) {
        if (promptActive && promptNeedsInput) {
            promptField.forceActiveFocus()
            promptField.insert(promptField.cursorPosition, text)
        } else {
            passwordInput.forceActiveFocus()
            passwordInput.insert(passwordInput.cursorPosition, text)
        }
    }

    function setPrompt(id, kind, message) {
        promptId = id
        promptKind = kind
        promptMessage = message
        if (promptNeedsInput) {
            if (stagedPromptResponse.length > 0) {
                promptField.text = stagedPromptResponse
                stagedPromptResponse = ""
                passwordInput.text = ""
            } else {
                promptField.text = ""
            }
            promptField.forceActiveFocus()
        } else {
            promptField.text = ""
        }
    }

    function clearPrompt() {
        promptId = -1
        promptKind = ""
        promptMessage = ""
        promptField.text = ""
        stagedPromptResponse = ""
        passwordInput.text = ""
    }

    function submitPrompt() {
        if (!promptActive || !backend) {
            return
        }
        if (promptNeedsInput && promptField.text.length === 0) {
            setStatus("response is required", "warning")
            return
        }
        setStatus("", "info")
        hasAttemptedUnlock = true
        if (promptNeedsInput) {
            backend.respondPrompt(promptId, promptField.text)
        } else {
            backend.ackPrompt(promptId)
        }
        clearPrompt()
    }

    Connections {
        target: backend
        function onPromptReceived(id, kind, message) {
            setPrompt(id, kind, message)
        }
        function onMessageReceived(kind, message) {
            setStatus(message, kind)
        }
        function onErrorReceived(code, message) {
            setStatus(message, "error")
            if (hasAttemptedUnlock) {
                wrongPasswordShakeAnim.restart()
            }
        }
        function onBackendCrashed(message) {
            setStatus(message, "error")
        }
        function onSuccess() {
            setStatus("Welcome", "success")
            authenticated()
        }
    }

    ColumnLayout {
        id: cardLayout
        anchors.centerIn: parent
        spacing: 16 * Token.fontScale

        Item {
            id: avatarContainer
            Layout.alignment: Qt.AlignHCenter
            width: 120 * Token.fontScale
            height: 120 * Token.fontScale

            Rectangle {
                anchors.centerIn: parent
                width: parent.width + 4
                height: parent.height + 4
                radius: width / 2
                color: Token.accent
                opacity: 0.35
            }

            Rectangle {
                anchors.fill: parent
                radius: width / 2
                color: Token.accent

                Text {
                    anchors.centerIn: parent
                    text: (lockUser ? defaultUser : usernameField.text).length > 0
                        ? (lockUser ? defaultUser : usernameField.text).charAt(0).toUpperCase()
                        : "U"
                    font.pixelSize: 48 * Token.fontScale
                    font.family: Token.fontFamily
                    font.weight: Font.Medium
                    color: Token.accentFg
                }
            }
        }

        Text {
            Layout.alignment: Qt.AlignHCenter
            text: lockUser ? defaultUser : (usernameField.text.length > 0 ? usernameField.text : "User")
            font.pixelSize: 24 * Token.fontScale
            font.family: Token.fontFamily
            font.weight: Font.Medium
            color: Token.fg
        }

        TextField {
            id: usernameField
            placeholderText: "Username"
            visible: !lockUser
            enabled: !busy && !promptActive
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: 280 * Token.fontScale
            Layout.topMargin: 6 * Token.fontScale
            font.family: Token.fontFamily
            font.pixelSize: 14 * Token.fontScale
        }

        Text {
            Layout.alignment: Qt.AlignHCenter
            text: promptActive && promptMessage.length > 0 ? promptMessage : "Sign in to continue"
            font.pixelSize: 13 * Token.fontScale
            font.family: Token.fontFamily
            color: Token.subfg
            Layout.topMargin: 4 * Token.fontScale
        }

        Rectangle {
            id: passwordContainer
            Layout.alignment: Qt.AlignHCenter
            width: 280 * Token.fontScale
            height: 40 * Token.fontScale
            radius: 10 * Token.fontScale
            color: Token.inputBg
            border.color: statusKind === "error"
                ? Token.error
                : ((promptActive ? promptField.activeFocus : passwordInput.activeFocus)
                ? Token.inputBorderFocus
                : Token.inputBorder)
            border.width: (promptActive ? promptField.activeFocus : passwordInput.activeFocus) ? 2 : 1

            Behavior on border.color {
                NumberAnimation {
                    duration: Token.motionFast
                    easing.type: Token.easingOut
                }
            }
            Behavior on border.width {
                NumberAnimation {
                    duration: Token.motionFast
                    easing.type: Token.easingOut
                }
            }

            transform: Translate { x: root.shakeOffset }

            Rectangle {
                anchors.bottom: parent.bottom
                anchors.horizontalCenter: parent.horizontalCenter
                height: 2
                width: (promptActive ? promptField.activeFocus : passwordInput.activeFocus)
                    ? (parent.width - 6)
                    : 0
                radius: 1
                color: Token.inputBorderFocus
                Behavior on width {
                    NumberAnimation {
                        duration: Token.motionFast
                        easing.type: Token.easingOut
                    }
                }
            }

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 8
                spacing: 8

                TextField {
                    id: passwordInput
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    placeholderText: statusKind === "error" && hasAttemptedUnlock
                        ? errorPlaceholder
                        : "Password"
                    echoMode: showPassword ? TextInput.Normal : TextInput.Password
                    visible: !promptActive
                    enabled: !busy
                    font.family: Token.fontFamily
                    font.pixelSize: 14 * Token.fontScale
                    color: Token.fg
                    onAccepted: doLogin()
                }

                TextField {
                    id: promptField
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    placeholderText: promptKind === "secret" ? "Password" : "Response"
                    echoMode: (promptKind === "secret" && !showPassword) ? TextInput.Password : TextInput.Normal
                    visible: promptActive && promptNeedsInput
                    enabled: !busy
                    font.family: Token.fontFamily
                    font.pixelSize: 14 * Token.fontScale
                    color: Token.fg
                    onAccepted: submitPrompt()
                }

                Rectangle {
                    id: submitButton
                    Layout.preferredWidth: 28 * Token.fontScale
                    Layout.preferredHeight: 28 * Token.fontScale
                    radius: 6 * Token.fontScale
                    color: Token.accent

                    Text {
                        anchors.centerIn: parent
                        text: ">"
                        color: Token.accentFg
                        font.pixelSize: 14 * Token.fontScale
                        font.family: Token.fontFamily
                    }

                    MouseArea {
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: {
                            if (promptActive) {
                                submitPrompt()
                            } else {
                                doLogin()
                            }
                        }
                    }
                }
            }
        }

        CheckBox {
            text: "Show password"
            visible: showPasswordToggle && (promptKind === "secret" || !promptActive)
            enabled: !busy
            checked: showPassword
            font.pixelSize: 12 * Token.fontScale
            onToggled: showPassword = checked
        }

        Text {
            Layout.alignment: Qt.AlignHCenter
            text: statusMessage
            font.pixelSize: 12 * Token.fontScale
            font.family: Token.fontFamily
            color: statusKind === "error"
                ? Token.error
                : (statusKind === "warning" ? "#f2c14e" : Token.subfg)
            visible: statusMessage.length > 0
        }
    }

    SequentialAnimation {
        id: wrongPasswordShakeAnim
        running: false
        NumberAnimation { target: root; property: "shakeOffset"; to: -16; duration: 50 }
        NumberAnimation { target: root; property: "shakeOffset"; to: 16; duration: 60 }
        NumberAnimation { target: root; property: "shakeOffset"; to: -10; duration: 60 }
        NumberAnimation { target: root; property: "shakeOffset"; to: 10; duration: 50 }
        NumberAnimation { target: root; property: "shakeOffset"; to: 0; duration: 70 }
    }
}
