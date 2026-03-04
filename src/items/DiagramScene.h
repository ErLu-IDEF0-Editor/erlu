#pragma once
#include <QGraphicsScene>
#include <QPointer>
#include <optional>
#include <QVariantMap>

#include "BlockItem.h"
class ArrowItem;
class JunctionItem;
class HeaderFooterItem;

class DiagramScene final : public QGraphicsScene {
  Q_OBJECT
public:
  explicit DiagramScene(QObject* parent = nullptr);

  BlockItem* createBlockAt(const QPointF& scenePos);
  BlockItem* createBlockWithId(const QPointF& scenePos, int id, const QString& title);
  void requestSnapshot();
  void applyMeta(const QVariantMap& meta);
  void updateMeta(const QVariantMap& patch);
  QVariantMap meta() const { return m_meta; }
  QRectF contentRect() const { return m_contentRect; }
  QString currentNodeLabel() const;
  QString currentDiagramTitle() const;
  int currentBlockId() const { return m_currentBlockId; }
  bool goDownIntoSelected();
  bool goDownIntoBlock(BlockItem* b);
  bool goUp();
  void propagateLabelFrom(ArrowItem* root);
  QVariantMap exportToVariant();
  bool importFromVariant(const QVariantMap& map);
  bool startCallMechanism(BlockItem* origin, BlockItem* refBlock, const QString& label);
  bool hasCallMechanism(const BlockItem* origin) const;
  void updateCallMechanismLabels();
signals:
  void changed();
  void metaChanged(const QVariantMap& meta);

protected:
  void mousePressEvent(QGraphicsSceneMouseEvent* e) override;
  void mouseMoveEvent(QGraphicsSceneMouseEvent* e) override;
  void mouseReleaseEvent(QGraphicsSceneMouseEvent* e) override;
  void keyPressEvent(QKeyEvent* e) override;

private:
  struct Snapshot {
    struct Endpoint {
      enum class Kind { None, Block, Junction, Scene } kind = Kind::None;
      int id = -1;
      BlockItem::Port port = BlockItem::Port::Input;
      std::optional<QPointF> localPos;
      std::optional<QPointF> scenePos;
      bool tunneled = false;
    };
    struct Block { int id; QString title; QPointF pos; bool hasDecomp = false; QString number; std::optional<qreal> price; std::optional<QString> color; };
    struct Junction { int id; QPointF pos; };
    struct Arrow {
      Endpoint from;
      Endpoint to;
      qreal bend = 0.0;
      qreal top = 0.0;
      qreal bottom = 0.0;
      QString label;
      QPointF labelOffset;
      bool labelHidden = false;
      bool labelInherited = false;
      bool isInterface = false;
      bool isInterfaceStub = false;
      bool labelLocked = false;
      Endpoint interfaceEdge;
      std::optional<QString> color;
      bool callMechanism = false;
      int callRefId = -1;
    };
    QString state;
    bool hasState = false;
    QVector<Block> blocks;
    QVector<Junction> junctions;
    QVector<Arrow> arrows;
  };

  struct HierNode { int blockId; Snapshot snapshot; QString prefix; };

  ArrowItem* m_dragArrow = nullptr;
  QPointer<BlockItem> m_dragFromBlock;
  QPointer<JunctionItem> m_dragFromJunction;
  BlockItem::Port m_dragFromPort = BlockItem::Port::Output;
  int m_nextBlockId = 1;
  int m_nextJunctionId = 1;
  QString m_currentPrefix = QStringLiteral("A");
  bool m_snapshotScheduled = false;
  bool m_restoringSnapshot = false;
  bool m_itemDragActive = false;
  QHash<QGraphicsItem*, QPointF> m_pressPositions;
  int m_currentBlockId = -1;
  QVector<HierNode> m_hierarchy;
  QHash<QString, Snapshot> m_children; // hierarchical key ("1/2/3") -> saved child diagram
  ArrowItem* m_dragInterfaceStub = nullptr;
  QVariantMap m_meta;
  HeaderFooterItem* m_headerFooter = nullptr;
  QRectF m_contentRect;

  bool tryStartArrowDrag(const QPointF& scenePos, Qt::KeyboardModifiers mods);
  bool tryFinishArrowDrag(const QPointF& scenePos);
  bool tryBranchAtArrow(const QPointF& scenePos);
  void cancelCurrentDrag();
  void deleteSelection();

  QVector<Snapshot> m_history;
  int m_historyIndex = -1;

  Snapshot captureSnapshot() const;
  void restoreSnapshot(const Snapshot& snap, bool resetHistoryState = true);
  void pushSnapshot();
  void scheduleSnapshot();
  void undo();
  void redo();
  void maybeSnapshotMovedItems();
  void resetHistory(const Snapshot& base);
  void ensureFrame();
  void ensureHeaderFooter();
  bool childHasContent(int blockId) const;
  QString currentPathKey() const;
  QString childKeyFor(int blockId) const;
  QString assignNumber(BlockItem* b);
  void connectBlockSignals(BlockItem* b);
  void purgeBrokenCallMechanisms();

  enum class Edge { None, Left, Right, Top, Bottom };
  Edge hitTestEdge(const QPointF& scenePos, QPointF* outScenePoint = nullptr) const;

  friend QJsonObject endpointToJson(const Snapshot::Endpoint& ep);
  friend Snapshot::Endpoint endpointFromJson(const QJsonObject& o);
};
