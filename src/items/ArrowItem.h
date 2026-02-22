#pragma once
#include <QGraphicsPathItem>
#include <QPointer>
#include <optional>
#include <QString>

#include "BlockItem.h"
class JunctionItem;
class QGraphicsSimpleTextItem;

class ArrowItem final : public QGraphicsPathItem {
public:
  enum { Type = UserType + 3 };
  struct Endpoint {
    QPointer<BlockItem> block;
    QPointer<JunctionItem> junction;
    BlockItem::Port port = BlockItem::Port::Input;
    std::optional<QPointF> localPos;  // точка на стороне блока (в локальных координатах)
    std::optional<QPointF> scenePos;  // произвольная точка на сцене (например, граница)
  };

  ArrowItem(QGraphicsItem* parent = nullptr);
  ~ArrowItem() override;

  void setFrom(const Endpoint& ep);
  void setTo(const Endpoint& ep);

  void setTempEndPoint(const QPointF& scenePos); // когда тянем мышью
  void finalize(); // убираем "временный" режим, если надо

  Endpoint from() const { return m_from; }
  Endpoint to() const { return m_to; }

  QString label() const { return m_label; }
  void setLabel(const QString& text);
  QPointF labelOffset() const { return m_labelOffset; }
  void setOffsets(qreal bend, qreal top, qreal bottom);
  void setLabelOffset(const QPointF& off);
  qreal bendOffset() const { return m_bendOffset; }
  qreal topOffset() const { return m_topOffset; }
  qreal bottomOffset() const { return m_bottomOffset; }
  int type() const override { return Type; }

  void updatePath();
  std::optional<QPointF> hitTest(const QPointF& scenePos, qreal radius) const;

private:
  enum class DragPart { None, BendX, TopHorizontal, BottomHorizontal };

  Endpoint m_from;
  Endpoint m_to;
  bool m_hasTempEnd = false;
  QPointF m_tempEndScene;
  qreal m_bendOffset = 0.0;
  qreal m_topOffset = 0.0;
  qreal m_bottomOffset = 0.0;
  QString m_label;
  QGraphicsSimpleTextItem* m_labelItem = nullptr;
  QPointF m_labelOffset; // in scene-space relative to anchor point
  bool m_labelDragging = false;
  QPointF m_labelDragLastScene;
  QVector<QPointF> m_lastPolyline;

  DragPart m_dragPart = DragPart::None;
  QPointF m_lastDragScenePos;

  void drawArrowHead(QPainterPath& path, const QPointF& tip, const QPointF& from);
  QVector<QPointF> computePolyline() const;
  static QPointF portDirection(const Endpoint& ep, bool isFrom, const QPointF& fallbackDir, const QGraphicsScene* scene);
  static QVector<QPointF> simplifyPolyline(const QVector<QPointF>& pts);
  static qreal distToSegment(const QPointF& p, const QPointF& a, const QPointF& b);
  void updateLabelItem(const QVector<QPointF>& pts);

protected:
  void hoverMoveEvent(QGraphicsSceneHoverEvent* e) override;
  void mousePressEvent(QGraphicsSceneMouseEvent* e) override;
  void mouseMoveEvent(QGraphicsSceneMouseEvent* e) override;
  void mouseReleaseEvent(QGraphicsSceneMouseEvent* e) override;
  void mouseDoubleClickEvent(QGraphicsSceneMouseEvent* e) override;
};
