#[cfg(target_arch = "wasm32")]
fn main() {
    erlu_app::link_web_exports();
}
#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use erlu_app::mcp::ErluMcp;
    use erlu_core::*;
    use std::sync::{Arc, Mutex};
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) == Some("schema") {
        println!(
            "{}",
            serde_json::to_string_pretty(&schemars::schema_for!(Document))?
        );
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("actions") {
        let mut specs = action_specs();
        specs.extend(host_action_specs());
        println!("{}", serde_json::to_string_pretty(&specs)?);
        return Ok(());
    }
    let new_meta = args
        .iter()
        .position(|s| s == "--new-meta")
        .and_then(|i| args.get(i + 1));
    let file = if new_meta.is_some() {
        None
    } else {
        args.iter().find(|s| !s.starts_with('-') && *s != "mcp")
    };
    let startup_name = file.cloned();
    let engine = if let Some(meta) = new_meta {
        let doc = Document::with_meta(serde_json::from_str(meta)?);
        Engine::new(doc)?
    } else if let Some(path) = file {
        Engine::new(read_document(&std::fs::read_to_string(path)?)?)?
    } else {
        Engine::default()
    };
    let shared = Arc::new(Mutex::new(engine));
    if args.first().map(String::as_str) == Some("mcp") {
        let server = ErluMcp {
            shared,
            plugins: Arc::new(Mutex::new(erlu_plugins::PluginHost::default())),
            repaint: None,
        };
        return tokio::runtime::Runtime::new()?.block_on(erlu_app::mcp::stdio(server));
    }
    let mcp = args.iter().any(|a| a == "--mcp");
    let startup = file.is_none() && new_meta.is_none();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("erlu IDEF0 editor")
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/icons/erlu.png"))
                    .expect("Application icon"),
            )
            .with_inner_size([1150., 850.]),
        ..Default::default()
    };
    eframe::run_native(
        "erlu",
        options,
        Box::new(move |cc| {
            let mut app = erlu_app::app::ErluApp::new(cc, shared.clone(), startup);
            if let Some(name) = &startup_name {
                app.set_file_name(name.clone());
            }
            if mcp {
                let token = uuid::Uuid::new_v4().to_string();
                app.ai_token = token.clone();
                let server = ErluMcp {
                    shared: shared.clone(),
                    plugins: app.plugins.clone(),
                    repaint: Some(cc.egui_ctx.clone()),
                };
                std::thread::spawn(move || {
                    let result = tokio::runtime::Runtime::new()
                        .unwrap()
                        .block_on(erlu_app::mcp::http(server, token));
                    if let Err(e) = result {
                        eprintln!("MCP server: {e}");
                    }
                });
            }
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| e.to_string().into())
}
