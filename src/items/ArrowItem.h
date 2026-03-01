#pragma once
#include <QGraphicsPathItem>
#include <QPointer>
#include <QSet>
#include <optional>
#include <QString>
#include <QColor>

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
    bool tunneled = false;
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
  void setLabelHidden(bool hidden);
  bool isLabelHidden() const { return m_labelHidden; }
  void setLabelInherited(bool inherited);
  bool isLabelInherited() const { return m_labelInherited; }
  void setLabelSource(ArrowItem* src);
  ArrowItem* labelSource() const { return m_labelSource; }
  ArrowItem* labelSourceRoot() const;
  qreal bendOffset() const { return m_bendOffset; }
  qreal topOffset() const { return m_topOffset; }
  qreal bottomOffset() const { return m_bottomOffset; }
  int type() const override { return Type; }
  int internalId() const { return m_internalId; }
  bool isInterface() const { return m_isInterface; }
  bool isInterfaceStub() const { return m_isInterface && m_interfaceStubOnly; }
  bool isLabelLocked() const { return m_labelLocked; }
  std::optional<Endpoint> interfaceEdge() const { return m_interfaceEdge; }
  void setInterfaceStub(const Endpoint& edge, const QString& label);
  void setInterfaceIsStub(bool stub);
  void resetInterfaceStub();
  void setLabelLocked(bool locked);
  static void setVisualTheme(const QColor& lineColor, const QColor& textColor);
  void setCustomColor(const QColor& color);
  void clearCustomColor();
  std::optional<QColor> customColor() const { return m_customColor; }
  void setCallMechanism(bool v) { m_isCallMechanism = v; }
  bool isCallMechanism() const { return m_isCallMechanism; }
  void setCallRefId(int id) { m_callRefId = id; }
  int callRefId() const { return m_callRefId; }
  void setFromTunneled(bool v);
  void setToTunneled(bool v);
  bool isFromTunneled() const { return m_from.tunneled; }
  bool isToTunneled() const { return m_to.tunneled; }

  void updatePath();
  std::optional<QPointF> hitTest(const QPointF& scenePos, qreal radius) const;

private:
  enum class DragPart { None, BendX, TopHorizontal, BottomHorizontal, FromEnd, ToEnd };

  Endpoint m_from;
  Endpoint m_to;
  bool m_hasTempEnd = false;
  QPointF m_tempEndScene;
  qreal m_bendOffset = 0.0;
  qreal m_topOffset = 0.0;
  qreal m_bottomOffset = 0.0;
  QString m_label;
  QGraphicsSimpleTextItem* m_labelItem = nullptr;
  bool m_labelHidden = false;
  bool m_labelInherited = false;
  QPointF m_labelOffset; // in scene-space relative to anchor point
  bool m_labelDragging = false;
  QPointF m_labelDragLastScene;
  QVector<QPointF> m_lastPolyline;
  bool m_isInterface = false;
  bool m_interfaceStubOnly = false;
  bool m_labelLocked = false;
  std::optional<Endpoint> m_interfaceEdge;
  ArrowItem* m_labelSource = nullptr;
  static QColor s_lineColor;
  static QColor s_textColor;
  std::optional<QColor> m_customColor;
  int m_internalId = -1;
  bool m_isCallMechanism = false;
  int m_callRefId = -1;

  DragPart m_dragPart = DragPart::None;
  QPointF m_lastDragScenePos;

  void drawArrowHead(QPainterPath& path, const QPointF& tip, const QPointF& from);
  void drawTunnelBrackets(QPainterPath& path, const QPointF& tip, const QPointF& from);
  QVector<QPointF> computePolyline() const;
  static QPointF portDirection(const Endpoint& ep, bool isFrom, const QPointF& fallbackDir, const QGraphicsScene* scene);
  static QVector<QPointF> simplifyPolyline(const QVector<QPointF>& pts);
  static qreal distToSegment(const QPointF& p, const QPointF& a, const QPointF& b);
  void updateLabelItem(const QVector<QPointF>& pts);

protected:
  QRectF boundingRect() const override;
  QPainterPath shape() const override;
  void hoverMoveEvent(QGraphicsSceneHoverEvent* e) override;
  void mousePressEvent(QGraphicsSceneMouseEvent* e) override;
  void mouseMoveEvent(QGraphicsSceneMouseEvent* e) override;
  void mouseReleaseEvent(QGraphicsSceneMouseEvent* e) override;
  void mouseDoubleClickEvent(QGraphicsSceneMouseEvent* e) override;
  bool adjustEndpoint(Endpoint& ep, const QPointF& delta);
};
