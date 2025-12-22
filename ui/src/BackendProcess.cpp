#include "BackendProcess.h"

#include <QCoreApplication>
#include <QDir>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QStandardPaths>

BackendProcess::BackendProcess(QObject *parent)
    : QObject(parent) {
    startBackend();
}

BackendProcess::~BackendProcess() {
    if (m_proc.state() != QProcess::NotRunning) {
        m_proc.terminate();
        m_proc.waitForFinished(1000);
    }
}

void BackendProcess::startBackend() {
    const QString backendPath = resolveBackendPath();
    m_proc.setProgram(backendPath);
    m_proc.setProcessChannelMode(QProcess::SeparateChannels);

    connect(&m_proc, &QProcess::readyReadStandardOutput, this, &BackendProcess::handleStdout);
    connect(&m_proc, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished),
            this, &BackendProcess::handleFinished);
    connect(&m_proc, &QProcess::errorOccurred, this, &BackendProcess::handleError);

    m_proc.start();
}

void BackendProcess::authenticate(const QString &username) {
    m_allowExit = false;
    QJsonObject obj;
    obj.insert("type", "auth");
    obj.insert("username", username);
    if (!m_selectedSessionId.isEmpty()) {
        obj.insert("session_id", m_selectedSessionId);
    }
    if (!m_selectedProfileId.isEmpty()) {
        obj.insert("profile_id", m_selectedProfileId);
    }
    if (!m_selectedLocale.isEmpty()) {
        obj.insert("locale", m_selectedLocale);
    }
    if (!m_sessionCommand.isEmpty()) {
        QJsonArray cmd;
        for (const auto &part : m_sessionCommand) {
            cmd.append(part);
        }
        obj.insert("command", cmd);
    }
    if (!m_sessionEnv.isEmpty()) {
        QJsonObject envObj;
        for (auto it = m_sessionEnv.constBegin(); it != m_sessionEnv.constEnd(); ++it) {
            envObj.insert(it.key(), it.value().toString());
        }
        obj.insert("env", envObj);
    }
    sendJson(obj);
}

void BackendProcess::respondPrompt(int id, const QString &response) {
    QJsonObject obj;
    obj.insert("type", "prompt_response");
    obj.insert("id", id);
    obj.insert("response", response);
    sendJson(obj);
}

void BackendProcess::ackPrompt(int id) {
    QJsonObject obj;
    obj.insert("type", "prompt_response");
    obj.insert("id", id);
    obj.insert("response", QJsonValue::Null);
    sendJson(obj);
}

void BackendProcess::cancelAuth() {
    QJsonObject obj;
    obj.insert("type", "cancel");
    sendJson(obj);
}

void BackendProcess::startSession(const QStringList &command) {
    m_allowExit = false;
    QJsonObject obj;
    obj.insert("type", "start");
    QJsonArray cmd;
    for (const auto &part : command) {
        cmd.append(part);
    }
    obj.insert("command", cmd);
    if (!m_sessionEnv.isEmpty()) {
        QJsonObject envObj;
        for (auto it = m_sessionEnv.constBegin(); it != m_sessionEnv.constEnd(); ++it) {
            envObj.insert(it.key(), it.value().toString());
        }
        obj.insert("env", envObj);
    }
    sendJson(obj);
}

void BackendProcess::requestPower(const QString &action) {
    QJsonObject obj;
    obj.insert("type", "power");
    obj.insert("action", action);
    sendJson(obj);
}

void BackendProcess::setSessionCommand(const QStringList &command) {
    if (m_sessionCommand == command) {
        return;
    }
    m_sessionCommand = command;
    emit sessionConfigChanged();
}

void BackendProcess::setSessionEnv(const QVariantMap &env) {
    if (m_sessionEnv == env) {
        return;
    }
    m_sessionEnv = env;
    emit sessionConfigChanged();
}

void BackendProcess::setSelectedSessionId(const QString &sessionId) {
    if (m_selectedSessionId == sessionId) {
        return;
    }
    m_selectedSessionId = sessionId;
    emit sessionConfigChanged();
}

void BackendProcess::setSelectedProfileId(const QString &profileId) {
    if (m_selectedProfileId == profileId) {
        return;
    }
    m_selectedProfileId = profileId;
    emit sessionConfigChanged();
}

void BackendProcess::setSelectedLocale(const QString &locale) {
    if (m_selectedLocale == locale) {
        return;
    }
    m_selectedLocale = locale;
    emit sessionConfigChanged();
}

void BackendProcess::handleStdout() {
    while (m_proc.canReadLine()) {
        const QByteArray line = m_proc.readLine();
        QJsonParseError err;
        const QJsonDocument doc = QJsonDocument::fromJson(line, &err);
        if (err.error != QJsonParseError::NoError || !doc.isObject()) {
            continue;
        }
        const QJsonObject obj = doc.object();
        const QString type = obj.value("type").toString();
        if (type == "state") {
            m_phase = obj.value("phase").toString();
            emit phaseChanged();
        } else if (type == "prompt") {
            emit promptReceived(
                obj.value("id").toInt(),
                obj.value("kind").toString(),
                obj.value("message").toString(),
                obj.value("echo").toBool());
        } else if (type == "error") {
            const QString code = obj.value("code").toString();
            emit errorReceived(code.isEmpty() ? QStringLiteral("pam_error") : code,
                               obj.value("message").toString());
        } else if (type == "success") {
            m_allowExit = true;
            m_phase = "success";
            emit phaseChanged();
            emit success();
        }
    }
}

void BackendProcess::handleFinished(int exitCode, QProcess::ExitStatus status) {
    if (m_allowExit && status == QProcess::NormalExit && exitCode == 0) {
        return;
    }
    const QString msg = QString("backend exited: code=%1 status=%2")
                            .arg(exitCode)
                            .arg(status == QProcess::NormalExit ? "normal" : "crash");
    emit errorReceived(QStringLiteral("backend_crash"), msg);
    emit backendCrashed(msg);
}

void BackendProcess::handleError(QProcess::ProcessError error) {
    if (m_allowExit) {
        return;
    }
    QString kind = "unknown";
    switch (error) {
    case QProcess::FailedToStart:
        kind = "failed-to-start";
        break;
    case QProcess::Crashed:
        kind = "crashed";
        break;
    case QProcess::Timedout:
        kind = "timed-out";
        break;
    case QProcess::WriteError:
        kind = "write-error";
        break;
    case QProcess::ReadError:
        kind = "read-error";
        break;
    case QProcess::UnknownError:
        kind = "unknown-error";
        break;
    }
    const QString msg = QString("backend error: %1 (%2)").arg(kind, m_proc.errorString());
    emit errorReceived(QStringLiteral("backend_crash"), msg);
    emit backendCrashed(msg);
}

QString BackendProcess::resolveBackendPath() const {
    const QString envPath = qEnvironmentVariable("II_GREETD_BACKEND");
    if (!envPath.isEmpty()) {
        return envPath;
    }

    const QString appDir = QCoreApplication::applicationDirPath();
    const QStringList candidates = {
        QDir(appDir).filePath("ii-greetd-backend"),
        QDir(appDir).filePath("../lib/ii-greetd/ii-greetd-backend"),
        "/usr/lib/ii-greetd/ii-greetd-backend",
        "/usr/local/lib/ii-greetd/ii-greetd-backend",
    };

    for (const auto &candidate : candidates) {
        QFileInfo info(candidate);
        if (info.exists() && info.isExecutable()) {
            return candidate;
        }
    }

    const QString inPath = QStandardPaths::findExecutable("ii-greetd-backend");
    if (!inPath.isEmpty()) {
        return inPath;
    }

    return "ii-greetd-backend";
}

void BackendProcess::sendJson(const QJsonObject &obj) {
    if (m_proc.state() == QProcess::NotRunning) {
        emit backendCrashed("backend is not running");
        return;
    }
    const QJsonDocument doc(obj);
    QByteArray payload = doc.toJson(QJsonDocument::Compact);
    payload.append('\n');
    m_proc.write(payload);
    m_proc.waitForBytesWritten(100);
}
