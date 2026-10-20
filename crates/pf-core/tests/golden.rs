//! Golden-file tests: exports must match committed files byte for byte.
//! Regenerate intentionally with `UPDATE_GOLDEN=1 cargo test -p pf-core`.

use pf_core::{export, list_components, render, BoardRef, Format, Project, Units, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden")
}

fn project(params: &[(&str, Value)], board: &str) -> Project {
    Project {
        component_id: "fefco-0201".into(),
        component_version: None,
        params: params
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect::<BTreeMap<_, _>>(),
        board: BoardRef::Preset(board.into()),
        units: Units::Mm,
    }
}

fn check(name: &str, actual: &[u8]) {
    let path = golden_dir().join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(golden_dir()).expect("create golden dir");
        std::fs::write(&path, actual).expect("write golden");
        return;
    }
    let expected = std::fs::read(&path).unwrap_or_else(|_| {
        panic!(
            "missing golden file {}; run with UPDATE_GOLDEN=1",
            path.display()
        )
    });
    assert!(
        expected == actual,
        "{name} differs from golden file; review and regenerate with UPDATE_GOLDEN=1"
    );
}

fn cases() -> Vec<(&'static str, Project)> {
    vec![
        (
            "fefco-0201_300x200x150_C",
            project(
                &[
                    ("L", Value::Number(300.0)),
                    ("W", Value::Number(200.0)),
                    ("H", Value::Number(150.0)),
                ],
                "C-flute",
            ),
        ),
        (
            "fefco-0201_400x300x250_B_right",
            project(
                &[
                    ("L", Value::Number(400.0)),
                    ("W", Value::Number(300.0)),
                    ("H", Value::Number(250.0)),
                    ("glue_side", Value::Text("right".into())),
                ],
                "B-flute",
            ),
        ),
    ]
}

#[test]
fn golden_exports() {
    for (name, p) in cases() {
        for f in [Format::Svg, Format::Dxf] {
            let file = export(&p, f).expect("export");
            check(&format!("{name}.{}", f.extension()), &file.bytes);
        }
        let stats = serde_json::to_string_pretty(&render(&p).expect("render").stats).expect("json");
        check(&format!("{name}.stats.json"), stats.as_bytes());
    }
}

#[test]
fn exports_are_deterministic() {
    for (_, p) in cases() {
        for f in [Format::Svg, Format::Dxf] {
            assert_eq!(export(&p, f).expect("a"), export(&p, f).expect("b"));
        }
    }
}

#[test]
fn library_lists_fefco_0201() {
    let list = list_components().expect("library compiles");
    assert!(list.iter().any(|c| c.id == "fefco-0201"));
}

#[test]
fn project_json_round_trip_and_errors() {
    let p: Project = serde_json::from_str(
        r#"{"component_id":"fefco-0201","params":{"L":300,"W":200,"H":150},"board":"C-flute"}"#,
    )
    .expect("parse");
    assert!(render(&p).is_ok());

    let mut bad = p.clone();
    bad.board = BoardRef::Preset("Z-flute".into());
    assert_eq!(render(&bad).unwrap_err().code(), "unknown_board");

    let mut bad = p.clone();
    bad.component_id = "fefco-9999".into();
    assert_eq!(render(&bad).unwrap_err().code(), "unknown_component");

    let mut bad = p;
    bad.params.insert("H".into(), Value::Number(-1.0));
    assert_eq!(render(&bad).unwrap_err().code(), "invalid_params");
}
