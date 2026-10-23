//! MCP server (JSON-RPC 2.0 over newline-delimited stdio), implemented directly on the spec so
//! the binary stays small and synchronous: the hidden webview owns the main thread and every
//! tool call is a blocking render on this worker thread.
//!
//! Design goal: one tool call gives an agent everything it needs — validity, a numbered problem
//! list with lines and fixes, and an image where the same numbers are drawn on the diagram.

use std::io::{BufRead, Write};

use base64::Engine as _;
use odg_core::analyze::{excerpt, Analysis};
use odg_core::model::RawRender;
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
validate_diagram checks text without saving. inspect_diagram describes the layout in text for agents without vision. \
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
            "open_in_viewer" => {
                let path = s("path").ok_or(ToolError("`path` is required".into()))?;
                let rel = self.ws.rel(path)?;
                let url = format!("odiagramador://open?workspace={}&path={}", pct(&self.ws.root().to_string_lossy()), pct(&rel));
                open_url(&url).map_err(ToolError)?;
                Ok(text(format!("Asked the ODiagramador app to show {rel}.")))
            }
            _ => Err(ToolError(format!("unknown tool: {name}"))),
        }
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
        Ok(self.respond(&a, source, Some(&rel), args.get("include_image").and_then(Value::as_bool).unwrap_or(true)))
    }

    /// The standard answer: report text, next step, files, and the most useful image.
    fn respond(&self, a: &Analysis, source: &str, rel: Option<&str>, include_image: bool) -> Value {
        let r = &a.report;
        let mut t = r.to_markdown();
        if let Some(line) = r.first_error().and_then(|p| p.line) {
            t.push_str(&format!("\nSource around line {line}:\n```\n{}\n```\n", excerpt(source, line, 3)));
        }
        t.push_str(&next_step(r, Lang::parse(&r.lang)));
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

fn open_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(url).status();
    #[cfg(windows)]
    let r = std::process::Command::new("cmd").args(["/C", "start", "", url]).status();
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
        "lang": {"type": "string", "enum": ["en", "pt-BR"], "description": "Language of messages (default: server setting)"}
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
        {"name": "validate_diagram", "title": "Validate Mermaid",
         "description": "Check Mermaid source without saving: syntax errors with line/column/fix hints plus layout warnings.",
         "inputSchema": with(json!({"source": {"type": "string"}}), &["source"]),
         "annotations": ro},
        {"name": "render_diagram", "title": "See diagram",
         "description": "Render a saved diagram (path) or inline source to an image so you can look at the result.",
         "inputSchema": with(json!({"path": {"type": "string"}, "source": {"type": "string"},
            "format": {"type": "string", "enum": ["png", "svg"]},
            "annotated": {"type": "boolean", "description": "Draw numbered problem markers. Default true."},
            "scale": {"type": "number", "description": "PNG scale factor (0.5–4, default 1.5)"}}), &[]),
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
         "description": "Offline Mermaid cheat-sheet with a complete example. Topics: overview, flowchart, sequence, class, state, er, gantt, pie, mindmap.",
         "inputSchema": {"type": "object", "properties": {"topic": {"type": "string"}}}, "annotations": ro},
        {"name": "open_in_viewer", "title": "Show to the human",
         "description": "Open the diagram in the ODiagramador desktop app so the human can see it.",
         "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]},
         "annotations": {"readOnlyHint": true, "openWorldHint": false}}
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
    }

    #[test]
    fn tool_schemas_are_objects() {
        for t in tool_definitions().as_array().unwrap() {
            assert_eq!(t["inputSchema"]["type"], "object", "{}", t["name"]);
        }
    }
}
