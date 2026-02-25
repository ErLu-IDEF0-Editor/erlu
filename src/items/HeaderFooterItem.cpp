#include "HeaderFooterItem.h"
#include "items/DiagramScene.h"

#include <QPainter>
#include <QDialog>
#include <QFormLayout>
#include <QLineEdit>
#include <QDialogButtonBox>
#include <QCheckBox>
#include <QRadioButton>
#include <QButtonGroup>
#include <QVBoxLayout>
#include <QGraphicsSceneMouseEvent>

HeaderFooterItem::HeaderFooterItem(DiagramScene* scene)
  : QGraphicsObject(nullptr)
  , m_scene(scene)
{
  setZValue(-4);
  setAcceptedMouseButtons(Qt::LeftButton);
  setFlag(QGraphicsItem::ItemIsSelectable, false);
}

void HeaderFooterItem::setMeta(const QVariantMap& meta) {
  m_meta = meta;
  update();
}

void HeaderFooterItem::setPageRect(const QRectF& rect) {
  if (m_pageRect == rect) return;
  prepareGeometryChange();
  m_pageRect = rect;
  update();
}

void HeaderFooterItem::setShowHeaderFooter(bool showHeader, bool showFooter) {
  m_showHeader = showHeader;
  m_showFooter = showFooter;
  update();
}

QRectF HeaderFooterItem::boundingRect() const {
  return m_pageRect;
}

QString HeaderFooterItem::metaText(const QString& key, const QString& fallback) const {
  const auto val = m_meta.value(key);
  if (val.isNull()) return fallback;
  return val.toString();
}

bool HeaderFooterItem::metaBool(const QString& key, bool fallback) const {
  if (!m_meta.contains(key)) return fallback;
  return m_meta.value(key).toBool();
}

bool HeaderFooterItem::hitHeaderFooter(const QPointF& scenePos) const {
  if (!m_pageRect.contains(scenePos)) return false;
  const qreal headerH = m_pageRect.height() * 0.12;
  const qreal footerH = m_pageRect.height() * 0.08;
  const qreal top = m_pageRect.top();
  const qreal bottom = m_pageRect.bottom();
  if (m_showHeader && scenePos.y() <= top + headerH) return true;
  if (m_showFooter && scenePos.y() >= bottom - footerH) return true;
  return false;
}

void HeaderFooterItem::paint(QPainter* painter, const QStyleOptionGraphicsItem*, QWidget*) {
  if (m_pageRect.isNull()) return;

  painter->save();
  painter->setRenderHint(QPainter::Antialiasing, true);

  const bool darkMode = metaBool("resolvedDarkMode", metaBool("darkMode", false));
  const QColor fg = darkMode ? QColor(235, 235, 235) : QColor(20, 20, 20);
  const QColor bg = darkMode ? QColor(26, 26, 26) : QColor(255, 255, 255);

  const qreal w = m_pageRect.width();
  const qreal h = m_pageRect.height();
  const qreal headerH = h * 0.12;
  const qreal footerH = h * 0.08;

  QPen linePen(fg, 1.0);
  painter->setPen(linePen);
  painter->setBrush(Qt::NoBrush);

  QFont labelFont = painter->font();
  labelFont.setPointSizeF(labelFont.pointSizeF() - 1);
  painter->setFont(labelFont);

  if (m_showHeader) {
    QRectF headerRect(m_pageRect.left(), m_pageRect.top(), w, headerH);
    painter->drawRect(headerRect);

    const qreal leftW = w * 0.62;
    const qreal rightW = w - leftW;
    const qreal xSplit = headerRect.left() + leftW;
    painter->drawLine(QPointF(xSplit, headerRect.top()), QPointF(xSplit, headerRect.bottom()));

    const qreal rowH = headerH / 4.0;
    for (int i = 1; i < 4; ++i) {
      const qreal y = headerRect.top() + i * rowH;
      painter->drawLine(QPointF(headerRect.left(), y), QPointF(xSplit, y));
    }

    // Left area text
    const QString organization = metaText("organization");
    const QString usedAt = organization.isEmpty() ? metaText("usedAt") : organization;
    const QString author = metaText("author");
    const QString title = metaText("title");
    const QString project = title.isEmpty() ? metaText("project") : title;
    const QString notes = metaText("notes");
    const QString date = metaText("date");
    const QString rev = metaText("rev");

    auto drawRow = [&](int row, const QString& label, const QString& value) {
      QRectF r(headerRect.left() + 6, headerRect.top() + row * rowH + 2, leftW - 12, rowH - 4);
      painter->drawText(r, Qt::AlignLeft | Qt::AlignVCenter, label + (value.isEmpty() ? "" : " " + value));
    };
    drawRow(0, organization.isEmpty() ? tr("USED AT:") : tr("ORGANIZATION:"), usedAt);
    drawRow(1, tr("AUTHOR:"), author);
    drawRow(2, title.isEmpty() ? tr("PROJECT:") : tr("TITLE:"), project);
    drawRow(3, tr("NOTES:"), notes);

    // Date/rev in left area top-right corner
    QRectF dateRect(headerRect.left() + leftW * 0.55, headerRect.top() + 2, leftW * 0.45 - 6, rowH - 4);
    painter->drawText(dateRect, Qt::AlignRight | Qt::AlignVCenter, tr("DATE: ") + date);
    QRectF revRect(headerRect.left() + leftW * 0.55, headerRect.top() + rowH + 2, leftW * 0.45 - 6, rowH - 4);
    painter->drawText(revRect, Qt::AlignRight | Qt::AlignVCenter, tr("REV: ") + rev);

    // Right area split
    QRectF rightRect(xSplit, headerRect.top(), rightW, headerH);
    const qreal statusW = rightW * 0.55;
    const qreal infoW = rightW - statusW;
    const qreal statusX = rightRect.left();
    const qreal infoX = rightRect.left() + statusW;
    painter->drawLine(QPointF(infoX, headerRect.top()), QPointF(infoX, headerRect.bottom()));

    const qreal statusRowH = headerH / 4.0;
    for (int i = 1; i < 4; ++i) {
      const qreal y = headerRect.top() + i * statusRowH;
      painter->drawLine(QPointF(statusX, y), QPointF(infoX, y));
    }

    QString activeState = metaText("state");
    if (activeState.isEmpty()) {
      if (metaBool("publication", false)) activeState = "publication";
      else if (metaBool("recommended", false)) activeState = "recommended";
      else if (metaBool("draft", false)) activeState = "draft";
      else if (metaBool("working", false)) activeState = "working";
    }
    auto drawStatus = [&](int row, const QString& label, const QString& key) {
      QRectF r(statusX, headerRect.top() + row * statusRowH, statusW, statusRowH);

      const qreal box = statusRowH;
      const qreal bx = r.left();
      const qreal by = r.top();
      QRectF boxRect(bx, by, box, box);
      painter->setBrush(activeState == key ? fg : bg);
      painter->drawRect(boxRect);
      painter->setBrush(Qt::NoBrush);

      QRectF textRect = r.adjusted(box + 6, 0, -6, 0);
      painter->drawText(textRect, Qt::AlignLeft | Qt::AlignVCenter, label);
    };
    drawStatus(0, tr("WORKING"), "working");
    drawStatus(1, tr("DRAFT"), "draft");
    drawStatus(2, tr("RECOMMENDED"), "recommended");
    drawStatus(3, tr("PUBLICATION"), "publication");

    // Info column
    const qreal infoRowH = headerH / 3.0;
    for (int i = 1; i < 3; ++i) {
      const qreal y = headerRect.top() + i * infoRowH;
      painter->drawLine(QPointF(infoX, y), QPointF(rightRect.right(), y));
    }
    QRectF readerRect(infoX + 6, headerRect.top() + 2, infoW - 12, infoRowH - 4);
    painter->drawText(readerRect, Qt::AlignLeft | Qt::AlignVCenter, tr("READER: ") + metaText("reader"));
    QRectF readerDateRect(infoX + 6, headerRect.top() + infoRowH + 2, infoW - 12, infoRowH - 4);
    painter->drawText(readerDateRect, Qt::AlignLeft | Qt::AlignVCenter, tr("DATE: ") + metaText("readerDate"));
    QRectF contextRect(infoX + 6, headerRect.top() + 2 * infoRowH + 2, infoW - 12, infoRowH - 4);
    painter->drawText(contextRect, Qt::AlignLeft | Qt::AlignVCenter, tr("CONTEXT: ") + metaText("context"));
  }

  if (m_showFooter) {
    QRectF footerRect(m_pageRect.left(), m_pageRect.bottom() - footerH, w, footerH);
    painter->drawRect(footerRect);

    const qreal leftW = w * 0.2;
    const qreal rightW = w * 0.2;
    const qreal middleW = w - leftW - rightW;
    const qreal x1 = footerRect.left() + leftW;
    const qreal x2 = x1 + middleW;
    painter->drawLine(QPointF(x1, footerRect.top()), QPointF(x1, footerRect.bottom()));
    painter->drawLine(QPointF(x2, footerRect.top()), QPointF(x2, footerRect.bottom()));

    QString nodeValue = metaText("footerNodeOverride");
    if (nodeValue.isEmpty() && m_scene) nodeValue = m_scene->currentNodeLabel();
    if (nodeValue.isEmpty()) nodeValue = metaText("node", "A0");
    QRectF nodeRect(footerRect.left() + 6, footerRect.top() + 2, leftW - 12, footerH - 4);
    painter->drawText(nodeRect, Qt::AlignLeft | Qt::AlignVCenter, tr("NODE: ") + nodeValue);

    QString titleValue = metaText("footerTitleOverride");
    if (titleValue.isEmpty() && m_scene) titleValue = m_scene->currentDiagramTitle();
    if (titleValue.isEmpty()) titleValue = metaText("title");
    QRectF titleRect(x1 + 6, footerRect.top() + 2, middleW - 12, footerH - 4);
    painter->drawText(titleRect, Qt::AlignHCenter | Qt::AlignVCenter, titleValue);

    QString numberValue = metaText("footerNumberOverride");
    if (numberValue.isEmpty()) numberValue = metaText("number");
    QRectF numberRect(x2 + 6, footerRect.top() + 2, rightW - 12, footerH - 4);
    painter->drawText(numberRect, Qt::AlignLeft | Qt::AlignVCenter, tr("NUMBER: ") + numberValue);
  }

  painter->restore();
}

void HeaderFooterItem::mouseDoubleClickEvent(QGraphicsSceneMouseEvent* e) {
  if (!hitHeaderFooter(e->scenePos())) {
    QGraphicsObject::mouseDoubleClickEvent(e);
    return;
  }

  auto* dlg = new QDialog();
  dlg->setWindowTitle(tr("Edit header/footer"));
  auto* layout = new QVBoxLayout(dlg);
  auto* form = new QFormLayout();

  auto* usedAt = new QLineEdit(metaText("usedAt"), dlg);
  auto* author = new QLineEdit(metaText("author"), dlg);
  auto* project = new QLineEdit(metaText("project"), dlg);
  auto* notes = new QLineEdit(metaText("notes"), dlg);
  auto* date = new QLineEdit(metaText("date"), dlg);
  auto* rev = new QLineEdit(metaText("rev"), dlg);
  auto* reader = new QLineEdit(metaText("reader"), dlg);
  auto* readerDate = new QLineEdit(metaText("readerDate"), dlg);
  auto* context = new QLineEdit(metaText("context"), dlg);
  auto* node = new QLineEdit(metaText("node", "A0"), dlg);
  auto* title = new QLineEdit(metaText("title"), dlg);
  auto* number = new QLineEdit(metaText("number"), dlg);

  const QString state = metaText("state");
  auto* working = new QRadioButton(tr("Working"), dlg);
  auto* draft = new QRadioButton(tr("Draft"), dlg);
  auto* recommended = new QRadioButton(tr("Recommended"), dlg);
  auto* publication = new QRadioButton(tr("Publication"), dlg);
  auto* group = new QButtonGroup(dlg);
  group->addButton(working);
  group->addButton(draft);
  group->addButton(recommended);
  group->addButton(publication);
  if (state == "draft") {
    draft->setChecked(true);
  } else if (state == "recommended") {
    recommended->setChecked(true);
  } else if (state == "publication") {
    publication->setChecked(true);
  } else if (state == "working") {
    working->setChecked(true);
  } else {
    // Backward-compatible: map old flags to a single selection.
    if (metaBool("publication", false)) publication->setChecked(true);
    else if (metaBool("recommended", false)) recommended->setChecked(true);
    else if (metaBool("draft", false)) draft->setChecked(true);
    else if (metaBool("working", false)) working->setChecked(true);
  }

  form->addRow(tr("Used at:"), usedAt);
  form->addRow(tr("Author:"), author);
  form->addRow(tr("Project:"), project);
  form->addRow(tr("Notes:"), notes);
  form->addRow(tr("Date:"), date);
  form->addRow(tr("Rev:"), rev);
  form->addRow(tr("Reader:"), reader);
  form->addRow(tr("Reader date:"), readerDate);
  form->addRow(tr("Context:"), context);
  form->addRow(tr("Node:"), node);
  form->addRow(tr("Title:"), title);
  form->addRow(tr("Number:"), number);
  form->addRow(tr("State:"), working);
  form->addRow(QString(), draft);
  form->addRow(QString(), recommended);
  form->addRow(QString(), publication);
  layout->addLayout(form);

  auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, dlg);
  connect(buttons, &QDialogButtonBox::accepted, dlg, &QDialog::accept);
  connect(buttons, &QDialogButtonBox::rejected, dlg, &QDialog::reject);
  layout->addWidget(buttons);

  if (dlg->exec() == QDialog::Accepted) {
    QVariantMap patch;
    patch["usedAt"] = usedAt->text();
    patch["author"] = author->text();
    patch["project"] = project->text();
    patch["notes"] = notes->text();
    patch["date"] = date->text();
    patch["rev"] = rev->text();
    patch["reader"] = reader->text();
    patch["readerDate"] = readerDate->text();
    patch["context"] = context->text();
    patch["node"] = node->text();
    patch["title"] = title->text();
    patch["number"] = number->text();
    QString nextState;
    if (working->isChecked()) nextState = "working";
    else if (draft->isChecked()) nextState = "draft";
    else if (recommended->isChecked()) nextState = "recommended";
    else if (publication->isChecked()) nextState = "publication";
    patch["state"] = nextState;
    patch["working"] = (nextState == "working");
    patch["draft"] = (nextState == "draft");
    patch["recommended"] = (nextState == "recommended");
    patch["publication"] = (nextState == "publication");
    if (m_scene) m_scene->updateMeta(patch);
  }

  dlg->deleteLater();
  QGraphicsObject::mouseDoubleClickEvent(e);
}
