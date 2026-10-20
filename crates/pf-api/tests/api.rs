use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use pf_api::keys::{sha256_hex, KeyEntry, KeyStore};
use pf_api::{app, Settings};
use serde_json::{json, Value};
use tower::ServiceExt;

const KEY: &str = "pk_test_secret";

fn server(rate_per_minute: u32) -> Router {
    let keys = KeyStore::from_entries(vec![KeyEntry {
        id: "test".into(),
        hash: format!("sha256:{}", sha256_hex(KEY.as_bytes())),
        plan: "enterprise".into(),
        rate_per_minute,
    }])
    .expect("keys");
    app(Settings {
        keys: Some(keys),
        cors_origins: vec!["https://shop.example".into()],
        max_body_bytes: 64 * 1024,
    })
}

struct Res {
    status: StatusCode,
    headers: header::HeaderMap,
    body: Vec<u8>,
}

impl Res {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body)
            .unwrap_or_else(|_| panic!("not JSON: {}", String::from_utf8_lossy(&self.body)))
    }
    fn header(&self, name: &str) -> &str {
        self.headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
    }
}

async fn send(app: &Router, req: Request<Body>) -> Res {
    let res = app.clone().oneshot(req).await.expect("response");
    let status = res.status();
    let headers = res.headers().clone();
    let body = res
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes()
        .to_vec();
    Res {
        status,
        headers,
        body,
    }
}

fn get(path: &str, key: Option<&str>) -> Request<Body> {
    let mut b = Request::get(path);
    if let Some(k) = key {
        b = b.header(header::AUTHORIZATION, format!("Bearer {k}"));
    }
    b.body(Body::empty()).expect("request")
}

fn post(path: &str, body: Value) -> Request<Body> {
    Request::post(path)
        .header(header::AUTHORIZATION, format!("Bearer {KEY}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request")
}

fn project() -> Value {
    json!({ "component_id": "fefco-0201", "params": { "L": 300, "W": 200, "H": 150 }, "board": "C-flute" })
}

#[tokio::test]
async fn health_and_openapi_are_public() {
    let app = server(100);
    let r = send(&app, get("/v1/health", None)).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json()["status"], "ok");

    let r = send(&app, get("/v1/openapi.json", None)).await;
    assert_eq!(r.status, StatusCode::OK);
    let doc = r.json();
    assert_eq!(doc["openapi"], "3.1.0");
    // Every documented path is routed (none fall through to the 404 fallback).
    for path in doc["paths"].as_object().expect("paths").keys() {
        let concrete = path.replace("{id}", "fefco-0201");
        let r = send(&app, get(&concrete, Some(KEY))).await;
        assert_ne!(
            r.status,
            StatusCode::NOT_FOUND,
            "{path} is documented but not routed"
        );
    }
}

#[tokio::test]
async fn protected_routes_need_a_valid_key() {
    let app = server(100);
    let r = send(&app, get("/v1/components", None)).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    assert_eq!(r.header("www-authenticate"), "Bearer");
    assert_eq!(r.header("content-type"), "application/problem+json");
    assert_eq!(r.json()["code"], "unauthorized");

    let r = send(&app, get("/v1/components", Some("pk_test_wrong"))).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);

    let r = send(&app, get("/v1/components", Some(KEY))).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r
        .json()
        .as_array()
        .expect("array")
        .iter()
        .any(|c| c["id"] == "fefco-0201"));
    assert!(!r.header("x-ratelimit-remaining").is_empty());
}

#[tokio::test]
async fn describe_and_boards() {
    let app = server(100);
    let r = send(&app, get("/v1/components/fefco-0201", Some(KEY))).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json()["params"][0]["id"], "L");

    let r = send(&app, get("/v1/components/nope", Some(KEY))).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(r.json()["code"], "unknown_component");

    let r = send(&app, get("/v1/boards", Some(KEY))).await;
    assert!(r
        .json()
        .as_array()
        .expect("array")
        .iter()
        .any(|b| b["id"] == "C-flute"));
}

#[tokio::test]
async fn render_returns_stats_and_validation_errors() {
    let app = server(100);
    let r = send(&app, post("/v1/render", json!({ "project": project() }))).await;
    assert_eq!(r.status, StatusCode::OK);
    let body = r.json();
    assert_eq!(body["stats"]["blank_width"], 1051.0);
    assert_eq!(body["stats"]["blank_height"], 362.0);

    let mut bad = project();
    bad["params"]["L"] = json!(5);
    bad["params"]["glue_side"] = json!("top");
    let r = send(&app, post("/v1/render", json!({ "project": bad }))).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    let p = r.json();
    assert_eq!(p["code"], "invalid_params");
    let params: Vec<_> = p["errors"]
        .as_array()
        .expect("errors")
        .iter()
        .map(|e| e["param"].as_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(params, ["L", "glue_side"]);

    let mut bad = project();
    bad["board"] = json!("Z-flute");
    let r = send(&app, post("/v1/render", json!({ "project": bad }))).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.json()["code"], "unknown_board");
}

#[tokio::test]
async fn export_sets_file_headers_and_supports_etags() {
    let app = server(100);
    let r = send(
        &app,
        post(
            "/v1/export",
            json!({ "project": project(), "format": "dxf" }),
        ),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.header("content-type"), "application/dxf");
    assert_eq!(
        r.header("content-disposition"),
        "attachment; filename=\"fefco-0201.dxf\""
    );
    assert!(String::from_utf8_lossy(&r.body).ends_with("EOF\n"));
    let etag = r.header("etag").to_string();
    assert!(etag.starts_with('"') && etag.len() == 34, "{etag}");

    let again = Request::post("/v1/export")
        .header(header::AUTHORIZATION, format!("Bearer {KEY}"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_NONE_MATCH, &etag)
        .body(Body::from(
            json!({ "project": project(), "format": "dxf" }).to_string(),
        ))
        .expect("request");
    let r = send(&app, again).await;
    assert_eq!(r.status, StatusCode::NOT_MODIFIED);
    assert!(r.body.is_empty());

    let r = send(
        &app,
        post(
            "/v1/export",
            json!({ "project": project(), "format": "svg" }),
        ),
    )
    .await;
    assert_eq!(r.header("content-type"), "image/svg+xml");
    assert!(String::from_utf8_lossy(&r.body).starts_with("<svg"));

    let r = send(
        &app,
        post(
            "/v1/export",
            json!({ "project": project(), "format": "pdf" }),
        ),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.json()["code"], "unknown_format");
}

#[tokio::test]
async fn bad_bodies_are_problem_json() {
    let app = server(100);
    let malformed = Request::post("/v1/render")
        .header(header::AUTHORIZATION, format!("Bearer {KEY}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{not json"))
        .expect("request");
    let r = send(&app, malformed).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.json()["code"], "invalid_request");

    let no_type = Request::post("/v1/render")
        .header(header::AUTHORIZATION, format!("Bearer {KEY}"))
        .body(Body::from(json!({ "project": project() }).to_string()))
        .expect("request");
    let r = send(&app, no_type).await;
    assert_eq!(r.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);

    let huge = json!({ "project": project(), "padding": "x".repeat(100 * 1024) });
    let r = send(&app, post("/v1/render", huge)).await;
    assert_eq!(r.status, StatusCode::PAYLOAD_TOO_LARGE);

    let r = send(&app, get("/v2/anything", Some(KEY))).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(r.header("content-type"), "application/problem+json");
}

#[tokio::test]
async fn rate_limit_returns_429_with_retry_after() {
    let app = server(2);
    for _ in 0..2 {
        assert_eq!(
            send(&app, get("/v1/boards", Some(KEY))).await.status,
            StatusCode::OK
        );
    }
    let r = send(&app, get("/v1/boards", Some(KEY))).await;
    assert_eq!(r.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(r.json()["code"], "rate_limited");
    let retry: u64 = r
        .header("retry-after")
        .parse()
        .expect("retry-after seconds");
    assert!((1..=30).contains(&retry), "{retry}");
}

#[tokio::test]
async fn cors_allows_configured_origins_only() {
    let app = server(100);
    let preflight = |origin: &str| {
        Request::builder()
            .method("OPTIONS")
            .uri("/v1/render")
            .header(header::ORIGIN, origin)
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
            .header(
                header::ACCESS_CONTROL_REQUEST_HEADERS,
                "authorization,content-type",
            )
            .body(Body::empty())
            .expect("request")
    };
    let ok = send(&app, preflight("https://shop.example")).await;
    assert_eq!(
        ok.header("access-control-allow-origin"),
        "https://shop.example"
    );
    let other = send(&app, preflight("https://evil.example")).await;
    assert_eq!(other.header("access-control-allow-origin"), "");
}

#[tokio::test]
async fn dev_mode_skips_auth() {
    let app = app(Settings {
        keys: None,
        ..Settings::default()
    });
    let r = send(&app, get("/v1/components", None)).await;
    assert_eq!(r.status, StatusCode::OK);
}
