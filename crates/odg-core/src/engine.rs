//! Diagram engines and render backends.
//!
//! * A [`RenderBackend`] runs the render-kit somewhere (hidden wry webview, an app's WebView...).
//! * A [`DiagramEngine`] knows a source language and turns it into a render-kit request.
//!
//! Mermaid is the only engine today. UML plugs in as another engine (see `docs/UML.md`): it can
//! translate its source to Mermaid (class / sequence / state diagrams) or ship its own renderer,
//! and the lint, reports, MCP tools and desktop apps keep working unchanged.

use crate::model::{ProcessRequest, RawRender};

/// Something that can execute the render-kit (`render-kit/bridge.js`).
pub trait RenderBackend {
    fn process(&self, req: &ProcessRequest) -> Result<RawRender, String>;
    /// Rasterizes an SVG document to PNG bytes.
    fn rasterize(&self, svg: &str, scale: f64, background: &str) -> Result<Vec<u8>, String>;
}

pub trait DiagramEngine: Send + Sync {
    /// Stable id, e.g. "mermaid".
    fn id(&self) -> &'static str;
    /// File extensions (without dot) handled by this engine.
    fn extensions(&self) -> &'static [&'static str];
    /// False for engines that are registered but not usable yet.
    fn available(&self) -> bool {
        true
    }
    /// Turns the engine's source into the render-kit's input (Mermaid text).
    fn prepare(&self, source: &str) -> Result<String, String>;
}

pub struct MermaidEngine;

impl DiagramEngine for MermaidEngine {
    fn id(&self) -> &'static str {
        "mermaid"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["mmd", "mermaid"]
    }

    fn prepare(&self, source: &str) -> Result<String, String> {
        Ok(source.to_string())
    }
}

/// Placeholder for the upcoming UML engine (PlantUML-like text or XMI → Mermaid). Registered so
/// `.puml`/`.uml` files are recognized and reported clearly instead of being ignored.
pub struct UmlEngine;

impl DiagramEngine for UmlEngine {
    fn id(&self) -> &'static str {
        "uml"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["puml", "uml"]
    }

    fn available(&self) -> bool {
        false
    }

    fn prepare(&self, _source: &str) -> Result<String, String> {
        Err("the UML engine is not available yet; use Mermaid classDiagram / sequenceDiagram / stateDiagram-v2".into())
    }
}

pub fn engines() -> &'static [&'static dyn DiagramEngine] {
    &[&MermaidEngine, &UmlEngine]
}

pub fn engine_for_path(path: &str) -> Option<&'static dyn DiagramEngine> {
    let ext = path.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase())?;
    engines().iter().copied().find(|e| e.extensions().contains(&ext.as_str()))
}

pub fn is_diagram_path(path: &str) -> bool {
    engine_for_path(path).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_engine_by_extension() {
        assert_eq!(engine_for_path("a/b.mmd").unwrap().id(), "mermaid");
        assert_eq!(engine_for_path("x.MERMAID").unwrap().id(), "mermaid");
        assert!(!engine_for_path("x.puml").unwrap().available());
        assert!(engine_for_path("x.txt").is_none());
    }
}
