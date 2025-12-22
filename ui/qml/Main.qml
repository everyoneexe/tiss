import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import IIGreetd 1.0

ApplicationWindow {
    id: root
    width: 1280
    height: 720
    visible: true
    title: "ii-greetd"
    color: "#0e0f12"

    property string defaultUser: iiDefaultUser
    property bool lockUser: iiLockUser
    property bool busy: backend.busy
    property bool hasUser: (lockUser ? defaultUser.length > 0 : usernameField.text.length > 0)
    property bool showPassword: false

    BackendProcess {
        id: backend
        onPhaseChanged: {
            if (phase === "authenticating") {
                statusText.text = "Authenticating..."
            } else if (phase === "starting") {
                statusText.text = "Starting session..."
            } else if (phase === "idle") {
                statusText.text = ""
            }
        }
        onErrorReceived: message => {
            statusText.text = message
            passwordField.forceActiveFocus()
        }
        onBackendCrashed: message => {
            statusText.text = message
            passwordField.forceActiveFocus()
        }
        onSuccess: {
            statusText.text = "Welcome"
            Qt.quit()
        }
    }

    Component.onCompleted: {
        if (defaultUser.length > 0) {
            usernameField.text = defaultUser
        }
        if (lockUser && defaultUser.length === 0) {
            statusText.text = "II_GREETD_DEFAULT_USER is required"
        }
        if (lockUser || defaultUser.length > 0) {
            passwordField.forceActiveFocus()
        } else {
            usernameField.forceActiveFocus()
        }
    }

    function doLogin() {
        if (!hasUser) {
            statusText.text = "username is required"
            return
        }
        if (passwordField.text.length === 0) {
            statusText.text = "password is required"
            return
        }
        statusText.text = ""
        backend.authenticate(lockUser ? defaultUser : usernameField.text, passwordField.text)
        passwordField.text = ""
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

        TextField {
            id: passwordField
            placeholderText: "Password"
            echoMode: root.showPassword ? TextInput.Normal : TextInput.Password
            Layout.preferredWidth: 360
            Layout.alignment: Qt.AlignHCenter
            enabled: !busy
            onAccepted: root.doLogin()
        }

        CheckBox {
            id: showPasswordCheck
            text: "Show password"
            checked: root.showPassword
            enabled: !busy
            Layout.alignment: Qt.AlignHCenter
            onToggled: root.showPassword = checked
        }

        Button {
            id: loginButton
            text: busy ? "Working..." : "Log in"
            enabled: passwordField.text.length > 0 && hasUser && !busy
            Layout.preferredWidth: 200
            Layout.alignment: Qt.AlignHCenter
            onClicked: {
                root.doLogin()
            }
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
