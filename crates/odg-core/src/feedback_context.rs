//! The short context a human comment carries to the agent: which diagram was open, which element was
//! selected, where it is declared and how it is connected. Built from the source only, so every app and
//! engine produces the same text; the human sees it (and may edit it) before sending.

use regex::Regex;

use crate::source;
use crate::Lang;

/// Header keyword of the diagram ("flowchart", "sequenceDiagram"…), after front matter and comments.
fn diagram_kind(src: &str) -> Option<String> {
    src.lines()
        .skip(source::front_matter_lines(src) as usize)
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with("%%"))
        .and_then(|l| l.split_whitespace().next())
        .map(str::to_string)
}

/// Visible text of an element where it is declared: `id[Label]`, `id("Label")`, `participant id as Label`…
fn label_of(line: &str, id: &str) -> Option<String> {
    let e = regex::escape(id);
    let shapes = Regex::new(&format!(r#"(?:^|[^\w-]){e}\s*(?:\[\[|\[\(|\(\[|\(\(|\[/|\[\\|\{{\{{|\[|\(|\{{|>)\s*"?([^"\]\)\}}]+?)"?\s*(?:\]\]|\)\]|\]\)|\)\)|/\]|\\\]|\}}\}}|\]|\)|\}})"#)).ok()?;
    if let Some(c) = shapes.captures(line) {
        return Some(c[1].trim().to_string());
    }
    let alias = Regex::new(&format!(r"(?:participant|actor)\s+{e}\s+as\s+(.+)$")).ok()?;
    alias.captures(line.trim()).map(|c| c[1].trim().to_string())
}

fn is_connection(line: &str) -> bool {
    ["-->", "->", "--", "==", "-.", "..", "<|", "|>", "}|", "|{", "o{", "}o", "|o", "o|", "~~"].iter().any(|a| line.contains(a))
}

/// The context text for a comment. `element` is the selected element id, if any.
pub fn build(src: &str, path: &str, element: Option<&str>, lang: Lang) -> String {
    let pt = lang == Lang::PtBr;
    let mut out = vec![];
    let kind = diagram_kind(src).map(|k| format!(" ({k})")).unwrap_or_default();
    out.push(format!("{} {path}{kind}", if pt { "Diagrama:" } else { "Diagram:" }));
    let Some(id) = element.map(str::trim).filter(|e| !e.is_empty()) else {
        out.push(if pt { "Nenhum elemento selecionado." } else { "No element selected." }.to_string());
        return out.join("\n");
    };
    let decl = source::find_line(src, id);
    let label = decl.and_then(|l| source::line_text(src, l)).and_then(|t| label_of(t, id));
    out.push(match &label {
        Some(l) if l != id => format!("{} {id} \"{l}\"", if pt { "Elemento selecionado:" } else { "Selected element:" }),
        _ => format!("{} {id}", if pt { "Elemento selecionado:" } else { "Selected element:" }),
    });
    let word = Regex::new(&format!(r"(^|[^\w-]){}($|[^\w-])", regex::escape(id))).unwrap();
    let skip = source::front_matter_lines(src) as usize;
    let mentions: Vec<(usize, &str)> = src
        .lines()
        .enumerate()
        .skip(skip)
        .filter(|(_, l)| !l.trim_start().starts_with("%%") && word.is_match(l))
        .map(|(i, l)| (i + 1, l.trim()))
        .collect();
    if let Some((n, text)) = mentions.first() {
        out.push(format!("{} {n}: {text}", if pt { "Linha" } else { "Line" }));
    }
    // every line connecting it, in source order
    let links: Vec<String> = mentions
        .iter()
        .filter(|(_, t)| is_connection(t))
        .take(8)
        .map(|(n, t)| format!("  - {} {n}: {t}", if pt { "linha" } else { "line" }))
        .collect();
    if !links.is_empty() {
        out.push(if pt { "Ligações:" } else { "Connections:" }.to_string());
        out.extend(links);
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "---\ntitle: Order flow\n---\nflowchart LR\n    start([Customer]) --> validate{Stock?}\n    validate -->|yes| pay[Charge card]\n    pay --> ship[Ship order]\n";

    #[test]
    fn context_names_diagram_element_line_and_links() {
        let c = build(SRC, "diagrams/order-flow.mmd", Some("pay"), Lang::PtBr);
        assert!(c.starts_with("Diagrama: diagrams/order-flow.mmd (flowchart)"), "{c}");
        assert!(c.contains("Elemento selecionado: pay \"Charge card\""), "{c}");
        assert!(c.contains("Linha 6: validate -->|yes| pay[Charge card]"), "{c}");
        assert!(c.contains("linha 7: pay --> ship[Ship order]"), "{c}");
        let none = build(SRC, "diagrams/order-flow.mmd", None, Lang::En);
        assert!(none.ends_with("No element selected."));
        assert_eq!(label_of("    participant A as API", "A").as_deref(), Some("API"));
        assert_eq!(label_of("    validate{Stock available?}", "validate").as_deref(), Some("Stock available?"));
    }
}
