//! Visual-quality geometry on the rendered model: edge crossings, edges drawn through other nodes,
//! labels covering nodes or other labels, edges too short for their arrowhead.
//! Edges carry sampled `points` (render-kit) in the same user coordinates as the node boxes.

use crate::model::{BBox, DiagramModel, Edge};

type P = [f64; 2];

fn segments(e: &Edge) -> impl Iterator<Item = (P, P)> + '_ {
    e.points.windows(2).map(|w| (w[0], w[1]))
}

/// Intersection of segments ab and cd; half-open ([0, 1)) so a crossing exactly at a sample point counts once.
fn intersect(a: P, b: P, c: P, d: P) -> Option<P> {
    let r = [b[0] - a[0], b[1] - a[1]];
    let s = [d[0] - c[0], d[1] - c[1]];
    let den = r[0] * s[1] - r[1] * s[0];
    if den.abs() < 1e-9 {
        return None; // parallel or collinear: overlapping routes are not counted as crossings
    }
    let t = ((c[0] - a[0]) * s[1] - (c[1] - a[1]) * s[0]) / den;
    let u = ((c[0] - a[0]) * r[1] - (c[1] - a[1]) * r[0]) / den;
    ((0.0..1.0).contains(&t) && (0.0..1.0).contains(&u)).then(|| [a[0] + t * r[0], a[1] + t * r[1]])
}

fn dist(a: P, b: P) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// Crossings between edges that do not share a node, ignoring points next to an edge's ends:
/// (index of edge a, index of edge b, where).
pub fn crossings(model: &DiagramModel) -> Vec<(usize, usize, P)> {
    let mut out = vec![];
    let edges: Vec<(usize, &Edge)> = model.edges.iter().enumerate().filter(|(_, e)| e.points.len() > 1).take(300).collect();
    for (i, (ia, a)) in edges.iter().enumerate() {
        for (ib, b) in &edges[i + 1..] {
            if a.from == b.from || a.from == b.to || a.to == b.from || a.to == b.to {
                continue;
            }
            let ends = [a.points[0], *a.points.last().unwrap(), b.points[0], *b.points.last().unwrap()];
            let mut found: Vec<P> = vec![];
            for (p, q) in segments(a) {
                for (r, s) in segments(b) {
                    if let Some(x) = intersect(p, q, r, s) {
                        if ends.iter().all(|e| dist(*e, x) > 8.0) && found.iter().all(|f| dist(*f, x) > 4.0) {
                            found.push(x);
                        }
                    }
                }
            }
            out.extend(found.into_iter().map(|x| (*ia, *ib, x)));
        }
    }
    out
}

fn inside(b: &BBox, p: P, margin: f64) -> bool {
    p[0] > b.x + margin && p[0] < b.x + b.w - margin && p[1] > b.y + margin && p[1] < b.y + b.h - margin
}

/// Edges drawn across a node that is not one of their ends: (edge index, node index).
pub fn edges_through_nodes(model: &DiagramModel) -> Vec<(usize, usize)> {
    let mut out = vec![];
    for (ei, e) in model.edges.iter().enumerate() {
        if e.points.len() < 2 {
            continue;
        }
        for (ni, n) in model.nodes.iter().enumerate() {
            if n.id == e.from || n.id == e.to {
                continue;
            }
            let Some(b) = n.bbox.filter(|b| b.w > 8.0 && b.h > 8.0) else { continue };
            // samples and segment midpoints inside the box: two or more mean the path really goes through
            let hits = e.points.iter().filter(|p| inside(&b, **p, 3.0)).count()
                + segments(e).filter(|(p, q)| inside(&b, [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0], 3.0)).count();
            if hits >= 2 {
                out.push((ei, ni));
            }
        }
    }
    out
}

/// Edge labels covering a node: (edge index, node index, share of the label covered).
pub fn labels_over_nodes(model: &DiagramModel) -> Vec<(usize, usize, f64)> {
    let mut out = vec![];
    for (ei, e) in model.edges.iter().enumerate() {
        let Some(l) = e.label_bbox.filter(|l| l.area() > 0.0) else { continue };
        for (ni, n) in model.nodes.iter().enumerate() {
            let Some(b) = n.bbox else { continue };
            let share = l.intersection(&b) / l.area();
            if share > 0.15 {
                out.push((ei, ni, share));
            }
        }
    }
    out
}

/// Pairs of edge labels drawn over each other.
pub fn overlapping_labels(model: &DiagramModel) -> Vec<(usize, usize)> {
    let labelled: Vec<(usize, BBox)> = model.edges.iter().enumerate().filter_map(|(i, e)| e.label_bbox.filter(|l| l.area() > 0.0).map(|l| (i, l))).collect();
    let mut out = vec![];
    for (k, (i, a)) in labelled.iter().enumerate() {
        for (j, b) in &labelled[k + 1..] {
            if a.intersection(b) / a.area().min(b.area()) > 0.2 {
                out.push((*i, *j));
            }
        }
    }
    out
}

/// Edges whose drawn length leaves no room for the arrowhead (it can look reversed or vanish).
pub fn short_edges(model: &DiagramModel) -> Vec<usize> {
    model.edges.iter().enumerate().filter(|(_, e)| e.length.is_some_and(|l| l < 12.0)).map(|(i, _)| i).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Node;

    fn node(id: &str, x: f64, y: f64) -> Node {
        Node { id: id.into(), bbox: Some(BBox { x, y, w: 100.0, h: 40.0 }), ..Default::default() }
    }

    fn edge(id: &str, from: &str, to: &str, pts: &[P]) -> Edge {
        let length = pts.windows(2).map(|w| dist(w[0], w[1])).sum();
        Edge { id: id.into(), from: from.into(), to: to.into(), points: pts.to_vec(), length: Some(length), ..Default::default() }
    }

    #[test]
    fn finds_crossings_through_nodes_and_short_edges() {
        let line = |a: P, b: P| (0..=20).map(|i| [a[0] + (b[0] - a[0]) * i as f64 / 20.0, a[1] + (b[1] - a[1]) * i as f64 / 20.0]).collect::<Vec<_>>();
        let model = DiagramModel {
            nodes: vec![node("a", 0.0, 0.0), node("b", 200.0, 0.0), node("c", 400.0, 0.0), node("d", 200.0, 200.0), node("e", 200.0, -200.0)],
            edges: vec![
                edge("ac", "a", "c", &line([100.0, 20.0], [400.0, 20.0])), // passes through b
                edge("de", "d", "e", &line([250.0, 200.0], [250.0, -160.0])), // crosses ac (inside b, fine)
                edge("cb", "c", "b", &[[302.0, 20.0], [300.0, 20.0]]),        // 2 px long
            ],
            groups: vec![],
        };
        assert_eq!(crossings(&model).len(), 1);
        let through: Vec<(&str, &str)> = edges_through_nodes(&model).iter().map(|(e, n)| (model.edges[*e].id.as_str(), model.nodes[*n].id.as_str())).collect();
        assert!(through.contains(&("ac", "b")));
        assert!(through.contains(&("de", "b")));
        assert_eq!(short_edges(&model), vec![2]);
    }
}
