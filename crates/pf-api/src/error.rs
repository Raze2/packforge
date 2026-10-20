//! RFC 7807 `application/problem+json` errors.

use axum::extract::rejection::JsonRejection;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use pf_core::{CoreError, RenderError};
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FieldError {
    pub param: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub title: &'static str,
    pub detail: String,
    pub errors: Vec<FieldError>,
    pub retry_after: Option<Duration>,
}

impl ApiError {
    pub fn new(
        status: StatusCode,
        code: &'static str,
        title: &'static str,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            status,
            code,
            title,
            detail: detail.into(),
            errors: Vec::new(),
            retry_after: None,
        }
    }

    pub fn unauthorized(detail: impl Into<String>) -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "Missing or invalid API key",
            detail,
        )
    }

    pub fn rate_limited(retry_after: Duration) -> Self {
        Self {
            retry_after: Some(retry_after),
            ..Self::new(
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "Too many requests",
                "rate limit exceeded for this API key",
            )
        }
    }

    pub fn not_found(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", "Not found", detail)
    }
}

#[derive(Serialize)]
struct Problem<'a> {
    #[serde(rename = "type")]
    kind: String,
    title: &'a str,
    status: u16,
    detail: &'a str,
    code: &'a str,
    #[serde(skip_serializing_if = "<[FieldError]>::is_empty")]
    errors: &'a [FieldError],
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Problem {
            kind: format!(
                "https://packforge.dev/problems/{}",
                self.code.replace('_', "-")
            ),
            title: self.title,
            status: self.status.as_u16(),
            detail: &self.detail,
            code: self.code,
            errors: &self.errors,
        };
        let json = serde_json::to_vec(&body).unwrap_or_default();
        let mut res = (self.status, json).into_response();
        let h = res.headers_mut();
        h.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );
        if self.status == StatusCode::UNAUTHORIZED {
            h.insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
        }
        if let Some(d) = self.retry_after {
            let secs = d.as_secs() + u64::from(d.subsec_nanos() > 0);
            if let Ok(v) = HeaderValue::from_str(&secs.max(1).to_string()) {
                h.insert(header::RETRY_AFTER, v);
            }
        }
        res
    }
}

impl From<CoreError> for ApiError {
    fn from(e: CoreError) -> Self {
        let detail = e.to_string();
        match e {
            CoreError::UnknownComponent(_) => Self::new(
                StatusCode::NOT_FOUND,
                "unknown_component",
                "Unknown component",
                detail,
            ),
            CoreError::UnknownBoard(_) => Self::new(
                StatusCode::BAD_REQUEST,
                "unknown_board",
                "Unknown board preset",
                detail,
            ),
            CoreError::Render(RenderError::Params(issues)) => Self {
                errors: issues
                    .into_iter()
                    .map(|i| FieldError {
                        param: i.param,
                        message: i.message,
                    })
                    .collect(),
                ..Self::new(
                    StatusCode::BAD_REQUEST,
                    "invalid_params",
                    "Invalid parameters",
                    detail,
                )
            },
            CoreError::Render(RenderError::Rule(_)) => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "rule_violation",
                "Design rule violated",
                detail,
            ),
            CoreError::Render(RenderError::Eval { .. } | RenderError::Geometry(_)) => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "geometry_error",
                "Design cannot be generated",
                detail,
            ),
            CoreError::Export(_) => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "export_error",
                "Export options cannot be applied",
                detail,
            ),
            CoreError::Fold(_) => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "fold_error",
                "Design cannot be folded",
                detail,
            ),
            CoreError::Library { .. } => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "library_error",
                "Component library error",
                detail,
            ),
        }
    }
}

impl From<JsonRejection> for ApiError {
    fn from(r: JsonRejection) -> Self {
        Self::new(
            r.status(),
            "invalid_request",
            "Invalid request body",
            r.body_text(),
        )
    }
}
