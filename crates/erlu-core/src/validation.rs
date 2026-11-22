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
}
pub fn validate(doc: &Document) -> Vec<Diagnostic> {
    let mut out = vec![];
    for d in doc.diagrams.values() {
        let mut push = |severity: &str, code: &str, object: Option<Id>, message: String| {
            out.push(Diagnostic {
                severity: severity.into(),
                code: code.into(),
                diagram: d.id,
                object,
                message,
            })
        };
        if d.id == doc.root && d.blocks.len() > 1 {
            push(
                "error",
                "context_block_count",
                None,
                "The context diagram should contain one block".into(),
            );
        }
        if d.id != doc.root && !(3..=8).contains(&d.blocks.len()) {
            push(
                "error",
                "decomposition_block_count",
                None,
                "A decomposition should contain 3–8 blocks".into(),
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
        for b in d.blocks.values() {
            if b.title.trim().is_empty()
                || ["Function", "Функция", "Fonction"].contains(&b.title.trim())
            {
                push(
                    "error",
                    "block_name",
                    Some(b.id),
                    format!("{}: provide a meaningful function name", b.number),
                );
            }
            let incoming:Vec<_>=d.arrows.values().filter(|a|matches!(a.to.anchor,Anchor::Block{id,port:Port::Input|Port::Control,..}if id==b.id)).collect();
            let outgoing: Vec<_> = d
                .arrows
                .values()
                .filter(
                    |a| matches!(a.from.anchor,Anchor::Block{id,port:Port::Output,..}if id==b.id),
                )
                .collect();
            if incoming.is_empty() {
                push(
                    "error",
                    "missing_input_control",
                    Some(b.id),
                    format!("{}: at least one input or control is required", b.number),
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
                        "error",
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
            if let Anchor::Block { id, port, .. } = a.to.anchor
                && port != Port::Mechanism
                && !a.label.trim().is_empty()
            {
                *duplicates
                    .entry(format!("{id}:{port:?}:{}", a.label.trim()))
                    .or_default() += 1;
            }
            if d.id != doc.root
                && let (Anchor::Boundary { port: from, .. }, Anchor::Block { port: to, id, .. }) =
                    (&a.from.anchor, &a.to.anchor)
                && matches!(
                    (from, to),
                    (Port::Mechanism, Port::Input | Port::Control)
                        | (Port::Control | Port::Input, Port::Mechanism)
                )
            {
                push(
                    "warning",
                    "interface_role_change",
                    Some(*id),
                    "A boundary flow changes its ICOM role".into(),
                );
            }
            if matches!(
                (&a.from.anchor, &a.to.anchor),
                (
                    Anchor::Block {
                        port: Port::Output,
                        ..
                    },
                    Anchor::Block {
                        port: Port::Mechanism,
                        ..
                    }
                )
            ) {
                push(
                    "warning",
                    "output_as_mechanism",
                    Some(a.id),
                    "A block output acts as another block's mechanism".into(),
                );
            }
        }
        for (key, n) in duplicates {
            if n > 1 {
                push(
                    "error",
                    "duplicate_arrow",
                    None,
                    format!("The same labelled arrow enters a block more than once: {key}"),
                );
            }
        }
    }
    out
}
