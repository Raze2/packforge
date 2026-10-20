//! HPGL / PLT writer for plotters and sample-cutting tables.
//! Units: 40 plotter units per millimetre. The drawing is moved so its lower-left corner is at
//! the origin (plotters do not accept negative coordinates). Pens come from the output profile.

use crate::profile::Profile;
use pf_geom::Segment;
use pf_model::{DieLine, LineType};
use std::fmt::Write;

pub const UNITS_PER_MM: f64 = 40.0;

fn u(v: f64) -> i64 {
    (v * UNITS_PER_MM).round() as i64
}

pub fn to_hpgl(d: &DieLine, profile: &Profile) -> String {
    let b = d.bbox();
    let (ox, oy) = if b.is_empty() {
        (0.0, 0.0)
    } else {
        (b.min.x, b.min.y)
    };
    let mut s = String::from("IN;\n");
    for t in LineType::ALL {
        let Some(m) = profile.line(t) else { continue };
        let ents: Vec<_> = d.entities.iter().filter(|e| e.line_type == t).collect();
        if ents.is_empty() {
            continue;
        }
        let _ = writeln!(s, "SP{};", m.pen);
        for e in ents {
            let mut pen_at: Option<(i64, i64)> = None;
            for seg in &e.path.segments {
                let a = seg.start_point();
                let start = (u(a.x - ox), u(a.y - oy));
                if pen_at != Some(start) {
                    let _ = writeln!(s, "PU{},{};", start.0, start.1);
                }
                match *seg {
                    Segment::Line { to, .. } => {
                        let end = (u(to.x - ox), u(to.y - oy));
                        let _ = writeln!(s, "PD{},{};", end.0, end.1);
                        pen_at = Some(end);
                    }
                    Segment::Arc { center, sweep, .. } => {
                        // AA: arc absolute around the centre, sweep in degrees (positive = CCW).
                        let _ = writeln!(
                            s,
                            "PD;AA{},{},{:.3};",
                            u(center.x - ox),
                            u(center.y - oy),
                            sweep.to_degrees()
                        );
                        let z = seg.end_point();
                        pen_at = Some((u(z.x - ox), u(z.y - oy)));
                    }
                }
            }
        }
    }
    s.push_str("PU;SP0;\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use pf_geom::{Path, Point};
    use std::collections::BTreeMap;

    /// Minimal HPGL interpreter: total drawn length per pen (mm).
    fn drawn(hpgl: &str) -> BTreeMap<u8, f64> {
        let mut out = BTreeMap::new();
        let (mut pen, mut x, mut y) = (0u8, 0.0f64, 0.0f64);
        for cmd in hpgl.split(';').map(str::trim).filter(|c| !c.is_empty()) {
            let (op, args) = cmd.split_at(2);
            let nums: Vec<f64> = args
                .split(',')
                .filter(|a| !a.is_empty())
                .map(|a| a.parse().expect("num"))
                .collect();
            match op {
                "SP" => pen = nums.first().copied().unwrap_or(0.0) as u8,
                "PU" if nums.len() == 2 => (x, y) = (nums[0], nums[1]),
                "PD" if nums.len() == 2 => {
                    *out.entry(pen).or_insert(0.0) +=
                        (nums[0] - x).hypot(nums[1] - y) / UNITS_PER_MM;
                    (x, y) = (nums[0], nums[1]);
                }
                "AA" => {
                    let (cx, cy, deg) = (nums[0], nums[1], nums[2]);
                    let r = (x - cx).hypot(y - cy);
                    *out.entry(pen).or_insert(0.0) += r * deg.to_radians().abs() / UNITS_PER_MM;
                    let a = (y - cy).atan2(x - cx) + deg.to_radians();
                    (x, y) = (cx + r * a.cos(), cy + r * a.sin());
                }
                _ => {}
            }
        }
        out
    }

    #[test]
    fn plots_cut_and_crease_with_their_pens() {
        let mut d = DieLine::default();
        d.push(
            LineType::Cut,
            Path::polyline(
                &[
                    Point::new(-10.0, -10.0),
                    Point::new(90.0, -10.0),
                    Point::new(90.0, 40.0),
                    Point::new(-10.0, 40.0),
                ],
                true,
            ),
        );
        d.push(
            LineType::Crease,
            Path::line(Point::new(40.0, -10.0), Point::new(40.0, 40.0)),
        );
        d.push(
            LineType::Cut,
            Path {
                segments: vec![Segment::Arc {
                    center: Point::new(15.0, 15.0),
                    radius: 8.0,
                    start: 0.0,
                    sweep: std::f64::consts::PI,
                }],
                closed: false,
            },
        );
        let s = to_hpgl(&d, &Profile::standard());
        assert!(s.starts_with("IN;\nSP1;\n") && s.ends_with("PU;SP0;\n"));
        assert!(!s.contains('-'), "coordinates must be non-negative:\n{s}");
        let lengths = drawn(&s);
        assert!(
            (lengths[&1] - d.length_of(LineType::Cut)).abs() < 0.05,
            "{lengths:?}"
        );
        assert!((lengths[&2] - 50.0).abs() < 1e-9);
    }
}
