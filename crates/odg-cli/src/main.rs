//! `odg` — ODiagramador command line and MCP server.
//!
//!   odg mcp    [--workspace DIR]            MCP server over stdio (for AI agents)
//!   odg check  FILES... [--json]            validate; exit code 1 when any file has errors
//!   odg render FILE [--out F] [--svg]       write a PNG/SVG preview
//!   odg watch  [--workspace DIR]            re-validate diagrams whenever they change
//!   odg mcp-config [--client NAME]          print the MCP config snippet for an agent

mod mcp;
mod native;

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, SystemTime};

use clap::{Parser, Subcommand, ValueEnum};
use odg_core::workspace::Actor;
use odg_core::{analyze, engine_for_path, AnalyzeOptions, Lang, Workspace};

#[derive(Parser)]
#[command(name = "odg", version, about = "ODiagramador — AI-friendly Mermaid viewer: CLI and MCP server")]
struct Cli {
    /// Interface language for messages: en | pt-BR (default: workspace config, then $LANG)
    #[arg(long, global = true)]
    lang: Option<String>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run the MCP server (stdio by default, or HTTP with --http)
    Mcp {
        /// Workspace folder (default: current directory)
        #[arg(long, short)]
        workspace: Option<PathBuf>,
        /// Serve MCP over HTTP instead of stdio, e.g. 127.0.0.1:7410 or 0.0.0.0:7410 (remote)
        #[arg(long)]
        http: Option<String>,
        /// Bearer token for HTTP clients (or ODG_MCP_TOKEN); required unless bound to loopback
        #[arg(long)]
        token: Option<String>,
    },
    /// Validate diagrams; prints problems and exits with 1 if any has errors
    Check {
        files: Vec<PathBuf>,
        #[arg(long, short)]
        workspace: Option<PathBuf>,
        /// Print JSON reports
        #[arg(long)]
        json: bool,
        /// Do not write reports/previews into .odiagramador/
        #[arg(long)]
        no_save: bool,
    },
    /// Render a diagram to PNG (default) or SVG
    Render {
        file: PathBuf,
        #[arg(long, short)]
        out: Option<PathBuf>,
        #[arg(long)]
        svg: bool,
        /// Draw numbered problem markers (or an error card when the diagram is invalid)
        #[arg(long)]
        annotated: bool,
        #[arg(long, default_value = "default")]
        theme: String,
        #[arg(long, default_value_t = 2.0)]
        scale: f64,
    },
    /// Watch the workspace and re-validate diagrams when they change
    Watch {
        #[arg(long, short)]
        workspace: Option<PathBuf>,
    },
    /// Print the MCP configuration snippet for an agent
    McpConfig {
        #[arg(long, value_enum, default_value_t = Client::Generic)]
        client: Client,
        #[arg(long, short)]
        workspace: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Client {
    Generic,
    ClaudeCode,
    Codex,
}

fn lang_for(cli: Option<&str>, ws: Option<&Workspace>) -> Lang {
    if let Some(l) = cli {
        return Lang::parse(l);
    }
    if let Some(l) = ws.and_then(|w| w.config().lang) {
        return Lang::parse(&l);
    }
    Lang::from_env()
}

fn open_ws(dir: Option<PathBuf>) -> Result<Workspace, String> {
    let dir = dir.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    Workspace::open(&dir).map_err(|e| format!("cannot open workspace {}: {e}", dir.display()))
}

fn main() -> ExitCode {
    native::ensure_native();
    let cli = Cli::parse();
    let lang_arg = cli.lang.clone();
    let result = match cli.cmd {
        Cmd::McpConfig { client, workspace } => mcp_config(client, workspace),
        Cmd::Mcp { workspace, http, token } => open_ws(workspace).and_then(|ws| {
            let lang = lang_for(lang_arg.as_deref(), Some(&ws));
            match http {
                None => odg_render::run(move |r| mcp::serve(ws, r, lang))?,
                Some(addr) => {
                    let token = token.or_else(|| std::env::var("ODG_MCP_TOKEN").ok()).filter(|t| !t.trim().is_empty());
                    let host = addr.rsplit_once(':').map(|(h, _)| h).unwrap_or(&addr);
                    let loopback = matches!(host, "127.0.0.1" | "localhost" | "[::1]" | "::1");
                    if !loopback && token.is_none() {
                        return Err(format!("listening on {addr} exposes the workspace to the network: pass --token (or ODG_MCP_TOKEN)"));
                    }
                    odg_render::run(move |r| mcp::serve_http(ws, r, lang, &addr, token))??;
                }
            }
            Ok(ExitCode::SUCCESS)
        }),
        Cmd::Check { files, workspace, json, no_save } => open_ws(workspace).and_then(|ws| {
            let lang = lang_for(lang_arg.as_deref(), Some(&ws));
            odg_render::run(move |r| check(&ws, &r, files, json, !no_save, lang))?
        }),
        Cmd::Render { file, out, svg, annotated, theme, scale } => {
            let lang = lang_for(lang_arg.as_deref(), None);
            odg_render::run(move |r| render(&r, file, out, svg, annotated, theme, scale, lang)).and_then(|x| x)
        }
        Cmd::Watch { workspace } => open_ws(workspace).and_then(|ws| {
            let lang = lang_for(lang_arg.as_deref(), Some(&ws));
            odg_render::run(move |r| watch(&ws, &r, lang))?
        }),
    };
    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("odg: {e}");
            ExitCode::from(2)
        }
    }
}

fn check(ws: &Workspace, r: &odg_render::Renderer, files: Vec<PathBuf>, json: bool, save: bool, lang: Lang) -> Result<ExitCode, String> {
    let targets: Vec<PathBuf> = if files.is_empty() {
        ws.list().map_err(|e| e.to_string())?.into_iter().map(|d| ws.root().join(d.path)).collect()
    } else {
        let mut out = vec![];
        for f in files {
            if f.is_dir() {
                collect_diagrams(&f, &mut out);
            } else {
                out.push(f);
            }
        }
        out
    };
    let opts = AnalyzeOptions { lang, png: save, annotated: save, font: ws.config().font, ..Default::default() };
    let mut failed = false;
    let mut reports = vec![];
    for file in targets {
        let src = std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        let name = file.to_string_lossy().to_string();
        let engine = engine_for_path(&name).unwrap_or(&odg_core::MermaidEngine);
        let mut a = analyze(engine, &src, r, &opts);
        let rel = std::fs::canonicalize(&file).ok().and_then(|abs| ws.rel(abs.to_str()?).ok());
        match (&rel, save) {
            (Some(rel), true) => ws.save_analysis(rel, &mut a, Some(Actor::Cli)).map_err(|e| e.to_string())?,
            _ => a.report.path = Some(rel.unwrap_or(name)),
        }
        failed |= !a.report.valid;
        if json {
            reports.push(a.report);
        } else {
            print!("{}", a.report.to_markdown());
        }
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&reports).unwrap());
    }
    Ok(if failed { ExitCode::from(1) } else { ExitCode::SUCCESS })
}

fn collect_diagrams(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if p.is_dir() && !name.starts_with('.') && name != "node_modules" && name != "target" {
            collect_diagrams(&p, out);
        } else if p.is_file() && odg_core::engine::is_diagram_path(&name) {
            out.push(p);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render(r: &odg_render::Renderer, file: PathBuf, out: Option<PathBuf>, svg: bool, annotated: bool, theme: String, scale: f64, lang: Lang) -> Result<ExitCode, String> {
    let src = std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    let engine = engine_for_path(&file.to_string_lossy()).unwrap_or(&odg_core::MermaidEngine);
    let opts = AnalyzeOptions { lang, theme, scale, png: !svg, annotated, ..Default::default() };
    let a = analyze(engine, &src, r, &opts);
    let ext = if svg { "svg" } else { "png" };
    let out = out.unwrap_or_else(|| file.with_extension(if annotated && !svg { "annotated.png" } else { ext }));
    let bytes: Option<Vec<u8>> = if svg {
        a.svg.map(String::into_bytes)
    } else if annotated {
        a.annotated_png.or(a.png)
    } else {
        a.png
    };
    eprint!("{}", a.report.to_markdown());
    match bytes {
        Some(b) => {
            std::fs::write(&out, b).map_err(|e| format!("{}: {e}", out.display()))?;
            println!("{}", out.display());
            Ok(if a.report.valid { ExitCode::SUCCESS } else { ExitCode::from(1) })
        }
        None => Ok(ExitCode::from(1)),
    }
}

fn watch(ws: &Workspace, r: &odg_render::Renderer, lang: Lang) -> Result<ExitCode, String> {
    let opts = AnalyzeOptions { lang, font: ws.config().font, ..Default::default() };
    let mut seen: std::collections::HashMap<String, SystemTime> = Default::default();
    eprintln!("watching {} (Ctrl+C to stop)", ws.root().display());
    loop {
        for d in ws.list().map_err(|e| e.to_string())? {
            let abs = ws.root().join(&d.path);
            let Ok(mtime) = std::fs::metadata(&abs).and_then(|m| m.modified()) else { continue };
            if seen.get(&d.path) == Some(&mtime) {
                continue;
            }
            seen.insert(d.path.clone(), mtime);
            if d.status.as_ref().is_some_and(|s| !s.stale) {
                continue;
            }
            let Ok(src) = std::fs::read_to_string(&abs) else { continue };
            let engine = engine_for_path(&d.path).unwrap_or(&odg_core::MermaidEngine);
            let mut a = analyze(engine, &src, r, &opts);
            ws.save_analysis(&d.path, &mut a, Some(Actor::Cli)).map_err(|e| e.to_string())?;
            print!("{}", a.report.to_markdown());
        }
        std::thread::sleep(Duration::from_millis(700));
    }
}

fn mcp_config(client: Client, workspace: Option<PathBuf>) -> Result<ExitCode, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe.canonicalize().unwrap_or(exe);
    let ws = workspace.map(|w| w.canonicalize().unwrap_or(w));
    let mut args = vec!["mcp".to_string()];
    if let Some(w) = &ws {
        args.push("--workspace".into());
        args.push(w.display().to_string());
    }
    let exe_s = exe.display().to_string();
    match client {
        Client::ClaudeCode => {
            let quoted: Vec<String> = args.iter().map(|a| shell_quote(a)).collect();
            println!("claude mcp add odiagramador -- {} {}", shell_quote(&exe_s), quoted.join(" "));
        }
        Client::Codex => {
            println!("[mcp_servers.odiagramador]\ncommand = {:?}\nargs = {:?}", exe_s, args);
        }
        Client::Generic => {
            let cfg = serde_json::json!({"mcpServers": {"odiagramador": {"command": exe_s, "args": args}}});
            println!("{}", serde_json::to_string_pretty(&cfg).unwrap());
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn shell_quote(s: &str) -> String {
    if s.chars().all(|c| c.is_ascii_alphanumeric() || "/._-:=".contains(c)) {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', r"'\''"))
    }
}
