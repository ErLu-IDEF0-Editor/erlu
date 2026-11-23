use erlu_core::*;
use erlu_plugins::*;
use serde_json::json;
#[test]
fn colors_component_is_portable_and_transactional() {
    let host = PluginHost::default();
    assert!(
        host.manifests().iter().any(|m| m.id == "colors"),
        "Build the component first: npm --prefix web run plugins"
    );
    let mut state = Engine::default();
    state
        .call("selection_set", json!({"ids":[2]}), None)
        .unwrap();
    let commands = host
        .invoke("colors", "colors.set", json!({"color":"#abcdef"}), &state)
        .unwrap();
    assert_eq!(state.document.block(2).unwrap().color, None);
    state
        .dispatch(Command::Transaction { commands }, Some(0))
        .unwrap();
    assert_eq!(
        state.document.block(2).unwrap().color.as_deref(),
        Some("#abcdef")
    );
    state.dispatch(Command::Undo {}, None).unwrap();
    assert_eq!(state.document.block(2).unwrap().color, None);
}
#[test]
fn runaway_plugin_is_interrupted_without_edits() {
    let dir = tempfile::tempdir().unwrap();
    let module=wat::parse_str(r#"(component
  (core module $m
   (memory (export "memory") 1)
   (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 1024)
   (func (export "run") (param i32 i32 i32 i32) (result i32) (loop $spin (br $spin)) i32.const 0))
  (core instance $i (instantiate $m))
  (func (export "run") (param "action" string) (param "context" string) (result (result string (error string)))
    (canon lift (core func $i "run") (memory (core memory $i "memory")) (realloc (core func $i "realloc")))))"#).unwrap();
    std::fs::write(dir.path().join("spin.wasm"), module).unwrap();
    std::fs::write(dir.path().join("plugin.json"),json!({"id":"spin","api_version":1,"version":"0.1.0","component":"spin.wasm","actions":[{"name":"spin","title":"Spin","kind":"command","input_schema":{"type":"object"}}]}).to_string()).unwrap();
    let mut host = PluginHost::default();
    host.load(&dir.path().join("plugin.json")).unwrap();
    let state = Engine::default();
    let doc = state.document.clone();
    assert_eq!(
        host.invoke("spin", "spin", json!({}), &state)
            .unwrap_err()
            .code,
        "plugin_runtime"
    );
    assert_eq!(state.document, doc);
}
#[test]
fn invalid_manifest_is_rejected() {
    let mut m: Manifest =
        serde_json::from_str(include_str!("../../../plugins/colors/plugin.json")).unwrap();
    m.api_version = 2;
    assert!(validate_manifest(&m).is_err());
}

fn responsibility_action(
    host: &PluginHost,
    state: &mut Engine,
    action: &str,
    args: serde_json::Value,
) -> serde_json::Value {
    let output = host
        .invoke_output("responsibility_map", action, args, state)
        .unwrap();
    state
        .dispatch(
            Command::Transaction {
                commands: output.commands,
            },
            Some(state.revision),
        )
        .unwrap();
    output.result
}
#[test]
fn responsibility_component_persists_raci_and_is_undoable() {
    let host = PluginHost::default();
    assert!(
        host.manifests()
            .iter()
            .any(|m| m.id == "responsibility_map"),
        "Build both components: npm --prefix web run plugins"
    );
    let mut state = Engine::default();
    state
        .call(
            "extension_set",
            json!({"namespace":"other","version":1,"data":{"keep":true}}),
            None,
        )
        .unwrap();
    state
        .call("selection_set", json!({"ids":[2]}), None)
        .unwrap();
    responsibility_action(
        &host,
        &mut state,
        "role.add",
        json!({"name":"Операционный отдел"}),
    );
    responsibility_action(
        &host,
        &mut state,
        "assignment.set",
        json!({"role":"Операционный отдел","responsibility":"R"}),
    );
    let before = state.document.clone();
    let output = host
        .invoke_output(
            "responsibility_map",
            "assignment.set",
            json!({"role":"Операционный отдел","responsibility":"A"}),
            &state,
        )
        .unwrap();
    assert_eq!(
        state.document, before,
        "Plugin proposals never mutate the live document"
    );
    state
        .dispatch(
            Command::Transaction {
                commands: output.commands,
            },
            Some(state.revision),
        )
        .unwrap();
    let revision = state.revision;
    let report = host
        .invoke_output("responsibility_map", "validate", json!({}), &state)
        .unwrap();
    assert!(report.commands.is_empty());
    assert_eq!(report.result["valid"], true);
    assert_eq!(state.revision, revision);
    let matrix = host
        .invoke_output(
            "responsibility_map",
            "matrix.export",
            json!({"format":"csv"}),
            &state,
        )
        .unwrap();
    assert!(matrix.commands.is_empty());
    assert!(matrix.result["text"].as_str().unwrap().contains("R/A"));
    assert_eq!(matrix.result["filename"], "responsibility-map.csv");
    let reopened = Engine::new(read_document(&state.serialize_document()).unwrap()).unwrap();
    assert_eq!(
        host.invoke_output("responsibility_map", "validate", json!({}), &reopened)
            .unwrap()
            .result["valid"],
        true
    );
    assert_eq!(reopened.document.extensions["other"]["data"]["keep"], true);
    state.dispatch(Command::Undo {}, None).unwrap();
    assert_eq!(state.document, before);
    state.dispatch(Command::Redo {}, None).unwrap();
    assert_eq!(
        host.invoke_output("responsibility_map", "validate", json!({}), &state)
            .unwrap()
            .result["valid"],
        true
    );
}
#[test]
fn responsibility_component_errors_are_atomic_and_revision_checked() {
    let host = PluginHost::default();
    let mut state = Engine::default();
    responsibility_action(&host, &mut state, "role.add", json!({"name":"Owner"}));
    let before = state.document.clone();
    assert!(
        host.invoke_output(
            "responsibility_map",
            "assignment.set",
            json!({"role":"Owner","responsibility":"A"}),
            &state
        )
        .is_err()
    );
    assert_eq!(
        state.document, before,
        "Missing selection must not change extension data"
    );
    state
        .call("selection_set", json!({"ids":[2]}), None)
        .unwrap();
    let revision = state.revision;
    let output = host
        .invoke_output(
            "responsibility_map",
            "assignment.set",
            json!({"role":"Owner","responsibility":"A"}),
            &state,
        )
        .unwrap();
    state
        .call(
            "block_update",
            json!({"id":2,"title":"Changed concurrently"}),
            None,
        )
        .unwrap();
    assert_eq!(
        state
            .dispatch(
                Command::Transaction {
                    commands: output.commands
                },
                Some(revision)
            )
            .unwrap_err()
            .code,
        "revision_conflict"
    );
    assert_eq!(
        state.document.extensions["responsibility_map"]["data"]["assignments"],
        json!({})
    );
}

fn dictionary_action(
    host: &PluginHost,
    state: &mut Engine,
    action: &str,
    args: serde_json::Value,
) -> serde_json::Value {
    let output = host
        .invoke_output("semantic_dictionary", action, args, state)
        .unwrap();
    state
        .dispatch(
            Command::Transaction {
                commands: output.commands,
            },
            Some(state.revision),
        )
        .unwrap();
    output.result
}
#[test]
fn dictionary_component_previews_normalizes_and_roundtrips() {
    let host = PluginHost::default();
    let mut state = Engine::default();
    assert!(
        host.manifests()
            .iter()
            .any(|m| m.id == "semantic_dictionary"),
        "Build plugins first"
    );
    let arrow=state.call("arrow_create",json!({"from":{"anchor":{"kind":"boundary","port":"input","t":0.5}},"to":{"anchor":{"kind":"block","id":2,"port":"input","t":0.5}},"label":"req"}),None).unwrap().result["id"].as_u64().unwrap();
    state
        .call("selection_set", json!({"ids":[arrow]}), None)
        .unwrap();
    dictionary_action(
        &host,
        &mut state,
        "concept.add",
        json!({"code":"request","term":"Заявка клиента","definition":"Запрос клиента"}),
    );
    dictionary_action(
        &host,
        &mut state,
        "alias.add",
        json!({"code":"request","alias":"req"}),
    );
    dictionary_action(&host, &mut state, "binding.set", json!({"code":"request"}));
    let before = state.document.clone();
    let revision = state.revision;
    let preview = host
        .invoke_output(
            "semantic_dictionary",
            "normalize.preview",
            json!({"scope":"selection"}),
            &state,
        )
        .unwrap();
    assert!(preview.commands.is_empty());
    assert_eq!(state.document, before);
    let confirmation: PluginConfirmation =
        serde_json::from_value(preview.result["confirm_action"].clone()).unwrap();
    assert_eq!(confirmation.expected_revision, revision);
    dictionary_action(
        &host,
        &mut state,
        "normalize.apply",
        confirmation.arguments.clone(),
    );
    assert_eq!(
        state.document.diagrams[&1].arrows[&arrow].label,
        "Заявка клиента"
    );
    state.dispatch(Command::Undo {}, None).unwrap();
    assert_eq!(state.document, before);
    assert!(
        host.invoke_output(
            "semantic_dictionary",
            "normalize.apply",
            confirmation.arguments,
            &state
        )
        .is_err()
    );
    state.dispatch(Command::Redo {}, None).unwrap();
    let reopened = Engine::new(read_document(&state.serialize_document()).unwrap()).unwrap();
    let exported = host
        .invoke_output(
            "semantic_dictionary",
            "catalog.export",
            json!({"format":"json"}),
            &reopened,
        )
        .unwrap();
    assert!(exported.commands.is_empty());
    let catalog: serde_json::Value =
        serde_json::from_str(exported.result["text"].as_str().unwrap()).unwrap();
    let mut fresh = Engine::default();
    let preview = host
        .invoke_output(
            "semantic_dictionary",
            "catalog.import_preview",
            json!({"catalog":catalog}),
            &fresh,
        )
        .unwrap();
    dictionary_action(
        &host,
        &mut fresh,
        "catalog.import_apply",
        preview.result["confirm_action"]["arguments"].clone(),
    );
    assert_eq!(
        fresh.document.extensions["semantic_dictionary"]["data"]["bindings"],
        json!({})
    );
}
#[test]
fn confirmation_cannot_invoke_another_plugin_or_a_read_only_action() {
    let manifest: Manifest = serde_json::from_str(include_str!(
        "../../../plugins/semantic_dictionary/plugin.json"
    ))
    .unwrap();
    let valid = json!({"name":"plugin_semantic_dictionary_normalize_apply","arguments":{"roots":[10],"preview_revision":4},"expected_revision":4});
    validate_confirmation(&manifest, &json!({"confirm_action":valid}), 4).unwrap();
    for field in [
        "plugin_colors_colors_set",
        "document_open",
        "plugin_semantic_dictionary_normalize_preview",
    ] {
        let mut bad = valid.clone();
        bad["name"] = json!(field);
        assert!(validate_confirmation(&manifest, &json!({"confirm_action":bad}), 4).is_err());
    }
    assert!(validate_confirmation(&manifest, &json!({"confirm_action":valid}), 5).is_err());
}
