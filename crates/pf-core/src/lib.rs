//! `pf-core` — the stable API every PackForge shell calls (WASM, Tauri, HTTP, CLI).
//!
//! Pure: no filesystem, network, clock or randomness. The built-in library is compiled into
//! the binary so the same code runs in browsers and servers.

mod export;
mod library;

pub use export::{export, export_with, nest_preview, ExportOptions, NestSummary};

pub use library::{boards, BUILTIN_COMPONENTS};
pub use pf_component::{ComponentInfo, ParamDef, ParamIssue, ParamType, RenderError, Tier};
pub use pf_export::{DxfOptions, Format, PageSize, Profile, SvgOptions};
pub use pf_fold3d::{FoldError, Folded, Panel3, Vec3};
pub use pf_model::{
    Board, BoardRef, Diagnostic, DieLine, FoldModel, LineType, Project, RenderResult, Severity,
    Stats, Units, Value,
};
pub use pf_optimize::Nesting;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CoreError {
    #[error("unknown component '{0}'")]
    UnknownComponent(String),
    #[error("unknown board preset '{0}'")]
    UnknownBoard(String),
    #[error("{0}")]
    Render(#[from] pf_component::RenderError),
    #[error("built-in component '{id}' is invalid: {message}")]
    Library { id: String, message: String },
    #[error("cannot fold this design: {0}")]
    Fold(#[from] FoldError),
    #[error("{0}")]
    Export(String),
}

impl CoreError {
    /// Stable machine-readable code (used by the HTTP API and JS bindings).
    pub fn code(&self) -> &'static str {
        match self {
            CoreError::UnknownComponent(_) => "unknown_component",
            CoreError::UnknownBoard(_) => "unknown_board",
            CoreError::Render(pf_component::RenderError::Params(_)) => "invalid_params",
            CoreError::Render(pf_component::RenderError::Rule(_)) => "rule_violation",
            CoreError::Render(pf_component::RenderError::Eval { .. }) => "evaluation_error",
            CoreError::Render(pf_component::RenderError::Geometry(_)) => "geometry_error",
            CoreError::Library { .. } => "library_error",
            CoreError::Fold(_) => "fold_error",
            CoreError::Export(_) => "export_error",
        }
    }
}

/// Catalogue entry.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComponentSummary {
    pub id: String,
    pub version: String,
    pub name: String,
    pub standard: Option<String>,
    pub category: String,
    pub tier: Tier,
}

/// Exported file.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportFile {
    pub bytes: Vec<u8>,
    pub mime: &'static str,
    pub filename: String,
}

/// Lists the built-in components, sorted by id.
pub fn list_components() -> Result<Vec<ComponentSummary>, CoreError> {
    let mut out = Vec::new();
    for c in library::all()? {
        out.push(ComponentSummary {
            id: c.meta.id.clone(),
            version: c.meta.version.clone(),
            name: c.meta.name.clone(),
            standard: c.meta.standard.clone(),
            category: c.meta.category.clone(),
            tier: c.meta.tier,
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// Parameter schema for a component (for building forms).
pub fn describe(component_id: &str) -> Result<ComponentInfo, CoreError> {
    Ok(library::get(component_id)?.info())
}

pub fn resolve_board(board: &BoardRef) -> Result<Board, CoreError> {
    match board {
        BoardRef::Custom(b) => Ok(b.clone()),
        BoardRef::Preset(id) => boards()
            .into_iter()
            .find(|b| b.id == *id)
            .ok_or_else(|| CoreError::UnknownBoard(id.clone())),
    }
}

/// Renders a project to die-line, fold model and stats.
pub fn render(project: &Project) -> Result<RenderResult, CoreError> {
    let component = library::get(&project.component_id)?;
    let board = resolve_board(&project.board)?;
    let rendered = component.render(&project.params, &board)?;
    let mut diagnostics = rendered.diagnostics;
    if let Some(v) = &project.component_version {
        if *v != component.meta.version {
            diagnostics.push(Diagnostic {
                severity: Severity::Warning,
                message: format!(
                    "project was saved with version {v}; rendered with {}",
                    component.meta.version
                ),
            });
        }
    }
    let stats = rendered.dieline.stats();
    Ok(RenderResult {
        component_id: component.meta.id.clone(),
        component_version: component.meta.version.clone(),
        board,
        dieline: rendered.dieline,
        fold: rendered.fold,
        stats,
        diagnostics,
    })
}

/// Folds a project's design to `progress` (0 = flat die-line, 1 = erected box) for 3D display.
pub fn fold(project: &Project, progress: f64) -> Result<Folded, CoreError> {
    let r = render(project)?;
    Ok(pf_fold3d::fold(&r.fold, progress)?)
}

/// Display colour of a board: kraft for corrugated (fluted), off-white for boxboard.
pub fn board_colour(board: &Board) -> [f32; 4] {
    if board.flute.is_some() {
        [0.78, 0.62, 0.42, 1.0]
    } else {
        [0.95, 0.94, 0.90, 1.0]
    }
}
