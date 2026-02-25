#pragma once

#include <QObject>
#include <QLibrary>
#include <memory>
#include <vector>

#include "plugins/PluginApi.h"

class QMenu;
class MainWindow;
class DiagramScene;

class PluginManager final : public QObject {
  Q_OBJECT
public:
  explicit PluginManager(MainWindow* window, QMenu* pluginsMenu, QObject* parent = nullptr);
  void loadPlugins();

  QMenu* pluginsMenu() const;
  DiagramScene* currentScene() const;
  QWidget* window() const;

private:
  struct HostContext {
    PluginManager* manager = nullptr;
    QString pluginPath;
    QByteArray pluginDirUtf8;
    Idef0Host host;
  };
  struct LoadedPlugin {
    std::unique_ptr<QLibrary> lib;
    std::unique_ptr<HostContext> ctx;
    LoadedPlugin() = default;
    LoadedPlugin(std::unique_ptr<QLibrary>&& l, std::unique_ptr<HostContext>&& c)
      : lib(std::move(l)), ctx(std::move(c)) {}
    LoadedPlugin(LoadedPlugin&&) noexcept = default;
    LoadedPlugin& operator=(LoadedPlugin&&) noexcept = default;
    LoadedPlugin(const LoadedPlugin&) = delete;
    LoadedPlugin& operator=(const LoadedPlugin&) = delete;
  };

  QStringList pluginSearchPaths() const;
  std::unique_ptr<HostContext> makeContext(const QString& pluginPath);
  void addAction(const QString& text, Idef0ActionCallback cb, void* userData, HostContext* ctx);
  static size_t apiSelectedItems(void* opaque, Idef0SelectedItem* out, size_t max);
  static bool apiSetItemColor(void* opaque, int kind, int id, const char* color_hex);
  static bool apiClearItemColor(void* opaque, int kind, int id);
  static void apiAddMenuAction(void* opaque, const char* text, Idef0ActionCallback cb, void* user_data);
  static const char* apiPluginDir(void* opaque);

  MainWindow* m_window = nullptr;
  QMenu* m_pluginsMenu = nullptr;
  std::vector<LoadedPlugin> m_plugins;
};
