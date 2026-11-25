use super::*;

impl ErluApp {
    pub(super) fn keyboard(&mut self, ctx: &egui::Context, s: &Engine) {
        if ctx.text_edit_focused() {
            return;
        }
        let pressed = |modifiers, key| ctx.input_mut(|i| i.consume_key(modifiers, key));
        use egui::Modifiers;
        if pressed(Modifiers::NONE, Key::Escape) {
            self.drag = None;
            self.pending_call = None;
            self.box_rect = None;
        }
        if pressed(Modifiers::NONE, Key::Delete) || pressed(Modifiers::NONE, Key::Backspace) {
            self.execute(Command::ObjectsDelete {
                ids: s.view.selection.iter().copied().collect(),
            });
        }
        if pressed(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)
            || pressed(Modifiers::COMMAND, Key::Y)
        {
            self.execute(Command::Redo {});
        } else if pressed(Modifiers::COMMAND, Key::Z) {
            self.execute(Command::Undo {});
        }
        if pressed(Modifiers::COMMAND | Modifiers::SHIFT, Key::S) {
            self.save(true);
        } else if pressed(Modifiers::COMMAND, Key::S) {
            self.save(false);
        }
        if pressed(Modifiers::COMMAND, Key::O) {
            self.request(Pending::Open);
        }
        if pressed(Modifiers::COMMAND, Key::N) {
            self.request(Pending::New);
        }
        if pressed(Modifiers::NONE, Key::G) {
            self.execute(Command::GridSet {
                enabled: !s.view.grid.enabled,
                spacing: None,
            });
        }
        if pressed(Modifiers::NONE, Key::F) {
            self.execute(Command::ViewFit {});
        }
        if (pressed(Modifiers::CTRL, Key::ArrowDown) || pressed(Modifiers::CTRL, Key::PageDown))
            && s.view.selection.len() == 1
        {
            let id = *s.view.selection.first().unwrap();
            if s.document.block(id).is_some() {
                self.execute(Command::Decompose { block: id });
            }
        }
        if pressed(Modifiers::CTRL, Key::ArrowUp) || pressed(Modifiers::CTRL, Key::PageUp) {
            self.execute(Command::NavigateUp {});
        }
    }

    pub(super) fn canvas(&mut self, ui: &mut egui::Ui, s: &Engine) {
        let (response, painter) =
            ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
        ui_target("canvas", &response, None);
        let rect = response.rect;
        let page = s.document.page();
        let base = ((rect.width() - 40.) / page.x as f32)
            .min((rect.height() - 40.) / page.y as f32)
            .max(0.1);
        let scale = base * s.view.zoom as f32;
        let to_screen = |p: Point| {
            rect.center()
                + Vec2::new(
                    (p.x - s.view.center.x) as f32 * scale,
                    (p.y - s.view.center.y) as f32 * scale,
                )
        };
        let from_screen = |p: Pos2| {
            Point::new(
                (p.x - rect.center().x) as f64 / scale as f64 + s.view.center.x,
                (p.y - rect.center().y) as f64 / scale as f64 + s.view.center.y,
            )
        };
        let pointer = response
            .hover_pos()
            .or_else(|| ui.input(|i| i.pointer.interact_pos()));
        let point = pointer.map(from_screen);
        self.cursor = point;
        let mut preview = s.document.clone();
        if let (Some(Drag::Move { start, positions }), Some(p)) = (&self.drag, point) {
            let delta = drag_delta(positions, p - *start, s.view.grid);
            for (id, old) in positions {
                if let Some(d) = preview.diagram_of(*id) {
                    if let Some(b) = preview.diagrams.get_mut(&d).unwrap().blocks.get_mut(id) {
                        b.pos = *old + delta;
                    } else if let Some(j) =
                        preview.diagrams.get_mut(&d).unwrap().junctions.get_mut(id)
                    {
                        j.pos = *old + delta;
                    }
                }
            }
            routing::normalize_moves(
                &s.document,
                &mut preview,
                &positions.keys().copied().collect(),
            );
        }
        if let (
            Some(Drag::Segment {
                id,
                index,
                path,
                start,
            }),
            Some(p),
        ) = (&self.drag, point)
        {
            let arrow = routing::drag_segment(
                &s.document,
                s.view.diagram,
                &s.document.diagrams[&s.view.diagram].arrows[id],
                path.clone(),
                *index,
                p - *start,
                s.view.grid,
            );
            preview
                .diagrams
                .get_mut(&s.view.diagram)
                .unwrap()
                .arrows
                .insert(*id, arrow);
        }
        if let (Some(Drag::Label { id, start, offset }), Some(p)) = (&self.drag, point) {
            preview
                .diagrams
                .get_mut(&s.view.diagram)
                .unwrap()
                .arrows
                .get_mut(id)
                .unwrap()
                .label_offset = *offset + p - *start;
        }
        let dark = match s.view.theme {
            Theme::Dark => true,
            Theme::Light => false,
            Theme::System => ui.visuals().dark_mode,
        };
        let scene = render::scene_localized(&preview, s.view.diagram, dark, None, &self.language);
        for (primitive_index, primitive) in scene.primitives.into_iter().enumerate() {
            match primitive {
                Primitive::Rect {
                    min,
                    max,
                    stroke,
                    fill,
                    width,
                    radius,
                    dashed,
                } => {
                    let r = Rect::from_two_pos(to_screen(min), to_screen(max));
                    if fill != "none" {
                        painter.rect_filled(r, radius as f32 * scale, parse_color(&fill));
                    }
                    if dashed {
                        for (a, b) in [
                            (r.left_top(), r.right_top()),
                            (r.right_top(), r.right_bottom()),
                            (r.right_bottom(), r.left_bottom()),
                            (r.left_bottom(), r.left_top()),
                        ] {
                            painter.add(egui::Shape::dashed_line(
                                &[a, b],
                                Stroke::new((width as f32 * scale).max(0.7), parse_color(&stroke)),
                                2. * scale,
                                1.2 * scale,
                            ));
                        }
                    } else {
                        painter.rect_stroke(
                            r,
                            radius as f32 * scale,
                            Stroke::new((width as f32 * scale).max(0.7), parse_color(&stroke)),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
                Primitive::Line {
                    points,
                    color,
                    width,
                    dashed,
                } => {
                    if dashed {
                        painter.add(egui::Shape::dashed_line(
                            &points.into_iter().map(to_screen).collect::<Vec<_>>(),
                            Stroke::new((width as f32 * scale).max(0.7), parse_color(&color)),
                            2. * scale,
                            1.2 * scale,
                        ));
                    } else {
                        painter.add(egui::Shape::line(
                            points.into_iter().map(to_screen).collect(),
                            Stroke::new((width as f32 * scale).max(0.7), parse_color(&color)),
                        ));
                    }
                }
                Primitive::Text {
                    pos,
                    text,
                    size,
                    color,
                    center,
                    right,
                } => {
                    painter.text(
                        to_screen(pos),
                        if right {
                            egui::Align2::RIGHT_CENTER
                        } else if center {
                            egui::Align2::CENTER_CENTER
                        } else {
                            egui::Align2::LEFT_CENTER
                        },
                        localization::display_text(&text),
                        egui::FontId::proportional((size as f32 * scale).max(5.)),
                        parse_color(&color),
                    );
                }
                Primitive::Circle {
                    pos,
                    radius,
                    color,
                    fill,
                } => {
                    painter.circle(
                        to_screen(pos),
                        radius as f32 * scale,
                        parse_color(&fill),
                        Stroke::new(0.8, parse_color(&color)),
                    );
                }
            }

            if primitive_index == 0 && s.view.grid.enabled {
                let (lo, hi) = s.document.content();
                let spacing = s.view.grid.spacing;
                if spacing.is_finite() && spacing >= 1.0 {
                    let step = spacing * (10.0 / (spacing * scale as f64)).ceil().max(1.0);
                    let color = if dark {
                        Color32::from_gray(70)
                    } else {
                        Color32::from_gray(205)
                    };
                    let mut x = (lo.x / step).ceil() * step;
                    while x < hi.x {
                        let mut y = (lo.y / step).ceil() * step;
                        while y < hi.y {
                            painter.circle_filled(to_screen(Point::new(x, y)), 0.8, color);
                            y += step;
                        }
                        x += step;
                    }
                }
            }
        }
        let diag = &s.document.diagrams[&s.view.diagram];
        for id in &s.view.selection {
            if let Some(b) = preview.diagrams[&s.view.diagram].blocks.get(id) {
                painter.rect_stroke(
                    Rect::from_two_pos(to_screen(b.pos), to_screen(b.pos + Point::new(53., 26.5)))
                        .expand(2.),
                    0.,
                    Stroke::new(1.5, Color32::from_rgb(60, 140, 240)),
                    egui::StrokeKind::Outside,
                );
            }
            if let Some(a) = preview.diagrams[&s.view.diagram].arrows.get(id) {
                let path = render::route(&preview, s.view.diagram, a);
                painter.add(egui::Shape::line(
                    path.iter().copied().map(to_screen).collect(),
                    Stroke::new(1.5, Color32::from_rgb(70, 150, 240)),
                ));
                for ep in [&a.from, &a.to] {
                    if let Some(p) = s.document.endpoint(s.view.diagram, ep) {
                        painter.circle_stroke(
                            to_screen(p),
                            4.,
                            Stroke::new(1., Color32::LIGHT_BLUE),
                        );
                    }
                }
            }
        }
        if let Some(p) = point {
            match &self.drag {
                Some(Drag::Connect { from, .. }) => {
                    let mut doc = preview.clone();
                    let mut target = hit_endpoint(s, p, 10.0 / scale as f64).unwrap_or_else(|| {
                        doc.diagrams
                            .get_mut(&s.view.diagram)
                            .unwrap()
                            .junctions
                            .insert(
                                u64::MAX,
                                Junction {
                                    id: u64::MAX,
                                    pos: s.view.grid.snap(p),
                                    extra: Extra::new(),
                                },
                            );
                        Endpoint::new(Anchor::Junction { id: u64::MAX })
                    });
                    routing::snap_endpoint(&doc, s.view.diagram, &mut target, s.view.grid);
                    let mut arrow = Arrow {
                        id: 0,
                        from: from.clone(),
                        to: target,
                        bends: vec![],
                        label: String::new(),
                        label_offset: Point::default(),
                        label_hidden: false,
                        label_source: None,
                        color: None,
                        interface: None,
                        call_ref: None,
                        extra: Extra::new(),
                    };
                    routing::align_ports(&doc, s.view.diagram, &mut arrow);
                    let points = render::route(&doc, s.view.diagram, &arrow);
                    painter.add(egui::Shape::line(
                        points.into_iter().map(to_screen).collect(),
                        Stroke::new(1.5, Color32::LIGHT_BLUE),
                    ));
                }
                Some(Drag::Branch { point: start, .. }) => {
                    painter.line_segment(
                        [to_screen(*start), to_screen(p)],
                        Stroke::new(1.5, Color32::LIGHT_BLUE),
                    );
                }
                Some(Drag::Box { start }) => {
                    let r = Rect::from_two_pos(to_screen(*start), to_screen(p));
                    painter.rect_stroke(
                        r,
                        0.,
                        Stroke::new(1., Color32::LIGHT_BLUE),
                        egui::StrokeKind::Inside,
                    );
                }
                _ => {}
            }
        }
        if self.has_modal() {
            return;
        }
        if !response.hovered() && self.drag.is_none() {
            return;
        }
        let input = ui.input(|i| i.clone());
        let Some(p) = point else { return };
        let hit = hit_object(s, p, 7. / scale as f64);
        let mut port = hit_endpoint(s, p, 10. / scale as f64);
        if let Some(endpoint) = &mut port {
            routing::snap_endpoint(&s.document, s.view.diagram, endpoint, s.view.grid);
        }
        let mods = input.modifiers;
        if response.clicked_by(PointerButton::Secondary) {
            if let Some((block, reference)) = self.pending_call.take() {
                if let Some(Endpoint {
                    anchor:
                        Anchor::Boundary {
                            port: Port::Mechanism,
                            t,
                        },
                    ..
                }) = port
                {
                    self.execute(Command::CallMechanism {
                        block,
                        reference,
                        t,
                    });
                }
            } else {
                let (lo, hi) = s.document.content();
                if inside(p, lo, hi) {
                    self.execute(Command::BlockCreate {
                        diagram: None,
                        pos: s.view.grid.snap(p - Point::new(26.5, 13.25)),
                        title: String::new(),
                    });
                }
            }
        }
        if response.clicked_by(PointerButton::Middle) {
            if let Some(id) = hit.filter(|id| diag.blocks.contains_key(id)) {
                self.execute(Command::Decompose { block: id });
            } else {
                self.execute(Command::NavigateUp {});
            }
        }
        if response.double_clicked_by(PointerButton::Primary) {
            if let Some(id) = hit {
                if let Some(b) = diag.blocks.get(&id) {
                    self.dialogs.block = Some(b.clone());
                } else if let Some(a) = diag.arrows.get(&id)
                    && a.call_ref.is_none()
                {
                    self.dialogs.arrow = Some(a.clone());
                }
            } else {
                let (lo, hi) = s.document.content();
                if !inside(p, lo, hi) {
                    self.dialogs.metadata = Some(s.document.meta.clone());
                    self.dialogs.state = s.document.diagrams[&s.view.diagram].state;
                    self.dialogs.new_metadata = false;
                    self.dialogs.tab = 3;
                }
            }
        }
        if input.pointer.button_pressed(PointerButton::Primary)
            && response.hovered()
            && !response.double_clicked()
        {
            self.drag_revision = s.revision;
            if let Some((block, reference)) = self.pending_call
                && let Some(Endpoint {
                    anchor:
                        Anchor::Boundary {
                            port: Port::Mechanism,
                            t,
                        },
                    ..
                }) = &port
            {
                self.execute(Command::CallMechanism {
                    block,
                    reference,
                    t: *t,
                });
                self.pending_call = None;
                return;
            }
            let selected_arrow = hit
                .and_then(|id| diag.arrows.get(&id))
                .or_else(|| s.view.selection.iter().find_map(|id| diag.arrows.get(id)));
            if let Some(a) = selected_arrow {
                let path = render::route(&s.document, s.view.diagram, a);
                let label = render::arrow_label_position(&path, a);
                let at_stub = a.interface.as_ref().is_some_and(|i| i.stub)
                    && path
                        .first()
                        .is_some_and(|q| q.distance(p) < 10. / scale as f64);
                if !at_stub
                    && !a.label_hidden
                    && !a.label.is_empty()
                    && (p.y - label.y).abs() < 3.
                    && (p.x - label.x).abs() < a.label.chars().count() as f64 * 0.9 + 3.
                {
                    self.drag = Some(Drag::Label {
                        id: a.id,
                        start: p,
                        offset: a.label_offset,
                    });
                    return;
                }
                for (from, ep) in [(true, &a.from), (false, &a.to)] {
                    if !at_stub
                        && s.document
                            .endpoint(s.view.diagram, ep)
                            .is_some_and(|v| v.distance(p) < 7. / scale as f64)
                    {
                        self.drag = Some(Drag::End { id: a.id, from });
                        return;
                    }
                }
            }
            if mods.ctrl
                && let Some(id) = hit.filter(|id| diag.arrows.contains_key(id))
            {
                let path = render::route(&s.document, s.view.diagram, &diag.arrows[&id]);
                let (q, _, _) = render::nearest_on_path(p, &path).unwrap();
                self.drag = Some(Drag::Branch { id, point: q });
                return;
            }
            for a in diag.arrows.values() {
                if let Some(i) = &a.interface
                    && i.stub
                {
                    let path = render::route(&s.document, s.view.diagram, a);
                    if path
                        .last()
                        .is_some_and(|q| q.distance(p) < 10. / scale as f64)
                    {
                        self.drag = Some(if mods.shift {
                            Drag::Stub { id: a.id }
                        } else {
                            Drag::Connect {
                                from: if i.port == Port::Output {
                                    a.to.clone()
                                } else {
                                    a.from.clone()
                                },
                                interface: Some(a.id),
                            }
                        });
                        return;
                    }
                }
            }
            if let Some(ep) = port.clone()
                && matches!(
                    ep.anchor,
                    Anchor::Block {
                        port: Port::Output,
                        ..
                    } | Anchor::Boundary {
                        port: Port::Input | Port::Control | Port::Mechanism,
                        ..
                    } | Anchor::Junction { .. }
                )
            {
                if mods.shift && matches!(ep.anchor, Anchor::Junction { .. }) {
                } else {
                    self.drag = Some(Drag::Connect {
                        from: ep,
                        interface: None,
                    });
                    return;
                }
            }
            if let Some(id) = hit {
                let mut ids = s.view.selection.clone();
                if mods.command || mods.shift {
                    if !ids.insert(id) {
                        ids.remove(&id);
                    }
                } else if !ids.contains(&id) {
                    ids = BTreeSet::from([id]);
                }
                self.select(ids.iter().copied().collect());
                let positions = ids
                    .iter()
                    .filter_map(|id| {
                        diag.blocks
                            .get(id)
                            .map(|b| (*id, b.pos))
                            .or_else(|| diag.junctions.get(id).map(|j| (*id, j.pos)))
                    })
                    .collect::<BTreeMap<_, _>>();
                if !positions.is_empty() {
                    self.drag = Some(Drag::Move {
                        start: p,
                        positions,
                    });
                } else if let Some(a) = diag.arrows.get(&id) {
                    let path = render::route(&s.document, s.view.diagram, a);
                    if let Some((_, index, _)) = render::nearest_on_path(p, &path) {
                        self.drag = Some(Drag::Segment {
                            id,
                            index,
                            path,
                            start: p,
                        });
                    }
                }
            } else {
                if !mods.shift {
                    self.select(vec![]);
                }
                self.drag = Some(Drag::Box { start: p });
            }
        }
        if response.drag_started_by(PointerButton::Middle) && mods.shift {
            self.drag = Some(Drag::Pan {
                start: p,
                center: s.view.center,
            });
        }
        if input.pointer.any_released()
            && let Some(drag) = self.drag.take()
        {
            let command = match drag {
                Drag::Move { start, positions } => {
                    let delta = drag_delta(&positions, p - start, s.view.grid);
                    if delta.distance(Point::default()) < 0.1 {
                        None
                    } else {
                        Some(Command::ObjectsMove {
                            positions: positions
                                .into_iter()
                                .map(|(id, v)| (id, v + delta))
                                .collect(),
                        })
                    }
                }
                Drag::Connect { from, interface } => {
                    let target = port.clone();
                    let output_stub = diag.arrows.values().find(|a| {
                        a.interface
                            .as_ref()
                            .is_some_and(|i| i.stub && i.port == Port::Output)
                            && render::route(&s.document, s.view.diagram, a)
                                .last()
                                .is_some_and(|q| q.distance(p) < 10. / scale as f64)
                    });
                    if let Some(a) = output_stub.filter(|_| interface.is_none()) {
                        Some(Command::InterfaceConnect {
                            id: a.id,
                            endpoint: from,
                        })
                    } else if let Some(id) = interface {
                        target.map(|endpoint| Command::InterfaceConnect { id, endpoint })
                    } else if let Some(to) = target {
                        let target_iface = diag.arrows.values().find(|a| {
                            a.interface
                                .as_ref()
                                .is_some_and(|i| i.stub && i.port == Port::Output)
                                && render::route(&s.document, s.view.diagram, a)
                                    .last()
                                    .is_some_and(|q| q.distance(p) < 10. / scale as f64)
                        });
                        if let Some(a) = target_iface {
                            Some(Command::InterfaceConnect {
                                id: a.id,
                                endpoint: from,
                            })
                        } else {
                            Some(Command::ArrowCreate {
                                diagram: None,
                                from,
                                to,
                                label: String::new(),
                            })
                        }
                    } else if let Some(id) = hit.filter(|id| diag.arrows.contains_key(id)) {
                        let path = render::route(&s.document, s.view.diagram, &diag.arrows[&id]);
                        let (q, _, _) = render::nearest_on_path(p, &path).unwrap();
                        let junction = s.next_id();
                        Some(Command::Transaction {
                            commands: vec![
                                Command::ArrowBranch {
                                    id,
                                    point: q,
                                    target: None,
                                },
                                Command::ArrowCreate {
                                    diagram: None,
                                    from,
                                    to: Endpoint::new(Anchor::Junction { id: junction }),
                                    label: String::new(),
                                },
                            ],
                        })
                    } else {
                        None
                    }
                }
                Drag::Branch { id, point } => port.map(|target| Command::ArrowBranch {
                    id,
                    point,
                    target: Some(target),
                }),
                Drag::Label { id, start, offset } => Some(Command::ArrowUpdate {
                    id,
                    from: None,
                    to: None,
                    label: None,
                    label_offset: Some(offset + p - start),
                    label_hidden: None,
                    bends: None,
                }),
                Drag::End { id, from } => port.map(|ep| Command::ArrowUpdate {
                    id,
                    from: from.then_some(ep.clone()),
                    to: (!from).then_some(ep),
                    label: None,
                    label_offset: None,
                    label_hidden: None,
                    bends: None,
                }),
                Drag::Segment {
                    id,
                    index,
                    path,
                    start,
                } => {
                    let edited = routing::drag_segment(
                        &s.document,
                        s.view.diagram,
                        &diag.arrows[&id],
                        path,
                        index,
                        p - start,
                        s.view.grid,
                    );
                    Some(Command::ArrowUpdate {
                        id,
                        from: Some(edited.from),
                        to: Some(edited.to),
                        label: None,
                        label_offset: None,
                        label_hidden: None,
                        bends: Some(edited.bends),
                    })
                }
                Drag::Box { start } => {
                    let lo = Point::new(start.x.min(p.x), start.y.min(p.y));
                    let hi = Point::new(start.x.max(p.x), start.y.max(p.y));
                    let ids = diag
                        .blocks
                        .values()
                        .filter(|b| inside(b.pos + Point::new(26.5, 13.25), lo, hi))
                        .map(|b| b.id)
                        .chain(
                            diag.junctions
                                .values()
                                .filter(|j| inside(j.pos, lo, hi))
                                .map(|j| j.id),
                        )
                        .collect();
                    self.select(ids);
                    None
                }
                Drag::Stub { id } => {
                    let i = diag.arrows[&id].interface.as_ref().unwrap();
                    let (lo, hi) = s.document.content();
                    let t = match i.port {
                        Port::Input | Port::Output => (p.y - lo.y) / (hi.y - lo.y),
                        _ => (p.x - lo.x) / (hi.x - lo.x),
                    };
                    Some(Command::InterfaceMove {
                        id,
                        t: t.clamp(0., 1.),
                    })
                }
                Drag::Pan { start, center } => Some(Command::ViewSet {
                    zoom: None,
                    center: Some(center + start - p),
                    theme: None,
                }),
            };
            if let Some(cmd) = command {
                self.execute_at(cmd, self.drag_revision);
            }
        }
        if response.hovered() {
            let zoom = input.zoom_delta();
            let pinch_event = !mods.command && (zoom - 1.).abs() > 1e-5;
            if pinch_event {
                if input.time - self.pinch_last > 0.3 {
                    self.pinch_scale = 1.;
                    self.pinch_handled = false;
                }
                self.pinch_last = input.time;
                self.pinch_scale *= zoom;
            }
            if pinch_event && !self.pinch_handled && self.pinch_scale > 1.15 {
                if let Some(id) = hit.filter(|id| diag.blocks.contains_key(id)) {
                    self.execute(Command::Decompose { block: id });
                    self.pinch_handled = true;
                }
            } else if pinch_event && !self.pinch_handled && self.pinch_scale < 0.85 {
                self.execute(Command::NavigateUp {});
                self.pinch_handled = true;
            }
            if input.smooth_scroll_delta != Vec2::ZERO {
                if mods.command {
                    let z = (s.view.zoom * (input.smooth_scroll_delta.y as f64 * 0.003).exp())
                        .clamp(0.05, 40.);
                    self.execute(Command::ViewSet {
                        zoom: Some(z),
                        center: None,
                        theme: None,
                    });
                } else {
                    self.execute(Command::ViewSet {
                        zoom: None,
                        center: Some(
                            s.view.center
                                - Point::new(
                                    input.smooth_scroll_delta.x as f64 / scale as f64,
                                    input.smooth_scroll_delta.y as f64 / scale as f64,
                                ),
                        ),
                        theme: None,
                    });
                }
            }
        }
    }
}

fn drag_delta(positions: &BTreeMap<Id, Point>, delta: Point, grid: routing::Grid) -> Point {
    positions
        .values()
        .next()
        .map_or(delta, |origin| grid.snap(*origin + delta) - *origin)
}
