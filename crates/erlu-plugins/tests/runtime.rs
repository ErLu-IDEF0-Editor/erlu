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
