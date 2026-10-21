//! Pallet plans, case sizing and the pallet report.

use pf_core::{
    box_outer, case_optimize, pallet_plan, pallet_report, BoardRef, CaseRequest, PalletChoice,
    PalletRequest, Project, Units, Value,
};
use std::collections::BTreeMap;

fn rsc(l: f64, w: f64, h: f64) -> Project {
    Project {
        component_id: "fefco-0201".into(),
        component_version: None,
        params: BTreeMap::from([
            ("L".to_string(), Value::Number(l)),
            ("W".to_string(), Value::Number(w)),
            ("H".to_string(), Value::Number(h)),
        ]),
        board: BoardRef::Preset("C-flute".into()),
        units: Units::Mm,
    }
}

#[test]
fn outer_size_adds_two_calipers() {
    assert_eq!(
        box_outer(&rsc(300.0, 200.0, 150.0)).expect("outer"),
        [308.0, 208.0, 158.0]
    );
}

#[test]
fn pallet_plan_for_a_design() {
    // Inside 392 × 192 × 292 → outer 400 × 200 × 300: a perfect 12 per EUR layer.
    let r = pallet_plan(&rsc(392.0, 192.0, 292.0), &PalletRequest::default()).expect("plan");
    assert_eq!(r.box_outer, [400.0, 200.0, 300.0]);
    assert_eq!((r.plan.per_layer, r.plan.layers, r.plan.total), (12, 5, 60));
    assert!(r.pallet_name.contains("EUR"));

    let custom = PalletRequest {
        pallet: PalletChoice::Custom(pf_core::Pallet {
            length: 1000.0,
            width: 1000.0,
            height: 100.0,
        }),
        box_outer: Some([250.0, 250.0, 250.0]),
        max_height: 1100.0,
        ..PalletRequest::default()
    };
    let r = pallet_plan(&rsc(300.0, 200.0, 150.0), &custom).expect("custom");
    assert_eq!((r.plan.per_layer, r.plan.layers), (16, 4));

    let bad = PalletRequest {
        pallet: PalletChoice::Preset("moon".into()),
        ..PalletRequest::default()
    };
    assert_eq!(
        pallet_plan(&rsc(300.0, 200.0, 150.0), &bad)
            .unwrap_err()
            .code(),
        "export_error"
    );
}

#[test]
fn case_sizing_ranks_by_board_use_and_respects_the_count() {
    let req = CaseRequest {
        product: [100.0, 80.0, 60.0],
        count: 12,
        component_id: "fefco-0201".into(),
        board: BoardRef::Preset("C-flute".into()),
        clearance: 4.0,
        allow_rotation: true,
        max_results: 5,
        pallet: None,
        board_price_per_m2: Some(0.65),
    };
    let c = case_optimize(&req).expect("cases");
    assert!(!c.is_empty() && c.len() <= 5);
    for k in &c {
        assert_eq!(k.arrangement.iter().product::<usize>(), 12);
        for i in 0..3 {
            let want = k.arrangement[i] as f64 * k.product_axes[i] + 4.0;
            assert!((k.inside[i] - want).abs() < 1e-9);
        }
        assert!(k.inside[0] >= k.inside[1]);
        let cost = k.board_cost_per_case.expect("cost");
        assert!((cost - 0.65 * k.blank_area_m2).abs() < 1e-12);
    }
    for pair in c.windows(2) {
        assert!(pair[0].board_m2_per_product <= pair[1].board_m2_per_product);
    }
    // Every product dimension appears once per candidate (a valid orientation).
    let mut dims = c[0].product_axes;
    dims.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    assert_eq!(dims, [60.0, 80.0, 100.0]);
}

#[test]
fn case_sizing_with_a_pallet_ranks_by_products_per_pallet() {
    let req = CaseRequest {
        product: [100.0, 80.0, 60.0],
        count: 12,
        component_id: "fefco-0201".into(),
        board: BoardRef::Preset("C-flute".into()),
        clearance: 4.0,
        allow_rotation: true,
        max_results: 10,
        pallet: Some(PalletRequest::default()),
        board_price_per_m2: None,
    };
    let c = case_optimize(&req).expect("cases");
    let per = |k: &pf_core::CaseCandidate| k.products_per_pallet.expect("pallet");
    for pair in c.windows(2) {
        assert!(per(&pair[0]) >= per(&pair[1]));
    }
    assert!(per(&c[0]) > 0);
}

#[test]
fn upright_only_keeps_the_product_height_vertical() {
    let req = CaseRequest {
        product: [100.0, 80.0, 200.0],
        count: 6,
        component_id: "fefco-0201".into(),
        board: BoardRef::Preset("B-flute".into()),
        clearance: 2.0,
        allow_rotation: false,
        max_results: 20,
        pallet: None,
        board_price_per_m2: None,
    };
    for k in case_optimize(&req).expect("cases") {
        assert_eq!(k.product_axes[2], 200.0);
    }
    let bad = CaseRequest { count: 0, ..req };
    assert!(case_optimize(&bad).is_err());
}

#[test]
fn pallet_report_is_a_pdf() {
    let pdf = pallet_report(&rsc(392.0, 192.0, 292.0), &PalletRequest::default()).expect("report");
    let s = String::from_utf8(pdf).expect("ascii");
    assert!(s.starts_with("%PDF-1.4"));
    assert!(s.contains("12 per layer x 5 layers = 60 boxes"));
}
