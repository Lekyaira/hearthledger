use axum::http::StatusCode;

mod app_state;
mod bundle;
pub mod db;
mod item;
pub mod routes;
mod user;

fn database_error_to_status(error: sqlx::Error) -> StatusCode {
    match error {
        sqlx::Error::Database(database_error) if database_error.is_unique_violation() => {
            StatusCode::CONFLICT
        }
        sqlx::Error::Database(_) => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
