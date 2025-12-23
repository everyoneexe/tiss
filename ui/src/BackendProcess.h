#pragma once

#include <QObject>
#include <QProcess>
#include <QStringList>
#include <QVariantMap>

class BackendProcess : public QObject {
    Q_OBJECT
    Q_PROPERTY(QString phase READ phase NOTIFY phaseChanged)
    Q_PROPERTY(bool busy READ busy NOTIFY phaseChanged)
    Q_PROPERTY(QStringList sessionCommand READ sessionCommand WRITE setSessionCommand NOTIFY sessionConfigChanged)
    Q_PROPERTY(QVariantMap sessionEnv READ sessionEnv WRITE setSessionEnv NOTIFY sessionConfigChanged)
    Q_PROPERTY(QString selectedSessionId READ selectedSessionId WRITE setSelectedSessionId NOTIFY sessionConfigChanged)
    Q_PROPERTY(QString selectedProfileId READ selectedProfileId WRITE setSelectedProfileId NOTIFY sessionConfigChanged)
    Q_PROPERTY(QString selectedLocale READ selectedLocale WRITE setSelectedLocale NOTIFY sessionConfigChanged)
public:
    explicit BackendProcess(QObject *parent = nullptr);
    ~BackendProcess() override;

    Q_INVOKABLE void authenticate(const QString &username);
    Q_INVOKABLE void respondPrompt(int id, const QString &response);
    Q_INVOKABLE void ackPrompt(int id);
    Q_INVOKABLE void cancelAuth();
    Q_INVOKABLE void startSession(const QStringList &command);
    Q_INVOKABLE void requestPower(const QString &action);
    Q_INVOKABLE void ackSuccess();

    QString phase() const { return m_phase; }
    bool busy() const { return m_phase == "auth" || m_phase == "waiting"; }
    QStringList sessionCommand() const { return m_sessionCommand; }
    QVariantMap sessionEnv() const { return m_sessionEnv; }
    QString selectedSessionId() const { return m_selectedSessionId; }
    QString selectedProfileId() const { return m_selectedProfileId; }
    QString selectedLocale() const { return m_selectedLocale; }

    void setSessionCommand(const QStringList &command);
    void setSessionEnv(const QVariantMap &env);
    void setSelectedSessionId(const QString &sessionId);
    void setSelectedProfileId(const QString &profileId);
    void setSelectedLocale(const QString &locale);

signals:
    void phaseChanged();
    void promptReceived(int id, const QString &kind, const QString &message, bool echo);
    void messageReceived(const QString &kind, const QString &message);
    void errorReceived(const QString &code, const QString &message);
    void success();
    void backendCrashed(const QString &message);
    void sessionConfigChanged();

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
    QStringList m_sessionCommand;
    QVariantMap m_sessionEnv;
    QString m_selectedSessionId;
    QString m_selectedProfileId;
    QString m_selectedLocale;
};
