# ODiagramador para agentes de IA / for AI agents

ODiagramador gives an agent three things a plain Mermaid file cannot:

1. **Validation with exact locations** — syntax errors with line, column, the offending source line and how to fix it.
2. **Eyes** — every write returns a PNG. When something is wrong the image carries **numbered markers that match the problem list** (or, if the diagram does not parse, a picture of the source with the bad line highlighted).
3. **A channel to the human** — the person watching the desktop app leaves comments on diagrams/elements; the agent reads and resolves them.

Everything runs offline (Mermaid 11.17.2 is embedded) and diagrams are plain `.mmd` files in the workspace, versionable with git.

## 1. Configure the MCP server

The server is the `odg` binary (inside `ODiagramador.app/Contents/MacOS/`, next to `ODiagramador.exe`, or in `target/release/` when built from source). The desktop app has a **Copy MCP config** button that fills in the right paths.

**Claude Code**

```bash
claude mcp add odiagramador -- /path/to/odg mcp --workspace /path/to/project
```

**Codex** (`~/.codex/config.toml`)

```toml
[mcp_servers.odiagramador]
command = "/path/to/odg"
args = ["mcp", "--workspace", "/path/to/project"]
```

**Cursor / Claude Desktop / any MCP client** (JSON)

```json
{ "mcpServers": { "odiagramador": { "command": "/path/to/odg", "args": ["mcp", "--workspace", "/path/to/project"] } } }
```

Options: `--lang pt-BR` (or `en`) for message language; default comes from `.odiagramador/config.json`, then `$LANG`.
On a Linux server without a display run `xvfb-run -a odg mcp …` (the renderer uses the system WebKit).

## 2. Tools

| Tool | Use it to |
|---|---|
| `get_syntax_help(topic)` | Read an offline cheat-sheet (`overview`, `flowchart`, `sequence`, `class`, `state`, `er`, `gantt`, `pie`, `mindmap`) before writing. |
| `create_diagram(name\|path, source)` | Save a new diagram → report + image. |
| `update_diagram(path, source)` | Replace the whole file → report + image. |
| `patch_diagram(path, edits[{old_text,new_text}])` | Small fixes; each `old_text` must occur exactly once. |
| `validate_diagram(source)` | Check text without saving. |
| `render_diagram(path\|source, format, annotated)` | Look at the result again (PNG or SVG). |
| `inspect_diagram(path\|source)` | Text description of the layout (nodes with positions, edges, groups) — for models without vision. |
| `list_diagrams()` / `get_diagram(path)` | Status of every diagram / numbered source + last report + open comments. |
| `get_feedback(path?)` / `resolve_feedback(id, resolution)` | Read and close the human's comments. |
| `get_activity(path?)` | Who changed what recently. |
| `open_in_viewer(path)` | Bring the diagram up in the desktop app for the human. |
| `delete_diagram(path)` | Remove a diagram and its previews. |

Resources: `diagram://<path>` (source), `diagram://<path>#report` (JSON), `diagram://<path>#preview` (PNG).
Prompts: `fix_diagram_problems(path)`, `create_diagram_from_description(description, type)`.

Every write tool also returns `structuredContent` with the full report (see below), so programmatic clients do not need to parse text.

## 3. Recommended loop

```
get_syntax_help → create_diagram
loop:
  read report: errors first (line + hint), then warnings
  look at the image: markers ①②③ = problems 1,2,3
  patch_diagram with the smallest fix
until valid and no warnings (or warnings are intentional)
get_feedback → address comments → resolve_feedback
```

## 4. Problem codes

| Code | Severity | Meaning |
|---|---|---|
| `E_EMPTY` | error | No content. |
| `E_UNKNOWN_TYPE` | error | First line does not declare a diagram type. |
| `E_PARSE` | error | Syntax error; `line`/`column`, offending line, expected tokens and unclosed-bracket detection in `hint`. |
| `E_RENDER` | error | Parsed but could not be drawn. |
| `E_BACKEND` | error | The renderer (system WebView) is unavailable. |
| `W_ORPHAN_NODE` | warning (info for class/ER) | Node without edges. |
| `W_OVERLAP` | warning | Two nodes overlap in the rendered layout. |
| `W_LABEL_OVERFLOW` | warning | Text does not fit inside its shape. |
| `W_DUPLICATE_LABEL` | warning | Different ids share a label (usually the same node declared twice). |
| `W_LONG_LABEL` | info | Label over 60 characters. |
| `W_TOO_LARGE` | info | More than 60 nodes / 100 edges — consider splitting. |
| `W_WIDE` / `W_TALL` | info | Extreme aspect ratio — change direction. |
| `W_FOREIGN_OBJECT` | info | HTML labels; PNG may miss text (use SVG). |
| `W_RASTER` | warning | PNG could not be produced. |

Codes are stable English identifiers; `message` and `hint` follow the configured language.

## 5. Without MCP

Any agent that can run commands can use the CLI and read files:

```bash
odg check diagrams/*.mmd --json      # exit code 1 when any diagram has errors
odg render diagrams/flow.mmd --annotated   # writes diagrams/flow.annotated.png
odg watch                            # keeps reports fresh while you edit files directly
```

After a check, `.odiagramador/reports/<path>.json` holds the report and `.odiagramador/previews/` the images.

## 6. Report shape

```json
{
  "path": "diagrams/login.mmd", "engine": "mermaid", "diagram_type": "flowchart-v2",
  "valid": false, "error_count": 1, "warning_count": 0, "info_count": 0,
  "summary": "Invalid diagram: 1 error(s). Fix the first error and validate again.",
  "problems": [{
    "index": 1, "severity": "error", "code": "E_PARSE", "line": 3, "column": 18,
    "message": "Syntax error at line 3: unexpected '|'",
    "hint": "Line 3 opens '[' and never closes it: add ']'. … [3: `B -->|yes| C[Save`]",
    "element_id": null, "bbox": null
  }],
  "stats": { "nodes": 0, "edges": 0, "groups": 0, "width": 0, "height": 0, "lines": 4 },
  "previews": { "annotated_png": ".odiagramador/previews/diagrams__login.mmd.annotated.png" }
}
```
