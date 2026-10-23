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
    fn blank() {
        assert!(is_blank("\n %% hi\n"));
        assert!(!is_blank("graph TD"));
    }
}
