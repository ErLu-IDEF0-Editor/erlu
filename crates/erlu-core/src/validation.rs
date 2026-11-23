use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: String,
    pub code: String,
    pub diagram: Id,
    pub object: Option<Id>,
    pub message: String,
    /// Published source (which may be informative) or the editor's own checks.
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reference: Option<String>,
}
pub fn validate(doc: &Document) -> Vec<Diagnostic> {
    let mut out = vec![];
    // See docs/validation.md for evidence, scope and unverified provisions.
    for (code, value, message) in [
        ("model_name", &doc.meta.title, "Provide the full model name"),
        (
            "model_abbreviation",
            &doc.meta.model_abbreviation,
            "Provide the model name abbreviation",
        ),
        (
            "model_purpose",
            &doc.meta.purpose,
            "Provide the model purpose statement",
        ),
        (
            "model_viewpoint",
            &doc.meta.viewpoint,
            "Provide the model viewpoint statement",
        ),
    ] {
        if value.trim().is_empty() {
            out.push(diagnostic("error", code, doc.root, None, message.into()));
        }
    }
    for d in doc.diagrams.values() {
        let mut push = |severity: &str, code: &str, object: Option<Id>, message: String| {
            out.push(diagnostic(severity, code, d.id, object, message))
        };
        if d.id == doc.root && d.blocks.len() != 1 {
            push(
                "error",
                "context_block_count",
                None,
                "The context diagram should contain one block".into(),
            );
        }
        if d.id != doc.root && !(2..=9).contains(&d.blocks.len()) {
            push(
                "error",
                "decomposition_block_count",
                None,
                "A decomposition should contain 2–9 blocks".into(),
            );
        }
        if d.arrows.values().any(|a| {
            a.interface.as_ref().is_some_and(|i| i.stub) && !a.from.tunneled && !a.to.tunneled
        }) {
            push(
                "error",
                "incomplete_interface",
                None,
                "Decomposition has unconnected interface circles".into(),
            );
        }
        let mut duplicates = BTreeMap::<String, usize>::new();
        // Visual node numbers are distinct from stable object IDs.
        let prefix = d.parent_block.and_then(|id| doc.block(id)).map(|b| {
            if b.number == "A0" {
                "A"
            } else {
                b.number.as_str()
            }
        });
        let expected: BTreeSet<String> = if d.id == doc.root {
            BTreeSet::from(["A0".into()])
        } else {
            (1..=d.blocks.len())
                .map(|n| format!("{}{n}", prefix.unwrap_or("A")))
                .collect()
        };
        let actual: BTreeSet<_> = d.blocks.values().map(|b| b.number.clone()).collect();
        if actual != expected || actual.len() != d.blocks.len() {
            push("error", "node_numbering", None,
                "Use A0 for the context box and consecutive child node numbers formed by appending 1–9 to the parent node".into());
        }
        for b in d.blocks.values() {
            if b.title.trim().is_empty() {
                push(
                    "error",
                    "block_name",
                    Some(b.id),
                    format!("{}: provide a meaningful function name", b.number),
                );
            } else if ["Function", "Функция", "Fonction"].contains(&b.title.trim()) {
                push(
                    "warning",
                    "block_name",
                    Some(b.id),
                    format!("{}: replace the placeholder function name", b.number),
                );
            }
            let incoming:Vec<_>=d.arrows.values().filter(|a|matches!(a.to.anchor,Anchor::Block{id,port:Port::Input|Port::Control,..}if id==b.id)).collect();
            let outgoing: Vec<_> = d
                .arrows
                .values()
                .filter(|a| {
                    a.call_ref.is_none()
                        && !a.interface.as_ref().is_some_and(|i| i.stub)
                        && matches!(a.from.anchor,Anchor::Block{id,port:Port::Output,..}if id==b.id)
                })
                .collect();
            let control = incoming.iter().any(|a| {
                a.call_ref.is_none()
                    && !a.interface.as_ref().is_some_and(|i| i.stub)
                    && matches!(
                        a.to.anchor,
                        Anchor::Block {
                            port: Port::Control,
                            ..
                        }
                    )
            });
            if !control {
                push(
                    "error",
                    "missing_control",
                    Some(b.id),
                    format!("{}: at least one control is required", b.number),
                );
            }
            if outgoing.is_empty() {
                push(
                    "error",
                    "missing_output",
                    Some(b.id),
                    format!("{}: at least one output is required", b.number),
                );
            }
            let names: BTreeSet<_> = incoming
                .iter()
                .filter(|a| {
                    matches!(
                        a.to.anchor,
                        Anchor::Block {
                            port: Port::Input,
                            ..
                        }
                    )
                })
                .map(|a| a.label.trim())
                .filter(|s| !s.is_empty())
                .collect();
            for a in &outgoing {
                if names.contains(a.label.trim()) {
                    push(
                        "warning",
                        "identical_input_output",
                        Some(b.id),
                        format!("{}: input and output share the name {}", b.number, a.label),
                    );
                }
            }
            let incident: Vec<_> = d
                .arrows
                .values()
                .flat_map(|a| [&a.from, &a.to])
                .filter(|e| matches!(e.anchor,Anchor::Block{id,..}if id==b.id))
                .collect();
            if !incident.is_empty() && incident.iter().all(|e| e.tunneled) {
                push(
                    "warning",
                    "black_box",
                    Some(b.id),
                    format!("{}: all connected endpoints are tunneled", b.number),
                );
            }
            let related=d.arrows.values().any(|a|matches!((&a.from.anchor,&a.to.anchor),(Anchor::Block{id:a,..},Anchor::Block{id:c,..})if a!=c&&(*a==b.id||*c==b.id)));
            if d.id != doc.root && d.blocks.len() > 1 && !related {
                push(
                    "warning",
                    "unrelated_block",
                    Some(b.id),
                    format!(
                        "{}: no direct relation to other decomposition blocks",
                        b.number
                    ),
                );
            }
            if let (Some(price), Some(c)) = (b.price, b.child) {
                let sum: f64 = doc.diagrams[&c]
                    .blocks
                    .values()
                    .filter_map(|b| b.price)
                    .sum();
                if sum > price + 1e-6 {
                    push(
                        "warning",
                        "cost_mismatch",
                        Some(b.id),
                        format!("{}: subprocess cost exceeds parent cost", b.number),
                    );
                }
            }
        }
        for a in d.arrows.values() {
            if a.call_ref.is_none() && !a.interface.as_ref().is_some_and(|i| i.stub) {
                for (endpoint, outgoing) in [(&a.from, false), (&a.to, true)] {
                    if let Anchor::Boundary { port, .. } = endpoint.anchor {
                        if outgoing != (port == Port::Output) {
                            push("error", "boundary_direction", Some(a.id),
                                "Incoming boundary arrows use input, control or mechanism; outgoing boundary arrows use output".into());
                        }
                        if d.id != doc.root && !endpoint.tunneled && a.interface.is_none() {
                            push("error", "missing_boundary_interface", Some(a.id),
                                "An untunneled child boundary arrow must reference its parent box interface".into());
                        }
                    }
                }
                if let Some(interface) = &a.interface {
                    let parent = d
                        .parent_block
                        .and_then(|b| doc.diagram_of(b))
                        .and_then(|id| doc.diagrams.get(&id))
                        .and_then(|d| d.arrows.get(&interface.parent_arrow));
                    let consistent = parent.is_some_and(|parent| {
                        let output = matches!(a.to.anchor, Anchor::Boundary { .. });
                        let endpoint = if output { &parent.from } else { &parent.to };
                        matches!(endpoint.anchor, Anchor::Block { id, .. } if Some(id) == d.parent_block)
                            && parent.call_ref.is_none()
                            && (matches!(a.from.anchor, Anchor::Boundary { .. }) || output)
                    });
                    if !consistent {
                        push("error", "invalid_parent_interface", Some(a.id),
                            "A child interface must reference an arrow connected to its own parent box in the same incoming/outgoing direction".into());
                    }
                }
            }
            if let Anchor::Block { id, port, .. } = a.to.anchor
                && port != Port::Mechanism
                && !a.label.trim().is_empty()
            {
                *duplicates
                    .entry(format!("{id}:{port:?}:{}", a.label.trim()))
                    .or_default() += 1;
            }
        }
        for (key, n) in duplicates {
            if n > 1 {
                push(
                    "warning",
                    "duplicate_arrow",
                    None,
                    format!("The same labelled arrow enters a block more than once: {key}"),
                );
            }
        }
    }
    out
}

fn diagnostic(
    severity: &str,
    code: &str,
    diagram: Id,
    object: Option<Id>,
    message: String,
) -> Diagnostic {
    let reference = match code {
        "model_name" | "model_abbreviation" | "model_purpose" | "model_viewpoint" => Some("2.1.1"),
        "context_block_count" => Some("2.1.1"),
        "node_numbering" | "decomposition_block_count" => {
            Some("Annex B, B.2.2, definitions 13–14 (informative formalization)")
        }
        "missing_control" | "missing_output" => {
            Some("Annex B, B.2.1, definition 12 (informative formalization)")
        }
        "incomplete_interface" | "missing_boundary_interface" | "invalid_parent_interface" => {
            Some("Annex B, B.3, definition 20 (informative formalization)")
        }
        "boundary_direction" => Some(
            "Annex B, B.3, definition 20 (incoming/outgoing ICOM coding; informative formalization)",
        ),
        _ => None,
    };
    Diagnostic {
        severity: severity.into(),
        code: code.into(),
        diagram,
        object,
        message,
        source: if reference.is_some() {
            "ISO/IEC/IEEE 31320-1:2012"
        } else {
            "erlu"
        }
        .into(),
        reference: reference.map(str::to_owned),
    }
}
