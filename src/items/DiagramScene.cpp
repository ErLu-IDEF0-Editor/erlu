#include "DiagramScene.h"
#include "items/BlockItem.h"
#include "items/ArrowItem.h"
#include "items/JunctionItem.h"

#include <QGraphicsSceneMouseEvent>
#include <QKeyEvent>
#include <QTimer>
#include <QHash>
#include <QSet>
#include <algorithm>
#include <cmath>

DiagramScene::DiagramScene(QObject* parent)
  : QGraphicsScene(parent)
{
  // Ограничиваем рабочую область A4 (210x297 мм) в пикселях при 96 dpi (~793x1122).
  // По умолчанию используем альбомную ориентацию (ширина больше высоты).
  const qreal w = 1122;
  const qreal h = 793;
  setSceneRect(-w * 0.5, -h * 0.5, w, h);
  auto* frame = addRect(sceneRect(), QPen(QColor(80, 80, 80), 1.5, Qt::DashLine), Qt::NoBrush);
  frame->setZValue(-5);
  frame->setEnabled(false);
  frame->setAcceptedMouseButtons(Qt::NoButton);
  frame->setData(0, QVariant(QStringLiteral("static-frame")));
  pushSnapshot();
}

BlockItem* DiagramScene::createBlockAt(const QPointF& scenePos) {
  auto* b = new BlockItem("Function", nullptr, m_nextBlockId++);
  addItem(b);
  b->setPos(scenePos - QPointF(100, 50)); // центрируем
  pushSnapshot();
  return b;
}

BlockItem* DiagramScene::createBlockWithId(const QPointF& scenePos, int id, const QString& title) {
  m_nextBlockId = std::max(m_nextBlockId, id + 1);
  auto* b = new BlockItem(title, nullptr, id);
  addItem(b);
  b->setPos(scenePos);
  return b;
}

DiagramScene::Edge DiagramScene::hitTestEdge(const QPointF& scenePos, QPointF* outScenePoint) const {
  const QRectF r = sceneRect();
  const qreal tol = 12.0;
  Edge edge = Edge::None;
  QPointF proj = scenePos;

  if (scenePos.y() >= r.top() && scenePos.y() <= r.bottom()) {
    if (std::abs(scenePos.x() - r.left()) <= tol) {
      edge = Edge::Left;
      proj = QPointF(r.left(), std::clamp(scenePos.y(), r.top(), r.bottom()));
    } else if (std::abs(scenePos.x() - r.right()) <= tol) {
      edge = Edge::Right;
      proj = QPointF(r.right(), std::clamp(scenePos.y(), r.top(), r.bottom()));
    }
  }
  if (scenePos.x() >= r.left() && scenePos.x() <= r.right()) {
    if (std::abs(scenePos.y() - r.top()) <= tol) {
      edge = Edge::Top;
      proj = QPointF(std::clamp(scenePos.x(), r.left(), r.right()), r.top());
    } else if (std::abs(scenePos.y() - r.bottom()) <= tol) {
      edge = Edge::Bottom;
      proj = QPointF(std::clamp(scenePos.x(), r.left(), r.right()), r.bottom());
    }
  }

  if (outScenePoint) *outScenePoint = proj;
  return edge;
}

bool DiagramScene::tryStartArrowDrag(const QPointF& scenePos) {
  const auto itemsUnder = items(scenePos);
  // проверяем, не стартуем ли с существующего junction
  for (QGraphicsItem* it : itemsUnder) {
    if (auto* j = qgraphicsitem_cast<JunctionItem*>(it)) {
      if (!j->hitTest(scenePos, 8.0)) continue;
      auto* a = new ArrowItem();
      addItem(a);
      ArrowItem::Endpoint from;
      from.junction = j;
      a->setFrom(from);
      a->setTempEndPoint(scenePos);
      m_dragArrow = a;
      m_dragFromBlock.clear();
      m_dragFromJunction = j;
      m_dragFromPort = BlockItem::Port::Output;
      return true;
    }
  }

  // ищем блок под курсором
  for (QGraphicsItem* it : itemsUnder) {
    auto* b = qgraphicsitem_cast<BlockItem*>(it);
    if (!b) continue;

    BlockItem::Port port{};
    QPointF localPos;
    if (!b->hitTestPort(scenePos, &port, &localPos)) continue;
    if (port != BlockItem::Port::Output) continue; // стартуем только с выходной стороны

    // Стартуем стрелку из порта
    m_dragFromBlock = b;
    m_dragFromJunction.clear();
    m_dragFromPort = port;

    auto* a = new ArrowItem();
    addItem(a);

    ArrowItem::Endpoint from;
    from.block = b;
    from.port = port;
    from.localPos = localPos;
    a->setFrom(from);
    a->setTempEndPoint(scenePos);
    m_dragArrow = a;
    return true;
  }

  QPointF edgePoint;
  Edge edge = hitTestEdge(scenePos, &edgePoint);
  if (edge != Edge::None && edge != Edge::Right) { // разрешаем старт с левой/верхней/нижней границы
    auto* a = new ArrowItem();
    addItem(a);
    ArrowItem::Endpoint from;
    from.scenePos = edgePoint;
    from.port = BlockItem::Port::Input; // ориентация не важна для свободной точки
    a->setFrom(from);
    a->setTempEndPoint(scenePos);
    m_dragArrow = a;
    m_dragFromBlock.clear();
    m_dragFromJunction.clear();
    m_dragFromPort = BlockItem::Port::Output;
    return true;
  }

  return false;
}

bool DiagramScene::tryFinishArrowDrag(const QPointF& scenePos) {
  if (!m_dragArrow) return false;

  const auto itemsUnder = items(scenePos);
  // сначала пробуем попасть в блок
  for (QGraphicsItem* it : itemsUnder) {
    auto* b = qgraphicsitem_cast<BlockItem*>(it);
    if (!b) continue;

    BlockItem::Port port{};
    QPointF localPos;
    if (!b->hitTestPort(scenePos, &port, &localPos)) continue;

    // запретим соединять в тот же самый порт того же блока (упрощение)
    if (b == m_dragFromBlock && port == m_dragFromPort) continue;

    ArrowItem::Endpoint to;
    to.block = b;
    to.port = port;
    to.localPos = localPos;
    m_dragArrow->setTo(to);
    m_dragArrow->finalize();

    m_dragArrow = nullptr;
    m_dragFromBlock.clear();
    m_dragFromJunction.clear();
    pushSnapshot();
    return true;
  }

  // затем — уже существующий junction
  for (QGraphicsItem* it : itemsUnder) {
    auto* j = qgraphicsitem_cast<JunctionItem*>(it);
    if (!j) continue;
    if (!j->hitTest(scenePos, 8.0)) continue;
    if (j == m_dragFromJunction) continue;

    ArrowItem::Endpoint to;
    to.junction = j;
    to.port = BlockItem::Port::Input;
    m_dragArrow->setTo(to);
    m_dragArrow->finalize();

    m_dragArrow = nullptr;
    m_dragFromBlock.clear();
    m_dragFromJunction.clear();
    pushSnapshot();
    return true;
  }

  // Попали в существующую стрелку — создаем junction и расщепляем.
  ArrowItem* targetArrow = nullptr;
  QPointF splitPoint;
  qreal bestDist = 8.0;
  for (QGraphicsItem* it : itemsUnder) {
    auto* arrow = qgraphicsitem_cast<ArrowItem*>(it);
    if (!arrow || arrow == m_dragArrow) continue;
    auto proj = arrow->hitTest(scenePos, bestDist);
    if (proj) {
      bestDist = std::hypot(proj->x() - scenePos.x(), proj->y() - scenePos.y());
      splitPoint = *proj;
      targetArrow = arrow;
    }
  }
  if (targetArrow) {
    auto* junction = new JunctionItem(nullptr, m_nextJunctionId++);
    addItem(junction);
    junction->setPos(splitPoint);

    ArrowItem::Endpoint mid;
    mid.junction = junction;
    mid.port = BlockItem::Port::Input;

    const ArrowItem::Endpoint origTo = targetArrow->to();
    targetArrow->setTo(mid);
    targetArrow->finalize();

    auto* forward = new ArrowItem();
    addItem(forward);
    forward->setFrom(mid);
    forward->setTo(origTo);
    forward->finalize();

    m_dragArrow->setTo(mid);
    m_dragArrow->finalize();

    m_dragArrow = nullptr;
    m_dragFromBlock.clear();
    m_dragFromJunction.clear();
    pushSnapshot();
    return true;
  }

  // если не попали в блок — попробуем закрепиться на границе сцены
  QPointF edgePoint;
  Edge edge = hitTestEdge(scenePos, &edgePoint);
  if (edge != Edge::None) {
    // правая граница может быть только приемником (уже гарантировано)
    ArrowItem::Endpoint to;
    to.scenePos = edgePoint;
    to.port = BlockItem::Port::Input;
    m_dragArrow->setTo(to);
    m_dragArrow->finalize();
    m_dragArrow = nullptr;
    m_dragFromBlock.clear();
    m_dragFromJunction.clear();
    pushSnapshot();
    return true;
  }

  return false;
}

bool DiagramScene::tryBranchAtArrow(const QPointF& scenePos) {
  ArrowItem* targetArrow = nullptr;
  QPointF splitPoint;
  qreal bestDist = 8.0;

  const auto itemsUnder = items(scenePos);
  for (QGraphicsItem* it : itemsUnder) {
    auto* arrow = qgraphicsitem_cast<ArrowItem*>(it);
    if (!arrow || arrow == m_dragArrow) continue;
    auto proj = arrow->hitTest(scenePos, bestDist);
    if (proj) {
      bestDist = std::hypot(proj->x() - scenePos.x(), proj->y() - scenePos.y());
      splitPoint = *proj;
      targetArrow = arrow;
    }
  }

  if (!targetArrow) return false;

  auto* junction = new JunctionItem(nullptr, m_nextJunctionId++);
  addItem(junction);
  junction->setPos(splitPoint);

  ArrowItem::Endpoint mid;
  mid.junction = junction;
  mid.port = BlockItem::Port::Input;

  const ArrowItem::Endpoint origTo = targetArrow->to();
  targetArrow->setTo(mid);
  targetArrow->finalize();

  auto* forward = new ArrowItem();
  addItem(forward);
  forward->setFrom(mid);
  forward->setTo(origTo);
  forward->finalize();

  auto* branch = new ArrowItem();
  addItem(branch);
  branch->setFrom(mid);
  branch->setTempEndPoint(scenePos);
  m_dragArrow = branch;
  m_dragFromBlock.clear();
  m_dragFromJunction = junction;
  m_dragFromPort = BlockItem::Port::Output;
  return true;
}

DiagramScene::Snapshot DiagramScene::captureSnapshot() const {
  Snapshot s;

  QSet<BlockItem*> blockSet;
  QSet<JunctionItem*> junctionSet;
  for (QGraphicsItem* it : items()) {
    if (auto* b = qgraphicsitem_cast<BlockItem*>(it)) blockSet.insert(b);
    if (auto* j = qgraphicsitem_cast<JunctionItem*>(it)) junctionSet.insert(j);
  }

  auto encodeEp = [&](const ArrowItem::Endpoint& ep) {
    Snapshot::Endpoint out;
    if (ep.block && blockSet.contains(ep.block) && ep.block->scene() == this) {
      out.kind = Snapshot::Endpoint::Kind::Block;
      out.id = ep.block->id();
      out.port = ep.port;
      if (ep.localPos) out.localPos = *ep.localPos;
    } else if (ep.junction && junctionSet.contains(ep.junction) && ep.junction->scene() == this) {
      out.kind = Snapshot::Endpoint::Kind::Junction;
      out.id = ep.junction->id();
    } else if (ep.scenePos) {
      out.kind = Snapshot::Endpoint::Kind::Scene;
      out.scenePos = *ep.scenePos;
    }
    return out;
  };

  for (QGraphicsItem* it : items()) {
    if (auto* b = qgraphicsitem_cast<BlockItem*>(it)) {
      s.blocks.push_back({b->id(), b->title(), b->pos()});
    }
  }

  for (QGraphicsItem* it : items()) {
    if (auto* j = qgraphicsitem_cast<JunctionItem*>(it)) {
      s.junctions.push_back({j->id(), j->pos()});
    }
  }

  for (QGraphicsItem* it : items()) {
    if (auto* a = qgraphicsitem_cast<ArrowItem*>(it)) {
      Snapshot::Arrow ar;
      ar.from = encodeEp(a->from());
      ar.to = encodeEp(a->to());
      if (ar.from.kind == Snapshot::Endpoint::Kind::None || ar.to.kind == Snapshot::Endpoint::Kind::None) {
        continue; // skip incomplete arrows
      }
      ar.bend = a->bendOffset();
      ar.top = a->topOffset();
      ar.bottom = a->bottomOffset();
      ar.label = a->label();
      ar.labelOffset = a->labelOffset();
      s.arrows.push_back(std::move(ar));
    }
  }
  return s;
}

void DiagramScene::restoreSnapshot(const Snapshot& snap) {
  m_restoringSnapshot = true;
  cancelCurrentDrag();
  // Clear dynamic items but keep the static frame.
  QList<QGraphicsItem*> toRemove;
  for (QGraphicsItem* it : items()) {
    if (it->data(0).toString() == QStringLiteral("static-frame")) continue;
    toRemove.append(it);
  }
  for (QGraphicsItem* it : toRemove) {
    removeItem(it);
    delete it;
  }

  m_nextBlockId = 1;
  m_nextJunctionId = 1;

  QHash<int, BlockItem*> blockMap;
  QHash<int, JunctionItem*> junctionMap;

  for (const auto& b : snap.blocks) {
    auto* blk = createBlockWithId(b.pos, b.id, b.title);
    blockMap.insert(b.id, blk);
    m_nextBlockId = std::max(m_nextBlockId, b.id + 1);
  }
  for (const auto& j : snap.junctions) {
    m_nextJunctionId = std::max(m_nextJunctionId, j.id + 1);
    auto* jun = new JunctionItem(nullptr, j.id);
    addItem(jun);
    jun->setPos(j.pos);
    junctionMap.insert(j.id, jun);
  }

  auto decodeEp = [&](const Snapshot::Endpoint& ep) {
    ArrowItem::Endpoint out;
    switch (ep.kind) {
      case Snapshot::Endpoint::Kind::Block:
        out.block = blockMap.value(ep.id, nullptr);
        out.port = ep.port;
        out.localPos = ep.localPos;
        break;
      case Snapshot::Endpoint::Kind::Junction:
        out.junction = junctionMap.value(ep.id, nullptr);
        break;
      case Snapshot::Endpoint::Kind::Scene:
        out.scenePos = ep.scenePos;
        out.port = BlockItem::Port::Input;
        break;
      case Snapshot::Endpoint::Kind::None:
        break;
    }
    return out;
  };

  for (const auto& a : snap.arrows) {
    auto* ar = new ArrowItem();
    addItem(ar);
    ar->setFrom(decodeEp(a.from));
    ar->setTo(decodeEp(a.to));
    ar->setOffsets(a.bend, a.top, a.bottom);
    ar->setLabel(a.label);
    ar->setLabelOffset(a.labelOffset);
    ar->finalize();
  }

  m_restoringSnapshot = false;
}

void DiagramScene::pushSnapshot() {
  if (m_restoringSnapshot) return;
  Snapshot s = captureSnapshot();
  if (m_historyIndex + 1 < m_history.size()) {
    m_history.resize(m_historyIndex + 1);
  }
  m_history.push_back(std::move(s));
  m_historyIndex = m_history.size() - 1;
}

void DiagramScene::scheduleSnapshot() {
  if (m_restoringSnapshot || m_snapshotScheduled) return;
  m_snapshotScheduled = true;
  QTimer::singleShot(0, this, [this]{
    m_snapshotScheduled = false;
    pushSnapshot();
  });
}

void DiagramScene::undo() {
  if (m_historyIndex <= 0) return;
  m_historyIndex -= 1;
  restoreSnapshot(m_history[m_historyIndex]);
}

void DiagramScene::mousePressEvent(QGraphicsSceneMouseEvent* e) {
  if (e->button() == Qt::LeftButton) {
    if (e->modifiers().testFlag(Qt::ControlModifier)) {
      if (tryBranchAtArrow(e->scenePos())) {
        e->accept();
        return;
      }
    }
    // если кликнули на порт — начинаем протягивание стрелки
    if (tryStartArrowDrag(e->scenePos())) {
      e->accept();
      return;
    }
    // начало перемещения существующих элементов — запомним позиции
    m_pressPositions.clear();
    const auto sel = selectedItems();
    for (QGraphicsItem* it : sel) {
      if (it->flags() & QGraphicsItem::ItemIsMovable) {
        m_pressPositions.insert(it, it->pos());
      }
    }
    m_itemDragActive = !m_pressPositions.isEmpty();
  }

  if (e->button() == Qt::RightButton) {
    // правой кнопкой — добавить блок
    createBlockAt(e->scenePos());
    e->accept();
    return;
  }

  QGraphicsScene::mousePressEvent(e);
}

void DiagramScene::mouseMoveEvent(QGraphicsSceneMouseEvent* e) {
  if (m_dragArrow) {
    m_dragArrow->setTempEndPoint(e->scenePos());
    e->accept();
    return;
  }
  QGraphicsScene::mouseMoveEvent(e);
}

void DiagramScene::mouseReleaseEvent(QGraphicsSceneMouseEvent* e) {
  if (m_dragArrow && e->button() == Qt::LeftButton) {
    if (!tryFinishArrowDrag(e->scenePos())) {
      // не получилось — удаляем временную стрелку
      cancelCurrentDrag();
    }
    e->accept();
  }
  if (m_itemDragActive && e->button() == Qt::LeftButton) {
    maybeSnapshotMovedItems();
    m_itemDragActive = false;
  }
  QGraphicsScene::mouseReleaseEvent(e);
}

void DiagramScene::keyPressEvent(QKeyEvent* e) {
  if (e->key() == Qt::Key_Escape) {
    cancelCurrentDrag();
    e->accept();
    return;
  }
  if (e->key() == Qt::Key_Delete || e->key() == Qt::Key_Backspace) {
    deleteSelection();
    e->accept();
    return;
  }
  if (e->matches(QKeySequence::Undo)) {
    undo();
    e->accept();
    return;
  }
  QGraphicsScene::keyPressEvent(e);
}

void DiagramScene::cancelCurrentDrag() {
  if (m_dragArrow) {
    removeItem(m_dragArrow);
    delete m_dragArrow;
    m_dragArrow = nullptr;
    m_dragFromBlock.clear();
    m_dragFromJunction.clear();
  }
}

void DiagramScene::deleteSelection() {
  if (m_dragArrow) {
    cancelCurrentDrag();
    return;
  }

  QSet<QGraphicsItem*> toDelete;
  const auto sel = selectedItems();
  for (QGraphicsItem* it : sel) {
    if (qgraphicsitem_cast<BlockItem*>(it) || qgraphicsitem_cast<JunctionItem*>(it) || qgraphicsitem_cast<ArrowItem*>(it)) {
      toDelete.insert(it);
    }
  }

  // also delete arrows connected to selected blocks/junctions
  for (QGraphicsItem* it : items()) {
    auto* arrow = qgraphicsitem_cast<ArrowItem*>(it);
    if (!arrow) continue;
    const auto f = arrow->from();
    const auto t = arrow->to();
    if ((f.block && toDelete.contains(f.block)) ||
        (t.block && toDelete.contains(t.block)) ||
        (f.junction && toDelete.contains(f.junction)) ||
        (t.junction && toDelete.contains(t.junction))) {
      toDelete.insert(arrow);
    }
  }

  // delete arrows first to avoid dangling removal from deleted blocks/junctions
  for (QGraphicsItem* it : toDelete) {
    if (qgraphicsitem_cast<ArrowItem*>(it)) {
      removeItem(it);
      delete it;
    }
  }
  for (QGraphicsItem* it : toDelete) {
    if (!qgraphicsitem_cast<ArrowItem*>(it)) {
      removeItem(it);
      delete it;
    }
  }

  if (!toDelete.isEmpty()) pushSnapshot();
}

void DiagramScene::requestSnapshot() {
  pushSnapshot();
}

void DiagramScene::maybeSnapshotMovedItems() {
  bool moved = false;
  for (auto it = m_pressPositions.cbegin(); it != m_pressPositions.cend(); ++it) {
    if (!it.key()) continue;
    const QPointF cur = it.key()->pos();
    const QPointF old = it.value();
    if (!qFuzzyCompare(cur.x(), old.x()) || !qFuzzyCompare(cur.y(), old.y())) {
      moved = true;
      break;
    }
  }
  m_pressPositions.clear();
  if (moved) pushSnapshot();
}
