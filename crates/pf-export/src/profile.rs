//! Output profiles: how each line type maps to tools/pens/layers on a machine.
//!
//! Profiles are data (TOML) so shops can match their cutting table or CAM import without code.

use pf_model::LineType;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Settings for one line type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineMap {
    /// Whether this line type is written at all.
    #[serde(default = "yes")]
    pub include: bool,
    /// DXF layer name.
    pub layer: String,
    /// CF2 line type code (1 cut, 2 crease, … — check your CAM software's convention).
    pub cf2_type: u8,
    /// CF2 rule thickness ("pointage") in points.
    #[serde(default = "two")]
    pub cf2_pointage: f64,
    /// HPGL pen / tool number.
    pub pen: u8,
}

fn yes() -> bool {
    true
}

fn two() -> f64 {
    2.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub lines: BTreeMap<LineType, LineMap>,
}

impl Profile {
    pub fn from_toml(src: &str) -> Result<Profile, String> {
        toml::from_str(src).map_err(|e| e.to_string())
    }

    /// Mapping for a line type; unknown types are skipped.
    pub fn line(&self, t: LineType) -> Option<&LineMap> {
        self.lines.get(&t).filter(|m| m.include)
    }

    /// The default profile: tooling lines included, drawing aids only in DXF/SVG/PDF.
    pub fn standard() -> Profile {
        let m = |layer: &str, cf2_type: u8, pen: u8, include: bool| LineMap {
            include,
            layer: layer.into(),
            cf2_type,
            cf2_pointage: 2.0,
            pen,
        };
        Profile {
            id: "standard".into(),
            name: "Standard".into(),
            lines: BTreeMap::from([
                (LineType::Cut, m("CUT", 1, 1, true)),
                (LineType::Crease, m("CREASE", 2, 2, true)),
                (LineType::ReverseCrease, m("REVERSE_CREASE", 2, 3, true)),
                (LineType::Perforation, m("PERFORATION", 3, 4, true)),
                (LineType::HalfCut, m("HALF_CUT", 4, 5, true)),
                (LineType::Bleed, m("BLEED", 5, 6, false)),
                (LineType::Dimension, m("DIMENSION", 6, 7, false)),
                (LineType::Annotation, m("ANNOTATION", 7, 8, false)),
            ]),
        }
    }

    /// Built-in profiles selectable by id.
    pub fn builtin(id: &str) -> Option<Profile> {
        match id {
            "standard" => Some(Self::standard()),
            // Plotters/sample tables that only cut and crease (no perforation tool).
            "cut-crease" => {
                let mut p = Self::standard();
                p.id = "cut-crease".into();
                p.name = "Cut and crease only".into();
                for (t, m) in p.lines.iter_mut() {
                    m.include = matches!(
                        t,
                        LineType::Cut | LineType::Crease | LineType::ReverseCrease
                    );
                }
                Some(p)
            }
            _ => None,
        }
    }
}

/// RGB used in drawings (SVG/PDF), matching the designer's colours.
pub fn line_rgb(t: LineType) -> (f64, f64, f64) {
    match t {
        LineType::Cut => (0.11, 0.11, 0.12),
        LineType::Crease => (0.82, 0.01, 0.11),
        LineType::ReverseCrease => (0.61, 0.11, 0.82),
        LineType::Perforation => (0.10, 0.56, 0.24),
        LineType::HalfCut => (0.12, 0.37, 0.82),
        LineType::Bleed => (0.0, 0.64, 0.77),
        LineType::Dimension => (0.54, 0.54, 0.56),
        LineType::Annotation => (0.69, 0.49, 0.0),
    }
}

/// Dash pattern in millimetres (empty = solid).
pub fn line_dash_mm(t: LineType) -> &'static [f64] {
    match t {
        LineType::Crease => &[6.0, 3.0],
        LineType::ReverseCrease => &[6.0, 2.0, 1.0, 2.0],
        LineType::Perforation => &[2.0, 2.0],
        LineType::HalfCut => &[8.0, 2.0],
        LineType::Bleed => &[1.0, 2.0],
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_parse_from_toml() {
        let src = r#"
id = "shop"
name = "Shop table"
[lines.cut]
layer = "KNIFE"
cf2_type = 1
pen = 3
[lines.crease]
layer = "CREASE"
cf2_type = 2
pen = 4
include = false
"#;
        let p = Profile::from_toml(src).expect("parse");
        assert_eq!(p.line(LineType::Cut).map(|m| m.pen), Some(3));
        assert!(p.line(LineType::Crease).is_none()); // excluded
        assert!(p.line(LineType::Perforation).is_none()); // not mapped
    }

    #[test]
    fn builtin_profiles() {
        assert!(Profile::builtin("standard").is_some());
        let cc = Profile::builtin("cut-crease").expect("cut-crease");
        assert!(cc.line(LineType::Perforation).is_none());
        assert!(cc.line(LineType::Crease).is_some());
        assert!(Profile::builtin("nope").is_none());
    }
}
