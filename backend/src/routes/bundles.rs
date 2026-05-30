use crate::app_state::AppState;
use crate::bundle::{BundleListEntry, BundleRecord};
use crate::routes::records_to_bundle_list_entries;

use axum::{Json, extract::State, http::StatusCode};

pub(super) async fn list_bundles(
    State(state): State<AppState>,
) -> Result<Json<Vec<BundleListEntry>>, StatusCode> {
    let records = sqlx::query_as::<_, BundleRecord>(
        r#"
        SELECT id, CAST(user AS TEXT) AS user, created_at, bundled, fulfilled_at
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
