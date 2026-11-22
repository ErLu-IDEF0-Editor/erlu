use crate::app::SharedEngine;
use erlu_core::*;
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt, model::*, service::RequestContext};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
#[derive(Clone)]
pub struct ErluMcp {
    pub shared: SharedEngine,
    pub plugins: Arc<Mutex<erlu_plugins::PluginHost>>,
    pub repaint: Option<eframe::egui::Context>,
}
impl ErluMcp {
    fn tools(&self) -> Vec<Tool> {
        let mut specs = action_specs();
        specs.extend(host_action_specs());
        specs.extend(self.plugins.lock().unwrap().specs());
        specs.into_iter().map(|s|{let mut schema=s.input_schema.as_object().cloned().unwrap_or_default();schema.entry("properties").or_insert(json!({}))["expected_revision"]=json!({"type":"integer","minimum":0,"description":"Reject changes if the current document revision differs"});Tool::new(s.name,s.description,schema).with_annotations(ToolAnnotations::new().read_only(s.read_only).open_world(false))}).collect()
    }
}
impl ServerHandler for ErluMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build()).with_instructions("IDEF0 desktop editor. Every edit uses the GUI transaction engine. Coordinates are millimetres. Read document_state first and pass expected_revision with edits. document_serialize returns a file; no implicit disk writes.")
    }
    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> std::result::Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(self.tools()))
    }
    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.tools().into_iter().find(|t| t.name == name)
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> std::result::Result<CallToolResponse, ErrorData> {
        let mut args = request.arguments.unwrap_or_default();
        let expected = match args.remove("expected_revision") {
            None => None,
            Some(v) => match v.as_u64() {
                Some(r) => Some(r),
                None => {
                    return Ok(CallToolResult::error(vec![ContentBlock::text(
                        serde_json::to_string(&error(
                            "invalid_arguments",
                            "expected_revision must be a non-negative integer",
                        ))
                        .unwrap(),
                    )])
                    .into());
                }
            },
        };
        let name = request.name.to_string();
        let resolved = self.plugins.lock().unwrap().resolve(&name);
        let result = if name == "plugin_list" {
            let state = self.shared.lock().unwrap();
            Ok(Response {
                revision: state.revision,
                dirty: state.dirty(),
                result: json!(self.plugins.lock().unwrap().manifests()),
            })
        } else if name == "plugin_load" {
            let source = args
                .get("source")
                .and_then(Value::as_str)
                .ok_or_else(|| error("invalid_arguments", "Plugin manifest source required"));
            match source.and_then(|path| {
                self.plugins
                    .lock()
                    .unwrap()
                    .load(std::path::Path::new(path))
            }) {
                Ok(()) => {
                    let state = self.shared.lock().unwrap();
                    Ok(Response {
                        revision: state.revision,
                        dirty: state.dirty(),
                        result: json!({}),
                    })
                }
                Err(e) => Err(e),
            }
        } else if let Some((id, action)) = resolved {
            let snapshot = self.shared.lock().unwrap().clone();
            if expected.is_some_and(|r| r != snapshot.revision) {
                Err(error("revision_conflict", "Document changed"))
            } else {
                let output = self.plugins.lock().unwrap().invoke_output(
                    &id,
                    &action,
                    json!(args),
                    &snapshot,
                );
                match output {
                    Ok(output) => self
                        .shared
                        .lock()
                        .unwrap()
                        .dispatch(
                            Command::Transaction {
                                commands: output.commands,
                            },
                            Some(snapshot.revision),
                        )
                        .map(|mut r| {
                            r.result = output.result;
                            r
                        }),
                    Err(e) => Err(e),
                }
            }
        } else {
            self.shared
                .lock()
                .unwrap()
                .call(&name, Value::Object(args), expected)
        };
        if let Some(ctx) = &self.repaint {
            ctx.request_repaint();
        }
        let response = match result {
            Ok(value) => CallToolResult::structured(json!(value)),
            Err(e) => {
                CallToolResult::error(vec![ContentBlock::text(serde_json::to_string(&e).unwrap())])
            }
        };
        Ok(response.into())
    }
}
pub async fn stdio(server: ErluMcp) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
pub async fn http(
    server: ErluMcp,
    token: String,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:38479").await?;
    http_on(server, token, listener).await
}
async fn http_on(
    server: ErluMcp,
    token: String,
    listener: tokio::net::TcpListener,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use axum::{
        Router,
        extract::{Request, State},
        http::StatusCode,
        middleware::{self, Next},
        response::Response,
    };
    use rmcp::transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    };
    async fn guard(
        State(token): State<String>,
        request: Request,
        next: Next,
    ) -> std::result::Result<Response, StatusCode> {
        let auth = request
            .headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok());
        if auth != Some(format!("Bearer {token}").as_str()) {
            return Err(StatusCode::UNAUTHORIZED);
        }
        if let Some(origin) = request.headers().get("origin") {
            let origin = origin.to_str().map_err(|_| StatusCode::FORBIDDEN)?;
            if origin != "http://127.0.0.1:38479" && origin != "http://localhost:38479" {
                return Err(StatusCode::FORBIDDEN);
            }
        }
        Ok(next.run(request).await)
    }
    let service = StreamableHttpService::new(
        move || Ok(server.clone()),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    );
    let app = Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn_with_state(token, guard));
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn http_requires_token_and_rejects_untrusted_origin() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = ErluMcp {
            shared: Arc::new(Mutex::new(Engine::default())),
            plugins: Arc::new(Mutex::new(erlu_plugins::PluginHost::default())),
            repaint: None,
        };
        let task = tokio::spawn(async move {
            http_on(server, "integration-token".into(), listener)
                .await
                .unwrap();
        });
        async fn send(address: std::net::SocketAddr, headers: &str) -> String {
            send_body(address, headers, "").await
        }
        async fn send_body(address: std::net::SocketAddr, headers: &str, body: &str) -> String {
            let mut socket = loop {
                if let Ok(s) = tokio::net::TcpStream::connect(address).await {
                    break s;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            };
            let request = format!(
                "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:38479\r\nConnection: close\r\nContent-Length: {}\r\n{headers}\r\n{body}",
                body.len()
            );
            socket.write_all(request.as_bytes()).await.unwrap();
            let mut bytes = vec![];
            tokio::time::timeout(
                std::time::Duration::from_secs(3),
                socket.read_to_end(&mut bytes),
            )
            .await
            .unwrap()
            .unwrap();
            String::from_utf8(bytes).unwrap()
        }
        assert!(send(address, "").await.starts_with("HTTP/1.1 401"));
        assert!(
            send(
                address,
                "Authorization: Bearer integration-token\r\nOrigin: https://untrusted.example\r\n"
            )
            .await
            .starts_with("HTTP/1.1 403")
        );
        let headers = "Authorization: Bearer integration-token\r\nAccept: application/json, text/event-stream\r\nContent-Type: application/json\r\n";
        let initialized=send_body(address,headers,r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"erlu-test","version":"1"}}}"#).await;
        assert!(initialized.starts_with("HTTP/1.1 200"), "{initialized}");
        assert!(initialized.contains("protocolVersion"));
        task.abort();
    }
}
