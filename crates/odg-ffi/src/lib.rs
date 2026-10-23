//! C ABI over odg-core for the native desktop apps (Swift, C#, anything with a C FFI).
//!
//! One entry point keeps every binding trivial:
//!
//! ```c
//! char *odg_invoke(const char *method, const char *args_json);  // -> {"ok":true,"result":...}
//! void  odg_free(char *s);                                       //    {"ok":false,"error":"..."}
//! ```
//!
//! See `include/odg_ffi.h` for the method list.

use std::ffi::{c_char, CStr, CString};
use std::path::PathBuf;

use base64::Engine as _;
use odg_core::analyze::{annotate_svg, Analysis};
use odg_core::lint::LintConfig;
use odg_core::model::{BBox, RawRender};
use odg_core::workspace::{Actor, WorkspaceConfig};
use odg_core::{build_report, i18n, source, syntax, Lang, Problem, Workspace};
use serde_json::{json, Value};

type R = Result<Value, String>;

fn arg<'a>(a: &'a Value, k: &str) -> Result<&'a str, String> {
    a.get(k).and_then(Value::as_str).ok_or_else(|| format!("missing string argument `{k}`"))
}

fn opt<'a>(a: &'a Value, k: &str) -> Option<&'a str> {
    a.get(k).and_then(Value::as_str).filter(|s| !s.is_empty())
}

fn ws(a: &Value) -> Result<Workspace, String> {
    Workspace::open(PathBuf::from(arg(a, "workspace")?)).map_err(|e| e.to_string())
}

fn lang(a: &Value) -> Lang {
    opt(a, "lang").map(Lang::parse).unwrap_or_else(Lang::from_env)
}

fn actor(a: &Value) -> Actor {
    match opt(a, "actor") {
        Some("agent") => Actor::Agent,
        Some("cli") => Actor::Cli,
        _ => Actor::Human,
    }
}

fn to_value<T: serde::Serialize>(v: T) -> R {
    serde_json::to_value(v).map_err(|e| e.to_string())
}

/// Dispatches one call. Public for Rust consumers (the GTK app uses odg-core directly).
pub fn invoke(method: &str, a: &Value) -> R {
    match method {
        "version" => Ok(json!({"version": odg_core::VERSION, "mermaid": odg_core::mermaid_version()})),
        "strings" => to_value(i18n::all_strings(lang(a))),
        "t" => {
            let key = arg(a, "key")?;
            let args = a.get("args").and_then(Value::as_object).cloned().unwrap_or_default();
            let owned: Vec<(String, Value)> = args.into_iter().collect();
            let refs: Vec<(&str, i18n::Arg)> = owned
                .iter()
                .map(|(k, v)| {
                    (
                        k.as_str(),
                        match v {
                            Value::Number(n) => i18n::Arg::Num(n.as_f64().unwrap_or(0.0)),
                            Value::String(s) => i18n::Arg::Str(s.as_str()),
                            _ => i18n::Arg::Str(""),
                        },
                    )
                })
                .collect();
            Ok(Value::String(i18n::t(lang(a), key, &refs)))
        }
        // Builds the report from the render-kit result the app's WebView produced.
        // args: source, raw (object), lang?, workspace?, path?, save? (bool), actor?
        "analyze" => {
            let src = arg(a, "source")?;
            let raw: RawRender = serde_json::from_value(a.get("raw").cloned().unwrap_or(Value::Null)).map_err(|e| format!("bad raw render: {e}"))?;
            let engine = opt(a, "path").and_then(odg_core::engine_for_path).map(|e| e.id()).unwrap_or("mermaid");
            let report = build_report(src, &raw, engine, lang(a), &LintConfig::default(), raw.duration_ms);
            let mut analysis = Analysis {
                report,
                svg: raw.svg.clone(),
                png: raw.png.as_ref().and_then(|b| base64::engine::general_purpose::STANDARD.decode(b).ok()),
                annotated_png: None,
                raw,
            };
            if let (Some(path), true) = (opt(a, "path"), a.get("save").and_then(Value::as_bool).unwrap_or(false)) {
                let w = ws(a)?;
                let rel = w.rel(path).map_err(|e| e.to_string())?;
                let log = a.get("log").and_then(Value::as_bool).unwrap_or(false);
                w.save_analysis(&rel, &mut analysis, log.then(|| actor(a))).map_err(|e| e.to_string())?;
            }
            to_value(&analysis.report)
        }
        "annotate_svg" => {
            let svg = arg(a, "svg")?;
            let vb: Option<BBox> = a.get("view_box").cloned().and_then(|v| serde_json::from_value(v).ok());
            let problems: Vec<Problem> = serde_json::from_value(a.get("problems").cloned().unwrap_or(json!([]))).map_err(|e| e.to_string())?;
            Ok(Value::String(annotate_svg(svg, vb, &problems)))
        }
        "find_line" => Ok(json!(source::find_line(arg(a, "source")?, arg(a, "id")?))),
        "syntax_help" => Ok(json!(syntax::help(opt(a, "topic").unwrap_or("overview")))),
        "list" => to_value(ws(a)?.list().map_err(|e| e.to_string())?),
        "read" => Ok(Value::String(ws(a)?.read(arg(a, "path")?).map_err(|e| e.to_string())?)),
        "write" => {
            let rel = ws(a)?.write(arg(a, "path")?, arg(a, "content")?, actor(a), opt(a, "client")).map_err(|e| e.to_string())?;
            Ok(Value::String(rel))
        }
        "delete" => {
            ws(a)?.delete(arg(a, "path")?, actor(a)).map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        "report" => to_value(ws(a)?.load_report(arg(a, "path")?).map_err(|e| e.to_string())?),
        "events" => {
            let limit = a.get("limit").and_then(Value::as_u64).unwrap_or(50) as usize;
            to_value(ws(a)?.events(limit, opt(a, "path")).map_err(|e| e.to_string())?)
        }
        "feedback" => {
            let all = a.get("include_resolved").and_then(Value::as_bool).unwrap_or(false);
            to_value(ws(a)?.feedback(opt(a, "path"), all).map_err(|e| e.to_string())?)
        }
        "add_feedback" => {
            let line = a.get("line").and_then(Value::as_u64).map(|l| l as u32);
            let fb = ws(a)?
                .add_feedback(arg(a, "path")?, arg(a, "text")?, opt(a, "element_id"), line, opt(a, "author").unwrap_or("human"))
                .map_err(|e| e.to_string())?;
            to_value(fb)
        }
        "resolve_feedback" => to_value(ws(a)?.resolve_feedback(arg(a, "id")?, opt(a, "resolution"), actor(a)).map_err(|e| e.to_string())?),
        "config" => to_value(ws(a)?.config()),
        "set_config" => {
            let w = ws(a)?;
            let cfg = WorkspaceConfig { lang: opt(a, "lang").map(str::to_string), theme: opt(a, "theme").map(str::to_string) };
            w.set_config(&cfg).map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        // MCP configuration snippets for the "Copy MCP config" button.
        "mcp_config" => {
            let exe = arg(a, "odg_path")?;
            let w = arg(a, "workspace")?;
            let generic = json!({"mcpServers": {"odiagramador": {"command": exe, "args": ["mcp", "--workspace", w]}}});
            Ok(json!({
                "json": serde_json::to_string_pretty(&generic).unwrap(),
                "claude_code": format!("claude mcp add odiagramador -- \"{exe}\" mcp --workspace \"{w}\""),
                "codex": format!("[mcp_servers.odiagramador]\ncommand = {exe:?}\nargs = [\"mcp\", \"--workspace\", {w:?}]"),
            }))
        }
        _ => Err(format!("unknown method `{method}`")),
    }
}

fn into_c(v: Value) -> *mut c_char {
    let s = serde_json::to_string(&v).unwrap_or_else(|_| r#"{"ok":false,"error":"serialization"}"#.into());
    CString::new(s).unwrap_or_else(|_| CString::new(r#"{"ok":false,"error":"nul byte"}"#).unwrap()).into_raw()
}

/// # Safety
/// `method` and `args_json` must be valid NUL-terminated UTF-8 strings (args may be NULL).
/// The returned string must be released with [`odg_free`].
#[no_mangle]
pub unsafe extern "C" fn odg_invoke(method: *const c_char, args_json: *const c_char) -> *mut c_char {
    let result = std::panic::catch_unwind(|| {
        if method.is_null() {
            return Err("method is NULL".to_string());
        }
        let method = CStr::from_ptr(method).to_str().map_err(|e| e.to_string())?;
        let args: Value = if args_json.is_null() {
            json!({})
        } else {
            let s = CStr::from_ptr(args_json).to_str().map_err(|e| e.to_string())?;
            if s.trim().is_empty() { json!({}) } else { serde_json::from_str(s).map_err(|e| format!("invalid args json: {e}"))? }
        };
        invoke(method, &args)
    });
    into_c(match result {
        Ok(Ok(v)) => json!({"ok": true, "result": v}),
        Ok(Err(e)) => json!({"ok": false, "error": e}),
        Err(_) => json!({"ok": false, "error": "internal panic"}),
    })
}

/// # Safety
/// `s` must come from [`odg_invoke`] and be freed only once.
#[no_mangle]
pub unsafe extern "C" fn odg_free(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(method: &str, args: Value) -> Value {
        let m = CString::new(method).unwrap();
        let a = CString::new(args.to_string()).unwrap();
        unsafe {
            let p = odg_invoke(m.as_ptr(), a.as_ptr());
            let out: Value = serde_json::from_str(CStr::from_ptr(p).to_str().unwrap()).unwrap();
            odg_free(p);
            out
        }
    }

    #[test]
    fn round_trip() {
        assert_eq!(call("version", json!({}))["ok"], true);
        assert_eq!(call("strings", json!({"lang": "pt-BR"}))["result"]["ui-save"], "Salvar");
        assert_eq!(call("nope", json!({}))["ok"], false);
        let t = call("t", json!({"lang": "en", "key": "ui-line", "args": {"line": 4}}));
        assert_eq!(t["result"], "Line 4");
    }

    #[test]
    fn analyze_and_save() {
        let dir = tempfile::tempdir().unwrap();
        let w = dir.path().to_str().unwrap();
        assert_eq!(call("write", json!({"workspace": w, "path": "d/a.mmd", "content": "flowchart LR\n A\n B"}))["result"], "d/a.mmd");
        let raw = json!({"ok": true, "diagram_type": "flowchart-v2", "svg": "<svg></svg>",
            "model": {"nodes": [{"id": "A", "label": "A"}, {"id": "B", "label": "B"}], "edges": [], "groups": []}});
        let r = call("analyze", json!({"workspace": w, "path": "d/a.mmd", "source": "flowchart LR\n A\n B", "raw": raw, "save": true, "lang": "en"}));
        assert_eq!(r["result"]["warning_count"], 2);
        let list = call("list", json!({"workspace": w}));
        assert_eq!(list["result"][0]["status"]["warnings"], 2);
        assert_eq!(list["result"][0]["status"]["stale"], false);
    }
}
