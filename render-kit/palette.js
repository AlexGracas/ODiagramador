/*
 * ODiagramador constructor palette (viewer mode only).
 *
 * Shows the constructors of the open diagram type plus a direction picker. Each button opens a
 * small form; submitting posts { op: "construct", construct, params } to the host, which asks the
 * Rust core (odg-core/src/construct.rs) for the new source, saves it and re-renders.
 */
(function () {
  "use strict";
  if (new URLSearchParams(location.search).get("mode") !== "viewer") return;

  // English defaults; hosts send localized "ui-c-*" strings.
  var T = {
    "ui-c-direction": "Direction",
    "ui-c-node": "Node", "ui-c-decision": "Decision", "ui-c-startend": "Start/End", "ui-c-edge": "Edge", "ui-c-subgraph": "Group",
    "ui-c-participant": "Participant", "ui-c-actor": "Actor", "ui-c-message": "Message", "ui-c-block": "Block", "ui-c-note": "Note", "ui-c-autonumber": "Numbering",
    "ui-c-class": "Class", "ui-c-relation": "Relation",
    "ui-c-state": "State", "ui-c-transition": "Transition", "ui-c-start": "Start", "ui-c-end": "End", "ui-c-choice": "Choice",
    "ui-c-entity": "Entity",
    "ui-f-label": "Text", "ui-f-shape": "Shape", "ui-f-connect-from": "Connect from", "ui-f-edge-label": "Edge text",
    "ui-f-from": "From", "ui-f-to": "To", "ui-f-style": "Line", "ui-f-title": "Title", "ui-f-members": "Members",
    "ui-f-type": "Type", "ui-f-condition": "Condition", "ui-f-position": "Position", "ui-f-other": "Also over",
    "ui-f-name": "Name", "ui-f-stereotype": "Stereotype", "ui-f-attributes": "Attributes (one per line)", "ui-f-methods": "Methods (one per line)",
    "ui-f-from-card": "Cardinality (from)", "ui-f-to-card": "Cardinality (to)", "ui-f-for": "Attached to",
    "ui-f-add": "Add", "ui-f-cancel": "Cancel", "ui-f-none": "(none)",
    "ui-o-rect": "Rectangle", "ui-o-round": "Rounded", "ui-o-stadium": "Stadium", "ui-o-decision": "Decision", "ui-o-database": "Database",
    "ui-o-circle": "Circle", "ui-o-subroutine": "Subroutine", "ui-o-io": "Input/Output", "ui-o-hexagon": "Hexagon", "ui-o-flag": "Flag",
    "ui-o-arrow": "Arrow", "ui-o-open": "Line", "ui-o-dotted": "Dotted", "ui-o-thick": "Thick", "ui-o-both": "Both ways",
    "ui-o-sync": "Request (->>)", "ui-o-reply": "Reply (-->>)", "ui-o-async": "Async (-))", "ui-o-lost": "Lost (-x)",
    "ui-o-right": "Right of", "ui-o-left": "Left of", "ui-o-over": "Over",
    "ui-o-inheritance": "Inheritance", "ui-o-composition": "Composition", "ui-o-aggregation": "Aggregation", "ui-o-association": "Association",
    "ui-o-dependency": "Dependency", "ui-o-realization": "Realization",
    "ui-o-one": "Exactly one", "ui-o-zero_one": "Zero or one", "ui-o-zero_many": "Zero or many", "ui-o-one_many": "One or many",
    "ui-m-rename": "Rename…", "ui-m-edit-text": "Edit text…", "ui-m-shape": "Shape", "ui-m-color": "Colour", "ui-m-no-color": "No colour",
    "ui-m-connect": "Connect to", "ui-m-properties": "Properties…", "ui-m-connections": "connections",
    "ui-i-node": "Edit element", "ui-i-edge": "Edit connection", "ui-i-save": "Save", "ui-i-delete": "Delete", "ui-i-confirm": "Click again to delete",
    "ui-dir-TB": "Top → bottom (TB)", "ui-dir-BT": "Bottom → top (BT)", "ui-dir-LR": "Left → right (LR)", "ui-dir-RL": "Right → left (RL)"
  };
  function t(k) { return T[k] || k; }

  var SHAPES = ["rect", "round", "stadium", "decision", "database", "circle", "subroutine", "io", "hexagon"];
  var EDGE_STYLES = ["arrow", "open", "dotted", "thick", "both"];
  var CARD = ["one", "zero_one", "zero_many", "one_many"];

  function f(k, type, label, extra) {
    var o = { k: k, type: type, label: label };
    for (var x in extra || {}) o[x] = extra[x];
    return o;
  }
  function opts(values, prefix) { return values.map(function (v) { return [v, prefix + v]; }); }

  // id, icon, label key, fields, preset params
  var PALETTE = {
    flowchart: [
      { id: "flow.node", icon: "▭", label: "ui-c-node", fields: [
        f("label", "text", "ui-f-label", { required: true }),
        f("shape", "select", "ui-f-shape", { options: opts(SHAPES, "ui-o-") }),
        f("from", "node", "ui-f-connect-from", { optional: true, selected: true }),
        f("edge_label", "text", "ui-f-edge-label")] },
      { id: "flow.node", icon: "◇", label: "ui-c-decision", preset: { shape: "decision" }, fields: [
        f("label", "text", "ui-f-label", { required: true }),
        f("from", "node", "ui-f-connect-from", { optional: true, selected: true }),
        f("edge_label", "text", "ui-f-edge-label")] },
      { id: "flow.node", icon: "⬭", label: "ui-c-startend", preset: { shape: "stadium" }, fields: [
        f("label", "text", "ui-f-label", { required: true }),
        f("from", "node", "ui-f-connect-from", { optional: true, selected: true })] },
      { id: "flow.edge", icon: "→", label: "ui-c-edge", fields: [
        f("from", "node", "ui-f-from", { selected: true }),
        f("to", "node", "ui-f-to"),
        f("label", "text", "ui-f-label"),
        f("style", "select", "ui-f-style", { options: opts(EDGE_STYLES, "ui-o-") })] },
      { id: "flow.subgraph", icon: "▢", label: "ui-c-subgraph", fields: [
        f("title", "text", "ui-f-title", { required: true }),
        f("members", "nodes", "ui-f-members")] }
    ],
    sequence: [
      { id: "seq.participant", icon: "▯", label: "ui-c-participant", fields: [f("label", "text", "ui-f-name", { required: true })] },
      { id: "seq.participant", icon: "☺", label: "ui-c-actor", preset: { actor: "true" }, fields: [f("label", "text", "ui-f-name", { required: true })] },
      { id: "seq.message", icon: "→", label: "ui-c-message", fields: [
        f("from", "node", "ui-f-from", { selected: true }),
        f("to", "node", "ui-f-to"),
        f("text", "text", "ui-f-label", { required: true }),
        f("type", "select", "ui-f-type", { options: opts(["sync", "reply", "async", "lost"], "ui-o-") })] },
      { id: "seq.block", icon: "⧉", label: "ui-c-block", fields: [
        f("block", "select", "ui-f-type", { options: [["alt", "alt / else"], ["opt", "opt"], ["loop", "loop"], ["par", "par"], ["critical", "critical"], ["break", "break"]] }),
        f("condition", "text", "ui-f-condition", { required: true }),
        f("from", "node", "ui-f-from", { selected: true }),
        f("to", "node", "ui-f-to")] },
      { id: "seq.note", icon: "✎", label: "ui-c-note", fields: [
        f("participant", "node", "ui-f-for", { selected: true }),
        f("position", "select", "ui-f-position", { options: opts(["right", "left", "over"], "ui-o-") }),
        f("other", "node", "ui-f-other", { optional: true }),
        f("text", "text", "ui-f-label", { required: true })] },
      { id: "seq.autonumber", icon: "#", label: "ui-c-autonumber", fields: [] }
    ],
    class: [
      { id: "class.class", icon: "▤", label: "ui-c-class", fields: [
        f("name", "text", "ui-f-name", { required: true }),
        f("stereotype", "select", "ui-f-stereotype", { options: [["", "ui-f-none"], ["interface", "interface"], ["abstract", "abstract"], ["enumeration", "enumeration"], ["service", "service"]] }),
        f("attributes", "lines", "ui-f-attributes", { placeholder: "+String name\n-int age" }),
        f("methods", "lines", "ui-f-methods", { placeholder: "+total() Money" })] },
      { id: "class.relation", icon: "→", label: "ui-c-relation", fields: [
        f("from", "node", "ui-f-from", { selected: true }),
        f("to", "node", "ui-f-to"),
        f("type", "select", "ui-f-type", { options: opts(["association", "inheritance", "composition", "aggregation", "dependency", "realization"], "ui-o-") }),
        f("from_card", "text", "ui-f-from-card", { placeholder: "1" }),
        f("to_card", "text", "ui-f-to-card", { placeholder: "*" }),
        f("label", "text", "ui-f-label")] },
      { id: "class.note", icon: "✎", label: "ui-c-note", fields: [
        f("for", "node", "ui-f-for", { optional: true, selected: true }),
        f("text", "text", "ui-f-label", { required: true })] }
    ],
    state: [
      { id: "state.state", icon: "▭", label: "ui-c-state", fields: [f("label", "text", "ui-f-name", { required: true })] },
      { id: "state.transition", icon: "→", label: "ui-c-transition", fields: [
        f("from", "node", "ui-f-from", { selected: true }),
        f("to", "node", "ui-f-to"),
        f("label", "text", "ui-f-label")] },
      { id: "state.start", icon: "●", label: "ui-c-start", fields: [f("to", "node", "ui-f-to", { selected: true })] },
      { id: "state.end", icon: "◉", label: "ui-c-end", fields: [f("from", "node", "ui-f-from", { selected: true })] },
      { id: "state.choice", icon: "◇", label: "ui-c-choice", fields: [f("label", "text", "ui-f-name")] },
      { id: "state.note", icon: "✎", label: "ui-c-note", fields: [
        f("state", "node", "ui-f-for", { selected: true }),
        f("position", "select", "ui-f-position", { options: opts(["right", "left"], "ui-o-") }),
        f("text", "text", "ui-f-label", { required: true })] }
    ],
    er: [
      { id: "er.entity", icon: "▤", label: "ui-c-entity", fields: [
        f("name", "text", "ui-f-name", { required: true }),
        f("attributes", "lines", "ui-f-attributes", { placeholder: "int id PK\nstring name" })] },
      { id: "er.relation", icon: "⟷", label: "ui-c-relation", fields: [
        f("from", "node", "ui-f-from", { selected: true }),
        f("to", "node", "ui-f-to"),
        f("from_card", "select", "ui-f-from-card", { options: opts(CARD, "ui-o-") }),
        f("to_card", "select", "ui-f-to-card", { options: opts(["zero_many", "one_many", "one", "zero_one"], "ui-o-") }),
        f("label", "text", "ui-f-label", { required: true })] }
    ]
  };

  var state = { kind: null, nodes: [], edges: [], groups: [], selected: null, direction: null, open: null };

  // Mermaid's internal shape names → the constructor shape keys
  var SHAPE_OF = {
    squareRect: "rect", rect: "rect", rounded: "round", stadium: "stadium", diamond: "decision", question: "decision",
    cylinder: "database", circle: "circle", doublecircle: "circle", subroutine: "subroutine", lean_r: "io", lean_l: "io",
    hexagon: "hexagon", odd: "flag"
  };
  var bar = document.getElementById("palette");
  var ctors = document.getElementById("pal-ctors"); // creation buttons (the bar also holds mode and undo/redo)
  var ctorSep = document.getElementById("pal-ctor-sep");
  var form = document.getElementById("ctor-form");

  function kindOf(type) {
    if (!type) return null;
    if (/^(flowchart|graph)/.test(type)) return "flowchart";
    if (/^sequence/.test(type)) return "sequence";
    if (/^class/.test(type)) return "class";
    if (/^state/.test(type)) return "state";
    if (/^er/.test(type)) return "er";
    return null;
  }

  function directionOf(source, kind) {
    var lines = String(source || "").split("\n");
    if (kind === "flowchart") {
      for (var i = 0; i < lines.length; i++) {
        var m = lines[i].match(/^\s*(flowchart|graph)\s+(TB|TD|BT|LR|RL)\b/);
        if (m) return m[2] === "TD" ? "TB" : m[2];
        if (/^\s*(flowchart|graph)\s*$/.test(lines[i])) return "TB";
      }
      return "TB";
    }
    for (var j = 0; j < lines.length; j++) {
      var d = lines[j].match(/^\s*direction\s+(TB|TD|BT|LR|RL)\b/);
      if (d) return d[1] === "TD" ? "TB" : d[1];
    }
    return "TB";
  }

  function el(tag, attrs, text) {
    var e = document.createElement(tag);
    for (var k in attrs || {}) {
      if (k === "class") e.className = attrs[k]; else e.setAttribute(k, attrs[k]);
    }
    if (text != null) e.textContent = text;
    return e;
  }

  function renderBar() {
    ctors.innerHTML = "";
    var items = PALETTE[state.kind];
    ctors.hidden = !items;
    ctorSep.hidden = !items;
    document.body.classList.add("has-palette");
    requestAnimationFrame(function () {
      document.documentElement.style.setProperty("--palette-h", bar.getBoundingClientRect().height + "px");
    });
    if (!items) return;
    if (state.kind !== "sequence") {
      var label = el("label", { class: "pal-dir" });
      label.appendChild(el("span", null, t("ui-c-direction")));
      var sel = el("select", { id: "pal-direction" });
      ["TB", "BT", "LR", "RL"].forEach(function (d) {
        var o = el("option", { value: d }, t("ui-dir-" + d));
        if (d === state.direction) o.selected = true;
        sel.appendChild(o);
      });
      sel.addEventListener("change", function () {
        window.odg.post({ op: "construct", construct: "direction", params: { dir: sel.value } });
      });
      label.appendChild(sel);
      ctors.appendChild(label);
      ctors.appendChild(el("span", { class: "pal-sep" }));
    }
    items.forEach(function (item, i) {
      var b = el("button", { class: "pal-btn", "data-i": String(i), title: t(item.label) });
      b.appendChild(el("span", { class: "pal-icon" }, item.icon));
      b.appendChild(el("span", null, t(item.label)));
      b.addEventListener("click", function () {
        if (!item.fields.length) return submit(item, {});
        openForm(item, b);
      });
      ctors.appendChild(b);
    });
  }

  function nodeOptions(select, field) {
    if (field.optional) select.appendChild(el("option", { value: "" }, t("ui-f-none")));
    state.nodes.forEach(function (n) {
      var text = n.label && n.label !== n.id ? n.label + " (" + n.id + ")" : n.id;
      var o = el("option", { value: n.id }, text);
      select.appendChild(o);
    });
    if (field.selected && state.selected && state.nodes.some(function (n) { return n.id === state.selected; })) {
      select.value = state.selected;
    } else if (!field.optional && field.k === "to" && state.nodes.length > 1) {
      select.selectedIndex = 1;
    }
  }

  function openForm(item, anchor) {
    form.innerHTML = "";
    state.open = item;
    var title = el("div", { class: "ctor-title" }, item.icon + "  " + t(item.label));
    form.appendChild(title);
    var inputs = {};
    item.fields.forEach(function (field) {
      var row = el("label", { class: "ctor-row" });
      row.appendChild(el("span", null, t(field.label)));
      var input;
      if (field.type === "select") {
        input = el("select");
        field.options.forEach(function (o) { input.appendChild(el("option", { value: o[0] }, t(o[1]))); });
      } else if (field.type === "node") {
        input = el("select");
        nodeOptions(input, field);
      } else if (field.type === "nodes") {
        input = el("select", { multiple: "multiple", size: String(Math.min(6, Math.max(2, state.nodes.length))) });
        nodeOptions(input, { optional: false });
        if (state.selected) Array.prototype.forEach.call(input.options, function (o) { o.selected = o.value === state.selected; });
      } else if (field.type === "lines") {
        input = el("textarea", { rows: "3", placeholder: field.placeholder || "" });
      } else {
        input = el("input", { type: "text", placeholder: field.placeholder || "" });
      }
      input.setAttribute("data-k", field.k);
      inputs[field.k] = { input: input, field: field };
      row.appendChild(input);
      form.appendChild(row);
    });
    var actions = el("div", { class: "ctor-actions" });
    var cancel = el("button", { type: "button" }, t("ui-f-cancel"));
    var ok = el("button", { type: "button", class: "primary" }, t("ui-f-add"));
    cancel.addEventListener("click", closeForm);
    ok.addEventListener("click", function () { collect(item, inputs); });
    actions.appendChild(cancel);
    actions.appendChild(ok);
    form.appendChild(actions);
    form.hidden = false;
    var r = anchor.getBoundingClientRect();
    form.style.top = bar.getBoundingClientRect().bottom + 6 + "px";
    form.style.left = Math.max(8, Math.min(r.left, window.innerWidth - form.offsetWidth - 8)) + "px";
    var first = form.querySelector("input, textarea, select");
    if (first) first.focus();
    form.onkeydown = function (e) {
      if (e.key === "Escape") closeForm();
      if (e.key === "Enter" && e.target.tagName !== "TEXTAREA") { e.preventDefault(); collect(item, inputs); }
    };
  }

  function collect(item, inputs) {
    var params = {};
    var missing = false;
    Object.keys(inputs).forEach(function (k) {
      var x = inputs[k];
      var v;
      if (x.field.type === "nodes") {
        v = Array.prototype.filter.call(x.input.options, function (o) { return o.selected; }).map(function (o) { return o.value; });
      } else {
        v = x.input.value;
      }
      x.input.classList.remove("invalid");
      if (x.field.required && !String(v).trim()) {
        x.input.classList.add("invalid");
        missing = true;
      }
      if (v !== "" && !(Array.isArray(v) && !v.length)) params[k] = v;
    });
    if (missing) return;
    for (var p in item.preset || {}) params[p] = item.preset[p];
    submit(item, params);
  }

  function submit(item, params) {
    window.odg.post({ op: "construct", construct: item.id, params: params });
    closeForm();
  }

  function closeForm() {
    form.hidden = true;
    form.innerHTML = "";
    state.open = null;
  }

  // ---- inspector: edit or delete an existing element
  function inspect(target, x, y) {
    if (!state.kind || !target) return;
    form.innerHTML = "";
    state.open = target;
    var isEdge = target.type === "edge";
    var title = isEdge ? t("ui-i-edge") + "  " + target.from + " → " + target.to : t("ui-i-node") + "  " + target.id;
    form.appendChild(el("div", { class: "ctor-title" }, title));
    var labelRow = el("label", { class: "ctor-row" });
    labelRow.appendChild(el("span", null, t(isEdge ? "ui-f-label" : (state.kind === "class" || state.kind === "er" ? "ui-f-name" : "ui-f-label"))));
    var labelInput = el("input", { type: "text" });
    labelInput.value = target.label || "";
    labelRow.appendChild(labelInput);
    form.appendChild(labelRow);
    var shapeInput = null;
    if (!isEdge && state.kind === "flowchart" && !target.group) {
      var shapeRow = el("label", { class: "ctor-row" });
      shapeRow.appendChild(el("span", null, t("ui-f-shape")));
      shapeInput = el("select");
      SHAPES.concat(["flag"]).forEach(function (s) { shapeInput.appendChild(el("option", { value: s }, t("ui-o-" + s))); });
      shapeInput.value = SHAPE_OF[target.shape] || "rect";
      shapeRow.appendChild(shapeInput);
      form.appendChild(shapeRow);
    }
    var actions = el("div", { class: "ctor-actions" });
    var del = el("button", { type: "button", class: "danger" }, t("ui-i-delete"));
    var cancel = el("button", { type: "button" }, t("ui-f-cancel"));
    var save = el("button", { type: "button", class: "primary" }, t("ui-i-save"));
    // deletes at once: Undo (↶ / Cmd+Z) brings it back
    del.addEventListener("click", function () { remove(target); });
    cancel.addEventListener("click", closeForm);
    var originalShape = shapeInput ? shapeInput.value : null;
    function doSave() {
      var label = labelInput.value.trim();
      if (isEdge) {
        if (label !== (target.label || "")) return edit("edge_label", { from: target.from, to: target.to, index: target.index, label: label });
        return closeForm();
      }
      if (shapeInput && shapeInput.value !== originalShape) return edit("shape", { id: target.id, shape: shapeInput.value, label: label || target.label || target.id });
      if (label && label !== (target.label || "")) return edit("rename", { id: target.id, label: label });
      closeForm();
    }
    save.addEventListener("click", doSave);
    actions.appendChild(del);
    actions.appendChild(el("span", { style: "flex:1" }));
    actions.appendChild(cancel);
    actions.appendChild(save);
    form.appendChild(actions);
    form.hidden = false;
    var top = Math.max(bar.hidden ? 8 : bar.getBoundingClientRect().bottom + 6, Math.min(y + 12, window.innerHeight - form.offsetHeight - 8));
    form.style.top = top + "px";
    form.style.left = Math.max(8, Math.min(x - 40, window.innerWidth - form.offsetWidth - 8)) + "px";
    labelInput.focus();
    labelInput.select();
    form.onkeydown = function (e) {
      if (e.key === "Escape") closeForm();
      if (e.key === "Enter") { e.preventDefault(); doSave(); }
    };
  }

  function edit(action, params) {
    window.odg.post({ op: "edit", action: action, params: params });
    closeForm();
  }

  function targetFor(id, isEdge) {
    if (isEdge) {
      var e = state.edges.filter(function (x) { return x.id === id; })[0];
      if (!e || state.kind !== "flowchart") return null;
      // n-th connection between the same pair, in source order (Mermaid's id suffix is not that)
      var n = 0;
      for (var i = 0; i < state.edges.length && state.edges[i].id !== id; i++) {
        if (state.edges[i].from === e.from && state.edges[i].to === e.to) n++;
      }
      return { type: "edge", id: id, from: e.from, to: e.to, index: n, label: e.label || "" };
    }
    var node = state.nodes.concat(state.groups).filter(function (x) { return x.id === id; })[0];
    if (!node) return null;
    var group = state.groups.some(function (g) { return g.id === id; });
    return { type: "node", id: id, label: node.label || id, shape: node.shape, group: group };
  }

  function remove(target) {
    if (target.type === "edge") edit("edge_delete", { from: target.from, to: target.to, index: target.index });
    else edit("delete", { id: target.id });
  }

  // ---- double-click / "Rename…": just the text, in place
  function rename(target, x, y) {
    if (!target) return;
    form.innerHTML = "";
    state.open = target;
    var isEdge = target.type === "edge";
    form.appendChild(el("div", { class: "ctor-title" }, (isEdge ? t("ui-m-edit-text") : t("ui-m-rename")) + "  " + (isEdge ? target.from + " → " + target.to : target.id)));
    var input = el("input", { type: "text" });
    input.value = target.label || "";
    form.appendChild(input);
    form.hidden = false;
    form.style.top = Math.max(8, Math.min(y + 12, window.innerHeight - form.offsetHeight - 8)) + "px";
    form.style.left = Math.max(8, Math.min(x - 40, window.innerWidth - form.offsetWidth - 8)) + "px";
    input.focus();
    input.select();
    form.onkeydown = function (e) {
      if (e.key === "Escape") closeForm();
      if (e.key !== "Enter") return;
      e.preventDefault();
      var label = input.value.trim();
      if (isEdge) return label !== (target.label || "") ? edit("edge_label", { from: target.from, to: target.to, index: target.index, label: label }) : closeForm();
      if (label && label !== (target.label || "")) edit("rename", { id: target.id, label: label });
      else closeForm();
    };
  }

  // ---- right-click menu
  var menu = el("div", { id: "ctx-menu", hidden: "" });
  document.body.appendChild(menu);
  var COLOURS = [
    ["#e8f0fe", "#1a73e8"], ["#e6f4ea", "#1e8e3e"], ["#fef7e0", "#f9ab00"], ["#fce8e6", "#d93025"], ["#f3e8fd", "#9334e6"],
    ["#e0f7fa", "#00838f"], ["#fff3e0", "#ef6c00"], ["#eceff1", "#455a64"], ["#ffffff", "#202124"], ["#202124", "#000000"]
  ];

  function closeMenu() {
    menu.hidden = true;
    menu.innerHTML = "";
  }

  function menuItem(parent, label, action, opts) {
    var b = el("button", { type: "button", class: "ctx-item" + (opts && opts.danger ? " danger" : "") });
    b.appendChild(el("span", null, label));
    if (opts && opts.sub) b.appendChild(el("span", { class: "ctx-more" }, "›"));
    b.addEventListener("click", function (e) {
      e.stopPropagation();
      if (opts && opts.sub) return;
      closeMenu();
      action();
    });
    if (opts && opts.sub) {
      var sub = el("div", { id: "", class: "ctx-sub" });
      sub.setAttribute("style", "position:absolute;left:100%;top:-4px;margin-left:4px;min-width:170px;padding:4px;border:1px solid var(--border);border-radius:8px;background:var(--bg);box-shadow:0 10px 28px rgba(0,0,0,.22);display:none");
      opts.sub(sub);
      b.appendChild(sub);
      b.addEventListener("mouseenter", function () {
        Array.prototype.forEach.call(menu.querySelectorAll(".ctx-sub"), function (x) { x.style.display = "none"; });
        Array.prototype.forEach.call(menu.querySelectorAll(".ctx-item.open"), function (x) { x.classList.remove("open"); });
        sub.style.display = "block";
        sub.style.left = "100%";
        sub.style.right = "auto";
        sub.style.top = "-4px";
        b.classList.add("open");
        // no room on the right (or below): open towards the other side
        var sr = sub.getBoundingClientRect();
        if (sr.right > window.innerWidth - 8) {
          sub.style.left = "auto";
          sub.style.right = "100%";
          sub.style.marginLeft = "0";
          sub.style.marginRight = "4px";
          // neither side has room (narrow window): keep it inside the window, over the menu if needed
          var fr = sub.getBoundingClientRect();
          if (fr.left < 8) {
            var left = Math.max(8, Math.min(window.innerWidth - 8 - fr.width, fr.left));
            sub.style.right = "auto";
            sub.style.marginRight = "0";
            sub.style.left = (left - b.getBoundingClientRect().left) + "px";
          }
        }
        if (sr.bottom > window.innerHeight - 8) sub.style.top = Math.min(-4, window.innerHeight - 8 - sr.bottom) + "px";
      });
    } else {
      b.addEventListener("mouseenter", function () {
        if (b.parentNode !== menu) return;
        Array.prototype.forEach.call(menu.querySelectorAll(".ctx-sub"), function (x) { x.style.display = "none"; });
        Array.prototype.forEach.call(menu.querySelectorAll(".ctx-item.open"), function (x) { x.classList.remove("open"); });
      });
    }
    parent.appendChild(b);
    return b;
  }

  function contextMenu(id, isEdge, x, y, editing) {
    closeForm();
    menu.innerHTML = "";
    var target = targetFor(id, isEdge);
    var node = !isEdge && state.nodes.concat(state.groups).filter(function (n) { return n.id === id; })[0];
    var links = state.edges.filter(function (e) { return e.from === id || e.to === id; }).length;
    var head = isEdge ? (target ? target.from + " → " + target.to : id) : id + (node && node.label && node.label !== id ? " · " + node.label : "") + (isEdge ? "" : "  ·  " + links + " " + t("ui-m-connections"));
    menu.appendChild(el("div", { class: "ctx-head" }, head));
    if (editing && target) {
      menuItem(menu, isEdge ? t("ui-m-edit-text") : t("ui-m-rename"), function () { rename(target, x, y); });
      if (!isEdge && state.kind === "flowchart" && !target.group) {
        menuItem(menu, t("ui-m-shape"), null, { sub: function (sub) {
          SHAPES.concat(["flag"]).forEach(function (sh) {
            menuItem(sub, t("ui-o-" + sh), function () { edit("shape", { id: id, shape: sh, label: target.label || id }); });
          });
        } });
      }
      if (!isEdge && (state.kind === "flowchart" || state.kind === "class")) {
        menuItem(menu, t("ui-m-color"), null, { sub: function (sub) {
          var grid = el("div", { class: "ctx-swatches" });
          COLOURS.forEach(function (c) {
            var sw = el("button", { type: "button", class: "ctx-swatch", title: c[0] });
            sw.style.background = c[0];
            sw.style.borderColor = c[1];
            sw.addEventListener("click", function (e) {
              e.stopPropagation();
              closeMenu();
              edit("style", { id: id, fill: c[0], stroke: c[1], color: c[0] === "#202124" ? "#ffffff" : "" });
            });
            grid.appendChild(sw);
          });
          sub.appendChild(grid);
          menuItem(sub, t("ui-m-no-color"), function () { edit("style", { id: id, fill: "", stroke: "", color: "" }); });
        } });
      }
      var others = state.nodes.filter(function (n) { return n.id !== id; });
      var connect = { flowchart: "flow.edge", state: "state.transition", class: "class.relation" }[state.kind];
      if (!isEdge && connect && others.length) {
        menuItem(menu, t("ui-m-connect"), null, { sub: function (sub) {
          others.slice(0, 24).forEach(function (n) {
            menuItem(sub, n.label && n.label !== n.id ? n.label : n.id, function () {
              var params = { from: id, to: n.id };
              if (connect === "class.relation") params.type = "association";
              window.odg.post({ op: "construct", construct: connect, params: params });
            });
          });
        } });
      }
      menu.appendChild(el("div", { class: "ctx-sep" }));
      menuItem(menu, t("ui-i-delete"), function () { remove(target); }, { danger: true });
      menu.appendChild(el("div", { class: "ctx-sep" }));
      menuItem(menu, t("ui-m-properties"), function () { inspect(target, x, y); });
    }
    menu.hidden = false;
    menu.style.left = Math.max(8, Math.min(x, window.innerWidth - menu.offsetWidth - 8)) + "px";
    menu.style.top = Math.max(8, Math.min(y, window.innerHeight - menu.offsetHeight - 8)) + "px";
  }

  document.addEventListener("mousedown", function (e) {
    if (!menu.hidden && !menu.contains(e.target)) closeMenu();
  }, true);
  document.addEventListener("keydown", function (e) {
    if (e.key === "Escape" && !menu.hidden) closeMenu();
  });

  // a click anywhere outside the form closes it, like Cancel
  document.addEventListener("mousedown", function (e) {
    if (form.hidden || !state.open) return;
    if (form.contains(e.target)) return;
    closeForm();
  }, true);

  window.odgPalette = {
    close: function () { closeForm(); closeMenu(); },
    rename: function (id, isEdge, x, y) { rename(targetFor(id, isEdge), x, y); },
    contextMenu: function (id, isEdge, x, y, editing) { contextMenu(id, isEdge, x, y, editing); },
    /** Opens the inspector for a clicked element (bridge.js calls this). */
    inspect: function (id, isEdge, x, y) {
      inspect(targetFor(id, isEdge), x, y);
    },
    update: function (result, source) {
      var kind = result && result.ok ? kindOf(result.diagram_type) : state.kind;
      var nodes = result && result.ok && result.model ? result.model.nodes.filter(function (n) { return !/^root_(start|end)$/.test(n.id); }) : state.nodes;
      if (result && result.ok && result.model) {
        state.edges = result.model.edges || [];
        state.groups = result.model.groups || [];
      }
      var changed = kind !== state.kind;
      state.kind = kind;
      state.nodes = nodes;
      state.direction = kind ? directionOf(source, kind) : null;
      if (changed) closeForm();
      renderBar();
    },
    clear: function () {
      state.kind = null;
      closeForm();
      renderBar();
    },
    select: function (id) { state.selected = id; },
    strings: function (s) {
      for (var k in s) if (/^ui-(c|f|o|i|m|dir)-/.test(k)) T[k] = s[k];
      renderBar();
    }
  };
})();
