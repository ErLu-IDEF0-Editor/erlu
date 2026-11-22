//! Native host for pure WIT components. No WASI, filesystem or network imports.
use erlu_core::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Duration,
};
use wasmtime::{
    Config, Engine as WasmEngine, Store, StoreLimits, StoreLimitsBuilder,
    component::{Component, Linker},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginAction {
    pub name: String,
    pub title: String,
    pub kind: String,
    pub input_schema: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub api_version: u32,
    pub version: String,
    pub component: String,
    pub web: Option<String>,
    pub actions: Vec<PluginAction>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Output {
    #[serde(default)]
    pub commands: Vec<Command>,
    #[serde(default)]
    pub result: Value,
}
struct Loaded {
    manifest: Manifest,
    component: Component,
}
pub struct PluginHost {
    runtime: WasmEngine,
    plugins: BTreeMap<String, Loaded>,
}
impl Default for PluginHost {
    fn default() -> Self {
        let mut c = Config::new();
        c.consume_fuel(true).epoch_interruption(true);
        let runtime = WasmEngine::new(&c).expect("Wasmtime engine");
        let mut host = Self {
            runtime,
            plugins: BTreeMap::new(),
        };
        let paths = [
            std::env::var_os("ERLU_PLUGIN_DIR")
                .map(|p| PathBuf::from(p).join("colors/plugin.json")),
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.join("plugins/colors/plugin.json"))),
            Some(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plugins/colors/plugin.json"),
            ),
        ];
        for p in paths.into_iter().flatten() {
            if p.exists() && host.load(&p).is_ok() {
                break;
            }
        }
        host
    }
}
impl PluginHost {
    pub fn load(&mut self, path: &Path) -> Result<()> {
        let bytes = std::fs::read(path).map_err(|e| error("plugin_load", e.to_string()))?;
        let manifest: Manifest =
            serde_json::from_slice(&bytes).map_err(|e| error("plugin_manifest", e.to_string()))?;
        validate_manifest(&manifest)?;
        let root = path.parent().unwrap_or(Path::new("."));
        let component = Component::from_file(&self.runtime, root.join(&manifest.component))
            .map_err(|e| error("plugin_load", e.to_string()))?;
        let mut store = self.store()?;
        let linker = Linker::<StoreLimits>::new(&self.runtime);
        let instance = linker
            .instantiate(&mut store, &component)
            .map_err(|e| error("plugin_import", e.to_string()))?;
        instance
            .get_typed_func::<(String, String), (std::result::Result<String, String>,)>(
                &mut store, "run",
            )
            .map_err(|e| error("plugin_api", e.to_string()))?;
        let existing = self.specs();
        for action in &manifest.actions {
            let name = format!("plugin_{}_{}", manifest.id, action.name.replace('.', "_"));
            if existing.iter().any(|s| s.name == name) && !self.plugins.contains_key(&manifest.id) {
                return Err(error("plugin_manifest", "Plugin tool name collision"));
            }
        }
        self.plugins.insert(
            manifest.id.clone(),
            Loaded {
                manifest,
                component,
            },
        );
        Ok(())
    }
    fn store(&self) -> Result<Store<StoreLimits>> {
        let limits = StoreLimitsBuilder::new()
            .memory_size(32 * 1024 * 1024)
            .instances(4)
            .memories(4)
            .tables(4)
            .trap_on_grow_failure(true)
            .build();
        let mut store = Store::new(&self.runtime, limits);
        store.limiter(|s| s);
        store
            .set_fuel(10_000_000)
            .map_err(|e| error("plugin_runtime", e.to_string()))?;
        store.set_epoch_deadline(1);
        Ok(store)
    }
    pub fn manifests(&self) -> Vec<Manifest> {
        self.plugins.values().map(|p| p.manifest.clone()).collect()
    }
    pub fn specs(&self) -> Vec<ActionSpec> {
        self.plugins
            .values()
            .flat_map(|p| {
                p.manifest.actions.iter().map(|a| ActionSpec {
                    name: format!("plugin_{}_{}", p.manifest.id, a.name.replace('.', "_")),
                    description: a.title.clone(),
                    input_schema: a.input_schema.clone(),
                    read_only: a.kind != "command",
                })
            })
            .collect()
    }
    pub fn resolve(&self, name: &str) -> Option<(String, String)> {
        for p in self.plugins.values() {
            for a in &p.manifest.actions {
                if format!("plugin_{}_{}", p.manifest.id, a.name.replace('.', "_")) == name {
                    return Some((p.manifest.id.clone(), a.name.clone()));
                }
            }
        }
        None
    }
    pub fn invoke(
        &self,
        id: &str,
        action: &str,
        args: Value,
        state: &erlu_core::Engine,
    ) -> Result<Vec<Command>> {
        Ok(self.invoke_output(id, action, args, state)?.commands)
    }
    pub fn invoke_output(
        &self,
        id: &str,
        action: &str,
        args: Value,
        state: &erlu_core::Engine,
    ) -> Result<Output> {
        let plugin = self
            .plugins
            .get(id)
            .ok_or_else(|| error("plugin_not_found", format!("Plugin {id} is not loaded")))?;
        let spec = plugin
            .manifest
            .actions
            .iter()
            .find(|a| a.name == action)
            .ok_or_else(|| error("not_found", "Plugin action not registered"))?;
        let context = json!({"api_version":1,"revision":state.revision,"document":state.document,"view":state.view,"arguments":args});
        let mut store = self.store()?;
        let linker = Linker::<StoreLimits>::new(&self.runtime);
        let instance = linker
            .instantiate(&mut store, &plugin.component)
            .map_err(|e| error("plugin_runtime", e.to_string()))?;
        let func = instance
            .get_typed_func::<(String, String), (std::result::Result<String, String>,)>(
                &mut store, "run",
            )
            .map_err(|e| error("plugin_api", e.to_string()))?;
        // Epoch interruption is a wall-time bound in addition to deterministic fuel.
        let runtime = self.runtime.clone();
        let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
        let timer = std::thread::spawn(move || {
            if done_rx.recv_timeout(Duration::from_secs(2)).is_err() {
                runtime.increment_epoch();
            }
        });
        let result = func.call(&mut store, (action.to_string(), context.to_string()));
        let _ = done_tx.send(());
        let _ = timer.join();
        let (result,) = result.map_err(|e| error("plugin_runtime", e.to_string()))?;
        let text = result.map_err(|e| error("plugin_error", e))?;
        if text.len() > 4 * 1024 * 1024 {
            return Err(error("plugin_limit", "Plugin response exceeds 4 MiB"));
        }
        let output: Output =
            serde_json::from_str(&text).map_err(|e| error("plugin_output", e.to_string()))?;
        if spec.kind != "command" && !output.commands.is_empty() {
            return Err(error(
                "plugin_capability",
                "Read-only plugin action proposed edits",
            ));
        }
        let mut check = state.clone();
        check.dispatch(
            Command::Transaction {
                commands: output.commands.clone(),
            },
            Some(state.revision),
        )?;
        Ok(output)
    }
}
pub fn validate_manifest(m: &Manifest) -> Result<()> {
    if m.api_version != 1
        || m.id.is_empty()
        || !m.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(error(
            "plugin_manifest",
            "Expected API v1 and alphanumeric plugin ID",
        ));
    }
    let mut names = std::collections::BTreeSet::new();
    for a in &m.actions {
        if !names.insert(a.name.replace('.', "_"))
            || a.name.is_empty()
            || !a
                .name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
            || !["command", "validation", "export"].contains(&a.kind.as_str())
            || !a.input_schema.is_object()
        {
            return Err(error("plugin_manifest", "Invalid action declaration"));
        }
    }
    Ok(())
}
