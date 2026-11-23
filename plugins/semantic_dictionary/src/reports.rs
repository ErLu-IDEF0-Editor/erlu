use crate::model::*;
use crate::{arg, optional, output};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
pub fn arrow_value(a: &Arrow, root: u64) -> Value {
    json!({"arrow":a.id,"diagram":a.diagram,"node":a.node,"label":a.label,"root":root,"inherited":a.id!=root,"icom":a.icom})
}
pub fn scoped_concepts(
    data: &Data,
    graph: &Graph,
    ctx: &Value,
) -> Result<BTreeSet<String>, String> {
    let scope = optional(&ctx["arguments"], "scope", "document")?;
    if scope == "document" {
        return Ok(data.concepts.keys().cloned().collect());
    }
    let arrows = graph.scoped(ctx, scope)?;
    Ok(arrows
        .iter()
        .flat_map(|id| graph.concepts(graph.roots[id], data))
        .collect())
}
pub fn search(data: &Data, graph: &Graph, ctx: &Value) -> Result<String, String> {
    let query = key(optional(&ctx["arguments"], "query", "")?);
    let relevant = scoped_concepts(data, graph, ctx)?;
    let mut found: Vec<_> = data
        .concepts
        .iter()
        .filter(|(id, c)| {
            relevant.contains(*id)
                && [&c.code, &c.term, &c.definition, &c.owner, &c.source]
                    .into_iter()
                    .chain(c.aliases.iter())
                    .any(|s| key(s).contains(&query))
        })
        .map(|(_, c)| c.catalog_record())
        .collect();
    found.sort_by_key(|v| key(v["code"].as_str().unwrap()));
    let mut text = format!("Semantic Dictionary: {} concept(s)\n", found.len());
    for c in &found {
        text.push_str(&format!(
            "\n{} — {}\n{}\nSynonyms: {}\nOwner: {}\nSource: {}\n",
            c["code"].as_str().unwrap(),
            c["term"].as_str().unwrap(),
            c["definition"].as_str().unwrap(),
            c["aliases"],
            c["owner"].as_str().unwrap(),
            c["source"].as_str().unwrap()
        ));
    }
    output(vec![], json!({"text":text,"matches":found}))
}
pub fn usage(data: &Data, graph: &Graph, ctx: &Value) -> Result<String, String> {
    let concept = data.id(arg(&ctx["arguments"], "code")?)?;
    let ids = graph.scoped(ctx, optional(&ctx["arguments"], "scope", "document")?)?;
    let uses: Vec<_> = ids
        .iter()
        .filter(|id| graph.concepts(graph.roots[id], data).contains(&concept))
        .map(|id| arrow_value(&graph.arrows[id], graph.roots[id]))
        .collect();
    let mut text = format!(
        "{} — {}: {} use(s)\n",
        data.concepts[&concept].code,
        data.concepts[&concept].term,
        uses.len()
    );
    for a in &uses {
        text.push_str(&format!(
            "\nDiagram {}, arrow {}: {:?}; {}{}",
            a["node"].as_str().unwrap(),
            a["arrow"],
            a["label"].as_str().unwrap(),
            a["icom"].as_str().unwrap(),
            if a["inherited"] == true {
                " (inherited label)"
            } else {
                ""
            }
        ));
    }
    output(vec![], json!({"text":text,"uses":uses}))
}
pub fn collisions(data: &Data) -> Vec<Value> {
    let mut spellings: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for c in data.concepts.values() {
        for spelling in c.spellings() {
            spellings
                .entry(spelling)
                .or_default()
                .insert(c.code.clone());
        }
    }
    spellings
        .into_iter()
        .filter(|(_, codes)| codes.len() > 1)
        .map(|(spelling, codes)| json!({"spelling":spelling,"codes":codes}))
        .collect()
}
pub fn validate(data: &Data, graph: &Graph, ctx: &Value) -> Result<String, String> {
    let scope = optional(&ctx["arguments"], "scope", "document")?;
    let ids = graph.scoped(ctx, scope)?;
    let roots: BTreeSet<_> = ids.iter().map(|id| graph.roots[id]).collect();
    let relevant = scoped_concepts(data, graph, ctx)?;
    let mut diagnostics = Vec::new();
    let mut add = |severity: &str, code: &str, object: Value, message: String| {
        diagnostics.push(
            json!({"severity":severity,"code":code,"object":object,"message":message,"source":NS}),
        );
    };
    for id in relevant {
        if let Some(c) = data.concepts.get(&id)
            && c.definition.trim().is_empty()
        {
            add(
                "warning",
                "missing_definition",
                json!(c.code),
                format!("{}: provide a definition", c.code),
            );
        }
    }
    // Spelling conflicts are global: the same label can denote a different concept elsewhere.
    for collision in collisions(data) {
        add(
            "warning",
            "ambiguous_spelling",
            Value::Null,
            format!(
                "{} identifies multiple concepts: {}",
                collision["spelling"].as_str().unwrap(),
                collision["codes"]
            ),
        );
    }
    for root in roots {
        let bound = graph.concepts(root, data);
        let a = &graph.arrows[&root];
        if bound.len() > 1 {
            add(
                "error",
                "conflicting_bindings",
                json!(root),
                format!("Label group {root} has conflicting concept bindings"),
            );
        }
        for id in &bound {
            if let Some(c) = data.concepts.get(id) {
                if a.label != c.term {
                    let code = if c.spellings().contains(&key(&a.label)) {
                        "noncanonical_label"
                    } else {
                        "binding_label_mismatch"
                    };
                    add(
                        "warning",
                        code,
                        json!(root),
                        format!(
                            "Diagram {}, arrow {root}: {:?} differs from {} ({:?})",
                            a.node, a.label, c.code, c.term
                        ),
                    );
                }
            } else {
                add(
                    "error",
                    "unknown_concept",
                    json!(root),
                    format!("Label group {root} references unknown concept {id}"),
                );
            }
        }
        if bound.is_empty() && !a.label.trim().is_empty() {
            let candidates: Vec<_> = data
                .concepts
                .values()
                .filter(|c| c.spellings().contains(&key(&a.label)))
                .map(|c| c.code.clone())
                .collect();
            let code = if candidates.len() > 1 {
                "ambiguous_label"
            } else {
                "unbound_label"
            };
            add(
                "warning",
                code,
                json!(root),
                format!(
                    "Diagram {}, arrow {root}: {:?} is not bound; candidate codes: {}",
                    a.node,
                    a.label,
                    candidates.join(", ")
                ),
            );
        }
    }
    if scope == "document" {
        for id in data.bindings.keys() {
            if !id
                .parse::<u64>()
                .ok()
                .is_some_and(|id| graph.arrows.get(&id).is_some_and(|a| a.eligible))
            {
                add(
                    "warning",
                    "orphan_binding",
                    json!(id),
                    format!(
                        "Binding for deleted or unsupported arrow {id}; run Prune deleted arrows"
                    ),
                );
            }
        }
    }
    let errors = diagnostics
        .iter()
        .filter(|d| d["severity"] == "error")
        .count();
    let mut text = format!(
        "Semantic Dictionary policy (not an ISO conformance check)\n{} diagnostic(s), {errors} error(s)\n",
        diagnostics.len()
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
        vec![],
        json!({"text":text,"valid":errors==0,"diagnostics":diagnostics}),
    )
}
