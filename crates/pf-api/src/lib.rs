//! PackForge HTTP API (`/v1`). Stateless: every response is computed from the request by
//! `pf-core`, so instances scale horizontally and responses can be cached by request hash.

pub mod error;
pub mod keys;
pub mod ratelimit;

use axum::extract::{FromRequest, Path, Request, State};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use error::ApiError;
use keys::{sha256_hex, KeyStore};
use pf_core::{Format, Project};
use ratelimit::{Decision, RateLimiter};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

const OPENAPI: &str = include_str!("openapi.json");

#[derive(Debug, Clone)]
pub struct Settings {
    /// `None` disables authentication (local development only).
    pub keys: Option<KeyStore>,
    /// Origins allowed for browser calls (CORS). Empty = no CORS headers.
    pub cors_origins: Vec<String>,
    /// Maximum request body size in bytes.
    pub max_body_bytes: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            keys: Some(KeyStore::default()),
            cors_origins: Vec::new(),
            max_body_bytes: 256 * 1024,
        }
    }
}

struct AppState {
    keys: Option<KeyStore>,
    limiter: RateLimiter,
}

/// Builds the full application router.
pub fn app(settings: Settings) -> Router {
    let state = Arc::new(AppState {
        keys: settings.keys,
        limiter: RateLimiter::default(),
    });

    let protected = Router::new()
        .route("/v1/components", get(list_components))
        .route("/v1/components/{id}", get(describe))
        .route("/v1/boards", get(boards))
        .route("/v1/render", post(render))
        .route("/v1/export", post(export))
        .route("/v1/nest", post(nest))
        .route("/v1/fold", post(fold))
        .route("/v1/pallet", post(pallet))
        .route("/v1/pallet/report", post(pallet_report))
        .route("/v1/case-optimize", post(case_optimize))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth));

    let mut router = Router::new()
        .route("/v1/health", get(health))
        .route("/v1/openapi.json", get(openapi))
        .merge(protected)
        .fallback(|| async { ApiError::not_found("no such endpoint") })
        .with_state(state)
        .layer(RequestBodyLimitLayer::new(settings.max_body_bytes))
        .layer(TraceLayer::new_for_http());

    if !settings.cors_origins.is_empty() {
        let origins: Vec<HeaderValue> = settings
            .cors_origins
            .iter()
            .filter_map(|o| HeaderValue::from_str(o).ok())
            .collect();
        router = router.layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(origins))
                .allow_methods([Method::GET, Method::POST])
                .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
                .expose_headers([
                    header::CONTENT_DISPOSITION,
                    header::ETAG,
                    HeaderName::from_static("x-ratelimit-remaining"),
                ]),
        );
    }
    router
}

// ── Middleware ────────────────────────────────────────────────────────────────

async fn auth(State(state): State<Arc<AppState>>, req: Request, next: Next) -> Response {
    let Some(store) = &state.keys else {
        return next.run(req).await;
    };
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim);
    let Some(token) = token else {
        return ApiError::unauthorized("send 'Authorization: Bearer <api key>'").into_response();
    };
    let Some(entry) = store.verify(token) else {
        return ApiError::unauthorized("API key not recognised").into_response();
    };
    let (id, limit) = (entry.id.clone(), entry.rate_per_minute);
    match state.limiter.check(&id, limit, Instant::now()) {
        Decision::Limited { retry_after } => ApiError::rate_limited(retry_after).into_response(),
        Decision::Allowed { remaining } => {
            let mut res = next.run(req).await;
            let h = res.headers_mut();
            h.insert("x-ratelimit-limit", HeaderValue::from(limit));
            h.insert("x-ratelimit-remaining", HeaderValue::from(remaining));
            res
        }
    }
}

// ── Extractor ─────────────────────────────────────────────────────────────────

/// JSON body whose rejections become problem+json errors.
#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(ApiError))]
struct ApiJson<T>(T);

// ── Handlers ──────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct Health {
    status: &'static str,
    version: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn openapi() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "application/json")], OPENAPI)
}

async fn list_components() -> Result<Json<Vec<pf_core::ComponentSummary>>, ApiError> {
    Ok(Json(pf_core::list_components()?))
}

async fn describe(Path(id): Path<String>) -> Result<Json<pf_core::ComponentInfo>, ApiError> {
    Ok(Json(pf_core::describe(&id)?))
}

async fn boards() -> Json<Vec<pf_core::Board>> {
    Json(pf_core::boards())
}

#[derive(Deserialize)]
struct RenderRequest {
    project: Project,
}

async fn render(
    ApiJson(req): ApiJson<RenderRequest>,
) -> Result<Json<pf_core::RenderResult>, ApiError> {
    Ok(Json(pf_core::render(&req.project)?))
}

#[derive(Deserialize)]
struct ExportRequest {
    project: Project,
    format: String,
    #[serde(default)]
    options: pf_core::ExportOptions,
}

#[derive(Deserialize)]
struct NestRequest {
    project: Project,
    nesting: pf_core::Nesting,
}

async fn nest(ApiJson(req): ApiJson<NestRequest>) -> Result<Json<pf_core::NestSummary>, ApiError> {
    Ok(Json(pf_core::nest_preview(&req.project, &req.nesting)?))
}

#[derive(Deserialize)]
struct FoldRequest {
    project: Project,
    #[serde(default = "erected")]
    progress: f64,
}

fn erected() -> f64 {
    1.0
}

#[derive(Deserialize)]
struct PalletBody {
    project: Project,
    #[serde(default)]
    pallet: pf_core::PalletRequest,
}

async fn pallet(
    ApiJson(req): ApiJson<PalletBody>,
) -> Result<Json<pf_core::PalletResult>, ApiError> {
    Ok(Json(pf_core::pallet_plan(&req.project, &req.pallet)?))
}

async fn pallet_report(ApiJson(req): ApiJson<PalletBody>) -> Result<Response, ApiError> {
    let pdf = pf_core::pallet_report(&req.project, &req.pallet)?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!(
                    "attachment; filename=\"{}-pallet.pdf\"",
                    req.project.component_id
                ),
            ),
        ],
        pdf,
    )
        .into_response())
}

async fn case_optimize(
    ApiJson(req): ApiJson<pf_core::CaseRequest>,
) -> Result<Json<Vec<pf_core::CaseCandidate>>, ApiError> {
    Ok(Json(pf_core::case_optimize(&req)?))
}

async fn fold(ApiJson(req): ApiJson<FoldRequest>) -> Result<Json<pf_core::Folded>, ApiError> {
    Ok(Json(pf_core::fold(&req.project, req.progress)?))
}

async fn export(
    headers: HeaderMap,
    ApiJson(req): ApiJson<ExportRequest>,
) -> Result<Response, ApiError> {
    let format = Format::parse(&req.format).ok_or_else(|| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "unknown_format",
            "Unknown export format",
            format!(
                "unsupported format '{}' (use svg, dxf, pdf, cf2, hpgl or glb)",
                req.format
            ),
        )
    })?;
    let file = pf_core::export_with(&req.project, format, &req.options)?;
    // Exports are deterministic, so the content hash is a strong ETag.
    let etag = format!("\"{}\"", &sha256_hex(&file.bytes)[..32]);
    if headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.split(',').any(|t| t.trim() == etag))
    {
        return Ok((StatusCode::NOT_MODIFIED, [(header::ETAG, etag)]).into_response());
    }
    let disposition = format!("attachment; filename=\"{}\"", file.filename);
    Ok((
        [
            (header::CONTENT_TYPE, file.mime.to_string()),
            (header::CONTENT_DISPOSITION, disposition),
            (header::ETAG, etag),
            (
                header::CACHE_CONTROL,
                "private, max-age=0, must-revalidate".to_string(),
            ),
        ],
        file.bytes,
    )
        .into_response())
}
