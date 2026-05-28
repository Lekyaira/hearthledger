use std::collections::HashMap;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::get,
};
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

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BundledItem {
    pub item_id: i64,
    pub item: String,
    pub quantity: f64,
}

#[derive(Debug, Serialize)]
pub struct Bundle {
    pub id: i64,
    pub user: String,
    pub items: Vec<BundledItem>,
    pub created_at: String,
    pub bundled: bool,
    pub fulfilled_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BundleListEntry {
    pub id: i64,
    pub user: String,
    pub items: Vec<BundledItem>,
    pub created_at: String,
    pub bundled: bool,
}

#[derive(Debug, Deserialize)]
pub struct NewBundledItem {
    pub item_id: i64,
    pub quantity: f64,
}

#[derive(Debug, Deserialize)]
pub struct NewBundle {
    pub user: String,
    pub bundled: Option<bool>,
    pub items: Vec<NewBundledItem>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatedBundle {
    pub id: i64,
    pub user: String,
    pub bundled: bool,
    pub fulfilled_at: Option<String>,
    pub items: Vec<NewBundledItem>,
}

#[derive(Debug, Deserialize)]
struct BundleQuery {
    id: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct BundleRecord {
    id: i64,
    user: String,
    created_at: String,
    bundled: bool,
    fulfilled_at: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BundleSelectionItem {
    pub user: String,
    pub item_id: i64,
    pub item: String,
    pub quantity: f64,
}

pub async fn create_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    SqlitePoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _metadata| {
            Box::pin(async move {
                sqlx::query("PRAGMA foreign_keys = ON")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
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
        .route(
            "/v1/users",
            get(list_users)
                .post(create_users)
                .put(update_users)
                .delete(delete_users),
        )
        .route("/v1/bundles", get(list_bundles))
        .route(
            "/v1/bundle",
            get(get_bundle)
                .post(create_bundle)
                .put(update_bundle)
                .delete(delete_bundle),
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
        SELECT
            i.item,
            i.quantity - COALESCE(SUM(CASE WHEN b.id IS NOT NULL THEN bi.quantity ELSE 0.0 END), 0.0) AS quantity,
            i.quantity_type
        FROM inventory i
        LEFT JOIN bundled_items bi ON bi.item_id = i.id
        LEFT JOIN bundles b ON b.id = bi.bundle_id AND b.fulfilled_at IS NULL
        GROUP BY i.id, i.item, i.quantity, i.quantity_type
        ORDER BY i.item COLLATE NOCASE
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

async fn list_users(State(state): State<AppState>) -> Result<Json<Vec<User>>, StatusCode> {
    let users = sqlx::query_as::<_, User>(
        r#"
        SELECT id, name
        FROM users
        ORDER BY name COLLATE NOCASE, id COLLATE NOCASE
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(users))
}

async fn create_users(
    State(state): State<AppState>,
    Json(new_users): Json<Vec<User>>,
) -> Result<(StatusCode, Json<Vec<User>>), StatusCode> {
    let mut transaction = state
        .pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut created_users = Vec::with_capacity(new_users.len());

    for new_user in new_users {
        let created_user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, name)
            VALUES (?, ?)
            RETURNING id, name
            "#,
        )
        .bind(new_user.id)
        .bind(new_user.name)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| match error {
            sqlx::Error::Database(database_error) if database_error.is_unique_violation() => {
                StatusCode::CONFLICT
            }
            sqlx::Error::Database(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

        created_users.push(created_user);
    }

    transaction
        .commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(created_users)))
}

async fn update_users(
    State(state): State<AppState>,
    Json(updated_users): Json<Vec<User>>,
) -> Result<Json<Vec<User>>, StatusCode> {
    let mut transaction = state
        .pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut saved_users = Vec::with_capacity(updated_users.len());

    for updated_user in updated_users {
        let saved_user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET name = ?
            WHERE id = ?
            RETURNING id, name
            "#,
        )
        .bind(updated_user.name)
        .bind(updated_user.id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|error| match error {
            sqlx::Error::Database(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

        saved_users.push(saved_user);
    }

    transaction
        .commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(saved_users))
}

async fn delete_users(
    State(state): State<AppState>,
    Json(users): Json<Vec<User>>,
) -> Result<StatusCode, StatusCode> {
    let mut transaction = state
        .pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for user in users {
        let result = sqlx::query(
            r#"
            DELETE FROM users
            WHERE id = ? AND name = ?
            "#,
        )
        .bind(user.id)
        .bind(user.name)
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

async fn list_bundles(
    State(state): State<AppState>,
) -> Result<Json<Vec<BundleListEntry>>, StatusCode> {
    let records = sqlx::query_as::<_, BundleRecord>(
        r#"
        SELECT id, user, created_at, bundled, fulfilled_at
        FROM bundles
        WHERE fulfilled_at IS NULL
        ORDER BY created_at, id
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    records_to_bundle_list_entries(&state.pool, records)
        .await
        .map(Json)
}

async fn get_bundle(
    State(state): State<AppState>,
    Query(query): Query<BundleQuery>,
) -> Result<Json<Bundle>, StatusCode> {
    load_bundle(&state.pool, query.id).await.map(Json)
}

async fn create_bundle(
    State(state): State<AppState>,
    Json(new_bundle): Json<NewBundle>,
) -> Result<(StatusCode, Json<Bundle>), StatusCode> {
    let mut transaction = state
        .pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    validate_bundle_quantities(&mut transaction, None, &new_bundle.items).await?;

    let created_bundle = sqlx::query_as::<_, BundleRecord>(
        r#"
        INSERT INTO bundles (user, bundled)
        VALUES (?, ?)
        RETURNING id, user, created_at, bundled, fulfilled_at
        "#,
    )
    .bind(new_bundle.user)
    .bind(new_bundle.bundled.unwrap_or(false))
    .fetch_one(&mut *transaction)
    .await
    .map_err(database_error_to_status)?;

    replace_bundled_items(&mut transaction, created_bundle.id, new_bundle.items).await?;

    transaction
        .commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let bundle = load_bundle(&state.pool, created_bundle.id).await?;

    Ok((StatusCode::CREATED, Json(bundle)))
}

async fn update_bundle(
    State(state): State<AppState>,
    Json(updated_bundle): Json<UpdatedBundle>,
) -> Result<Json<Bundle>, StatusCode> {
    let mut transaction = state
        .pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let existing_bundle = sqlx::query_as::<_, BundleRecord>(
        r#"
        SELECT id, user, created_at, bundled, fulfilled_at
        FROM bundles
        WHERE id = ?
        "#,
    )
    .bind(updated_bundle.id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    if existing_bundle.fulfilled_at.is_some() {
        return Err(StatusCode::CONFLICT);
    }

    let should_fulfill = updated_bundle.fulfilled_at.is_some();

    validate_bundle_quantities(
        &mut transaction,
        Some(updated_bundle.id),
        &updated_bundle.items,
    )
    .await?;

    let saved_bundle = sqlx::query_as::<_, BundleRecord>(
        r#"
        UPDATE bundles
        SET user = ?, bundled = ?, fulfilled_at = ?
        WHERE id = ?
        RETURNING id, user, created_at, bundled, fulfilled_at
        "#,
    )
    .bind(updated_bundle.user)
    .bind(updated_bundle.bundled)
    .bind(updated_bundle.fulfilled_at)
    .bind(updated_bundle.id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(database_error_to_status)?;

    replace_bundled_items(&mut transaction, updated_bundle.id, updated_bundle.items).await?;

    if should_fulfill {
        remove_bundle_items_from_inventory(&mut transaction, updated_bundle.id).await?;
    }

    transaction
        .commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let bundle = bundle_from_record(&state.pool, saved_bundle).await?;

    Ok(Json(bundle))
}

async fn delete_bundle(
    State(state): State<AppState>,
    Query(query): Query<BundleQuery>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query(
        r#"
        DELETE FROM bundles
        WHERE id = ?
        "#,
    )
    .bind(query.id)
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn bundle_selection_items(
    pool: &SqlitePool,
    bundle_id: i64,
) -> Result<Vec<BundleSelectionItem>, sqlx::Error> {
    sqlx::query_as::<_, BundleSelectionItem>(
        r#"
        SELECT b.user, bi.item_id, i.item, bi.quantity
        FROM bundles b
        JOIN bundled_items bi ON bi.bundle_id = b.id
        JOIN inventory i ON i.id = bi.item_id
        WHERE b.id = ?
        ORDER BY bi.id
        "#,
    )
    .bind(bundle_id)
    .fetch_all(pool)
    .await
}

async fn records_to_bundle_list_entries(
    pool: &SqlitePool,
    records: Vec<BundleRecord>,
) -> Result<Vec<BundleListEntry>, StatusCode> {
    let mut bundles = Vec::with_capacity(records.len());

    for record in records {
        let bundle = bundle_from_record(pool, record).await?;
        bundles.push(BundleListEntry {
            id: bundle.id,
            user: bundle.user,
            items: bundle.items,
            created_at: bundle.created_at,
            bundled: bundle.bundled,
        });
    }

    Ok(bundles)
}

async fn load_bundle(pool: &SqlitePool, bundle_id: i64) -> Result<Bundle, StatusCode> {
    let record = sqlx::query_as::<_, BundleRecord>(
        r#"
        SELECT id, user, created_at, bundled, fulfilled_at
        FROM bundles
        WHERE id = ?
        "#,
    )
    .bind(bundle_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    bundle_from_record(pool, record).await
}

async fn bundle_from_record(pool: &SqlitePool, record: BundleRecord) -> Result<Bundle, StatusCode> {
    let items = sqlx::query_as::<_, BundledItem>(
        r#"
        SELECT bi.item_id, i.item, bi.quantity
        FROM bundled_items bi
        JOIN inventory i ON i.id = bi.item_id
        WHERE bi.bundle_id = ?
        ORDER BY bi.id
        "#,
    )
    .bind(record.id)
    .fetch_all(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Bundle {
        id: record.id,
        user: record.user,
        items,
        created_at: record.created_at,
        bundled: record.bundled,
        fulfilled_at: record.fulfilled_at,
    })
}

async fn replace_bundled_items(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    bundle_id: i64,
    items: Vec<NewBundledItem>,
) -> Result<(), StatusCode> {
    sqlx::query(
        r#"
        DELETE FROM bundled_items
        WHERE bundle_id = ?
        "#,
    )
    .bind(bundle_id)
    .execute(&mut **transaction)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for item in items {
        sqlx::query(
            r#"
            INSERT INTO bundled_items (bundle_id, item_id, quantity)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(bundle_id)
        .bind(item.item_id)
        .bind(item.quantity)
        .execute(&mut **transaction)
        .await
        .map_err(database_error_to_status)?;
    }

    Ok(())
}

async fn validate_bundle_quantities(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    excluded_bundle_id: Option<i64>,
    items: &[NewBundledItem],
) -> Result<(), StatusCode> {
    let mut requested_quantities = HashMap::<i64, f64>::new();

    for item in items {
        if item.quantity <= 0.0 {
            return Err(StatusCode::BAD_REQUEST);
        }

        *requested_quantities.entry(item.item_id).or_insert(0.0) += item.quantity;
    }

    for (item_id, requested_quantity) in requested_quantities {
        let available_quantity = sqlx::query_scalar::<_, f64>(
            r#"
            SELECT i.quantity - COALESCE(
                (
                    SELECT SUM(bi.quantity)
                    FROM bundled_items bi
                    JOIN bundles b ON b.id = bi.bundle_id
                    WHERE bi.item_id = i.id
                        AND b.fulfilled_at IS NULL
                        AND (? IS NULL OR b.id != ?)
                ),
                0.0
            )
            FROM inventory i
            WHERE i.id = ?
            "#,
        )
        .bind(excluded_bundle_id)
        .bind(excluded_bundle_id)
        .bind(item_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::BAD_REQUEST)?;

        if requested_quantity > available_quantity {
            return Err(StatusCode::CONFLICT);
        }
    }

    Ok(())
}

async fn remove_bundle_items_from_inventory(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    bundle_id: i64,
) -> Result<(), StatusCode> {
    let items = sqlx::query_as::<_, BundledItem>(
        r#"
        SELECT bi.item_id, i.item, bi.quantity
        FROM bundled_items bi
        JOIN inventory i ON i.id = bi.item_id
        WHERE bi.bundle_id = ?
        ORDER BY bi.id
        "#,
    )
    .bind(bundle_id)
    .fetch_all(&mut **transaction)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for item in items {
        let result = sqlx::query(
            r#"
            UPDATE inventory
            SET quantity = quantity - ?
            WHERE id = ? AND quantity >= ?
            "#,
        )
        .bind(item.quantity)
        .bind(item.item_id)
        .bind(item.quantity)
        .execute(&mut **transaction)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if result.rows_affected() == 0 {
            return Err(StatusCode::CONFLICT);
        }
    }

    Ok(())
}

fn database_error_to_status(error: sqlx::Error) -> StatusCode {
    match error {
        sqlx::Error::Database(database_error) if database_error.is_unique_violation() => {
            StatusCode::CONFLICT
        }
        sqlx::Error::Database(_) => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
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

    #[tokio::test]
    async fn users_endpoint_returns_empty_list() {
        let response = app(test_pool().await)
            .oneshot(
                Request::builder()
                    .uri("/v1/users")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn users_endpoint_creates_users() {
        let response = app(test_pool().await)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/users")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"
                        [
                            {
                                "id": "usr_alex",
                                "name": "Alex"
                            },
                            {
                                "id": "usr_blair",
                                "name": "Blair"
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
        let users = json.as_array().unwrap();

        assert_eq!(users.len(), 2);
        assert!(users.iter().any(|row| {
            row.get("id") == Some(&Value::String("usr_alex".to_owned()))
                && row.get("name") == Some(&Value::String("Alex".to_owned()))
        }));
        assert!(users.iter().any(|row| {
            row.get("id") == Some(&Value::String("usr_blair".to_owned()))
                && row.get("name") == Some(&Value::String("Blair".to_owned()))
        }));
    }

    #[tokio::test]
    async fn users_endpoint_rejects_duplicate_ids() {
        let app = app(test_pool().await);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/users")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"
                        [
                            {
                                "id": "usr_alex",
                                "name": "Alex"
                            }
                        ]
                        "#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/users")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"
                        [
                            {
                                "id": "usr_alex",
                                "name": "Alex Updated"
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
    async fn users_endpoint_updates_users() {
        let app = app(test_pool().await);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/users")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"
                        [
                            {
                                "id": "usr_alex",
                                "name": "Alex"
                            },
                            {
                                "id": "usr_blair",
                                "name": "Blair"
                            }
                        ]
                        "#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/v1/users")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"
                        [
                            {
                                "id": "usr_alex",
                                "name": "Alex Morgan"
                            },
                            {
                                "id": "usr_blair",
                                "name": "Blair Chen"
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
        let users = json.as_array().unwrap();

        assert_eq!(users.len(), 2);
        assert!(users.iter().any(|row| {
            row.get("id") == Some(&Value::String("usr_alex".to_owned()))
                && row.get("name") == Some(&Value::String("Alex Morgan".to_owned()))
        }));
        assert!(users.iter().any(|row| {
            row.get("id") == Some(&Value::String("usr_blair".to_owned()))
                && row.get("name") == Some(&Value::String("Blair Chen".to_owned()))
        }));
    }

    #[tokio::test]
    async fn users_endpoint_rejects_update_for_missing_user() {
        let response = app(test_pool().await)
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/v1/users")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"
                        [
                            {
                                "id": "usr_missing",
                                "name": "Missing User"
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
    async fn users_endpoint_deletes_users() {
        let app = app(test_pool().await);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/users")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"
                        [
                            {
                                "id": "usr_alex",
                                "name": "Alex"
                            }
                        ]
                        "#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/v1/users")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"
                        [
                            {
                                "id": "usr_alex",
                                "name": "Alex"
                            }
                        ]
                        "#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/users")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn bundle_endpoint_creates_lists_and_selects_bundle_items() {
        let pool = test_pool().await;
        sqlx::query("INSERT INTO users (id, name) VALUES ('usr_alex', 'Alex')")
            .execute(&pool)
            .await
            .unwrap();

        let tomatoes_id: i64 =
            sqlx::query_scalar("SELECT id FROM inventory WHERE item = 'Canned tomatoes'")
                .fetch_one(&pool)
                .await
                .unwrap();

        let app = app(pool.clone());
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/bundle")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"
                        {{
                            "user": "usr_alex",
                            "bundled": false,
                            "items": [
                                {{
                                    "item_id": {tomatoes_id},
                                    "quantity": 2
                                }}
                            ]
                        }}
                        "#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        let bundle_id = json.get("id").and_then(Value::as_i64).unwrap();

        assert_eq!(
            json.get("user"),
            Some(&Value::String("usr_alex".to_owned()))
        );
        assert_eq!(json.get("bundled"), Some(&Value::Bool(false)));
        assert_eq!(
            json.pointer("/items/0/item"),
            Some(&Value::String("Canned tomatoes".to_owned()))
        );
        assert_eq!(
            json.pointer("/items/0/quantity").and_then(Value::as_f64),
            Some(2.0)
        );

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/bundles")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        let bundles = json.as_array().unwrap();

        assert_eq!(bundles.len(), 1);
        assert_eq!(
            bundles[0].get("id").and_then(Value::as_i64),
            Some(bundle_id)
        );
        assert!(bundles[0].get("fulfilled_at").is_none());

        let selected_items = bundle_selection_items(&pool, bundle_id).await.unwrap();

        assert_eq!(selected_items.len(), 1);
        assert_eq!(selected_items[0].user, "usr_alex");
        assert_eq!(selected_items[0].item_id, tomatoes_id);
        assert_eq!(selected_items[0].item, "Canned tomatoes");
        assert_eq!(selected_items[0].quantity, 2.0);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/bundle?id={bundle_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/v1/bundle?id={bundle_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn bundle_endpoint_fulfillment_removes_items_from_inventory() {
        let pool = test_pool().await;
        sqlx::query("INSERT INTO users (id, name) VALUES ('usr_blair', 'Blair')")
            .execute(&pool)
            .await
            .unwrap();

        let paper_towels_id: i64 =
            sqlx::query_scalar("SELECT id FROM inventory WHERE item = 'Paper towels'")
                .fetch_one(&pool)
                .await
                .unwrap();

        let app = app(pool.clone());
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/bundle")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"
                        {{
                            "user": "usr_blair",
                            "items": [
                                {{
                                    "item_id": {paper_towels_id},
                                    "quantity": 3
                                }}
                            ]
                        }}
                        "#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        let bundle_id = json.get("id").and_then(Value::as_i64).unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/v1/bundle")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"
                        {{
                            "id": {bundle_id},
                            "user": "usr_blair",
                            "bundled": true,
                            "fulfilled_at": "2026-05-28T12:00:00Z",
                            "items": [
                                {{
                                    "item_id": {paper_towels_id},
                                    "quantity": 3
                                }}
                            ]
                        }}
                        "#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let remaining_quantity: f64 =
            sqlx::query_scalar("SELECT quantity FROM inventory WHERE id = ?")
                .bind(paper_towels_id)
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(remaining_quantity, 7.0);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/bundles")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn inventory_endpoint_subtracts_pending_bundle_quantities() {
        let pool = test_pool().await;
        sqlx::query("INSERT INTO users (id, name) VALUES ('usr_casey', 'Casey')")
            .execute(&pool)
            .await
            .unwrap();

        let tomatoes_id: i64 =
            sqlx::query_scalar("SELECT id FROM inventory WHERE item = 'Canned tomatoes'")
                .fetch_one(&pool)
                .await
                .unwrap();

        let app = app(pool);
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/bundle")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"
                        {{
                            "user": "usr_casey",
                            "items": [
                                {{
                                    "item_id": {tomatoes_id},
                                    "quantity": 4
                                }}
                            ]
                        }}
                        "#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let response = app
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
                && row.get("quantity").and_then(Value::as_f64) == Some(20.0)
        }));
    }

    #[tokio::test]
    async fn bundle_endpoint_rejects_requests_over_available_quantity() {
        let pool = test_pool().await;
        sqlx::query(
            r#"
            INSERT INTO users (id, name)
            VALUES ('usr_drew', 'Drew'), ('usr_ellis', 'Ellis')
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        let paper_towels_id: i64 =
            sqlx::query_scalar("SELECT id FROM inventory WHERE item = 'Paper towels'")
                .fetch_one(&pool)
                .await
                .unwrap();

        let app = app(pool);
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/bundle")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"
                        {{
                            "user": "usr_drew",
                            "items": [
                                {{
                                    "item_id": {paper_towels_id},
                                    "quantity": 8
                                }}
                            ]
                        }}
                        "#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        let bundle_id = json.get("id").and_then(Value::as_i64).unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/bundle")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"
                        {{
                            "user": "usr_ellis",
                            "items": [
                                {{
                                    "item_id": {paper_towels_id},
                                    "quantity": 3
                                }}
                            ]
                        }}
                        "#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/v1/bundle")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"
                        {{
                            "id": {bundle_id},
                            "user": "usr_drew",
                            "bundled": false,
                            "fulfilled_at": null,
                            "items": [
                                {{
                                    "item_id": {paper_towels_id},
                                    "quantity": 9
                                }}
                            ]
                        }}
                        "#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/v1/bundle")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"
                        {{
                            "id": {bundle_id},
                            "user": "usr_drew",
                            "bundled": false,
                            "fulfilled_at": null,
                            "items": [
                                {{
                                    "item_id": {paper_towels_id},
                                    "quantity": 11
                                }}
                            ]
                        }}
                        "#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }
}
