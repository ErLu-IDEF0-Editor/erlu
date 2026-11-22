use super::*;

impl ErluApp {
    pub(super) fn dialogs(&mut self, ctx: &egui::Context, s: &Engine) {
        if let Some(mut form) = self.dialogs.plugin.take() {
            let mut keep = true;
            egui::Window::new(&form.spec.description)
                .collapsible(false)
                .show(ctx, |ui| {
                    schema_form(ui, &form.spec.input_schema, &mut form.arguments);
                    ui.horizontal(|ui| {
                        if ui.button(self.tr("Run")).clicked() {
                            self.invoke_registered(&form.spec.name, form.arguments.clone(), s);
                            keep = false;
                        }
                        if ui.button(self.tr("Cancel")).clicked() {
                            keep = false;
                        }
                    });
                });
            if keep {
                self.dialogs.plugin = Some(form);
            }
        }

        if self.dialogs.startup {
            egui::Window::new(self.tr("Welcome"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(self.tr("Start a new diagram or open an existing one?"));
                    ui.horizontal(|ui| {
                        if ui.button(self.tr("Open…")).clicked() {
                            self.open_file();
                        }
                        if ui.button(self.tr("New")).clicked() {
                            self.perform(Pending::New);
                            self.dialogs.startup = false;
                        }
                        if ui.button(self.tr("Cancel")).clicked() {
                            self.dialogs.startup = false;
                            #[cfg(not(target_arch = "wasm32"))]
                            if !self.has_document {
                                self.allow_close = true;
                            }
                        }
                    });
                });
        }
        if let Some(mut meta) = self.dialogs.metadata.take() {
            let mut keep = true;
            egui::Window::new(self.tr(if self.dialogs.new_metadata {
                "Welcome"
            } else {
                "Properties…"
            }))
            .default_width(480.)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (i, name) in ["General", "ABC Units", "Page Setup", "Edit header/footer"]
                        .iter()
                        .enumerate()
                    {
                        if ui
                            .selectable_label(self.dialogs.tab == i, self.tr(name))
                            .clicked()
                        {
                            self.dialogs.tab = i;
                        }
                    }
                });
                ui.separator();
                egui::Grid::new("metadata")
                    .num_columns(2)
                    .spacing([12., 8.])
                    .show(ui, |ui| match self.dialogs.tab {
                        0 => {
                            field(ui, &self.tr("Title:"), &mut meta.title);
                            field(ui, &self.tr("Organization:"), &mut meta.organization);
                            let old = meta.author.clone();
                            let old_initials = old
                                .split_whitespace()
                                .filter_map(|s| s.chars().next())
                                .flat_map(char::to_uppercase)
                                .collect::<String>();
                            field(ui, &self.tr("Author:"), &mut meta.author);
                            if meta.author != old
                                && (meta.initials.is_empty() || meta.initials == old_initials)
                            {
                                meta.initials = meta
                                    .author
                                    .split_whitespace()
                                    .filter_map(|s| s.chars().next())
                                    .flat_map(char::to_uppercase)
                                    .collect();
                            }
                            field(ui, &self.tr("Author's initials:"), &mut meta.initials);
                        }
                        1 => {
                            ui.label(self.tr("Currency:"));
                            ui.horizontal(|ui| {
                                ui.text_edit_singleline(&mut meta.currency);
                                ui.menu_button("▾", |ui| {
                                    for choice in ["$", "₽", "€", "person-hours"] {
                                        if ui.button(self.tr(choice)).clicked() {
                                            meta.currency = choice.into();
                                            if choice == "person-hours" {
                                                meta.currency_placement = "1 ?".into();
                                            }
                                            ui.close();
                                        }
                                    }
                                });
                            });
                            ui.end_row();
                            ui.label(self.tr("Symbol placement:"));
                            egui::ComboBox::from_id_salt("placement")
                                .selected_text(&meta.currency_placement)
                                .show_ui(ui, |ui| {
                                    for v in ["1?", "?1", "1 ?", "? 1"] {
                                        ui.selectable_value(
                                            &mut meta.currency_placement,
                                            v.into(),
                                            v,
                                        );
                                    }
                                });
                            ui.end_row();
                            ui.label(self.tr("Time Unit:"));
                            egui::ComboBox::from_id_salt("unit")
                                .selected_text(self.tr(&meta.time_unit))
                                .show_ui(ui, |ui| {
                                    for v in [
                                        "seconds", "minutes", "hours", "days", "weeks", "months",
                                        "years",
                                    ] {
                                        ui.selectable_value(
                                            &mut meta.time_unit,
                                            v.into(),
                                            self.tr(v),
                                        );
                                    }
                                });
                            ui.end_row();
                        }
                        2 => {
                            ui.label(self.tr("Size:"));
                            egui::ComboBox::from_id_salt("size")
                                .selected_text(&meta.page_size)
                                .show_ui(ui, |ui| {
                                    for v in ["A4", "A3", "A2", "A1"] {
                                        ui.selectable_value(&mut meta.page_size, v.into(), v);
                                    }
                                });
                            ui.end_row();
                            ui.label(self.tr("Type:"));
                            egui::ComboBox::from_id_salt("orientation")
                                .selected_text(self.tr(if meta.portrait {
                                    "Portrait"
                                } else {
                                    "Landscape"
                                }))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut meta.portrait,
                                        false,
                                        self.tr("Landscape"),
                                    );
                                    ui.selectable_value(
                                        &mut meta.portrait,
                                        true,
                                        self.tr("Portrait"),
                                    );
                                });
                            ui.end_row();
                            ui.checkbox(&mut meta.with_header, self.tr("With header"));
                            ui.end_row();
                            ui.checkbox(&mut meta.with_footer, self.tr("With footer"));
                            ui.end_row();
                        }
                        _ => {
                            for (label, value) in [
                                ("Used at:", &mut meta.used_at),
                                ("Author:", &mut meta.author),
                                ("Project:", &mut meta.project),
                                ("Notes:", &mut meta.notes),
                                ("Date:", &mut meta.date),
                                ("Rev:", &mut meta.revision),
                                ("Reader:", &mut meta.reader),
                                ("Reader date:", &mut meta.reader_date),
                                ("Context:", &mut meta.context),
                                ("Title:", &mut meta.title),
                                ("Number:", &mut meta.number),
                                ("Node override:", &mut meta.footer_node_override),
                                ("Title override:", &mut meta.footer_title_override),
                                ("Number override:", &mut meta.footer_number_override),
                            ] {
                                field(ui, &self.tr(label), value);
                            }
                            for (label, state) in [
                                ("Working", DiagramState::Working),
                                ("Draft", DiagramState::Draft),
                                ("Recommended", DiagramState::Recommended),
                                ("Publication", DiagramState::Publication),
                            ] {
                                let caption = self.tr(label);
                                ui.radio_value(&mut self.dialogs.state, state, caption);
                                ui.end_row();
                            }
                        }
                    });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(self.tr("OK")).clicked() {
                        #[cfg(not(target_arch = "wasm32"))]
                        if self.dialogs.new_metadata && self.has_document {
                            match std::env::var_os("ERLU_EXECUTABLE")
                                .map(std::path::PathBuf::from)
                                .map(Ok)
                                .unwrap_or_else(std::env::current_exe)
                                .and_then(|exe| {
                                    std::process::Command::new(exe)
                                        .arg("--new-meta")
                                        .arg(serde_json::to_string(&meta).unwrap())
                                        .spawn()
                                }) {
                                Ok(_) => keep = false,
                                Err(e) => self.status = e.to_string(),
                            }
                        }
                        if keep {
                            let command = if self.dialogs.new_metadata {
                                Command::DocumentNew {
                                    meta: Box::new(meta.clone()),
                                    discard: true,
                                }
                            } else {
                                Command::Transaction {
                                    commands: vec![
                                        Command::DocumentMetadata { patch: json!(meta) },
                                        Command::DiagramState {
                                            diagram: Some(s.view.diagram),
                                            state: self.dialogs.state,
                                        },
                                    ],
                                }
                            };
                            if self.execute(command).is_some() {
                                keep = false;
                                if self.dialogs.new_metadata {
                                    self.file_name.clear();
                                    self.has_document = true;
                                }
                            }
                        }
                    }
                    if ui.button(self.tr("Cancel")).clicked() {
                        keep = false;
                    }
                });
            });
            if keep {
                self.dialogs.metadata = Some(meta);
            }
        }
        if let Some(mut block) = self.dialogs.block.take() {
            let mut keep = true;
            let mut has_price = block.price.is_some();
            let mut price = block.price.unwrap_or(0.);
            egui::Window::new(self.tr("Edit block"))
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(self.tr("Title:"));
                    ui.add(egui::TextEdit::multiline(&mut block.title).desired_rows(3));
                    ui.checkbox(&mut has_price, self.tr("Set price"));
                    ui.add_enabled(
                        has_price,
                        egui::DragValue::new(&mut price)
                            .range(0.0..=1e9)
                            .fixed_decimals(2)
                            .suffix(format!(" {}", s.document.meta.currency)),
                    );
                    block.price = has_price.then_some(price);
                    ui.horizontal(|ui| {
                        if ui.button(self.tr("OK")).clicked() {
                            self.execute(Command::BlockUpdate {
                                id: block.id,
                                title: Some(block.title.clone()),
                                price: block.price,
                                clear_price: block.price.is_none(),
                            });
                            keep = false;
                        }
                        if ui.button(self.tr("Cancel")).clicked() {
                            keep = false;
                        }
                    });
                });
            if keep {
                self.dialogs.block = Some(block);
            }
        }
        if let Some(mut arrow) = self.dialogs.arrow.take() {
            let mut keep = true;
            egui::Window::new(self.tr("Arrow label"))
                .collapsible(false)
                .show(ctx, |ui| {
                    if arrow.interface.is_none() {
                        field(ui, &self.tr("Label:"), &mut arrow.label);
                    } else {
                        ui.label(&arrow.label);
                    }
                    ui.checkbox(&mut arrow.label_hidden, self.tr("Hide label"));
                    ui.checkbox(&mut arrow.from.tunneled, self.tr("Tunnel source"));
                    ui.checkbox(&mut arrow.to.tunneled, self.tr("Tunnel target"));
                    ui.horizontal(|ui| {
                        if ui.button(self.tr("OK")).clicked() {
                            self.execute(Command::ArrowUpdate {
                                id: arrow.id,
                                from: Some(arrow.from.clone()),
                                to: Some(arrow.to.clone()),
                                label: arrow.interface.is_none().then_some(arrow.label.clone()),
                                label_offset: None,
                                label_hidden: Some(arrow.label_hidden),
                                bends: None,
                            });
                            keep = false;
                        }
                        if ui.button(self.tr("Cancel")).clicked() {
                            keep = false;
                        }
                    });
                });
            if keep {
                self.dialogs.arrow = Some(arrow);
            }
        }
        if self.dialogs.tree {
            let mut open = true;
            egui::Window::new(self.tr("Node tree…"))
                .open(&mut open)
                .default_width(400.)
                .show(ctx, |ui| {
                    self.tree_ui(ui, s, s.document.root);
                });
            self.dialogs.tree = open;
        }
        if let Some(mut report) = self.dialogs.report.take() {
            let mut open = true;
            egui::Window::new(self.tr("Report"))
                .open(&mut open)
                .default_size([650., 450.])
                .show(ctx, |ui| {
                    egui::ScrollArea::both().show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut report)
                                .desired_width(f32::INFINITY)
                                .interactive(false),
                        );
                    });
                });
            if open {
                self.dialogs.report = Some(report);
            }
        }
        if self.dialogs.export {
            let mut open = true;
            egui::Window::new(self.tr("Export to PDF…"))
                .open(&mut open)
                .show(ctx, |ui| {
                    {
                        let label = self.tr("All diagrams");
                        ui.checkbox(&mut self.dialogs.all, label);
                    }
                    {
                        let label = self.tr("Page numbers");
                        ui.checkbox(&mut self.dialogs.numbers, label);
                    }
                    {
                        let label = self.tr("Force light theme");
                        ui.checkbox(&mut self.dialogs.light, label);
                    }
                    if ui.button(self.tr("Export")).clicked() {
                        self.export("pdf");
                        self.dialogs.export = false;
                    }
                });
            self.dialogs.export &= open;
        }
        if self.dialogs.colors {
            let mut open = true;
            egui::Window::new(self.tr("Select item color"))
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.color_edit_button_srgb(&mut self.dialogs.color);
                    if ui.button(self.tr("OK")).clicked() {
                        let [r, g, b] = self.dialogs.color;
                        self.plugin_color(s, Some(format!("#{r:02x}{g:02x}{b:02x}")));
                        self.dialogs.colors = false;
                    }
                });
            self.dialogs.colors &= open;
        }
        if self.dialogs.call {
            let mut open = true;
            egui::Window::new(self.tr("Call Mechanism"))
                .open(&mut open)
                .show(ctx, |ui| {
                    if s.view.selection.len() != 1
                        || s.document
                            .block(*s.view.selection.first().unwrap_or(&0))
                            .is_none()
                    {
                        ui.label(self.tr("Select exactly one block to add a call mechanism."));
                        return;
                    }
                    let block = *s.view.selection.first().unwrap();
                    ui.label(self.tr("Choose target block:"));
                    for b in s.document.diagrams[&s.view.diagram]
                        .blocks
                        .values()
                        .filter(|b| b.id != block && b.number != "A0")
                    {
                        ui.selectable_value(
                            &mut self.dialogs.call_ref,
                            Some(b.id),
                            format!("{} — {}", b.number, b.title),
                        );
                    }
                    if ui.button(self.tr("OK")).clicked()
                        && let Some(reference) = self.dialogs.call_ref
                    {
                        self.pending_call = Some((block, reference));
                        self.status = self
                            .tr("Click bottom frame to place call mechanism arrow. Esc to cancel.");
                        self.dialogs.call = false;
                    }
                });
            self.dialogs.call &= open;
        }
        if self.dialogs.swap {
            let mut open = true;
            egui::Window::new(self.tr("Swap numbers…"))
                .open(&mut open)
                .show(ctx, |ui| {
                    for i in 0..2 {
                        egui::ComboBox::from_id_salt(("swap", i))
                            .selected_text(
                                self.dialogs.swap_pair[i]
                                    .and_then(|id| s.document.block(id))
                                    .map(|b| b.number.as_str())
                                    .unwrap_or("—"),
                            )
                            .show_ui(ui, |ui| {
                                for b in s.document.diagrams[&s.view.diagram].blocks.values() {
                                    ui.selectable_value(
                                        &mut self.dialogs.swap_pair[i],
                                        Some(b.id),
                                        format!("{} {}", b.number, b.title),
                                    );
                                }
                            });
                    }
                    if ui.button(self.tr("OK")).clicked()
                        && let [Some(first), Some(second)] = self.dialogs.swap_pair
                    {
                        self.execute(Command::NumbersSwap { first, second });
                        self.dialogs.swap = false;
                    }
                });
            self.dialogs.swap &= open;
        }
        if self.pending.is_some() {
            egui::Window::new(self.tr("Unsaved changes"))
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(self.tr("Save changes before continuing?"));
                    ui.horizontal(|ui| {
                        if ui.button(self.tr("Save")).clicked() {
                            self.save(false);
                        }
                        if ui.button(self.tr("Discard")).clicked() {
                            let p = self.pending.take().unwrap();
                            self.perform(p);
                        }
                        if ui.button(self.tr("Cancel")).clicked() {
                            self.pending = None;
                        }
                    });
                });
        }
    }
    pub(super) fn tree_ui(&mut self, ui: &mut egui::Ui, s: &Engine, d: Id) {
        let label = format!("{} {}", s.document.node(d), s.document.title(d));
        egui::CollapsingHeader::new(label)
            .id_salt(d)
            .default_open(true)
            .show(ui, |ui| {
                if ui.button(self.tr("Open diagram")).clicked() {
                    self.execute(Command::Navigate { diagram: d });
                }
                for b in s.document.diagrams[&d].blocks.values() {
                    if let Some(c) = b.child {
                        self.tree_ui(ui, s, c);
                    } else if ui
                        .selectable_label(
                            s.view.selection.contains(&b.id),
                            format!("{} {}", b.number, b.title),
                        )
                        .clicked()
                    {
                        self.execute(Command::Navigate { diagram: d });
                        self.select(vec![b.id]);
                    }
                }
            });
    }
}
