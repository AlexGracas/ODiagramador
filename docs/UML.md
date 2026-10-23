# Preparação para UML / UML readiness

Mermaid already covers a good part of UML — `classDiagram`, `sequenceDiagram` and `stateDiagram-v2` — and ODiagramador lints them today. This document describes how a dedicated UML engine plugs in without touching the apps or the MCP server.

## Extension points already in place

| Piece | Where | What UML gets for free |
|---|---|---|
| `DiagramEngine` trait | `crates/odg-core/src/engine.rs` | Engine chosen by file extension. `UmlEngine` is registered (`.puml`, `.uml`) but `available() == false`, so such files are listed and reported clearly instead of ignored. |
| `RenderBackend` trait | same file | Any engine can reuse the hidden-WebView renderer (headless) or the apps' WebViews. |
| Neutral `DiagramModel` | `crates/odg-core/src/model.rs` | nodes / edges / groups with bounding boxes. All layout lint rules (overlap, orphan, overflow, size) run on this model, not on Mermaid. |
| `Report` / `Problem` | `crates/odg-core/src/report.rs` | Same report, same codes, same annotated image for every engine. |
| i18n | `locales/*/odg.ftl` | Add `problem-U_*` / `hint-U_*` keys for UML-specific rules. |

## Recommended path (no Java)

PlantUML needs a JVM, which the project explicitly avoids. The plan is a **UML front-end that compiles to Mermaid** for the diagram kinds Mermaid renders well, plus native rules on the UML model:

1. **Phase 1 — PlantUML-subset → Mermaid transpiler (Rust).** Parse the common PlantUML syntax for class, sequence, state, use-case and activity diagrams into a typed UML AST (`crates/odg-uml`). Emit Mermaid (`classDiagram`, `sequenceDiagram`, `stateDiagram-v2`, `flowchart` for activity/use-case). `UmlEngine::prepare` returns that Mermaid; keep a line map so parse/lint locations point at the original `.puml` lines.
2. **Phase 2 — UML semantic lint on the AST**, e.g. `U_ABSTRACT_INSTANTIATED`, `U_INTERFACE_WITH_FIELDS`, `U_CYCLIC_INHERITANCE`, `U_UNREACHABLE_STATE`, `U_MESSAGE_TO_UNDECLARED_PARTICIPANT`, `U_MISSING_MULTIPLICITY`.
3. **Phase 3 — XMI import/export** (UML 2.5 XMI from StarUML, Visual Paradigm, Enterprise Architect) to the same AST, so agents can read models from modelling tools.
4. **Phase 4 (optional) — native UML renderer** in the render-kit (e.g. ELK.js layout + custom SVG shapes) for diagram types Mermaid lacks (component, deployment, object, timing). It would be another render-kit op; the bridge protocol (`process` → `RawRender`) stays the same.

## MCP surface for UML

No new tools are needed: `create_diagram(path: "model/orders.puml", …)` picks the engine from the extension. Optional additions once Phase 2 exists: `convert_diagram(path, to: "mermaid" | "plantuml" | "xmi")` and `get_syntax_help("uml-class")`.

## Checklist to enable the engine

- [ ] `crates/odg-uml` with parser + Mermaid emitter + line map, unit-tested against fixtures in `examples/uml/`.
- [ ] `UmlEngine::available()` → `true`, `prepare()` calls the transpiler.
- [ ] Map Mermaid error lines back through the line map in `lint::syntax_problem`.
- [ ] Add `docs/syntax/uml-*.md` cheat-sheets and topic aliases in `syntax.rs`.
- [ ] Add `.puml` to the apps' "new diagram" options.
