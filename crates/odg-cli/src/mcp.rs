//! MCP server (JSON-RPC 2.0 over newline-delimited stdio), implemented directly on the spec so
//! the binary stays small and synchronous: the hidden webview owns the main thread and every
//! tool call is a blocking render on this worker thread.
//!
//! Design goal: one tool call gives an agent everything it needs — validity, a numbered problem
//! list with lines and fixes, and an image where the same numbers are drawn on the diagram.

use std::io::{BufRead, Read, Write};

use base64::Engine as _;
use odg_core::analyze::{excerpt, Analysis};
use odg_core::engine::RenderBackend as _;
use odg_core::model::RawRender;
use odg_core::channel::Message;
use odg_core::workspace::{Actor, Workspace};
use odg_core::{analyze, engine_for_path, syntax, AnalyzeOptions, Lang, MermaidEngine, Report, Severity};
use odg_render::Renderer;
use serde_json::{json, Value};

const PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

const INSTRUCTIONS: &str = "ODiagramador renders Mermaid diagrams offline and tells you what is wrong with them.\n\
Workflow: (1) unsure about syntax? call get_syntax_help; (2) create_diagram / update_diagram / patch_diagram \
save the file AND return the validation report plus an image; (3) look at the image: problems are numbered \
markers that match the report; (4) fix errors first (line numbers given), then warnings; (5) call get_feedback \
to read comments the human left in the desktop viewer, and resolve_feedback when done. \
Talking to the human: open_diagram / show_diagram change the diagram open in their desktop viewer (show_diagram can highlight elements with a note); \
set_zoom changes the zoom (factor, \"150%\", fit, in, out) and center_on_element scrolls an element to the middle of the screen; \
highlight_elements puts elements in evidence; notify_user shows a message; ask_user asks a question (optionally with choices) \
and waits for the answer; get_user_replies reads answers later; get_viewer_status tells whether the human has the viewer open \
and what they are looking at. Write tools accept show=true to open the changed diagram for the human. \
validate_diagram checks text without saving. arrange_diagram fixes the layout (ELK, drag-like positions) without touching the diagram content. inspect_diagram describes the layout in text for agents without vision. \
Diagrams are plain .mmd files in the workspace; the human sees your changes live in the ODiagramador app.";

struct Server {
    ws: Workspace,
    renderer: Renderer,
    lang: Lang,
    client: Option<String>,
}

pub fn serve(ws: Workspace, renderer: Renderer, lang: Lang) {
    let mut server = Server { ws, renderer, lang, client: None };
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let responses = match serde_json::from_str::<Value>(&line) {
            Ok(Value::Array(batch)) => batch.into_iter().filter_map(|m| server.dispatch(m)).collect(),
            Ok(msg) => server.dispatch(msg).into_iter().collect(),
            Err(e) => vec![json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": format!("parse error: {e}")}})],
        };
        for r in responses {
            let _ = writeln!(stdout, "{r}");
            let _ = stdout.flush();
        }
    }
}

/// MCP "Streamable HTTP" transport (POST JSON-RPC, JSON responses; no server-initiated SSE).
/// `token`: required `Authorization: Bearer <token>`; mandatory when listening beyond loopback.
pub fn serve_http(ws: Workspace, renderer: Renderer, lang: Lang, addr: &str, token: Option<String>) -> Result<(), String> {
    let http = tiny_http::Server::http(addr).map_err(|e| format!("cannot listen on {addr}: {e}"))?;
    eprintln!("odg: MCP over HTTP at http://{addr}/mcp{}", if token.is_some() { " (bearer token required)" } else { "" });
    let mut server = Server { ws, renderer, lang, client: None };
    let header = |name: &str, value: &str| tiny_http::Header::from_bytes(name.as_bytes(), value.as_bytes()).unwrap();
    for mut req in http.incoming_requests() {
        let get = |name: &str| {
            req.headers()
                .iter()
                .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
                .map(|h| h.value.as_str().to_string())
        };
        let path = req.url().split('?').next().unwrap_or("").to_string();
        let method = req.method().clone();
        if path == "/health" {
            let _ = req.respond(tiny_http::Response::from_string("ok"));
            continue;
        }
        // auth first; without a token, refuse browser origins (DNS-rebinding protection)
        let authorized = match &token {
            Some(t) => get("authorization").is_some_and(|v| constant_eq(v.trim(), &format!("Bearer {t}"))),
            None => get("origin").is_none_or(|o| is_local_origin(&o)),
        };
        if !authorized {
            let _ = req.respond(tiny_http::Response::from_string("unauthorized").with_status_code(401).with_header(header("WWW-Authenticate", "Bearer")));
            continue;
        }
        if path != "/mcp" && path != "/" {
            let _ = req.respond(tiny_http::Response::from_string("not found").with_status_code(404));
            continue;
        }
        match method {
            tiny_http::Method::Post => {
                let mut body = String::new();
                if req.as_reader().take(32 * 1024 * 1024).read_to_string(&mut body).is_err() {
                    let _ = req.respond(tiny_http::Response::from_string("bad body").with_status_code(400));
                    continue;
                }
                let parsed = serde_json::from_str::<Value>(&body);
                let is_init = matches!(&parsed, Ok(v) if v.get("method").and_then(Value::as_str) == Some("initialize"));
                let reply: Option<Value> = match parsed {
                    Ok(Value::Array(batch)) => {
                        let out: Vec<Value> = batch.into_iter().filter_map(|m| server.dispatch(m)).collect();
                        (!out.is_empty()).then_some(Value::Array(out))
                    }
                    Ok(msg) => server.dispatch(msg),
                    Err(e) => Some(json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": format!("parse error: {e}")}})),
                };
                let resp = match reply {
                    None => tiny_http::Response::from_string("").with_status_code(202),
                    Some(v) => {
                        let mut r = tiny_http::Response::from_string(v.to_string()).with_header(header("Content-Type", "application/json"));
                        if is_init {
                            r = r.with_header(header("Mcp-Session-Id", &session_id()));
                        }
                        r
                    }
                };
                let _ = req.respond(resp);
            }
            tiny_http::Method::Delete => {
                let _ = req.respond(tiny_http::Response::from_string("").with_status_code(200));
            }
            _ => {
                // no server-initiated stream: clients fall back to plain POST
                let _ = req.respond(tiny_http::Response::from_string("method not allowed").with_status_code(405).with_header(header("Allow", "POST, DELETE")));
            }
        }
    }
    Ok(())
}

fn constant_eq(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn is_local_origin(origin: &str) -> bool {
    let host = origin.split("://").nth(1).unwrap_or(origin).split(['/', ':']).next().unwrap_or("");
    matches!(host, "localhost" | "127.0.0.1" | "[::1]" | "::1")
}

fn session_id() -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    odg_core::sha256_hex(format!("{now}{}", std::process::id()).as_bytes())[..32].to_string()
}

fn ok(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn err(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message.into()}})
}

struct ToolError(String);

impl<E: std::fmt::Display> From<E> for ToolError {
    fn from(e: E) -> Self {
        ToolError(e.to_string())
    }
}

type ToolResult = Result<Value, ToolError>;

impl Server {
    fn dispatch(&mut self, msg: Value) -> Option<Value> {
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("").to_string();
        let params = msg.get("params").cloned().unwrap_or(json!({}));
        let id = id?; // notifications (initialized, cancelled, ...) need no answer
        let res = match method.as_str() {
            "initialize" => {
                self.client = params.pointer("/clientInfo/name").and_then(Value::as_str).map(str::to_string);
                let asked = params.get("protocolVersion").and_then(Value::as_str).unwrap_or("");
                let version = PROTOCOL_VERSIONS.iter().find(|v| **v == asked).unwrap_or(&PROTOCOL_VERSIONS[0]);
                ok(
                    id,
                    json!({
                        "protocolVersion": version,
                        "capabilities": {"tools": {}, "resources": {}, "prompts": {}},
                        "serverInfo": {"name": "odiagramador", "title": "ODiagramador", "version": odg_core::VERSION},
                        "instructions": INSTRUCTIONS,
                    }),
                )
            }
            "ping" => ok(id, json!({})),
            "tools/list" => ok(id, json!({"tools": tool_definitions()})),
            "tools/call" => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                match self.call_tool(name, &args) {
                    Ok(v) => ok(id, v),
                    Err(ToolError(m)) => ok(id, json!({"content": [{"type": "text", "text": format!("Error: {m}")}], "isError": true})),
                }
            }
            "resources/list" => match self.resources() {
                Ok(v) => ok(id, json!({"resources": v})),
                Err(ToolError(m)) => err(id, -32603, m),
            },
            "resources/templates/list" => ok(id, json!({"resourceTemplates": [
                {"uriTemplate": "diagram://{path}", "name": "Diagram source", "mimeType": "text/vnd.mermaid"},
                {"uriTemplate": "diagram://{path}#report", "name": "Last validation report", "mimeType": "application/json"},
                {"uriTemplate": "diagram://{path}#preview", "name": "Annotated PNG preview", "mimeType": "image/png"},
            ]})),
            "resources/read" => {
                let uri = params.get("uri").and_then(Value::as_str).unwrap_or("");
                match self.read_resource(uri) {
                    Ok(v) => ok(id, json!({"contents": [v]})),
                    Err(ToolError(m)) => err(id, -32002, m),
                }
            }
            "prompts/list" => ok(id, json!({"prompts": prompt_definitions()})),
            "prompts/get" => match self.prompt(&params) {
                Ok(v) => ok(id, v),
                Err(ToolError(m)) => err(id, -32602, m),
            },
            "logging/setLevel" => ok(id, json!({})),
            _ => err(id, -32601, format!("method not found: {method}")),
        };
        Some(res)
    }

    fn opts(&self, args: &Value) -> AnalyzeOptions {
        AnalyzeOptions {
            lang: args.get("lang").and_then(Value::as_str).map(Lang::parse).unwrap_or(self.lang),
            theme: args.get("theme").and_then(Value::as_str).unwrap_or("default").to_string(),
            png: true,
            annotated: true,
            scale: args.get("scale").and_then(Value::as_f64).unwrap_or(1.5).clamp(0.5, 4.0),
            font: self.ws.config().font,
            ..Default::default()
        }
    }

    fn run(&self, path: &str, src: &str, args: &Value) -> Analysis {
        let engine = engine_for_path(path).unwrap_or(&MermaidEngine);
        analyze(engine, src, &self.renderer, &self.opts(args))
    }

    fn call_tool(&mut self, name: &str, args: &Value) -> ToolResult {
        let s = |k: &str| args.get(k).and_then(Value::as_str);
        let b = |k: &str, d: bool| args.get(k).and_then(Value::as_bool).unwrap_or(d);
        match name {
            "create_diagram" => {
                let source = s("source").ok_or(ToolError("`source` is required".into()))?;
                let path = match (s("path"), s("name")) {
                    (Some(p), _) => p.to_string(),
                    (None, Some(n)) => format!("diagrams/{}.mmd", slug(n)),
                    _ => return Err(ToolError("give `path` (e.g. diagrams/flow.mmd) or `name`".into())),
                };
                let path = if engine_for_path(&path).is_none() { format!("{path}.mmd") } else { path };
                if self.ws.exists(&path) && !b("overwrite", false) {
                    return Err(ToolError(format!("{path} already exists; use update_diagram or overwrite=true")));
                }
                self.save_and_report(&path, source, args)
            }
            "update_diagram" => {
                let path = s("path").ok_or(ToolError("`path` is required".into()))?;
                let source = s("source").ok_or(ToolError("`source` is required".into()))?;
                if !self.ws.exists(path) {
                    return Err(ToolError(format!("{path} does not exist; use create_diagram")));
                }
                self.save_and_report(path, source, args)
            }
            "patch_diagram" => {
                let path = s("path").ok_or(ToolError("`path` is required".into()))?;
                let edits: Vec<(String, String)> = args
                    .get("edits")
                    .and_then(Value::as_array)
                    .ok_or(ToolError("`edits` must be a list of {old_text, new_text}".into()))?
                    .iter()
                    .map(|e| {
                        (
                            e.get("old_text").and_then(Value::as_str).unwrap_or("").to_string(),
                            e.get("new_text").and_then(Value::as_str).unwrap_or("").to_string(),
                        )
                    })
                    .collect();
                let current = self.ws.read(path)?;
                let next = odg_core::source::apply_edits(&current, &edits).map_err(ToolError)?;
                self.save_and_report(path, &next, args)
            }
            "arrange_diagram" => {
                let path = s("path").ok_or(ToolError("`path` is required".into()))?;
                let mut src = self.ws.read(path)?;
                if b("reset_positions", false) {
                    src = odg_core::source::clear_positions(&src, None);
                }
                if let Some(mode) = s("mode") {
                    src = odg_core::source::set_layout_mode(&src, mode).map_err(ToolError)?;
                }
                for p in args.get("positions").and_then(Value::as_array).into_iter().flatten() {
                    let id = p.get("id").and_then(Value::as_str).ok_or(ToolError("each position needs `id`".into()))?;
                    let x = p.get("x").and_then(Value::as_f64).ok_or(ToolError(format!("position of {id} needs `x`")))?;
                    let y = p.get("y").and_then(Value::as_f64).ok_or(ToolError(format!("position of {id} needs `y`")))?;
                    src = odg_core::source::set_node_position(&src, id, x, y).map_err(ToolError)?;
                }
                let mut numbered = false;
                for l in args.get("layers").and_then(Value::as_array).into_iter().flatten() {
                    let id = l.get("id").and_then(Value::as_str).ok_or(ToolError("each layer needs `id`".into()))?;
                    let layer = match l.get("layer") {
                        Some(Value::Number(n)) => n.as_u64().map(|n| n.to_string()),
                        Some(Value::String(v)) if v != "none" => Some(v.clone()),
                        _ => None,
                    };
                    numbered |= layer.as_deref().is_some_and(|v| v.parse::<u32>().is_ok());
                    src = odg_core::source::set_node_layer(&src, id, layer.as_deref()).map_err(ToolError)?;
                }
                // layer constraints need ELK
                if (numbered || args.get("layers").is_some_and(|l| l.as_array().is_some_and(|a| !a.is_empty())))
                    && odg_core::source::layout_directives(&src).mode == "auto"
                {
                    src = odg_core::source::set_layout_mode(&src, "elk").map_err(ToolError)?;
                }
                self.save_and_report(path, &src, args)
            }
            "add_element" => {
                let path = s("path").ok_or(ToolError("`path` is required".into()))?;
                let construct = s("construct").ok_or(ToolError("`construct` is required".into()))?;
                let params = args.get("params").cloned().unwrap_or(json!({}));
                let src = self.ws.read(path)?;
                let next = odg_core::construct::apply(&src, construct, &params).map_err(ToolError)?;
                self.save_and_report(path, &next, args)
            }
            "edit_element" => {
                let path = s("path").ok_or(ToolError("`path` is required".into()))?;
                let action = s("action").ok_or(ToolError("`action` is required".into()))?;
                let params = args.get("params").cloned().unwrap_or(json!({}));
                let src = self.ws.read(path)?;
                let next = odg_core::edit::apply(&src, action, &params).map_err(ToolError)?;
                self.save_and_report(path, &next, args)
            }
            "validate_diagram" => {
                let source = s("source").ok_or(ToolError("`source` is required".into()))?;
                let a = self.run("inline.mmd", source, args);
                Ok(self.respond(&a, source, None, b("include_image", true)))
            }
            "render_diagram" => {
                let (path, source) = self.source_arg(args)?;
                let a = self.run(path.as_deref().unwrap_or("inline.mmd"), &source, args);
                if s("format") == Some("svg") {
                    let svg = a.svg.clone().ok_or(ToolError(a.report.to_markdown()))?;
                    return Ok(json!({"content": [
                        {"type": "text", "text": a.report.to_markdown()},
                        {"type": "text", "text": svg},
                    ]}));
                }
                if s("focus").is_some() || args.get("region").is_some() {
                    return self.render_zoomed(&a, args);
                }
                let annotated = b("annotated", true);
                let png = if annotated { a.annotated_png.clone().or(a.png.clone()) } else { a.png.clone().or(a.annotated_png.clone()) };
                let mut content = vec![json!({"type": "text", "text": a.report.to_markdown()})];
                if let Some(p) = png {
                    content.push(image(&p));
                }
                Ok(json!({"content": content, "structuredContent": a.report}))
            }
            "inspect_diagram" => {
                let (path, source) = self.source_arg(args)?;
                let a = self.run(path.as_deref().unwrap_or("inline.mmd"), &source, args);
                Ok(json!({
                    "content": [{"type": "text", "text": describe(&a.raw, &a.report)}],
                    "structuredContent": {"report": a.report, "model": a.raw.model, "width": a.raw.width, "height": a.raw.height},
                }))
            }
            "list_diagrams" => {
                let list = self.ws.list()?;
                let mut text = format!("{} diagram(s) in {}\n", list.len(), self.ws.root().display());
                for d in &list {
                    let st = match &d.status {
                        Some(s) if s.stale => "? not validated since last change".to_string(),
                        Some(s) if s.valid => format!("✅ {} warning(s)", s.warnings),
                        Some(s) => format!("❌ {} error(s)", s.errors),
                        None => "? never validated".to_string(),
                    };
                    let fb = if d.open_feedback > 0 { format!(" · 💬 {} open comment(s)", d.open_feedback) } else { String::new() };
                    text.push_str(&format!("- {} — {st}{fb}\n", d.path));
                }
                Ok(json!({"content": [{"type": "text", "text": text}], "structuredContent": {"diagrams": list}}))
            }
            "get_diagram" => {
                let path = s("path").ok_or(ToolError("`path` is required".into()))?;
                let rel = self.ws.rel(path)?;
                let source = self.ws.read(&rel)?;
                let numbered: String = source.lines().enumerate().map(|(i, l)| format!("{:>4} | {l}\n", i + 1)).collect();
                let report = self.ws.load_report(&rel)?;
                let feedback = self.ws.feedback(Some(&rel), false)?;
                let mut text = format!("{rel}\n```mermaid\n{numbered}```\n");
                match (&report, self.ws.status(&rel)) {
                    (Some(r), Some(st)) => text.push_str(&format!("Last report{}:\n{}", if st.stale { " (stale: file changed since)" } else { "" }, r.to_markdown())),
                    _ => text.push_str("Not validated yet: call render_diagram or update_diagram.\n"),
                }
                if !feedback.is_empty() {
                    text.push_str(&format!("\n💬 {} open comment(s) from the human:\n", feedback.len()));
                    for f in &feedback {
                        text.push_str(&format!("- [{}] {}{}\n", f.id, f.element_id.as_ref().map(|e| format!("(on {e}) ")).unwrap_or_default(), f.text));
                    }
                }
                Ok(json!({"content": [{"type": "text", "text": text}], "structuredContent": {"path": rel, "source": source, "report": report, "feedback": feedback}}))
            }
            "delete_diagram" => {
                let path = s("path").ok_or(ToolError("`path` is required".into()))?;
                self.ws.delete(path, Actor::Agent)?;
                Ok(text(format!("deleted {path}")))
            }
            "get_feedback" => {
                let items = self.ws.feedback(s("path"), b("include_resolved", false))?;
                let mut t = if items.is_empty() { "No open comments.".to_string() } else { format!("{} comment(s):\n", items.len()) };
                for f in &items {
                    t.push_str(&format!(
                        "- [{}] {} {}{}{}: {}\n",
                        f.id,
                        if f.resolved { "✔" } else { "•" },
                        f.path,
                        f.element_id.as_ref().map(|e| format!(" (element {e})")).unwrap_or_default(),
                        f.line.map(|l| format!(" (line {l})")).unwrap_or_default(),
                        f.text
                    ));
                }
                Ok(json!({"content": [{"type": "text", "text": t}], "structuredContent": {"feedback": items}}))
            }
            "resolve_feedback" => {
                let id = s("id").ok_or(ToolError("`id` is required".into()))?;
                let f = self.ws.resolve_feedback(id, s("resolution"), Actor::Agent)?;
                Ok(text(format!("resolved {} on {}", f.id, f.path)))
            }
            "get_activity" => {
                let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(20) as usize;
                let events = self.ws.events(limit, s("path"))?;
                let t: String = events
                    .iter()
                    .map(|e| format!("- {} {:?} {} {}{}\n", e.ts, e.actor, e.action, e.path, e.valid.map(|v| if v { " ✅" } else { " ❌" }).unwrap_or("")))
                    .collect();
                Ok(json!({"content": [{"type": "text", "text": if t.is_empty() { "No activity yet.".into() } else { t }}], "structuredContent": {"events": events}}))
            }
            "get_syntax_help" => {
                let topic = s("topic").unwrap_or("overview");
                match syntax::help(topic) {
                    Some(doc) => Ok(text(doc.to_string())),
                    None => Ok(text(format!("No cheat-sheet for `{topic}`. Topics: {}", syntax::topics().join(", ")))),
                }
            }
            "show_diagram" | "open_in_viewer" => {
                let path = s("path").ok_or(ToolError("`path` is required".into()))?;
                if !self.ws.exists(path) {
                    return Err(ToolError(format!("{path} does not exist")));
                }
                let ids = strings(args, "element_ids");
                let kind = if ids.is_empty() { "show" } else { "highlight" };
                let (msg, note) = self.tell_viewer(Message { kind: kind.into(), path: Some(path.into()), element_ids: ids, text: s("message").map(str::to_string), level: s("level").map(str::to_string), ..Default::default() })?;
                Ok(json!({"content": [{"type": "text", "text": note}], "structuredContent": {"message": msg}}))
            }
            "open_diagram" => {
                let path = s("path").ok_or(ToolError("`path` is required".into()))?;
                if !self.ws.exists(path) {
                    return Err(ToolError(format!("{path} does not exist")));
                }
                let (msg, note) = self.tell_viewer(Message { kind: "show".into(), path: Some(path.into()), text: s("message").map(str::to_string), ..Default::default() })?;
                Ok(json!({"content": [{"type": "text", "text": note}], "structuredContent": {"message": msg}}))
            }
            "set_zoom" => {
                let current = self.ws.presence().filter(|p| p.is_live()).and_then(|p| p.zoom).unwrap_or(1.0);
                let (zoom, fit) = match args.get("zoom") {
                    // 150 = 150%, 1.5 = factor
                    Some(Value::Number(n)) => (n.as_f64().map(|x| if x > 8.0 { x / 100.0 } else { x }), false),
                    Some(Value::String(v)) => match v.trim().trim_end_matches('%') {
                        "fit" => (None, true),
                        "in" => (Some(current * 1.25), false),
                        "out" => (Some(current / 1.25), false),
                        other => match other.parse::<f64>() {
                            // "150" or "150%" = percent, "1.5" = factor
                            Ok(x) if v.ends_with('%') || x > 8.0 => (Some(x / 100.0), false),
                            Ok(x) => (Some(x), false),
                            Err(_) => return Err(ToolError("`zoom`: a factor (1 = 100%), a percentage like \"150%\", or fit | in | out".into())),
                        },
                    },
                    _ => return Err(ToolError("`zoom` is required: a factor (1 = 100%), \"150%\", or fit | in | out".into())),
                };
                let zoom = zoom.map(|z| z.clamp(0.05, 8.0));
                let (msg, mut note) = self.tell_viewer(Message { kind: "view".into(), path: s("path").map(str::to_string), zoom, fit: fit.then_some(true), center_on: s("center_on").map(str::to_string), ..Default::default() })?;
                if let Some(z) = zoom {
                    note.push_str(&format!(" Zoom: {:.0}%.", z * 100.0));
                }
                Ok(json!({"content": [{"type": "text", "text": note}], "structuredContent": {"message": msg}}))
            }
            "center_on_element" => {
                let id = s("element_id").ok_or(ToolError("`element_id` is required (a node id from the source / inspect_diagram)".into()))?;
                let zoom = args.get("zoom").and_then(Value::as_f64).map(|z| z.clamp(0.05, 8.0));
                let (msg, mut note) = self.tell_viewer(Message { kind: "view".into(), path: s("path").map(str::to_string), zoom, center_on: Some(id.into()), text: s("message").map(str::to_string), ..Default::default() })?;
                let path = s("path").map(str::to_string).or_else(|| self.ws.presence().and_then(|p| p.path));
                if let Some(src) = path.as_deref().and_then(|p| self.ws.read(p).ok()) {
                    if !mentions_id(&src, id) {
                        note.push_str(&format!("\nWarning: `{id}` was not found in {}.", path.unwrap_or_default()));
                    }
                }
                Ok(json!({"content": [{"type": "text", "text": note}], "structuredContent": {"message": msg}}))
            }
            "highlight_elements" => {
                let path = s("path").ok_or(ToolError("`path` is required".into()))?;
                let ids = strings(args, "element_ids");
                if ids.is_empty() {
                    return Err(ToolError("`element_ids` must list at least one node/edge id (see inspect_diagram)".into()));
                }
                let src = self.ws.read(path)?;
                let unknown: Vec<&String> = ids.iter().filter(|id| !mentions_id(&src, id) && !id.starts_with("L_")).collect();
                let (msg, mut note) = self.tell_viewer(Message { kind: "highlight".into(), path: Some(path.into()), element_ids: ids.clone(), text: s("message").map(str::to_string), level: s("level").map(str::to_string), ..Default::default() })?;
                if !unknown.is_empty() {
                    note.push_str(&format!("\nWarning: not found in the source: {} (use the ids from inspect_diagram).", unknown.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")));
                }
                Ok(json!({"content": [{"type": "text", "text": note}], "structuredContent": {"message": msg}}))
            }
            "notify_user" => {
                let message = s("message").ok_or(ToolError("`message` is required".into()))?;
                let (msg, note) = self.tell_viewer(Message { kind: "notify".into(), path: s("path").map(str::to_string), element_ids: strings(args, "element_ids"), text: Some(message.into()), level: s("level").map(str::to_string), ..Default::default() })?;
                Ok(json!({"content": [{"type": "text", "text": note}], "structuredContent": {"message": msg}}))
            }
            "ask_user" => {
                let question = s("question").ok_or(ToolError("`question` is required".into()))?;
                let options = strings(args, "options");
                let allow_text = b("allow_text", true) || options.is_empty();
                let wait = args.get("wait_seconds").and_then(Value::as_u64).unwrap_or(60).min(600);
                let (q, note) = self.tell_viewer(Message { kind: "ask".into(), path: s("path").map(str::to_string), element_ids: strings(args, "element_ids"), text: Some(question.into()), options, allow_text: Some(allow_text), level: Some("info".into()), ..Default::default() })?;
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(wait);
                loop {
                    if let Some(a) = self.ws.answer_to(&q.id)? {
                        let reply = a.text.clone().unwrap_or_default();
                        return Ok(json!({"content": [{"type": "text", "text": format!("The user answered: {reply}")}],
                            "structuredContent": {"answered": true, "question_id": q.id, "answer": reply, "answered_at": a.ts}}));
                    }
                    if std::time::Instant::now() >= deadline {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(400));
                }
                Ok(json!({"content": [{"type": "text", "text": format!("{note}\nNo answer yet after {wait}s. The question stays open in the viewer; call get_user_replies with question_id {} later (or continue and check back).", q.id)}],
                    "structuredContent": {"answered": false, "question_id": q.id}}))
            }
            "get_user_replies" => {
                if let Some(id) = s("question_id") {
                    return Ok(match self.ws.answer_to(id)? {
                        Some(a) => json!({"content": [{"type": "text", "text": format!("Answer to {id}: {}", a.text.clone().unwrap_or_default())}],
                            "structuredContent": {"answered": true, "question_id": id, "answer": a.text, "answered_at": a.ts}}),
                        None => json!({"content": [{"type": "text", "text": format!("{id}: not answered yet.")}],
                            "structuredContent": {"answered": false, "question_id": id}}),
                    });
                }
                let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(20) as usize;
                let messages = self.ws.recent_messages(limit)?;
                let mut t: String = messages
                    .iter()
                    .map(|m| {
                        let who = if m.from == Some(Actor::Human) { "human" } else { "agent" };
                        let about = m.path.as_deref().map(|p| format!(" [{p}]")).unwrap_or_default();
                        let re = m.reply_to.as_deref().map(|r| format!(" (reply to {r})")).unwrap_or_default();
                        format!("- {} {who} {} {}{about}{re}: {}\n", m.ts, m.kind, m.id, m.text.as_deref().unwrap_or(""))
                    })
                    .collect();
                if t.is_empty() {
                    t = "No messages yet.".into();
                }
                let open = self.ws.feedback(None, false).map(|f| f.len()).unwrap_or(0);
                if open > 0 {
                    t.push_str(&format!("\n{open} open comment(s) from the human on diagrams: call get_feedback.\n"));
                }
                Ok(json!({"content": [{"type": "text", "text": t}], "structuredContent": {"messages": messages}}))
            }
            "get_viewer_status" => {
                let p = self.ws.presence();
                let live = p.as_ref().is_some_and(|p| p.is_live());
                let t = match (&p, live) {
                    (Some(p), true) => format!(
                        "The human has the ODiagramador viewer open ({} {} on {}), looking at {}{}{}{}{}.",
                        p.app, p.version, p.platform,
                        p.path.as_deref().unwrap_or("no diagram"),
                        p.zoom.map(|z| format!(" at {:.0}% zoom", z * 100.0)).unwrap_or_default(),
                        p.center_element.as_deref().map(|e| format!(", centred on `{e}`")).unwrap_or_default(),
                        if p.visible_elements.is_empty() { String::new() } else { format!(", {} element(s) on screen: {}", p.visible_elements.len(), p.visible_elements.join(", ")) },
                        p.element_id.as_deref().map(|e| format!("; element `{e}` selected")).unwrap_or_default()
                    ),
                    _ => "The ODiagramador viewer is not open for this workspace. show_diagram / highlight_elements / ask_user will try to launch it.".into(),
                };
                Ok(json!({"content": [{"type": "text", "text": t}], "structuredContent": {"open": live, "presence": p}}))
            }
            _ => Err(ToolError(format!("unknown tool: {name}"))),
        }
    }

    /// render_diagram with `focus` (element / subgraph id) or `region`: only that part, at a legible scale.
    fn render_zoomed(&self, a: &Analysis, args: &Value) -> ToolResult {
        use odg_core::model::BBox;
        let svg = a.svg.clone().ok_or(ToolError(a.report.to_markdown()))?;
        let model = &a.raw.model;
        let (target, what) = if let Some(id) = args.get("focus").and_then(Value::as_str) {
            let b = model.node(id).and_then(|n| n.bbox).ok_or_else(|| {
                let ids: Vec<&str> = model.groups.iter().chain(model.nodes.iter()).filter(|n| n.bbox.is_some()).map(|n| n.id.as_str()).take(40).collect();
                ToolError(format!("`{id}` has no position in the drawing. Elements you can focus: {}", ids.join(", ")))
            })?;
            (b, format!("`{id}`"))
        } else {
            let r = args.get("region").cloned().unwrap_or_default();
            let f = |k: &str| r.get(k).and_then(Value::as_f64);
            match (f("x"), f("y"), f("w").or(f("width")), f("h").or(f("height"))) {
                (Some(x), Some(y), Some(w), Some(h)) if w > 0.0 && h > 0.0 => (BBox { x, y, w, h }, "the region".to_string()),
                _ => return Err(ToolError("`region` needs x, y, w, h (same coordinates as inspect_diagram)".into())),
            }
        };
        let pad = args.get("padding").and_then(Value::as_f64).unwrap_or(24.0).max(0.0);
        let mut region = BBox { x: target.x - pad, y: target.y - pad, w: target.w + 2.0 * pad, h: target.h + 2.0 * pad };
        if let Some(vb) = a.raw.view_box {
            let x0 = region.x.max(vb.x);
            let y0 = region.y.max(vb.y);
            region = BBox { x: x0, y: y0, w: (region.x + region.w).min(vb.x + vb.w) - x0, h: (region.y + region.h).min(vb.y + vb.h) - y0 };
        }
        // about 1600 px wide, never past 4000 px on either side
        let fit = (1600.0 / region.w.max(1.0)).clamp(1.0, 6.0);
        let scale = args.get("scale").and_then(Value::as_f64).unwrap_or(fit).min(4000.0 / region.w.max(region.h).max(1.0)).max(0.25);
        let dark = args.get("theme").and_then(Value::as_str) == Some("dark");
        let png = self.renderer.rasterize(&odg_core::analyze::crop_svg(&svg, region), scale, if dark { "#0d1117" } else { "#ffffff" }).map_err(ToolError)?;
        let inside: Vec<String> = a
            .report
            .problems
            .iter()
            .filter(|p| p.bbox.is_some_and(|b| b.intersection(&region) > 0.0))
            .map(|p| format!("{}. [{}] {}", p.index, p.code, p.message))
            .collect();
        let mut text = format!(
            "Zoom on {what}: region {:.0},{:.0} {:.0}×{:.0} of a {:.0}×{:.0} drawing, rendered at {scale:.1}×.\n",
            region.x, region.y, region.w, region.h, a.raw.width, a.raw.height
        );
        if inside.is_empty() {
            text.push_str("No reported problem inside this region.\n");
        } else {
            text.push_str(&format!("Problems inside this region:\n{}\n", inside.join("\n")));
        }
        Ok(json!({"content": [{"type": "text", "text": text}, image(&png)], "structuredContent": {"region": region, "scale": scale, "report": a.report}}))
    }

    fn source_arg(&self, args: &Value) -> Result<(Option<String>, String), ToolError> {
        match (args.get("path").and_then(Value::as_str), args.get("source").and_then(Value::as_str)) {
            (_, Some(src)) => Ok((args.get("path").and_then(Value::as_str).map(str::to_string), src.to_string())),
            (Some(p), None) => Ok((Some(p.to_string()), self.ws.read(p)?)),
            _ => Err(ToolError("give `path` or `source`".into())),
        }
    }

    fn save_and_report(&self, path: &str, source: &str, args: &Value) -> ToolResult {
        let rel = self.ws.write(path, source, Actor::Agent, self.client.as_deref())?;
        let mut a = self.run(&rel, source, args);
        self.ws.save_analysis(&rel, &mut a, Some(Actor::Agent))?;
        let mut out = self.respond(&a, source, Some(&rel), args.get("include_image").and_then(Value::as_bool).unwrap_or(true));
        if args.get("show").and_then(Value::as_bool).unwrap_or(false) {
            let ids = strings(args, "element_ids");
            let kind = if ids.is_empty() { "show" } else { "highlight" };
            let message = args.get("message").and_then(Value::as_str).map(str::to_string);
            let (_, note) = self.tell_viewer(Message { kind: kind.into(), path: Some(rel.clone()), element_ids: ids, text: message, ..Default::default() })?;
            if let Some(content) = out["content"].as_array_mut() {
                content.push(json!({"type": "text", "text": note}));
            }
        }
        Ok(out)
    }

    /// Posts a message for the human's viewer and makes sure it is seen: launches the app when it is
    /// closed (except for plain notifications). Returns the stored message and a note for the agent.
    fn tell_viewer(&self, mut msg: Message) -> Result<(Message, String), ToolError> {
        msg.from = Some(Actor::Agent);
        msg.client = self.client.clone();
        if !matches!(msg.level.as_deref(), None | Some("info" | "success" | "warning" | "error")) {
            msg.level = Some("info".into());
        }
        let msg = self.ws.post_message(msg)?;
        let live = self.ws.presence().is_some_and(|p| p.is_live());
        let what = match msg.kind.as_str() {
            "show" => format!("Opened {} in the human's viewer", msg.path.as_deref().unwrap_or("the diagram")),
            "highlight" => format!("Highlighted {} in {} for the human", msg.element_ids.join(", "), msg.path.as_deref().unwrap_or("the diagram")),
            "view" => match (&msg.center_on, msg.fit) {
                (Some(id), _) => format!("Centred the human's view on `{id}`"),
                (None, Some(true)) => "Fitted the diagram to the human's window".to_string(),
                _ => "Changed the zoom of the human's view".to_string(),
            },
            "ask" => format!("Asked the human (question id {})", msg.id),
            _ => "Sent the message to the human".to_string(),
        };
        if live {
            return Ok((msg, format!("{what}.")));
        }
        if msg.kind == "notify" {
            return Ok((msg, format!("{what}, but the viewer is closed: it will be shown if the app opens in the next {} s and stays in the conversation (get_user_replies).", odg_core::channel::LATE_DELIVERY_SECS)));
        }
        let note = match launch_viewer(self.ws.root(), msg.path.as_deref()) {
            Ok(()) => format!("{what}: the viewer was not showing this folder, so ODiagramador was asked to open it (the running window switches to it, or the app starts); it shows this as soon as it does."),
            Err(e) => format!("{what}, but the viewer is not open and could not be launched ({e}). The human will see it when they open ODiagramador on this folder."),
        };
        Ok((msg, note))
    }

    /// The standard answer: report text, next step, files, and the most useful image.
    fn respond(&self, a: &Analysis, source: &str, rel: Option<&str>, include_image: bool) -> Value {
        let r = &a.report;
        let mut t = r.to_markdown();
        if let Some(line) = r.first_error().and_then(|p| p.line) {
            t.push_str(&format!("\nSource around line {line}:\n```\n{}\n```\n", excerpt(source, line, 3)));
        }
        t.push_str(&next_step(r, Lang::parse(&r.lang)));
        if a.raw.width > 2000.0 || a.raw.height > 2000.0 {
            let groups: Vec<&str> = a.raw.model.groups.iter().map(|g| g.id.as_str()).take(12).collect();
            t.push_str(&format!(
                "\nThe drawing is {:.0}×{:.0} px: image viewers shrink it, so small text gets unreadable. Look at a part with render_diagram(focus=<element or subgraph id>){}.\n",
                a.raw.width,
                a.raw.height,
                if groups.is_empty() { String::new() } else { format!(" — subgraphs: {}", groups.join(", ")) }
            ));
        }
        if let Some(rel) = rel {
            let p = &r.previews;
            t.push_str(&format!("\nFiles: source `{rel}`"));
            if let Some(x) = &p.png {
                t.push_str(&format!(" · preview `{x}`"));
            }
            if let Some(x) = &p.annotated_png {
                t.push_str(&format!(" · annotated `{x}`"));
            }
            t.push_str(&format!(" · report `{}`\n", self.ws.report_path(rel).strip_prefix(self.ws.root()).unwrap_or(&self.ws.report_path(rel)).display()));
            if let Ok(fb) = self.ws.feedback(Some(rel), false) {
                if !fb.is_empty() {
                    t.push_str(&format!("💬 The human left {} open comment(s) on this diagram — call get_feedback.\n", fb.len()));
                }
            }
        }
        let mut content = vec![json!({"type": "text", "text": t})];
        if include_image {
            let has_markers = r.problems.iter().any(|p| p.bbox.is_some()) || !r.valid;
            let img = if has_markers { a.annotated_png.as_ref().or(a.png.as_ref()) } else { a.png.as_ref() };
            if let Some(png) = img {
                content.push(image(png));
            }
        }
        json!({"content": content, "structuredContent": r, "isError": false})
    }

    fn resources(&self) -> ToolResult {
        let mut out = vec![];
        for d in self.ws.list()? {
            out.push(json!({"uri": format!("diagram://{}", d.path), "name": d.path, "title": d.name, "mimeType": "text/vnd.mermaid"}));
            if d.status.is_some() {
                out.push(json!({"uri": format!("diagram://{}#report", d.path), "name": format!("{} (report)", d.path), "mimeType": "application/json"}));
                out.push(json!({"uri": format!("diagram://{}#preview", d.path), "name": format!("{} (preview)", d.path), "mimeType": "image/png"}));
            }
        }
        Ok(Value::Array(out))
    }

    fn read_resource(&self, uri: &str) -> ToolResult {
        let rest = uri.strip_prefix("diagram://").ok_or(ToolError(format!("unknown resource {uri}")))?;
        let (path, frag) = rest.split_once('#').unwrap_or((rest, ""));
        let rel = self.ws.rel(path)?;
        match frag {
            "" => Ok(json!({"uri": uri, "mimeType": "text/vnd.mermaid", "text": self.ws.read(&rel)?})),
            "report" => {
                let r = self.ws.load_report(&rel)?.ok_or(ToolError("no report yet".into()))?;
                Ok(json!({"uri": uri, "mimeType": "application/json", "text": serde_json::to_string_pretty(&r)?}))
            }
            "preview" => {
                let p = [self.ws.preview_path(&rel, "annotated.png"), self.ws.preview_path(&rel, "png")]
                    .into_iter()
                    .find(|p| p.is_file())
                    .ok_or(ToolError("no preview yet".into()))?;
                let bytes = std::fs::read(p)?;
                Ok(json!({"uri": uri, "mimeType": "image/png", "blob": base64::engine::general_purpose::STANDARD.encode(bytes)}))
            }
            other => Err(ToolError(format!("unknown fragment #{other}"))),
        }
    }

    fn prompt(&self, params: &Value) -> ToolResult {
        let name = params.get("name").and_then(Value::as_str).unwrap_or("");
        let args = params.get("arguments").cloned().unwrap_or(json!({}));
        let a = |k: &str| args.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        let text = match name {
            "fix_diagram_problems" => format!(
                "Fix the diagram `{}` in the ODiagramador workspace.\n\
                 1. Call get_diagram to read the numbered source, last report and human comments.\n\
                 2. Fix errors first, then warnings, using patch_diagram with small edits.\n\
                 3. After each change, check the returned report and the annotated image (numbers match the problem list).\n\
                 4. Resolve each human comment you addressed with resolve_feedback.\n\
                 Stop when the report is valid with no warnings, or explain which warnings are intentional.",
                a("path")
            ),
            "create_diagram_from_description" => format!(
                "Create a Mermaid {} diagram in the ODiagramador workspace that shows: {}\n\
                 Call get_syntax_help for the diagram type first, then create_diagram with a short `name`. \
                 Inspect the returned image and report; fix every error and warning with patch_diagram until it is clean.",
                if a("type").is_empty() { "flowchart".into() } else { a("type") },
                a("description")
            ),
            _ => return Err(ToolError(format!("unknown prompt {name}"))),
        };
        Ok(json!({"messages": [{"role": "user", "content": {"type": "text", "text": text}}]}))
    }
}

fn next_step(r: &Report, lang: Lang) -> String {
    let key = if !r.valid {
        "mcp-next-invalid"
    } else if r.problems.iter().any(|p| p.severity == Severity::Warning) {
        "mcp-next-warnings"
    } else {
        "mcp-next-clean"
    };
    format!("\n{}\n", odg_core::t(lang, key, &[]))
}

/// Text description of the layout for agents that cannot see images.
fn describe(raw: &RawRender, report: &Report) -> String {
    let m = &raw.model;
    let mut s = format!(
        "Type: {}\nCanvas: {:.0}×{:.0} px · {} nodes · {} edges · {} groups\n",
        report.diagram_type.as_deref().unwrap_or("?"),
        raw.width,
        raw.height,
        m.nodes.len(),
        m.edges.len(),
        m.groups.len()
    );
    if !report.valid {
        s.push_str(&report.to_markdown());
        return s;
    }
    s.push_str("\nNodes (id \"label\" @ x,y size):\n");
    for n in &m.nodes {
        let pos = n.bbox.map(|b| format!(" @ {:.0},{:.0} {:.0}×{:.0}", b.x, b.y, b.w, b.h)).unwrap_or_default();
        let parent = n.parent.as_ref().map(|p| format!(" in {p}")).unwrap_or_default();
        s.push_str(&format!("- {} \"{}\"{pos}{parent}\n", n.id, n.label));
    }
    if !m.groups.is_empty() {
        s.push_str("\nGroups:\n");
        for g in &m.groups {
            s.push_str(&format!("- {} \"{}\"\n", g.id, g.label));
        }
    }
    s.push_str("\nEdges:\n");
    for e in &m.edges {
        let label = if e.label.is_empty() { String::new() } else { format!(" \"{}\"", e.label) };
        s.push_str(&format!("- {} → {}{label}\n", e.from, e.to));
    }
    s.push('\n');
    s.push_str(&report.to_markdown());
    s
}

fn image(png: &[u8]) -> Value {
    json!({"type": "image", "mimeType": "image/png", "data": base64::engine::general_purpose::STANDARD.encode(png)})
}

/// Whether `id` occurs in the source as a whole identifier (not inside a longer word).
fn mentions_id(src: &str, id: &str) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '_' || c == '-';
    src.match_indices(id).any(|(i, _)| {
        let before = src[..i].chars().next_back();
        let after = src[i + id.len()..].chars().next();
        !before.is_some_and(word) && !after.is_some_and(word)
    })
}

/// A list of strings argument (also accepts a single string).
fn strings(args: &Value, key: &str) -> Vec<String> {
    match args.get(key) {
        Some(Value::Array(items)) => items.iter().filter_map(Value::as_str).map(str::to_string).filter(|s| !s.is_empty()).collect(),
        Some(Value::String(s)) if !s.is_empty() => vec![s.clone()],
        _ => vec![],
    }
}

fn text(t: String) -> Value {
    json!({"content": [{"type": "text", "text": t}]})
}

fn slug(name: &str) -> String {
    let mut s = String::new();
    for c in name.trim().chars() {
        if c.is_alphanumeric() {
            s.extend(c.to_lowercase());
        } else if !s.ends_with('-') && !s.is_empty() {
            s.push('-');
        }
    }
    let s = s.trim_end_matches('-').to_string();
    if s.is_empty() { "diagram".into() } else { s }
}

fn pct(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Starts the desktop viewer on `workspace` (the message itself reaches it through the workspace inbox).
fn launch_viewer(workspace: &std::path::Path, path: Option<&str>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        // `odg` sits inside ODiagramador.app, so LaunchServices may take this very process for the running
        // app and hand it the odiagramador:// URL. When the viewer is not running, start a new instance.
        let gui_running = std::process::Command::new("pgrep").args(["-x", "ODiagramador"]).output().map(|o| o.status.success()).unwrap_or(false);
        if !gui_running {
            let bundle = std::env::current_exe().ok().and_then(|exe| exe.ancestors().find(|p| p.extension().is_some_and(|e| e == "app")).map(|p| p.to_path_buf()));
            let mut cmd = std::process::Command::new("open");
            cmd.arg("-n");
            match &bundle {
                Some(b) => cmd.arg("-a").arg(b),
                None => cmd.args(["-b", "io.odiagramador.app"]),
            };
            cmd.arg("--args").arg("--workspace").arg(workspace);
            return match cmd.status() {
                Ok(s) if s.success() => Ok(()),
                Ok(_) => Err("the ODiagramador app is not installed".into()),
                Err(e) => Err(e.to_string()),
            };
        }
    }
    let mut url = format!("odiagramador://open?workspace={}", pct(&workspace.to_string_lossy()));
    if let Some(p) = path {
        url.push_str(&format!("&path={}", pct(p)));
    }
    open_url(&url)
}

fn open_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(url).status();
    // not `cmd /C start`: cmd would cut the URL at its `&`
    #[cfg(windows)]
    let r = std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", url]).status();
    #[cfg(all(unix, not(target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(url).status();
    match r {
        Ok(s) if s.success() => Ok(()),
        Ok(_) => Err("the ODiagramador app is not installed or did not register the odiagramador:// URL scheme".into()),
        Err(e) => Err(e.to_string()),
    }
}

fn prompt_definitions() -> Value {
    json!([
        {"name": "fix_diagram_problems", "title": "Fix diagram problems",
         "description": "Fix every error, warning and open human comment on a diagram",
         "arguments": [{"name": "path", "description": "Diagram path, e.g. diagrams/flow.mmd", "required": true}]},
        {"name": "create_diagram_from_description", "title": "Create a diagram",
         "description": "Create a clean Mermaid diagram from a description",
         "arguments": [{"name": "description", "required": true}, {"name": "type", "description": "flowchart, sequence, class, state, er…"}]}
    ])
}

fn tool_definitions() -> Value {
    let common = json!({
        "include_image": {"type": "boolean", "description": "Return a PNG (annotated with numbered problem markers when there are problems). Default true."},
        "theme": {"type": "string", "enum": ["default", "dark", "forest", "neutral", "base"]},
        "lang": {"type": "string", "enum": ["en", "pt-BR"], "description": "Language of messages (default: server setting)"},
        "show": {"type": "boolean", "description": "Also open the diagram in the human's desktop viewer (write tools; default false). With element_ids/message: highlight those elements with that note."},
        "element_ids": {"type": "array", "items": {"type": "string"}, "description": "With show=true: elements to highlight"},
        "message": {"type": "string", "description": "With show=true: note shown to the human next to the diagram"}
    });
    let with = |extra: Value, required: &[&str]| {
        let mut props = common.as_object().unwrap().clone();
        props.extend(extra.as_object().unwrap().clone());
        json!({"type": "object", "properties": props, "required": required})
    };
    let ro = json!({"readOnlyHint": true, "openWorldHint": false});
    let write = json!({"readOnlyHint": false, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false});
    json!([
        {"name": "create_diagram", "title": "Create diagram",
         "description": "Create a Mermaid diagram file in the workspace, validate and render it. Returns the problem report and an image (numbered markers = problems).",
         "inputSchema": with(json!({
            "name": {"type": "string", "description": "Short name; saved as diagrams/<name>.mmd"},
            "path": {"type": "string", "description": "Explicit workspace-relative path (.mmd). Overrides name."},
            "source": {"type": "string", "description": "Mermaid source"},
            "overwrite": {"type": "boolean"}}), &["source"]),
         "annotations": write},
        {"name": "update_diagram", "title": "Replace diagram",
         "description": "Replace the whole source of an existing diagram, then validate and render it.",
         "inputSchema": with(json!({"path": {"type": "string"}, "source": {"type": "string"}}), &["path", "source"]),
         "annotations": write},
        {"name": "patch_diagram", "title": "Edit diagram",
         "description": "Apply small text replacements (each old_text must occur exactly once), then validate and render.",
         "inputSchema": with(json!({"path": {"type": "string"},
            "edits": {"type": "array", "items": {"type": "object", "properties": {"old_text": {"type": "string"}, "new_text": {"type": "string"}}, "required": ["old_text", "new_text"]}}}),
            &["path", "edits"]),
         "annotations": write},
        {"name": "arrange_diagram", "title": "Fix layout",
         "description": "Change how a graph diagram (flowchart, state, class, ER, requirement) is laid out, stored as `%% @odg` comments so the file stays plain Mermaid. mode: auto (Mermaid default) | elk (ELK automatic) | elk-interactive (positions are hints: ELK keeps their layer and order and routes the edges) | exact (nodes stay exactly at the given positions). positions: node centres in the same coordinates as inspect_diagram's bounding boxes (centre = x + w/2, y + h/2); giving positions switches auto/elk to elk-interactive. layers: pin a node or subgraph to the first/last layer, or give numbers (0, 1, 2…) to fix the order of the layers even when some arrows point backwards. Returns the report and the new image.",
         "inputSchema": with(json!({"path": {"type": "string"},
            "mode": {"type": "string", "enum": ["auto", "elk", "elk-interactive", "exact"]},
            "positions": {"type": "array", "items": {"type": "object", "properties": {"id": {"type": "string"}, "x": {"type": "number"}, "y": {"type": "number"}}, "required": ["id", "x", "y"]}},
            "layers": {"type": "array", "description": "Pin layers: first/last, or numbers to fix the order of layers (0 before 1 before 2…, nodes or subgraphs; arrows going back no longer reorder them). Switches mode auto to elk.", "items": {"type": "object", "properties": {"id": {"type": "string"}, "layer": {"description": "first | last | none | 0, 1, 2…", "anyOf": [{"type": "string"}, {"type": "integer", "minimum": 0}]}}, "required": ["id", "layer"]}},
            "reset_positions": {"type": "boolean", "description": "Remove every stored position first"}}), &["path"]),
         "annotations": write},
        {"name": "add_element", "title": "Add element",
         "description": "Insert a well-formed element with the same constructors as the desktop palette (no syntax to remember). construct: direction{dir: TB|TD|BT|LR|RL} · flow.node{label, shape: rect|round|stadium|decision|database|circle|subroutine|io|hexagon|flag, from?, edge_label?, style?} · flow.edge{from, to, label?, style: arrow|open|dotted|thick|both} · flow.subgraph{title, members[]} · seq.participant{label, actor?} · seq.message{from, to, text, type: sync|reply|async|lost|line} · seq.block{block: alt|opt|loop|par|critical|break, from, to, condition} · seq.note{participant, text, position: right|left|over, other?} · seq.autonumber · class.class{name, stereotype?, attributes, methods} · class.relation{from, to, type: inheritance|composition|aggregation|association|dependency|realization|link, from_card?, to_card?, label?} · class.note{text, for?} · state.state{label} · state.transition{from, to, label?} · state.start{to} · state.end{from} · state.choice{label?} · state.note{state, text, position?} · er.entity{name, attributes} · er.relation{from, to, from_card: one|zero_one|zero_many|one_many, to_card, label, identifying?}. Returns the report and image.",
         "inputSchema": with(json!({"path": {"type": "string"}, "construct": {"type": "string"}, "params": {"type": "object"}}), &["path", "construct"]),
         "annotations": write},
        {"name": "edit_element", "title": "Edit element",
         "description": "Rename, reshape or delete an existing element without rewriting the file (same operations as the desktop inspector). action: rename{id, label} (flowchart/sequence/state change the label; class/ER rename the identifier everywhere) · shape{id, shape: rect|round|stadium|decision|database|circle|subroutine|io|hexagon|flag} (flowchart) · delete{id} (removes the element and its connections; neighbours keep their definitions) · edge_label{from, to, index?, label} · edge_delete{from, to, index?} (flowchart; index = n-th connection between the same pair, as in edge ids L_from_to_n). Returns the report and image.",
         "inputSchema": with(json!({"path": {"type": "string"}, "action": {"type": "string", "enum": ["rename", "shape", "delete", "edge_label", "edge_delete"]}, "params": {"type": "object"}}), &["path", "action", "params"]),
         "annotations": {"readOnlyHint": false, "destructiveHint": true, "openWorldHint": false}},
        {"name": "validate_diagram", "title": "Validate Mermaid",
         "description": "Check Mermaid source without saving: syntax errors with line/column/fix hints plus layout warnings.",
         "inputSchema": with(json!({"source": {"type": "string"}}), &["source"]),
         "annotations": ro},
        {"name": "render_diagram", "title": "See diagram",
         "description": "Render a saved diagram (path) or inline source to an image so you can look at the result. For big drawings, focus=<element or subgraph id> (or region) renders just that part at a legible scale.",
         "inputSchema": with(json!({"path": {"type": "string"}, "source": {"type": "string"},
            "format": {"type": "string", "enum": ["png", "svg"]},
            "annotated": {"type": "boolean", "description": "Draw numbered problem markers. Default true."},
            "scale": {"type": "number", "description": "PNG scale factor (0.5–4, default 1.5; with focus/region: chosen for ~1600 px)"},
            "focus": {"type": "string", "description": "Zoom: render only this element or subgraph (id), legibly, with the problems inside it"},
            "region": {"type": "object", "description": "Zoom: render only this rectangle (inspect_diagram coordinates)", "properties": {"x": {"type": "number"}, "y": {"type": "number"}, "w": {"type": "number"}, "h": {"type": "number"}}},
            "padding": {"type": "number", "description": "Margin around focus/region (default 24)"}}), &[]),
         "annotations": ro},
        {"name": "inspect_diagram", "title": "Describe layout",
         "description": "Text description of the rendered layout (nodes with positions, edges, groups, problems) for agents without vision.",
         "inputSchema": with(json!({"path": {"type": "string"}, "source": {"type": "string"}}), &[]),
         "annotations": ro},
        {"name": "list_diagrams", "title": "List diagrams",
         "description": "List diagrams in the workspace with their validation status and open human comments.",
         "inputSchema": {"type": "object", "properties": {}}, "annotations": ro},
        {"name": "get_diagram", "title": "Read diagram",
         "description": "Read a diagram's numbered source, its last report and open human comments.",
         "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}, "annotations": ro},
        {"name": "delete_diagram", "title": "Delete diagram",
         "description": "Delete a diagram file and its stored previews.",
         "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]},
         "annotations": {"readOnlyHint": false, "destructiveHint": true, "openWorldHint": false}},
        {"name": "get_feedback", "title": "Read human comments",
         "description": "Comments the human left on diagrams in the desktop viewer (what they want changed).",
         "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}, "include_resolved": {"type": "boolean"}}}, "annotations": ro},
        {"name": "resolve_feedback", "title": "Resolve comment",
         "description": "Mark a human comment as addressed, with a short note of what you changed.",
         "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}, "resolution": {"type": "string"}}, "required": ["id"]},
         "annotations": write},
        {"name": "get_activity", "title": "Recent activity",
         "description": "Recent changes in the workspace (who changed which diagram, validation results).",
         "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}, "limit": {"type": "integer"}}}, "annotations": ro},
        {"name": "get_syntax_help", "title": "Mermaid syntax help",
         "description": "Offline Mermaid syntax with complete examples, for every Mermaid diagram type. Topics: overview (all types and their headers), layout, flowchart, sequence, class, state, er, usecase, requirement, c4, architecture, block, swimlane, zenuml, gantt, timeline, journey, kanban, gitgraph, mindmap, treeview, ishikawa, eventmodeling, agentflow, railroad, packet, pie, xychart, quadrant, radar, sankey, treemap, venn, wardley, cynefin — or any Mermaid header (\"quadrantChart\", \"xychart-beta\"). Add \" full\" for the official reference digest.",
         "inputSchema": {"type": "object", "properties": {"topic": {"type": "string"}}}, "annotations": ro},
        {"name": "show_diagram", "title": "Show diagram to the human",
         "description": "Open a diagram in the human's ODiagramador desktop viewer (launching the app if it is closed). Optionally highlight elements and show a note next to them, e.g. \"I added the payment step here\".",
         "inputSchema": {"type": "object", "properties": {
            "path": {"type": "string"},
            "element_ids": {"type": "array", "items": {"type": "string"}, "description": "Node/edge ids to put in evidence (ids as in the source / inspect_diagram)"},
            "message": {"type": "string", "description": "Short note shown to the human"},
            "level": {"type": "string", "enum": ["info", "success", "warning", "error"]}}, "required": ["path"]},
         "annotations": {"readOnlyHint": false, "destructiveHint": false, "openWorldHint": false}},
        {"name": "open_diagram", "title": "Change the open diagram",
         "description": "Switch the diagram open in the human's desktop viewer (launching the app if it is closed). Optional message shown beside it.",
         "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}, "message": {"type": "string"}}, "required": ["path"]},
         "annotations": {"readOnlyHint": false, "destructiveHint": false, "openWorldHint": false}},
        {"name": "set_zoom", "title": "Zoom the human's view",
         "description": "Change the zoom of the diagram in the human's viewer: a factor (1 = 100%, 2 = 200%), a percentage like \"150%\", or fit (whole diagram) | in | out. Optional path opens that diagram first; optional center_on keeps an element in the middle. get_viewer_status tells the current zoom.",
         "inputSchema": {"type": "object", "properties": {
            "zoom": {"description": "Factor, \"150%\", or fit | in | out", "anyOf": [{"type": "number"}, {"type": "string"}]},
            "path": {"type": "string"},
            "center_on": {"type": "string", "description": "Element id to keep centred"}}, "required": ["zoom"]},
         "annotations": {"readOnlyHint": false, "destructiveHint": false, "openWorldHint": false}},
        {"name": "center_on_element", "title": "Centre the human's view on an element",
         "description": "Scroll the human's viewer so an element is in the middle of the screen (it pulses briefly so they spot it), optionally at a zoom factor and with a note. Optional path opens that diagram first. Use highlight_elements instead to keep elements emphasised.",
         "inputSchema": {"type": "object", "properties": {
            "element_id": {"type": "string"},
            "path": {"type": "string"},
            "zoom": {"type": "number", "description": "Factor (1 = 100%)"},
            "message": {"type": "string"}}, "required": ["element_id"]},
         "annotations": {"readOnlyHint": false, "destructiveHint": false, "openWorldHint": false}},
        {"name": "highlight_elements", "title": "Highlight elements for the human",
         "description": "Put elements of a diagram in evidence in the human's viewer: the diagram opens, the elements glow, the rest dims and the view centres on them, with an optional note. Use it to point at what you changed or what you are asking about.",
         "inputSchema": {"type": "object", "properties": {
            "path": {"type": "string"},
            "element_ids": {"type": "array", "items": {"type": "string"}},
            "message": {"type": "string"},
            "level": {"type": "string", "enum": ["info", "success", "warning", "error"]}}, "required": ["path", "element_ids"]},
         "annotations": {"readOnlyHint": false, "destructiveHint": false, "openWorldHint": false}},
        {"name": "notify_user", "title": "Send a message to the human",
         "description": "Show a message to the human in the desktop viewer (progress, summary of what you did, a warning). Optionally about a diagram/elements. Does not wait for a reply; use ask_user for questions.",
         "inputSchema": {"type": "object", "properties": {
            "message": {"type": "string"},
            "level": {"type": "string", "enum": ["info", "success", "warning", "error"]},
            "path": {"type": "string"},
            "element_ids": {"type": "array", "items": {"type": "string"}}}, "required": ["message"]},
         "annotations": {"readOnlyHint": false, "destructiveHint": false, "openWorldHint": false}},
        {"name": "ask_user", "title": "Ask the human",
         "description": "Ask the human a question in the desktop viewer and wait for the answer (up to wait_seconds, default 60, max 600). Give options for one-click answers; allow_text lets them type instead. Point at a diagram/elements to ask about them. If nobody answers in time you get a question_id: continue and call get_user_replies later.",
         "inputSchema": {"type": "object", "properties": {
            "question": {"type": "string"},
            "options": {"type": "array", "items": {"type": "string"}, "description": "Choices shown as buttons"},
            "allow_text": {"type": "boolean", "description": "Allow a typed answer besides the options (default true)"},
            "path": {"type": "string"},
            "element_ids": {"type": "array", "items": {"type": "string"}},
            "wait_seconds": {"type": "integer", "minimum": 0, "maximum": 600}}, "required": ["question"]},
         "annotations": {"readOnlyHint": false, "destructiveHint": false, "openWorldHint": false}},
        {"name": "get_user_replies", "title": "Read the human's answers",
         "description": "The answer to a question (question_id), or the recent conversation between agents and the human (questions, answers, notes).",
         "inputSchema": {"type": "object", "properties": {"question_id": {"type": "string"}, "limit": {"type": "integer"}}}, "annotations": ro},
        {"name": "get_viewer_status", "title": "Is the human watching?",
         "description": "Whether the ODiagramador desktop viewer is open on this workspace, which diagram the human is looking at, the zoom, the element in the centre of the screen, the elements on screen and the one they selected.",
         "inputSchema": {"type": "object", "properties": {}}, "annotations": ro},
        {"name": "open_in_viewer", "title": "Show to the human (legacy)",
         "description": "Same as show_diagram (kept for older prompts).",
         "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]},
         "annotations": {"readOnlyHint": false, "openWorldHint": false}}
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_and_urls() {
        assert_eq!(slug("Fluxo de Pedidos (v2)"), "fluxo-de-pedidos-v2");
        assert_eq!(slug("  "), "diagram");
        assert_eq!(pct("a b/ç"), "a%20b/%C3%A7");
        assert!(mentions_id("pay --> ship[Ship order]", "ship"));
        assert!(!mentions_id("start([Customer places order])", "C"));
    }

    #[test]
    fn tool_schemas_are_objects() {
        for t in tool_definitions().as_array().unwrap() {
            assert_eq!(t["inputSchema"]["type"], "object", "{}", t["name"]);
        }
    }
}
