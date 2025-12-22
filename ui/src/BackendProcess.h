#pragma once

#include <QObject>
#include <QProcess>

class BackendProcess : public QObject {
    Q_OBJECT
    Q_PROPERTY(QString phase READ phase NOTIFY phaseChanged)
    Q_PROPERTY(bool busy READ busy NOTIFY phaseChanged)
public:
    explicit BackendProcess(QObject *parent = nullptr);
    ~BackendProcess() override;

    Q_INVOKABLE void authenticate(const QString &username, const QString &password);
    Q_INVOKABLE void startSession(const QStringList &command);

    QString phase() const { return m_phase; }
    bool busy() const { return m_phase == "authenticating" || m_phase == "starting"; }

signals:
    void phaseChanged();
    void errorReceived(const QString &message);
    void success();
    void backendCrashed(const QString &message);

private slots:
    void handleStdout();
    void handleFinished(int exitCode, QProcess::ExitStatus status);
    void handleError(QProcess::ProcessError error);

private:
    void startBackend();
    QString resolveBackendPath() const;
    void sendJson(const QJsonObject &obj);

    QProcess m_proc;
    QString m_phase = "idle";
    bool m_allowExit = false;
};
