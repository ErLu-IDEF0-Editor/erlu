use crate::{Id, Point, Result, error, validation::Diagnostic};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const FORMAT: &str = "erlu-dfd";

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    #[default]
    GaneSarson,
    YourdonDemarco,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Process,
    External,
    Store,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum End {
    Node { id: Id },
    Boundary { parent_flow: Id },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub kind: Kind,
    pub title: String,
    pub position: Point,
    pub child: Option<Id>,
    pub data: Option<Id>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Flow {
    pub from: End,
    pub to: End,
    pub label: String,
    pub data: Option<Id>,
    #[serde(default)]
    pub bends: Vec<Point>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub name: String,
    pub description: String,
    pub components: Vec<Id>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Diagram {
    pub parent: Option<Id>,
    pub nodes: BTreeMap<Id, Node>,
    pub flows: BTreeMap<Id, Flow>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub format: String,
    pub format_version: crate::FormatVersion,
    pub title: String,
    pub style: Style,
    pub root: Id,
    pub id_sequence: Id,
    pub diagrams: BTreeMap<Id, Diagram>,
    pub dictionary: BTreeMap<Id, Definition>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum Edit {
    Style {
        style: Style,
    },
    NodeCreate {
        diagram: Option<Id>,
        kind: Kind,
        title: String,
        position: Point,
    },
    NodeUpdate {
        id: Id,
        title: String,
        data: Option<Id>,
    },
    FlowCreate {
        diagram: Option<Id>,
        flow: Flow,
    },
    FlowUpdate {
        id: Id,
        flow: Flow,
    },
    Decompose {
        id: Id,
    },
    DefinitionPut {
        id: Option<Id>,
        definition: Definition,
    },
    DefinitionDelete {
        id: Id,
    },
}

impl Document {
    pub fn new(title: String) -> Self {
        Self {
            format: FORMAT.into(),
            format_version: crate::FormatVersion {
                major: 1,
                minor: 0,
                extra: Default::default(),
            },
            style: Style::default(),
            root: 1,
            id_sequence: 3,
            diagrams: BTreeMap::from([(
                1,
                Diagram {
                    nodes: BTreeMap::from([(
                        2,
                        Node {
                            kind: Kind::Process,
                            title: title.clone(),
                            position: Point::default(),
                            child: None,
                            data: None,
                        },
                    )]),
                    ..Default::default()
                },
            )]),
            title,
            dictionary: BTreeMap::new(),
        }
    }

    pub fn read(text: &str) -> Result<Self> {
        let doc: Self =
            serde_json::from_str(text).map_err(|e| error("invalid_document", e.to_string()))?;
        doc.check()?;
        Ok(doc)
    }

    pub fn owner(&self, object: Id) -> Option<Id> {
        self.diagrams.iter().find_map(|(&id, d)| {
            (d.nodes.contains_key(&object) || d.flows.contains_key(&object)).then_some(id)
        })
    }

    pub fn parent(&self, diagram: Id) -> Option<Id> {
        self.owner(self.diagrams.get(&diagram)?.parent?)
    }

    pub fn interfaces(&self, diagram: Id) -> Vec<(Id, bool, &Flow)> {
        let Some(parent) = self.diagrams.get(&diagram).and_then(|d| d.parent) else {
            return vec![];
        };
        let Some(owner) = self.owner(parent) else {
            return vec![];
        };
        self.diagrams[&owner]
            .flows
            .iter()
            .filter_map(|(&id, f)| {
                let process = End::Node { id: parent };
                if f.to == process {
                    Some((id, true, f))
                } else if f.from == process {
                    Some((id, false, f))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn numbers(&self, diagram: Id) -> BTreeMap<Id, String> {
        let mut parents = Vec::new();
        let mut current = diagram;
        while let Some(owner) = self.parent(current) {
            parents.push((owner, self.diagrams[&current].parent.unwrap()));
            current = owner;
        }
        let mut prefix = String::new();
        for (owner, parent) in parents.into_iter().rev() {
            if owner != self.root {
                let number = self.diagrams[&owner]
                    .nodes
                    .iter()
                    .filter(|(_, node)| node.kind == Kind::Process)
                    .position(|(&id, _)| id == parent)
                    .unwrap_or_default()
                    + 1;
                prefix.push_str(&format!("{number}."));
            }
        }
        self.diagrams[&diagram]
            .nodes
            .iter()
            .filter(|(_, node)| node.kind == Kind::Process)
            .enumerate()
            .map(|(i, (&id, _))| {
                let number = if diagram == self.root { 0 } else { i + 1 };
                (id, format!("{prefix}{number}"))
            })
            .collect()
    }

    pub fn diagram_title(&self, id: Id) -> &str {
        self.diagrams[&id]
            .parent
            .and_then(|p| {
                self.owner(p)
                    .map(|d| self.diagrams[&d].nodes[&p].title.as_str())
            })
            .unwrap_or(&self.title)
    }

    pub fn order(&self) -> Vec<Id> {
        let mut order = Vec::new();
        let mut pending = vec![self.root];
        while let Some(id) = pending.pop() {
            order.push(id);
            pending.extend(
                self.diagrams[&id]
                    .nodes
                    .values()
                    .rev()
                    .filter_map(|node| node.child),
            );
        }
        order
    }

    pub fn tree(&self, id: Id) -> serde_json::Value {
        serde_json::json!({
            "id": id, "title": self.diagram_title(id),
            "children": self.diagrams[&id].nodes.values().filter_map(|n| n.child)
                .map(|child| self.tree(child)).collect::<Vec<_>>()
        })
    }

    fn fail(message: &str) -> crate::Error {
        error("invalid_dfd", message)
    }

    pub fn check(&self) -> Result<()> {
        if self.format != FORMAT || self.format_version.major != 1 || self.format_version.minor != 0
        {
            return Err(Self::fail("Unsupported DFD format version"));
        }
        let root = self
            .diagrams
            .get(&self.root)
            .ok_or_else(|| Self::fail("Missing DFD context"))?;
        if root.parent.is_some()
            || root
                .nodes
                .values()
                .filter(|n| n.kind == Kind::Process)
                .count()
                != 1
            || root.nodes.values().any(|n| n.kind == Kind::Store)
        {
            return Err(Self::fail(
                "DFD context requires one process and no data stores",
            ));
        }
        let mut ids = BTreeSet::new();
        let mut add = |id| id > 0 && id < self.id_sequence && ids.insert(id);
        for (&id, d) in &self.diagrams {
            if !add(id) {
                return Err(Self::fail("Invalid DFD diagram identifier"));
            }
            for (&object, n) in &d.nodes {
                if !add(object)
                    || !finite(n.position)
                    || !self.valid_data(n.data)
                    || (n.data.is_some() && n.kind != Kind::Store)
                    || (id != self.root && n.kind == Kind::External)
                {
                    return Err(Self::fail("Invalid DFD node"));
                }
                if let Some(child) = n.child
                    && (n.kind != Kind::Process
                        || self.diagrams.get(&child).and_then(|x| x.parent) != Some(object))
                {
                    return Err(Self::fail("Invalid DFD decomposition link"));
                }
            }
            for (&object, f) in &d.flows {
                if !add(object)
                    || f.bends.iter().any(|p| !finite(*p))
                    || !self.valid_data(f.data)
                    || !self.valid_flow(id, f)
                {
                    return Err(Self::fail("Invalid DFD flow or endpoints"));
                }
            }
        }
        let mut visited = BTreeSet::new();
        let mut pending = vec![self.root];
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                return Err(Self::fail("Cyclic DFD decomposition"));
            }
            pending.extend(self.diagrams[&id].nodes.values().filter_map(|n| n.child));
        }
        if visited.len() != self.diagrams.len() {
            return Err(Self::fail("Disconnected DFD decomposition"));
        }

        let mut names = BTreeSet::new();
        let mut remaining = BTreeSet::new();
        for (&id, definition) in &self.dictionary {
            let components: BTreeSet<_> = definition.components.iter().copied().collect();
            if !add(id)
                || definition.name.trim().is_empty()
                || !names.insert(normalize(&definition.name))
                || components.len() != definition.components.len()
                || !components.iter().all(|c| self.dictionary.contains_key(c))
            {
                return Err(Self::fail("Invalid or duplicate DFD dictionary definition"));
            }
            remaining.insert(id);
        }
        self.resolve_definitions(remaining)?;
        Ok(())
    }

    fn valid_data(&self, id: Option<Id>) -> bool {
        id.is_none_or(|id| self.dictionary.contains_key(&id))
    }

    fn valid_flow(&self, diagram: Id, flow: &Flow) -> bool {
        let d = &self.diagrams[&diagram];
        let node = |id| d.nodes.get(&id).map(|n| n.kind);
        let boundary = |flow_id, incoming| {
            self.interfaces(diagram)
                .iter()
                .any(|&(id, direction, _)| id == flow_id && direction == incoming)
        };
        match (flow.from, flow.to) {
            (End::Node { id: from }, End::Node { id: to }) => {
                from != to
                    && node(from).is_some()
                    && node(to).is_some()
                    && (node(from) == Some(Kind::Process) || node(to) == Some(Kind::Process))
            }
            (End::Boundary { parent_flow }, End::Node { id }) => {
                node(id) == Some(Kind::Process) && boundary(parent_flow, true)
            }
            (End::Node { id }, End::Boundary { parent_flow }) => {
                node(id) == Some(Kind::Process) && boundary(parent_flow, false)
            }
            _ => false,
        }
    }

    fn resolve_definitions(
        &self,
        mut remaining: BTreeSet<Id>,
    ) -> Result<BTreeMap<Id, BTreeSet<Id>>> {
        let mut atoms = BTreeMap::<Id, BTreeSet<Id>>::new();
        while !remaining.is_empty() {
            let ready: Vec<_> = remaining
                .iter()
                .copied()
                .filter(|id| {
                    self.dictionary[id]
                        .components
                        .iter()
                        .all(|c| atoms.contains_key(c))
                })
                .collect();
            if ready.is_empty() {
                return Err(Self::fail("Cyclic DFD dictionary definitions"));
            }
            for id in ready {
                let components = &self.dictionary[&id].components;
                let set = if components.is_empty() {
                    BTreeSet::from([id])
                } else {
                    components
                        .iter()
                        .flat_map(|c| atoms[c].iter().copied())
                        .collect()
                };
                atoms.insert(id, set);
                remaining.remove(&id);
            }
        }
        Ok(atoms)
    }

    pub fn effective_data(&self, diagram: Id, flow: &Flow) -> Option<Id> {
        let mut active = flow;
        let mut level = diagram;
        loop {
            if active.data.is_some() {
                return active.data;
            }
            let ((End::Boundary { parent_flow }, _) | (_, End::Boundary { parent_flow })) =
                (active.from, active.to)
            else {
                return None;
            };
            level = self.parent(level)?;
            active = self.diagrams[&level].flows.get(&parent_flow)?;
        }
    }

    pub fn validate(&self) -> Vec<Diagnostic> {
        let atoms = self
            .resolve_definitions(self.dictionary.keys().copied().collect())
            .unwrap_or_default();
        let mut diagnostics = Vec::new();
        for (&id, d) in &self.diagrams {
            let mut report =
                |severity: &str, code: &str, object, message: &str, reference: Option<&str>| {
                    diagnostics.push(Diagnostic {
                        severity: severity.into(),
                        code: code.into(),
                        diagram: id,
                        object,
                        message: message.into(),
                        source: if reference.is_some() {
                            "Ed Yourdon".into()
                        } else {
                            "erlu".into()
                        },
                        reference: reference.map(str::to_owned),
                    });
                };
            if d.nodes.is_empty() {
                report(
                    "warning",
                    "dfd_empty_diagram",
                    None,
                    "Add a subprocess to the decomposition.",
                    None,
                );
            }
            let mut labels = BTreeSet::new();
            for (&node, n) in &d.nodes {
                let end = End::Node { id: node };
                let input = d.flows.values().any(|f| f.to == end);
                let output = d.flows.values().any(|f| f.from == end);
                if n.title.trim().is_empty() {
                    report(
                        "warning",
                        "dfd_empty_name",
                        Some(node),
                        "Give this object a descriptive name.",
                        Some("§9.1"),
                    );
                } else if !labels.insert(normalize(&n.title)) {
                    report(
                        "warning",
                        "dfd_repeated_name",
                        Some(node),
                        "Distinguish objects with the same name.",
                        None,
                    );
                }
                if n.kind == Kind::Process {
                    if !input {
                        report(
                            "error",
                            "dfd_missing_input",
                            Some(node),
                            "Connect an input flow that supplies data to this process.",
                            Some("§9.2"),
                        );
                    }
                    if !output {
                        report(
                            "error",
                            "dfd_missing_output",
                            Some(node),
                            "Connect an output flow for the data this process produces.",
                            Some("§9.2"),
                        );
                    }
                } else if !input && !output {
                    report(
                        "warning",
                        "dfd_isolated_node",
                        Some(node),
                        "Connect this object to a process or remove it.",
                        Some("§9.2"),
                    );
                }
            }
            for (&flow, f) in &d.flows {
                if f.label.trim().is_empty()
                    && !matches!(
                        (f.from, f.to),
                        (End::Boundary { .. }, _) | (_, End::Boundary { .. })
                    )
                {
                    report(
                        "warning",
                        "dfd_empty_flow_name",
                        Some(flow),
                        "Name the data carried by this flow.",
                        Some("§9.1.2"),
                    );
                }
                if self.effective_data(id, f).is_none() {
                    report(
                        "warning",
                        "dfd_undocumented_flow",
                        Some(flow),
                        "Link the flow to a data dictionary definition.",
                        Some("Chapter 10"),
                    );
                }
            }
            for (parent_flow, incoming, parent) in self.interfaces(id) {
                let boundary = End::Boundary { parent_flow };
                let flows: Vec<_> = d
                    .flows
                    .iter()
                    .filter(|(_, f)| {
                        if incoming {
                            f.from == boundary
                        } else {
                            f.to == boundary
                        }
                    })
                    .collect();
                if flows.is_empty() {
                    report(
                        "error",
                        "dfd_unconnected_boundary",
                        None,
                        "Connect the inherited boundary flow to a subprocess.",
                        Some("§9.3"),
                    );
                    continue;
                }
                let parent_data = self
                    .parent(id)
                    .and_then(|level| self.effective_data(level, parent));
                let Some(expected) = parent_data.and_then(|x| atoms.get(&x)) else {
                    if flows.iter().any(|(_, f)| f.data.is_some()) {
                        report(
                            "warning",
                            "dfd_unknown_balance",
                            None,
                            "Define the parent flow in the dictionary to verify its components.",
                            Some("Chapter 10"),
                        );
                    }
                    continue;
                };
                let mut covered = BTreeSet::new();
                for (&flow_id, f) in flows {
                    let actual = f.data.and_then(|x| atoms.get(&x)).unwrap_or(expected);
                    if !actual.is_subset(expected) {
                        report(
                            "error",
                            "dfd_extra_component",
                            Some(flow_id),
                            "Use only dictionary components present in the parent flow.",
                            Some("§9.3"),
                        );
                    }
                    covered.extend(actual.iter().copied());
                }
                if !expected.is_subset(&covered) {
                    report(
                        "error",
                        "dfd_missing_component",
                        None,
                        "Connect the remaining components of the parent flow.",
                        Some("§9.3"),
                    );
                }
            }
        }
        diagnostics
    }

    pub fn edit(&mut self, edit: Edit, active: Id, next: &mut Id) -> Result<serde_json::Value> {
        use serde_json::json;
        let allocate = |next: &mut Id| -> Result<Id> {
            let id = *next;
            *next = id
                .checked_add(1)
                .ok_or_else(|| Self::fail("DFD identifiers exhausted"))?;
            Ok(id)
        };
        let missing = || error("not_found", "DFD object not found");
        let result = match edit {
            Edit::Style { style } => {
                self.style = style;
                json!({})
            }
            Edit::NodeCreate {
                diagram,
                kind,
                title,
                position,
            } => {
                let d = self
                    .diagrams
                    .get_mut(&diagram.unwrap_or(active))
                    .ok_or_else(missing)?;
                let id = allocate(next)?;
                d.nodes.insert(
                    id,
                    Node {
                        kind,
                        title,
                        position,
                        child: None,
                        data: None,
                    },
                );
                json!({"id":id})
            }
            Edit::NodeUpdate { id, title, data } => {
                let d = self.owner(id).ok_or_else(missing)?;
                let node = self
                    .diagrams
                    .get_mut(&d)
                    .unwrap()
                    .nodes
                    .get_mut(&id)
                    .ok_or_else(missing)?;
                node.title = title;
                node.data = data;
                json!({})
            }
            Edit::FlowCreate { diagram, flow } => {
                let d = self
                    .diagrams
                    .get_mut(&diagram.unwrap_or(active))
                    .ok_or_else(missing)?;
                let id = allocate(next)?;
                d.flows.insert(id, flow);
                json!({"id":id})
            }
            Edit::FlowUpdate { id, flow } => {
                let d = self.owner(id).ok_or_else(missing)?;
                *self
                    .diagrams
                    .get_mut(&d)
                    .unwrap()
                    .flows
                    .get_mut(&id)
                    .ok_or_else(missing)? = flow;
                if !self.valid_flow(d, &self.diagrams[&d].flows[&id]) {
                    return Err(Self::fail("Invalid DFD flow or endpoints"));
                }
                self.remove_invalid_boundaries();
                json!({})
            }
            Edit::Decompose { id } => {
                let d = self.owner(id).ok_or_else(missing)?;
                let n = self
                    .diagrams
                    .get_mut(&d)
                    .unwrap()
                    .nodes
                    .get_mut(&id)
                    .ok_or_else(missing)?;
                if n.kind != Kind::Process {
                    return Err(Self::fail("Only DFD processes can be decomposed"));
                }
                if let Some(child) = n.child {
                    return Ok(json!({"diagram":child}));
                }
                let child = allocate(next)?;
                n.child = Some(child);
                self.diagrams.insert(
                    child,
                    Diagram {
                        parent: Some(id),
                        ..Default::default()
                    },
                );
                json!({"diagram":child})
            }
            Edit::DefinitionPut { id, definition } => {
                let id = if let Some(id) = id {
                    if !self.dictionary.contains_key(&id) {
                        return Err(missing());
                    }
                    id
                } else {
                    allocate(next)?
                };
                self.dictionary.insert(id, definition);
                json!({"id":id})
            }
            Edit::DefinitionDelete { id } => {
                self.dictionary.remove(&id).ok_or_else(missing)?;
                json!({})
            }
        };
        self.id_sequence = *next;
        Ok(result)
    }

    pub fn delete(&mut self, objects: Vec<Id>) -> Result<()> {
        if objects.iter().any(|id| self.owner(*id).is_none()) {
            return Err(error("not_found", "DFD object not found"));
        }
        let mut diagrams = vec![];
        for id in objects {
            let Some(owner) = self.owner(id) else {
                continue;
            };
            let diagram = self.diagrams.get_mut(&owner).unwrap();
            if let Some(node) = diagram.nodes.remove(&id) {
                if let Some(child) = node.child {
                    diagrams.push(child);
                }
                let endpoint = End::Node { id };
                diagram
                    .flows
                    .retain(|_, f| f.from != endpoint && f.to != endpoint);
            } else {
                diagram.flows.remove(&id);
            }
        }
        while let Some(id) = diagrams.pop() {
            if let Some(d) = self.diagrams.remove(&id) {
                diagrams.extend(d.nodes.values().filter_map(|n| n.child));
            }
        }
        self.remove_invalid_boundaries();
        Ok(())
    }

    fn remove_invalid_boundaries(&mut self) {
        for id in self.order() {
            let available: BTreeSet<_> = self
                .interfaces(id)
                .iter()
                .map(|&(flow, incoming, _)| (flow, incoming))
                .collect();
            self.diagrams.get_mut(&id).unwrap().flows.retain(|_, f| {
                let valid = |end, incoming| match end {
                    End::Boundary { parent_flow } => available.contains(&(parent_flow, incoming)),
                    End::Node { .. } => true,
                };
                valid(f.from, true) && valid(f.to, false)
            });
        }
    }

    pub fn flow_label<'a>(&'a self, diagram: Id, flow: &'a Flow) -> &'a str {
        let mut current = flow;
        let mut level = diagram;
        while current.label.trim().is_empty() {
            if let Some(name) = current.data.and_then(|id| self.dictionary.get(&id)) {
                return &name.name;
            }
            let ((End::Boundary { parent_flow }, _) | (_, End::Boundary { parent_flow })) =
                (current.from, current.to)
            else {
                break;
            };
            let Some(owner) = self.parent(level) else {
                break;
            };
            let Some(parent) = self.diagrams[&owner].flows.get(&parent_flow) else {
                break;
            };
            current = parent;
            level = owner;
        }
        &current.label
    }
}

fn finite(p: Point) -> bool {
    p.x.is_finite() && p.y.is_finite()
}
fn normalize(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
