//! Production export pipeline: options, layout and every format for every style.

use pf_core::{
    export_with, list_components, nest_preview, BoardRef, ExportOptions, Format, LineType, Nesting,
    Project, Units, Value,
};
use std::collections::BTreeMap;

fn project(id: &str, board: &str) -> Project {
    Project {
        component_id: id.into(),
        component_version: None,
        params: BTreeMap::from([
            ("L".to_string(), Value::Number(300.0)),
            ("W".to_string(), Value::Number(200.0)),
            ("H".to_string(), Value::Number(150.0)),
        ]),
        board: BoardRef::Preset(board.into()),
        units: Units::Mm,
    }
}

fn rsc() -> Project {
    project("fefco-0201", "C-flute")
}

fn text(p: &Project, f: Format, o: &ExportOptions) -> String {
    String::from_utf8(export_with(p, f, o).expect("export").bytes).expect("utf8")
}

fn sheet(w: f64, h: f64) -> Nesting {
    Nesting {
        sheet_width: w,
        sheet_height: h,
        gap: 8.0,
        margin: 10.0,
        allow_rotation: true,
        max_count: None,
    }
}

/// Sum of CF2 line lengths of a given type code.
fn cf2_length(cf2: &str, ty: &str) -> f64 {
    cf2.lines()
        .filter(|l| l.starts_with("L,"))
        .map(|l| l.split(',').collect::<Vec<_>>())
        .filter(|f| f[2] == ty)
        .map(|f| {
            let n = |i: usize| f[i].parse::<f64>().expect("num");
            (n(6) - n(4)).hypot(n(7) - n(5))
        })
        .sum()
}

#[test]
fn every_format_for_every_style() {
    for c in list_components().expect("library") {
        let board = if c.id.starts_with("tuck-end") {
            "FBB-0.5"
        } else {
            "C-flute"
        };
        let p = project(&c.id, board);
        for f in [
            Format::Svg,
            Format::Dxf,
            Format::Pdf,
            Format::Cf2,
            Format::Hpgl,
            Format::Glb,
        ] {
            let file = export_with(&p, f, &ExportOptions::default())
                .unwrap_or_else(|e| panic!("{} {f:?}: {e}", c.id));
            let head: &[u8] = match f {
                Format::Svg => b"<svg",
                Format::Dxf => b"  0\nSECTION",
                Format::Pdf => b"%PDF-1.4",
                Format::Cf2 => b"$BOF",
                Format::Hpgl => b"IN;",
                Format::Glb => b"glTF",
            };
            assert!(file.bytes.starts_with(head), "{} {f:?}", c.id);
            assert_eq!(file.filename, format!("{}.{}", c.id, f.extension()));
        }
    }
}

#[test]
fn rotation_turns_the_blank_and_keeps_lengths() {
    let plain = text(&rsc(), Format::Cf2, &ExportOptions::default());
    let turned = text(
        &rsc(),
        Format::Cf2,
        &ExportOptions {
            rotate: 90,
            ..ExportOptions::default()
        },
    );
    // Blank 1051 × 362 becomes 362 × 1051, normalised to the origin.
    assert!(plain.contains("LL,0,0\nUR,1051,362\n"), "{}", &plain[..120]);
    assert!(
        turned.contains("LL,0,0\nUR,362,1051\n"),
        "{}",
        &turned[..120]
    );
    for ty in ["1", "2"] {
        assert!((cf2_length(&plain, ty) - cf2_length(&turned, ty)).abs() < 1e-6);
    }
    let mirrored = text(
        &rsc(),
        Format::Cf2,
        &ExportOptions {
            mirror: true,
            ..ExportOptions::default()
        },
    );
    assert!((cf2_length(&plain, "1") - cf2_length(&mirrored, "1")).abs() < 1e-6);
    assert!(export_with(
        &rsc(),
        Format::Dxf,
        &ExportOptions {
            rotate: 45,
            ..ExportOptions::default()
        }
    )
    .is_err());
}

#[test]
fn nesting_places_copies_and_reports_utilisation() {
    let n = sheet(2400.0, 1600.0);
    let summary = nest_preview(&rsc(), &n).expect("nest");
    // 1051 × 362 blank on 2380 × 1580 usable: 2 across × 4 down = 8.
    assert_eq!(
        (summary.cols, summary.rows, summary.count),
        (2, 4, 8),
        "{summary:?}"
    );
    assert!(!summary.rotated);
    assert!(
        summary.utilisation > 0.5 && summary.utilisation < 1.0,
        "{}",
        summary.utilisation
    );

    let opts = ExportOptions {
        nesting: Some(n),
        ..ExportOptions::default()
    };
    let single = text(&rsc(), Format::Cf2, &ExportOptions::default());
    let nested = text(&rsc(), Format::Cf2, &opts);
    assert!((cf2_length(&nested, "1") - 8.0 * cf2_length(&single, "1")).abs() < 1e-3);
    // The sheet outline is a drawing aid: present in SVG, not sent to the cutter.
    assert!(text(&rsc(), Format::Svg, &opts).contains(r#"id="annotation""#));
    assert!(!nested.contains(",7,0,")); // annotation CF2 type not written
    assert!(text(&rsc(), Format::Pdf, &opts).contains("8 up"));

    let too_small = sheet(500.0, 500.0);
    let err = nest_preview(&rsc(), &too_small).expect_err("too small");
    assert_eq!(err.code(), "export_error");
}

#[test]
fn profiles_control_what_reaches_the_machine() {
    let mut p = rsc();
    // Use a style with only cut and crease lines; the cut-crease profile must keep both.
    p.component_id = "fefco-0201".into();
    let cc = text(
        &p,
        Format::Hpgl,
        &ExportOptions {
            profile: Some("cut-crease".into()),
            ..ExportOptions::default()
        },
    );
    assert!(cc.contains("SP1;") && cc.contains("SP2;"));
    let mut custom = pf_core::Profile::standard();
    custom
        .lines
        .get_mut(&LineType::Crease)
        .expect("crease")
        .include = false;
    custom.lines.get_mut(&LineType::Cut).expect("cut").layer = "KNIFE".into();
    let opts = ExportOptions {
        custom_profile: Some(custom),
        ..ExportOptions::default()
    };
    let hpgl = text(&p, Format::Hpgl, &opts);
    assert!(hpgl.contains("SP1;") && !hpgl.contains("SP2;"));
    assert!(text(&p, Format::Dxf, &opts).contains("KNIFE"));
    assert!(export_with(
        &p,
        Format::Cf2,
        &ExportOptions {
            profile: Some("unknown".into()),
            ..ExportOptions::default()
        }
    )
    .is_err());
}

#[test]
fn pdf_page_options() {
    let actual = text(&rsc(), Format::Pdf, &ExportOptions::default());
    assert!(actual.contains("Scale 1:1"));
    let a4 = text(
        &rsc(),
        Format::Pdf,
        &ExportOptions {
            page: Some("a4".into()),
            watermark: Some("PackForge Free".into()),
            ..ExportOptions::default()
        },
    );
    assert!(a4.contains("fitted to page") && a4.contains("(PackForge Free) Tj"));
    assert!(a4.contains("Regular slotted container \\(FEFCO 0201\\)"));
    assert!(a4.contains("L=300  W=200  H=150"));
    assert!(export_with(
        &rsc(),
        Format::Pdf,
        &ExportOptions {
            page: Some("tabloid".into()),
            ..ExportOptions::default()
        }
    )
    .is_err());
}
