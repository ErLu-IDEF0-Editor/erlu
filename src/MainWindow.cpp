#include "MainWindow.h"
#include "items/DiagramScene.h"

#include <QGraphicsView>
#include <QToolBar>
#include <QAction>
#include <QStatusBar>

MainWindow::MainWindow(QWidget* parent)
  : QMainWindow(parent)
{
  setupUi();
  setupActions();
  statusBar()->showMessage("RMB: add block. LMB on port: drag arrow. Double click block: rename.");
}

void MainWindow::setupUi() {
  m_scene = new DiagramScene(this);
  m_view = new QGraphicsView(m_scene, this);
  m_view->setRenderHint(QPainter::Antialiasing, true);
  m_view->setDragMode(QGraphicsView::RubberBandDrag);
  // Полное обновление избавляет от графических артефактов во время drag временной стрелки.
  m_view->setViewportUpdateMode(QGraphicsView::FullViewportUpdate);

  setCentralWidget(m_view);

  // стартовые блоки
  m_scene->createBlockAt(QPointF(-200, -50))->setPos(-300, -150);
  m_scene->createBlockAt(QPointF(200, -50))->setPos(200, -150);
}

void MainWindow::setupActions() {
  auto* tb = addToolBar("Tools");
  auto* actFit = new QAction("Fit", this);
  connect(actFit, &QAction::triggered, this, [this]{
    m_view->fitInView(m_scene->itemsBoundingRect().adjusted(-50,-50,50,50), Qt::KeepAspectRatio);
  });
  tb->addAction(actFit);
}
