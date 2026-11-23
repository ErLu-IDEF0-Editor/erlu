//! Optional terminology annotations. Dictionary diagnostics are not ISO requirements.
mod model;
mod reports;
mod transfer;
use model::*;
use serde_json::{Value, json};
use std::collections::BTreeSet;
#[cfg(target_arch = "wasm32")]
wit_bindgen::generate!({world:"plugin",path:"wit"});
#[cfg(target_arch = "wasm32")]
struct SemanticDictionary;
#[cfg(target_arch = "wasm32")]
impl Guest for SemanticDictionary {
    fn run(action: String, context: String) -> Result<String, String> {
        run(&action, &context)
    }
}
#[cfg(target_arch = "wasm32")]
export!(SemanticDictionary);
pub(crate) fn arg<'a>(args: &'a Value, field: &str) -> Result<&'a str, String> {
    args[field]
        .as_str()
        .ok_or_else(|| format!("{field} must be a string"))
}
pub(crate) fn optional<'a>(
    args: &'a Value,
    field: &str,
    default: &'a str,
) -> Result<&'a str, String> {
    match args.get(field) {
        Some(v) => v
            .as_str()
            .ok_or_else(|| format!("{field} must be a string")),
        None => Ok(default),
    }
}
pub(crate) fn output(commands: Vec<Value>, result: Value) -> Result<String, String> {
    if commands.len() > 1000 {
        return Err("Too many changes; narrow the scope to a diagram or selection".into());
    }
    let text = json!({"commands":commands,"result":result}).to_string();
    if text.len() > 3 * 1024 * 1024 {
        return Err("Dictionary response is too large; narrow the scope".into());
    }
    Ok(text)
}
fn saved(data: &Data, text: String) -> Result<String, String> {
    output(vec![data.command()], json!({"text":text}))
}
pub(crate) fn confirm(ctx: &Value, action: &str, mut args: Value) -> Value {
    args["preview_revision"] = ctx["revision"].clone();
    json!({"name":format!("plugin_semantic_dictionary_{}",action.replace('.',"_")),
        "arguments":args,"expected_revision":ctx["revision"]})
}
pub(crate) fn check_revision(ctx: &Value) -> Result<(), String> {
    let expected = ctx["arguments"]["preview_revision"]
        .as_u64()
        .ok_or("Generate a preview before applying changes")?;
    if ctx["revision"].as_u64() != Some(expected) {
        return Err("revision_conflict: preview is stale; generate a new preview".into());
    }
    Ok(())
}
pub fn run(action: &str, context: &str) -> Result<String, String> {
    let ctx: Value = serde_json::from_str(context).map_err(|e| e.to_string())?;
    if ctx["api_version"] != 1 || !ctx["arguments"].is_object() {
        return Err("Expected API v1 and object arguments".into());
    }
    let args = &ctx["arguments"];
    let mut data = Data::read(&ctx["document"])?;
    let graph = Graph::read(&ctx["document"])?;
    match action {
        "concept.add" => {
            let code = name(arg(args, "code")?)?;
            let term = name(arg(args, "term")?)?;
            if data.id(&code).is_ok() {
                return Err(format!("Concept code already exists: {code}"));
            }
            data.insert(Concept {
                code: code.clone(),
                term,
                definition: optional(args, "definition", "")?.into(),
                owner: optional(args, "owner", "")?.into(),
                source: optional(args, "source", "")?.into(),
                aliases: BTreeSet::new(),
                extra: Extra::new(),
            })?;
            saved(&data, format!("Concept added: {code}"))
        }
        "concept.update" => {
            let id = data.id(arg(args, "code")?)?;
            let field = arg(args, "field")?;
            let value = arg(args, "value")?;
            let concept = data.concepts.get_mut(&id).unwrap();
            match field {
                "term" => {
                    concept.term = name(value)?;
                }
                "definition" => concept.definition = value.into(),
                "owner" => concept.owner = value.into(),
                "source" => concept.source = value.into(),
                _ => return Err(
                    "Editable fields: term, definition, owner, source; concept codes are immutable"
                        .into(),
                ),
            }
            saved(
                &data,
                "Concept updated; arrow labels were not changed".into(),
            )
        }
        "concept.remove" => {
            let id = data.id(arg(args, "code")?)?;
            if data.bindings.values().any(|bound| bound == &id) {
                return Err(
                    "Unbind this concept before removing it; prune deleted arrows if needed".into(),
                );
            }
            data.concepts.remove(&id);
            saved(&data, "Concept removed".into())
        }
        "alias.add" | "alias.remove" => {
            let id = data.id(arg(args, "code")?)?;
            let alias = name(arg(args, "alias")?)?;
            let concept = data.concepts.get_mut(&id).unwrap();
            if action == "alias.add" {
                if key(&alias) == key(&concept.term) {
                    return Err("The alias duplicates the primary term".into());
                }
                if !concept.aliases.iter().any(|s| key(s) == key(&alias)) {
                    concept.aliases.insert(alias);
                }
            } else {
                concept.aliases.retain(|s| key(s) != key(&alias));
            }
            saved(&data, "Synonyms updated".into())
        }
        "binding.set" | "binding.clear" => {
            let ids = graph.scoped(&ctx, "selection")?;
            let roots: BTreeSet<_> = ids.iter().map(|id| graph.roots[id]).collect();
            let target = if action == "binding.set" {
                Some(data.id(arg(args, "code")?)?)
            } else {
                None
            };
            let replace = match args.get("replace") {
                Some(v) => v.as_bool().ok_or("replace must be boolean")?,
                None => false,
            };
            if let Some(target) = &target {
                for root in &roots {
                    if !replace && graph.concepts(*root, &data).iter().any(|id| id != target) {
                        return Err("A label group is already bound to another concept; enable explicit replacement".into());
                    }
                }
            }
            let mut affected = 0;
            for root in roots {
                for id in &graph.groups[&root] {
                    if let Some(target) = &target {
                        data.bindings.insert(id.to_string(), target.clone());
                    } else {
                        data.bindings.remove(&id.to_string());
                    }
                    affected += 1;
                }
            }
            saved(
                &data,
                format!(
                    "Updated bindings for {affected} arrows in the selected label groups; labels were not changed"
                ),
            )
        }
        "search" => reports::search(&data, &graph, &ctx),
        "usage" => reports::usage(&data, &graph, &ctx),
        "validate" => reports::validate(&data, &graph, &ctx),
        "normalize.preview" | "normalize.apply" => normalize(action, &data, &graph, &ctx),
        "catalog.export" => transfer::export(&data, &graph, &ctx),
        "catalog.import_preview" | "catalog.import_apply" => {
            transfer::import(action, &mut data, &ctx)
        }
        "data.prune" => {
            let before = data.bindings.len();
            data.bindings.retain(|id, _| {
                id.parse::<u64>()
                    .ok()
                    .is_some_and(|id| graph.arrows.get(&id).is_some_and(|a| a.eligible))
            });
            saved(
                &data,
                format!(
                    "Removed {} bindings to deleted or unsupported arrows",
                    before - data.bindings.len()
                ),
            )
        }
        _ => Err(format!("Unknown Semantic Dictionary action: {action}")),
    }
}
fn normalize(action: &str, data: &Data, graph: &Graph, ctx: &Value) -> Result<String, String> {
    let args = &ctx["arguments"];
    let roots: BTreeSet<u64> = if action == "normalize.apply" {
        check_revision(ctx)?;
        args["roots"]
            .as_array()
            .ok_or("Missing preview roots")?
            .iter()
            .map(|v| v.as_u64().ok_or("Invalid preview root"))
            .collect::<Result<_, _>>()?
    } else {
        let ids = graph.scoped(ctx, optional(args, "scope", "document")?)?;
        ids.iter().map(|id| graph.roots[id]).collect()
    };
    let filter = optional(args, "code", "")?;
    let filter = if filter.trim().is_empty() {
        None
    } else {
        Some(data.id(filter)?)
    };
    let mut changes = Vec::new();
    let mut commands = Vec::new();
    let mut changed_roots = Vec::new();
    for root in roots {
        let members = graph
            .groups
            .get(&root)
            .ok_or("Preview root is no longer a label source")?;
        let bound = graph.concepts(root, data);
        if bound.len() > 1 {
            return Err(format!(
                "Conflicting concept bindings in label group {root}; resolve them before normalization"
            ));
        }
        let Some(id) = bound.first() else { continue };
        if filter.as_ref().is_some_and(|f| f != id) {
            continue;
        }
        let concept = data
            .concepts
            .get(id)
            .ok_or("Unknown concept in binding; resolve it before normalization")?;
        let old = &graph.arrows[&root].label;
        if old == &concept.term {
            continue;
        }
        let uses: Vec<_> = members
            .iter()
            .map(|id| reports::arrow_value(&graph.arrows[id], root))
            .collect();
        changes.push(
            json!({"root":root,"code":concept.code,"old":old,"new":concept.term,"uses":uses}),
        );
        commands.push(json!({"action":"arrow_update","args":{"id":root,"label":concept.term}}));
        changed_roots.push(root);
    }
    if action == "normalize.apply" {
        return output(
            commands,
            json!({"text":format!("Normalized {} label groups",changes.len()),"changes":changes}),
        );
    }
    let mut text = format!(
        "Label normalization preview: {} group(s).\nAll inherited uses below will change, including uses outside the selected scope.\n",
        changes.len()
    );
    for change in &changes {
        text.push_str(&format!(
            "\n{}: {:?} → {:?}\n",
            change["code"].as_str().unwrap(),
            change["old"].as_str().unwrap(),
            change["new"].as_str().unwrap()
        ));
        for usage in change["uses"].as_array().unwrap() {
            text.push_str(&format!(
                "  diagram {}, arrow {}: {}\n",
                usage["node"].as_str().unwrap(),
                usage["arrow"],
                usage["icom"].as_str().unwrap()
            ));
        }
    }
    let mut result = json!({"text":text,"changes":changes});
    if !changed_roots.is_empty() {
        result["confirm_action"] = confirm(ctx, "normalize.apply", json!({"roots":changed_roots}));
    }
    if commands.len() > 1000 {
        return Err("Too many changes; narrow the scope".into());
    }
    output(vec![], result)
}
#[cfg(test)]
mod tests;
