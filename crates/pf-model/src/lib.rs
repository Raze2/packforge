//! Domain model shared by every PackForge crate and shell.

use pf_geom::{BBox, Path, Point};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Manufacturing meaning of a die-line entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LineType {
    Cut,
    Crease,
    ReverseCrease,
    Perforation,
    HalfCut,
    Bleed,
    Dimension,
    Annotation,
}

impl LineType {
    pub const ALL: [LineType; 8] = [
        LineType::Cut,
        LineType::Crease,
        LineType::ReverseCrease,
        LineType::Perforation,
        LineType::HalfCut,
        LineType::Bleed,
        LineType::Dimension,
        LineType::Annotation,
    ];

    /// Stable identifier, also used as the default DXF layer name (upper-cased).
    pub fn id(self) -> &'static str {
        match self {
            LineType::Cut => "cut",
            LineType::Crease => "crease",
            LineType::ReverseCrease => "reverse-crease",
            LineType::Perforation => "perforation",
            LineType::HalfCut => "half-cut",
            LineType::Bleed => "bleed",
            LineType::Dimension => "dimension",
            LineType::Annotation => "annotation",
        }
    }

    pub fn parse(s: &str) -> Option<LineType> {
        LineType::ALL.into_iter().find(|t| t.id() == s)
    }

    /// Whether the entity is part of the physical tooling (vs. drawing aids).
    pub fn is_tooling(self) -> bool {
        !matches!(
            self,
            LineType::Bleed | LineType::Dimension | LineType::Annotation
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entity {
    pub line_type: LineType,
    pub path: Path,
}

/// The flat drawing of a blank.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DieLine {
    pub entities: Vec<Entity>,
}

impl DieLine {
    pub fn push(&mut self, line_type: LineType, path: Path) {
        self.entities.push(Entity { line_type, path });
    }

    /// Bounding box of tooling entities (the blank).
    pub fn bbox(&self) -> BBox {
        let mut b = BBox::empty();
        for e in self.entities.iter().filter(|e| e.line_type.is_tooling()) {
            b.union(&e.path.bbox());
        }
        b
    }

    /// Bounding box of every entity, including drawing aids (sheet outlines, notes).
    pub fn bbox_all(&self) -> BBox {
        let mut b = BBox::empty();
        for e in &self.entities {
            b.union(&e.path.bbox());
        }
        b
    }

    pub fn length_of(&self, t: LineType) -> f64 {
        self.entities
            .iter()
            .filter(|e| e.line_type == t)
            .map(|e| e.path.length())
            .sum()
    }

    /// Area enclosed by closed cut outlines (mm²), using the outer loop with the largest area.
    pub fn outline_area(&self) -> f64 {
        self.entities
            .iter()
            .filter(|e| e.line_type == LineType::Cut && e.path.closed)
            .map(|e| e.path.signed_area().abs())
            .fold(0.0, f64::max)
    }

    pub fn stats(&self) -> Stats {
        let b = self.bbox();
        Stats {
            blank_width: b.width(),
            blank_height: b.height(),
            blank_area_m2: b.width() * b.height() / 1e6,
            outline_area_m2: self.outline_area() / 1e6,
            cut_length: self.length_of(LineType::Cut),
            crease_length: self.length_of(LineType::Crease)
                + self.length_of(LineType::ReverseCrease),
        }
    }
}

/// Derived numbers shown to users and used for estimating.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    /// Width of the rectangular blank (mm).
    pub blank_width: f64,
    /// Height of the rectangular blank (mm).
    pub blank_height: f64,
    /// Rectangular blank area (m²) — board consumed before waste.
    pub blank_area_m2: f64,
    /// Area inside the cut outline (m²).
    pub outline_area_m2: f64,
    /// Total cut length (mm) — drives knife/rule cost.
    pub cut_length: f64,
    /// Total crease length (mm).
    pub crease_length: f64,
}

/// A rigid rectangular panel used for folding and area calculations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Panel {
    pub id: String,
    pub origin: Point,
    pub width: f64,
    pub height: f64,
}

/// Fold line between two panels. `closed_angle` is the fold angle (degrees) in the erected box.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hinge {
    pub a: String,
    pub b: String,
    pub from: Point,
    pub to: Point,
    pub closed_angle: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FoldModel {
    pub panels: Vec<Panel>,
    pub hinges: Vec<Hinge>,
    pub thickness: f64,
}

/// Board / material.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Board {
    pub id: String,
    #[serde(default)]
    pub name: String,
    /// Thickness in millimetres.
    pub caliper_mm: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flute: Option<String>,
}

/// A parameter value supplied by a user or API client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Bool(bool),
    Number(f64),
    Text(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Units {
    #[default]
    Mm,
    Cm,
    In,
}

impl Units {
    /// Millimetres per unit.
    pub fn mm_per_unit(self) -> f64 {
        match self {
            Units::Mm => 1.0,
            Units::Cm => 10.0,
            Units::In => 25.4,
        }
    }
}

/// Board selection in a project: a preset id, or an explicit board.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BoardRef {
    Preset(String),
    Custom(Board),
}

/// A saved design. Lengths in `params` are always millimetres; `units` is display preference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub component_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component_version: Option<String>,
    #[serde(default)]
    pub params: BTreeMap<String, Value>,
    pub board: BoardRef,
    #[serde(default)]
    pub units: Units,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
}

/// Everything a shell needs to display a design.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderResult {
    pub component_id: String,
    pub component_version: String,
    pub board: Board,
    pub dieline: DieLine,
    pub fold: FoldModel,
    pub stats: Stats,
    pub diagnostics: Vec<Diagnostic>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_type_ids_round_trip() {
        for t in LineType::ALL {
            assert_eq!(LineType::parse(t.id()), Some(t));
        }
    }

    #[test]
    fn project_json_accepts_preset_or_custom_board() {
        let a: Project = serde_json::from_str(
            r#"{"component_id":"fefco-0201","params":{"L":300},"board":"C-flute"}"#,
        )
        .expect("preset board");
        assert_eq!(a.board, BoardRef::Preset("C-flute".into()));
        let b: Project =
            serde_json::from_str(r#"{"component_id":"x","board":{"id":"mine","caliper_mm":2.5}}"#)
                .expect("custom board");
        assert!(matches!(b.board, BoardRef::Custom(Board { caliper_mm, .. }) if caliper_mm == 2.5));
    }

    #[test]
    fn stats_from_rectangle() {
        let mut d = DieLine::default();
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(100.0, 50.0),
            Point::new(0.0, 50.0),
        ];
        d.push(LineType::Cut, Path::polyline(&pts, true));
        d.push(
            LineType::Crease,
            Path::line(Point::new(50.0, 0.0), Point::new(50.0, 50.0)),
        );
        let s = d.stats();
        assert_eq!(s.blank_width, 100.0);
        assert!((s.blank_area_m2 - 0.005).abs() < 1e-12);
        assert_eq!(s.cut_length, 300.0);
        assert_eq!(s.crease_length, 50.0);
    }
}
