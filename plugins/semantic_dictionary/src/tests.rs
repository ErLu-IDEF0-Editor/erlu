use super::*;
fn context() -> Value {
    json!({"api_version":1,"revision":0,"arguments":{},"view":{"diagram":3,"selection":[20]},"document":{"extensions":{"other":{"version":1,"data":{"keep":true}}},"diagrams":{
        "1":{"parent_block":null,"blocks":{"2":{"number":"A0"}},"arrows":{"10":{"label":"req"}}},
        "3":{"parent_block":2,"blocks":{"4":{"number":"A1"}},"arrows":{
            "20":{"label":"req","interface":{"parent_arrow":10,"stub":false}},
            "21":{"label":"req","label_source":20},"22":{"label":"req","label_source":21},
            "40":{"label":"Unrelated"},"41":{"label":"Call","call_ref":4},"42":{"label":"Placeholder","interface":{"parent_arrow":10,"stub":true}}}},
        "5":{"parent_block":4,"blocks":{"6":{"number":"A11"}},"arrows":{"30":{"label":"req","interface":{"parent_arrow":20,"stub":false}}}}
    }}})
}
fn invoke(ctx: &mut Value, action: &str, args: Value) -> Result<Value, String> {
    ctx["arguments"] = args;
    let out: Value = serde_json::from_str(&run(action, &ctx.to_string())?).unwrap();
    let before = ctx["document"].clone();
    for cmd in out["commands"].as_array().unwrap() {
        match cmd["action"].as_str().unwrap() {
            "extension_set" => {
                ctx["document"]["extensions"][NS] = json!({"version":1,"data":cmd["args"]["data"]})
            }
            "arrow_update" => {
                let id = cmd["args"]["id"].as_u64().unwrap().to_string();
                for d in ctx["document"]["diagrams"]
                    .as_object_mut()
                    .unwrap()
                    .values_mut()
                {
                    if let Some(a) = d["arrows"].get_mut(&id) {
                        a["label"] = cmd["args"]["label"].clone();
                    }
                }
            }
            _ => panic!("Unexpected proposal"),
        }
    }
    if ctx["document"] != before {
        ctx["revision"] = json!(ctx["revision"].as_u64().unwrap() + 1);
    }
    Ok(out)
}
fn add(ctx: &mut Value, code: &str, term: &str) {
    invoke(
        ctx,
        "concept.add",
        json!({"code":code,"term":term,"definition":"Definition"}),
    )
    .unwrap();
}
fn bind(ctx: &mut Value, code: &str) {
    invoke(ctx, "binding.set", json!({"code":code})).unwrap();
}
fn query(ctx: &mut Value, action: &str, args: Value) -> Value {
    let before = ctx["document"].clone();
    let revision = ctx["revision"].clone();
    let out = invoke(ctx, action, args).unwrap();
    assert_eq!(out["commands"], json!([]));
    assert_eq!(ctx["document"], before);
    assert_eq!(ctx["revision"], revision);
    out["result"].clone()
}
#[test]
fn homonyms_are_explicit_and_spelling_comparison_collapses_case_and_whitespace() {
    let mut ctx = context();
    add(&mut ctx, "customer", "Заявка");
    add(&mut ctx, "supplier", "Заявка");
    invoke(
        &mut ctx,
        "alias.add",
        json!({"code":"CUSTOMER","alias":"  Purchase   request  "}),
    )
    .unwrap();
    invoke(
        &mut ctx,
        "alias.add",
        json!({"code":"supplier","alias":"purchase request"}),
    )
    .unwrap();
    let report = query(&mut ctx, "validate", json!({}));
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "ambiguous_spelling")
    );
    let found = query(&mut ctx, "search", json!({"query":" PURCHASE request "}));
    assert_eq!(found["matches"].as_array().unwrap().len(), 2);
    assert_eq!(
        ctx["document"]["extensions"][NS]["data"]["bindings"],
        json!({})
    );
    assert!(
        invoke(
            &mut ctx,
            "concept.add",
            json!({"code":" CUSTOMER ","term":"Duplicate"})
        )
        .is_err()
    );
}
#[test]
fn binding_tracks_the_entire_inherited_group_and_new_segments() {
    let mut ctx = context();
    add(&mut ctx, "request", "Customer request");
    bind(&mut ctx, "request");
    let uses = query(&mut ctx, "usage", json!({"code":"request"}));
    assert_eq!(uses["uses"].as_array().unwrap().len(), 5);
    ctx["document"]["diagrams"]["3"]["arrows"]["60"] = json!({"label":"req","label_source":10});
    assert_eq!(
        query(&mut ctx, "usage", json!({"code":"request"}))["uses"]
            .as_array()
            .unwrap()
            .len(),
        6
    );
    ctx["view"]["selection"] = json!([41, 42]);
    assert!(invoke(&mut ctx, "binding.set", json!({"code":"request"})).is_err());
}
#[test]
fn normalization_previews_all_uses_and_freezes_groups_instead_of_selection() {
    let mut ctx = context();
    add(&mut ctx, "request", "Customer request");
    bind(&mut ctx, "request");
    let preview = query(&mut ctx, "normalize.preview", json!({"scope":"selection"}));
    assert_eq!(preview["changes"][0]["root"], 10);
    assert_eq!(preview["changes"][0]["uses"].as_array().unwrap().len(), 5);
    ctx["view"]["selection"] = json!([40]);
    ctx["view"]["diagram"] = json!(1);
    let out = invoke(
        &mut ctx,
        "normalize.apply",
        preview["confirm_action"]["arguments"].clone(),
    )
    .unwrap();
    assert_eq!(out["commands"].as_array().unwrap().len(), 1);
    assert_eq!(out["commands"][0]["args"]["id"], 10);
    assert_eq!(
        ctx["document"]["diagrams"]["1"]["arrows"]["10"]["label"],
        "Customer request"
    );
    assert_eq!(
        ctx["document"]["diagrams"]["3"]["arrows"]["40"]["label"],
        "Unrelated"
    );
}
#[test]
fn stale_previews_and_conflicting_bindings_cannot_apply_partial_changes() {
    let mut ctx = context();
    add(&mut ctx, "request", "Request");
    add(&mut ctx, "other", "Other");
    bind(&mut ctx, "request");
    let preview = query(&mut ctx, "normalize.preview", json!({}));
    invoke(
        &mut ctx,
        "concept.update",
        json!({"code":"request","field":"owner","value":"Manager"}),
    )
    .unwrap();
    let before = ctx["document"].clone();
    assert!(
        invoke(
            &mut ctx,
            "normalize.apply",
            preview["confirm_action"]["arguments"].clone()
        )
        .unwrap_err()
        .contains("revision_conflict")
    );
    assert_eq!(ctx["document"], before);
    let id = Data::read(&ctx["document"]).unwrap().id("other").unwrap();
    ctx["document"]["extensions"][NS]["data"]["bindings"]["21"] = json!(id);
    assert!(invoke(&mut ctx, "normalize.preview", json!({})).is_err());
    assert!(invoke(&mut ctx, "binding.set", json!({"code":"request"})).is_err());
    invoke(
        &mut ctx,
        "binding.set",
        json!({"code":"request","replace":true}),
    )
    .unwrap();
    assert!(
        query(&mut ctx, "normalize.preview", json!({}))
            .get("confirm_action")
            .is_some()
    );
    assert!(invoke(&mut ctx, "normalize.apply", json!({"roots":[10]})).is_err());
}
#[test]
fn renaming_only_changes_the_dictionary_and_preserves_unknown_metadata() {
    let mut ctx = context();
    add(&mut ctx, "request", "Request");
    bind(&mut ctx, "request");
    let id = Data::read(&ctx["document"]).unwrap().id("request").unwrap();
    ctx["document"]["extensions"][NS]["data"]["future"] = json!({"keep":true});
    ctx["document"]["extensions"][NS]["data"]["concepts"][&id]["future_note"] = json!("keep");
    invoke(
        &mut ctx,
        "concept.update",
        json!({"code":"request","field":"term","value":"Canonical"}),
    )
    .unwrap();
    assert_eq!(
        ctx["document"]["diagrams"]["1"]["arrows"]["10"]["label"],
        "req"
    );
    assert_eq!(
        ctx["document"]["extensions"][NS]["data"]["bindings"]["20"],
        id
    );
    assert_eq!(
        ctx["document"]["extensions"][NS]["data"]["future"]["keep"],
        true
    );
    assert_eq!(
        ctx["document"]["extensions"][NS]["data"]["concepts"][&id]["future_note"],
        "keep"
    );
    assert_eq!(ctx["document"]["extensions"]["other"]["data"]["keep"], true);
    assert!(invoke(&mut ctx, "concept.remove", json!({"code":"request"})).is_err());
}
#[test]
fn import_is_portable_and_conflicts_block_the_entire_catalog() {
    let mut ctx = context();
    add(&mut ctx, "existing", "Existing");
    let catalog = json!({"version":1,"concepts":[{"code":"new","term":"New","definition":"New definition","id":"foreign_id","bindings":{"10":"foreign"}}],"bindings":{"999":"foreign"}});
    let preview = query(
        &mut ctx,
        "catalog.import_preview",
        json!({"catalog":catalog}),
    );
    invoke(
        &mut ctx,
        "catalog.import_apply",
        preview["confirm_action"]["arguments"].clone(),
    )
    .unwrap();
    let data = Data::read(&ctx["document"]).unwrap();
    assert!(data.bindings.is_empty());
    assert!(!data.concepts.contains_key("foreign_id"));
    let repeated = query(
        &mut ctx,
        "catalog.import_preview",
        json!({"catalog":catalog}),
    );
    assert_eq!(repeated["skipped"], 1);
    assert!(repeated.get("confirm_action").is_none());
    let conflict = json!({"version":1,"concepts":[{"code":"fresh","term":"Fresh"},{"code":"existing","term":"Overwritten"}]});
    let before = ctx["document"].clone();
    let preview = query(
        &mut ctx,
        "catalog.import_preview",
        json!({"catalog":conflict}),
    );
    assert_eq!(preview["conflicts"], json!(["existing"]));
    assert!(preview.get("confirm_action").is_none());
    let revision = ctx["revision"].clone();
    assert!(
        invoke(
            &mut ctx,
            "catalog.import_apply",
            json!({"catalog":conflict,"preview_revision":revision})
        )
        .is_err()
    );
    assert_eq!(ctx["document"], before);
}
#[test]
fn importing_homonyms_reports_them_and_stale_imports_are_rejected() {
    let mut ctx = context();
    add(&mut ctx, "one", "Request");
    let catalog = json!({"version":1,"concepts":[{"code":"two","term":"Request"}]});
    let preview = query(
        &mut ctx,
        "catalog.import_preview",
        json!({"catalog":catalog}),
    );
    assert_eq!(preview["warnings"].as_array().unwrap().len(), 1);
    add(&mut ctx, "third", "Third");
    assert!(
        invoke(
            &mut ctx,
            "catalog.import_apply",
            preview["confirm_action"]["arguments"].clone()
        )
        .unwrap_err()
        .contains("revision_conflict")
    );
    assert!(invoke(&mut ctx,"catalog.import_preview",json!({"catalog":{"version":1,"concepts":[{"code":"a","term":"A"},{"code":" A ","term":"B"}]}})).is_err());
}
#[test]
fn exports_roundtrip_without_bindings_and_escape_csv_and_markdown() {
    let mut ctx = context();
    add(&mut ctx, "request", "=Request | <b>");
    bind(&mut ctx, "request");
    invoke(
        &mut ctx,
        "concept.update",
        json!({"code":"request","field":"definition","value":"First\n\"second\""}),
    )
    .unwrap();
    let catalog = query(&mut ctx, "catalog.export", json!({"format":"json"}));
    let catalog: Value = serde_json::from_str(catalog["text"].as_str().unwrap()).unwrap();
    assert!(catalog.get("bindings").is_none());
    let mut fresh = context();
    let preview = query(
        &mut fresh,
        "catalog.import_preview",
        json!({"catalog":catalog}),
    );
    invoke(
        &mut fresh,
        "catalog.import_apply",
        preview["confirm_action"]["arguments"].clone(),
    )
    .unwrap();
    assert!(Data::read(&fresh["document"]).unwrap().bindings.is_empty());
    let csv = query(&mut ctx, "catalog.export", json!({"format":"csv"}));
    let text = csv["text"].as_str().unwrap();
    assert!(text.contains("\"'=Request"));
    assert!(text.contains("\"\"second\"\""));
    assert!(text.ends_with("\r\n"));
    let md = query(&mut ctx, "catalog.export", json!({"format":"markdown"}));
    let text = md["text"].as_str().unwrap();
    assert!(text.contains("\\|"));
    assert!(text.contains("&lt;b&gt;"));
    assert!(text.contains("<br>"));
}
#[test]
fn validation_scopes_and_pruning_do_not_erase_live_bindings() {
    let mut ctx = context();
    add(&mut ctx, "request", "Request");
    bind(&mut ctx, "request");
    let id = Data::read(&ctx["document"]).unwrap().id("request").unwrap();
    ctx["document"]["extensions"][NS]["data"]["bindings"]["999"] = json!(id);
    let report = query(&mut ctx, "validate", json!({}));
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "orphan_binding")
    );
    assert_eq!(
        query(
            &mut ctx,
            "usage",
            json!({"code":"request","scope":"diagram"})
        )["uses"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    invoke(&mut ctx, "data.prune", json!({})).unwrap();
    let data = Data::read(&ctx["document"]).unwrap();
    assert_eq!(data.bindings.len(), 5);
    invoke(&mut ctx, "binding.clear", json!({})).unwrap();
    invoke(&mut ctx, "concept.remove", json!({"code":"request"})).unwrap();
    assert!(Data::read(&ctx["document"]).unwrap().concepts.is_empty());
}
#[test]
fn unknown_versions_and_unknown_concept_references_are_explicit() {
    let mut ctx = context();
    add(&mut ctx, "request", "Request");
    bind(&mut ctx, "request");
    for value in ctx["document"]["extensions"][NS]["data"]["bindings"]
        .as_object_mut()
        .unwrap()
        .values_mut()
    {
        *value = json!("missing");
    }
    let report = query(&mut ctx, "validate", json!({}));
    assert_eq!(report["valid"], false);
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "unknown_concept")
    );
    ctx["document"]["extensions"][NS]["version"] = json!(2);
    let before = ctx["document"].clone();
    assert!(invoke(&mut ctx, "concept.add", json!({"code":"new","term":"New"})).is_err());
    assert_eq!(ctx["document"], before);
}
