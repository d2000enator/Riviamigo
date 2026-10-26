//! Exercise the real production router without a database or Redis server.
//! Missing authentication must reject before either lazy client is used.
use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{
        header::{CACHE_CONTROL, CONTENT_LENGTH, VARY},
        Method, Request, StatusCode,
    },
    Router,
};
use jsonwebtoken::{DecodingKey, EncodingKey};
use riviamigo_api::{
    config::{Config, OriginBindConfig, RateLimitConfig, RecoveryConfig},
    ingestion::supervisor::SupervisorHandle,
    middleware::auth::{AppState, JwtKeys},
    routes,
};
use sqlx::postgres::PgPoolOptions;
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tower::ServiceExt;

fn app() -> Router {
    routes::build_router(AppState {
        pool: PgPoolOptions::new()
            .connect_lazy("postgresql://unused:unused@127.0.0.1:9/unused")
            .unwrap(),
        redis: redis::Client::open("redis://127.0.0.1:9/").unwrap(),
        // This test sends no tokens; no token is signed or verified.
        jwt_keys: Arc::new(JwtKeys {
            encoding: EncodingKey::from_secret(b"unused-test-only"),
            decoding: DecodingKey::from_secret(b"unused-test-only"),
        }),
        age_key: String::new(),
        config: Config {
            database_url: "postgresql://unused:unused@127.0.0.1:9/unused".into(),
            redis_url: "redis://127.0.0.1:6379/".into(),
            jwt_secret: None,
            jwt_public_key: None,
            age_encryption_key: None,
            port: 0,
            allowed_origins: vec!["http://localhost:3000".into()],
            s3_endpoint: None,
            s3_access_key: None,
            s3_secret_key: None,
            backup_artifact_dir: std::env::temp_dir()
                .join("riviamigo-auth-test-backups")
                .to_string_lossy()
                .into_owned(),
            backup_driver: "pg_dump".into(),
            backup_poll_interval_seconds: 60,
            restore_agent_url: "http://127.0.0.1:3002".into(),
            restore_agent_key_file: "/backups/.restore-agent-key".into(),
            recovery: RecoveryConfig::default(),
            origin_bind: OriginBindConfig::default(),
            rivian_ws_reconnect_initial_seconds: 10,
            rivian_ws_reconnect_max_seconds: 900,
            rivian_raw_event_retention_days: 7,
            rivian_persist_raw_events: true,
            rivian_suppress_duplicate_telemetry: true,
            riviamigo_env: None,
            cookie_insecure: None,
            allow_insecure_lan_http_auth: false,
            rate_limit: RateLimitConfig::default(),
            vehicle_image_cache_dir: std::env::temp_dir()
                .join("riviamigo-auth-test-images")
                .to_string_lossy()
                .into_owned(),
        },
        nominatim_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        supervisor: SupervisorHandle::noop(),
    })
}

async fn request(router: Router, method: Method, path: &str, bytes: usize, expected: StatusCode) {
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header(CONTENT_LENGTH, bytes)
        .extension(ConnectInfo(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            12345,
        )))
        .body(Body::from(vec![b'x'; bytes]))
        .unwrap();
    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), expected, "{path}");
    assert_eq!(
        response.headers().get(CACHE_CONTROL).unwrap(),
        "private, no-store",
        "{path}"
    );
    let vary = response
        .headers()
        .get_all(VARY)
        .iter()
        .map(|v| v.to_str().unwrap())
        .collect::<Vec<_>>()
        .join(",")
        .to_ascii_lowercase();
    assert!(
        vary.contains("authorization") && vary.contains("cookie"),
        "{path}: {vary}"
    );
}

#[tokio::test]
async fn assistant_routes_reject_unsigned_requests_without_cacheable_responses() {
    let router = app();
    let vehicle = "00000000-0000-0000-0000-000000000001";
    let object = "00000000-0000-0000-0000-000000000002";
    for (method, suffix) in [
        (Method::GET, String::new()),
        (Method::PUT, String::new()),
        (Method::GET, "/weekly".into()),
        (Method::POST, "/preview".into()),
        (Method::POST, format!("/alerts/{object}/acknowledge")),
        (Method::GET, format!("/receipts/{object}")),
        (Method::POST, format!("/maintenance/{object}/receipts")),
    ] {
        request(
            router.clone(),
            method,
            &format!("/v1/vehicles/{vehicle}/assistant{suffix}"),
            0,
            StatusCode::UNAUTHORIZED,
        )
        .await;
    }
}

#[tokio::test]
async fn oversized_assistant_upload_rejection_is_also_private() {
    request(app(),Method::POST,"/v1/vehicles/00000000-0000-0000-0000-000000000001/assistant/maintenance/00000000-0000-0000-0000-000000000002/receipts",8*1024*1024,StatusCode::PAYLOAD_TOO_LARGE).await;
}
