#include "ArrowItem.h"
#include "BlockItem.h"
#include "JunctionItem.h"

#include <QPen>
#include <QtMath>
#include <QGraphicsScene>
#include <QGraphicsSceneMouseEvent>
#include <QGraphicsSimpleTextItem>
#include <QCursor>
#include <QInputDialog>
#include <QLineEdit>
#include <QDebug>
#include <QObject>
#include "DiagramScene.h"
#include <algorithm>
#include <QSet>
#include <queue>
#include <vector>
#include <limits>
#include <utility>

static bool almostEqual(qreal a, qreal b, qreal eps = 1e-6);
static bool almostEqual(const QPointF& a, const QPointF& b, qreal eps = 1e-6);
static QPointF scenePortPos(const ArrowItem::Endpoint& ep);

static QRectF labelDragHitRect(const QGraphicsSimpleTextItem* item) {
  if (!item || !item->isVisible()) return QRectF();
  // Generous hit area makes label dragging much less fiddly.
  return item->sceneBoundingRect().adjusted(-16, -10, 16, 10);
}
QColor ArrowItem::s_lineColor = QColor(10, 10, 10);
QColor ArrowItem::s_textColor = QColor(10, 10, 10);

ArrowItem::ArrowItem(QGraphicsItem* parent)
  : QGraphicsPathItem(parent)
{
  QPen pen(s_lineColor);
  pen.setWidthF(1.4);
  pen.setCapStyle(Qt::RoundCap);
  pen.setJoinStyle(Qt::RoundJoin);
  setPen(pen);
  setZValue(-1); // стрелки под блоками

  setAcceptHoverEvents(true);
  setAcceptedMouseButtons(Qt::LeftButton);
  setFlag(QGraphicsItem::ItemIsSelectable, true);

  m_labelItem = new QGraphicsSimpleTextItem(this);
  m_labelItem->setBrush(s_textColor);
  m_labelItem->setZValue(2);
  m_labelItem->setFlag(QGraphicsItem::ItemIgnoresTransformations, true);
  m_labelItem->setVisible(false);
  m_labelOffset = QPointF(0, 0);
}

void ArrowItem::setVisualTheme(const QColor& lineColor, const QColor& textColor) {
  s_lineColor = lineColor;
  s_textColor = textColor;
}

ArrowItem::~ArrowItem() {
  if (m_from.block) m_from.block->removeArrow(this);
  if (m_to.block) m_to.block->removeArrow(this);
  if (m_from.junction) m_from.junction->removeArrow(this);
  if (m_to.junction) m_to.junction->removeArrow(this);
}

void ArrowItem::setLabel(const QString& text) {
  if (text == m_label) return;
  m_label = text;
  if (m_labelItem) {
    m_labelItem->setText(m_label);
    m_labelItem->setVisible(!m_label.isEmpty() && !m_labelHidden);
  }
  updateLabelItem(computePolyline());
}

void ArrowItem::setOffsets(qreal bend, qreal top, qreal bottom) {
  m_bendOffset = bend;
  m_topOffset = top;
  m_bottomOffset = bottom;
  updatePath();
}

void ArrowItem::setLabelOffset(const QPointF& off) {
  m_labelOffset = off;
  updateLabelItem(m_lastPolyline.isEmpty() ? computePolyline() : m_lastPolyline);
}

void ArrowItem::setLabelHidden(bool hidden) {
  m_labelHidden = hidden;
  updateLabelItem(m_lastPolyline.isEmpty() ? computePolyline() : m_lastPolyline);
}

void ArrowItem::setLabelInherited(bool inherited) {
  m_labelInherited = inherited;
}

void ArrowItem::setLabelSource(ArrowItem* src) {
  m_labelSource = src;
}

ArrowItem* ArrowItem::labelSourceRoot() const {
  const ArrowItem* cur = this;
  QSet<const ArrowItem*> visited;
  while (cur) {
    if (visited.contains(cur)) break;
    visited.insert(cur);
    if (!cur->m_labelSource) return const_cast<ArrowItem*>(cur);
    cur = cur->m_labelSource;
  }
  return const_cast<ArrowItem*>(cur);
}

void ArrowItem::setInterfaceStub(const Endpoint& edge, const QString& label) {
  m_isInterface = true;
  m_interfaceStubOnly = true;
  m_labelLocked = true;
  m_labelHidden = false;
  m_interfaceEdge = edge;
  setLabel(label);

  const QPointF posScene = scenePortPos(edge);
  Endpoint stub = edge;
  stub.scenePos = posScene;
  stub.block = nullptr;
  stub.junction = nullptr;
  stub.localPos.reset();

  setFrom(stub);
  setTo(stub);
  finalize();
}

void ArrowItem::resetInterfaceStub() {
  if (!m_isInterface || !m_interfaceEdge) return;
  setInterfaceStub(*m_interfaceEdge, m_label);
}

void ArrowItem::setInterfaceIsStub(bool stub) {
  m_interfaceStubOnly = stub;
  updatePath();
}

void ArrowItem::setLabelLocked(bool locked) {
  m_labelLocked = locked;
}

bool ArrowItem::adjustEndpoint(Endpoint& ep, const QPointF& delta) {
  if (ep.block) {
    auto seg = ep.block->portSegment(ep.port);
    QPointF local = ep.localPos.value_or(ep.block->portLocalPos(ep.port));
    local += ep.block->mapFromScene(delta);
    if (qFuzzyCompare(seg.first.x(), seg.second.x())) {
      local.setX(seg.first.x());
      local.setY(std::clamp(local.y(), seg.first.y(), seg.second.y()));
    } else {
      local.setY(seg.first.y());
      local.setX(std::clamp(local.x(), seg.first.x(), seg.second.x()));
    }
    ep.localPos = local;
    return true;
  }
  if (ep.scenePos) {
    QPointF p = *ep.scenePos + delta;
    if (scene()) {
      QRectF r = scene()->sceneRect();
      if (auto* ds = qobject_cast<DiagramScene*>(scene())) {
        const QRectF cr = ds->contentRect();
        if (!cr.isNull()) r = cr;
      }
      // snap to nearest edge it was originally on
      const qreal tol = 4.0;
      if (std::abs(ep.scenePos->x() - r.left()) < tol) {
        p.setX(r.left());
        p.setY(std::clamp(p.y(), r.top(), r.bottom()));
      } else if (std::abs(ep.scenePos->x() - r.right()) < tol) {
        p.setX(r.right());
        p.setY(std::clamp(p.y(), r.top(), r.bottom()));
      } else if (std::abs(ep.scenePos->y() - r.top()) < tol) {
        p.setY(r.top());
        p.setX(std::clamp(p.x(), r.left(), r.right()));
      } else if (std::abs(ep.scenePos->y() - r.bottom()) < tol) {
        p.setY(r.bottom());
        p.setX(std::clamp(p.x(), r.left(), r.right()));
      }
    }
    ep.scenePos = p;
    return true;
  }
  return false;
}

void ArrowItem::setFrom(const Endpoint& ep) {
  if (m_from.block) m_from.block->removeArrow(this);
  if (m_from.junction) m_from.junction->removeArrow(this);
  m_from = ep;
  if (m_from.block) m_from.block->addArrow(this);
  if (m_from.junction) m_from.junction->addArrow(this);
  updatePath();
}

void ArrowItem::setTo(const Endpoint& ep) {
  if (m_to.block) m_to.block->removeArrow(this);
  if (m_to.junction) m_to.junction->removeArrow(this);
  m_to = ep;
  if (m_to.block) m_to.block->addArrow(this);
  if (m_to.junction) m_to.junction->addArrow(this);
  m_hasTempEnd = false;
  updatePath();
}

void ArrowItem::setTempEndPoint(const QPointF& scenePos) {
  m_hasTempEnd = true;
  m_tempEndScene = scenePos;
  updatePath();
}

void ArrowItem::finalize() {
  m_hasTempEnd = false;
  updatePath();
}

static QPointF scenePortPos(const ArrowItem::Endpoint& ep) {
  if (ep.junction) {
    if (ep.localPos) return ep.junction->mapToScene(*ep.localPos);
    return ep.junction->scenePos();
  }
  if (ep.scenePos) return *ep.scenePos;
  if (ep.block) {
    if (ep.localPos) return ep.block->mapToScene(*ep.localPos);
    return ep.block->portScenePos(ep.port);
  }
  return QPointF();
}

qreal ArrowItem::distToSegment(const QPointF& p, const QPointF& a, const QPointF& b) {
  const QPointF ab = b - a;
  const qreal ab2 = ab.x()*ab.x() + ab.y()*ab.y();
  if (ab2 < 1e-6) return std::hypot(p.x() - a.x(), p.y() - a.y());
  const qreal t = std::clamp(((p - a).x()*ab.x() + (p - a).y()*ab.y()) / ab2, 0.0, 1.0);
  const QPointF proj = a + ab * t;
  return std::hypot(p.x() - proj.x(), p.y() - proj.y());
}

static QPointF normalizedOr(const QPointF& v, const QPointF& fallback) {
  qreal len = std::hypot(v.x(), v.y());
  if (len < 1e-6) return fallback;
  return v / len;
}

static bool almostEqual(qreal a, qreal b, qreal eps) {
  return std::abs(a - b) < eps;
}

static bool almostEqual(const QPointF& a, const QPointF& b, qreal eps) {
  return almostEqual(a.x(), b.x(), eps) && almostEqual(a.y(), b.y(), eps);
}

static bool pointInside(const QPointF& p, const QRectF& r) {
  return p.x() > r.left() && p.x() < r.right() && p.y() > r.top() && p.y() < r.bottom();
}

static bool segmentBlocked(const QPointF& a, const QPointF& b, const QVector<QRectF>& obstacles) {
  const bool vertical = almostEqual(a.x(), b.x());
  const bool horizontal = almostEqual(a.y(), b.y());
  if (!vertical && !horizontal) {
    return true; // Manhattan only
  }

  for (const auto& r : obstacles) {
    if (vertical) {
      const qreal x = a.x();
      if (x > r.left() && x < r.right()) {
        const qreal lo = std::min(a.y(), b.y());
        const qreal hi = std::max(a.y(), b.y());
        if (hi > r.top() && lo < r.bottom()) return true;
      }
    } else if (horizontal) {
      const qreal y = a.y();
      if (y > r.top() && y < r.bottom()) {
        const qreal lo = std::min(a.x(), b.x());
        const qreal hi = std::max(a.x(), b.x());
        if (hi > r.left() && lo < r.right()) return true;
      }
    }
  }
  return false;
}

static QVector<QPointF> manhattanRoute(const QPointF& start,
                                       const QPointF& end,
                                       const QVector<QRectF>& obstacles,
                                       const QRectF& sceneRect,
                                       const QVector<qreal>& extraXs,
                                       const QVector<qreal>& extraYs) {
  QVector<qreal> xs = extraXs;
  QVector<qreal> ys = extraYs;
  xs << start.x() << end.x();
  ys << start.y() << end.y();

  for (const auto& r : obstacles) {
    xs << r.left() << r.right();
    ys << r.top() << r.bottom();
  }
  if (!sceneRect.isNull()) {
    xs << sceneRect.left() << sceneRect.right();
    ys << sceneRect.top() << sceneRect.bottom();
  }

  auto dedupSort = [](QVector<qreal>& v) {
    std::sort(v.begin(), v.end());
    v.erase(std::unique(v.begin(), v.end(), [](qreal lhs, qreal rhs) {
      return almostEqual(lhs, rhs);
    }), v.end());
  };
  dedupSort(xs);
  dedupSort(ys);

  const int nx = xs.size();
  const int ny = ys.size();
  if (nx == 0 || ny == 0) return {};

  struct Node {
    QPointF pos;
    QVector<int> neighbors;
  };

  QVector<Node> nodes;
  nodes.reserve(nx * ny);
  QVector<int> grid(nx * ny, -1);

  auto gridIndex = [nx](int xi, int yi) { return yi * nx + xi; };

  auto registerNode = [&](int xi, int yi) -> int {
    const QPointF p(xs[xi], ys[yi]);
    for (const auto& r : obstacles) {
      if (pointInside(p, r)) return -1;
    }
    const int idx = nodes.size();
    nodes.push_back(Node{p, {}});
    grid[gridIndex(xi, yi)] = idx;
    return idx;
  };

  int startIdx = -1;
  int endIdx = -1;

  for (int yi = 0; yi < ny; ++yi) {
    for (int xi = 0; xi < nx; ++xi) {
      const int idx = registerNode(xi, yi);
      if (idx < 0) continue;
      const QPointF p = nodes[idx].pos;
      if (startIdx < 0 && almostEqual(p, start)) startIdx = idx;
      if (endIdx < 0 && almostEqual(p, end)) endIdx = idx;
    }
  }

  if (startIdx < 0 || endIdx < 0) return {};

  // Horizontal connections
  for (int yi = 0; yi < ny; ++yi) {
    int lastIdx = -1;
    QPointF lastPt;
    for (int xi = 0; xi < nx; ++xi) {
      const int idx = grid[gridIndex(xi, yi)];
      if (idx < 0) continue;
      const QPointF pt(xs[xi], ys[yi]);
      if (lastIdx >= 0) {
        if (!segmentBlocked(lastPt, pt, obstacles)) {
          nodes[lastIdx].neighbors.push_back(idx);
          nodes[idx].neighbors.push_back(lastIdx);
        } else {
          lastIdx = idx;
          lastPt = pt;
          continue;
        }
      }
      lastIdx = idx;
      lastPt = pt;
    }
  }

  // Vertical connections
  for (int xi = 0; xi < nx; ++xi) {
    int lastIdx = -1;
    QPointF lastPt;
    for (int yi = 0; yi < ny; ++yi) {
      const int idx = grid[gridIndex(xi, yi)];
      if (idx < 0) continue;
      const QPointF pt(xs[xi], ys[yi]);
      if (lastIdx >= 0) {
        if (!segmentBlocked(lastPt, pt, obstacles)) {
          nodes[lastIdx].neighbors.push_back(idx);
          nodes[idx].neighbors.push_back(lastIdx);
        } else {
          lastIdx = idx;
          lastPt = pt;
          continue;
        }
      }
      lastIdx = idx;
      lastPt = pt;
    }
  }

  const int n = nodes.size();
  QVector<qreal> gScore(n, std::numeric_limits<qreal>::infinity());
  QVector<int> cameFrom(n, -1);
  QVector<bool> closed(n, false);

  auto manhattan = [](const QPointF& p0, const QPointF& p1) {
    return std::abs(p0.x() - p1.x()) + std::abs(p0.y() - p1.y());
  };

  struct QueueItem {
    int idx;
    qreal f;
  };
  auto cmp = [](const QueueItem& lhs, const QueueItem& rhs) { return lhs.f > rhs.f; };
  std::priority_queue<QueueItem, std::vector<QueueItem>, decltype(cmp)> open(cmp);

  gScore[startIdx] = 0.0;
  open.push({startIdx, manhattan(nodes[startIdx].pos, nodes[endIdx].pos)});

  while (!open.empty()) {
    const int current = open.top().idx;
    open.pop();
    if (closed[current]) continue;
    closed[current] = true;
    if (current == endIdx) break;

    for (int nb : nodes[current].neighbors) {
      if (closed[nb]) continue;
      const qreal tentative = gScore[current] + manhattan(nodes[current].pos, nodes[nb].pos);
      if (tentative + 1e-6 < gScore[nb]) {
        cameFrom[nb] = current;
        gScore[nb] = tentative;
        const qreal fScore = tentative + manhattan(nodes[nb].pos, nodes[endIdx].pos);
        open.push({nb, fScore});
      }
    }
  }

  QVector<QPointF> path;
  if (cameFrom[endIdx] == -1 && startIdx != endIdx) {
    return path; // unreachable
  }

  int cur = endIdx;
  path.push_front(nodes[cur].pos);
  while (cur != startIdx) {
    cur = cameFrom[cur];
    if (cur < 0) break;
    path.push_front(nodes[cur].pos);
  }
  return path;
}

QPointF ArrowItem::portDirection(const Endpoint& ep, bool isFrom, const QPointF& fallbackDir, const QGraphicsScene* scene) {
  if (ep.junction) return QPointF(); // junction is isotropic
  if (ep.scenePos) {
    const QRectF r = scene ? scene->sceneRect() : QRectF();
    const QPointF p = *ep.scenePos;
    const qreal dl = std::abs(p.x() - r.left());
    const qreal dr = std::abs(p.x() - r.right());
    const qreal dt = std::abs(p.y() - r.top());
    const qreal db = std::abs(p.y() - r.bottom());
    const qreal m = std::min({dl, dr, dt, db});
    if (m == dl) return isFrom ? QPointF(1,0) : QPointF(-1,0);
    if (m == dr) return isFrom ? QPointF(-1,0) : QPointF(1,0); // right edge is target-only
    if (m == dt) return isFrom ? QPointF(0,1) : QPointF(0,-1);
    if (m == db) return isFrom ? QPointF(0,-1) : QPointF(0,1);
  }
  switch (ep.port) {
    case BlockItem::Port::Input: return QPointF(-1, 0);
    case BlockItem::Port::Output: return QPointF(1, 0);
    case BlockItem::Port::Control: return QPointF(0, -1);
    case BlockItem::Port::Mechanism: return QPointF(0, 1);
  }
  return fallbackDir;
}

QVector<QPointF> ArrowItem::simplifyPolyline(const QVector<QPointF>& ptsIn) {
  QVector<QPointF> pts = ptsIn;
  // убрать дубли
  pts.erase(std::unique(pts.begin(), pts.end(), [](const QPointF& lhs, const QPointF& rhs){
    return qFuzzyCompare(lhs.x(), rhs.x()) && qFuzzyCompare(lhs.y(), rhs.y());
  }), pts.end());

  auto isCollinear = [](const QPointF& a, const QPointF& b, const QPointF& c) {
    const QPointF ab = b - a;
    const QPointF bc = c - b;
    const qreal cross = ab.x()*bc.y() - ab.y()*bc.x();
    if (std::abs(cross) > 1e-6) return false;
    return (ab.x()*bc.x() + ab.y()*bc.y()) >= 0;
  };

  for (int i = 0; i + 2 < pts.size();) {
    if (isCollinear(pts[i], pts[i+1], pts[i+2])) {
      pts.removeAt(i+1);
    } else {
      ++i;
    }
  }
  return pts;
}

QVector<QPointF> ArrowItem::computePolyline() const {
  if (m_isInterface && m_interfaceStubOnly && m_interfaceEdge) {
    const QPointF p = scenePortPos(*m_interfaceEdge);
    return {p, p};
  }

  const qreal gap = 24.0;
  QPointF a = scenePortPos(m_from);
  QPointF b = m_hasTempEnd ? m_tempEndScene : scenePortPos(m_to);

  // Направления
  QPointF dirA = portDirection(m_from, true, QPointF(1,0), scene());
  QPointF dirB = m_hasTempEnd
                   ? normalizedOr((std::abs((a - b).x()) >= std::abs((a - b).y()))
                                    ? QPointF(((a - b).x() >= 0 ? 1 : -1), 0)
                                    : QPointF(0, ((a - b).y() >= 0 ? 1 : -1)),
                                  QPointF(-1,0))
                   : portDirection(m_to, false, QPointF(-1,0), scene());
  // ensure right border always behaves like an input (approach from left)
  if (!m_hasTempEnd && m_to.scenePos && scene()) {
    const qreal rightX = scene()->sceneRect().right();
    if (almostEqual(m_to.scenePos->x(), rightX)) {
      dirB = QPointF(-1, 0);
    }
  }

  auto expandRect = [](const QRectF& r, qreal m) {
    return QRectF(r.x()-m, r.y()-m, r.width()+2*m, r.height()+2*m);
  };

  const QPointF startBase = a + dirA * gap;
  const QPointF endBase = b + dirB * gap;
  const bool startHorizontal = std::abs(dirA.x()) >= std::abs(dirA.y());
  const bool endHorizontal = std::abs(dirB.x()) >= std::abs(dirB.y());
  const QPointF start = startBase + (startHorizontal ? QPointF(0, m_topOffset) : QPointF());
  const QPointF end = endBase + (endHorizontal ? QPointF(0, m_bottomOffset) : QPointF());

  QVector<QRectF> obstacles;
  if (scene()) {
    const auto itemsInScene = scene()->items();
    for (QGraphicsItem* it : itemsInScene) {
      if (auto* blk = qgraphicsitem_cast<BlockItem*>(it)) {
        obstacles << expandRect(blk->mapRectToScene(blk->boundingRect()), gap);
      }
    }
  }

  QVector<qreal> xHints;
  QVector<qreal> yHints;
  xHints << startBase.x() << endBase.x() << a.x() << b.x();
  yHints << startBase.y() << endBase.y() << a.y() << b.y();
  if (!almostEqual(m_bendOffset, 0.0)) {
    xHints << start.x() + m_bendOffset << end.x() + m_bendOffset;
    yHints << start.y() + m_bendOffset << end.y() + m_bendOffset;
  }

  const QRectF sceneBounds = scene() ? scene()->sceneRect() : QRectF();
  QVector<QPointF> spine = manhattanRoute(start, end, obstacles, sceneBounds, xHints, yHints);
  if (spine.isEmpty()) {
    spine << start;
    if (!almostEqual(start.x(), end.x()) && !almostEqual(start.y(), end.y())) {
      spine << QPointF(end.x(), start.y());
    }
    spine << end;
  }

  QVector<QPointF> pts;
  pts << a;
  if (!almostEqual(a, startBase)) pts << startBase;
  for (const QPointF& p : spine) {
    if (pts.isEmpty() || !almostEqual(pts.last(), p)) pts << p;
  }
  if (!almostEqual(end, endBase)) pts << endBase;
  pts << b;

  return simplifyPolyline(pts);
}

std::optional<QPointF> ArrowItem::hitTest(const QPointF& scenePos, qreal radius) const {
  const QVector<QPointF> pts = computePolyline();
  std::optional<QPointF> bestPoint;
  qreal bestDist = radius;

  for (int i = 0; i + 1 < pts.size(); ++i) {
    const QPointF a = pts[i];
    const QPointF b = pts[i+1];
    const bool vertical = almostEqual(a.x(), b.x());
    const bool horizontal = almostEqual(a.y(), b.y());
    if (!vertical && !horizontal) continue;

    const qreal d = distToSegment(scenePos, a, b);
    if (d > bestDist + 1e-6) continue;

    QPointF proj;
    if (vertical) {
      const qreal y = std::clamp(scenePos.y(), std::min(a.y(), b.y()), std::max(a.y(), b.y()));
      proj = QPointF(a.x(), y);
    } else {
      const qreal x = std::clamp(scenePos.x(), std::min(a.x(), b.x()), std::max(a.x(), b.x()));
      proj = QPointF(x, a.y());
    }

    const qreal toA = std::hypot(proj.x() - a.x(), proj.y() - a.y());
    const qreal toB = std::hypot(proj.x() - b.x(), proj.y() - b.y());
    if (toA < 2.0 || toB < 2.0) continue; // skip near endpoints

    bestDist = d;
    bestPoint = proj;
  }

  return bestPoint;
}

void ArrowItem::updateLabelItem(const QVector<QPointF>& pts) {
  if (!m_labelItem) return;
  const bool stubOnly = m_isInterface && m_interfaceStubOnly && m_interfaceEdge;
  if (stubOnly) {
    if (m_label.isEmpty() || m_labelHidden) {
      m_labelItem->setVisible(false);
      return;
    }
    const QPointF p = scenePortPos(*m_interfaceEdge);
    const QRectF br = m_labelItem->boundingRect();
    m_labelItem->setPos(p + m_labelOffset + QPointF(10, -br.height() * 0.5));
    m_labelItem->setVisible(true);
    return;
  }
  if (m_labelHidden || m_label.isEmpty() || pts.size() < 2) {
    m_labelItem->setVisible(false);
    return;
  }

  qreal totalLen = 0.0;
  for (int i = 0; i + 1 < pts.size(); ++i) {
    totalLen += std::abs(pts[i+1].x() - pts[i].x()) + std::abs(pts[i+1].y() - pts[i].y());
  }
  if (totalLen < 1e-3) {
    m_labelItem->setVisible(false);
    return;
  }

  const qreal target = totalLen * 0.5;
  qreal acc = 0.0;
  QPointF pos = pts.first();
  for (int i = 0; i + 1 < pts.size(); ++i) {
    const QPointF a = pts[i];
    const QPointF b = pts[i+1];
    const qreal segLen = std::abs(b.x() - a.x()) + std::abs(b.y() - a.y());
    if (segLen < 1e-6) continue;

    if (acc + segLen >= target) {
      const qreal t = (target - acc) / segLen;
      pos = a + (b - a) * t;
      break;
    }
    acc += segLen;
    pos = b;
  }

  const QRectF br = m_labelItem->boundingRect();
  m_labelItem->setPos(pos + m_labelOffset - QPointF(br.width() * 0.5, br.height() * 0.5));
  m_labelItem->setVisible(true);
}

// Обновляем маршрут ортогонально, добавляя/схлопывая сегменты при необходимости.
void ArrowItem::updatePath() {
  QPen themedPen = pen();
  themedPen.setColor(s_lineColor);
  setPen(themedPen);
  if (m_labelItem) {
    m_labelItem->setBrush(s_textColor);
  }

  const QRectF oldSceneRect = mapRectToScene(boundingRect());

  QVector<QPointF> pts = computePolyline();
  m_lastPolyline = pts;

  QPainterPath p;
  const bool stubOnly = m_isInterface && m_interfaceStubOnly;
  if (!pts.isEmpty() && !stubOnly) {
    p.moveTo(pts.first());
    for (int i = 1; i < pts.size(); ++i) {
      p.lineTo(pts[i]);
    }
  }

  // interface stub circle at edge
  if (m_isInterface && m_interfaceEdge && m_interfaceStubOnly) {
    const QPointF stubPos = scenePortPos(*m_interfaceEdge);
    const qreal r = 6.0;
    p.addEllipse(stubPos, r, r);
  }

  // наконечник стрелки
  // Draw arrow head only if the path truly ends (not branching through a junction).
  if (!stubOnly && pts.size() >= 2 && !m_to.junction) {
    const QPointF tip = pts.last();
    const QPointF from = pts[pts.size() - 2];
    drawArrowHead(p, tip, from);
  }

  // Обновляем геометрию, чтобы сцена инвалидацировала старую и новую области.
  prepareGeometryChange();
  setPath(p);
  updateLabelItem(pts);

  // Явно инвалидируем область (фон + элементы), чтобы очистить следы при drag.
  if (scene()) {
    const QRectF newSceneRect = mapRectToScene(boundingRect());
    scene()->invalidate(oldSceneRect.united(newSceneRect),
                        QGraphicsScene::BackgroundLayer | QGraphicsScene::ItemLayer);
  }
}

void ArrowItem::hoverMoveEvent(QGraphicsSceneHoverEvent* e) {
  // Keep label geometry in sync before hit-testing drag area.
  updateLabelItem(m_lastPolyline.isEmpty() ? computePolyline() : m_lastPolyline);

  const qreal tol = 6.0;
  const QVector<QPointF> pts = computePolyline();
  QPointF pos = e->scenePos();

  if (!m_labelLocked && m_labelItem && labelDragHitRect(m_labelItem).contains(pos)) {
    setCursor(Qt::SizeAllCursor);
    QGraphicsPathItem::hoverMoveEvent(e);
    return;
  }

  Qt::CursorShape cursor = Qt::ArrowCursor;
  for (int i = 0; i + 1 < pts.size(); ++i) {
    const QPointF a = pts[i];
    const QPointF b = pts[i+1];
    if (std::abs(a.x() - b.x()) < 1e-6) {
      if (distToSegment(pos, a, b) <= tol) { cursor = Qt::SizeHorCursor; break; }
    } else if (std::abs(a.y() - b.y()) < 1e-6) {
      if (distToSegment(pos, a, b) <= tol) { cursor = Qt::SizeVerCursor; break; }
    }
  }
  setCursor(cursor);
  QGraphicsPathItem::hoverMoveEvent(e);
}

void ArrowItem::mousePressEvent(QGraphicsSceneMouseEvent* e) {
  if (e->button() != Qt::LeftButton) {
    QGraphicsPathItem::mousePressEvent(e);
    return;
  }

  // Keep label geometry in sync before hit-testing drag area.
  updateLabelItem(m_lastPolyline.isEmpty() ? computePolyline() : m_lastPolyline);

  if (!m_labelLocked && m_labelItem && labelDragHitRect(m_labelItem).contains(e->scenePos())) {
    m_labelDragging = true;
    m_labelDragLastScene = e->scenePos();
    setCursor(Qt::SizeAllCursor);
    e->accept();
    return;
  }

  const qreal tol = 6.0;
  const QVector<QPointF> pts = computePolyline();
  const QPointF pos = e->scenePos();

  // Shift+drag endpoint: slide connection along the same side.
  const bool shift = e->modifiers().testFlag(Qt::ShiftModifier);
  if (!pts.isEmpty()) {
    if (shift && QLineF(pos, pts.first()).length() <= tol) {
      m_dragPart = DragPart::FromEnd;
      m_lastDragScenePos = pos;
      e->accept();
      return;
    }
    if (shift && QLineF(pos, pts.last()).length() <= tol) {
      m_dragPart = DragPart::ToEnd;
      m_lastDragScenePos = pos;
      e->accept();
      return;
    }
  }

  m_dragPart = DragPart::None;
  int closestIdx = -1;
  qreal best = tol;
  for (int i = 0; i + 1 < pts.size(); ++i) {
    const QPointF a = pts[i], b = pts[i+1];
    if (std::abs(a.x() - b.x()) < 1e-6 || std::abs(a.y() - b.y()) < 1e-6) {
      qreal d = distToSegment(pos, a, b);
      if (d <= best) {
        best = d;
        closestIdx = i;
      }
    }
  }

  if (closestIdx >= 0) {
    const QPointF a = pts[closestIdx], b = pts[closestIdx+1];
    if (std::abs(a.x() - b.x()) < 1e-6) {
      m_dragPart = DragPart::BendX; // вертикальный сегмент
    } else {
      // горизонтальный — решаем, к какому концу ближе
      m_dragPart = (closestIdx <= (pts.size() / 2)) ? DragPart::TopHorizontal : DragPart::BottomHorizontal;
    }
  }

  if (m_dragPart != DragPart::None) {
    m_lastDragScenePos = e->scenePos();
    e->accept();
    return;
  }

  QGraphicsPathItem::mousePressEvent(e);
}

void ArrowItem::mouseMoveEvent(QGraphicsSceneMouseEvent* e) {
  if (m_labelDragging) {
    const QPointF delta = e->scenePos() - m_labelDragLastScene;
    m_labelDragLastScene = e->scenePos();
    m_labelOffset += delta;
    updateLabelItem(m_lastPolyline.isEmpty() ? computePolyline() : m_lastPolyline);
    e->accept();
    return;
  }

  if (m_dragPart == DragPart::FromEnd) {
    const QPointF delta = e->scenePos() - m_lastDragScenePos;
    m_lastDragScenePos = e->scenePos();
    if (adjustEndpoint(m_from, delta)) {
      updatePath();
    }
    e->accept();
    return;
  }
  if (m_dragPart == DragPart::ToEnd) {
    const QPointF delta = e->scenePos() - m_lastDragScenePos;
    m_lastDragScenePos = e->scenePos();
    if (adjustEndpoint(m_to, delta)) {
      updatePath();
    }
    e->accept();
    return;
  }

  if (m_dragPart == DragPart::None) {
    QGraphicsPathItem::mouseMoveEvent(e);
    return;
  }

  const QPointF delta = e->scenePos() - m_lastDragScenePos;
  m_lastDragScenePos = e->scenePos();

  switch (m_dragPart) {
    case DragPart::BendX:
      m_bendOffset += delta.x();
      break;
    case DragPart::TopHorizontal:
      m_topOffset += delta.y();
      break;
    case DragPart::BottomHorizontal:
      m_bottomOffset += delta.y();
      break;
    case DragPart::FromEnd:
    case DragPart::ToEnd:
    case DragPart::None:
      break;
  }

  updatePath();
  e->accept();
}

void ArrowItem::mouseReleaseEvent(QGraphicsSceneMouseEvent* e) {
  if (m_labelDragging && e->button() == Qt::LeftButton) {
    m_labelDragging = false;
    unsetCursor();
    if (auto* ds = qobject_cast<DiagramScene*>(scene())) {
      ds->requestSnapshot();
    }
    e->accept();
    return;
  }
  if ((m_dragPart == DragPart::FromEnd || m_dragPart == DragPart::ToEnd) && e->button() == Qt::LeftButton) {
    m_dragPart = DragPart::None;
    unsetCursor();
    if (auto* ds = qobject_cast<DiagramScene*>(scene())) {
      ds->requestSnapshot();
    }
    e->accept();
    return;
  }
  if (m_dragPart != DragPart::None && e->button() == Qt::LeftButton) {
    m_dragPart = DragPart::None;
    unsetCursor();
    if (auto* ds = qobject_cast<DiagramScene*>(scene())) {
      ds->requestSnapshot();
    }
    e->accept();
    return;
  }
  QGraphicsPathItem::mouseReleaseEvent(e);
}

void ArrowItem::mouseDoubleClickEvent(QGraphicsSceneMouseEvent* e) {
  if (e->button() == Qt::LeftButton && !m_labelLocked) {
    bool ok = false;
    const QString text = QInputDialog::getText(nullptr, QObject::tr("Arrow label"), QObject::tr("Label:"), QLineEdit::Normal, m_label, &ok);
    if (ok) {
      setLabel(text);
      setLabelHidden(false);
      setLabelInherited(false);
      setLabelSource(nullptr);
      if (text.isEmpty()) m_labelOffset = QPointF();
      if (auto* ds = qobject_cast<DiagramScene*>(scene())) {
        ds->propagateLabelFrom(this);
      }
    }
    e->accept();
    return;
  }
  QGraphicsPathItem::mouseDoubleClickEvent(e);
}

void ArrowItem::drawArrowHead(QPainterPath& path, const QPointF& tip, const QPointF& from) {
  const QPointF v = tip - from;
  const qreal len = std::hypot(v.x(), v.y());
  if (len < 0.001) return;

  const QPointF dir = v / len;
  const QPointF n(-dir.y(), dir.x());

  const qreal size = 10.0;
  const QPointF back = tip - dir * size;
  const QPointF left = back + n * (size * 0.45);
  const QPointF right = back - n * (size * 0.45);

  path.moveTo(left);
  path.lineTo(tip);
  path.lineTo(right);
}
