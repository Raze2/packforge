//! Pallet plans for a design, case sizing ranked by board use, cost and a PDF pallet report.

use crate::{render, resolve_board, CoreError};
use pf_export::{PageSize, PdfOptions};
use pf_geom::{Path, Point};
use pf_model::{BoardRef, DieLine, LineType, Project, Units, Value};
use pf_optimize::pallet::{plan, Load, Pallet, PalletPlan};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A pallet preset id ("eur", "gma", …) or explicit dimensions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PalletChoice {
    Preset(String),
    Custom(Pallet),
}

impl Default for PalletChoice {
    fn default() -> Self {
        PalletChoice::Preset("eur".into())
    }
}

fn default_height() -> f64 {
    1800.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PalletRequest {
    #[serde(default)]
    pub pallet: PalletChoice,
    /// Maximum total height including the pallet (mm).
    #[serde(default = "default_height")]
    pub max_height: f64,
    #[serde(default)]
    pub box_weight: Option<f64>,
    #[serde(default)]
    pub max_weight: Option<f64>,
    #[serde(default)]
    pub overhang: f64,
    #[serde(default)]
    pub allow_on_side: bool,
    /// Measured outer size [L, W, H] (mm). Default: nominal, inside + two calipers.
    #[serde(default)]
    pub box_outer: Option<[f64; 3]>,
}

impl Default for PalletRequest {
    fn default() -> Self {
        Self {
            pallet: PalletChoice::default(),
            max_height: default_height(),
            box_weight: None,
            max_weight: None,
            overhang: 0.0,
            allow_on_side: false,
            box_outer: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PalletResult {
    pub pallet_name: String,
    /// Outer box size used [L, W, H] (mm).
    pub box_outer: [f64; 3],
    pub plan: PalletPlan,
}

fn resolve_pallet(choice: &PalletChoice) -> Result<(Pallet, String), CoreError> {
    match choice {
        PalletChoice::Preset(id) => Pallet::preset(id)
            .map(|(p, name)| (p, name.to_string()))
            .ok_or_else(|| {
                CoreError::Export(format!(
                    "unknown pallet '{id}' (use {})",
                    Pallet::PRESETS.join(", ")
                ))
            }),
        PalletChoice::Custom(p) => {
            if p.length > 0.0 && p.width > 0.0 && p.height >= 0.0 {
                Ok((*p, format!("Custom {:.0} × {:.0}", p.length, p.width)))
            } else {
                Err(CoreError::Export(
                    "pallet dimensions must be positive".into(),
                ))
            }
        }
    }
}

fn param(project: &Project, id: &str) -> Option<f64> {
    match project.params.get(id) {
        Some(Value::Number(n)) => Some(*n),
        _ => crate::describe(&project.component_id).ok().and_then(|i| {
            i.params
                .into_iter()
                .find(|p| p.id == id)
                .and_then(|p| match p.default {
                    Value::Number(n) => Some(n),
                    _ => None,
                })
        }),
    }
}

/// Nominal outer size of the erected box: inside L/W/H plus two board thicknesses each.
pub fn box_outer(project: &Project) -> Result<[f64; 3], CoreError> {
    let board = resolve_board(&project.board)?;
    let t = board.caliper_mm;
    let get = |id: &str| {
        param(project, id).ok_or_else(|| {
            CoreError::Export(format!(
                "'{}' has no '{id}' dimension; give the outer size explicitly",
                project.component_id
            ))
        })
    };
    Ok([
        get("L")? + 2.0 * t,
        get("W")? + 2.0 * t,
        get("H")? + 2.0 * t,
    ])
}

/// Pallet plan for the project's box.
pub fn pallet_plan(project: &Project, req: &PalletRequest) -> Result<PalletResult, CoreError> {
    render(project)?; // validates the design
    let (pallet, pallet_name) = resolve_pallet(&req.pallet)?;
    let outer = match req.box_outer {
        Some(o) => o,
        None => box_outer(project)?,
    };
    pallet_for(outer, pallet, pallet_name, req)
}

fn pallet_for(
    outer: [f64; 3],
    pallet: Pallet,
    pallet_name: String,
    req: &PalletRequest,
) -> Result<PalletResult, CoreError> {
    let load = Load {
        box_length: outer[0],
        box_width: outer[1],
        box_height: outer[2],
        box_weight: req.box_weight,
        max_height: req.max_height,
        max_weight: req.max_weight,
        overhang: req.overhang,
        allow_on_side: req.allow_on_side,
    };
    let plan = plan(pallet, &load).map_err(|e| CoreError::Export(e.to_string()))?;
    Ok(PalletResult {
        pallet_name,
        box_outer: outer,
        plan,
    })
}

// ── Case sizing ────────────────────────────────────────────────────────────

fn default_component() -> String {
    "fefco-0201".into()
}
fn default_board() -> BoardRef {
    BoardRef::Preset("C-flute".into())
}
fn default_clearance() -> f64 {
    4.0
}
fn default_results() -> usize {
    10
}
fn yes() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseRequest {
    /// Product size [length, width, height] (mm).
    pub product: [f64; 3],
    /// Products per case.
    pub count: usize,
    #[serde(default = "default_component")]
    pub component_id: String,
    #[serde(default = "default_board")]
    pub board: BoardRef,
    /// Extra inside space per dimension (mm), shared by both walls.
    #[serde(default = "default_clearance")]
    pub clearance: f64,
    /// Products may be turned in any direction (false: product height stays vertical).
    #[serde(default = "yes")]
    pub allow_rotation: bool,
    #[serde(default = "default_results")]
    pub max_results: usize,
    /// Also plan pallets and rank by products per pallet.
    #[serde(default)]
    pub pallet: Option<PalletRequest>,
    /// Board price per m² for a cost estimate.
    #[serde(default)]
    pub board_price_per_m2: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CaseCandidate {
    /// Products along the case's length, width and height.
    pub arrangement: [usize; 3],
    /// Product dimensions along the case's length, width and height.
    pub product_axes: [f64; 3],
    /// Inside size [L, W, H] to use as the design's parameters.
    pub inside: [f64; 3],
    pub outer: [f64; 3],
    pub blank_area_m2: f64,
    pub board_m2_per_product: f64,
    pub board_cost_per_case: Option<f64>,
    pub cases_per_pallet: Option<usize>,
    pub products_per_pallet: Option<usize>,
}

fn factorizations(n: usize) -> Vec<[usize; 3]> {
    let mut out = Vec::new();
    for a in 1..=n {
        if n % a != 0 {
            continue;
        }
        for b in 1..=n / a {
            if (n / a) % b == 0 {
                out.push([a, b, n / a / b]);
            }
        }
    }
    out
}

/// Ranks case designs for `count` products: least board per product first (or most products per
/// pallet first when a pallet is given).
pub fn case_optimize(req: &CaseRequest) -> Result<Vec<CaseCandidate>, CoreError> {
    let [pa, pb, pc] = req.product;
    if !(pa > 0.0 && pb > 0.0 && pc > 0.0) || req.count == 0 || req.count > 10_000 {
        return Err(CoreError::Export(
            "product size must be positive and count between 1 and 10 000".into(),
        ));
    }
    let board = resolve_board(&req.board)?;
    let t = board.caliper_mm;
    let pallet = req
        .pallet
        .as_ref()
        .map(|p| resolve_pallet(&p.pallet).map(|r| (r, p)))
        .transpose()?;

    let orientations: Vec<[f64; 3]> = if req.allow_rotation {
        vec![
            [pa, pb, pc],
            [pb, pa, pc],
            [pa, pc, pb],
            [pc, pa, pb],
            [pb, pc, pa],
            [pc, pb, pa],
        ]
    } else {
        vec![[pa, pb, pc], [pb, pa, pc]]
    };

    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for axes in orientations {
        for arr in factorizations(req.count) {
            let mut inside = [
                arr[0] as f64 * axes[0] + req.clearance,
                arr[1] as f64 * axes[1] + req.clearance,
                arr[2] as f64 * axes[2] + req.clearance,
            ];
            let (mut arr, mut axes) = (arr, axes);
            // Length is the longer side by convention.
            if inside[1] > inside[0] {
                inside.swap(0, 1);
                arr.swap(0, 1);
                axes.swap(0, 1);
            }
            let key = inside.map(|v| (v * 10.0).round() as i64);
            if !seen.insert(key) {
                continue;
            }
            let project = Project {
                component_id: req.component_id.clone(),
                component_version: None,
                params: BTreeMap::from([
                    ("L".to_string(), Value::Number(inside[0])),
                    ("W".to_string(), Value::Number(inside[1])),
                    ("H".to_string(), Value::Number(inside[2])),
                ]),
                board: req.board.clone(),
                units: Units::Mm,
            };
            let Ok(r) = render(&project) else { continue };
            let outer = inside.map(|v| v + 2.0 * t);
            let (cases, products) = match &pallet {
                Some(((p, name), preq)) => match pallet_for(outer, *p, name.clone(), preq) {
                    Ok(pr) => (Some(pr.plan.total), Some(pr.plan.total * req.count)),
                    Err(_) => (Some(0), Some(0)),
                },
                None => (None, None),
            };
            out.push(CaseCandidate {
                arrangement: arr,
                product_axes: axes,
                inside,
                outer,
                blank_area_m2: r.stats.blank_area_m2,
                board_m2_per_product: r.stats.blank_area_m2 / req.count as f64,
                board_cost_per_case: req.board_price_per_m2.map(|p| p * r.stats.blank_area_m2),
                cases_per_pallet: cases,
                products_per_pallet: products,
            });
        }
    }
    if out.is_empty() {
        return Err(CoreError::Export(format!(
            "no case of style '{}' can hold {} products of this size",
            req.component_id, req.count
        )));
    }
    out.sort_by(|a, b| {
        let by_pallet = b.products_per_pallet.cmp(&a.products_per_pallet);
        let by_board = a
            .board_m2_per_product
            .partial_cmp(&b.board_m2_per_product)
            .unwrap_or(std::cmp::Ordering::Equal);
        if pallet.is_some() {
            by_pallet.then(by_board)
        } else {
            by_board
        }
    });
    out.truncate(req.max_results.max(1));
    Ok(out)
}

// ── Pallet report ──────────────────────────────────────────────────────────

/// A4 PDF with the layer pattern (top view), the stack (side view) and the figures.
pub fn pallet_report(project: &Project, req: &PalletRequest) -> Result<Vec<u8>, CoreError> {
    let res = pallet_plan(project, req)?;
    let p = &res.plan;
    let mut d = DieLine::default();
    let rect = |x: f64, y: f64, w: f64, h: f64| {
        Path::polyline(
            &[
                Point::new(x, y),
                Point::new(x + w, y),
                Point::new(x + w, y + h),
                Point::new(x, y + h),
            ],
            true,
        )
    };
    // Top view: pallet outline and the boxes of one layer.
    d.push(
        LineType::Annotation,
        rect(0.0, 0.0, p.pallet.length, p.pallet.width),
    );
    for r in &p.layer {
        d.push(LineType::Cut, rect(r.x, r.y, r.w, r.h));
    }
    // Side view to the right: pallet block and the stacked layers (along the pallet length).
    let sx = p.pallet.length + 300.0;
    d.push(
        LineType::Annotation,
        rect(sx, 0.0, p.pallet.length, p.pallet.height),
    );
    let (min_x, max_x) = p
        .layer
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), r| {
            (a.min(r.x), b.max(r.x + r.w))
        });
    for k in 0..p.layers {
        let y = p.pallet.height + k as f64 * p.layer_height;
        d.push(
            LineType::Cut,
            rect(sx + min_x, y, max_x - min_x, p.layer_height),
        );
    }
    let r = render(project)?;
    let [l, w, h] = res.box_outer;
    let mut lines = vec![
        format!("Pallet plan - {} on {}", r.component_id, res.pallet_name),
        format!(
            "Box outer {:.0} x {:.0} x {:.0} mm, standing on its {}  -  pattern: {}{}",
            l,
            w,
            h,
            p.standing_on,
            p.pattern.label(),
            if p.alternate {
                " (alternate layers mirrored)"
            } else {
                ""
            }
        ),
        format!(
            "{} per layer x {} layers = {} boxes  -  load height {:.0} mm (limit {:.0}, by {})",
            p.per_layer, p.layers, p.total, p.load_height, req.max_height, p.limited_by
        ),
        format!(
            "Layer area used {:.0}%  -  volume used {:.0}%{}",
            p.area_efficiency * 100.0,
            p.volume_efficiency * 100.0,
            p.load_weight
                .map(|w| format!("  -  load weight {w:.0} kg"))
                .unwrap_or_default()
        ),
    ];
    lines.push("Left: top view of one layer. Right: side view of the stack. PackForge".into());
    Ok(pf_export::to_pdf(
        &d,
        &PdfOptions {
            page: PageSize::A4,
            title_lines: lines,
            dimensions: false,
            legend: false,
            ..PdfOptions::default()
        },
    ))
}
