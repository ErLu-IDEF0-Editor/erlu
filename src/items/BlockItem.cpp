#include "BlockItem.h"
#include "ArrowItem.h"
#include "items/DiagramScene.h"

#include <algorithm>
#include <limits>
#include <cmath>
#include <QPainter>
#include <QGraphicsSceneMouseEvent>
#include <QDialog>
#include <QDialogButtonBox>
#include <QDoubleSpinBox>
#include <QFormLayout>
#include <QLineEdit>
#include <QPlainTextEdit>
#include <QCheckBox>
#include <QVBoxLayout>
#include <QLocale>
#include <QApplication>

#include "DiagramScene.h"

QString BlockItem::s_currencySymbol = QStringLiteral("$");
QString BlockItem::s_currencyPlacement = QStringLiteral("1?");
QColor BlockItem::s_foregroundColor = QColor(20, 20, 20);
QColor BlockItem::s_backgroundColor = QColor(255, 255, 255);
QColor BlockItem::s_borderColor = QColor(30, 30, 30);
QColor BlockItem::s_fontColor = QColor(20, 20, 20);
QColor BlockItem::s_selectedBackgroundColor = QColor(240, 240, 255);

BlockItem::BlockItem(QString title, QGraphicsItem* parent, int id)
  : QGraphicsObject(parent),
    m_title(std::move(title)),
    m_rect(0, 0, 200, 100),
    m_id(id)
{
  if (m_title.isEmpty()) {
    m_title = tr("Function");
  }
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
  const QColor borderColor = m_customColor.value_or(s_borderColor);
  const QColor fontColor = m_customColor.value_or(s_fontColor);
  const QColor foregroundColor = m_customColor.value_or(s_foregroundColor);
  const QColor selectedBg = m_customColor ? m_customColor->lighter(160) : s_selectedBackgroundColor;
  p->setBrush(isSelected() ? selectedBg : s_backgroundColor);
  p->drawRoundedRect(m_rect, 6, 6);

  // рамка
  QPen pen(borderColor);
  pen.setWidthF(1.5);
  p->setPen(pen);
  p->setBrush(Qt::NoBrush);
  p->drawRoundedRect(m_rect, 6, 6);

  // dog-ear indicator when no decomposition
  if (!m_hasDecomposition) {
    const qreal ear = 12.0;
    QPainterPath fold;
    fold.moveTo(m_rect.topLeft());
    fold.lineTo(m_rect.topLeft() + QPointF(ear, 0));
    fold.lineTo(m_rect.topLeft() + QPointF(0, ear));
    fold.closeSubpath();
    p->setBrush(s_backgroundColor.darker(108));
    p->drawPath(fold);
    p->setPen(QPen(borderColor.lighter(130), 1.0));
    p->drawLine(m_rect.topLeft() + QPointF(ear, 0), m_rect.topLeft() + QPointF(0, ear));
  }

  // заголовок
  p->setPen(fontColor);
  p->drawText(m_rect.adjusted(10, 8, -10, -8), Qt::AlignLeft | Qt::AlignTop, m_title);
  if (!m_number.isEmpty()) {
    QFont f = p->font();
    f.setBold(true);
    p->setFont(f);
    p->setPen(foregroundColor);
    p->drawText(m_rect.adjusted(8, 4, -8, -8), Qt::AlignRight | Qt::AlignBottom, m_number);
  }
  if (m_price.has_value()) {
    p->setPen(foregroundColor);
    p->drawText(m_rect.adjusted(8, 4, -8, -8), Qt::AlignLeft | Qt::AlignBottom, formattedPrice());
  }
}

void BlockItem::setTitle(const QString& t) {
  if (t == m_title) return;
  m_title = t;
  update();
  emit titleChanged(m_title);
}

void BlockItem::setNumber(const QString& n) {
  if (n == m_number) return;
  m_number = n;
  update();
  emit numberChanged(m_number);
}

void BlockItem::setPrice(std::optional<qreal> price) {
  if (m_price == price) return;
  m_price = price;
  update();
}

void BlockItem::setCustomColor(const QColor& color) {
  if (!color.isValid()) return;
  m_customColor = color;
  update();
}

void BlockItem::clearCustomColor() {
  if (!m_customColor) return;
  m_customColor.reset();
  update();
}

void BlockItem::setCurrencyFormat(const QString& symbol, const QString& placement) {
  QString effectiveSymbol = symbol;
  if (effectiveSymbol.isEmpty()) {
    const QLocale loc;
    effectiveSymbol = loc.currencySymbol(QLocale::CurrencySymbol);
    if (effectiveSymbol.isEmpty()) {
      const auto terr = loc.territory();
      if (terr == QLocale::Russia || terr == QLocale::RussianFederation) {
        effectiveSymbol = QString::fromUtf8("\u20BD");
      } else {
        effectiveSymbol = QStringLiteral("$");
      }
    }
  }
  s_currencySymbol = effectiveSymbol;
  s_currencyPlacement = placement.isEmpty() ? QStringLiteral("1?") : placement;
}

void BlockItem::setVisualTheme(const QColor& foreground,
                               const QColor& background,
                               const QColor& border,
                               const QColor& font,
                               const QColor& selectedBackground) {
  s_foregroundColor = foreground;
  s_backgroundColor = background;
  s_borderColor = border;
  s_fontColor = font;
  s_selectedBackgroundColor = selectedBackground;
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
    // уведомим стрелки, что нужно пересчитать геометрию
    for (auto& a : m_arrows) {
      if (a) a->updatePath();
    }
    emit geometryChanged();
  }
  return QGraphicsObject::itemChange(change, value);
}

QPair<QString, QString> BlockItem::currencyAffixes() {
  const bool prefix = s_currencyPlacement.startsWith("?");
  const bool spaced = s_currencyPlacement.contains(' ');
  if (prefix) {
    return {s_currencySymbol + (spaced ? " " : ""), QString()};
  }
  return {QString(), (spaced ? " " : "") + s_currencySymbol};
}

QString BlockItem::formattedPrice() const {
  if (!m_price.has_value()) return QString();
  const auto affixes = currencyAffixes();
  const QString value = QLocale().toString(*m_price, 'f', 2);
  return affixes.first + value + affixes.second;
}

void BlockItem::mouseDoubleClickEvent(QGraphicsSceneMouseEvent* e) {
  auto* dlg = new QDialog(QApplication::activeWindow());
  dlg->setWindowTitle(tr("Edit block"));

  auto* layout = new QVBoxLayout(dlg);
  auto* form = new QFormLayout();
  auto* titleEdit = new QPlainTextEdit(dlg);
  titleEdit->setPlainText(m_title);
  titleEdit->setTabChangesFocus(true);
  titleEdit->setMinimumHeight(72);
  form->addRow(tr("Title:"), titleEdit);

  auto affixes = currencyAffixes();
  auto* priceSpin = new QDoubleSpinBox(dlg);
  priceSpin->setRange(0.0, 1e9);
  priceSpin->setDecimals(2);
  priceSpin->setPrefix(affixes.first);
  priceSpin->setSuffix(affixes.second);
  priceSpin->setValue(m_price.value_or(0.0));

  auto* priceToggle = new QCheckBox(tr("Set price"), dlg);
  priceToggle->setChecked(m_price.has_value());
  priceSpin->setEnabled(m_price.has_value());
  connect(priceToggle, &QCheckBox::toggled, priceSpin, &QWidget::setEnabled);

  form->addRow(tr("Price:"), priceSpin);
  form->addRow(QString(), priceToggle);
  layout->addLayout(form);

  auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, dlg);
  connect(buttons, &QDialogButtonBox::accepted, dlg, &QDialog::accept);
  connect(buttons, &QDialogButtonBox::rejected, dlg, &QDialog::reject);
  layout->addWidget(buttons);

  bool changed = false;
  if (dlg->exec() == QDialog::Accepted) {
    QString newTitle = titleEdit->toPlainText();
    newTitle.replace('\r', "");
    if (m_title != newTitle) {
      setTitle(newTitle);
      changed = true;
    }
    std::optional<qreal> newPrice;
    if (priceToggle->isChecked()) newPrice = priceSpin->value();
    if (m_price != newPrice) {
      setPrice(newPrice);
      changed = true;
    }
    if (changed) {
      if (auto* ds = qobject_cast<DiagramScene*>(scene())) {
        ds->requestSnapshot();
      }
    }
  }
  dlg->deleteLater();
  QGraphicsObject::mouseDoubleClickEvent(e);
}
