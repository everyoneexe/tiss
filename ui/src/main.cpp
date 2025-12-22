#include <QCoreApplication>
#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QTextStream>

#include "BackendProcess.h"

static void ensureCacheEnv() {
    if (qEnvironmentVariableIsEmpty("QML_DISABLE_DISK_CACHE")) {
        qputenv("QML_DISABLE_DISK_CACHE", "1");
    }

    QByteArray cacheHome = qgetenv("XDG_CACHE_HOME");
    if (cacheHome.isEmpty()) {
        const QString fallback = QDir::temp().filePath("ii-greetd-cache");
        QDir().mkpath(fallback);
        qputenv("XDG_CACHE_HOME", fallback.toUtf8());
    } else {
        const QString cachePath = QString::fromUtf8(cacheHome);
        QDir().mkpath(cachePath);
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
    const bool lockUser = !qEnvironmentVariableIsEmpty("II_GREETD_LOCK_USER");
    engine.rootContext()->setContextProperty("iiDefaultUser", defaultUser);
    engine.rootContext()->setContextProperty("iiLockUser", lockUser);
    const QString qmlUri = qEnvironmentVariable("II_GREETD_QML_URI").isEmpty()
                               ? QStringLiteral("IIGreetd")
                               : qEnvironmentVariable("II_GREETD_QML_URI");
    const QString qmlFileOverride = qEnvironmentVariable("II_GREETD_QML_FILE");

    bool loaded = false;
    if (!qmlFileOverride.isEmpty()) {
        engine.load(QUrl::fromLocalFile(qmlFileOverride));
        loaded = !engine.rootObjects().isEmpty();
    }
    if (!loaded) {
        loaded = loadMain(engine, qmlUri);
    }
    if (!loaded) {
        loaded = loadFromQrc(engine, qmlUri);
    }
    if (!loaded) {
        loaded = loadFromDisk(engine);
    }
    if (!loaded) {
        return 1;
    }

    return app.exec();
}
