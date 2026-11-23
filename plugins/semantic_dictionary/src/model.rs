use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
pub const NS: &str = "semantic_dictionary";
pub type Extra = BTreeMap<String, Value>;
pub fn key(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
pub fn name(s: &str) -> Result<String, String> {
    let s = s.trim();
    if s.is_empty() || s.chars().count() > 200 || s.chars().any(char::is_control) {
        return Err(
            "Names and codes must contain 1–200 characters without control characters".into(),
        );
    }
    Ok(s.into())
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Concept {
    pub code: String,
    pub term: String,
    #[serde(default)]
    pub definition: String,
    #[serde(default)]
    pub aliases: BTreeSet<String>,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub source: String,
    #[serde(flatten)]
    pub extra: Extra,
}
impl Concept {
    pub fn spellings(&self) -> BTreeSet<String> {
        self.aliases
            .iter()
            .chain([&self.term])
            .map(|s| key(s))
            .collect()
    }
    pub fn catalog_record(&self) -> Value {
        json!({"code":self.code,"term":self.term,"definition":self.definition,
            "aliases":self.aliases,"owner":self.owner,"source":self.source})
    }
    pub fn validate(&self) -> Result<(), String> {
        name(&self.code)?;
        name(&self.term)?;
        for alias in &self.aliases {
            name(alias)?;
        }
        Ok(())
    }
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Data {
    pub next_id: u64,
    pub concepts: BTreeMap<String, Concept>,
    pub bindings: BTreeMap<String, String>,
    #[serde(flatten)]
    pub extra: Extra,
}
impl Default for Data {
    fn default() -> Self {
        Self {
            next_id: 1,
            concepts: BTreeMap::new(),
            bindings: BTreeMap::new(),
            extra: Extra::new(),
        }
    }
}
impl Data {
    pub fn read(doc: &Value) -> Result<Self, String> {
        let Some(ext) = doc["extensions"].get(NS) else {
            return Ok(Self::default());
        };
        if ext["version"].as_u64() != Some(1) {
            return Err(
                "Unsupported Semantic Dictionary data version; existing data was not changed"
                    .into(),
            );
        }
        let data: Self = serde_json::from_value(ext["data"].clone())
            .map_err(|e| format!("Invalid dictionary: {e}"))?;
        let mut codes = BTreeSet::new();
        for concept in data.concepts.values() {
            concept.validate()?;
            if !codes.insert(key(&concept.code)) {
                return Err("Duplicate concept codes in dictionary".into());
            }
        }
        Ok(data)
    }
    pub fn id(&self, code: &str) -> Result<String, String> {
        self.concepts
            .iter()
            .find(|(_, c)| key(&c.code) == key(code))
            .map(|(id, _)| id.clone())
            .ok_or_else(|| format!("Unknown concept code: {code}"))
    }
    pub fn insert(&mut self, concept: Concept) -> Result<String, String> {
        if self.concepts.len() >= 1000 {
            return Err("At most 1000 concepts are supported".into());
        }
        let id = loop {
            let id = format!("concept_{}", self.next_id);
            self.next_id = self
                .next_id
                .checked_add(1)
                .ok_or("Concept ID sequence exhausted")?;
            if !self.concepts.contains_key(&id) {
                break id;
            }
        };
        self.concepts.insert(id.clone(), concept);
        Ok(id)
    }
    pub fn command(&self) -> Value {
        json!({"action":"extension_set","args":{"namespace":NS,"version":1,"data":self}})
    }
}
#[derive(Clone)]
pub struct Arrow {
    pub id: u64,
    pub diagram: u64,
    pub node: String,
    pub label: String,
    pub source: Option<u64>,
    pub eligible: bool,
    pub icom: String,
}
pub struct Graph {
    pub arrows: BTreeMap<u64, Arrow>,
    pub roots: BTreeMap<u64, u64>,
    pub groups: BTreeMap<u64, Vec<u64>>,
}
impl Graph {
    pub fn read(doc: &Value) -> Result<Self, String> {
        let diagrams = doc["diagrams"].as_object().ok_or("Missing diagrams")?;
        let mut numbers = BTreeMap::new();
        for d in diagrams.values() {
            for (id, b) in d["blocks"].as_object().ok_or("Missing blocks")? {
                numbers.insert(
                    id.parse::<u64>().map_err(|_| "Invalid block ID")?,
                    b["number"].as_str().unwrap_or("").to_string(),
                );
            }
        }
        let mut arrows = BTreeMap::new();
        for (diagram, d) in diagrams {
            let diagram = diagram.parse::<u64>().map_err(|_| "Invalid diagram ID")?;
            let node = d["parent_block"]
                .as_u64()
                .and_then(|id| numbers.get(&id))
                .cloned()
                .unwrap_or_else(|| "A-0".into());
            for (id, a) in d["arrows"].as_object().ok_or("Missing arrows")? {
                let id = id.parse::<u64>().map_err(|_| "Invalid arrow ID")?;
                let ports = [&a["from"], &a["to"]]
                    .iter()
                    .filter_map(|e| e["anchor"]["port"].as_str())
                    .collect::<Vec<_>>()
                    .join(" → ");
                arrows.insert(
                    id,
                    Arrow {
                        id,
                        diagram,
                        node: node.clone(),
                        label: a["label"].as_str().unwrap_or("").into(),
                        source: a["interface"]["parent_arrow"]
                            .as_u64()
                            .or_else(|| a["label_source"].as_u64()),
                        eligible: a["call_ref"].is_null() && a["interface"]["stub"] != true,
                        icom: ports,
                    },
                );
            }
        }
        let mut roots = BTreeMap::new();
        let mut groups: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
        for arrow in arrows.values().filter(|a| a.eligible) {
            let mut id = arrow.id;
            let mut seen = BTreeSet::new();
            loop {
                if !seen.insert(id) {
                    return Err("Cyclic label inheritance".into());
                }
                let a = arrows.get(&id).ok_or("Missing label source")?;
                if let Some(source) = a.source {
                    id = source;
                } else {
                    break;
                }
            }
            if !arrows[&id].eligible {
                return Err("Unsupported label source".into());
            }
            roots.insert(arrow.id, id);
            groups.entry(id).or_default().push(arrow.id);
        }
        Ok(Self {
            arrows,
            roots,
            groups,
        })
    }
    pub fn scoped(&self, ctx: &Value, scope: &str) -> Result<Vec<u64>, String> {
        let selected: BTreeSet<u64> = ctx["view"]["selection"]
            .as_array()
            .map(|v| v.iter().filter_map(Value::as_u64).collect())
            .unwrap_or_default();
        let ids: Vec<_> = self
            .arrows
            .values()
            .filter(|a| {
                a.eligible
                    && match scope {
                        "document" => true,
                        "diagram" => Some(a.diagram) == ctx["view"]["diagram"].as_u64(),
                        "selection" => selected.contains(&a.id),
                        _ => false,
                    }
            })
            .map(|a| a.id)
            .collect();
        if !["document", "diagram", "selection"].contains(&scope) {
            return Err("Invalid scope".into());
        }
        if scope == "selection" && ids.is_empty() {
            return Err("Select at least one ordinary arrow; call arrows and interface placeholders are excluded".into());
        }
        Ok(ids)
    }
    pub fn concepts(&self, root: u64, data: &Data) -> BTreeSet<String> {
        self.groups[&root]
            .iter()
            .filter_map(|id| data.bindings.get(&id.to_string()).cloned())
            .collect()
    }
}
