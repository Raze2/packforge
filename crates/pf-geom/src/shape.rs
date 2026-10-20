//! Building paths from vertices with rounded corners and explicit arcs.

use crate::{Path, Point, Segment, EPS};
use std::f64::consts::{PI, TAU};

/// One resolved entry of a path description.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Node {
    /// A vertex; `round > 0` replaces the corner with a tangent arc of that radius.
    Corner { p: Point, round: f64 },
    /// A circular arc from the previous node to `p`.
    ArcTo {
        p: Point,
        radius: f64,
        ccw: bool,
        large: bool,
    },
}

impl Node {
    fn point(&self) -> Point {
        match *self {
            Node::Corner { p, .. } | Node::ArcTo { p, .. } => p,
        }
    }
}

/// Result of [`build`]: the path plus notes about adjustments (e.g. a radius that was reduced).
#[derive(Debug, Clone, PartialEq)]
pub struct Built {
    pub path: Path,
    pub notes: Vec<String>,
}

fn sub(a: Point, b: Point) -> (f64, f64) {
    (a.x - b.x, a.y - b.y)
}

fn len(v: (f64, f64)) -> f64 {
    v.0.hypot(v.1)
}

fn angle_of(c: Point, p: Point) -> f64 {
    (p.y - c.y).atan2(p.x - c.x)
}

/// Arc from `a` to `b` with `radius` (SVG semantics). A radius too small for the chord is
/// enlarged to half the chord, like SVG renderers do.
pub fn arc_between(
    a: Point,
    b: Point,
    radius: f64,
    ccw: bool,
    large: bool,
) -> Result<(Segment, Option<String>), String> {
    let chord = sub(b, a);
    let d = len(chord);
    if d <= EPS {
        return Err("arc start and end coincide".into());
    }
    if !(radius.is_finite() && radius > 0.0) {
        return Err(format!("arc radius must be positive, got {radius}"));
    }
    let mut note = None;
    let mut r = radius;
    if r < d / 2.0 - 1e-9 {
        note = Some(format!(
            "arc radius {radius:.3} is smaller than half the chord; using {:.3}",
            d / 2.0
        ));
        r = d / 2.0;
    }
    let h = (r * r - (d / 2.0) * (d / 2.0)).max(0.0).sqrt();
    let m = Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    // Unit normal pointing left of a→b.
    let n = (-chord.1 / d, chord.0 / d);
    let side = if ccw != large { 1.0 } else { -1.0 };
    let c = Point::new(m.x + n.0 * h * side, m.y + n.1 * h * side);
    let (s, e) = (angle_of(c, a), angle_of(c, b));
    let sweep = if ccw {
        let v = (e - s).rem_euclid(TAU);
        if v <= EPS {
            TAU
        } else {
            v
        }
    } else {
        let v = (s - e).rem_euclid(TAU);
        -(if v <= EPS { TAU } else { v })
    };
    Ok((
        Segment::Arc {
            center: c,
            radius: r,
            start: s,
            sweep,
        },
        note,
    ))
}

/// Builds a path. Rounded corners need straight segments on both sides; their tangent length
/// is limited to half of each adjacent edge, and the radius is reduced (with a note) if needed.
pub fn build(nodes: &[Node], closed: bool) -> Result<Built, String> {
    let n = nodes.len();
    if n < 2 {
        return Err("a path needs at least two points".into());
    }
    let mut notes = Vec::new();
    // Trimmed endpoints of incoming/outgoing straight edges per node, plus fillet arcs.
    let mut in_pt: Vec<Point> = nodes.iter().map(Node::point).collect();
    let mut out_pt = in_pt.clone();
    let mut fillet: Vec<Option<Segment>> = vec![None; n];

    let incoming_is_line = |i: usize| -> bool {
        // Segment arriving at node i; for i == 0 that's the closing segment (always a line).
        i == 0 || matches!(nodes[i], Node::Corner { .. })
    };
    let outgoing_is_line = |i: usize| -> bool {
        let next = (i + 1) % n;
        next == 0 || matches!(nodes[next], Node::Corner { .. })
    };

    for i in 0..n {
        let Node::Corner { p, round } = nodes[i] else {
            continue;
        };
        if round <= 0.0 {
            continue;
        }
        let at_open_end = !closed && (i == 0 || i == n - 1);
        if at_open_end {
            notes.push(format!(
                "rounding ignored at the open end of a path (point {i})"
            ));
            continue;
        }
        if !incoming_is_line(i) || !outgoing_is_line(i) {
            return Err(format!(
                "point {i}: 'round' needs straight edges on both sides"
            ));
        }
        let prev = nodes[(i + n - 1) % n].point();
        let next = nodes[(i + 1) % n].point();
        let (lu, lv) = (len(sub(prev, p)), len(sub(next, p)));
        if lu <= EPS || lv <= EPS {
            continue;
        }
        let u = ((prev.x - p.x) / lu, (prev.y - p.y) / lu);
        let v = ((next.x - p.x) / lv, (next.y - p.y) / lv);
        let cos_phi = (u.0 * v.0 + u.1 * v.1).clamp(-1.0, 1.0);
        let phi = cos_phi.acos();
        if phi <= 1e-6 || (PI - phi) <= 1e-6 {
            continue; // straight or folded back: nothing to round
        }
        let half = phi / 2.0;
        let max_t = lu.min(lv) / 2.0;
        let mut r = round;
        let mut t = r / half.tan();
        if t > max_t {
            t = max_t;
            r = t * half.tan();
            notes.push(format!(
                "corner radius {round:.3} at point {i} reduced to {r:.3} to fit the edges"
            ));
        }
        let t1 = Point::new(p.x + u.0 * t, p.y + u.1 * t);
        let t2 = Point::new(p.x + v.0 * t, p.y + v.1 * t);
        let bis = (u.0 + v.0, u.1 + v.1);
        let bl = len(bis);
        let dist = r / half.sin();
        let c = Point::new(p.x + bis.0 / bl * dist, p.y + bis.1 / bl * dist);
        let (s, e) = (angle_of(c, t1), angle_of(c, t2));
        let mut sweep = (e - s).rem_euclid(TAU);
        if sweep > PI {
            sweep -= TAU;
        }
        in_pt[i] = t1;
        out_pt[i] = t2;
        fillet[i] = Some(Segment::Arc {
            center: c,
            radius: r,
            start: s,
            sweep,
        });
    }

    let mut segments = Vec::new();
    let push_line = |segs: &mut Vec<Segment>, a: Point, b: Point| {
        if !a.approx_eq(b) {
            segs.push(Segment::line(a, b));
        }
    };
    // Fillet at node 0 is emitted last (it joins the closing segment to the first edge).
    for i in 1..n {
        let from = out_pt[i - 1];
        match nodes[i] {
            Node::Corner { .. } => push_line(&mut segments, from, in_pt[i]),
            Node::ArcTo {
                p,
                radius,
                ccw,
                large,
            } => {
                let (seg, note) = arc_between(from, p, radius, ccw, large)
                    .map_err(|e| format!("point {i}: {e}"))?;
                segments.push(seg);
                notes.extend(note);
            }
        }
        if let Some(f) = fillet[i] {
            segments.push(f);
        }
    }
    if closed {
        push_line(&mut segments, out_pt[n - 1], in_pt[0]);
        if let Some(f) = fillet[0] {
            segments.push(f);
        }
        // Start the path at the end of the first fillet so it reads naturally.
        if fillet[0].is_some() {
            segments.rotate_right(1);
        }
    }
    Ok(Built {
        path: Path { segments, closed },
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corner(x: f64, y: f64, round: f64) -> Node {
        Node::Corner {
            p: Point::new(x, y),
            round,
        }
    }

    #[test]
    fn rounded_rectangle_is_continuous_with_expected_length() {
        let r = 2.0;
        let nodes = [
            corner(0.0, 0.0, r),
            corner(10.0, 0.0, r),
            corner(10.0, 6.0, r),
            corner(0.0, 6.0, r),
        ];
        let b = build(&nodes, true).expect("build");
        assert!(b.notes.is_empty(), "{:?}", b.notes);
        assert!(b.path.is_continuous());
        assert_eq!(b.path.segments.len(), 8);
        let expected = 2.0 * (10.0 - 2.0 * r) + 2.0 * (6.0 - 2.0 * r) + TAU * r;
        assert!(
            (b.path.length() - expected).abs() < 1e-9,
            "{}",
            b.path.length()
        );
        let bb = b.path.bbox();
        assert!((bb.width() - 10.0).abs() < 1e-9 && (bb.height() - 6.0).abs() < 1e-9);
    }

    #[test]
    fn oversized_radius_is_reduced_with_a_note() {
        let nodes = [
            corner(0.0, 0.0, 0.0),
            corner(4.0, 0.0, 10.0),
            corner(4.0, 4.0, 0.0),
        ];
        let b = build(&nodes, false).expect("build");
        assert_eq!(b.notes.len(), 1);
        assert!(b.path.is_continuous());
    }

    #[test]
    fn explicit_arc_semicircle() {
        let nodes = [
            corner(0.0, 0.0, 0.0),
            Node::ArcTo {
                p: Point::new(10.0, 0.0),
                radius: 5.0,
                ccw: false,
                large: false,
            },
        ];
        let b = build(&nodes, false).expect("build");
        // Clockwise from (0,0) to (10,0) bulges upward (y > 0).
        let bb = b.path.bbox();
        assert!((bb.max.y - 5.0).abs() < 1e-9, "{bb:?}");
        assert!((b.path.length() - PI * 5.0).abs() < 1e-9);
        let ccw = build(
            &[
                corner(0.0, 0.0, 0.0),
                Node::ArcTo {
                    p: Point::new(10.0, 0.0),
                    radius: 5.0,
                    ccw: true,
                    large: false,
                },
            ],
            false,
        )
        .expect("build");
        assert!((ccw.path.bbox().min.y + 5.0).abs() < 1e-9);
    }

    #[test]
    fn small_and_large_arcs_differ() {
        let mk = |large| {
            build(
                &[
                    corner(0.0, 0.0, 0.0),
                    Node::ArcTo {
                        p: Point::new(6.0, 0.0),
                        radius: 5.0,
                        ccw: true,
                        large,
                    },
                ],
                false,
            )
            .expect("build")
            .path
            .length()
        };
        assert!(mk(true) > mk(false));
        assert!((mk(true) + mk(false) - TAU * 5.0).abs() < 1e-9);
    }

    #[test]
    fn rounding_next_to_an_arc_is_rejected() {
        let nodes = [
            corner(0.0, 0.0, 0.0),
            corner(5.0, 0.0, 1.0),
            Node::ArcTo {
                p: Point::new(10.0, 0.0),
                radius: 5.0,
                ccw: true,
                large: false,
            },
        ];
        assert!(build(&nodes, false).is_err());
    }
}
