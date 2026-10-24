// ODiagramador layouts for Mermaid graph diagrams (flowchart, state, class, ER, requirement).
//
//   odg-elk          ELK layered, automatic (also registered as "elk" so `layout: elk` works offline)
//   odg-interactive  ELK layered in INTERACTIVE mode: positions dragged by the user are hints;
//                    ELK keeps their layer/order and still routes every edge
//   odg-exact        nodes with a position stay exactly there; the rest comes from ELK
//
// `%% @odg layer <id> first|last|<n>`: first/last pin a node to the first/last layer; numbers order the
// layers of nodes and subgraphs (0 before 1 before 2…) through ELK partitioning.
//
// Positions arrive from `%% @odg pos <id> <x>,<y>` comments (parsed by bridge.js into
// layoutHints). Coordinates are node centres in the diagram's SVG user space — the same space as
// the bounding boxes the bridge reports — so a dragged node lands where it was dropped.
import ELK from "elkjs/lib/elk.bundled.js";

const elk = new ELK();

/** Filled by the bridge right before each render. */
export const layoutHints = { pos: {}, layer: {} };

const DIRECTION = { TB: "DOWN", TD: "DOWN", BT: "UP", LR: "RIGHT", RL: "LEFT" };
const GROUP_PADDING = "[top=34,left=14,bottom=14,right=14]";
const INTERACTIVE = {
  "elk.layered.layering.strategy": "INTERACTIVE",
  "elk.layered.crossingMinimization.strategy": "INTERACTIVE",
  "elk.layered.cycleBreaking.strategy": "INTERACTIVE",
};

function baseOptions(data) {
  const cfg = data.config || {};
  const flow = cfg.flowchart || {};
  return {
    "elk.algorithm": "layered",
    "elk.direction": DIRECTION[data.direction] || "DOWN",
    "elk.hierarchyHandling": "INCLUDE_CHILDREN",
    "elk.spacing.nodeNode": String(flow.nodeSpacing ?? 50),
    "elk.layered.spacing.nodeNodeBetweenLayers": String(flow.rankSpacing ?? 50),
    "elk.layered.spacing.edgeNodeBetweenLayers": "20",
    "elk.edgeRouting": "ORTHOGONAL",
    "elk.layered.unnecessaryBendpoints": "true",
    "elk.layered.considerModelOrder.strategy": "NODES_AND_EDGES",
  };
}

/** Builds the ELK graph. `abs` = absolute top-left positions to feed interactive mode. */
function buildGraph(data, abs, interactive) {
  const opts = baseOptions(data);
  const root = { id: "__root", layoutOptions: interactive ? { ...opts, ...INTERACTIVE } : opts, children: [], edges: [] };
  const elkNodes = new Map();
  const partitioned = new Set(); // graphs (root or groups) with numbered layers among their children
  for (const n of data.nodes) {
    // never send `width: undefined`: elkjs crashes on explicit undefined sizes
    const node = { id: n.id, layoutOptions: {} };
    if (!n.isGroup) {
      node.width = Math.max(1, n.width || 1);
      node.height = Math.max(1, n.height || 1);
    }
    if (n.isGroup) {
      node.children = [];
      node.layoutOptions["elk.padding"] = GROUP_PADDING;
      if (interactive) Object.assign(node.layoutOptions, INTERACTIVE);
      if (n.dir && DIRECTION[n.dir]) node.layoutOptions["elk.direction"] = DIRECTION[n.dir];
    }
    const layer = layoutHints.layer[n.id];
    if (layer === "first") node.layoutOptions["elk.layered.layering.layerConstraint"] = "FIRST";
    if (layer === "last") node.layoutOptions["elk.layered.layering.layerConstraint"] = "LAST";
    // numbered layers (0, 1, 2…): ELK partitions keep that order even when arrows point backwards
    if (/^\d+$/.test(String(layer))) {
      node.layoutOptions["elk.partitioning.partition"] = String(layer);
      partitioned.add(n.parentId || root.id);
    }
    elkNodes.set(n.id, node);
  }
  for (const n of data.nodes) {
    const parent = n.parentId && elkNodes.get(n.parentId);
    (parent ? parent.children : root.children).push(elkNodes.get(n.id));
  }
  for (const id of partitioned) {
    const graph = id === root.id ? root : elkNodes.get(id);
    if (graph) graph.layoutOptions["elk.partitioning.activate"] = "true";
  }
  if (abs) {
    // ELK wants coordinates relative to the parent
    for (const n of data.nodes) {
      const p = abs.get(n.id);
      if (!p) continue;
      const parentPos = n.parentId ? abs.get(n.parentId) : null;
      const node = elkNodes.get(n.id);
      node.x = p.x - (parentPos ? parentPos.x : 0);
      node.y = p.y - (parentPos ? parentPos.y : 0);
    }
  }
  for (const e of data.edges) {
    if (!elkNodes.has(e.start) || !elkNodes.has(e.end)) continue;
    const edge = { id: e.id, sources: [e.start], targets: [e.end] };
    if (e.label && (e.width || e.height)) edge.labels = [{ text: String(e.label), width: e.width || 0, height: e.height || 0 }];
    root.edges.push(edge);
  }
  return root;
}

/** Absolute top-left + size of every node in an ELK result. */
function flatten(graph) {
  const out = new Map();
  const walk = (node, ox, oy) => {
    for (const c of node.children || []) {
      const x = ox + (c.x || 0);
      const y = oy + (c.y || 0);
      out.set(c.id, { x, y, w: c.width || 0, h: c.height || 0 });
      walk(c, x, y);
    }
  };
  walk(graph, 0, 0);
  return out;
}

function edgeContainerOffset(graph, edge, abs) {
  // elkjs reports edge coordinates relative to the edge's container
  const container = edge.container && edge.container !== "__root" ? abs.get(edge.container) : null;
  return container ? { x: container.x, y: container.y } : { x: 0, y: 0 };
}

function applyNodes(data, abs) {
  for (const n of data.nodes) {
    const b = abs.get(n.id);
    if (!b) continue;
    n.x = b.x + b.w / 2;
    n.y = b.y + b.h / 2;
    if (n.isGroup) {
      n.width = b.w;
      n.height = b.h;
    }
  }
}

// Edge labels are placed at edge.x/edge.y: use the point halfway along the route.
function setLabelPoint(e, pts) {
  let total = 0;
  for (let i = 1; i < pts.length; i++) total += Math.hypot(pts[i].x - pts[i - 1].x, pts[i].y - pts[i - 1].y);
  let left = total / 2;
  for (let i = 1; i < pts.length; i++) {
    const len = Math.hypot(pts[i].x - pts[i - 1].x, pts[i].y - pts[i - 1].y);
    if (left <= len && len > 0) {
      const t = left / len;
      e.x = pts[i - 1].x + (pts[i].x - pts[i - 1].x) * t;
      e.y = pts[i - 1].y + (pts[i].y - pts[i - 1].y) * t;
      return;
    }
    left -= len;
  }
  e.x = pts[0].x;
  e.y = pts[0].y;
}

/** True when a route is orthogonal (ELK's interactive mode can return broken routes across groups). */
function isOrthogonal(pts) {
  for (let i = 1; i < pts.length; i++) {
    if (Math.abs(pts[i].x - pts[i - 1].x) > 0.5 && Math.abs(pts[i].y - pts[i - 1].y) > 0.5) return false;
  }
  return pts.length >= 2;
}

/** Simple orthogonal connector between two boxes (used for moved nodes). */
function elbowRoute(a, b) {
  const ac = { x: a.x + a.w / 2, y: a.y + a.h / 2 };
  const bc = { x: b.x + b.w / 2, y: b.y + b.h / 2 };
  if (a === b) {
    const r = a.x + a.w;
    return [{ x: r, y: ac.y - 6 }, { x: r + 24, y: ac.y - 6 }, { x: r + 24, y: a.y - 18 }, { x: ac.x, y: a.y - 18 }, { x: ac.x, y: a.y }];
  }
  const vGap = bc.y > ac.y ? b.y - (a.y + a.h) : a.y - (b.y + b.h);
  const hGap = bc.x > ac.x ? b.x - (a.x + a.w) : a.x - (b.x + b.w);
  if (vGap >= hGap) {
    const down = bc.y > ac.y;
    const s = { x: ac.x, y: down ? a.y + a.h : a.y };
    const t = { x: bc.x, y: down ? b.y : b.y + b.h };
    if (Math.abs(s.x - t.x) < 1) return [s, t];
    const mid = (s.y + t.y) / 2;
    return [s, { x: s.x, y: mid }, { x: t.x, y: mid }, t];
  }
  const right = bc.x > ac.x;
  const s = { x: right ? a.x + a.w : a.x, y: ac.y };
  const t = { x: right ? b.x : b.x + b.w, y: bc.y };
  if (Math.abs(s.y - t.y) < 1) return [s, t];
  const mid = (s.x + t.x) / 2;
  return [s, { x: mid, y: s.y }, { x: mid, y: t.y }, t];
}

function applyElbowEdges(data, abs) {
  for (const e of data.edges) {
    const a = abs.get(e.start);
    const b = abs.get(e.end);
    if (!a || !b) {
      straightEdge(data, e);
      continue;
    }
    const pts = elbowRoute(a, e.start === e.end ? a : b);
    e.points = pts;
    e.odgRouted = true;
    e.curve = "rounded";
    setLabelPoint(e, pts);
  }
}

function applyRoutedEdges(data, graph, abs) {
  const byId = new Map((graph.edges || []).map((e) => [e.id, e]));
  for (const e of data.edges) {
    const routed = byId.get(e.id);
    const s = routed && routed.sections && routed.sections[0];
    if (!s) {
      straightEdge(data, e);
      continue;
    }
    const o = edgeContainerOffset(graph, routed, abs);
    const pts = [s.startPoint, ...(s.bendPoints || []), s.endPoint].map((p) => ({ x: p.x + o.x, y: p.y + o.y }));
    if (!isOrthogonal(pts)) {
      const a = abs.get(e.start);
      const b = abs.get(e.end);
      if (a && b) {
        const r = elbowRoute(a, e.start === e.end ? a : b);
        e.points = r;
        e.odgRouted = true;
        e.curve = "rounded";
        setLabelPoint(e, r);
        continue;
      }
    }
    // ELK routes already start and end on the shape borders: paint them as they are
    e.points = pts;
    e.odgRouted = true;
    e.curve = "rounded"; // orthogonal ELK routes: basis smoothing would loop on short segments
    setLabelPoint(e, pts);
  }
}

function straightEdge(data, e) {
  const a = data.nodes.find((n) => n.id === e.start);
  const b = data.nodes.find((n) => n.id === e.end);
  if (!a || !b) return;
  // dagre convention: centre → middle → centre; Mermaid clips the ends at the shapes
  e.odgRouted = false;
  e.points = [
    { x: a.x, y: a.y },
    { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 },
    { x: b.x, y: b.y },
  ];
  setLabelPoint(e, e.points);
}

/** Grows every group to contain its children (deepest groups first). */
function fitGroups(data, abs) {
  const depth = (n) => {
    let d = 0;
    let p = n.parentId;
    while (p) {
      d++;
      p = data.nodes.find((x) => x.id === p)?.parentId;
    }
    return d;
  };
  const groups = data.nodes.filter((n) => n.isGroup).sort((a, b) => depth(b) - depth(a));
  for (const g of groups) {
    const kids = data.nodes.filter((n) => n.parentId === g.id).map((n) => abs.get(n.id)).filter(Boolean);
    if (!kids.length) continue;
    const x0 = Math.min(...kids.map((k) => k.x)) - 14;
    const y0 = Math.min(...kids.map((k) => k.y)) - 34;
    const x1 = Math.max(...kids.map((k) => k.x + k.w)) + 14;
    const y1 = Math.max(...kids.map((k) => k.y + k.h)) + 14;
    abs.set(g.id, { x: x0, y: y0, w: x1 - x0, h: y1 - y0 });
  }
}

/** Overrides the positions of hinted nodes (hints are centres). */
function applyHints(data, abs) {
  let any = false;
  for (const n of data.nodes) {
    const h = layoutHints.pos[n.id];
    const b = abs.get(n.id);
    if (!h || !b || n.isGroup) continue;
    abs.set(n.id, { ...b, x: h[0] - b.w / 2, y: h[1] - b.h / 2 });
    any = true;
  }
  return any;
}

async function autoLayout(data) {
  const graph = await elk.layout(buildGraph(data, null, false));
  return { graph, abs: flatten(graph) };
}

const paintOptions = { skipIntersect: (edge) => edge.odgRouted === true };

export function createLayouts(createCommonLayoutRenderer) {
  const auto = createCommonLayoutRenderer({
    paintOptions,
    runLayoutCore: async (data) => {
      const { graph, abs } = await autoLayout(data);
      applyNodes(data, abs);
      applyRoutedEdges(data, graph, abs);
    },
  });

  const interactive = createCommonLayoutRenderer({
    paintOptions,
    runLayoutCore: async (data) => {
      let { graph, abs } = await autoLayout(data);
      if (applyHints(data, abs)) {
        fitGroups(data, abs);
        graph = await elk.layout(buildGraph(data, abs, true));
        abs = flatten(graph);
        fitGroups(data, abs); // interactive mode may leave a moved child outside its group
      }
      applyNodes(data, abs);
      applyRoutedEdges(data, graph, abs);
    },
  });

  const exact = createCommonLayoutRenderer({
    paintOptions,
    runLayoutCore: async (data) => {
      const first = await autoLayout(data);
      const abs = first.abs;
      const moved = applyHints(data, abs);
      fitGroups(data, abs);
      applyNodes(data, abs);
      if (moved) {
        // pinned nodes: simple orthogonal connectors between the final boxes
        applyElbowEdges(data, abs);
      } else {
        applyRoutedEdges(data, first.graph, abs);
      }
    },
  });

  const loader = (render) => async () => ({ render });
  return [
    { name: "elk", loader: loader(auto) },
    { name: "odg-elk", loader: loader(auto) },
    { name: "odg-interactive", loader: loader(interactive) },
    { name: "odg-exact", loader: loader(exact) },
  ];
}
