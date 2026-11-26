use crate::{Command, Engine, Id, Response, Result, Theme, dfd, dfd_render, error};
use serde_json::{Value, json};

#[derive(Clone)]
pub(crate) struct Session {
    pub document: dfd::Document,
    pub saved: dfd::Document,
    pub next: Id,
    history: Vec<dfd::Document>,
    cursor: usize,
}

impl Engine {
    pub fn dfd_document(&self) -> Option<&dfd::Document> {
        self.dfd.as_ref().map(|s| &s.document)
    }

    pub fn from_file(text: &str) -> Result<Self> {
        if Self::is_dfd_json(text) {
            let mut engine = Self::default();
            engine.replace_dfd(dfd::Document::read(text)?);
            Ok(engine)
        } else {
            Self::new(crate::read_document(text)?)
        }
    }

    pub(crate) fn is_dfd_json(text: &str) -> bool {
        serde_json::from_str::<Value>(text).is_ok_and(|v| v["format"] == dfd::FORMAT)
    }

    pub(crate) fn wrong_notation() -> crate::Error {
        error(
            "wrong_notation",
            "This action does not apply to the active notation",
        )
    }

    pub(crate) fn replace_dfd(&mut self, doc: dfd::Document) {
        self.replace_with_idef0(Self::default());
        self.view.diagram = doc.root;
        self.dfd = Some(Session {
            saved: doc.clone(),
            history: vec![doc.clone()],
            next: doc.id_sequence,
            document: doc,
            cursor: 0,
        });
    }

    pub(crate) fn dispatch_dfd(&mut self, command: Command) -> Result<Response> {
        match command {
            Command::DfdNew { ref title, discard } => {
                self.check_discard(discard)?;
                self.replace_dfd(dfd::Document::new(title.clone()));
                return Ok(self.dfd_response(json!({})));
            }
            Command::DocumentOpen { ref json, discard } => {
                self.check_discard(discard)?;
                if Self::is_dfd_json(json) {
                    self.replace_dfd(dfd::Document::read(json)?);
                } else {
                    self.replace_with_idef0(Self::new(crate::read_document(json)?)?);
                }
                return Ok(self.dfd_response(json!({})));
            }
            Command::DocumentNew { ref meta, discard } => {
                self.check_discard(discard)?;
                self.replace_with_idef0(Self::new(crate::Document::with_meta(*meta.clone()))?);
                return Ok(self.dfd_response(json!({})));
            }
            Command::DocumentSerialize {} => {
                return Ok(self.dfd_response(json!({"json":self.serialize_document()})));
            }
            Command::DocumentSaved { revision } => {
                self.mark_saved(revision)?;
                return Ok(self.dfd_response(json!({})));
            }
            Command::ViewFit {} => {
                let (scene, center) =
                    dfd_render::scene(self.dfd_document().unwrap(), self.view.diagram, false, None);
                self.view.zoom = (270. / scene.size.x)
                    .min(180. / scene.size.y)
                    .clamp(0.05, 40.);
                self.view.center = center;
                return Ok(self.dfd_response(json!({})));
            }
            Command::GridSet { .. } | Command::ViewSet { .. } | Command::LanguageSet { .. } => {
                let mut candidate = self.clone();
                let result = candidate.apply(command)?;
                self.view = candidate.view;
                return Ok(self.dfd_response(result));
            }
            _ => {}
        }
        let dark = self.view.theme == Theme::Dark
            || (self.view.theme == Theme::System && self.view.resolved_dark);
        let s = self.dfd.as_mut().ok_or_else(Self::wrong_notation)?;
        let diagram = self.view.diagram;
        let result = match command {
            Command::DocumentState {} => json!({
                "document":s.document, "view":self.view, "can_undo":s.cursor > 0,
                "can_redo":s.cursor + 1 < s.history.len()
            }),
            Command::NodeTree {} => s.document.tree(s.document.root),
            Command::ObjectGet { id } => {
                if let Some(owner) = s.document.owner(id) {
                    let d = &s.document.diagrams[&owner];
                    let object = d
                        .nodes
                        .get(&id)
                        .map(|n| json!(n))
                        .or_else(|| d.flows.get(&id).map(|f| json!(f)));
                    json!({"diagram":owner, "object":object})
                } else if let Some(definition) = s.document.dictionary.get(&id) {
                    json!({"diagram":null, "object":definition})
                } else {
                    return Err(error("not_found", "DFD object not found"));
                }
            }
            Command::Validate {} => json!(s.document.validate()),
            Command::ExportMarkdown {} => json!({"text":dfd_render::markdown(&s.document)}),
            Command::ExportSvg { diagram: id, light } => {
                let id = id.unwrap_or(diagram);
                if !s.document.diagrams.contains_key(&id) {
                    return Err(error("not_found", "Diagram not found"));
                }
                let (scene, center) = dfd_render::scene(&s.document, id, dark && !light, None);
                json!({"text":crate::render::scene_svg(scene, center)})
            }
            Command::ExportPdf {
                all,
                number_pages,
                light,
            } => {
                let ids = if all {
                    s.document.order()
                } else {
                    vec![diagram]
                };
                let pages = ids.into_iter().enumerate().map(|(index, id)| {
                    let (scene, center) = dfd_render::scene(
                        &s.document,
                        id,
                        dark && !light,
                        number_pages.then_some(index + 1),
                    );
                    let size = scene.size;
                    (crate::render::scene_svg(scene, center), size)
                });
                json!({"bytes":crate::render::pdf_pages(pages)?, "mime":"application/pdf"})
            }
            Command::Navigate { diagram } => {
                if !s.document.diagrams.contains_key(&diagram) {
                    return Err(error("not_found", "Diagram not found"));
                }
                self.view.diagram = diagram;
                self.view.selection.clear();
                self.view.center = Default::default();
                json!({})
            }
            Command::NavigateUp {} => {
                if let Some(parent) = s.document.parent(diagram) {
                    self.view.diagram = parent;
                    self.view.selection.clear();
                    self.view.center = Default::default();
                }
                json!({})
            }
            Command::SelectionSet { ids } => {
                if ids.iter().any(|id| s.document.owner(*id) != Some(diagram)) {
                    return Err(error("not_found", "DFD object not in this diagram"));
                }
                self.view.selection = ids.into_iter().collect();
                json!({})
            }
            Command::Undo {} | Command::Redo {} => {
                let target = if matches!(command, Command::Undo {}) {
                    s.cursor.checked_sub(1)
                } else {
                    (s.cursor + 1 < s.history.len()).then_some(s.cursor + 1)
                };
                if let Some(target) = target {
                    s.cursor = target;
                    s.document = s.history[target].clone();
                    self.revision += 1;
                }
                json!({})
            }
            command => {
                let mut draft = s.document.clone();
                let mut next = s.next;
                let result = if let Command::Transaction { commands } = command {
                    if commands.len() > 1000
                        || commands.iter().any(|c| {
                            !c.edits_document() || matches!(c, Command::Transaction { .. })
                        })
                    {
                        return Err(error(
                            "invalid_arguments",
                            "Transactions accept up to 1000 non-nested document edits",
                        ));
                    }
                    let results = commands
                        .into_iter()
                        .map(|c| Self::dfd_edit(&mut draft, diagram, &mut next, c))
                        .collect::<Result<Vec<_>>>()?;
                    json!({"results":results})
                } else {
                    Self::dfd_edit(&mut draft, diagram, &mut next, command)?
                };
                draft.check()?;
                if draft != s.document {
                    s.next = next;
                    s.document = draft;
                    s.history.truncate(s.cursor + 1);
                    s.history.push(s.document.clone());
                    s.cursor += 1;
                    self.revision += 1;
                }
                result
            }
        };
        if !s.document.diagrams.contains_key(&self.view.diagram) {
            self.view.diagram = s.document.root;
            self.view.center = Default::default();
        }
        self.view
            .selection
            .retain(|id| s.document.owner(*id) == Some(self.view.diagram));
        Ok(self.dfd_response(result))
    }

    fn dfd_edit(
        doc: &mut dfd::Document,
        diagram: Id,
        next: &mut Id,
        command: Command,
    ) -> Result<Value> {
        match command {
            Command::DfdEdit { edit } => doc.edit(edit, diagram, next),
            Command::ObjectsDelete { ids } => {
                doc.delete(ids)?;
                Ok(json!({}))
            }
            Command::ObjectsMove { positions } => {
                for (id, position) in positions {
                    let node = doc
                        .owner(id)
                        .and_then(|d| doc.diagrams.get_mut(&d))
                        .and_then(|d| d.nodes.get_mut(&id))
                        .ok_or_else(|| error("not_found", "DFD node not found"))?;
                    node.position = position;
                }
                Ok(json!({}))
            }
            Command::DocumentMetadata { patch } => {
                let title = patch
                    .as_object()
                    .filter(|o| o.len() == 1)
                    .and_then(|o| o.get("title"))
                    .and_then(Value::as_str)
                    .ok_or_else(|| error("invalid_arguments", "Provide only the DFD title"))?;
                doc.title = title.to_owned();
                Ok(json!({}))
            }
            _ => Err(Self::wrong_notation()),
        }
    }

    fn check_discard(&self, discard: bool) -> Result<()> {
        if self.dirty() && !discard {
            return Err(error(
                "unsaved_changes",
                "Save or discard the current document first",
            ));
        }
        Ok(())
    }

    fn replace_with_idef0(&mut self, mut engine: Self) {
        engine.view.theme = self.view.theme;
        engine.view.resolved_dark = self.view.resolved_dark;
        engine.view.language = self.view.language.clone();
        engine.view.grid = self.view.grid;
        engine.revision = self.revision + 1;
        engine.document_generation = self.document_generation + 1;
        *self = engine;
    }

    fn dfd_response(&self, result: Value) -> Response {
        Response {
            revision: self.revision,
            dirty: self.dirty(),
            result,
        }
    }
}
