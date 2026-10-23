# Mermaid quick reference (ODiagramador)

Pinned engine: Mermaid 11.17.2 (offline). Every diagram starts with a type header:

| Header | Use for | Help topic |
|---|---|---|
| `flowchart TD` / `flowchart LR` | processes, architectures, decision trees | `flowchart` |
| `sequenceDiagram` | messages between participants over time | `sequence` |
| `classDiagram` | UML classes, interfaces, relations | `class` |
| `stateDiagram-v2` | UML state machines | `state` |
| `erDiagram` | database entities and cardinality | `er` |
| `gantt` | schedules | `gantt` |
| `pie` | proportions | `pie` |
| `mindmap` | idea trees | `mindmap` |

Rules that avoid most errors:
- One statement per line. Comments start with `%%` at the beginning of a line.
- Quote labels with special characters: `A["Price (R$) / month"]`.
- Node ids are simple words (`orderService`, `db1`); put the text in the label.
- Line breaks inside labels: `<br/>`.
- Optional front matter must be the very first lines:
  ```
  ---
  title: My diagram
  ---
  ```
- Words such as `end`, `graph`, `subgraph` cannot be bare node ids; use `End[End]` or `e1[end]`.

Workflow for agents: `validate_diagram` → fix the first error → `create_diagram`/`update_diagram`
→ look at the returned image → fix warnings (numbers on the annotated image match the report).
