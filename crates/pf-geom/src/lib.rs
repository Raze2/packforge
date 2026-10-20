//! 2D geometry for die-lines. All coordinates are millimetres (`f64`), y axis pointing up.

use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;

pub mod shape;

/// Tolerance used for geometric equality (mm).
pub const EPS: f64 = 1e-6;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance(self, other: Point) -> f64 {
        (other.x - self.x).hypot(other.y - self.y)
    }

    pub fn approx_eq(self, other: Point) -> bool {
        self.distance(other) <= EPS
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

/// A single path segment.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Segment {
    Line {
        from: Point,
        to: Point,
    },
    /// Circular arc around `center`, starting at `start` radians and sweeping `sweep` radians
    /// (positive = counter-clockwise).
    Arc {
        center: Point,
        radius: f64,
        start: f64,
        sweep: f64,
    },
}

impl Segment {
    pub fn line(from: Point, to: Point) -> Self {
        Segment::Line { from, to }
    }

    pub fn start_point(&self) -> Point {
        match *self {
            Segment::Line { from, .. } => from,
            Segment::Arc {
                center,
                radius,
                start,
                ..
            } => polar(center, radius, start),
        }
    }

    pub fn end_point(&self) -> Point {
        match *self {
            Segment::Line { to, .. } => to,
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            } => polar(center, radius, start + sweep),
        }
    }

    pub fn length(&self) -> f64 {
        match *self {
            Segment::Line { from, to } => from.distance(to),
            Segment::Arc { radius, sweep, .. } => radius * sweep.abs(),
        }
    }

    pub fn is_finite(&self) -> bool {
        match *self {
            Segment::Line { from, to } => from.is_finite() && to.is_finite(),
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            } => center.is_finite() && radius.is_finite() && start.is_finite() && sweep.is_finite(),
        }
    }

    /// Bounding box, exact for arcs (includes axis extremes inside the sweep).
    pub fn bbox(&self) -> BBox {
        match *self {
            Segment::Line { from, to } => BBox::from_points(&[from, to]),
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            } => {
                let mut b = BBox::from_points(&[self.start_point(), self.end_point()]);
                let (lo, hi) = if sweep >= 0.0 {
                    (start, start + sweep)
                } else {
                    (start + sweep, start)
                };
                // Axis-aligned extremes at multiples of 90°.
                let mut k = (lo / (TAU / 4.0)).ceil();
                while k * (TAU / 4.0) <= hi + EPS {
                    b.include(polar(center, radius, k * (TAU / 4.0)));
                    k += 1.0;
                }
                b
            }
        }
    }

    pub fn transform(&self, t: &Transform) -> Segment {
        match *self {
            Segment::Line { from, to } => Segment::Line {
                from: t.apply(from),
                to: t.apply(to),
            },
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            } => {
                if t.mirror_x {
                    // Mirroring across a vertical axis maps angle a → π − a and flips direction.
                    Segment::Arc {
                        center: t.apply(center),
                        radius,
                        start: std::f64::consts::PI - start,
                        sweep: -sweep,
                    }
                } else {
                    Segment::Arc {
                        center: t.apply(center),
                        radius,
                        start,
                        sweep,
                    }
                }
            }
        }
    }
}

/// True when two segments intersect at a single interior point of both (a proper crossing).
fn segments_cross((a, b): (Point, Point), (c, d): (Point, Point)) -> bool {
    let orient =
        |p: Point, q: Point, r: Point| (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x);
    let eps = 1e-9;
    let (o1, o2, o3, o4) = (
        orient(a, b, c),
        orient(a, b, d),
        orient(c, d, a),
        orient(c, d, b),
    );
    o1 * o2 < -eps && o3 * o4 < -eps
}

fn polar(center: Point, radius: f64, angle: f64) -> Point {
    Point::new(
        center.x + radius * angle.cos(),
        center.y + radius * angle.sin(),
    )
}

/// A connected sequence of segments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Path {
    pub segments: Vec<Segment>,
    pub closed: bool,
}

impl Path {
    pub fn line(from: Point, to: Point) -> Self {
        Self {
            segments: vec![Segment::line(from, to)],
            closed: false,
        }
    }

    /// Builds a polyline. With `closed`, a closing segment back to the first point is added
    /// unless the last point already coincides with it.
    pub fn polyline(points: &[Point], closed: bool) -> Self {
        let mut segments: Vec<Segment> = points
            .windows(2)
            .filter(|w| !w[0].approx_eq(w[1]))
            .map(|w| Segment::line(w[0], w[1]))
            .collect();
        if closed && points.len() > 2 {
            let (first, last) = (points[0], points[points.len() - 1]);
            if !first.approx_eq(last) {
                segments.push(Segment::line(last, first));
            }
        }
        Self { segments, closed }
    }

    pub fn length(&self) -> f64 {
        self.segments.iter().map(Segment::length).sum()
    }

    pub fn bbox(&self) -> BBox {
        let mut b = BBox::empty();
        for s in &self.segments {
            b.union(&s.bbox());
        }
        b
    }

    pub fn is_finite(&self) -> bool {
        self.segments.iter().all(Segment::is_finite)
    }

    /// True when consecutive segments connect, and (for closed paths) the last ends at the first.
    pub fn is_continuous(&self) -> bool {
        let connected = self
            .segments
            .windows(2)
            .all(|w| w[0].end_point().approx_eq(w[1].start_point()));
        let loop_ok = !self.closed
            || match (self.segments.first(), self.segments.last()) {
                (Some(a), Some(b)) => b.end_point().approx_eq(a.start_point()),
                _ => false,
            };
        connected && loop_ok
    }

    /// Signed area of a closed path (positive = counter-clockwise): shoelace over the chords
    /// plus the exact circular-segment area of every arc.
    pub fn signed_area(&self) -> f64 {
        self.segments
            .iter()
            .map(|s| {
                let (a, b) = (s.start_point(), s.end_point());
                let chord = (a.x * b.y - b.x * a.y) / 2.0;
                match *s {
                    Segment::Line { .. } => chord,
                    Segment::Arc { radius, sweep, .. } => {
                        chord + 0.5 * radius * radius * (sweep - sweep.sin())
                    }
                }
            })
            .sum()
    }

    /// Points along the path; arcs are split so that the chord error stays below `tolerance`.
    pub fn flatten(&self, tolerance: f64) -> Vec<Point> {
        let mut pts = Vec::new();
        for s in &self.segments {
            if pts.is_empty() {
                pts.push(s.start_point());
            }
            match *s {
                Segment::Line { to, .. } => pts.push(to),
                Segment::Arc {
                    center,
                    radius,
                    start,
                    sweep,
                } => {
                    let max_step = 2.0 * (1.0 - tolerance.min(radius) / radius).acos().max(1e-3);
                    let n = ((sweep.abs() / max_step).ceil() as usize).clamp(1, 512);
                    for i in 1..=n {
                        pts.push(polar(center, radius, start + sweep * i as f64 / n as f64));
                    }
                }
            }
        }
        pts
    }

    /// Number of places where the path properly crosses itself (touching at a shared vertex is
    /// allowed). Uses a flattened copy for arcs.
    pub fn self_crossings(&self) -> usize {
        let mut pts = self.flatten(0.05);
        if self.closed && pts.len() > 2 && !pts[0].approx_eq(pts[pts.len() - 1]) {
            pts.push(pts[0]);
        }
        let segs: Vec<(Point, Point)> = pts.windows(2).map(|w| (w[0], w[1])).collect();
        let n = segs.len();
        let mut count = 0;
        for i in 0..n {
            for j in (i + 2)..n {
                if self.closed && i == 0 && j == n - 1 {
                    continue; // first and last segment are adjacent in a closed path
                }
                if segments_cross(segs[i], segs[j]) {
                    count += 1;
                }
            }
        }
        count
    }

    pub fn transform(&self, t: &Transform) -> Path {
        Path {
            segments: self.segments.iter().map(|s| s.transform(t)).collect(),
            closed: self.closed,
        }
    }
}

/// Axis-aligned bounding box.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BBox {
    pub min: Point,
    pub max: Point,
}

impl BBox {
    pub fn empty() -> Self {
        Self {
            min: Point::new(f64::INFINITY, f64::INFINITY),
            max: Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        }
    }

    pub fn from_points(points: &[Point]) -> Self {
        let mut b = Self::empty();
        for p in points {
            b.include(*p);
        }
        b
    }

    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x || self.min.y > self.max.y
    }

    pub fn include(&mut self, p: Point) {
        self.min.x = self.min.x.min(p.x);
        self.min.y = self.min.y.min(p.y);
        self.max.x = self.max.x.max(p.x);
        self.max.y = self.max.y.max(p.y);
    }

    pub fn union(&mut self, other: &BBox) {
        if !other.is_empty() {
            self.include(other.min);
            self.include(other.max);
        }
    }

    pub fn width(&self) -> f64 {
        if self.is_empty() {
            0.0
        } else {
            self.max.x - self.min.x
        }
    }

    pub fn height(&self) -> f64 {
        if self.is_empty() {
            0.0
        } else {
            self.max.y - self.min.y
        }
    }
}

/// Translation plus optional mirror across the vertical line `x = mirror_axis`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Transform {
    pub dx: f64,
    pub dy: f64,
    pub mirror_x: bool,
    pub mirror_axis: f64,
}

impl Transform {
    pub fn translate(dx: f64, dy: f64) -> Self {
        Self {
            dx,
            dy,
            ..Self::default()
        }
    }

    pub fn mirror_x(axis: f64) -> Self {
        Self {
            mirror_x: true,
            mirror_axis: axis,
            ..Self::default()
        }
    }

    pub fn apply(&self, p: Point) -> Point {
        let x = if self.mirror_x {
            2.0 * self.mirror_axis - p.x
        } else {
            p.x
        };
        Point::new(x + self.dx, p.y + self.dy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn closed_polyline_adds_closing_segment() {
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 5.0),
        ];
        let p = Path::polyline(&pts, true);
        assert_eq!(p.segments.len(), 3);
        assert!(p.is_continuous());
        assert!((p.length() - (10.0 + 5.0 + 125f64.sqrt())).abs() < 1e-9);
    }

    #[test]
    fn rectangle_area_and_bbox() {
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(4.0, 0.0),
            Point::new(4.0, 3.0),
            Point::new(0.0, 3.0),
        ];
        let p = Path::polyline(&pts, true);
        assert!((p.signed_area() - 12.0).abs() < 1e-9);
        let b = p.bbox();
        assert_eq!((b.width(), b.height()), (4.0, 3.0));
    }

    #[test]
    fn area_of_rounded_rectangle_is_exact() {
        let r = 2.0;
        let nodes =
            [(0.0, 0.0), (10.0, 0.0), (10.0, 6.0), (0.0, 6.0)].map(|(x, y)| shape::Node::Corner {
                p: Point::new(x, y),
                round: r,
            });
        let p = shape::build(&nodes, true).expect("build").path;
        let exact = 60.0 - (4.0 - PI) * r * r;
        assert!(
            (p.signed_area() - exact).abs() < 1e-9,
            "{}",
            p.signed_area()
        );
    }

    #[test]
    fn detects_self_crossing_bowtie() {
        let bowtie = Path::polyline(
            &[
                Point::new(0.0, 0.0),
                Point::new(10.0, 10.0),
                Point::new(10.0, 0.0),
                Point::new(0.0, 10.0),
            ],
            true,
        );
        assert_eq!(bowtie.self_crossings(), 1);
        let square = Path::polyline(
            &[
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(10.0, 10.0),
                Point::new(0.0, 10.0),
            ],
            true,
        );
        assert_eq!(square.self_crossings(), 0);
        // Touching at a shared vertex (a "pinch") is not a crossing.
        let pinch = Path::polyline(
            &[
                Point::new(0.0, 0.0),
                Point::new(4.0, 4.0),
                Point::new(8.0, 0.0),
                Point::new(8.0, 8.0),
                Point::new(4.0, 4.0),
                Point::new(0.0, 8.0),
            ],
            true,
        );
        assert_eq!(pinch.self_crossings(), 0);
    }

    #[test]
    fn arc_bbox_includes_extremes() {
        let a = Segment::Arc {
            center: Point::new(0.0, 0.0),
            radius: 2.0,
            start: 0.0,
            sweep: PI,
        };
        let b = a.bbox();
        assert!((b.max.y - 2.0).abs() < 1e-9);
        assert!((b.min.x + 2.0).abs() < 1e-9);
        assert!((a.length() - 2.0 * PI).abs() < 1e-9);
    }

    #[test]
    fn mirror_keeps_arc_endpoints_consistent() {
        let a = Segment::Arc {
            center: Point::new(1.0, 0.0),
            radius: 1.0,
            start: 0.0,
            sweep: PI / 2.0,
        };
        let t = Transform::mirror_x(0.0);
        let m = a.transform(&t);
        assert!(m.start_point().approx_eq(t.apply(a.start_point())));
        assert!(m.end_point().approx_eq(t.apply(a.end_point())));
    }
}
