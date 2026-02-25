#include "MainWindow.h"
#include "items/DiagramScene.h"

#include <QGraphicsView>
#include <QToolBar>
#include <QAction>
#include <QStatusBar>
#include <QDialog>
#include <QTabWidget>
#include <QVBoxLayout>
#include <QFormLayout>
#include <QLineEdit>
#include <QComboBox>
#include <QCheckBox>
#include <QLabel>
#include <QDialogButtonBox>
#include <QLocale>
#include <QTimer>
#include <QMenuBar>
#include <QFileDialog>
#include <QFile>
#include <QMessageBox>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <QFileInfo>
#include <QPointF>
#include <QVariantMap>
#include <QPushButton>
#include <QRegularExpression>
#include <QSignalBlocker>
#include <QCloseEvent>
#include <QInputDialog>
#include <QShortcut>
#include <QColorDialog>
#include <QApplication>
#include <QPalette>
#include <QStyleHints>
#include <cmath>
#include <QGestureEvent>
#include <QPinchGesture>
#include <QNativeGestureEvent>
#include <QPdfWriter>
#include <QPageSize>
#include <QPageLayout>
#include <QPainter>
#include "items/BlockItem.h"

static const char* kDiagramFileFilter = "IDEF0 Diagram (*.idef0);;JSON Diagram (*.json)";
static const char* kPdfFileFilter = "PDF (*.pdf)";

static QPageSize::PageSizeId pageSizeFromMeta(const QVariantMap& meta) {
  const QString size = meta.value("pageSize", "A4").toString();
  if (size == "A1") return QPageSize::A1;
  if (size == "A2") return QPageSize::A2;
  if (size == "A3") return QPageSize::A3;
  return QPageSize::A4;
}

static QPageLayout::Orientation pageOrientationFromMeta(const QVariantMap& meta) {
  const QString orient = meta.value("pageOrientation").toString();
  if (orient == QObject::tr("Portrait") || orient == "Portrait") {
    return QPageLayout::Portrait;
  }
  return QPageLayout::Landscape;
}

MainWindow::MainWindow(const QString& startupPath, QWidget* parent)
  : QMainWindow(parent)
{
  setupUi();
  setupActions();
  statusBar()->showMessage(tr("RMB: add block. LMB on port: drag arrow. Double click block: rename."));
  if (!startupPath.isEmpty()) {
    if (!loadDiagramFromPath(startupPath)) {
      QTimer::singleShot(0, this, &QWidget::close);
      return;
    }
  } else if (!promptStartup()) {
    QTimer::singleShot(0, this, &QWidget::close);
    return;
  }
  markDirty(false);
}

void MainWindow::setupUi() {
  if (!m_scene) {
    m_scene = new DiagramScene(this);
    connect(m_scene, &DiagramScene::changed, this, [this]{ markDirty(true); });
    connect(m_scene, &DiagramScene::metaChanged, this, [this](const QVariantMap& meta){
      m_welcomeState = meta;
      markDirty(true);
    });
    m_scene->applyMeta(m_welcomeState);
  }
  m_view = new QGraphicsView(m_scene, this);
  m_view->setRenderHint(QPainter::Antialiasing, true);
  m_view->setDragMode(QGraphicsView::RubberBandDrag);
  // Полное обновление избавляет от графических артефактов во время drag временной стрелки.
  m_view->setViewportUpdateMode(QGraphicsView::FullViewportUpdate);
  m_view->viewport()->grabGesture(Qt::PinchGesture);
  m_view->viewport()->installEventFilter(this);

  setCentralWidget(m_view);
}

void MainWindow::setupActions() {
  auto* fileMenu = menuBar()->addMenu(tr("&File"));
  auto* actNew = new QAction(tr("New"), this);
  auto* actOpen = new QAction(tr("Open…"), this);
  auto* actSave = new QAction(tr("Save"), this);
  auto* actSaveAs = new QAction(tr("Save As…"), this);
  auto* actExportPdf = new QAction(tr("Export to PDF…"), this);
  actNew->setShortcut(QKeySequence::New);
  actOpen->setShortcut(QKeySequence::Open);
  actSave->setShortcut(QKeySequence::Save);
  actSaveAs->setShortcut(QKeySequence::SaveAs);
  fileMenu->addAction(actNew);
  fileMenu->addAction(actOpen);
  fileMenu->addSeparator();
  fileMenu->addAction(actSave);
  fileMenu->addAction(actSaveAs);
  fileMenu->addSeparator();
  fileMenu->addAction(actExportPdf);

  auto* editMenu = menuBar()->addMenu(tr("&Edit"));
  auto* actSwapNums = new QAction(tr("Swap numbers…"), this);
  editMenu->addAction(actSwapNums);

  auto* viewMenu = menuBar()->addMenu(tr("&View"));
  auto* actFit = new QAction(tr("Fit"), this);
  actFit->setShortcut(Qt::Key_F);
  connect(actFit, &QAction::triggered, this, [this]{
    m_view->fitInView(m_scene->itemsBoundingRect().adjusted(-50,-50,50,50), Qt::KeepAspectRatio);
  });
  viewMenu->addAction(actFit);

  auto* actZoomIn = new QAction(tr("Scale +"), this);
  actZoomIn->setShortcut(QKeySequence::ZoomIn);
  connect(actZoomIn, &QAction::triggered, this, [this]{
    m_view->scale(1.1, 1.1);
  });
  viewMenu->addAction(actZoomIn);

  auto* actZoomOut = new QAction(tr("Scale -"), this);
  actZoomOut->setShortcut(QKeySequence::ZoomOut);
  connect(actZoomOut, &QAction::triggered, this, [this]{
    m_view->scale(1/1.1, 1/1.1);
  });
  viewMenu->addAction(actZoomOut);

  m_actColorfulMode = new QAction(tr("Colorful mode"), this);
  m_actColorfulMode->setCheckable(true);
  m_actColorfulMode->setChecked(m_welcomeState.value("colorfulMode", false).toBool());
  viewMenu->addSeparator();
  viewMenu->addAction(m_actColorfulMode);

  m_actDarkMode = new QAction(tr("Dark mode"), this);
  m_actDarkMode->setCheckable(true);
  m_actDarkMode->setChecked(m_welcomeState.value("darkMode", false).toBool());
  viewMenu->addAction(m_actDarkMode);

  m_actFollowSystemTheme = new QAction(tr("Follow system theme"), this);
  m_actFollowSystemTheme->setCheckable(true);
  m_actFollowSystemTheme->setChecked(m_welcomeState.value("followSystemTheme", false).toBool());
  viewMenu->addAction(m_actFollowSystemTheme);

  m_actArrowColor = new QAction(tr("Arrow color..."), this);
  m_actBorderColor = new QAction(tr("Block border color..."), this);
  m_actFontColor = new QAction(tr("Block font color..."), this);
  viewMenu->addAction(m_actArrowColor);
  viewMenu->addAction(m_actBorderColor);
  viewMenu->addAction(m_actFontColor);

  auto ensureDefaultColor = [this](const char* key, const QColor& fallback) {
    QColor c(m_welcomeState.value(key).toString());
    if (!c.isValid()) m_welcomeState[key] = fallback.name();
  };
  ensureDefaultColor("arrowColor", QColor("#2b6ee6"));
  ensureDefaultColor("blockBorderColor", QColor("#0f766e"));
  ensureDefaultColor("blockFontColor", QColor("#991b1b"));

  auto applyModes = [this](bool mark) {
    const bool colorful = m_actColorfulMode->isChecked();
    const bool followSystem = m_actFollowSystemTheme->isChecked();
    const bool dark = m_actDarkMode->isChecked();
    m_welcomeState["colorfulMode"] = colorful;
    m_welcomeState["followSystemTheme"] = followSystem;
    m_welcomeState["darkMode"] = dark;
    m_welcomeState["resolvedDarkMode"] = effectiveDarkMode();

    m_actArrowColor->setEnabled(colorful);
    m_actBorderColor->setEnabled(colorful);
    m_actFontColor->setEnabled(colorful);
    m_actDarkMode->setEnabled(!followSystem);

    applyAppPalette(m_welcomeState.value("resolvedDarkMode").toBool());
    m_scene->applyMeta(m_welcomeState);
    if (mark) markDirty(true);
  };

  connect(m_actColorfulMode, &QAction::toggled, this, [applyModes]{ applyModes(true); });
  connect(m_actDarkMode, &QAction::toggled, this, [applyModes]{ applyModes(true); });
  connect(m_actFollowSystemTheme, &QAction::toggled, this, [applyModes]{ applyModes(true); });
  connect(qApp->styleHints(), &QStyleHints::colorSchemeChanged, this, [applyModes](Qt::ColorScheme){
    applyModes(false);
  });
  connect(m_actArrowColor, &QAction::triggered, this, [this, applyModes]{
    const QColor cur(m_welcomeState.value("arrowColor").toString());
    const QColor chosen = QColorDialog::getColor(cur.isValid() ? cur : QColor("#2b6ee6"), this, tr("Arrow color"));
    if (!chosen.isValid()) return;
    m_welcomeState["arrowColor"] = chosen.name();
    applyModes(true);
  });
  connect(m_actBorderColor, &QAction::triggered, this, [this, applyModes]{
    const QColor cur(m_welcomeState.value("blockBorderColor").toString());
    const QColor chosen = QColorDialog::getColor(cur.isValid() ? cur : QColor("#0f766e"), this, tr("Block border color"));
    if (!chosen.isValid()) return;
    m_welcomeState["blockBorderColor"] = chosen.name();
    applyModes(true);
  });
  connect(m_actFontColor, &QAction::triggered, this, [this, applyModes]{
    const QColor cur(m_welcomeState.value("blockFontColor").toString());
    const QColor chosen = QColorDialog::getColor(cur.isValid() ? cur : QColor("#991b1b"), this, tr("Block font color"));
    if (!chosen.isValid()) return;
    m_welcomeState["blockFontColor"] = chosen.name();
    applyModes(true);
  });
  applyModes(false);

  connect(actSwapNums, &QAction::triggered, this, [this]{
    const auto sel = m_scene->selectedItems();
    if (sel.size() != 1) {
      QMessageBox::information(this, tr("Swap numbers"), tr("Select a single block to swap its number."));
      return;
    }
    auto* blk = qgraphicsitem_cast<BlockItem*>(sel.first());
    if (!blk) {
      QMessageBox::information(this, tr("Swap numbers"), tr("Select a block."));
      return;
    }
    QStringList choices;
    QHash<QString, BlockItem*> lookup;
    for (QGraphicsItem* it : m_scene->items()) {
      auto* other = qgraphicsitem_cast<BlockItem*>(it);
      if (!other || other == blk) continue;
      choices << other->number();
      lookup.insert(other->number(), other);
    }
    choices.removeAll(QString());
    if (choices.isEmpty()) {
      QMessageBox::information(this, tr("Swap numbers"), tr("No other blocks to swap with."));
      return;
    }
    bool ok = false;
    const QString choice = QInputDialog::getItem(this, tr("Swap numbers"), tr("Swap with:"), choices, 0, false, &ok);
    if (!ok || choice.isEmpty()) return;
    BlockItem* other = lookup.value(choice, nullptr);
    if (!other) return;
    const QString numA = blk->number();
    const QString numB = other->number();
    blk->setNumber(numB);
    other->setNumber(numA);
    m_scene->requestSnapshot();
    markDirty(true);
  });

  connect(actNew, &QAction::triggered, this, [this]{ newDiagram(); });
  connect(actOpen, &QAction::triggered, this, [this]{
    const QString path = QFileDialog::getOpenFileName(this, tr("Open diagram"), QString(), tr(kDiagramFileFilter));
    if (!path.isEmpty()) {
      loadDiagramFromPath(path);
    }
  });

  auto saveTo = [this](const QString& path) {
    QString outPath = path;
    if (QFileInfo(outPath).suffix().isEmpty()) {
      outPath += ".idef0";
    }
    const QVariantMap exported = m_scene->exportToVariant();
    QVariantMap root;
    root["diagram"] = exported.value("diagram");
    root["children"] = exported.value("children");
    root["meta"] = m_welcomeState;
    QJsonDocument doc = QJsonDocument::fromVariant(root);
    QFile f(outPath);
    if (!f.open(QIODevice::WriteOnly | QIODevice::Truncate)) {
      QMessageBox::warning(this, tr("Save failed"), tr("Could not open file for writing."));
      return false;
    }
    f.write(doc.toJson(QJsonDocument::Indented));
    f.close();
    m_currentFile = outPath;
    markDirty(false);
    return true;
  };

  connect(actSave, &QAction::triggered, this, [this, saveTo]{
    if (m_currentFile.isEmpty()) {
      const QString path = QFileDialog::getSaveFileName(this, tr("Save diagram"), QString(), tr(kDiagramFileFilter));
      if (!path.isEmpty()) saveTo(path);
    } else {
      saveTo(m_currentFile);
    }
  });
  connect(actSaveAs, &QAction::triggered, this, [this, saveTo]{
    const QString path = QFileDialog::getSaveFileName(this, tr("Save diagram as"), QString(), tr(kDiagramFileFilter));
    if (!path.isEmpty()) saveTo(path);
  });
  connect(actExportPdf, &QAction::triggered, this, [this]{
    QDialog dlg(this);
    dlg.setWindowTitle(tr("Export to PDF"));
    auto* layout = new QVBoxLayout(&dlg);
    auto* form = new QFormLayout();
    auto* scope = new QComboBox(&dlg);
    scope->addItems({tr("Current diagram"), tr("All diagrams")});
    auto* pageNumbers = new QCheckBox(tr("Number pages in footer (NUMBER field)"), &dlg);
    pageNumbers->setChecked(false);
    form->addRow(tr("Export scope:"), scope);
    form->addRow(QString(), pageNumbers);
    layout->addLayout(form);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dlg);
    connect(buttons, &QDialogButtonBox::accepted, &dlg, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, &dlg, &QDialog::reject);
    layout->addWidget(buttons);
    if (dlg.exec() != QDialog::Accepted) return;
    exportPdf(scope->currentIndex() == 1, pageNumbers->isChecked());
  });
}

static QString currencySymbolForLocale(const QLocale& loc) {
  const QString sym = loc.currencySymbol(QLocale::CurrencySymbol);
  if (!sym.isEmpty()) return sym;
  const auto terr = loc.territory();
  if (terr == QLocale::Russia || terr == QLocale::RussianFederation) return QString::fromUtf8("\u20BD");
  return "$";
}

static QString symbolPlacementDefault(const QLocale& loc, const QString& sym) {
  const QString sample = loc.toCurrencyString(123.0);
  if (sample.startsWith(sym)) {
    const QString rest = sample.mid(sym.size());
    return rest.startsWith(' ') ? "? 1" : "?1";
  }
  if (sample.endsWith(sym)) {
    const QString rest = sample.left(sample.size() - sym.size());
    return rest.endsWith(' ') ? "1 ?" : "1?";
  }
  return "?1";
}

bool MainWindow::showWelcome() {
  auto* dlg = new QDialog(this);
  dlg->setWindowTitle(tr("Welcome"));
  auto* tabs = new QTabWidget(dlg);

  // General tab
  auto* general = new QWidget(dlg);
  auto* genForm = new QFormLayout(general);
  auto* author = new QLineEdit(general);
  author->setPlaceholderText(tr("John Doe"));
  author->setText(m_welcomeState.value("author").toString());
  auto* title = new QLineEdit(general);
  title->setPlaceholderText(tr("Main process"));
  title->setText(m_welcomeState.value("title").toString());
  auto* organization = new QLineEdit(general);
  organization->setPlaceholderText(tr("Example.Org Inc."));
  organization->setText(m_welcomeState.value("organization").toString());
  auto* initials = new QLineEdit(general);
  initials->setPlaceholderText(tr("JD"));
  initials->setText(m_welcomeState.value("initials").toString());
  autoInitialsFromAuthor(author, initials);
  connect(author, &QLineEdit::textChanged, this, [this, author, initials]{
    autoInitialsFromAuthor(author, initials);
  });
  genForm->addRow(tr("Title:"), title);
  genForm->addRow(tr("Organization:"), organization);
  genForm->addRow(tr("Author:"), author);
  genForm->addRow(tr("Author's initials:"), initials);
  general->setLayout(genForm);
  tabs->addTab(general, tr("General"));

  // Numbering tab (placeholder)
  auto* numbering = new QWidget(dlg);
  auto* numLayout = new QVBoxLayout(numbering);
  numLayout->addStretch();
  numbering->setLayout(numLayout);
  tabs->addTab(numbering, tr("Numbering"));

  // Display tab (placeholder)
  auto* display = new QWidget(dlg);
  auto* disLayout = new QVBoxLayout(display);
  disLayout->addStretch();
  display->setLayout(disLayout);
  tabs->addTab(display, tr("Display"));

  // ABC Units tab
  auto* units = new QWidget(dlg);
  auto* unitsForm = new QFormLayout(units);
  const QLocale loc;
  const auto terr = loc.territory();
  const QString curSym = (terr == QLocale::Russia || terr == QLocale::RussianFederation)
                           ? QString::fromUtf8("\u20BD")
                           : currencySymbolForLocale(loc);
  auto* currency = new QComboBox(units);
  currency->setEditable(false);
  currency->addItem(curSym);
  if (curSym != "$") currency->addItem("$");
  const QString existingCur = m_welcomeState.value("currency").toString();
  if (!existingCur.isEmpty() && currency->findText(existingCur) >= 0) {
    currency->setCurrentText(existingCur);
  } else {
    currency->setCurrentIndex(0);
  }

  auto* placement = new QComboBox(units);
  placement->addItems({"1?", "?1", "1 ?", "? 1"});
  const QString placementVal = m_welcomeState.value("currencyPlacement").toString();
  if (!placementVal.isEmpty() && placement->findText(placementVal) >= 0) {
    placement->setCurrentText(placementVal);
  } else {
    placement->setCurrentText(symbolPlacementDefault(loc, currency->currentText()));
  }

  auto* timeUnit = new QComboBox(units);
  const QStringList unitsList = {tr("Seconds"), tr("Minutes"), tr("Hours"), tr("Days"), tr("Weeks"), tr("Months"), tr("Years")};
  timeUnit->addItems(unitsList);
  const QString timeVal = m_welcomeState.value("timeUnit").toString();
  timeUnit->setCurrentText(timeVal.isEmpty() ? tr("Days") : timeVal);

  unitsForm->addRow(tr("Currency:"), currency);
  unitsForm->addRow(tr("Symbol placement:"), placement);
  unitsForm->addRow(tr("Time Unit:"), timeUnit);
  units->setLayout(unitsForm);
  tabs->addTab(units, tr("ABC Units"));

  // Page Setup tab
  auto* page = new QWidget(dlg);
  auto* pageForm = new QFormLayout(page);
  auto* sizeCombo = new QComboBox(page);
  sizeCombo->addItems({"A4","A3","A2","A1"});
  const QString sizeVal = m_welcomeState.value("pageSize").toString();
  sizeCombo->setCurrentText(sizeVal.isEmpty() ? "A4" : sizeVal);

  auto* orientCombo = new QComboBox(page);
  orientCombo->addItems({tr("Landscape"),tr("Portrait")});
  const QString orientVal = m_welcomeState.value("pageOrientation").toString();
  orientCombo->setCurrentText(orientVal.isEmpty() ? tr("Landscape") : orientVal);

  auto* headerChk = new QCheckBox(tr("With header"), page);
  headerChk->setChecked(m_welcomeState.value("withHeader", true).toBool());
  auto* footerChk = new QCheckBox(tr("With footer"), page);
  footerChk->setChecked(m_welcomeState.value("withFooter", true).toBool());

  pageForm->addRow(tr("Size:"), sizeCombo);
  pageForm->addRow(tr("Type:"), orientCombo);
  pageForm->addRow(headerChk);
  pageForm->addRow(footerChk);
  page->setLayout(pageForm);
  tabs->addTab(page, tr("Page Setup"));

  auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, dlg);
  connect(buttons, &QDialogButtonBox::accepted, dlg, &QDialog::accept);
  connect(buttons, &QDialogButtonBox::rejected, dlg, &QDialog::reject);

  auto* layout = new QVBoxLayout(dlg);
  layout->addWidget(tabs);
  layout->addWidget(buttons);
  dlg->setLayout(layout);

  if (dlg->exec() != QDialog::Accepted) {
    dlg->deleteLater();
    return false;
  }

  m_welcomeDialog = dlg;
  m_welcomeState["author"] = author->text();
  m_welcomeState["title"] = title->text();
  m_welcomeState["organization"] = organization->text();
  m_welcomeState["initials"] = initials->text();
  m_welcomeState["currency"] = currency->currentText();
  m_welcomeState["currencyPlacement"] = placement->currentText();
  m_welcomeState["timeUnit"] = timeUnit->currentText();
  m_welcomeState["pageSize"] = sizeCombo->currentText();
  m_welcomeState["pageOrientation"] = orientCombo->currentText();
  m_welcomeState["withHeader"] = headerChk->isChecked();
  m_welcomeState["withFooter"] = footerChk->isChecked();

  dlg->deleteLater();
  return true;
}

void MainWindow::resetScene() {
  auto* old = m_scene;
  m_scene = new DiagramScene(this);
  connect(m_scene, &DiagramScene::changed, this, [this]{ markDirty(true); });
  connect(m_scene, &DiagramScene::metaChanged, this, [this](const QVariantMap& meta){
    m_welcomeState = meta;
    markDirty(true);
  });
  m_scene->applyMeta(m_welcomeState);
  if (m_view) m_view->setScene(m_scene);
  if (old) old->deleteLater();
}

void MainWindow::newDiagram() {
  if (!showWelcome()) return;
  resetScene();
  m_scene->createBlockAt(QPointF(-200, -50))->setPos(-300, -150);
  m_scene->createBlockAt(QPointF(200, -50))->setPos(200, -150);
  m_currentFile.clear();
  markDirty(false);
}

bool MainWindow::promptStartup() {
  QMessageBox box(this);
  box.setWindowTitle(tr("Start"));
  box.setText(tr("Start a new diagram or open an existing one?"));
  QPushButton* newBtn = box.addButton(tr("New"), QMessageBox::AcceptRole);
  QPushButton* openBtn = box.addButton(tr("Open…"), QMessageBox::ActionRole);
  QPushButton* cancelBtn = box.addButton(QMessageBox::Cancel);
  box.exec();
  if (box.clickedButton() == cancelBtn) return false;
  if (box.clickedButton() == openBtn) {
    const QString path = QFileDialog::getOpenFileName(this, tr("Open diagram"), QString(), tr(kDiagramFileFilter));
    if (path.isEmpty()) return false;
    return loadDiagramFromPath(path);
  }
  if (!showWelcome()) return false;
  resetScene();
  m_scene->createBlockAt(QPointF(-100, -50));
  m_currentFile.clear();
  markDirty(false);
  return true;
}

bool MainWindow::loadDiagramFromPath(const QString& path) {
  QFile f(path);
  if (!f.open(QIODevice::ReadOnly)) {
    QMessageBox::warning(this, tr("Open failed"), tr("Could not open file for reading."));
    return false;
  }
  const auto doc = QJsonDocument::fromJson(f.readAll());
  f.close();
  if (!doc.isObject()) {
    QMessageBox::warning(this, tr("Open failed"), tr("File format is invalid."));
    return false;
  }

  const QVariantMap root = doc.object().toVariantMap();
  resetScene();
  if (!m_scene->importFromVariant(root)) {
    QMessageBox::warning(this, tr("Open failed"), tr("Diagram data missing or corrupted."));
    return false;
  }
  if (root.contains("meta")) {
    m_welcomeState = root.value("meta").toMap();
  }
  if (m_actColorfulMode && m_actDarkMode && m_actFollowSystemTheme) {
    const QSignalBlocker b1(m_actColorfulMode);
    const QSignalBlocker b2(m_actDarkMode);
    const QSignalBlocker b3(m_actFollowSystemTheme);
    m_actColorfulMode->setChecked(m_welcomeState.value("colorfulMode", false).toBool());
    m_actDarkMode->setChecked(m_welcomeState.value("darkMode", false).toBool());
    m_actFollowSystemTheme->setChecked(m_welcomeState.value("followSystemTheme", false).toBool());
    m_actDarkMode->setEnabled(!m_actFollowSystemTheme->isChecked());
  }
  if (m_actArrowColor && m_actBorderColor && m_actFontColor) {
    const bool colorful = m_welcomeState.value("colorfulMode", false).toBool();
    m_actArrowColor->setEnabled(colorful);
    m_actBorderColor->setEnabled(colorful);
    m_actFontColor->setEnabled(colorful);
  }
  m_welcomeState["resolvedDarkMode"] = effectiveDarkMode();
  applyAppPalette(m_welcomeState.value("resolvedDarkMode").toBool());
  m_scene->applyMeta(m_welcomeState);
  m_currentFile = path;
  markDirty(false);
  return true;
}

bool MainWindow::exportPdf(bool allDiagrams, bool numberPages) {
  QString path = QFileDialog::getSaveFileName(this, tr("Export to PDF"), QString(), tr(kPdfFileFilter));
  if (path.isEmpty()) return false;
  if (QFileInfo(path).suffix().isEmpty()) path += ".pdf";

  QPdfWriter writer(path);
  writer.setPageSize(QPageSize(pageSizeFromMeta(m_welcomeState)));
  writer.setPageOrientation(pageOrientationFromMeta(m_welcomeState));
  writer.setResolution(96);
  writer.setTitle(tr("IDEF0 Diagram Export"));

  QPainter painter(&writer);
  if (!painter.isActive()) {
    QMessageBox::warning(this, tr("Export failed"), tr("Could not initialize PDF painter."));
    return false;
  }
  const QVariantMap originalMeta = m_scene->meta();

  int pageNo = 0;
  auto renderScene = [&](QGraphicsScene* scene, bool newPage) {
    if (newPage) writer.newPage();
    ++pageNo;
    if (auto* ds = qobject_cast<DiagramScene*>(scene)) {
      QVariantMap meta = ds->meta();
      if (numberPages) {
        meta["footerNumberOverride"] = QString::number(pageNo);
      } else {
        meta.remove("footerNumberOverride");
      }
      ds->applyMeta(meta);
    }
    const QRect target = writer.pageLayout().paintRectPixels(writer.resolution());
    scene->render(&painter, QRectF(target), scene->sceneRect(), Qt::KeepAspectRatio);
  };

  if (!allDiagrams) {
    renderScene(m_scene, false);
    m_scene->applyMeta(originalMeta);
    painter.end();
    return true;
  }

  const QVariantMap exported = m_scene->exportToVariant();
  const QVariantMap children = exported.value("children").toMap();
  QVector<QString> keys;
  QSet<QString> visited;
  std::function<void(const QString&, const QVariantMap&)> collectKeys;
  collectKeys = [&](const QString& prefix, const QVariantMap& diagramMap) {
    const QJsonObject root = QJsonObject::fromVariantMap(diagramMap);
    const QJsonArray blocks = root.value("blocks").toArray();
    for (const auto& vb : blocks) {
      const QJsonObject bo = vb.toObject();
      if (!bo.value("hasDecomp").toBool(false)) continue; // dog-eared: no subdiagram export
      const int id = bo.value("id").toInt(-1);
      if (id < 0) continue;
      const QString key = prefix.isEmpty() ? QString::number(id) : (prefix + "/" + QString::number(id));
      if (!children.contains(key) || visited.contains(key)) continue;
      visited.insert(key);
      keys.push_back(key);
      collectKeys(key, children.value(key).toMap());
    }
  };
  collectKeys(QString(), exported.value("diagram").toMap());

  DiagramScene tempScene;
  tempScene.applyMeta(m_welcomeState);

  QVariantMap mapRoot;
  mapRoot["diagram"] = exported.value("diagram");
  mapRoot["children"] = children;
  bool first = true;
  if (tempScene.importFromVariant(mapRoot)) {
    tempScene.applyMeta(m_welcomeState);
    renderScene(&tempScene, false);
    first = false;
  }

  for (const QString& key : keys) {
    QVariantMap mapOne;
    mapOne["diagram"] = children.value(key).toMap();

    QVariantMap relChildren;
    const QString prefix = key + "/";
    for (auto it = children.cbegin(); it != children.cend(); ++it) {
      if (it.key().startsWith(prefix)) {
        relChildren.insert(it.key().mid(prefix.size()), it.value());
      }
    }
    mapOne["children"] = relChildren;

    if (!tempScene.importFromVariant(mapOne)) continue;
    tempScene.applyMeta(m_welcomeState);
    renderScene(&tempScene, !first);
    first = false;
  }

  m_scene->applyMeta(originalMeta);
  painter.end();
  return true;
}

bool MainWindow::effectiveDarkMode() const {
  const bool followSystem = m_welcomeState.value("followSystemTheme", false).toBool();
  if (!followSystem) {
    return m_welcomeState.value("darkMode", false).toBool();
  }
  const Qt::ColorScheme scheme = qApp->styleHints()->colorScheme();
  if (scheme == Qt::ColorScheme::Dark) return true;
  if (scheme == Qt::ColorScheme::Light) return false;
  return m_welcomeState.value("darkMode", false).toBool();
}

void MainWindow::applyAppPalette(bool darkMode) {
  static const QPalette defaultPalette = qApp->palette();
  if (!darkMode) {
    qApp->setPalette(defaultPalette);
    return;
  }
  QPalette p;
  p.setColor(QPalette::Window, QColor(36, 36, 36));
  p.setColor(QPalette::WindowText, QColor(230, 230, 230));
  p.setColor(QPalette::Base, QColor(26, 26, 26));
  p.setColor(QPalette::AlternateBase, QColor(42, 42, 42));
  p.setColor(QPalette::ToolTipBase, QColor(46, 46, 46));
  p.setColor(QPalette::ToolTipText, QColor(230, 230, 230));
  p.setColor(QPalette::Text, QColor(230, 230, 230));
  p.setColor(QPalette::Button, QColor(46, 46, 46));
  p.setColor(QPalette::ButtonText, QColor(230, 230, 230));
  p.setColor(QPalette::BrightText, QColor(255, 90, 90));
  p.setColor(QPalette::Highlight, QColor(75, 110, 180));
  p.setColor(QPalette::HighlightedText, QColor(255, 255, 255));
  qApp->setPalette(p);
}

bool MainWindow::openDiagramPath(const QString& path) {
  return loadDiagramFromPath(path);
}

void MainWindow::closeEvent(QCloseEvent* e) {
  if (!m_dirty) {
    e->accept();
    return;
  }
  QMessageBox box(this);
  box.setWindowTitle(tr("Unsaved changes"));
  box.setText(tr("You have unsaved changes. Quit anyway?"));
  QPushButton* yes = box.addButton(tr("Yes"), QMessageBox::AcceptRole);
  QPushButton* no = box.addButton(tr("No"), QMessageBox::RejectRole);
  box.setDefaultButton(no);
  box.exec();
  if (box.clickedButton() == yes) {
    e->accept();
  } else {
    e->ignore();
  }
}

bool MainWindow::eventFilter(QObject* obj, QEvent* event) {
  if (obj == m_view->viewport()) {
    if (event->type() == QEvent::Gesture) {
      auto* ge = static_cast<QGestureEvent*>(event);
      if (QGesture* g = ge->gesture(Qt::PinchGesture)) {
        auto* pinch = static_cast<QPinchGesture*>(g);
        if (pinch->state() == Qt::GestureStarted) {
          m_pinchHandled = false;
        }
        if (!m_pinchHandled && pinch->state() == Qt::GestureFinished) {
          const qreal scale = pinch->totalScaleFactor();
          const QPointF viewPt = pinch->centerPoint();
          const QPointF scenePt = m_view->mapToScene(viewPt.toPoint());
          if (scale > 1.15) {
            const auto itemsUnder = m_scene->items(scenePt);
            for (QGraphicsItem* it : itemsUnder) {
              if (auto* b = qgraphicsitem_cast<BlockItem*>(it)) {
                if (m_scene->goDownIntoBlock(b)) {
                  m_pinchHandled = true;
                  event->accept();
                  return true;
                }
              }
            }
          } else if (scale < 0.85) {
            if (m_scene->goUp()) {
              m_pinchHandled = true;
              event->accept();
              return true;
            }
          }
        }
      }
    } else if (event->type() == QEvent::NativeGesture) {
      auto* ng = static_cast<QNativeGestureEvent*>(event);
      if (ng->gestureType() == Qt::ZoomNativeGesture && !m_pinchHandled) {
        const qreal delta = ng->value();
        if (std::abs(delta) > 0.25) {
          const QPointF scenePt = m_view->mapToScene(ng->position().toPoint());
          if (delta > 0) {
            const auto itemsUnder = m_scene->items(scenePt);
            for (QGraphicsItem* it : itemsUnder) {
              if (auto* b = qgraphicsitem_cast<BlockItem*>(it)) {
                if (m_scene->goDownIntoBlock(b)) {
                  m_pinchHandled = true;
                  event->accept();
                  return true;
                }
              }
            }
          } else if (delta < 0) {
            if (m_scene->goUp()) {
              m_pinchHandled = true;
              event->accept();
              return true;
            }
          }
        }
      }
    }
  }
  return QMainWindow::eventFilter(obj, event);
}

void MainWindow::autoInitialsFromAuthor(QLineEdit* author, QLineEdit* initials) {
  if (!author || !initials) return;
  const QString currentInit = initials->text();
  if (!initials->isModified() || currentInit.isEmpty()) {
    const auto parts = author->text().split(QRegularExpression("\\s+"), Qt::SkipEmptyParts);
    QString gen;
    for (const auto& p : parts) {
      gen.append(p.left(1).toUpper());
    }
    if (!gen.isEmpty()) {
      QSignalBlocker b(initials);
      initials->setText(gen);
    }
  }
}

void MainWindow::markDirty(bool dirty) {
  m_dirty = dirty;
  updateWindowTitle();
}

void MainWindow::updateWindowTitle() {
  QString name = m_currentFile.isEmpty() ? tr("Untitled") : QFileInfo(m_currentFile).fileName();
  if (m_dirty) name += " *";
  setWindowTitle(name + tr(" — IDEF0 editor"));
}
