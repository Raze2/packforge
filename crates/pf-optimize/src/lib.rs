//! Optimisation: sheet nesting today; case sizing and pallet loading in Stage 8.

use serde::{Deserialize, Serialize};

/// Where to place blanks on a sheet. Millimetres.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Nesting {
    pub sheet_width: f64,
    pub sheet_height: f64,
    /// Space between blanks (knife clearance).
    #[serde(default = "default_gap")]
    pub gap: f64,
    /// Unused border around the sheet (grippers, trim).
    #[serde(default = "default_margin")]
    pub margin: f64,
    /// Allow turning blanks 90° when more fit that way.
    #[serde(default = "yes")]
    pub allow_rotation: bool,
    /// Place at most this many blanks.
    #[serde(default)]
    pub max_count: Option<usize>,
}

fn default_gap() -> f64 {
    8.0
}
fn default_margin() -> f64 {
    10.0
}
fn yes() -> bool {
    true
}

/// Lower-left corner of a placed blank (after optional 90° rotation).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Placement {
    pub x: f64,
    pub y: f64,
    pub rotated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Layout {
    pub placements: Vec<Placement>,
    pub cols: usize,
    pub rows: usize,
    pub rotated: bool,
}

impl Layout {
    pub fn count(&self) -> usize {
        self.placements.len()
    }
}

fn fit(len: f64, space: f64, gap: f64) -> usize {
    if len <= 0.0 || space < len {
        return 0;
    }
    (((space + gap) / (len + gap)) + 1e-9).floor() as usize
}

/// Grid nesting of identical rectangular blanks (`w` × `h`) in one orientation, whichever fits
/// more. Mixed orientations in the leftover strip are a later improvement.
pub fn nest_grid(w: f64, h: f64, n: &Nesting) -> Layout {
    let (sw, sh) = (
        n.sheet_width - 2.0 * n.margin,
        n.sheet_height - 2.0 * n.margin,
    );
    let mut best: Option<(usize, usize, bool)> = None;
    let mut orientations = vec![(w, h, false)];
    if n.allow_rotation {
        orientations.push((h, w, true));
    }
    for (bw, bh, rot) in orientations {
        let (cols, rows) = (fit(bw, sw, n.gap), fit(bh, sh, n.gap));
        let count = cols * rows;
        if count > best.map_or(0, |(c, r, _)| c * r) {
            best = Some((cols, rows, rot));
        }
    }
    let Some((cols, rows, rotated)) = best else {
        return Layout {
            placements: vec![],
            cols: 0,
            rows: 0,
            rotated: false,
        };
    };
    let (bw, bh) = if rotated { (h, w) } else { (w, h) };
    let limit = n.max_count.unwrap_or(usize::MAX);
    let mut placements = Vec::new();
    'rows: for r in 0..rows {
        for c in 0..cols {
            if placements.len() >= limit {
                break 'rows;
            }
            placements.push(Placement {
                x: n.margin + c as f64 * (bw + n.gap),
                y: n.margin + r as f64 * (bh + n.gap),
                rotated,
            });
        }
    }
    Layout {
        placements,
        cols,
        rows,
        rotated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet(w: f64, h: f64) -> Nesting {
        Nesting {
            sheet_width: w,
            sheet_height: h,
            gap: 10.0,
            margin: 10.0,
            allow_rotation: true,
            max_count: None,
        }
    }

    #[test]
    fn fills_a_grid_and_respects_margins_and_gaps() {
        let l = nest_grid(300.0, 200.0, &sheet(1000.0, 650.0));
        // usable 980 × 630: 3 × 3 (300+10 fits 3 times in 990, 200+10 three times in 640).
        assert_eq!((l.cols, l.rows, l.count()), (3, 3, 9));
        let last = l.placements[8];
        assert!(last.x + 300.0 <= 1000.0 - 10.0 + 1e-9 && last.y + 200.0 <= 650.0 - 10.0 + 1e-9);
        assert!((l.placements[1].x - l.placements[0].x - 310.0).abs() < 1e-9);
    }

    #[test]
    fn rotates_when_more_fit() {
        let l = nest_grid(500.0, 100.0, &sheet(330.0, 1100.0));
        // Usable 310 × 1080: turned blanks are 100 × 500, so 2 across × 2 down; unturned none fit.
        assert!(l.rotated && l.count() == 4, "{l:?}");
        let mut no_rot = sheet(330.0, 1100.0);
        no_rot.allow_rotation = false;
        assert_eq!(nest_grid(500.0, 100.0, &no_rot).count(), 0);
    }

    #[test]
    fn max_count_and_too_big() {
        let mut s = sheet(2000.0, 2000.0);
        s.max_count = Some(4);
        assert_eq!(nest_grid(100.0, 100.0, &s).count(), 4);
        assert_eq!(nest_grid(3000.0, 100.0, &sheet(1000.0, 1000.0)).count(), 0);
    }
}
