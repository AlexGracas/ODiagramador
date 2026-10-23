//! Editing existing elements from the viewer's inspector: rename, change shape, delete.
//!
//! Works on the Mermaid text line by line, touching only what the edit needs, so the rest of the
//! author's formatting is preserved. Flowchart lines are parsed into chains
//! (`A[x] --> B & C -.-> D`) so deleting a node removes exactly its connections and keeps the
//! definitions of its neighbours.

use regex::Regex;
use serde_json::Value;
use std::sync::OnceLock;

use crate::construct::{flow_label, flow_shape, free_text, header_index, kind_of, type_name, Kind};

fn lines_of(src: &str) -> Vec<String> {
    src.lines().map(str::to_string).collect()
}

fn join(lines: Vec<String>) -> String {
    let mut s = lines.join("\n");
    s.push('\n');
    s
}

fn indent(line: &str) -> String {
    line.chars().take_while(|c| c.is_whitespace()).collect()
}

fn param<'a>(params: &'a Value, k: &str) -> Result<&'a str, String> {
    params.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).ok_or_else(|| format!("missing `{k}`"))
}

fn token_re(id: &str) -> Regex {
    Regex::new(&format!(r"(^|[^\w-]){}($|[^\w-])", regex::escape(id))).unwrap()
}

fn has_token(text: &str, id: &str) -> bool {
    token_re(id).is_match(text)
}

/// The part of a line before the first `:` that is outside quotes (labels live after it).
fn code_part(line: &str) -> &str {
    let mut quoted = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => quoted = !quoted,
            ':' if !quoted => return &line[..i],
            _ => {}
        }
    }
    line
}

/// Replaces `old` with `new` as a whole token in the code part of every line.
fn rename_token(src: &str, old: &str, new: &str) -> String {
    let re = token_re(old);
    join(
        src.lines()
            .map(|l| {
                let code = code_part(l);
                let rest = &l[code.len()..];
                let replaced = re.replace_all(code, |c: &regex::Captures| format!("{}{}{}", &c[1], new, &c[2]));
                // replace twice: adjacent matches share a separator
                let replaced = re.replace_all(&replaced, |c: &regex::Captures| format!("{}{}{}", &c[1], new, &c[2])).into_owned();
                replaced + rest
            })
            .collect(),
    )
}

fn drop_layout_lines(src: &str, id: &str) -> String {
    let s = crate::source::clear_positions(src, Some(id));
    crate::source::set_node_layer(&s, id, None).unwrap_or(s)
}

/// Removes `{`…`}` blocks that start on lines matching `start` (inclusive, brace-balanced).
fn remove_blocks(lines: Vec<String>, start: impl Fn(&str) -> bool) -> Vec<String> {
    let mut out = vec![];
    let mut depth = 0i32;
    for l in lines {
        if depth > 0 {
            depth += l.matches('{').count() as i32 - l.matches('}').count() as i32;
            continue;
        }
        if start(&l) && l.contains('{') {
            depth = l.matches('{').count() as i32 - l.matches('}').count() as i32;
            continue;
        }
        out.push(l);
    }
    out
}

// ====================================================================== flowchart chains

#[derive(Debug, Clone, PartialEq)]
struct NodeRef {
    id: String,
    open: String,
    text: String,
    close: String,
    suffix: String,
}

impl NodeRef {
    fn has_def(&self) -> bool {
        !self.open.is_empty()
    }

    fn render(&self) -> String {
        format!("{}{}{}{}{}", self.id, self.open, self.text, self.close, self.suffix)
    }
}

const SHAPES: &[(&str, &str, &str)] = &[
    ("(((", ")))", "doublecircle"),
    ("([", "])", "stadium"),
    ("[[", "]]", "subroutine"),
    ("[(", ")]", "database"),
    ("((", "))", "circle"),
    ("{{", "}}", "hexagon"),
    ("[/", "/]", "io"),
    ("[\\", "\\]", "io"),
    ("[/", "\\]", "trapezoid"),
    ("[\\", "/]", "trapezoid"),
    (">", "]", "flag"),
    ("[", "]", "rect"),
    ("(", ")", "round"),
    ("{", "}", "decision"),
];

fn parse_node(part: &str) -> NodeRef {
    let t = part.trim();
    let id_end = t
        .char_indices()
        .find(|(i, c)| "[({>".contains(*c) || c.is_whitespace() || t[*i..].starts_with(":::"))
        .map(|(i, _)| i)
        .unwrap_or(t.len());
    let id = t[..id_end].to_string();
    let mut rest = t[id_end..].trim_start().to_string();
    let mut suffix = String::new();
    if let Some(pos) = rest.rfind(":::") {
        let after_close = rest[..pos].ends_with([']', ')', '}']) || rest[..pos].is_empty();
        if after_close {
            suffix = rest[pos..].to_string();
            rest.truncate(pos);
        }
    }
    for (open, close, _) in SHAPES {
        if rest.len() >= open.len() + close.len() && rest.starts_with(open) && rest.ends_with(close) {
            return NodeRef {
                id,
                open: open.to_string(),
                text: rest[open.len()..rest.len() - close.len()].to_string(),
                close: close.to_string(),
                suffix,
            };
        }
    }
    NodeRef { id, open: String::new(), text: String::new(), close: String::new(), suffix }
}

/// A byte-length-preserving copy where quoted / bracketed / piped text is blanked out, so link
/// operators inside labels (`A["a --> b"]`) are not mistaken for real ones.
fn mask(line: &str) -> String {
    let mut out = line.as_bytes().to_vec();
    let (mut depth, mut quoted, mut piped) = (0i32, false, false);
    for (i, c) in line.char_indices() {
        let inside = depth > 0 || quoted || piped;
        match c {
            '"' => quoted = !quoted,
            '[' | '(' | '{' if !quoted && !piped => depth += 1,
            ']' | ')' | '}' if !quoted && !piped && depth > 0 => depth -= 1,
            '|' if !quoted && depth == 0 => piped = !piped,
            _ => {
                if inside {
                    for b in &mut out[i..i + c.len_utf8()] {
                        *b = b'_';
                    }
                }
            }
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| line.to_string())
}

fn link_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?x)
            (
                <?--\s[^-|]*?\s-{2,}[>ox]?
              | <?==\s[^=|]*?\s={2,}[>ox]?
              | <?-\.\s[^|]*?\s\.-[>ox]?
              | <?-\.+-[>ox]? | -\.+-
              | [ox<]?={2,}[>ox] | ={3,}
              | [ox<]?-{2,}[>ox] | -{3,}
              | ~~~
            )
            (\|[^|]*\|)?",
        )
        .unwrap()
    })
}

#[derive(Debug)]
struct Chain {
    indent: String,
    /// (start, end) byte ranges of the trimmed parts in the original line
    parts: Vec<(usize, usize)>,
    links: Vec<(usize, usize)>,
}

const FLOW_KEYWORDS: &[&str] = &["subgraph", "end", "classDef", "class", "style", "linkStyle", "click", "direction", "flowchart", "graph"];

fn trimmed_range(line: &str, a: usize, b: usize) -> (usize, usize) {
    let s = &line[a..b];
    let start = a + (s.len() - s.trim_start().len());
    let end = b - (s.len() - s.trim_end().len());
    (start, end.max(start))
}

fn parse_chain(line: &str) -> Option<Chain> {
    let t = line.trim();
    if t.is_empty() || t.starts_with("%%") || t == "end" || t == "---" {
        return None;
    }
    let first = t.split_whitespace().next().unwrap_or("");
    if FLOW_KEYWORDS.contains(&first) {
        return None;
    }
    let masked = mask(line);
    let mut parts = vec![];
    let mut links = vec![];
    let mut last = 0;
    for m in link_re().find_iter(&masked) {
        parts.push(trimmed_range(line, last, m.start()));
        links.push(trimmed_range(line, m.start(), m.end()));
        last = m.end();
    }
    parts.push(trimmed_range(line, last, line.len()));
    if parts.iter().any(|(a, b)| a == b) {
        return None; // dangling operator: not a chain we understand
    }
    Some(Chain { indent: indent(line), parts, links })
}

fn split_group(part: &str) -> Vec<NodeRef> {
    let masked = mask(part);
    let mut out = vec![];
    let mut last = 0;
    for (i, c) in masked.char_indices() {
        if c == '&' {
            out.push(parse_node(&part[last..i]));
            last = i + 1;
        }
    }
    out.push(parse_node(&part[last..]));
    out
}

fn render_group(nodes: &[NodeRef]) -> String {
    nodes.iter().map(NodeRef::render).collect::<Vec<_>>().join(" & ")
}

/// Rebuilds a chain line without node `deleted` and/or without the chain edge `drop_edge`.
fn rebuild_chain(line: &str, chain: &Chain, deleted: Option<&str>, drop_edge: Option<usize>) -> Vec<String> {
    let parts: Vec<Option<String>> = chain
        .parts
        .iter()
        .map(|&(a, b)| {
            let text = &line[a..b];
            match deleted {
                Some(d) => {
                    let nodes = split_group(text);
                    let kept: Vec<NodeRef> = nodes.iter().filter(|n| n.id != d).cloned().collect();
                    if kept.is_empty() {
                        None
                    } else if kept.len() == nodes.len() {
                        Some(text.to_string())
                    } else {
                        Some(render_group(&kept))
                    }
                }
                None => Some(text.to_string()),
            }
        })
        .collect();
    let links: Vec<&str> = chain.links.iter().map(|&(a, b)| &line[a..b]).collect();
    let alive: Vec<bool> = (0..links.len()).map(|i| parts[i].is_some() && parts[i + 1].is_some() && drop_edge != Some(i)).collect();
    let mut out = vec![];
    let mut covered = vec![false; parts.len()];
    let mut i = 0;
    while i < links.len() {
        if !alive[i] {
            i += 1;
            continue;
        }
        let mut s = parts[i].clone().unwrap();
        covered[i] = true;
        while i < links.len() && alive[i] {
            s.push_str(&format!(" {} {}", links[i], parts[i + 1].as_ref().unwrap()));
            covered[i + 1] = true;
            i += 1;
        }
        out.push(format!("{}{s}", chain.indent));
    }
    // keep definitions that would otherwise disappear with their connections
    for (k, p) in parts.iter().enumerate() {
        if let (false, Some(p)) = (covered[k], p) {
            if links.is_empty() || split_group(p).iter().any(NodeRef::has_def) {
                out.push(format!("{}{p}", chain.indent));
            }
        }
    }
    out
}

struct EdgeHit {
    line: usize,
    link: usize,
    grouped: bool,
}

/// The `index`-th edge from `from` to `to` (Mermaid ids are `L_<from>_<to>_<index>`).
fn find_edge(lines: &[String], from: &str, to: &str, index: usize) -> Option<EdgeHit> {
    let mut seen = 0;
    for (li, line) in lines.iter().enumerate() {
        let Some(chain) = parse_chain(line) else { continue };
        for k in 0..chain.links.len() {
            let a = split_group(&line[chain.parts[k].0..chain.parts[k].1]);
            let b = split_group(&line[chain.parts[k + 1].0..chain.parts[k + 1].1]);
            if a.iter().any(|n| n.id == from) && b.iter().any(|n| n.id == to) {
                if seen == index {
                    return Some(EdgeHit { line: li, link: k, grouped: a.len() > 1 || b.len() > 1 });
                }
                seen += 1;
            }
        }
    }
    None
}

fn base_operator(link: &str) -> String {
    let no_label = link.split('|').next().unwrap_or(link).trim();
    let compact: String = no_label.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.contains(' ') {
        // text form: "-- text -->" → the closing operator
        let closing = compact.rsplit(' ').next().unwrap_or("-->");
        return match closing.chars().next() {
            Some('=') => "==>".into(),
            Some('.') => "-.->".into(),
            _ if closing.ends_with('>') => "-->".into(),
            _ => "---".into(),
        };
    }
    compact
}

fn flow_rename(src: &str, id: &str, label: &str, shape: Option<&str>) -> Result<String, String> {
    let mut lines = lines_of(src);
    // subgraph title
    for l in lines.iter_mut() {
        let t = l.trim_start();
        if let Some(rest) = t.strip_prefix("subgraph ") {
            let sid = rest.split(|c: char| c == '[' || c.is_whitespace()).next().unwrap_or("");
            if sid == id {
                *l = format!("{}subgraph {id} [{}]", indent(l), flow_label(label));
                return Ok(join(lines));
            }
        }
    }
    // first definition, else first occurrence
    let mut fallback: Option<(usize, usize, usize, usize)> = None; // line, part range, node index
    for li in 0..lines.len() {
        let Some(chain) = parse_chain(&lines[li]) else { continue };
        for &(a, b) in &chain.parts {
            let mut nodes = split_group(&lines[li][a..b]);
            if let Some(k) = nodes.iter().position(|n| n.id == id) {
                if nodes[k].has_def() {
                    let new_shape = shape.map(|s| flow_shape(s, label)).unwrap_or_else(|| format!("{}{}{}", nodes[k].open, flow_label(label), nodes[k].close));
                    let parsed = parse_node(&format!("{id}{new_shape}"));
                    nodes[k].open = parsed.open;
                    nodes[k].text = parsed.text;
                    nodes[k].close = parsed.close;
                    let new_part = render_group(&nodes);
                    lines[li].replace_range(a..b, &new_part);
                    return Ok(join(lines));
                }
                if fallback.is_none() {
                    fallback = Some((li, a, b, k));
                }
            }
        }
    }
    let (li, a, b, k) = fallback.ok_or_else(|| format!("node `{id}` not found"))?;
    let mut nodes = split_group(&lines[li][a..b]);
    let parsed = parse_node(&format!("{id}{}", flow_shape(shape.unwrap_or("rect"), label)));
    nodes[k].open = parsed.open;
    nodes[k].text = parsed.text;
    nodes[k].close = parsed.close;
    lines[li].replace_range(a..b, &render_group(&nodes));
    Ok(join(lines))
}

fn current_label(src: &str, id: &str) -> String {
    for line in src.lines() {
        let Some(chain) = parse_chain(line) else { continue };
        for &(a, b) in &chain.parts {
            if let Some(n) = split_group(&line[a..b]).into_iter().find(|n| n.id == id && n.has_def()) {
                return n.text.trim_matches('"').replace("#quot;", "\"");
            }
        }
    }
    id.to_string()
}

fn flow_delete(src: &str, id: &str) -> Result<String, String> {
    let lines = lines_of(src);
    let mut out = vec![];
    // subgraph: drop the header and its matching end, keep the members
    let mut skip_end_at_depth: Option<i32> = None;
    let mut depth = 0i32;
    let mut found = false;
    for line in &lines {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("subgraph ") {
            depth += 1;
            let sid = rest.split(|c: char| c == '[' || c.is_whitespace()).next().unwrap_or("");
            if sid == id {
                skip_end_at_depth = Some(depth);
                found = true;
                continue;
            }
            out.push(line.clone());
            continue;
        }
        if t == "end" {
            let close = skip_end_at_depth == Some(depth);
            depth -= 1;
            if close {
                skip_end_at_depth = None;
                continue;
            }
            out.push(line.clone());
            continue;
        }
        let first = t.split_whitespace().next().unwrap_or("");
        if (first == "style" || first == "click") && t.split_whitespace().nth(1) == Some(id) {
            found = true;
            continue;
        }
        if first == "class" {
            let mut words = t.split_whitespace();
            words.next();
            if let (Some(list), Some(cls)) = (words.next(), words.next()) {
                let ids: Vec<&str> = list.split(',').collect();
                if ids.contains(&id) {
                    found = true;
                    let kept: Vec<&str> = ids.into_iter().filter(|x| *x != id).collect();
                    if !kept.is_empty() {
                        out.push(format!("{}class {} {cls}", indent(line), kept.join(",")));
                    }
                    continue;
                }
            }
        }
        match parse_chain(line) {
            Some(chain) => {
                let touches = chain.parts.iter().any(|&(a, b)| split_group(&line[a..b]).iter().any(|n| n.id == id));
                if touches {
                    found = true;
                    out.extend(rebuild_chain(line, &chain, Some(id), None));
                } else {
                    out.push(line.clone());
                }
            }
            None => out.push(line.clone()),
        }
    }
    if !found {
        return Err(format!("node `{id}` not found"));
    }
    Ok(drop_layout_lines(&join(out), id))
}

fn flow_edge(src: &str, from: &str, to: &str, index: usize, new_label: Option<Option<&str>>) -> Result<String, String> {
    let mut lines = lines_of(src);
    let hit = find_edge(&lines, from, to, index).ok_or_else(|| format!("edge {from} → {to} not found"))?;
    let line = lines[hit.line].clone();
    let chain = parse_chain(&line).unwrap();
    match new_label {
        Some(label) => {
            let (a, b) = chain.links[hit.link];
            let op = base_operator(&line[a..b]);
            let link = match label {
                Some(l) if !l.trim().is_empty() => format!("{op}|{}|", flow_label(l)),
                _ => op,
            };
            lines[hit.line].replace_range(a..b, &link);
            Ok(join(lines))
        }
        None => {
            if hit.grouped {
                return Err("this connection is part of an `A & B` group; edit it in the code".into());
            }
            let rebuilt = rebuild_chain(&line, &chain, None, Some(hit.link));
            lines.splice(hit.line..=hit.line, rebuilt);
            Ok(join(lines))
        }
    }
}

// ====================================================================== sequence

fn participant_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*(participant|actor)\s+(\S+?)(?:\s+as\s+(.+?))?\s*$").unwrap())
}

fn message_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*([^\s:]+?)\s*(<<-->>|<<->>|-->>|->>|-->|->|--x|-x|--\)|-\))\s*[+-]?\s*([^\s:]+?)\s*:").unwrap())
}

fn seq_rename(src: &str, id: &str, label: &str) -> Result<String, String> {
    let mut lines = lines_of(src);
    let alias = free_text(label);
    let decl = |kw: &str, ind: &str| if alias == id { format!("{ind}{kw} {id}") } else { format!("{ind}{kw} {id} as {alias}") };
    for l in lines.iter_mut() {
        if let Some(c) = participant_re().captures(l) {
            if &c[2] == id {
                let kw = c[1].to_string();
                *l = decl(&kw, &indent(l));
                return Ok(join(lines));
            }
        }
    }
    let header = header_index(&src.lines().collect::<Vec<_>>()).ok_or("no header")?;
    let first = lines.iter().enumerate().skip(header + 1).find(|(_, l)| has_token(code_part(l), id)).map(|(i, _)| i).ok_or_else(|| format!("participant `{id}` not found"))?;
    let ind = indent(&lines[first]);
    let ind = if ind.is_empty() { "    ".to_string() } else { ind };
    lines.insert(first, decl("participant", &ind));
    Ok(join(lines))
}

fn seq_delete(src: &str, id: &str) -> Result<String, String> {
    let note = Regex::new(r"^(\s*)(?i:note)\s+(right of|left of|over)\s+([^:]+?)\s*:(.*)$").unwrap();
    let mut out = vec![];
    let mut found = false;
    for l in lines_of(src) {
        let t = l.trim();
        if participant_re().captures(&l).is_some_and(|c| &c[2] == id) {
            found = true;
            continue;
        }
        if let Some(c) = message_re().captures(&l) {
            if &c[1] == id || &c[3] == id {
                found = true;
                continue;
            }
        }
        if let Some(c) = note.captures(&l) {
            let who: Vec<&str> = c[3].split(',').map(str::trim).collect();
            if who.contains(&id) {
                found = true;
                let kept: Vec<&str> = who.into_iter().filter(|w| *w != id).collect();
                if !kept.is_empty() && &c[2] == "over" {
                    out.push(format!("{}Note over {}:{}", &c[1], kept.join(","), &c[4]));
                }
                continue;
            }
        }
        let words: Vec<&str> = t.split_whitespace().collect();
        if matches!(words.first(), Some(&"activate") | Some(&"deactivate") | Some(&"destroy")) && words.get(1) == Some(&id) {
            continue;
        }
        if words.first() == Some(&"create") && words.get(2) == Some(&id) {
            continue;
        }
        out.push(l);
    }
    if !found {
        return Err(format!("participant `{id}` not found"));
    }
    Ok(join(out))
}

// ====================================================================== class / ER / state

fn ident_rename(src: &str, id: &str, name: &str, kind: Kind) -> Result<String, String> {
    let new = type_name("", name, kind == Kind::Er, if kind == Kind::Er { "ENTITY" } else { "Class" });
    if new == id {
        return Ok(src.to_string());
    }
    if crate::source::find_line(src, &new).is_some() {
        return Err(format!("`{new}` already exists"));
    }
    if crate::source::find_line(src, id).is_none() {
        return Err(format!("`{id}` not found"));
    }
    Ok(rename_token(src, id, &new))
}

/// Deletes every line whose code part mentions `id` (relations, members, notes) and its block.
fn ident_delete(src: &str, id: &str, kind: Kind) -> Result<String, String> {
    if crate::source::find_line(src, id).is_none() {
        return Err(format!("`{id}` not found"));
    }
    let header = header_index(&src.lines().collect::<Vec<_>>()).unwrap_or(0);
    let lines = lines_of(src);
    let block_start = |l: &str| {
        let c = code_part(l);
        let t = c.trim();
        match kind {
            Kind::Class => (t.starts_with("class ") || t.starts_with("namespace ")) && has_token(c, id),
            Kind::Er => t.split(|ch: char| ch == '{' || ch == '[' || ch.is_whitespace()).next() == Some(id),
            _ => t.starts_with("state ") && has_token(c, id),
        }
    };
    let lines = remove_blocks(lines, block_start);
    let note_for = Regex::new(&format!(r"^\s*note\s+(for|right of|left of)\s+{}\b", regex::escape(id))).unwrap();
    let out: Vec<String> = lines
        .into_iter()
        .enumerate()
        .filter(|(i, l)| *i <= header || !(has_token(code_part(l), id) || note_for.is_match(l)))
        .map(|(_, l)| l)
        .collect();
    Ok(drop_layout_lines(&join(out), id))
}

fn state_rename(src: &str, id: &str, label: &str) -> Result<String, String> {
    let mut lines = lines_of(src);
    let text = free_text(label).replace('"', "'");
    let alias = Regex::new(&format!(r#"^(\s*)state\s+"[^"]*"\s+as\s+{}\b(.*)$"#, regex::escape(id))).unwrap();
    let desc = Regex::new(&format!(r"^(\s*){}\s*:\s*.*$", regex::escape(id))).unwrap();
    for l in lines.iter_mut() {
        if let Some(c) = alias.captures(l) {
            *l = format!("{}state \"{text}\" as {id}{}", &c[1], &c[2]);
            return Ok(join(lines));
        }
    }
    for l in lines.iter_mut() {
        if let Some(c) = desc.captures(l) {
            *l = format!("{}{id} : {text}", &c[1]);
            return Ok(join(lines));
        }
    }
    if crate::source::find_line(src, id).is_none() {
        return Err(format!("state `{id}` not found"));
    }
    let header = header_index(&src.lines().collect::<Vec<_>>()).ok_or("no header")?;
    let mut at = header + 1;
    if lines.get(at).is_some_and(|l| l.trim().starts_with("direction ")) {
        at += 1;
    }
    let ind = lines.get(at).map(|l| indent(l)).filter(|s| !s.is_empty()).unwrap_or_else(|| "    ".into());
    lines.insert(at, format!("{ind}state \"{text}\" as {id}"));
    Ok(join(lines))
}

// ====================================================================== entry point

/// Inspector edits. `action`: `rename {id, label}` · `shape {id, shape, label?}` (flowchart) ·
/// `delete {id}` · `edge_label {from, to, index, label}` · `edge_delete {from, to, index}` (flowchart).
pub fn apply(src: &str, action: &str, params: &Value) -> Result<String, String> {
    let kind = kind_of(src).ok_or("editing is available for flowchart, sequence, class, state and ER diagrams")?;
    let index = params.get("index").and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))).unwrap_or(0) as usize;
    match (action, kind) {
        ("rename", Kind::Flowchart) => flow_rename(src, param(params, "id")?, param(params, "label")?, None),
        ("rename", Kind::Sequence) => seq_rename(src, param(params, "id")?, param(params, "label")?),
        ("rename", Kind::State) => state_rename(src, param(params, "id")?, param(params, "label")?),
        ("rename", k) => ident_rename(src, param(params, "id")?, param(params, "label")?, k),
        ("shape", Kind::Flowchart) => {
            let id = param(params, "id")?;
            let label = params.get("label").and_then(Value::as_str).map(str::to_string).filter(|s| !s.trim().is_empty()).unwrap_or_else(|| current_label(src, id));
            flow_rename(src, id, &label, Some(param(params, "shape")?))
        }
        ("delete", Kind::Flowchart) => flow_delete(src, param(params, "id")?),
        ("delete", Kind::Sequence) => seq_delete(src, param(params, "id")?),
        ("delete", k) => ident_delete(src, param(params, "id")?, k),
        ("edge_label", Kind::Flowchart) => flow_edge(src, param(params, "from")?, param(params, "to")?, index, Some(params.get("label").and_then(Value::as_str))),
        ("edge_delete", Kind::Flowchart) => flow_edge(src, param(params, "from")?, param(params, "to")?, index, None),
        (a, k) => Err(format!("`{a}` is not available for {} diagrams", k.id())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const FLOW: &str = "flowchart LR\n    a([Start]) --> b{Valid?}\n    b -->|yes| c[Save] --> d[(DB)]\n    b -- no --> e[Retry]\n    e -.-> a\n    subgraph g [Group]\n        c\n    end\n    style c fill:#f9f\n    class c,e hot\n%% @odg pos c 10,20\n";

    #[test]
    fn rename_and_reshape_flowchart() {
        let s = apply(FLOW, "rename", &json!({"id": "c", "label": "Gravar (R$)"})).unwrap();
        assert!(s.contains("b -->|yes| c[\"Gravar (R$)\"] --> d[(DB)]"));
        let s = apply(FLOW, "shape", &json!({"id": "b", "shape": "hexagon"})).unwrap();
        assert!(s.contains("a([Start]) --> b{{Valid?}}"));
        // node without definition gets one at its first occurrence
        let s = apply("flowchart TD\n  x --> y\n", "rename", &json!({"id": "y", "label": "Why"})).unwrap();
        assert_eq!(s, "flowchart TD\n  x --> y[Why]\n");
        let s = apply(FLOW, "rename", &json!({"id": "g", "label": "Financeiro"})).unwrap();
        assert!(s.contains("    subgraph g [Financeiro]\n"));
    }

    #[test]
    fn delete_flowchart_node_keeps_neighbours() {
        let s = apply(FLOW, "delete", &json!({"id": "c"})).unwrap();
        assert!(!s.contains("c[Save]"));
        assert!(s.contains("    d[(DB)]\n"), "{s}");
        assert!(!s.contains("style c"));
        assert!(s.contains("    class e hot\n"));
        assert!(!s.contains("@odg pos c"));
        assert!(s.contains("subgraph g [Group]\n    end"), "{s}");
        let s = apply(FLOW, "delete", &json!({"id": "g"})).unwrap();
        assert!(!s.contains("subgraph") && s.contains("        c\n") && !s.contains("    end\n"));
        assert!(apply(FLOW, "delete", &json!({"id": "zz"})).is_err());
    }

    #[test]
    fn edit_flowchart_edges() {
        let s = apply(FLOW, "edge_label", &json!({"from": "b", "to": "e", "index": 0, "label": "não"})).unwrap();
        assert!(s.contains("    b -->|não| e[Retry]\n"), "{s}");
        let s = apply(FLOW, "edge_label", &json!({"from": "b", "to": "c", "label": ""})).unwrap();
        assert!(s.contains("    b --> c[Save] --> d[(DB)]\n"));
        let s = apply(FLOW, "edge_delete", &json!({"from": "c", "to": "d"})).unwrap();
        assert!(s.contains("    b -->|yes| c[Save]\n    d[(DB)]\n"), "{s}");
        let s = apply("flowchart LR\n A --> B\n A --> B\n", "edge_delete", &json!({"from": "A", "to": "B", "index": 1})).unwrap();
        assert_eq!(s, "flowchart LR\n A --> B\n");
        assert!(apply("flowchart LR\n A & C --> B\n", "edge_delete", &json!({"from": "A", "to": "B"})).is_err());
    }

    #[test]
    fn labels_with_operators_are_not_split() {
        let s = apply("flowchart LR\n  a[\"x --> y\"] --> b\n", "delete", &json!({"id": "b"})).unwrap();
        assert_eq!(s, "flowchart LR\n  a[\"x --> y\"]\n");
    }

    #[test]
    fn sequence_edits() {
        let src = "sequenceDiagram\n    participant A as Alice\n    A->>B: hi\n    B-->>A: hello\n    Note over A,B: talk\n    activate B\n";
        let s = apply(src, "rename", &json!({"id": "A", "label": "Ana"})).unwrap();
        assert!(s.contains("    participant A as Ana\n"));
        let s = apply(src, "rename", &json!({"id": "B", "label": "Bob"})).unwrap();
        assert!(s.contains("    participant B as Bob\n    A->>B: hi"), "{s}");
        let s = apply(src, "delete", &json!({"id": "B"})).unwrap();
        assert_eq!(s, "sequenceDiagram\n    participant A as Alice\n    Note over A: talk\n");
    }

    #[test]
    fn class_er_state_edits() {
        let cls = "classDiagram\n    class Animal {\n        +name\n    }\n    class Duck\n    Animal <|-- Duck : is a\n    Duck : +swim()\n    note for Duck \"quack\"\n";
        let s = apply(cls, "rename", &json!({"id": "Duck", "label": "Pato"})).unwrap();
        assert!(s.contains("class Pato\n    Animal <|-- Pato : is a\n    Pato : +swim()\n    note for Pato"), "{s}");
        let s = apply(cls, "delete", &json!({"id": "Animal"})).unwrap();
        assert_eq!(s, "classDiagram\n    class Duck\n    Duck : +swim()\n    note for Duck \"quack\"\n");
        let er = "erDiagram\n    CLIENTE ||--o{ PEDIDO : faz\n    CLIENTE {\n        int id PK\n    }\n";
        let s = apply(er, "delete", &json!({"id": "CLIENTE"})).unwrap();
        assert_eq!(s, "erDiagram\n");
        let s = apply(er, "rename", &json!({"id": "PEDIDO", "label": "ordem de compra"})).unwrap();
        assert!(s.contains("CLIENTE ||--o{ ORDEM_DE_COMPRA : faz"));
        let st = "stateDiagram-v2\n    [*] --> Idle\n    Idle --> Run : go\n    Run --> [*]\n";
        let s = apply(st, "rename", &json!({"id": "Run", "label": "Executando"})).unwrap();
        assert!(s.starts_with("stateDiagram-v2\n    state \"Executando\" as Run\n"));
        let s = apply(st, "delete", &json!({"id": "Run"})).unwrap();
        assert_eq!(s, "stateDiagram-v2\n    [*] --> Idle\n");
        assert!(apply(st, "shape", &json!({"id": "Run", "shape": "rect"})).is_err());
    }
}
