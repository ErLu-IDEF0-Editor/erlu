use super::*;
use erlu_core::{
    dfd::{Definition, Document, Edit, End, Flow, Kind, Style},
    dfd_render,
};

#[derive(Default)]
pub(super) struct Editor {
    pub(super) validation: bool,
    pub(super) dictionary: bool,
    definition: Option<(Option<Id>, Definition)>,
    definition_revision: u64,
    connect: bool,
    source: Option<End>,
    placement: Option<Kind>,
    draft: Option<Draft>,
    draft_revision: u64,
    drag: Option<(Id, Point, u64)>,
    level: Option<Id>,
}

enum Draft {
    Node {
        id: Id,
        title: String,
        data: Option<Id>,
        kind: Kind,
    },
    Flow {
        id: Option<Id>,
        flow: Flow,
    },
    Title(String),
}

impl ErluApp {
    fn edit_dfd(&mut self, edit: Edit, revision: u64) -> Option<Response> {
        self.execute_at(Command::DfdEdit { edit }, revision)
    }

    pub(super) fn dfd_keyboard(&mut self, ctx: &egui::Context, s: &Engine) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let input = ctx.input(|i| i.clone());
        if input.modifiers.command && input.key_pressed(Key::Z) {
            self.execute(if input.modifiers.shift {
                Command::Redo {}
            } else {
                Command::Undo {}
            });
        } else if input.modifiers.command && input.key_pressed(Key::S) {
            self.save(input.modifiers.shift);
        } else if input.key_pressed(Key::Delete) || input.key_pressed(Key::Backspace) {
            self.execute_at(
                Command::ObjectsDelete {
                    ids: s.view.selection.iter().copied().collect(),
                },
                s.revision,
            );
        } else if input.key_pressed(Key::F) {
            self.execute(Command::ViewFit {});
        } else if input.key_pressed(Key::G) {
            self.execute(Command::GridSet {
                enabled: !s.view.grid.enabled,
                spacing: None,
            });
        } else if input.key_pressed(Key::Escape) {
            self.dfd_ui.connect = false;
            self.dfd_ui.source = None;
            self.dfd_ui.placement = None;
            self.dfd_ui.draft = None;
        }
    }

    pub(super) fn dfd_canvas(&mut self, ui: &mut egui::Ui, s: &Engine, doc: &Document) {
        let diagram = s.view.diagram;
        let data = &doc.diagrams[&diagram];
        if self.dfd_ui.level != Some(diagram) {
            self.dfd_ui.level = Some(diagram);
            self.dfd_ui.draft = None;
            self.dfd_ui.drag = None;
            self.dfd_ui.source = None;
            self.dfd_ui.placement = None;
            self.dfd_ui.connect = false;
        }
        let enabled = !self.has_modal();
        ui.add_enabled_ui(enabled, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (kind, label, id, available) in [
                    (Kind::Process, "Process", "process", diagram != doc.root),
                    (
                        Kind::External,
                        "External entity",
                        "external",
                        diagram == doc.root,
                    ),
                    (Kind::Store, "Data store", "store", diagram != doc.root),
                ] {
                    let button = ui.add_enabled(
                        available,
                        egui::Button::selectable(
                            self.dfd_ui.placement == Some(kind),
                            self.ui_text(label),
                        ),
                    );
                    ui_target(&format!("dfd.add.{id}"), &button, None);
                    if button.clicked() {
                        self.dfd_ui.placement = Some(kind);
                        self.dfd_ui.connect = false;
                        self.dfd_ui.source = None;
                    }
                }
                let flow = ui.selectable_label(self.dfd_ui.connect, self.ui_text("Data flow"));
                ui_target("dfd.flow", &flow, None);
                if flow.clicked() {
                    self.dfd_ui.connect = !self.dfd_ui.connect;
                    self.dfd_ui.source = None;
                    self.dfd_ui.placement = None;
                }
                ui.separator();
                let mut style = doc.style;
                let style_response = egui::ComboBox::from_id_salt("dfd-style")
                    .selected_text(style_name(style))
                    .show_ui(ui, |ui| {
                        for option in [Style::GaneSarson, Style::YourdonDemarco] {
                            let choice =
                                ui.selectable_value(&mut style, option, style_name(option));
                            ui_target(
                                match option {
                                    Style::GaneSarson => "dfd.style.gane",
                                    Style::YourdonDemarco => "dfd.style.yourdon",
                                },
                                &choice,
                                None,
                            );
                        }
                    });
                ui_target("dfd.style", &style_response.response, None);
                if style != doc.style {
                    self.edit_dfd(Edit::Style { style }, s.revision);
                }
                let validation = ui.button(self.ui_text("Validation"));
                ui_target("dfd.validation", &validation, None);
                if validation.clicked() {
                    self.dfd_ui.validation ^= true;
                }
                let dictionary = ui.button(self.ui_text("Data dictionary"));
                ui_target("dfd.dictionary", &dictionary, None);
                if dictionary.clicked() {
                    self.dfd_ui.dictionary ^= true;
                }
                if ui.button(self.ui_text("Properties…")).clicked() {
                    self.dfd_ui.draft_revision = s.revision;
                    self.dfd_ui.draft = Some(Draft::Title(doc.title.clone()));
                }
                if let Some(id) = s.view.selection.iter().next().copied()
                    && data.nodes.get(&id).is_some_and(|n| n.kind == Kind::Process)
                {
                    let decompose = ui.button(self.ui_text("Decompose"));
                    ui_target("dfd.decompose", &decompose, None);
                    if decompose.clicked() {
                        self.dfd_open_child(id, s.revision);
                    }
                }
            });
            if self.dfd_ui.connect {
                ui.label(self.ui_text(if self.dfd_ui.source.is_none() {
                    "Select the flow source."
                } else {
                    "Select the flow destination."
                }));
            } else if self.dfd_ui.placement.is_some() {
                ui.label(self.ui_text("Click the canvas to place the element."));
            }
        });
        let (rect, response) =
            ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
        let base = ((rect.width() - 24.) / 270.)
            .min((rect.height() - 24.) / 180.)
            .max(0.1);
        let scale = (s.view.zoom as f32 * base).max(0.1);
        ui_target("dfd.canvas", &response, Some(&scale.to_string()));
        let screen = |p: Point| {
            rect.center()
                + Vec2::new(
                    (p.x - s.view.center.x) as f32 * scale,
                    (p.y - s.view.center.y) as f32 * scale,
                )
        };
        let world = |p: Pos2| {
            let offset = (p - rect.center()) / scale;
            s.view.center + Point::new(f64::from(offset.x), f64::from(offset.y))
        };
        let painter = ui.painter_at(rect);
        if s.view.grid.enabled && s.view.grid.spacing as f32 * scale >= 5. {
            let min = world(rect.min);
            let max = world(rect.max);
            let spacing = s.view.grid.spacing;
            let first_x = (min.x / spacing).ceil() * spacing;
            let first_y = (min.y / spacing).ceil() * spacing;
            let grid = Stroke::new(
                0.5,
                ui.visuals()
                    .widgets
                    .inactive
                    .bg_stroke
                    .color
                    .gamma_multiply(0.55),
            );
            for column in 0..=((max.x - first_x) / spacing).floor().max(0.) as usize {
                let x = first_x + column as f64 * spacing;
                painter.line_segment(
                    [screen(Point::new(x, min.y)), screen(Point::new(x, max.y))],
                    grid,
                );
            }
            for row in 0..=((max.y - first_y) / spacing).floor().max(0.) as usize {
                let y = first_y + row as f64 * spacing;
                painter.line_segment(
                    [screen(Point::new(min.x, y)), screen(Point::new(max.x, y))],
                    grid,
                );
            }
        }
        let dark = ui.visuals().dark_mode;
        let (scene, _) = dfd_render::scene(doc, diagram, dark, None);
        for primitive in scene.primitives.into_iter().skip(1) {
            match primitive {
                Primitive::Rect {
                    min,
                    max,
                    stroke,
                    fill,
                    width,
                    radius,
                    ..
                } => {
                    painter.rect(
                        Rect::from_two_pos(screen(min), screen(max)),
                        (radius as f32 * scale).min(255.),
                        parse_color(&fill),
                        Stroke::new(width as f32 * scale, parse_color(&stroke)),
                        egui::StrokeKind::Middle,
                    );
                }
                Primitive::Line {
                    points,
                    color,
                    width,
                    ..
                } => {
                    if points.len() > 1 {
                        painter.add(egui::Shape::line(
                            points.into_iter().map(screen).collect(),
                            Stroke::new(width as f32 * scale, parse_color(&color)),
                        ));
                    }
                }
                Primitive::Text {
                    pos,
                    text,
                    size,
                    color,
                    ..
                } => {
                    painter.text(
                        screen(pos),
                        egui::Align2::CENTER_CENTER,
                        localization::display_text(&text),
                        egui::FontId::proportional(size as f32 * scale),
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
                        screen(pos),
                        radius as f32 * scale,
                        parse_color(&fill),
                        Stroke::new(0.4 * scale, parse_color(&color)),
                    );
                }
            }
        }
        let boundaries = dfd_render::boundaries(doc, diagram);
        let accent = ui.visuals().selection.stroke;
        for id in &s.view.selection {
            if let Some(n) = data.nodes.get(id) {
                let half = dfd_render::half_size(n.kind, doc.style) + Point::new(1.4, 1.4);
                painter.rect_stroke(
                    Rect::from_two_pos(screen(n.position - half), screen(n.position + half)),
                    2.,
                    accent,
                    egui::StrokeKind::Outside,
                );
            } else if let Some(f) = data.flows.get(id) {
                let points = dfd_render::route(doc, diagram, f, &boundaries);
                if points.len() > 1 {
                    painter.add(egui::Shape::line(
                        points.into_iter().map(screen).collect(),
                        accent,
                    ));
                }
            }
        }
        if enabled {
            if response.hovered() {
                let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll != 0. {
                    self.execute(Command::ViewSet {
                        zoom: Some(
                            (s.view.zoom * f64::from((scroll * 0.002).exp())).clamp(0.05, 40.),
                        ),
                        center: None,
                        theme: None,
                    });
                }
            }
            if response.dragged_by(PointerButton::Middle) {
                let delta = ui.input(|i| i.pointer.delta()) / scale;
                self.execute(Command::ViewSet {
                    zoom: None,
                    center: Some(s.view.center - Point::new(delta.x.into(), delta.y.into())),
                    theme: None,
                });
            }
            if let Some(pointer) = response.interact_pointer_pos() {
                let p = world(pointer);
                let node = data
                    .nodes
                    .iter()
                    .rev()
                    .find(|(_, n)| dfd_render::contains(n, doc.style, p))
                    .map(|(&id, _)| id);
                let flow = || {
                    data.flows.iter().rev().find_map(|(&id, f)| {
                        let path = dfd_render::route(doc, diagram, f, &boundaries);
                        render::nearest_on_path(p, &path)
                            .filter(|(q, _, _)| q.distance(p) * f64::from(scale) <= 7.)
                            .map(|_| id)
                    })
                };
                let end = || {
                    node.map(|id| End::Node { id }).or_else(|| {
                        boundaries
                            .iter()
                            .find(|(_, pos)| pos.distance(p) * f64::from(scale) <= 12.)
                            .map(|(&parent_flow, _)| End::Boundary { parent_flow })
                    })
                };
                if response.double_clicked_by(PointerButton::Primary) && !self.dfd_ui.connect {
                    self.open_dfd_object(data, node.or_else(flow), s.revision);
                } else if response.clicked_by(PointerButton::Primary) {
                    if let Some(kind) = self.dfd_ui.placement.take() {
                        self.place_dfd_node(kind, p, s);
                    } else if self.dfd_ui.connect {
                        if let Some(endpoint) = end() {
                            if let Some(from) = self.dfd_ui.source.take() {
                                self.dfd_ui.draft_revision = s.revision;
                                self.dfd_ui.draft = Some(Draft::Flow {
                                    id: None,
                                    flow: Flow {
                                        from,
                                        to: endpoint,
                                        label: String::new(),
                                        data: None,
                                        bends: vec![],
                                    },
                                });
                                self.dfd_ui.connect = false;
                            } else {
                                self.dfd_ui.source = Some(endpoint);
                            }
                        }
                    } else {
                        self.select(node.or_else(flow).into_iter().collect());
                    }
                } else if response.clicked_by(PointerButton::Middle) {
                    if let Some(id) = node.filter(|id| data.nodes[id].child.is_some()) {
                        self.dfd_open_child(id, s.revision);
                    } else if node.is_none() {
                        self.execute(Command::NavigateUp {});
                    }
                }
                if !self.dfd_ui.connect && self.dfd_ui.placement.is_none() {
                    if response.drag_started_by(PointerButton::Primary)
                        && let Some(start) = ui.input(|i| i.pointer.press_origin()).map(world)
                        && let Some((&id, _)) = data
                            .nodes
                            .iter()
                            .rev()
                            .find(|(_, n)| dfd_render::contains(n, doc.style, start))
                    {
                        self.dfd_ui.drag = Some((id, start, s.revision));
                    }
                    if let Some((id, start, revision)) = self.dfd_ui.drag
                        && let Some(n) = data.nodes.get(&id)
                    {
                        let pos = s.view.grid.snap(n.position + (p - start));
                        if response.drag_stopped_by(PointerButton::Primary) {
                            self.dfd_ui.drag = None;
                            self.execute_at(
                                Command::ObjectsMove {
                                    positions: BTreeMap::from([(id, pos)]),
                                },
                                revision,
                            );
                        } else {
                            painter.circle_stroke(screen(pos), 5., accent);
                        }
                    }
                }
                response.context_menu(|ui| {
                    if let Some(id) = node.or_else(flow) {
                        if ui.button(self.ui_text("Properties…")).clicked() {
                            self.open_dfd_object(data, Some(id), s.revision);
                            ui.close();
                        }
                        if data.nodes.get(&id).is_some_and(|n| n.kind == Kind::Process)
                            && ui.button(self.ui_text("Decompose")).clicked()
                        {
                            self.dfd_open_child(id, s.revision);
                            ui.close();
                        }
                    } else {
                        let choices = if diagram == doc.root {
                            vec![(Kind::External, "External entity")]
                        } else {
                            vec![(Kind::Process, "Process"), (Kind::Store, "Data store")]
                        };
                        for (kind, label) in choices {
                            if ui.button(self.ui_text(label)).clicked() {
                                self.place_dfd_node(kind, p, s);
                                ui.close();
                            }
                        }
                    }
                });
            }
        }
        self.dfd_windows(ui.ctx(), s, doc);
    }

    fn place_dfd_node(&mut self, kind: Kind, position: Point, s: &Engine) {
        let position = s.view.grid.snap(position);
        let title = match kind {
            Kind::Process => "Process",
            Kind::External => "External entity",
            Kind::Store => "Data store",
        };
        if let Some(result) = self.edit_dfd(
            Edit::NodeCreate {
                diagram: None,
                kind,
                title: self.tr(title),
                position,
            },
            s.revision,
        ) && let Some(id) = result.result["id"].as_u64()
        {
            self.select(vec![id]);
            self.dfd_ui.draft_revision = result.revision;
            self.dfd_ui.draft = Some(Draft::Node {
                id,
                title: self.tr(title),
                data: None,
                kind,
            });
        }
    }

    fn dfd_open_child(&mut self, id: Id, revision: u64) {
        if let Some(result) = self.edit_dfd(Edit::Decompose { id }, revision)
            && let Some(diagram) = result.result["diagram"].as_u64()
        {
            self.execute(Command::Navigate { diagram });
        }
    }

    fn open_dfd_object(&mut self, diagram: &dfd::Diagram, id: Option<Id>, revision: u64) {
        let Some(id) = id else {
            return;
        };
        self.select(vec![id]);
        self.dfd_ui.draft_revision = revision;
        if let Some(n) = diagram.nodes.get(&id) {
            self.dfd_ui.draft = Some(Draft::Node {
                id,
                title: n.title.clone(),
                data: n.data,
                kind: n.kind,
            });
        } else if let Some(flow) = diagram.flows.get(&id) {
            self.dfd_ui.draft = Some(Draft::Flow {
                id: Some(id),
                flow: flow.clone(),
            });
        }
    }

    fn dfd_windows(&mut self, ctx: &egui::Context, s: &Engine, doc: &Document) {
        if self.dfd_ui.validation {
            let mut open = true;
            egui::Window::new(self.ui_text("Validation"))
                .id(egui::Id::unique("dfd-validation"))
                .open(&mut open)
                .default_width(470.)
                .show(ctx, |ui| {
                    let diagnostics = doc.validate();
                    if diagnostics.is_empty() {
                        ui.label(self.ui_text("No issues found."));
                    }
                    egui::ScrollArea::vertical()
                        .max_height(380.)
                        .show(ui, |ui| {
                            for d in diagnostics {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(if d.severity == "error" {
                                        self.ui_text("Error")
                                    } else {
                                        self.ui_text("Warning")
                                    });
                                    if ui.link(self.ui_text(&d.message)).clicked() {
                                        self.execute(Command::Navigate { diagram: d.diagram });
                                        self.select(d.object.into_iter().collect());
                                    }
                                    ui.weak(match d.reference {
                                        Some(reference) => format!("{} · {reference}", d.source),
                                        None => d.source,
                                    });
                                });
                            }
                        });
                });
            self.dfd_ui.validation = open;
        }
        if self.dfd_ui.dictionary {
            self.dictionary_window(ctx, s, doc);
        }
        if let Some(mut draft) = self.dfd_ui.draft.take() {
            let mut keep = true;
            egui::Window::new(self.ui_text("Properties…"))
                .id(egui::Id::unique("dfd-properties"))
                .default_width(280.)
                .collapsible(false)
                .show(ctx, |ui| {
                    match &mut draft {
                        Draft::Node {
                            title, data, kind, ..
                        } => {
                            ui.label(self.ui_text("Title:"));
                            let name = ui.text_edit_singleline(title);
                            ui_target("dfd.properties.title", &name, None);
                            if *kind == Kind::Store {
                                self.definition_picker(ui, doc, data);
                            }
                        }
                        Draft::Title(title) => {
                            ui.text_edit_singleline(title);
                        }
                        Draft::Flow { flow, .. } => {
                            ui.label(self.ui_text("Data flow"));
                            let name = ui.text_edit_singleline(&mut flow.label);
                            ui_target("dfd.properties.title", &name, None);
                            self.definition_picker(ui, doc, &mut flow.data);
                            ui.collapsing(self.ui_text("Waypoints"), |ui| {
                                let mut remove = None;
                                for (i, point) in flow.bends.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.add(egui::DragValue::new(&mut point.x).prefix("x "));
                                        ui.add(egui::DragValue::new(&mut point.y).prefix("y "));
                                        if ui.button("×").clicked() {
                                            remove = Some(i);
                                        }
                                    });
                                }
                                if let Some(i) = remove {
                                    flow.bends.remove(i);
                                }
                                if ui.button(self.ui_text("Add waypoint")).clicked() {
                                    flow.bends.push(s.view.center);
                                }
                            });
                        }
                    }
                    ui.horizontal(|ui| {
                        let save = ui.button(self.ui_text("Save"));
                        ui_target("dfd.properties.save", &save, None);
                        if save.clicked() {
                            let command = match &draft {
                                Draft::Title(title) => Command::DocumentMetadata {
                                    patch: json!({"title":title}),
                                },
                                Draft::Node {
                                    id, title, data, ..
                                } => Command::DfdEdit {
                                    edit: Edit::NodeUpdate {
                                        id: *id,
                                        title: title.clone(),
                                        data: *data,
                                    },
                                },
                                Draft::Flow { id, flow } => Command::DfdEdit {
                                    edit: match id {
                                        Some(id) => Edit::FlowUpdate {
                                            id: *id,
                                            flow: flow.clone(),
                                        },
                                        None => Edit::FlowCreate {
                                            diagram: Some(s.view.diagram),
                                            flow: flow.clone(),
                                        },
                                    },
                                },
                            };
                            if let Some(result) =
                                self.execute_at(command, self.dfd_ui.draft_revision)
                            {
                                if let Some(id) = result.result["id"].as_u64() {
                                    self.select(vec![id]);
                                }
                                keep = false;
                            }
                        }
                        if ui.button(self.ui_text("Cancel")).clicked() {
                            keep = false;
                        }
                    });
                });
            if keep {
                self.dfd_ui.draft = Some(draft);
            }
        }
    }

    fn definition_picker(&self, ui: &mut egui::Ui, doc: &Document, value: &mut Option<Id>) {
        let none = self.ui_text("No dictionary definition");
        egui::ComboBox::from_id_salt("dfd-definition-picker")
            .selected_text(
                value
                    .and_then(|id| doc.dictionary.get(&id))
                    .map_or(none.as_str(), |d| d.name.as_str()),
            )
            .show_ui(ui, |ui| {
                ui.selectable_value(value, None, none);
                for (&id, definition) in &doc.dictionary {
                    ui.selectable_value(value, Some(id), &definition.name);
                }
            });
    }

    fn dictionary_window(&mut self, ctx: &egui::Context, s: &Engine, doc: &Document) {
        let mut open = true;
        egui::Window::new(self.ui_text("Data dictionary"))
            .id(egui::Id::unique("dfd-dictionary"))
            .open(&mut open)
            .default_width(410.)
            .show(ctx, |ui| {
                if ui.button(self.ui_text("Add definition")).clicked() {
                    self.dfd_ui.definition_revision = s.revision;
                    self.dfd_ui.definition = Some((None, Definition::default()));
                }
                egui::ScrollArea::vertical()
                    .max_height(220.)
                    .show(ui, |ui| {
                        for (&id, d) in &doc.dictionary {
                            if ui.button(&d.name).clicked() {
                                self.dfd_ui.definition_revision = s.revision;
                                self.dfd_ui.definition = Some((Some(id), d.clone()));
                            }
                        }
                    });
                let Some((id, mut definition)) = self.dfd_ui.definition.take() else {
                    return;
                };
                ui.separator();
                ui.label(self.ui_text("Name"));
                ui.text_edit_singleline(&mut definition.name);
                ui.label(self.ui_text("Description"));
                ui.text_edit_multiline(&mut definition.description);
                ui.collapsing(self.ui_text("Components"), |ui| {
                    for (&other, d) in &doc.dictionary {
                        if Some(other) == id {
                            continue;
                        }
                        let mut included = definition.components.contains(&other);
                        if ui.checkbox(&mut included, &d.name).changed() {
                            if included {
                                definition.components.push(other);
                            } else {
                                definition.components.retain(|&c| c != other);
                            }
                        }
                    }
                });
                if let Some(id) = id {
                    ui.collapsing(self.ui_text("Used by"), |ui| {
                        for d in doc
                            .dictionary
                            .values()
                            .filter(|d| d.components.contains(&id))
                        {
                            ui.label(&d.name);
                        }
                        for (&diagram, d) in &doc.diagrams {
                            let flows = d
                                .flows
                                .iter()
                                .filter(|(_, f)| f.data == Some(id))
                                .map(|(&object, f)| (object, doc.flow_label(diagram, f)));
                            let stores = d
                                .nodes
                                .iter()
                                .filter(|(_, n)| n.data == Some(id))
                                .map(|(&object, n)| (object, n.title.as_str()));
                            for (object, label) in flows.chain(stores) {
                                if ui
                                    .link(format!("{} · {}", doc.diagram_title(diagram), label))
                                    .clicked()
                                {
                                    self.execute(Command::Navigate { diagram });
                                    self.select(vec![object]);
                                }
                            }
                        }
                    });
                }
                let mut keep = true;
                ui.horizontal(|ui| {
                    if ui.button(self.ui_text("Save")).clicked()
                        && self
                            .edit_dfd(
                                Edit::DefinitionPut {
                                    id,
                                    definition: definition.clone(),
                                },
                                self.dfd_ui.definition_revision,
                            )
                            .is_some()
                    {
                        keep = false;
                    }
                    if let Some(id) = id
                        && ui.button(self.ui_text("Delete")).clicked()
                        && self
                            .edit_dfd(
                                Edit::DefinitionDelete { id },
                                self.dfd_ui.definition_revision,
                            )
                            .is_some()
                    {
                        keep = false;
                    }
                    if ui.button(self.ui_text("Cancel")).clicked() {
                        keep = false;
                    }
                });
                if keep {
                    self.dfd_ui.definition = Some((id, definition));
                }
            });
        self.dfd_ui.dictionary = open;
    }
}

fn style_name(style: Style) -> &'static str {
    match style {
        Style::GaneSarson => "Gane–Sarson",
        Style::YourdonDemarco => "Yourdon–DeMarco",
    }
}
