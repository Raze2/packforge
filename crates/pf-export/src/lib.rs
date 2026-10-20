//! Export writers. Every writer is deterministic: same die-line ⇒ identical bytes.

mod cf2;
mod dxf;
mod hpgl;
mod pdf;
mod profile;
mod svg;

pub use cf2::to_cf2;
pub use dxf::{to_dxf, DxfOptions};
pub use hpgl::to_hpgl;
pub use pdf::{to_pdf, PageSize, PdfOptions};
pub use profile::{LineMap, Profile};
pub use svg::{to_svg, SvgOptions};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Svg,
    Dxf,
    Pdf,
    Cf2,
    /// HPGL plot file (.plt).
    Hpgl,
    /// Binary glTF of the erected box (written by pf-fold3d).
    Glb,
}

impl Format {
    pub fn parse(s: &str) -> Option<Format> {
        match s.to_ascii_lowercase().as_str() {
            "svg" => Some(Format::Svg),
            "dxf" => Some(Format::Dxf),
            "pdf" => Some(Format::Pdf),
            "cf2" => Some(Format::Cf2),
            "hpgl" | "plt" => Some(Format::Hpgl),
            "glb" => Some(Format::Glb),
            _ => None,
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Format::Svg => "svg",
            Format::Dxf => "dxf",
            Format::Pdf => "pdf",
            Format::Cf2 => "cf2",
            Format::Hpgl => "plt",
            Format::Glb => "glb",
        }
    }

    pub fn mime(self) -> &'static str {
        match self {
            Format::Svg => "image/svg+xml",
            Format::Dxf => "application/dxf",
            Format::Pdf => "application/pdf",
            Format::Cf2 => "application/vnd.cf2",
            Format::Hpgl => "application/vnd.hp-hpgl",
            Format::Glb => "model/gltf-binary",
        }
    }
}

/// Formats a millimetre value with at most 4 decimals and no trailing zeros or `-0`.
pub(crate) fn num(v: f64) -> String {
    let r = (v * 10_000.0).round() / 10_000.0;
    if r == 0.0 {
        "0".to_string()
    } else {
        format!("{r}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_formatting_is_stable() {
        assert_eq!(num(1051.0), "1051");
        assert_eq!(num(0.1 + 0.2), "0.3");
        assert_eq!(num(-0.00001), "0");
        assert_eq!(num(12.345_678), "12.3457");
        assert_eq!(num(-2.5), "-2.5");
    }
}
