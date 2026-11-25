//! Input assistance and conservative cleanup of orthogonal arrow geometry.
use crate::*;

pub const ALIGN_DISTANCE: f64 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Grid {
    pub enabled: bool,
    pub spacing: f64,
}
impl Default for Grid {
    fn default() -> Self {
        Self {
            enabled: false,
            spacing: 5.0,
        }
    }
}
impl Grid {
    pub fn snap(self, point: Point) -> Point {
        if !self.enabled || !self.spacing.is_finite() || self.spacing <= 0.0 {
            return point;
        }
        Point::new(
            (point.x / self.spacing).round() * self.spacing,
            (point.y / self.spacing).round() * self.spacing,
        )
    }
}

/// Snap along an edge, keeping the endpoint attached to its original object.
pub fn snap_endpoint(doc: &Document, diagram: Id, endpoint: &mut Endpoint, grid: Grid) {
    if !grid.enabled {
        return;
    }
    let Some(point) = doc.endpoint(diagram, endpoint) else {
        return;
    };
    let snapped = grid.snap(point);
    match &mut endpoint.anchor {
        Anchor::Block { id, port, t } => {
            let pos = doc.diagrams[&diagram].blocks[id].pos;
            *t = match port {
                Port::Input | Port::Output => (snapped.y - pos.y) / 26.5,
                _ => (snapped.x - pos.x) / 53.0,
            }
            .clamp(0.0, 1.0);
        }
        Anchor::Boundary { port, t } => {
            let (lo, hi) = doc.content();
            *t = match port {
                Port::Input | Port::Output => (snapped.y - lo.y) / (hi.y - lo.y),
                _ => (snapped.x - lo.x) / (hi.x - lo.x),
            }
            .clamp(0.0, 1.0);
        }
        Anchor::Junction { .. } => {}
    }
}

/// A nearly aligned border endpoint follows the block port, never the reverse.
/// Inherited interfaces retain their parent's fixed boundary coordinate.
pub fn align_boundary(doc: &Document, diagram: Id, arrow: &mut Arrow) {
    if arrow.interface.is_some() {
        return;
    }
    let (block, boundary) = match (&arrow.from.anchor, &arrow.to.anchor) {
        (Anchor::Block { .. }, Anchor::Boundary { .. }) => (&arrow.from, &mut arrow.to),
        (Anchor::Boundary { .. }, Anchor::Block { .. }) => (&arrow.to, &mut arrow.from),
        _ => return,
    };
    let Anchor::Block {
        port: block_port, ..
    } = block.anchor
    else {
        return;
    };
    let Some(point) = doc.endpoint(diagram, block) else {
        return;
    };
    let Some(edge) = doc.endpoint(diagram, boundary) else {
        return;
    };
    let Anchor::Boundary { port, t } = &mut boundary.anchor else {
        return;
    };
    if *port != block_port {
        return;
    }
    let (lo, hi) = doc.content();
    match port {
        Port::Input | Port::Output if (point.y - edge.y).abs() <= ALIGN_DISTANCE => {
            *t = ((point.y - lo.y) / (hi.y - lo.y)).clamp(0.0, 1.0)
        }
        Port::Control | Port::Mechanism if (point.x - edge.x).abs() <= ALIGN_DISTANCE => {
            *t = ((point.x - lo.x) / (hi.x - lo.x)).clamp(0.0, 1.0)
        }
        _ => {}
    }
}

fn collinear(a: Point, b: Point, c: Point) -> bool {
    ((a.x - b.x).abs() < 1e-7 && (b.x - c.x).abs() < 1e-7)
        || ((a.y - b.y).abs() < 1e-7 && (b.y - c.y).abs() < 1e-7)
}
fn exact(path: Vec<Point>) -> Vec<Point> {
    let mut out: Vec<Point> = vec![];
    for point in path {
        while out.len() >= 2 && collinear(out[out.len() - 2], out[out.len() - 1], point) {
            out.pop();
        }
        if out.last().is_none_or(|last| last.distance(point) > 1e-7) {
            out.push(point);
        }
    }
    out
}
fn clear(a: Point, b: Point, obstacles: &[(Point, Point)]) -> bool {
    !obstacles.iter().any(|(lo, hi)| {
        if (a.x - b.x).abs() < 1e-7 {
            a.x > lo.x && a.x < hi.x && a.y.max(b.y) > lo.y && a.y.min(b.y) < hi.y
        } else {
            a.y > lo.y && a.y < hi.y && a.x.max(b.x) > lo.x && a.x.min(b.x) < hi.x
        }
    })
}
/// Remove retracing U-turns and small zigzags only if their replacement clears
/// every block. Large deliberate steps and obstacle detours remain untouched.
pub fn simplify(path: Vec<Point>, obstacles: &[(Point, Point)]) -> Vec<Point> {
    let mut path = exact(path);
    loop {
        let mut replacement = None;
        for i in 0..path.len().saturating_sub(3) {
            let [a, b, c, d] = [path[i], path[i + 1], path[i + 2], path[i + 3]];
            let first = b - a;
            let last = d - c;
            let retraces = first.x * last.x + first.y * last.y < 0.0;
            if !retraces && b.distance(c) > ALIGN_DISTANCE {
                continue;
            }
            let old_length = a.distance(b) + b.distance(c) + c.distance(d);
            let mut candidates = vec![];
            for corner in [Point::new(a.x, d.y), Point::new(d.x, a.y)] {
                if !clear(a, corner, obstacles) || !clear(corner, d, obstacles) {
                    continue;
                }
                if a.distance(corner) + corner.distance(d) > old_length + 1e-7 {
                    continue;
                }
                let mut next = path[..i].to_vec();
                next.extend([a, corner, d]);
                next.extend_from_slice(&path[i + 4..]);
                let next = exact(next);
                if next.len() < path.len() {
                    candidates.push(next);
                }
            }
            if let Some(next) = candidates.into_iter().min_by_key(Vec::len) {
                replacement = Some(next);
                break;
            }
        }
        let Some(next) = replacement else { break };
        path = next;
    }
    path
}

/// Project a dragged port back onto its edge; junctions remain shared and fixed.
pub fn slide_endpoint(doc: &Document, diagram: Id, endpoint: &mut Endpoint, point: Point) {
    match &mut endpoint.anchor {
        Anchor::Block { id, port, t } => {
            let pos = doc.diagrams[&diagram].blocks[id].pos;
            *t = match port {
                Port::Input | Port::Output => (point.y - pos.y) / 26.5,
                _ => (point.x - pos.x) / 53.0,
            }
            .clamp(0.0, 1.0);
        }
        Anchor::Boundary { port, t } => {
            let (lo, hi) = doc.content();
            *t = match port {
                Port::Input | Port::Output => (point.y - lo.y) / (hi.y - lo.y),
                _ => (point.x - lo.x) / (hi.x - lo.x),
            }
            .clamp(0.0, 1.0);
        }
        Anchor::Junction { .. } => {}
    }
}

/// The preview and committed drag use identical geometry. Dragging an end
/// segment slides its port along the edge instead of leaving a tiny dogleg.
pub fn drag_segment(
    doc: &Document,
    diagram: Id,
    arrow: &Arrow,
    mut path: Vec<Point>,
    index: usize,
    delta: Point,
    grid: Grid,
) -> Arrow {
    let mut out = arrow.clone();
    if index + 1 >= path.len() {
        return out;
    }
    let vertical = (path[index].x - path[index + 1].x).abs() < 1e-7;
    if vertical {
        let x = grid.snap(path[index] + delta).x;
        path[index].x = x;
        path[index + 1].x = x;
    } else {
        let y = grid.snap(path[index] + delta).y;
        path[index].y = y;
        path[index + 1].y = y;
    }
    if index == 0
        && (out.interface.is_none() || !matches!(out.from.anchor, Anchor::Boundary { .. }))
    {
        slide_endpoint(doc, diagram, &mut out.from, path[0]);
    }
    if index + 2 == path.len()
        && (out.interface.is_none() || !matches!(out.to.anchor, Anchor::Boundary { .. }))
    {
        slide_endpoint(doc, diagram, &mut out.to, *path.last().unwrap());
    }
    out.bends = path.into_iter().skip(1).collect();
    out.bends.pop();
    align_boundary(doc, diagram, &mut out);
    out
}

/// Normalize only arrows attached to moved objects. Translating a group keeps
/// internal manual bends with that group instead of pinning them to the page.
pub fn normalize_moves(
    before: &Document,
    after: &mut Document,
    moved: &std::collections::BTreeSet<Id>,
) {
    let mut affected = vec![];
    for (&d, diag) in &after.diagrams {
        for a in diag.arrows.values() {
            let touches = |ep: &Endpoint| match ep.anchor {
                Anchor::Block { id, .. } | Anchor::Junction { id } => moved.contains(&id),
                _ => false,
            };
            if touches(&a.from) || touches(&a.to) {
                affected.push((d, a.id, touches(&a.from) && touches(&a.to)));
            }
        }
    }
    for (d, id, both) in affected {
        if both {
            let a = &after.diagrams[&d].arrows[&id];
            let delta = after.endpoint(d, &a.from).unwrap() - before.endpoint(d, &a.from).unwrap();
            let other = after.endpoint(d, &a.to).unwrap() - before.endpoint(d, &a.to).unwrap();
            if delta.distance(other) < 1e-7 {
                for point in &mut after
                    .diagrams
                    .get_mut(&d)
                    .unwrap()
                    .arrows
                    .get_mut(&id)
                    .unwrap()
                    .bends
                {
                    *point = *point + delta;
                }
            }
        }
        normalize_arrow(after, d, id);
    }
}
pub fn normalize_arrow(doc: &mut Document, d: Id, id: Id) {
    let mut arrow = doc.diagrams[&d].arrows[&id].clone();
    align_boundary(doc, d, &mut arrow);
    if !arrow.bends.is_empty() {
        let path = render::route(doc, d, &arrow);
        arrow.bends = path.into_iter().skip(1).collect();
        arrow.bends.pop();
    }
    doc.diagrams.get_mut(&d).unwrap().arrows.insert(id, arrow);
}
