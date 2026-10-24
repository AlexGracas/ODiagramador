# ODiagramador para agentes de IA / for AI agents

ODiagramador gives an agent three things a plain Mermaid file cannot:

1. **Validation with exact locations** — syntax errors with line, column, the offending source line and how to fix it.
2. **Eyes** — every write returns a PNG. When something is wrong the image carries **numbered markers that match the problem list** (or, if the diagram does not parse, a picture of the source with the bad line highlighted).
3. **A channel to the human** — the person watching the desktop app leaves comments on diagrams/elements; the agent reads and resolves them.

Everything runs offline (Mermaid 12.2.0 with the ZenUML plugin is embedded: every Mermaid diagram type, see `get_syntax_help("overview")`) and diagrams are plain `.mmd` files in the workspace, versionable with git.

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

### Remote (HTTP) access

`odg mcp --workspace DIR --http 127.0.0.1:7410` serves MCP over Streamable HTTP at `http://127.0.0.1:7410/mcp`
(JSON responses, `Mcp-Session-Id` on initialize, `GET /health`). To listen on the network (`--http 0.0.0.0:7410`)
a bearer token is **required** — pass `--token` or set `ODG_MCP_TOKEN`; odg refuses to start without it.
Clients then send `Authorization: Bearer <token>`:

```bash
claude mcp add --transport http odiagramador http://HOST:7410/mcp --header "Authorization: Bearer TOKEN"
```

The desktop apps can run this for you: **Settings → MCP → Remote access** (port, local-only or LAN, token) and
copy ready-made configs for each client. Traffic is plain HTTP: outside a trusted LAN use an SSH tunnel or a VPN.

## 2. Tools

| Tool | Use it to |
|---|---|
| `get_syntax_help(topic)` | Offline syntax with examples for every Mermaid type (`overview` lists them all; `flowchart`, `usecase`, `c4`, `venn`, `wardley`, … or any header such as `xychart-beta`; add ` full` for the official reference digest). |
| `create_diagram(name\|path, source)` | Save a new diagram → report + image. |
| `update_diagram(path, source)` | Replace the whole file → report + image. |
| `patch_diagram(path, edits[{old_text,new_text}])` | Small fixes; each `old_text` must occur exactly once. |
| `validate_diagram(source)` | Check text without saving. |
| `render_diagram(path\|source, format, annotated, focus?, region?)` | Look at the result again (PNG or SVG). Big drawings: `focus=<element or subgraph id>` (or `region`) renders only that part, legibly, listing the problems inside it. |
| `add_element(path, construct, params)` | Insert a well-formed element with the desktop palette's constructors (`flow.node`, `flow.edge`, `seq.message`, `class.relation`, `er.relation`, `direction`…); see the tool description for every construct and parameter. |
| `edit_element(path, action, params)` | Rename, reshape or delete an existing element (`rename`, `shape`, `delete`, `edge_label`, `edge_delete`) without rewriting the file — deleting a node also removes its connections and keeps the neighbours' definitions. |
| `arrange_diagram(path, mode?, positions?, layers?, reset_positions?)` | Fix the layout of graph diagrams: `elk`, `elk-interactive` (positions as hints) or `exact` (exact positions); `layers` pins first/last or fixes the order of layers with numbers (0, 1, 2…, nodes or subgraphs). Stored as `%% @odg` comments — see `get_syntax_help("layout")`. |
| `inspect_diagram(path\|source)` | Text description of the layout (nodes with positions, edges, groups) — for models without vision. |
| `list_diagrams()` / `get_diagram(path)` | Status of every diagram / numbered source + last report + open comments. |
| `get_feedback(path?)` / `resolve_feedback(id, resolution)` | Read and close the human's comments. Each comment carries the context the human saw and could edit (diagram, selected element, its source lines). |
| `wait_for_feedback(path?, wait_seconds?, only_new?)` | Watch the human's comments: returns open ones at once, otherwise wakes up the moment they write one. Act, `resolve_feedback`, call again. |
| `get_activity(path?)` | Who changed what recently. |
| `show_diagram(path, element_ids?, message?, level?)` | Open a diagram in the human's desktop viewer (launching the app if needed), optionally highlighting elements with a note. `open_in_viewer` is the old name. |
| `open_diagram(path, message?)` | Change the diagram open in the human's viewer. |
| `set_zoom(zoom, path?, center_on?)` | Zoom the human's view: factor (`1.5`), percentage (`"150%"`), or `fit` / `in` / `out`. |
| `center_on_element(element_id, path?, zoom?, message?)` | Scroll so an element is in the middle of the screen (it pulses briefly). |
| `select_element(element_id, path?, center?, message?)` | Select an element as if the human clicked it (outline, inspector, source line); empty id clears. `get_viewer_status` then reports it as selected. |
| `highlight_elements(path, element_ids, message?, level?)` | Put elements in evidence: they glow, the rest dims, the view centres on them, and the note appears beside them. |
| `notify_user(message, level?, path?, element_ids?)` | Show a message to the human (progress, summary, warning). Does not wait. |
| `ask_user(question, options?, allow_text?, path?, element_ids?, wait_seconds?)` | Ask the human; options become buttons. Waits up to `wait_seconds` (default 60, max 600) and returns the answer, or a `question_id` to check later. |
| `get_user_replies(question_id?)` | The answer to a question, or the recent conversation. |
| `get_viewer_status()` | Is the viewer open on this workspace? Which diagram, zoom, element in the centre, elements on screen and selected element? |
| `delete_diagram(path)` | Remove a diagram and its previews. |

Resources: `diagram://<path>` (source), `diagram://<path>#report` (JSON), `diagram://<path>#preview` (PNG).
Prompts: `fix_diagram_problems(path)`, `create_diagram_from_description(description, type)`.

Every write tool also returns `structuredContent` with the full report (see below), so programmatic clients do not need to parse text.
Write tools (`create_diagram`, `update_diagram`, `patch_diagram`, `arrange_diagram`, `add_element`, `edit_element`)
accept `show: true` (plus optional `element_ids` and `message`) to open the result for the human in the same call.

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

Talking to the human (desktop viewer open on the same folder):

```
patch_diagram(path, edits, show=true, element_ids=["pay"], message="Added the payment step")
ask_user("Notify by e-mail or SMS?", options=["E-mail","SMS"], path, element_ids=["notify"])
  → "The user answered: E-mail"        (or a question_id → get_user_replies later)
notify_user("Done: 3 diagrams updated", level="success")
```

Messages live in `.odiagramador/messages.jsonl` (the conversation, also shown in the app's Activity tab) and the
viewer's presence in `.odiagramador/viewer.json`, so this works with any number of agents and needs no network.
The human can also leave comments on elements for you: `get_feedback`.

**Acting as soon as the human comments.** Agents only act when they are running, so keep one watching:
- with MCP only: loop `wait_for_feedback` → act → `resolve_feedback` → `wait_for_feedback`;
- in Claude Code (or any agent that runs shell commands in the background and is woken when they end):
  run `odg wait-feedback --workspace <dir> --only-new` in the background; it exits, printing the comment and its
  context, the moment the human clicks "Adicionar comentário", which wakes the agent. Re-launch it after acting.

Layout is separate from content: if the diagram is correct but hard to read, use
`arrange_diagram` (switch to `elk`, pin nodes with `positions`, or `layers: first/last`)
instead of rewriting the source. Coordinates are node centres in the same space as
`inspect_diagram` bounding boxes.

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
| `W_EDGE_THROUGH_NODE` | warning | An arrow is drawn across a node that is not one of its ends. |
| `W_EDGE_TOO_SHORT` | warning | An arrow is too short for its arrowhead (it can look reversed or vanish), e.g. adjacent blocks. |
| `W_LABEL_OVER_NODE` | warning | An arrow label is drawn over a node's text. |
| `W_LABEL_OVERLAP` | warning | Two arrow labels are drawn over each other. |
| `W_EDGE_CROSSINGS` | info | Arrow crossings (count and pairs); `stats.crossings` lets you compare versions. |
| `W_LABEL_OVERFLOW` | warning | Text does not fit inside its shape. |
| `W_DUPLICATE_LABEL` | warning | Different ids share a label (usually the same node declared twice). |
| `W_LONG_LABEL` | info | Label over 60 characters. |
| `W_TOO_LARGE` | info | More than 60 nodes / 100 edges — consider splitting. |
| `W_WIDE` / `W_TALL` | info | Extreme aspect ratio — change direction. |
| `W_FOREIGN_OBJECT` | info | HTML labels; the PNG draws them as plain text (SVG / viewer are exact). |
| `W_RASTER` | warning | PNG could not be produced. |
| `W_LAYOUT_MODE` | warning | Unknown `%% @odg layout` mode. |
| `W_LAYOUT_UNKNOWN_ID` | warning | A `%% @odg pos/layer` line names a node that does not exist. |
| `W_LAYOUT_IGNORED` | info | Positions given while the mode is `auto`/`elk`. |
| `W_LAYOUT_UNSUPPORTED` | info | Layout comments on a diagram type without graph layout (sequence, gantt, pie…): Mermaid's own layout is used. |

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
