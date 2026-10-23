//! The validate → render → lint → annotate pipeline shared by the CLI, MCP server and apps.

use std::time::Instant;

use base64::Engine as _;
use regex::Regex;

use crate::engine::{DiagramEngine, RenderBackend};
use crate::i18n::{t, Arg, Lang};
use crate::lint::{lint, LintConfig};
use crate::model::{BBox, ProcessRequest, RawRender};
use crate::report::{Previews, Problem, Report, Severity, Stats};
use crate::{now_rfc3339, sha256_hex, source};

#[derive(Debug, Clone)]
pub struct AnalyzeOptions {
    pub lang: Lang,
    /// Mermaid theme: default | dark | forest | neutral | base
    pub theme: String,
    pub png: bool,
    pub annotated: bool,
    pub scale: f64,
    pub lint: LintConfig,
}

impl Default for AnalyzeOptions {
    fn default() -> Self {
        AnalyzeOptions { lang: Lang::En, theme: "default".into(), png: true, annotated: true, scale: 2.0, lint: LintConfig::default() }
    }
}

#[derive(Debug, Clone)]
pub struct Analysis {
    pub report: Report,
    pub raw: RawRender,
    pub svg: Option<String>,
    pub png: Option<Vec<u8>>,
    /// Diagram with numbered problem markers and a legend, or an error card with the
    /// offending source line when the diagram does not parse.
    pub annotated_png: Option<Vec<u8>>,
}

/// Builds the report from a render-kit result. Used directly by the desktop apps (via FFI),
/// which run the render-kit in their own WebView.
pub fn build_report(src: &str, raw: &RawRender, engine: &str, lang: Lang, cfg: &LintConfig, duration_ms: u64) -> Report {
    let problems = lint(src, raw, lang, cfg);
    let count = |s: Severity| problems.iter().filter(|p| p.severity == s).count();
    let (errors, warnings, infos) = (count(Severity::Error), count(Severity::Warning), count(Severity::Info));
    let stats = Stats {
        nodes: raw.model.nodes.len(),
        edges: raw.model.edges.len(),
        groups: raw.model.groups.len(),
        width: raw.width,
        height: raw.height,
        lines: src.lines().count(),
    };
    let summary = if errors > 0 {
        t(lang, "summary-invalid", &[("errors", Arg::from(errors))])
    } else {
        let problems_txt = t(lang, "summary-problems", &[("warnings", Arg::from(warnings))]);
        t(
            lang,
            "summary-valid",
            &[
                ("type", Arg::Str(raw.diagram_type.as_deref().unwrap_or(engine))),
                ("nodes", Arg::from(stats.nodes)),
                ("edges", Arg::from(stats.edges)),
                ("problems", Arg::Str(&problems_txt)),
            ],
        )
    };
    Report {
        path: None,
        engine: engine.to_string(),
        diagram_type: raw.diagram_type.clone(),
        valid: errors == 0,
        error_count: errors,
        warning_count: warnings,
        info_count: infos,
        summary,
        problems,
        stats,
        previews: Previews::default(),
        source_sha256: sha256_hex(src.as_bytes()),
        lang: lang.code().to_string(),
        generated_at: now_rfc3339(),
        duration_ms,
    }
}

/// Full pipeline with a render backend (headless webview in the CLI / MCP server).
pub fn analyze(engine: &dyn DiagramEngine, src: &str, backend: &dyn RenderBackend, opts: &AnalyzeOptions) -> Analysis {
    let started = Instant::now();
    let failure = |code: &str, detail: String| {
        let raw = RawRender::default();
        let mut report = build_report(src, &raw, engine.id(), opts.lang, &opts.lint, 0);
        let p = Problem {
            index: 1,
            severity: Severity::Error,
            code: code.to_string(),
            message: t(opts.lang, &format!("problem-{code}"), &[("detail", Arg::Str(&detail))]),
            hint: t(opts.lang, &format!("hint-{code}"), &[]),
            line: None,
            column: None,
            element_id: None,
            related: vec![],
            bbox: None,
        };
        report.problems = vec![p];
        report.valid = false;
        report.error_count = 1;
        report.warning_count = 0;
        report.info_count = 0;
        report.summary = t(opts.lang, "summary-invalid", &[("errors", Arg::from(1usize))]);
        Analysis { report, raw, svg: None, png: None, annotated_png: None }
    };

    let mermaid = match engine.prepare(src) {
        Ok(m) => m,
        Err(e) => return failure("E_RENDER", e),
    };
    let req = ProcessRequest {
        source: mermaid,
        theme: opts.theme.clone(),
        png: opts.png,
        scale: opts.scale,
        background: None,
    };
    let raw = match backend.process(&req) {
        Ok(r) => r,
        Err(e) => return failure("E_BACKEND", e),
    };
    let elapsed = started.elapsed().as_millis() as u64;
    let report = build_report(src, &raw, engine.id(), opts.lang, &opts.lint, elapsed);
    let png = raw.png.as_ref().and_then(|b| base64::engine::general_purpose::STANDARD.decode(b).ok());
    let svg = raw.svg.clone();
    let bg = if opts.theme == "dark" { "#0d1117" } else { "#ffffff" };

    let annotated_png = if !opts.annotated {
        None
    } else if let Some(svg) = svg.as_deref().filter(|_| !report.problems.is_empty()) {
        let annotated = annotate_svg(svg, raw.view_box, &report.problems);
        backend.rasterize(&annotated, opts.scale, bg).ok()
    } else if !report.valid {
        let card = error_card_svg(src, &report);
        backend.rasterize(&card, 1.5, "#ffffff").ok()
    } else {
        None
    };

    Analysis { report, raw, svg, png, annotated_png }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn color(sev: Severity) -> &'static str {
    match sev {
        Severity::Error => "#d1242f",
        Severity::Warning => "#d97706",
        Severity::Info => "#0969da",
    }
}

/// Draws numbered markers around problem areas plus a legend band under the diagram.
pub fn annotate_svg(svg: &str, view_box: Option<BBox>, problems: &[Problem]) -> String {
    let vb = view_box.unwrap_or(BBox { x: 0.0, y: 0.0, w: 800.0, h: 600.0 });
    let margin = 20.0;
    let line_h = 20.0;
    let legend: Vec<&Problem> = problems.iter().take(12).collect();
    let legend_h = if legend.is_empty() { 0.0 } else { 16.0 + line_h * legend.len() as f64 };
    let width = vb.w.max(560.0) + margin * 2.0;
    let nvb = BBox { x: vb.x - margin, y: vb.y - margin, w: width, h: vb.h + margin * 2.0 + legend_h };

    let mut g = String::from(r#"<g id="odg-annotations" font-family="Arial, Helvetica, sans-serif">"#);
    // one dashed box per area; badges for problems sharing an area are stacked side by side
    let mut drawn: Vec<(BBox, usize)> = vec![];
    for p in problems.iter().filter(|p| p.bbox.is_some()) {
        let b = p.bbox.unwrap();
        let c = color(p.severity);
        let pad = 5.0;
        let slot = match drawn.iter_mut().find(|(d, _)| *d == b) {
            Some((_, n)) => {
                *n += 1;
                *n - 1
            }
            None => {
                drawn.push((b, 1));
                g.push_str(&format!(
                    r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="5" fill="{c}" fill-opacity="0.06" stroke="{c}" stroke-width="2.5" stroke-dasharray="7 4"/>"#,
                    b.x - pad, b.y - pad, b.w + pad * 2.0, b.h + pad * 2.0
                ));
                0
            }
        };
        let cx = b.x - pad + 22.0 * slot as f64;
        g.push_str(&format!(
            r##"<circle cx="{:.1}" cy="{:.1}" r="10" fill="{c}" stroke="#fff" stroke-width="1.5"/><text x="{:.1}" y="{:.1}" font-size="12" font-weight="bold" fill="#fff" text-anchor="middle">{}</text>"##,
            cx, b.y - pad, cx, b.y - pad + 4.0, p.index
        ));
    }
    if !legend.is_empty() {
        let top = vb.y + vb.h + margin;
        g.push_str(&format!(
            r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="#ffffff" stroke="#d0d7de"/>"##,
            nvb.x, top, nvb.w, legend_h + margin
        ));
        for (i, p) in legend.iter().enumerate() {
            let y = top + 18.0 + line_h * i as f64;
            let c = color(p.severity);
            let max_chars = ((nvb.w - 44.0) / 6.7).max(20.0) as usize;
            let full = format!("{} — {}", p.code, p.message);
            let text: String = if full.chars().count() > max_chars {
                full.chars().take(max_chars - 1).collect::<String>() + "…"
            } else {
                full
            };
            g.push_str(&format!(
                r##"<circle cx="{:.1}" cy="{:.1}" r="8" fill="{c}"/><text x="{:.1}" y="{:.1}" font-size="10" font-weight="bold" fill="#fff" text-anchor="middle">{}</text><text x="{:.1}" y="{:.1}" font-size="12" fill="#1f2328">{}</text>"##,
                nvb.x + 18.0, y - 4.0, nvb.x + 18.0, y - 0.5, p.index, nvb.x + 32.0, y, esc(&text)
            ));
        }
    }
    g.push_str("</g>");

    // grow the root viewBox / size so markers and legend are not clipped
    let open_end = svg.find('>').unwrap_or(0);
    let (head, rest) = svg.split_at(open_end);
    let set = |h: &str, attr: &str, val: String| -> String {
        let re = Regex::new(&format!(r#"\s{attr}="[^"]*""#)).unwrap();
        if re.is_match(h) {
            re.replace(h, format!(r#" {attr}="{val}""#).as_str()).into_owned()
        } else {
            format!(r#"{h} {attr}="{val}""#)
        }
    };
    let mut head = set(head, "viewBox", format!("{:.1} {:.1} {:.1} {:.1}", nvb.x, nvb.y, nvb.w, nvb.h));
    head = set(&head, "width", format!("{}", nvb.w.ceil()));
    head = set(&head, "height", format!("{}", nvb.h.ceil()));
    let mut out = head + rest;
    if let Some(pos) = out.rfind("</svg>") {
        out.insert_str(pos, &g);
    }
    out
}

/// When the diagram does not parse there is nothing to draw: show the source around the first
/// error instead, with the offending line highlighted. Agents with vision see exactly what is wrong.
pub fn error_card_svg(src: &str, report: &Report) -> String {
    let first = report.first_error();
    let err_line = first.and_then(|p| p.line);
    let lines: Vec<&str> = src.lines().collect();
    let (from, to) = match err_line {
        Some(l) => {
            let l = l as usize;
            (l.saturating_sub(7), (l + 5).min(lines.len()))
        }
        None => (0, lines.len().min(14)),
    };
    let char_w = 7.8;
    let line_h = 20.0;
    let longest = lines[from..to].iter().map(|l| l.chars().count()).max().unwrap_or(0).min(110);
    let width = (90.0 + longest as f64 * char_w).max(720.0);
    let hint_full = first.map(|p| p.hint.clone()).unwrap_or_default();
    let per_line = ((width - 32.0) / 6.6) as usize;
    let hint_lines = wrap(&hint_full, per_line, 3);
    let header_h = 62.0 + 16.0 * hint_lines.len() as f64;
    let height = header_h + line_h * (to - from) as f64 + 24.0;
    let msg: String = first.map(|p| p.message.clone()).unwrap_or_default().chars().take(140).collect();

    let mut s = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}"><rect width="100%" height="100%" fill="#ffffff"/><rect width="100%" height="{hh}" fill="#ffebe9"/><text x="16" y="28" font-family="Arial, Helvetica, sans-serif" font-size="15" font-weight="bold" fill="#d1242f">{msg}</text>"##,
        w = width.ceil(),
        h = height.ceil(),
        hh = header_h - 12.0,
        msg = esc(&msg),
    );
    for (i, l) in hint_lines.iter().enumerate() {
        s.push_str(&format!(
            r##"<text x="16" y="{:.1}" font-family="Arial, Helvetica, sans-serif" font-size="12" fill="#57606a">{}</text>"##,
            52.0 + 16.0 * i as f64,
            esc(l)
        ));
    }
    for (i, text) in lines[from..to].iter().enumerate() {
        let n = from + i + 1;
        let y = header_h + line_h * i as f64 + 10.0;
        let is_err = Some(n as u32) == err_line;
        if is_err {
            s.push_str(&format!(r##"<rect x="0" y="{:.1}" width="{:.1}" height="{line_h}" fill="#ffd7d5"/>"##, y - 14.0, width));
        }
        let shown: String = text.chars().take(110).collect();
        s.push_str(&format!(
            r##"<text x="40" y="{y:.1}" font-family="Menlo, Consolas, 'DejaVu Sans Mono', monospace" font-size="13" fill="#8c959f" text-anchor="end">{n}</text><text x="52" y="{y:.1}" font-family="Menlo, Consolas, 'DejaVu Sans Mono', monospace" font-size="13" fill="{}" xml:space="preserve">{}</text>"##,
            if is_err { "#82071e" } else { "#1f2328" },
            esc(&shown)
        ));
        if is_err {
            if let Some(col) = first.and_then(|p| p.column) {
                let x = 52.0 + (col.saturating_sub(1) as f64) * char_w;
                s.push_str(&format!(r##"<path d="M{x:.1} {:.1} l5 6 h-10 z" fill="#d1242f"/>"##, y + 5.0));
            }
        }
    }
    if err_line.is_some_and(|l| l as usize > lines.len()) {
        let y = header_h + line_h * (to - from) as f64 + 10.0;
        s.push_str(&format!(
            r##"<text x="52" y="{y:.1}" font-family="Menlo, Consolas, monospace" font-size="13" fill="#d1242f">⏎ end of input</text>"##
        ));
    }
    s.push_str("</svg>");
    s
}

/// Greedy word wrap into at most `max_lines` lines of `width` chars (last line gets "…").
fn wrap(text: &str, width: usize, max_lines: usize) -> Vec<String> {
    let mut lines: Vec<String> = vec![];
    let mut cur = String::new();
    for word in text.split_whitespace() {
        if !cur.is_empty() && cur.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    if lines.len() > max_lines {
        lines.truncate(max_lines);
        let last = lines.last_mut().unwrap();
        let keep: String = last.chars().take(width.saturating_sub(1)).collect();
        *last = keep + "…";
    }
    lines
}

/// Lines of `src` around `line`, numbered — used in text-only responses.
pub fn excerpt(src: &str, line: u32, radius: usize) -> String {
    let l = line as usize;
    src.lines()
        .enumerate()
        .skip(l.saturating_sub(radius + 1))
        .take(radius * 2 + 1)
        .map(|(i, t)| format!("{}{:>4} | {}", if i + 1 == l { ">" } else { " " }, i + 1, t))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Line/col helpers re-exported for consumers that only have the source.
pub fn first_error_excerpt(src: &str, report: &Report) -> Option<String> {
    report.first_error().and_then(|p| p.line).map(|l| excerpt(src, l, 3)).filter(|_| !source::is_blank(src))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::MermaidEngine;
    use crate::model::{DiagramModel, Node, SourceError};

    struct FakeBackend(RawRender);

    impl RenderBackend for FakeBackend {
        fn process(&self, _req: &ProcessRequest) -> Result<RawRender, String> {
            Ok(self.0.clone())
        }
        fn rasterize(&self, svg: &str, _scale: f64, _bg: &str) -> Result<Vec<u8>, String> {
            Ok(svg.as_bytes().to_vec())
        }
    }

    #[test]
    fn valid_diagram_report() {
        let raw = RawRender {
            ok: true,
            diagram_type: Some("flowchart-v2".into()),
            svg: Some(r#"<svg id="x" width="100" height="50" viewBox="0 0 100 50"><g/></svg>"#.into()),
            view_box: Some(BBox { x: 0.0, y: 0.0, w: 100.0, h: 50.0 }),
            model: DiagramModel {
                nodes: vec![
                    Node { id: "A".into(), label: "A".into(), bbox: Some(BBox { x: 0.0, y: 0.0, w: 10.0, h: 10.0 }), ..Default::default() },
                    Node { id: "B".into(), label: "B".into(), ..Default::default() },
                ],
                ..Default::default()
            },
            ..Default::default()
        };
        let a = analyze(&MermaidEngine, "flowchart LR\n A\n B", &FakeBackend(raw), &AnalyzeOptions::default());
        assert!(a.report.valid);
        assert_eq!(a.report.warning_count, 2); // two orphans
        let annotated = String::from_utf8(a.annotated_png.unwrap()).unwrap();
        assert!(annotated.contains("odg-annotations"));
        assert!(annotated.contains(r#"viewBox="-20.0 -20.0"#));
        assert!(annotated.contains("W_ORPHAN_NODE"));
    }

    #[test]
    fn invalid_diagram_gets_error_card() {
        let raw = RawRender {
            parse_error: Some(SourceError { kind: "parse".into(), message: "Parse error on line 2: bad".into(), line: Some(2), column: Some(3), ..Default::default() }),
            ..Default::default()
        };
        let a = analyze(&MermaidEngine, "flowchart LR\n A -- > B", &FakeBackend(raw), &AnalyzeOptions::default());
        assert!(!a.report.valid);
        let card = String::from_utf8(a.annotated_png.unwrap()).unwrap();
        assert!(card.contains("A -- &gt; B"));
        assert!(card.contains("#ffd7d5"));
        assert_eq!(excerpt("a\nb\nc", 2, 1), "    1 | a\n>   2 | b\n    3 | c");
    }
}
