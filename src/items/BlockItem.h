#pragma once
#include <QGraphicsObject>
#include <QSet>
#include <optional>
#include <QColor>

class ArrowItem;

class BlockItem final : public QGraphicsObject {
  Q_OBJECT
public:
  enum class Port { Input, Control, Output, Mechanism };
  enum { Type = UserType + 1 };

  explicit BlockItem(QString title = "Function", QGraphicsItem* parent = nullptr, int id = -1);

  QRectF boundingRect() const override;
  void paint(QPainter* p, const QStyleOptionGraphicsItem*, QWidget*) override;
  int type() const override { return Type; }

  QString title() const { return m_title; }
  void setTitle(const QString& t);
  QString number() const { return m_number; }
  void setNumber(const QString& n);
  bool hasDecomposition() const { return m_hasDecomposition; }
  void setHasDecomposition(bool v) { m_hasDecomposition = v; update(); }
  int id() const { return m_id; }
  void setId(int id) { m_id = id; }

  QPointF portScenePos(Port p) const;
  QPair<QPointF, QPointF> portSegment(Port p) const;
  bool hitTestPort(const QPointF& scenePos, Port* outPort, QPointF* outLocalPos = nullptr, qreal radiusPx = 10.0) const;
  QPointF portLocalPos(Port p) const;

  void addArrow(ArrowItem* a);
  void removeArrow(ArrowItem* a);

  std::optional<qreal> price() const { return m_price; }
  void setPrice(std::optional<qreal> price);
  static void setCurrencyFormat(const QString& symbol, const QString& placement);
  static void setVisualTheme(const QColor& foreground,
                             const QColor& background,
                             const QColor& border,
                             const QColor& font,
                             const QColor& selectedBackground);

signals:
  void geometryChanged();

protected:
  QVariant itemChange(GraphicsItemChange change, const QVariant& value) override;
  void mouseDoubleClickEvent(QGraphicsSceneMouseEvent* e) override;

private:
  QString formattedPrice() const;
  static QPair<QString, QString> currencyAffixes();

  QString m_title;
  QString m_number;
  QRectF m_rect;          // local rect
  int m_id;
  bool m_hasDecomposition = false;
  QSet<ArrowItem*> m_arrows;
  std::optional<qreal> m_price;

  static QString s_currencySymbol;
  static QString s_currencyPlacement;
  static QColor s_foregroundColor;
  static QColor s_backgroundColor;
  static QColor s_borderColor;
  static QColor s_fontColor;
  static QColor s_selectedBackgroundColor;
};
