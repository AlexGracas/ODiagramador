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

problem-W_LAYOUT_MODE = Unknown layout mode "{ $mode }".
hint-W_LAYOUT_MODE = Use one of: { $modes }.

problem-W_LAYOUT_UNKNOWN_ID = The layout comment refers to "{ $id }", which is not in the diagram.
hint-W_LAYOUT_UNKNOWN_ID = Fix the id or remove the `%% @odg` line (ids are case-sensitive).

problem-W_LAYOUT_IGNORED = Node positions are ignored in layout mode "{ $mode }".
hint-W_LAYOUT_IGNORED = Add `%% @odg layout elk-interactive` (positions as hints) or `%% @odg layout exact` (exact positions).

problem-W_LAYOUT_UNSUPPORTED = Layout comments have no effect on { $type } diagrams.
hint-W_LAYOUT_UNSUPPORTED = Positions work on flowchart, state, class, ER and requirement diagrams.

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
ui-layout = Layout
ui-layout-auto = Auto (Mermaid)
ui-layout-elk = ELK
ui-layout-interactive = ELK interactive (drag)
ui-layout-exact = Exact positions (drag)
ui-reset-positions = Reset positions
ui-cancel = Cancel
ui-create = Create

## Palette (constructors)

ui-new-diagram-type = Diagram type
ui-type-flowchart = Flowchart
ui-type-sequence = Sequence
ui-type-class = Class (UML)
ui-type-state = State machine (UML)
ui-type-er = Entity-relationship (ER)
ui-c-direction = Direction
ui-c-node = Node
ui-c-decision = Decision
ui-c-startend = Start/End
ui-c-edge = Edge
ui-c-subgraph = Group
ui-c-participant = Participant
ui-c-actor = Actor
ui-c-message = Message
ui-c-block = Block
ui-c-note = Note
ui-c-autonumber = Numbering
ui-c-class = Class
ui-c-relation = Relation
ui-c-state = State
ui-c-transition = Transition
ui-c-start = Start
ui-c-end = End
ui-c-choice = Choice
ui-c-entity = Entity
ui-f-label = Text
ui-f-shape = Shape
ui-f-connect-from = Connect from
ui-f-edge-label = Edge text
ui-f-from = From
ui-f-to = To
ui-f-style = Line
ui-f-title = Title
ui-f-members = Members
ui-f-type = Type
ui-f-condition = Condition
ui-f-position = Position
ui-f-other = Also over
ui-f-name = Name
ui-f-stereotype = Stereotype
ui-f-attributes = Attributes (one per line)
ui-f-methods = Methods (one per line)
ui-f-from-card = Cardinality (from)
ui-f-to-card = Cardinality (to)
ui-f-for = Attached to
ui-f-add = Add
ui-f-cancel = Cancel
ui-f-none = (none)
ui-o-rect = Rectangle
ui-o-round = Rounded
ui-o-stadium = Stadium
ui-o-decision = Decision
ui-o-database = Database
ui-o-circle = Circle
ui-o-subroutine = Subroutine
ui-o-io = Input/Output
ui-o-hexagon = Hexagon
ui-o-arrow = Arrow
ui-o-open = Line
ui-o-dotted = Dotted
ui-o-thick = Thick
ui-o-both = Both ways
ui-o-sync = Request (->>)
ui-o-reply = Reply (-->>)
ui-o-async = Async (-))
ui-o-lost = Lost (-x)
ui-o-right = Right of
ui-o-left = Left of
ui-o-over = Over
ui-o-inheritance = Inheritance
ui-o-composition = Composition
ui-o-aggregation = Aggregation
ui-o-association = Association
ui-o-dependency = Dependency
ui-o-realization = Realization
ui-o-one = Exactly one
ui-o-zero_one = Zero or one
ui-o-zero_many = Zero or many
ui-o-one_many = One or many
ui-dir-TB = Top → bottom (TB)
ui-dir-BT = Bottom → top (BT)
ui-dir-LR = Left → right (LR)
ui-dir-RL = Right → left (RL)
ui-o-flag = Flag
ui-i-node = Edit element
ui-i-edge = Edit connection
ui-i-save = Save
ui-i-delete = Delete
ui-i-confirm = Click again to delete

## Settings, updates, MCP menu

ui-export = Export
ui-settings = Settings
ui-settings-general = General
ui-settings-editor = Editor & diagram
ui-settings-mcp = MCP
ui-appearance = Appearance
ui-appearance-system = Same as system
ui-appearance-light = Light
ui-appearance-dark = Dark
ui-editor-font = Editor font
ui-editor-font-size = Editor font size
ui-diagram-font = Diagram font
ui-diagram-font-hint = Saved in the folder (.odiagramador/config.json) and also used by AI agents, so everyone sees the same layout.
ui-font-default = Default (Arial)
ui-updates = Updates
ui-auto-update = Check for updates automatically
ui-check-updates = Check for updates…
ui-version = Version { $version }
ui-up-to-date = You have the latest version ({ $version }).
ui-update-available = ODiagramador { $version } is available (you have { $current }).
ui-update-now = Update now
ui-update-later = Later
ui-update-release-page = See on GitHub
ui-updating = Downloading the update…
ui-update-restart = The app will restart to finish the update.
ui-update-failed = The update failed: { $detail }
ui-update-manual = This copy cannot replace itself. Download the new version from GitHub.
ui-mcp-configure = Configure MCP
ui-mcp-copy-for = Copy configuration for:
ui-mcp-other = Other (JSON)
ui-mcp-settings = MCP settings…
ui-mcp-copied = MCP configuration for { $app } copied to the clipboard.
ui-mcp-copy-failed = Could not use the clipboard. Copy the configuration below:
ui-mcp-local = Local (this computer)
ui-mcp-local-hint = Agents on this computer start their own ODiagramador server; use the buttons above to copy the configuration.
ui-mcp-remote = Remote access
ui-mcp-remote-enable = Expose MCP on a network port
ui-mcp-port = Port
ui-mcp-scope = Who can connect
ui-mcp-scope-local = Only this computer (127.0.0.1)
ui-mcp-scope-lan = Other computers on the network (token required)
ui-mcp-token = Access token
ui-mcp-regenerate = New token
ui-mcp-running = Running at { $url }
ui-mcp-stopped = Stopped
ui-mcp-error = Could not start: { $detail }
ui-mcp-remote-warning = Anyone with the token can read and change the diagrams in this folder. Traffic is plain HTTP: outside your local network use an SSH tunnel or a VPN.
ui-mcp-copy-remote = Copy remote configuration
ui-auto-install = Install updates automatically (the app restarts)
