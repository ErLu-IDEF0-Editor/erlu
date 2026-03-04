#include "MainWindow.h"
#include "items/DiagramScene.h"

#include <QGraphicsView>
#include <QToolBar>
#include <QAction>
#include <QStatusBar>
#include <QDialog>
#include <QTabWidget>
#include <QTreeWidget>
#include <QTextEdit>
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
#include <QCoreApplication>
#include <QtGlobal>
#include "items/BlockItem.h"
#include "plugins/PluginManager.h"

static const char* kDiagramFileFilter = QT_TR_NOOP("IDEF0 Diagram (*.idef0);;JSON Diagram (*.json)");
static const char* kPdfFileFilter = QT_TR_NOOP("PDF (*.pdf)");
static const char* kMarkdownFileFilter = QT_TR_NOOP("Markdown (*.md)");

static QPageSize::PageSizeId pageSizeFromMeta(const QVariantMap& meta) {
  const QString size = meta.value("pageSize", "A4").toString();
  if (size == "A1") return QPageSize::A1;
  if (size == "A2") return QPageSize::A2;
  if (size == "A3") return QPageSize::A3;
  return QPageSize::A4;
}

static QPageLayout::Orientation pageOrientationFromMeta(const QVariantMap& meta) {
  const QString orient = meta.value("pageOrientation").toString();
  const QString portrait = QCoreApplication::translate("MainWindow", "Portrait");
  if (orient == portrait || orient == "Portrait") {
    return QPageLayout::Portrait;
  }
  return QPageLayout::Landscape;
}

static QVariantMap stripPerDiagramState(QVariantMap meta) {
  meta.remove("state");
  meta.remove("working");
  meta.remove("draft");
  meta.remove("recommended");
  meta.remove("publication");
  return meta;
}

static QString normalizeCurrencyName(const QString& currency) {
  if (currency == QCoreApplication::translate("MainWindow", "man-hours") || currency == QStringLiteral("man-hours")) {
    return QCoreApplication::translate("MainWindow", "person-hours");
  }
  return currency;
}

static QVariantMap mergeWithDiagramState(QVariantMap globalMeta, const QVariantMap& diagramMeta) {
  for (const char* key : {"state", "working", "draft", "recommended", "publication"}) {
    const QString k = QString::fromLatin1(key);
    if (diagramMeta.contains(k)) globalMeta[k] = diagramMeta.value(k);
  }
  return globalMeta;
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
      m_welcomeState = stripPerDiagramState(meta);
      markDirty(true);
    });
    m_scene->applyMeta(mergeWithDiagramState(m_welcomeState, m_scene->meta()));
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
  auto* actExportMarkdown = new QAction(tr("Export to Markdown…"), this);
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
  fileMenu->addAction(actExportMarkdown);

  auto* editMenu = menuBar()->addMenu(tr("&Edit"));
  auto* toolsMenu = menuBar()->addMenu(tr("&Tools"));
  auto* actSwapNums = new QAction(tr("Swap numbers…"), this);
  auto* actCallMech = new QAction(tr("Call Mechanism add"), this);
  auto* actValidation = new QAction(tr("Validation"), this);
  auto* actCalcPrices = new QAction(tr("Calculate Prices"), this);
  editMenu->addAction(actSwapNums);
  editMenu->addAction(actCallMech);
  toolsMenu->addAction(actValidation);
  toolsMenu->addAction(actCalcPrices);

  auto* viewMenu = menuBar()->addMenu(tr("&View"));
  auto* actFit = new QAction(tr("Fit"), this);
  actFit->setShortcut(Qt::Key_F);
  connect(actFit, &QAction::triggered, this, [this]{
    m_view->fitInView(m_scene->itemsBoundingRect().adjusted(-50,-50,50,50), Qt::KeepAspectRatio);
  });
  viewMenu->addAction(actFit);
  auto* pluginsMenu = menuBar()->addMenu(tr("&Plugins"));

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

  auto* actNodeTree = new QAction(tr("Node tree…"), this);
  connect(actNodeTree, &QAction::triggered, this, [this]{
    showNodeTreeDialog();
  });
  toolsMenu->addAction(actNodeTree);

  m_actDarkMode = new QAction(tr("Dark mode"), this);
  m_actDarkMode->setCheckable(true);
  m_actDarkMode->setChecked(m_welcomeState.value("darkMode", false).toBool());
  viewMenu->addAction(m_actDarkMode);

  m_actFollowSystemTheme = new QAction(tr("Follow system theme"), this);
  m_actFollowSystemTheme->setCheckable(true);
  m_actFollowSystemTheme->setChecked(m_welcomeState.value("followSystemTheme", false).toBool());
  viewMenu->addAction(m_actFollowSystemTheme);

  auto applyModes = [this](bool mark) {
    const bool followSystem = m_actFollowSystemTheme->isChecked();
    const bool dark = m_actDarkMode->isChecked();
    m_welcomeState["followSystemTheme"] = followSystem;
    m_welcomeState["darkMode"] = dark;
    m_welcomeState["resolvedDarkMode"] = effectiveDarkMode();
    m_actDarkMode->setEnabled(!followSystem);
    applyAppPalette(m_welcomeState.value("resolvedDarkMode").toBool());
    m_scene->applyMeta(mergeWithDiagramState(m_welcomeState, m_scene->meta()));
    if (mark) markDirty(true);
  };
  connect(m_actDarkMode, &QAction::toggled, this, [applyModes]{ applyModes(true); });
  connect(m_actFollowSystemTheme, &QAction::toggled, this, [applyModes]{ applyModes(true); });
  connect(qApp->styleHints(), &QStyleHints::colorSchemeChanged, this, [applyModes](Qt::ColorScheme){
    applyModes(false);
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
    root["meta"] = stripPerDiagramState(m_welcomeState);
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
    auto* forceLight = new QCheckBox(tr("Use a light theme regardless of the document theme"), &dlg);
    forceLight->setChecked(true);
    form->addRow(tr("Export scope:"), scope);
    form->addRow(QString(), pageNumbers);
    form->addRow(QString(), forceLight);
    layout->addLayout(form);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dlg);
    connect(buttons, &QDialogButtonBox::accepted, &dlg, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, &dlg, &QDialog::reject);
    layout->addWidget(buttons);
    if (dlg.exec() != QDialog::Accepted) return;
    exportPdf(scope->currentIndex() == 1, pageNumbers->isChecked(), forceLight->isChecked());
  });
  connect(actExportMarkdown, &QAction::triggered, this, [this]{
    exportMarkdownExplanation();
  });

  connect(actCallMech, &QAction::triggered, this, [this]{
    if (!m_scene) return;
    const auto sel = m_scene->selectedItems();
    if (sel.size() != 1) {
      QMessageBox::information(this, tr("Call Mechanism"), tr("Select exactly one block to add a call mechanism."));
      return;
    }
    auto* target = qgraphicsitem_cast<BlockItem*>(sel.first());
    if (!target) {
      QMessageBox::information(this, tr("Call Mechanism"), tr("Select a block to add a call mechanism."));
      return;
    }
    if (m_scene->hasCallMechanism(target)) {
      QMessageBox::information(this, tr("Call Mechanism"), tr("Call mechanism already exists for this block."));
      return;
    }
    struct Option { QString display; BlockItem* block = nullptr; };
    QVector<Option> options;
    for (QGraphicsItem* it : m_scene->items()) {
      auto* b = qgraphicsitem_cast<BlockItem*>(it);
      if (!b) continue;
      const QString n = b->number();
      if (n.isEmpty() || n.compare("A0", Qt::CaseInsensitive) == 0) continue;
      if (b == target) continue;
      Option opt{QStringLiteral("%1 — %2").arg(n, b->title()), b};
      options.push_back(opt);
    }
    std::sort(options.begin(), options.end(), [](const Option& a, const Option& b){ return a.display.localeAwareCompare(b.display) < 0; });
    if (options.isEmpty()) {
      QMessageBox::information(this, tr("Call Mechanism"), tr("No eligible target blocks found."));
      return;
    }
    QStringList displayList;
    for (const auto& opt : options) displayList << opt.display;
    bool ok = false;
    const QString choice = QInputDialog::getItem(this, tr("Call Mechanism"), tr("Choose target block:"), displayList, 0, false, &ok);
    if (!ok || choice.isEmpty()) return;
    BlockItem* refBlock = nullptr;
    for (const auto& opt : options) {
      if (opt.display == choice) { refBlock = opt.block; break; }
    }
    if (!refBlock) return;
    const QString label = QStringLiteral("%1 %2").arg(refBlock->number(), refBlock->title());
    if (!m_scene->startCallMechanism(target, refBlock, label)) {
      QMessageBox::information(this, tr("Call Mechanism"), tr("Call mechanism already exists for this block or selection is invalid."));
      return;
    }
    statusBar()->showMessage(tr("Click bottom frame to place call mechanism arrow. Esc to cancel."), 5000);
  });

  connect(actValidation, &QAction::triggered, this, [this]{
    runValidation();
  });
  connect(actCalcPrices, &QAction::triggered, this, [this]{
    calculatePrices();
  });

  m_pluginManager = new PluginManager(this, pluginsMenu, this);
  m_pluginManager->loadPlugins();
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
  auto* title = new QLineEdit(general);
  title->setPlaceholderText(tr("Main process"));
  title->setText(m_welcomeState.value("title").toString());
  auto* organization = new QLineEdit(general);
  organization->setPlaceholderText(tr("Example.Org Inc."));
  organization->setText(m_welcomeState.value("organization").toString());
  auto* author = new QLineEdit(general);
  author->setPlaceholderText(tr("John Doe"));
  author->setText(m_welcomeState.value("author").toString());
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
  setTabOrder(title, organization);
  setTabOrder(organization, author);
  setTabOrder(author, initials);
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
  currency->setEditable(true);
  currency->addItem(curSym);
  if (curSym != "$") currency->addItem("$");
  currency->addItem(tr("person-hours"));
  const QString existingCur = normalizeCurrencyName(m_welcomeState.value("currency").toString());
  if (!existingCur.isEmpty()) {
    currency->setCurrentText(existingCur);
  } else {
    currency->setCurrentIndex(0);
  }

  auto* placement = new QComboBox(units);
  placement->addItems({"1?", "?1", "1 ?", "? 1"});
  const QString personHours = tr("person-hours");
  const QString placementVal = m_welcomeState.value("currencyPlacement").toString();
  if (!placementVal.isEmpty() && placement->findText(placementVal) >= 0) {
    placement->setCurrentText(placementVal);
  } else {
    placement->setCurrentText(currency->currentText() == personHours
                                ? QStringLiteral("1 ?")
                                : symbolPlacementDefault(loc, currency->currentText()));
  }
  connect(currency, &QComboBox::currentTextChanged, this, [placement, personHours](const QString& cur){
    if (cur == personHours) placement->setCurrentText(QStringLiteral("1 ?"));
  });

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
  m_welcomeState["currency"] = normalizeCurrencyName(currency->currentText());
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
    m_welcomeState = stripPerDiagramState(meta);
    markDirty(true);
  });
  m_scene->applyMeta(mergeWithDiagramState(m_welcomeState, m_scene->meta()));
  if (m_view) m_view->setScene(m_scene);
  if (old) old->deleteLater();
}

void MainWindow::newDiagram() {
  if (!showWelcome()) return;
  resetScene();
  const QRectF bounds = m_scene->contentRect().isNull() ? m_scene->sceneRect() : m_scene->contentRect();
  m_scene->createBlockAt(bounds.center());
  m_currentFile.clear();
  markDirty(false);
  // Keep main window visible and focused after closing modal creation dialogs.
  QTimer::singleShot(0, this, [this]{
    setWindowState(windowState() & ~Qt::WindowMinimized);
    showNormal();
    raise();
    activateWindow();
  });
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
  const QRectF bounds = m_scene->contentRect().isNull() ? m_scene->sceneRect() : m_scene->contentRect();
  m_scene->createBlockAt(bounds.center());
  m_currentFile.clear();
  markDirty(false);
  QTimer::singleShot(0, this, [this]{
    setWindowState(windowState() & ~Qt::WindowMinimized);
    showNormal();
    raise();
    activateWindow();
  });
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
    m_welcomeState = stripPerDiagramState(root.value("meta").toMap());
    m_welcomeState["currency"] = normalizeCurrencyName(m_welcomeState.value("currency").toString());
  }
  if (m_actDarkMode && m_actFollowSystemTheme) {
    const QSignalBlocker b2(m_actDarkMode);
    const QSignalBlocker b3(m_actFollowSystemTheme);
    m_actDarkMode->setChecked(m_welcomeState.value("darkMode", false).toBool());
    m_actFollowSystemTheme->setChecked(m_welcomeState.value("followSystemTheme", false).toBool());
    m_actDarkMode->setEnabled(!m_actFollowSystemTheme->isChecked());
  }
  m_welcomeState["resolvedDarkMode"] = effectiveDarkMode();
  applyAppPalette(m_welcomeState.value("resolvedDarkMode").toBool());
  m_scene->applyMeta(mergeWithDiagramState(m_welcomeState, m_scene->meta()));
  m_currentFile = path;
  markDirty(false);
  return true;
}

bool MainWindow::exportPdf(bool allDiagrams, bool numberPages, bool forceLightTheme) {
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
      if (forceLightTheme) {
        meta["darkMode"] = false;
        meta["resolvedDarkMode"] = false;
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
  tempScene.applyMeta(mergeWithDiagramState(m_welcomeState, tempScene.meta()));

  QVariantMap mapRoot;
  mapRoot["diagram"] = exported.value("diagram");
  mapRoot["children"] = children;
  bool first = true;
  if (tempScene.importFromVariant(mapRoot)) {
    tempScene.applyMeta(mergeWithDiagramState(m_welcomeState, tempScene.meta()));
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
    tempScene.applyMeta(mergeWithDiagramState(m_welcomeState, tempScene.meta()));
    renderScene(&tempScene, !first);
    first = false;
  }

  m_scene->applyMeta(originalMeta);
  painter.end();
  return true;
}

bool MainWindow::exportMarkdownExplanation() {
  if (!m_scene) return false;
  QString path = QFileDialog::getSaveFileName(this, tr("Export to Markdown"), QString(), tr(kMarkdownFileFilter));
  if (path.isEmpty()) return false;
  if (QFileInfo(path).suffix().isEmpty()) path += ".md";

  const QVariantMap exported = m_scene->exportToVariant();
  const QVariantMap rootDiagram = exported.value("diagram").toMap();
  const QVariantMap children = exported.value("children").toMap();

  auto sortedBlocks = [](const QVariantMap& diagram) {
    QVector<QVariantMap> out;
    const QVariantList blocks = diagram.value("blocks").toList();
    out.reserve(blocks.size());
    for (const QVariant& vb : blocks) out.push_back(vb.toMap());
    std::sort(out.begin(), out.end(), [](const QVariantMap& a, const QVariantMap& b) {
      const QString na = a.value("number").toString();
      const QString nb = b.value("number").toString();
      if (na == nb) return a.value("id", -1).toInt() < b.value("id", -1).toInt();
      return na.localeAwareCompare(nb) < 0;
    });
    return out;
  };

  auto addUniqueSorted = [](QStringList& list, const QString& value) {
    const QString v = value.trimmed();
    if (v.isEmpty() || list.contains(v)) return;
    list.push_back(v);
  };

  auto joinOrDash = [this](QStringList list) {
    if (list.isEmpty()) return tr("none");
    std::sort(list.begin(), list.end(), [](const QString& a, const QString& b) {
      return a.localeAwareCompare(b) < 0;
    });
    return list.join(", ");
  };

  struct Ports {
    QStringList inputs;
    QStringList outputs;
    QStringList mechanisms;
    QStringList controls;
  };
  auto collectPorts = [&](const QVariantMap& diagram) {
    Ports p;
    const QVariantList arrows = diagram.value("arrows").toList();
    for (const QVariant& va : arrows) {
      const QVariantMap a = va.toMap();
      const QString label = a.value("label").toString().trimmed();
      if (label.isEmpty()) continue;
      const QVariantMap from = a.value("from").toMap();
      const QVariantMap to = a.value("to").toMap();
      const int fk = from.value("kind").toInt(0);
      const int fp = from.value("port").toInt(0);
      const int tk = to.value("kind").toInt(0);
      const int tp = to.value("port").toInt(0);
      if (fk == 3) {
        if (fp == 0) addUniqueSorted(p.inputs, label);
        if (fp == 1) addUniqueSorted(p.controls, label);
        if (fp == 2) addUniqueSorted(p.outputs, label);
        if (fp == 3) addUniqueSorted(p.mechanisms, label);
      }
      if (tk == 3) {
        if (tp == 0) addUniqueSorted(p.inputs, label);
        if (tp == 1) addUniqueSorted(p.controls, label);
        if (tp == 2) addUniqueSorted(p.outputs, label);
        if (tp == 3) addUniqueSorted(p.mechanisms, label);
      }
    }
    return p;
  };

  auto stars = [](int count) { return QString(count, '*'); };
  auto safeName = [this](const QString& s) {
    const QString trimmed = s.trimmed();
    return trimmed.isEmpty() ? tr("Untitled") : trimmed;
  };

  QString md;
  std::function<void(const QVariantMap&, const QString&, const QString&, int, const QString&)> emitNode;
  emitNode = [&](const QVariantMap& diagram, const QString& key, const QString& number, int depth, const QString& title) {
    const Ports p = collectPorts(diagram);
    md += QStringLiteral("%1 %2: %3\n").arg(stars(depth), number, safeName(title));
    md += QStringLiteral("%1 Inputs: %2\n").arg(stars(depth + 1), joinOrDash(p.inputs));
    md += QStringLiteral("%1 Outputs: %2\n").arg(stars(depth + 1), joinOrDash(p.outputs));
    md += QStringLiteral("%1 Mechanisms: %2\n").arg(stars(depth + 1), joinOrDash(p.mechanisms));
    md += QStringLiteral("%1 Controls: %2\n").arg(stars(depth + 1), joinOrDash(p.controls));
    md += QStringLiteral("%1 Subprocesses:\n").arg(stars(depth + 1));

    const QVector<QVariantMap> blocks = sortedBlocks(diagram);
    if (blocks.isEmpty()) {
      md += QStringLiteral("%1 %2\n").arg(stars(depth + 2), tr("none"));
      return;
    }

    for (const QVariantMap& b : blocks) {
      const int id = b.value("id", -1).toInt();
      const QString childKey = key.isEmpty() ? QString::number(id) : (key + "/" + QString::number(id));
      const QString childNumber = b.value("number").toString().trimmed().isEmpty()
        ? QStringLiteral("A?")
        : b.value("number").toString().trimmed();
      const QString childTitle = safeName(b.value("title").toString());
      if (id >= 0 && children.contains(childKey)) {
        emitNode(children.value(childKey).toMap(), childKey, childNumber, depth + 2, childTitle);
      } else {
        md += QStringLiteral("%1 %2: %3\n").arg(stars(depth + 2), childNumber, childTitle);
      }
    }
  };

  QString rootTitle = m_welcomeState.value("title").toString();
  for (const QVariantMap& b : sortedBlocks(rootDiagram)) {
    const QString n = b.value("number").toString();
    if (n.compare(QStringLiteral("A0"), Qt::CaseInsensitive) == 0) {
      rootTitle = b.value("title").toString();
      break;
    }
  }
  emitNode(rootDiagram, QString(), QStringLiteral("A0"), 1, rootTitle);

  QFile f(path);
  if (!f.open(QIODevice::WriteOnly | QIODevice::Truncate | QIODevice::Text)) {
    QMessageBox::warning(this, tr("Export failed"), tr("Could not open file for writing."));
    return false;
  }
  f.write(md.toUtf8());
  f.close();
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

void MainWindow::showNodeTreeDialog() {
  if (!m_scene) return;
  const QVariantMap exported = m_scene->exportToVariant();
  const QVariantMap rootDiagram = exported.value("diagram").toMap();
  const QVariantMap children = exported.value("children").toMap();

  auto* dlg = new QDialog(this);
  dlg->setWindowTitle(tr("Node tree"));
  auto* layout = new QVBoxLayout(dlg);
  auto* tree = new QTreeWidget(dlg);
  tree->setHeaderHidden(true);
  tree->setExpandsOnDoubleClick(false);

  auto itemText = [](const QVariantMap& block) {
    const QString num = block.value("number").toString();
    const QString title = block.value("title").toString();
    if (num.isEmpty()) return title;
    if (title.isEmpty()) return num;
    return QStringLiteral("%1 %2").arg(num, title);
  };

  std::function<void(QTreeWidgetItem*, const QVariantMap&, const QString&)> addSubtree;
  addSubtree = [&](QTreeWidgetItem* parent, const QVariantMap& diagram, const QString& prefix) {
    const QVariantList blockList = diagram.value("blocks").toList();
    QVector<QVariant> blocks;
    blocks.reserve(blockList.size());
    for (const QVariant& v : blockList) blocks.push_back(v);
    std::sort(blocks.begin(), blocks.end(), [](const QVariant& a, const QVariant& b) {
      const QVariantMap ma = a.toMap();
      const QVariantMap mb = b.toMap();
      const QString na = ma.value("number").toString();
      const QString nb = mb.value("number").toString();
      if (na == nb) return ma.value("id", -1).toInt() < mb.value("id", -1).toInt();
      return na.localeAwareCompare(nb) < 0;
    });
    for (const QVariant& vb : blocks) {
      const QVariantMap block = vb.toMap();
      const int id = block.value("id", -1).toInt();
      auto* it = new QTreeWidgetItem();
      it->setText(0, itemText(block));
      it->setData(0, Qt::UserRole, prefix);        // diagram path containing this block
      it->setData(0, Qt::UserRole + 1, id);        // block id in that diagram
      parent->addChild(it);
      if (id < 0) continue;
      const QString key = prefix.isEmpty() ? QString::number(id) : (prefix + "/" + QString::number(id));
      if (children.contains(key)) {
        addSubtree(it, children.value(key).toMap(), key);
      }
    }
  };

  const QString rootName = m_welcomeState.value("title").toString().isEmpty()
    ? tr("Model")
    : m_welcomeState.value("title").toString();
  auto* root = new QTreeWidgetItem(tree);
  root->setText(0, QStringLiteral("A-0 %1").arg(rootName));
  root->setData(0, Qt::UserRole, QString());
  root->setData(0, Qt::UserRole + 1, -1);
  tree->addTopLevelItem(root);
  addSubtree(root, rootDiagram, QString());
  tree->expandAll();

  connect(tree, &QTreeWidget::itemDoubleClicked, this, [this, tree](QTreeWidgetItem* item, int) {
    if (!item || !m_scene) return;
    const QString path = item->data(0, Qt::UserRole).toString();
    const int blockId = item->data(0, Qt::UserRole + 1).toInt();

    while (m_scene->goUp()) {}

    const QStringList parts = path.split('/', Qt::SkipEmptyParts);
    for (const QString& part : parts) {
      bool ok = false;
      const int id = part.toInt(&ok);
      if (!ok) break;
      BlockItem* target = nullptr;
      for (QGraphicsItem* gi : m_scene->items()) {
        if (auto* b = qgraphicsitem_cast<BlockItem*>(gi)) {
          if (b->id() == id) {
            target = b;
            break;
          }
        }
      }
      if (!target || !m_scene->goDownIntoBlock(target)) break;
    }

    m_scene->clearSelection();
    if (blockId >= 0) {
      for (QGraphicsItem* gi : m_scene->items()) {
        if (auto* b = qgraphicsitem_cast<BlockItem*>(gi)) {
          if (b->id() == blockId) {
            b->setSelected(true);
            if (m_view) m_view->centerOn(b);
            break;
          }
        }
      }
    } else if (m_view) {
      m_view->centerOn(m_scene->sceneRect().center());
    }
    tree->setCurrentItem(item);
  });

  layout->addWidget(tree);
  auto* buttons = new QDialogButtonBox(QDialogButtonBox::Close, dlg);
  connect(buttons, &QDialogButtonBox::rejected, dlg, &QDialog::reject);
  connect(buttons, &QDialogButtonBox::accepted, dlg, &QDialog::accept);
  layout->addWidget(buttons);
  dlg->resize(640, 520);
  dlg->exec();
  dlg->deleteLater();
}

void MainWindow::runValidation() {
  if (!m_scene) return;

  const QVariantMap exported = m_scene->exportToVariant();
  const QVariantMap rootDiagram = exported.value("diagram").toMap();
  const QVariantMap children = exported.value("children").toMap();

  struct DiagCtx {
    QString key;         // "" for root, "1/2/3" for descendants
    QString node;        // A0, A1, A1.2, ...
    QVariantMap diagram; // variant map with blocks/arrows
  };

  QHash<QString, DiagCtx> diags;
  std::function<void(const QString&, const QString&, const QVariantMap&)> collect;
  collect = [&](const QString& key, const QString& node, const QVariantMap& diagram) {
    diags.insert(key, DiagCtx{key, node, diagram});
    const QVariantList blocks = diagram.value("blocks").toList();
    for (const QVariant& vb : blocks) {
      const QVariantMap b = vb.toMap();
      const int id = b.value("id", -1).toInt();
      if (id < 0) continue;
      const QString childKey = key.isEmpty() ? QString::number(id) : key + "/" + QString::number(id);
      if (!children.contains(childKey)) continue;
      const QString childNode = b.value("number").toString().isEmpty() ? node : b.value("number").toString();
      collect(childKey, childNode, children.value(childKey).toMap());
    }
  };
  collect(QString(), QStringLiteral("A0"), rootDiagram);

  auto epKind = [](const QVariantMap& ep) { return ep.value("kind").toInt(0); }; // 1=Block, 2=Junction, 3=Scene
  auto epId = [](const QVariantMap& ep) { return ep.value("id", -1).toInt(); };
  auto epPort = [](const QVariantMap& ep) { return ep.value("port", 0).toInt(); }; // 0=I,1=C,2=O,3=M
  auto epTun = [](const QVariantMap& ep) { return ep.value("tunneled").toBool(); };
  auto blockLabel = [](const QVariantMap& b) {
    const QString num = b.value("number").toString();
    const QString title = b.value("title").toString();
    if (num.isEmpty()) return title;
    return title.isEmpty() ? num : QStringLiteral("%1 %2").arg(num, title);
  };

  QStringList errors;
  QStringList warnings;

  // rule 5
  if (rootDiagram.value("blocks").toList().size() > 1) {
    errors << tr("There are several blocks in the context (root) diagram (there should be only one).");
  }

  for (const DiagCtx& d : diags) {
    const bool isRoot = d.key.isEmpty();
    const QVariantList blocks = d.diagram.value("blocks").toList();
    const QVariantList arrows = d.diagram.value("arrows").toList();

    QHash<int, QVariantMap> blockById;
    for (const QVariant& vb : blocks) {
      const QVariantMap b = vb.toMap();
      blockById.insert(b.value("id", -1).toInt(), b);
    }

    // rules 6,7 for non-root
    if (!isRoot) {
      if (blocks.size() < 3) {
        errors << tr("The decomposition diagram (%1) contains fewer than 3 blocks (there should be 3–8).").arg(d.node);
      } else if (blocks.size() > 8) {
        errors << tr("The decomposition diagram (%1) contains more than 8 blocks (there should be 3–8).").arg(d.node);
      }
    }

    // rule 8 hanging interface circles
    if (!isRoot) {
      bool hasHanging = false;
      for (const QVariant& va : arrows) {
        const QVariantMap a = va.toMap();
        if (a.value("isInterface").toBool() &&
            a.value("isInterfaceStub").toBool() &&
            !a.value("interfaceEdge").toMap().value("tunneled").toBool()) {
          hasHanging = true;
          break;
        }
      }
      if (hasHanging) {
        errors << tr("The decomposition of block (%1) is incomplete; there are hanging white connector circles.").arg(d.node);
      }
    }

    QHash<int, int> inIC;
    QHash<int, int> outO;
    QHash<int, QStringList> inByBlockLabel;
    QHash<int, QStringList> outByBlockLabel;
    QHash<QString, int> sameLabelEntries; // blockId:port:label
    QHash<int, bool> hasOtherBlockLink;
    QHash<int, bool> hasIncident;
    QHash<int, bool> allIncidentTunneled;

    for (auto it = blockById.cbegin(); it != blockById.cend(); ++it) {
      hasOtherBlockLink[it.key()] = false;
      hasIncident[it.key()] = false;
      allIncidentTunneled[it.key()] = true;
    }

    for (const QVariant& va : arrows) {
      const QVariantMap a = va.toMap();
      const QVariantMap f = a.value("from").toMap();
      const QVariantMap t = a.value("to").toMap();
      const QString lbl = a.value("label").toString().trimmed();

      const int fk = epKind(f), tk = epKind(t);
      const int fid = epId(f), tid = epId(t);
      const int fp = epPort(f), tp = epPort(t);

      if (fk == 1 && blockById.contains(fid)) {
        hasIncident[fid] = true;
        allIncidentTunneled[fid] = allIncidentTunneled[fid] && epTun(f);
      }
      if (tk == 1 && blockById.contains(tid)) {
        hasIncident[tid] = true;
        allIncidentTunneled[tid] = allIncidentTunneled[tid] && epTun(t);
      }

      if (tk == 1 && blockById.contains(tid) && (tp == 0 || tp == 1)) {
        inIC[tid] += 1;
      }
      if (fk == 1 && blockById.contains(fid) && fp == 2) {
        outO[fid] += 1;
      }

      if (tk == 1 && blockById.contains(tid) && tp == 0 && !lbl.isEmpty()) {
        inByBlockLabel[tid].push_back(lbl);
      }
      if (fk == 1 && blockById.contains(fid) && fp == 2 && !lbl.isEmpty()) {
        outByBlockLabel[fid].push_back(lbl);
      }

      if (tk == 1 && blockById.contains(tid) && (tp == 0 || tp == 1 || tp == 2) && !lbl.isEmpty()) {
        const QString key = QStringLiteral("%1:%2:%3").arg(tid).arg(tp).arg(lbl);
        sameLabelEntries[key] += 1;
      }

      if (fk == 1 && tk == 1 && blockById.contains(fid) && blockById.contains(tid) && fid != tid) {
        hasOtherBlockLink[fid] = true;
        hasOtherBlockLink[tid] = true;
      }

      // warnings 1..4
      if (!isRoot && fk == 3 && tk == 1) {
        if (fp == 3 && tp == 0) warnings << tr("The mechanism of diagram %1 acts as an input for block %2.").arg(d.node, blockById.value(tid).value("number").toString());
        if (fp == 3 && tp == 1) warnings << tr("The mechanism of diagram %1 acts as a control for block %2.").arg(d.node, blockById.value(tid).value("number").toString());
        if (fp == 1 && tp == 3) warnings << tr("The control of diagram %1 acts as a mechanism for block %2.").arg(d.node, blockById.value(tid).value("number").toString());
        if (fp == 0 && tp == 3) warnings << tr("The input of diagram %1 acts as a mechanism for block %2.").arg(d.node, blockById.value(tid).value("number").toString());
      }

      // warning 5
      if (fk == 1 && tk == 1 && fp == 2 && tp == 3 && blockById.contains(fid) && blockById.contains(tid)) {
        warnings << tr("The output of block %1 acts as a mechanism for block %2.")
                    .arg(blockById.value(fid).value("number").toString(),
                         blockById.value(tid).value("number").toString());
      }
    }

    for (auto it = blockById.cbegin(); it != blockById.cend(); ++it) {
      const int id = it.key();
      const QVariantMap b = it.value();
      const QString title = b.value("title").toString();
      const QString titleTrim = title.trimmed();

      // rules 1,2
      if (title == tr("Function") || title == QStringLiteral("Function")) {
        errors << tr("The block name should reflect what it does. Do not leave block %1 with a default name.").arg(blockLabel(b));
      }
      if (titleTrim.isEmpty()) {
        errors << tr("The block name should reflect what it does. Do not leave block %1 with an empty name.").arg(blockLabel(b));
      }

      // rules 3,4
      if (inIC.value(id, 0) == 0) {
        errors << tr("Each block must have at least one input or control: %1.").arg(blockLabel(b));
      }
      if (outO.value(id, 0) == 0) {
        errors << tr("Each block must have at least one output: %1.").arg(blockLabel(b));
      }

      // rule 11
      QSet<QString> inNames, outNames;
      for (const QString& s : inByBlockLabel.value(id)) inNames.insert(s.trimmed());
      for (const QString& s : outByBlockLabel.value(id)) outNames.insert(s.trimmed());
      for (const QString& s : inNames) {
        if (!s.isEmpty() && outNames.contains(s)) {
          errors << tr("Input and output names are identical (%1) in process %2.").arg(s, b.value("number").toString());
        }
      }

      // warning 6
      if (hasIncident.value(id, false) && allIncidentTunneled.value(id, false)) {
        warnings << tr("Block %1 is a black box (all connected endpoints are tunneled).").arg(b.value("number").toString());
      }

      // warning 8
      if (!isRoot && blocks.size() > 1 && !hasOtherBlockLink.value(id, false)) {
        warnings << tr("Block %1 is not related to other blocks in decomposition %2.").arg(b.value("number").toString(), d.node);
      }
    }

    // rule 10
    for (auto it = sameLabelEntries.cbegin(); it != sameLabelEntries.cend(); ++it) {
      if (it.value() > 1) {
        errors << tr("The same arrow enters the same block several times in diagram %1 (%2).").arg(d.node, it.key());
      }
    }

    // warning 7
    for (const QVariant& vb : blocks) {
      const QVariantMap b = vb.toMap();
      if (!b.contains("price") || b.value("price").isNull()) continue;
      const int id = b.value("id", -1).toInt();
      if (id < 0) continue;
      const QString childKey = d.key.isEmpty() ? QString::number(id) : d.key + "/" + QString::number(id);
      if (!children.contains(childKey)) continue;
      const QVariantList childBlocks = children.value(childKey).toMap().value("blocks").toList();
      qreal sum = 0.0;
      for (const QVariant& vcb : childBlocks) {
        const QVariantMap cb = vcb.toMap();
        if (cb.contains("price") && !cb.value("price").isNull()) sum += cb.value("price").toDouble();
      }
      const qreal parentPrice = b.value("price").toDouble();
      if (sum > parentPrice + 1e-6) {
        warnings << tr("The total cost of process %1 is lower than the total cost of its subprocesses. Use Tools >> Calculate Prices.")
                    .arg(b.value("number").toString());
      }
    }
  }

  QString report;
  report += tr("Errors:") + "\n";
  if (errors.isEmpty()) report += tr("None.") + "\n";
  else for (int i = 0; i < errors.size(); ++i) report += QString::number(i + 1) + ". " + errors[i] + "\n";
  report += "\n" + tr("Warnings:") + "\n";
  if (warnings.isEmpty()) report += tr("None.") + "\n";
  else for (int i = 0; i < warnings.size(); ++i) report += QString::number(i + 1) + ". " + warnings[i] + "\n";

  auto* dlg = new QDialog(this);
  dlg->setWindowTitle(tr("Validation"));
  auto* layout = new QVBoxLayout(dlg);
  auto* txt = new QTextEdit(dlg);
  txt->setReadOnly(true);
  txt->setPlainText(report);
  layout->addWidget(txt);
  auto* buttons = new QDialogButtonBox(QDialogButtonBox::Close, dlg);
  connect(buttons, &QDialogButtonBox::rejected, dlg, &QDialog::reject);
  connect(buttons, &QDialogButtonBox::accepted, dlg, &QDialog::accept);
  layout->addWidget(buttons);
  dlg->resize(860, 640);
  dlg->exec();
  dlg->deleteLater();
}

void MainWindow::calculatePrices() {
  if (!m_scene) return;
  QVariantMap exported = m_scene->exportToVariant();
  QVariantMap rootDiagram = exported.value("diagram").toMap();
  QVariantMap children = exported.value("children").toMap();

  int updatedBlocks = 0;
  std::function<void(const QString&, QVariantMap&)> recalc;
  recalc = [&](const QString& key, QVariantMap& diagram) {
    QVariantList blocks = diagram.value("blocks").toList();
    for (int i = 0; i < blocks.size(); ++i) {
      QVariantMap b = blocks[i].toMap();
      const int id = b.value("id", -1).toInt();
      if (id < 0) continue;
      const QString childKey = key.isEmpty() ? QString::number(id) : key + "/" + QString::number(id);
      if (!children.contains(childKey)) continue;
      QVariantMap child = children.value(childKey).toMap();
      recalc(childKey, child);
      children[childKey] = child;
      const QVariantList childBlocks = child.value("blocks").toList();
      qreal sum = 0.0;
      bool hasAny = false;
      for (const QVariant& vcb : childBlocks) {
        const QVariantMap cb = vcb.toMap();
        if (cb.contains("price") && !cb.value("price").isNull()) {
          sum += cb.value("price").toDouble();
          hasAny = true;
        }
      }
      if (!hasAny) continue;
      const qreal oldPrice = b.contains("price") && !b.value("price").isNull() ? b.value("price").toDouble() : std::numeric_limits<qreal>::quiet_NaN();
      if (std::isnan(oldPrice) || std::abs(oldPrice - sum) > 1e-6) {
        b["price"] = sum;
        blocks[i] = b;
        updatedBlocks += 1;
      }
    }
    diagram["blocks"] = blocks;
  };

  recalc(QString(), rootDiagram);
  exported["diagram"] = rootDiagram;
  exported["children"] = children;
  exported["meta"] = stripPerDiagramState(m_welcomeState);

  if (!m_scene->importFromVariant(exported)) {
    QMessageBox::warning(this, tr("Calculate Prices"), tr("Failed to apply calculated prices."));
    return;
  }
  m_scene->applyMeta(mergeWithDiagramState(m_welcomeState, m_scene->meta()));
  markDirty(true);
  QMessageBox::information(this, tr("Calculate Prices"), tr("Updated prices for %1 block(s).").arg(updatedBlocks));
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
  setWindowTitle(name + tr(" — erlu IDEF0 editor"));
}
