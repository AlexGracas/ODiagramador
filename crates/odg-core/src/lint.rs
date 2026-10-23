//! Turns a raw render (syntax errors + measured geometry) into localized problems.
//!
//! Rules work on the engine-neutral [`DiagramModel`], so a future UML engine gets them for free.

use std::collections::{BTreeMap, HashSet};

use regex::Regex;

use crate::i18n::{t, Arg, Lang};
use crate::model::{BBox, Node, RawRender, SourceError};
use crate::report::{Problem, Severity};
use crate::source;

#[derive(Debug, Clone)]
pub struct LintConfig {
    pub max_label_chars: usize,
    pub max_nodes: usize,
    pub max_edges: usize,
    /// width/height (or height/width) ratio considered extreme
    pub aspect_ratio: f64,
    /// only flag extreme aspect ratios above this many px on the long side
    pub aspect_min_px: f64,
    /// intersection / smaller area above which two nodes overlap
    pub overlap_ratio: f64,
}

impl Default for LintConfig {
    fn default() -> Self {
        LintConfig {
            max_label_chars: 60,
            max_nodes: 60,
            max_edges: 100,
            aspect_ratio: 4.0,
            aspect_min_px: 1600.0,
            overlap_ratio: 0.05,
        }
    }
}

enum V {
    S(String),
    N(f64),
}

impl From<&str> for V {
    fn from(s: &str) -> Self {
        V::S(s.to_string())
    }
}
impl From<String> for V {
    fn from(s: String) -> Self {
        V::S(s)
    }
}
impl From<usize> for V {
    fn from(n: usize) -> Self {
        V::N(n as f64)
    }
}
impl From<u32> for V {
    fn from(n: u32) -> Self {
        V::N(n as f64)
    }
}
impl From<f64> for V {
    fn from(n: f64) -> Self {
        V::N(n.round())
    }
}

fn tr(lang: Lang, key: &str, args: &[(&str, V)]) -> String {
    let a: Vec<(&str, Arg)> = args
        .iter()
        .map(|(k, v)| {
            (
                *k,
                match v {
                    V::S(s) => Arg::Str(s.as_str()),
                    V::N(n) => Arg::Num(*n),
                },
            )
        })
        .collect();
    t(lang, key, &a)
}

struct Draft {
    severity: Severity,
    code: &'static str,
    message: String,
    hint: String,
    line: Option<u32>,
    column: Option<u32>,
    element_id: Option<String>,
    related: Vec<String>,
    bbox: Option<BBox>,
}

impl Draft {
    fn new(lang: Lang, severity: Severity, code: &'static str, args: &[(&str, V)]) -> Draft {
        Draft {
            severity,
            code,
            message: tr(lang, &format!("problem-{code}"), args),
            hint: tr(lang, &format!("hint-{code}"), args),
            line: None,
            column: None,
            element_id: None,
            related: vec![],
            bbox: None,
        }
    }

    fn at_node(mut self, src: &str, n: &Node) -> Draft {
        self.line = source::find_line(src, &n.id);
        self.element_id = Some(n.id.clone());
        self.bbox = n.bbox;
        self
    }
}

/// Builds the localized problem list for `source` given what the render-kit reported.
pub fn lint(src: &str, raw: &RawRender, lang: Lang, cfg: &LintConfig) -> Vec<Problem> {
    let mut out: Vec<Draft> = vec![];

    if source::is_blank(src) {
        out.push(Draft::new(lang, Severity::Error, "E_EMPTY", &[]));
        return finish(out);
    }
    if let Some(err) = &raw.parse_error {
        out.push(syntax_problem(src, err, lang));
        return finish(out);
    }
    if let Some(err) = &raw.render_error {
        let mut d = Draft::new(lang, Severity::Error, "E_RENDER", &[("detail", clean_detail(&err.message).into())]);
        d.line = err.line.map(|l| l + source::front_matter_lines(src));
        out.push(d);
        return finish(out);
    }

    let kind = raw.diagram_type.clone().unwrap_or_default();
    let model = &raw.model;

    // ---- orphans: graph-like diagrams only
    let graph_like = ["flowchart", "graph", "class", "state", "er", "sequence", "requirement"]
        .iter()
        .any(|p| kind.starts_with(p));
    if graph_like && model.nodes.len() > 1 {
        let mut linked: HashSet<&str> = HashSet::new();
        for e in &model.edges {
            linked.insert(e.from.as_str());
            linked.insert(e.to.as_str());
        }
        let sev = if kind.starts_with("class") || kind.starts_with("er") { Severity::Info } else { Severity::Warning };
        for n in &model.nodes {
            if linked.contains(n.id.as_str()) || n.id.ends_with("_start") || n.id.ends_with("_end") {
                continue;
            }
            let label = display(n);
            out.push(
                Draft::new(lang, sev, "W_ORPHAN_NODE", &[("label", label.into()), ("id", n.id.as_str().into())])
                    .at_node(src, n),
            );
        }
    }

    // ---- overlaps between nodes that are not nested
    let measured: Vec<&Node> = model.nodes.iter().filter(|n| n.bbox.is_some()).take(400).collect();
    for (i, a) in measured.iter().enumerate() {
        for b in &measured[i + 1..] {
            let (ba, bb) = (a.bbox.unwrap(), b.bbox.unwrap());
            if ba.contains(&bb) || bb.contains(&ba) {
                continue;
            }
            let inter = ba.intersection(&bb);
            let smaller = ba.area().min(bb.area());
            if smaller > 0.0 && inter / smaller > cfg.overlap_ratio {
                let mut d = Draft::new(
                    lang,
                    Severity::Warning,
                    "W_OVERLAP",
                    &[("a", display(a).into()), ("b", display(b).into())],
                )
                .at_node(src, a);
                d.related = vec![b.id.clone()];
                d.bbox = Some(ba.union(&bb));
                out.push(d);
            }
        }
    }

    // ---- labels
    for n in &model.nodes {
        if n.label_overflow == Some(true) {
            out.push(Draft::new(lang, Severity::Warning, "W_LABEL_OVERFLOW", &[("label", display(n).into())]).at_node(src, n));
        }
        let len = n.label.chars().count();
        if len > cfg.max_label_chars {
            out.push(
                Draft::new(
                    lang,
                    Severity::Info,
                    "W_LONG_LABEL",
                    &[("id", n.id.as_str().into()), ("length", len.into()), ("max", cfg.max_label_chars.into())],
                )
                .at_node(src, n),
            );
        }
    }

    // ---- duplicate labels (flowcharts: usually a node accidentally declared twice)
    if kind.starts_with("flowchart") || kind.starts_with("graph") {
        let mut by_label: BTreeMap<String, Vec<&Node>> = BTreeMap::new();
        for n in &model.nodes {
            let l = n.label.trim().to_lowercase();
            if l.chars().count() > 1 {
                by_label.entry(l).or_default().push(n);
            }
        }
        for nodes in by_label.values().filter(|v| v.len() > 1) {
            let ids = nodes.iter().map(|n| n.id.as_str()).collect::<Vec<_>>().join(", ");
            let mut d = Draft::new(
                lang,
                Severity::Warning,
                "W_DUPLICATE_LABEL",
                &[("label", nodes[0].label.as_str().into()), ("ids", ids.into())],
            )
            .at_node(src, nodes[0]);
            d.related = nodes[1..].iter().map(|n| n.id.clone()).collect();
            d.bbox = nodes.iter().filter_map(|n| n.bbox).reduce(|a, b| a.union(&b));
            out.push(d);
        }
    }

    // ---- @odg layout directives
    let dirs = source::layout_directives(src);
    let has_layout = dirs.mode != "auto" || !dirs.positions.is_empty() || !dirs.layers.is_empty();
    let layout_capable = ["flowchart", "graph", "state", "class", "er", "requirement"].iter().any(|p| kind.starts_with(p));
    if has_layout && !layout_capable {
        let mut d = Draft::new(lang, Severity::Info, "W_LAYOUT_UNSUPPORTED", &[("type", kind.as_str().into())]);
        d.line = dirs.mode_line.or(dirs.positions.first().map(|p| p.line));
        out.push(d);
    } else if has_layout {
        if !source::LAYOUT_MODES.contains(&dirs.mode.as_str()) {
            let mut d = Draft::new(lang, Severity::Warning, "W_LAYOUT_MODE", &[("mode", dirs.mode.as_str().into()), ("modes", source::LAYOUT_MODES.join(", ").into())]);
            d.line = dirs.mode_line;
            out.push(d);
        }
        let known = |id: &str| model.node(id).is_some();
        for (id, line) in dirs.positions.iter().map(|p| (&p.id, p.line)).chain(dirs.layers.iter().map(|l| (&l.id, l.line))) {
            if !known(id) {
                let mut d = Draft::new(lang, Severity::Warning, "W_LAYOUT_UNKNOWN_ID", &[("id", id.as_str().into())]);
                d.line = Some(line);
                out.push(d);
            }
        }
        if !dirs.positions.is_empty() && (dirs.mode == "auto" || dirs.mode == "elk") {
            let mut d = Draft::new(lang, Severity::Info, "W_LAYOUT_IGNORED", &[("mode", dirs.mode.as_str().into())]);
            d.line = Some(dirs.positions[0].line);
            out.push(d);
        }
    }

    // ---- size and shape
    if model.nodes.len() > cfg.max_nodes || model.edges.len() > cfg.max_edges {
        out.push(Draft::new(
            lang,
            Severity::Info,
            "W_TOO_LARGE",
            &[("nodes", model.nodes.len().into()), ("edges", model.edges.len().into())],
        ));
    }
    let (w, h) = (raw.width, raw.height);
    let dims = || [("width", V::from(w)), ("height", V::from(h))];
    if w > cfg.aspect_min_px && h > 0.0 && w / h > cfg.aspect_ratio {
        out.push(Draft::new(lang, Severity::Info, "W_WIDE", &dims()));
    }
    let naturally_tall = kind.starts_with("sequence") || kind.starts_with("gantt") || kind.starts_with("timeline");
    if !naturally_tall && h > cfg.aspect_min_px && w > 0.0 && h / w > cfg.aspect_ratio {
        out.push(Draft::new(lang, Severity::Info, "W_TALL", &dims()));
    }

    if raw.has_foreign_object {
        out.push(Draft::new(lang, Severity::Info, "W_FOREIGN_OBJECT", &[]));
    }
    if let Some(e) = &raw.raster_error {
        out.push(Draft::new(lang, Severity::Warning, "W_RASTER", &[("detail", e.as_str().into())]));
    }
    finish(out)
}

fn display(n: &Node) -> String {
    if n.label.trim().is_empty() {
        n.id.clone()
    } else {
        n.label.clone()
    }
}

fn syntax_problem(src: &str, err: &SourceError, lang: Lang) -> Draft {
    if err.kind == "unknown_type" {
        let mut d = Draft::new(lang, Severity::Error, "E_UNKNOWN_TYPE", &[]);
        let skip = source::front_matter_lines(src) as usize;
        d.line = src
            .lines()
            .enumerate()
            .skip(skip)
            .find(|(_, l)| !l.trim().is_empty() && !l.trim_start().starts_with("%%"))
            .map(|(i, _)| i as u32 + 1);
        return d;
    }
    let line = err.line.map(|l| l + source::front_matter_lines(src));
    let mut detail = clean_detail(&err.message);
    if detail.starts_with("unexpected ") {
        let symbolic = |t: &str| t.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
        if let Some(tok) = err.token.as_deref().filter(|t| !t.trim().is_empty() && !symbolic(t)) {
            detail = tr(lang, "detail-unexpected-token", &[("token", tok.trim().into())]);
        } else if detail.contains("'EOF'") {
            detail = tr(lang, "detail-unexpected-eof", &[]);
        } else if detail.contains("'NEWLINE'") {
            detail = tr(lang, "detail-unexpected-eol", &[]);
        }
    }
    let expected = err
        .expected
        .as_ref()
        .filter(|v| !v.is_empty())
        .map(|v| {
            let tokens = v.iter().take(8).map(|s| friendly_token(s.trim_matches('\''))).collect::<Vec<_>>().join(", ");
            tr(lang, "hint-expected", &[("tokens", tokens.into())])
        })
        .unwrap_or_default();
    let mut d = match line {
        Some(l) => {
            let mut d = Draft::new(
                lang,
                Severity::Error,
                "E_PARSE",
                &[("line", l.into()), ("detail", detail.into()), ("expected", expected.into())],
            );
            if let Some(text) = source::line_text(src, l) {
                if let Some(open) = source::unclosed_bracket(text) {
                    let close = match open {
                        '[' => ']',
                        '(' => ')',
                        _ => '}',
                    };
                    let extra = tr(lang, "hint-unclosed", &[("line", l.into()), ("open", open.to_string().into()), ("close", close.to_string().into())]);
                    d.hint = format!("{extra} {}", d.hint);
                }
                d.hint = format!("{} [{}: `{}`]", d.hint.trim_end(), l, text.trim());
            }
            d
        }
        None => {
            let mut d = Draft::new(lang, Severity::Error, "E_PARSE", &[("detail", detail.clone().into())]);
            d.message = tr(lang, "problem-E_PARSE-noline", &[("detail", detail.into())]);
            d.hint = tr(lang, "hint-E_PARSE-noline", &[]);
            d
        }
    };
    d.line = line;
    d.column = err.column;
    d
}

/// Mermaid grammar token names → what the author has to type.
fn friendly_token(t: &str) -> String {
    let sym = match t {
        "SQE" => "]",
        "SQS" => "[",
        "PE" => ")",
        "PS" => "(",
        "DIAMOND_STOP" => "}",
        "DIAMOND_START" => "{",
        "PIPE" => "|",
        "COLON" => ":",
        "SEMI" => ";",
        "AMP" => "&",
        "COMMA" => ",",
        "MINUS" => "-",
        "STR" => "\"text\"",
        "TEXT" | "TEXT_TOKEN" | "UNICODE_TEXT" => "text",
        "NODE_STRING" => "node id",
        "NEWLINE" => "new line",
        "EOF" => "end of input",
        "ARROW_POINT" | "LINK" | "ARROW" => "-->",
        _ => return t.to_string(),
    };
    format!("{sym} ({t})")
}

/// Strips Mermaid's context lines and prefixes, keeping the actual complaint.
pub fn clean_detail(message: &str) -> String {
    let body = message
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.starts_with("...") && !(t.chars().all(|c| c == '-' || c == '^') && !t.is_empty())
        })
        .collect::<Vec<_>>()
        .join(" ");
    let prefix = Regex::new(r"^\s*(Parsing failed:\s*)?(Parse error on line \d+(, column \d+)?:\s*)?").unwrap();
    let mut s = prefix.replace(&body, "").trim().to_string();
    if s.starts_with("Expecting ") {
        if let Some(pos) = s.rfind(", got ") {
            s = format!("unexpected {}", &s[pos + 6..]);
        }
    }
    if s.chars().count() > 300 {
        s = s.chars().take(300).collect::<String>() + "…";
    }
    s
}

fn finish(mut drafts: Vec<Draft>) -> Vec<Problem> {
    drafts.sort_by_key(|d| d.severity);
    drafts
        .into_iter()
        .enumerate()
        .map(|(i, d)| Problem {
            index: i + 1,
            severity: d.severity,
            code: d.code.to_string(),
            message: d.message,
            hint: d.hint,
            line: d.line,
            column: d.column,
            element_id: d.element_id,
            related: d.related,
            bbox: d.bbox,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DiagramModel, Edge};

    fn node(id: &str, label: &str, x: f64, y: f64) -> Node {
        Node { id: id.into(), label: label.into(), bbox: Some(BBox { x, y, w: 50.0, h: 30.0 }), ..Default::default() }
    }

    fn flow(nodes: Vec<Node>, edges: Vec<(&str, &str)>) -> RawRender {
        RawRender {
            ok: true,
            diagram_type: Some("flowchart-v2".into()),
            width: 400.0,
            height: 300.0,
            model: DiagramModel {
                nodes,
                edges: edges
                    .into_iter()
                    .map(|(a, b)| Edge { id: format!("{a}{b}"), from: a.into(), to: b.into(), ..Default::default() })
                    .collect(),
                groups: vec![],
            },
            ..Default::default()
        }
    }

    fn codes(p: &[Problem]) -> Vec<&str> {
        p.iter().map(|p| p.code.as_str()).collect()
    }

    #[test]
    fn clean_graph_has_no_problems() {
        let raw = flow(vec![node("A", "Start", 0.0, 0.0), node("B", "End", 100.0, 0.0)], vec![("A", "B")]);
        assert!(lint("flowchart LR\n A --> B", &raw, Lang::En, &LintConfig::default()).is_empty());
    }

    #[test]
    fn detects_orphan_overlap_and_duplicates() {
        let src = "flowchart LR\n A[Start] --> B\n C[Start]\n";
        let raw = flow(
            vec![node("A", "Start", 0.0, 0.0), node("B", "B", 20.0, 10.0), node("C", "Start", 300.0, 0.0)],
            vec![("A", "B")],
        );
        let p = lint(src, &raw, Lang::En, &LintConfig::default());
        assert_eq!(codes(&p), vec!["W_ORPHAN_NODE", "W_OVERLAP", "W_DUPLICATE_LABEL"]);
        assert_eq!(p[0].line, Some(3));
        assert_eq!(p[0].element_id.as_deref(), Some("C"));
        assert_eq!(p[0].index, 1);
    }

    #[test]
    fn parse_error_is_shifted_by_front_matter_and_localized() {
        let src = "---\ntitle: X\n---\nflowchart TD\n  A --> B\n  B --> (C";
        let raw = RawRender {
            parse_error: Some(SourceError {
                kind: "parse".into(),
                message: "Parse error on line 3:\n...B --> (C\n----^\nExpecting 'AMP', 'COLON', got 'PS'".into(),
                line: Some(3),
                column: Some(4),
                token: Some("(".into()),
                expected: Some(vec!["'AMP'".into(), "'COLON'".into()]),
            }),
            ..Default::default()
        };
        let p = lint(src, &raw, Lang::PtBr, &LintConfig::default());
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].code, "E_PARSE");
        assert_eq!(p[0].line, Some(6));
        assert_eq!(p[0].message, "Erro de sintaxe na linha 6: símbolo inesperado '('");
        assert!(p[0].hint.contains("& (AMP), : (COLON)"));
        assert!(p[0].hint.contains("abre '('"));
        assert!(p[0].hint.contains("`B --> (C`"));
    }

    #[test]
    fn empty_and_unknown() {
        assert_eq!(codes(&lint("  \n", &RawRender::default(), Lang::En, &LintConfig::default())), vec!["E_EMPTY"]);
        let raw = RawRender {
            parse_error: Some(SourceError { kind: "unknown_type".into(), message: "No diagram type detected".into(), ..Default::default() }),
            ..Default::default()
        };
        let p = lint("%% c\nhello", &raw, Lang::En, &LintConfig::default());
        assert_eq!(codes(&p), vec!["E_UNKNOWN_TYPE"]);
        assert_eq!(p[0].line, Some(2));
    }

    #[test]
    fn layout_directive_problems() {
        let src = "flowchart TD\n A --> B\n%% @odg layout exact\n%% @odg pos Z 1,2\n";
        let raw = flow(vec![node("A", "a", 0.0, 0.0), node("B", "b", 100.0, 0.0)], vec![("A", "B")]);
        let p = lint(src, &raw, Lang::En, &LintConfig::default());
        assert_eq!(codes(&p), vec!["W_LAYOUT_UNKNOWN_ID"]);
        assert_eq!(p[0].line, Some(4));
        let p = lint("flowchart TD\n A --> B\n%% @odg pos A 1,2\n", &raw, Lang::En, &LintConfig::default());
        assert_eq!(codes(&p), vec!["W_LAYOUT_IGNORED"]);
        let p = lint("flowchart TD\n A --> B\n%% @odg layout grid\n", &raw, Lang::En, &LintConfig::default());
        assert_eq!(codes(&p), vec!["W_LAYOUT_MODE"]);
        let mut seq = raw.clone();
        seq.diagram_type = Some("sequence".into());
        let p = lint("sequenceDiagram\n A->>B: x\n%% @odg layout exact\n", &seq, Lang::En, &LintConfig::default());
        assert_eq!(codes(&p), vec!["W_LAYOUT_UNSUPPORTED"]);
    }

    #[test]
    fn langium_message_is_cleaned() {
        assert_eq!(
            clean_detail("Parsing failed:  Parse error on line 3, column 10: Expecting token of type ':' but found `85`."),
            "Expecting token of type ':' but found `85`."
        );
    }

    #[test]
    fn wide_diagrams() {
        let mut raw = flow(vec![node("A", "a", 0.0, 0.0), node("B", "b", 100.0, 0.0)], vec![("A", "B")]);
        raw.width = 5000.0;
        raw.height = 200.0;
        assert_eq!(codes(&lint("flowchart LR\nA-->B", &raw, Lang::En, &LintConfig::default())), vec!["W_WIDE"]);
    }
}
