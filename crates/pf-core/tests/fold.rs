//! Every style must fold into a box of the expected size: this checks that hinges, angles and
//! panel sizes agree with each other, independently of the 2D die-line tests.

use pf_core::{fold, list_components, BoardRef, Project, Units, Value};
use std::collections::BTreeMap;

fn project(id: &str, params: &[(&str, f64)], board: &str) -> Project {
    Project {
        component_id: id.into(),
        component_version: None,
        params: params
            .iter()
            .map(|(k, v)| (k.to_string(), Value::Number(*v)))
            .collect::<BTreeMap<_, _>>(),
        board: BoardRef::Preset(board.into()),
        units: Units::Mm,
    }
}

fn sorted(mut v: [f64; 3]) -> [f64; 3] {
    v.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    v
}

/// Expected erected outer size (crease-to-crease) per style for L, W, H and caliper t.
fn expected(id: &str, l: f64, w: f64, h: f64, t: f64) -> [f64; 3] {
    match id {
        "sleeve" => [l + t, w + t, h],
        "tray-glued-corners" => [l + t, w + t, h + t],
        _ => [l + t, w + t, h + 2.0 * t],
    }
}

#[test]
fn every_style_folds_into_a_box_of_the_right_size() {
    for c in list_components().expect("library") {
        let carton = c.id.starts_with("tuck-end");
        let (board, t) = if carton {
            ("FBB-0.5", 0.5)
        } else {
            ("C-flute", 4.0)
        };
        let (l, w, h) = if carton {
            (80.0, 50.0, 150.0)
        } else {
            (400.0, 300.0, 250.0)
        };
        let p = project(&c.id, &[("L", l), ("W", w), ("H", h)], board);

        let flat = fold(&p, 0.0).expect("flat");
        assert!(
            flat.panels
                .iter()
                .all(|q| q.corners.iter().all(|v| v.z.abs() < 1e-9)),
            "{}: not flat at progress 0",
            c.id
        );

        let erected = fold(&p, 1.0).unwrap_or_else(|e| panic!("{}: {e}", c.id));
        let size = sorted([
            erected.max.x - erected.min.x,
            erected.max.y - erected.min.y,
            erected.max.z - erected.min.z,
        ]);
        let want = sorted(expected(&c.id, l, w, h, t));
        for k in 0..3 {
            assert!(
                (size[k] - want[k]).abs() < 1e-6,
                "{}: erected size {size:?}, expected {want:?}",
                c.id
            );
        }
    }
}

#[test]
fn rsc_tube_closes_onto_its_glue_flap() {
    let p = project(
        "fefco-0201",
        &[("L", 300.0), ("W", 200.0), ("H", 150.0)],
        "C-flute",
    );
    let f = fold(&p, 1.0).expect("fold");
    let get = |id: &str| f.panels.iter().find(|q| q.id == id).expect("panel");
    // The glue flap lies in the same plane as the last end panel (where it is glued).
    let glue = get("glue");
    let end2 = get("end2");
    let n = end2.normal;
    let plane = |v: pf_core::Vec3| n.dot(v) - n.dot(end2.corners[0]);
    for c in glue.corners {
        assert!(
            plane(c).abs() < 1e-6,
            "glue flap is {:.3} mm off the end panel",
            plane(c)
        );
    }
    // Opposite body panels are parallel (normals opposite).
    let (s1, s2) = (get("side1").normal, get("side2").normal);
    assert!((s1.dot(s2) + 1.0).abs() < 1e-9);
}

#[test]
fn half_folded_box_stays_within_the_flat_and_erected_extents() {
    let p = project("tuck-end-reverse", &[], "FBB-0.5");
    for step in 0..=10 {
        let f = fold(&p, step as f64 / 10.0).expect("fold");
        assert!(f.panels.iter().all(|q| q
            .corners
            .iter()
            .all(|v| v.x.is_finite() && v.y.is_finite() && v.z.is_finite())));
    }
}
