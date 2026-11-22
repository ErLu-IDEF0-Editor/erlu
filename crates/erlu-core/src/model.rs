use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub type Id = u64;
pub type Extra = BTreeMap<String, Value>;
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn distance(self, p: Self) -> f64 {
        (self.x - p.x).hypot(self.y - p.y)
    }
    pub fn finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}
impl std::ops::Add for Point {
    type Output = Self;
    fn add(self, p: Self) -> Self {
        Self::new(self.x + p.x, self.y + p.y)
    }
}
impl std::ops::Sub for Point {
    type Output = Self;
    fn sub(self, p: Self) -> Self {
        Self::new(self.x - p.x, self.y - p.y)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Port {
    Input,
    Control,
    Output,
    Mechanism,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Anchor {
    Block {
        id: Id,
        port: Port,
        #[serde(default = "half")]
        t: f64,
    },
    Junction {
        id: Id,
    },
    Boundary {
        port: Port,
        t: f64,
    },
}
fn half() -> f64 {
    0.5
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Endpoint {
    pub anchor: Anchor,
    #[serde(default)]
    pub tunneled: bool,
}
impl Endpoint {
    pub fn new(anchor: Anchor) -> Self {
        Self {
            anchor,
            tunneled: false,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Block {
    pub id: Id,
    pub title: String,
    pub number: String,
    pub pos: Point,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub child: Option<Id>,
    #[serde(default, flatten)]
    pub extra: Extra,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Junction {
    pub id: Id,
    pub pos: Point,
    #[serde(default, flatten)]
    pub extra: Extra,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Interface {
    pub parent_arrow: Id,
    pub port: Port,
    pub t: f64,
    pub stub: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Arrow {
    pub id: Id,
    pub from: Endpoint,
    pub to: Endpoint,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub label_offset: Point,
    #[serde(default)]
    pub label_hidden: bool,
    #[serde(default)]
    pub label_source: Option<Id>,
    #[serde(default)]
    pub bends: Vec<Point>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub interface: Option<Interface>,
    #[serde(default)]
    pub call_ref: Option<Id>,
    #[serde(default, flatten)]
    pub extra: Extra,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiagramState {
    #[default]
    Working,
    Draft,
    Recommended,
    Publication,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Diagram {
    pub id: Id,
    pub parent_block: Option<Id>,
    pub notation: String,
    #[serde(default)]
    pub state: DiagramState,
    #[serde(default)]
    pub blocks: BTreeMap<Id, Block>,
    #[serde(default)]
    pub junctions: BTreeMap<Id, Junction>,
    #[serde(default)]
    pub arrows: BTreeMap<Id, Arrow>,
    #[serde(default, flatten)]
    pub extra: Extra,
}
impl Diagram {
    pub fn empty(id: Id, parent: Option<Id>) -> Self {
        Self {
            id,
            parent_block: parent,
            notation: "idef0".into(),
            state: DiagramState::Working,
            blocks: BTreeMap::new(),
            junctions: BTreeMap::new(),
            arrows: BTreeMap::new(),
            extra: Extra::new(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Metadata {
    pub title: String,
    pub organization: String,
    pub author: String,
    pub initials: String,
    pub currency: String,
    pub currency_placement: String,
    pub time_unit: String,
    pub page_size: String,
    pub portrait: bool,
    pub with_header: bool,
    pub with_footer: bool,
    pub used_at: String,
    pub project: String,
    pub notes: String,
    pub date: String,
    pub revision: String,
    pub reader: String,
    pub reader_date: String,
    pub context: String,
    pub number: String,
    pub footer_node_override: String,
    pub footer_title_override: String,
    pub footer_number_override: String,
    #[serde(flatten)]
    pub extra: Extra,
}
impl Default for Metadata {
    fn default() -> Self {
        Self {
            title: String::new(),
            organization: String::new(),
            author: String::new(),
            initials: String::new(),
            currency: "$".into(),
            currency_placement: "1?".into(),
            time_unit: "days".into(),
            page_size: "A4".into(),
            portrait: false,
            with_header: true,
            with_footer: true,
            used_at: String::new(),
            project: String::new(),
            notes: String::new(),
            date: String::new(),
            revision: String::new(),
            reader: String::new(),
            reader_date: String::new(),
            context: String::new(),
            number: String::new(),
            footer_node_override: String::new(),
            footer_title_override: String::new(),
            footer_number_override: String::new(),
            extra: Extra::new(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FormatVersion {
    pub major: u32,
    pub minor: u32,
    #[serde(default, flatten)]
    pub extra: Extra,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Features {
    pub required: BTreeSet<String>,
    #[serde(default)]
    pub optional: BTreeSet<String>,
    #[serde(default, flatten)]
    pub extra: Extra,
}
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema)]
pub struct Document {
    #[serde(skip)]
    pub(crate) preserved: Value,
    pub format: String,
    pub format_version: FormatVersion,
    pub features: Features,
    pub root: Id,
    pub id_sequence: Id,
    pub meta: Metadata,
    pub diagrams: BTreeMap<Id, Diagram>,
    #[serde(default)]
    pub extensions: Extra,
    #[serde(default, flatten)]
    pub extra: Extra,
}
pub const FEATURES: &[&str] = &[
    "idef0",
    "idef0.decomposition",
    "idef0.tunnels",
    "idef0.calls",
    "abc.costs",
    "appearance.colors",
];
impl Default for Document {
    fn default() -> Self {
        Self {
            preserved: Value::Null,
            format: "erlu".into(),
            format_version: FormatVersion {
                major: crate::format::CURRENT_MAJOR,
                minor: crate::format::CURRENT_MINOR,
                extra: Extra::new(),
            },
            features: Features {
                required: FEATURES.iter().map(|s| s.to_string()).collect(),
                optional: BTreeSet::new(),
                extra: Extra::new(),
            },
            root: 1,
            id_sequence: 3,
            meta: Metadata::default(),
            diagrams: BTreeMap::from([(1, {
                let mut d = Diagram::empty(1, None);
                d.blocks.insert(
                    2,
                    Block {
                        id: 2,
                        title: "Function".into(),
                        number: "A0".into(),
                        pos: Point::new(-26.5, -9.05),
                        price: None,
                        color: None,
                        child: None,
                        extra: Extra::new(),
                    },
                );
                d
            })]),
            extensions: Extra::new(),
            extra: Extra::new(),
        }
    }
}
impl Document {
    pub fn with_meta(meta: Metadata) -> Self {
        let mut doc = Self {
            meta,
            ..Self::default()
        };
        let (min, max) = doc.content();
        doc.diagrams
            .get_mut(&doc.root)
            .unwrap()
            .blocks
            .get_mut(&2)
            .unwrap()
            .pos = Point::new((min.x + max.x - 53.) / 2., (min.y + max.y - 26.5) / 2.);
        doc
    }
    pub fn next_id(&self) -> Id {
        self.id_sequence
    }
    pub fn block(&self, id: Id) -> Option<&Block> {
        self.diagrams.values().find_map(|d| d.blocks.get(&id))
    }
    pub fn diagram_of(&self, id: Id) -> Option<Id> {
        self.diagrams
            .values()
            .find(|d| {
                d.blocks.contains_key(&id)
                    || d.arrows.contains_key(&id)
                    || d.junctions.contains_key(&id)
            })
            .map(|d| d.id)
    }
    pub fn page(&self) -> Point {
        let (w, h) = match self.meta.page_size.as_str() {
            "A1" => (594., 841.),
            "A2" => (420., 594.),
            "A3" => (297., 420.),
            _ => (210., 297.),
        };
        if self.meta.portrait {
            Point::new(w, h)
        } else {
            Point::new(h, w)
        }
    }
    pub fn content(&self) -> (Point, Point) {
        let s = self.page();
        (
            Point::new(
                -s.x / 2.,
                -s.y / 2.
                    + if self.meta.with_header {
                        s.y * 0.12
                    } else {
                        0.
                    },
            ),
            Point::new(
                s.x / 2.,
                s.y / 2.
                    - if self.meta.with_footer {
                        s.y * 0.08
                    } else {
                        0.
                    },
            ),
        )
    }
    pub fn node(&self, diagram: Id) -> String {
        self.diagrams
            .get(&diagram)
            .and_then(|d| d.parent_block)
            .and_then(|id| self.block(id))
            .map(|b| b.number.clone())
            .unwrap_or("TOP".into())
    }
    pub fn title(&self, diagram: Id) -> String {
        self.diagrams
            .get(&diagram)
            .and_then(|d| d.parent_block)
            .and_then(|id| self.block(id))
            .map(|b| b.title.clone())
            .unwrap_or(self.meta.title.clone())
    }
    pub fn endpoint(&self, diagram: Id, ep: &Endpoint) -> Option<Point> {
        let d = self.diagrams.get(&diagram)?;
        let (min, max) = self.content();
        Some(match ep.anchor {
            Anchor::Block { id, port, t } => {
                let b = d.blocks.get(&id)?;
                match port {
                    Port::Input => b.pos + Point::new(0., 26.5 * t),
                    Port::Output => b.pos + Point::new(53., 26.5 * t),
                    Port::Control => b.pos + Point::new(53. * t, 0.),
                    Port::Mechanism => b.pos + Point::new(53. * t, 26.5),
                }
            }
            Anchor::Junction { id } => d.junctions.get(&id)?.pos,
            Anchor::Boundary { port, t } => match port {
                Port::Input => Point::new(min.x, min.y + (max.y - min.y) * t),
                Port::Output => Point::new(max.x, min.y + (max.y - min.y) * t),
                Port::Control => Point::new(min.x + (max.x - min.x) * t, min.y),
                Port::Mechanism => Point::new(min.x + (max.x - min.x) * t, max.y),
            },
        })
    }
}

// A lossless overlay retains optional future fields even inside small value types
// such as Point and Endpoint. Entity maps never resurrect deleted entities.
impl Serialize for Document {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let mut value = serde_json::json!({"format":self.format,"format_version":self.format_version,"features":self.features,"root":self.root,"id_sequence":self.id_sequence,"meta":self.meta,"diagrams":self.diagrams,"extensions":self.extensions});
        for (key, v) in &self.extra {
            value[key] = v.clone();
        }
        merge_future(&mut value, &self.preserved, "");
        value.serialize(serializer)
    }
}
fn merge_future(current: &mut Value, old: &Value, field: &str) {
    if field == "extensions" {
        return;
    }
    if current.get("kind").is_some() && current.get("kind") != old.get("kind") {
        return;
    }
    match (current, old) {
        (Value::Object(now), Value::Object(prior)) => {
            for (key, value) in prior {
                if let Some(current) = now.get_mut(key) {
                    merge_future(current, value, key);
                } else if !["diagrams", "blocks", "arrows", "junctions"].contains(&field) {
                    now.insert(key.clone(), value.clone());
                }
            }
        }
        (Value::Array(now), Value::Array(prior)) => {
            for (current, old) in now.iter_mut().zip(prior) {
                merge_future(current, old, "");
            }
        }
        _ => {}
    }
}

pub(crate) fn unknown_fields(input: &Value, known: &Value) -> Value {
    match (input, known) {
        (Value::Object(original), Value::Object(parsed)) => {
            let mut out = serde_json::Map::new();
            for (key, value) in original {
                let unknown = if let Some(v) = parsed.get(key) {
                    unknown_fields(value, v)
                } else {
                    value.clone()
                };
                if !unknown.is_null() {
                    out.insert(key.clone(), unknown);
                }
            }
            if out.is_empty() {
                Value::Null
            } else {
                Value::Object(out)
            }
        }
        (Value::Array(original), Value::Array(parsed)) => {
            let out = original
                .iter()
                .zip(parsed)
                .map(|(a, b)| unknown_fields(a, b))
                .collect::<Vec<_>>();
            if out.iter().all(Value::is_null) {
                Value::Null
            } else {
                Value::Array(out)
            }
        }
        _ => Value::Null,
    }
}
