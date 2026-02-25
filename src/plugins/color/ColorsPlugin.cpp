#include "plugins/color/ColorsPlugin.h"

#include <QColorDialog>
#include <QObject>
#include <QTranslator>
#include <QLocale>
#include <QCoreApplication>
#include <QDebug>

static void ensureTranslator(Idef0Host* host) {
  static bool loaded = false;
  static QTranslator* translator = nullptr;
  if (loaded || !host || !host->plugin_dir) return;
  const char* dirC = host->plugin_dir(host->opaque);
  if (!dirC) return;
  const QString dir = QString::fromUtf8(dirC);
  const QString baseLocale = QLocale().name().replace('-', '_');
  const QString shortLocale = baseLocale.section('_', 0, 0);

  QStringList candidates;
  auto addCandidate = [&](const QString& c) {
    if (!c.isEmpty() && !c.contains("C")) candidates << c;
  };
  addCandidate(baseLocale);
  addCandidate(shortLocale);
  const QString sysName = QLocale::system().name().replace('-', '_');
  addCandidate(sysName);
  addCandidate(sysName.section('_', 0, 0));
  for (const QString& lang : QLocale::system().uiLanguages()) {
    const QString norm = QString(lang).replace('-', '_');
    addCandidate(norm);
    addCandidate(norm.section('_', 0, 0));
  }
  const QString envLang = QString::fromLocal8Bit(qgetenv("LANG")).section('.', 0, 0).replace('-', '_');
  addCandidate(envLang);
  addCandidate(envLang.section('_', 0, 0));

  candidates.removeDuplicates();
  if (candidates.isEmpty()) candidates << QStringLiteral("en");

  translator = new QTranslator(qApp);
  qInfo() << "[colors plugin] translator search in" << dir << "candidates" << candidates;
  for (const QString& loc : candidates) {
    if (loc.isEmpty()) continue;
    const QString baseName = QStringLiteral("colors_%1").arg(loc);
    if (translator->load(baseName, dir + "/translations")) {
      qApp->installTranslator(translator);
      qInfo() << "[colors plugin] translator loaded" << baseName;
      loaded = true;
      break;
    }
  }
  if (!loaded) {
    qWarning() << "[colors plugin] translator not found, falling back to default language";
    delete translator;
    translator = nullptr;
  }
}

static void onSetColor(void*, Idef0Host* host) {
  if (!host || !host->selected_items || !host->set_item_color) return;
  ensureTranslator(host);
  Idef0SelectedItem items[64];
  const size_t count = host->selected_items(host->opaque, items, 64);
  if (count == 0) return;
  QColor initial("#2b6ee6");
  const QColor chosen = QColorDialog::getColor(initial, nullptr, QObject::tr("Select item color"));
  if (!chosen.isValid()) return;
  const QByteArray hex = chosen.name(QColor::HexRgb).toUtf8();
  for (size_t i = 0; i < count; ++i) {
    host->set_item_color(host->opaque, items[i].kind, items[i].id, hex.constData());
  }
}

static void onClearColor(void*, Idef0Host* host) {
  if (!host || !host->selected_items || !host->clear_item_color) return;
  ensureTranslator(host);
  Idef0SelectedItem items[64];
  const size_t count = host->selected_items(host->opaque, items, 64);
  for (size_t i = 0; i < count; ++i) {
    host->clear_item_color(host->opaque, items[i].kind, items[i].id);
  }
}

extern "C" bool idef0_plugin_init_v1(Idef0Host* host) {
  if (!host || !host->add_menu_action) return false;
  ensureTranslator(host);
  qInfo() << "[colors plugin] init, plugin dir:" << (host->plugin_dir ? host->plugin_dir(host->opaque) : "(none)");
  host->add_menu_action(host->opaque, QObject::tr("Set item color…").toUtf8().constData(), &onSetColor, nullptr);
  host->add_menu_action(host->opaque, QObject::tr("Clear item colors").toUtf8().constData(), &onClearColor, nullptr);
  return true;
}
