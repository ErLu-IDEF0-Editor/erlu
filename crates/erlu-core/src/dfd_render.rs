use crate::{
    Id, Point,
    dfd::{Document, End, Flow, Kind, Node, Style},
    render::{Primitive, Scene, label_position, manhattan},
};
use std::collections::BTreeMap;

pub fn half_size(kind: Kind, style: Style) -> Point {
    match kind {
        Kind::Process if style == Style::YourdonDemarco => Point::new(16., 16.),
        Kind::Process => Point::new(24., 11.),
        Kind::External => Point::new(21., 8.),
        Kind::Store => Point::new(23., 7.),
    }
}

pub fn contains(node: &Node, style: Style, p: Point) -> bool {
    let half = half_size(node.kind, style);
    let delta = p - node.position;
    if node.kind == Kind::Process && style == Style::YourdonDemarco {
        return delta.distance(Point::default()) <= half.x;
    }
    delta.x.abs() <= half.x && delta.y.abs() <= half.y
}

pub fn boundaries(doc: &Document, diagram: Id) -> BTreeMap<Id, Point> {
    let nodes = &doc.diagrams[&diagram].nodes;
    let left = nodes
        .values()
        .map(|n| n.position.x - half_size(n.kind, doc.style).x)
        .fold(-45., f64::min)
        - 26.;
    let right = nodes
        .values()
        .map(|n| n.position.x + half_size(n.kind, doc.style).x)
        .fold(45., f64::max)
        + 26.;
    let interfaces = doc.interfaces(diagram);
    let count_in = interfaces.iter().filter(|x| x.1).count() as f64;
    let count_out = interfaces.len() as f64 - count_in;
    let (mut i, mut o) = (0., 0.);
    interfaces
        .into_iter()
        .map(|(id, incoming, _)| {
            let (x, slot, count) = if incoming {
                i += 1.;
                (left, i, count_in)
            } else {
                o += 1.;
                (right, o, count_out)
            };
            (id, Point::new(x, (slot - (count + 1.) / 2.) * 15.))
        })
        .collect()
}

pub fn route(
    doc: &Document,
    diagram: Id,
    flow: &Flow,
    boundary: &BTreeMap<Id, Point>,
) -> Vec<Point> {
    let nodes = &doc.diagrams[&diagram].nodes;
    let center = |end| match end {
        End::Node { id } => nodes[&id].position,
        End::Boundary { parent_flow } => boundary[&parent_flow],
    };
    let attach = |end, reference: Point| {
        let p = center(end);
        let End::Node { id } = end else {
            return (p, p);
        };
        let n = &nodes[&id];
        let half = half_size(n.kind, doc.style);
        let delta = reference - p;
        let horizontal = delta.x.abs() / half.x >= delta.y.abs() / half.y;
        let (axis, length) = if horizontal {
            (Point::new(if delta.x < 0. { -1. } else { 1. }, 0.), half.x)
        } else {
            (Point::new(0., if delta.y < 0. { -1. } else { 1. }), half.y)
        };
        (
            Point::new(p.x + axis.x * length, p.y + axis.y * length),
            Point::new(p.x + axis.x * (length + 3.), p.y + axis.y * (length + 3.)),
        )
    };
    let obstacles = nodes
        .values()
        .map(|n| {
            let half = half_size(n.kind, doc.style) + Point::new(1., 1.);
            (n.position - half, n.position + half)
        })
        .collect::<Vec<_>>();
    let bends: Vec<_> = flow
        .bends
        .iter()
        .copied()
        .filter(|p| {
            !obstacles
                .iter()
                .any(|(a, b)| a.x < p.x && b.x > p.x && a.y < p.y && b.y > p.y)
        })
        .collect();
    let (start, lead) = attach(
        flow.from,
        bends.first().copied().unwrap_or_else(|| center(flow.to)),
    );
    let (end, tail) = attach(
        flow.to,
        bends.last().copied().unwrap_or_else(|| center(flow.from)),
    );
    let mut path = vec![start, lead];
    let mut current = lead;
    for target in bends.into_iter().chain([tail]) {
        path.extend(manhattan(current, target, &obstacles).into_iter().skip(1));
        current = target;
    }
    path.push(end);
    path.dedup();
    path
}

pub fn scene(doc: &Document, diagram: Id, dark: bool, page: Option<usize>) -> (Scene, Point) {
    let d = &doc.diagrams[&diagram];
    let (ink, paper) = if dark {
        ("#E0E8ED", "#222A31")
    } else {
        ("#1D3443", "#FFFFFF")
    };
    let boundary = boundaries(doc, diagram);
    let numbers = doc.numbers(diagram);
    let mut min = Point::new(-135., -90.);
    let mut max = Point::new(135., 90.);
    let mut include = |p: Point, margin: Point| {
        min.x = min.x.min(p.x - margin.x);
        min.y = min.y.min(p.y - margin.y);
        max.x = max.x.max(p.x + margin.x);
        max.y = max.y.max(p.y + margin.y);
    };
    let mut shapes = vec![];
    let line = |points: Vec<Point>| Primitive::Line {
        points,
        color: ink.into(),
        width: 0.4,
        dashed: false,
    };
    let text = |pos, text: String, size| Primitive::Text {
        pos,
        text,
        size,
        color: ink.into(),
        center: true,
        right: false,
    };
    for flow in d.flows.values() {
        let path = route(doc, diagram, flow, &boundary);
        for &p in &path {
            include(p, Point::new(14., 12.));
        }
        if let [.., before, tip] = path.as_slice() {
            let delta = *tip - *before;
            let length = before.distance(*tip);
            if length > 0. {
                let u = Point::new(delta.x / length, delta.y / length);
                let back = Point::new(tip.x - 2.5 * u.x, tip.y - 2.5 * u.y);
                let wing = Point::new(-u.y, u.x);
                shapes.push(line(vec![back + wing, *tip, back - wing]));
            }
        }
        if path.len() > 1 {
            let name = doc.flow_label(diagram, flow);
            let pos = label_position(&path, Point::new(0., -2.5));
            include(pos, Point::new(8. + name.chars().count() as f64 * 0.9, 10.));
            shapes.push(text(pos, name.into(), 3.));
        }
        shapes.push(line(path));
    }
    for (&id, node) in &d.nodes {
        let p = node.position;
        let half = half_size(node.kind, doc.style);
        let lo = p - half;
        let hi = p + half;
        include(p, half + Point::new(12., 12.));
        match (node.kind, doc.style) {
            (Kind::Process, Style::YourdonDemarco) => {
                shapes.push(Primitive::Circle {
                    pos: p,
                    radius: half.x,
                    color: ink.into(),
                    fill: paper.into(),
                });
            }
            (Kind::Store, style) => {
                shapes.push(line(vec![lo, Point::new(hi.x, lo.y)]));
                shapes.push(line(vec![Point::new(lo.x, hi.y), hi]));
                if style == Style::GaneSarson {
                    shapes.push(line(vec![lo, Point::new(lo.x, hi.y)]));
                    shapes.push(line(vec![
                        Point::new(lo.x + 9., lo.y),
                        Point::new(lo.x + 9., hi.y),
                    ]));
                    shapes.push(text(Point::new(lo.x + 4.5, p.y), format!("D{id}"), 2.6));
                }
            }
            (kind, _) => {
                shapes.push(Primitive::Rect {
                    min: lo,
                    max: hi,
                    stroke: ink.into(),
                    fill: paper.into(),
                    width: 0.4,
                    radius: if kind == Kind::Process { 2. } else { 0. },
                    dashed: false,
                });
                if kind == Kind::Process {
                    shapes.push(line(vec![
                        Point::new(lo.x, lo.y + 6.),
                        Point::new(hi.x, lo.y + 6.),
                    ]));
                }
            }
        }
        if let Some(number) = numbers.get(&id) {
            let y = if doc.style == Style::GaneSarson {
                lo.y + 3.
            } else {
                p.y - 8.
            };
            shapes.push(text(Point::new(p.x, y), number.clone(), 2.8));
        }
        let x = if node.kind == Kind::Store && doc.style == Style::GaneSarson {
            p.x + 4.5
        } else {
            p.x
        };
        let y = p.y + if node.kind == Kind::Process { 2. } else { 0. };
        let limit = if doc.style == Style::YourdonDemarco && node.kind == Kind::Process {
            16
        } else {
            24
        };
        for (index, row) in wrap(&node.title, limit).into_iter().enumerate() {
            shapes.push(text(Point::new(x, y + index as f64 * 3.3), row, 3.));
        }
        if node.child.is_some() {
            shapes.push(text(Point::new(p.x, hi.y + 2.5), "↓".into(), 3.2));
        }
    }
    for (id, _, flow) in doc.interfaces(diagram) {
        let point = boundary[&id];
        let name = doc.flow_label(doc.parent(diagram).unwrap(), flow);
        include(
            point,
            Point::new(10. + name.chars().count() as f64 * 0.9, 10.),
        );
        shapes.push(Primitive::Circle {
            pos: point,
            radius: 1.5,
            color: ink.into(),
            fill: paper.into(),
        });
        shapes.push(text(point - Point::new(0., 4.), name.into(), 2.8));
    }
    if let Some(page) = page {
        shapes.push(text(
            Point::new(max.x - 8., max.y - 5.),
            page.to_string(),
            3.,
        ));
    }
    shapes.insert(
        0,
        Primitive::Rect {
            min,
            max,
            stroke: paper.into(),
            fill: paper.into(),
            width: 0.,
            radius: 0.,
            dashed: false,
        },
    );
    (
        Scene {
            size: max - min,
            primitives: shapes,
        },
        Point::new((min.x + max.x) / 2., (min.y + max.y) / 2.),
    )
}

fn wrap(text: &str, limit: usize) -> Vec<String> {
    let characters: Vec<_> = text.chars().collect();
    characters
        .chunks(limit)
        .take(3)
        .enumerate()
        .map(|(index, row)| {
            let mut line: String = row.iter().collect();
            if index == 2 && characters.len() > limit * 3 {
                line.pop();
                line.push('…');
            }
            line
        })
        .collect()
}

pub fn markdown(doc: &Document) -> String {
    fn escape(value: &str) -> String {
        value
            .replace('\\', "\\\\")
            .replace('|', "\\|")
            .replace('\n', " ")
    }
    let mut result = format!("# {}\n", escape(&doc.title));
    for id in doc.order() {
        result.push_str(&format!("\n## {}\n\n", escape(doc.diagram_title(id))));
        let diagram = &doc.diagrams[&id];
        let numbers = doc.numbers(id);
        let label = |end| match end {
            End::Node { id } => escape(&diagram.nodes[&id].title),
            End::Boundary { parent_flow } => {
                let parent = doc.parent(id).unwrap();
                escape(doc.flow_label(parent, &doc.diagrams[&parent].flows[&parent_flow]))
            }
        };
        for (&id, node) in &diagram.nodes {
            let code = numbers
                .get(&id)
                .map(|n| format!("{n} "))
                .unwrap_or_default();
            result.push_str(&format!("- {}{}\n", code, escape(&node.title)));
        }
        for flow in diagram.flows.values() {
            result.push_str(&format!(
                "- {} → {} — {}\n",
                label(flow.from),
                label(flow.to),
                escape(doc.flow_label(id, flow))
            ));
        }
    }
    result.push_str(
        "\n## Data dictionary\n\n| Name | Description | Components |\n| --- | --- | --- |\n",
    );
    for definition in doc.dictionary.values() {
        let components = definition
            .components
            .iter()
            .map(|id| escape(&doc.dictionary[id].name))
            .collect::<Vec<_>>();
        result.push_str(&format!(
            "| {} | {} | {} |\n",
            escape(&definition.name),
            escape(&definition.description),
            components.join(", ")
        ));
    }
    result
}
