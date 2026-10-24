//! A workspace is a plain folder. Diagrams are `.mmd` files anywhere inside it; ODiagramador's
//! own state lives in `.odiagramador/`:
//!
//! ```text
//! .odiagramador/
//!   config.json            { "lang": "pt-BR", "theme": "default" }       (optional)
//!   reports/<key>.json     last Report for each diagram
//!   previews/<key>.svg|.png|.annotated.png
//!   events.jsonl           who changed what (agent / human / cli)
//!   feedback.json          comments humans leave for agents in the desktop app
//! ```
//! `<key>` is the diagram's relative path with `/` replaced by `__`.

use std::fs;
use std::io::Write as _;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::analyze::Analysis;
use crate::engine::{engine_for_path, is_diagram_path};
use crate::report::Report;
use crate::{now_rfc3339, sha256_hex};

pub const STATE_DIR: &str = ".odiagramador";
const SKIP_DIRS: &[&str] = &[".git", "node_modules", "target", ".odiagramador", ".build", "build", "dist", "bin", "obj"];

#[derive(Debug, thiserror::Error)]
pub enum WsError {
    #[error("path escapes the workspace: {0}")]
    OutsideWorkspace(String),
    #[error("not a diagram file (use .mmd or .mermaid): {0}")]
    NotADiagram(String),
    #[error("diagram not found: {0}")]
    NotFound(String),
    #[error("feedback not found: {0}")]
    FeedbackNotFound(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
}

pub type WsResult<T> = Result<T, WsError>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    /// Diagram font family, shared by the apps and the agents' headless renders.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramStatus {
    pub valid: bool,
    pub errors: usize,
    pub warnings: usize,
    /// The report was produced for a different version of the file.
    pub stale: bool,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramInfo {
    pub path: String,
    pub name: String,
    pub engine: String,
    pub size: u64,
    pub modified: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<DiagramStatus>,
    pub open_feedback: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Actor {
    Agent,
    Human,
    Cli,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub ts: String,
    pub actor: Actor,
    /// create | update | delete | validate | feedback | resolve
    pub action: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub errors: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warnings: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Free-form agent/client name, e.g. "claude-code".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feedback {
    pub id: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    pub text: String,
    /// What was on screen when the human wrote it (diagram, selected element, its source lines),
    /// shown to them before sending and editable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    pub author: String,
    pub created_at: String,
    #[serde(default)]
    pub resolved: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Workspace {
    root: PathBuf,
}

impl Workspace {
    pub fn open(root: impl AsRef<Path>) -> WsResult<Workspace> {
        let root = root.as_ref();
        fs::create_dir_all(root)?;
        Ok(Workspace { root: root.canonicalize()? })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn state_dir(&self) -> PathBuf {
        self.root.join(STATE_DIR)
    }

    /// Normalizes a user/agent supplied path to a safe workspace-relative path with `/`.
    /// Absolute paths are accepted when they point inside the workspace.
    pub fn rel(&self, path: &str) -> WsResult<String> {
        let p = Path::new(path.trim());
        let p = if p.is_absolute() {
            let canon = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
            canon.strip_prefix(&self.root).map_err(|_| WsError::OutsideWorkspace(path.into()))?.to_path_buf()
        } else {
            p.to_path_buf()
        };
        let mut parts: Vec<String> = vec![];
        for c in p.components() {
            match c {
                Component::Normal(s) => parts.push(s.to_string_lossy().into_owned()),
                Component::CurDir => {}
                _ => return Err(WsError::OutsideWorkspace(path.into())),
            }
        }
        if parts.is_empty() || parts[0] == STATE_DIR {
            return Err(WsError::OutsideWorkspace(path.into()));
        }
        Ok(parts.join("/"))
    }

    pub fn abs(&self, rel: &str) -> WsResult<PathBuf> {
        Ok(self.root.join(self.rel(rel)?))
    }

    fn key(rel: &str) -> String {
        rel.replace('/', "__")
    }

    pub fn report_path(&self, rel: &str) -> PathBuf {
        self.state_dir().join("reports").join(format!("{}.json", Self::key(rel)))
    }

    pub fn preview_path(&self, rel: &str, ext: &str) -> PathBuf {
        self.state_dir().join("previews").join(format!("{}.{ext}", Self::key(rel)))
    }

    fn rel_of(&self, abs: &Path) -> String {
        abs.strip_prefix(&self.root).unwrap_or(abs).to_string_lossy().replace('\\', "/")
    }

    // ------------------------------------------------------------------ config

    pub fn config(&self) -> WorkspaceConfig {
        fs::read_to_string(self.state_dir().join("config.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn set_config(&self, cfg: &WorkspaceConfig) -> WsResult<()> {
        fs::create_dir_all(self.state_dir())?;
        fs::write(self.state_dir().join("config.json"), serde_json::to_vec_pretty(cfg)?)?;
        Ok(())
    }

    // ------------------------------------------------------------------ diagrams

    pub fn list(&self) -> WsResult<Vec<DiagramInfo>> {
        let mut files = vec![];
        walk(&self.root, &mut files)?;
        let feedback = self.feedback(None, false).unwrap_or_default();
        let mut out: Vec<DiagramInfo> = files
            .into_iter()
            .filter_map(|abs| {
                let rel = self.rel_of(&abs);
                let engine = engine_for_path(&rel)?;
                let meta = fs::metadata(&abs).ok()?;
                let modified = meta
                    .modified()
                    .ok()
                    .map(|m| time::OffsetDateTime::from(m).format(&time::format_description::well_known::Rfc3339).unwrap_or_default())
                    .unwrap_or_default();
                let status = self.status(&rel);
                Some(DiagramInfo {
                    name: abs.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
                    engine: engine.id().to_string(),
                    size: meta.len(),
                    modified,
                    status,
                    open_feedback: feedback.iter().filter(|f| f.path == rel).count(),
                    path: rel,
                })
            })
            .collect();
        out.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(out)
    }

    pub fn status(&self, rel: &str) -> Option<DiagramStatus> {
        let report = self.load_report(rel).ok()??;
        let current = fs::read(self.root.join(rel)).ok().map(|b| sha256_hex(&b));
        Some(DiagramStatus {
            valid: report.valid,
            errors: report.error_count,
            warnings: report.warning_count,
            stale: current.as_deref() != Some(report.source_sha256.as_str()),
            summary: report.summary,
        })
    }

    pub fn read(&self, path: &str) -> WsResult<String> {
        let rel = self.rel(path)?;
        let abs = self.root.join(&rel);
        if !abs.is_file() {
            return Err(WsError::NotFound(rel));
        }
        Ok(fs::read_to_string(abs)?)
    }

    pub fn exists(&self, path: &str) -> bool {
        self.abs(path).map(|p| p.is_file()).unwrap_or(false)
    }

    /// Writes a diagram (creating folders) and logs the change. Returns the relative path.
    pub fn write(&self, path: &str, content: &str, actor: Actor, client: Option<&str>) -> WsResult<String> {
        let rel = self.rel(path)?;
        if !is_diagram_path(&rel) {
            return Err(WsError::NotADiagram(rel));
        }
        let abs = self.root.join(&rel);
        let action = if abs.exists() { "update" } else { "create" };
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent)?;
        }
        // write-then-rename so watchers never see a half written file
        let tmp = abs.with_extension("odg-tmp");
        fs::write(&tmp, content)?;
        fs::rename(&tmp, &abs)?;
        self.log(Event {
            ts: now_rfc3339(),
            actor,
            action: action.into(),
            path: rel.clone(),
            valid: None,
            errors: None,
            warnings: None,
            note: None,
            client: client.map(str::to_string),
        })?;
        Ok(rel)
    }

    pub fn delete(&self, path: &str, actor: Actor) -> WsResult<()> {
        let rel = self.rel(path)?;
        let abs = self.root.join(&rel);
        if !abs.is_file() {
            return Err(WsError::NotFound(rel));
        }
        fs::remove_file(abs)?;
        for p in [self.report_path(&rel), self.preview_path(&rel, "svg"), self.preview_path(&rel, "png"), self.preview_path(&rel, "annotated.png")] {
            let _ = fs::remove_file(p);
        }
        self.log(Event { ts: now_rfc3339(), actor, action: "delete".into(), path: rel, valid: None, errors: None, warnings: None, note: None, client: None })
    }

    // ------------------------------------------------------------------ reports

    /// Stores report + previews for `rel` and fills `report.path` / `report.previews`.
    /// `actor: None` stores silently (e.g. the viewer refreshing a stale report).
    pub fn save_analysis(&self, rel: &str, analysis: &mut Analysis, actor: Option<Actor>) -> WsResult<()> {
        fs::create_dir_all(self.state_dir().join("previews"))?;
        fs::create_dir_all(self.state_dir().join("reports"))?;
        let previews = &mut analysis.report.previews;
        let store = |ext: &str, bytes: Option<&[u8]>| -> WsResult<Option<String>> {
            let p = self.preview_path(rel, ext);
            match bytes {
                Some(b) => {
                    fs::write(&p, b)?;
                    Ok(Some(self.rel_of(&p)))
                }
                None => {
                    let _ = fs::remove_file(&p);
                    Ok(None)
                }
            }
        };
        previews.svg = store("svg", analysis.svg.as_deref().map(str::as_bytes))?;
        previews.png = store("png", analysis.png.as_deref())?;
        previews.annotated_png = store("annotated.png", analysis.annotated_png.as_deref())?;
        analysis.report.path = Some(rel.to_string());
        self.save_report(rel, &analysis.report)?;
        let Some(actor) = actor else { return Ok(()) };
        let r = &analysis.report;
        self.log(Event {
            ts: now_rfc3339(),
            actor,
            action: "validate".into(),
            path: rel.to_string(),
            valid: Some(r.valid),
            errors: Some(r.error_count),
            warnings: Some(r.warning_count),
            note: None,
            client: None,
        })
    }

    pub fn save_report(&self, rel: &str, report: &Report) -> WsResult<()> {
        let p = self.report_path(rel);
        fs::create_dir_all(p.parent().unwrap())?;
        fs::write(p, serde_json::to_vec_pretty(report)?)?;
        Ok(())
    }

    pub fn load_report(&self, path: &str) -> WsResult<Option<Report>> {
        let rel = self.rel(path)?;
        match fs::read_to_string(self.report_path(&rel)) {
            Ok(s) => Ok(serde_json::from_str(&s).ok()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    // ------------------------------------------------------------------ events

    pub fn log(&self, event: Event) -> WsResult<()> {
        fs::create_dir_all(self.state_dir())?;
        let mut f = fs::OpenOptions::new().create(true).append(true).open(self.state_dir().join("events.jsonl"))?;
        writeln!(f, "{}", serde_json::to_string(&event)?)?;
        Ok(())
    }

    /// Most recent events first.
    pub fn events(&self, limit: usize, path: Option<&str>) -> WsResult<Vec<Event>> {
        let text = match fs::read_to_string(self.state_dir().join("events.jsonl")) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(e.into()),
        };
        let filter = path.map(|p| self.rel(p)).transpose()?;
        Ok(text
            .lines()
            .rev()
            .filter_map(|l| serde_json::from_str::<Event>(l).ok())
            .filter(|e| filter.as_deref().is_none_or(|p| e.path == p))
            .take(limit)
            .collect())
    }

    // ------------------------------------------------------------------ feedback

    fn feedback_file(&self) -> PathBuf {
        self.state_dir().join("feedback.json")
    }

    fn load_feedback(&self) -> WsResult<Vec<Feedback>> {
        match fs::read_to_string(self.feedback_file()) {
            Ok(s) => Ok(serde_json::from_str(&s)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
            Err(e) => Err(e.into()),
        }
    }

    fn store_feedback(&self, items: &[Feedback]) -> WsResult<()> {
        fs::create_dir_all(self.state_dir())?;
        fs::write(self.feedback_file(), serde_json::to_vec_pretty(items)?)?;
        Ok(())
    }

    pub fn feedback(&self, path: Option<&str>, include_resolved: bool) -> WsResult<Vec<Feedback>> {
        let filter = path.map(|p| self.rel(p)).transpose()?;
        Ok(self
            .load_feedback()?
            .into_iter()
            .filter(|f| include_resolved || !f.resolved)
            .filter(|f| filter.as_deref().is_none_or(|p| f.path == p))
            .collect())
    }

    pub fn add_feedback(&self, path: &str, text: &str, element_id: Option<&str>, line: Option<u32>, author: &str, context: Option<&str>) -> WsResult<Feedback> {
        let rel = self.rel(path)?;
        let mut items = self.load_feedback()?;
        let created_at = now_rfc3339();
        let id = format!("fb-{}", &sha256_hex(format!("{rel}{text}{created_at}{}", items.len()).as_bytes())[..10]);
        let fb = Feedback {
            id,
            path: rel.clone(),
            element_id: element_id.map(str::to_string),
            line,
            text: text.to_string(),
            context: context.map(str::trim).filter(|c| !c.is_empty()).map(str::to_string),
            author: author.to_string(),
            created_at,
            resolved: false,
            resolved_at: None,
            resolution: None,
        };
        items.push(fb.clone());
        self.store_feedback(&items)?;
        self.log(Event { ts: now_rfc3339(), actor: Actor::Human, action: "feedback".into(), path: rel, valid: None, errors: None, warnings: None, note: Some(text.chars().take(200).collect()), client: None })?;
        Ok(fb)
    }

    pub fn resolve_feedback(&self, id: &str, resolution: Option<&str>, actor: Actor) -> WsResult<Feedback> {
        let mut items = self.load_feedback()?;
        let fb = items.iter_mut().find(|f| f.id == id).ok_or_else(|| WsError::FeedbackNotFound(id.into()))?;
        fb.resolved = true;
        fb.resolved_at = Some(now_rfc3339());
        fb.resolution = resolution.map(str::to_string);
        let out = fb.clone();
        self.store_feedback(&items)?;
        self.log(Event { ts: now_rfc3339(), actor, action: "resolve".into(), path: out.path.clone(), valid: None, errors: None, warnings: None, note: resolution.map(str::to_string), client: None })?;
        Ok(out)
    }
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            if name.starts_with('.') || SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }
            walk(&entry.path(), out)?;
        } else if ft.is_file() && is_diagram_path(&name) {
            out.push(entry.path());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_escapes() {
        let dir = tempfile::tempdir().unwrap();
        let ws = Workspace::open(dir.path()).unwrap();
        assert!(ws.rel("../x.mmd").is_err());
        assert!(ws.rel("/etc/passwd").is_err());
        assert!(ws.rel(".odiagramador/config.json").is_err());
        assert_eq!(ws.rel("./a/b.mmd").unwrap(), "a/b.mmd");
        let abs = ws.root().join("c.mmd");
        assert_eq!(ws.rel(abs.to_str().unwrap()).unwrap(), "c.mmd");
    }

    #[test]
    fn write_list_events_feedback() {
        let dir = tempfile::tempdir().unwrap();
        let ws = Workspace::open(dir.path()).unwrap();
        assert!(ws.write("notes.txt", "x", Actor::Agent, None).is_err());
        let rel = ws.write("diagrams/flow.mmd", "flowchart LR\n A --> B", Actor::Agent, Some("test")).unwrap();
        ws.write("diagrams/flow.mmd", "flowchart LR\n A --> C", Actor::Human, None).unwrap();
        fs::create_dir_all(dir.path().join("node_modules")).unwrap();
        fs::write(dir.path().join("node_modules/skip.mmd"), "x").unwrap();
        let list = ws.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].path, "diagrams/flow.mmd");
        assert_eq!(list[0].name, "flow");
        let ev = ws.events(10, None).unwrap();
        assert_eq!(ev[0].action, "update");
        assert_eq!(ev[1].action, "create");
        assert_eq!(ev[1].client.as_deref(), Some("test"));

        let fb = ws.add_feedback(&rel, "make B red", Some("B"), None, "human", None).unwrap();
        assert_eq!(ws.feedback(Some(&rel), false).unwrap().len(), 1);
        assert_eq!(ws.list().unwrap()[0].open_feedback, 1);
        ws.resolve_feedback(&fb.id, Some("done"), Actor::Agent).unwrap();
        assert!(ws.feedback(None, false).unwrap().is_empty());
        assert_eq!(ws.feedback(None, true).unwrap().len(), 1);
    }
}
