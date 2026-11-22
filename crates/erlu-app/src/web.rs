use crate::app::{ErluApp, SharedEngine};
use erlu_core::*;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace=erluRuntime,js_name=checkpoint)]
    fn js_checkpoint(json: &str);
    #[wasm_bindgen(js_namespace=erluRuntime,js_name=configureAI)]
    fn configure_ai(enabled: bool, origin: &str);
    #[wasm_bindgen(js_namespace=erluRuntime,js_name=loadPlugin)]
    fn js_load_plugin();
    #[wasm_bindgen(js_namespace=erluRuntime,js_name=invokePluginUI)]
    fn js_invoke_plugin(id: &str, action: &str, args: &str, context: &str);
    #[wasm_bindgen(js_namespace=erluRuntime,js_name=pluginSpecs)]
    fn js_plugin_specs() -> String;
    #[wasm_bindgen(js_namespace=erluRuntime,js_name=invokeRegistered)]
    fn js_invoke_registered(name: &str, args: &str, revision: f64);
    #[wasm_bindgen(js_namespace=erluRuntime,js_name=pollResults)]
    fn js_poll_results() -> String;
    #[wasm_bindgen(js_namespace=erluRuntime,js_name=onState)]
    fn on_state(json: &str);
    #[wasm_bindgen(js_namespace=erluRuntime,js_name=setDirty)]
    fn set_dirty(dirty: bool);
}
#[derive(Default)]
pub struct WebEvents {
    last_state: String,
}
impl WebEvents {
    pub fn revoke(&self) {
        configure_ai(false, "");
    }
}
pub fn load_plugin() {
    js_load_plugin();
}
pub fn invoke_plugin(id: &str, action: &str, args: Value, state: &Engine) {
    js_invoke_plugin(id,action,&args.to_string(),&json!({"api_version":1,"document":state.document,"view":state.view,"revision":state.revision}).to_string());
}
pub fn plugin_specs() -> Vec<ActionSpec> {
    serde_json::from_str(&js_plugin_specs()).unwrap_or_default()
}
pub fn invoke_registered(name: &str, args: Value, revision: u64) {
    js_invoke_registered(name, &args.to_string(), revision as f64);
}
pub fn checkpoint(state: &Engine) {
    js_checkpoint(&state.recovery_json());
}
pub fn pump(app: &mut ErluApp, _ctx: &eframe::egui::Context) {
    configure_ai(app.ai_enabled, &app.ai_origin);
    if let Ok(results) = serde_json::from_str::<Vec<Value>>(&js_poll_results()) {
        for result in results {
            app.show_plugin_result(result);
        }
    }
    let state = app.shared.lock().unwrap();
    set_dirty(state.dirty());
    let summary =
        json!({"revision":state.revision,"dirty":state.dirty(),"view":state.view}).to_string();
    drop(state);
    if summary != app.web_events.last_state {
        app.web_events.last_state = summary.clone();
        on_state(&summary);
    }
}
pub fn download(name: &str, bytes: &[u8], mime: &str) -> std::result::Result<(), JsValue> {
    let options = web_sys::BlobPropertyBag::new();
    options.set_type(mime);
    let data = js_sys::Uint8Array::from(bytes);
    let parts = js_sys::Array::new();
    parts.push(&data.buffer());
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)?;
    let document = web_sys::window().unwrap().document().unwrap();
    let a = document
        .create_element("a")?
        .dyn_into::<web_sys::HtmlAnchorElement>()?;
    a.set_href(&url);
    a.set_download(name.rsplit(['/', '\\']).next().unwrap_or(name));
    document.body().unwrap().append_child(&a)?;
    a.click();
    a.remove();
    let revoke = Closure::once_into_js(move || {
        let _ = web_sys::Url::revoke_object_url(&url);
    });
    web_sys::window()
        .unwrap()
        .set_timeout_with_callback_and_timeout_and_arguments_0(revoke.unchecked_ref(), 30000)?;
    Ok(())
}

#[wasm_bindgen]
pub struct WebHandle {
    runner: eframe::WebRunner,
    shared: SharedEngine,
}
#[wasm_bindgen]
impl WebHandle {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            runner: eframe::WebRunner::new(),
            shared: Arc::new(Mutex::new(Engine::default())),
        }
    }
    pub async fn start(
        &self,
        canvas: web_sys::HtmlCanvasElement,
    ) -> std::result::Result<(), JsValue> {
        let shared = self.shared.clone();
        self.runner
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(move |cc| Ok(Box::new(ErluApp::new(cc, shared, true)))),
            )
            .await
    }
    pub fn actions(&self) -> String {
        let mut specs = action_specs();
        specs.extend(host_action_specs());
        serde_json::to_string(&specs).unwrap()
    }
    pub fn call(&self, name: &str, args: &str, expected: Option<u64>) -> String {
        let result = serde_json::from_str(args)
            .map_err(|e| error("invalid_arguments", e.to_string()))
            .and_then(|args| self.shared.lock().unwrap().call(name, args, expected));
        match result {
            Ok(r) => json!({"ok":true,"response":r}).to_string(),
            Err(e) => json!({"ok":false,"error":e}).to_string(),
        }
    }
    pub fn plugin_apply(&self, commands: &str, expected: u64) -> String {
        let result = serde_json::from_str::<Vec<Command>>(commands)
            .map_err(|e| error("plugin_output", e.to_string()))
            .and_then(|commands| {
                self.shared
                    .lock()
                    .unwrap()
                    .dispatch(Command::Transaction { commands }, Some(expected))
            });
        match result {
            Ok(r) => json!({"ok":true,"response":r}).to_string(),
            Err(e) => json!({"ok":false,"error":e}).to_string(),
        }
    }
    pub fn restore(&self, json: &str) -> String {
        let mut state = self.shared.lock().unwrap();
        match state.restore_recovery(json) {
            Ok(()) => json!({"ok":true}).to_string(),
            Err(e) => json!({"ok":false,"error":e}).to_string(),
        }
    }
}
