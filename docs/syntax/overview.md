# Mermaid quick reference (ODiagramador)

Pinned engine: Mermaid 12.2.0 (offline, with the ZenUML plugin). Every diagram starts with a type header.
All Mermaid diagram types are supported; `get_syntax_help("<topic>")` returns the syntax of each one
(hand-written cheat-sheets where marked, otherwise a digest of the official docs; add ` full` for the
official digest of any type).

| Header | Diagram | Help topic | ELK |
|---|---|---|---|
| `flowchart` | Flowchart | `flowchart` | ✓ |
| `sequenceDiagram` | Sequence | `sequence` |  |
| `classDiagram` | Class (UML) | `class` | ✓ |
| `stateDiagram-v2` | State machine (UML) | `state` | ✓ |
| `erDiagram` | Entity-relationship (ER) | `er` | ✓ |
| `usecase-beta` | Use case (UML) | `usecase` |  |
| `requirementDiagram` | Requirements | `requirement` | ✓ |
| `C4Context` | C4 architecture | `c4` |  |
| `architecture-beta` | Architecture (cloud / services) | `architecture` |  |
| `block` | Block diagram | `block` |  |
| `swimlane-beta` | Swimlanes | `swimlane` |  |
| `zenuml` | Sequence (ZenUML) | `zenuml` |  |
| `gantt` | Gantt schedule | `gantt` |  |
| `timeline` | Timeline | `timeline` |  |
| `journey` | User journey | `journey` |  |
| `kanban` | Kanban board | `kanban` |  |
| `gitGraph` | Git graph | `gitgraph` |  |
| `mindmap` | Mind map | `mindmap` |  |
| `treeView-beta` | Tree view | `treeview` |  |
| `ishikawa-beta` | Ishikawa (fishbone) | `ishikawa` |  |
| `eventmodeling` | Event modeling | `eventmodeling` |  |
| `agentflow-beta` | Agent flow | `agentflow` |  |
| `railroad-ebnf-beta` | Railroad (grammar) | `railroad` |  |
| `packet` | Network packet | `packet` |  |
| `pie` | Pie chart | `pie` |  |
| `xychart` | XY chart (bars / lines) | `xychart` |  |
| `quadrantChart` | Quadrant chart | `quadrant` |  |
| `radar-beta` | Radar chart | `radar` |  |
| `sankey` | Sankey (flows) | `sankey` |  |
| `treemap-beta` | Treemap | `treemap` |  |
| `venn-beta` | Venn diagram | `venn` |  |
| `wardley-beta` | Wardley map | `wardley` |  |
| `cynefin-beta` | Cynefin framework | `cynefin` |  |

✓ = ELK layout, dragging and exact positions (`%% @odg layout …`, `arrange_diagram`) apply; every other
type always uses Mermaid's own layout.

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
