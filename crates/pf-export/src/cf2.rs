//! CF2 ("Common File Format 2") writer — the ASCII die-line exchange format read by most
//! die-making and sample-cutting software. Line-type codes and rule thickness come from the
//! output profile, because shops configure these differently; verify with your CAM import.
//!
//! Layout written:
//! ```text
//! $BOF / V2 / ORDER / <name> / END / MAIN,<name> / UM / LL,x,y / UR,x,y / SCALE,1,1
//! L,pointage,type,0,x1,y1,x2,y2,0,0
//! A,pointage,type,0,x1,y1,x2,y2,cx,cy,dir,0,0      (dir: 1 = counter-clockwise, -1 = clockwise)
//! END / $EOF
//! ```

use crate::num;
use crate::profile::Profile;
use pf_geom::Segment;
use pf_model::{DieLine, LineType};
use std::fmt::Write;

pub fn to_cf2(d: &DieLine, profile: &Profile, name: &str) -> String {
    let name: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let b = d.bbox();
    let (min, max) = if b.is_empty() {
        (pf_geom::Point::new(0.0, 0.0), pf_geom::Point::new(0.0, 0.0))
    } else {
        (b.min, b.max)
    };
    let mut s = String::new();
    let _ = write!(
        s,
        "$BOF\nV2\nORDER\n{name}\nEND\nMAIN,{name}\nUM\nLL,{},{}\nUR,{},{}\nSCALE,1,1\n",
        num(min.x),
        num(min.y),
        num(max.x),
        num(max.y)
    );
    for t in LineType::ALL {
        let Some(m) = profile.line(t) else { continue };
        let (p, ty) = (num(m.cf2_pointage), m.cf2_type);
        for e in d.entities.iter().filter(|e| e.line_type == t) {
            for seg in &e.path.segments {
                let (a, z) = (seg.start_point(), seg.end_point());
                match *seg {
                    Segment::Line { .. } => {
                        let _ = writeln!(
                            s,
                            "L,{p},{ty},0,{},{},{},{},0,0",
                            num(a.x),
                            num(a.y),
                            num(z.x),
                            num(z.y)
                        );
                    }
                    Segment::Arc { center, sweep, .. } => {
                        // Full circles are written as two half arcs (start ≠ end).
                        let pieces = if sweep.abs() > 1.9 * std::f64::consts::PI {
                            2
                        } else {
                            1
                        };
                        for k in 0..pieces {
                            let part = split(seg, k, pieces);
                            let (a, z) = (part.start_point(), part.end_point());
                            let dir = if sweep > 0.0 { 1 } else { -1 };
                            let _ = writeln!(
                                s,
                                "A,{p},{ty},0,{},{},{},{},{},{},{dir},0,0",
                                num(a.x),
                                num(a.y),
                                num(z.x),
                                num(z.y),
                                num(center.x),
                                num(center.y)
                            );
                        }
                    }
                }
            }
        }
    }
    s.push_str("END\n$EOF\n");
    s
}

pub(crate) fn split(seg: &Segment, k: usize, n: usize) -> Segment {
    match *seg {
        Segment::Arc {
            center,
            radius,
            start,
            sweep,
        } => Segment::Arc {
            center,
            radius,
            start: start + sweep * k as f64 / n as f64,
            sweep: sweep / n as f64,
        },
        line => line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pf_geom::{Path, Point};

    /// Reads CF2 lines back into (type, length) totals.
    fn parse_lengths(cf2: &str) -> std::collections::BTreeMap<u8, f64> {
        let mut out = std::collections::BTreeMap::new();
        for line in cf2.lines() {
            let f: Vec<&str> = line.split(',').collect();
            let num = |i: usize| f[i].parse::<f64>().expect("number");
            match f[0] {
                "L" => {
                    let len = (num(6) - num(4)).hypot(num(7) - num(5));
                    *out.entry(f[2].parse().expect("type")).or_insert(0.0) += len;
                }
                "A" => {
                    let (sx, sy, ex, ey, cx, cy) = (num(4), num(5), num(6), num(7), num(8), num(9));
                    let r = (sx - cx).hypot(sy - cy);
                    let (a0, a1) = ((sy - cy).atan2(sx - cx), (ey - cy).atan2(ex - cx));
                    let dir: i32 = f[10].parse().expect("dir");
                    let tau = std::f64::consts::TAU;
                    let sweep = if dir > 0 {
                        (a1 - a0).rem_euclid(tau)
                    } else {
                        (a0 - a1).rem_euclid(tau)
                    };
                    *out.entry(f[2].parse().expect("type")).or_insert(0.0) += r * sweep;
                }
                _ => {}
            }
        }
        out
    }

    fn sample() -> DieLine {
        let mut d = DieLine::default();
        d.push(
            LineType::Cut,
            Path::polyline(
                &[
                    Point::new(0.0, 0.0),
                    Point::new(100.0, 0.0),
                    Point::new(100.0, 50.0),
                    Point::new(0.0, 50.0),
                ],
                true,
            ),
        );
        d.push(
            LineType::Crease,
            Path::line(Point::new(50.0, 0.0), Point::new(50.0, 50.0)),
        );
        d.push(
            LineType::Cut,
            Path {
                segments: vec![Segment::Arc {
                    center: Point::new(25.0, 25.0),
                    radius: 10.0,
                    start: 0.0,
                    sweep: std::f64::consts::TAU,
                }],
                closed: true,
            },
        );
        d.push(
            LineType::Cut,
            Path {
                segments: vec![Segment::Arc {
                    center: Point::new(75.0, 25.0),
                    radius: 5.0,
                    start: 0.5,
                    sweep: -2.0,
                }],
                closed: false,
            },
        );
        d.push(
            LineType::Annotation,
            Path::line(Point::new(0.0, 60.0), Point::new(10.0, 60.0)),
        );
        d
    }

    #[test]
    fn structure_and_header() {
        let s = to_cf2(&sample(), &Profile::standard(), "box 1");
        assert!(s.starts_with(
            "$BOF\nV2\nORDER\nbox_1\nEND\nMAIN,box_1\nUM\nLL,0,0\nUR,100,50\nSCALE,1,1\n"
        ));
        assert!(s.ends_with("END\n$EOF\n"));
    }

    #[test]
    fn lengths_survive_a_round_trip() {
        let d = sample();
        let lengths = parse_lengths(&to_cf2(&d, &Profile::standard(), "x"));
        let cut = d.length_of(LineType::Cut);
        assert!(
            (lengths[&1] - cut).abs() < 1e-2,
            "cut {} vs {}",
            lengths[&1],
            cut
        );
        assert!((lengths[&2] - 50.0).abs() < 1e-9);
        assert!(
            !lengths.contains_key(&7),
            "annotations are excluded by the standard profile"
        );
    }
}
