use super::*;

impl ErluApp {
    pub(super) fn menu(&mut self, ui: &mut egui::Ui, snapshot: &Engine) {
        egui::MenuBar::new().ui(ui, |ui| {
            let dfd = snapshot.dfd_document();
            ui.menu_button(self.ui_text("File"), |ui| {
                let new_dfd = ui.button(self.ui_text("New DFD"));
                ui_target("file.new_dfd", &new_dfd, None);
                if new_dfd.clicked() { self.request(Pending::NewDfd); ui.close(); }
                for (label,action) in [("New",0),("Open…",1),("Save",2),("Save As…",3),("Export to PDF…",4),("Export to Markdown…",5),("Export to SVG…",6)] {
                    if ui.button(self.ui_text(label)).clicked() {
                        ui.close();
                        match action {
                            0=>self.request(Pending::New), 1=>self.request(Pending::Open),
                            2=>self.save(false), 3=>self.save(true), 4=>self.dialogs.export=true,
                            5=>self.export("md"), _=>self.export("svg"),
                        }
                    }
                }
            });
            let edit_menu=ui.menu_button(self.ui_text("Edit"), |ui| {
                for (label,command) in [("Undo",Command::Undo{}),("Redo",Command::Redo{}),("Delete",Command::ObjectsDelete{ids:snapshot.view.selection.iter().copied().collect()})] {
                    if ui.button(self.ui_text(label)).clicked() {self.execute(command);ui.close();}
                }
                if dfd.is_some() { return; }
                if ui.button(self.ui_text("Swap numbers…")).clicked() {self.dialogs.swap=true;ui.close();}
                if ui.button(self.ui_text("Call Mechanism add")).clicked() {self.dialogs.call=true;ui.close();}
                let properties=ui.button(self.ui_text("Properties…"));
                ui_target("menu.properties",&properties,None);
                if properties.clicked() {
                    self.dialogs.metadata=Some(snapshot.document.meta.clone());
                    self.dialogs.state=snapshot.document.diagrams[&snapshot.view.diagram].state;
                    self.dialogs.new_metadata=false;self.dialogs.tab=0;ui.close();
                }
            });
            ui_target("menu.edit",&edit_menu.response,None);
            ui.menu_button(self.ui_text("Tools"), |ui| {
                if ui.button(self.ui_text("Validation")).clicked() {
                    if dfd.is_some() {
                        self.dfd_ui.validation = true;
                        ui.close();
                        return;
                    }
                    if let Some(r)=self.execute(Command::Validate{}) {self.dialogs.report_export=None;self.dialogs.report_confirm=None;self.dialogs.report=Some(self.validation_report(&r.result,snapshot));}
                    ui.close();
                }
                if dfd.is_some() {
                    if ui.button(self.ui_text("Data dictionary")).clicked() { self.dfd_ui.dictionary = true; ui.close(); }
                    return;
                }
                if ui.button(self.ui_text("Calculate Prices")).clicked() {
                    if let Some(r)=self.execute(Command::CalculatePrices{}) {self.dialogs.report_export=None;self.dialogs.report_confirm=None;self.dialogs.report=Some(format!("{}: {}",self.ui_text("Updated blocks"),r.result["updated_blocks"]));}
                    ui.close();
                }
                if ui.button(self.ui_text("Node tree…")).clicked() {self.dialogs.tree=true;ui.close();}
            });
            let view_menu=ui.menu_button(self.ui_text("View"), |ui| {
                for (label,zoom) in [("Fit",1.),("Scale +",snapshot.view.zoom*1.1),("Scale -",snapshot.view.zoom/1.1)] {
                    if ui.button(self.ui_text(label)).clicked() {
                        self.execute(if label=="Fit" {Command::ViewFit{}} else {Command::ViewSet{zoom:Some(zoom.clamp(0.05,40.)),center:None,theme:None}});
                        ui.close();
                    }
                }
                for (label,theme) in [("Follow system theme",Theme::System),("Dark mode",Theme::Dark),("Light mode",Theme::Light)] {
                    if ui.selectable_label(snapshot.view.theme==theme,self.ui_text(label)).clicked() {
                        self.execute(Command::ViewSet{zoom:None,center:None,theme:Some(theme)});ui.close();
                    }
                }
                ui.separator();
                let mut enabled=snapshot.view.grid.enabled;
                let grid=ui.checkbox(&mut enabled,self.ui_text("Grid mode"));
                ui_target("view.grid",&grid,Some(if enabled {"true"}else{"false"}));
                if grid.changed(){self.execute(Command::GridSet{enabled,spacing:None});}
                let mut spacing=snapshot.view.grid.spacing;
                ui.horizontal(|ui|{
                    ui.label(self.ui_text("Grid spacing (mm)"));
                    if ui.add(egui::DragValue::new(&mut spacing).range(1.0..=50.0).speed(1.0)).changed(){
                        self.execute(Command::GridSet{enabled,spacing:Some(spacing)});
                    }
                });
            });
            ui_target("menu.view",&view_menu.response,None);
            if dfd.is_none() {
            let plugins=self.plugin_menus();
            let menu=ui.menu_button(self.ui_text("Plugins"), |ui| {
                if ui.button(self.ui_text("Set item color...")).clicked() {self.dialogs.colors=true;ui.close();}
                if ui.button(self.ui_text("Clear item colors")).clicked() {self.plugin_color(snapshot,None);ui.close();}
                for plugin in plugins {
                    if plugin.id == "colors" {continue;}
                    let actions:Vec<_>=plugin.actions.iter().filter(|spec|spec.input_schema["x-erlu-confirm-only"] != true).collect();
                    if actions.len()>3 {
                        let submenu=ui.menu_button(self.ui_text(&plugin.title), |ui| {
                            for spec in &actions {self.plugin_menu_action(ui,spec,Some(&plugin.title));}
                        });
                        ui_target(&format!("plugin.menu.{}",plugin.id),&submenu.response,Some(&plugin.title));
                    } else {
                        for spec in actions {self.plugin_menu_action(ui,spec,None);}
                    }
                }
                ui.separator();
                let load=ui.button(self.ui_text("Connect external plugin…"));
                ui_target("plugin.load",&load,None);
                if load.clicked() {
                    #[cfg(not(target_arch="wasm32"))]
                    if let Some(path)=rfd::FileDialog::new().add_filter(self.tr("Plugin manifest"),&["json"]).pick_file()
                        && let Err(e)=self.plugins.lock().unwrap().load(&path) {self.status=e.to_string();}
                    #[cfg(target_arch="wasm32")]
                    crate::web::load_plugin();
                    ui.close();
                }
            });
            ui_target("menu.plugins",&menu.response,None);
            }
            let ai = egui::containers::menu::MenuButton::new("AI")
                .config(egui::containers::menu::MenuConfig::new().close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside))
                .ui(ui, |ui| {
                    #[cfg(target_arch="wasm32")]
                    {
                        let caption=self.ui_text("Allow agent for this tab");
                        let toggle=ui.checkbox(&mut self.ai_enabled,caption);
                        ui_target("ai.enabled", &toggle, None);
                        ui.label(self.ui_text("Allowed origin"));
                        let origin=ui.text_edit_singleline(&mut self.ai_origin);
                        ui_target("ai.origin", &origin, None);
                        if !self.ai_enabled {self.web_events.revoke();}
                        ui.label(self.ui_text("Connect using the erlu browser SDK"));
                    }
                    #[cfg(not(target_arch="wasm32"))]
                    {
                        if self.ai_token.is_empty() {ui.label(self.ui_text("Start with --mcp to enable MCP"));}
                        else {
                            ui.label("MCP: http://127.0.0.1:38479/mcp");ui.label(self.ui_text("Token"));
                            let mut token=self.ai_token.clone();ui.add(egui::TextEdit::singleline(&mut token).password(true).interactive(false));
                            if ui.button(self.ui_text("Copy connection")).clicked() {
                                ui.ctx().copy_text(json!({"url":"http://127.0.0.1:38479/mcp","headers":{"Authorization":format!("Bearer {}",self.ai_token)}}).to_string());
                            }
                        }
                    }
                });
            ui_target("menu.ai", &ai.0, None);
            let language_menu=ui.menu_button(self.ui_text("Language"), |ui| {
                for (language,label) in localization::LANGUAGES {
                    let option=ui.selectable_label(self.language==language,localization::display_text(label));
                    ui_target(&format!("language.{}",language),&option,Some(language));
                    if option.clicked() {
                        self.language=language.into();self.execute(Command::LanguageSet{language:language.into()});ui.close();
                    }
                }
            });
            ui_target("menu.language",&language_menu.response,None);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("{}{}",if snapshot.dirty(){"● "}else{""},dfd.map_or(snapshot.document.meta.title.as_str(), |doc| doc.title.as_str())));
            });
        });
    }
    fn validation_report(&self, result: &Value, snapshot: &Engine) -> String {
        let mut report = String::new();
        for (severity, title) in [("error", "Errors:"), ("warning", "Warnings:")] {
            report.push_str(&self.tr(title));
            report.push('\n');
            let mut count = 0;
            if let Some(items) = result.as_array() {
                for item in items.iter().filter(|v| v["severity"] == severity) {
                    count += 1;
                    let code = item["code"].as_str().unwrap_or("");
                    let message = match code {
                        "context_block_count" => "The context diagram should contain one block",
                        "model_name" => "Provide the full model name",
                        "model_abbreviation" => "Provide the model name abbreviation",
                        "model_purpose" => "Provide the model purpose statement",
                        "model_viewpoint" => "Provide the model viewpoint statement",
                        "node_numbering" => {
                            "Use A0 for the context box and consecutive child node numbers formed by appending 1–9 to the parent node"
                        }
                        "decomposition_block_count" => "A decomposition should contain 2–9 blocks",
                        "incomplete_interface" => "Decomposition has unconnected interface circles",
                        "block_name" => "Provide a meaningful function name",
                        "missing_control" => "At least one control is required",
                        "missing_output" => "At least one output is required",
                        "identical_input_output" => "Input and output share the same label",
                        "black_box" => "All connected endpoints are tunneled",
                        "unrelated_block" => "No direct relation to other decomposition blocks",
                        "cost_mismatch" => "Subprocess cost exceeds parent cost",
                        "duplicate_arrow" => {
                            "The same labelled arrow enters a block more than once"
                        }
                        _ => item["message"].as_str().unwrap_or(""),
                    };
                    let reference = item["reference"]
                        .as_str()
                        .map(|r| format!(" [{}; {r}]", item["source"].as_str().unwrap_or("")))
                        .unwrap_or_else(|| " [erlu]".into());
                    let node = item["diagram"]
                        .as_u64()
                        .map(|id| snapshot.document.node(id))
                        .unwrap_or_default();
                    let object = item["object"]
                        .as_u64()
                        .and_then(|id| snapshot.document.block(id))
                        .map(|b| format!(" — {} {}", b.number, b.title))
                        .unwrap_or_default();
                    report.push_str(&format!(
                        "{count}. {} ({node}{object}){reference}\n",
                        self.tr(message)
                    ));
                }
            }
            if count == 0 {
                report.push_str(&self.tr("None."));
                report.push('\n');
            }
            report.push('\n');
        }
        report
    }
    fn plugin_menu_action(&mut self, ui: &mut egui::Ui, spec: &ActionSpec, submenu: Option<&str>) {
        let description = self.tr(&spec.description);
        let prefix = submenu.map(|title| format!("{} — ", self.tr(title)));
        let label = prefix
            .as_deref()
            .and_then(|prefix| description.strip_prefix(prefix))
            .unwrap_or(&description);
        let button = ui.button(localization::display_text(label));
        ui_target(
            &format!("plugin.action.{}", spec.name),
            &button,
            Some(label),
        );
        if button.clicked() {
            let arguments = form_defaults(&spec.input_schema);
            self.dialogs.plugin = Some(PluginForm {
                spec: spec.clone(),
                arguments,
            });
            ui.close();
        }
    }
    fn plugin_menus(&self) -> Vec<PluginMenu> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let host = self.plugins.lock().unwrap();
            let specs = host.specs();
            host.manifests()
                .into_iter()
                .map(|manifest| {
                    let actions = manifest
                        .actions
                        .iter()
                        .filter_map(|action| {
                            let name =
                                format!("plugin_{}_{}", manifest.id, action.name.replace('.', "_"));
                            specs.iter().find(|spec| spec.name == name).cloned()
                        })
                        .collect();
                    PluginMenu {
                        title: manifest
                            .title
                            .filter(|title| !title.trim().is_empty())
                            .unwrap_or_else(|| manifest.id.clone()),
                        id: manifest.id,
                        actions,
                    }
                })
                .collect()
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::web::plugin_menus()
        }
    }
    pub(super) fn invoke_registered(&mut self, name: &str, args: Value, snapshot: &Engine) {
        self.invoke_registered_at(name, args, snapshot, snapshot.revision);
    }
    pub(super) fn invoke_registered_at(
        &mut self,
        name: &str,
        args: Value,
        snapshot: &Engine,
        expected_revision: u64,
    ) {
        if snapshot.revision != expected_revision {
            self.status = self.tr("Preview is stale; generate a new preview");
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let resolved = self.plugins.lock().unwrap().resolve(name);
            if let Some((id, action)) = resolved {
                let output = self
                    .plugins
                    .lock()
                    .unwrap()
                    .invoke_output(&id, &action, args, snapshot);
                match output {
                    Ok(output) => {
                        if self
                            .execute_at(
                                Command::Transaction {
                                    commands: output.commands,
                                },
                                snapshot.revision,
                            )
                            .is_some()
                            && !output.result.is_null()
                        {
                            self.set_plugin_report(&output.result);
                        }
                    }
                    Err(e) => self.status = e.to_string(),
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::web::invoke_registered(name, args, snapshot.revision);
        }
    }
    fn set_plugin_report(&mut self, result: &Value) {
        if result.is_null() || result == &json!({}) {
            return;
        }
        self.dialogs.report_confirm = result
            .get("confirm_action")
            .and_then(|v| serde_json::from_value(v.clone()).ok());
        self.dialogs.report_export = result["filename"]
            .as_str()
            .zip(result["mime"].as_str())
            .map(|(name, mime)| (name.into(), mime.into()));
        self.dialogs.report = Some(
            result["text"]
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| serde_json::to_string_pretty(result).unwrap()),
        );
    }
    pub fn show_plugin_result(&mut self, result: Value) {
        if result["ok"] == false {
            self.status = result["error"]["message"]
                .as_str()
                .unwrap_or("Plugin error")
                .into();
        } else if let Some(value) = result.get("response").and_then(|r| r.get("result"))
            && value != &json!({})
            && !value.is_null()
        {
            self.set_plugin_report(value);
        }
    }
    pub(super) fn plugin_color(&mut self, snapshot: &Engine, color: Option<String>) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let result = self.plugins.lock().unwrap().invoke(
                "colors",
                if color.is_some() {
                    "colors.set"
                } else {
                    "colors.clear"
                },
                json!({"color":color}),
                snapshot,
            );
            match result {
                Ok(commands) => {
                    self.execute_at(Command::Transaction { commands }, snapshot.revision);
                }
                Err(e) => {
                    self.status = e.to_string();
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        crate::web::invoke_plugin(
            "colors",
            if color.is_some() {
                "colors.set"
            } else {
                "colors.clear"
            },
            json!({"color":color}),
            snapshot,
        );
    }
    pub(super) fn has_modal(&self) -> bool {
        self.dialogs.startup
            || self.dialogs.metadata.is_some()
            || self.dialogs.block.is_some()
            || self.dialogs.arrow.is_some()
            || self.dialogs.report.is_some()
            || self.dialogs.tree
            || self.dialogs.export
            || self.dialogs.colors
            || self.dialogs.call
            || self.dialogs.swap
            || self.dialogs.plugin.is_some()
            || self.pending.is_some()
    }
}
