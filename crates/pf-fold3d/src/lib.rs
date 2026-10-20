//! Fold kinematics: turns a flat [`FoldModel`] (rectangular panels joined by hinges) into 3D
//! panel positions for any fold progress, and exports the result as binary glTF (GLB).
//!
//! Conventions: the die-line lies in the z = 0 plane, viewed from the print (outside) side,
//! which faces +z. Every hinge folds its child panel *away* from the viewer (towards -z), so the
//! printed side ends up outside. The root is the largest panel; it stays in place.

mod glb;

pub use glb::to_glb;

use pf_geom::Point;
use pf_model::FoldModel;
use serde::Serialize;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
    fn scale(self, s: f64) -> Vec3 {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
    pub fn dot(self, o: Vec3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn cross(self, o: Vec3) -> Vec3 {
        Vec3::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }
    pub fn length(self) -> f64 {
        self.dot(self).sqrt()
    }
    pub fn distance(self, o: Vec3) -> f64 {
        self.sub(o).length()
    }
}

/// Rigid transform: rotation matrix (row-major) plus translation.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Rigid {
    r: [[f64; 3]; 3],
    t: Vec3,
}

impl Rigid {
    const IDENTITY: Rigid = Rigid {
        r: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        t: Vec3::new(0.0, 0.0, 0.0),
    };

    fn rotate(&self, v: Vec3) -> Vec3 {
        let r = &self.r;
        Vec3::new(
            r[0][0] * v.x + r[0][1] * v.y + r[0][2] * v.z,
            r[1][0] * v.x + r[1][1] * v.y + r[1][2] * v.z,
            r[2][0] * v.x + r[2][1] * v.y + r[2][2] * v.z,
        )
    }

    fn apply(&self, p: Vec3) -> Vec3 {
        self.rotate(p).add(self.t)
    }

    /// `self ∘ other` (apply `other` first).
    fn then(&self, other: &Rigid) -> Rigid {
        let mut r = [[0.0; 3]; 3];
        for (i, row) in r.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = (0..3).map(|k| self.r[i][k] * other.r[k][j]).sum();
            }
        }
        Rigid {
            r,
            t: self.apply(other.t),
        }
    }

    /// Rotation by `angle` (radians) about the line through `a` with unit direction `d`.
    fn about_axis(a: Vec3, d: Vec3, angle: f64) -> Rigid {
        let (c, s) = (angle.cos(), angle.sin());
        let k = 1.0 - c;
        let r = [
            [
                c + d.x * d.x * k,
                d.x * d.y * k - d.z * s,
                d.x * d.z * k + d.y * s,
            ],
            [
                d.y * d.x * k + d.z * s,
                c + d.y * d.y * k,
                d.y * d.z * k - d.x * s,
            ],
            [
                d.z * d.x * k - d.y * s,
                d.z * d.y * k + d.x * s,
                c + d.z * d.z * k,
            ],
        ];
        let rot = Rigid {
            r,
            t: Vec3::new(0.0, 0.0, 0.0),
        };
        // p' = a + R (p - a)  =>  t = a - R a
        Rigid {
            r,
            t: a.sub(rot.rotate(a)),
        }
    }
}

/// One panel in 3D: corners in the panel's flat order (counter-clockwise from the print side).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Panel3 {
    pub id: String,
    pub corners: [Vec3; 4],
    /// Outward normal (from the print side).
    pub normal: Vec3,
    /// Distance from the root in the hinge tree (useful for draw ordering).
    pub depth: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Folded {
    /// Fold progress used, 0 = flat, 1 = erected.
    pub progress: f64,
    pub root: String,
    pub panels: Vec<Panel3>,
    pub min: Vec3,
    pub max: Vec3,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum FoldError {
    #[error("the design has no panels to fold")]
    NoPanels,
    #[error("hinge {0}–{1} references an unknown panel")]
    UnknownPanel(String, String),
    #[error("hinge {0}–{1} has a zero-length axis")]
    DegenerateHinge(String, String),
}

fn v(p: Point) -> Vec3 {
    Vec3::new(p.x, p.y, 0.0)
}

/// Folds the model to `progress` (clamped to 0..=1). Panels not connected to the root stay flat.
pub fn fold(model: &FoldModel, progress: f64) -> Result<Folded, FoldError> {
    let t = progress.clamp(0.0, 1.0);
    let root = model
        .panels
        .iter()
        .max_by(|a, b| {
            (a.width * a.height)
                .partial_cmp(&(b.width * b.height))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .ok_or(FoldError::NoPanels)?;

    let index: HashMap<&str, usize> = model
        .panels
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id.as_str(), i))
        .collect();
    for h in &model.hinges {
        if !index.contains_key(h.a.as_str()) || !index.contains_key(h.b.as_str()) {
            return Err(FoldError::UnknownPanel(h.a.clone(), h.b.clone()));
        }
        if h.from.distance(h.to) < 1e-9 {
            return Err(FoldError::DegenerateHinge(h.a.clone(), h.b.clone()));
        }
    }

    let n = model.panels.len();
    let mut world: Vec<Option<Rigid>> = vec![None; n];
    let mut depth = vec![0usize; n];
    let root_i = index[root.id.as_str()];
    world[root_i] = Some(Rigid::IDENTITY);
    let mut queue = VecDeque::from([root_i]);

    while let Some(parent) = queue.pop_front() {
        let parent_id = model.panels[parent].id.as_str();
        let parent_world = world[parent].unwrap_or(Rigid::IDENTITY);
        for h in &model.hinges {
            let child_id = if h.a == parent_id {
                h.b.as_str()
            } else if h.b == parent_id {
                h.a.as_str()
            } else {
                continue;
            };
            let child = index[child_id];
            if world[child].is_some() {
                continue;
            }
            let c = &model.panels[child];
            let centre = Vec3::new(c.origin.x + c.width / 2.0, c.origin.y + c.height / 2.0, 0.0);
            let (a, b) = (v(h.from), v(h.to));
            let axis = b.sub(a);
            let d = axis.scale(1.0 / axis.length());
            // Fold towards -z: choose the rotation sign so the child's centre moves to z < 0.
            let side = d.cross(centre.sub(a)).z;
            let angle = -side.signum() * h.closed_angle.to_radians() * t;
            let local = Rigid::about_axis(a, d, angle);
            world[child] = Some(parent_world.then(&local));
            depth[child] = depth[parent] + 1;
            queue.push_back(child);
        }
    }

    let mut panels = Vec::with_capacity(n);
    let mut min = Vec3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let mut max = Vec3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for (i, p) in model.panels.iter().enumerate() {
        let w = world[i].unwrap_or(Rigid::IDENTITY);
        let (x0, y0, x1, y1) = (
            p.origin.x,
            p.origin.y,
            p.origin.x + p.width,
            p.origin.y + p.height,
        );
        let corners = [
            Vec3::new(x0, y0, 0.0),
            Vec3::new(x1, y0, 0.0),
            Vec3::new(x1, y1, 0.0),
            Vec3::new(x0, y1, 0.0),
        ]
        .map(|c| w.apply(c));
        for c in &corners {
            min = Vec3::new(min.x.min(c.x), min.y.min(c.y), min.z.min(c.z));
            max = Vec3::new(max.x.max(c.x), max.y.max(c.y), max.z.max(c.z));
        }
        panels.push(Panel3 {
            id: p.id.clone(),
            corners,
            normal: w.rotate(Vec3::new(0.0, 0.0, 1.0)),
            depth: depth[i],
        });
    }
    Ok(Folded {
        progress: t,
        root: root.id.clone(),
        panels,
        min,
        max,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pf_model::{Hinge, Panel};

    fn panel(id: &str, x: f64, y: f64, w: f64, h: f64) -> Panel {
        Panel {
            id: id.into(),
            origin: Point::new(x, y),
            width: w,
            height: h,
        }
    }

    fn hinge(a: &str, b: &str, from: (f64, f64), to: (f64, f64), angle: f64) -> Hinge {
        Hinge {
            a: a.into(),
            b: b.into(),
            from: Point::new(from.0, from.1),
            to: Point::new(to.0, to.1),
            closed_angle: angle,
        }
    }

    /// A tray: 100×60 base with four 20 mm walls.
    fn tray() -> FoldModel {
        FoldModel {
            panels: vec![
                panel("base", 20.0, 20.0, 100.0, 60.0),
                panel("back", 20.0, 80.0, 100.0, 20.0),
                panel("front", 20.0, 0.0, 100.0, 20.0),
                panel("left", 0.0, 20.0, 20.0, 60.0),
                panel("right", 120.0, 20.0, 20.0, 60.0),
            ],
            hinges: vec![
                hinge("base", "back", (20.0, 80.0), (120.0, 80.0), 90.0),
                hinge("base", "front", (20.0, 20.0), (120.0, 20.0), 90.0),
                hinge("base", "left", (20.0, 20.0), (20.0, 80.0), 90.0),
                hinge("base", "right", (120.0, 20.0), (120.0, 80.0), 90.0),
            ],
            thickness: 1.0,
        }
    }

    #[test]
    fn flat_at_zero_and_erected_at_one() {
        let flat = fold(&tray(), 0.0).expect("fold");
        assert!(flat
            .panels
            .iter()
            .all(|p| p.corners.iter().all(|c| c.z.abs() < 1e-9)));
        let up = fold(&tray(), 1.0).expect("fold");
        assert_eq!(up.root, "base");
        // Walls stand at right angles, all on the -z side, 20 mm high.
        assert!((up.min.z + 20.0).abs() < 1e-9 && up.max.z.abs() < 1e-9);
        assert!((up.max.x - up.min.x - 100.0).abs() < 1e-9);
        assert!((up.max.y - up.min.y - 60.0).abs() < 1e-9);
        for p in up.panels.iter().filter(|p| p.id != "base") {
            assert!(p.normal.z.abs() < 1e-9, "{} is not vertical", p.id);
        }
    }

    #[test]
    fn corners_meet_when_erected() {
        let up = fold(&tray(), 1.0).expect("fold");
        let get = |id: &str| up.panels.iter().find(|p| p.id == id).expect("panel");
        // The back wall's left edge and the left wall's top edge coincide in the corner.
        let back = get("back");
        let left = get("left");
        let back_corner = back.corners[3]; // (x0, y1) of the back panel, folded
        assert!(left
            .corners
            .iter()
            .any(|c| c.distance(back_corner) < 1e-9 || c.distance(back.corners[0]) < 1e-9));
    }

    #[test]
    fn panels_stay_rigid() {
        let half = fold(&tray(), 0.37).expect("fold");
        for p in &half.panels {
            let [a, b, c, d] = p.corners;
            assert!((a.distance(b) - c.distance(d)).abs() < 1e-9);
            assert!((b.distance(c) - d.distance(a)).abs() < 1e-9);
            assert!(((a.distance(c)) - (b.distance(d))).abs() < 1e-9); // still a rectangle
        }
    }

    #[test]
    fn bad_models_are_reported() {
        assert_eq!(
            fold(&FoldModel::default(), 1.0).unwrap_err(),
            FoldError::NoPanels
        );
        let mut m = tray();
        m.hinges
            .push(hinge("base", "ghost", (0.0, 0.0), (1.0, 0.0), 90.0));
        assert!(matches!(fold(&m, 1.0), Err(FoldError::UnknownPanel(..))));
    }
}
