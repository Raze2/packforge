//! Checks that apply to every built-in component, plus per-style dimension formulas.

use pf_core::{
    describe, list_components, render, BoardRef, CoreError, LineType, ParamType, Project,
    RenderError, RenderResult, Severity, Units, Value,
};
use std::collections::BTreeMap;

fn is_carton(id: &str) -> bool {
    id.starts_with("tuck-end")
}

fn board_for(id: &str) -> &'static str {
    if is_carton(id) {
        "FBB-0.5"
    } else {
        "C-flute"
    }
}

fn project(id: &str, params: BTreeMap<String, Value>, board: &str) -> Project {
    Project {
        component_id: id.into(),
        component_version: None,
        params,
        board: BoardRef::Preset(board.into()),
        units: Units::Mm,
    }
}

fn ids() -> Vec<String> {
    list_components()
        .expect("library compiles")
        .into_iter()
        .map(|c| c.id)
        .collect()
}

/// Distance from point p to segment ab.
fn dist_to_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    };
    ((p.0 - a.0 - t * dx).powi(2) + (p.1 - a.1 - t * dy).powi(2)).sqrt()
}

/// Structural checks every design must pass (beyond the engine's own outline checks).
fn check_structure(id: &str, r: &RenderResult) {
    let blank = r.dieline.bbox();
    let tol = 1e-6;
    // Every panel lies inside the blank.
    for p in &r.fold.panels {
        assert!(
            p.origin.x >= blank.min.x - tol
                && p.origin.y >= blank.min.y - tol
                && p.origin.x + p.width <= blank.max.x + tol
                && p.origin.y + p.height <= blank.max.y + tol,
            "{id}: panel {} outside the blank",
            p.id
        );
    }
    // Every hinge axis is covered by a crease line, so the design folds where it is scored.
    let creases: Vec<((f64, f64), (f64, f64))> = r
        .dieline
        .entities
        .iter()
        .filter(|e| e.line_type == LineType::Crease)
        .flat_map(|e| {
            e.path
                .segments
                .iter()
                .map(|s| {
                    let (a, b) = (s.start_point(), s.end_point());
                    ((a.x, a.y), (b.x, b.y))
                })
                .collect::<Vec<_>>()
        })
        .collect();
    for h in &r.fold.hinges {
        for p in [
            (h.from.x, h.from.y),
            (h.to.x, h.to.y),
            ((h.from.x + h.to.x) / 2.0, (h.from.y + h.to.y) / 2.0),
        ] {
            assert!(
                creases
                    .iter()
                    .any(|(a, b)| dist_to_segment(p, *a, *b) < 1e-6),
                "{id}: hinge {}–{} is not on a crease line",
                h.a,
                h.b
            );
        }
        let ids: Vec<_> = r.fold.panels.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&h.a.as_str()) && ids.contains(&h.b.as_str()));
    }
    // The panel graph is connected (one piece that folds into a box).
    let mut seen = vec![r.fold.panels[0].id.clone()];
    loop {
        let before = seen.len();
        for h in &r.fold.hinges {
            if seen.contains(&h.a) && !seen.contains(&h.b) {
                seen.push(h.b.clone());
            } else if seen.contains(&h.b) && !seen.contains(&h.a) {
                seen.push(h.a.clone());
            }
        }
        if seen.len() == before {
            break;
        }
    }
    assert_eq!(
        seen.len(),
        r.fold.panels.len(),
        "{id}: panels are not all connected by hinges"
    );
    // The outline fits in its blank. (Cross-shaped blanks such as tall trays can legitimately
    // leave most of the bounding rectangle as waste, so only a loose lower bound is checked.)
    let s = r.stats;
    assert!(
        s.outline_area_m2 > 0.05 * s.blank_area_m2 && s.outline_area_m2 <= s.blank_area_m2 + 1e-9,
        "{id}: outline {} m² vs blank {} m²",
        s.outline_area_m2,
        s.blank_area_m2
    );
    assert!(s.cut_length > 0.0 && s.crease_length > 0.0);
}

#[test]
fn library_has_the_stage_5_styles() {
    let ids = ids();
    for want in [
        "fefco-0200",
        "fefco-0201",
        "fefco-0202",
        "fefco-0203",
        "fefco-0204",
        "sleeve",
        "tray-glued-corners",
        "tuck-end-straight",
        "tuck-end-reverse",
    ] {
        assert!(ids.iter().any(|i| i == want), "missing {want}");
    }
}

#[test]
fn every_style_renders_cleanly_at_defaults() {
    for id in ids() {
        let r = render(&project(&id, BTreeMap::new(), board_for(&id)))
            .unwrap_or_else(|e| panic!("{id}: {e}"));
        let notes: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.severity != Severity::Info || !d.message.is_empty())
            .collect();
        assert!(
            notes.is_empty(),
            "{id}: defaults should not produce notes: {notes:?}"
        );
        check_structure(&id, &r);
    }
}

#[test]
fn every_style_survives_a_parameter_sweep() {
    for id in ids() {
        let info = describe(&id).expect("describe");
        let numeric: Vec<_> = info
            .params
            .iter()
            .filter(|p| matches!(p.kind, ParamType::Length | ParamType::Number))
            .collect();
        // Each numeric parameter at min, a low value, its default and max, one at a time,
        // plus everything at min and everything at max.
        let mut cases: Vec<BTreeMap<String, Value>> = Vec::new();
        for p in &numeric {
            let (lo, hi) = (p.min.unwrap_or(1.0), p.max.unwrap_or(1000.0));
            for v in [lo, lo + (hi - lo) * 0.05, hi] {
                cases.push(BTreeMap::from([(p.id.clone(), Value::Number(v))]));
            }
        }
        for pick in [
            |p: &&pf_core::ParamDef| p.min,
            |p: &&pf_core::ParamDef| p.max,
        ] {
            cases.push(
                numeric
                    .iter()
                    .filter_map(|p| pick(p).map(|v| (p.id.clone(), Value::Number(v))))
                    .collect(),
            );
        }
        for board in [board_for(&id), "E-flute", "BC-double"] {
            for params in &cases {
                match render(&project(&id, params.clone(), board)) {
                    Ok(r) => check_structure(&id, &r),
                    // A declared design rule may reject extreme combinations; that is fine.
                    Err(CoreError::Render(RenderError::Rule(_))) => {}
                    Err(e) => panic!("{id} {params:?} on {board}: {e}"),
                }
            }
        }
    }
}

#[test]
fn glue_side_right_mirrors_every_style_that_has_it() {
    for id in ids() {
        let info = describe(&id).expect("describe");
        if !info.params.iter().any(|p| p.id == "glue_side") {
            continue;
        }
        let left = render(&project(&id, BTreeMap::new(), board_for(&id))).expect("left");
        let right = render(&project(
            &id,
            BTreeMap::from([("glue_side".into(), Value::Text("right".into()))]),
            board_for(&id),
        ))
        .expect("right");
        let (a, b) = (left.stats, right.stats);
        for (x, y) in [
            (a.blank_width, b.blank_width),
            (a.blank_height, b.blank_height),
            (a.outline_area_m2, b.outline_area_m2),
            (a.cut_length, b.cut_length),
            (a.crease_length, b.crease_length),
        ] {
            assert!(
                (x - y).abs() < 1e-9,
                "{id}: mirroring changed the stats ({x} vs {y})"
            );
        }
        check_structure(&id, &right);
    }
}

fn stats(id: &str, params: &[(&str, f64)], board: &str) -> pf_core::Stats {
    let p = params
        .iter()
        .map(|(k, v)| (k.to_string(), Value::Number(*v)))
        .collect();
    render(&project(id, p, board)).expect("render").stats
}

#[test]
fn slotted_family_flap_rules() {
    let dims = [("L", 400.0), ("W", 300.0), ("H", 250.0)];
    let t = 4.0; // C-flute
    let (pl, pw, ph) = (400.0 + t, 300.0 + t, 250.0 + 2.0 * t);
    let width = 35.0 + 2.0 * pl + 2.0 * pw;
    let close = |a: f64, b: f64| (a - b).abs() < 1e-6;

    let s = stats("fefco-0200", &dims, "C-flute");
    assert!(close(s.blank_width, width) && close(s.blank_height, ph + pw / 2.0)); // bottom flaps only
    let s = stats("fefco-0201", &dims, "C-flute");
    assert!(close(s.blank_height, ph + pw)); // flaps meet in the centre
    let s = stats(
        "fefco-0202",
        &[("L", 400.0), ("W", 300.0), ("H", 250.0), ("overlap", 50.0)],
        "C-flute",
    );
    assert!(close(s.blank_height, ph + pw + 50.0)); // each flap +overlap/2
    let s = stats("fefco-0203", &dims, "C-flute");
    assert!(close(s.blank_height, ph + 2.0 * pw)); // full-width flaps
    let s = stats("fefco-0204", &dims, "C-flute");
    assert!(close(s.blank_height, ph + pl)); // end flaps = L/2 dominate
}

#[test]
fn overlap_larger_than_width_is_rejected() {
    let params = BTreeMap::from([
        ("W".to_string(), Value::Number(100.0)),
        ("overlap".to_string(), Value::Number(200.0)),
    ]);
    assert!(matches!(
        render(&project("fefco-0202", params, "C-flute")),
        Err(CoreError::Render(RenderError::Rule(_)))
    ));
}

#[test]
fn sleeve_tray_and_carton_dimensions() {
    let close = |a: f64, b: f64| (a - b).abs() < 1e-6;
    // Sleeve: four panels plus glue flap; height is the sleeve depth.
    let s = stats(
        "sleeve",
        &[("L", 300.0), ("W", 200.0), ("H", 120.0)],
        "B-flute",
    );
    assert!(close(s.blank_width, 35.0 + 2.0 * 303.0 + 2.0 * 203.0) && close(s.blank_height, 120.0));
    // Tray: base plus a wall on every side.
    let s = stats(
        "tray-glued-corners",
        &[("L", 300.0), ("W", 200.0), ("H", 60.0)],
        "E-flute",
    );
    let (pl, pw, ph) = (301.6, 201.6, 61.6);
    assert!(close(s.blank_width, pl + 2.0 * ph) && close(s.blank_height, pw + 2.0 * ph));
    // Tuck-end cartons: body plus lid + tuck at top and bottom.
    for id in ["tuck-end-straight", "tuck-end-reverse"] {
        let s = stats(
            id,
            &[("L", 80.0), ("W", 50.0), ("H", 150.0), ("tuck", 15.0)],
            "FBB-0.5",
        );
        assert!(close(s.blank_width, 15.0 + 2.0 * 80.5 + 2.0 * 50.5), "{id}");
        assert!(close(s.blank_height, 151.0 + 2.0 * (50.5 + 15.0)), "{id}");
    }
}

#[test]
fn tray_corner_flaps_must_fit_the_end_wall() {
    let params = BTreeMap::from([
        ("W".to_string(), Value::Number(100.0)),
        ("corner_flap".to_string(), Value::Number(60.0)),
    ]);
    assert!(matches!(
        render(&project("tray-glued-corners", params, "C-flute")),
        Err(CoreError::Render(RenderError::Rule(_)))
    ));
}

#[test]
fn cartons_suggest_folding_boxboard() {
    for id in ["tuck-end-straight", "tuck-end-reverse"] {
        assert_eq!(
            describe(id)
                .expect("describe")
                .meta
                .default_board
                .as_deref(),
            Some("FBB-0.5")
        );
    }
    assert_eq!(
        describe("fefco-0201").expect("describe").meta.default_board,
        None
    );
}

#[test]
fn cartons_warn_on_corrugated_board() {
    let r = render(&project("tuck-end-straight", BTreeMap::new(), "C-flute")).expect("render");
    assert!(r
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Warning && d.message.contains("folding boxboard")));
}

#[test]
fn straight_and_reverse_tuck_differ_only_in_bottom_lid_position() {
    let ste = render(&project("tuck-end-straight", BTreeMap::new(), "FBB-0.5")).expect("ste");
    let rte = render(&project("tuck-end-reverse", BTreeMap::new(), "FBB-0.5")).expect("rte");
    assert!((ste.stats.outline_area_m2 - rte.stats.outline_area_m2).abs() < 1e-12);
    assert!((ste.stats.cut_length - rte.stats.cut_length).abs() < 1e-9);
    let lid_x = |r: &RenderResult| {
        r.fold
            .panels
            .iter()
            .find(|p| p.id == "lid_bottom")
            .expect("lid_bottom")
            .origin
            .x
    };
    let back_x = |r: &RenderResult| {
        r.fold
            .panels
            .iter()
            .find(|p| p.id == "back")
            .expect("back")
            .origin
            .x
    };
    let front_x = |r: &RenderResult| {
        r.fold
            .panels
            .iter()
            .find(|p| p.id == "front")
            .expect("front")
            .origin
            .x
    };
    assert_eq!(lid_x(&ste), back_x(&ste));
    assert_eq!(lid_x(&rte), front_x(&rte));
}
