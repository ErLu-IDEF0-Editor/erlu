use erlu_core::*;
use serde_json::json;

fn call(e: &mut Engine, action: &str, args: serde_json::Value) -> serde_json::Value {
    e.call(action, args, None).unwrap().result
}
fn boundary(port: Port) -> Endpoint {
    Endpoint::new(Anchor::Boundary { port, t: 0.5 })
}
fn endpoint(id: Id, port: Port) -> Endpoint {
    Endpoint::new(Anchor::Block { id, port, t: 0.5 })
}
fn arrow(e: &mut Engine, from: Endpoint, to: Endpoint, label: &str) -> Id {
    call(
        e,
        "arrow_create",
        json!({"from":from,"to":to,"label":label}),
    )["id"]
        .as_u64()
        .unwrap()
}
fn model() -> Engine {
    let mut e = Engine::default();
    call(
        &mut e,
        "document_metadata",
        json!({"patch": {
            "title":"Clock", "model_abbreviation":"CLK",
            "purpose":"Describe time signal generation", "viewpoint":"System designer"
        }}),
    );
    call(
        &mut e,
        "block_update",
        json!({"id":2,"title":"Generate time signals"}),
    );
    arrow(
        &mut e,
        boundary(Port::Control),
        endpoint(2, Port::Control),
        "Schedule",
    );
    arrow(
        &mut e,
        endpoint(2, Port::Output),
        boundary(Port::Output),
        "Time signals",
    );
    e
}
fn has(doc: &Document, code: &str) -> bool {
    validation::validate(doc).iter().any(|d| d.code == code)
}
fn decompose(e: &mut Engine, id: Id, count: usize) -> Vec<Id> {
    call(e, "decompose", json!({"block":id}));
    (0..count)
        .map(|n| {
            call(
                e,
                "block_create",
                json!({"pos":{"x":n as f64 * 60.,"y":0.},"title":"Perform work"}),
            )["id"]
                .as_u64()
                .unwrap()
        })
        .collect()
}

#[test]
fn control_and_output_suffice_without_input_or_mechanism() {
    assert!(validation::validate(&model().document).is_empty());
}

#[test]
fn input_does_not_satisfy_the_control_requirement() {
    let mut e = model();
    let control = e.document.diagrams[&1]
        .arrows
        .values()
        .find(|a| {
            matches!(
                a.to.anchor,
                Anchor::Block {
                    port: Port::Control,
                    ..
                }
            )
        })
        .unwrap()
        .id;
    call(&mut e, "objects_delete", json!({"ids":[control]}));
    arrow(
        &mut e,
        boundary(Port::Input),
        endpoint(2, Port::Input),
        "Raw data",
    );
    let diagnostics = validation::validate(&e.document);
    let missing = diagnostics
        .iter()
        .find(|d| d.code == "missing_control")
        .unwrap();
    assert_eq!(missing.severity, "error");
    assert_eq!(missing.object, Some(2));
    assert_eq!(missing.source, "ISO/IEC/IEEE 31320-1:2012");
    assert!(
        missing
            .reference
            .as_deref()
            .unwrap()
            .contains("informative")
    );
    assert!(!has(&e.document, "missing_output"));
}

#[test]
fn tunneled_control_is_still_a_control() {
    let mut e = model();
    let control = e.document.diagrams[&1]
        .arrows
        .values()
        .find(|a| {
            matches!(
                a.to.anchor,
                Anchor::Block {
                    port: Port::Control,
                    ..
                }
            )
        })
        .unwrap()
        .id;
    let mut to = endpoint(2, Port::Control);
    to.tunneled = true;
    call(&mut e, "arrow_update", json!({"id":control,"to":to}));
    assert!(!has(&e.document, "missing_control"));
}

#[test]
fn output_is_required_independently_of_control() {
    let mut e = model();
    let output = e.document.diagrams[&1]
        .arrows
        .values()
        .find(|a| {
            matches!(
                a.from.anchor,
                Anchor::Block {
                    port: Port::Output,
                    ..
                }
            )
        })
        .unwrap()
        .id;
    call(&mut e, "objects_delete", json!({"ids":[output]}));
    assert!(has(&e.document, "missing_output"));
    assert!(!has(&e.document, "missing_control"));
}

#[test]
fn context_requires_exactly_one_box_including_empty_documents() {
    let mut e = model();
    call(&mut e, "objects_delete", json!({"ids":[2]}));
    assert!(has(&e.document, "context_block_count"));
    let mut e = model();
    call(
        &mut e,
        "block_create",
        json!({"pos":{"x":80.,"y":0.},"title":"Extra work"}),
    );
    assert!(has(&e.document, "context_block_count"));
}

#[test]
fn decomposition_uses_the_international_two_to_nine_rule() {
    for count in [0, 1, 2, 3, 6, 8, 9, 10] {
        let mut e = model();
        decompose(&mut e, 2, count);
        assert_eq!(
            has(&e.document, "decomposition_block_count"),
            !(2..=9).contains(&count),
            "count={count}"
        );
    }
}

#[test]
fn all_four_model_statements_are_checked_and_editable_through_the_api() {
    for (field, code) in [
        ("title", "model_name"),
        ("model_abbreviation", "model_abbreviation"),
        ("purpose", "model_purpose"),
        ("viewpoint", "model_viewpoint"),
    ] {
        let mut e = model();
        call(&mut e, "document_metadata", json!({"patch":{field:" \n "}}));
        assert!(has(&e.document, code), "field={field}");
        call(&mut e, "undo", json!({}));
        assert!(!has(&e.document, code));
    }
}

#[test]
fn new_metadata_roundtrips_and_older_files_still_load() {
    let e = model();
    assert_eq!(read_document(&e.serialize_document()).unwrap(), e.document);
    let mut value = json!(e.document);
    for field in ["model_abbreviation", "purpose", "viewpoint"] {
        value["meta"].as_object_mut().unwrap().remove(field);
    }
    let old = read_document(&value.to_string()).unwrap();
    assert!(old.meta.purpose.is_empty());
    assert!(has(&old, "model_purpose"));
    assert!(
        Engine::new(old).is_ok(),
        "Semantic diagnostics must not prevent editing"
    );
}

#[test]
fn node_numbering_detects_duplicates_gaps_and_legacy_dotted_numbers() {
    let mut e = model();
    let children = decompose(&mut e, 2, 2);
    let child = e.view.diagram;
    assert!(!has(&e.document, "node_numbering"));
    for invalid in ["A1", "A3", "A0.2", "A10"] {
        e.document
            .diagrams
            .get_mut(&child)
            .unwrap()
            .blocks
            .get_mut(&children[1])
            .unwrap()
            .number = invalid.into();
        assert!(has(&e.document, "node_numbering"), "number={invalid}");
    }
}

#[test]
fn generated_descendant_numbers_and_swaps_follow_the_hierarchy() {
    let mut e = model();
    let children = decompose(&mut e, 2, 2);
    let grandchildren = decompose(&mut e, children[0], 2);
    assert_eq!(e.document.block(grandchildren[0]).unwrap().number, "A11");
    call(
        &mut e,
        "numbers_swap",
        json!({"first":children[0],"second":children[1]}),
    );
    assert_eq!(e.document.block(grandchildren[0]).unwrap().number, "A21");
    assert_eq!(e.document.node(e.document.root), "A-0");
    assert!(!has(&e.document, "node_numbering"));
    call(&mut e, "undo", json!({}));
    assert_eq!(e.document.block(grandchildren[0]).unwrap().number, "A11");
}

#[test]
fn deleting_a_sibling_renumbers_nodes_without_reassigning_ids() {
    let mut e = model();
    let children = decompose(&mut e, 2, 3);
    let grandchildren = decompose(&mut e, children[2], 2);
    call(&mut e, "objects_delete", json!({"ids":[children[1]]}));
    assert_eq!(e.document.block(children[2]).unwrap().number, "A2");
    assert_eq!(e.document.block(grandchildren[0]).unwrap().number, "A21");
    assert!(!has(&e.document, "node_numbering"));
    call(&mut e, "undo", json!({}));
    assert_eq!(e.document.block(children[2]).unwrap().number, "A3");
}

#[test]
fn matching_input_and_output_labels_are_an_editor_advisory() {
    let mut e = model();
    arrow(
        &mut e,
        boundary(Port::Input),
        endpoint(2, Port::Input),
        "Time signals",
    );
    let diagnostics = validation::validate(&e.document);
    let warning = diagnostics
        .iter()
        .find(|d| d.code == "identical_input_output")
        .unwrap();
    assert_eq!(warning.severity, "warning");
    assert_eq!(warning.source, "erlu");
    assert!(warning.reference.is_none());
}

#[test]
fn duplicate_labels_are_advisory_and_output_to_mechanism_is_allowed() {
    let mut e = model();
    let children = decompose(&mut e, 2, 2);
    arrow(
        &mut e,
        boundary(Port::Control),
        endpoint(children[0], Port::Control),
        "Policy",
    );
    arrow(
        &mut e,
        boundary(Port::Control),
        endpoint(children[0], Port::Control),
        "Policy",
    );
    arrow(
        &mut e,
        endpoint(children[0], Port::Output),
        endpoint(children[1], Port::Mechanism),
        "Prepared equipment",
    );
    let diagnostics = validation::validate(&e.document);
    let duplicate = diagnostics
        .iter()
        .find(|d| d.code == "duplicate_arrow")
        .unwrap();
    assert_eq!(duplicate.severity, "warning");
    assert_eq!(duplicate.source, "erlu");
    assert!(!diagnostics.iter().any(|d| d.code == "output_as_mechanism"));
}

#[test]
fn validation_action_returns_source_references_without_mutating_the_document() {
    let mut e = Engine::default();
    let original = e.document.clone();
    let revision = e.revision;
    let report = call(&mut e, "validate", json!({}));
    let item = report
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["code"] == "model_purpose")
        .unwrap();
    assert_eq!(item["source"], "ISO/IEC/IEEE 31320-1:2012");
    assert_eq!(item["reference"], "2.1.1");
    assert_eq!(e.document, original);
    assert_eq!(e.revision, revision);
}

#[test]
fn untunneled_child_boundary_requires_parent_mapping_but_role_changes_are_allowed() {
    let mut e = model();
    let children = decompose(&mut e, 2, 2);
    let child = e.view.diagram;
    let control = e.document.diagrams[&child]
        .arrows
        .values()
        .find(|a| {
            a.interface
                .as_ref()
                .is_some_and(|i| i.port == Port::Control)
        })
        .unwrap()
        .id;
    call(
        &mut e,
        "interface_connect",
        json!({"id":control,"endpoint":endpoint(children[0],Port::Mechanism)}),
    );
    assert!(!has(&e.document, "interface_role_change"));
    assert!(!has(&e.document, "invalid_parent_interface"));
    let unmapped = arrow(
        &mut e,
        boundary(Port::Input),
        endpoint(children[1], Port::Input),
        "Local data",
    );
    assert!(has(&e.document, "missing_boundary_interface"));
    let mut from = boundary(Port::Input);
    from.tunneled = true;
    call(&mut e, "arrow_update", json!({"id":unmapped,"from":from}));
    assert!(!has(&e.document, "missing_boundary_interface"));
}

#[test]
fn parent_interface_must_touch_the_parent_box_in_the_correct_direction() {
    let mut e = model();
    let children = decompose(&mut e, 2, 2);
    let child = e.view.diagram;
    let control = e.document.diagrams[&child]
        .arrows
        .values()
        .find(|a| {
            a.interface
                .as_ref()
                .is_some_and(|i| i.port == Port::Control)
        })
        .unwrap()
        .id;
    call(
        &mut e,
        "interface_connect",
        json!({"id":control,"endpoint":endpoint(children[0],Port::Control)}),
    );
    let parent_output = e.document.diagrams[&1]
        .arrows
        .values()
        .find(|a| {
            matches!(
                a.from.anchor,
                Anchor::Block {
                    port: Port::Output,
                    ..
                }
            )
        })
        .unwrap()
        .id;
    e.document
        .diagrams
        .get_mut(&child)
        .unwrap()
        .arrows
        .get_mut(&control)
        .unwrap()
        .interface
        .as_mut()
        .unwrap()
        .parent_arrow = parent_output;
    validate_structure(&e.document).unwrap();
    assert!(has(&e.document, "invalid_parent_interface"));
}

#[test]
fn outgoing_boundary_must_use_the_output_side() {
    let mut e = model();
    let output = e
        .document
        .diagrams
        .get_mut(&1)
        .unwrap()
        .arrows
        .values_mut()
        .find(|a| {
            matches!(
                a.from.anchor,
                Anchor::Block {
                    port: Port::Output,
                    ..
                }
            )
        })
        .unwrap();
    output.to = boundary(Port::Control);
    validate_structure(&e.document).unwrap();
    assert!(has(&e.document, "boundary_direction"));
}

#[test]
fn deleting_arrows_does_not_rewrite_legacy_node_numbers() {
    let mut e = model();
    let children = decompose(&mut e, 2, 2);
    let grandchildren = decompose(&mut e, children[0], 2);
    let child = e.view.diagram;
    e.document
        .diagrams
        .get_mut(&child)
        .unwrap()
        .blocks
        .get_mut(&grandchildren[0])
        .unwrap()
        .number = "A1.1".into();
    let id = arrow(
        &mut e,
        boundary(Port::Control),
        endpoint(grandchildren[0], Port::Control),
        "Policy",
    );
    call(&mut e, "objects_delete", json!({"ids":[id]}));
    assert_eq!(e.document.block(grandchildren[0]).unwrap().number, "A1.1");
}
