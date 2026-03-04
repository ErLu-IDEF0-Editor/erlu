#pragma once
#include <QMainWindow>
#include <QVariant>
class QLineEdit;
class QCloseEvent;

class QGraphicsView;
class DiagramScene;
class QDialog;
class QAction;
class PluginManager;

class MainWindow final : public QMainWindow {
  Q_OBJECT
public:
  explicit MainWindow(const QString& startupPath = QString(), QWidget* parent = nullptr);
  bool openDiagramPath(const QString& path);
  DiagramScene* scene() const { return m_scene; }

protected:
  bool eventFilter(QObject* obj, QEvent* event) override;

private:
  QGraphicsView* m_view = nullptr;
  DiagramScene* m_scene = nullptr;
  QDialog* m_welcomeDialog = nullptr;
  QVariantMap m_welcomeState;
  QString m_currentFile;
  bool m_dirty = false;
  bool m_pinchHandled = false;
  QAction* m_actFollowSystemTheme = nullptr;
  QAction* m_actDarkMode = nullptr;
  PluginManager* m_pluginManager = nullptr;

  void setupUi();
  void setupActions();
  bool showWelcome();
  void closeEvent(QCloseEvent* e) override;
  void resetScene();
  void newDiagram();
  bool promptStartup();
  bool loadDiagramFromPath(const QString& path);
  bool exportPdf(bool allDiagrams, bool numberPages, bool forceLightTheme);
  bool exportMarkdownExplanation();
  void runValidation();
  void calculatePrices();
  bool effectiveDarkMode() const;
  void applyAppPalette(bool darkMode);
  void autoInitialsFromAuthor(QLineEdit* author, QLineEdit* initials);
  void showNodeTreeDialog();
  void markDirty(bool dirty);
  void updateWindowTitle();
};
