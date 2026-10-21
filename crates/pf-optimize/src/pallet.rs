//! Pallet loading: how many identical boxes fit on a pallet, and in which layer pattern.
//!
//! Layer patterns (classic manufacturer's pallet loading heuristics):
//! - **block**: every box in the same orientation (columns);
//! - **two-block**: the pallet split once (along its length or width) into two blocks with
//!   different box orientations;
//! - **pinwheel**: four blocks turning around the centre (often fits more and interlocks).
//!
//! Layers alternate between the pattern and its mirror image when that differs, which ties the
//! stack together. Dimensions in millimetres, weights in kilograms.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pallet {
    pub length: f64,
    pub width: f64,
    /// Height of the pallet itself.
    pub height: f64,
}

impl Pallet {
    /// Common pallet sizes by id.
    pub fn preset(id: &str) -> Option<(Pallet, &'static str)> {
        let p = |length, width, height| Pallet {
            length,
            width,
            height,
        };
        Some(match id {
            "eur" => (p(1200.0, 800.0, 144.0), "EUR / EPAL 1200 × 800"),
            "eur-half" => (p(800.0, 600.0, 144.0), "Half EUR 800 × 600"),
            "iso-1200x1000" => (p(1200.0, 1000.0, 144.0), "ISO 1200 × 1000"),
            "gma" => (p(1219.0, 1016.0, 127.0), "GMA 48 × 40 in"),
            "asia" => (p(1100.0, 1100.0, 144.0), "Asia 1100 × 1100"),
            _ => return None,
        })
    }

    pub const PRESETS: [&'static str; 5] = ["eur", "eur-half", "iso-1200x1000", "gma", "asia"];
}

/// What to load and the limits.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Load {
    /// Outer box size: length, width, height.
    pub box_length: f64,
    pub box_width: f64,
    pub box_height: f64,
    /// Weight of one filled box (optional).
    #[serde(default)]
    pub box_weight: Option<f64>,
    /// Maximum total height including the pallet.
    pub max_height: f64,
    /// Maximum load weight (boxes only), if any.
    #[serde(default)]
    pub max_weight: Option<f64>,
    /// How far boxes may stick out past each pallet edge.
    #[serde(default)]
    pub overhang: f64,
    /// Allow boxes on their side (any dimension vertical), not only upright.
    #[serde(default)]
    pub allow_on_side: bool,
}

/// A box footprint in a layer (lower-left corner, size along x and y).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Pattern {
    Block,
    TwoBlock,
    Pinwheel,
}

impl Pattern {
    pub fn label(self) -> &'static str {
        match self {
            Pattern::Block => "block",
            Pattern::TwoBlock => "two-block",
            Pattern::Pinwheel => "pinwheel",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PalletPlan {
    pub pallet: Pallet,
    pub pattern: Pattern,
    /// Box dimension standing vertical: "height" (upright), "width" or "length" (on its side).
    pub standing_on: &'static str,
    pub per_layer: usize,
    pub layers: usize,
    pub total: usize,
    /// Box positions in the odd layers; even layers are mirrored when `alternate` is true.
    pub layer: Vec<Rect>,
    pub alternate: bool,
    /// Box height in the chosen orientation.
    pub layer_height: f64,
    pub load_height: f64,
    pub load_weight: Option<f64>,
    /// Pallet area covered by boxes in a layer (0..1).
    pub area_efficiency: f64,
    /// Volume used up to the maximum height (0..1).
    pub volume_efficiency: f64,
    /// Which limit stopped stacking: "height" or "weight".
    pub limited_by: &'static str,
}

/// Best layer for footprint a × b on an area `len` × `wid`.
pub fn best_layer(len: f64, wid: f64, a: f64, b: f64) -> (Pattern, Vec<Rect>) {
    let mut best = (Pattern::Block, Vec::new());
    let mut consider = |p: Pattern, rects: Vec<Rect>| {
        if rects.len() > best.1.len() {
            best = (p, rects);
        }
    };
    for (x, y) in [(a, b), (b, a)] {
        consider(Pattern::Block, grid(0.0, 0.0, len, wid, x, y));
    }
    for r in two_block(len, wid, a, b) {
        consider(Pattern::TwoBlock, r);
    }
    if let Some(r) = pinwheel(len, wid, a, b) {
        consider(Pattern::Pinwheel, r);
    }
    best
}

fn count_fit(space: f64, size: f64) -> usize {
    if size <= 0.0 || space < size - 1e-9 {
        0
    } else {
        ((space + 1e-9) / size).floor() as usize
    }
}

fn grid(x0: f64, y0: f64, len: f64, wid: f64, a: f64, b: f64) -> Vec<Rect> {
    let (nx, ny) = (count_fit(len, a), count_fit(wid, b));
    let mut v = Vec::with_capacity(nx * ny);
    for j in 0..ny {
        for i in 0..nx {
            v.push(Rect {
                x: x0 + i as f64 * a,
                y: y0 + j as f64 * b,
                w: a,
                h: b,
            });
        }
    }
    v
}

/// One guillotine cut (vertical or horizontal) with a different orientation on each side.
fn two_block(len: f64, wid: f64, a: f64, b: f64) -> Vec<Vec<Rect>> {
    let mut out = Vec::new();
    for (p, q) in [(a, b), (b, a)] {
        // Vertical split: k columns of p-wide boxes on the left, rotated boxes on the right.
        for k in 1..=count_fit(len, p) {
            let x = k as f64 * p;
            let mut r = grid(0.0, 0.0, x, wid, p, q);
            r.extend(grid(x, 0.0, len - x, wid, q, p));
            out.push(r);
        }
        // Horizontal split.
        for k in 1..=count_fit(wid, q) {
            let y = k as f64 * q;
            let mut r = grid(0.0, 0.0, len, y, p, q);
            r.extend(grid(0.0, y, len, wid - y, q, p));
            out.push(r);
        }
    }
    out
}

fn overlaps(r: &Rect, s: &Rect) -> bool {
    r.x < s.x + s.w - 1e-9
        && s.x < r.x + r.w - 1e-9
        && r.y < s.y + s.h - 1e-9
        && s.y < r.y + r.h - 1e-9
}

/// Symmetric four-block pinwheel: blocks in the four corners, alternately turned 90°.
fn pinwheel(len: f64, wid: f64, a: f64, b: f64) -> Option<Vec<Rect>> {
    let mut best: Option<Vec<Rect>> = None;
    let (max_a, max_b) = (
        count_fit(len.max(wid), a.min(b)).min(30),
        count_fit(len.max(wid), a.min(b)).min(30),
    );
    for n1 in 1..=max_a {
        for m1 in 1..=max_b {
            // Block 1 (bottom-left): n1 × m1 boxes lying a × b.
            let (w1, h1) = (n1 as f64 * a, m1 as f64 * b);
            if w1 > len || h1 > wid {
                break;
            }
            for n2 in 1..=max_a {
                for m2 in 1..=max_b {
                    // Block 2 (bottom-right): n2 × m2 boxes turned, b × a.
                    let (w2, h2) = (n2 as f64 * b, m2 as f64 * a);
                    if w2 > len || h2 > wid {
                        break;
                    }
                    if w1 + w2 > len + 1e-9 {
                        continue;
                    }
                    let blocks = [
                        Rect {
                            x: 0.0,
                            y: 0.0,
                            w: w1,
                            h: h1,
                        },
                        Rect {
                            x: len - w2,
                            y: 0.0,
                            w: w2,
                            h: h2,
                        },
                        Rect {
                            x: len - w1,
                            y: wid - h1,
                            w: w1,
                            h: h1,
                        },
                        Rect {
                            x: 0.0,
                            y: wid - h2,
                            w: w2,
                            h: h2,
                        },
                    ];
                    let clash =
                        (0..4).any(|i| (i + 1..4).any(|j| overlaps(&blocks[i], &blocks[j])));
                    if clash {
                        continue;
                    }
                    let count = 2 * (n1 * m1 + n2 * m2);
                    if count <= best.as_ref().map_or(0, Vec::len) {
                        continue;
                    }
                    let mut rects = Vec::with_capacity(count);
                    for (bi, blk) in blocks.iter().enumerate() {
                        let (bw, bh) = if bi % 2 == 0 { (a, b) } else { (b, a) };
                        rects.extend(grid(blk.x, blk.y, blk.w, blk.h, bw, bh));
                    }
                    best = Some(rects);
                }
            }
        }
    }
    best
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PalletError {
    #[error("box dimensions must be positive")]
    BadBox,
    #[error("the box does not fit on the pallet")]
    NoFit,
    #[error("the maximum height leaves no room for a layer")]
    TooLow,
}

/// Plans a pallet load.
pub fn plan(pallet: Pallet, load: &Load) -> Result<PalletPlan, PalletError> {
    let (l, w, h) = (load.box_length, load.box_width, load.box_height);
    if !(l > 0.0 && w > 0.0 && h > 0.0) {
        return Err(PalletError::BadBox);
    }
    let (plen, pwid) = (
        pallet.length + 2.0 * load.overhang,
        pallet.width + 2.0 * load.overhang,
    );
    let avail = load.max_height - pallet.height;

    // Candidate orientations: (footprint a, footprint b, vertical, name).
    let mut orientations = vec![(l, w, h, "height")];
    if load.allow_on_side {
        orientations.push((l, h, w, "width"));
        orientations.push((w, h, l, "length"));
    }

    let mut best: Option<PalletPlan> = None;
    let mut any_fit = false;
    for (a, b, v, name) in orientations {
        let (pattern, rects) = best_layer(plen, pwid, a, b);
        if rects.is_empty() {
            continue;
        }
        any_fit = true;
        let by_height = count_fit(avail, v);
        let mut layers = by_height;
        let mut limited_by = "height";
        if let (Some(bw), Some(max)) = (load.box_weight, load.max_weight) {
            if bw > 0.0 {
                let by_weight = ((max / bw).floor() as usize) / rects.len();
                if by_weight < layers {
                    layers = by_weight;
                    limited_by = "weight";
                }
            }
        }
        let per_layer = rects.len();
        let total = per_layer * layers;
        if best.as_ref().is_some_and(|b| b.total >= total) {
            continue;
        }
        // Shift so the layer is centred on the pallet (overhang shared on both sides).
        let (min_x, min_y, max_x, max_y) = rects.iter().fold(
            (
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ),
            |(a0, b0, a1, b1), r| {
                (
                    a0.min(r.x),
                    b0.min(r.y),
                    a1.max(r.x + r.w),
                    b1.max(r.y + r.h),
                )
            },
        );
        let dx = (pallet.length - (max_x - min_x)) / 2.0 - min_x;
        let dy = (pallet.width - (max_y - min_y)) / 2.0 - min_y;
        let layer: Vec<Rect> = rects
            .iter()
            .map(|r| Rect {
                x: r.x + dx,
                y: r.y + dy,
                ..*r
            })
            .collect();
        let box_area = a * b * per_layer as f64;
        best = Some(PalletPlan {
            pallet,
            pattern,
            standing_on: name,
            per_layer,
            layers,
            total,
            alternate: pattern != Pattern::Block,
            layer,
            layer_height: v,
            load_height: pallet.height + layers as f64 * v,
            load_weight: load.box_weight.map(|bw| bw * total as f64),
            area_efficiency: box_area / (pallet.length * pallet.width),
            volume_efficiency: if avail > 0.0 {
                (box_area * v * layers as f64) / (pallet.length * pallet.width * avail)
            } else {
                0.0
            },
            limited_by,
        });
    }
    match best {
        Some(p) if p.layers > 0 => Ok(p),
        Some(_) => Err(PalletError::TooLow),
        None if !any_fit => Err(PalletError::NoFit),
        None => Err(PalletError::TooLow),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eur() -> Pallet {
        Pallet::preset("eur").expect("eur").0
    }

    fn load(l: f64, w: f64, h: f64) -> Load {
        Load {
            box_length: l,
            box_width: w,
            box_height: h,
            box_weight: None,
            max_height: 1800.0,
            max_weight: None,
            overhang: 0.0,
            allow_on_side: false,
        }
    }

    fn assert_valid(p: &PalletPlan, load: &Load) {
        let o = load.overhang;
        for (i, r) in p.layer.iter().enumerate() {
            assert!(
                r.x >= -o - 1e-6 && r.y >= -o - 1e-6,
                "box {i} off the pallet: {r:?}"
            );
            assert!(
                r.x + r.w <= p.pallet.length + o + 1e-6 && r.y + r.h <= p.pallet.width + o + 1e-6
            );
            for s in &p.layer[i + 1..] {
                assert!(!overlaps(r, s), "boxes overlap: {r:?} {s:?}");
            }
        }
        assert!(p.load_height <= load.max_height + 1e-9);
    }

    #[test]
    fn exact_block_fit() {
        // 400 × 200 on 1200 × 800: 3 × 4 = 12 per layer in one orientation.
        let l = load(400.0, 200.0, 300.0);
        let p = plan(eur(), &l).expect("plan");
        assert_eq!(p.per_layer, 12);
        assert_eq!(p.layers, 5); // (1800 - 144) / 300 = 5.52
        assert_eq!(p.total, 60);
        assert!((p.area_efficiency - 1.0).abs() < 1e-9);
        assert_valid(&p, &l);
    }

    #[test]
    fn mixed_patterns_beat_a_single_block() {
        // 300 × 200 × 150 on EUR: block gives 4 × 4 = 16 (area 960 000 of 960 000) — perfect.
        let l = load(300.0, 200.0, 150.0);
        assert_eq!(plan(eur(), &l).expect("plan").per_layer, 16);
        // 350 × 250: block ≤ 3 × 3 = 9; mixing orientations fits more.
        let l = load(350.0, 250.0, 200.0);
        let p = plan(eur(), &l).expect("plan");
        let block = best_layer(1200.0, 800.0, 350.0, 250.0);
        assert!(p.per_layer >= 9 && p.per_layer == block.1.len());
        assert!(
            p.per_layer > 9,
            "expected a mixed pattern to beat 9, got {p:?}"
        );
        assert_ne!(p.pattern, Pattern::Block);
        assert!(p.alternate);
        assert_valid(&p, &l);
    }

    #[test]
    fn layouts_are_always_valid() {
        for &(a, b) in &[
            (410.0, 290.0),
            (365.0, 245.0),
            (600.0, 400.0),
            (230.0, 190.0),
            (515.0, 355.0),
        ] {
            let l = load(a, b, 250.0);
            let p = plan(eur(), &l).expect("plan");
            assert_valid(&p, &l);
            // Never worse than the simple block in either orientation.
            let simple = (count_fit(1200.0, a) * count_fit(800.0, b))
                .max(count_fit(1200.0, b) * count_fit(800.0, a));
            assert!(p.per_layer >= simple);
        }
    }

    #[test]
    fn weight_and_height_limits() {
        let mut l = load(400.0, 200.0, 300.0);
        l.box_weight = Some(10.0);
        l.max_weight = Some(400.0); // 40 boxes → 3 full layers of 12
        let p = plan(eur(), &l).expect("plan");
        assert_eq!((p.layers, p.total, p.limited_by), (3, 36, "weight"));
        assert_eq!(p.load_weight, Some(360.0));
        l.max_height = 300.0; // pallet 144 leaves 156 < 300
        assert_eq!(plan(eur(), &l).unwrap_err(), PalletError::TooLow);
        assert_eq!(
            plan(eur(), &load(1500.0, 900.0, 100.0)).unwrap_err(),
            PalletError::NoFit
        );
    }

    #[test]
    fn overhang_and_lying_on_side() {
        let mut l = load(410.0, 200.0, 300.0);
        // 2 × 4 lengthwise leaves 380 mm: room for one turned box (200 × 410) only.
        assert_eq!(plan(eur(), &l).expect("plan").per_layer, 9);
        l.overhang = 15.0; // 1230 usable: 3 × 4
        let p = plan(eur(), &l).expect("plan");
        assert_eq!(p.per_layer, 12);
        assert_valid(&p, &l);
        // A tall box fits more when laid down.
        let mut tall = load(300.0, 200.0, 1000.0);
        assert_eq!(plan(eur(), &tall).expect("plan").total, 16);
        tall.allow_on_side = true;
        let p = plan(eur(), &tall).expect("plan");
        assert!(p.total > 16 && p.standing_on != "height", "{p:?}");
    }
}
