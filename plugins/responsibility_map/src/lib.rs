//! RACI is this plugin's responsibility policy, not an IDEF0/ISO requirement.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(target_arch = "wasm32")]
wit_bindgen::generate!({world: "plugin", path: "wit"});
#[cfg(target_arch = "wasm32")]
struct ResponsibilityMap;
#[cfg(target_arch = "wasm32")]
impl Guest for ResponsibilityMap {
    fn run(action: String, context: String) -> Result<String, String> {
        run(&action, &context)
    }
}
#[cfg(target_arch = "wasm32")]
export!(ResponsibilityMap);

const NAMESPACE: &str = "responsibility_map";
const CATEGORIES: [&str; 4] = ["R", "A", "C", "I"];
type Extra = BTreeMap<String, Value>;
#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(default)]
struct Assignment {
    #[serde(rename = "R")]
    responsible: BTreeSet<String>,
    #[serde(rename = "A")]
    accountable: BTreeSet<String>,
    #[serde(rename = "C")]
    consulted: BTreeSet<String>,
    #[serde(rename = "I")]
    informed: BTreeSet<String>,
    #[serde(flatten)]
    extra: Extra,
}
impl Assignment {
    fn category(&self, category: &str) -> &BTreeSet<String> {
        match category {
            "R" => &self.responsible,
            "A" => &self.accountable,
            "C" => &self.consulted,
            "I" => &self.informed,
            _ => unreachable!(),
        }
    }
    fn category_mut(&mut self, category: &str) -> &mut BTreeSet<String> {
        match category {
            "R" => &mut self.responsible,
            "A" => &mut self.accountable,
            "C" => &mut self.consulted,
            "I" => &mut self.informed,
            _ => unreachable!(),
        }
    }
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(default)]
struct Data {
    next_role_id: u64,
    roles: BTreeMap<String, String>,
    assignments: BTreeMap<String, Assignment>,
    #[serde(flatten)]
    extra: Extra,
}
impl Default for Data {
    fn default() -> Self {
        Self {
            next_role_id: 1,
            roles: BTreeMap::new(),
            assignments: BTreeMap::new(),
            extra: Extra::new(),
        }
    }
}
#[derive(Clone)]
struct Block {
    id: u64,
    diagram: u64,
    node: String,
    title: String,
}
fn text_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args[key]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("{key} must be a nonempty string"))
}
fn name_arg(args: &Value, key: &str) -> Result<String, String> {
    let name = text_arg(args, key)?;
    if name.chars().count() > 160 || name.chars().any(char::is_control) {
        return Err(format!(
            "{key} must be at most 160 characters without control characters"
        ));
    }
    Ok(name.to_string())
}
fn role_id(data: &Data, name: &str) -> Result<String, String> {
    let key = name.to_lowercase();
    data.roles
        .iter()
        .find(|(_, label)| label.to_lowercase() == key)
        .map(|(id, _)| id.clone())
        .ok_or_else(|| format!("Role '{name}' is not registered; add the role first"))
}
fn collect_blocks(doc: &Value) -> Result<Vec<Block>, String> {
    let diagrams = doc["diagrams"]
        .as_object()
        .ok_or("Missing document diagrams")?;
    let mut blocks = Vec::new();
    for (key, diagram) in diagrams {
        let diagram_id = key.parse::<u64>().map_err(|_| "Invalid diagram ID")?;
        for (key, block) in diagram["blocks"]
            .as_object()
            .ok_or("Missing diagram blocks")?
        {
            blocks.push(Block {
                id: key.parse::<u64>().map_err(|_| "Invalid block ID")?,
                diagram: diagram_id,
                node: block["number"]
                    .as_str()
                    .ok_or("Missing block number")?
                    .into(),
                title: block["title"].as_str().ok_or("Missing block title")?.into(),
            });
        }
    }
    blocks.sort_by(|a, b| a.node.cmp(&b.node).then(a.id.cmp(&b.id)));
    Ok(blocks)
}
fn scoped_blocks<'a>(all: &'a [Block], ctx: &Value, scope: &str) -> Result<Vec<&'a Block>, String> {
    match scope {
        "document" => Ok(all.iter().collect()),
        "diagram" => {
            let diagram = ctx["view"]["diagram"]
                .as_u64()
                .ok_or("Missing active diagram")?;
            Ok(all.iter().filter(|b| b.diagram == diagram).collect())
        }
        "selection" => {
            let ids = ctx["view"]["selection"]
                .as_array()
                .ok_or("Missing selection")?;
            let selected: BTreeSet<u64> = ids
                .iter()
                .map(|id| id.as_u64().ok_or("Invalid selection ID"))
                .collect::<Result<_, _>>()?;
            let result: Vec<_> = all.iter().filter(|b| selected.contains(&b.id)).collect();
            if result.is_empty() {
                return Err("Select at least one function block".into());
            }
            Ok(result)
        }
        _ => Err("scope must be document, diagram or selection".into()),
    }
}
fn read_data(doc: &Value) -> Result<Data, String> {
    let Some(extension) = doc["extensions"].get(NAMESPACE) else {
        return Ok(Data::default());
    };
    if extension["version"].as_u64() != Some(1) {
        return Err(
            "Unsupported Responsibility Map data version; existing data was not changed".into(),
        );
    }
    let data: Data = serde_json::from_value(extension["data"].clone())
        .map_err(|e| format!("Invalid Responsibility Map data: {e}"))?;
    let mut names = BTreeSet::new();
    for (id, name) in &data.roles {
        if id.is_empty()
            || name.trim().is_empty()
            || name != name.trim()
            || name.chars().any(char::is_control)
            || !names.insert(name.to_lowercase())
        {
            return Err("Invalid or duplicate role names in Responsibility Map data".into());
        }
    }
    Ok(data)
}
fn output(data: Option<&Data>, result: Value) -> Result<String, String> {
    let commands = data
        .map(|d| {
            vec![json!({"action":"extension_set","args":{
                "namespace":NAMESPACE,"version":1,"data":d
            }})]
        })
        .unwrap_or_default();
    let result = json!({"commands":commands,"result":result}).to_string();
    if result.len() > 3 * 1024 * 1024 {
        return Err(
            "Responsibility Map response is too large; export a diagram or selection instead"
                .into(),
        );
    }
    Ok(result)
}
/// Same pure entry point is exercised by native unit tests and the WIT guest.
pub fn run(action: &str, context: &str) -> Result<String, String> {
    let ctx: Value = serde_json::from_str(context).map_err(|e| e.to_string())?;
    if ctx["api_version"] != 1 {
        return Err("Responsibility Map requires plugin API v1".into());
    }
    let doc = &ctx["document"];
    let args = &ctx["arguments"];
    if !args.is_object() {
        return Err("Arguments must be an object".into());
    }
    let all = collect_blocks(doc)?;
    let mut data = read_data(doc)?;
    match action {
        "role.add" => {
            let name = name_arg(args, "name")?;
            if role_id(&data, &name).is_ok() {
                return Err(format!("Role '{name}' already exists"));
            }
            if data.roles.len() >= 256 {
                return Err("At most 256 roles are supported".into());
            }
            let id = loop {
                let id = format!("role_{}", data.next_role_id);
                data.next_role_id = data
                    .next_role_id
                    .checked_add(1)
                    .ok_or("Role ID sequence exhausted")?;
                if !data.roles.contains_key(&id) {
                    break id;
                }
            };
            data.roles.insert(id.clone(), name.clone());
            output(
                Some(&data),
                json!({"role_id":id,"text":format!("Role added: {name}")}),
            )
        }
        "role.rename" => {
            let name = name_arg(args, "name")?;
            let next = name_arg(args, "new_name")?;
            let id = role_id(&data, &name)?;
            if role_id(&data, &next).is_ok_and(|other| other != id) {
                return Err(format!("Role '{next}' already exists"));
            }
            data.roles.insert(id, next.clone());
            output(
                Some(&data),
                json!({"text":format!("Role renamed: {name} → {next}")}),
            )
        }
        "role.remove" => {
            let name = name_arg(args, "name")?;
            let id = role_id(&data, &name)?;
            data.roles.remove(&id);
            for assignment in data.assignments.values_mut() {
                for category in CATEGORIES {
                    assignment.category_mut(category).remove(&id);
                }
            }
            output(
                Some(&data),
                json!({"text":format!("Role and its assignments removed: {name}")}),
            )
        }
        "assignment.set" | "assignment.remove" => {
            let role = name_arg(args, "role")?;
            let id = role_id(&data, &role)?;
            let category = text_arg(args, "responsibility")?;
            if !CATEGORIES.contains(&category) {
                return Err("responsibility must be R, A, C or I".into());
            }
            let replace = match args.get("replace") {
                Some(v) => v.as_bool().ok_or("replace must be a boolean")?,
                None => false,
            };
            let blocks = scoped_blocks(&all, &ctx, "selection")?;
            for block in &blocks {
                if action == "assignment.set" {
                    let roles = data
                        .assignments
                        .entry(block.id.to_string())
                        .or_default()
                        .category_mut(category);
                    if replace {
                        roles.clear();
                    }
                    roles.insert(id.clone());
                } else if let Some(assignment) = data.assignments.get_mut(&block.id.to_string()) {
                    assignment.category_mut(category).remove(&id);
                }
            }
            output(
                Some(&data),
                json!({"text":format!("{}: {category} / {role}; {} function(s)", if action == "assignment.set" {"Assigned"} else {"Removed"}, blocks.len()),"block_count":blocks.len()}),
            )
        }
        "assignment.clear" => {
            let blocks = scoped_blocks(&all, &ctx, "selection")?;
            for block in &blocks {
                data.assignments.remove(&block.id.to_string());
            }
            output(
                Some(&data),
                json!({"text":format!("Cleared assignments for {} function(s)",blocks.len()),"block_count":blocks.len()}),
            )
        }
        "data.prune" => {
            let live: BTreeSet<_> = all.iter().map(|b| b.id.to_string()).collect();
            let before = data.assignments.len();
            data.assignments.retain(|id, _| live.contains(id));
            let removed = before - data.assignments.len();
            output(
                Some(&data),
                json!({"removed":removed,"text":format!("Removed {removed} assignment record(s) for deleted functions")}),
            )
        }
        "matrix.export" | "validate" => {
            let scope = args
                .get("scope")
                .map(|_| text_arg(args, "scope"))
                .transpose()?
                .unwrap_or("document");
            let blocks = scoped_blocks(&all, &ctx, scope)?;
            if action == "validate" {
                validate(&data, &all, &blocks, scope)
            } else {
                let format = args
                    .get("format")
                    .map(|_| text_arg(args, "format"))
                    .transpose()?
                    .unwrap_or("markdown");
                export(&data, &blocks, format)
            }
        }
        _ => Err(format!("Unknown Responsibility Map action: {action}")),
    }
}
fn validate(data: &Data, all: &[Block], blocks: &[&Block], scope: &str) -> Result<String, String> {
    let mut diagnostics = Vec::new();
    let mut add =
        |severity: &str, code: &str, diagram: Option<u64>, object: Value, message: String| {
            diagnostics.push(
                json!({"severity":severity,"code":code,"diagram":diagram,"object":object,
            "message":message,"source":NAMESPACE}),
            );
        };
    for block in blocks {
        let assignment = data
            .assignments
            .get(&block.id.to_string())
            .cloned()
            .unwrap_or_default();
        let responsible = assignment
            .responsible
            .iter()
            .filter(|id| data.roles.contains_key(*id))
            .count();
        let accountable = assignment
            .accountable
            .iter()
            .filter(|id| data.roles.contains_key(*id))
            .count();
        for (missing, code, message) in [
            (
                responsible == 0,
                "missing_responsible",
                "Assign at least one Responsible (R) role",
            ),
            (
                accountable == 0,
                "missing_accountable",
                "Assign one Accountable (A) role",
            ),
            (
                accountable > 1,
                "multiple_accountable",
                "Multiple Accountable (A) roles; choose one owner",
            ),
        ] {
            if missing {
                add(
                    "error",
                    code,
                    Some(block.diagram),
                    json!(block.id),
                    format!("{}: {message}", block.node),
                );
            }
        }
        for category in CATEGORIES {
            for id in assignment.category(category) {
                if !data.roles.contains_key(id) {
                    add(
                        "error",
                        "unknown_role",
                        Some(block.diagram),
                        json!(block.id),
                        format!("{}: {category} references unknown role {id}", block.node),
                    );
                }
            }
        }
    }
    if scope == "document" {
        let live: BTreeSet<_> = all.iter().map(|b| b.id.to_string()).collect();
        for id in data.assignments.keys().filter(|id| !live.contains(*id)) {
            add(
                "warning",
                "orphan_assignment",
                None,
                json!(id),
                format!("Assignments refer to deleted function {id}; run Prune deleted functions"),
            );
        }
    }
    let errors = diagnostics
        .iter()
        .filter(|d| d["severity"] == "error")
        .count();
    let warnings = diagnostics.len() - errors;
    let mut text = format!(
        "Responsibility Map — RACI policy (plugin policy, not an ISO requirement)\n{} function(s); {errors} error(s), {warnings} warning(s).\n",
        blocks.len()
    );
    for d in &diagnostics {
        text.push_str(&format!(
            "\n{} [{}]: {}",
            d["severity"].as_str().unwrap(),
            d["code"].as_str().unwrap(),
            d["message"].as_str().unwrap()
        ));
    }
    output(
        None,
        json!({"valid":errors == 0,"diagnostics":diagnostics,"text":text}),
    )
}
fn markdown_cell(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('`', "\\`")
        .replace('\r', "")
        .replace('\n', "<br>")
}
fn csv_cell(value: &str) -> String {
    // Quoting alone does not stop spreadsheet formula interpretation.
    let dangerous =
        value.trim_start().starts_with(['=', '+', '-', '@']) || value.starts_with(['\t', '\r']);
    format!(
        "\"{}{}\"",
        if dangerous { "'" } else { "" },
        value.replace('"', "\"\"")
    )
}
fn export(data: &Data, blocks: &[&Block], format: &str) -> Result<String, String> {
    if !["markdown", "csv", "json"].contains(&format) {
        return Err("format must be markdown, csv or json".into());
    }
    if blocks.len().saturating_mul(data.roles.len().max(1)) > 200_000 {
        return Err("Matrix is too large; export a diagram or selection instead".into());
    }
    let mut roles = data.roles.iter().collect::<Vec<_>>();
    roles.sort_by(|(a_id, a), (b_id, b)| {
        a.to_lowercase().cmp(&b.to_lowercase()).then(a_id.cmp(b_id))
    });
    let mut rows = Vec::new();
    let mut table = vec![vec!["Node".into(), "Function".into()]];
    table[0].extend(roles.iter().map(|(_, name)| (*name).clone()));
    for block in blocks {
        let assignment = data
            .assignments
            .get(&block.id.to_string())
            .cloned()
            .unwrap_or_default();
        let mut cells = vec![block.node.clone(), block.title.clone()];
        cells.extend(roles.iter().map(|(id, _)| {
            CATEGORIES
                .iter()
                .filter(|c| assignment.category(c).contains(*id))
                .copied()
                .collect::<Vec<_>>()
                .join("/")
        }));
        table.push(cells);
        rows.push(json!({"block":block.id,"diagram":block.diagram,"node":block.node,"function":block.title,
            "R":assignment.responsible,"A":assignment.accountable,"C":assignment.consulted,"I":assignment.informed}));
    }
    let matrix = json!({"roles":roles.iter().map(|(id,name)| json!({"id":id,"name":name})).collect::<Vec<_>>(),"rows":rows});
    let (text, mime, suffix) = match format {
        "json" => (
            serde_json::to_string_pretty(&matrix).map_err(|e| e.to_string())?,
            "application/json",
            "json",
        ),
        "csv" => (
            table
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|s| csv_cell(s))
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .collect::<Vec<_>>()
                .join("\r\n")
                + "\r\n",
            "text/csv;charset=utf-8",
            "csv",
        ),
        _ => {
            let mut text = String::from(
                "# Responsibility Map\n\nR = Responsible; A = Accountable; C = Consulted; I = Informed.\nAssignments are explicit per function; parent roles are not inherited.\n\n",
            );
            for (i, row) in table.iter().enumerate() {
                text.push_str(&format!(
                    "| {} |\n",
                    row.iter()
                        .map(|s| markdown_cell(s))
                        .collect::<Vec<_>>()
                        .join(" | ")
                ));
                if i == 0 {
                    text.push_str(&format!("| {} |\n", vec!["---"; row.len()].join(" | ")));
                }
            }
            (text, "text/markdown;charset=utf-8", "md")
        }
    };
    output(
        None,
        json!({"filename":format!("responsibility-map.{suffix}"),"mime":mime,"text":text,"row_count":blocks.len()}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn context() -> Value {
        json!({"api_version":1,"revision":0,"view":{"diagram":1,"selection":[2]},"arguments":{},
        "document":{"extensions":{"other_plugin":{"version":1,"data":{"keep":true}}},"diagrams":{
            "1":{"blocks":{"2":{"number":"A0","title":"Fulfil orders"}}},
            "3":{"blocks":{"4":{"number":"A1","title":"Receive order"},"5":{"number":"A2","title":"Ship order"}}}
        }}})
    }
    fn invoke(ctx: &mut Value, action: &str, args: Value) -> Result<Value, String> {
        ctx["arguments"] = args;
        let out: Value = serde_json::from_str(&run(action, &ctx.to_string())?).unwrap();
        for command in out["commands"].as_array().unwrap() {
            assert_eq!(command["action"], "extension_set");
            assert_eq!(command["args"]["namespace"], NAMESPACE);
            let args = &command["args"];
            ctx["document"]["extensions"][NAMESPACE] =
                json!({"version":args["version"],"data":args["data"]});
        }
        Ok(out)
    }
    fn assign(ctx: &mut Value, role: &str, category: &str) {
        invoke(
            ctx,
            "assignment.set",
            json!({"role":role,"responsibility":category}),
        )
        .unwrap();
    }
    fn check(ctx: &mut Value) -> Value {
        invoke(ctx, "validate", json!({"scope":"selection"})).unwrap()["result"].clone()
    }
    #[test]
    fn roles_can_be_renamed_without_losing_any_raci_assignments() {
        let mut ctx = context();
        let id=invoke(&mut ctx,"role.add",json!({"name":"Operations"})).unwrap()["result"]["role_id"].clone();
        for category in CATEGORIES {
            assign(&mut ctx, "Operations", category);
        }
        assert_eq!(check(&mut ctx)["valid"], true);
        invoke(
            &mut ctx,
            "role.rename",
            json!({"name":"Operations","new_name":"Order team"}),
        )
        .unwrap();
        assert_eq!(
            ctx["document"]["extensions"][NAMESPACE]["data"]["roles"][id.as_str().unwrap()],
            "Order team"
        );
        assert_eq!(check(&mut ctx)["valid"], true);
        invoke(&mut ctx, "role.remove", json!({"name":"Order team"})).unwrap();
        let data = &ctx["document"]["extensions"][NAMESPACE]["data"];
        assert!(data["roles"].as_object().unwrap().is_empty());
        for category in CATEGORIES {
            assert_eq!(data["assignments"]["2"][category], json!([]));
        }
        assert_eq!(
            ctx["document"]["extensions"]["other_plugin"]["data"]["keep"],
            true
        );
    }
    #[test]
    fn role_names_are_trimmed_case_insensitive_and_unambiguous() {
        let mut ctx = context();
        invoke(&mut ctx, "role.add", json!({"name":"  Руководитель  "})).unwrap();
        assert!(invoke(&mut ctx, "role.add", json!({"name":"РУКОВОДИТЕЛЬ"})).is_err());
        assign(&mut ctx, "руководитель", "A");
        assert!(invoke(&mut ctx, "role.add", json!({"name":"bad\nname"})).is_err());
        assert!(invoke(&mut ctx, "role.add", json!({"name":" "})).is_err());
    }
    #[test]
    fn conflicting_owners_are_reported_and_replace_only_changes_one_category() {
        let mut ctx = context();
        for name in ["Owner", "Backup"] {
            invoke(&mut ctx, "role.add", json!({"name":name})).unwrap();
        }
        assign(&mut ctx, "Owner", "R");
        assign(&mut ctx, "Owner", "A");
        assign(&mut ctx, "Backup", "A");
        let report = check(&mut ctx);
        assert_eq!(report["diagnostics"][0]["code"], "multiple_accountable");
        assert_eq!(report["diagnostics"][0]["source"], NAMESPACE);
        invoke(
            &mut ctx,
            "assignment.set",
            json!({"role":"Backup","responsibility":"A","replace":true}),
        )
        .unwrap();
        assert_eq!(check(&mut ctx)["valid"], true);
        assign(&mut ctx, "Backup", "A");
        assert_eq!(
            check(&mut ctx)["valid"],
            true,
            "Adding the same role is idempotent"
        );
        invoke(
            &mut ctx,
            "assignment.remove",
            json!({"role":"Owner","responsibility":"R"}),
        )
        .unwrap();
        assert_eq!(
            check(&mut ctx)["diagnostics"][0]["code"],
            "missing_responsible"
        );
    }
    #[test]
    fn only_selected_functions_are_changed_and_an_empty_selection_is_rejected() {
        let mut ctx = context();
        invoke(&mut ctx, "role.add", json!({"name":"Team"})).unwrap();
        ctx["view"]["selection"] = json!([4, 5, 999]);
        assign(&mut ctx, "Team", "R");
        let assignments = &ctx["document"]["extensions"][NAMESPACE]["data"]["assignments"];
        assert_eq!(assignments.as_object().unwrap().len(), 2);
        assert!(assignments.get("2").is_none());
        ctx["view"]["selection"] = json!([999]);
        let before = ctx["document"].clone();
        assert!(
            invoke(
                &mut ctx,
                "assignment.set",
                json!({"role":"Team","responsibility":"R"})
            )
            .is_err()
        );
        assert_eq!(ctx["document"], before);
    }
    #[test]
    fn parent_responsibility_is_not_implicitly_inherited_and_exports_respect_scope() {
        let mut ctx = context();
        invoke(&mut ctx, "role.add", json!({"name":"Team"})).unwrap();
        assign(&mut ctx, "Team", "R");
        assign(&mut ctx, "Team", "A");
        let out = invoke(&mut ctx, "validate", json!({})).unwrap();
        assert!(out["commands"].as_array().unwrap().is_empty());
        assert_eq!(out["result"]["diagnostics"].as_array().unwrap().len(), 4);
        for (scope, count) in [("document", 3), ("diagram", 1), ("selection", 1)] {
            let out = invoke(
                &mut ctx,
                "matrix.export",
                json!({"scope":scope,"format":"json"}),
            )
            .unwrap();
            assert_eq!(out["result"]["row_count"], count);
            assert!(out["commands"].as_array().unwrap().is_empty());
        }
    }
    #[test]
    fn unknown_roles_and_deleted_functions_are_diagnosed_and_can_be_pruned() {
        let mut ctx = context();
        invoke(&mut ctx, "role.add", json!({"name":"Team"})).unwrap();
        ctx["document"]["extensions"][NAMESPACE]["data"]["assignments"] =
            json!({"2":{"R":["absent"]},"999":{"A":["role_1"]}});
        let out = invoke(&mut ctx, "validate", json!({})).unwrap();
        let codes: Vec<_> = out["result"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["code"].as_str().unwrap())
            .collect();
        assert!(codes.contains(&"unknown_role"));
        assert!(codes.contains(&"orphan_assignment"));
        assert_eq!(
            invoke(&mut ctx, "data.prune", json!({})).unwrap()["result"]["removed"],
            1
        );
        assert!(
            ctx["document"]["extensions"][NAMESPACE]["data"]["assignments"]
                .get("2")
                .is_some()
        );
    }
    #[test]
    fn version_errors_and_bad_arguments_do_not_overwrite_data() {
        let mut ctx = context();
        invoke(&mut ctx, "role.add", json!({"name":"Team"})).unwrap();
        for args in [
            json!({"role":"unknown","responsibility":"R"}),
            json!({"role":"Team","responsibility":"Z"}),
            json!({"role":"Team","responsibility":"R","replace":"yes"}),
        ] {
            let before = ctx["document"].clone();
            assert!(invoke(&mut ctx, "assignment.set", args).is_err());
            assert_eq!(ctx["document"], before);
        }
        ctx["document"]["extensions"][NAMESPACE]["version"] = json!(2);
        let before = ctx["document"].clone();
        assert!(invoke(&mut ctx, "role.add", json!({"name":"Next"})).is_err());
        assert_eq!(ctx["document"], before);
    }
    #[test]
    fn same_version_extra_data_is_preserved_during_edits() {
        let mut ctx = context();
        invoke(&mut ctx, "role.add", json!({"name":"Team"})).unwrap();
        ctx["document"]["extensions"][NAMESPACE]["data"]["future_note"] = json!({"keep":true});
        ctx["document"]["extensions"][NAMESPACE]["data"]["assignments"]["2"] =
            json!({"future":"keep"});
        assign(&mut ctx, "Team", "R");
        let data = &ctx["document"]["extensions"][NAMESPACE]["data"];
        assert_eq!(data["future_note"]["keep"], true);
        assert_eq!(data["assignments"]["2"]["future"], "keep");
        invoke(&mut ctx, "assignment.clear", json!({})).unwrap();
        assert!(
            ctx["document"]["extensions"][NAMESPACE]["data"]["assignments"]
                .get("2")
                .is_none()
        );
    }
    #[test]
    fn exports_escape_table_cells_csv_quotes_newlines_and_formula_prefixes() {
        let mut ctx = context();
        ctx["document"]["diagrams"]["1"]["blocks"]["2"]["title"] =
            json!("=SUM(1,2)\n\"quoted\" | <b>");
        invoke(&mut ctx, "role.add", json!({"name":"=Role | <script>"})).unwrap();
        for format in ["markdown", "csv", "json"] {
            let out = invoke(
                &mut ctx,
                "matrix.export",
                json!({"format":format,"scope":"selection"}),
            )
            .unwrap();
            let text = out["result"]["text"].as_str().unwrap();
            match format {
                "markdown" => {
                    assert!(text.contains("\\|"));
                    assert!(text.contains("&lt;b&gt;"));
                    assert!(text.contains("<br>"));
                }
                "csv" => {
                    assert!(text.contains("\"'=SUM(1,2)"));
                    assert!(text.contains("\"'=Role"));
                    assert!(text.contains("\"\"quoted\"\""));
                    assert!(text.ends_with("\r\n"));
                }
                _ => {
                    let parsed: Value = serde_json::from_str(text).unwrap();
                    assert_eq!(
                        parsed["rows"][0]["function"],
                        ctx["document"]["diagrams"]["1"]["blocks"]["2"]["title"]
                    );
                }
            }
        }
    }
}
