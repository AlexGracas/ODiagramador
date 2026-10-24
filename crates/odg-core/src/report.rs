//! The validation report every consumer reads: MCP tools, CLI `--json`, desktop problem panels,
//! and the `.odiagramador/reports/*.json` files agents can open without MCP.

use serde::{Deserialize, Serialize};

use crate::model::BBox;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Problem {
    /// 1-based position in the report; the same number is drawn on the annotated preview.
    pub index: usize,
    pub severity: Severity,
    /// Stable English code, e.g. `E_PARSE`, `W_OVERLAP`.
    pub code: String,
    /// Localized, human readable.
    pub message: String,
    /// Localized suggestion on how to fix it.
    pub hint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox: Option<BBox>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    pub nodes: usize,
    pub edges: usize,
    pub groups: usize,
    pub width: f64,
    pub height: f64,
    pub lines: usize,
    /// Edge crossings in the drawing (lower is easier to read; compare versions with it).
    #[serde(default)]
    pub crossings: usize,
}

/// Workspace-relative paths of the files written next to the report.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Previews {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub svg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub png: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotated_png: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub engine: String,
    pub diagram_type: Option<String>,
    pub valid: bool,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    /// One localized sentence; good enough for an agent to decide what to do next.
    pub summary: String,
    pub problems: Vec<Problem>,
    pub stats: Stats,
    #[serde(default)]
    pub previews: Previews,
    pub source_sha256: String,
    pub lang: String,
    pub generated_at: String,
    pub duration_ms: u64,
}

impl Report {
    pub fn first_error(&self) -> Option<&Problem> {
        self.problems.iter().find(|p| p.severity == Severity::Error)
    }

    /// Compact Markdown rendering used in MCP text content and the CLI.
    pub fn to_markdown(&self) -> String {
        let mut s = String::new();
        let status = if self.valid { "✅" } else { "❌" };
        if let Some(p) = &self.path {
            s.push_str(&format!("{status} **{p}** — {}\n", self.summary));
        } else {
            s.push_str(&format!("{status} {}\n", self.summary));
        }
        for p in &self.problems {
            let sev = match p.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "info",
            };
            let loc = match (p.line, p.column) {
                (Some(l), Some(c)) => format!(" (line {l}:{c})"),
                (Some(l), None) => format!(" (line {l})"),
                _ => String::new(),
            };
            s.push_str(&format!("{}. [{sev} {}]{loc} {}\n   → {}\n", p.index, p.code, p.message, p.hint));
        }
        s
    }
}
