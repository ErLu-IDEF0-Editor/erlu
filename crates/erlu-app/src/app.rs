mod canvas;
mod dialogs;
mod menus;

use eframe::egui::{self, Color32, Key, PointerButton, Pos2, Rect, Stroke, Vec2};
use erlu_core::{
    render::{self, Primitive},
    *,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex, mpsc},
};

pub type SharedEngine = Arc<Mutex<Engine>>;
#[derive(Clone)]
enum Drag {
    Move {
        start: Point,
        positions: BTreeMap<Id, Point>,
    },
    Connect {
        from: Endpoint,
        interface: Option<Id>,
    },
    Branch {
        id: Id,
        point: Point,
    },
    Label {
        id: Id,
        start: Point,
        offset: Point,
    },
    End {
        id: Id,
        from: bool,
    },
    Segment {
        id: Id,
        index: usize,
        path: Vec<Point>,
        start: Point,
    },
    Box {
        start: Point,
    },
    Stub {
        id: Id,
    },
    Pan {
        start: Point,
        center: Point,
    },
}
enum Pending {
    New,
    Open,
    Loaded(String, String),
    Close,
}
enum FileEvent {
    Opened(String, String),
    Saved(u64, String),
    Error(String),
}
struct PluginForm {
    spec: ActionSpec,
    arguments: Value,
}
#[derive(Default)]
struct Dialogs {
    plugin: Option<PluginForm>,
    state: DiagramState,
    startup: bool,
    metadata: Option<Metadata>,
    new_metadata: bool,
    tab: usize,
    block: Option<Block>,
    arrow: Option<Arrow>,
    report: Option<String>,
    tree: bool,
    export: bool,
    all: bool,
    numbers: bool,
    light: bool,
    colors: bool,
    color: [u8; 3],
    call: bool,
    call_ref: Option<Id>,
    swap: bool,
    swap_pair: [Option<Id>; 2],
}
pub struct ErluApp {
    pub shared: SharedEngine,
    dialogs: Dialogs,
    pending: Option<Pending>,
    drag: Option<Drag>,
    drag_revision: u64,
    cursor: Option<Point>,
    box_rect: Option<(Point, Point)>,
    file_name: String,
    status: String,
    tx: mpsc::Sender<FileEvent>,
    rx: mpsc::Receiver<FileEvent>,
    language: String,
    translations: Value,
    allow_close: bool,
    pending_call: Option<(Id, Id)>,
    pub ai_enabled: bool,
    pub ai_origin: String,
    pub ai_token: String,
    #[cfg(not(target_arch = "wasm32"))]
    pub plugins: Arc<Mutex<erlu_plugins::PluginHost>>,
    #[cfg(target_arch = "wasm32")]
    pub web_events: crate::web::WebEvents,
    last_revision: u64,
    last_dirty: bool,
    has_document: bool,
    pinch_scale: f32,
    pinch_last: f64,
    pinch_handled: bool,
}
impl ErluApp {
    pub fn new(cc: &eframe::CreationContext<'_>, shared: SharedEngine, startup: bool) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "erlu".into(),
            egui::FontData::from_static(render::FONT).into(),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "erlu".into());
        cc.egui_ctx.set_fonts(fonts);
        let (tx, rx) = mpsc::channel();
        let language = cc
            .storage
            .and_then(|s| s.get_string("language"))
            .unwrap_or_else(default_language);
        let theme = cc
            .storage
            .and_then(|s| s.get_string("theme"))
            .and_then(|s| serde_json::from_str::<Theme>(&s).ok())
            .unwrap_or_default();
        {
            let mut state = shared.lock().unwrap();
            state.view.theme = theme;
            state.view.language = language.clone();
        }
        Self {
            shared,
            dialogs: Dialogs {
                startup,
                light: true,
                color: [43, 110, 230],
                ..Default::default()
            },
            pending: None,
            drag: None,
            drag_revision: 0,
            cursor: None,
            box_rect: None,
            file_name: String::new(),
            status: String::new(),
            tx,
            rx,
            language,
            translations: serde_json::from_str(include_str!("../../../assets/i18n.json")).unwrap(),
            allow_close: false,
            pending_call: None,
            ai_enabled: false,
            ai_origin: String::new(),
            ai_token: String::new(),
            #[cfg(not(target_arch = "wasm32"))]
            plugins: Arc::new(Mutex::new(erlu_plugins::PluginHost::default())),
            #[cfg(target_arch = "wasm32")]
            web_events: crate::web::WebEvents::default(),
            last_revision: 0,
            last_dirty: false,
            has_document: !startup,
            pinch_scale: 1.,
            pinch_last: 0.,
            pinch_handled: false,
        }
    }
    pub fn set_file_name(&mut self, name: String) {
        self.file_name = name;
    }
    pub fn tr(&self, s: &str) -> String {
        self.translations[&self.language][s]
            .as_str()
            .unwrap_or(s)
            .to_owned()
    }
    fn execute(&mut self, c: Command) -> Option<Response> {
        let result = self.shared.lock().unwrap().dispatch(c, None);
        match result {
            Ok(r) => {
                self.status.clear();
                Some(r)
            }
            Err(e) => {
                self.status = e.to_string();
                None
            }
        }
    }
    fn execute_at(&mut self, c: Command, revision: u64) -> Option<Response> {
        let result = self.shared.lock().unwrap().dispatch(c, Some(revision));
        match result {
            Ok(r) => Some(r),
            Err(e) => {
                self.status = e.to_string();
                None
            }
        }
    }
    fn select(&mut self, ids: Vec<Id>) {
        self.execute(Command::SelectionSet { ids });
    }
    fn request(&mut self, p: Pending) {
        #[cfg(not(target_arch = "wasm32"))]
        if matches!(p, Pending::New) {
            self.perform(p);
            return;
        }
        if self.shared.lock().unwrap().dirty() {
            self.pending = Some(p);
        } else {
            self.perform(p);
        }
    }
    fn perform(&mut self, p: Pending) {
        match p {
            Pending::New => {
                let mut meta = Metadata::default();
                if self.language == "ru" {
                    meta.currency = "₽".into();
                    meta.currency_placement = "1 ?".into();
                } else if self.language == "fr" {
                    meta.currency = "€".into();
                    meta.currency_placement = "1 ?".into();
                }
                self.dialogs.metadata = Some(meta);
                self.dialogs.new_metadata = true;
                self.dialogs.state = DiagramState::Working;
                self.dialogs.tab = 0;
            }
            Pending::Open => self.open_file(),
            Pending::Loaded(content, name) => {
                if self
                    .execute(Command::DocumentOpen {
                        json: content,
                        discard: true,
                    })
                    .is_some()
                {
                    self.file_name = name;
                    self.dialogs.startup = false;
                    self.has_document = true;
                }
            }
            Pending::Close => self.allow_close = true,
        }
    }
    fn open_file(&mut self) {
        let tx = self.tx.clone();
        let ctx_name = self.tr("Open…");
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            if let Some(path) = rfd::FileDialog::new()
                .set_title(ctx_name)
                .add_filter("IDEF0", &["idef0", "json"])
                .pick_file()
            {
                match std::fs::read_to_string(&path) {
                    Ok(text) => {
                        let _ = tx.send(FileEvent::Opened(text, path.to_string_lossy().into()));
                    }
                    Err(e) => {
                        let _ = tx.send(FileEvent::Error(e.to_string()));
                    }
                }
            }
        });
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            if let Some(file) = rfd::AsyncFileDialog::new()
                .set_title(ctx_name)
                .add_filter("IDEF0", &["idef0", "json"])
                .pick_file()
                .await
            {
                match String::from_utf8(file.read().await) {
                    Ok(text) => {
                        let _ = tx.send(FileEvent::Opened(text, file.file_name()));
                    }
                    Err(e) => {
                        let _ = tx.send(FileEvent::Error(e.to_string()));
                    }
                }
            }
        });
    }
    fn save(&mut self, save_as: bool) {
        let engine = self.shared.lock().unwrap();
        let revision = engine.revision;
        let text = engine.serialize_document();
        drop(engine);
        let tx = self.tx.clone();
        let name = if self.file_name.is_empty() {
            "diagram.idef0".into()
        } else {
            self.file_name.clone()
        };
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            let path = if save_as || name == "diagram.idef0" {
                rfd::FileDialog::new()
                    .set_file_name(&name)
                    .add_filter("IDEF0", &["idef0"])
                    .save_file()
            } else {
                Some(std::path::PathBuf::from(name))
            };
            if let Some(path) = path {
                match crate::atomic_write(&path, text.as_bytes()) {
                    Ok(()) => {
                        let _ = tx.send(FileEvent::Saved(revision, path.to_string_lossy().into()));
                    }
                    Err(e) => {
                        let _ = tx.send(FileEvent::Error(e.to_string()));
                    }
                }
            }
        });
        #[cfg(target_arch = "wasm32")]
        {
            let _ = save_as;
            match crate::web::download(&name, text.as_bytes(), "application/json") {
                Ok(()) => {
                    let _ = tx.send(FileEvent::Saved(revision, name));
                }
                Err(e) => {
                    let _ = tx.send(FileEvent::Error(format!("{e:?}")));
                }
            }
        }
    }
    fn export(&mut self, kind: &str) {
        let cmd = match kind {
            "pdf" => Command::ExportPdf {
                all: self.dialogs.all,
                number_pages: self.dialogs.numbers,
                light: self.dialogs.light,
            },
            "svg" => Command::ExportSvg {
                diagram: None,
                light: true,
            },
            _ => Command::ExportMarkdown {},
        };
        let Some(result) = self.execute(cmd) else {
            return;
        };
        let bytes = if kind == "pdf" {
            serde_json::from_value::<Vec<u8>>(result.result["bytes"].clone()).unwrap()
        } else {
            result.result["text"].as_str().unwrap().as_bytes().to_vec()
        };
        let name = format!("diagram.{kind}");
        let tx = self.tx.clone();
        #[cfg(not(target_arch = "wasm32"))]
        {
            let kind = kind.to_string();
            std::thread::spawn(move || {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name(name)
                    .add_filter(&kind, &[&kind])
                    .save_file()
                    && let Err(e) = crate::atomic_write(&path, &bytes)
                {
                    let _ = tx.send(FileEvent::Error(e.to_string()));
                }
            });
        }
        #[cfg(target_arch = "wasm32")]
        {
            let mime = if kind == "pdf" {
                "application/pdf"
            } else {
                "text/plain;charset=utf-8"
            };
            if let Err(e) = crate::web::download(&name, &bytes, mime) {
                let _ = tx.send(FileEvent::Error(format!("{e:?}")));
            }
        }
    }
}
impl eframe::App for ErluApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        for file in ctx.input(|i| i.raw.dropped_files.clone()) {
            let name = file.path().to_string_lossy().to_string();
            let tx = self.tx.clone();
            #[cfg(not(target_arch = "wasm32"))]
            std::thread::spawn(move || {
                let result = file
                    .bytes()
                    .and_then(|bytes| String::from_utf8(bytes).map_err(|e| e.to_string()));
                let event = match result {
                    Ok(content) => FileEvent::Opened(content, name),
                    Err(e) => FileEvent::Error(e),
                };
                let _ = tx.send(event);
            });
            #[cfg(target_arch = "wasm32")]
            wasm_bindgen_futures::spawn_local(async move {
                let result = file
                    .bytes_async()
                    .await
                    .and_then(|bytes| String::from_utf8(bytes).map_err(|e| e.to_string()));
                let event = match result {
                    Ok(content) => FileEvent::Opened(content, name),
                    Err(e) => FileEvent::Error(e),
                };
                let _ = tx.send(event);
            });
        }
        while let Ok(event) = self.rx.try_recv() {
            match event {
                FileEvent::Opened(text, name) => self.request(Pending::Loaded(text, name)),
                FileEvent::Saved(revision, name) => {
                    let result = self.shared.lock().unwrap().mark_saved(revision);
                    match result {
                        Ok(()) => {
                            self.file_name = name;
                            if let Some(p) = self.pending.take() {
                                self.perform(p);
                            }
                        }
                        Err(e) => self.status = e.to_string(),
                    }
                }
                FileEvent::Error(e) => self.status = e,
            }
        }
        #[cfg(target_arch = "wasm32")]
        crate::web::pump(self, &ctx);
        {
            let mut state = self.shared.lock().unwrap();
            state.view.resolved_dark = ui.visuals().dark_mode;
        }
        let s = self.shared.lock().unwrap().clone();
        self.language = s.view.language.clone();
        if s.revision > 0 {
            self.dialogs.startup = false;
        }
        match s.view.theme {
            Theme::Dark => ctx.set_visuals(egui::Visuals::dark()),
            Theme::Light => ctx.set_visuals(egui::Visuals::light()),
            Theme::System => ctx.set_theme(egui::ThemePreference::System),
        }
        if s.revision != self.last_revision || s.dirty() != self.last_dirty {
            self.last_revision = s.revision;
            self.last_dirty = s.dirty();
            #[cfg(target_arch = "wasm32")]
            crate::web::checkpoint(&s);
        }
        if self.has_modal() && ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.pending = None;
            self.dialogs.metadata = None;
            self.dialogs.block = None;
            self.dialogs.arrow = None;
            self.dialogs.report = None;
            self.dialogs.plugin = None;
            self.dialogs.export = false;
            self.dialogs.colors = false;
            self.dialogs.call = false;
            self.dialogs.swap = false;
            self.dialogs.tree = false;
            self.dialogs.startup = false;
            #[cfg(not(target_arch = "wasm32"))]
            if !self.has_document {
                self.allow_close = true;
            }
        } else if !self.has_modal() {
            self.keyboard(&ctx, &s);
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close && s.dirty() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Pending::Close);
        }
        if self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
            "{}{} — erlu IDEF0",
            if s.dirty() { "*" } else { "" },
            if self.file_name.is_empty() {
                "Untitled"
            } else {
                &self.file_name
            }
        )));
        egui::Panel::top("menu").show(ui, |ui| {
            ui.add_enabled_ui(!self.has_modal(), |ui| self.menu(ui, &s));
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if s.view.diagram != s.document.root && ui.button("↑").clicked() {
                    self.execute(Command::NavigateUp {});
                }
                ui.label(format!(
                    "{} · {}",
                    s.document.node(s.view.diagram),
                    s.document.title(s.view.diagram)
                ));
                ui.separator();
                if self.status.is_empty() {
                    ui.label(self.tr(
                        "RMB: add block. LMB on port: drag arrow. Double click block: rename.",
                    ));
                } else {
                    ui.colored_label(Color32::from_rgb(200, 100, 60), &self.status);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("{:.0}%", s.view.zoom * 100.));
                });
            });
        });
        egui::CentralPanel::default().show(ui, |ui| self.canvas(ui, &s));
        self.dialogs(&ctx, &s);
        ctx.request_repaint_after(std::time::Duration::from_millis(150));
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string("language", self.language.clone());
        storage.set_string(
            "theme",
            serde_json::to_string(&self.shared.lock().unwrap().view.theme).unwrap(),
        );
    }
    #[cfg(target_arch = "wasm32")]
    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}
fn field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.label(label);
    ui.text_edit_singleline(value);
    ui.end_row();
}
fn parse_color(s: &str) -> Color32 {
    if s.len() == 7
        && let Ok(v) = u32::from_str_radix(&s[1..], 16)
    {
        return Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
    }
    Color32::TRANSPARENT
}
fn inside(p: Point, min: Point, max: Point) -> bool {
    p.x >= min.x && p.x <= max.x && p.y >= min.y && p.y <= max.y
}
fn hit_object(s: &Engine, p: Point, tolerance: f64) -> Option<Id> {
    let d = &s.document.diagrams[&s.view.diagram];
    for b in d.blocks.values().rev() {
        if inside(p, b.pos, b.pos + Point::new(53., 26.5)) {
            return Some(b.id);
        }
    }
    for j in d.junctions.values() {
        if j.pos.distance(p) < tolerance {
            return Some(j.id);
        }
    }
    for a in d.arrows.values() {
        if !a.label_hidden && !a.label.is_empty() {
            let label = render::label_position(
                &render::route(&s.document, s.view.diagram, a),
                a.label_offset,
            );
            if (p.y - label.y).abs() < 2.5
                && (p.x - label.x).abs() < a.label.chars().count() as f64 * 0.85 + 2.
            {
                return Some(a.id);
            }
        }
    }
    d.arrows
        .values()
        .filter_map(|a| {
            render::nearest_on_path(p, &render::route(&s.document, s.view.diagram, a))
                .filter(|(_, _, dist)| *dist < tolerance)
                .map(|(_, _, dist)| (a.id, dist))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}
fn hit_endpoint(s: &Engine, p: Point, tolerance: f64) -> Option<Endpoint> {
    let d = &s.document.diagrams[&s.view.diagram];
    for b in d.blocks.values().rev() {
        let q = p - b.pos;
        let edges = [
            (Port::Input, Point::new(0., 0.), Point::new(0., 26.5)),
            (Port::Output, Point::new(53., 0.), Point::new(53., 26.5)),
            (Port::Control, Point::new(0., 0.), Point::new(53., 0.)),
            (Port::Mechanism, Point::new(0., 26.5), Point::new(53., 26.5)),
        ];
        if let Some((port, proj, dist)) = edges
            .into_iter()
            .map(|(port, a, c)| {
                let v = render::project(q, a, c);
                (port, v, v.distance(q))
            })
            .min_by(|a, b| a.2.total_cmp(&b.2))
            && dist < tolerance
        {
            let t = match port {
                Port::Input | Port::Output => proj.y / 26.5,
                _ => proj.x / 53.,
            };
            return Some(Endpoint::new(Anchor::Block {
                id: b.id,
                port,
                t: t.clamp(0., 1.),
            }));
        }
    }
    for j in d.junctions.values() {
        if p.distance(j.pos) < tolerance {
            return Some(Endpoint::new(Anchor::Junction { id: j.id }));
        }
    }
    let (lo, hi) = s.document.content();
    let edges = [
        (Port::Input, Point::new(lo.x, lo.y), Point::new(lo.x, hi.y)),
        (Port::Output, Point::new(hi.x, lo.y), Point::new(hi.x, hi.y)),
        (
            Port::Control,
            Point::new(lo.x, lo.y),
            Point::new(hi.x, lo.y),
        ),
        (
            Port::Mechanism,
            Point::new(lo.x, hi.y),
            Point::new(hi.x, hi.y),
        ),
    ];
    let (port, proj, dist) = edges
        .into_iter()
        .map(|(port, a, b)| {
            let q = render::project(p, a, b);
            (port, q, q.distance(p))
        })
        .min_by(|a, b| a.2.total_cmp(&b.2))?;
    if dist < tolerance {
        let t = match port {
            Port::Input | Port::Output => (proj.y - lo.y) / (hi.y - lo.y),
            _ => (proj.x - lo.x) / (hi.x - lo.x),
        };
        return Some(Endpoint {
            anchor: Anchor::Boundary { port, t },
            tunneled: s.view.diagram != s.document.root,
        });
    }
    None
}
fn default_language() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    let locale = std::env::var("LANG").unwrap_or_default();
    #[cfg(target_arch = "wasm32")]
    let locale = js_sys::Reflect::get(&web_sys::window().unwrap(), &"navigator".into())
        .ok()
        .and_then(|v| js_sys::Reflect::get(&v, &"language".into()).ok())
        .and_then(|v| v.as_string())
        .unwrap_or_default();
    locale
        .get(..2)
        .filter(|s| ["en", "ru", "fr"].contains(s))
        .unwrap_or("en")
        .into()
}

fn form_defaults(schema: &Value) -> Value {
    let mut args = serde_json::Map::new();
    if let Some(props) = schema["properties"].as_object() {
        for (key, value) in props {
            let initial =
                value
                    .get("default")
                    .cloned()
                    .unwrap_or_else(|| match value["type"].as_str() {
                        Some("boolean") => json!(false),
                        Some("integer") | Some("number") => json!(0),
                        Some("array") => json!([]),
                        Some("object") => json!({}),
                        _ => json!(""),
                    });
            args.insert(key.clone(), initial);
        }
    }
    Value::Object(args)
}
fn schema_form(ui: &mut egui::Ui, schema: &Value, args: &mut Value) {
    if let Some(props) = schema["properties"].as_object() {
        for (key, spec) in props {
            ui.push_id(key, |ui| {
                let label = spec["title"].as_str().unwrap_or(key);
                if let Some(options) = spec["enum"].as_array() {
                    egui::ComboBox::from_label(label)
                        .selected_text(args[key].to_string())
                        .show_ui(ui, |ui| {
                            for option in options {
                                ui.selectable_value(
                                    &mut args[key],
                                    option.clone(),
                                    option
                                        .as_str()
                                        .map(str::to_string)
                                        .unwrap_or_else(|| option.to_string()),
                                );
                            }
                        });
                } else {
                    match spec["type"].as_str() {
                        Some("boolean") => {
                            let mut v = args[key].as_bool().unwrap_or(false);
                            if ui.checkbox(&mut v, label).changed() {
                                args[key] = json!(v);
                            }
                        }
                        Some("integer") | Some("number") => {
                            let mut v = args[key].as_f64().unwrap_or(0.);
                            ui.horizontal(|ui| {
                                ui.label(label);
                                if ui.add(egui::DragValue::new(&mut v)).changed() {
                                    args[key] = if spec["type"] == "integer" {
                                        json!(v as i64)
                                    } else {
                                        json!(v)
                                    };
                                }
                            });
                        }
                        _ => {
                            let mut v = args[key]
                                .as_str()
                                .map(str::to_string)
                                .unwrap_or_else(|| args[key].to_string());
                            ui.label(label);
                            if ui.text_edit_singleline(&mut v).changed() {
                                args[key] = if spec["type"] == "array" || spec["type"] == "object" {
                                    serde_json::from_str(&v).unwrap_or(json!(v))
                                } else {
                                    json!(v)
                                };
                            }
                        }
                    }
                }
            });
        }
    }
}
