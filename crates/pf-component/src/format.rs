//! Serde representation of `*.pfc.toml` files.

use pf_model::{Severity, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// An expression in a component file: either a plain number or formula text.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub(crate) enum ExprSrc {
    Num(f64),
    Text(String),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct File {
    pub component: Meta,
    #[serde(default, rename = "param")]
    pub params: Vec<ParamDef>,
    #[serde(default)]
    pub derived: BTreeMap<String, ExprSrc>,
    #[serde(default, rename = "rule")]
    pub rules: Vec<RuleDef>,
    #[serde(default)]
    pub layout: Option<LayoutDef>,
    #[serde(default, rename = "panel")]
    pub panels: Vec<PanelDef>,
    #[serde(default, rename = "hinge")]
    pub hinges: Vec<HingeDef>,
    #[serde(default, rename = "line")]
    pub lines: Vec<LineDef>,
    #[serde(default, rename = "path")]
    pub paths: Vec<PathDef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    #[default]
    Free,
    Premium,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Meta {
    pub id: String,
    pub version: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standard: Option<String>,
    pub category: String,
    #[serde(default)]
    pub tier: Tier,
    /// Whether dimensions are inside or outside measurements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimension_reference: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Board preset suggested when a user picks this component (e.g. folding boxboard).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_board: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParamType {
    Length,
    Number,
    Angle,
    Bool,
    Enum,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParamDef {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub kind: ParamType,
    pub default: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}

impl ParamDef {
    /// Checks a value against the declared type, range and choices.
    pub fn check_value(&self, v: &Value) -> Result<(), String> {
        match (self.kind, v) {
            (ParamType::Length | ParamType::Number | ParamType::Angle, Value::Number(n)) => {
                if !n.is_finite() {
                    return Err("must be a finite number".into());
                }
                if let Some(min) = self.min {
                    if *n < min {
                        return Err(format!("must be at least {min}"));
                    }
                }
                if let Some(max) = self.max {
                    if *n > max {
                        return Err(format!("must be at most {max}"));
                    }
                }
                Ok(())
            }
            (ParamType::Bool, Value::Bool(_)) => Ok(()),
            (ParamType::Enum, Value::Text(s)) => {
                if self.values.iter().any(|x| x == s) {
                    Ok(())
                } else {
                    Err(format!("must be one of: {}", self.values.join(", ")))
                }
            }
            (kind, _) => Err(format!("expected a {kind:?} value").to_lowercase()),
        }
    }
}

/// Public description of a component, used to build parameter forms.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComponentInfo {
    #[serde(flatten)]
    pub meta: Meta,
    pub params: Vec<ParamDef>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RuleDef {
    pub when: ExprSrc,
    #[serde(default = "default_level")]
    pub level: Severity,
    pub message: String,
}

fn default_level() -> Severity {
    Severity::Warning
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LayoutDef {
    /// When true, the finished die-line is mirrored left↔right.
    #[serde(default)]
    pub mirror_x: Option<ExprSrc>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PanelDef {
    pub id: String,
    pub x: ExprSrc,
    pub y: ExprSrc,
    pub w: ExprSrc,
    pub h: ExprSrc,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HingeDef {
    pub a: String,
    pub b: String,
    /// `[x1, y1, x2, y2]`
    pub axis: [ExprSrc; 4],
    pub closed_angle: ExprSrc,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LineDef {
    #[serde(rename = "type")]
    pub line_type: String,
    pub from: [ExprSrc; 2],
    pub to: [ExprSrc; 2],
    #[serde(default)]
    pub when: Option<ExprSrc>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PathDef {
    #[serde(rename = "type")]
    pub line_type: String,
    pub points: Vec<PathItem>,
    #[serde(default)]
    pub closed: bool,
    #[serde(default)]
    pub when: Option<ExprSrc>,
}

/// One entry of a path's `points` list.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum PathItem {
    /// `["x", "y"]` — a sharp vertex.
    Point([ExprSrc; 2]),
    /// `{ at = ["x", "y"], round = "r" }` — a vertex rounded with a tangent arc of radius r.
    Rounded(RoundedDef),
    /// `{ to = ["x", "y"], radius = "r", ccw = true, large = false }` — circular arc from the
    /// previous point to `to` (SVG-style: direction and large/small arc choose the centre).
    Arc(ArcDef),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RoundedDef {
    pub at: [ExprSrc; 2],
    pub round: ExprSrc,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArcDef {
    pub to: [ExprSrc; 2],
    pub radius: ExprSrc,
    #[serde(default)]
    pub ccw: bool,
    #[serde(default)]
    pub large: bool,
}
