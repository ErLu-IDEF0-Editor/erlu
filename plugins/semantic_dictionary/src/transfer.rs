use crate::model::*;
use crate::{arg, check_revision, confirm, optional, output};
use serde_json::{Value, json};
use std::collections::BTreeSet;
fn md(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('`', "\\`")
        .replace('\r', "")
        .replace('\n', "<br>")
}
fn csv(s: &str) -> String {
    let formula = s.trim_start().starts_with(['=', '+', '-', '@']) || s.starts_with(['\t', '\r']);
    format!(
        "\"{}{}\"",
        if formula { "'" } else { "" },
        s.replace('"', "\"\"")
    )
}
pub fn export(data: &Data, graph: &Graph, ctx: &Value) -> Result<String, String> {
    let relevant = crate::reports::scoped_concepts(data, graph, ctx)?;
    let mut concepts: Vec<_> = data
        .concepts
        .iter()
        .filter(|(id, _)| relevant.contains(*id))
        .map(|(_, c)| c)
        .collect();
    concepts.sort_by_key(|c| key(&c.code));
    let catalog = json!({"version":1,"concepts":concepts.iter().map(|c|c.catalog_record()).collect::<Vec<_>>()});
    let format = optional(&ctx["arguments"], "format", "markdown")?;
    let mut rows = vec![vec![
        "Code".into(),
        "Term".into(),
        "Definition".into(),
        "Synonyms".into(),
        "Owner".into(),
        "Source".into(),
    ]];
    rows.extend(concepts.iter().map(|c| {
        vec![
            c.code.clone(),
            c.term.clone(),
            c.definition.clone(),
            c.aliases.iter().cloned().collect::<Vec<_>>().join("\n"),
            c.owner.clone(),
            c.source.clone(),
        ]
    }));
    let (text, mime, suffix) = match format {
        "json" => (
            serde_json::to_string_pretty(&catalog).map_err(|e| e.to_string())?,
            "application/json",
            "json",
        ),
        "csv" => (
            rows.iter()
                .map(|r| r.iter().map(|s| csv(s)).collect::<Vec<_>>().join(","))
                .collect::<Vec<_>>()
                .join("\r\n")
                + "\r\n",
            "text/csv;charset=utf-8",
            "csv",
        ),
        "markdown" => {
            let mut text = String::from("# Semantic Dictionary\n\n");
            for (i, row) in rows.iter().enumerate() {
                text.push_str(&format!(
                    "| {} |\n",
                    row.iter().map(|s| md(s)).collect::<Vec<_>>().join(" | ")
                ));
                if i == 0 {
                    text.push_str("| --- | --- | --- | --- | --- | --- |\n");
                }
            }
            (text, "text/markdown;charset=utf-8", "md")
        }
        _ => return Err("Supported formats: markdown, csv, json".into()),
    };
    output(
        vec![],
        json!({"text":text,"mime":mime,"filename":format!("semantic-dictionary.{suffix}"),"concept_count":concepts.len()}),
    )
}
pub fn import(action: &str, data: &mut Data, ctx: &Value) -> Result<String, String> {
    let catalog = &ctx["arguments"]["catalog"];
    if catalog["version"].as_u64() != Some(1) {
        return Err("Expected catalog version 1".into());
    }
    let records = catalog["concepts"]
        .as_array()
        .ok_or("Catalog concepts must be an array")?;
    if records.len() > 1000 {
        return Err("Catalog exceeds 1000 concepts".into());
    }
    let mut seen = BTreeSet::new();
    let mut additions = Vec::new();
    let mut conflicts = Vec::new();
    let mut skipped = 0;
    for record in records {
        // Only transferable fields are imported; IDs, bindings and unknown object references are ignored.
        let aliases = record.get("aliases").cloned().unwrap_or_else(|| json!([]));
        let aliases: BTreeSet<String> =
            serde_json::from_value(aliases).map_err(|e| format!("Invalid aliases: {e}"))?;
        let concept = Concept {
            code: name(arg(record, "code")?)?,
            term: name(arg(record, "term")?)?,
            definition: optional(record, "definition", "")?.into(),
            owner: optional(record, "owner", "")?.into(),
            source: optional(record, "source", "")?.into(),
            aliases: aliases
                .into_iter()
                .map(|s| name(&s))
                .collect::<Result<_, _>>()?,
            extra: Extra::new(),
        };
        if !seen.insert(key(&concept.code)) {
            return Err(format!("Duplicate code in import: {}", concept.code));
        }
        if let Ok(id) = data.id(&concept.code) {
            let mut existing = data.concepts[&id].catalog_record();
            let mut incoming = concept.catalog_record();
            existing["code"] = json!(key(&concept.code));
            incoming["code"] = existing["code"].clone();
            if existing == incoming {
                skipped += 1;
            } else {
                conflicts.push(concept.code.clone());
            }
        } else {
            additions.push(concept);
        }
    }
    if data.concepts.len() + additions.len() > 1000 {
        return Err("Import would exceed 1000 concepts".into());
    }
    if action == "catalog.import_apply" {
        check_revision(ctx)?;
        if !conflicts.is_empty() {
            return Err(format!(
                "Import conflicts; nothing changed: {}",
                conflicts.join(", ")
            ));
        }
        for concept in additions {
            data.insert(concept)?;
        }
        return output(
            vec![data.command()],
            json!({"text":format!("Catalog imported; {skipped} identical concept(s) skipped")}),
        );
    }
    let mut projected = data.clone();
    for concept in &additions {
        projected.insert(concept.clone())?;
    }
    let warnings = crate::reports::collisions(&projected);
    let mut text = format!(
        "Catalog import preview\n{} additions, {skipped} identical records skipped, {} conflicts\n",
        additions.len(),
        conflicts.len()
    );
    for c in &additions {
        text.push_str(&format!("\nAdd {} — {}", c.code, c.term));
    }
    for code in &conflicts {
        text.push_str(&format!(
            "\nConflict: {code}; existing entry will not be overwritten"
        ));
    }
    for warning in &warnings {
        text.push_str(&format!(
            "\nAmbiguous spelling: {} — {}",
            warning["spelling"], warning["codes"]
        ));
    }
    let mut result = json!({"text":text,"additions":additions.iter().map(|c|c.catalog_record()).collect::<Vec<_>>(),"skipped":skipped,"conflicts":conflicts,"warnings":warnings});
    if conflicts.is_empty() && !additions.is_empty() {
        result["confirm_action"] = confirm(ctx, "catalog.import_apply", json!({"catalog":catalog}));
    }
    output(vec![], result)
}
