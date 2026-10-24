/* ODiagramador render-kit: which renderer drew the diagram (chip in the bottom toolbar) and a modal
   explaining every renderer — where it is used, what it does and how they differ. Shared by every desktop app. */
(function () {
  "use strict";

  // id → [drag, keeps positions, layer order]; "move" = dragging switches the diagram to ELK interactive
  var RENDERERS = [
    { id: "dagre", drag: "move", pos: false, layers: false },
    { id: "elk", drag: "move", pos: false, layers: true },
    { id: "elk-interactive", drag: true, pos: true, layers: true },
    { id: "exact", drag: true, pos: true, layers: false },
    { id: "mermaid", drag: false, pos: false, layers: false },
    { id: "zenuml", drag: false, pos: false, layers: false }
  ];

  var T = {
    "ui-r-tip": "Renderer in use — click for help",
    "ui-r-title": "Renderers",
    "ui-r-intro": "Mermaid always draws the diagram; what changes is who places the elements (the layout). Flowcharts, state, class, ER and requirement diagrams let you choose the layout in the box beside this button; every other type uses Mermaid's own layout for that type.",
    "ui-r-agents": "Images sent to agents (MCP) and exports use the same renderer and layout you see here.",
    "ui-r-in-use": "in use",
    "ui-r-now": "This diagram uses:",
    "ui-r-compare": "Comparison",
    "ui-r-renderer": "Renderer",
    "ui-r-close": "Close",
    "ui-r-where": "Where",
    "ui-r-drag": "Drag nodes",
    "ui-r-positions": "Keeps positions",
    "ui-r-layers": "Layer order",
    "ui-r-best": "Best for",
    "ui-r-yes": "yes",
    "ui-r-no": "no",
    "ui-r-move": "switches to ELK interactive",
    "ui-r-fallback": "This diagram type has no ELK layout: Mermaid's own layout was used.",
    "ui-r-chip-dagre": "Mermaid · dagre",
    "ui-r-chip-elk": "ELK",
    "ui-r-chip-elk-interactive": "ELK interactive",
    "ui-r-chip-exact": "Exact positions",
    "ui-r-chip-mermaid": "Mermaid · own layout",
    "ui-r-chip-zenuml": "ZenUML",
    "ui-r-name-dagre": "Mermaid · dagre (Auto)",
    "ui-r-where-dagre": "Default for graph diagrams: flowchart, state, class, ER, requirement.",
    "ui-r-what-dagre": "Fast, compact layered layout. Positions are not kept: every change recomputes the whole drawing. Dragging a node in Edit mode switches the diagram to ELK interactive.",
    "ui-r-best-dagre": "small and medium diagrams",
    "ui-r-name-elk": "ELK (Eclipse Layout Kernel)",
    "ui-r-where-elk": "Graph diagrams, when chosen (directive %% @odg layout elk).",
    "ui-r-what-elk": "More careful layered layout for large diagrams: fewer crossings, orthogonal arrows, tidy subgraphs. Accepts a layer order (%% @odg layer id first|last|n).",
    "ui-r-best-elk": "large graphs, fewer crossings",
    "ui-r-name-elk-interactive": "ELK interactive",
    "ui-r-where-elk-interactive": "Graph diagrams; turned on by dragging a node or by %% @odg layout elk-interactive.",
    "ui-r-what-elk-interactive": "ELK that respects the positions you dragged (%% @odg pos id x,y) and arranges everything else around them.",
    "ui-r-best-elk-interactive": "hand adjustments on top of the automatic layout",
    "ui-r-name-exact": "Exact positions",
    "ui-r-where-exact": "Graph diagrams with %% @odg layout exact.",
    "ui-r-what-exact": "Every node stays exactly at its saved coordinates; only the arrows are re-routed. Nodes without a position go to the end.",
    "ui-r-best-exact": "a drawing that must not move",
    "ui-r-name-mermaid": "Mermaid · own layout of the type",
    "ui-r-where-mermaid": "Sequence, Gantt, timeline, charts, mind maps, C4, architecture, block and every other non-graph type.",
    "ui-r-what-mermaid": "Each type has its own drawing algorithm in Mermaid. The layout box is hidden, elements cannot be dragged and layout directives are ignored (with a warning).",
    "ui-r-best-mermaid": "non-graph diagrams",
    "ui-r-name-zenuml": "ZenUML",
    "ui-r-where-zenuml": "zenuml diagrams (sequence written as code).",
    "ui-r-what-zenuml": "ZenUML's own renderer, plugged into Mermaid. It draws in HTML; PNG export uses a compatibility mode.",
    "ui-r-best-zenuml": "sequence diagrams written like code"
  };
  var VERSIONS = "elkjs 0.9.3 · ZenUML 1.0.1";

  var chip = document.getElementById("renderer-chip");
  var label = document.getElementById("renderer-label");
  var modal = document.getElementById("renderer-modal");
  var panel = modal && modal.querySelector(".rm-dialog");
  if (!chip || !panel) return;
  var current = { id: null, fallback: false };

  function t(k) { return T[k] || k; }

  function el(tag, cls, text) {
    var e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text != null) e.textContent = text;
    return e;
  }

  /// The renderer for a diagram type (as reported by Mermaid) and a layout mode.
  function rendererFor(type, mode, graph) {
    if (/^zenuml/i.test(type || "")) return "zenuml";
    if (!graph) return "mermaid";
    if (mode === "elk" || mode === "elk-interactive" || mode === "exact") return mode;
    return "dagre";
  }

  function update(type, mode, graph, fallback) {
    if (!type) { chip.hidden = true; return; }
    current.id = rendererFor(type, mode, graph);
    current.fallback = !!fallback;
    chip.hidden = false;
    label.textContent = t("ui-r-chip-" + current.id);
    chip.title = t("ui-r-tip");
    chip.classList.toggle("warn", current.fallback);
    if (!modal.hidden) render();
  }

  function mark(v) {
    var s = el("span", "rh-flag " + (v === true ? "yes" : v === "move" ? "move" : "no"));
    s.textContent = v === true ? "✓ " + t("ui-r-yes") : v === "move" ? "↪ " + t("ui-r-move") : "— " + t("ui-r-no");
    return s;
  }

  function render() {
    panel.innerHTML = "";
    var head = el("div", "rm-head");
    head.appendChild(el("h2", null, t("ui-r-title")));
    var x = el("button", "rm-x", "×");
    x.title = t("ui-r-close");
    x.addEventListener("click", hide);
    head.appendChild(x);
    panel.appendChild(head);

    var body = el("div", "rm-body");
    body.appendChild(el("p", "rm-intro", t("ui-r-intro")));
    if (current.id) {
      var now = el("p", "rm-now");
      now.appendChild(document.createTextNode(t("ui-r-now") + " "));
      now.appendChild(el("b", null, t("ui-r-name-" + current.id)));
      body.appendChild(now);
    }
    if (current.fallback) body.appendChild(el("p", "rm-warn", t("ui-r-fallback")));

    // one card per renderer: where it is used and what it does
    var cards = el("div", "rm-cards");
    RENDERERS.forEach(function (r) {
      var card = el("section", "rm-card" + (r.id === current.id ? " active" : ""));
      var title = el("div", "rm-name", t("ui-r-name-" + r.id));
      if (r.id === current.id) title.appendChild(el("span", "rm-badge", t("ui-r-in-use")));
      card.appendChild(title);
      var where = el("div", "rm-where");
      where.appendChild(el("b", null, t("ui-r-where") + ": "));
      where.appendChild(document.createTextNode(t("ui-r-where-" + r.id)));
      card.appendChild(where);
      card.appendChild(el("div", "rm-what", t("ui-r-what-" + r.id)));
      cards.appendChild(card);
    });
    body.appendChild(cards);

    // how they differ, side by side
    body.appendChild(el("h3", null, t("ui-r-compare")));
    var wrap = el("div", "rm-table-wrap");
    var table = el("table", "rm-table");
    var hr = el("tr");
    ["ui-r-renderer", "ui-r-drag", "ui-r-positions", "ui-r-layers", "ui-r-best"].forEach(function (k) { hr.appendChild(el("th", null, t(k))); });
    table.appendChild(el("thead")).appendChild(hr);
    var tb = el("tbody");
    RENDERERS.forEach(function (r) {
      var tr = el("tr", r.id === current.id ? "active" : "");
      tr.appendChild(el("td", "rm-rname", t("ui-r-chip-" + r.id)));
      [r.drag, r.pos, r.layers].forEach(function (v) { tr.appendChild(el("td")).appendChild(mark(v)); });
      tr.appendChild(el("td", null, t("ui-r-best-" + r.id)));
      tb.appendChild(tr);
    });
    table.appendChild(tb);
    wrap.appendChild(table);
    body.appendChild(wrap);
    body.appendChild(el("p", "rm-foot", t("ui-r-agents")));
    panel.appendChild(body);

    var foot = el("div", "rm-actions");
    foot.appendChild(el("span", "rm-versions", "Mermaid " + ((window.mermaid && mermaid.version) || "") + " · " + VERSIONS));
    var ok = el("button", "rm-ok", t("ui-r-close"));
    ok.addEventListener("click", hide);
    foot.appendChild(ok);
    panel.appendChild(foot);
  }

  function show() {
    render();
    modal.hidden = false;
    chip.classList.add("open");
    var ok = panel.querySelector(".rm-ok");
    if (ok) ok.focus();
  }
  function hide() {
    modal.hidden = true;
    chip.classList.remove("open");
  }

  chip.addEventListener("click", function (e) {
    e.stopPropagation();
    if (modal.hidden) show(); else hide();
  });
  // a click on the dimmed backdrop (outside the dialog) closes it
  modal.addEventListener("mousedown", function (e) { if (e.target === modal) hide(); });
  document.addEventListener("keydown", function (e) {
    if (e.key === "Escape" && !modal.hidden) { e.stopPropagation(); hide(); }
  }, true);

  window.odgRenderers = {
    update: update,
    strings: function (s) {
      Object.keys(s || {}).forEach(function (k) { if (/^ui-r-/.test(k)) T[k] = s[k]; });
      if (current.id) label.textContent = t("ui-r-chip-" + current.id);
      chip.title = t("ui-r-tip");
      if (!modal.hidden) render();
    },
    hide: hide
  };
})();
