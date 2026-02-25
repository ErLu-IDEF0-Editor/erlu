#pragma once
#include <QGraphicsObject>
#include <QSet>
#include <QColor>

class ArrowItem;

// Small movable node that lets several arrows meet at a single point.
class JunctionItem final : public QGraphicsObject {
  Q_OBJECT
public:
  enum { Type = UserType + 2 };
  explicit JunctionItem(QGraphicsItem* parent = nullptr, int id = -1);

  QRectF boundingRect() const override;
  void paint(QPainter* p, const QStyleOptionGraphicsItem* opt, QWidget* widget) override;
  int type() const override { return Type; }

  void addArrow(ArrowItem* a);
  void removeArrow(ArrowItem* a);

  bool hitTest(const QPointF& scenePos, qreal radius) const;
  int id() const { return m_id; }
  void setId(int id) { m_id = id; }
  static void setVisualTheme(const QColor& normalColor, const QColor& selectedColor);

protected:
  QVariant itemChange(GraphicsItemChange change, const QVariant& value) override;

private:
  qreal m_radius = 5.0;
  int m_id;
  QSet<ArrowItem*> m_arrows;
  static QColor s_normalColor;
  static QColor s_selectedColor;
};
