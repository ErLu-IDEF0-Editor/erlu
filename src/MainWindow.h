#pragma once
#include <QMainWindow>

class QGraphicsView;
class DiagramScene;

class MainWindow final : public QMainWindow {
  Q_OBJECT
public:
  explicit MainWindow(QWidget* parent = nullptr);

private:
  QGraphicsView* m_view = nullptr;
  DiagramScene* m_scene = nullptr;

  void setupUi();
  void setupActions();
};
