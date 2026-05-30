use crate::app_state::AppState;
use axum::{Router, routing::get};
use bundle::*;
use bundles::*;
use inventory::*;
use sqlx::SqlitePool;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use users::*;

mod bundle;
mod bundles;
mod inventory;
#[cfg(test)]
mod test_support;
mod users;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tests::test_pool;
    use crate::routes::test_support::{empty_request, json_body, json_request};
    use axum::http::{Method, StatusCode};
    use serde_json::Value;
    use tower::ServiceExt;

    #[tokio::test]
    async fn router_returns_not_found_for_unknown_route() {
        let response = app(test_pool().await)
            .oneshot(empty_request("/v1/not-a-route"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn bundle_endpoint_creates_lists_and_selects_bundle_items() {
        let pool = test_pool().await;
        let user_id: i64 = sqlx::query_scalar(
            "INSERT INTO users (name, role) VALUES ('Alex', 'member') RETURNING id",
        )
        .fetch_one(&pool)
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
            .oneshot(json_request(
                Method::POST,
                "/v1/bundle",
                format!(
                    r#"
                        {{
                            "user": "{user_id}",
                            "bundled": false,
                            "items": [
                                {{
                                    "item_id": {tomatoes_id},
                                    "quantity": 2
                                }}
                            ]
                        }}
                        "#
                ),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let json = json_body(response).await;
        let bundle_id = json.get("id").and_then(Value::as_i64).unwrap();

        assert_eq!(json.get("user"), Some(&Value::String(user_id.to_string())));
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
            .oneshot(empty_request("/v1/bundles"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let json = json_body(response).await;
        let bundles = json.as_array().unwrap();

        assert_eq!(bundles.len(), 1);
        assert_eq!(
            bundles[0].get("id").and_then(Value::as_i64),
            Some(bundle_id)
        );
        assert!(bundles[0].get("fulfilled_at").is_none());
        assert_eq!(
            bundles[0]
                .pointer("/items/0/item_id")
                .and_then(Value::as_i64),
            Some(tomatoes_id)
        );
        assert_eq!(
            bundles[0].pointer("/items/0/item"),
            Some(&Value::String("Canned tomatoes".to_owned()))
        );
        assert_eq!(
            bundles[0]
                .pointer("/items/0/quantity")
                .and_then(Value::as_f64),
            Some(2.0)
        );

        let response = app
            .clone()
            .oneshot(empty_request(format!("/v1/bundle?id={bundle_id}")))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(json_request(
                Method::DELETE,
                format!("/v1/bundle?id={bundle_id}"),
                "",
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn bundle_endpoint_fulfillment_removes_items_from_inventory() {
        let pool = test_pool().await;
        let user_id: i64 = sqlx::query_scalar(
            "INSERT INTO users (name, role) VALUES ('Blair', 'member') RETURNING id",
        )
        .fetch_one(&pool)
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
            .oneshot(json_request(
                Method::POST,
                "/v1/bundle",
                format!(
                    r#"
                        {{
                            "user": "{user_id}",
                            "items": [
                                {{
                                    "item_id": {paper_towels_id},
                                    "quantity": 3
                                }}
                            ]
                        }}
                        "#
                ),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let json = json_body(response).await;
        let bundle_id = json.get("id").and_then(Value::as_i64).unwrap();

        let response = app
            .clone()
            .oneshot(json_request(
                Method::PUT,
                "/v1/bundle",
                format!(
                    r#"
                        {{
                            "id": {bundle_id},
                            "user": "{user_id}",
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
                ),
            ))
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

        let response = app.oneshot(empty_request("/v1/bundles")).await.unwrap();

        let json = json_body(response).await;

        assert_eq!(json.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn inventory_endpoint_subtracts_pending_bundle_quantities() {
        let pool = test_pool().await;
        let user_id: i64 = sqlx::query_scalar(
            "INSERT INTO users (name, role) VALUES ('Casey', 'member') RETURNING id",
        )
        .fetch_one(&pool)
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
            .oneshot(json_request(
                Method::POST,
                "/v1/bundle",
                format!(
                    r#"
                        {{
                            "user": "{user_id}",
                            "items": [
                                {{
                                    "item_id": {tomatoes_id},
                                    "quantity": 4
                                }}
                            ]
                        }}
                        "#
                ),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let response = app.oneshot(empty_request("/v1/inventory")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let json = json_body(response).await;

        assert!(json.as_array().unwrap().iter().any(|row| {
            row.get("item") == Some(&Value::String("Canned tomatoes".to_owned()))
                && row.get("quantity").and_then(Value::as_f64) == Some(20.0)
        }));
    }
}
