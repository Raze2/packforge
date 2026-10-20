//! SVG writer. Units are millimetres; the y axis is flipped so the drawing appears upright.

use crate::num;
use pf_geom::{Path, Segment};
use pf_model::{DieLine, LineType};
use std::f64::consts::PI;
use std::fmt::Write;

#[derive(Debug, Clone)]
pub struct SvgOptions {
    /// Empty border around the blank (mm).
    pub margin: f64,
    /// Stroke width (mm).
    pub stroke_width: f64,
    /// Optional title embedded in the document.
    pub title: Option<String>,
}

impl Default for SvgOptions {
    fn default() -> Self {
        Self {
            margin: 5.0,
            stroke_width: 0.35,
            title: None,
        }
    }
}

struct Style {
    stroke: &'static str,
    dash: Option<&'static str>,
}

fn style(t: LineType) -> Style {
    let (stroke, dash) = match t {
        LineType::Cut => ("#1d1d1f", None),
        LineType::Crease => ("#d0021b", Some("6 3")),
        LineType::ReverseCrease => ("#9b1bd0", Some("6 2 1 2")),
        LineType::Perforation => ("#1a8f3c", Some("2 2")),
        LineType::HalfCut => ("#1f5fd0", Some("8 2")),
        LineType::Bleed => ("#00a3c4", Some("1 2")),
        LineType::Dimension => ("#8a8a8e", None),
        LineType::Annotation => ("#b07d00", None),
    };
    Style { stroke, dash }
}

pub fn to_svg(d: &DieLine, opts: &SvgOptions) -> String {
    let b = d.bbox();
    let (w, h) = if b.is_empty() {
        (0.0, 0.0)
    } else {
        (b.width(), b.height())
    };
    let m = opts.margin;
    let (min_x, max_y) = if b.is_empty() {
        (0.0, 0.0)
    } else {
        (b.min.x, b.max.y)
    };
    let tx = |x: f64| num(x - min_x + m);
    let ty = |y: f64| num(max_y - y + m);

    let (vw, vh) = (num(w + 2.0 * m), num(h + 2.0 * m));
    let mut s = String::new();
    let _ = writeln!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{vw}mm" height="{vh}mm" viewBox="0 0 {vw} {vh}">"#
    );
    if let Some(title) = &opts.title {
        let _ = writeln!(s, "  <title>{}</title>", escape(title));
    }
    for t in LineType::ALL {
        let entities: Vec<&Path> = d
            .entities
            .iter()
            .filter(|e| e.line_type == t)
            .map(|e| &e.path)
            .collect();
        if entities.is_empty() {
            continue;
        }
        let st = style(t);
        let dash = st
            .dash
            .map(|d| format!(r#" stroke-dasharray="{d}""#))
            .unwrap_or_default();
        let _ = writeln!(
            s,
            r#"  <g id="{id}" class="pf-{id}" fill="none" stroke="{stroke}" stroke-width="{sw}" stroke-linecap="round" stroke-linejoin="round"{dash}>"#,
            id = t.id(),
            stroke = st.stroke,
            sw = num(opts.stroke_width),
        );
        for p in entities {
            let _ = writeln!(s, r#"    <path d="{}"/>"#, path_data(p, &tx, &ty));
        }
        let _ = writeln!(s, "  </g>");
    }
    s.push_str("</svg>\n");
    s
}

fn path_data(p: &Path, tx: &dyn Fn(f64) -> String, ty: &dyn Fn(f64) -> String) -> String {
    let mut d = String::new();
    let mut cursor = None;
    for seg in &p.segments {
        let start = seg.start_point();
        if cursor.is_none_or(|c: pf_geom::Point| !c.approx_eq(start)) {
            let _ = write!(d, "M{} {}", tx(start.x), ty(start.y));
        }
        match *seg {
            Segment::Line { to, .. } => {
                let _ = write!(d, "L{} {}", tx(to.x), ty(to.y));
            }
            Segment::Arc {
                center,
                radius,
                start: a0,
                sweep,
            } => {
                // Split near-full circles: SVG cannot draw an arc whose ends coincide.
                let parts = if sweep.abs() > 1.9 * PI { 2 } else { 1 };
                for i in 1..=parts {
                    let a = a0 + sweep * (i as f64) / (parts as f64);
                    let end = pf_geom::Point::new(
                        center.x + radius * a.cos(),
                        center.y + radius * a.sin(),
                    );
                    let part_sweep = sweep / parts as f64;
                    let large = u8::from(part_sweep.abs() > PI);
                    // y is flipped, so counter-clockwise in model space is sweep-flag 0.
                    let flag = u8::from(part_sweep < 0.0);
                    let r = num(radius);
                    let _ = write!(d, "A{r} {r} 0 {large} {flag} {} {}", tx(end.x), ty(end.y));
                }
            }
        }
        cursor = Some(seg.end_point());
    }
    if p.closed {
        d.push('Z');
    }
    d
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pf_geom::Point;

    #[test]
    fn rectangle_is_flipped_and_offset() {
        let mut d = DieLine::default();
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 4.0),
            Point::new(0.0, 4.0),
        ];
        d.push(LineType::Cut, Path::polyline(&pts, true));
        let svg = to_svg(&d, &SvgOptions::default());
        assert!(svg.contains(r#"width="20mm" height="14mm""#), "{svg}");
        // (0,0) bottom-left maps to (5, 9) in SVG space.
        assert!(svg.contains("M5 9L15 9L15 5L5 5L5 9Z"), "{svg}");
        assert!(svg.contains(r#"id="cut""#));
        assert!(!svg.contains("crease"));
    }
}
