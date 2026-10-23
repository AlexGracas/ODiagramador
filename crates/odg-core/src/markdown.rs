//! Markdown (release notes, agent messages) turned into simple blocks the native apps draw with their
//! own widgets — SwiftUI text, a WPF FlowDocument — or into Pango markup for GTK labels.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Span {
    pub text: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bold: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub italic: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub code: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Block {
    /// heading | paragraph | bullet | code | quote | rule
    pub kind: String,
    /// Heading level (1–6) or list depth (1 = top level).
    pub level: u8,
    /// Number of an ordered-list item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<u64>,
    pub spans: Vec<Span>,
}

impl Block {
    fn new(kind: &str, level: u8) -> Block {
        Block { kind: kind.into(), level, ..Default::default() }
    }

    pub fn plain_text(&self) -> String {
        self.spans.iter().map(|s| s.text.as_str()).collect()
    }
}

#[derive(Default)]
struct Style {
    bold: u32,
    italic: u32,
    link: Option<String>,
}

pub fn to_blocks(md: &str) -> Vec<Block> {
    let mut out: Vec<Block> = vec![];
    let mut style = Style::default();
    // ordered-list counters (None = bullet list), one per nesting level
    let mut lists: Vec<Option<u64>> = vec![];
    let mut in_item = false;
    let mut quote = 0u32;
    let mut cell = 0usize;
    let push_text = |out: &mut Vec<Block>, style: &Style, text: &str, code: bool| {
        if out.is_empty() {
            out.push(Block::new("paragraph", 0));
        }
        let b = out.last_mut().unwrap();
        b.spans.push(Span { text: text.to_string(), bold: style.bold > 0, italic: style.italic > 0, code, link: style.link.clone() });
    };
    for event in Parser::new_ext(md, Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => out.push(Block::new("heading", level as u8)),
            Event::Start(Tag::Paragraph) => {
                // the first paragraph of a list item continues the bullet itself
                let continues_item = in_item && out.last().is_some_and(|b| b.kind == "bullet" && b.spans.is_empty());
                if !continues_item {
                    out.push(Block::new(if quote > 0 { "quote" } else { "paragraph" }, lists.len() as u8));
                }
            }
            Event::Start(Tag::List(start)) => lists.push(start),
            Event::End(TagEnd::List(_)) => {
                lists.pop();
            }
            Event::Start(Tag::Item) => {
                in_item = true;
                let mut b = Block::new("bullet", lists.len().max(1) as u8);
                if let Some(Some(n)) = lists.last_mut() {
                    b.number = Some(*n);
                    *n += 1;
                }
                out.push(b);
            }
            // still inside the outer item when a nested list's item ends
            Event::End(TagEnd::Item) => in_item = lists.len() > 1,
            Event::Start(Tag::CodeBlock(_)) => out.push(Block::new("code", 0)),
            Event::Start(Tag::BlockQuote(_)) => quote += 1,
            Event::End(TagEnd::BlockQuote(_)) => quote = quote.saturating_sub(1),
            Event::Start(Tag::Strong) => style.bold += 1,
            Event::End(TagEnd::Strong) => style.bold = style.bold.saturating_sub(1),
            Event::Start(Tag::Emphasis) => style.italic += 1,
            Event::End(TagEnd::Emphasis) => style.italic = style.italic.saturating_sub(1),
            Event::Start(Tag::Link { dest_url, .. }) => style.link = Some(dest_url.to_string()),
            Event::End(TagEnd::Link) => style.link = None,
            // tables become one line per row: "cell — cell"
            Event::Start(Tag::TableHead) | Event::Start(Tag::TableRow) => {
                out.push(Block::new("paragraph", 0));
                cell = 0;
            }
            Event::Start(Tag::TableCell) => {
                if cell > 0 {
                    push_text(&mut out, &style, " — ", false);
                }
                cell += 1;
            }
            Event::Text(t) => {
                let code = out.last().is_some_and(|b| b.kind == "code");
                let text = if code { t.trim_end_matches('\n').to_string() } else { t.to_string() };
                push_text(&mut out, &style, &text, code);
            }
            Event::Code(t) => push_text(&mut out, &style, &t, true),
            Event::SoftBreak => push_text(&mut out, &style, " ", false),
            Event::HardBreak => push_text(&mut out, &style, "\n", false),
            Event::Rule => out.push(Block::new("rule", 0)),
            _ => {}
        }
    }
    // drop empty paragraphs (e.g. from table heads without text)
    out.retain(|b| b.kind == "rule" || b.spans.iter().any(|s| !s.text.trim().is_empty()));
    out
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Pango markup (GTK labels / dialogs): headings in bold, bullets with "•", code in monospace.
pub fn to_pango(md: &str) -> String {
    let mut lines = vec![];
    for b in to_blocks(md) {
        let inline: String = b
            .spans
            .iter()
            .map(|s| {
                let mut t = escape(&s.text);
                if s.code {
                    t = format!("<tt>{t}</tt>");
                }
                if s.bold {
                    t = format!("<b>{t}</b>");
                }
                if s.italic {
                    t = format!("<i>{t}</i>");
                }
                if let Some(href) = &s.link {
                    t = format!("<a href=\"{}\">{t}</a>", escape(href).replace('"', "&quot;"));
                }
                t
            })
            .collect();
        lines.push(match b.kind.as_str() {
            "heading" => format!("\n<b><big>{inline}</big></b>"),
            "bullet" => {
                let indent = "    ".repeat(b.level.saturating_sub(1) as usize);
                let mark = b.number.map(|n| format!("{n}.")).unwrap_or_else(|| "•".into());
                format!("{indent}{mark} {inline}")
            }
            "code" => format!("<tt>{inline}</tt>"),
            "quote" => format!("<i>{inline}</i>"),
            "rule" => "──────────".into(),
            _ => inline,
        });
    }
    lines.join("\n").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_from_release_notes() {
        let md = "## Novidades da 0.3.0\n- **O agente** conversa com você:\n  - `show_diagram` abre\n- Botões\n\n| A | B |\n|---|---|\n| x | y |\n\nVeja [o site](https://e.x).";
        let b = to_blocks(md);
        assert_eq!(b[0].kind, "heading");
        assert_eq!(b[0].level, 2);
        assert_eq!(b[1].kind, "bullet");
        assert!(b[1].spans[0].bold && b[1].spans[0].text == "O agente");
        assert_eq!(b[2].kind, "bullet");
        assert_eq!(b[2].level, 2);
        assert!(b[2].spans[0].code);
        assert_eq!(b[3].plain_text(), "Botões");
        assert_eq!(b[4].plain_text(), "A — B");
        assert_eq!(b[5].plain_text(), "x — y");
        assert_eq!(b.last().unwrap().spans[1].link.as_deref(), Some("https://e.x"));
        let pango = to_pango(md);
        assert!(pango.contains("<b><big>Novidades da 0.3.0</big></b>"));
        assert!(pango.contains("    • <tt>show_diagram</tt> abre"));
        assert!(pango.contains("<a href=\"https://e.x\">o site</a>"));
    }

    #[test]
    fn ordered_lists_are_numbered() {
        let b = to_blocks("1. one\n2. two");
        assert_eq!((b[0].number, b[1].number), (Some(1), Some(2)));
    }
}
