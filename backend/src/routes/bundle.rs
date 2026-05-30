use crate::app_state::AppState;
use crate::bundle::*;
use crate::database_error_to_status;
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::get,
};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use std::collections::HashMap;

pub(super) async fn get_bundle(
    State(state): State<AppState>,
    Query(query): Query<BundleQuery>,
) -> Result<Json<Bundle>, StatusCode> {
    load_bundle(&state.pool, query.id).await.map(Json)
}

pub(super) async fn create_bundle(
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
        RETURNING id, CAST(user AS TEXT) AS user, created_at, bundled, fulfilled_at
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

pub(super) async fn update_bundle(
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
        SELECT id, CAST(user AS TEXT) AS user, created_at, bundled, fulfilled_at
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
        RETURNING id, CAST(user AS TEXT) AS user, created_at, bundled, fulfilled_at
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

pub(super) async fn delete_bundle(
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
        SELECT CAST(b.user AS TEXT) AS user, bi.item_id, i.item, bi.quantity
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

pub(super) async fn records_to_bundle_list_entries(
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
        SELECT id, CAST(user AS TEXT) AS user, created_at, bundled, fulfilled_at
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
