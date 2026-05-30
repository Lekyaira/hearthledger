use crate::app_state::AppState;
use crate::item::{InventoryItem, NewInventoryItem};
use axum::{Json, extract::State, http::StatusCode};

pub(super) async fn list_inventory(
    State(state): State<AppState>,
) -> Result<Json<Vec<InventoryItem>>, axum::http::StatusCode> {
    let items = sqlx::query_as::<_, InventoryItem>(
        r#"
        SELECT
            i.id,
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

pub(super) async fn create_inventory(
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
            RETURNING id, item, quantity, quantity_type
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

pub(super) async fn update_inventory(
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
            RETURNING id, item, quantity, quantity_type
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

pub(super) async fn delete_inventory(
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
