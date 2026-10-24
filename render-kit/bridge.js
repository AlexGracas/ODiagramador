/*
 * ODiagramador bridge — shared by every host (headless wry, WKWebView, WebView2, WebKitGTK).
 *
 * Host -> page:  odg.handle({ op, id, ... })            (via evaluateJavaScript / ExecuteScript)
 * Page -> host:  JSON string posted on the host channel  (see post())
 *
 * Ops:
 *   process   { id, source, theme?, png?, scale?, background? }  -> { op:"result", id, result: RawRender }
 *   rasterize { id, svg, scale?, background? }                    -> { op:"png", id, png }
 *   show      { problems: [...] }            (viewer) draws problem markers over the last diagram
 *   highlight { element_id }                 (viewer) selects/centers an element
 *   theme     { theme: "light" | "dark" }    (viewer) page chrome theme
 *   strings   { strings: { key: text } }     (viewer) localized UI strings
 *   fit / zoom { factor }                    (viewer)
 *
 * Page events (viewer): ready, select { element_id }, select-problem { index }
 */
(function () {
  "use strict";

  var FONT = 'Arial, "Liberation Sans", Helvetica, sans-serif';

  // WebKit (macOS / Linux) sometimes reports a 0x0 getBBox for text Mermaid measures off-screen (e.g. Event
  // Modeling), and Mermaid then aborts with "svg element not in render tree". Chromium measures it fine.
  // Fall back to the client box, or to an estimate from the font, only for <text>/<tspan> that report nothing.
  (function () {
    var proto = window.SVGGraphicsElement && SVGGraphicsElement.prototype;
    if (!proto || proto.__odgBBox) return;
    var native = proto.getBBox;
    proto.getBBox = function () {
      var b = native.apply(this, arguments);
      var tag = this.tagName && this.tagName.toLowerCase();
      if (b.width > 0 || b.height > 0 || (tag !== "text" && tag !== "tspan")) return b;
      var text = (this.textContent || "").replace(/\u200b/g, "");
      var cs = window.getComputedStyle(this);
      var size = parseFloat(cs.fontSize) || 16;
      var r = this.getBoundingClientRect();
      var width = r.width > 0 ? r.width : text.length * size * 0.6;
      var height = r.height > 0 ? r.height : size * 1.2;
      return { x: b.x, y: b.y - size, width: width, height: height, top: b.y - size, left: b.x, right: b.x + width, bottom: b.y - size + height };
    };
    proto.__odgBBox = true;
  })();
  var MAX_CANVAS_PIXELS = 16000000; // WebKit canvas limit is ~16.7M px
  var params = new URLSearchParams(location.search);
  var mode = params.get("mode") || "headless";
  var counter = 0;
  var configuredTheme = null;
  var queue = Promise.resolve();
  var strings = { fit: "Fit", empty: "No diagram" };
  var view = { scale: 1, x: 0, y: 0, svg: null, width: 0, height: 0, fitted: false };

  // ---------------------------------------------------------------- transport
  function post(msg) {
    if (msg && /^(construct|edit|move|layout-mode|reset-positions)$/.test(msg.op) && typeof history === "object") history.breakNext = true;
    var s = JSON.stringify(msg);
    try {
      if (window.ipc && typeof window.ipc.postMessage === "function") return window.ipc.postMessage(s); // wry
      if (window.chrome && window.chrome.webview) return window.chrome.webview.postMessage(s); // WebView2
      if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.odg) {
        return window.webkit.messageHandlers.odg.postMessage(s); // WKWebView / WebKitGTK
      }
    } catch (e) {
      console.error("[odg] post failed", e);
    }
    console.log("[odg]", s);
  }

  // ---------------------------------------------------------------- mermaid setup
  function configure(theme, layout, font) {
    var family = font && String(font).trim() ? String(font) : FONT;
    var key = (theme || "default") + "|" + (layout || "dagre") + "|" + family;
    if (configuredTheme === key) return;
    var noMax = { useMaxWidth: false };
    mermaid.initialize({
      startOnLoad: false,
      layout: layout || "dagre",
      securityLevel: "strict",
      theme: theme || "default",
      fontFamily: family,
      themeVariables: { fontFamily: family },
      htmlLabels: false,
      flowchart: { htmlLabels: false, useMaxWidth: false },
      class: { htmlLabels: false, useMaxWidth: false },
      state: noMax, sequence: noMax, er: noMax, gantt: noMax, journey: noMax, pie: noMax,
      mindmap: noMax, timeline: noMax, requirement: noMax, gitGraph: noMax, quadrantChart: noMax,
      xyChart: noMax, block: noMax, sankey: noMax, packet: noMax, architecture: noMax, c4: noMax,
      kanban: noMax, radar: noMax, treemap: noMax
    });
    configuredTheme = key;
  }

  // ---------------------------------------------------------------- @odg layout directives
  // %% @odg layout auto|elk|elk-interactive|exact
  // %% @odg pos <id> <x>,<y>        node centre in diagram (SVG user) coordinates
  // %% @odg layer <id> first|last|<n>   (n: layer order, 0 first)
  var LAYOUTS = { elk: "odg-elk", "elk-interactive": "odg-interactive", exact: "odg-exact" };
  var GRAPH_TYPES = /^(flowchart|graph|state|class|er|requirement)/;

  function parseDirectives(src) {
    var d = { mode: "auto", pos: {}, layer: {} };
    src.split(/\r?\n/).forEach(function (line) {
      var m = line.match(/^\s*%%\s*@odg\s+(\S+)\s*(.*)$/);
      if (!m) return;
      var args = m[2].trim().split(/\s+/);
      if (m[1] === "layout" && args[0]) d.mode = args[0];
      else if (m[1] === "pos" && args.length >= 2) {
        var xy = args[1].split(",");
        var x = parseFloat(xy[0]), y = parseFloat(xy[1]);
        if (isFinite(x) && isFinite(y)) d.pos[args[0]] = [x, y];
      } else if (m[1] === "layer" && args.length >= 2) d.layer[args[0]] = args[1];
    });
    return d;
  }

  // a `layout:` key in the front matter would override ours: swap its value, keep line numbers
  function withLayout(src, name) {
    var lines = src.split("\n");
    if (!lines.length || lines[0].trim() !== "---") return src;
    for (var i = 1; i < lines.length && lines[i].trim() !== "---"; i++) {
      lines[i] = lines[i].replace(/^(\s*layout:\s*)\S+/, "$1" + name);
    }
    return lines.join("\n");
  }

  // ---------------------------------------------------------------- errors
  function normalizeError(e, kindHint) {
    var msg = String((e && (e.message || e.str)) || e);
    var line = null, column = null, token = null, expected = null;
    var h = e && e.hash;
    if (h) {
      if (h.loc) {
        // jison's loc spans the previous token up to the offending one: its end is the culprit
        line = h.loc.first_line;
        if (h.loc.last_line === h.loc.first_line) column = h.loc.last_column + 1;
      } else if (typeof h.line === "number") {
        line = h.line + 1;
      }
      token = h.text && String(h.text).trim() ? h.text : h.token;
      expected = h.expected;
    }
    var r = e && e.result;
    if (line == null && r) {
      var le = r.lexerErrors && r.lexerErrors[0];
      var pe = r.parserErrors && r.parserErrors[0];
      if (le) {
        line = le.line;
        column = le.column;
      } else if (pe && pe.token) {
        line = pe.token.startLine;
        column = pe.token.startColumn;
      }
    }
    if (line == null) {
      var m = msg.match(/line (\d+)/i);
      if (m) line = +m[1];
      var c = msg.match(/column (\d+)/i);
      if (c) column = +c[1];
    }
    var kind = kindHint || "parse";
    if (/No diagram type detected/i.test(msg) || (e && e.name === "UnknownDiagramError")) kind = "unknown_type";
    return {
      kind: kind,
      message: msg.slice(0, 2000),
      line: line,
      column: column,
      token: token == null ? null : String(token),
      expected: Array.isArray(expected) ? expected.slice(0, 12).map(String) : null
    };
  }

  // ---------------------------------------------------------------- model
  function plain(label) {
    if (label == null) return "";
    var s = String(label);
    s = s.replace(/<br\s*\/?>/gi, " ").replace(/<[^>]+>/g, "");
    s = s.replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&amp;/g, "&").replace(/&quot;/g, '"');
    s = s.replace(/^"(.*)"$/, "$1").replace(/^`(.*)`$/, "$1");
    return s.trim();
  }

  function extractModel(diag) {
    var model = { nodes: [], edges: [], groups: [] };
    var db = diag && diag.db;
    if (!db) return model;
    if (typeof db.getData === "function") {
      var d = db.getData();
      (d.nodes || []).forEach(function (n) {
        var id = String(n.id);
        var label = plain(n.label != null ? n.label : n.id);
        var domKey = n.domId ? stripCounter(String(n.domId)) : null;
        var er = !n.domId && id.match(/^entity-(.+)-\d+$/); // erDiagram: id is the dom id
        if (er) {
          domKey = stripCounter(id);
          id = er[1];
        }
        var item = { id: id, label: label, parent: n.parentId || null, shape: n.shape || null, dom_key: domKey };
        if (n.isGroup) model.groups.push(item); else model.nodes.push(item);
      });
      (d.edges || []).forEach(function (e) {
        model.edges.push({ id: String(e.id), from: String(e.start), to: String(e.end), label: plain(e.label) });
      });
      return model;
    }
    if (typeof db.getBlocksFlat === "function" && typeof db.getEdges === "function") { // block-beta
      var parentOf = {};
      (typeof db.getBlocks === "function" ? db.getBlocks() : []).forEach(function walk(b, _i, _a, parent) {
        if (parent) parentOf[b.id] = parent;
        (b.children || []).forEach(function (c) { walk(c, 0, null, b.type === "composite" && b.id !== "root" ? b.id : null); });
      });
      db.getBlocksFlat().forEach(function (b) {
        var id = String(b.id);
        if (id === "root" || b.type === "space") return;
        var item = { id: id, label: plain(b.label != null ? b.label : b.id), parent: parentOf[id] || null, shape: b.type || null, dom_key: null };
        if (b.type === "composite") model.groups.push(item); else model.nodes.push(item);
      });
      db.getEdges().forEach(function (e) {
        model.edges.push({ id: String(e.id), from: String(e.start), to: String(e.end), label: plain(e.label) });
      });
      return model;
    }
    if (typeof db.getActors === "function") { // sequence
      var actors = db.getActors();
      var it = actors instanceof Map ? actors.values() : Object.values(actors);
      Array.from(it).forEach(function (a) {
        model.nodes.push({ id: String(a.name), label: plain(a.description || a.name), parent: null, shape: a.type || "participant" });
      });
      var msgs = typeof db.getMessages === "function" ? db.getMessages() : [];
      msgs.forEach(function (m, i) {
        if (m.from && m.to) model.edges.push({ id: "msg" + i, from: String(m.from), to: String(m.to), label: plain(m.message) });
      });
    }
    return model;
  }

  // ---------------------------------------------------------------- geometry
  function viewBoxOf(svg) {
    var vb = svg.viewBox && svg.viewBox.baseVal;
    if (vb && vb.width > 0) return { x: vb.x, y: vb.y, w: vb.width, h: vb.height };
    var b = svg.getBBox();
    return { x: b.x, y: b.y, w: b.width, h: b.height };
  }

  // bbox of el in the svg's user (viewBox) coordinates
  function userBox(el, svg, vb) {
    var b;
    try { b = el.getBBox(); } catch (e) { return null; }
    var m = el.getCTM();
    var root = svg.getCTM();
    if (!m) return { x: b.x, y: b.y, w: b.width, h: b.height };
    // CTM maps to the svg viewport; undo the viewBox->viewport transform of the root
    var toUser = root ? root.inverse().multiply(m) : m;
    var pts = [[b.x, b.y], [b.x + b.width, b.y], [b.x, b.y + b.height], [b.x + b.width, b.y + b.height]];
    var xs = [], ys = [];
    pts.forEach(function (p) {
      var pt = svg.createSVGPoint();
      pt.x = p[0];
      pt.y = p[1];
      var t = pt.matrixTransform(toUser);
      xs.push(t.x);
      ys.push(t.y);
    });
    var x0 = Math.min.apply(null, xs), y0 = Math.min.apply(null, ys);
    return round({ x: x0, y: y0, w: Math.max.apply(null, xs) - x0, h: Math.max.apply(null, ys) - y0 });
  }

  /** Points along a path, in the svg's user coordinates (for crossings / edges through nodes). */
  function userPoints(path, svg) {
    if (!path || typeof path.getTotalLength !== "function") return null;
    var len;
    try { len = path.getTotalLength(); } catch (e) { return null; }
    var m = path.getCTM(), root = svg.getCTM();
    var toUser = m ? (root ? root.inverse().multiply(m) : m) : null;
    var n = Math.max(2, Math.min(48, Math.ceil(len / 10)));
    var pts = [];
    for (var i = 0; i <= n; i++) {
      var p = path.getPointAtLength((len * i) / n);
      if (toUser) {
        var q = svg.createSVGPoint();
        q.x = p.x;
        q.y = p.y;
        p = q.matrixTransform(toUser);
      }
      pts.push([Math.round(p.x * 10) / 10, Math.round(p.y * 10) / 10]);
    }
    var scale = toUser ? Math.sqrt(Math.abs(toUser.a * toUser.d - toUser.b * toUser.c)) || 1 : 1;
    return { points: pts, length: Math.round(len * scale * 10) / 10 };
  }

  function round(b) {
    return { x: Math.round(b.x * 10) / 10, y: Math.round(b.y * 10) / 10, w: Math.round(b.w * 10) / 10, h: Math.round(b.h * 10) / 10 };
  }

  function shapeOf(g) {
    for (var i = 0; i < g.children.length; i++) {
      var c = g.children[i];
      if (c.classList && c.classList.contains("label")) continue;
      if (/^(rect|polygon|circle|ellipse|path|g)$/.test(c.tagName)) return c;
    }
    return null;
  }

  function labelOf(g) {
    return g.querySelector(":scope > .label, :scope > g.label, :scope > text") || g.querySelector(".label, text");
  }

  function stripCounter(domId) {
    return domId.replace(/-\d+$/, "");
  }

  function measureGeometry(svg, model, type) {
    var checkOverflow = /^(flowchart|state|graph)/.test(type || "");
    var vb = viewBoxOf(svg);
    var byId = {};
    // unified renderer nodes carry "<svgId>-<type>-<id>-<n>" ids without data-id: tag them
    var prefix = svg.id ? svg.id + "-" : "";
    var byKey = {};
    var gnodes = svg.querySelectorAll("g.node, g.cluster");
    for (var k = 0; k < gnodes.length; k++) {
      var gid = gnodes[k].id || "";
      if (prefix && gid.indexOf(prefix) === 0) gid = gid.slice(prefix.length);
      byKey[stripCounter(gid)] = gnodes[k];
      byKey[gid] = byKey[gid] || gnodes[k];
    }
    model.nodes.concat(model.groups).forEach(function (n) {
      var el = (n.dom_key && byKey[n.dom_key]) || byKey[n.id];
      if (el && !el.getAttribute("data-id")) el.setAttribute("data-id", n.id);
      delete n.dom_key;
    });
    var els = svg.querySelectorAll("[data-id]");
    for (var i = 0; i < els.length; i++) {
      var el = els[i];
      var id = el.getAttribute("data-id");
      var et = el.getAttribute("data-et");
      if (et === "life-line" || et === "edge") continue;
      if (el.closest(".edgeLabel")) continue;
      if (!byId[id]) byId[id] = el; // sequence: first is the top actor box
    }
    function enrich(item, isGroup) {
      var el = byId[item.id];
      if (!el) return;
      item.dom_id = el.id || null;
      item.bbox = userBox(el, svg, vb);
      if (isGroup || !checkOverflow) return;
      var shape = shapeOf(el), label = labelOf(el);
      if (shape && label) {
        var sb = userBox(shape, svg, vb), lb = userBox(label, svg, vb);
        if (sb && lb) {
          item.label_bbox = lb;
          item.label_overflow = lb.w > sb.w + 2 || lb.h > sb.h + 2;
        }
      }
    }
    model.nodes.forEach(function (n) { enrich(n, false); });
    model.groups.forEach(function (g) { enrich(g, true); });
    // nodes the model did not know about (diagram types without getData)
    if (model.nodes.length === 0) {
      Object.keys(byId).forEach(function (id) {
        var el = byId[id];
        var cls = el.getAttribute("class") || "";
        if (!/\bnode\b/.test(cls) && el.getAttribute("data-et") !== "participant") return;
        var item = { id: id, label: plain(el.textContent), parent: null, shape: null };
        enrich(item, false);
        model.nodes.push(item);
      });
    }
    var edgeEls = svg.querySelectorAll('[data-et="edge"], path.flowchart-link[data-id], path[data-id][marker-end]');
    var edgeById = {};
    for (var j = 0; j < edgeEls.length; j++) {
      var did = edgeEls[j].getAttribute("data-id");
      edgeById[did] = edgeById[did] || edgeEls[j];
      if (prefix && did.indexOf(prefix) === 0) edgeById[did.slice(prefix.length)] = edgeById[did.slice(prefix.length)] || edgeEls[j];
    }
    // edge labels: by data-id when the renderer sets it, otherwise by text (in order)
    var labelEls = Array.prototype.slice.call(svg.querySelectorAll("g.edgeLabel"));
    var usedLabels = [];
    function edgeLabelBox(e) {
      var want = (e.label || "").trim();
      if (!want) return null;
      for (var i = 0; i < labelEls.length; i++) {
        var le = labelEls[i];
        if (usedLabels.indexOf(le) >= 0) continue;
        var tagged = le.querySelector("[data-id]");
        var tid = tagged && tagged.getAttribute("data-id");
        var match = tid ? (tid === e.id || tid === prefix + e.id) : plain(le.textContent) === want;
        if (!match) continue;
        usedLabels.push(le);
        var inner = le.querySelector("rect.background, rect, foreignObject, text") || le;
        var b = userBox(inner, svg, vb);
        return b && b.w > 0 && b.h > 0 ? b : null;
      }
      return null;
    }
    model.edges.forEach(function (e) {
      var el = edgeById[e.id];
      if (el) {
        e.bbox = userBox(el, svg, vb);
        var path = el.tagName && el.tagName.toLowerCase() === "path" ? el : el.querySelector("path");
        var sampled = userPoints(path, svg);
        if (sampled) {
          e.points = sampled.points;
          e.length = sampled.length;
        }
      }
      var lb = edgeLabelBox(e);
      if (lb) e.label_bbox = lb;
    });
    return vb;
  }

  // ---------------------------------------------------------------- svg helpers
  function fixSize(svg) {
    var vb = viewBoxOf(svg);
    svg.setAttribute("width", String(Math.ceil(vb.w)));
    svg.setAttribute("height", String(Math.ceil(vb.h)));
    svg.style.maxWidth = "";
    svg.removeAttribute("style");
    return vb;
  }

  function serialize(svg) {
    var s = new XMLSerializer().serializeToString(svg);
    if (s.indexOf('xmlns="http://www.w3.org/2000/svg"') < 0) s = s.replace("<svg", '<svg xmlns="http://www.w3.org/2000/svg"');
    return s;
  }

  function rasterize(svgText, scale, background) {
    return new Promise(function (resolve, reject) {
      var doc = new DOMParser().parseFromString(svgText, "image/svg+xml");
      var root = doc.documentElement;
      var w = parseFloat(root.getAttribute("width")) || 800;
      var h = parseFloat(root.getAttribute("height")) || 600;
      var s = scale || 2;
      if (w * h * s * s > MAX_CANVAS_PIXELS) s = Math.sqrt(MAX_CANVAS_PIXELS / (w * h));
      var blob = new Blob([svgText], { type: "image/svg+xml;charset=utf-8" });
      var url = URL.createObjectURL(blob);
      var img = new Image();
      img.onload = function () {
        try {
          var canvas = document.createElement("canvas");
          canvas.width = Math.max(1, Math.round(w * s));
          canvas.height = Math.max(1, Math.round(h * s));
          var ctx = canvas.getContext("2d");
          if (background && background !== "transparent") {
            ctx.fillStyle = background;
            ctx.fillRect(0, 0, canvas.width, canvas.height);
          }
          ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
          URL.revokeObjectURL(url);
          resolve({ png: canvas.toDataURL("image/png").split(",")[1], width: canvas.width, height: canvas.height });
        } catch (e) {
          URL.revokeObjectURL(url);
          reject(e);
        }
      };
      img.onerror = function () {
        URL.revokeObjectURL(url);
        reject(new Error("svg image failed to load (unsupported content such as foreignObject?)"));
      };
      img.src = url;
    });
  }

  /** Text of an HTML label: <br> and block elements become line breaks. */
  function htmlLines(el) {
    var out = [""];
    (function walk(n) {
      for (var c = n.firstChild; c; c = c.nextSibling) {
        if (c.nodeType === 3) out[out.length - 1] += c.nodeValue;
        else if (c.nodeType === 1) {
          var tag = c.tagName.toLowerCase();
          var block = tag === "br" || tag === "div" || tag === "p" || tag === "li";
          if (block && out[out.length - 1].trim()) out.push("");
          if (tag !== "br") walk(c);
          if (block && out[out.length - 1].trim()) out.push("");
        }
      }
    })(el);
    return out.map(function (l) { return l.replace(/\s+/g, " ").trim(); }).filter(Boolean);
  }

  /**
   * WebKit refuses to export a canvas that drew an SVG with <foreignObject> (HTML labels used by some
   * diagram types). For the PNG only, swap each HTML label for SVG text at its centre, in its font.
   */
  function foreignObjectsAsText(svg) {
    var NS = "http://www.w3.org/2000/svg";
    var clone = svg.cloneNode(true);
    var live = svg.querySelectorAll("foreignObject");
    var copies = clone.querySelectorAll("foreignObject");
    for (var i = 0; i < live.length && i < copies.length; i++) {
      var fo = live[i];
      var w = parseFloat(fo.getAttribute("width")) || 0, h = parseFloat(fo.getAttribute("height")) || 0;
      var cx = (parseFloat(fo.getAttribute("x")) || 0) + w / 2, cy = (parseFloat(fo.getAttribute("y")) || 0) + h / 2;
      var lines = htmlLines(fo);
      var styled = fo.querySelector("span, p, div") || fo.firstElementChild || fo;
      var cs = window.getComputedStyle(styled);
      var size = parseFloat(cs.fontSize) || 14;
      var t = document.createElementNS(NS, "text");
      t.setAttribute("text-anchor", "middle");
      t.setAttribute("font-size", size);
      t.setAttribute("font-family", cs.fontFamily || FONT);
      if (cs.fontWeight && cs.fontWeight !== "400" && cs.fontWeight !== "normal") t.setAttribute("font-weight", cs.fontWeight);
      t.setAttribute("fill", cs.color || "#333");
      var lh = size * 1.25, y0 = cy - ((lines.length - 1) * lh) / 2;
      lines.forEach(function (line, k) {
        var span = document.createElementNS(NS, "tspan");
        span.setAttribute("x", cx);
        span.setAttribute("y", y0 + k * lh);
        span.setAttribute("dominant-baseline", "central");
        span.textContent = line;
        t.appendChild(span);
      });
      copies[i].parentNode.replaceChild(t, copies[i]);
    }
    return serialize(clone);
  }

  // ---------------------------------------------------------------- process
  async function processDiagram(req) {
    var t0 = performance.now();
    var source = req.source == null ? "" : String(req.source);
    var out = {
      ok: false, diagram_type: null, parse_error: null, render_error: null, raster_error: null,
      svg: null, png: null, png_width: 0, png_height: 0, width: 0, height: 0,
      view_box: null, model: { nodes: [], edges: [], groups: [] }, has_foreign_object: false,
      mermaid_version: (window.mermaid && mermaid.version) || null, duration_ms: 0
    };
    if (window.odgReady) await window.odgReady; // external diagrams (ZenUML) registered
    if (mode === "viewer") recordHistory(req.doc, source);
    var dirs = parseDirectives(source);
    var layoutName = LAYOUTS[dirs.mode] || null;
    // ELK (and dragging / exact positions) only for graph diagrams; every other type keeps Mermaid's own layout
    if (layoutName) {
      var detected = null;
      try { detected = mermaid.detectType(source); } catch (e) { /* unknown type: the parse below reports it */ }
      if (detected && !GRAPH_TYPES.test(detected)) {
        out.layout_fallback = dirs.mode;
        layoutName = null;
        dirs.mode = "auto";
      }
    }
    if (window.odgLayout) {
      window.odgLayout.hints.pos = dirs.pos;
      window.odgLayout.hints.layer = dirs.layer;
    }
    configure(req.theme, layoutName, req.font);
    out.layout_mode = dirs.mode;
    if (layoutName) source = withLayout(source, layoutName);
    try {
      await mermaid.parse(source);
    } catch (e) {
      out.parse_error = normalizeError(e, "parse");
      out.duration_ms = Math.round(performance.now() - t0);
      return out;
    }
    try {
      var diag = await mermaid.mermaidAPI.getDiagramFromText(source);
      out.diagram_type = diag.type;
      out.model = extractModel(diag);
    } catch (e) {
      out.model_error = String((e && e.message) || e);
    }
    var host = mode === "viewer" ? document.getElementById("measure") : document.getElementById("measure");
    var id = "odg" + (++counter);
    var svgText;
    try {
      var res = await mermaid.render(id, source);
      svgText = res.svg;
    } catch (e) {
      out.render_error = normalizeError(e, "render");
      cleanupTemp(id);
      out.duration_ms = Math.round(performance.now() - t0);
      return out;
    }
    cleanupTemp(id);
    host.innerHTML = svgText;
    var svg = host.querySelector("svg");
    var vb = fixSize(svg);
    try {
      measureGeometry(svg, out.model, out.diagram_type);
    } catch (e) {
      out.model_error = String((e && e.message) || e);
    }
    out.view_box = round(vb);
    out.width = Math.ceil(vb.w);
    out.height = Math.ceil(vb.h);
    out.has_foreign_object = !!svg.querySelector("foreignObject");
    out.svg = serialize(svg);
    out.ok = true;
    if (req.png) {
      try {
        var bg = req.background || (req.theme === "dark" ? "#0d1117" : "#ffffff");
        var r;
        try {
          r = await rasterize(out.svg, req.scale || 2, bg);
        } catch (e) {
          if (!out.has_foreign_object) throw e;
          r = await rasterize(foreignObjectsAsText(svg), req.scale || 2, bg);
          out.png_text_fallback = true;
        }
        out.png = r.png;
        out.png_width = r.width;
        out.png_height = r.height;
      } catch (e) {
        out.raster_error = String((e && e.message) || e);
      }
    }
    if (mode === "viewer") {
      showSvg(svg);
      updateLayoutControls(out.diagram_type, dirs.mode);
      if (window.odgPalette) window.odgPalette.update(out, req.source);
    }
    else host.innerHTML = "";
    out.duration_ms = Math.round(performance.now() - t0);
    return out;
  }

  function cleanupTemp(id) {
    ["d" + id, id, "i" + id].forEach(function (x) {
      var el = document.getElementById(x);
      if (el && el.parentNode && el.closest("#stage") == null && el.closest("#measure") == null) el.parentNode.removeChild(el);
    });
  }

  // ---------------------------------------------------------------- viewer
  var viewport = document.getElementById("viewport");
  var stage = document.getElementById("stage");

  function applyTransform() {
    stage.style.transform = "translate(" + view.x + "px," + view.y + "px) scale(" + view.scale + ")";
    // arrow click strips stay ~16 screen px wide at any zoom
    stage.style.setProperty("--odg-hit", (16 / (view.scale || 1)).toFixed(2) + "px");
    var label = document.getElementById("zoom-label");
    if (label) label.textContent = Math.round(view.scale * 100) + "%";
    reportView();
  }

  // ---- what the human sees (zoom, element in the centre, elements on screen) for agents: get_viewer_status
  var viewReportTimer = null;
  function reportView() {
    if (mode !== "viewer") return;
    clearTimeout(viewReportTimer);
    viewReportTimer = setTimeout(function () {
      if (!view.svg) return post({ op: "view", zoom: view.scale, center_element: null, visible_elements: [] });
      var vr = viewport.getBoundingClientRect();
      var cx = vr.left + vr.width / 2, cy = vr.top + vr.height / 2;
      var seen = {}, visible = [], best = null, bestD = Infinity;
      var els = view.svg.querySelectorAll("[data-id]");
      for (var i = 0; i < els.length; i++) {
        var el = els[i], id = el.getAttribute("data-id");
        if (!id || seen[id] || el.getAttribute("data-et") === "edge" || el.closest(".edgeLabel")) continue;
        seen[id] = true;
        var b = el.getBoundingClientRect();
        if (b.right < vr.left || b.left > vr.right || b.bottom < vr.top || b.top > vr.bottom) continue;
        visible.push(id);
        var d = Math.pow((b.left + b.right) / 2 - cx, 2) + Math.pow((b.top + b.bottom) / 2 - cy, 2);
        if (d < bestD) { bestD = d; best = id; }
      }
      post({ op: "view", zoom: Math.round(view.scale * 1000) / 1000, center_element: best, visible_elements: visible.slice(0, 60) });
    }, 400);
  }

  function fit() {
    if (!view.svg) return;
    var pad = 24;
    // keep the drawing below the constructor palette and above the bottom toolbar
    var palette = document.getElementById("palette");
    var top = palette && !palette.hidden ? palette.getBoundingClientRect().bottom : 0;
    var bottom = 44;
    var vw = viewport.clientWidth - pad * 2, vh = viewport.clientHeight - top - bottom - pad * 2;
    var s = Math.min(vw / view.width, vh / view.height, 2);
    if (!isFinite(s) || s <= 0) s = 1;
    view.scale = s;
    view.x = (viewport.clientWidth - view.width * s) / 2;
    view.y = top + pad + (vh - view.height * s) / 2;
    applyTransform();
  }

  function zoom(factor, cx, cy) {
    var nx = cx == null ? viewport.clientWidth / 2 : cx;
    var ny = cy == null ? viewport.clientHeight / 2 : cy;
    var ns = Math.min(8, Math.max(0.05, view.scale * factor));
    view.x = nx - (nx - view.x) * (ns / view.scale);
    view.y = ny - (ny - view.y) * (ns / view.scale);
    view.scale = ns;
    applyTransform();
  }

  function showSvg(svg) {
    document.getElementById("banner").hidden = true;
    document.getElementById("empty").hidden = true;
    document.getElementById("toolbar").hidden = false;
    document.getElementById("modebar").hidden = false;
    stage.innerHTML = "";
    stage.appendChild(svg);
    var firstTime = !view.svg;
    var pw = view.width, ph = view.height;
    view.svg = svg;
    view.width = parseFloat(svg.getAttribute("width")) || 1;
    view.height = parseFloat(svg.getAttribute("height")) || 1;
    // keep the user's zoom while editing, but re-frame when the drawing changes size a lot
    var resized = Math.abs(view.width - pw) / Math.max(pw, 1) > 0.25 || Math.abs(view.height - ph) / Math.max(ph, 1) > 0.25;
    if (firstTime || !view.fitted || resized) {
      fit();
      view.fitted = true;
    } else {
      applyTransform();
    }
    addEdgeHitAreas(svg);
    if (spotlight.pending) applySpotlight();
    else restoreSpotlight();
    if (agentView.pending) applyView();
    if (agentSelect.pending) applySelect();
  }

  /** Arrows are 1–2 px wide: give each an invisible ~14 px strip along its path so a click near it selects it. */
  function addEdgeHitAreas(svg) {
    var NS = "http://www.w3.org/2000/svg";
    var paths = svg.querySelectorAll('path[data-et="edge"], path.flowchart-link[data-id], path[data-id][marker-end]');
    for (var i = 0; i < paths.length; i++) {
      var p = paths[i];
      if (p.classList.contains("odg-hit") || !p.getAttribute("d")) continue;
      var hit = document.createElementNS(NS, "path");
      hit.setAttribute("d", p.getAttribute("d"));
      if (p.getAttribute("transform")) hit.setAttribute("transform", p.getAttribute("transform"));
      hit.setAttribute("class", "odg-hit");
      hit.setAttribute("data-id", p.getAttribute("data-id"));
      hit.setAttribute("data-et", "edge");
      p.parentNode.insertBefore(hit, p.nextSibling);
    }
  }

  function showError(err) {
    var banner = document.getElementById("banner");
    var where = err.line ? " (line " + err.line + (err.column ? ":" + err.column : "") + ")" : "";
    banner.textContent = err.message + where;
    banner.hidden = false;
    document.getElementById("empty").hidden = true;
  }

  function showEmpty() {
    stage.innerHTML = "";
    view.svg = null;
    document.getElementById("toolbar").hidden = true;
    document.getElementById("modebar").hidden = true;
    document.getElementById("banner").hidden = true;
    var empty = document.getElementById("empty");
    empty.textContent = strings.empty;
    empty.hidden = false;
  }

  function drawMarkers(problems) {
    if (!view.svg) return;
    var NS = "http://www.w3.org/2000/svg";
    var old = view.svg.querySelector("#odg-overlay");
    if (old) old.parentNode.removeChild(old);
    var layer = document.createElementNS(NS, "g");
    layer.setAttribute("id", "odg-overlay");
    var slots = {}; // problems on the same area get side-by-side badges
    (problems || []).forEach(function (p) {
      if (!p.bbox) return;
      var key = [p.bbox.x, p.bbox.y, p.bbox.w, p.bbox.h].join(",");
      var slot = slots[key] || 0;
      slots[key] = slot + 1;
      var g = document.createElementNS(NS, "g");
      g.setAttribute("class", "odg-marker " + (p.severity || "warning"));
      g.setAttribute("data-problem", String(p.index));
      var pad = 4;
      var r = document.createElementNS(NS, "rect");
      r.setAttribute("x", p.bbox.x - pad);
      r.setAttribute("y", p.bbox.y - pad);
      r.setAttribute("width", p.bbox.w + pad * 2);
      r.setAttribute("height", p.bbox.h + pad * 2);
      r.setAttribute("rx", 4);
      if (slot > 0) r.setAttribute("visibility", "hidden");
      var bx = p.bbox.x - pad + 22 * slot;
      var c = document.createElementNS(NS, "circle");
      c.setAttribute("cx", bx);
      c.setAttribute("cy", p.bbox.y - pad);
      c.setAttribute("r", 9);
      var t = document.createElementNS(NS, "text");
      t.setAttribute("x", bx);
      t.setAttribute("y", p.bbox.y - pad + 4);
      t.setAttribute("text-anchor", "middle");
      t.textContent = String(p.index);
      var title = document.createElementNS(NS, "title");
      title.textContent = (p.code ? p.code + ": " : "") + (p.message || "");
      g.appendChild(title);
      g.appendChild(r);
      g.appendChild(c);
      g.appendChild(t);
      layer.appendChild(g);
    });
    view.svg.appendChild(layer);
  }

  function highlight(elementId) {
    if (!view.svg) return;
    var prev = view.svg.querySelectorAll(".odg-selected");
    for (var i = 0; i < prev.length; i++) prev[i].classList.remove("odg-selected");
    if (!elementId) return;
    var el = view.svg.querySelector('[data-id="' + CSS.escape(elementId) + '"]');
    if (!el) return;
    el.classList.add("odg-selected");
    var b = el.getBoundingClientRect();
    var vr = viewport.getBoundingClientRect();
    if (b.right < vr.left || b.left > vr.right || b.bottom < vr.top || b.top > vr.bottom) {
      view.x += vr.left + vr.width / 2 - (b.left + b.width / 2);
      view.y += vr.top + vr.height / 2 - (b.top + b.height / 2);
      applyTransform();
    }
  }

  // ---- edit / view mode: view mode shows and selects only (no palette, inspector, dragging, editor typing)
  var editing = true;

  function readStoredMode() {
    try { return localStorage.getItem("odg.mode") !== "view"; } catch (e) { return true; }
  }

  function setEditing(on, user) {
    editing = !!on;
    document.body.classList.toggle("view-mode", !editing);
    document.getElementById("mode-edit").setAttribute("aria-checked", editing ? "true" : "false");
    document.getElementById("mode-view").setAttribute("aria-checked", editing ? "false" : "true");
    if (!editing && window.odgPalette && typeof window.odgPalette.close === "function") window.odgPalette.close();
    document.body.classList.toggle("can-drag", editing && view.graph && (view.layoutMode === "elk-interactive" || view.layoutMode === "exact"));
    if (user) { try { localStorage.setItem("odg.mode", editing ? "edit" : "view"); } catch (e) { /* private storage */ } }
    post({ op: "edit-mode", editing: editing });
    if (view.svg && view.fitted) fit();
  }

  // ---- undo / redo: every version of the open diagram the viewer drew (typing in bursts, palette, inspector,
  // dragging, agent edits); undo/redo hands a version back to the host, which saves it like any edit
  var history = { doc: null, stack: [], index: -1, at: 0, breakNext: false };

  function recordHistory(doc, source) {
    var d = doc || "";
    if (d !== history.doc || !source) {
      history = { doc: d, stack: source ? [source] : [], index: source ? 0 : -1, at: Date.now(), breakNext: false };
      updateHistoryButtons();
      return;
    }
    if (source === history.stack[history.index]) return; // undo / redo coming back, or a re-render
    history.stack = history.stack.slice(0, history.index + 1);
    var now = Date.now();
    // typing arrives as many renders: one step per burst, but actions always get their own step
    if (!history.breakNext && history.stack.length > 1 && now - history.at < 1200) history.stack[history.index] = source;
    else history.stack.push(source);
    if (history.stack.length > 200) history.stack.shift();
    history.index = history.stack.length - 1;
    history.at = now;
    history.breakNext = false;
    updateHistoryButtons();
  }

  function updateHistoryButtons() {
    document.getElementById("undo-btn").disabled = history.index <= 0;
    document.getElementById("redo-btn").disabled = history.index < 0 || history.index >= history.stack.length - 1;
  }

  function undo() {
    if (!editing || history.index <= 0) return;
    history.index--;
    history.breakNext = true;
    updateHistoryButtons();
    post({ op: "set-source", source: history.stack[history.index], reason: "undo" });
  }

  function redo() {
    if (!editing || history.index >= history.stack.length - 1) return;
    history.index++;
    history.breakNext = true;
    updateHistoryButtons();
    post({ op: "set-source", source: history.stack[history.index], reason: "redo" });
  }

  // ---- layout controls (shared by every desktop app)
  var layoutSelect = document.getElementById("layout-select");
  var resetButton = document.getElementById("reset-positions");
  var layoutBox = document.getElementById("layout-box");

  function updateLayoutControls(type, layoutMode) {
    view.graph = GRAPH_TYPES.test(type || "");
    view.layoutMode = layoutMode || "auto";
    layoutBox.hidden = !view.graph;
    if (layoutSelect.querySelector('option[value="' + view.layoutMode + '"]')) layoutSelect.value = view.layoutMode;
    resetButton.disabled = view.layoutMode === "auto" || view.layoutMode === "elk";
    document.body.classList.toggle("can-drag", editing && view.graph && (view.layoutMode === "elk-interactive" || view.layoutMode === "exact"));
  }

  function nodeCentre(el) {
    var vb = viewBoxOf(view.svg);
    var b = userBox(el, view.svg, vb);
    return b ? { x: b.x + b.w / 2, y: b.y + b.h / 2 } : null;
  }

  function setupViewer() {
    var drag = null;
    layoutSelect.addEventListener("change", function () {
      post({ op: "layout-mode", mode: layoutSelect.value });
    });
    resetButton.addEventListener("click", function () {
      post({ op: "reset-positions" });
    });
    viewport.addEventListener("mousedown", function (e) {
      if (e.button !== 0) return;
      // dragging a node moves it only in the drag-enabled layouts (the menu says "drag"); a plain
      // click must never rewrite the file, so auto/ELK layouts just select and pan
      var draggable = editing && view.graph && (view.layoutMode === "elk-interactive" || view.layoutMode === "exact");
      var node = draggable && e.target.closest && e.target.closest("g.node[data-id]");
      if (node) {
        drag = { node: node, id: node.getAttribute("data-id"), centre: nodeCentre(node), transform: node.getAttribute("transform") || "", x: e.clientX, y: e.clientY, moved: false };
        node.classList.add("odg-dragging");
        return;
      }
      drag = { x: e.clientX, y: e.clientY, vx: view.x, vy: view.y, moved: false };
      viewport.classList.add("dragging");
    });
    window.addEventListener("mousemove", function (e) {
      if (!drag) return;
      var dx = e.clientX - drag.x, dy = e.clientY - drag.y;
      if (Math.abs(dx) + Math.abs(dy) > 6) drag.moved = true;
      if (drag.node) {
        if (drag.moved) drag.node.setAttribute("transform", "translate(" + dx / view.scale + "," + dy / view.scale + ") " + drag.transform);
        return;
      }
      view.x = drag.vx + dx;
      view.y = drag.vy + dy;
      applyTransform();
    });
    window.addEventListener("mouseup", function (e) {
      if (!drag) return;
      var moved = drag.moved;
      var d = drag;
      drag = null;
      viewport.classList.remove("dragging");
      if (d.node) {
        d.node.classList.remove("odg-dragging");
        if (moved && d.centre) {
          var x = Math.round(d.centre.x + (e.clientX - d.x) / view.scale);
          var y = Math.round(d.centre.y + (e.clientY - d.y) / view.scale);
          post({ op: "move", element_id: d.id, x: x, y: y });
          return;
        }
      }
      if (moved) return;
      if (spotlight.active) clearSpotlight();
      var marker = e.target.closest && e.target.closest(".odg-marker");
      if (marker) return post({ op: "select-problem", index: +marker.getAttribute("data-problem") });
      var node = e.target.closest && e.target.closest("[data-id]");
      var isEdge = node && (node.getAttribute("data-et") === "edge" || !!node.closest(".edgeLabel"));
      if (node && !isEdge) {
        if (window.odgPalette) window.odgPalette.select(node.getAttribute("data-id"));
        highlight(node.getAttribute("data-id"));
        post({ op: "select", element_id: node.getAttribute("data-id") });
      }
      if (node && editing && window.odgPalette) window.odgPalette.inspect(node.getAttribute("data-id"), isEdge, e.clientX, e.clientY);
    });
    viewport.addEventListener("wheel", function (e) {
      e.preventDefault();
      if (e.ctrlKey || e.metaKey) {
        var r = viewport.getBoundingClientRect();
        zoom(Math.exp(-e.deltaY * 0.01), e.clientX - r.left, e.clientY - r.top);
      } else {
        view.x -= e.deltaX;
        view.y -= e.deltaY;
        applyTransform();
      }
    }, { passive: false });
    document.getElementById("toolbar").addEventListener("click", function (e) {
      var act = e.target.getAttribute("data-act");
      if (act === "zoom-in") zoom(1.25);
      else if (act === "zoom-out") zoom(0.8);
      else if (act === "fit") fit();
    });
    window.addEventListener("resize", function () { if (view.svg && view.fitted) fit(); });
    document.getElementById("modebar").addEventListener("click", function (e) {
      var b = e.target.closest && e.target.closest("button");
      if (!b || b.disabled) return;
      if (b.getAttribute("data-mode")) setEditing(b.getAttribute("data-mode") === "edit", true);
      else if (b.getAttribute("data-act") === "undo") undo();
      else if (b.getAttribute("data-act") === "redo") redo();
    });
    // Cmd/Ctrl+Z, Shift+Cmd/Ctrl+Z, Ctrl+Y in the preview (form fields keep their own undo)
    document.addEventListener("keydown", function (e) {
      if (!(e.metaKey || e.ctrlKey) || e.altKey) return;
      var tag = (e.target && e.target.tagName || "").toLowerCase();
      if (tag === "input" || tag === "textarea" || tag === "select" || (e.target && e.target.isContentEditable)) return;
      var k = String(e.key).toLowerCase();
      if (k === "z" && !e.shiftKey) { e.preventDefault(); undo(); }
      else if ((k === "z" && e.shiftKey) || (k === "y" && e.ctrlKey)) { e.preventDefault(); redo(); }
    });
    setEditing(readStoredMode(), false);
    showEmpty();
  }

  // ---------------------------------------------------------------- agent ⇄ human
  // Messages agents send through MCP (show_diagram, highlight_elements, notify_user, ask_user) arrive from
  // the host as { op: "agent", message, switched }. Elements are put in evidence (glow + dim the rest) and
  // a card shows the note / question; answers go back as { op: "answer", question_id, text }.
  // req: the last request, kept so the evidence survives re-renders (an agent edit reloads the drawing)
  var spotlight = { pending: null, active: false, els: [], req: null, at: 0 };
  var shownMessages = {};

  function tr(key, fallback) { return strings[key] || fallback; }

  function clearSpotlight() {
    spotlight.active = false;
    spotlight.req = null;
    if (!view.svg) return;
    view.svg.classList.remove("odg-dimmed");
    var lit = view.svg.querySelectorAll(".odg-spotlight");
    for (var i = 0; i < lit.length; i++) lit[i].classList.remove("odg-spotlight", "lvl-info", "lvl-success", "lvl-warning", "lvl-error");
  }

  function applySpotlight() {
    var sp = spotlight.pending;
    if (!sp || !view.svg) return;
    var found = [];
    sp.ids.forEach(function (id) {
      var el = view.svg.querySelector('[data-id="' + CSS.escape(id) + '"]');
      if (el) found.push(el);
    });
    if (!found.length) {
      // the diagram may still be rendering: keep the request for the next couple of renders
      if (++sp.tries > 2) spotlight.pending = null;
      return;
    }
    spotlight.pending = null;
    clearSpotlight();
    view.svg.classList.add("odg-dimmed");
    found.forEach(function (el) { el.classList.add("odg-spotlight", "lvl-" + sp.level); });
    spotlight.active = true;
    spotlight.els = found;
    spotlight.req = { ids: sp.ids, level: sp.level };
    // centre now, and again on the re-renders of the next few seconds (not later: the human may be editing)
    if (!sp.quiet) {
      spotlight.at = Date.now();
      focusOn(found);
    } else if (Date.now() - spotlight.at < 4000) {
      focusOn(found);
    }
  }

  /** After a new drawing replaced the old one: light the same elements again. */
  function restoreSpotlight() {
    if (!spotlight.active || !spotlight.req) return;
    spotlight.pending = { ids: spotlight.req.ids, level: spotlight.req.level, tries: 2, quiet: true };
    applySpotlight();
  }

  /** Zooms out if needed and pans so every element is visible, centred. */
  function focusOn(els) {
    function union() {
      var r = { l: Infinity, t: Infinity, r: -Infinity, b: -Infinity };
      els.forEach(function (el) {
        var b = el.getBoundingClientRect();
        r.l = Math.min(r.l, b.left); r.t = Math.min(r.t, b.top); r.r = Math.max(r.r, b.right); r.b = Math.max(r.b, b.bottom);
      });
      return r;
    }
    var vp = viewport.getBoundingClientRect();
    var vr = { left: vp.left, top: vp.top, width: vp.width, height: vp.height };
    var cards = document.getElementById("agent-cards");
    if (cards && cards.children.length) {
      var free = cards.getBoundingClientRect().left - 12 - vp.left;
      if (free > 220) vr.width = free;
    }
    var u = union();
    var f = Math.min(1, (vr.width * 0.7) / Math.max(u.r - u.l, 1), (vr.height * 0.6) / Math.max(u.b - u.t, 1));
    if (f < 1) zoom(f, (u.l + u.r) / 2 - vr.left, (u.t + u.b) / 2 - vr.top);
    u = union();
    view.x += vr.left + vr.width / 2 - (u.l + u.r) / 2;
    view.y += vr.top + vr.height / 2 - (u.t + u.b) / 2;
    applyTransform();
  }

  // set_zoom / center_on_element: { zoom?, fit?, center_on? }
  var agentView = { pending: null, last: null };

  function applyView() {
    var v = agentView.pending;
    if (!v || !view.svg) return;
    agentView.pending = null;
    agentView.last = v;
    if (v.fit) fit();
    if (v.zoom) zoom(v.zoom / view.scale);
    if (v.center) {
      var el = view.svg.querySelector('[data-id="' + CSS.escape(v.center) + '"]');
      if (el) {
        centerOn(el);
        el.classList.remove("odg-pulse");
        void el.getBoundingClientRect(); // restart the animation
        el.classList.add("odg-pulse");
        setTimeout(function () { el.classList.remove("odg-pulse"); }, 2600);
      }
    }
    applyTransform();
  }

  // select_element: the same as the human clicking the element (inspector, editor line, presence)
  var agentSelect = { pending: null };

  function applySelect() {
    var sel = agentSelect.pending;
    if (!sel || !view.svg) return;
    agentSelect.pending = null;
    if (!sel.id) {
      highlight(null);
      if (window.odgPalette) window.odgPalette.clear();
      post({ op: "select", element_id: null });
      return;
    }
    var el = view.svg.querySelector('[data-id="' + CSS.escape(sel.id) + '"]');
    if (!el) return;
    if (window.odgPalette) window.odgPalette.select(sel.id);
    highlight(sel.id);
    post({ op: "select", element_id: sel.id });
    if (agentView.pending) applyView();
  }

  /** Pans (no zoom change) so the element sits in the middle of the free part of the preview. */
  function centerOn(el) {
    var vp = viewport.getBoundingClientRect();
    var width = vp.width;
    var cards = document.getElementById("agent-cards");
    if (cards && cards.children.length) {
      var free = cards.getBoundingClientRect().left - 12 - vp.left;
      if (free > 220) width = free;
    }
    var b = el.getBoundingClientRect();
    view.x += vp.left + width / 2 - (b.left + b.right) / 2;
    view.y += vp.top + vp.height / 2 - (b.top + b.bottom) / 2;
    applyTransform();
  }

  function agentCards() {
    var box = document.getElementById("agent-cards");
    if (!box) {
      box = document.createElement("div");
      box.id = "agent-cards";
      document.body.appendChild(box);
    }
    return box;
  }

  function el(tag, cls, text) {
    var e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text != null) e.textContent = text;
    return e;
  }

  function dismiss(card) {
    if (!card.parentNode) return;
    if (card.dataset.spotlight && spotlight.active) clearSpotlight();
    card.classList.add("leaving");
    setTimeout(function () { if (card.parentNode) card.parentNode.removeChild(card); }, 200);
  }

  function agentMessage(m, switched) {
    if (!m || (m.id && shownMessages[m.id])) return;
    if (m.id) shownMessages[m.id] = true;
    if (m.kind === "select") {
      if (m.text) addCard(m);
      agentSelect.pending = { id: (m.element_ids || [])[0] || null, center: m.center_on != null };
      if (m.center_on) agentView.pending = { center: m.center_on, at: Date.now() };
      if (!switched) applySelect();
      return;
    }
    if (m.kind === "view") {
      if (m.text) addCard(m);
      agentView.pending = { zoom: m.zoom, fit: m.fit, center: m.center_on, at: Date.now() };
      if (!switched) applyView();
      return;
    }
    var ids = m.element_ids || [];
    // card first, so the spotlight can centre the elements in the space the cards leave free
    if (m.text) addCard(m);
    if (ids.length) {
      spotlight.pending = { ids: ids, level: m.level || "info", tries: 0 };
      // a diagram switch is rendering: the new drawing applies it (showSvg)
      if (!switched) applySpotlight();
    }
  }

  function addCard(m) {
    var box = agentCards();
    var level = m.level || (m.kind === "ask" ? "info" : "info");
    var card = el("div", "agent-card lvl-" + level + (m.kind === "ask" ? " ask" : ""));
    if ((m.element_ids || []).length) card.dataset.spotlight = "1";
    var head = el("div", "agent-head");
    head.appendChild(el("span", "agent-icon", m.kind === "ask" ? "?" : "\u2726"));
    head.appendChild(el("b", null, m.kind === "ask" ? tr("ui-agent-question", "Question from the agent") : tr("ui-agent", "Agent")));
    if (m.client) head.appendChild(el("span", "agent-meta", m.client));
    var close = el("button", "agent-close", "\u00d7");
    close.title = tr("ui-agent-close", "Close");
    close.onclick = function () { dismiss(card); };
    head.appendChild(close);
    card.appendChild(head);
    if (m.path) card.appendChild(el("div", "agent-where", m.path + ((m.element_ids || []).length ? " \u00b7 " + m.element_ids.join(", ") : "")));
    if (m.text) card.appendChild(el("div", "agent-text", m.text));

    if (m.kind === "ask") {
      var send = function (answer) {
        answer = String(answer || "").trim();
        if (!answer) return;
        post({ op: "answer", question_id: m.id, text: answer });
        var inputs = card.querySelectorAll(".agent-options, .agent-reply");
        for (var i = 0; i < inputs.length; i++) card.removeChild(inputs[i]);
        card.appendChild(el("div", "agent-sent", "\u2713 " + tr("ui-agent-sent", "Answer sent") + ": " + answer));
        setTimeout(function () { dismiss(card); }, 4000);
      };
      if ((m.options || []).length) {
        var opts = el("div", "agent-options");
        m.options.forEach(function (o) {
          var b = el("button", null, o);
          b.onclick = function () { send(o); };
          opts.appendChild(b);
        });
        card.appendChild(opts);
      }
      if (m.allow_text !== false) {
        var form = el("form", "agent-reply");
        var input = el("input");
        input.placeholder = tr("ui-agent-reply-placeholder", "Type your answer\u2026");
        var go = el("button", "primary", tr("ui-agent-send", "Send"));
        go.type = "submit";
        form.appendChild(input);
        form.appendChild(go);
        form.onsubmit = function (e) { e.preventDefault(); send(input.value); };
        card.appendChild(form);
        setTimeout(function () { input.focus(); }, 50);
      }
    } else if (m.kind === "notify" && (level === "info" || level === "success")) {
      // plain notes fade out by themselves unless the pointer is on them
      var timer = setTimeout(function () { dismiss(card); }, 20000);
      card.onmouseenter = function () { clearTimeout(timer); };
      card.onmouseleave = function () { timer = setTimeout(function () { dismiss(card); }, 6000); };
    }
    box.insertBefore(card, box.firstChild);
    while (box.children.length > 3) box.removeChild(box.lastChild);
  }

  // ---------------------------------------------------------------- dispatch
  function handle(msg) {
    if (typeof msg === "string") msg = JSON.parse(msg);
    var op = msg.op;
    if (op === "process") {
      queue = queue.then(function () {
        return processDiagram(msg).then(function (result) {
          if (mode === "viewer") {
            if (!String(msg.source || "").trim()) {
              showEmpty();
              if (window.odgPalette) window.odgPalette.clear();
            }
            else if (result.parse_error) showError(result.parse_error);
            else if (result.render_error) showError(result.render_error);
          }
          post({ op: "result", id: msg.id, result: result });
        }, function (e) {
          post({ op: "result", id: msg.id, result: { ok: false, render_error: normalizeError(e, "render") } });
        });
      });
      return;
    }
    if (op === "rasterize") {
      queue = queue.then(function () {
        return rasterize(msg.svg, msg.scale, msg.background).then(function (r) {
          post({ op: "png", id: msg.id, png: r.png, width: r.width, height: r.height });
        }, function (e) {
          post({ op: "png", id: msg.id, error: String((e && e.message) || e) });
        });
      });
      return;
    }
    if (op === "show") return drawMarkers(msg.problems);
    if (op === "highlight") return highlight(msg.element_id);
    if (op === "agent") return agentMessage(msg.message, msg.switched);
    if (op === "fit") {
      fit();
      // hosts fit a freshly opened diagram after it renders: keep the agent's elements in view
      if (spotlight.active && spotlight.els.length && spotlight.els[0].isConnected) focusOn(spotlight.els);
      // an agent's zoom / centre from a moment ago wins over the host's automatic fit
      if (agentView.last && Date.now() - agentView.last.at < 4000) {
        agentView.pending = agentView.last;
        applyView();
      }
      return;
    }
    if (op === "zoom") return zoom(msg.factor || 1.25);
    if (op === "theme") {
      document.documentElement.setAttribute("data-theme", msg.theme === "dark" ? "dark" : "light");
      return;
    }
    if (op === "strings") {
      Object.assign(strings, msg.strings || {});
      if (strings["ui-fit"]) strings.fit = strings["ui-fit"];
      if (strings["ui-empty"]) strings.empty = strings["ui-empty"];
      var fb = document.getElementById("fit-btn");
      if (fb) fb.textContent = strings.fit;
      [["auto", "ui-layout-auto"], ["elk", "ui-layout-elk"], ["elk-interactive", "ui-layout-interactive"], ["exact", "ui-layout-exact"]].forEach(function (p) {
        var o = layoutSelect.querySelector('option[value="' + p[0] + '"]');
        if (o && strings[p[1]]) o.textContent = strings[p[1]];
      });
      if (strings["ui-reset-positions"]) resetButton.title = strings["ui-reset-positions"];
      if (strings["ui-layout"]) layoutSelect.title = strings["ui-layout"];
      var mk = function (id, key, tip) {
        var b = document.getElementById(id);
        if (!b) return;
        var span = b.querySelector("span");
        if (span && strings[key]) span.textContent = strings[key];
        if (strings[tip || key]) b.title = strings[tip || key];
      };
      mk("mode-edit", "ui-mode-edit", "ui-mode-edit-tip");
      mk("mode-view", "ui-mode-view", "ui-mode-view-tip");
      mk("undo-btn", "ui-undo", "ui-undo-tip");
      mk("redo-btn", "ui-redo", "ui-redo-tip");
      if (window.odgPalette) window.odgPalette.strings(strings);
      if (!view.svg) document.getElementById("empty").textContent = strings.empty;
      return;
    }
    if (op === "ping") return post({ op: "pong", id: msg.id });
  }

  window.odg = { handle: handle, post: post, version: 1 };
  if (mode === "viewer") setupViewer();
  post({ op: "ready", mode: mode, mermaid_version: (window.mermaid && mermaid.version) || null });
  if (mode === "viewer") post({ op: "edit-mode", editing: editing }); // the host makes its editor read-only in view mode
})();
