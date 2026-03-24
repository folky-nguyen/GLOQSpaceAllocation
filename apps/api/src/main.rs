use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde::Serialize;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{env, net::SocketAddr};
use tokio::net::TcpListener;
use tracing::warn;

#[derive(Clone)]
struct AppState {
    pool: Option<PgPool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthResponse {
    status: &'static str,
    database: &'static str,
    snapshot_strategy: &'static str,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            env::var("RUST_LOG").unwrap_or_else(|_| "gloq_api=info,tower_http=info".to_owned()),
        )
        .init();

    let state = AppState {
        pool: connect_pool_from_env().await,
    };
    let address = SocketAddr::from(([127, 0, 0, 1], 4000));
    let listener = TcpListener::bind(address).await?;

    println!("gloq-api listening on http://{address}");

    axum::serve(listener, app(state)).await?;
    Ok(())
}

fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "ok",
            database: database_status(&state),
            snapshot_strategy: "versioned_jsonb_snapshots",
        }),
    )
}

fn database_status(state: &AppState) -> &'static str {
    if state.pool.is_some() {
        "connected"
    } else {
        "offline"
    }
}

async fn connect_pool_from_env() -> Option<PgPool> {
    let database_url = match env::var("DATABASE_URL") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => return None,
    };

    match PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(pool) => Some(pool),
        Err(error) => {
            warn!(%error, "postgres connection failed");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{app, AppState};
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use serde_json::Value;
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_route_reports_snapshot_strategy() {
        let response = app(AppState { pool: None })
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["status"], "ok");
        assert_eq!(payload["database"], "offline");
        assert_eq!(payload["snapshotStrategy"], "versioned_jsonb_snapshots");
    }
}
