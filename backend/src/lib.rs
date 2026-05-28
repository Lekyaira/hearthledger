use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pool: SqlitePool,
}

impl AppState {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, Serialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub enum QuantityType {
    Count,
    Grams,
    Ounces,
    Pounds,
    Liters,
    Milliliters,
    Gallons,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct InventoryItem {
    pub item: String,
    pub quantity: f64,
    pub quantity_type: QuantityType,
}

pub async fn create_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await
}

pub async fn migrate(pool: &SqlitePool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

pub fn app(pool: SqlitePool) -> Router {
    Router::new()
        .route("/v1/inventory", get(list_inventory))
        .with_state(AppState::new(pool))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

async fn list_inventory(
    State(state): State<AppState>,
) -> Result<Json<Vec<InventoryItem>>, axum::http::StatusCode> {
    let items = sqlx::query_as::<_, InventoryItem>(
        r#"
        SELECT item, quantity, quantity_type
        FROM inventory
        ORDER BY item COLLATE NOCASE
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(items))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::ServiceExt;

    async fn test_pool() -> SqlitePool {
        let pool = create_pool("sqlite::memory:").await.unwrap();
        migrate(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn migrations_seed_inventory_rows() {
        let pool = test_pool().await;

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM inventory")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert!(count > 0);
    }

    #[tokio::test]
    async fn inventory_endpoint_returns_seeded_items() {
        let response = app(test_pool().await)
            .oneshot(
                Request::builder()
                    .uri("/v1/inventory")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();

        assert!(json.as_array().unwrap().iter().any(|row| {
            row.get("item") == Some(&Value::String("Canned tomatoes".to_owned()))
                && row.get("quantity").and_then(Value::as_f64) == Some(24.0)
                && row.get("quantity_type") == Some(&Value::String("count".to_owned()))
        }));
    }
}
