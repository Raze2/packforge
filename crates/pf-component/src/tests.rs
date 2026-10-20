use super::*;
use pf_model::{Board, LineType, Value};

const FEFCO_0201: &str = include_str!("../../../library/fefco/0201.pfc.toml");

fn board(caliper: f64) -> Board {
    Board {
        id: "test".into(),
        name: "Test board".into(),
        caliper_mm: caliper,
        flute: None,
    }
}

fn params(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

fn rsc() -> Component {
    Component::from_toml(FEFCO_0201).expect("0201 compiles")
}

#[test]
fn fefco_0201_compiles_with_schema() {
    let c = rsc();
    assert_eq!(c.id(), "fefco-0201");
    let ids: Vec<_> = c.info().params.iter().map(|p| p.id.clone()).collect();
    assert_eq!(ids, ["L", "W", "H", "glue_width", "glue_side"]);
}

#[test]
fn fefco_0201_default_dimensions() {
    let r = rsc()
        .render(&BTreeMap::new(), &board(4.0))
        .expect("renders");
    let s = r.dieline.stats();
    // width = g + 2(L+t) + 2(W+t) = 35 + 608 + 408
    assert!((s.blank_width - 1051.0).abs() < 1e-9, "{}", s.blank_width);
    // height = 2·(W+t)/2 + (H+2t) = 204 + 158
    assert!((s.blank_height - 362.0).abs() < 1e-9, "{}", s.blank_height);
    assert!(s.outline_area_m2 < s.blank_area_m2);
    assert!(s.outline_area_m2 > 0.8 * s.blank_area_m2);
    assert_eq!(
        r.dieline
            .entities
            .iter()
            .filter(|e| e.line_type == LineType::Crease)
            .count(),
        12
    );
    assert_eq!(r.fold.panels.len(), 13);
    assert_eq!(r.fold.hinges.len(), 12);
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
}

#[test]
fn glue_side_right_mirrors_the_blank() {
    let c = rsc();
    let left = c.render(&BTreeMap::new(), &board(4.0)).expect("left");
    let right = c
        .render(
            &params(&[("glue_side", Value::Text("right".into()))]),
            &board(4.0),
        )
        .expect("right");
    let (bl, br) = (left.dieline.bbox(), right.dieline.bbox());
    assert!((bl.width() - br.width()).abs() < 1e-9);
    let has_crease_at = |r: &Rendered, x: f64| {
        r.dieline.entities.iter().any(|e| {
            e.line_type == LineType::Crease && {
                let b = e.path.bbox();
                (b.min.x - x).abs() < 1e-9 && (b.max.x - x).abs() < 1e-9
            }
        })
    };
    assert!(has_crease_at(&left, 35.0));
    assert!(has_crease_at(&right, 1051.0 - 35.0));
    assert!(!has_crease_at(&right, 35.0));
}

#[test]
fn invalid_params_are_reported_per_param() {
    let err = rsc()
        .render(
            &params(&[
                ("L", Value::Number(5.0)),
                ("glue_side", Value::Text("top".into())),
            ]),
            &board(4.0),
        )
        .unwrap_err();
    match err {
        RenderError::Params(issues) => {
            let ids: Vec<_> = issues.iter().map(|i| i.param.as_str()).collect();
            assert_eq!(ids, ["L", "glue_side"]);
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn rules_produce_warnings() {
    let r = rsc()
        .render(
            &params(&[("L", Value::Number(200.0)), ("W", Value::Number(300.0))]),
            &board(4.0),
        )
        .expect("renders");
    assert!(r.diagnostics.iter().any(|d| d.message.contains("Width")));
}

#[test]
fn unknown_parameter_is_a_warning_not_an_error() {
    let r = rsc()
        .render(
            &params(&[("colour", Value::Text("red".into()))]),
            &board(4.0),
        )
        .expect("renders");
    assert!(r.diagnostics.iter().any(|d| d.message.contains("colour")));
}

#[test]
fn sweep_sizes_and_boards() {
    let c = rsc();
    for &t in &[0.5, 1.6, 3.0, 4.0, 7.0] {
        for &l in &[40.0, 120.0, 600.0, 2500.0] {
            for &w in &[40.0, 100.0, 400.0] {
                for &h in &[20.0, 150.0, 1200.0] {
                    let p = params(&[
                        ("L", Value::Number(l)),
                        ("W", Value::Number(w)),
                        ("H", Value::Number(h)),
                    ]);
                    let r = c
                        .render(&p, &board(t))
                        .unwrap_or_else(|e| panic!("L{l} W{w} H{h} t{t}: {e}"));
                    let s = r.dieline.stats();
                    let exp_w = 35.0 + 2.0 * (l + t) + 2.0 * (w + t);
                    let exp_h = (w + t) + (h + 2.0 * t);
                    assert!((s.blank_width - exp_w).abs() < 1e-6);
                    assert!((s.blank_height - exp_h).abs() < 1e-6);
                }
            }
        }
    }
}

#[test]
fn compile_errors_name_the_location() {
    let base = r#"
[component]
id = "x"
version = "0.1.0"
name = "x"
category = "test"
[[param]]
id = "L"
label = "L"
type = "length"
default = 10
"#;
    let unknown = format!("{base}\n[derived]\na = \"L + nope\"\n");
    let e = Component::from_toml(&unknown).unwrap_err().to_string();
    assert!(e.contains("derived 'a'") && e.contains("nope"), "{e}");

    let cycle = format!("{base}\n[derived]\na = \"b + 1\"\nb = \"a + 1\"\n");
    let e = Component::from_toml(&cycle).unwrap_err().to_string();
    assert!(e.contains("circular"), "{e}");

    let bad_type = format!(
        "{base}\n[[path]]\ntype = \"laser\"\nclosed = true\npoints = [[\"0\",\"0\"],[\"L\",\"0\"],[\"L\",\"L\"]]\n"
    );
    let e = Component::from_toml(&bad_type).unwrap_err().to_string();
    assert!(e.contains("laser"), "{e}");

    let typo = format!("{base}\n[[pannel]]\nid = \"p\"\n");
    assert!(matches!(
        Component::from_toml(&typo),
        Err(ComponentError::Syntax(_))
    ));
}

#[test]
fn missing_outline_is_a_geometry_error() {
    let src = r#"
[component]
id = "x"
version = "0.1.0"
name = "x"
category = "test"
[[line]]
type = "crease"
from = [0, 0]
to = [10, 0]
"#;
    let c = Component::from_toml(src).expect("compiles");
    assert!(matches!(
        c.render(&BTreeMap::new(), &board(1.0)),
        Err(RenderError::Geometry(_))
    ));
}
