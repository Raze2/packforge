//! Evaluating a compiled component for concrete parameters and board.

use crate::{CExpr, CItem, CPoint, Component, ParamIssue, RenderError};
use pf_expr::{Env, Value as EValue};
use pf_geom::shape::{self, Node};
use pf_geom::{Point, Transform, EPS};
use pf_model::{Board, Diagnostic, DieLine, FoldModel, Hinge, LineType, Panel, Severity, Value};
use std::collections::BTreeMap;

/// Output of [`Component::render`].
#[derive(Debug, Clone, PartialEq)]
pub struct Rendered {
    pub dieline: DieLine,
    pub fold: FoldModel,
    pub diagnostics: Vec<Diagnostic>,
}

struct Scope(BTreeMap<String, EValue>);

impl Env for Scope {
    fn get(&self, name: &str) -> Option<EValue> {
        self.0.get(name).cloned()
    }
}

impl Scope {
    fn num(&self, e: &CExpr) -> Result<f64, RenderError> {
        e.expr.eval_num(self).map_err(|err| eval_err(e, err))
    }

    fn boolean(&self, e: &CExpr) -> Result<bool, RenderError> {
        e.expr.eval_bool(self).map_err(|err| eval_err(e, err))
    }

    fn point(&self, p: &CPoint) -> Result<Point, RenderError> {
        Ok(Point::new(self.num(&p.0)?, self.num(&p.1)?))
    }
}

fn eval_err(e: &CExpr, err: pf_expr::ExprError) -> RenderError {
    RenderError::Eval {
        context: e.context.clone(),
        message: err.message,
    }
}

impl Component {
    /// Renders the component. Missing params take their defaults; params are millimetres.
    pub fn render(
        &self,
        params: &BTreeMap<String, Value>,
        board: &Board,
    ) -> Result<Rendered, RenderError> {
        let mut diagnostics = Vec::new();
        let mut scope = Scope(BTreeMap::new());

        // 1. Parameters.
        let mut issues = Vec::new();
        for def in &self.params {
            let v = params.get(&def.id).unwrap_or(&def.default);
            match def.check_value(v) {
                Ok(()) => {
                    scope.0.insert(def.id.clone(), to_expr_value(v));
                }
                Err(message) => issues.push(ParamIssue {
                    param: def.id.clone(),
                    message,
                }),
            }
        }
        if !issues.is_empty() {
            return Err(RenderError::Params(issues));
        }
        for key in params.keys() {
            if !self.params.iter().any(|p| &p.id == key) {
                diagnostics.push(Diagnostic {
                    severity: Severity::Warning,
                    message: format!("unknown parameter '{key}' ignored"),
                });
            }
        }

        // 2. Board.
        if !(board.caliper_mm.is_finite() && board.caliper_mm > 0.0) {
            return Err(RenderError::Params(vec![ParamIssue {
                param: "board.caliper".into(),
                message: "must be a positive number".into(),
            }]));
        }
        scope
            .0
            .insert("board.caliper".into(), EValue::Num(board.caliper_mm));
        scope
            .0
            .insert("board.id".into(), EValue::Str(board.id.clone()));
        scope.0.insert(
            "board.flute".into(),
            EValue::Str(board.flute.clone().unwrap_or_default()),
        );

        // 3. Derived values (already in dependency order).
        for (name, e) in &self.derived {
            let v = e.expr.eval(&scope).map_err(|err| eval_err(e, err))?;
            scope.0.insert(name.clone(), v);
        }

        // 4. Rules.
        for r in &self.rules {
            if scope.boolean(&r.when)? {
                if r.severity == Severity::Error {
                    return Err(RenderError::Rule(r.message.clone()));
                }
                diagnostics.push(Diagnostic {
                    severity: r.severity,
                    message: r.message.clone(),
                });
            }
        }

        // 5. Geometry.
        let mut dieline = DieLine::default();
        for p in &self.paths {
            if let Some(w) = &p.when {
                if !scope.boolean(w)? {
                    continue;
                }
            }
            let mut nodes = Vec::with_capacity(p.points.len());
            for item in &p.points {
                nodes.push(match item {
                    CItem::Point(pt) => Node::Corner {
                        p: scope.point(pt)?,
                        round: 0.0,
                    },
                    CItem::Rounded(pt, r) => Node::Corner {
                        p: scope.point(pt)?,
                        round: scope.num(r)?,
                    },
                    CItem::Arc {
                        to,
                        radius,
                        ccw,
                        large,
                    } => Node::ArcTo {
                        p: scope.point(to)?,
                        radius: scope.num(radius)?,
                        ccw: *ccw,
                        large: *large,
                    },
                });
            }
            let built = shape::build(&nodes, p.closed).map_err(|message| RenderError::Eval {
                context: p.context.clone(),
                message,
            })?;
            diagnostics.extend(built.notes.into_iter().map(|n| Diagnostic {
                severity: Severity::Info,
                message: format!("{}: {n}", p.context),
            }));
            let path = built.path;
            if path.segments.is_empty() {
                diagnostics.push(Diagnostic {
                    severity: Severity::Info,
                    message: format!("zero-length {} skipped", p.line_type.id()),
                });
                continue;
            }
            dieline.push(p.line_type, path);
        }

        let mut fold = FoldModel {
            thickness: board.caliper_mm,
            ..FoldModel::default()
        };
        for p in &self.panels {
            let (w, h) = (scope.num(&p.w)?, scope.num(&p.h)?);
            if w <= EPS || h <= EPS {
                return Err(RenderError::Geometry(format!(
                    "panel '{}' has non-positive size {w:.3} × {h:.3}",
                    p.id
                )));
            }
            fold.panels.push(Panel {
                id: p.id.clone(),
                origin: Point::new(scope.num(&p.x)?, scope.num(&p.y)?),
                width: w,
                height: h,
            });
        }
        for h in &self.hinges {
            fold.hinges.push(Hinge {
                a: h.a.clone(),
                b: h.b.clone(),
                from: scope.point(&h.from)?,
                to: scope.point(&h.to)?,
                closed_angle: scope.num(&h.closed_angle)?,
            });
        }

        // 6. Layout transforms.
        if let Some(m) = &self.mirror_x {
            if scope.boolean(m)? {
                mirror(&mut dieline, &mut fold);
            }
        }

        check_geometry(&dieline)?;
        Ok(Rendered {
            dieline,
            fold,
            diagnostics,
        })
    }
}

fn to_expr_value(v: &Value) -> EValue {
    match v {
        Value::Number(n) => EValue::Num(*n),
        Value::Bool(b) => EValue::Bool(*b),
        Value::Text(s) => EValue::Str(s.clone()),
    }
}

/// Mirrors the design left↔right around the centre of the blank.
fn mirror(dieline: &mut DieLine, fold: &mut FoldModel) {
    let b = dieline.bbox();
    let t = Transform::mirror_x((b.min.x + b.max.x) / 2.0);
    for e in &mut dieline.entities {
        e.path = e.path.transform(&t);
    }
    for p in &mut fold.panels {
        let right = t.apply(Point::new(p.origin.x + p.width, p.origin.y));
        p.origin = right;
    }
    for h in &mut fold.hinges {
        h.from = t.apply(h.from);
        h.to = t.apply(h.to);
    }
}

fn check_geometry(d: &DieLine) -> Result<(), RenderError> {
    if let Some(e) = d.entities.iter().find(|e| !e.path.is_finite()) {
        return Err(RenderError::Geometry(format!(
            "non-finite coordinates in a {} line",
            e.line_type.id()
        )));
    }
    let outlines: Vec<_> = d
        .entities
        .iter()
        .filter(|e| e.line_type == LineType::Cut && e.path.closed)
        .collect();
    if outlines.is_empty() {
        return Err(RenderError::Geometry("no closed cut outline".into()));
    }
    for o in &outlines {
        if !o.path.is_continuous() {
            return Err(RenderError::Geometry(
                "cut outline is not continuous".into(),
            ));
        }
        if o.path.signed_area().abs() <= EPS {
            return Err(RenderError::Geometry("cut outline has zero area".into()));
        }
        let crossings = o.path.self_crossings();
        if crossings > 0 {
            return Err(RenderError::Geometry(format!(
                "cut outline crosses itself ({crossings} place(s)); check the dimensions"
            )));
        }
    }
    // Every tooling line must lie within the outer cut outline's bounds.
    let mut b = pf_geom::BBox::empty();
    for o in &outlines {
        b.union(&o.path.bbox());
    }
    let tol = 1e-6;
    for e in d.entities.iter().filter(|e| e.line_type.is_tooling()) {
        let eb = e.path.bbox();
        if eb.min.x < b.min.x - tol
            || eb.min.y < b.min.y - tol
            || eb.max.x > b.max.x + tol
            || eb.max.y > b.max.y + tol
        {
            return Err(RenderError::Geometry(format!(
                "{} line lies outside the blank",
                e.line_type.id()
            )));
        }
    }
    Ok(())
}
