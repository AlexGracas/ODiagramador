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
    ("layout", include_str!("../../../docs/syntax/layout.md")),
];

pub fn topics() -> Vec<&'static str> {
    let mut t = vec!["overview", "layout"];
    t.extend(crate::kinds::KINDS.iter().map(|k| k.id));
    t
}

/// Accepts topic names and Mermaid headers/types ("sequenceDiagram", "flowchart-v2", "xychart-beta", ...).
/// Types with a hand-written cheat-sheet return it; every other type returns the digest of the official
/// Mermaid docs. `"<type> full"` (or `"reference <type>"`) always returns the official digest.
pub fn help(topic: &str) -> Option<String> {
    let t = topic.trim().to_ascii_lowercase();
    let (t, full) = match (t.strip_suffix(" full"), t.strip_prefix("reference ")) {
        (Some(x), _) | (_, Some(x)) => (x.trim().to_string(), true),
        _ => (t, false),
    };
    let curated = match t.as_str() {
        "" | "all" | "overview" | "index" | "types" => Some("overview"),
        s if s.starts_with("layout") || s.starts_with("elk") || s.starts_with("position") => Some("layout"),
        _ => None,
    };
    if let Some(key) = curated {
        return TOPICS.iter().find(|(k, _)| *k == key).map(|(_, v)| v.to_string());
    }
    let kind = crate::kinds::find(&t)?;
    if !full {
        if let Some((_, sheet)) = TOPICS.iter().find(|(k, _)| *k == kind.id) {
            return Some(format!("{sheet}\n---\nFull official reference: get_syntax_help(\"{} full\").\n", kind.id));
        }
    }
    Some(kind.reference.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn resolves_aliases() {
        assert!(super::help("sequenceDiagram").unwrap().contains("autonumber"));
        assert!(super::help("flowchart-v2").unwrap().starts_with("# flowchart"));
        assert!(super::help("erDiagram").unwrap().starts_with("# erDiagram"));
        assert!(super::help("venn").unwrap().contains("venn-beta"));
        assert!(super::help("quadrantChart").unwrap().contains("quadrantChart"));
        assert!(super::help("flowchart full").unwrap().contains("Digest of Mermaid"));
        assert!(super::help("nonsense").is_none());
    }
}
