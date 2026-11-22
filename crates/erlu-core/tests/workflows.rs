use erlu_core::*;
use serde_json::{Value, json};
fn call(e: &mut Engine, name: &str, args: Value) -> Response {
    e.call(name, args, None).unwrap()
}
fn block(e: &mut Engine, x: f64, y: f64) -> Id {
    call(
        e,
        "block_create",
        json!({"pos":{"x":x,"y":y},"title":"Work"}),
    )
    .result["id"]
        .as_u64()
        .unwrap()
}
fn ep(id: Id, port: Port) -> Endpoint {
    Endpoint::new(Anchor::Block { id, port, t: 0.5 })
}
fn edge(port: Port) -> Endpoint {
    Endpoint::new(Anchor::Boundary { port, t: 0.5 })
}
#[test]
fn transactions_rollback_all_and_revisions_detect_conflicts() {
    let mut e = Engine::default();
    let original = e.document.clone();
    let result = e.dispatch(
        Command::Transaction {
            commands: vec![
                Command::BlockCreate {
                    diagram: None,
                    pos: Point::new(0., 0.),
                    title: "A".into(),
                },
                Command::BlockUpdate {
                    id: 999,
                    title: None,
                    price: Some(3.),
                    clear_price: false,
                },
            ],
        },
        Some(0),
    );
    assert!(result.is_err());
    assert_eq!(e.document, original);
    assert_eq!(e.revision, 0);
    block(&mut e, 0., 0.);
    let snapshot = e.document.clone();
    assert_eq!(
        e.call("block_create", json!({"pos":{"x":1,"y":1}}), Some(0))
            .unwrap_err()
            .code,
        "revision_conflict"
    );
    assert_eq!(e.document, snapshot);
}
#[test]
fn history_survives_navigation_and_groups_transactions() {
    let mut e = Engine::default();
    let child = call(&mut e, "decompose", json!({"block":2})).result["diagram"]
        .as_u64()
        .unwrap();
    let a = block(&mut e, -60., 0.);
    let b = block(&mut e, 20., 0.);
    let initial = e.document.clone();
    call(
        &mut e,
        "transaction",
        json!({"commands":[{"action":"block_update","args":{"id":a,"price":10}},{"action":"block_update","args":{"id":b,"price":20}}]}),
    );
    call(&mut e, "navigate_up", json!({}));
    call(&mut e, "undo", json!({}));
    assert_eq!(e.document, initial);
    assert_eq!(e.view.diagram, 1);
    call(&mut e, "redo", json!({}));
    assert_eq!(e.document.diagrams[&child].blocks[&a].price, Some(10.));
    call(&mut e, "calculate_prices", json!({}));
    assert_eq!(e.document.block(2).unwrap().price, Some(30.));
}
#[test]
fn interfaces_inherit_labels_tunnels_and_restore_after_delete() {
    let mut e = Engine::default();
    let incoming = call(
        &mut e,
        "arrow_create",
        json!({"from":edge(Port::Input),"to":ep(2,Port::Input),"label":"Material"}),
    )
    .result["id"]
        .as_u64()
        .unwrap();
    call(&mut e, "decompose", json!({"block":2}));
    let child = e.view.diagram;
    let stub = e.document.diagrams[&child]
        .arrows
        .values()
        .find(|a| a.interface.is_some())
        .unwrap()
        .id;
    let b = block(&mut e, 0., 0.);
    call(
        &mut e,
        "interface_connect",
        json!({"id":stub,"endpoint":ep(b,Port::Input)}),
    );
    let mut from = e.document.diagrams[&child].arrows[&stub].from.clone();
    from.tunneled = true;
    call(&mut e, "arrow_update", json!({"id":stub,"from":from}));
    assert!(e.document.diagrams[&1].arrows[&incoming].to.tunneled);
    call(&mut e, "objects_delete", json!({"ids":[b,stub]}));
    assert!(
        e.document.diagrams[&child]
            .arrows
            .values()
            .any(|a| a.interface.as_ref().is_some_and(|i| i.stub))
    );
}
#[test]
fn unknown_optional_data_roundtrips_and_required_is_rejected() {
    let e = Engine::default();
    let mut v = json!(e.document);
    v["format_version"]["minor"] = 99.into();
    v["features"]["optional"] = json!(["future.notes"]);
    v["future_top"] = json!({"unchanged":[1,2,3]});
    v["diagrams"]["1"]["blocks"]["2"]["future_block"] = json!({"x":42});
    v["extensions"]["absent_plugin"] = json!({"version":8,"data":{"x":true}});
    let loaded = read_document(&v.to_string()).unwrap();
    assert_eq!(json!(loaded), v);
    v["features"]["required"]
        .as_array_mut()
        .unwrap()
        .push(json!("future.required"));
    assert_eq!(
        read_document(&v.to_string()).unwrap_err().code,
        "unsupported_feature"
    );
}
#[test]
fn malformed_file_does_not_replace_document_or_history() {
    let mut e = Engine::default();
    block(&mut e, 0., 0.);
    let doc = e.document.clone();
    assert!(
        e.call(
            "document_open",
            json!({"json":"{\"diagram\":{}}","discard":true}),
            None
        )
        .is_err()
    );
    assert_eq!(e.document, doc);
    assert!(e.dirty());
    call(&mut e, "undo", json!({}));
    assert_eq!(e.document, Document::default());
}
#[test]
fn branch_delete_and_label_propagation_preserve_integrity() {
    let mut e = Engine::default();
    let b = block(&mut e, 60., 0.);
    let id = call(
        &mut e,
        "arrow_create",
        json!({"from":ep(2,Port::Output),"to":ep(b,Port::Input),"label":"Output"}),
    )
    .result["id"]
        .as_u64()
        .unwrap();
    let branch = call(
        &mut e,
        "arrow_branch",
        json!({"id":id,"point":{"x":40,"y":0},"target":edge(Port::Output)}),
    );
    let forward = branch.result["forward"].as_u64().unwrap();
    call(&mut e, "arrow_update", json!({"id":id,"label":"Updated"}));
    assert_eq!(e.document.diagrams[&1].arrows[&forward].label, "Updated");
    call(&mut e, "objects_delete", json!({"ids":[2,id]}));
    validate_structure(&e.document).unwrap();
}
#[test]
fn invalid_ports_and_prices_rollback() {
    let mut e = Engine::default();
    assert!(
        e.call(
            "arrow_create",
            json!({"from":ep(2,Port::Input),"to":ep(2,Port::Output)}),
            None
        )
        .is_err()
    );
    assert!(
        e.call("block_update", json!({"id":2,"price":-1}), None)
            .is_err()
    );
    assert_eq!(e.document, Document::default());
}
#[test]
fn orthogonal_route_avoids_obstacle() {
    let mut e = Engine::default();
    call(
        &mut e,
        "objects_move",
        json!({"positions":{"2":{"x":-115,"y":-13.25}}}),
    );
    let target = block(&mut e, 60., -13.25);
    let obstacle = block(&mut e, -25., -13.25);
    let id = call(
        &mut e,
        "arrow_create",
        json!({"from":ep(2,Port::Output),"to":ep(target,Port::Input)}),
    )
    .result["id"]
        .as_u64()
        .unwrap();
    let path = render::route(&e.document, 1, &e.document.diagrams[&1].arrows[&id]);
    let o = e.document.block(obstacle).unwrap();
    for w in path.windows(2) {
        assert!((w[0].x - w[1].x).abs() < 1e-6 || (w[0].y - w[1].y).abs() < 1e-6);
        let mid = Point::new((w[0].x + w[1].x) / 2., (w[0].y + w[1].y) / 2.);
        assert!(
            !(mid.x > o.pos.x
                && mid.x < o.pos.x + 53.
                && mid.y > o.pos.y
                && mid.y < o.pos.y + 26.5)
        );
    }
}
#[test]
fn registry_covers_decode_and_pdf_has_pages() {
    let specs = action_specs();
    assert!(specs.len() >= 30);
    assert!(specs.iter().any(|s| s.name == "arrow_branch"));
    let mut e = Engine::default();
    call(&mut e, "decompose", json!({"block":2}));
    let r = call(
        &mut e,
        "export_pdf",
        json!({"all":true,"number_pages":true,"light":true}),
    );
    let bytes: Vec<u8> = serde_json::from_value(r.result["bytes"].clone()).unwrap();
    assert!(bytes.starts_with(b"%PDF"));
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("/Count 2"));
    let svg = render::svg(&e.document, 1, false, None);
    assert!(svg.contains("DejaVu Sans"));
}
#[test]
fn feature_presence_is_enforced_and_cycles_rejected() {
    let mut doc = Document::default();
    doc.features.required.remove("abc.costs");
    doc.diagrams
        .get_mut(&1)
        .unwrap()
        .blocks
        .get_mut(&2)
        .unwrap()
        .price = Some(1.);
    assert!(validate_structure(&doc).is_err());
    let mut doc = Document::default();
    doc.diagrams
        .get_mut(&1)
        .unwrap()
        .blocks
        .get_mut(&2)
        .unwrap()
        .child = Some(1);
    assert!(validate_structure(&doc).is_err());
}

#[test]
fn future_nested_fields_survive_edits_without_resurrecting_deleted_entities() {
    let mut e = Engine::default();
    let b = block(&mut e, 60., 0.);
    let id = call(
        &mut e,
        "arrow_create",
        json!({"from":ep(2,Port::Output),"to":ep(b,Port::Input)}),
    )
    .result["id"]
        .as_u64()
        .unwrap();
    let mut v = json!(e.document);
    v["diagrams"]["1"]["blocks"]["2"]["pos"]["future_unit"] = json!("mm");
    v["diagrams"]["1"]["arrows"][id.to_string()]["from"]["future_endpoint"] = json!({"keep":true});
    let doc = read_document(&v.to_string()).unwrap();
    let mut e = Engine::new(doc).unwrap();
    call(
        &mut e,
        "objects_move",
        json!({"positions":{"2":{"x":-80,"y":0}}}),
    );
    let out = json!(e.document);
    assert_eq!(
        out["diagrams"]["1"]["blocks"]["2"]["pos"]["future_unit"],
        "mm"
    );
    assert_eq!(
        out["diagrams"]["1"]["arrows"][id.to_string()]["from"]["future_endpoint"],
        json!({"keep":true})
    );
    call(&mut e, "objects_delete", json!({"ids":[b]}));
    assert!(
        json!(e.document)["diagrams"]["1"]["blocks"]
            .get(b.to_string())
            .is_none()
    );
}
#[test]
fn recovery_keeps_saved_baseline_and_lifecycle_invalidates_pending_edits() {
    let mut e = Engine::default();
    block(&mut e, 20., 0.);
    let snapshot = e.recovery_json();
    let mut restored = Engine::default();
    restored.restore_recovery(&snapshot).unwrap();
    assert!(restored.dirty());
    assert_eq!(restored.document, e.document);
    let mut clean = Engine::default();
    let old = clean.revision;
    call(&mut clean, "document_new", json!({}));
    assert!(clean.revision > old);
}

#[test]
fn ids_are_not_reused_after_delete_or_undo_and_survive_save() {
    let mut e = Engine::default();
    let first = block(&mut e, 0., 0.);
    call(&mut e, "undo", json!({}));
    let second = block(&mut e, 0., 0.);
    assert!(second > first);
    call(&mut e, "objects_delete", json!({"ids":[second]}));
    let json = e.serialize_document();
    let mut reopened = Engine::new(read_document(&json).unwrap()).unwrap();
    let third = block(&mut reopened, 0., 0.);
    assert!(third > second);
}

#[test]
fn migration_registry_is_contiguous_and_feature_registry_matches_schema() {
    let mut expected = 0;
    for step in format::MIGRATIONS {
        assert_eq!(step.from_minor, expected);
        expected += 1;
    }
    assert_eq!(expected, format::CURRENT_MINOR);
    let registry: std::collections::BTreeSet<_> =
        format::FEATURE_REGISTRY.iter().map(|f| f.name).collect();
    assert_eq!(registry, FEATURES.iter().copied().collect());
}

#[test]
fn pdf_normalized_xobject_is_scaled_to_physical_page() {
    let doc = Document::default();
    let pdf = render::pdf(&doc, &[doc.root], false, false).unwrap();
    let text = String::from_utf8_lossy(&pdf);
    let matrices = text
        .lines()
        .filter(|line| line.ends_with(" cm"))
        .filter_map(|line| {
            let values = line
                .split_whitespace()
                .take(6)
                .map(str::parse::<f64>)
                .collect::<std::result::Result<Vec<_>, _>>()
                .ok()?;
            (values.len() == 6).then_some(values)
        })
        .collect::<Vec<_>>();
    let p = doc.page();
    assert!(matrices.iter().any(|m|(m[0]-p.x*72./25.4).abs()<0.1&&(m[3]-p.y*72./25.4).abs()<0.1),"PDF must scale svg2pdf's normalized unit-square XObject to the physical page");
}
