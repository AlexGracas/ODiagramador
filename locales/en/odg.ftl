# ODiagramador — English strings.
# Shared by the MCP server, the CLI and the three desktop apps (via odg-ffi).
# Problem codes (E_*, W_*, I_*) are stable identifiers; only the text is translated.

## Problems

problem-E_EMPTY = The diagram source is empty.
hint-E_EMPTY = Start with a diagram header such as `flowchart TD`, `sequenceDiagram` or `classDiagram`.

problem-E_UNKNOWN_TYPE = Diagram type not recognized.
hint-E_UNKNOWN_TYPE = The first non-comment line must declare the type, e.g. `flowchart LR`, `sequenceDiagram`, `classDiagram`, `stateDiagram-v2`, `erDiagram`, `gantt`, `pie`, `mindmap`. Call get_syntax_help for examples.

problem-E_PARSE = Syntax error at line { $line }: { $detail }
problem-E_PARSE-noline = Syntax error: { $detail }
hint-E_PARSE = Check line { $line } and the line before it. { $expected }
hint-E_PARSE-noline = Validate the diagram piece by piece; call get_syntax_help for the correct syntax.
detail-unexpected-token = unexpected '{ $token }'
detail-unexpected-eof = the diagram ends too early (something is missing at the end)
detail-unexpected-eol = the line ends too early (something is missing at the end of the line)
hint-unclosed = Line { $line } opens '{ $open }' and never closes it: add '{ $close }'.
hint-expected = Expected one of: { $tokens }.

problem-E_RENDER = The diagram parsed but could not be drawn: { $detail }
hint-E_RENDER = Simplify the construct mentioned in the message or split the diagram.

problem-E_BACKEND = The renderer is unavailable: { $detail }
hint-E_BACKEND = On Linux servers run under a display (e.g. `xvfb-run odg mcp`); on Windows install the WebView2 runtime.

problem-W_ORPHAN_NODE = "{ $label }" is not connected to anything.
hint-W_ORPHAN_NODE = Connect it with an edge (e.g. `{ $id } --> Other`) or remove it if it is leftover.

problem-W_OVERLAP = "{ $a }" overlaps "{ $b }".
hint-W_OVERLAP = Change the direction (TB/LR), move one node into a subgraph or shorten the labels.

problem-W_LABEL_OVERFLOW = The text of "{ $label }" does not fit inside its shape.
hint-W_LABEL_OVERFLOW = Shorten the text or break it with `<br/>`.

problem-W_LONG_LABEL = The label of "{ $id }" is very long ({ $length } characters).
hint-W_LONG_LABEL = Keep labels under { $max } characters; break lines with `<br/>` or move details to a note.

problem-W_DUPLICATE_LABEL = Different elements share the label "{ $label }": { $ids }.
hint-W_DUPLICATE_LABEL = If they are the same element, reuse a single id; otherwise make the labels distinct.

problem-W_TOO_LARGE = The diagram is large ({ $nodes } nodes, { $edges } edges).
hint-W_TOO_LARGE = Consider splitting it into smaller diagrams or grouping with subgraphs.

problem-W_WIDE = The diagram is very wide ({ $width }×{ $height } px).
hint-W_WIDE = Use a top-to-bottom direction (`flowchart TB`) or split long chains.

problem-W_TALL = The diagram is very tall ({ $width }×{ $height } px).
hint-W_TALL = Use a left-to-right direction (`flowchart LR`) or split the diagram.

problem-W_FOREIGN_OBJECT = This diagram type uses HTML labels; the PNG preview may miss text.
hint-W_FOREIGN_OBJECT = Prefer the SVG preview for this diagram.

problem-W_RASTER = The PNG preview could not be produced: { $detail }
hint-W_RASTER = Use the SVG preview instead.

## Summaries

summary-valid = Valid { $type } diagram: { $nodes } nodes, { $edges } edges. { $problems }
summary-invalid = Invalid diagram: { $errors } error(s). Fix the first error and validate again.
summary-problems = { $warnings ->
    [0] No visual problems found.
    [one] 1 warning.
   *[other] { $warnings } warnings.
}

## MCP

mcp-next-invalid = Next: fix the first error (exact line above), then send the file again with update_diagram, or a small fix with patch_diagram.
mcp-next-warnings = Next: the diagram renders. Look at the image (markers are numbered like the list) and fix the warnings with patch_diagram unless they are intentional.
mcp-next-clean = Next: nothing to fix. Show the image to the user or call open_in_viewer.

## Desktop UI

ui-app-title = ODiagramador
ui-open-folder = Open folder…
ui-new-diagram = New diagram
ui-new-diagram-name = Diagram name
ui-save = Save
ui-export-png = Export PNG…
ui-export-svg = Export SVG…
ui-diagrams = Diagrams
ui-no-workspace = Open a folder to start. Diagrams are plain .mmd files inside it.
ui-no-diagrams = No diagrams yet. Create one here or ask your AI agent.
ui-empty = No diagram
ui-fit = Fit
ui-problems = Problems
ui-no-problems = No problems
ui-source = Source
ui-preview = Preview
ui-feedback = Comments for agents
ui-add-feedback = Add comment
ui-feedback-placeholder = Tell the agent what to change…
ui-resolved = Resolved
ui-mark-resolved = Mark resolved
ui-activity = Activity
ui-changed-by-agent = Changed by agent
ui-changed-by-human = Changed by you
ui-language = Language
ui-theme = Theme
ui-theme-light = Light
ui-theme-dark = Dark
ui-copy-mcp-config = Copy MCP config
ui-copied = Copied
ui-line = Line { $line }
ui-valid = Valid
ui-invalid = Invalid
ui-severity-error = Error
ui-severity-warning = Warning
ui-severity-info = Info
ui-cancel = Cancel
ui-create = Create
