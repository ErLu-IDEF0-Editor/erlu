#include "plugins/PluginManager.h"

#include <QCoreApplication>
#include <QDir>
#include <QMenu>
#include <QDebug>
#include <QFileInfo>
#include <QColor>

#include "MainWindow.h"
#include "items/DiagramScene.h"
#include "items/BlockItem.h"
#include "items/ArrowItem.h"

PluginManager::PluginManager(MainWindow* window, QMenu* pluginsMenu, QObject* parent)
  : QObject(parent),
  m_window(window),
    m_pluginsMenu(pluginsMenu)
{
}

QMenu* PluginManager::pluginsMenu() const {
  return m_pluginsMenu;
}

DiagramScene* PluginManager::currentScene() const {
  return m_window ? m_window->scene() : nullptr;
}

QWidget* PluginManager::window() const {
  return m_window;
}

QStringList PluginManager::pluginSearchPaths() const {
  QStringList paths;
  const QString appDir = QCoreApplication::applicationDirPath();
  paths << (appDir + "/plugins");
  paths << (appDir + "/../plugins");
  return paths;
}

void PluginManager::loadPlugins() {
  const QStringList roots = pluginSearchPaths();
  qInfo() << "PluginManager scanning paths:" << roots;
  QStringList queue = roots;
  while (!queue.isEmpty()) {
    const QString dirPath = queue.takeFirst();
    QDir dir(dirPath);
    if (!dir.exists()) continue;
    // Enqueue subdirectories for recursive scan (one level deep is enough here).
    const auto subdirs = dir.entryInfoList(QDir::Dirs | QDir::NoDotAndDotDot);
    for (const QFileInfo& di : subdirs) {
      queue << di.absoluteFilePath();
    }
    // Load plugins from this directory.
    const auto entries = dir.entryInfoList(QDir::Files);
    for (const QFileInfo& fi : entries) {
      if (!QLibrary::isLibrary(fi.absoluteFilePath())) continue;
      auto lib = std::make_unique<QLibrary>(fi.absoluteFilePath());
      if (!lib->load()) {
        qWarning() << "Plugin load failed:" << fi.absoluteFilePath() << lib->errorString();
        continue;
      }
      auto init = reinterpret_cast<Idef0PluginInit>(lib->resolve("idef0_plugin_init_v1"));
      if (!init) {
        qWarning() << "Plugin missing entrypoint idef0_plugin_init_v1:" << fi.absoluteFilePath();
        continue;
      }
      auto ctx = makeContext(fi.absoluteFilePath());
      if (!ctx) continue;
      if (!init(&ctx->host)) {
        qWarning() << "Plugin init returned false:" << fi.absoluteFilePath();
        continue;
      }
      qInfo() << "Plugin loaded:" << fi.absoluteFilePath();
      m_plugins.emplace_back(std::move(lib), std::move(ctx));
    }
  }
}

std::unique_ptr<PluginManager::HostContext> PluginManager::makeContext(const QString& pluginPath) {
  auto ctx = std::make_unique<HostContext>();
  ctx->manager = this;
  ctx->pluginPath = pluginPath;
  ctx->pluginDirUtf8 = QFileInfo(pluginPath).absolutePath().toUtf8();
  ctx->host.opaque = ctx.get();
  ctx->host.selected_items = &PluginManager::apiSelectedItems;
  ctx->host.set_item_color = &PluginManager::apiSetItemColor;
  ctx->host.clear_item_color = &PluginManager::apiClearItemColor;
  ctx->host.add_menu_action = &PluginManager::apiAddMenuAction;
  ctx->host.plugin_dir = &PluginManager::apiPluginDir;
  return ctx;
}

void PluginManager::addAction(const QString& text, Idef0ActionCallback cb, void* userData, HostContext* ctx) {
  if (!m_pluginsMenu || !cb || !ctx) return;
  QAction* act = m_pluginsMenu->addAction(text);
  connect(act, &QAction::triggered, this, [cb, userData, ctx]{
    cb(userData, &ctx->host);
  });
}

size_t PluginManager::apiSelectedItems(void* opaque, Idef0SelectedItem* out, size_t max) {
  auto* ctx = static_cast<HostContext*>(opaque);
  if (!ctx || !ctx->manager || !ctx->manager->m_window || !out || max == 0) return 0;
  DiagramScene* scene = ctx->manager->currentScene();
  if (!scene) return 0;
  const auto sel = scene->selectedItems();
  const size_t selCount = static_cast<size_t>(sel.size());
  const size_t count = std::min(selCount, max);
  for (size_t i = 0; i < count; ++i) {
    Idef0SelectedItem item{0, -1};
    if (auto* b = qgraphicsitem_cast<BlockItem*>(sel[int(i)])) {
      item.kind = IDEF0_ITEM_BLOCK;
      item.id = b->id();
    } else if (auto* a = qgraphicsitem_cast<ArrowItem*>(sel[int(i)])) {
      item.kind = IDEF0_ITEM_ARROW;
      item.id = a->internalId();
    }
    out[i] = item;
  }
  return count;
}

static bool applyColorToScene(DiagramScene* scene, int kind, int id, const QColor& col, bool clear) {
  if (!scene) return false;
  bool changed = false;
  const auto items = scene->items();
  for (QGraphicsItem* it : items) {
    if (kind == IDEF0_ITEM_BLOCK) {
      if (auto* b = qgraphicsitem_cast<BlockItem*>(it)) {
        if (b->id() == id) {
          clear ? b->clearCustomColor() : b->setCustomColor(col);
          changed = true;
        }
      }
    } else if (kind == IDEF0_ITEM_ARROW) {
      if (auto* a = qgraphicsitem_cast<ArrowItem*>(it)) {
        if (a->internalId() == id) {
          clear ? a->clearCustomColor() : a->setCustomColor(col);
          a->updatePath();
          changed = true;
        }
      }
    }
  }
  if (changed) scene->requestSnapshot();
  return changed;
}

bool PluginManager::apiSetItemColor(void* opaque, int kind, int id, const char* color_hex) {
  auto* ctx = static_cast<HostContext*>(opaque);
  if (!ctx || !ctx->manager || !ctx->manager->m_window || !color_hex) return false;
  QColor c(color_hex);
  if (!c.isValid()) return false;
  return applyColorToScene(ctx->manager->currentScene(), kind, id, c, false);
}

bool PluginManager::apiClearItemColor(void* opaque, int kind, int id) {
  auto* ctx = static_cast<HostContext*>(opaque);
  if (!ctx || !ctx->manager || !ctx->manager->m_window) return false;
  return applyColorToScene(ctx->manager->currentScene(), kind, id, QColor(), true);
}

void PluginManager::apiAddMenuAction(void* opaque, const char* text, Idef0ActionCallback cb, void* user_data) {
  auto* ctx = static_cast<HostContext*>(opaque);
  if (!ctx || !ctx->manager || !text || !cb) return;
  ctx->manager->addAction(QString::fromUtf8(text), cb, user_data, ctx);
}

const char* PluginManager::apiPluginDir(void* opaque) {
  auto* ctx = static_cast<HostContext*>(opaque);
  if (!ctx) return nullptr;
  return ctx->pluginDirUtf8.constData();
}
