//! Minimal deterministic PDF 1.4 writer for die-lines: vector paths (arcs as Bézier curves),
//! dash patterns per line type, overall dimensions, legend, title block, optional watermark.
//! Text uses the built-in Helvetica font (Latin characters; others are replaced by '?').

use crate::num;
use crate::profile::{line_dash_mm, line_rgb};
use pf_geom::{BBox, Point, Segment};
use pf_model::{DieLine, LineType};
use std::f64::consts::FRAC_PI_2;
use std::fmt::Write;

const PT_PER_MM: f64 = 72.0 / 25.4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PageSize {
    /// Page sized to the drawing: prints at true 1:1 scale.
    Actual,
    /// Fit on a sheet (width × height in mm, portrait); orientation is chosen automatically.
    Sheet { width: f64, height: f64 },
}

impl PageSize {
    pub const A4: PageSize = PageSize::Sheet {
        width: 210.0,
        height: 297.0,
    };
    pub const A3: PageSize = PageSize::Sheet {
        width: 297.0,
        height: 420.0,
    };
    pub const LETTER: PageSize = PageSize::Sheet {
        width: 215.9,
        height: 279.4,
    };

    pub fn parse(s: &str) -> Option<PageSize> {
        match s.to_ascii_lowercase().as_str() {
            "actual" | "1:1" => Some(PageSize::Actual),
            "a4" => Some(PageSize::A4),
            "a3" => Some(PageSize::A3),
            "letter" => Some(PageSize::LETTER),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PdfOptions {
    pub page: PageSize,
    /// Title block lines (first is the title).
    pub title_lines: Vec<String>,
    /// Large diagonal text behind the drawing (e.g. for the free tier).
    pub watermark: Option<String>,
    /// Line types to draw (in this order); others are skipped.
    pub include: Vec<LineType>,
    /// Draw overall dimension lines.
    pub dimensions: bool,
    /// Draw the line-type legend.
    pub legend: bool,
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            page: PageSize::Actual,
            title_lines: Vec::new(),
            watermark: None,
            include: LineType::ALL.to_vec(),
            dimensions: true,
            legend: true,
        }
    }
}

const MARGIN: f64 = 12.0; // mm around the drawing
const BLOCK_H: f64 = 34.0; // mm title block height

fn pdf_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '(' | ')' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            '—' | '–' => out.push('-'),
            '×' => out.push('x'),
            c if (' '..='~').contains(&c) => out.push(c),
            _ => out.push('?'),
        }
    }
    out
}

/// Approximate width of Helvetica text (pt) — enough for centring short labels.
fn text_width(s: &str, size: f64) -> f64 {
    s.chars().count() as f64 * size * 0.52
}

struct Content {
    ops: String,
    scale: f64, // pt per drawing mm
    ox: f64,    // drawing origin offset on the page (pt)
    oy: f64,
}

impl Content {
    fn p(&self, q: Point) -> (f64, f64) {
        (self.ox + q.x * self.scale, self.oy + q.y * self.scale)
    }

    fn op(&mut self, s: &str) {
        self.ops.push_str(s);
        self.ops.push('\n');
    }

    fn segment(&mut self, seg: &Segment, first: bool) {
        if first {
            let (x, y) = self.p(seg.start_point());
            self.op(&format!("{} {} m", num(x), num(y)));
        }
        match *seg {
            Segment::Line { to, .. } => {
                let (x, y) = self.p(to);
                self.op(&format!("{} {} l", num(x), num(y)));
            }
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            } => {
                // Split into ≤ 90° pieces, each one cubic Bézier.
                let n = (sweep.abs() / FRAC_PI_2).ceil().max(1.0) as usize;
                let step = sweep / n as f64;
                let k = 4.0 / 3.0 * (step / 4.0).tan();
                for i in 0..n {
                    let a0 = start + step * i as f64;
                    let a1 = a0 + step;
                    let (c0, s0) = (a0.cos(), a0.sin());
                    let (c1, s1) = (a1.cos(), a1.sin());
                    let p1 = Point::new(
                        center.x + radius * (c0 - k * s0),
                        center.y + radius * (s0 + k * c0),
                    );
                    let p2 = Point::new(
                        center.x + radius * (c1 + k * s1),
                        center.y + radius * (s1 - k * c1),
                    );
                    let p3 = Point::new(center.x + radius * c1, center.y + radius * s1);
                    let ((x1, y1), (x2, y2), (x3, y3)) = (self.p(p1), self.p(p2), self.p(p3));
                    self.op(&format!(
                        "{} {} {} {} {} {} c",
                        num(x1),
                        num(y1),
                        num(x2),
                        num(y2),
                        num(x3),
                        num(y3)
                    ));
                }
            }
        }
    }

    fn text(&mut self, x: f64, y: f64, size: f64, s: &str) {
        self.op(&format!(
            "BT /F1 {} Tf {} {} Td ({}) Tj ET",
            num(size),
            num(x),
            num(y),
            pdf_text(s)
        ));
    }
}

/// Writes a PDF of the die-line. Returns the bytes.
pub fn to_pdf(d: &DieLine, opts: &PdfOptions) -> Vec<u8> {
    let b = {
        let b = d.bbox_all();
        if b.is_empty() {
            BBox::from_points(&[Point::new(0.0, 0.0), Point::new(1.0, 1.0)])
        } else {
            b
        }
    };
    let (dw, dh) = (b.width(), b.height());
    // Space needed around the drawing for dimension lines, margins and the title block (mm).
    let extra_w = 2.0 * MARGIN + 14.0;
    let extra_h = 2.0 * MARGIN + 22.0 + BLOCK_H;

    let (page_w, page_h, scale) = match opts.page {
        PageSize::Actual => (dw + extra_w, dh + extra_h, 1.0),
        PageSize::Sheet { width, height } => {
            // Landscape if the drawing is wider than tall.
            let (pw, ph) = if dw > dh {
                (height.max(width), height.min(width))
            } else {
                (width.min(height), height.max(width))
            };
            let s = ((pw - extra_w) / dw).min((ph - extra_h) / dh).min(1.0);
            (pw, ph, s)
        }
    };

    let mut c = Content {
        ops: String::new(),
        scale: scale * PT_PER_MM,
        ox: (MARGIN + 4.0) * PT_PER_MM - b.min.x * scale * PT_PER_MM,
        oy: (MARGIN + BLOCK_H + 18.0) * PT_PER_MM - b.min.y * scale * PT_PER_MM,
    };
    let page_w_pt = page_w * PT_PER_MM;
    let page_h_pt = page_h * PT_PER_MM;

    // Watermark (under everything).
    if let Some(w) = &opts.watermark {
        let size = (page_w_pt.min(page_h_pt) / 7.0).clamp(24.0, 220.0);
        let (cx, cy) = (page_w_pt / 2.0, page_h_pt / 2.0 + BLOCK_H * PT_PER_MM / 2.0);
        let (s, co) = (0.5_f64.sin(), 0.5_f64.cos());
        let tw = text_width(w, size);
        c.op("q 0.9 0.9 0.9 rg");
        c.op(&format!(
            "BT /F1 {} Tf {} {} {} {} {} {} Tm ({}) Tj ET",
            num(size),
            num(co),
            num(s),
            num(-s),
            num(co),
            num(cx - co * tw / 2.0),
            num(cy - s * tw / 2.0),
            pdf_text(w)
        ));
        c.op("Q");
    }

    // Die-line, grouped by line type.
    let line_w = 0.25 * PT_PER_MM; // 0.25 mm on paper
    for t in &opts.include {
        let ents: Vec<_> = d.entities.iter().filter(|e| e.line_type == *t).collect();
        if ents.is_empty() {
            continue;
        }
        let (r, g, bl) = line_rgb(*t);
        let dash: Vec<String> = line_dash_mm(*t)
            .iter()
            .map(|v| num(v * PT_PER_MM))
            .collect();
        c.op(&format!(
            "q {} {} {} RG {} w 1 J 1 j [{}] 0 d",
            num(r),
            num(g),
            num(bl),
            num(line_w),
            dash.join(" ")
        ));
        for e in ents {
            for (i, seg) in e.path.segments.iter().enumerate() {
                let continues = i > 0
                    && e.path.segments[i - 1]
                        .end_point()
                        .approx_eq(seg.start_point());
                c.segment(seg, !continues);
            }
            if e.path.closed {
                c.op("h");
            }
            c.op("S");
        }
        c.op("Q");
    }

    // Overall dimensions (true sizes, even when scaled to fit).
    if opts.dimensions {
        let gap = 6.0 / scale; // 6 mm on paper, in drawing units
        let tick = 2.0 / scale;
        let font = 8.0;
        c.op(&format!(
            "q 0.45 0.45 0.47 RG 0.45 0.45 0.47 rg {} w",
            num(0.2 * PT_PER_MM)
        ));
        let y = b.min.y - gap;
        for (p0, p1) in [
            (Point::new(b.min.x, y), Point::new(b.max.x, y)),
            (Point::new(b.min.x, y - tick), Point::new(b.min.x, y + tick)),
            (Point::new(b.max.x, y - tick), Point::new(b.max.x, y + tick)),
        ] {
            let ((x0, y0), (x1, y1)) = (c.p(p0), c.p(p1));
            c.op(&format!(
                "{} {} m {} {} l S",
                num(x0),
                num(y0),
                num(x1),
                num(y1)
            ));
        }
        let wl = format!("{} mm", num(dw));
        let (mx, my) = c.p(Point::new((b.min.x + b.max.x) / 2.0, y));
        c.text(mx - text_width(&wl, font) / 2.0, my - font - 2.0, font, &wl);
        let x = b.max.x + gap;
        for (p0, p1) in [
            (Point::new(x, b.min.y), Point::new(x, b.max.y)),
            (Point::new(x - tick, b.min.y), Point::new(x + tick, b.min.y)),
            (Point::new(x - tick, b.max.y), Point::new(x + tick, b.max.y)),
        ] {
            let ((x0, y0), (x1, y1)) = (c.p(p0), c.p(p1));
            c.op(&format!(
                "{} {} m {} {} l S",
                num(x0),
                num(y0),
                num(x1),
                num(y1)
            ));
        }
        let hl = format!("{} mm", num(dh));
        let (hx, hy) = c.p(Point::new(x, (b.min.y + b.max.y) / 2.0));
        c.text(hx + 4.0, hy - font / 2.0, font, &hl);
        c.op("Q");
    }

    // Title block and legend along the bottom of the page.
    let bx = MARGIN * PT_PER_MM;
    let by = MARGIN * PT_PER_MM;
    let bw = page_w_pt - 2.0 * MARGIN * PT_PER_MM;
    let bh = BLOCK_H * PT_PER_MM;
    c.op(&format!(
        "q 0.2 0.2 0.22 RG 0.6 w {} {} {} {} re S Q",
        num(bx),
        num(by),
        num(bw),
        num(bh)
    ));
    c.op("q 0.1 0.1 0.12 rg");
    let mut ty = by + bh - 16.0;
    for (i, line) in opts.title_lines.iter().enumerate() {
        let size = if i == 0 { 13.0 } else { 8.5 };
        c.text(bx + 10.0, ty, size, line);
        ty -= if i == 0 { 15.0 } else { 11.0 };
        if ty < by + 6.0 {
            break;
        }
    }
    let scale_label = if (scale - 1.0).abs() < 1e-9 {
        "Scale 1:1 (print at 100%)".to_string()
    } else {
        format!("Scale 1:{:.2} (fitted to page)", 1.0 / scale)
    };
    c.text(
        bx + bw - text_width(&scale_label, 8.5) - 10.0,
        by + 8.0,
        8.5,
        &scale_label,
    );
    c.op("Q");

    // Legend: one swatch per line type present.
    let mut lx = bx + bw * 0.55;
    let ly = by + bh - 16.0;
    for t in opts.include.iter().filter(|_| opts.legend) {
        if !d.entities.iter().any(|e| e.line_type == *t) {
            continue;
        }
        let (r, g, bl) = line_rgb(*t);
        let dash: Vec<String> = line_dash_mm(*t)
            .iter()
            .map(|v| num(v * PT_PER_MM * 0.6))
            .collect();
        c.op(&format!(
            "q {} {} {} RG 1.2 w [{}] 0 d {} {} m {} {} l S Q",
            num(r),
            num(g),
            num(bl),
            dash.join(" "),
            num(lx),
            num(ly + 3.0),
            num(lx + 22.0),
            num(ly + 3.0)
        ));
        c.op("q 0.1 0.1 0.12 rg");
        let label = t.id().replace('-', " ");
        c.text(lx + 26.0, ly, 8.5, &label);
        c.op("Q");
        lx += 26.0 + text_width(&label, 8.5) + 14.0;
        if lx > bx + bw - 60.0 {
            break;
        }
    }

    assemble(page_w_pt, page_h_pt, &c.ops)
}

fn assemble(w: f64, h: f64, content: &str) -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
            num(w),
            num(h)
        ),
        format!("<< /Length {} >>\nstream\n{}endstream", content.len(), content),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_string(),
        "<< /Producer (PackForge) /Title (Die-line) >>".to_string(),
    ];
    let mut out = String::from("%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{}\nendobj\n", i + 1, body);
    }
    let xref = out.len();
    let _ = write!(out, "xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for o in offsets {
        let _ = writeln!(out, "{o:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R /Info 6 0 R >>\nstartxref\n{}\n%%EOF\n",
        objects.len() + 1,
        xref
    );
    out.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pf_geom::Path;

    fn sample() -> DieLine {
        let mut d = DieLine::default();
        d.push(
            LineType::Cut,
            Path::polyline(
                &[
                    Point::new(0.0, 0.0),
                    Point::new(200.0, 0.0),
                    Point::new(200.0, 100.0),
                    Point::new(0.0, 100.0),
                ],
                true,
            ),
        );
        d.push(
            LineType::Crease,
            Path::line(Point::new(100.0, 0.0), Point::new(100.0, 100.0)),
        );
        d.push(
            LineType::Cut,
            Path {
                segments: vec![Segment::Arc {
                    center: Point::new(50.0, 50.0),
                    radius: 10.0,
                    start: 0.0,
                    sweep: std::f64::consts::TAU,
                }],
                closed: true,
            },
        );
        d
    }

    #[test]
    fn pdf_structure_and_xref_offsets_are_valid() {
        let pdf = to_pdf(
            &sample(),
            &PdfOptions {
                title_lines: vec!["Test (box)".into(), "L=300 × W=200".into()],
                ..PdfOptions::default()
            },
        );
        let s = String::from_utf8(pdf.clone()).expect("ascii");
        assert!(s.starts_with("%PDF-1.4\n") && s.ends_with("%%EOF\n"));
        // startxref points at the xref table, and every xref entry points at "<n> 0 obj".
        let sx: usize = s
            .rsplit("startxref\n")
            .next()
            .and_then(|t| t.lines().next())
            .and_then(|v| v.parse().ok())
            .expect("startxref");
        assert!(s[sx..].starts_with("xref\n"));
        for (i, line) in s[sx..].lines().skip(3).take(6).enumerate() {
            let off: usize = line[..10].parse().expect("offset");
            assert!(
                s[off..].starts_with(&format!("{} 0 obj", i + 1)),
                "object {}",
                i + 1
            );
        }
        // Content stream length matches.
        let start = s.find("stream\n").expect("stream") + 7;
        let end = s.find("endstream").expect("endstream");
        let declared: usize = s[..start]
            .rsplit("/Length ")
            .next()
            .and_then(|t| t.split_whitespace().next())
            .and_then(|v| v.parse().ok())
            .expect("length");
        assert_eq!(declared, end - start);
        // Text is escaped and non-ASCII replaced.
        assert!(s.contains("(Test \\(box\\)) Tj"));
        assert!(s.contains("(L=300 x W=200) Tj"));
        // Arcs became Bézier curves; crease is dashed.
        assert!(s.contains(" c\n"));
        assert!(s.contains("] 0 d"));
    }

    #[test]
    fn actual_size_page_is_one_to_one() {
        let s = String::from_utf8(to_pdf(&sample(), &PdfOptions::default())).expect("ascii");
        // 200 mm drawing + margins → MediaBox width in points.
        let expected_w = (200.0 + 2.0 * MARGIN + 14.0) * PT_PER_MM; // width extra unchanged
        assert!(s.contains(&format!("/MediaBox [0 0 {}", num(expected_w))));
        assert!(s.contains("Scale 1:1"));
    }

    #[test]
    fn fitted_page_scales_down_and_says_so() {
        let mut d = sample();
        d.push(
            LineType::Cut,
            Path::line(Point::new(0.0, 0.0), Point::new(1500.0, 0.0)),
        );
        let s = String::from_utf8(to_pdf(
            &d,
            &PdfOptions {
                page: PageSize::A4,
                ..PdfOptions::default()
            },
        ))
        .expect("ascii");
        assert!(s.contains(&format!(
            "/MediaBox [0 0 {} {}]",
            num(297.0 * PT_PER_MM),
            num(210.0 * PT_PER_MM)
        )));
        assert!(s.contains("fitted to page"));
    }

    #[test]
    fn watermark_and_determinism() {
        let o = PdfOptions {
            watermark: Some("PackForge Free".into()),
            ..PdfOptions::default()
        };
        let a = to_pdf(&sample(), &o);
        assert_eq!(a, to_pdf(&sample(), &o));
        assert!(String::from_utf8(a)
            .expect("ascii")
            .contains("(PackForge Free) Tj"));
    }
}
