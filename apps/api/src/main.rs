mod auth;
mod config;
mod error;

use crate::{
    auth::{require_bearer_auth, AuthContext, AuthVerifier},
    config::AppConfig,
    error::ApiError,
};
use axum::{
    extract::{Extension, State},
    http::{HeaderValue, Method},
    middleware,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

const DEFAULT_LOG_FILTER: &str = "gloq_api=info,tower_http=info";
const LOCAL_WEB_ORIGINS: [&str; 4] = [
    "http://localhost:5173",
    "http://127.0.0.1:5173",
    "http://localhost:3001",
    "http://127.0.0.1:3001",
];

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    auth: AuthVerifier,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize)]
struct VersionResponse {
    name: &'static str,
    version: &'static str,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::from_env()?;
    init_tracing();

    let state = AppState {
        pool: connect_pool(&config.database_url).await?,
        auth: AuthVerifier::new(config.supabase_issuer(), config.supabase_jwks_url()),
    };
    let listener = TcpListener::bind((config.host.as_str(), config.port)).await?;
    let address = listener.local_addr()?;

    info!(address = %address, "gloq-api listening");

    axum::serve(listener, app(state)).await?;
    Ok(())
}

fn app(state: AppState) -> Router {
    let auth = state.auth.clone();

    Router::new()
        .nest("/api", api_router(auth))
        .layer(build_cors_layer())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

fn api_router(auth: AuthVerifier) -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/version", get(version))
        .route(
            "/me",
            get(me).route_layer(middleware::from_fn_with_state(auth, require_bearer_auth)),
        )
        .fallback(api_not_found)
}

fn build_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_methods([Method::GET, Method::OPTIONS])
        .allow_origin(LOCAL_WEB_ORIGINS.map(HeaderValue::from_static))
}

fn init_tracing() {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

async fn health(State(state): State<AppState>) -> Result<Json<HealthResponse>, ApiError> {
    if state.pool.is_closed() {
        return Err(ApiError::internal("Database pool is closed."));
    }

    Ok(Json(HealthResponse { status: "ok" }))
}

async fn version() -> Json<VersionResponse> {
    Json(VersionResponse {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn me(Extension(auth): Extension<AuthContext>) -> Json<AuthContext> {
    Json(auth)
}

async fn connect_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    info!("postgres pool connected");
    Ok(pool)
}

async fn api_not_found() -> ApiError {
    ApiError::not_found("Route not found.")
}

#[cfg(test)]
mod tests {
    use super::{app, AppState};
    use crate::auth::AuthVerifier;
    use axum::{
        body::{to_bytes, Body},
        extract::State,
        http::{
            header::{ACCESS_CONTROL_ALLOW_ORIGIN, AUTHORIZATION, ORIGIN},
            HeaderValue, Request, StatusCode,
        },
        routing::get,
        Json, Router,
    };
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use serde_json::{json, Value};
    use sqlx::postgres::PgPoolOptions;
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, RwLock,
        },
        time::{SystemTime, UNIX_EPOCH},
    };
    use tokio::{net::TcpListener, task::JoinHandle};
    use tower::{Service, ServiceExt};

    const TEST_PRIVATE_KEY: &str = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEAiDs2aMfJsOK86s+9ydNTpddBvXLpHlYodUliHPRjbepAHUbd\npYuCw+y05JdiuWYJM+/efpDdQ7v3S0EX+NTEzalVzfLZV4Y1CSiw854fcmb5OAxl\nOr68CRI/5EJcMs0pBzLMUOD0updEtf2fGUxPes7FXDPJcVcvv1cZ3XlpP8vV1AhZ\nW9Y/P4Mg9dfvl/PiE+JiSQ8EKbyT2ooEgni4+AF2NgmxqR4FIuLeoQ+7fBsK5rxg\nahD+tfmbAQ0CJaGoC0Lirdso7D+ztri7gZRKvbMywyVhuMWqK/SuJSa/fPV3DvR4\n6bccrIiiYHRryBJAAMipL2UXHFmNHngoqrW4swIDAQABAoIBAChEdFcxYr0NsDCp\n+hvLgsic4Vopup1Ucz4D1GPhmvg0ywG8XiyeGadm8qs00iIh9mtrJfV8RWLNjxGn\n6nmLDqcJvAqVk0erLEcKR4+i+AGdTWITS+K62SLHSapjMRR1DwEJM1pevAfhSZaP\nonTcWQvgTXbs+ciuMDda/BK6XfFUlO2olR3xi/VeFvxavWn3+IlJkUA2ctRPaYdG\n5DiMpU/YzGKDI9HEmbHOtRxm1JVRVGDYbd26Xp/DX3SeqpMqZOko9h8QANZO/ZSn\nfe1wW31nRGr7DE8rQqWEU9FH9g68W/qbmaqV1FeqOsuvtu+4i2dIGmIi4lGjEy/A\nOt6IYwUCgYEAv8wAlzJtUcD52Zbc5fP+G8avTDZrUZsc7cDH+fp4jB2cOSJGKu6r\nZP49uPMaw8YX+W6bxqBRWiHCAhEAF+o4/0EWcjiFQmx8kCQEas/kzzmf4Js5ov9f\nZkoaX9N2qasXS2FgDriCiqN19ZX6doU0JRc3aRaIksto2zUFT3oAtccCgYEAtdWH\nJnPG2Z6fUTXl3Pu2CzdEdQwih9rn2/vEVYJM0ACorFN2DcowL6mUOeCtjxp4G1vL\n2tn7AFZsKjoI3lcsr/VUboCNvjA1X5ag2sgU5cbQ3sUcUGFaB74NiYWsUHyJrrOU\nFK/T5RtZRzZzqMqx0azvf+kca4R0bxWg4dzVNbUCgYBAqjdIwue4uKeEhSjVHv59\nvu87ct2cFgAa6PSDg79A/nq9iKC/uNhwpIeK4+wSNae/oVtEDKlhCiCvMawmZAHz\nja5TtFq5mnok3v/eQ1mRxIvy3mMAYbl4c2ORC2rmqZihAaOxUuQwegw7UOWxMBf0\nqW81LzO8ynf/8FBqC2hR4QKBgQCqmlH4mO38JmCSQICPqrctpMgdDaqkTpX2By05\nkUxiaAvZy2DbJVW6kl/ZQd11g78m5CTLDHP86BkKMXM6sQ3jdcmm+ASFahPZwKjh\nPJKm17gHG2cqX3yqAP4QhpOa3I4NlL2d/y5PKi7EqukveCYIdTosh9m7YwYfZ2qQ\nH7MHdQKBgQCOLc2mHsKfvxc7yYYfdBU/My5U9FSaxVvJvSdZFmgxNn9CfS5w4Two\nkPYlqahvM8AP449WHclHZ3X7chWSS4MsqbQG7/sNSkcUa+C9n6HFhKOSUWZiqdTr\nBqkYz3sgxFX/8LwSPF0nqZZR8H8oa8FkyJKYU/aw3nWu6wgMY427XQ==\n-----END RSA PRIVATE KEY-----\n";
    const TEST_PUBLIC_KEY_MODULUS: &str = "iDs2aMfJsOK86s-9ydNTpddBvXLpHlYodUliHPRjbepAHUbdpYuCw-y05JdiuWYJM-_efpDdQ7v3S0EX-NTEzalVzfLZV4Y1CSiw854fcmb5OAxlOr68CRI_5EJcMs0pBzLMUOD0updEtf2fGUxPes7FXDPJcVcvv1cZ3XlpP8vV1AhZW9Y_P4Mg9dfvl_PiE-JiSQ8EKbyT2ooEgni4-AF2NgmxqR4FIuLeoQ-7fBsK5rxgahD-tfmbAQ0CJaGoC0Lirdso7D-ztri7gZRKvbMywyVhuMWqK_SuJSa_fPV3DvR46bccrIiiYHRryBJAAMipL2UXHFmNHngoqrW4sw";
    const TEST_PUBLIC_KEY_EXPONENT: &str = "AQAB";
    const PRIMARY_KID: &str = "primary-kid";
    const ROTATED_KID: &str = "rotated-kid";
    const TEST_USER_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
    const TEST_EMAIL: &str = "user@example.com";

    #[derive(Clone)]
    struct JwksServerState {
        body: Arc<RwLock<Value>>,
        hits: Arc<AtomicUsize>,
    }

    struct TestJwksServer {
        base_url: String,
        body: Arc<RwLock<Value>>,
        hits: Arc<AtomicUsize>,
        task: JoinHandle<()>,
    }

    impl Drop for TestJwksServer {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    impl TestJwksServer {
        async fn spawn(initial_kid: &str) -> Self {
            let state = JwksServerState {
                body: Arc::new(RwLock::new(build_jwks(initial_kid))),
                hits: Arc::new(AtomicUsize::new(0)),
            };
            let app = Router::new()
                .route("/auth/v1/.well-known/jwks.json", get(jwks))
                .with_state(state.clone());
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base_url = format!("http://{}", listener.local_addr().unwrap());
            let task = tokio::spawn(async move {
                axum::serve(listener, app).await.unwrap();
            });

            Self {
                base_url,
                body: state.body,
                hits: state.hits,
                task,
            }
        }

        fn set_active_kid(&self, kid: &str) {
            let mut body = self.body.write().unwrap();
            *body = build_jwks(kid);
        }

        fn hits(&self) -> usize {
            self.hits.load(Ordering::SeqCst)
        }
    }

    fn test_state(base_url: &str) -> AppState {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@127.0.0.1/postgres")
            .unwrap();
        let auth = AuthVerifier::new(
            format!("{base_url}/auth/v1"),
            format!("{base_url}/auth/v1/.well-known/jwks.json"),
        );

        AppState { pool, auth }
    }

    async fn jwks(State(state): State<JwksServerState>) -> Json<Value> {
        state.hits.fetch_add(1, Ordering::SeqCst);

        Json(state.body.read().unwrap().clone())
    }

    fn build_jwks(kid: &str) -> Value {
        json!({
            "keys": [{
                "kty": "RSA",
                "alg": "RS256",
                "use": "sig",
                "kid": kid,
                "n": TEST_PUBLIC_KEY_MODULUS,
                "e": TEST_PUBLIC_KEY_EXPONENT
            }]
        })
    }

    fn issue_token(base_url: &str, kid: &str) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(kid.to_owned());

        let expiration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 3600;
        let claims = json!({
            "aud": "authenticated",
            "email": TEST_EMAIL,
            "exp": expiration,
            "iss": format!("{base_url}/auth/v1"),
            "role": "authenticated",
            "sub": TEST_USER_ID
        });

        encode(
            &header,
            &claims,
            &EncodingKey::from_rsa_pem(TEST_PRIVATE_KEY.as_bytes()).unwrap(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn health_route_returns_ok_status() {
        let jwks_server = TestJwksServer::spawn(PRIMARY_KID).await;
        let response = app(test_state(&jwks_server.base_url))
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["status"], "ok");
    }

    #[tokio::test]
    async fn version_route_returns_package_metadata() {
        let jwks_server = TestJwksServer::spawn(PRIMARY_KID).await;
        let response = app(test_state(&jwks_server.base_url))
            .oneshot(
                Request::builder()
                    .uri("/api/version")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["name"], env!("CARGO_PKG_NAME"));
        assert_eq!(payload["version"], env!("CARGO_PKG_VERSION"));
    }

    #[tokio::test]
    async fn api_fallback_returns_json_not_found() {
        let jwks_server = TestJwksServer::spawn(PRIMARY_KID).await;
        let response = app(test_state(&jwks_server.base_url))
            .oneshot(
                Request::builder()
                    .uri("/api/does-not-exist")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["error"]["code"], "not_found");
        assert_eq!(payload["error"]["message"], "Route not found.");
    }

    #[tokio::test]
    async fn cors_allows_local_web_origin() {
        let jwks_server = TestJwksServer::spawn(PRIMARY_KID).await;
        let response = app(test_state(&jwks_server.base_url))
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .header(ORIGIN, "http://localhost:5173")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
            Some(&HeaderValue::from_static("http://localhost:5173"))
        );
    }

    #[tokio::test]
    async fn me_route_rejects_missing_auth_header() {
        let jwks_server = TestJwksServer::spawn(PRIMARY_KID).await;
        let response = app(test_state(&jwks_server.base_url))
            .oneshot(
                Request::builder()
                    .uri("/api/me")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["error"]["code"], "unauthorized");
    }

    #[tokio::test]
    async fn me_route_returns_authenticated_user_context() {
        let jwks_server = TestJwksServer::spawn(PRIMARY_KID).await;
        let token = issue_token(&jwks_server.base_url, PRIMARY_KID);

        let response = app(test_state(&jwks_server.base_url))
            .oneshot(
                Request::builder()
                    .uri("/api/me")
                    .header(AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["user_id"], TEST_USER_ID);
        assert_eq!(payload["email"], TEST_EMAIL);
        assert_eq!(payload["role"], "authenticated");
        assert_eq!(jwks_server.hits(), 1);
    }

    #[tokio::test]
    async fn me_route_refreshes_jwks_when_kid_changes() {
        let jwks_server = TestJwksServer::spawn(PRIMARY_KID).await;
        let mut router = app(test_state(&jwks_server.base_url));

        let first_token = issue_token(&jwks_server.base_url, PRIMARY_KID);
        let first_response = Service::call(
            &mut router,
            Request::builder()
                .uri("/api/me")
                .header(AUTHORIZATION, format!("Bearer {first_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(first_response.status(), StatusCode::OK);
        assert_eq!(jwks_server.hits(), 1);

        jwks_server.set_active_kid(ROTATED_KID);
        let second_token = issue_token(&jwks_server.base_url, ROTATED_KID);
        let second_response = Service::call(
            &mut router,
            Request::builder()
                .uri("/api/me")
                .header(AUTHORIZATION, format!("Bearer {second_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(second_response.status(), StatusCode::OK);
        assert_eq!(jwks_server.hits(), 2);
    }
}
