#include "BlockItem.h"
#include "ArrowItem.h"

#include <algorithm>
#include <limits>
#include <cmath>
#include <QPainter>
#include <QGraphicsSceneMouseEvent>
#include <QInputDialog>

BlockItem::BlockItem(QString title, QGraphicsItem* parent, int id)
  : QGraphicsObject(parent),
    m_title(std::move(title)),
    m_rect(0, 0, 200, 100),
    m_id(id)
{
  setFlags(ItemIsMovable | ItemIsSelectable | ItemSendsGeometryChanges);
}

QRectF BlockItem::boundingRect() const {
  // небольшой запас под обводку/выделение
  return m_rect.adjusted(-2, -2, 2, 2);
}

void BlockItem::paint(QPainter* p, const QStyleOptionGraphicsItem*, QWidget*) {
  p->setRenderHint(QPainter::Antialiasing, true);

  // фон
  p->setPen(Qt::NoPen);
  p->setBrush(isSelected() ? QColor(240,240,255) : QColor(250,250,250));
  p->drawRoundedRect(m_rect, 6, 6);

  // рамка
  QPen pen(QColor(30,30,30));
  pen.setWidthF(1.5);
  p->setPen(pen);
  p->setBrush(Qt::NoBrush);
  p->drawRoundedRect(m_rect, 6, 6);

  // заголовок
  p->setPen(QColor(20,20,20));
  p->drawText(m_rect.adjusted(10, 8, -10, -8), Qt::AlignLeft | Qt::AlignTop, m_title);
}

void BlockItem::setTitle(const QString& t) {
  if (t == m_title) return;
  m_title = t;
  update();
}

QPointF BlockItem::portLocalPos(Port p) const {
  const qreal x0 = m_rect.left();
  const qreal x1 = m_rect.right();
  const qreal y0 = m_rect.top();
  const qreal y1 = m_rect.bottom();
  const qreal yc = m_rect.center().y();
  const qreal xc = m_rect.center().x();

  switch (p) {
    case Port::Input:     return QPointF(x0, yc);
    case Port::Output:    return QPointF(x1, yc);
    case Port::Control:   return QPointF(xc, y0);
    case Port::Mechanism: return QPointF(xc, y1);
  }
  return QPointF();
}

QPair<QPointF, QPointF> BlockItem::portSegment(Port p) const {
  const qreal x0 = m_rect.left();
  const qreal x1 = m_rect.right();
  const qreal y0 = m_rect.top();
  const qreal y1 = m_rect.bottom();

  switch (p) {
    case Port::Input:     return {QPointF(x0, y0), QPointF(x0, y1)};
    case Port::Output:    return {QPointF(x1, y0), QPointF(x1, y1)};
    case Port::Control:   return {QPointF(x0, y0), QPointF(x1, y0)};
    case Port::Mechanism: return {QPointF(x0, y1), QPointF(x1, y1)};
  }
  return {};
}

QPointF BlockItem::portScenePos(Port p) const {
  return mapToScene(portLocalPos(p));
}

static qreal distToSegmentLocal(const QPointF& p, const QPointF& a, const QPointF& b, QPointF* outProj = nullptr) {
  const QPointF ab = b - a;
  const qreal ab2 = ab.x()*ab.x() + ab.y()*ab.y();
  if (ab2 < 1e-6) {
    if (outProj) *outProj = a;
    return std::hypot(p.x() - a.x(), p.y() - a.y());
  }
  const qreal t = std::clamp(((p - a).x()*ab.x() + (p - a).y()*ab.y()) / ab2, 0.0, 1.0);
  const QPointF proj = a + ab * t;
  if (outProj) *outProj = proj;
  return std::hypot(p.x() - proj.x(), p.y() - proj.y());
}

bool BlockItem::hitTestPort(const QPointF& scenePos, Port* outPort, QPointF* outLocalPos, qreal radiusPx) const {
  const QPointF local = mapFromScene(scenePos);

  qreal bestDist = std::numeric_limits<qreal>::max();
  Port bestPort = Port::Input;
  QPointF bestProj;

  const Port ports[] = {Port::Input, Port::Control, Port::Output, Port::Mechanism};
  for (Port p : ports) {
    const auto seg = portSegment(p);
    QPointF proj;
    const qreal d = distToSegmentLocal(local, seg.first, seg.second, &proj);
    if (d < bestDist) {
      bestDist = d;
      bestPort = p;
      bestProj = proj;
    }
  }

  if (bestDist <= radiusPx) {
    if (outPort) *outPort = bestPort;
    if (outLocalPos) *outLocalPos = bestProj;
    return true;
  }
  return false;
}

void BlockItem::addArrow(ArrowItem* a) {
  m_arrows.insert(a);
}

void BlockItem::removeArrow(ArrowItem* a) {
  m_arrows.remove(a);
}

QVariant BlockItem::itemChange(GraphicsItemChange change, const QVariant& value) {
  if (change == ItemPositionHasChanged || change == ItemTransformHasChanged) {
    // уведомим стрелки, что нужно пересчитать геометрию
    for (auto& a : m_arrows) {
      if (a) a->updatePath();
    }
    emit geometryChanged();
  }
  return QGraphicsObject::itemChange(change, value);
}

void BlockItem::mouseDoubleClickEvent(QGraphicsSceneMouseEvent* e) {
  bool ok = false;
  const QString t = QInputDialog::getText(nullptr, "Rename function", "Title:", QLineEdit::Normal, m_title, &ok);
  if (ok) setTitle(t);
  QGraphicsObject::mouseDoubleClickEvent(e);
}
