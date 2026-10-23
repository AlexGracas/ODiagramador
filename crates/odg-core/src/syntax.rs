//! Offline Mermaid cheat-sheets served to agents by `get_syntax_help` (from `docs/syntax`).

const TOPICS: &[(&str, &str)] = &[
    ("overview", include_str!("../../../docs/syntax/overview.md")),
    ("flowchart", include_str!("../../../docs/syntax/flowchart.md")),
    ("sequence", include_str!("../../../docs/syntax/sequence.md")),
    ("class", include_str!("../../../docs/syntax/class.md")),
    ("state", include_str!("../../../docs/syntax/state.md")),
    ("er", include_str!("../../../docs/syntax/er.md")),
    ("gantt", include_str!("../../../docs/syntax/gantt.md")),
    ("pie", include_str!("../../../docs/syntax/pie.md")),
    ("mindmap", include_str!("../../../docs/syntax/mindmap.md")),
];

pub fn topics() -> Vec<&'static str> {
    TOPICS.iter().map(|(k, _)| *k).collect()
}

/// Accepts topic names and Mermaid headers/types ("sequenceDiagram", "flowchart-v2", "graph", ...).
pub fn help(topic: &str) -> Option<&'static str> {
    let t = topic.trim().to_ascii_lowercase();
    let key = match t.as_str() {
        "" | "all" | "overview" | "index" => "overview",
        s if s.starts_with("flowchart") || s.starts_with("graph") => "flowchart",
        s if s.starts_with("sequence") => "sequence",
        s if s.starts_with("class") => "class",
        s if s.starts_with("state") => "state",
        s if s.starts_with("er") => "er",
        s if s.starts_with("gantt") => "gantt",
        s if s.starts_with("pie") => "pie",
        s if s.starts_with("mindmap") => "mindmap",
        _ => return None,
    };
    TOPICS.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

#[cfg(test)]
mod tests {
    #[test]
    fn resolves_aliases() {
        assert!(super::help("sequenceDiagram").unwrap().contains("autonumber"));
        assert!(super::help("flowchart-v2").unwrap().starts_with("# flowchart"));
        assert!(super::help("erDiagram").unwrap().starts_with("# erDiagram"));
        assert!(super::help("venn").is_none());
    }
}
