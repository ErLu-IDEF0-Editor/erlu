#include <QApplication>
#include <QTranslator>
#include <QLocale>
#include <QDir>
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

  QTranslator translator;
  const QLocale locale;
  const QString baseName = QStringLiteral("idef0_%1").arg(locale.name());
  const QStringList searchPaths = {
    QDir::currentPath() + "/translations",
    QCoreApplication::applicationDirPath() + "/../share/idef0/translations",
    ":/i18n"
  };
  for (const QString& path : searchPaths) {
    if (translator.load(baseName, path)) {
      app.installTranslator(&translator);
      break;
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
  w.resize(1200, 800);
  w.show();
  return app.exec();
}
