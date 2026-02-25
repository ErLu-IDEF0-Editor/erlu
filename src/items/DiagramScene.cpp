#include "DiagramScene.h"
#include "items/BlockItem.h"
#include "items/ArrowItem.h"
#include "items/JunctionItem.h"
#include "items/HeaderFooterItem.h"

#include <QGraphicsSceneMouseEvent>
#include <QKeyEvent>
#include <QTimer>
#include <QHash>
#include <QSet>
#include <QDebug>
#include <QJsonObject>
#include <QJsonArray>
#include <QJsonDocument>
#include <QVariantMap>
#include <QGraphicsRectItem>
#include <QSizeF>
#include <QBrush>
#include <QColor>
#include <algorithm>
#include <cmath>
#include <optional>

DiagramScene::DiagramScene(QObject* parent)
  : QGraphicsScene(parent)
{
  m_currentPrefix.clear();
  // Ограничиваем рабочую область A4 (210x297 мм) в пикселях при 96 dpi (~793x1122).
  // По умолчанию используем альбомную ориентацию (ширина больше высоты).
  const qreal w = 1122;
  const qreal h = 793;
  setSceneRect(-w * 0.5, -h * 0.5, w, h);
  ensureFrame();
  ensureHeaderFooter();
  pushSnapshot();
}

BlockItem* DiagramScene::createBlockAt(const QPointF& scenePos) {
  auto* b = new BlockItem(QString(), nullptr, m_nextBlockId++);
  addItem(b);
  b->setNumber(assignNumber(b));
  connectBlockSignals(b);
  const QRectF r = m_contentRect.isNull() ? sceneRect() : m_contentRect;
  const QPointF desired = scenePos - QPointF(100, 50); // центрируем
  const QRectF br = b->boundingRect().translated(desired);
  QPointF clamped = desired;
  if (br.left() < r.left()) clamped.setX(desired.x() + (r.left() - br.left()));
  if (br.right() > r.right()) clamped.setX(clamped.x() - (br.right() - r.right()));
  if (br.top() < r.top()) clamped.setY(desired.y() + (r.top() - br.top()));
  if (br.bottom() > r.bottom()) clamped.setY(clamped.y() - (br.bottom() - r.bottom()));
  b->setPos(clamped);
  pushSnapshot();
  return b;
}

BlockItem* DiagramScene::createBlockWithId(const QPointF& scenePos, int id, const QString& title) {
  m_nextBlockId = std::max(m_nextBlockId, id + 1);
  auto* b = new BlockItem(title, nullptr, id);
  addItem(b);
  b->setPos(scenePos);
  if (b->number().isEmpty()) b->setNumber(assignNumber(b));
  connectBlockSignals(b);
  return b;
}

void DiagramScene::applyMeta(const QVariantMap& meta) {
  m_meta = meta;
  const bool colorfulMode = meta.value("colorfulMode", false).toBool();
  const bool darkMode = meta.value("resolvedDarkMode",
                                   meta.value("darkMode", false)).toBool();

  const QColor fg = darkMode ? QColor(235, 235, 235) : QColor(20, 20, 20);
  const QColor bg = darkMode ? QColor(26, 26, 26) : QColor(255, 255, 255);
  QColor arrowColor = fg;
  QColor blockBorderColor = fg;
  QColor blockFontColor = fg;
  if (colorfulMode) {
    const QColor a(meta.value("arrowColor").toString());
    const QColor b(meta.value("blockBorderColor").toString());
    const QColor f(meta.value("blockFontColor").toString());
    if (a.isValid()) arrowColor = a;
    if (b.isValid()) blockBorderColor = b;
    if (f.isValid()) blockFontColor = f;
  }

  setBackgroundBrush(QBrush(bg));
  BlockItem::setVisualTheme(fg,
                            bg,
                            blockBorderColor,
                            blockFontColor,
                            darkMode ? QColor(60, 60, 80) : QColor(240, 240, 255));
  ArrowItem::setVisualTheme(arrowColor, blockFontColor);
  JunctionItem::setVisualTheme(arrowColor,
                               darkMode ? QColor(120, 180, 255) : QColor(40, 100, 255));

  const QString currency = meta.value("currency").toString();
  const QString placement = meta.value("currencyPlacement").toString();
  BlockItem::setCurrencyFormat(currency, placement);
  for (QGraphicsItem* it : items()) {
    if (auto* b = qgraphicsitem_cast<BlockItem*>(it)) {
      b->update();
    } else if (auto* a = qgraphicsitem_cast<ArrowItem*>(it)) {
      a->updatePath();
    } else if (auto* j = qgraphicsitem_cast<JunctionItem*>(it)) {
      j->update();
    }
  }

  const QString size = meta.value("pageSize").toString();
  const QString orient = meta.value("pageOrientation").toString();
  auto sizeToMm = [](const QString& s) {
    if (s == "A1") return QSizeF(594, 841);
    if (s == "A2") return QSizeF(420, 594);
    if (s == "A3") return QSizeF(297, 420);
    return QSizeF(210, 297);
  };
  auto toPx = [](qreal mm) {
    return mm * 96.0 / 25.4;
  };
  QSizeF mm = sizeToMm(size.isEmpty() ? "A4" : size);
  QSizeF px(toPx(mm.width()), toPx(mm.height()));
  if (orient == tr("Landscape") || orient == "Landscape") {
    px = QSizeF(std::max(px.width(), px.height()), std::min(px.width(), px.height()));
  } else if (orient == tr("Portrait") || orient == "Portrait") {
    px = QSizeF(std::min(px.width(), px.height()), std::max(px.width(), px.height()));
  }
  const QRectF newRect(-px.width() * 0.5, -px.height() * 0.5, px.width(), px.height());
  if (sceneRect() != newRect) {
    setSceneRect(newRect);
  }

  const bool withHeader = meta.value("withHeader", true).toBool();
  const bool withFooter = meta.value("withFooter", true).toBool();
  const qreal h = sceneRect().height();
  const qreal headerH = withHeader ? h * 0.12 : 0.0;
  const qreal footerH = withFooter ? h * 0.08 : 0.0;
  m_contentRect = sceneRect().adjusted(0, headerH, 0, -footerH);

  ensureFrame();
  ensureHeaderFooter();
  if (m_headerFooter) {
    m_headerFooter->setShowHeaderFooter(withHeader, withFooter);
    m_headerFooter->setMeta(m_meta);
    m_headerFooter->setPageRect(sceneRect());
    m_headerFooter->update();
  }
}

void DiagramScene::updateMeta(const QVariantMap& patch) {
  QVariantMap merged = m_meta;
  for (auto it = patch.cbegin(); it != patch.cend(); ++it) {
    merged[it.key()] = it.value();
  }
  applyMeta(merged);
  emit metaChanged(m_meta);
}

QString DiagramScene::currentNodeLabel() const {
  if (m_currentBlockId < 0) return tr("TOP");
  if (!m_currentPrefix.isEmpty()) return m_currentPrefix;
  return tr("A0");
}

QString DiagramScene::currentDiagramTitle() const {
  if (m_currentBlockId < 0) {
    return m_meta.value("project").toString();
  }
  if (!m_hierarchy.isEmpty()) {
    const Snapshot& parent = m_hierarchy.last().snapshot;
    for (const auto& b : parent.blocks) {
      if (b.id == m_currentBlockId) return b.title;
    }
  }
  return m_meta.value("title").toString();
}

DiagramScene::Edge DiagramScene::hitTestEdge(const QPointF& scenePos, QPointF* outScenePoint) const {
  const QRectF r = m_contentRect.isNull() ? sceneRect() : m_contentRect;
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

bool DiagramScene::tryStartArrowDrag(const QPointF& scenePos, Qt::KeyboardModifiers mods) {
  const auto itemsUnder = items(scenePos);
  // интерфейсный круг
  for (QGraphicsItem* it : itemsUnder) {
    if (auto* a = qgraphicsitem_cast<ArrowItem*>(it)) {
      if (!a->isInterfaceStub()) continue;
      auto edge = a->interfaceEdge();
      if (!edge) continue;

      // Shift+drag to move along edge
      if (mods.testFlag(Qt::ShiftModifier)) {
        m_dragInterfaceStub = a;
        return true;
      }

      a->setInterfaceIsStub(false);
      a->setTempEndPoint(scenePos);
      a->setLabelLocked(true);
      m_dragArrow = a;
      m_dragFromBlock.clear();
      m_dragFromJunction.clear();
      m_dragFromPort = edge->port;
      return true;
    }
  }

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
  // попадание в интерфейсный круг
  for (QGraphicsItem* it : itemsUnder) {
    if (auto* iface = qgraphicsitem_cast<ArrowItem*>(it)) {
      if (!iface->isInterfaceStub()) continue;
      auto edge = iface->interfaceEdge();
      if (!edge) continue;
      // only allow connecting into output-side stubs (targets)
      if (edge->port != BlockItem::Port::Output) continue;
      ArrowItem::Endpoint stubEp = *edge;
      if (!stubEp.scenePos) stubEp.scenePos = scenePos;
      ArrowItem::Endpoint src = m_dragArrow->from();

      iface->setInterfaceIsStub(false);
      iface->setFrom(src);
      iface->setTo(stubEp);
      iface->setLabel(iface->label());
      iface->setLabelLocked(true);
      iface->finalize();

      removeItem(m_dragArrow);
      delete m_dragArrow;
      m_dragArrow = nullptr;
      m_dragFromBlock.clear();
      m_dragFromJunction.clear();
      pushSnapshot();
      return true;
    }
  }

  // сначала пробуем попасть в блок
  for (QGraphicsItem* it : itemsUnder) {
    auto* b = qgraphicsitem_cast<BlockItem*>(it);
    if (!b) continue;

    BlockItem::Port port{};
    QPointF localPos;
    if (!b->hitTestPort(scenePos, &port, &localPos)) continue;

    // запретим соединять в тот же самый порт того же блока (упрощение)
    if (b == m_dragFromBlock && port == m_dragFromPort) continue;

    if (m_dragArrow->isInterface()) {
      auto edge = m_dragArrow->interfaceEdge();
      if (edge) {
        ArrowItem::Endpoint stubEp = *edge;
        if (!stubEp.scenePos) stubEp.scenePos = scenePos;
        ArrowItem::Endpoint blockEp;
        blockEp.block = b;
        blockEp.port = port;
        blockEp.localPos = localPos;
        if (stubEp.port == BlockItem::Port::Output) {
          m_dragArrow->setFrom(blockEp);
          m_dragArrow->setTo(stubEp);
        } else {
          m_dragArrow->setFrom(stubEp);
          m_dragArrow->setTo(blockEp);
        }
        m_dragArrow->setLabelLocked(true);
        m_dragArrow->finalize();
      }
    } else {
      ArrowItem::Endpoint to;
      to.block = b;
      to.port = port;
      to.localPos = localPos;
      m_dragArrow->setTo(to);
      m_dragArrow->finalize();
    }

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

    if (m_dragArrow->isInterface()) {
      auto edge = m_dragArrow->interfaceEdge();
      if (edge) {
        ArrowItem::Endpoint stubEp = *edge;
        if (!stubEp.scenePos) stubEp.scenePos = scenePos;
        ArrowItem::Endpoint jun;
        jun.junction = j;
        jun.port = BlockItem::Port::Input;
        if (stubEp.port == BlockItem::Port::Output) {
          m_dragArrow->setFrom(jun);
          m_dragArrow->setTo(stubEp);
        } else {
          m_dragArrow->setFrom(stubEp);
          m_dragArrow->setTo(jun);
        }
        m_dragArrow->setLabelLocked(true);
        m_dragArrow->finalize();
      }
    } else {
      ArrowItem::Endpoint to;
      to.junction = j;
      to.port = BlockItem::Port::Input;
      m_dragArrow->setTo(to);
      m_dragArrow->finalize();
    }

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

    ArrowItem* sourceRoot = targetArrow->labelSourceRoot();
    const QString inheritedLabel = targetArrow->label();
    const bool hideInherited = !inheritedLabel.isEmpty();

    auto* forward = new ArrowItem();
    addItem(forward);
    forward->setFrom(mid);
    forward->setTo(origTo);
    forward->setLabel(inheritedLabel);
    forward->setLabelHidden(hideInherited);
    forward->setLabelInherited(true);
    forward->setLabelSource(sourceRoot);
    forward->finalize();

    m_dragArrow->setTo(mid);
    m_dragArrow->setLabel(inheritedLabel);
    m_dragArrow->setLabelHidden(hideInherited);
    m_dragArrow->setLabelInherited(true);
    m_dragArrow->setLabelSource(sourceRoot);
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
    const auto fromEp = m_dragArrow->from();
    if (fromEp.scenePos.has_value() && !fromEp.block && !fromEp.junction) {
      // запрет: нельзя начинать и заканчивать стрелку на границе
      return false;
    }
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

    ArrowItem* sourceRoot = targetArrow->labelSourceRoot();
    const QString inheritedLabel = targetArrow->label();
    const bool hideInherited = !inheritedLabel.isEmpty();

    auto* forward = new ArrowItem();
    addItem(forward);
    forward->setFrom(mid);
    forward->setTo(origTo);
    forward->setLabel(inheritedLabel);
    forward->setLabelHidden(hideInherited);
    forward->setLabelInherited(true);
    forward->setLabelSource(sourceRoot);
    forward->finalize();

    auto* branch = new ArrowItem();
    addItem(branch);
    branch->setFrom(mid);
    branch->setLabel(inheritedLabel);
    branch->setLabelHidden(hideInherited);
    branch->setLabelInherited(true);
    branch->setLabelSource(sourceRoot);
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
      out.port = ep.port;
    } else if (ep.scenePos) {
      out.kind = Snapshot::Endpoint::Kind::Scene;
      out.scenePos = *ep.scenePos;
      out.port = ep.port;
    }
    return out;
  };

  for (QGraphicsItem* it : items()) {
    if (auto* b = qgraphicsitem_cast<BlockItem*>(it)) {
      const bool hasDecomp = childHasContent(b->id());
      b->setHasDecomposition(hasDecomp);
      Snapshot::Block blk;
      blk.id = b->id();
      blk.title = b->title();
      blk.pos = b->pos();
      blk.hasDecomp = hasDecomp;
      blk.number = b->number();
      blk.price = b->price();
      if (auto col = b->customColor()) {
        blk.color = col->name();
      }
      s.blocks.push_back(blk);
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
      ar.labelHidden = a->isLabelHidden();
      ar.labelOffset = a->labelOffset();
      ar.isInterface = a->isInterface();
      ar.isInterfaceStub = a->isInterfaceStub();
      ar.labelLocked = a->isLabelLocked();
      ar.callMechanism = a->isCallMechanism();
      ar.callRefId = a->callRefId();
      if (a->interfaceEdge()) {
        ar.interfaceEdge = encodeEp(*a->interfaceEdge());
      }
      if (auto col = a->customColor()) {
        ar.color = col->name();
      }
      s.arrows.push_back(std::move(ar));
    }
  }
  return s;
}

void DiagramScene::restoreSnapshot(const Snapshot& snap, bool resetHistoryState) {
  m_restoringSnapshot = true;
  cancelCurrentDrag();
  clear();
  m_headerFooter = nullptr;
  ensureFrame();
  ensureHeaderFooter();

  m_nextBlockId = 1;
  m_nextJunctionId = 1;

  QHash<int, BlockItem*> blockMap;
  QHash<int, JunctionItem*> junctionMap;

  for (const auto& b : snap.blocks) {
    auto* blk = createBlockWithId(b.pos, b.id, b.title);
    blk->setHasDecomposition(b.hasDecomp || childHasContent(b.id));
    if (!b.number.isEmpty()) {
      blk->setNumber(b.number);
    } else {
      blk->setNumber(assignNumber(blk));
    }
    if (b.price.has_value()) {
      blk->setPrice(b.price);
    }
    if (b.color.has_value()) {
      const QColor col(*b.color);
      if (col.isValid()) blk->setCustomColor(col);
    }
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
        out.port = ep.port;
        break;
      case Snapshot::Endpoint::Kind::Scene:
        out.scenePos = ep.scenePos;
        out.port = ep.port;
        break;
      case Snapshot::Endpoint::Kind::None:
        break;
    }
    return out;
  };

  for (const auto& a : snap.arrows) {
    auto* ar = new ArrowItem();
    addItem(ar);
    const auto fromEp = decodeEp(a.from);
    const auto toEp = decodeEp(a.to);
    if (a.isInterface && a.interfaceEdge.kind != Snapshot::Endpoint::Kind::None) {
      ar->setInterfaceStub(decodeEp(a.interfaceEdge), a.label);
      ar->setInterfaceIsStub(a.isInterfaceStub);
      if (!a.isInterfaceStub) {
        ar->setFrom(fromEp);
        ar->setTo(toEp);
      }
    } else {
      ar->setFrom(fromEp);
      ar->setTo(toEp);
    }
    ar->setOffsets(a.bend, a.top, a.bottom);
    ar->setLabel(a.label);
    ar->setLabelOffset(a.labelOffset);
    ar->setLabelHidden(a.labelHidden);
    ar->setLabelInherited(a.labelInherited);
    ar->setLabelLocked(a.labelLocked);
    ar->setCallMechanism(a.callMechanism);
    ar->setCallRefId(a.callRefId);
    if (a.color.has_value()) {
      const QColor col(*a.color);
      if (col.isValid()) ar->setCustomColor(col);
    }
    ar->finalize();
  }

  m_restoringSnapshot = false;
  updateCallMechanismLabels();
  if (resetHistoryState) {
    resetHistory(captureSnapshot());
  }
}

void DiagramScene::pushSnapshot() {
  if (m_restoringSnapshot) return;
  Snapshot s = captureSnapshot();
  if (m_historyIndex + 1 < m_history.size()) {
    m_history.resize(m_historyIndex + 1);
  }
  m_history.push_back(std::move(s));
  m_historyIndex = m_history.size() - 1;
  // refresh dog-ears based on known children
  for (QGraphicsItem* it : items()) {
    if (auto* blk = qgraphicsitem_cast<BlockItem*>(it)) {
      blk->setHasDecomposition(childHasContent(blk->id()));
    }
  }
  emit changed();
}

void DiagramScene::scheduleSnapshot() {
  if (m_restoringSnapshot || m_snapshotScheduled) return;
  m_snapshotScheduled = true;
  QTimer::singleShot(0, this, [this]{
    m_snapshotScheduled = false;
    pushSnapshot();
    updateCallMechanismLabels();
  });
}

void DiagramScene::connectBlockSignals(BlockItem* b) {
  if (!b) return;
  connect(b, &BlockItem::titleChanged, this, [this]{ updateCallMechanismLabels(); });
  connect(b, &BlockItem::numberChanged, this, [this]{ updateCallMechanismLabels(); });
}

void DiagramScene::updateCallMechanismLabels() {
  QHash<int, BlockItem*> blockById;
  for (QGraphicsItem* it : items()) {
    if (auto* b = qgraphicsitem_cast<BlockItem*>(it)) {
      blockById.insert(b->id(), b);
    }
  }
  for (QGraphicsItem* it : items()) {
    auto* a = qgraphicsitem_cast<ArrowItem*>(it);
    if (!a || !a->isCallMechanism()) continue;
    BlockItem* ref = blockById.value(a->callRefId(), nullptr);
    if (!ref) continue;
    const QString num = ref->number();
    const QString title = ref->title();
    const QString label = num.isEmpty() ? title : QStringLiteral("%1 %2").arg(num, title);
    a->setLabel(label);
    a->setLabelHidden(false);
  }
}

void DiagramScene::purgeBrokenCallMechanisms() {
  QSet<BlockItem*> blocks;
  for (QGraphicsItem* it : items()) {
    if (auto* b = qgraphicsitem_cast<BlockItem*>(it)) blocks.insert(b);
  }
  QVector<ArrowItem*> toRemove;
  for (QGraphicsItem* it : items()) {
    if (auto* a = qgraphicsitem_cast<ArrowItem*>(it)) {
      if (!a->isCallMechanism()) continue;
      auto from = a->from();
      auto to = a->to();
      bool ok = true;
      if (from.block && !blocks.contains(from.block)) ok = false;
      if (to.block && !blocks.contains(to.block)) ok = false;
      if (!ok) toRemove.push_back(a);
    }
  }
  for (ArrowItem* a : toRemove) {
    removeItem(a);
    delete a;
  }
}

void DiagramScene::undo() {
  if (m_historyIndex <= 0) return;
  m_historyIndex -= 1;
  restoreSnapshot(m_history[m_historyIndex], false);
}

bool DiagramScene::goDownIntoSelected() {
  const auto sel = selectedItems();
  if (sel.size() != 1) return false;
  auto* b = qgraphicsitem_cast<BlockItem*>(sel.first());
  if (!b) return false;

  return goDownIntoBlock(b);
}

bool DiagramScene::goUp() {
  if (m_hierarchy.isEmpty()) return false;
  Snapshot child = captureSnapshot();
  if (m_currentBlockId >= 0) {
    m_children[currentPathKey()] = child;
  }
  const HierNode parent = m_hierarchy.takeLast();
  if (m_currentBlockId >= 0) {
    for (QGraphicsItem* it : items()) {
      if (auto* blk = qgraphicsitem_cast<BlockItem*>(it)) {
        if (blk->id() == m_currentBlockId) {
          blk->setHasDecomposition(true);
          break;
        }
      }
    }
  }
  m_currentBlockId = parent.blockId;
  m_currentPrefix = parent.prefix;
  restoreSnapshot(parent.snapshot);
  return true;
}

bool DiagramScene::goDownIntoBlock(BlockItem* b) {
  if (!b) return false;
  const QString childKey = childKeyFor(b->id());
  Snapshot current = captureSnapshot();
  m_hierarchy.push_back({m_currentBlockId, current, m_currentPrefix});
  m_currentBlockId = b->id();
  m_currentPrefix = b->number().isEmpty() ? m_currentPrefix : b->number();
  Snapshot child;
  if (m_children.contains(childKey)) {
    child = m_children.value(childKey);
    QVector<Snapshot::Arrow> preserved;
    preserved.reserve(child.arrows.size());
    for (const auto& ar : child.arrows) {
      if (!(ar.isInterface && ar.isInterfaceStub)) preserved.push_back(ar);
    }
    child.arrows = std::move(preserved);
  }

  // build interface stubs from current parent connections
  const QRectF frame = m_contentRect.isNull() ? sceneRect() : m_contentRect;
  auto normClamp = [](qreal v) { return std::clamp(v, 0.0, 1.0); };
  auto frameEdgePoint = [&](BlockItem::Port port, qreal t, bool inward) {
    const qreal inset = 80.0;
    switch (port) {
      case BlockItem::Port::Input: {
        QPointF edge(frame.left(), frame.top() + t * frame.height());
        return inward ? edge + QPointF(inset, 0) : edge;
      }
      case BlockItem::Port::Output: {
        QPointF edge(frame.right(), frame.top() + t * frame.height());
        return inward ? edge - QPointF(inset, 0) : edge;
      }
      case BlockItem::Port::Control: {
        QPointF edge(frame.left() + t * frame.width(), frame.top());
        return inward ? edge + QPointF(0, inset) : edge;
      }
      case BlockItem::Port::Mechanism: {
        QPointF edge(frame.left() + t * frame.width(), frame.bottom());
        return inward ? edge - QPointF(0, inset) : edge;
      }
    }
    return QPointF();
  };

  auto scenePosFor = [](const ArrowItem::Endpoint& ep) {
    if (ep.block) {
      if (ep.localPos) return ep.block->mapToScene(*ep.localPos);
      return ep.block->portScenePos(ep.port);
    }
    if (ep.junction) return ep.junction->scenePos();
    if (ep.scenePos) return *ep.scenePos;
    return QPointF();
  };

  auto makeSceneEndpoint = [](const QPointF& pos, BlockItem::Port port) {
    Snapshot::Endpoint ep;
    ep.kind = Snapshot::Endpoint::Kind::Scene;
    ep.scenePos = pos;
    ep.port = port;
    return ep;
  };

  auto edgeKey = [](const Snapshot::Endpoint& ep) {
    QPointF p = ep.scenePos.value_or(QPointF());
    // coarse rounding to avoid mismatch on re-entry
    return QStringLiteral("%1:%2:%3").arg(int(ep.port)).arg(p.x(), 0, 'f', 1).arg(p.y(), 0, 'f', 1);
  };
  auto labelKey = [](const Snapshot::Arrow& ar) {
    return QStringLiteral("%1:%2").arg(int(ar.interfaceEdge.port)).arg(ar.label);
  };

  QSet<QString> usedInterfaces;
  for (const auto& ar : child.arrows) {
    if (!ar.isInterface) continue;
    if (!ar.label.isEmpty()) {
      usedInterfaces.insert(labelKey(ar));
    } else {
      usedInterfaces.insert(edgeKey(ar.interfaceEdge));
    }
  }

  const QRectF br = b->mapRectToScene(b->boundingRect());
  auto tY = [&](const QPointF& p) { return normClamp((p.y() - br.top()) / br.height()); };
  auto tX = [&](const QPointF& p) { return normClamp((p.x() - br.left()) / br.width()); };

  for (QGraphicsItem* it : items()) {
    auto* a = qgraphicsitem_cast<ArrowItem*>(it);
    if (!a) continue;
    const auto f = a->from();
    const auto t = a->to();
    if (t.block == b) {
      const QPointF portPos = scenePosFor(t);
      const qreal tt = (t.port == BlockItem::Port::Control || t.port == BlockItem::Port::Mechanism) ? tX(portPos) : tY(portPos);
      const QPointF edgeScene = frameEdgePoint(t.port, tt, false);
      Snapshot::Arrow ar;
      ar.from = makeSceneEndpoint(edgeScene, t.port);
      ar.to = ar.from;
      ar.label = a->label();
      ar.isInterface = true;
      ar.isInterfaceStub = true;
      ar.labelLocked = true;
      ar.interfaceEdge = ar.from;
      const QString key = ar.label.isEmpty() ? edgeKey(ar.interfaceEdge) : labelKey(ar);
      if (!usedInterfaces.contains(key)) {
        usedInterfaces.insert(key);
        child.arrows.push_back(ar);
      }
    } else if (f.block == b) {
      const QPointF portPos = scenePosFor(f);
      const qreal tt = (f.port == BlockItem::Port::Control || f.port == BlockItem::Port::Mechanism) ? tX(portPos) : tY(portPos);
      const QPointF edgeScene = frameEdgePoint(f.port, tt, false);
      Snapshot::Arrow ar;
      ar.from = makeSceneEndpoint(edgeScene, f.port);
      ar.to = ar.from;
      ar.label = a->label();
      ar.isInterface = true;
      ar.isInterfaceStub = true;
      ar.labelLocked = true;
      ar.interfaceEdge = ar.to;
      const QString key = ar.label.isEmpty() ? edgeKey(ar.interfaceEdge) : labelKey(ar);
      if (!usedInterfaces.contains(key)) {
        usedInterfaces.insert(key);
        child.arrows.push_back(ar);
      }
    }
  }
  restoreSnapshot(child);
  return true;
}

void DiagramScene::mousePressEvent(QGraphicsSceneMouseEvent* e) {
  if (e->button() == Qt::MiddleButton) {
    const auto itemsUnder = items(e->scenePos());
    for (QGraphicsItem* it : itemsUnder) {
      if (auto* b = qgraphicsitem_cast<BlockItem*>(it)) {
        if (goDownIntoBlock(b)) {
          e->accept();
          return;
        }
      }
    }
    if (goUp()) {
      e->accept();
      return;
    }
  }

  if (e->button() == Qt::LeftButton) {
    if (e->modifiers().testFlag(Qt::ControlModifier)) {
      if (tryBranchAtArrow(e->scenePos())) {
        e->accept();
        return;
      }
    }
    // если кликнули на порт — начинаем протягивание стрелки
    if (tryStartArrowDrag(e->scenePos(), e->modifiers())) {
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
    const QRectF r = m_contentRect.isNull() ? sceneRect() : m_contentRect;
    if (r.contains(e->scenePos())) {
      createBlockAt(e->scenePos());
      e->accept();
      return;
    }
  }

  QGraphicsScene::mousePressEvent(e);
}

void DiagramScene::mouseMoveEvent(QGraphicsSceneMouseEvent* e) {
  if (m_dragInterfaceStub && (e->buttons() & Qt::LeftButton)) {
    if (auto edge = m_dragInterfaceStub->interfaceEdge()) {
      const QRectF r = m_contentRect.isNull() ? sceneRect() : m_contentRect;
      QPointF p = e->scenePos();
      switch (edge->port) {
        case BlockItem::Port::Input:
          p.setX(r.left());
          p.setY(std::clamp(p.y(), r.top(), r.bottom()));
          break;
        case BlockItem::Port::Output:
          p.setX(r.right());
          p.setY(std::clamp(p.y(), r.top(), r.bottom()));
          break;
        case BlockItem::Port::Control:
          p.setY(r.top());
          p.setX(std::clamp(p.x(), r.left(), r.right()));
          break;
        case BlockItem::Port::Mechanism:
          p.setY(r.bottom());
          p.setX(std::clamp(p.x(), r.left(), r.right()));
          break;
      }
      ArrowItem::Endpoint ep = *edge;
      ep.scenePos = p;
      m_dragInterfaceStub->setInterfaceStub(ep, m_dragInterfaceStub->label());
    }
    e->accept();
    return;
  }
  if (m_dragArrow) {
    m_dragArrow->setTempEndPoint(e->scenePos());
    e->accept();
    return;
  }
  QGraphicsScene::mouseMoveEvent(e);
}

void DiagramScene::mouseReleaseEvent(QGraphicsSceneMouseEvent* e) {
  if (m_dragInterfaceStub && e->button() == Qt::LeftButton) {
    m_dragInterfaceStub = nullptr;
    pushSnapshot();
    e->accept();
    return;
  }
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
  purgeBrokenCallMechanisms();
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
  if ((e->key() == Qt::Key_Down || e->key() == Qt::Key_PageDown) && e->modifiers().testFlag(Qt::ControlModifier)) {
    if (goDownIntoSelected()) {
      e->accept();
      return;
    }
  }
  if ((e->key() == Qt::Key_Up || e->key() == Qt::Key_PageUp) && e->modifiers().testFlag(Qt::ControlModifier)) {
    if (goUp()) {
      e->accept();
      return;
    }
  }
  QGraphicsScene::keyPressEvent(e);
}

bool DiagramScene::hasCallMechanism(const BlockItem* target) const {
  if (!target) return false;
  const auto all = items();
  for (QGraphicsItem* it : all) {
    if (auto* a = qgraphicsitem_cast<ArrowItem*>(it)) {
      if (!a->isCallMechanism()) continue;
      const auto from = a->from();
      if (from.block && from.block == target) {
        return true;
      }
    }
  }
  return false;
}

bool DiagramScene::startCallMechanism(BlockItem* origin, BlockItem* refBlock, const QString& label) {
  if (!origin || !refBlock) return false;
  if (origin == refBlock) return false;
  if (hasCallMechanism(origin)) return false;
  cancelCurrentDrag();
  auto* a = new ArrowItem();
  addItem(a);
  ArrowItem::Endpoint from;
  from.scenePos = origin->portScenePos(BlockItem::Port::Mechanism) + QPointF(0, 40);
  from.port = BlockItem::Port::Mechanism;
  ArrowItem::Endpoint to;
  to.block = origin;
  to.port = BlockItem::Port::Mechanism;
  a->setFrom(from);
  a->setTo(to);
  a->setCallMechanism(true);
  a->setCallRefId(refBlock->id());
  a->setLabel(label);
  a->setLabelHidden(false);
  a->setLabelLocked(true);
  a->setLabelInherited(false);
  a->finalize();
  pushSnapshot();
  return true;
}

void DiagramScene::cancelCurrentDrag() {
  if (m_dragArrow) {
    if (m_dragArrow->isInterface()) {
      m_dragArrow->resetInterfaceStub();
    } else {
      removeItem(m_dragArrow);
      delete m_dragArrow;
    }
    m_dragArrow = nullptr;
    m_dragFromBlock.clear();
    m_dragFromJunction.clear();
  }
  m_dragInterfaceStub = nullptr;
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

  QVector<ArrowItem*> arrowsToDelete;
  QVector<QGraphicsItem*> othersToDelete;
  for (QGraphicsItem* it : toDelete) {
    if (it->type() == ArrowItem::Type) {
      if (auto* a = static_cast<ArrowItem*>(it)) arrowsToDelete.push_back(a);
    } else {
      othersToDelete.push_back(it);
    }
  }
  for (ArrowItem* a : arrowsToDelete) {
    if (a->isInterface()) {
      a->resetInterfaceStub();
    } else {
      delete a;
    }
  }
  for (QGraphicsItem* it : othersToDelete) {
    delete it;
  }

  if (!toDelete.isEmpty()) {
    purgeBrokenCallMechanisms();
    pushSnapshot();
  }
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

bool DiagramScene::childHasContent(int blockId) const {
  const QString key = childKeyFor(blockId);
  if (!m_children.contains(key)) return false;
  const auto& snap = m_children.value(key);
  if (!snap.blocks.isEmpty()) return true;
  if (!snap.junctions.isEmpty()) return true;
  for (const auto& ar : snap.arrows) {
    if (!(ar.isInterface && ar.isInterfaceStub)) return true;
  }
  return false;
}

QString DiagramScene::currentPathKey() const {
  QStringList parts;
  for (const auto& node : m_hierarchy) {
    if (node.blockId >= 0) parts.push_back(QString::number(node.blockId));
  }
  if (m_currentBlockId >= 0) parts.push_back(QString::number(m_currentBlockId));
  return parts.join('/');
}

QString DiagramScene::childKeyFor(int blockId) const {
  const QString base = currentPathKey();
  if (base.isEmpty()) return QString::number(blockId);
  return base + "/" + QString::number(blockId);
}

QString DiagramScene::assignNumber(BlockItem* b) {
  const QString prefix = m_currentPrefix;
  const auto all = items();

  if (prefix.isEmpty()) {
    int rootCount = 0;
    for (QGraphicsItem* it : all) {
      auto* blk = qgraphicsitem_cast<BlockItem*>(it);
      if (!blk || blk == b) continue;
      const QString num = blk->number();
      if (num.isEmpty() || num.contains('.')) continue;
      rootCount++;
    }
    return "A" + QString("0").repeated(rootCount + 1);
  }

  if (prefix == "A0") {
    int maxIdx = 0;
    for (QGraphicsItem* it : all) {
      auto* blk = qgraphicsitem_cast<BlockItem*>(it);
      if (!blk || blk == b) continue;
      const QString num = blk->number();
      if (num.startsWith("A") && !num.contains('.')) {
        bool ok = false;
        int idx = num.mid(1).toInt(&ok);
        if (ok) maxIdx = std::max(maxIdx, idx);
      }
    }
    int next = std::max(1, maxIdx + 1);
    return "A" + QString::number(next);
  }

  int maxIdx = 0;
  const QString want = prefix + ".";
  for (QGraphicsItem* it : all) {
    auto* blk = qgraphicsitem_cast<BlockItem*>(it);
    if (!blk || blk == b) continue;
    const QString num = blk->number();
    if (num.startsWith(want)) {
      bool ok = false;
      int idx = num.mid(want.size()).toInt(&ok);
      if (ok) maxIdx = std::max(maxIdx, idx);
    }
  }
  return prefix + "." + QString::number(maxIdx + 1);
}

void DiagramScene::propagateLabelFrom(ArrowItem* root) {
  if (!root || !root->scene()) return;
  ArrowItem* source = root->labelSourceRoot();
  const QString lbl = source ? source->label() : QString();
  for (QGraphicsItem* it : root->scene()->items()) {
    auto* a = qgraphicsitem_cast<ArrowItem*>(it);
    if (!a || a == source) continue;
    if (!a->isLabelInherited()) continue;
    if (a->labelSourceRoot() == source) {
      a->setLabel(lbl);
      a->setLabelHidden(!lbl.isEmpty());
    }
  }
}

void DiagramScene::resetHistory(const Snapshot& base) {
  m_history.clear();
  m_history.push_back(base);
  m_historyIndex = 0;
}

QJsonObject endpointToJson(const DiagramScene::Snapshot::Endpoint& ep) {
  QJsonObject o;
  o["kind"] = int(ep.kind);
  o["id"] = ep.id;
  o["port"] = int(ep.port);
  if (ep.localPos) {
    o["localPos"] = QJsonArray{ep.localPos->x(), ep.localPos->y()};
  }
  if (ep.scenePos) {
    o["scenePos"] = QJsonArray{ep.scenePos->x(), ep.scenePos->y()};
  }
  return o;
}

DiagramScene::Snapshot::Endpoint endpointFromJson(const QJsonObject& o) {
  DiagramScene::Snapshot::Endpoint ep;
  ep.kind = DiagramScene::Snapshot::Endpoint::Kind(o.value("kind").toInt());
  ep.id = o.value("id").toInt(-1);
  ep.port = BlockItem::Port(o.value("port").toInt(int(BlockItem::Port::Input)));
  if (o.contains("localPos")) {
    const auto arr = o.value("localPos").toArray();
    if (arr.size() == 2) ep.localPos = QPointF(arr[0].toDouble(), arr[1].toDouble());
  }
  if (o.contains("scenePos")) {
    const auto arr = o.value("scenePos").toArray();
    if (arr.size() == 2) ep.scenePos = QPointF(arr[0].toDouble(), arr[1].toDouble());
  }
  return ep;
}

QVariantMap DiagramScene::exportToVariant() {
  auto snapshotToVariant = [](const Snapshot& snap) -> QVariantMap {
    QJsonObject root;
    QJsonArray blocks;
    for (const auto& b : snap.blocks) {
      QJsonObject o;
      o["id"] = b.id;
      o["title"] = b.title;
      o["pos"] = QJsonArray{b.pos.x(), b.pos.y()};
      o["hasDecomp"] = b.hasDecomp;
      o["number"] = b.number;
      if (b.price.has_value()) o["price"] = *b.price;
      if (b.color.has_value()) o["color"] = *b.color;
      blocks.append(o);
    }
    root["blocks"] = blocks;

    QJsonArray junctions;
    for (const auto& j : snap.junctions) {
      QJsonObject o;
      o["id"] = j.id;
      o["pos"] = QJsonArray{j.pos.x(), j.pos.y()};
      junctions.append(o);
    }
    root["junctions"] = junctions;

    QJsonArray arrows;
    for (const auto& a : snap.arrows) {
      QJsonObject o;
      o["from"] = endpointToJson(a.from);
      o["to"] = endpointToJson(a.to);
      o["bend"] = a.bend;
      o["top"] = a.top;
      o["bottom"] = a.bottom;
      o["label"] = a.label;
      o["labelOffset"] = QJsonArray{a.labelOffset.x(), a.labelOffset.y()};
      o["labelHidden"] = a.labelHidden;
      o["labelInherited"] = a.labelInherited;
      o["isInterface"] = a.isInterface;
      o["isInterfaceStub"] = a.isInterfaceStub;
      o["labelLocked"] = a.labelLocked;
      o["interfaceEdge"] = endpointToJson(a.interfaceEdge);
      o["callMechanism"] = a.callMechanism;
      o["callRefId"] = a.callRefId;
      if (a.color.has_value()) o["color"] = *a.color;
      arrows.append(o);
    }
    root["arrows"] = arrows;
    return root.toVariantMap();
  };

  if (m_currentBlockId >= 0) {
    m_children[currentPathKey()] = captureSnapshot();
  }

  Snapshot rootSnap = captureSnapshot();
  if (!m_hierarchy.isEmpty()) {
    rootSnap = m_hierarchy.first().snapshot;
  }

  QVariantMap out;
  out["diagram"] = snapshotToVariant(rootSnap);

  QJsonObject children;
  for (auto it = m_children.cbegin(); it != m_children.cend(); ++it) {
    children[it.key()] = QJsonObject::fromVariantMap(snapshotToVariant(it.value()));
  }
  out["children"] = children.toVariantMap();
  return out;
}

bool DiagramScene::importFromVariant(const QVariantMap& map) {
  auto toSnap = [](const QVariantMap& vm, DiagramScene* self) -> std::optional<Snapshot> {
    QJsonObject root = QJsonObject::fromVariantMap(vm);
    if (!root.contains("blocks") || !root.contains("arrows")) return std::nullopt;
    Snapshot snap;
    for (const auto& vb : root.value("blocks").toArray()) {
      const auto o = vb.toObject();
    const auto posArr = o.value("pos").toArray();
      if (posArr.size() != 2) continue;
      DiagramScene::Snapshot::Block blk;
      blk.id = o.value("id").toInt();
      blk.title = o.value("title").toString();
      blk.pos = QPointF(posArr[0].toDouble(), posArr[1].toDouble());
      blk.hasDecomp = o.value("hasDecomp").toBool(false);
      blk.number = o.value("number").toString();
      if (o.contains("price") && !o.value("price").isNull()) {
        blk.price = o.value("price").toDouble();
      }
      const QString blkColor = o.value("color").toString();
      if (!blkColor.isEmpty()) blk.color = blkColor;
      snap.blocks.push_back(blk);
    }
    for (const auto& vj : root.value("junctions").toArray()) {
      const auto o = vj.toObject();
      const auto posArr = o.value("pos").toArray();
      if (posArr.size() != 2) continue;
      snap.junctions.push_back({o.value("id").toInt(), QPointF(posArr[0].toDouble(), posArr[1].toDouble())});
    }
    for (const auto& va : root.value("arrows").toArray()) {
      const auto o = va.toObject();
      Snapshot::Arrow ar;
      ar.from = endpointFromJson(o.value("from").toObject());
      ar.to = endpointFromJson(o.value("to").toObject());
      ar.bend = o.value("bend").toDouble();
      ar.top = o.value("top").toDouble();
      ar.bottom = o.value("bottom").toDouble();
      ar.label = o.value("label").toString();
      const auto lo = o.value("labelOffset").toArray();
      if (lo.size() == 2) ar.labelOffset = QPointF(lo[0].toDouble(), lo[1].toDouble());
      ar.labelHidden = o.value("labelHidden").toBool(false);
      ar.labelInherited = o.value("labelInherited").toBool(false);
      ar.isInterface = o.value("isInterface").toBool(false);
      ar.isInterfaceStub = o.value("isInterfaceStub").toBool(false);
      ar.labelLocked = o.value("labelLocked").toBool(false);
      ar.interfaceEdge = endpointFromJson(o.value("interfaceEdge").toObject());
      ar.callMechanism = o.value("callMechanism").toBool(false);
      ar.callRefId = o.value("callRefId").toInt(-1);
      const QString arrowColor = o.value("color").toString();
      if (!arrowColor.isEmpty()) ar.color = arrowColor;
      snap.arrows.push_back(ar);
    }
    return snap;
  };

  auto diagIt = map.find("diagram");
  if (diagIt == map.end()) return false;
  auto maybeRoot = toSnap(diagIt.value().toMap(), this);
  if (!maybeRoot) return false;
  m_children.clear();
  auto childrenIt = map.find("children");
  if (childrenIt != map.end()) {
    const QVariantMap childrenMap = childrenIt.value().toMap();
    for (auto it = childrenMap.cbegin(); it != childrenMap.cend(); ++it) {
      auto maybeChild = toSnap(it.value().toMap(), this);
      if (maybeChild) m_children[it.key()] = *maybeChild;
    }
  }

  m_hierarchy.clear();
  m_currentPrefix.clear();
  m_currentBlockId = -1;
  restoreSnapshot(*maybeRoot);
  const int currentId = QJsonObject::fromVariantMap(diagIt.value().toMap()).value("currentBlockId").toInt(-1);
  m_currentBlockId = currentId;
  return true;
}

void DiagramScene::ensureFrame() {
  bool exists = false;
  for (QGraphicsItem* it : items()) {
    if (it->data(0).toString() == QStringLiteral("static-frame")) {
      exists = true;
      if (auto* rect = qgraphicsitem_cast<QGraphicsRectItem*>(it)) {
        rect->setRect(m_contentRect.isNull() ? sceneRect() : m_contentRect);
        const bool darkMode = m_meta.value("resolvedDarkMode",
                                           m_meta.value("darkMode", false)).toBool();
        const QColor frameColor = darkMode ? QColor(220, 220, 220) : QColor(80, 80, 80);
        rect->setPen(QPen(frameColor, 1.5, Qt::DashLine));
      }
      break;
    }
  }
  if (!exists) {
    const bool darkMode = m_meta.value("resolvedDarkMode",
                                       m_meta.value("darkMode", false)).toBool();
    const QColor frameColor = darkMode ? QColor(220, 220, 220) : QColor(80, 80, 80);
    auto* frame = addRect(m_contentRect.isNull() ? sceneRect() : m_contentRect,
                          QPen(frameColor, 1.5, Qt::DashLine),
                          Qt::NoBrush);
    frame->setZValue(-5);
    frame->setEnabled(false);
    frame->setAcceptedMouseButtons(Qt::NoButton);
    frame->setData(0, QVariant(QStringLiteral("static-frame")));
  }
}

void DiagramScene::ensureHeaderFooter() {
  if (!m_headerFooter) {
    m_headerFooter = new HeaderFooterItem(this);
    addItem(m_headerFooter);
  }
  m_headerFooter->setPageRect(sceneRect());
  m_headerFooter->setMeta(m_meta);
}
