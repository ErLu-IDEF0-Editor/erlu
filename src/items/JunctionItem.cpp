#include "JunctionItem.h"
#include "ArrowItem.h"
#include "items/DiagramScene.h"

#include <QPainter>
#include <QStyleOptionGraphicsItem>
#include <cmath>

QColor JunctionItem::s_normalColor = QColor(30, 30, 30);
QColor JunctionItem::s_selectedColor = QColor(40, 100, 255);

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
  p->setBrush(isSelected() ? s_selectedColor : s_normalColor);
  p->drawEllipse(QPointF(0, 0), m_radius, m_radius);
}

void JunctionItem::setVisualTheme(const QColor& normalColor, const QColor& selectedColor) {
  s_normalColor = normalColor;
  s_selectedColor = selectedColor;
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
  if (change == ItemPositionChange) {
    const QPointF desired = value.toPointF();
    if (auto* ds = qobject_cast<DiagramScene*>(scene())) {
      QRectF bounds = ds->contentRect();
      if (bounds.isNull()) bounds = ds->sceneRect();
      QRectF rect = boundingRect().translated(desired);
      QPointF clamped = desired;
      if (rect.left() < bounds.left()) clamped.setX(desired.x() + (bounds.left() - rect.left()));
      if (rect.right() > bounds.right()) clamped.setX(clamped.x() - (rect.right() - bounds.right()));
      if (rect.top() < bounds.top()) clamped.setY(desired.y() + (bounds.top() - rect.top()));
      if (rect.bottom() > bounds.bottom()) clamped.setY(clamped.y() - (rect.bottom() - bounds.bottom()));
      return clamped;
    }
  }
  if (change == ItemPositionHasChanged || change == ItemTransformHasChanged) {
    for (ArrowItem* a : m_arrows) {
      if (a) a->updatePath();
    }
  }
  return QGraphicsObject::itemChange(change, value);
}
