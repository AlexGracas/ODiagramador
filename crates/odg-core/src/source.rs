//! Helpers that work on the raw diagram text (no rendering needed).

use regex::Regex;

/// Number of lines taken by a leading YAML front matter block (`---` ... `---`), or 0.
/// Mermaid strips it before parsing, so parser line numbers must be shifted by this amount.
pub fn front_matter_lines(source: &str) -> u32 {
    let mut lines = source.lines();
    match lines.next() {
        Some(first) if first.trim_end() == "---" => {}
        _ => return 0,
    }
    for (i, line) in lines.enumerate() {
        if line.trim_end() == "---" {
            return i as u32 + 2;
        }
    }
    0
}

/// 1-based line where `id` first appears as a whole token (outside comments and front matter).
pub fn find_line(source: &str, id: &str) -> Option<u32> {
    if id.is_empty() {
        return None;
    }
    let re = Regex::new(&format!(r"(^|[^\w-]){}($|[^\w-])", regex::escape(id))).ok()?;
    let skip = front_matter_lines(source) as usize;
    source
        .lines()
        .enumerate()
        .skip(skip)
        .filter(|(_, l)| !l.trim_start().starts_with("%%"))
        .find(|(_, l)| re.is_match(l))
        .map(|(i, _)| i as u32 + 1)
}

/// Text of a 1-based line, if present.
pub fn line_text(source: &str, line: u32) -> Option<&str> {
    source.lines().nth(line.checked_sub(1)? as usize)
}

/// True when the source has no meaningful content (only blanks / comments / front matter).
pub fn is_blank(source: &str) -> bool {
    let skip = front_matter_lines(source) as usize;
    source.lines().skip(skip).all(|l| {
        let t = l.trim();
        t.is_empty() || t.starts_with("%%")
    })
}

/// First bracket opened on `line` and never closed on it (quoted text is ignored).
pub fn unclosed_bracket(line: &str) -> Option<char> {
    let mut stack: Vec<char> = vec![];
    let mut quoted = false;
    for c in line.chars() {
        match c {
            '"' => quoted = !quoted,
            _ if quoted => {}
            '[' | '(' | '{' => stack.push(c),
            ']' | ')' | '}' => {
                let open = match c {
                    ']' => '[',
                    ')' => '(',
                    _ => '{',
                };
                if stack.last() == Some(&open) {
                    stack.pop();
                }
            }
            _ => {}
        }
    }
    if line.trim_start().starts_with("%%") {
        return None;
    }
    stack.first().copied()
}

// ---------------------------------------------------------------- @odg layout directives
//
// Positions live in Mermaid comments so the file stays valid Mermaid everywhere else:
//   %% @odg layout auto|elk|elk-interactive|exact
//   %% @odg pos <id> <x>,<y>          node centre, same coordinates as the report's bounding boxes
//   %% @odg layer <id> first|last

pub const LAYOUT_MODES: &[&str] = &["auto", "elk", "elk-interactive", "exact"];

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NodePosition {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NodeLayer {
    pub id: String,
    pub layer: String,
    pub line: u32,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LayoutDirectives {
    /// auto | elk | elk-interactive | exact (as written; validated by lint)
    pub mode: String,
    pub mode_line: Option<u32>,
    pub positions: Vec<NodePosition>,
    pub layers: Vec<NodeLayer>,
}

fn directive(line: &str) -> Option<(&str, Vec<&str>)> {
    let rest = line.trim_start().strip_prefix("%%")?.trim_start().strip_prefix("@odg")?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let mut parts = rest.split_whitespace();
    let kind = parts.next()?;
    Some((kind, parts.collect()))
}

pub fn layout_directives(src: &str) -> LayoutDirectives {
    let mut d = LayoutDirectives { mode: "auto".into(), ..Default::default() };
    for (i, line) in src.lines().enumerate() {
        let Some((kind, args)) = directive(line) else { continue };
        let n = i as u32 + 1;
        match (kind, args.as_slice()) {
            ("layout", [mode, ..]) => {
                d.mode = mode.to_string();
                d.mode_line = Some(n);
            }
            ("pos", [id, xy, ..]) => {
                if let Some((x, y)) = xy.split_once(',') {
                    if let (Ok(x), Ok(y)) = (x.trim().parse::<f64>(), y.trim().parse::<f64>()) {
                        d.positions.push(NodePosition { id: id.to_string(), x, y, line: n });
                    }
                }
            }
            ("layer", [id, layer, ..]) => d.layers.push(NodeLayer { id: id.to_string(), layer: layer.to_string(), line: n }),
            _ => {}
        }
    }
    d
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_alphanumeric() || "_-.:".contains(c))
}

/// Replaces the directive lines selected by `matches` with `new_line` (or appends it at the end).
fn upsert(src: &str, matches: impl Fn(&str, &[&str]) -> bool, new_line: Option<String>) -> String {
    let mut lines: Vec<String> = vec![];
    let mut placed = false;
    for line in src.lines() {
        let hit = directive(line).is_some_and(|(k, a)| matches(k, &a));
        if !hit {
            lines.push(line.to_string());
        } else if let (false, Some(nl)) = (placed, &new_line) {
            lines.push(nl.clone());
            placed = true;
        }
    }
    if let (false, Some(nl)) = (placed, new_line) {
        while lines.last().is_some_and(|l| l.trim().is_empty()) {
            lines.pop();
        }
        lines.push(nl);
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

pub fn set_layout_mode(src: &str, mode: &str) -> Result<String, String> {
    if !LAYOUT_MODES.contains(&mode) {
        return Err(format!("unknown layout mode `{mode}` (use {})", LAYOUT_MODES.join(", ")));
    }
    let line = (mode != "auto").then(|| format!("%% @odg layout {mode}"));
    Ok(upsert(src, |k, _| k == "layout", line))
}

/// Pins a node (centre coordinates). Switches `auto`/`elk` diagrams to `elk-interactive` so the
/// position is honoured.
pub fn set_node_position(src: &str, id: &str, x: f64, y: f64) -> Result<String, String> {
    if !valid_id(id) {
        return Err(format!("invalid node id `{id}`"));
    }
    let line = format!("%% @odg pos {id} {},{}", x.round() as i64, y.round() as i64);
    let mut out = upsert(src, |k, a| k == "pos" && a.first() == Some(&id), Some(line));
    let mode = layout_directives(&out).mode;
    if mode == "auto" || mode == "elk" {
        out = set_layout_mode(&out, "elk-interactive")?;
    }
    Ok(out)
}

/// `layer`: Some("first" | "last") or None to remove the constraint.
pub fn set_node_layer(src: &str, id: &str, layer: Option<&str>) -> Result<String, String> {
    if !valid_id(id) {
        return Err(format!("invalid node id `{id}`"));
    }
    if let Some(l) = layer {
        if l != "first" && l != "last" {
            return Err(format!("layer must be first or last, got `{l}`"));
        }
    }
    let line = layer.map(|l| format!("%% @odg layer {id} {l}"));
    Ok(upsert(src, |k, a| k == "layer" && a.first() == Some(&id), line))
}

/// Removes every `pos` directive (`id = None`) or only one node's.
pub fn clear_positions(src: &str, id: Option<&str>) -> String {
    upsert(src, |k, a| k == "pos" && id.is_none_or(|i| a.first() == Some(&i)), None)
}

/// Applies `old -> new` replacements in order; each `old` must occur exactly once.
pub fn apply_edits(source: &str, edits: &[(String, String)]) -> Result<String, String> {
    let mut out = source.to_string();
    for (i, (old, new)) in edits.iter().enumerate() {
        if old.is_empty() {
            return Err(format!("edit #{}: old_text is empty", i + 1));
        }
        let count = out.matches(old.as_str()).count();
        if count != 1 {
            return Err(format!("edit #{}: old_text must occur exactly once, found {} occurrence(s)", i + 1, count));
        }
        out = out.replacen(old.as_str(), new, 1);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_matter() {
        assert_eq!(front_matter_lines("---\ntitle: X\n---\nflowchart TD"), 3);
        assert_eq!(front_matter_lines("flowchart TD\n---"), 0);
        assert_eq!(front_matter_lines("---\nunterminated"), 0);
    }

    #[test]
    fn finds_ids_as_tokens() {
        let src = "flowchart LR\n  %% A is a comment\n  AB --> C\n  A[Start] --> AB";
        assert_eq!(find_line(src, "A"), Some(4));
        assert_eq!(find_line(src, "AB"), Some(3));
        assert_eq!(find_line(src, "Z"), None);
    }

    #[test]
    fn brackets() {
        assert_eq!(unclosed_bracket("  B -->|yes| C[Save"), Some('['));
        assert_eq!(unclosed_bracket(r#"  A["f(x"] --> B(ok)"#), None);
        assert_eq!(unclosed_bracket("  A{x} --> (C"), Some('('));
    }

    #[test]
    fn edits() {
        assert_eq!(apply_edits("a b c", &[("b".into(), "x".into())]).unwrap(), "a x c");
        assert!(apply_edits("a a", &[("a".into(), "x".into())]).is_err());
        assert!(apply_edits("a", &[("z".into(), "x".into())]).is_err());
    }

    #[test]
    fn layout_directives_round_trip() {
        let src = "flowchart TD\n  A --> B\n";
        let s1 = set_node_position(src, "B", 120.4, 80.6).unwrap();
        assert_eq!(s1, "flowchart TD\n  A --> B\n%% @odg pos B 120,81\n%% @odg layout elk-interactive\n");
        let s2 = set_node_position(&s1, "B", 10.0, 20.0).unwrap();
        assert!(s2.contains("%% @odg pos B 10,20") && !s2.contains("120,81"));
        let s3 = set_layout_mode(&s2, "exact").unwrap();
        let d = layout_directives(&s3);
        assert_eq!(d.mode, "exact");
        assert_eq!(d.positions, vec![NodePosition { id: "B".into(), x: 10.0, y: 20.0, line: 3 }]);
        let s4 = set_node_layer(&s3, "A", Some("first")).unwrap();
        assert_eq!(layout_directives(&s4).layers[0].layer, "first");
        let s5 = clear_positions(&s4, None);
        assert!(layout_directives(&s5).positions.is_empty());
        assert_eq!(set_layout_mode(&s5, "auto").unwrap().matches("@odg layout").count(), 0);
        assert!(set_layout_mode(src, "grid").is_err());
        assert!(set_node_position(src, "bad id", 0.0, 0.0).is_err());
        assert!(set_node_layer(src, "A", Some("middle")).is_err());
    }

    #[test]
    fn ignores_lookalike_comments() {
        let d = layout_directives("flowchart TD\n%% @odgx pos A 1,2\n%% note: @odg pos A 1,2\n%%@odg layout exact\n");
        assert_eq!(d.mode, "exact");
        assert!(d.positions.is_empty());
    }

    #[test]
    fn blank() {
        assert!(is_blank("\n %% hi\n"));
        assert!(!is_blank("graph TD"));
    }
}
