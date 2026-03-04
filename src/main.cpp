#include <QApplication>
#include <QTranslator>
#include <QLocale>
#include <QDir>
#include <QIcon>
#include <QFile>
#include <QCoreApplication>
#include <QEvent>
#include <QFileOpenEvent>
#include <functional>
#include "MainWindow.h"

class IdefApplication final : public QApplication {
public:
  using QApplication::QApplication;

  QString pendingFileOpen;
  std::function<void(const QString&)> fileOpenHandler;

  bool event(QEvent* ev) override {
    if (ev->type() == QEvent::FileOpen) {
      auto* foe = static_cast<QFileOpenEvent*>(ev);
      if (foe && !foe->file().isEmpty()) {
        if (fileOpenHandler) {
          fileOpenHandler(foe->file());
        } else {
          pendingFileOpen = foe->file();
        }
        return true;
      }
    }
    return QApplication::event(ev);
  }
};

int main(int argc, char** argv) {
  IdefApplication app(argc, argv);
  const QString resIcon = QStringLiteral(":/icons/erlu.svg");
  if (QFile::exists(resIcon)) {
    app.setWindowIcon(QIcon(resIcon));
  } else {
    const QString appDir = QCoreApplication::applicationDirPath();
    const QString fallback1 = appDir + QStringLiteral("/../share/icons/hicolor/scalable/apps/erlu.svg");
    const QString fallback2 = QDir::currentPath() + QStringLiteral("/assets/icons/erlu.svg");
    if (QFile::exists(fallback1)) {
      app.setWindowIcon(QIcon(fallback1));
    } else if (QFile::exists(fallback2)) {
      app.setWindowIcon(QIcon(fallback2));
    }
  }

  const QLocale systemLocale = QLocale::system();
  QLocale::setDefault(systemLocale);

  QTranslator translator;
  QStringList candidates;
  auto addCandidate = [&](const QString& c) {
    if (c.isEmpty()) return;
    const QString normalized = QString(c).replace('-', '_');
    if (normalized == QStringLiteral("C") || normalized == QStringLiteral("POSIX")) return;
    candidates << normalized;
    const QString shortName = normalized.section('_', 0, 0);
    if (!shortName.isEmpty()) candidates << shortName;
  };
  addCandidate(QLocale().name());
  addCandidate(systemLocale.name());
  for (const QString& lang : systemLocale.uiLanguages()) {
    addCandidate(lang);
  }
  addCandidate(QString::fromLocal8Bit(qgetenv("LANG")).section('.', 0, 0));
  candidates.removeDuplicates();
  if (candidates.isEmpty()) candidates << QStringLiteral("en");

  const QStringList searchPaths = {
    QDir::currentPath() + "/translations",
    QCoreApplication::applicationDirPath() + "/../share/idef0/translations",
    ":/i18n"
  };
  bool loaded = false;
  for (const QString& loc : candidates) {
    const QString baseName = QStringLiteral("idef0_%1").arg(loc);
    for (const QString& path : searchPaths) {
      if (translator.load(baseName, path)) {
        app.installTranslator(&translator);
        loaded = true;
        break;
      }
    }
    if (loaded) break;
  }
  if (!loaded) {
    for (const QString& path : searchPaths) {
      if (translator.load(QStringLiteral("idef0_en"), path)) {
        app.installTranslator(&translator);
        break;
      }
    }
  }

  QString startupPath;
  const QStringList args = QCoreApplication::arguments();
  if (args.size() > 1) {
    startupPath = args.at(1);
  }
  if (!app.pendingFileOpen.isEmpty()) {
    startupPath = app.pendingFileOpen;
  }

  MainWindow w(startupPath);
  app.fileOpenHandler = [&w](const QString& path) {
    w.openDiagramPath(path);
  };
  w.resize(1150, 850);
  w.show();
  return app.exec();
}
