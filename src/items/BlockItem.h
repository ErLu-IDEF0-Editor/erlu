#pragma once
#include <QGraphicsObject>
#include <QSet>

class ArrowItem;

class BlockItem final : public QGraphicsObject {
  Q_OBJECT
public:
  enum class Port { Input, Control, Output, Mechanism };
  enum { Type = UserType + 1 };

  explicit BlockItem(QString title = "Function", QGraphicsItem* parent = nullptr, int id = -1);

  QRectF boundingRect() const override;
  void paint(QPainter* p, const QStyleOptionGraphicsItem*, QWidget*) override;
  int type() const override { return Type; }

  QString title() const { return m_title; }
  void setTitle(const QString& t);
  int id() const { return m_id; }
  void setId(int id) { m_id = id; }

  QPointF portScenePos(Port p) const;
  QPair<QPointF, QPointF> portSegment(Port p) const;
  bool hitTestPort(const QPointF& scenePos, Port* outPort, QPointF* outLocalPos = nullptr, qreal radiusPx = 10.0) const;
  QPointF portLocalPos(Port p) const;

  void addArrow(ArrowItem* a);
  void removeArrow(ArrowItem* a);

signals:
  void geometryChanged();

protected:
  QVariant itemChange(GraphicsItemChange change, const QVariant& value) override;
  void mouseDoubleClickEvent(QGraphicsSceneMouseEvent* e) override;

private:
  QString m_title;
  QRectF m_rect;          // local rect
  int m_id;
  QSet<ArrowItem*> m_arrows;
};
