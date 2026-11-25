use super::*;

impl ErluApp {
    pub(super) fn menu(&mut self, ui: &mut egui::Ui, snapshot: &Engine) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button(self.ui_text("File"), |ui| {
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
                    if let Some(r)=self.execute(Command::Validate{}) {self.dialogs.report_export=None;self.dialogs.report_confirm=None;self.dialogs.report=Some(self.validation_report(&r.result,snapshot));}
                    ui.close();
                }
                if ui.button(self.ui_text("Calculate Prices")).clicked() {
                    if let Some(r)=self.execute(Command::CalculatePrices{}) {self.dialogs.report_export=None;self.dialogs.report_confirm=None;self.dialogs.report=Some(format!("{}: {}",self.ui_text("Updated blocks"),r.result["updated_blocks"]));}
                    ui.close();
                }
                if ui.button(self.ui_text("Node tree…")).clicked() {self.dialogs.tree=true;ui.close();}
            });
            ui.menu_button(self.ui_text("View"), |ui| {
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
            });
            let specs=self.plugin_specs();
            ui.menu_button(self.ui_text("Plugins"), |ui| {
                if ui.button(self.ui_text("Set item color...")).clicked() {self.dialogs.colors=true;ui.close();}
                if ui.button(self.ui_text("Clear item colors")).clicked() {self.plugin_color(snapshot,None);ui.close();}
                for spec in specs {
                    if spec.name.starts_with("plugin_colors_") || spec.input_schema["x-erlu-confirm-only"] == true {continue;}
                    if ui.button(self.ui_text(&spec.description)).clicked() {
                        let arguments=form_defaults(&spec.input_schema);self.dialogs.plugin=Some(PluginForm{spec,arguments});ui.close();
                    }
                }
                #[cfg(not(target_arch="wasm32"))]
                if ui.button(self.ui_text("Load plugin…")).clicked() {
                    if let Some(path)=rfd::FileDialog::new().add_filter(self.tr("Plugin manifest"),&["json"]).pick_file()
                        && let Err(e)=self.plugins.lock().unwrap().load(&path) {self.status=e.to_string();}
                    ui.close();
                }
                #[cfg(target_arch="wasm32")]
                if ui.button(self.ui_text("Load plugin…")).clicked() {crate::web::load_plugin();ui.close();}
            });
            egui::containers::menu::MenuButton::new("AI")
                .config(egui::containers::menu::MenuConfig::new().close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside))
                .ui(ui, |ui| {
                    #[cfg(target_arch="wasm32")]
                    {
                        let caption=self.ui_text("Allow agent for this tab");ui.checkbox(&mut self.ai_enabled,caption);
                        ui.label(self.ui_text("Allowed origin"));ui.text_edit_singleline(&mut self.ai_origin);
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
                ui.label(format!("{}{}",if snapshot.dirty(){"● "}else{""},snapshot.document.meta.title));
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
    pub(super) fn plugin_specs(&self) -> Vec<ActionSpec> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.plugins.lock().unwrap().specs()
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::web::plugin_specs()
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
