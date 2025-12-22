#include <QCoreApplication>
#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QGuiApplication>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QMap>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QStringList>
#include <QTextStream>
#include <QVariantList>
#include <QVariantMap>

#include "BackendProcess.h"

static bool envBool(const char *name, bool fallback) {
    if (qEnvironmentVariableIsEmpty(name)) {
        return fallback;
    }
    const QString value = qEnvironmentVariable(name).trimmed().toLower();
    if (value == "true" || value == "1" || value == "yes" || value == "on") {
        return true;
    }
    if (value == "false" || value == "0" || value == "no" || value == "off") {
        return false;
    }
    return fallback;
}

static QStringList parseSessionCommandJson(const QString &raw) {
    if (raw.trimmed().isEmpty()) {
        return {};
    }
    QJsonParseError err;
    const QJsonDocument doc = QJsonDocument::fromJson(raw.toUtf8(), &err);
    if (err.error != QJsonParseError::NoError || !doc.isArray()) {
        qWarning() << "invalid II_GREETD_SESSION_JSON";
        return {};
    }
    QStringList result;
    const QJsonArray arr = doc.array();
    for (const auto &value : arr) {
        if (value.isString()) {
            result << value.toString();
        }
    }
    return result;
}

static QVariantMap parseSessionEnvJson(const QString &raw) {
    QVariantMap result;
    if (raw.trimmed().isEmpty()) {
        return result;
    }
    QJsonParseError err;
    const QJsonDocument doc = QJsonDocument::fromJson(raw.toUtf8(), &err);
    if (err.error != QJsonParseError::NoError || !doc.isObject()) {
        qWarning() << "invalid II_GREETD_SESSION_ENV_JSON";
        return result;
    }
    const QJsonObject obj = doc.object();
    for (auto it = obj.begin(); it != obj.end(); ++it) {
        const QJsonValue value = it.value();
        if (value.isString()) {
            result.insert(it.key(), value.toString());
        } else {
            result.insert(it.key(), value.toVariant().toString());
        }
    }
    return result;
}

static QVariantList parseSessionsJson(const QString &raw) {
    QVariantList result;
    if (raw.trimmed().isEmpty()) {
        return result;
    }
    QJsonParseError err;
    const QJsonDocument doc = QJsonDocument::fromJson(raw.toUtf8(), &err);
    if (err.error != QJsonParseError::NoError || !doc.isArray()) {
        qWarning() << "invalid II_GREETD_SESSIONS_JSON";
        return result;
    }
    const QJsonArray arr = doc.array();
    for (const auto &value : arr) {
        if (!value.isObject()) {
            continue;
        }
        result << value.toObject().toVariantMap();
    }
    return result;
}

static QVariantList parseProfilesJson(const QString &raw) {
    QVariantList result;
    if (raw.trimmed().isEmpty()) {
        return result;
    }
    QJsonParseError err;
    const QJsonDocument doc = QJsonDocument::fromJson(raw.toUtf8(), &err);
    if (err.error != QJsonParseError::NoError || !doc.isArray()) {
        qWarning() << "invalid II_GREETD_PROFILES_JSON";
        return result;
    }
    const QJsonArray arr = doc.array();
    for (const auto &value : arr) {
        if (!value.isObject()) {
            continue;
        }
        result << value.toObject().toVariantMap();
    }
    return result;
}

static QVariantMap parseLocalesJson(const QString &raw) {
    QVariantMap result;
    if (raw.trimmed().isEmpty()) {
        return result;
    }
    QJsonParseError err;
    const QJsonDocument doc = QJsonDocument::fromJson(raw.toUtf8(), &err);
    if (err.error != QJsonParseError::NoError || !doc.isObject()) {
        qWarning() << "invalid II_GREETD_LOCALES_JSON";
        return result;
    }
    const QJsonObject obj = doc.object();
    if (obj.contains("default") && obj.value("default").isString()) {
        result.insert("default", obj.value("default").toString());
    }
    if (obj.contains("available") && obj.value("available").isArray()) {
        result.insert("available", obj.value("available").toArray().toVariantList());
    }
    return result;
}

static QVariantList parsePowerActionsJson(const QString &raw) {
    QVariantList result;
    if (raw.trimmed().isEmpty()) {
        return result;
    }
    QJsonParseError err;
    const QJsonDocument doc = QJsonDocument::fromJson(raw.toUtf8(), &err);
    if (err.error != QJsonParseError::NoError || !doc.isArray()) {
        qWarning() << "invalid II_GREETD_POWER_ACTIONS_JSON";
        return result;
    }
    const QJsonArray arr = doc.array();
    for (const auto &value : arr) {
        if (value.isString()) {
            result << value.toString();
        }
    }
    return result;
}

static void ensureCacheEnv() {
    if (qEnvironmentVariableIsEmpty("QML_DISABLE_DISK_CACHE")) {
        qputenv("QML_DISABLE_DISK_CACHE", "1");
    }

    QByteArray cacheHome = qgetenv("XDG_CACHE_HOME");
    QString cachePath;
    if (cacheHome.isEmpty()) {
        const QString fallback = QDir::temp().filePath("ii-greetd-cache");
        QDir().mkpath(fallback);
        qputenv("XDG_CACHE_HOME", fallback.toUtf8());
        cachePath = fallback;
    } else {
        cachePath = QString::fromUtf8(cacheHome);
        QDir().mkpath(cachePath);
    }

    if (qEnvironmentVariableIsEmpty("MESA_SHADER_CACHE_DIR")) {
        const QString mesaCache = QDir(cachePath).filePath("mesa");
        QDir().mkpath(mesaCache);
        qputenv("MESA_SHADER_CACHE_DIR", mesaCache.toUtf8());
    }
}

static QString readUidFromProc() {
    QFile file("/proc/self/status");
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return {};
    }
    QTextStream in(&file);
    while (!in.atEnd()) {
        const QString line = in.readLine();
        if (line.startsWith("Uid:")) {
            const QStringList parts = line.mid(4).simplified().split(' ');
            if (!parts.isEmpty()) {
                return parts.first();
            }
        }
    }
    return {};
}

static QString defaultLogDir() {
    const QString envDir = qEnvironmentVariable("II_GREETD_LOG_DIR");
    if (!envDir.isEmpty()) {
        return envDir;
    }

    const QString uid = readUidFromProc();
    if (!uid.isEmpty()) {
        return QDir::temp().filePath(QString("ii-greetd-%1").arg(uid));
    }

    QString user = qEnvironmentVariable("USER");
    if (user.isEmpty()) {
        user = qEnvironmentVariable("LOGNAME");
    }
    if (user.isEmpty()) {
        user = qEnvironmentVariable("UID");
    }
    if (user.isEmpty()) {
        user = "unknown";
    }
    return QDir::temp().filePath(QString("ii-greetd-%1").arg(user));
}

static QFile *g_logFile = nullptr;

static void messageHandler(QtMsgType type, const QMessageLogContext &, const QString &message) {
    const char *typeStr = "INFO";
    switch (type) {
    case QtDebugMsg:
        typeStr = "DEBUG";
        break;
    case QtInfoMsg:
        typeStr = "INFO";
        break;
    case QtWarningMsg:
        typeStr = "WARN";
        break;
    case QtCriticalMsg:
        typeStr = "ERROR";
        break;
    case QtFatalMsg:
        typeStr = "FATAL";
        break;
    }

    const QString line = QString("[%1] %2 %3\n")
                             .arg(QDateTime::currentDateTime().toString(Qt::ISODateWithMs))
                             .arg(QString::fromUtf8(typeStr))
                             .arg(message);

    if (g_logFile && g_logFile->isOpen()) {
        g_logFile->write(line.toUtf8());
        g_logFile->flush();
    } else {
        QTextStream err(stderr);
        err << line;
    }
}

static void initLogging() {
    const QString dir = defaultLogDir();
    QDir().mkpath(dir);
    g_logFile = new QFile(QDir(dir).filePath("ii-greetd-ui.log"));
    if (!g_logFile->open(QIODevice::Append | QIODevice::Text)) {
        delete g_logFile;
        g_logFile = nullptr;
        return;
    }
    qInstallMessageHandler(messageHandler);
    qInfo() << "logging to" << g_logFile->fileName();
}

static bool loadMain(QQmlApplicationEngine &engine, const QString &module) {
    engine.loadFromModule(module, "Main");
    return !engine.rootObjects().isEmpty();
}

static bool loadFromQrc(QQmlApplicationEngine &engine, const QString &module) {
    const QStringList prefixes = {
        QStringLiteral("qrc:/qt/qml/%1/Main.qml").arg(module),
        QStringLiteral("qrc:/qt/qml/%1/qml/Main.qml").arg(module),
        QStringLiteral("qrc:/%1/Main.qml").arg(module),
        QStringLiteral("qrc:/%1/qml/Main.qml").arg(module),
    };
    for (const auto &path : prefixes) {
        engine.load(QUrl(path));
        if (!engine.rootObjects().isEmpty()) {
            return true;
        }
    }
    return false;
}

static QStringList themeSearchRoots() {
    QStringList roots;
    const QString home = QDir::homePath();
    if (!home.isEmpty()) {
        roots << QDir(home).filePath(".local/share/ii-greetd/themes");
    }
    roots << QStringLiteral("/usr/local/share/ii-greetd/themes");
    roots << QStringLiteral("/usr/share/ii-greetd/themes");
    const QString appDir = QCoreApplication::applicationDirPath();
    roots << QDir(appDir).filePath("../themes");
    roots << QDir(appDir).filePath("../../themes");
    roots << QDir(appDir).filePath("themes");
    return roots;
}

static QString themeDirCandidate(const QString &themeDir) {
    if (themeDir.isEmpty()) {
        return {};
    }
    QFileInfo info(themeDir);
    if (info.exists() && info.isFile()) {
        return info.absoluteFilePath();
    }
    const QString dirPath = info.exists() ? info.absoluteFilePath() : themeDir;
    return QDir(dirPath).filePath("Main.qml");
}

static QStringList themeNameCandidates(const QString &themeName) {
    QStringList candidates;
    if (themeName.isEmpty()) {
        return candidates;
    }
    const QStringList roots = themeSearchRoots();
    for (const auto &root : roots) {
        candidates << QDir(root).filePath(QString("%1/Main.qml").arg(themeName));
    }
    return candidates;
}

static QString firstExistingPath(const QStringList &candidates) {
    for (const auto &candidate : candidates) {
        if (QFileInfo::exists(candidate)) {
            return candidate;
        }
    }
    return {};
}

static bool loadFromThemeDir(QQmlApplicationEngine &engine, const QString &themeDir) {
    if (themeDir.isEmpty()) {
        return false;
    }
    const QString candidate = themeDirCandidate(themeDir);
    if (!QFileInfo::exists(candidate)) {
        return false;
    }
    engine.load(QUrl::fromLocalFile(candidate));
    return !engine.rootObjects().isEmpty();
}

static bool loadFromThemeName(QQmlApplicationEngine &engine, const QString &themeName) {
    if (themeName.isEmpty()) {
        return false;
    }
    const QStringList candidates = themeNameCandidates(themeName);
    for (const auto &candidate : candidates) {
        if (!QFileInfo::exists(candidate)) {
            continue;
        }
        engine.load(QUrl::fromLocalFile(candidate));
        if (!engine.rootObjects().isEmpty()) {
            return true;
        }
    }
    return false;
}

static bool loadFromDisk(QQmlApplicationEngine &engine) {
    const QString appDir = QCoreApplication::applicationDirPath();
    const QStringList candidates = {
        QDir(appDir).filePath("qml/Main.qml"),
        QDir(appDir).filePath("../qml/Main.qml"),
        QStringLiteral("/usr/local/share/ii-greetd/qml/Main.qml"),
        QStringLiteral("/usr/share/ii-greetd/qml/Main.qml"),
    };
    for (const auto &candidate : candidates) {
        if (!QFileInfo::exists(candidate)) {
            continue;
        }
        engine.load(QUrl::fromLocalFile(candidate));
        if (!engine.rootObjects().isEmpty()) {
            return true;
        }
    }
    return false;
}

static QString logFilePath() {
    if (g_logFile && g_logFile->isOpen()) {
        return g_logFile->fileName();
    }
    return QDir(defaultLogDir()).filePath("ii-greetd-ui.log");
}

static bool loadThemeError(QQmlApplicationEngine &engine, const QString &title, const QString &detail) {
    engine.rootContext()->setContextProperty("iiThemeErrorTitle", title);
    engine.rootContext()->setContextProperty("iiThemeErrorDetail", detail);
    engine.rootContext()->setContextProperty("iiThemeErrorHint", QStringLiteral("Fix the theme path or QML errors, then restart greetd."));
    static const char kErrorQml[] = R"QML(
import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import QtQuick.Window 2.15

ApplicationWindow {
    id: root
    property bool outputReady: Screen.width > 0 && Screen.height > 0
    width: outputReady ? Screen.width : 1280
    height: outputReady ? Screen.height : 720
    visible: true
    title: "ii-greetd: theme error"
    color: "#0e0f12"

    ColumnLayout {
        anchors.centerIn: parent
        spacing: 12
        width: parent.width * 0.8

        Text {
            text: iiThemeErrorTitle
            color: "#f2c1c1"
            font.pixelSize: 26
            font.bold: true
            horizontalAlignment: Text.AlignHCenter
            Layout.alignment: Qt.AlignHCenter
            wrapMode: Text.WordWrap
        }

        Text {
            text: iiThemeErrorDetail
            color: "#e1e5ea"
            font.pixelSize: 14
            horizontalAlignment: Text.AlignHCenter
            Layout.alignment: Qt.AlignHCenter
            wrapMode: Text.WordWrap
        }

        Text {
            text: iiThemeErrorHint
            color: "#9aa3ad"
            font.pixelSize: 12
            horizontalAlignment: Text.AlignHCenter
            Layout.alignment: Qt.AlignHCenter
            wrapMode: Text.WordWrap
        }
    }
}
)QML";
    engine.loadData(QByteArray(kErrorQml), QUrl("qrc:/ii-greetd-theme-error.qml"));
    return !engine.rootObjects().isEmpty();
}

int main(int argc, char *argv[]) {
    ensureCacheEnv();
    QGuiApplication app(argc, argv);
    initLogging();

    qmlRegisterType<BackendProcess>("IIGreetd", 1, 0, "BackendProcess");
    qmlRegisterType<BackendProcess>("IIGreeter", 1, 0, "BackendProcess");

    QQmlApplicationEngine engine;
    engine.setOutputWarningsToStandardError(true);
    QObject::connect(&engine, &QQmlEngine::warnings, [](const QList<QQmlError> &warnings) {
        QTextStream err(stderr);
        for (const auto &warning : warnings) {
            err << warning.toString() << '\n';
        }
    });
    const QString defaultUser = qEnvironmentVariable("II_GREETD_DEFAULT_USER");
    const bool lockUser = envBool("II_GREETD_LOCK_USER", false);
    const bool showPasswordToggle = envBool("II_GREETD_SHOW_PASSWORD_TOGGLE", true);
    const QStringList sessionCommand = parseSessionCommandJson(qEnvironmentVariable("II_GREETD_SESSION_JSON"));
    const QVariantMap sessionEnv = parseSessionEnvJson(qEnvironmentVariable("II_GREETD_SESSION_ENV_JSON"));
    const QVariantList sessions = parseSessionsJson(qEnvironmentVariable("II_GREETD_SESSIONS_JSON"));
    const QString lastSessionId = qEnvironmentVariable("II_GREETD_LAST_SESSION_ID");
    const QVariantList profiles = parseProfilesJson(qEnvironmentVariable("II_GREETD_PROFILES_JSON"));
    const QVariantMap locales = parseLocalesJson(qEnvironmentVariable("II_GREETD_LOCALES_JSON"));
    const QVariantList powerActions = parsePowerActionsJson(qEnvironmentVariable("II_GREETD_POWER_ACTIONS_JSON"));
    const QString lastProfileId = qEnvironmentVariable("II_GREETD_LAST_PROFILE_ID");
    const QString lastLocale = qEnvironmentVariable("II_GREETD_LAST_LOCALE");
    engine.rootContext()->setContextProperty("iiDefaultUser", defaultUser);
    engine.rootContext()->setContextProperty("iiLockUser", lockUser);
    engine.rootContext()->setContextProperty("iiShowPasswordToggle", showPasswordToggle);
    engine.rootContext()->setContextProperty("iiSessionCommand", sessionCommand);
    engine.rootContext()->setContextProperty("iiSessionEnv", sessionEnv);
    engine.rootContext()->setContextProperty("iiSessions", sessions);
    engine.rootContext()->setContextProperty("iiLastSessionId", lastSessionId);
    engine.rootContext()->setContextProperty("iiProfiles", profiles);
    engine.rootContext()->setContextProperty("iiLocales", locales);
    engine.rootContext()->setContextProperty("iiPowerActions", powerActions);
    engine.rootContext()->setContextProperty("iiLastProfileId", lastProfileId);
    engine.rootContext()->setContextProperty("iiLastLocale", lastLocale);
    const bool qmlUriExplicit = !qEnvironmentVariableIsEmpty("II_GREETD_QML_URI");
    QString qmlUri = qEnvironmentVariable("II_GREETD_QML_URI");
    if (qmlUri.isEmpty()) {
        qmlUri = QStringLiteral("IIGreetd");
    }
    QString qmlFileOverride = qEnvironmentVariable("II_GREETD_QML_FILE");
    QString themeDir = qEnvironmentVariable("II_GREETD_THEME_DIR");
    QString themeName = qEnvironmentVariable("II_GREETD_THEME");

    bool loaded = false;
    QString errorDetail;
    const bool qmlFileExplicit = !qmlFileOverride.isEmpty();
    const bool themeDirExplicit = !themeDir.isEmpty();
    const bool themeNameExplicit = !themeName.isEmpty();

    if (qmlFileExplicit) {
        if (QFileInfo::exists(qmlFileOverride)) {
            engine.load(QUrl::fromLocalFile(qmlFileOverride));
            loaded = !engine.rootObjects().isEmpty();
        }
        if (!loaded) {
            if (QFileInfo::exists(qmlFileOverride)) {
                errorDetail = QString("Failed to load QML file: %1").arg(qmlFileOverride);
            } else {
                errorDetail = QString("QML file not found: %1").arg(qmlFileOverride);
            }
        }
    } else if (themeDirExplicit) {
        const QString candidate = themeDirCandidate(themeDir);
        loaded = loadFromThemeDir(engine, themeDir);
        if (!loaded) {
            if (!candidate.isEmpty() && QFileInfo::exists(candidate)) {
                errorDetail = QString("Failed to load theme dir: %1").arg(themeDir);
            } else {
                errorDetail = QString("Theme dir missing Main.qml: %1").arg(candidate);
            }
        }
    } else if (themeNameExplicit) {
        const QStringList candidates = themeNameCandidates(themeName);
        const QString existing = firstExistingPath(candidates);
        loaded = loadFromThemeName(engine, themeName);
        if (!loaded) {
            if (!existing.isEmpty()) {
                errorDetail = QString("Failed to load theme '%1': %2").arg(themeName, existing);
            } else {
                errorDetail = QString("Theme not found: %1\nSearched:\n- %2")
                                   .arg(themeName, candidates.join("\n- "));
            }
        }
    } else if (qmlUriExplicit) {
        loaded = loadMain(engine, qmlUri);
        if (!loaded) {
            loaded = loadFromQrc(engine, qmlUri);
        }
        if (!loaded) {
            errorDetail = QString("Failed to load QML module: %1 (Main.qml)").arg(qmlUri);
        }
    } else {
        loaded = loadMain(engine, qmlUri);
        if (!loaded) {
            loaded = loadFromQrc(engine, qmlUri);
        }
        if (!loaded) {
            loaded = loadFromDisk(engine);
        }
        if (!loaded) {
            errorDetail = "No QML theme found in built-in or system locations.";
        }
    }

    if (!loaded) {
        const QString detail = QString("%1\nLog: %2")
                                   .arg(errorDetail.isEmpty() ? "Theme load failed." : errorDetail,
                                        logFilePath());
        qWarning() << "theme load failed:" << detail;
        if (!loadThemeError(engine, "Theme load failed", detail)) {
            return 1;
        }
    }

    return app.exec();
}
