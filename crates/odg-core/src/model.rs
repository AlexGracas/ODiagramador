//! Engine-neutral diagram model and the raw render result produced by the render-kit.
//!
//! `RawRender` mirrors exactly what `render-kit/bridge.js` posts back (`op: "result"`), so every
//! host (headless wry, WKWebView, WebView2, WebKitGTK) can hand the same JSON to the core.

use serde::{Deserialize, Serialize};

/// Axis-aligned box in the diagram's SVG user (viewBox) coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BBox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl BBox {
    pub fn area(&self) -> f64 {
        self.w.max(0.0) * self.h.max(0.0)
    }

    pub fn intersection(&self, o: &BBox) -> f64 {
        let x0 = self.x.max(o.x);
        let y0 = self.y.max(o.y);
        let x1 = (self.x + self.w).min(o.x + o.w);
        let y1 = (self.y + self.h).min(o.y + o.h);
        if x1 <= x0 || y1 <= y0 {
            0.0
        } else {
            (x1 - x0) * (y1 - y0)
        }
    }

    pub fn contains(&self, o: &BBox) -> bool {
        o.x >= self.x - 0.5
            && o.y >= self.y - 0.5
            && o.x + o.w <= self.x + self.w + 0.5
            && o.y + o.h <= self.y + self.h + 0.5
    }

    pub fn union(&self, o: &BBox) -> BBox {
        let x = self.x.min(o.x);
        let y = self.y.min(o.y);
        BBox {
            x,
            y,
            w: (self.x + self.w).max(o.x + o.w) - x,
            h: (self.y + self.h).max(o.y + o.h) - y,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub shape: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dom_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<BBox>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_bbox: Option<BBox>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_overflow: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<BBox>,
}

/// Engine-neutral structure. Mermaid fills it today; future UML engines produce the same shape,
/// so lint, MCP `inspect_diagram` and the desktop apps work unchanged.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DiagramModel {
    #[serde(default)]
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub edges: Vec<Edge>,
    #[serde(default)]
    pub groups: Vec<Node>,
}

impl DiagramModel {
    pub fn node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id).or_else(|| self.groups.iter().find(|n| n.id == id))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SourceError {
    /// "parse" | "unknown_type" | "render"
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub line: Option<u32>,
    #[serde(default)]
    pub column: Option<u32>,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub expected: Option<Vec<String>>,
}

/// Result of `odg.handle({op:"process"})` in the render-kit.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RawRender {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub diagram_type: Option<String>,
    #[serde(default)]
    pub parse_error: Option<SourceError>,
    #[serde(default)]
    pub render_error: Option<SourceError>,
    #[serde(default)]
    pub raster_error: Option<String>,
    #[serde(default)]
    pub model_error: Option<String>,
    #[serde(default)]
    pub svg: Option<String>,
    /// base64 PNG (no data: prefix)
    #[serde(default)]
    pub png: Option<String>,
    #[serde(default)]
    pub width: f64,
    #[serde(default)]
    pub height: f64,
    #[serde(default)]
    pub view_box: Option<BBox>,
    #[serde(default)]
    pub model: DiagramModel,
    #[serde(default)]
    pub has_foreign_object: bool,
    #[serde(default)]
    pub duration_ms: u64,
}

/// Request sent to a render backend (`op: "process"`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRequest {
    pub source: String,
    /// Mermaid theme: default | dark | forest | neutral | base
    pub theme: String,
    pub png: bool,
    pub scale: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
}

impl ProcessRequest {
    pub fn new(source: impl Into<String>) -> Self {
        ProcessRequest { source: source.into(), theme: "default".into(), png: true, scale: 2.0, background: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bbox_math() {
        let a = BBox { x: 0.0, y: 0.0, w: 10.0, h: 10.0 };
        let b = BBox { x: 5.0, y: 5.0, w: 10.0, h: 10.0 };
        assert_eq!(a.intersection(&b), 25.0);
        assert!(!a.contains(&b));
        assert!(a.union(&b).contains(&b));
    }

    #[test]
    fn raw_render_from_bridge_json() {
        let json = r#"{"ok":true,"diagram_type":"flowchart-v2","width":100,"height":50,
            "model":{"nodes":[{"id":"A","label":"Start","parent":null,"shape":"squareRect",
            "bbox":{"x":1,"y":2,"w":3,"h":4}}],"edges":[],"groups":[]}}"#;
        let raw: RawRender = serde_json::from_str(json).unwrap();
        assert!(raw.ok);
        assert_eq!(raw.model.nodes[0].bbox.unwrap().h, 4.0);
    }
}
