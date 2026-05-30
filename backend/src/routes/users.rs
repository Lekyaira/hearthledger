use crate::app_state::AppState;
use crate::user::{NewUser, User};
use axum::{Json, extract::State, http::StatusCode};

pub(super) async fn list_users(
    State(state): State<AppState>,
) -> Result<Json<Vec<User>>, StatusCode> {
    let users = sqlx::query_as::<_, User>(
        r#"
        SELECT CAST(id AS TEXT) AS id, name, role
        FROM users
        ORDER BY name COLLATE NOCASE, id COLLATE NOCASE
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(users))
}

pub(super) async fn create_users(
    State(state): State<AppState>,
    Json(new_users): Json<Vec<NewUser>>,
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
            INSERT INTO users (name, role)
            VALUES (?, ?)
            RETURNING CAST(id AS TEXT) AS id, name, role
            "#,
        )
        .bind(new_user.name)
        .bind(new_user.role.as_str())
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

pub(super) async fn update_users(
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
            SET name = ?, role = ?
            WHERE id = ?
            RETURNING CAST(id AS TEXT) AS id, name, role
            "#,
        )
        .bind(updated_user.name)
        .bind(updated_user.role.as_str())
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

pub(super) async fn delete_users(
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

#[cfg(test)]
mod tests {
    use crate::db::tests::test_pool;
    use crate::routes::app;
    use crate::routes::test_support::{empty_request, json_body, json_request};
    use axum::http::{Method, StatusCode};
    use serde_json::Value;
    use tower::ServiceExt;

    #[tokio::test]
    async fn users_endpoint_returns_seeded_test_users() {
        let response = app(test_pool().await)
            .oneshot(empty_request("/v1/users"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let json = json_body(response).await;
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
            .oneshot(json_request(
                Method::POST,
                "/v1/users",
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
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let json = json_body(response).await;
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
            .oneshot(json_request(
                Method::POST,
                "/v1/users",
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
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let response = app
            .oneshot(json_request(
                Method::PUT,
                "/v1/users",
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
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let json = json_body(response).await;
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
            .oneshot(json_request(
                Method::PUT,
                "/v1/users",
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
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn users_endpoint_deletes_users() {
        let app = app(test_pool().await);

        let response = app
            .clone()
            .oneshot(json_request(
                Method::POST,
                "/v1/users",
                r#"
                [
                    {
                        "name": "Alex",
                        "role": "member"
                    }
                ]
                "#,
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let response = app
            .clone()
            .oneshot(json_request(
                Method::DELETE,
                "/v1/users",
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
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let response = app.oneshot(empty_request("/v1/users")).await.unwrap();

        let json = json_body(response).await;

        assert_eq!(json.as_array().unwrap().len(), 2);
    }
}
