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

problem-W_EDGE_TOO_SHORT = The arrow { $edge } is only { $length } px long: there is no room for its arrowhead, which can look reversed or disappear.
hint-W_EDGE_TOO_SHORT = Leave room between the two elements (in block diagrams add `space` between them or put them in different rows), or change the direction.
problem-W_EDGE_THROUGH_NODE = The arrow { $edge } is drawn across "{ $node }".
hint-W_EDGE_THROUGH_NODE = Reorder the elements, change the direction, or (graph diagrams) use arrange_diagram with ELK / layers so the arrow goes around.
problem-W_LABEL_OVER_NODE = The arrow label "{ $label }" is drawn over "{ $node }".
hint-W_LABEL_OVER_NODE = Make room between the elements, shorten the label, or move the text into a note.
problem-W_LABEL_OVERLAP = The arrow labels "{ $a }" and "{ $b }" are drawn over each other.
hint-W_LABEL_OVERLAP = Spread the arrows apart (direction, order, ELK layout) or shorten one of the labels.
problem-W_EDGE_CROSSINGS = { $count } arrow crossing(s): { $pairs }.
hint-W_EDGE_CROSSINGS = Fewer crossings read better: reorder declarations, change the direction, or try arrange_diagram mode=elk; stats.crossings lets you compare versions.
summary-crossings = { $count } arrow crossing(s).
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

problem-W_FOREIGN_OBJECT = This diagram type uses HTML labels; in the PNG preview they are drawn as plain text.
hint-W_FOREIGN_OBJECT = The SVG preview and the desktop viewer show them exactly.

problem-W_RASTER = The PNG preview could not be produced: { $detail }
hint-W_RASTER = Use the SVG preview instead.

problem-W_LAYOUT_MODE = Unknown layout mode "{ $mode }".
hint-W_LAYOUT_MODE = Use one of: { $modes }.

problem-W_LAYOUT_UNKNOWN_ID = The layout comment refers to "{ $id }", which is not in the diagram.
hint-W_LAYOUT_UNKNOWN_ID = Fix the id or remove the `%% @odg` line (ids are case-sensitive).

problem-W_LAYOUT_IGNORED = Node positions are ignored in layout mode "{ $mode }".
hint-W_LAYOUT_IGNORED = Add `%% @odg layout elk-interactive` (positions as hints) or `%% @odg layout exact` (exact positions).

problem-W_LAYOUT_UNSUPPORTED = Layout comments have no effect on { $type } diagrams: ELK only lays out graph diagrams, so Mermaid's own layout is used.
hint-W_LAYOUT_UNSUPPORTED = ELK, dragging and exact positions work on flowchart, state, class, ER and requirement diagrams; remove the `%% @odg` lines here.

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
ui-new-diagram-name = File name (e.g. checkout)
ui-new-diagram-project = Project (required)
ui-new-diagram-project-placeholder = Project name
ui-new-diagram-title = Diagram title
ui-new-diagram-description = Description
ui-properties = Diagram properties
ui-diagram-title = Title
ui-diagram-description = Description
ui-diagram-project = Project (required)
ui-properties-saved = Properties updated
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
ui-mcp-installed = Installed into { $app } ({ $path }) and copied to the clipboard.
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
ui-mcp-need-folder = Open the diagrams folder first: the MCP configuration points to it. Open a folder now?
ui-mcp-copy-again = Copy again
ui-mcp-next-step = Next step:
ui-mcp-howto-claude-code = Paste it in a terminal (PowerShell, Command Prompt or Terminal) and press Enter. Then restart Claude Code and check with /mcp.
ui-mcp-howto-codex = Paste it into Codex's configuration file: ~/.codex/config.toml (on Windows: %USERPROFILE%\.codex\config.toml). Then restart Codex.
ui-mcp-howto-cursor = In Cursor: Settings → MCP → Add new MCP server, or paste into ~/.cursor/mcp.json (on Windows: %USERPROFILE%\.cursor\mcp.json). If the file already has "mcpServers", merge only the "odiagramador" block.
ui-mcp-howto-claude-desktop = In Claude Desktop: Settings → Developer → Edit Config. Paste into claude_desktop_config.json (on Windows: %APPDATA%\Claude\claude_desktop_config.json); if it already has "mcpServers", merge only the "odiagramador" block. Then restart Claude Desktop.
ui-mcp-howto-kiro = In Kiro: paste into ~/.kiro/settings/mcp.json (on Windows: %USERPROFILE%\.kiro\settings\mcp.json); if it already has "mcpServers", merge only the "odiagramador" block. Then restart Kiro.
ui-mcp-howto-antigravity = In Antigravity: paste into ~/.gemini/antigravity/mcp_config.json (or on Windows: %USERPROFILE%\.gemini\config\mcp_config.json); if it already has "mcpServers", merge only the "odiagramador" block.
ui-mcp-howto-json = Paste it into your client's MCP configuration ("mcpServers" format).
ui-logs = Diagnostics
ui-open-logs = Open logs folder
ui-agent = Agent
ui-agent-question = Question from the agent
ui-agent-close = Close
ui-agent-reply-placeholder = Type your answer…
ui-agent-send = Send
ui-agent-sent = Answer sent
ui-feedback-context = Context sent to the agent
ui-feedback-context-include = Send context
ui-feedback-context-reset = Regenerate
ui-feedback-context-hint = Built from the open diagram and the selected element; you can edit it before sending.
ui-mode-edit = Edit
ui-mode-edit-tip = Edit mode: palette, inspector, dragging and typing in the source
ui-mode-view = View
ui-mode-view-tip = View mode: look and select only (nothing changes)
ui-undo = Undo
ui-undo-tip = Undo the last change to the diagram (⌘Z / Ctrl+Z)
ui-redo = Redo
ui-redo-tip = Redo (⇧⌘Z / Ctrl+Y)
ui-problems-help = Syntax errors and visual warnings for this diagram, checked on every change. Each item's number is the same marker on the preview; click to go to the line and highlight the element.
ui-feedback-help = Ask the AI agent for changes: the comment goes with the context below, the agent reads it through MCP and marks it resolved when done.
ui-activity-help = The folder's history: who changed what and when — you, an agent (✦) or the command line — and whether the diagram stayed valid.
ui-split-code-tip = Drag to resize · double-click (or click the arrow) to collapse or show the code
ui-split-bottom-tip = Drag to resize · double-click (or click the arrow) to collapse or show the bottom panel
ui-m-rename = Rename…
ui-m-edit-text = Edit text…
ui-m-shape = Shape
ui-m-color = Colour
ui-m-no-color = No colour
ui-m-connect = Connect to
ui-m-properties = Properties…
ui-m-connections = connections
ui-m-add-attribute = Add attribute…
ui-m-add-operation = Add operation…
ui-m-attribute-example = +String name
ui-m-operation-example = +total(): Money
ui-m-member-hint = Visibility: + public · - private · # protected · ~ package. End with $ for static, * for abstract. Enter adds, Esc cancels.

## Renderer chip and help (render-kit/renderers.js)
ui-r-tip = Renderers — what they are and which one to use
ui-r-title = Renderers
ui-r-agents = Images sent to agents (MCP) and exports use the same renderer and layout you see here.
ui-r-in-use = in use
ui-r-now = This diagram uses:
ui-r-compare = Comparison
ui-r-renderer = Renderer
ui-r-close = Close
ui-r-what-title = What is a renderer?
ui-r-what-text = Turning the text of a diagram into a picture takes two steps:
ui-r-step1-title = 1 · Layout
ui-r-step1 = decides where each box, arrow and label goes — like arranging furniture in a room before decorating it.
ui-r-step2-title = 2 · Drawing
ui-r-step2 = paints the shapes, texts and arrows in those places. Mermaid always does this part.
ui-r-what-end = So what changes between the renderers below is mostly the layout: the same diagram can come out arranged in different ways.
ui-r-choose-title = Which one should I use?
ui-r-choose-dagre = A small or medium diagram that should simply look good → Automatic (dagre).
ui-r-choose-elk = Many arrows crossing, or a big diagram that looks messy → ELK.
ui-r-choose-elk-interactive = You want to drag a few boxes and keep them where you put them → ELK interactive.
ui-r-choose-exact = The drawing must come out identical every time (documentation, slides) → Exact positions.
ui-r-choose-other = Sequence, Gantt, charts and other non-graph types: nothing to choose — Mermaid uses the layout made for that type.
ui-r-howto-title = How do I switch?
ui-r-howto = In Edit mode, use the Layout box in this toolbar (it appears for flowchart, state, class, ER and requirement diagrams). The choice is saved in the code itself, as a line like %% @odg layout elk, so agents and other people see the same result. Dragging a box switches to ELK interactive by itself.
ui-r-details-title = The renderers in detail
ui-r-where = Where
ui-r-drag = Drag nodes
ui-r-positions = Keeps positions
ui-r-layers = Layer order
ui-r-best = Best for
ui-r-yes = yes
ui-r-no = no
ui-r-move = switches to ELK interactive
ui-r-fallback = This diagram type has no ELK layout: Mermaid's own layout was used.
ui-r-chip-dagre = Mermaid · dagre
ui-r-chip-elk = ELK
ui-r-chip-elk-interactive = ELK interactive
ui-r-chip-exact = Exact positions
ui-r-chip-mermaid = Mermaid · own layout
ui-r-chip-zenuml = ZenUML
ui-r-name-dagre = Mermaid · dagre (Auto)
ui-r-where-dagre = Default for graph diagrams: flowchart, state, class, ER, requirement.
ui-r-what-dagre = Fast, compact layered layout. Positions are not kept: every change recomputes the whole drawing. Dragging a node in Edit mode switches the diagram to ELK interactive.
ui-r-best-dagre = small and medium diagrams
ui-r-name-elk = ELK (Eclipse Layout Kernel)
ui-r-where-elk = Graph diagrams, when chosen (directive %% @odg layout elk).
ui-r-what-elk = More careful layered layout for large diagrams: fewer crossings, orthogonal arrows, tidy subgraphs. Accepts a layer order (%% @odg layer id first|last|n).
ui-r-best-elk = large graphs, fewer crossings
ui-r-name-elk-interactive = ELK interactive
ui-r-where-elk-interactive = Graph diagrams; turned on by dragging a node or by %% @odg layout elk-interactive.
ui-r-what-elk-interactive = ELK that respects the positions you dragged (%% @odg pos id x,y) and arranges everything else around them.
ui-r-best-elk-interactive = hand adjustments on top of the automatic layout
ui-r-name-exact = Exact positions
ui-r-where-exact = Graph diagrams with %% @odg layout exact.
ui-r-what-exact = Every node stays exactly at its saved coordinates; only the arrows are re-routed. Nodes without a position go to the end.
ui-r-best-exact = a drawing that must not move
ui-r-name-mermaid = Mermaid · own layout of the type
ui-r-where-mermaid = Sequence, Gantt, timeline, charts, mind maps, C4, architecture, block and every other non-graph type.
ui-r-what-mermaid = Each type has its own drawing algorithm in Mermaid. The layout box is hidden, elements cannot be dragged and layout directives are ignored (with a warning).
ui-r-best-mermaid = non-graph diagrams
ui-r-name-zenuml = ZenUML
ui-r-where-zenuml = zenuml diagrams (sequence written as code).
ui-r-what-zenuml = ZenUML's own renderer, plugged into Mermaid. It draws in HTML; PNG export uses a compatibility mode.
ui-r-best-zenuml = sequence diagrams written like code

## Diagram list: type, grouping and sorting
ui-group-by = Group the list
ui-group-project = By project
ui-group-none = No groups
ui-group-kind = By type
ui-group-folder = By folder
ui-no-project = No project
ui-sort-by = Sort the list
ui-sort-name = Name (A–Z)
ui-sort-date = Date (newest)
ui-root-folder = Root folder
ui-kind-unknown = Unrecognised type
ui-bottom-panel = Problems · Comments · Activity
ui-split-code-show = The code is collapsed — click (or drag) to show it
ui-split-bottom-show = The bottom panel is collapsed — click (or drag) to show it
ui-split-list-show = The diagram list is collapsed — click to show it
ui-split-list-hide = Collapse the diagram list (a strip on the left brings it back)

## UML class diagram toolbox
ui-c-abstract = Abstract class
ui-c-interface = Interface
ui-c-enum = Enumeration
ui-c-package = Package
ui-c-rel-link = Association
ui-c-rel-association = Directed association (navigable)
ui-c-rel-inheritance = Generalization (inheritance)
ui-c-rel-realization = Realization (implements an interface)
ui-c-rel-dependency = Dependency
ui-c-rel-aggregation = Aggregation
ui-c-rel-composition = Composition
ui-c-rel-lollipop = Provided interface (ball)
ui-f-general = General class (parent)
ui-f-specific = Specific class (child)
ui-f-implementer = Class that implements
ui-f-interface = Interface
ui-f-client = Class that depends (client)
ui-f-supplier = Class it uses (supplier)
ui-f-whole = Whole
ui-f-part = Part
ui-f-provider = Class that provides it
ui-f-iface-name = Interface name
ui-f-navigable-to = To (navigable end)
ui-f-literals = Literals (one per line)
ui-f-operations = Operations
ui-f-classes-in = Existing classes
ui-f-classes-new = New classes (one per line)
ui-h-abstract = A class that cannot be instantiated; it exists to be specialized. Mark abstract operations with * at the end: +area() double*
ui-h-interface = A contract: only operations, no state. Classes realize it (dashed line with a hollow triangle).
ui-h-enum = A type with a fixed list of values (literals), e.g. order status.
ui-h-package = Groups related classes under a name (UML package; Mermaid draws it as a namespace).
ui-h-link = Plain solid line: the two classes are related. Multiplicities say how many on each side (1, 0..1, *, 1..*).
ui-h-association = Solid line with an open arrow: from the first class you can reach the second (navigability).
ui-h-inheritance = Solid line with a hollow triangle pointing at the general class: the child inherits its attributes and operations ("is a").
ui-h-realization = Dashed line with a hollow triangle pointing at the interface: the class implements its operations.
ui-h-dependency = Dashed line with an open arrow: the client uses the supplier for a while (parameter, local variable, «use»).
ui-h-aggregation = Hollow diamond on the whole: the part can exist on its own and be shared ("has a").
ui-h-composition = Filled diamond on the whole: the part belongs to a single whole and dies with it.
ui-h-lollipop = UML "ball" notation: the class offers this interface to others.
