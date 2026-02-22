#pragma once
#include <QGraphicsScene>
#include <QPointer>

#include "BlockItem.h"
class ArrowItem;
class JunctionItem;

class DiagramScene final : public QGraphicsScene {
  Q_OBJECT
public:
  explicit DiagramScene(QObject* parent = nullptr);

  BlockItem* createBlockAt(const QPointF& scenePos);
  BlockItem* createBlockWithId(const QPointF& scenePos, int id, const QString& title);
  void requestSnapshot();

protected:
  void mousePressEvent(QGraphicsSceneMouseEvent* e) override;
  void mouseMoveEvent(QGraphicsSceneMouseEvent* e) override;
  void mouseReleaseEvent(QGraphicsSceneMouseEvent* e) override;
  void keyPressEvent(QKeyEvent* e) override;

private:
  ArrowItem* m_dragArrow = nullptr;
  QPointer<BlockItem> m_dragFromBlock;
  QPointer<JunctionItem> m_dragFromJunction;
  BlockItem::Port m_dragFromPort = BlockItem::Port::Output;
  int m_nextBlockId = 1;
  int m_nextJunctionId = 1;
  bool m_snapshotScheduled = false;
  bool m_restoringSnapshot = false;
  bool m_itemDragActive = false;
  QHash<QGraphicsItem*, QPointF> m_pressPositions;

  bool tryStartArrowDrag(const QPointF& scenePos);
  bool tryFinishArrowDrag(const QPointF& scenePos);
  bool tryBranchAtArrow(const QPointF& scenePos);
  void cancelCurrentDrag();
  void deleteSelection();

  struct Snapshot {
    struct Endpoint {
      enum class Kind { None, Block, Junction, Scene } kind = Kind::None;
      int id = -1;
      BlockItem::Port port = BlockItem::Port::Input;
      QPointF localPos;
      QPointF scenePos;
    };
    struct Block { int id; QString title; QPointF pos; };
    struct Junction { int id; QPointF pos; };
    struct Arrow {
      Endpoint from;
      Endpoint to;
      qreal bend = 0.0;
      qreal top = 0.0;
      qreal bottom = 0.0;
      QString label;
      QPointF labelOffset;
    };
    QVector<Block> blocks;
    QVector<Junction> junctions;
    QVector<Arrow> arrows;
  };

  QVector<Snapshot> m_history;
  int m_historyIndex = -1;

  Snapshot captureSnapshot() const;
  void restoreSnapshot(const Snapshot& snap);
  void pushSnapshot();
  void scheduleSnapshot();
  void undo();
  void maybeSnapshotMovedItems();

  enum class Edge { None, Left, Right, Top, Bottom };
  Edge hitTestEdge(const QPointF& scenePos, QPointF* outScenePoint = nullptr) const;
};
