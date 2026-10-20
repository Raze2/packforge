//! DXF writer — ASCII R12 (AC1009) for the widest CAD/CAM compatibility.
//! One layer per line type; coordinates in millimetres.

use crate::num;
use pf_geom::Segment;
use pf_model::{DieLine, LineType};
use std::collections::BTreeMap;
use std::fmt::Write;

#[derive(Debug, Clone, Default)]
pub struct DxfOptions {
    /// Overrides for layer names (default: upper-case line-type id, `-` → `_`).
    pub layer_names: BTreeMap<LineType, String>,
}

impl DxfOptions {
    fn layer(&self, t: LineType) -> String {
        self.layer_names
            .get(&t)
            .cloned()
            .unwrap_or_else(|| t.id().to_ascii_uppercase().replace('-', "_"))
    }
}

/// AutoCAD Color Index per line type.
fn aci(t: LineType) -> u8 {
    match t {
        LineType::Cut => 7,
        LineType::Crease => 1,
        LineType::ReverseCrease => 6,
        LineType::Perforation => 3,
        LineType::HalfCut => 5,
        LineType::Bleed => 4,
        LineType::Dimension => 8,
        LineType::Annotation => 2,
    }
}

struct Out(String);

impl Out {
    fn pair(&mut self, code: u16, value: impl std::fmt::Display) {
        let _ = write!(self.0, "{code:>3}\n{value}\n");
    }
}

pub fn to_dxf(d: &DieLine, opts: &DxfOptions) -> String {
    let mut o = Out(String::new());
    let used: Vec<LineType> = LineType::ALL
        .into_iter()
        .filter(|t| d.entities.iter().any(|e| e.line_type == *t))
        .collect();
    let b = d.bbox();
    let (min, max) = if b.is_empty() {
        (pf_geom::Point::new(0.0, 0.0), pf_geom::Point::new(0.0, 0.0))
    } else {
        (b.min, b.max)
    };

    // HEADER
    o.pair(0, "SECTION");
    o.pair(2, "HEADER");
    o.pair(9, "$ACADVER");
    o.pair(1, "AC1009");
    o.pair(9, "$INSUNITS");
    o.pair(70, 4); // millimetres
    o.pair(9, "$EXTMIN");
    o.pair(10, num(min.x));
    o.pair(20, num(min.y));
    o.pair(9, "$EXTMAX");
    o.pair(10, num(max.x));
    o.pair(20, num(max.y));
    o.pair(0, "ENDSEC");

    // TABLES: line types and layers
    o.pair(0, "SECTION");
    o.pair(2, "TABLES");
    o.pair(0, "TABLE");
    o.pair(2, "LTYPE");
    o.pair(70, 1);
    o.pair(0, "LTYPE");
    o.pair(2, "CONTINUOUS");
    o.pair(70, 0);
    o.pair(3, "Solid line");
    o.pair(72, 65);
    o.pair(73, 0);
    o.pair(40, "0.0");
    o.pair(0, "ENDTAB");
    o.pair(0, "TABLE");
    o.pair(2, "LAYER");
    o.pair(70, used.len());
    for t in &used {
        o.pair(0, "LAYER");
        o.pair(2, opts.layer(*t));
        o.pair(70, 0);
        o.pair(62, aci(*t));
        o.pair(6, "CONTINUOUS");
    }
    o.pair(0, "ENDTAB");
    o.pair(0, "ENDSEC");

    // ENTITIES
    o.pair(0, "SECTION");
    o.pair(2, "ENTITIES");
    for t in &used {
        let layer = opts.layer(*t);
        for e in d.entities.iter().filter(|e| e.line_type == *t) {
            for seg in &e.path.segments {
                match *seg {
                    Segment::Line { from, to } => {
                        o.pair(0, "LINE");
                        o.pair(8, &layer);
                        o.pair(10, num(from.x));
                        o.pair(20, num(from.y));
                        o.pair(30, 0);
                        o.pair(11, num(to.x));
                        o.pair(21, num(to.y));
                        o.pair(31, 0);
                    }
                    Segment::Arc {
                        center,
                        radius,
                        start,
                        sweep,
                    } => {
                        // DXF arcs are always counter-clockwise from 50 to 51 (degrees).
                        let (a0, a1) = if sweep >= 0.0 {
                            (start, start + sweep)
                        } else {
                            (start + sweep, start)
                        };
                        o.pair(0, "ARC");
                        o.pair(8, &layer);
                        o.pair(10, num(center.x));
                        o.pair(20, num(center.y));
                        o.pair(30, 0);
                        o.pair(40, num(radius));
                        o.pair(50, num(normalize_deg(a0.to_degrees())));
                        o.pair(51, num(normalize_deg(a1.to_degrees())));
                    }
                }
            }
        }
    }
    o.pair(0, "ENDSEC");
    o.pair(0, "EOF");
    o.0
}

fn normalize_deg(a: f64) -> f64 {
    let r = a.rem_euclid(360.0);
    if (r - 360.0).abs() < 1e-9 {
        0.0
    } else {
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pf_geom::{Path, Point};

    #[test]
    fn writes_layers_and_lines() {
        let mut d = DieLine::default();
        d.push(
            LineType::Cut,
            Path::line(Point::new(0.0, 0.0), Point::new(10.0, 0.0)),
        );
        d.push(
            LineType::ReverseCrease,
            Path::line(Point::new(0.0, 1.0), Point::new(10.0, 1.0)),
        );
        let s = to_dxf(&d, &DxfOptions::default());
        assert!(s.starts_with("  0\nSECTION\n  2\nHEADER\n"));
        assert!(s.contains("  2\nCUT\n"));
        assert!(s.contains("  2\nREVERSE_CREASE\n"));
        assert!(!s.contains("PERFORATION"));
        assert_eq!(s.matches("\nLINE\n").count(), 2);
        assert!(s.ends_with("  0\nEOF\n"));
    }

    #[test]
    fn negative_arc_sweep_becomes_ccw() {
        let mut d = DieLine::default();
        d.push(
            LineType::Cut,
            Path {
                segments: vec![Segment::Arc {
                    center: Point::new(0.0, 0.0),
                    radius: 5.0,
                    start: std::f64::consts::FRAC_PI_2,
                    sweep: -std::f64::consts::FRAC_PI_2,
                }],
                closed: false,
            },
        );
        let s = to_dxf(&d, &DxfOptions::default());
        assert!(s.contains(" 50\n0\n 51\n90\n"), "{s}");
    }
}
