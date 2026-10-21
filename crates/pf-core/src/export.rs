//! Export pipeline: render → layout (rotate, mirror, nest on a sheet) → writer.

use crate::{board_colour, render, CoreError, ExportFile};
use pf_export::{Format, PageSize, PdfOptions, Profile, SvgOptions};
use pf_geom::{Path, Point, Transform};
use pf_model::{DieLine, LineType, Project, RenderResult, Value};
use pf_optimize::{nest_grid, Nesting};
use serde::{Deserialize, Serialize};

/// Production options for an export. Every field is optional; defaults give a plain 1:1 file.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ExportOptions {
    /// Rotate the design counter-clockwise: 0, 90, 180 or 270 degrees.
    pub rotate: i32,
    /// Mirror left↔right (e.g. to cut from the inside/unprinted side).
    pub mirror: bool,
    /// Lay out several blanks on a sheet.
    pub nesting: Option<Nesting>,
    /// PDF page: "actual" (1:1, default), "a4", "a3" or "letter" (fitted).
    pub page: Option<String>,
    /// Diagonal text behind PDF drawings (e.g. "PackForge Free").
    pub watermark: Option<String>,
    /// Built-in output profile id ("standard", "cut-crease") for DXF/CF2/HPGL mapping.
    pub profile: Option<String>,
    /// A custom output profile (overrides `profile`).
    pub custom_profile: Option<Profile>,
}

/// Result of a nesting preview.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NestSummary {
    pub count: usize,
    pub cols: usize,
    pub rows: usize,
    pub rotated: bool,
    pub blank_width: f64,
    pub blank_height: f64,
    pub sheet_area_m2: f64,
    /// Share of the sheet covered by board inside the cut outlines (0..1).
    pub utilisation: f64,
}

/// Renders and exports with default options.
pub fn export(project: &Project, format: Format) -> Result<ExportFile, CoreError> {
    export_with(project, format, &ExportOptions::default())
}

/// Renders and exports with production options.
pub fn export_with(
    project: &Project,
    format: Format,
    opts: &ExportOptions,
) -> Result<ExportFile, CoreError> {
    let r = render(project)?;
    let filename = format!("{}.{}", r.component_id, format.extension());
    if format == Format::Glb {
        let folded = pf_fold3d::fold(&r.fold, 1.0)?;
        return Ok(ExportFile {
            bytes: pf_fold3d::to_glb(&folded, board_colour(&r.board)),
            mime: format.mime(),
            filename,
        });
    }

    let profile = match (&opts.custom_profile, &opts.profile) {
        (Some(p), _) => p.clone(),
        (None, Some(id)) => Profile::builtin(id)
            .ok_or_else(|| CoreError::Export(format!("unknown output profile '{id}'")))?,
        (None, None) => Profile::standard(),
    };
    let (dieline, nest) = layout(&r, opts)?;

    let bytes = match format {
        Format::Svg => pf_export::to_svg(
            &dieline,
            &SvgOptions {
                title: Some(format!("{} — {}", r.component_id, r.board.id)),
                ..SvgOptions::default()
            },
        )
        .into_bytes(),
        Format::Dxf => {
            let layer_names = profile
                .lines
                .iter()
                .map(|(t, m)| (*t, m.layer.clone()))
                .collect();
            pf_export::to_dxf(&dieline, &pf_export::DxfOptions { layer_names }).into_bytes()
        }
        Format::Cf2 => pf_export::to_cf2(&dieline, &profile, &r.component_id).into_bytes(),
        Format::Hpgl => pf_export::to_hpgl(&dieline, &profile).into_bytes(),
        Format::Pdf => {
            let page = match &opts.page {
                None => PageSize::Actual,
                Some(p) => PageSize::parse(p)
                    .ok_or_else(|| CoreError::Export(format!("unknown page size '{p}'")))?,
            };
            pf_export::to_pdf(
                &dieline,
                &PdfOptions {
                    page,
                    title_lines: title_lines(project, &r, nest.as_ref()),
                    watermark: opts.watermark.clone(),
                    ..PdfOptions::default()
                },
            )
        }
        Format::Glb => unreachable!("handled above"),
    };
    Ok(ExportFile {
        bytes,
        mime: format.mime(),
        filename,
    })
}

/// How many blanks fit on a sheet, without exporting.
pub fn nest_preview(project: &Project, nesting: &Nesting) -> Result<NestSummary, CoreError> {
    let r = render(project)?;
    let opts = ExportOptions {
        nesting: Some(*nesting),
        ..ExportOptions::default()
    };
    let (_, nest) = layout(&r, &opts)?;
    nest.ok_or_else(|| CoreError::Export("nesting failed".into()))
}

/// Applies rotation/mirroring, then nesting, to the rendered die-line.
fn layout(
    r: &RenderResult,
    opts: &ExportOptions,
) -> Result<(DieLine, Option<NestSummary>), CoreError> {
    if opts.rotate.rem_euclid(90) != 0 {
        return Err(CoreError::Export(format!(
            "rotation must be 0, 90, 180 or 270 degrees, got {}",
            opts.rotate
        )));
    }
    let b = r.dieline.bbox();
    let centre = Point::new((b.min.x + b.max.x) / 2.0, (b.min.y + b.max.y) / 2.0);
    let mut t = Transform::rotate(f64::from(opts.rotate), centre);
    if opts.mirror {
        t = t.then(&Transform::mirror_x(centre.x));
    }
    let mut single = transform(&r.dieline, &t);
    // Normalise so the blank's lower-left corner is at the origin.
    let nb = single.bbox();
    single = transform(&single, &Transform::translate(-nb.min.x, -nb.min.y));
    let (w, h) = (nb.width(), nb.height());

    let Some(n) = &opts.nesting else {
        return Ok((single, None));
    };
    if !(n.sheet_width > 0.0 && n.sheet_height > 0.0 && n.gap >= 0.0 && n.margin >= 0.0) {
        return Err(CoreError::Export(
            "sheet size, gap and margin must be positive".into(),
        ));
    }
    let grid = nest_grid(w, h, n);
    if grid.count() == 0 {
        return Err(CoreError::Export(format!(
            "a {:.0} × {:.0} mm blank does not fit on a {:.0} × {:.0} mm sheet with a {:.0} mm margin",
            w, h, n.sheet_width, n.sheet_height, n.margin
        )));
    }
    let turned = transform(
        &single,
        &Transform::translate(h, 0.0).then(&Transform::rotate(90.0, Point::new(0.0, 0.0))),
    );
    let mut sheet = DieLine::default();
    for p in &grid.placements {
        let src = if p.rotated { &turned } else { &single };
        let moved = transform(src, &Transform::translate(p.x, p.y));
        sheet.entities.extend(moved.entities);
    }
    // Sheet outline as a drawing aid (excluded from CF2/HPGL by the standard profile).
    sheet.push(
        LineType::Annotation,
        Path::polyline(
            &[
                Point::new(0.0, 0.0),
                Point::new(n.sheet_width, 0.0),
                Point::new(n.sheet_width, n.sheet_height),
                Point::new(0.0, n.sheet_height),
            ],
            true,
        ),
    );
    let sheet_area = n.sheet_width * n.sheet_height;
    let summary = NestSummary {
        count: grid.count(),
        cols: grid.cols,
        rows: grid.rows,
        rotated: grid.rotated,
        blank_width: w,
        blank_height: h,
        sheet_area_m2: sheet_area / 1e6,
        utilisation: grid.count() as f64 * r.dieline.outline_area() / sheet_area,
    };
    Ok((sheet, Some(summary)))
}

fn transform(d: &DieLine, t: &Transform) -> DieLine {
    let mut out = DieLine::default();
    for e in &d.entities {
        out.push(e.line_type, e.path.transform(t));
    }
    out
}

fn title_lines(project: &Project, r: &RenderResult, nest: Option<&NestSummary>) -> Vec<String> {
    let info = crate::describe(&r.component_id).ok();
    let name = info
        .as_ref()
        .map(|i| i.meta.name.clone())
        .unwrap_or_default();
    let standard = info
        .as_ref()
        .and_then(|i| i.meta.standard.clone())
        .map(|s| format!(" ({s})"))
        .unwrap_or_default();
    let params: Vec<String> = info
        .map(|i| {
            i.params
                .iter()
                .map(|p| {
                    let v = project.params.get(&p.id).unwrap_or(&p.default);
                    let shown = match v {
                        Value::Number(n) => format!("{}", (n * 100.0).round() / 100.0),
                        Value::Bool(b) => b.to_string(),
                        Value::Text(t) => t.clone(),
                    };
                    format!("{}={}", p.id, shown)
                })
                .collect()
        })
        .unwrap_or_default();
    let s = r.stats;
    let mut lines = vec![
        format!("{name}{standard}"),
        format!("{} (mm)", params.join("  ")),
        format!(
            "Board {} ({} mm)  -  blank {:.1} x {:.1} mm  -  {:.3} m2  -  cut {:.2} m  -  crease {:.2} m",
            r.board.id,
            r.board.caliper_mm,
            s.blank_width,
            s.blank_height,
            s.blank_area_m2,
            s.cut_length / 1000.0,
            s.crease_length / 1000.0
        ),
    ];
    if let Some(n) = nest {
        lines.push(format!(
            "{} up ({} x {}{}) on the sheet  -  {:.0}% of the sheet used",
            n.count,
            n.cols,
            n.rows,
            if n.rotated { ", turned 90 deg" } else { "" },
            n.utilisation * 100.0
        ));
    }
    lines.push(format!(
        "{} v{}  -  PackForge",
        r.component_id, r.component_version
    ));
    lines
}
