use crate::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize, thiserror::Error)]
#[error("{code}: {message}")]
pub struct Error {
    pub code: String,
    pub message: String,
}
pub type Result<T> = std::result::Result<T, Error>;
pub fn error(code: &str, message: impl Into<String>) -> Error {
    Error {
        code: code.into(),
        message: message.into(),
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct View {
    pub diagram: Id,
    pub selection: BTreeSet<Id>,
    pub zoom: f64,
    pub center: Point,
    pub theme: Theme,
    pub language: String,
    pub resolved_dark: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Response {
    pub revision: u64,
    pub dirty: bool,
    pub result: Value,
}
#[derive(Clone)]
pub struct Engine {
    pub document: Document,
    pub view: View,
    pub revision: u64,
    saved: Document,
    history: Vec<Document>,
    cursor: usize,
    high_water: Id,
}
impl Default for Engine {
    fn default() -> Self {
        Self::new(Document::default()).unwrap()
    }
}
impl Engine {
    pub fn new(document: Document) -> Result<Self> {
        validate_structure(&document)?;
        Ok(Self {
            view: View {
                diagram: document.root,
                selection: BTreeSet::new(),
                zoom: 1.,
                center: Point::default(),
                theme: Theme::System,
                language: "en".into(),
                resolved_dark: false,
            },
            high_water: document.id_sequence,
            saved: document.clone(),
            history: vec![document.clone()],
            document,
            revision: 0,
            cursor: 0,
        })
    }
    pub fn next_id(&self) -> Id {
        self.high_water.max(self.document.id_sequence)
    }
    fn allocate_id(&mut self) -> Id {
        let id = self.next_id();
        self.document.id_sequence = id + 1;
        self.high_water = id + 1;
        id
    }
    pub fn serialize_document(&self) -> String {
        let mut doc = self.document.clone();
        doc.id_sequence = self.next_id();
        serde_json::to_string_pretty(&doc).unwrap()
    }
    pub fn recovery_json(&self) -> String {
        json!({"recovery_version":1,"document":serde_json::from_str::<Value>(&self.serialize_document()).unwrap(),"saved":self.saved,"dirty":self.dirty()}).to_string()
    }
    pub fn restore_recovery(&mut self, text: &str) -> Result<()> {
        if self.dirty() {
            return Err(error(
                "unsaved_changes",
                "Save or discard before restoring recovery",
            ));
        }
        let value: Value =
            serde_json::from_str(text).map_err(|e| error("invalid_document", e.to_string()))?;
        let doc = read_document(&value["document"].to_string())?;
        let saved = read_document(&value["saved"].to_string())?;
        let mut restored = Self::new(doc)?;
        restored.saved = saved;
        restored.revision = self.revision + 1;
        restored.view.language = self.view.language.clone();
        restored.view.theme = self.view.theme;
        *self = restored;
        Ok(())
    }
    pub fn dirty(&self) -> bool {
        self.document != self.saved
    }
    pub fn mark_saved(&mut self, revision: u64) -> Result<()> {
        if self.revision != revision {
            return Err(error("revision_conflict", "Document changed during save"));
        }
        self.saved = self.document.clone();
        Ok(())
    }
    pub fn dispatch(&mut self, command: Command, expected: Option<u64>) -> Result<Response> {
        if expected.is_some_and(|r| r != self.revision) {
            return Err(error(
                "revision_conflict",
                format!("Expected {:?}, current {}", expected, self.revision),
            ));
        }
        let before = self.clone();
        let lifecycle = matches!(
            command,
            Command::DocumentNew { .. } | Command::DocumentOpen { .. }
        );
        let is_history = matches!(
            command,
            Command::Undo {}
                | Command::Redo {}
                | Command::DocumentNew { .. }
                | Command::DocumentOpen { .. }
        );
        let result = match self.apply(command).and_then(|r| {
            validate_structure(&self.document)?;
            Ok(r)
        }) {
            Ok(r) => r,
            Err(e) => {
                *self = before;
                return Err(e);
            }
        };
        if self.document != before.document || lifecycle {
            self.revision = before.revision + 1;
            if !is_history {
                self.history.truncate(self.cursor + 1);
                self.history.push(self.document.clone());
                self.cursor += 1;
            }
        }
        if !self.document.diagrams.contains_key(&self.view.diagram) {
            self.view.diagram = self.document.root;
        }
        self.view
            .selection
            .retain(|id| self.document.diagram_of(*id) == Some(self.view.diagram));
        Ok(Response {
            revision: self.revision,
            dirty: self.dirty(),
            result,
        })
    }
    pub fn call(&mut self, name: &str, args: Value, expected: Option<u64>) -> Result<Response> {
        let cmd =
            decode_action(name, args).map_err(|e| error("invalid_arguments", e.to_string()))?;
        self.dispatch(cmd, expected)
    }
    fn diagram_mut(&mut self, id: Id) -> Result<&mut Diagram> {
        self.document
            .diagrams
            .get_mut(&id)
            .ok_or_else(|| error("not_found", "Diagram not found"))
    }
    fn object_diagram(&self, id: Id) -> Result<Id> {
        self.document
            .diagram_of(id)
            .ok_or_else(|| error("not_found", format!("Object {id} not found")))
    }
    fn block_mut(&mut self, id: Id) -> Result<&mut Block> {
        let d = self.object_diagram(id)?;
        self.diagram_mut(d)?
            .blocks
            .get_mut(&id)
            .ok_or_else(|| error("not_found", "Block not found"))
    }
    fn arrow_mut(&mut self, id: Id) -> Result<&mut Arrow> {
        let d = self.object_diagram(id)?;
        self.diagram_mut(d)?
            .arrows
            .get_mut(&id)
            .ok_or_else(|| error("not_found", "Arrow not found"))
    }
    fn apply(&mut self, command: Command) -> Result<Value> {
        match command {
            Command::DocumentNew { meta, discard } => {
                if self.dirty() && !discard {
                    return Err(error(
                        "unsaved_changes",
                        "Save or explicitly discard the current document",
                    ));
                }
                let doc = Document::with_meta(*meta);
                let rev = self.revision;
                let theme = self.view.theme;
                let language = self.view.language.clone();
                let resolved_dark = self.view.resolved_dark;
                *self = Self::new(doc)?;
                self.revision = rev;
                self.view.theme = theme;
                self.view.language = language;
                self.view.resolved_dark = resolved_dark;
                Ok(json!({}))
            }
            Command::DocumentOpen {
                json: content,
                discard,
            } => {
                if self.dirty() && !discard {
                    return Err(error(
                        "unsaved_changes",
                        "Save or explicitly discard the current document",
                    ));
                }
                let doc = read_document(&content)?;
                let rev = self.revision;
                let theme = self.view.theme;
                let language = self.view.language.clone();
                let resolved_dark = self.view.resolved_dark;
                *self = Self::new(doc)?;
                self.revision = rev;
                self.view.theme = theme;
                self.view.language = language;
                self.view.resolved_dark = resolved_dark;
                Ok(json!({}))
            }
            Command::DocumentSerialize {} => Ok(json!({"json":self.serialize_document()})),
            Command::DocumentSaved { revision } => {
                self.mark_saved(revision)?;
                Ok(json!({}))
            }
            Command::DocumentMetadata { patch } => {
                if !patch.is_object() {
                    return Err(error(
                        "invalid_arguments",
                        "Metadata patch must be an object",
                    ));
                }
                let mut v = serde_json::to_value(&self.document.meta).unwrap();
                for (k, val) in patch.as_object().unwrap() {
                    v[k] = val.clone();
                }
                self.document.meta = serde_json::from_value(v)
                    .map_err(|e| error("invalid_arguments", e.to_string()))?;
                Ok(json!({}))
            }
            Command::DocumentState {} => Ok(
                json!({"document":self.document,"view":self.view,"can_undo":self.cursor>0,"can_redo":self.cursor+1<self.history.len()}),
            ),
            Command::NodeTree {} => Ok(self.tree(self.document.root)),
            Command::ObjectGet { id } => {
                let d = self.object_diagram(id)?;
                let diag = &self.document.diagrams[&d];
                Ok(
                    json!({"diagram":d,"object":diag.blocks.get(&id).map(|v|json!(v)).or_else(||diag.arrows.get(&id).map(|v|json!(v))).or_else(||diag.junctions.get(&id).map(|v|json!(v))).unwrap()}),
                )
            }
            Command::BlockCreate {
                diagram,
                pos,
                title,
            } => {
                let d = diagram.unwrap_or(self.view.diagram);
                let id = self.allocate_id();
                let number = self.next_number(d)?;
                let pos = clamp_block(&self.document, pos);
                self.diagram_mut(d)?.blocks.insert(
                    id,
                    Block {
                        id,
                        title: if title.is_empty() {
                            "Function".into()
                        } else {
                            title
                        },
                        number,
                        pos,
                        price: None,
                        color: None,
                        child: None,
                        extra: Extra::new(),
                    },
                );
                Ok(json!({"id":id}))
            }
            Command::BlockUpdate {
                id,
                title,
                price,
                clear_price,
            } => {
                let b = self.block_mut(id)?;
                if let Some(t) = title {
                    b.title = t.replace('\r', "");
                }
                if clear_price {
                    b.price = None;
                } else if let Some(p) = price {
                    b.price = Some(p);
                }
                self.refresh_calls();
                Ok(json!({}))
            }
            Command::ObjectsMove { positions } => {
                for (id, pos) in positions {
                    let d = self.object_diagram(id)?;
                    let pos_block = clamp_block(&self.document, pos);
                    let diag = self.diagram_mut(d)?;
                    if let Some(b) = diag.blocks.get_mut(&id) {
                        b.pos = pos_block;
                    } else if let Some(j) = diag.junctions.get_mut(&id) {
                        j.pos = pos;
                    } else {
                        return Err(error(
                            "invalid_arguments",
                            "Use arrow_update for arrow geometry",
                        ));
                    }
                }
                Ok(json!({}))
            }
            Command::ObjectsDelete { ids } => {
                for id in &ids {
                    self.object_diagram(*id)?;
                }
                for id in ids {
                    if self.document.diagram_of(id).is_some() {
                        self.delete_object(id)?;
                    }
                }
                self.refresh_interfaces()?;
                Ok(json!({}))
            }
            Command::NumbersSwap { first, second } => {
                if self.object_diagram(first)? != self.object_diagram(second)? {
                    return Err(error(
                        "invalid_arguments",
                        "Blocks must belong to the same diagram",
                    ));
                }
                let a = self
                    .document
                    .block(first)
                    .ok_or_else(|| error("not_found", "First block not found"))?
                    .number
                    .clone();
                let b = self
                    .document
                    .block(second)
                    .ok_or_else(|| error("not_found", "Second block not found"))?
                    .number
                    .clone();
                self.block_mut(first)?.number = b;
                self.block_mut(second)?.number = a;
                self.renumber_children(first)?;
                self.renumber_children(second)?;
                self.refresh_calls();
                Ok(json!({}))
            }
            Command::ArrowCreate {
                diagram,
                from,
                to,
                label,
            } => {
                let d = diagram.unwrap_or(self.view.diagram);
                let id = self.create_arrow(d, from, to, label)?;
                self.refresh_interfaces()?;
                Ok(json!({"id":id}))
            }
            Command::ArrowUpdate {
                id,
                from,
                to,
                label,
                label_offset,
                label_hidden,
                bends,
            } => {
                let inherited = self.arrow_mut(id)?.interface.is_some();
                if inherited && label.is_some() {
                    return Err(error(
                        "locked_label",
                        "Interface labels inherit their parent arrow",
                    ));
                }
                let a = self.arrow_mut(id)?;
                if let Some(v) = from {
                    a.from = v;
                }
                if let Some(v) = to {
                    a.to = v;
                }
                if let Some(v) = label {
                    a.label = v;
                    a.label_source = None;
                    a.label_hidden = false;
                }
                if let Some(v) = label_offset {
                    a.label_offset = v;
                }
                if let Some(v) = label_hidden {
                    a.label_hidden = v;
                }
                if let Some(v) = bends {
                    a.bends = v;
                }
                self.propagate_labels(id);
                self.sync_tunnels(id)?;
                self.refresh_interfaces()?;
                Ok(json!({}))
            }
            Command::ArrowBranch { id, point, target } => {
                let d = self.object_diagram(id)?;
                let original = self.document.diagrams[&d]
                    .arrows
                    .get(&id)
                    .cloned()
                    .ok_or_else(|| error("not_found", "Arrow not found"))?;
                if original.call_ref.is_some() {
                    return Err(error("invalid_arguments", "Cannot branch a call mechanism"));
                }
                let junction = self.allocate_id();
                self.diagram_mut(d)?.junctions.insert(
                    junction,
                    Junction {
                        id: junction,
                        pos: point,
                        extra: Extra::new(),
                    },
                );
                let mid = Endpoint::new(Anchor::Junction { id: junction });
                self.arrow_mut(id)?.to = mid.clone();
                let forward =
                    self.create_arrow(d, mid.clone(), original.to, original.label.clone())?;
                let source = original.label_source.unwrap_or(id);
                {
                    let a = self.arrow_mut(forward)?;
                    a.label_source = Some(source);
                    a.label_hidden = !a.label.is_empty();
                }
                let branch = if let Some(to) = target {
                    let n = self.create_arrow(d, mid, to, original.label)?;
                    let a = self.arrow_mut(n)?;
                    a.label_source = Some(source);
                    a.label_hidden = !a.label.is_empty();
                    Some(n)
                } else {
                    None
                };
                self.refresh_interfaces()?;
                Ok(json!({"junction":junction,"forward":forward,"branch":branch}))
            }
            Command::InterfaceConnect { id, endpoint } => {
                let a = self.arrow_mut(id)?;
                let iface = a
                    .interface
                    .as_mut()
                    .ok_or_else(|| error("invalid_arguments", "Not an interface arrow"))?;
                if !iface.stub {
                    return Err(error("invalid_arguments", "Interface is already connected"));
                }
                if iface.port == Port::Output {
                    a.from = endpoint;
                } else {
                    a.to = endpoint;
                }
                iface.stub = false;
                Ok(json!({}))
            }
            Command::InterfaceMove { id, t } => {
                let a = self.arrow_mut(id)?;
                let iface = a
                    .interface
                    .as_mut()
                    .ok_or_else(|| error("invalid_arguments", "Not an interface arrow"))?;
                iface.t = t;
                for ep in [&mut a.from, &mut a.to] {
                    if let Anchor::Boundary { t: old, .. } = &mut ep.anchor {
                        *old = t;
                    }
                }
                Ok(json!({}))
            }
            Command::CallMechanism {
                block,
                reference,
                t,
            } => {
                let d = self.object_diagram(block)?;
                if block == reference || self.object_diagram(reference)? != d {
                    return Err(error(
                        "invalid_arguments",
                        "Reference must be a different block in this diagram",
                    ));
                }
                let target = self
                    .document
                    .block(reference)
                    .ok_or_else(|| error("not_found", "Reference not found"))?;
                if target.number == "A0" {
                    return Err(error("invalid_arguments", "A0 cannot be a call reference"));
                }
                if self.document.diagrams[&d].arrows.values().any(|a| {
                    a.call_ref.is_some() && matches!(a.to.anchor,Anchor::Block{id,..}if id==block)
                }) {
                    return Err(error("already_exists", "Call mechanism already exists"));
                }
                let label = format!("{} {}", target.number, target.title);
                let id = self.allocate_id();
                let a = Arrow {
                    id,
                    from: Endpoint::new(Anchor::Boundary {
                        port: Port::Mechanism,
                        t,
                    }),
                    to: Endpoint::new(Anchor::Block {
                        id: block,
                        port: Port::Mechanism,
                        t: 0.5,
                    }),
                    label,
                    label_offset: Point::default(),
                    label_hidden: false,
                    label_source: None,
                    bends: vec![],
                    color: None,
                    interface: None,
                    call_ref: Some(reference),
                    extra: Extra::new(),
                };
                self.diagram_mut(d)?.arrows.insert(id, a);
                Ok(json!({"id":id}))
            }
            Command::ColorsSet { ids, color } => {
                if color.as_ref().is_some_and(|c| !valid_color(c)) {
                    return Err(error("invalid_arguments", "Expected #rrggbb"));
                }
                for id in ids {
                    let d = self.object_diagram(id)?;
                    let diag = self.diagram_mut(d)?;
                    if let Some(b) = diag.blocks.get_mut(&id) {
                        b.color = color.clone();
                    } else if let Some(a) = diag.arrows.get_mut(&id) {
                        a.color = color.clone();
                    } else {
                        return Err(error(
                            "invalid_arguments",
                            "Only blocks and arrows can have colors",
                        ));
                    }
                }
                Ok(json!({}))
            }
            Command::DiagramState { diagram, state } => {
                self.diagram_mut(diagram.unwrap_or(self.view.diagram))?
                    .state = state;
                Ok(json!({}))
            }
            Command::Decompose { block } => {
                let existing = self
                    .document
                    .block(block)
                    .ok_or_else(|| error("not_found", "Block not found"))?
                    .child;
                let id = if let Some(id) = existing {
                    id
                } else {
                    let id = self.allocate_id();
                    self.document
                        .diagrams
                        .insert(id, Diagram::empty(id, Some(block)));
                    self.block_mut(block)?.child = Some(id);
                    id
                };
                self.refresh_interfaces()?;
                self.view.diagram = id;
                self.view.selection.clear();
                Ok(json!({"diagram":id}))
            }
            Command::Navigate { diagram } => {
                if !self.document.diagrams.contains_key(&diagram) {
                    return Err(error("not_found", "Diagram not found"));
                }
                self.view.diagram = diagram;
                self.view.selection.clear();
                Ok(json!({}))
            }
            Command::NavigateUp {} => {
                if let Some(b) = self.document.diagrams[&self.view.diagram].parent_block {
                    self.view.diagram = self.object_diagram(b)?;
                    self.view.selection.clear();
                }
                Ok(json!({}))
            }
            Command::SelectionSet { ids } => {
                for id in &ids {
                    if self.object_diagram(*id)? != self.view.diagram {
                        return Err(error(
                            "invalid_arguments",
                            "Selection must be in the active diagram",
                        ));
                    }
                }
                self.view.selection = ids.into_iter().collect();
                Ok(json!({}))
            }
            Command::ViewSet {
                zoom,
                center,
                theme,
            } => {
                if let Some(z) = zoom {
                    if !z.is_finite() || !(0.05..=40.).contains(&z) {
                        return Err(error("invalid_arguments", "Zoom must be in [0.05, 40]"));
                    }
                    self.view.zoom = z;
                }
                if let Some(p) = center {
                    if !p.finite() {
                        return Err(error("invalid_arguments", "Invalid view center"));
                    }
                    self.view.center = p;
                }
                if let Some(t) = theme {
                    self.view.theme = t;
                }
                Ok(json!({}))
            }
            Command::LanguageSet { language } => {
                if !["en", "ru", "fr"].contains(&language.as_str()) {
                    return Err(error(
                        "invalid_arguments",
                        "Supported languages: en, ru, fr",
                    ));
                }
                self.view.language = language;
                Ok(json!({}))
            }
            Command::ViewFit {} => {
                self.view.center = Point::default();
                self.view.zoom = 1.;
                Ok(json!({}))
            }
            Command::Undo {} => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.document = self.history[self.cursor].clone();
                }
                Ok(json!({}))
            }
            Command::Redo {} => {
                if self.cursor + 1 < self.history.len() {
                    self.cursor += 1;
                    self.document = self.history[self.cursor].clone();
                }
                Ok(json!({}))
            }
            Command::Validate {} => Ok(json!(crate::validation::validate(&self.document))),
            Command::CalculatePrices {} => {
                let mut count = 0;
                self.recalculate(self.document.root, &mut count)?;
                Ok(json!({"updated_blocks":count}))
            }
            Command::ExportMarkdown {} => {
                Ok(json!({"text":crate::render::markdown(&self.document)}))
            }
            Command::ExportSvg { diagram, light } => {
                let d = diagram.unwrap_or(self.view.diagram);
                if !self.document.diagrams.contains_key(&d) {
                    return Err(error("not_found", "Diagram not found"));
                }
                Ok(
                    json!({"text":crate::render::svg_localized(&self.document,d,!light&&(self.view.theme==Theme::Dark||self.view.theme==Theme::System&&self.view.resolved_dark),None,&self.view.language)}),
                )
            }
            Command::ExportPdf {
                all,
                number_pages,
                light,
            } => {
                let ids = if all {
                    self.order()
                } else {
                    vec![self.view.diagram]
                };
                let bytes = crate::render::pdf_localized(
                    &self.document,
                    &ids,
                    number_pages,
                    !light
                        && (self.view.theme == Theme::Dark
                            || self.view.theme == Theme::System && self.view.resolved_dark),
                    &self.view.language,
                )?;
                Ok(json!({"bytes":bytes,"mime":"application/pdf"}))
            }
            Command::ExtensionSet {
                namespace,
                version,
                data,
            } => {
                if namespace.is_empty() || version == 0 {
                    return Err(error(
                        "invalid_arguments",
                        "Extension namespace and positive version required",
                    ));
                }
                self.document
                    .extensions
                    .insert(namespace, json!({"version":version,"data":data}));
                Ok(json!({}))
            }
            Command::Transaction { commands } => {
                if commands.len() > 1000 {
                    return Err(error(
                        "invalid_arguments",
                        "At most 1000 commands per transaction",
                    ));
                }
                let mut results = vec![];
                for c in commands {
                    if !c.edits_document() || matches!(c, Command::Transaction { .. }) {
                        return Err(error(
                            "invalid_arguments",
                            "Transactions accept non-nested document edits only",
                        ));
                    }
                    results.push(self.apply(c)?);
                }
                Ok(json!(results))
            }
        }
    }
    pub fn order(&self) -> Vec<Id> {
        fn visit(doc: &Document, id: Id, out: &mut Vec<Id>) {
            out.push(id);
            for b in doc.diagrams[&id].blocks.values() {
                if let Some(c) = b.child {
                    visit(doc, c, out);
                }
            }
        }
        let mut out = vec![];
        visit(&self.document, self.document.root, &mut out);
        out
    }
    fn tree(&self, id: Id) -> Value {
        let d = &self.document.diagrams[&id];
        json!({"diagram":id,"node":self.document.node(id),"title":self.document.title(id),"blocks":d.blocks.values().map(|b|json!({"id":b.id,"number":b.number,"title":b.title,"child":b.child.map(|c|self.tree(c))})).collect::<Vec<_>>()})
    }
    fn next_number(&self, d: Id) -> Result<String> {
        let diag = self
            .document
            .diagrams
            .get(&d)
            .ok_or_else(|| error("not_found", "Diagram not found"))?;
        if d == self.document.root {
            return Ok(format!("A{}", "0".repeat(diag.blocks.len() + 1)));
        }
        let prefix = self.document.node(d);
        let prefix = if prefix == "A0" {
            "A".to_string()
        } else {
            format!("{prefix}.")
        };
        let n = diag
            .blocks
            .values()
            .filter_map(|b| b.number.strip_prefix(&prefix)?.parse::<u32>().ok())
            .max()
            .unwrap_or(0)
            + 1;
        Ok(format!("{prefix}{n}"))
    }
    fn create_arrow(&mut self, d: Id, from: Endpoint, to: Endpoint, label: String) -> Result<Id> {
        let id = self.allocate_id();
        self.diagram_mut(d)?.arrows.insert(
            id,
            Arrow {
                id,
                from,
                to,
                label,
                label_offset: Point::default(),
                label_hidden: false,
                label_source: None,
                bends: vec![],
                color: None,
                interface: None,
                call_ref: None,
                extra: Extra::new(),
            },
        );
        Ok(id)
    }
    fn renumber_children(&mut self, id: Id) -> Result<()> {
        let b = self.document.block(id).unwrap().clone();
        if let Some(c) = b.child {
            let ids = self.document.diagrams[&c]
                .blocks
                .keys()
                .copied()
                .collect::<Vec<_>>();
            for (i, child) in ids.into_iter().enumerate() {
                self.block_mut(child)?.number = if b.number == "A0" {
                    format!("A{}", i + 1)
                } else {
                    format!("{}.{}", b.number, i + 1)
                };
                self.renumber_children(child)?;
            }
        }
        Ok(())
    }
    fn refresh_calls(&mut self) {
        let labels: BTreeMap<Id, String> = self
            .document
            .diagrams
            .values()
            .flat_map(|d| {
                d.blocks
                    .values()
                    .map(|b| (b.id, format!("{} {}", b.number, b.title)))
            })
            .collect();
        for d in self.document.diagrams.values_mut() {
            for a in d.arrows.values_mut() {
                if let Some(id) = a.call_ref
                    && let Some(l) = labels.get(&id)
                {
                    a.label = l.clone();
                }
            }
        }
    }
    fn propagate_labels(&mut self, id: Id) {
        let label = self
            .document
            .diagrams
            .values()
            .find_map(|d| d.arrows.get(&id).map(|a| a.label.clone()));
        if let Some(label) = label {
            for d in self.document.diagrams.values_mut() {
                for a in d.arrows.values_mut() {
                    if a.label_source == Some(id) {
                        a.label = label.clone();
                        a.label_hidden = !label.is_empty();
                    }
                }
            }
        }
    }
    fn delete_object(&mut self, id: Id) -> Result<()> {
        let d = self.object_diagram(id)?;
        if let Some(child) = self.document.diagrams[&d]
            .blocks
            .get(&id)
            .and_then(|b| b.child)
        {
            self.delete_diagram(child);
        }
        let diag = self.diagram_mut(d)?;
        diag.blocks.remove(&id);
        diag.junctions.remove(&id);
        diag.arrows.remove(&id);
        let removed: Vec<_> = diag
            .arrows
            .values()
            .filter(|a| ep_has(&a.from, id) || ep_has(&a.to, id) || a.call_ref == Some(id))
            .map(|a| a.id)
            .collect();
        let removed_set: BTreeSet<_> = removed.iter().copied().chain([id]).collect();
        for a in removed {
            diag.arrows.remove(&a);
        }
        for a in diag.arrows.values_mut() {
            if a.label_source.is_some_and(|s| removed_set.contains(&s)) {
                a.label_source = None;
                a.label_hidden = false;
            }
        }
        let used: BTreeSet<_> = diag
            .arrows
            .values()
            .flat_map(|a| [&a.from, &a.to])
            .filter_map(|ep| {
                if let Anchor::Junction { id } = ep.anchor {
                    Some(id)
                } else {
                    None
                }
            })
            .collect();
        diag.junctions.retain(|id, _| used.contains(id));
        Ok(())
    }
    fn delete_diagram(&mut self, id: Id) {
        if let Some(d) = self.document.diagrams.remove(&id) {
            for b in d.blocks.values() {
                if let Some(c) = b.child {
                    self.delete_diagram(c);
                }
            }
        }
    }
    fn recalculate(&mut self, d: Id, count: &mut usize) -> Result<Option<f64>> {
        let ids = self.document.diagrams[&d]
            .blocks
            .keys()
            .copied()
            .collect::<Vec<_>>();
        let mut sum = 0.;
        let mut any = false;
        for id in ids {
            let child = self.document.block(id).unwrap().child;
            if let Some(c) = child
                && let Some(p) = self.recalculate(c, count)?
            {
                let b = self.block_mut(id)?;
                if b.price != Some(p) {
                    b.price = Some(p);
                    *count += 1;
                }
            }
            if let Some(p) = self.document.block(id).unwrap().price {
                sum += p;
                any = true;
            }
        }
        Ok(any.then_some(sum))
    }
    fn refresh_interfaces(&mut self) -> Result<()> {
        let parents = self.order();
        for d in parents {
            let blocks = self.document.diagrams[&d]
                .blocks
                .values()
                .filter_map(|b| b.child.map(|c| (b.clone(), c)))
                .collect::<Vec<_>>();
            for (block, child) in blocks {
                let parent_arrows = self.document.diagrams[&d]
                    .arrows
                    .values()
                    .filter(|a| a.call_ref.is_none())
                    .cloned()
                    .collect::<Vec<_>>();
                let mut wanted = BTreeSet::new();
                for a in parent_arrows {
                    let ep = if ep_has(&a.from, block.id) {
                        Some(&a.from)
                    } else if ep_has(&a.to, block.id) {
                        Some(&a.to)
                    } else {
                        None
                    };
                    let Some(ep) = ep else { continue };
                    let Anchor::Block { port, t, .. } = ep.anchor else {
                        continue;
                    };
                    wanted.insert(a.id);
                    let existing = self.document.diagrams[&child]
                        .arrows
                        .values()
                        .find(|x| x.interface.as_ref().is_some_and(|i| i.parent_arrow == a.id))
                        .map(|x| x.id);
                    if let Some(id) = existing {
                        let v = self.arrow_mut(id)?;
                        v.label = if ep.tunneled {
                            String::new()
                        } else {
                            a.label.clone()
                        };
                        if let Some(i) = &mut v.interface {
                            i.port = port;
                        }
                        for e in [&mut v.from, &mut v.to] {
                            if let Anchor::Boundary { port: p, .. } = &mut e.anchor {
                                *p = port;
                                e.tunneled = ep.tunneled;
                            }
                        }
                    } else {
                        let edge = Endpoint {
                            anchor: Anchor::Boundary { port, t },
                            tunneled: ep.tunneled,
                        };
                        let id = self.create_arrow(
                            child,
                            edge.clone(),
                            edge,
                            if ep.tunneled { String::new() } else { a.label },
                        )?;
                        self.arrow_mut(id)?.interface = Some(Interface {
                            parent_arrow: a.id,
                            port,
                            t,
                            stub: true,
                        });
                    }
                }
                self.diagram_mut(child)?.arrows.retain(|_, a| {
                    a.interface
                        .as_ref()
                        .is_none_or(|i| wanted.contains(&i.parent_arrow))
                });
            }
        }
        Ok(())
    }
    fn sync_tunnels(&mut self, id: Id) -> Result<()> {
        let d = self.object_diagram(id)?;
        let a = self.document.diagrams[&d].arrows[&id].clone();
        if let Some(i) = a.interface
            && let Some(block) = self.document.diagrams[&d].parent_block
        {
            let tunneled = if i.port == Port::Output {
                a.to.tunneled
            } else {
                a.from.tunneled
            };
            let parent = self.arrow_mut(i.parent_arrow)?;
            if ep_has(&parent.from, block) {
                parent.from.tunneled = tunneled;
            }
            if ep_has(&parent.to, block) {
                parent.to.tunneled = tunneled;
            }
        }
        Ok(())
    }
}
fn ep_has(ep: &Endpoint, id: Id) -> bool {
    matches!(ep.anchor,Anchor::Block{id:x,..}|Anchor::Junction{id:x}if x==id)
}
pub fn valid_color(c: &str) -> bool {
    c.len() == 7 && c.starts_with('#') && c[1..].bytes().all(|b| b.is_ascii_hexdigit())
}
pub fn clamp_block(doc: &Document, p: Point) -> Point {
    let (min, max) = doc.content();
    Point::new(
        p.x.clamp(min.x, max.x - 53.),
        p.y.clamp(min.y, max.y - 26.5),
    )
}
pub fn read_document(text: &str) -> Result<Document> {
    let mut value: Value =
        serde_json::from_str(text).map_err(|e| error("invalid_document", e.to_string()))?;
    crate::format::migrate(&mut value)?;
    if value["format"] != "erlu" || value["format_version"]["major"] != 1 {
        return Err(error(
            "unsupported_format",
            "Expected erlu format major 1; legacy Qt files are not supported",
        ));
    }
    let mut doc: Document = serde_json::from_value(value.clone())
        .map_err(|e| error("invalid_document", e.to_string()))?;
    doc.preserved = crate::model::unknown_fields(&value, &serde_json::to_value(&doc).unwrap());
    validate_structure(&doc)?;
    Ok(doc)
}
pub fn validate_structure(doc: &Document) -> Result<()> {
    let fail = |s: &str| error("invalid_document", s);
    if doc.format != "erlu" || doc.format_version.major != 1 {
        return Err(error("unsupported_format", "Unsupported format major"));
    }
    crate::format::check_features(doc)?;
    for f in &doc.features.required {
        if !FEATURES.contains(&f.as_str()) {
            return Err(error(
                "unsupported_feature",
                format!("Required feature: {f}"),
            ));
        }
    }
    if !doc.features.required.contains("idef0") {
        return Err(fail("idef0 must be a required feature"));
    }
    if !["A1", "A2", "A3", "A4"].contains(&doc.meta.page_size.as_str()) {
        return Err(fail("Invalid page size"));
    }
    if !["1?", "?1", "1 ?", "? 1"].contains(&doc.meta.currency_placement.as_str()) {
        return Err(fail("Invalid currency placement"));
    }
    if !doc.diagrams.contains_key(&doc.root) || doc.diagrams[&doc.root].parent_block.is_some() {
        return Err(fail("Invalid root diagram"));
    }
    let mut seen = BTreeSet::new();
    let mut child_links = BTreeSet::new();
    if doc.id_sequence < 3 || doc.id_sequence > 9_007_199_254_740_990 {
        return Err(fail("Invalid ID allocator"));
    }
    for (&id, d) in &doc.diagrams {
        if id != d.id || id == 0 || !seen.insert(id) {
            return Err(fail("Duplicate or mismatched diagram ID"));
        }
        if d.notation != "idef0" {
            return Err(error(
                "unsupported_feature",
                format!("Notation {}", d.notation),
            ));
        }
        for (&id, b) in &d.blocks {
            if id != b.id || id == 0 || !seen.insert(id) || !b.pos.finite() {
                return Err(fail("Invalid block identity/position"));
            }
            if b.price
                .is_some_and(|p| !p.is_finite() || !(0.0..=1e9).contains(&p))
            {
                return Err(fail("Price outside [0, 1e9]"));
            }
            if b.color.as_ref().is_some_and(|c| !valid_color(c)) {
                return Err(fail("Invalid block color"));
            }
            if let Some(c) = b.child
                && (!child_links.insert(c)
                    || doc
                        .diagrams
                        .get(&c)
                        .is_none_or(|d| d.parent_block != Some(id)))
            {
                return Err(fail("Invalid decomposition link"));
            }
        }
        for (&id, j) in &d.junctions {
            if id != j.id || id == 0 || !seen.insert(id) || !j.pos.finite() {
                return Err(fail("Invalid junction"));
            }
        }
        for (&id, a) in &d.arrows {
            if id != a.id
                || id == 0
                || !seen.insert(id)
                || !a.label_offset.finite()
                || a.bends.iter().any(|p| !p.finite())
            {
                return Err(fail("Invalid arrow"));
            }
            if a.color.as_ref().is_some_and(|c| !valid_color(c)) {
                return Err(fail("Invalid arrow color"));
            }
            for ep in [&a.from, &a.to] {
                match ep.anchor {
                    Anchor::Block { id, t, .. } => {
                        if !d.blocks.contains_key(&id) || !t.is_finite() || !(0.0..=1.).contains(&t)
                        {
                            return Err(fail("Invalid block endpoint"));
                        }
                    }
                    Anchor::Junction { id } => {
                        if !d.junctions.contains_key(&id) {
                            return Err(fail("Invalid junction endpoint"));
                        }
                    }
                    Anchor::Boundary { t, .. } => {
                        if !t.is_finite() || !(0.0..=1.).contains(&t) {
                            return Err(fail("Invalid boundary endpoint"));
                        }
                    }
                }
            }
            let stub = a.interface.as_ref().is_some_and(|i| i.stub);
            if a.call_ref.is_none() && !stub {
                if matches!(a.from.anchor,Anchor::Block{port,..}if port!=Port::Output)
                    || matches!(
                        a.to.anchor,
                        Anchor::Block {
                            port: Port::Output,
                            ..
                        }
                    )
                {
                    return Err(fail("Invalid arrow port direction"));
                }
                if matches!(a.from.anchor, Anchor::Boundary { .. })
                    && matches!(a.to.anchor, Anchor::Boundary { .. })
                {
                    return Err(fail("Arrow cannot connect two boundaries"));
                }
            }
            if let Some(i) = &a.interface {
                if !i.t.is_finite() || !(0.0..=1.).contains(&i.t) {
                    return Err(fail("Invalid interface position"));
                }
                let Some(parent) = d.parent_block.and_then(|b| doc.diagram_of(b)) else {
                    return Err(fail("Interface requires a parent"));
                };
                if !doc.diagrams[&parent].arrows.contains_key(&i.parent_arrow) {
                    return Err(fail("Missing parent interface arrow"));
                }
            }
            if a.label_source
                .is_some_and(|s| s == id || !d.arrows.contains_key(&s))
            {
                return Err(fail("Invalid inherited label"));
            }
            if a.call_ref.is_some_and(|r| !d.blocks.contains_key(&r)) {
                return Err(fail("Missing call reference"));
            }
        }
    }
    if seen.iter().any(|id| *id >= doc.id_sequence) {
        return Err(fail("ID allocator must exceed every allocated ID"));
    }
    for diagram in doc.diagrams.values() {
        for arrow in diagram.arrows.values() {
            let mut ancestry = BTreeSet::new();
            let mut source = Some(arrow.id);
            while let Some(id) = source {
                if !ancestry.insert(id) {
                    return Err(fail("Cyclic label inheritance"));
                }
                source = diagram.arrows.get(&id).and_then(|a| a.label_source);
            }
        }
    }
    // Detect cycles and disconnected diagrams before recursive traversals.
    let mut visited = BTreeSet::new();
    let mut todo = vec![(doc.root, 0)];
    while let Some((id, depth)) = todo.pop() {
        if depth > 128 {
            return Err(fail("Decomposition depth exceeds 128 levels"));
        }
        if !visited.insert(id) {
            return Err(fail("Cyclic decomposition"));
        }
        for b in doc.diagrams[&id].blocks.values() {
            if let Some(c) = b.child {
                todo.push((c, depth + 1));
            }
        }
    }
    if visited.len() != doc.diagrams.len() {
        return Err(fail("Unreachable diagram"));
    }
    let used = |f: &str| doc.features.required.contains(f);
    if !used("idef0.decomposition") && doc.diagrams.len() > 1 {
        return Err(fail("Decomposition feature missing"));
    }
    for d in doc.diagrams.values() {
        if !used("abc.costs") && d.blocks.values().any(|b| b.price.is_some()) {
            return Err(fail("Cost feature missing"));
        }
        if !used("appearance.colors")
            && (d.blocks.values().any(|b| b.color.is_some())
                || d.arrows.values().any(|a| a.color.is_some()))
        {
            return Err(fail("Color feature missing"));
        }
        if !used("idef0.tunnels") && d.arrows.values().any(|a| a.from.tunneled || a.to.tunneled) {
            return Err(fail("Tunnel feature missing"));
        }
        if !used("idef0.calls") && d.arrows.values().any(|a| a.call_ref.is_some()) {
            return Err(fail("Call feature missing"));
        }
    }
    Ok(())
}
