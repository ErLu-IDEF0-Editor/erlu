use crate::*;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

pub const DISPLAY_FONT: &[u8] = include_bytes!("../../../assets/fonts/ErluDisplay.ttf");
pub const CJK_FONT: &[u8] = include_bytes!("../../../assets/fonts/NotoSansSC.ttf");
pub const FONT: &[u8] = include_bytes!("../../../assets/fonts/DejaVuSans.ttf");
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Primitive {
    Rect {
        min: Point,
        max: Point,
        stroke: String,
        fill: String,
        width: f64,
        radius: f64,
        dashed: bool,
    },
    Line {
        points: Vec<Point>,
        color: String,
        width: f64,
        dashed: bool,
    },
    Text {
        pos: Point,
        text: String,
        size: f64,
        color: String,
        center: bool,
        right: bool,
    },
    Circle {
        pos: Point,
        radius: f64,
        color: String,
        fill: String,
    },
}
#[derive(Clone, Debug)]
pub struct Scene {
    pub size: Point,
    pub primitives: Vec<Primitive>,
}
fn rect(min: Point, max: Point, stroke: &str, fill: &str, width: f64) -> Primitive {
    Primitive::Rect {
        min,
        max,
        stroke: stroke.into(),
        fill: fill.into(),
        width,
        radius: 0.,
        dashed: false,
    }
}
fn text(pos: Point, value: impl Into<String>, size: f64, color: &str, center: bool) -> Primitive {
    Primitive::Text {
        pos,
        text: value.into(),
        size,
        color: color.into(),
        center,
        right: false,
    }
}
fn line(points: Vec<Point>, color: &str, width: f64) -> Primitive {
    Primitive::Line {
        points,
        color: color.into(),
        width,
        dashed: false,
    }
}
pub fn project(p: Point, a: Point, b: Point) -> Point {
    let v = b - a;
    let n = v.x * v.x + v.y * v.y;
    let t = if n < 1e-10 {
        0.
    } else {
        ((p.x - a.x) * v.x + (p.y - a.y) * v.y) / n
    }
    .clamp(0., 1.);
    Point::new(a.x + v.x * t, a.y + v.y * t)
}
pub fn nearest_on_path(p: Point, path: &[Point]) -> Option<(Point, usize, f64)> {
    path.windows(2)
        .enumerate()
        .map(|(i, w)| {
            let q = project(p, w[0], w[1]);
            (q, i, q.distance(p))
        })
        .min_by(|a, b| a.2.total_cmp(&b.2))
}
fn direction(ep: &Endpoint, from: bool) -> Point {
    match ep.anchor {
        Anchor::Block { port, .. } => match port {
            Port::Input => Point::new(-1., 0.),
            Port::Output => Point::new(1., 0.),
            Port::Control => Point::new(0., -1.),
            Port::Mechanism => Point::new(0., 1.),
        },
        Anchor::Boundary { port, .. } => match port {
            Port::Input => Point::new(1., 0.),
            Port::Output => Point::new(-1., 0.),
            Port::Control => Point::new(0., 1.),
            Port::Mechanism => Point::new(0., -1.),
        },
        Anchor::Junction { .. } => Point::new(if from { 1. } else { -1. }, 0.),
    }
}
pub fn route(doc: &Document, d: Id, a: &Arrow) -> Vec<Point> {
    let Some(start) = doc.endpoint(d, &a.from) else {
        return vec![];
    };
    let Some(end) = doc.endpoint(d, &a.to) else {
        return vec![];
    };
    if a.interface.as_ref().is_some_and(|i| i.stub) {
        return vec![start, start];
    }
    let sd = direction(&a.from, true);
    let ed = direction(&a.to, false);
    let s = Point::new(start.x + sd.x * 4., start.y + sd.y * 4.);
    let e = Point::new(end.x + ed.x * 4., end.y + ed.y * 4.);
    let obstacles = doc.diagrams[&d]
        .blocks
        .values()
        .map(|b| (b.pos - Point::new(1.5, 1.5), b.pos + Point::new(54.5, 28.)))
        .collect::<Vec<_>>();
    let mut path = vec![start, s];
    let mut anchors = a.bends.clone();
    anchors.push(e);
    let mut current = s;
    for target in anchors {
        let leg = manhattan(current, target, &obstacles);
        path.extend(leg.into_iter().skip(1));
        current = target;
    }
    path.push(end);
    simplify(path)
}
fn simplify(path: Vec<Point>) -> Vec<Point> {
    let mut out: Vec<Point> = vec![];
    for p in path {
        if out.last().is_some_and(|q| q.distance(p) < 1e-6) {
            continue;
        }
        if out.len() > 1 {
            let a = out[out.len() - 2];
            let b = out[out.len() - 1];
            if ((b.x - a.x) * (p.y - b.y) - (b.y - a.y) * (p.x - b.x)).abs() < 1e-8
                && ((b.x - a.x) * (p.x - b.x) + (b.y - a.y) * (p.y - b.y)) >= 0.
            {
                out.pop();
            }
        }
        out.push(p);
    }
    out
}
fn blocked(a: Point, b: Point, obs: &[(Point, Point)]) -> bool {
    obs.iter().any(|(lo, hi)| {
        if (a.x - b.x).abs() < 1e-7 {
            a.x > lo.x && a.x < hi.x && a.y.max(b.y) > lo.y && a.y.min(b.y) < hi.y
        } else {
            a.y > lo.y && a.y < hi.y && a.x.max(b.x) > lo.x && a.x.min(b.x) < hi.x
        }
    })
}
fn manhattan(start: Point, end: Point, obs: &[(Point, Point)]) -> Vec<Point> {
    let mut xs = vec![start.x, end.x];
    let mut ys = vec![start.y, end.y];
    for (lo, hi) in obs {
        xs.extend([lo.x, hi.x]);
        ys.extend([lo.y, hi.y]);
    }
    xs.sort_by(f64::total_cmp);
    ys.sort_by(f64::total_cmp);
    xs.dedup();
    ys.dedup();
    let w = xs.len();
    let n = w * ys.len();
    let index = |p: Point| {
        ys.iter().position(|y| *y == p.y).unwrap() * w + xs.iter().position(|x| *x == p.x).unwrap()
    };
    let at = |i: usize| Point::new(xs[i % w], ys[i / w]);
    let s = index(start);
    let target = index(end);
    let mut costs = vec![u64::MAX; n * 3];
    let mut prev = vec![None; n * 3];
    let mut queue = BinaryHeap::new();
    costs[s * 3] = 0;
    queue.push(Reverse((0u64, s * 3)));
    let mut finish = None;
    while let Some(Reverse((cost, state))) = queue.pop() {
        if cost != costs[state] {
            continue;
        }
        let node = state / 3;
        let dir = state % 3;
        if node == target {
            finish = Some(state);
            break;
        }
        let x = node % w;
        let y = node / w;
        let neighbors = [
            (x.checked_sub(1).map(|_| node - 1), 1),
            (if x + 1 < w { Some(node + 1) } else { None }, 1),
            (y.checked_sub(1).map(|_| node - w), 2),
            (if node + w < n { Some(node + w) } else { None }, 2),
        ];
        for (next, nd) in neighbors {
            let Some(next) = next else { continue };
            let a = at(node);
            let b = at(next);
            if blocked(a, b, obs) {
                continue;
            }
            let next_state = next * 3 + nd;
            let nc = cost
                + (a.distance(b) * 1000.) as u64
                + if dir != 0 && dir != nd { 6000 } else { 0 };
            if nc < costs[next_state] {
                costs[next_state] = nc;
                prev[next_state] = Some(state);
                queue.push(Reverse((nc, next_state)));
            }
        }
    }
    if let Some(mut state) = finish {
        let mut path = vec![];
        loop {
            path.push(at(state / 3));
            if let Some(p) = prev[state] {
                state = p;
            } else {
                break;
            }
        }
        path.reverse();
        return simplify(path);
    }
    vec![start, Point::new(end.x, start.y), end]
}
pub fn scene(doc: &Document, d: Id, dark: bool, page: Option<usize>) -> Scene {
    let size = doc.page();
    let min = Point::new(-size.x / 2., -size.y / 2.);
    let max = Point::new(size.x / 2., size.y / 2.);
    let (lo, hi) = doc.content();
    let fg = if dark { "#ebebeb" } else { "#141414" };
    let bg = if dark { "#1a1a1a" } else { "#ffffff" };
    let mut frame = rect(lo, hi, fg, "none", 0.3);
    if let Primitive::Rect { dashed, .. } = &mut frame {
        *dashed = true;
    }
    let mut p = vec![rect(min, max, fg, bg, 0.3), frame];
    if doc.meta.with_header {
        let bottom = lo.y;
        let split = min.x + size.x * 0.62;
        let row = (bottom - min.y) / 4.;
        let info = split + size.x * 0.38 * 0.55;
        for x in [split, info] {
            p.push(line(
                vec![Point::new(x, min.y), Point::new(x, bottom)],
                fg,
                0.25,
            ));
        }
        for n in 1..4 {
            let y = min.y + row * n as f64;
            p.push(line(
                vec![Point::new(min.x, y), Point::new(info, y)],
                fg,
                0.2,
            ));
        }
        let used = if doc.meta.organization.is_empty() {
            format!("USED AT: {}", doc.meta.used_at)
        } else {
            format!("ORGANIZATION: {}", doc.meta.organization)
        };
        let project = if doc.meta.title.is_empty() {
            format!("PROJECT: {}", doc.meta.project)
        } else {
            format!("TITLE: {}", doc.meta.title)
        };
        for (i, t) in [
            used,
            format!("AUTHOR: {}", doc.meta.author),
            project,
            format!("NOTES: {}", doc.meta.notes),
        ]
        .into_iter()
        .enumerate()
        {
            p.push(text(
                Point::new(min.x + 1.6, min.y + row * (i as f64 + 0.5)),
                t,
                2.5,
                fg,
                false,
            ));
        }
        p.push(text(
            Point::new(min.x + size.x * 0.48, min.y + row * 0.5),
            format!("DATE: {}", doc.meta.date),
            2.5,
            fg,
            false,
        ));
        p.push(text(
            Point::new(min.x + size.x * 0.48, min.y + row * 1.5),
            format!("REV: {}", doc.meta.revision),
            2.5,
            fg,
            false,
        ));
        for (i, (label, state)) in [
            ("WORKING", DiagramState::Working),
            ("DRAFT", DiagramState::Draft),
            ("RECOMMENDED", DiagramState::Recommended),
            ("PUBLICATION", DiagramState::Publication),
        ]
        .into_iter()
        .enumerate()
        {
            let y = min.y + row * i as f64;
            p.push(rect(
                Point::new(split, y),
                Point::new(split + row, y + row),
                fg,
                if doc.diagrams[&d].state == state {
                    fg
                } else {
                    bg
                },
                0.2,
            ));
            p.push(text(
                Point::new(split + row + 1.6, y + row * 0.5),
                label,
                2.5,
                fg,
                false,
            ));
        }
        let info_row = (bottom - min.y) / 3.;
        for i in 1..3 {
            let y = min.y + info_row * i as f64;
            p.push(line(
                vec![Point::new(info, y), Point::new(max.x, y)],
                fg,
                0.2,
            ));
        }
        for (i, t) in [
            format!("READER: {}", doc.meta.reader),
            format!("DATE: {}", doc.meta.reader_date),
            format!("CONTEXT: {}", doc.meta.context),
        ]
        .into_iter()
        .enumerate()
        {
            p.push(text(
                Point::new(info + 1.6, min.y + info_row * (i as f64 + 0.5)),
                t,
                2.5,
                fg,
                false,
            ));
        }
    }
    if doc.meta.with_footer {
        let y = hi.y;
        for t in [0.2, 0.8] {
            p.push(line(
                vec![
                    Point::new(min.x + size.x * t, y),
                    Point::new(min.x + size.x * t, max.y),
                ],
                fg,
                0.2,
            ));
        }
        let cy = (y + max.y) / 2.;
        p.push(text(
            Point::new(min.x + 2., cy),
            format!(
                "NODE: {}",
                if doc.meta.footer_node_override.is_empty() {
                    doc.node(d)
                } else {
                    doc.meta.footer_node_override.clone()
                }
            ),
            2.6,
            fg,
            false,
        ));
        p.push(text(Point::new(0., cy), doc.title(d), 3., fg, true));
        p.push(text(
            Point::new(min.x + size.x * 0.8 + 2., cy),
            format!(
                "NUMBER: {}{}",
                if doc.meta.footer_number_override.is_empty() {
                    &doc.meta.number
                } else {
                    &doc.meta.footer_number_override
                },
                page.map(|p| format!(" / {p}")).unwrap_or_default()
            ),
            2.6,
            fg,
            false,
        ));
    }
    let diagram = &doc.diagrams[&d];
    let mut captions = Vec::new();
    for a in diagram.arrows.values() {
        let color = a.color.as_deref().unwrap_or(fg);
        let path = route(doc, d, a);
        let mut arrow_line = line(path.clone(), color, 0.35);
        if let Primitive::Line { dashed, .. } = &mut arrow_line {
            *dashed = a.call_ref.is_some();
        }
        p.push(arrow_line);
        if path.len() > 1 {
            let stub = a.interface.as_ref().is_some_and(|i| i.stub);
            let tip = *path.last().unwrap();
            if stub && !(a.from.tunneled || a.to.tunneled) {
                p.push(Primitive::Circle {
                    pos: tip,
                    radius: 1.8,
                    color: color.into(),
                    fill: bg.into(),
                });
            } else if !stub && !matches!(a.to.anchor, Anchor::Junction { .. }) {
                let from = path[path.len() - 2];
                let v = tip - from;
                let n = v.x.hypot(v.y).max(1e-9);
                let u = Point::new(v.x / n, v.y / n);
                p.push(line(
                    vec![
                        Point::new(tip.x - u.x * 2.4 - u.y, tip.y - u.y * 2.4 + u.x),
                        tip,
                        Point::new(tip.x - u.x * 2.4 + u.y, tip.y - u.y * 2.4 - u.x),
                    ],
                    color,
                    0.35,
                ));
            }
            for (tunneled, tip, neighbor) in [
                (a.from.tunneled, path[0], path[1]),
                (a.to.tunneled, tip, path[path.len() - 2]),
            ] {
                if tunneled && !stub {
                    let v = neighbor - tip;
                    let n = v.x.hypot(v.y).max(1e-9);
                    let u = Point::new(v.x / n, v.y / n);
                    for sign in [-1., 1.] {
                        p.push(line(
                            vec![
                                Point::new(
                                    tip.x - u.y * 2. * sign + u.x,
                                    tip.y + u.x * 2. * sign + u.y,
                                ),
                                Point::new(tip.x - u.y * 2. * sign, tip.y + u.x * 2. * sign),
                                Point::new(
                                    tip.x - u.y * 2. * sign - u.x,
                                    tip.y + u.x * 2. * sign - u.y,
                                ),
                            ],
                            color,
                            0.3,
                        ));
                    }
                }
            }
        }
        if !a.label_hidden && !a.label.is_empty() {
            let mut caption = text(
                label_position(&path, a.label_offset),
                a.label.clone(),
                2.8,
                color,
                true,
            );
            if let Some(i) = &a.interface
                && i.stub
                && let Primitive::Text {
                    pos, center, right, ..
                } = &mut caption
            {
                *pos = path[0] + a.label_offset;
                match i.port {
                    Port::Input => {
                        pos.x += 2.65;
                        *center = false;
                    }
                    Port::Output => {
                        pos.x -= 2.65;
                        *center = false;
                        *right = true;
                    }
                    Port::Control => pos.y += 4.,
                    Port::Mechanism => pos.y -= 4.,
                }
            }
            if let Primitive::Text {
                pos,
                text,
                size,
                center,
                right,
                ..
            } = &caption
            {
                let extent = text_extent(text, *size);
                let x = pos.x
                    - if *right {
                        extent.x
                    } else if *center {
                        extent.x / 2.
                    } else {
                        0.
                    };
                captions.push(rect(
                    Point::new(x - 0.6, pos.y - extent.y / 2. - 0.3),
                    Point::new(x + extent.x + 0.6, pos.y + extent.y / 2. + 0.3),
                    "none",
                    bg,
                    0.,
                ));
            }
            captions.push(caption);
        }
    }
    p.extend(captions);
    for j in diagram.junctions.values() {
        p.push(Primitive::Circle {
            pos: j.pos,
            radius: 0.8,
            color: fg.into(),
            fill: fg.into(),
        });
    }
    for b in diagram.blocks.values() {
        let color = b.color.as_deref().unwrap_or(fg);
        let mut block = rect(b.pos, b.pos + Point::new(53., 26.5), color, bg, 0.4);
        if let Primitive::Rect { radius, .. } = &mut block {
            *radius = 1.6;
        }
        p.push(block);
        let lines = wrap(&b.title, 27);
        for (i, t) in lines.iter().enumerate() {
            p.push(text(
                b.pos + Point::new(2.65, 4. + i as f64 * 3.5),
                t.clone(),
                3.,
                color,
                false,
            ));
        }
        p.push(text(
            b.pos + Point::new(48., 23.3),
            b.number.clone(),
            2.5,
            color,
            true,
        ));
        if let Some(price) = b.price {
            let num = format!("{price:.2}");
            let s = doc
                .meta
                .currency_placement
                .replace('1', &num)
                .replace('?', &doc.meta.currency);
            p.push(text(b.pos + Point::new(3., 23.3), s, 2.5, color, false));
        }
        if !b.child.is_some_and(|c| !doc.diagrams[&c].blocks.is_empty()) {
            p.push(line(
                vec![b.pos + Point::new(0., 3.2), b.pos + Point::new(3.2, 0.)],
                color,
                0.25,
            ));
        }
    }
    Scene {
        size,
        primitives: p,
    }
}
fn text_extent(text: &str, size: f64) -> Point {
    let face = ttf_parser::Face::parse(FONT, 0).expect("Bundled font");
    let cjk = ttf_parser::Face::parse(CJK_FONT, 0).expect("Bundled CJK font");
    let width = text
        .chars()
        .filter_map(|c| {
            let f = if face.glyph_index(c).is_some() {
                &face
            } else {
                &cjk
            };
            f.glyph_index(c)
                .and_then(|g| f.glyph_hor_advance(g))
                .map(|advance| advance as f64 / f.units_per_em() as f64)
        })
        .sum::<f64>()
        * size;
    Point::new(width, size * 1.2)
}
pub fn arrow_label_position(path: &[Point], a: &Arrow) -> Point {
    if let Some(i) = &a.interface
        && i.stub
        && !path.is_empty()
    {
        let mut pos = path[0] + a.label_offset;
        let half_width = text_extent(&a.label, 2.8).x / 2.;
        match i.port {
            Port::Input => pos.x += 2.65 + half_width,
            Port::Output => pos.x -= 2.65 + half_width,
            Port::Control => pos.y += 4.,
            Port::Mechanism => pos.y -= 4.,
        }
        pos
    } else {
        label_position(path, a.label_offset)
    }
}
pub fn label_position(path: &[Point], offset: Point) -> Point {
    if path.is_empty() {
        return offset;
    }
    let index = path
        .windows(2)
        .enumerate()
        .max_by(|(_, a), (_, b)| a[0].distance(a[1]).total_cmp(&b[0].distance(b[1])))
        .map(|(i, _)| i)
        .unwrap_or(0);
    let b = path.get(index + 1).copied().unwrap_or(path[index]);
    Point::new((path[index].x + b.x) / 2., (path[index].y + b.y) / 2. - 2.) + offset
}
fn wrap(s: &str, width: usize) -> Vec<String> {
    let mut lines = vec![];
    for raw in s.lines() {
        let mut line = String::new();
        for word in raw.split_whitespace() {
            if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
                lines.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        lines.push(line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
pub fn scene_localized(
    doc: &Document,
    d: Id,
    dark: bool,
    page: Option<usize>,
    language: &str,
) -> Scene {
    let mut result = scene(doc, d, dark, page);
    if language != "en" {
        let all = crate::localization::catalog();
        for primitive in &mut result.primitives {
            if let Primitive::Text { text, .. } = primitive {
                if let Some(v) = all[language][text.as_str()].as_str() {
                    *text = v.into();
                } else if let Some((prefix, value)) = text.split_once(": ") {
                    let key = format!("{prefix}: ");
                    let translated = all[language][&key]
                        .as_str()
                        .or_else(|| all[language][format!("{prefix}:")].as_str());
                    if let Some(t) = translated {
                        *text = format!("{} {}", t.trim_end(), value);
                    }
                }
            }
        }
    }
    result
}
pub fn svg(doc: &Document, d: Id, dark: bool, page: Option<usize>) -> String {
    svg_localized(doc, d, dark, page, "en")
}
pub fn svg_localized(
    doc: &Document,
    d: Id,
    dark: bool,
    page: Option<usize>,
    language: &str,
) -> String {
    let scene = scene_localized(doc, d, dark, page, language);
    let mut out = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}mm" height="{}mm" viewBox="{} {} {} {}">"#,
        scene.size.x,
        scene.size.y,
        -scene.size.x / 2.,
        -scene.size.y / 2.,
        scene.size.x,
        scene.size.y
    );
    for p in scene.primitives {
        out.push_str(&match p{Primitive::Rect{min,max,stroke,fill,width,radius,dashed}=>format!(r#"<rect rx="{radius}" stroke-dasharray="{}" x="{}" y="{}" width="{}" height="{}" stroke="{stroke}" fill="{fill}" stroke-width="{width}"/>"#,if dashed{"2 1.2"}else{"none"},min.x,min.y,max.x-min.x,max.y-min.y),Primitive::Line{points,color,width,dashed}=>format!(r#"<polyline stroke-dasharray="{}" points="{}" stroke="{color}" fill="none" stroke-width="{width}"/>"#,if dashed{"2 1.2"}else{"none"},points.iter().map(|p|format!("{},{}",p.x,p.y)).collect::<Vec<_>>().join(" ")),Primitive::Circle{pos,radius,color,fill}=>format!(r#"<circle cx="{}" cy="{}" r="{radius}" stroke="{color}" fill="{fill}" stroke-width="0.3"/>"#,pos.x,pos.y),Primitive::Text{pos,text,size,color,center,right}=>format!(r#"<text x="{}" y="{}" font-family="DejaVu Sans" font-size="{size}" fill="{color}" text-anchor="{}" dominant-baseline="middle">{}</text>"#,pos.x,pos.y,if right{"end"}else if center{"middle"}else{"start"},esc(&text))});
    }
    out.push_str("</svg>");
    out
}
pub fn markdown(doc: &Document) -> String {
    fn escape(s: &str) -> String {
        s.replace('|', "\\|").replace('\n', "<br>")
    }
    fn visit(doc: &Document, id: Id, out: &mut String, depth: usize) {
        out.push_str(&format!(
            "\n{} {} — {}\n\n",
            "#".repeat((depth + 2).min(6)),
            doc.node(id),
            doc.title(id)
        ));
        out.push_str("| Number | Function | Cost |\n| --- | --- | --- |\n");
        let d = &doc.diagrams[&id];
        for b in d.blocks.values() {
            out.push_str(&format!(
                "| {} | {} | {} |\n",
                b.number,
                escape(&b.title),
                b.price
                    .map(|p| format!("{p:.2} {}", doc.meta.currency))
                    .unwrap_or_default()
            ));
        }
        out.push_str("\nFlows:\n\n");
        for a in d.arrows.values() {
            let endpoint = |e: &Endpoint| match e.anchor {
                Anchor::Block { id, port, .. } => format!("{} ({port:?})", d.blocks[&id].number),
                Anchor::Boundary { port, .. } => format!("Boundary ({port:?})"),
                Anchor::Junction { id } => format!("Junction {id}"),
            };
            out.push_str(&format!(
                "- {} → {}: {}{}\n",
                endpoint(&a.from),
                endpoint(&a.to),
                a.label,
                if a.from.tunneled || a.to.tunneled {
                    " (tunneled)"
                } else {
                    ""
                }
            ));
        }
        for b in d.blocks.values() {
            if let Some(c) = b.child {
                visit(doc, c, out, depth + 1);
            }
        }
    }
    let mut out = format!(
        "# {}\n\n{} — {}\n",
        doc.meta.title, doc.meta.organization, doc.meta.author
    );
    visit(doc, doc.root, &mut out, 0);
    out
}
pub fn pdf(doc: &Document, diagrams: &[Id], numbers: bool, dark: bool) -> crate::Result<Vec<u8>> {
    pdf_localized(doc, diagrams, numbers, dark, "en")
}
pub fn pdf_localized(
    doc: &Document,
    diagrams: &[Id],
    numbers: bool,
    dark: bool,
    language: &str,
) -> crate::Result<Vec<u8>> {
    // PDF backend is shared by native and wasm; no system font or filesystem lookup.
    let mut options = svg2pdf::usvg::Options::default();
    options.fontdb_mut().load_font_data(FONT.to_vec());
    options.fontdb_mut().load_font_data(CJK_FONT.to_vec());
    let mut pdf = pdf_writer::Pdf::new();
    let catalog = pdf_writer::Ref::new(1);
    let pages = pdf_writer::Ref::new(2);
    let mut next = 3;
    let mut kids = vec![];
    for (i, &d) in diagrams.iter().enumerate() {
        let xml = svg_localized(doc, d, dark, numbers.then_some(i + 1), language);
        let tree = svg2pdf::usvg::Tree::from_str(&xml, &options)
            .map_err(|e| error("export_failed", e.to_string()))?;
        let (chunk, root) = svg2pdf::to_chunk(&tree, svg2pdf::ConversionOptions::default())
            .map_err(|e| error("export_failed", e.to_string()))?;
        let mut map = std::collections::HashMap::new();
        let mut allocate = |r| {
            *map.entry(r).or_insert_with(|| {
                let n = pdf_writer::Ref::new(next);
                next += 1;
                n
            })
        };
        let xobject = allocate(root);
        let chunk = chunk.renumber(&mut allocate);
        pdf.extend(&chunk);
        let page_id = pdf_writer::Ref::new(next);
        let content_id = pdf_writer::Ref::new(next + 1);
        next += 2;
        kids.push(page_id);
        let size = doc.page();
        let w = (size.x * 72. / 25.4) as f32;
        let h = (size.y * 72. / 25.4) as f32;
        {
            let mut page = pdf.page(page_id);
            page.parent(pages)
                .media_box(pdf_writer::Rect::new(0., 0., w, h))
                .contents(content_id);
            page.resources()
                .x_objects()
                .pair(pdf_writer::Name(b"Diagram"), xobject);
        }
        let mut content = pdf_writer::Content::new();
        content.save_state();
        content.transform([w, 0., 0., h, 0., 0.]);
        content.x_object(pdf_writer::Name(b"Diagram"));
        content.restore_state();
        pdf.stream(content_id, &content.finish());
    }
    pdf.catalog(catalog).pages(pages);
    pdf.pages(pages)
        .kids(kids.iter().copied())
        .count(kids.len() as i32);
    Ok(pdf.finish())
}
