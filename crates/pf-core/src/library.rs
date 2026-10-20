//! Built-in component library and board presets, compiled into the binary.

use crate::CoreError;
use pf_component::Component;
use pf_model::Board;
use std::sync::OnceLock;

/// `(id, TOML source)` of every built-in component.
pub const BUILTIN_COMPONENTS: &[(&str, &str)] = &[(
    "fefco-0201",
    include_str!("../../../library/fefco/0201.pfc.toml"),
)];

static COMPILED: OnceLock<Result<Vec<Component>, CoreError>> = OnceLock::new();

pub(crate) fn all() -> Result<&'static [Component], CoreError> {
    COMPILED
        .get_or_init(|| {
            BUILTIN_COMPONENTS
                .iter()
                .map(|(id, src)| {
                    let c = Component::from_toml(src).map_err(|e| CoreError::Library {
                        id: id.to_string(),
                        message: e.to_string(),
                    })?;
                    if c.id() != *id {
                        return Err(CoreError::Library {
                            id: id.to_string(),
                            message: format!("file declares id '{}'", c.id()),
                        });
                    }
                    Ok(c)
                })
                .collect()
        })
        .as_ref()
        .map(Vec::as_slice)
        .map_err(Clone::clone)
}

pub(crate) fn get(id: &str) -> Result<&'static Component, CoreError> {
    all()?
        .iter()
        .find(|c| c.id() == id)
        .ok_or_else(|| CoreError::UnknownComponent(id.to_string()))
}

/// Board presets with **nominal** calipers. Real values vary by mill; users can supply a
/// custom board with a measured caliper.
pub fn boards() -> Vec<Board> {
    let b = |id: &str, name: &str, caliper: f64, flute: Option<&str>| Board {
        id: id.into(),
        name: name.into(),
        caliper_mm: caliper,
        flute: flute.map(Into::into),
    };
    vec![
        b("E-flute", "Single wall E flute", 1.6, Some("E")),
        b("B-flute", "Single wall B flute", 3.0, Some("B")),
        b("C-flute", "Single wall C flute", 4.0, Some("C")),
        b("EB-double", "Double wall EB", 4.6, Some("EB")),
        b("BC-double", "Double wall BC", 7.0, Some("BC")),
        b("FBB-0.5", "Folding boxboard 0.5 mm", 0.5, None),
    ]
}
