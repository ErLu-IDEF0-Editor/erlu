use erlu_core::{Command, Engine, dfd};
use serde_json::{Value, json};

fn invoke(engine: &mut Engine, action: &str, args: Value) -> Value {
    engine.call(action, args, None).unwrap().result
}

fn edit(engine: &mut Engine, value: Value) -> Value {
    invoke(engine, "dfd_edit", json!({"edit":value}))
}

fn issues(engine: &mut Engine) -> Vec<String> {
    invoke(engine, "validate", json!({}))
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["code"].as_str().unwrap().into())
        .collect()
}

fn example() -> Engine {
    Engine::from_file(include_str!("../../../examples/order-fulfilment.dfd")).unwrap()
}

#[test]
fn balances_composite_interfaces_and_allows_full_inheritance() {
    let mut engine = example();
    assert!(issues(&mut engine).is_empty());
    invoke(&mut engine, "objects_delete", json!({"ids":[16]}));
    assert!(issues(&mut engine).contains(&"dfd_missing_component".into()));

    edit(
        &mut engine,
        json!({"operation":"flow_update","id":15,"flow":{
            "from":{"kind":"boundary","parent_flow":4},"to":{"kind":"node","id":8},
            "label":"","data":null
        }}),
    );
    assert!(issues(&mut engine).is_empty());

    edit(
        &mut engine,
        json!({"operation":"flow_update","id":15,"flow":{
            "from":{"kind":"boundary","parent_flow":4},"to":{"kind":"node","id":8},
            "label":"Wrong component","data":13
        }}),
    );
    let codes = issues(&mut engine);
    assert!(codes.contains(&"dfd_extra_component".into()));
    assert!(codes.contains(&"dfd_missing_component".into()));
    invoke(&mut engine, "undo", json!({}));
    assert!(issues(&mut engine).is_empty());

    edit(&mut engine, json!({"operation":"decompose","id":8}));
    let child = engine.dfd_document().unwrap().order()[2];
    let doc = engine.dfd_document().unwrap();
    assert_eq!(doc.interfaces(child).len(), 2);
    assert_eq!(
        doc.effective_data(7, &doc.diagrams[&7].flows[&15]),
        Some(12)
    );
}

#[test]
fn rejects_invalid_references_and_rolls_back_mixed_commands() {
    let mut engine = example();
    let initial = engine.serialize_document();
    let revision = engine.revision;
    let error = engine.call("transaction", json!({"commands":[
        {"action":"dfd_edit","args":{"edit":{"operation":"style","style":"yourdon_demarco"}}},
        {"action":"block_create","args":{"pos":{"x":0,"y":0}}}
    ]}), Some(revision)).unwrap_err();
    assert_eq!(error.code, "wrong_notation");
    assert_eq!(engine.serialize_document(), initial);
    assert_eq!(engine.revision, revision);

    for change in [
        json!({"operation":"flow_create","diagram":7,"flow":{
            "from":{"kind":"node","id":10},"to":{"kind":"boundary","parent_flow":5},"label":"","data":null
        }}),
        json!({"operation":"flow_create","diagram":7,"flow":{
            "from":{"kind":"boundary","parent_flow":5},"to":{"kind":"node","id":8},"label":"","data":null
        }}),
        json!({"operation":"definition_put","id":11,"definition":{"name":"Customer details","description":"","components":[12]}}),
        json!({"operation":"definition_delete","id":12}),
    ] {
        let error = engine
            .call("dfd_edit", json!({"edit":change}), None)
            .unwrap_err();
        assert_eq!(error.code, "invalid_dfd");
        assert_eq!(engine.serialize_document(), initial);
    }
    let error = engine
        .dispatch(Command::Undo {}, Some(revision + 1))
        .unwrap_err();
    assert_eq!(error.code, "revision_conflict");
}

#[test]
fn style_exports_round_trips_and_recovery_share_the_same_document() {
    let mut engine = example();
    let original = engine.dfd_document().unwrap().clone();
    let svg = invoke(&mut engine, "export_svg", json!({}));
    assert!(svg["text"].as_str().unwrap().contains("rx=\"2\""));
    edit(
        &mut engine,
        json!({"operation":"style","style":"yourdon_demarco"}),
    );
    let svg = invoke(&mut engine, "export_svg", json!({}));
    assert!(svg["text"].as_str().unwrap().contains("<circle"));
    let pdf = invoke(
        &mut engine,
        "export_pdf",
        json!({"all":true,"number_pages":true}),
    );
    let bytes: Vec<u8> = serde_json::from_value(pdf["bytes"].clone()).unwrap();
    assert!(bytes.starts_with(b"%PDF"));
    let markdown = invoke(&mut engine, "export_markdown", json!({}));
    assert!(
        markdown["text"]
            .as_str()
            .unwrap()
            .contains("Customer details, Items")
    );
    assert!(issues(&mut engine).is_empty());
    assert_eq!(engine.dfd_document().unwrap().diagrams, original.diagrams);
    let roundtrip = Engine::from_file(&engine.serialize_document()).unwrap();
    assert_eq!(roundtrip.dfd_document(), engine.dfd_document());
    let mut recovered = Engine::default();
    recovered.restore_recovery(&engine.recovery_json()).unwrap();
    assert_eq!(recovered.dfd_document(), engine.dfd_document());
    assert!(recovered.dirty());
    invoke(&mut engine, "undo", json!({}));
    assert_eq!(engine.dfd_document(), Some(&original));
    assert!(!engine.dirty());
    invoke(&mut engine, "redo", json!({}));
    assert_eq!(
        engine.dfd_document().unwrap().style,
        dfd::Style::YourdonDemarco
    );

    let bad = engine
        .serialize_document()
        .replace("\"erlu-dfd\"", "\"unknown-format\"");
    let revision = engine.revision;
    assert!(
        engine
            .call("document_open", json!({"json":bad,"discard":true}), None)
            .is_err()
    );
    assert_eq!(engine.revision, revision);
    invoke(&mut engine, "document_new", json!({"discard":true}));
    assert!(engine.dfd_document().is_none());
    assert_eq!(engine.revision, revision + 1);
}
