//! Component format (`*.pfc.toml`): parsing, static validation and evaluation.
//!
//! See `docs/04-component-format.md`. A [`Component`] is compiled once from TOML (all formulas
//! parsed, names checked, derived values ordered) and can then be rendered many times.

mod format;
mod render;

pub use format::{ComponentInfo, Meta, ParamDef, ParamType, Tier};
pub use render::Rendered;

use pf_expr::Expr;
use std::collections::{BTreeMap, BTreeSet};

/// Names every component may reference besides its own params and derived values.
pub const BOARD_NAMES: [&str; 3] = ["board.caliper", "board.id", "board.flute"];

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ComponentError {
    #[error("invalid component file: {0}")]
    Syntax(String),
    #[error("{context}: {message}")]
    Invalid { context: String, message: String },
}

impl ComponentError {
    fn invalid(context: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Invalid {
            context: context.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParamIssue {
    pub param: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RenderError {
    #[error("invalid parameters: {}", .0.iter().map(|i| format!("{}: {}", i.param, i.message)).collect::<Vec<_>>().join("; "))]
    Params(Vec<ParamIssue>),
    #[error("{0}")]
    Rule(String),
    #[error("{context}: {message}")]
    Eval { context: String, message: String },
    #[error("geometry check failed: {0}")]
    Geometry(String),
}

/// A compiled expression with where it came from (for error messages).
#[derive(Debug, Clone)]
pub(crate) struct CExpr {
    pub expr: Expr,
    pub context: String,
}

#[derive(Debug, Clone)]
pub(crate) struct CPoint(pub CExpr, pub CExpr);

#[derive(Debug, Clone)]
pub(crate) struct CRule {
    pub when: CExpr,
    pub severity: pf_model::Severity,
    pub message: String,
}

#[derive(Debug, Clone)]
pub(crate) struct CPanel {
    pub id: String,
    pub x: CExpr,
    pub y: CExpr,
    pub w: CExpr,
    pub h: CExpr,
}

#[derive(Debug, Clone)]
pub(crate) struct CHinge {
    pub a: String,
    pub b: String,
    pub from: CPoint,
    pub to: CPoint,
    pub closed_angle: CExpr,
}

#[derive(Debug, Clone)]
pub(crate) struct CPath {
    pub line_type: pf_model::LineType,
    pub points: Vec<CPoint>,
    pub closed: bool,
    pub when: Option<CExpr>,
}

/// A compiled, validated component ready to render.
#[derive(Debug, Clone)]
pub struct Component {
    pub meta: Meta,
    pub params: Vec<ParamDef>,
    pub(crate) derived: Vec<(String, CExpr)>,
    pub(crate) rules: Vec<CRule>,
    pub(crate) mirror_x: Option<CExpr>,
    pub(crate) panels: Vec<CPanel>,
    pub(crate) hinges: Vec<CHinge>,
    pub(crate) paths: Vec<CPath>,
}

impl Component {
    /// Parses and validates a component from TOML source.
    pub fn from_toml(src: &str) -> Result<Self, ComponentError> {
        let file: format::File =
            toml::from_str(src).map_err(|e| ComponentError::Syntax(e.message().to_string()))?;
        compile(file)
    }

    pub fn id(&self) -> &str {
        &self.meta.id
    }

    pub fn info(&self) -> ComponentInfo {
        ComponentInfo {
            meta: self.meta.clone(),
            params: self.params.clone(),
        }
    }
}

fn compile(file: format::File) -> Result<Component, ComponentError> {
    let meta = file.component;
    if meta.id.trim().is_empty() {
        return Err(ComponentError::invalid("component", "id must not be empty"));
    }

    // Parameters: unique ids, sane defaults.
    let mut known: BTreeSet<String> = BOARD_NAMES.iter().map(|s| s.to_string()).collect();
    for p in &file.params {
        let ctx = format!("param '{}'", p.id);
        if !is_identifier(&p.id) {
            return Err(ComponentError::invalid(
                ctx,
                "id must be a plain identifier",
            ));
        }
        if !known.insert(p.id.clone()) {
            return Err(ComponentError::invalid(ctx, "duplicate id"));
        }
        if let Err(m) = p.check_value(&p.default) {
            return Err(ComponentError::invalid(ctx, format!("default value: {m}")));
        }
    }

    // Derived values: parse, check names, order topologically.
    let derived_names: BTreeSet<String> = file.derived.keys().cloned().collect();
    for name in &derived_names {
        if !is_identifier(name) {
            return Err(ComponentError::invalid(
                format!("derived '{name}'"),
                "name must be a plain identifier",
            ));
        }
        if known.contains(name) {
            return Err(ComponentError::invalid(
                format!("derived '{name}'"),
                "name clashes with a parameter or board value",
            ));
        }
    }
    let mut all_names = known.clone();
    all_names.extend(derived_names.iter().cloned());

    let mut parsed = BTreeMap::new();
    for (name, src) in &file.derived {
        parsed.insert(
            name.clone(),
            cexpr(src, format!("derived '{name}'"), &all_names)?,
        );
    }
    let derived = topo_order(parsed, &derived_names)?;

    let c = |src: &format::ExprSrc, ctx: String| cexpr(src, ctx, &all_names);
    let cpoint = |p: &[format::ExprSrc; 2], ctx: &str| -> Result<CPoint, ComponentError> {
        Ok(CPoint(
            c(&p[0], format!("{ctx}.x"))?,
            c(&p[1], format!("{ctx}.y"))?,
        ))
    };

    let rules = file
        .rules
        .iter()
        .enumerate()
        .map(|(i, r)| {
            Ok(CRule {
                when: c(&r.when, format!("rule #{}", i + 1))?,
                severity: r.level,
                message: r.message.clone(),
            })
        })
        .collect::<Result<Vec<_>, ComponentError>>()?;

    let mirror_x = file
        .layout
        .as_ref()
        .and_then(|l| l.mirror_x.as_ref())
        .map(|e| c(e, "layout.mirror_x".into()))
        .transpose()?;

    let mut panel_ids = BTreeSet::new();
    let mut panels = Vec::new();
    for p in &file.panels {
        let ctx = format!("panel '{}'", p.id);
        if !panel_ids.insert(p.id.clone()) {
            return Err(ComponentError::invalid(ctx, "duplicate id"));
        }
        panels.push(CPanel {
            id: p.id.clone(),
            x: c(&p.x, format!("{ctx}.x"))?,
            y: c(&p.y, format!("{ctx}.y"))?,
            w: c(&p.w, format!("{ctx}.w"))?,
            h: c(&p.h, format!("{ctx}.h"))?,
        });
    }

    let mut hinges = Vec::new();
    for (i, h) in file.hinges.iter().enumerate() {
        let ctx = format!("hinge #{} ({}–{})", i + 1, h.a, h.b);
        for end in [&h.a, &h.b] {
            if !panel_ids.contains(end) {
                return Err(ComponentError::invalid(
                    &ctx,
                    format!("unknown panel '{end}'"),
                ));
            }
        }
        hinges.push(CHinge {
            a: h.a.clone(),
            b: h.b.clone(),
            from: cpoint(
                &[h.axis[0].clone(), h.axis[1].clone()],
                &format!("{ctx}.from"),
            )?,
            to: cpoint(
                &[h.axis[2].clone(), h.axis[3].clone()],
                &format!("{ctx}.to"),
            )?,
            closed_angle: c(&h.closed_angle, format!("{ctx}.closed_angle"))?,
        });
    }

    let mut paths = Vec::new();
    for (i, l) in file.lines.iter().enumerate() {
        let ctx = format!("line #{}", i + 1);
        paths.push(CPath {
            line_type: line_type(&l.line_type, &ctx)?,
            points: vec![
                cpoint(&l.from, &format!("{ctx}.from"))?,
                cpoint(&l.to, &format!("{ctx}.to"))?,
            ],
            closed: false,
            when: l
                .when
                .as_ref()
                .map(|w| c(w, format!("{ctx}.when")))
                .transpose()?,
        });
    }
    for (i, p) in file.paths.iter().enumerate() {
        let ctx = format!("path #{}", i + 1);
        if p.points.len() < 2 {
            return Err(ComponentError::invalid(ctx, "needs at least two points"));
        }
        let points = p
            .points
            .iter()
            .enumerate()
            .map(|(j, pt)| cpoint(pt, &format!("{ctx}.points[{j}]")))
            .collect::<Result<Vec<_>, _>>()?;
        paths.push(CPath {
            line_type: line_type(&p.line_type, &ctx)?,
            points,
            closed: p.closed,
            when: p
                .when
                .as_ref()
                .map(|w| c(w, format!("{ctx}.when")))
                .transpose()?,
        });
    }

    Ok(Component {
        meta,
        params: file.params,
        derived,
        rules,
        mirror_x,
        panels,
        hinges,
        paths,
    })
}

fn line_type(s: &str, ctx: &str) -> Result<pf_model::LineType, ComponentError> {
    pf_model::LineType::parse(s)
        .ok_or_else(|| ComponentError::invalid(ctx, format!("unknown line type '{s}'")))
}

fn cexpr(
    src: &format::ExprSrc,
    context: String,
    names: &BTreeSet<String>,
) -> Result<CExpr, ComponentError> {
    let expr = match src {
        format::ExprSrc::Num(n) => pf_expr::parse(&format!("{n}")),
        format::ExprSrc::Text(t) => pf_expr::parse(t),
    }
    .map_err(|e| ComponentError::invalid(&context, e.to_string()))?;
    if let Some(unknown) = expr.variables().into_iter().find(|v| !names.contains(v)) {
        return Err(ComponentError::invalid(
            context,
            format!("unknown name '{unknown}'"),
        ));
    }
    Ok(CExpr { expr, context })
}

/// Orders derived values so each only depends on earlier ones. Deterministic (name order).
fn topo_order(
    mut parsed: BTreeMap<String, CExpr>,
    derived: &BTreeSet<String>,
) -> Result<Vec<(String, CExpr)>, ComponentError> {
    let mut done: BTreeSet<String> = BTreeSet::new();
    let mut out = Vec::new();
    while !parsed.is_empty() {
        let ready: Vec<String> = parsed
            .iter()
            .filter(|(_, e)| {
                e.expr
                    .variables()
                    .iter()
                    .all(|v| !derived.contains(v) || done.contains(v))
            })
            .map(|(k, _)| k.clone())
            .collect();
        if ready.is_empty() {
            let cycle: Vec<_> = parsed.keys().cloned().collect();
            return Err(ComponentError::invalid(
                "derived",
                format!("circular dependency between: {}", cycle.join(", ")),
            ));
        }
        for k in ready {
            if let Some(e) = parsed.remove(&k) {
                done.insert(k.clone());
                out.push((k, e));
            }
        }
    }
    Ok(out)
}

fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests;
