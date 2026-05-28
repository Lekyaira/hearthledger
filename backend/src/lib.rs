use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Deserialize, Serialize, sqlx::Type)]
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

impl QuantityType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Count => "count",
            Self::Grams => "grams",
            Self::Ounces => "ounces",
            Self::Pounds => "pounds",
            Self::Liters => "liters",
            Self::Milliliters => "milliliters",
            Self::Gallons => "gallons",
        }
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct InventoryItem {
    pub item: String,
    pub quantity: f64,
    pub quantity_type: QuantityType,
}

#[derive(Debug, Deserialize)]
pub struct NewInventoryItem {
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
        .route(
            "/v1/inventory",
            get(list_inventory)
                .post(create_inventory)
                .put(update_inventory)
                .delete(delete_inventory),
        )
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

async fn create_inventory(
    State(state): State<AppState>,
    Json(new_items): Json<Vec<NewInventoryItem>>,
) -> Result<(StatusCode, Json<Vec<InventoryItem>>), StatusCode> {
    let mut transaction = state
        .pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut created_items = Vec::with_capacity(new_items.len());

    for new_item in new_items {
        let created_item = sqlx::query_as::<_, InventoryItem>(
            r#"
            INSERT INTO inventory (item, quantity, quantity_type)
            VALUES (?, ?, ?)
            RETURNING item, quantity, quantity_type
            "#,
        )
        .bind(new_item.item)
        .bind(new_item.quantity)
        .bind(new_item.quantity_type.as_str())
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| match error {
            sqlx::Error::Database(database_error) if database_error.is_unique_violation() => {
                StatusCode::CONFLICT
            }
            sqlx::Error::Database(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

        created_items.push(created_item);
    }

    transaction
        .commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(created_items)))
}

async fn update_inventory(
    State(state): State<AppState>,
    Json(updated_items): Json<Vec<NewInventoryItem>>,
) -> Result<Json<Vec<InventoryItem>>, StatusCode> {
    let mut transaction = state
        .pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut saved_items = Vec::with_capacity(updated_items.len());

    for updated_item in updated_items {
        let saved_item = sqlx::query_as::<_, InventoryItem>(
            r#"
            UPDATE inventory
            SET quantity = ?, quantity_type = ?
            WHERE item = ?
            RETURNING item, quantity, quantity_type
            "#,
        )
        .bind(updated_item.quantity)
        .bind(updated_item.quantity_type.as_str())
        .bind(updated_item.item)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|error| match error {
            sqlx::Error::Database(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

        saved_items.push(saved_item);
    }

    transaction
        .commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(saved_items))
}

async fn delete_inventory(
    State(state): State<AppState>,
    Json(items): Json<Vec<String>>,
) -> Result<StatusCode, StatusCode> {
    let mut transaction = state
        .pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for item in items {
        let result = sqlx::query(
            r#"
            DELETE FROM inventory
            WHERE item = ?
            "#,
        )
        .bind(item)
        .execute(&mut *transaction)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if result.rows_affected() == 0 {
            return Err(StatusCode::NOT_FOUND);
        }
    }

    transaction
        .commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
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

    #[tokio::test]
    async fn inventory_endpoint_creates_items() {
        let response = app(test_pool().await)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/inventory")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"
                        [
                            {
                                "item": "Jasmine rice",
                                "quantity": 5.5,
                                "quantity_type": "pounds"
                            },
                            {
                                "item": "Olive oil",
                                "quantity": 2,
                                "quantity_type": "liters"
                            }
                        ]
                        "#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        let items = json.as_array().unwrap();

        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|row| {
            row.get("item") == Some(&Value::String("Jasmine rice".to_owned()))
                && row.get("quantity").and_then(Value::as_f64) == Some(5.5)
                && row.get("quantity_type") == Some(&Value::String("pounds".to_owned()))
        }));
        assert!(items.iter().any(|row| {
            row.get("item") == Some(&Value::String("Olive oil".to_owned()))
                && row.get("quantity").and_then(Value::as_f64) == Some(2.0)
                && row.get("quantity_type") == Some(&Value::String("liters".to_owned()))
        }));
    }

    #[tokio::test]
    async fn inventory_endpoint_rejects_duplicate_items() {
        let response = app(test_pool().await)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/inventory")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"
                        [
                            {
                                "item": "Canned tomatoes",
                                "quantity": 4,
                                "quantity_type": "count"
                            }
                        ]
                        "#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn inventory_endpoint_updates_items() {
        let response = app(test_pool().await)
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/v1/inventory")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"
                        [
                            {
                                "item": "Canned tomatoes",
                                "quantity": 18,
                                "quantity_type": "count"
                            },
                            {
                                "item": "All-purpose flour",
                                "quantity": 8.25,
                                "quantity_type": "pounds"
                            }
                        ]
                        "#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        let items = json.as_array().unwrap();

        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|row| {
            row.get("item") == Some(&Value::String("Canned tomatoes".to_owned()))
                && row.get("quantity").and_then(Value::as_f64) == Some(18.0)
                && row.get("quantity_type") == Some(&Value::String("count".to_owned()))
        }));
        assert!(items.iter().any(|row| {
            row.get("item") == Some(&Value::String("All-purpose flour".to_owned()))
                && row.get("quantity").and_then(Value::as_f64) == Some(8.25)
                && row.get("quantity_type") == Some(&Value::String("pounds".to_owned()))
        }));
    }

    #[tokio::test]
    async fn inventory_endpoint_rejects_update_for_missing_item() {
        let response = app(test_pool().await)
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/v1/inventory")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"
                        [
                            {
                                "item": "Wildflower honey",
                                "quantity": 3,
                                "quantity_type": "count"
                            }
                        ]
                        "#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn inventory_endpoint_deletes_items() {
        let app = app(test_pool().await);

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/v1/inventory")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"["Paper towels"]"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }
}
