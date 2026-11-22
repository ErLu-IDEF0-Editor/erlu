use crate::model::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// This is the sole public action vocabulary for GUI, MCP, plugins and web.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "action", content = "args", rename_all = "snake_case")]
pub enum Command {
    DocumentNew {
        #[serde(default)]
        meta: Box<Metadata>,
        #[serde(default)]
        discard: bool,
    },
    DocumentOpen {
        json: String,
        #[serde(default)]
        discard: bool,
    },
    DocumentSerialize {},
    DocumentSaved {
        revision: u64,
    },
    DocumentMetadata {
        patch: Value,
    },
    DocumentState {},
    NodeTree {},
    ObjectGet {
        id: Id,
    },
    BlockCreate {
        #[serde(default)]
        diagram: Option<Id>,
        pos: Point,
        #[serde(default)]
        title: String,
    },
    BlockUpdate {
        id: Id,
        title: Option<String>,
        price: Option<f64>,
        #[serde(default)]
        clear_price: bool,
    },
    ObjectsMove {
        positions: std::collections::BTreeMap<Id, Point>,
    },
    ObjectsDelete {
        ids: Vec<Id>,
    },
    NumbersSwap {
        first: Id,
        second: Id,
    },
    ArrowCreate {
        #[serde(default)]
        diagram: Option<Id>,
        from: Endpoint,
        to: Endpoint,
        #[serde(default)]
        label: String,
    },
    ArrowUpdate {
        id: Id,
        from: Option<Endpoint>,
        to: Option<Endpoint>,
        label: Option<String>,
        label_offset: Option<Point>,
        label_hidden: Option<bool>,
        bends: Option<Vec<Point>>,
    },
    ArrowBranch {
        id: Id,
        point: Point,
        target: Option<Endpoint>,
    },
    InterfaceConnect {
        id: Id,
        endpoint: Endpoint,
    },
    InterfaceMove {
        id: Id,
        t: f64,
    },
    CallMechanism {
        block: Id,
        reference: Id,
        t: f64,
    },
    ColorsSet {
        ids: Vec<Id>,
        color: Option<String>,
    },
    DiagramState {
        #[serde(default)]
        diagram: Option<Id>,
        state: DiagramState,
    },
    Decompose {
        block: Id,
    },
    Navigate {
        diagram: Id,
    },
    NavigateUp {},
    SelectionSet {
        ids: Vec<Id>,
    },
    ViewSet {
        zoom: Option<f64>,
        center: Option<Point>,
        theme: Option<Theme>,
    },
    ViewFit {},
    LanguageSet {
        language: String,
    },
    Undo {},
    Redo {},
    Validate {},
    CalculatePrices {},
    ExportMarkdown {},
    ExportSvg {
        #[serde(default)]
        diagram: Option<Id>,
        #[serde(default)]
        light: bool,
    },
    ExportPdf {
        #[serde(default)]
        all: bool,
        #[serde(default)]
        number_pages: bool,
        #[serde(default)]
        light: bool,
    },
    ExtensionSet {
        namespace: String,
        version: u32,
        data: Value,
    },
    Transaction {
        commands: Vec<Command>,
    },
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Light,
    Dark,
    #[default]
    System,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionSpec {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub read_only: bool,
}
impl Command {
    pub fn name(&self) -> String {
        serde_json::to_value(self).unwrap()["action"]
            .as_str()
            .unwrap()
            .into()
    }
    pub fn edits_document(&self) -> bool {
        matches!(
            self,
            Self::DocumentMetadata { .. }
                | Self::BlockCreate { .. }
                | Self::BlockUpdate { .. }
                | Self::ObjectsMove { .. }
                | Self::ObjectsDelete { .. }
                | Self::NumbersSwap { .. }
                | Self::ArrowCreate { .. }
                | Self::ArrowUpdate { .. }
                | Self::ArrowBranch { .. }
                | Self::InterfaceConnect { .. }
                | Self::InterfaceMove { .. }
                | Self::CallMechanism { .. }
                | Self::ColorsSet { .. }
                | Self::DiagramState { .. }
                | Self::Decompose { .. }
                | Self::CalculatePrices { .. }
                | Self::ExtensionSet { .. }
                | Self::Transaction { .. }
        )
    }
}
pub fn action_specs() -> Vec<ActionSpec> {
    let schema = serde_json::to_value(schemars::schema_for!(Command)).unwrap();
    schema["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .map(|variant| {
            let name = variant["properties"]["action"]["const"]
                .as_str()
                .or_else(|| variant["properties"]["action"]["enum"][0].as_str())
                .unwrap()
                .to_string();
            let mut input = variant["properties"]["args"].clone();
            if input.is_null() {
                input = json!({"type":"object","properties":{}});
            }
            input["$defs"] = schema["$defs"].clone();
            let read_only = matches!(
                name.as_str(),
                "document_state"
                    | "document_serialize"
                    | "node_tree"
                    | "object_get"
                    | "validate"
                    | "export_markdown"
                    | "export_svg"
                    | "export_pdf"
            );
            ActionSpec {
                name: name.clone(),
                description: format!(
                    "erlu: {}. Coordinates are in millimetres; IDs are stable within the document.",
                    name.replace('_', " ")
                ),
                input_schema: input,
                read_only,
            }
        })
        .collect()
}
pub fn decode_action(name: &str, args: Value) -> Result<Command, serde_json::Error> {
    serde_json::from_value(json!({"action":name,"args":args}))
}

/// Host services share discovery schemas but are implemented by platform adapters.
pub fn host_action_specs() -> Vec<ActionSpec> {
    vec![
        ActionSpec {
            name: "plugin_list".into(),
            description: "List loaded plugin manifests and actions".into(),
            input_schema: json!({"type":"object","properties":{}}),
            read_only: true,
        },
        ActionSpec {
            name: "plugin_load".into(),
            description: "Load a plugin manifest (native path or same-origin web URL)".into(),
            input_schema: json!({"type":"object","properties":{"source":{"type":"string"}},"required":["source"]}),
            read_only: false,
        },
    ]
}
