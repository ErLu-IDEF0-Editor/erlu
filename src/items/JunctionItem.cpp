#include "JunctionItem.h"
#include "ArrowItem.h"

#include <QPainter>
#include <QStyleOptionGraphicsItem>
#include <cmath>

JunctionItem::JunctionItem(QGraphicsItem* parent, int id)
  : QGraphicsObject(parent),
    m_id(id)
{
  setFlags(ItemIsMovable | ItemIsSelectable | ItemSendsGeometryChanges);
  setZValue(1); // slightly above arrows
}

QRectF JunctionItem::boundingRect() const {
  return QRectF(-m_radius - 2, -m_radius - 2, (m_radius + 2) * 2, (m_radius + 2) * 2);
}

void JunctionItem::paint(QPainter* p, const QStyleOptionGraphicsItem*, QWidget*) {
  p->setRenderHint(QPainter::Antialiasing, true);
  p->setPen(Qt::NoPen);
  p->setBrush(isSelected() ? QColor(40, 100, 255) : QColor(30, 30, 30));
  p->drawEllipse(QPointF(0, 0), m_radius, m_radius);
}

void JunctionItem::addArrow(ArrowItem* a) {
  m_arrows.insert(a);
}

void JunctionItem::removeArrow(ArrowItem* a) {
  m_arrows.remove(a);
}

bool JunctionItem::hitTest(const QPointF& scenePos, qreal radius) const {
  const QPointF local = mapFromScene(scenePos);
  return std::hypot(local.x(), local.y()) <= radius;
}

QVariant JunctionItem::itemChange(GraphicsItemChange change, const QVariant& value) {
  if (change == ItemPositionHasChanged || change == ItemTransformHasChanged) {
    for (ArrowItem* a : m_arrows) {
      if (a) a->updatePath();
    }
  }
  return QGraphicsObject::itemChange(change, value);
}
