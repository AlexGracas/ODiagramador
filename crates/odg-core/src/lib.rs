//! ODiagramador core — shared by the CLI/MCP server (`odg`), the headless renderer and, through
//! `odg-ffi`, the native desktop apps (SwiftUI, WPF, GTK).

pub mod analyze;
pub mod engine;
pub mod i18n;
pub mod lint;
pub mod model;
pub mod report;
pub mod source;
pub mod syntax;
pub mod workspace;

pub use analyze::{analyze, build_report, Analysis, AnalyzeOptions};
pub use engine::{engine_for_path, DiagramEngine, MermaidEngine, RenderBackend};
pub use i18n::{t, Lang};
pub use model::{BBox, DiagramModel, ProcessRequest, RawRender};
pub use report::{Problem, Report, Severity};
pub use workspace::{Actor, Workspace};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Version of the vendored Mermaid build in `render-kit/vendor`.
pub fn mermaid_version() -> &'static str {
    include_str!("../../../render-kit/vendor/MERMAID_VERSION").trim()
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

pub fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}
