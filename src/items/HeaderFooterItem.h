#pragma once
#include <QGraphicsObject>
#include <QVariantMap>

class DiagramScene;

class HeaderFooterItem final : public QGraphicsObject {
  Q_OBJECT
public:
  explicit HeaderFooterItem(DiagramScene* scene);

  void setMeta(const QVariantMap& meta);
  void setPageRect(const QRectF& rect);
  void setShowHeaderFooter(bool showHeader, bool showFooter);

  QRectF boundingRect() const override;
  void paint(QPainter* painter, const QStyleOptionGraphicsItem* option, QWidget* widget) override;

protected:
  void mouseDoubleClickEvent(QGraphicsSceneMouseEvent* e) override;

private:
  DiagramScene* m_scene = nullptr;
  QVariantMap m_meta;
  QRectF m_pageRect;
  bool m_showHeader = true;
  bool m_showFooter = true;

  QString metaText(const QString& key, const QString& fallback = QString()) const;
  bool metaBool(const QString& key, bool fallback = false) const;
  bool hitHeaderFooter(const QPointF& scenePos) const;
};
