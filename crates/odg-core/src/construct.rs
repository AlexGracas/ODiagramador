//! Constructors: the palette buttons of the viewer ("+ Node", "+ Message", "Direction: LR"…)
//! turned into Mermaid text. The viewer only collects parameters; every app (and the MCP server)
//! shares this code, so the generated syntax is identical everywhere.
//!
//! `apply(source, construct, params)` returns the new source. New lines are inserted at the end
//! of the diagram body, before the `%% @odg` layout comments.

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Flowchart,
    Sequence,
    Class,
    State,
    Er,
}

impl Kind {
    pub fn id(self) -> &'static str {
        match self {
            Kind::Flowchart => "flowchart",
            Kind::Sequence => "sequence",
            Kind::Class => "class",
            Kind::State => "state",
            Kind::Er => "er",
        }
    }
}

/// Index of the header line (`flowchart LR`, `sequenceDiagram`…), skipping front matter,
/// blank lines, comments and `%%{init}%%` directives.
pub(crate) fn header_index(lines: &[&str]) -> Option<usize> {
    let mut i = 0;
    if lines.first().map(|l| l.trim()) == Some("---") {
        i = lines.iter().skip(1).position(|l| l.trim() == "---")? + 2;
    }
    lines.iter().enumerate().skip(i).find(|(_, l)| {
        let t = l.trim();
        !t.is_empty() && !t.starts_with("%%")
    }).map(|(n, _)| n)
}

pub fn kind_of(src: &str) -> Option<Kind> {
    let lines: Vec<&str> = src.lines().collect();
    let h = lines[header_index(&lines)?].trim();
    let word = h.split_whitespace().next().unwrap_or("");
    Some(match word {
        "flowchart" | "graph" => Kind::Flowchart,
        "sequenceDiagram" => Kind::Sequence,
        "classDiagram" | "classDiagram-v2" => Kind::Class,
        "stateDiagram" | "stateDiagram-v2" => Kind::State,
        "erDiagram" => Kind::Er,
        _ => return None,
    })
}

pub const DIRECTIONS: &[&str] = &["TB", "TD", "BT", "LR", "RL"];

/// Current direction (flowchart header or `direction X` line), if any.
pub fn direction_of(src: &str) -> Option<String> {
    let lines: Vec<&str> = src.lines().collect();
    let h = header_index(&lines)?;
    if matches!(kind_of(src), Some(Kind::Flowchart)) {
        return lines[h].split_whitespace().nth(1).map(str::to_string);
    }
    lines.iter().skip(h + 1).find_map(|l| l.trim().strip_prefix("direction ").map(|d| d.trim().to_string()))
}

fn indent_of(lines: &[&str], header: usize) -> String {
    lines
        .iter()
        .skip(header + 1)
        .find(|l| !l.trim().is_empty() && !l.trim_start().starts_with("%%"))
        .map(|l| l.chars().take_while(|c| c.is_whitespace()).collect::<String>())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "    ".into())
}

/// Inserts `new_lines` at the end of the body (before trailing `%% @odg` comments / blanks).
fn insert_body(src: &str, new_lines: &[String]) -> String {
    let mut lines: Vec<String> = src.lines().map(str::to_string).collect();
    let mut at = lines.len();
    while at > 0 {
        let t = lines[at - 1].trim();
        if t.is_empty() || t.starts_with("%% @odg") || t.starts_with("%%@odg") {
            at -= 1;
        } else {
            break;
        }
    }
    for (k, l) in new_lines.iter().enumerate() {
        lines.insert(at + k, l.clone());
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

fn ascii(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            'ñ' | 'Ñ' => 'n',
            c => c,
        })
        .collect()
}

const RESERVED: &[&str] = &["end", "graph", "subgraph", "flowchart", "style", "class", "classDef", "click", "default", "linkStyle", "direction", "state", "note", "as"];

/// Identifier derived from a label (`"Aprovação do gestor"` → `aprovacao_do_gestor`), unique in `src`.
pub fn unique_id(src: &str, label: &str, fallback: &str) -> String {
    let words: Vec<String> = ascii(label)
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .take(3)
        .map(|w| w.to_lowercase())
        .collect();
    let mut base = if words.is_empty() { fallback.to_string() } else { words.join("_") };
    if base.chars().next().is_some_and(|c| c.is_ascii_digit()) || RESERVED.contains(&base.as_str()) {
        base = format!("{fallback}_{base}");
    }
    let mut id = base.clone();
    let mut n = 2;
    while crate::source::find_line(src, &id).is_some() {
        id = format!("{base}{n}");
        n += 1;
    }
    id
}

/// Class / entity style names: `order item` → `OrderItem` (class) or `ORDER_ITEM` (entity).
pub(crate) fn type_name(src: &str, label: &str, upper: bool, fallback: &str) -> String {
    let words: Vec<String> = ascii(label).split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_string).collect();
    let mut base = if words.is_empty() {
        fallback.to_string()
    } else if upper {
        words.iter().map(|w| w.to_uppercase()).collect::<Vec<_>>().join("_")
    } else {
        words.iter().map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
        }).collect()
    };
    if base.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        base = format!("{fallback}{base}");
    }
    let mut id = base.clone();
    let mut n = 2;
    while crate::source::find_line(src, &id).is_some() {
        id = format!("{base}{n}");
        n += 1;
    }
    id
}

/// Quotes a flowchart label when it has characters Mermaid would read as syntax.
pub(crate) fn flow_label(text: &str) -> String {
    let t = text.trim().replace('\n', "<br/>");
    let plain = t.chars().all(|c| c.is_alphanumeric() || " _-.,:;!?'".contains(c));
    if plain && !t.is_empty() {
        t
    } else {
        format!("\"{}\"", t.replace('"', "#quot;"))
    }
}

pub(crate) fn free_text(text: &str) -> String {
    text.trim().replace('\n', " ").replace(';', ",").replace('#', "")
}

fn p<'a>(params: &'a Value, k: &str) -> Option<&'a str> {
    params.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty())
}

fn req<'a>(params: &'a Value, k: &str) -> Result<&'a str, String> {
    p(params, k).ok_or_else(|| format!("missing `{k}`"))
}

fn list(params: &Value, k: &str) -> Vec<String> {
    match params.get(k) {
        Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
        Some(Value::String(s)) => s.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect(),
        _ => vec![],
    }
}

pub(crate) fn flow_shape(shape: &str, label: &str) -> String {
    let l = flow_label(label);
    match shape {
        "round" => format!("({l})"),
        "stadium" => format!("([{l}])"),
        "subroutine" => format!("[[{l}]]"),
        "database" => format!("[({l})]"),
        "circle" => format!("(({l}))"),
        "decision" => format!("{{{l}}}"),
        "hexagon" => format!("{{{{{l}}}}}"),
        "io" => format!("[/{l}/]"),
        "flag" => format!(">{l}]"),
        _ => format!("[{l}]"),
    }
}

fn flow_link(style: &str, label: Option<&str>) -> String {
    let arrow = match style {
        "open" => "---",
        "dotted" => "-.->",
        "thick" => "==>",
        "both" => "<-->",
        _ => "-->",
    };
    match label {
        Some(l) => format!("{arrow}|{}|", flow_label(l)),
        None => arrow.to_string(),
    }
}

/// Starter text for a new diagram of `kind` (used by "New diagram" in the apps).
pub fn template(kind: &str, lang: crate::Lang) -> Option<String> {
    let pt = lang == crate::Lang::PtBr;
    let (a, b, c) = if pt { ("Início", "Etapa", "Fim") } else { ("Start", "Step", "End") };
    Some(match kind {
        "flowchart" => format!("flowchart LR\n    inicio([{a}]) --> etapa[{b}]\n    etapa --> fim([{c}])\n"),
        "sequence" => {
            let (u, s, m, r) = if pt { ("Usuário", "Sistema", "Solicita", "Responde") } else { ("User", "System", "Request", "Response") };
            format!("sequenceDiagram\n    actor U as {u}\n    participant S as {s}\n    U->>S: {m}\n    S-->>U: {r}\n")
        }
        "class" => "classDiagram\n    class Pedido {\n        +String id\n        +total() Money\n    }\n    class Cliente {\n        +String nome\n    }\n    Cliente \"1\" --> \"*\" Pedido : faz\n".to_string(),
        "state" => "stateDiagram-v2\n    [*] --> Aberto\n    Aberto --> Fechado : concluir\n    Fechado --> [*]\n".to_string(),
        "er" => "erDiagram\n    CLIENTE ||--o{ PEDIDO : faz\n    CLIENTE {\n        int id PK\n        string nome\n    }\n    PEDIDO {\n        int id PK\n        int cliente_id FK\n    }\n".to_string(),
        // every other Mermaid type starts from its official docs example
        other => return crate::kinds::find(other).and_then(|k| k.example()),
    })
}

/// Kinds offered by "new diagram" (all Mermaid diagram types; see `kinds::KINDS`).
pub fn template_kinds() -> Vec<&'static str> {
    crate::kinds::KINDS.iter().map(|k| k.id).collect()
}

/// Applies a constructor. `construct` ids: `direction`, `flow.node`, `flow.edge`, `flow.subgraph`,
/// `seq.participant`, `seq.message`, `seq.block`, `seq.note`, `seq.autonumber`, `class.class`,
/// `class.relation`, `class.note`, `state.state`, `state.transition`, `state.start`, `state.end`,
/// `state.choice`, `state.note`, `er.entity`, `er.relation`.
pub fn apply(src: &str, construct: &str, params: &Value) -> Result<String, String> {
    let lines: Vec<&str> = src.lines().collect();
    let header = header_index(&lines).ok_or("the diagram has no header line")?;
    let kind = kind_of(src).ok_or("constructors are available for flowchart, sequence, class, state and ER diagrams")?;
    let ind = indent_of(&lines, header);
    let family = construct.split('.').next().unwrap_or("");
    let expected = match kind {
        Kind::Flowchart => "flow",
        Kind::Sequence => "seq",
        Kind::Class => "class",
        Kind::State => "state",
        Kind::Er => "er",
    };
    if construct != "direction" && family != expected {
        return Err(format!("`{construct}` does not apply to {} diagrams", kind.id()));
    }

    let add = |new: Vec<String>| Ok(insert_body(src, &new.into_iter().map(|l| format!("{ind}{l}")).collect::<Vec<_>>()));

    match construct {
        "direction" => {
            let dir = req(params, "dir")?;
            if !DIRECTIONS.contains(&dir) {
                return Err(format!("direction must be one of {}", DIRECTIONS.join(", ")));
            }
            let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
            match kind {
                Kind::Flowchart => {
                    let mut parts: Vec<&str> = lines[header].split_whitespace().collect();
                    let lead: String = lines[header].chars().take_while(|c| c.is_whitespace()).collect();
                    if parts.len() >= 2 && DIRECTIONS.contains(&parts[1]) {
                        parts[1] = dir;
                    } else {
                        parts.insert(1, dir);
                    }
                    out[header] = format!("{lead}{}", parts.join(" "));
                }
                Kind::Sequence => return Err("sequence diagrams have no direction".into()),
                _ => {
                    let existing = out.iter().skip(header + 1).position(|l| l.trim().starts_with("direction "));
                    match existing {
                        Some(k) => out[header + 1 + k] = format!("{ind}direction {dir}"),
                        None => out.insert(header + 1, format!("{ind}direction {dir}")),
                    }
                }
            }
            let mut s = out.join("\n");
            s.push('\n');
            Ok(s)
        }

        // ---------------------------------------------------------------- flowchart
        "flow.node" => {
            let label = req(params, "label")?;
            let id = p(params, "id").map(str::to_string).unwrap_or_else(|| unique_id(src, label, "n"));
            let shape = flow_shape(p(params, "shape").unwrap_or("rect"), label);
            match p(params, "from") {
                Some(from) => add(vec![format!("{from} {} {id}{shape}", flow_link(p(params, "style").unwrap_or("arrow"), p(params, "edge_label")))]),
                None => add(vec![format!("{id}{shape}")]),
            }
        }
        "flow.edge" => {
            let (from, to) = (req(params, "from")?, req(params, "to")?);
            add(vec![format!("{from} {} {to}", flow_link(p(params, "style").unwrap_or("arrow"), p(params, "label")))])
        }
        "flow.subgraph" => {
            let title = req(params, "title")?;
            let id = unique_id(src, title, "g");
            let mut out = vec![format!("subgraph {id} [{}]", flow_label(title))];
            out.extend(list(params, "members").into_iter().map(|m| format!("{ind}{m}")));
            out.push("end".into());
            add(out)
        }

        // ---------------------------------------------------------------- sequence
        "seq.participant" => {
            let label = req(params, "label")?;
            let id = type_name(src, label, false, "P");
            let keyword = if p(params, "actor") == Some("true") || params.get("actor").and_then(Value::as_bool) == Some(true) { "actor" } else { "participant" };
            if id == label {
                add(vec![format!("{keyword} {id}")])
            } else {
                add(vec![format!("{keyword} {id} as {}", free_text(label))])
            }
        }
        "seq.message" => {
            let (from, to) = (req(params, "from")?, req(params, "to")?);
            let arrow = match p(params, "type").unwrap_or("sync") {
                "reply" => "-->>",
                "async" => "-)",
                "lost" => "-x",
                "line" => "->",
                _ => "->>",
            };
            add(vec![format!("{from}{arrow}{to}: {}", free_text(p(params, "text").unwrap_or("...")))])
        }
        "seq.block" => {
            let (from, to) = (req(params, "from")?, req(params, "to")?);
            let cond = free_text(p(params, "condition").unwrap_or("condição"));
            let block = p(params, "block").unwrap_or("alt");
            let mut out = vec![format!("{block} {cond}"), format!("{ind}{from}->>{to}: ...")];
            match block {
                "alt" => {
                    out.push(format!("else {}", free_text(p(params, "else").unwrap_or("senão"))));
                    out.push(format!("{ind}{to}-->>{from}: ..."));
                }
                "par" => {
                    out.push("and".into());
                    out.push(format!("{ind}{to}->>{from}: ..."));
                }
                "loop" | "opt" | "critical" | "break" => {}
                other => return Err(format!("unknown block `{other}`")),
            }
            out.push("end".into());
            add(out)
        }
        "seq.note" => {
            let a = req(params, "participant")?;
            let text = free_text(req(params, "text")?);
            let line = match (p(params, "position").unwrap_or("right"), p(params, "other")) {
                ("over", Some(b)) => format!("Note over {a},{b}: {text}"),
                ("over", None) => format!("Note over {a}: {text}"),
                ("left", _) => format!("Note left of {a}: {text}"),
                _ => format!("Note right of {a}: {text}"),
            };
            add(vec![line])
        }
        "seq.autonumber" => {
            if lines.iter().any(|l| l.trim() == "autonumber") {
                return Ok(src.to_string());
            }
            let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
            out.insert(header + 1, format!("{ind}autonumber"));
            Ok(out.join("\n") + "\n")
        }

        // ---------------------------------------------------------------- class
        "class.class" => {
            let name = type_name(src, req(params, "name")?, false, "Class");
            let mut body = vec![];
            if let Some(st) = p(params, "stereotype") {
                body.push(format!("<<{st}>>"));
            }
            body.extend(list(params, "attributes"));
            body.extend(list(params, "methods").into_iter().map(|m| if m.contains('(') { m } else { format!("{m}()") }));
            if body.is_empty() {
                return add(vec![format!("class {name}")]);
            }
            let mut out = vec![format!("class {name} {{")];
            out.extend(body.into_iter().map(|b| format!("{ind}{b}")));
            out.push("}".into());
            add(out)
        }
        "class.relation" => {
            let (from, to) = (req(params, "from")?, req(params, "to")?);
            let arrow = match p(params, "type").unwrap_or("association") {
                "inheritance" => "<|--",
                "composition" => "*--",
                "aggregation" => "o--",
                "dependency" => "..>",
                "realization" => "..|>",
                "link" => "--",
                _ => "-->",
            };
            let card = |k| p(params, k).map(|c| format!(" \"{c}\"")).unwrap_or_default();
            let label = p(params, "label").map(|l| format!(" : {}", free_text(l))).unwrap_or_default();
            add(vec![format!("{from}{} {arrow}{} {to}{label}", card("from_card"), card("to_card"))])
        }
        "class.note" => {
            let text = free_text(req(params, "text")?).replace('"', "'");
            match p(params, "for") {
                Some(c) => add(vec![format!("note for {c} \"{text}\"")]),
                None => add(vec![format!("note \"{text}\"")]),
            }
        }

        // ---------------------------------------------------------------- state
        "state.state" => {
            let label = req(params, "label")?;
            let id = type_name(src, label, false, "State");
            if id == label {
                add(vec![format!("state {id}")])
            } else {
                add(vec![format!("state \"{}\" as {id}", free_text(label).replace('"', "'"))])
            }
        }
        "state.transition" => {
            let (from, to) = (req(params, "from")?, req(params, "to")?);
            let label = p(params, "label").map(|l| format!(" : {}", free_text(l))).unwrap_or_default();
            add(vec![format!("{from} --> {to}{label}")])
        }
        "state.start" => add(vec![format!("[*] --> {}", req(params, "to")?)]),
        "state.end" => add(vec![format!("{} --> [*]", req(params, "from")?)]),
        "state.choice" => {
            let id = unique_id(src, p(params, "label").unwrap_or("choice"), "choice");
            add(vec![format!("state {id} <<choice>>")])
        }
        "state.note" => {
            let s = req(params, "state")?;
            let side = if p(params, "position") == Some("left") { "left" } else { "right" };
            add(vec![format!("note {side} of {s} : {}", free_text(req(params, "text")?))])
        }

        // ---------------------------------------------------------------- ER
        "er.entity" => {
            let name = type_name(src, req(params, "name")?, true, "ENTITY");
            let attrs = list(params, "attributes");
            if attrs.is_empty() {
                return add(vec![format!("{name} {{"), format!("{ind}int id PK"), "}".into()]);
            }
            let mut out = vec![format!("{name} {{")];
            out.extend(attrs.into_iter().map(|a| format!("{ind}{a}")));
            out.push("}".into());
            add(out)
        }
        "er.relation" => {
            let (from, to) = (req(params, "from")?, req(params, "to")?);
            let left = match p(params, "from_card").unwrap_or("one") {
                "zero_one" => "|o",
                "zero_many" => "}o",
                "one_many" => "}|",
                _ => "||",
            };
            let right = match p(params, "to_card").unwrap_or("zero_many") {
                "zero_one" => "o|",
                "one" => "||",
                "one_many" => "|{",
                _ => "o{",
            };
            let link = if p(params, "identifying") == Some("false") { ".." } else { "--" };
            let label = free_text(p(params, "label").unwrap_or("relates"));
            let label = if label.contains(' ') { format!("\"{label}\"") } else { label };
            add(vec![format!("{from} {left}{link}{right} {to} : {label}")])
        }

        other => Err(format!("unknown constructor `{other}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detects_kind_and_direction() {
        assert_eq!(kind_of("---\ntitle: x\n---\n%% c\nflowchart LR\n A"), Some(Kind::Flowchart));
        assert_eq!(direction_of("graph TD\n A"), Some("TD".into()));
        assert_eq!(direction_of("classDiagram\n  direction RL\n  class A"), Some("RL".into()));
        assert_eq!(kind_of("pie\n \"a\": 1"), None);
    }

    #[test]
    fn flowchart_constructors() {
        let src = "flowchart LR\n  a[Start] --> b[End]\n%% @odg layout exact\n";
        let s = apply(src, "direction", &json!({"dir": "TB"})).unwrap();
        assert!(s.starts_with("flowchart TB\n"));
        let s = apply(src, "flow.node", &json!({"label": "Aprovação do gestor", "shape": "decision", "from": "a", "edge_label": "sim"})).unwrap();
        assert_eq!(s, "flowchart LR\n  a[Start] --> b[End]\n  a -->|sim| aprovacao_do_gestor{Aprovação do gestor}\n%% @odg layout exact\n");
        let s = apply(src, "flow.node", &json!({"label": "Valor (R$)", "shape": "database"})).unwrap();
        assert!(s.contains("  valor_r[(\"Valor (R$)\")]"));
        let s = apply(src, "flow.edge", &json!({"from": "b", "to": "a", "style": "dotted"})).unwrap();
        assert!(s.contains("  b -.-> a\n"));
        let s = apply(src, "flow.subgraph", &json!({"title": "Financeiro", "members": ["a", "b"]})).unwrap();
        assert!(s.contains("  subgraph financeiro [Financeiro]\n    a\n    b\n  end\n"));
        assert!(apply(src, "seq.message", &json!({})).is_err());
        // reserved word and uniqueness
        assert_eq!(unique_id("flowchart LR\n end_x", "end", "n"), "n_end");
        assert_eq!(unique_id("flowchart LR\n  start --> b", "Start", "n"), "start2");
    }

    #[test]
    fn sequence_constructors() {
        let src = "sequenceDiagram\n    A->>B: hi\n";
        let s = apply(src, "seq.participant", &json!({"label": "Payment API"})).unwrap();
        assert!(s.ends_with("    participant PaymentAPI as Payment API\n"));
        let s = apply(src, "seq.message", &json!({"from": "B", "to": "A", "type": "reply", "text": "ok; done"})).unwrap();
        assert!(s.ends_with("    B-->>A: ok, done\n"));
        let s = apply(src, "seq.block", &json!({"block": "alt", "from": "A", "to": "B", "condition": "aprovado"})).unwrap();
        assert!(s.contains("    alt aprovado\n        A->>B: ...\n    else senão\n        B-->>A: ...\n    end\n"));
        let s = apply(src, "seq.autonumber", &json!({})).unwrap();
        assert!(s.starts_with("sequenceDiagram\n    autonumber\n"));
        assert!(apply(src, "direction", &json!({"dir": "LR"})).is_err());
    }

    #[test]
    fn class_state_er_constructors() {
        let s = apply("classDiagram\n  class A\n", "class.class", &json!({"name": "order item", "attributes": "+int qty\n+Money price", "methods": ["total"]})).unwrap();
        assert!(s.contains("  class OrderItem {\n    +int qty\n    +Money price\n    total()\n  }\n"));
        let s = apply("classDiagram\n  class A\n", "class.relation", &json!({"from": "A", "to": "B", "type": "composition", "from_card": "1", "to_card": "*", "label": "has"})).unwrap();
        assert!(s.contains("  A \"1\" *-- \"*\" B : has\n"));
        let s = apply("classDiagram\n  class A\n", "direction", &json!({"dir": "LR"})).unwrap();
        assert!(s.starts_with("classDiagram\n  direction LR\n"));
        let s = apply("stateDiagram-v2\n  [*] --> Idle\n", "state.state", &json!({"label": "Waiting for payment"})).unwrap();
        assert!(s.contains("  state \"Waiting for payment\" as WaitingForPayment\n"));
        let s = apply("stateDiagram-v2\n  [*] --> Idle\n", "state.end", &json!({"from": "Idle"})).unwrap();
        assert!(s.ends_with("  Idle --> [*]\n"));
        let s = apply("erDiagram\n  A ||--o{ B : has\n", "er.relation", &json!({"from": "A", "to": "C", "from_card": "one", "to_card": "one_many", "label": "is in"})).unwrap();
        assert!(s.contains("  A ||--|{ C : \"is in\"\n"));
        let s = apply("erDiagram\n  A ||--o{ B : has\n", "er.entity", &json!({"name": "order item"})).unwrap();
        assert!(s.contains("  ORDER_ITEM {\n    int id PK\n  }\n"));
    }
}
