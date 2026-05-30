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
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::ServiceExt;

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
    async fn users_endpoint_returns_seeded_test_users() {
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

        let users = json.as_array().unwrap();

        assert_eq!(users.len(), 2);
        assert!(users.iter().any(|row| {
            row.get("id") == Some(&Value::String("1".to_owned()))
                && row.get("name") == Some(&Value::String("Test Member".to_owned()))
                && row.get("role") == Some(&Value::String("member".to_owned()))
        }));
        assert!(users.iter().any(|row| {
            row.get("id") == Some(&Value::String("2".to_owned()))
                && row.get("name") == Some(&Value::String("Test Admin".to_owned()))
                && row.get("role") == Some(&Value::String("admin".to_owned()))
        }));
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
                                "name": "Alex",
                                "role": "member"
                            },
                            {
                                "name": "Blair",
                                "role": "admin"
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
            row.get("id") == Some(&Value::String("3".to_owned()))
                && row.get("name") == Some(&Value::String("Alex".to_owned()))
                && row.get("role") == Some(&Value::String("member".to_owned()))
        }));
        assert!(users.iter().any(|row| {
            row.get("id") == Some(&Value::String("4".to_owned()))
                && row.get("name") == Some(&Value::String("Blair".to_owned()))
                && row.get("role") == Some(&Value::String("admin".to_owned()))
        }));
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
                                "name": "Alex",
                                "role": "member"
                            },
                            {
                                "name": "Blair",
                                "role": "member"
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
                                "id": "3",
                                "name": "Alex Morgan",
                                "role": "admin"
                            },
                            {
                                "id": "4",
                                "name": "Blair Chen",
                                "role": "member"
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
            row.get("id") == Some(&Value::String("3".to_owned()))
                && row.get("name") == Some(&Value::String("Alex Morgan".to_owned()))
                && row.get("role") == Some(&Value::String("admin".to_owned()))
        }));
        assert!(users.iter().any(|row| {
            row.get("id") == Some(&Value::String("4".to_owned()))
                && row.get("name") == Some(&Value::String("Blair Chen".to_owned()))
                && row.get("role") == Some(&Value::String("member".to_owned()))
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
                                "id": "99",
                                "name": "Missing User",
                                "role": "member"
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
                                "name": "Alex",
                                "role": "member"
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
                                "id": "3",
                                "name": "Alex",
                                "role": "member"
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

        assert_eq!(json.as_array().unwrap().len(), 2);
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
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/bundle")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
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
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
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
        assert_eq!(
            bundles[0].pointer("/items/0/item_id").and_then(Value::as_i64),
            Some(tomatoes_id)
        );
        assert_eq!(
            bundles[0].pointer("/items/0/item"),
            Some(&Value::String("Canned tomatoes".to_owned()))
        );
        assert_eq!(
            bundles[0].pointer("/items/0/quantity").and_then(Value::as_f64),
            Some(2.0)
        );

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
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/bundle")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
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
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/bundle")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
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
        let drew_id: i64 = sqlx::query_scalar(
            "INSERT INTO users (name, role) VALUES ('Drew', 'member') RETURNING id",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let ellis_id: i64 = sqlx::query_scalar(
            "INSERT INTO users (name, role) VALUES ('Ellis', 'member') RETURNING id",
        )
        .fetch_one(&pool)
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
                            "user": "{drew_id}",
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
                            "user": "{ellis_id}",
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
                            "user": "{drew_id}",
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
                            "user": "{drew_id}",
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
